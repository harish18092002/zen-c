use async_trait::async_trait;
use zen_domain::{
    entities::{EnforcementPlan, Profile, ProfileId, Session, SessionId},
    errors::DomainError,
    events::DomainEvent,
};

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>, DomainError>;
    async fn find_active(&self) -> Result<Option<Session>, DomainError>;
    async fn append_event(&self, event: &DomainEvent) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ProfileRepository: Send + Sync {
    async fn save(&self, profile: &Profile) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &ProfileId) -> Result<Option<Profile>, DomainError>;
    async fn list_all(&self) -> Result<Vec<Profile>, DomainError>;
}

#[async_trait]
pub trait BlockingAdapter: Send + Sync {
    async fn apply_plan(&self, plan: &EnforcementPlan) -> Result<(), AdapterError>;
    async fn revoke_plan(&self, plan_revision: i64) -> Result<(), AdapterError>;
    async fn probe_health(&self) -> AdapterHealth;
}

#[derive(Debug)]
pub struct AdapterHealth {
    pub status: AdapterStatus,
    pub detail: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdapterStatus {
    Healthy,
    Degraded,
    Unavailable,
    PermissionDenied,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Adapter unavailable: {0}")]
    Unavailable(String),
    #[error("Rule application failed: {0}")]
    ApplicationFailed(String),
}
