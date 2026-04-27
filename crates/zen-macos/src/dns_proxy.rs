use async_trait::async_trait;
use zen_domain::entities::{EnforcementPlan, PermissionStatus};
use zen_os::adapters::{AdapterHealth, NetworkBlockingAdapter, OsError};

pub struct MacOsDnsProxy;

impl MacOsDnsProxy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsDnsProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkBlockingAdapter for MacOsDnsProxy {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError> {
        tracing::info!("macOS DNS proxy: applying {} domain rules", plan.domain_rules.len());
        // TODO: Reconfigure local DNS proxy resolver rules
        Ok(())
    }

    async fn revoke(&self, revision: i64) -> Result<(), OsError> {
        tracing::info!("macOS DNS proxy: revoking revision {}", revision);
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            status: PermissionStatus::Healthy,
            detail: None,
        }
    }
}
