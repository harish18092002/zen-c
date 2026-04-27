use async_trait::async_trait;
use zen_domain::entities::{EnforcementPlan, PermissionStatus};
use zen_os::adapters::{AdapterHealth, AppBlockingAdapter, OsError};

pub struct WindowsAppBlocker;

impl WindowsAppBlocker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsAppBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppBlockingAdapter for WindowsAppBlocker {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError> {
        tracing::info!("Windows: applying {} app rules", plan.app_rules.len());
        // TODO: Implement via Win32 process creation monitoring
        Ok(())
    }

    async fn revoke(&self, revision: i64) -> Result<(), OsError> {
        tracing::info!("Windows: revoking enforcement plan revision {}", revision);
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            status: PermissionStatus::Healthy,
            detail: None,
        }
    }
}
