use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use zen_core::ports::{AdapterError, AdapterHealth, AdapterStatus, BlockingAdapter};
use zen_domain::entities::EnforcementPlan;
use zen_os::adapters::{AppBlockingAdapter, NetworkBlockingAdapter};

use crate::app_blocker::WindowsAppBlocker;
use crate::wfp_blocker::WfpNetworkBlocker;

pub struct WindowsBlockingAdapter {
    app: WindowsAppBlocker,
    wfp: WfpNetworkBlocker,
    active_plan: Arc<Mutex<Option<EnforcementPlan>>>,
}

impl WindowsBlockingAdapter {
    pub fn new() -> Self {
        Self {
            app: WindowsAppBlocker::new(),
            wfp: WfpNetworkBlocker::new(),
            active_plan: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for WindowsBlockingAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockingAdapter for WindowsBlockingAdapter {
    async fn apply_plan(&self, plan: &EnforcementPlan) -> Result<(), AdapterError> {
        self.app
            .apply(plan)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("app: {e}")))?;
        self.wfp
            .apply(plan)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("wfp: {e}")))?;
        *self.active_plan.lock().await = Some(plan.clone());
        Ok(())
    }

    async fn revoke_plan(&self, revision: i64) -> Result<(), AdapterError> {
        self.app
            .revoke(revision)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("app: {e}")))?;
        self.wfp
            .revoke(revision)
            .await
            .map_err(|e| AdapterError::ApplicationFailed(format!("wfp: {e}")))?;
        let mut guard = self.active_plan.lock().await;
        if let Some(p) = guard.as_ref() {
            if p.revision == revision {
                *guard = None;
            }
        }
        Ok(())
    }

    async fn probe_health(&self) -> AdapterHealth {
        AdapterHealth {
            status: AdapterStatus::Degraded,
            detail: Some("windows blocker: stub enforcement".into()),
        }
    }
}
