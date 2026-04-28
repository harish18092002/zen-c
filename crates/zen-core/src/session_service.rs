use std::sync::Arc;

use time::OffsetDateTime;
use tracing::{info, warn};
use zen_domain::{
    entities::{AbortReason, ProfileId, Session, SessionId, SessionMode, Strictness},
    errors::DomainError,
    policy::PolicyCompiler,
    session::SessionFsm,
};

use crate::ports::{BlockingAdapter, ProfileRepository, SessionRepository};

pub struct SessionService {
    sessions: Arc<dyn SessionRepository>,
    profiles: Arc<dyn ProfileRepository>,
    blocker: Arc<dyn BlockingAdapter>,
}

impl SessionService {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        profiles: Arc<dyn ProfileRepository>,
        blocker: Arc<dyn BlockingAdapter>,
    ) -> Self {
        Self {
            sessions,
            profiles,
            blocker,
        }
    }

    pub async fn start_session(
        &self,
        profile_id: ProfileId,
        duration_secs: u64,
        mode: SessionMode,
        strictness: Strictness,
    ) -> Result<SessionId, DomainError> {
        let profile = self
            .profiles
            .find_by_id(&profile_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Profile".to_string(),
                id: format!("{:?}", profile_id),
            })?;

        let session = Session::new(profile_id, mode, duration_secs);
        let session_id = session.id.clone();

        let mut fsm = SessionFsm::new(session_id.clone(), duration_secs);

        let requested_event = fsm.request_start()?;
        self.sessions.append_event(&requested_event).await?;

        let plan = PolicyCompiler::compile(session_id.clone(), &profile, 1, strictness)?;

        let enforced_event = fsm.apply_enforcement(plan.clone())?;
        self.sessions.append_event(&enforced_event).await?;

        if let Err(e) = self.blocker.apply_plan(&plan).await {
            warn!("Blocking adapter reported error: {e}; continuing with degraded enforcement");
        }

        let now = OffsetDateTime::now_utc();
        let started_event = fsm.start(now)?;
        self.sessions.append_event(&started_event).await?;

        self.sessions.save(&session).await?;

        info!("Session {} started ({}s)", session_id.0, duration_secs);
        Ok(session_id)
    }

    pub async fn stop_session(
        &self,
        session_id: &SessionId,
        reason: AbortReason,
    ) -> Result<(), DomainError> {
        let session =
            self.sessions
                .find_by_id(session_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    entity: "Session".to_string(),
                    id: format!("{:?}", session_id),
                })?;

        let mut fsm = SessionFsm::new(session_id.clone(), session.planned_duration_secs);
        let aborted_event = fsm.abort(reason)?;
        self.sessions.append_event(&aborted_event).await?;

        if let Err(e) = self.blocker.revoke_plan(1).await {
            warn!("Failed to revoke enforcement plan: {e}");
        }

        info!("Session {} stopped", session_id.0);
        Ok(())
    }

    pub async fn recover_active_session(&self) -> Result<Option<SessionId>, DomainError> {
        let Some(session) = self.sessions.find_active().await? else {
            return Ok(None);
        };

        let session_id = session.id.clone();
        let mut fsm = SessionFsm::new(session_id.clone(), session.planned_duration_secs);
        let recovery_event = fsm.recover(session.planned_duration_secs);
        self.sessions.append_event(&recovery_event).await?;

        info!("Session {} recovered after restart", session_id.0);
        Ok(Some(session_id))
    }
}
