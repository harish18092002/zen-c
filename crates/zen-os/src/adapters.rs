use async_trait::async_trait;
use zen_domain::entities::{EnforcementPlan, PermissionStatus};

#[async_trait]
pub trait AppBlockingAdapter: Send + Sync {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError>;
    async fn revoke(&self, revision: i64) -> Result<(), OsError>;
    fn health(&self) -> AdapterHealth;
}

#[async_trait]
pub trait NetworkBlockingAdapter: Send + Sync {
    async fn apply(&self, plan: &EnforcementPlan) -> Result<(), OsError>;
    async fn revoke(&self, revision: i64) -> Result<(), OsError>;
    fn health(&self) -> AdapterHealth;
}

#[derive(Debug, Clone)]
pub struct AdapterHealth {
    pub status: PermissionStatus,
    pub detail: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OsError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("OS API error: {0}")]
    ApiError(String),
    #[error("Not supported on this platform: {0}")]
    Unsupported(String),
}
