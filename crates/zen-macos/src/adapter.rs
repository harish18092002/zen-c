use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use zen_core::ports::{AdapterError, AdapterHealth, AdapterStatus, BlockingAdapter};
use zen_domain::entities::EnforcementPlan;
use zen_os::adapters::{AppBlockingAdapter, NetworkBlockingAdapter};

use crate::app_blocker::MacOsAppBlocker;
use crate::dns_proxy::MacOsDnsProxy;

/// Composite macOS blocker. Delegates app rules to NSWorkspace-driven
/// enforcement and domain rules to the DNS proxy. Holds the currently active
/// plan so revoke / health checks can compare desired vs actual state.
pub struct MacOsBlockingAdapter {
    app: MacOsAppBlocker,
    dns: MacOsDnsProxy,
    active_plan: Arc<Mutex<Option<EnforcementPlan>>>,
}

impl MacOsBlockingAdapter {
    pub fn new() -> Self {
        Self {
            app: MacOsAppBlocker::new(),
            dns: MacOsDnsProxy::new(),
            active_plan: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for MacOsBlockingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockingAdapter for MacOsBlockingAdapter {
    async fn apply_plan(&self, plan: &EnforcementPlan) -> Result<(), AdapterError> {
        self.app
            .apply(plan)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("app: {e}")))?;
        self.dns
            .apply(plan)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("dns: {e}")))?;
        *self.active_plan.lock().await = Some(plan.clone());
        Ok(())
    }

    async fn revoke_plan(&self, revision: i64) -> Result<(), AdapterError> {
        self.app
            .revoke(revision)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("app: {e}")))?;
        self.dns
            .revoke(revision)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("dns: {e}")))?;
        let mut guard = self.active_plan.lock().await;
        if let Some(p) = guard.as_ref() {
            if p.revision == revision {
                *guard = None;
            }
        }
        Ok(())
    }

    async fn probe_health(&self) -> AdapterHealth {
        let app_health = self.app.health();
        let dns_health = self.dns.health();
        let worst = match (app_health.status, dns_health.status) {
            (zen_domain::entities::PermissionStatus::PermissionDenied, _)
            | (_, zen_domain::entities::PermissionStatus::PermissionDenied) => {
                AdapterStatus::PermissionDenied
            }
            (zen_domain::entities::PermissionStatus::Unavailable, _)
            | (_, zen_domain::entities::PermissionStatus::Unavailable) => {
                AdapterStatus::Unavailable
            }
            (zen_domain::entities::PermissionStatus::Degraded, _)
            | (_, zen_domain::entities::PermissionStatus::Degraded) => AdapterStatus::Degraded,
            _ => AdapterStatus::Healthy,
        };
        AdapterHealth {
            status: worst,
            detail: app_health.detail.or(dns_health.detail),
        }
    }
}
