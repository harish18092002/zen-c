use async_trait::async_trait;
use zen_domain::entities::{EnforcementPlan, PermissionStatus};
use zen_os::adapters::{AdapterHealth, AppBlockingAdapter, OsError};

pub struct MacOsAppBlocker;

impl MacOsAppBlocker {
    pub fn new() -> Self {
        Self
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
        tracing::info!("macOS: applying {} app rules", plan.app_rules.len());
        // TODO: Register NSWorkspace observer, enforce via Accessibility APIs
        Ok(())
    }

    async fn revoke(&self, revision: i64) -> Result<(), OsError> {
        tracing::info!("macOS: revoking enforcement plan revision {}", revision);
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            status: PermissionStatus::Healthy,
            detail: None,
        }
    }
}
