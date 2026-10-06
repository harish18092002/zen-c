use async_trait::async_trait;
use zen_domain::entities::EnforcementPlan;

use crate::ports::{AdapterError, AdapterHealth, AdapterStatus, BlockingAdapter};

/// A blocking adapter that records what would have been enforced but performs
/// no OS-level action. Used as a fallback on platforms with no real adapter
/// wired up yet, and inside tests where enforcement is out of scope.
pub struct NoopBlockingAdapter;

impl NoopBlockingAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopBlockingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockingAdapter for NoopBlockingAdapter {
    async fn apply_plan(&self, plan: &EnforcementPlan) -> Result<(), AdapterError> {
        tracing::debug!(
            revision = plan.revision,
            apps = plan.app_rules.len(),
            domains = plan.domain_rules.len(),
            "noop blocker: apply_plan"
        );
        Ok(())
    }

    async fn revoke_plan(&self, plan_revision: i64) -> Result<(), AdapterError> {
        tracing::debug!(revision = plan_revision, "noop blocker: revoke_plan");
        Ok(())
    }

    async fn probe_health(&self) -> AdapterHealth {
        AdapterHealth {
            status: AdapterStatus::Degraded,
            detail: Some("no platform blocker wired".into()),
        }
    }
}
