use std::sync::Arc;

use time::OffsetDateTime;
use tracing::{info, warn};
use zen_domain::{
    entities::{
        AbortReason, EnforcementPlan, Profile, ProfileId, Session, SessionId, SessionMode,
        SessionState, Strictness,
    },
    errors::DomainError,
    policy::PolicyCompiler,
    session::SessionFsm,
};

use crate::ports::{BlockingAdapter, ProfileRepository, SessionRepository};

/// Result returned after starting a session — callers (Tauri commands, timer
/// engine) use this to seed the timer with the right end timestamp without
/// having to round-trip through the DB.
#[derive(Debug, Clone)]
pub struct StartedSession {
    pub session_id: SessionId,
    pub started_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub plan: EnforcementPlan,
}

#[derive(Debug, Clone)]
pub struct RecoveredSession {
    pub session: Session,
    pub plan: EnforcementPlan,
    pub remaining_secs: u64,
    pub ends_at: OffsetDateTime,
    pub clock_rollback_suspected: bool,
}

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

    pub fn blocker(&self) -> Arc<dyn BlockingAdapter> {
        self.blocker.clone()
    }

    pub fn sessions(&self) -> Arc<dyn SessionRepository> {
        self.sessions.clone()
    }

    pub fn profiles(&self) -> Arc<dyn ProfileRepository> {
        self.profiles.clone()
    }

    pub async fn start_session(
        &self,
        profile_id: ProfileId,
        duration_secs: u64,
        mode: SessionMode,
        strictness: Strictness,
    ) -> Result<StartedSession, DomainError> {
        if duration_secs == 0 {
            return Err(DomainError::InvalidInput {
                reason: "duration_secs must be > 0".into(),
            });
        }
        if duration_secs > 24 * 60 * 60 {
            return Err(DomainError::InvalidInput {
                reason: "duration_secs must be <= 86400 (24h)".into(),
            });
        }

        // Refuse to start a second session if one is already running.
        if let Some(existing) = self.sessions.find_active().await? {
            return Err(DomainError::InvalidInput {
                reason: format!("another session is already active: {}", existing.id.0),
            });
        }

        let profile = self
            .profiles
            .find_by_id(&profile_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Profile".into(),
                id: profile_id.0.to_string(),
            })?;

        let mut session = Session::new(profile_id, mode, duration_secs);
        let session_id = session.id.clone();
        let mut fsm = SessionFsm::new(session_id.clone(), duration_secs);

        let requested = fsm.request_start()?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&requested).await?;

        let plan =
            PolicyCompiler::compile(session_id.clone(), &profile, profile.revision, strictness)?;
        let enforced = fsm.apply_enforcement(plan.clone())?;
        self.sessions.append_event(&enforced).await?;

        if let Err(e) = self.blocker.apply_plan(&plan).await {
            warn!(error = %e, "blocking adapter degraded; continuing");
        }

        let now = OffsetDateTime::now_utc();
        let started = fsm.start(now)?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&started).await?;

        let ends_at = now + time::Duration::seconds(duration_secs as i64);
        info!(session_id = %session_id.0, duration_secs, "session started");
        Ok(StartedSession {
            session_id,
            started_at: now,
            ends_at,
            plan,
        })
    }

    pub async fn pause_session(
        &self,
        session_id: &SessionId,
        remaining_secs: u64,
    ) -> Result<(), DomainError> {
        let mut session = self.load_session(session_id).await?;
        let mut fsm = SessionFsm::restore(
            session_id.clone(),
            session.state.clone(),
            session.planned_duration_secs,
        );
        let event = fsm.pause(remaining_secs)?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&event).await?;
        info!(session_id = %session_id.0, remaining_secs, "session paused");
        Ok(())
    }

    pub async fn resume_session(
        &self,
        session_id: &SessionId,
    ) -> Result<OffsetDateTime, DomainError> {
        let mut session = self.load_session(session_id).await?;
        let mut fsm = SessionFsm::restore(
            session_id.clone(),
            session.state.clone(),
            session.planned_duration_secs,
        );
        let now = OffsetDateTime::now_utc();
        let event = fsm.resume(now)?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&event).await?;
        let ends_at = match &session.state {
            SessionState::Active { ends_at, .. } => *ends_at,
            _ => now,
        };
        info!(session_id = %session_id.0, "session resumed");
        Ok(ends_at)
    }

    pub async fn complete_session(&self, session_id: &SessionId) -> Result<(), DomainError> {
        let mut session = self.load_session(session_id).await?;
        let mut fsm = SessionFsm::restore(
            session_id.clone(),
            session.state.clone(),
            session.planned_duration_secs,
        );
        let event = fsm.complete()?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&event).await?;

        if let Err(e) = self.blocker.revoke_plan(1).await {
            warn!(error = %e, "failed to revoke plan on complete");
        }
        info!(session_id = %session_id.0, "session completed");
        Ok(())
    }

    pub async fn stop_session(
        &self,
        session_id: &SessionId,
        reason: AbortReason,
    ) -> Result<(), DomainError> {
        let mut session = self.load_session(session_id).await?;
        let mut fsm = SessionFsm::restore(
            session_id.clone(),
            session.state.clone(),
            session.planned_duration_secs,
        );
        let event = fsm.abort(reason)?;
        session.state = fsm.state().clone();
        self.sessions.save(&session).await?;
        self.sessions.append_event(&event).await?;

        if let Err(e) = self.blocker.revoke_plan(1).await {
            warn!(error = %e, "failed to revoke plan on stop");
        }
        info!(session_id = %session_id.0, "session aborted");
        Ok(())
    }

    /// Called once on app startup. If an active session is found in the DB,
    /// re-compile its enforcement plan, re-apply it via the OS adapter, and
    /// return enough state for the timer engine to resume.
    pub async fn recover_active_session(&self) -> Result<Option<RecoveredSession>, DomainError> {
        let Some(session) = self.sessions.find_active().await? else {
            return Ok(None);
        };

        let profile = self
            .profiles
            .find_by_id(&session.profile_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Profile".into(),
                id: session.profile_id.0.to_string(),
            })?;

        let strictness = Strictness::Normal;
        let plan =
            PolicyCompiler::compile(session.id.clone(), &profile, profile.revision, strictness)?;

        // Try to re-apply enforcement; if it fails, we keep the session in
        // Recovering and surface the degradation up the stack.
        if let Err(e) = self.blocker.apply_plan(&plan).await {
            warn!(error = %e, "recovery: blocking adapter degraded");
        }

        let now = OffsetDateTime::now_utc();
        let mut clock_rollback_suspected = false;
        let (ends_at, remaining_secs) = match &session.state {
            SessionState::Active {
                started_at,
                ends_at,
            } => {
                if now < *started_at {
                    clock_rollback_suspected = true;
                }
                let remaining = (*ends_at - now).whole_seconds();
                let clamped = remaining.max(0) as u64;
                (*ends_at, clamped)
            }
            SessionState::Paused { remaining_secs } => (now, *remaining_secs),
            _ => (
                now + time::Duration::seconds(session.planned_duration_secs as i64),
                session.planned_duration_secs,
            ),
        };

        let mut fsm = SessionFsm::restore(
            session.id.clone(),
            session.state.clone(),
            session.planned_duration_secs,
        );
        let recovery_event = fsm.recover(remaining_secs);
        self.sessions.append_event(&recovery_event).await?;

        if clock_rollback_suspected {
            let event = zen_domain::events::DomainEvent::TamperDetected {
                kind: zen_domain::events::TamperKind::ClockRollback,
                detail: Some(format!("now ({now}) < started_at on recovery")),
                at: now,
            };
            self.sessions.append_event(&event).await?;
        }

        info!(session_id = %session.id.0, remaining_secs, clock_rollback_suspected, "session recovered");
        Ok(Some(RecoveredSession {
            session,
            plan,
            remaining_secs,
            ends_at,
            clock_rollback_suspected,
        }))
    }

    pub async fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<Session>, DomainError> {
        self.sessions.find_by_id(session_id).await
    }

    pub async fn get_active_session(&self) -> Result<Option<Session>, DomainError> {
        self.sessions.find_active().await
    }

    // ---------- Profile use cases ----------

    pub async fn save_profile(&self, profile: &Profile) -> Result<(), DomainError> {
        if profile.name.trim().is_empty() {
            return Err(DomainError::InvalidInput {
                reason: "profile name cannot be empty".into(),
            });
        }
        self.profiles.save(profile).await
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>, DomainError> {
        self.profiles.list_all().await
    }

    pub async fn get_profile(&self, id: &ProfileId) -> Result<Option<Profile>, DomainError> {
        self.profiles.find_by_id(id).await
    }

    async fn load_session(&self, session_id: &SessionId) -> Result<Session, DomainError> {
        self.sessions
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Session".into(),
                id: session_id.0.to_string(),
            })
    }
}
