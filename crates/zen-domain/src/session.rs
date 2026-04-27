use time::OffsetDateTime;

use crate::entities::{AbortReason, EnforcementPlan, SessionId, SessionState};
use crate::errors::DomainError;
use crate::events::DomainEvent;

pub struct SessionFsm {
    session_id: SessionId,
    state: SessionState,
    planned_duration_secs: u64,
}

impl SessionFsm {
    pub fn new(session_id: SessionId, planned_duration_secs: u64) -> Self {
        Self {
            session_id,
            state: SessionState::Idle,
            planned_duration_secs,
        }
    }

    pub fn restore(session_id: SessionId, state: SessionState, planned_duration_secs: u64) -> Self {
        Self {
            session_id,
            state,
            planned_duration_secs,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn request_start(&mut self) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Idle => {
                self.state = SessionState::Preparing;
                Ok(DomainEvent::SessionRequested {
                    session_id: self.session_id.clone(),
                    at: OffsetDateTime::now_utc(),
                })
            }
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "Preparing".to_string(),
            }),
        }
    }

    pub fn apply_enforcement(&mut self, plan: EnforcementPlan) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Preparing => Ok(DomainEvent::EnforcementApplied {
                session_id: self.session_id.clone(),
                plan: Box::new(plan),
                at: OffsetDateTime::now_utc(),
            }),
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "EnforcementApplied".to_string(),
            }),
        }
    }

    pub fn start(&mut self, now: OffsetDateTime) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Preparing => {
                let ends_at = now + time::Duration::seconds(self.planned_duration_secs as i64);
                self.state = SessionState::Active {
                    started_at: now,
                    ends_at,
                };
                Ok(DomainEvent::SessionStarted {
                    session_id: self.session_id.clone(),
                    planned_duration_secs: self.planned_duration_secs,
                    at: now,
                })
            }
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "Active".to_string(),
            }),
        }
    }

    pub fn pause(&mut self, remaining_secs: u64) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Active { .. } => {
                self.state = SessionState::Paused { remaining_secs };
                Ok(DomainEvent::SessionPaused {
                    session_id: self.session_id.clone(),
                    remaining_secs,
                    at: OffsetDateTime::now_utc(),
                })
            }
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "Paused".to_string(),
            }),
        }
    }

    pub fn resume(&mut self, now: OffsetDateTime) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Paused { remaining_secs } => {
                let remaining = *remaining_secs;
                let ends_at = now + time::Duration::seconds(remaining as i64);
                self.state = SessionState::Active {
                    started_at: now,
                    ends_at,
                };
                Ok(DomainEvent::SessionResumed {
                    session_id: self.session_id.clone(),
                    at: now,
                })
            }
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "Active (resumed)".to_string(),
            }),
        }
    }

    pub fn complete(&mut self) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Active { .. } | SessionState::Completing => {
                self.state = SessionState::Completed;
                Ok(DomainEvent::SessionCompleted {
                    session_id: self.session_id.clone(),
                    at: OffsetDateTime::now_utc(),
                })
            }
            other => Err(DomainError::InvalidTransition {
                from: format!("{:?}", other),
                to: "Completed".to_string(),
            }),
        }
    }

    pub fn abort(&mut self, reason: AbortReason) -> Result<DomainEvent, DomainError> {
        match &self.state {
            SessionState::Completed | SessionState::Aborted { .. } => {
                Err(DomainError::InvalidTransition {
                    from: format!("{:?}", self.state),
                    to: "Aborted".to_string(),
                })
            }
            _ => {
                self.state = SessionState::Aborted {
                    reason: reason.clone(),
                };
                Ok(DomainEvent::SessionAborted {
                    session_id: self.session_id.clone(),
                    reason,
                    at: OffsetDateTime::now_utc(),
                })
            }
        }
    }

    pub fn recover(&mut self, recovered_remaining_secs: u64) -> DomainEvent {
        self.state = SessionState::Recovering;
        DomainEvent::CrashRecovered {
            session_id: self.session_id.clone(),
            recovered_remaining_secs,
            at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{BlockAction, Strictness};

    fn make_plan(session_id: SessionId) -> EnforcementPlan {
        EnforcementPlan {
            revision: 1,
            session_id,
            app_rules: vec![],
            domain_rules: vec![],
            network_rules: vec![],
            strictness: Strictness::Normal,
            plan_hash: "abc123".to_string(),
        }
    }

    #[test]
    fn session_happy_path() {
        let id = SessionId::new();
        let mut fsm = SessionFsm::new(id.clone(), 25 * 60);

        assert!(matches!(fsm.state(), SessionState::Idle));

        fsm.request_start().unwrap();
        assert!(matches!(fsm.state(), SessionState::Preparing));

        fsm.apply_enforcement(make_plan(id)).unwrap();

        let now = OffsetDateTime::now_utc();
        fsm.start(now).unwrap();
        assert!(matches!(fsm.state(), SessionState::Active { .. }));

        fsm.complete().unwrap();
        assert!(matches!(fsm.state(), SessionState::Completed));
    }

    #[test]
    fn invalid_transition_returns_error() {
        let id = SessionId::new();
        let mut fsm = SessionFsm::new(id, 25 * 60);
        let result = fsm.pause(100);
        assert!(result.is_err());
    }
}
