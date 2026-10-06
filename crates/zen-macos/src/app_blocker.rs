use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use sysinfo::{ProcessRefreshKind, RefreshKind, Signal, System};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration, MissedTickBehavior};
use tracing::{debug, info, warn};
use zen_domain::entities::{BlockAction, EnforcementPlan, PermissionStatus};
use zen_os::adapters::{AdapterHealth, AppBlockingAdapter, OsError};

/// Polling-based macOS app blocker.
///
/// Every `POLL_INTERVAL` it walks the system process table and terminates any
/// process whose name (case-insensitive contains) matches a `Block` rule. We
/// also accept absolute executable paths and bundle-id-like strings —
/// `com.apple.Safari` matches a process whose path contains `Safari`. This is
/// a pragmatic v1: a proper Accessibility-driven bundle-id matcher is on the
/// Phase 2 roadmap.
const POLL_INTERVAL: Duration = Duration::from_millis(750);

struct ActiveEnforcement {
    revision: i64,
    /// Targets we asked to block, retained for `health()` and diagnostics.
    #[allow(dead_code)]
    blocked_targets: Vec<String>,
    handle: Option<JoinHandle<()>>,
}

pub struct MacOsAppBlocker {
    inner: Arc<Mutex<Option<ActiveEnforcement>>>,
}

impl MacOsAppBlocker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for MacOsAppBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppBlockingAdapter for MacOsAppBlocker {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError> {
        let targets: Vec<String> = plan
            .app_rules
            .iter()
            .filter(|r| matches!(r.action, BlockAction::Block))
            .filter_map(|r| {
                r.bundle_id
                    .clone()
                    .or_else(|| r.executable_path.clone())
                    .map(|s| s.to_lowercase())
            })
            .collect();

        info!(
            revision = plan.revision,
            targets = targets.len(),
            "macOS app blocker: starting enforcement loop"
        );

        let mut guard = self.inner.lock().await;
        if let Some(prev) = guard.take() {
            if let Some(handle) = prev.handle {
                handle.abort();
            }
        }

        let inner_targets = targets.clone();
        let handle = if inner_targets.is_empty() {
            None
        } else {
            Some(tokio::spawn(run_poll_loop(inner_targets)))
        };

        *guard = Some(ActiveEnforcement {
            revision: plan.revision,
            blocked_targets: targets,
            handle,
        });

        Ok(())
    }

    async fn revoke(&self, revision: i64) -> Result<(), OsError> {
        let mut guard = self.inner.lock().await;
        if let Some(active) = guard.as_ref() {
            if active.revision != revision {
                debug!(
                    current = active.revision,
                    requested = revision,
                    "macOS app blocker: revoke ignored — revision mismatch"
                );
                return Ok(());
            }
        }
        if let Some(prev) = guard.take() {
            if let Some(handle) = prev.handle {
                handle.abort();
            }
            info!(revision, "macOS app blocker: revoked");
        }
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        // Sync probe: AXIsProcessTrusted via FFI. If the user has not granted
        // Accessibility, we can still terminate processes (we use SIGTERM),
        // but window-level guarantees are not available.
        let granted = is_accessibility_trusted();
        AdapterHealth {
            status: if granted {
                PermissionStatus::Healthy
            } else {
                PermissionStatus::Degraded
            },
            detail: if granted {
                None
            } else {
                Some("Accessibility permission not granted".into())
            },
        }
    }
}

async fn run_poll_loop(targets: Vec<String>) {
    let needles: HashSet<String> = targets.into_iter().flat_map(|t| split_target(&t)).collect();
    if needles.is_empty() {
        return;
    }
    info!(
        needle_count = needles.len(),
        "macOS app blocker: poll loop started"
    );

    let mut ticker = interval(POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

    loop {
        ticker.tick().await;
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            let exe = process
                .exe()
                .map(|p| p.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if matches_any(&needles, &name, &exe) {
                debug!(pid = pid.as_u32(), name = %name, "macOS app blocker: terminating");
                if !process.kill_with(Signal::Term).unwrap_or(false) {
                    warn!(pid = pid.as_u32(), "macOS app blocker: SIGTERM failed");
                }
            }
        }
    }
}

/// Split a target like `com.apple.Safari` into matcher needles. We always
/// keep the original string plus, for bundle-id-looking strings, the trailing
/// component (`safari`). This lets a single rule match both bundle IDs and
/// process names.
fn split_target(target: &str) -> Vec<String> {
    let trimmed = target.trim().to_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut out = vec![trimmed.clone()];
    if let Some(last_segment) = trimmed.rsplit('.').next() {
        if !last_segment.is_empty() && last_segment != trimmed {
            out.push(last_segment.to_string());
        }
    }
    out
}

fn matches_any(needles: &HashSet<String>, name: &str, exe: &str) -> bool {
    needles
        .iter()
        .any(|needle| name == needle || exe.contains(needle) || name.contains(needle))
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
fn is_accessibility_trusted() -> bool {
    // Safety: AXIsProcessTrusted is documented thread-safe and returns a
    // POSIX-style Boolean (unsigned char) with no parameters.
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
fn is_accessibility_trusted() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_target_emits_bundle_and_tail() {
        let out = split_target("com.apple.Safari");
        assert!(out.contains(&"com.apple.safari".to_string()));
        assert!(out.contains(&"safari".to_string()));
    }

    #[test]
    fn split_target_empty_for_blank() {
        assert!(split_target("   ").is_empty());
    }

    #[test]
    fn matches_any_finds_substring() {
        let needles: HashSet<String> = ["safari".into()].into_iter().collect();
        assert!(matches_any(&needles, "safari", ""));
        assert!(matches_any(
            &needles,
            "Safari Helper".to_lowercase().as_str(),
            ""
        ));
        assert!(matches_any(
            &needles,
            "",
            "/applications/safari.app/contents/macos/safari"
        ));
        assert!(!matches_any(&needles, "chrome", "/usr/bin/chrome"));
    }
}
