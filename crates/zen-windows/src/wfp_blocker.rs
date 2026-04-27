use async_trait::async_trait;
use zen_domain::entities::{EnforcementPlan, PermissionStatus};
use zen_os::adapters::{AdapterHealth, NetworkBlockingAdapter, OsError};

pub struct WfpNetworkBlocker;

impl WfpNetworkBlocker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WfpNetworkBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkBlockingAdapter for WfpNetworkBlocker {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError> {
        tracing::info!("WFP: applying {} domain rules", plan.domain_rules.len());
        // TODO: Implement via Windows Filtering Platform API
        Ok(())
    }

    async fn revoke(&self, revision: i64) -> Result<(), OsError> {
        tracing::info!("WFP: revoking revision {}", revision);
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            status: PermissionStatus::Healthy,
            detail: None,
        }
    }
}
