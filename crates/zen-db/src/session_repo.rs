use async_trait::async_trait;
use sqlx::SqlitePool;
use zen_domain::{
    entities::{Session, SessionId},
    errors::DomainError,
    events::DomainEvent,
};

use zen_core::ports::SessionRepository;

pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn save(&self, _session: &Session) -> Result<(), DomainError> {
        // TODO: Implement full session persistence
        Ok(())
    }

    async fn find_by_id(&self, _id: &SessionId) -> Result<Option<Session>, DomainError> {
        // TODO: Implement query by session ID
        Ok(None)
    }

    async fn find_active(&self) -> Result<Option<Session>, DomainError> {
        // TODO: Query for sessions in Active/Preparing/Recovering state
        Ok(None)
    }

    async fn append_event(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let payload = serde_json::to_string(event).map_err(|e| {
            DomainError::PlanCompilationFailed {
                reason: e.to_string(),
            }
        })?;
        tracing::debug!("Event appended: {}", payload);
        // TODO: Insert into session_events table
        Ok(())
    }
}
