use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::entities::{AbortReason, EnforcementPlan, SessionId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DomainEvent {
    SessionRequested {
        session_id: SessionId,
        at: OffsetDateTime,
    },
    SessionPreparationStarted {
        session_id: SessionId,
        at: OffsetDateTime,
    },
    EnforcementApplied {
        session_id: SessionId,
        plan: Box<EnforcementPlan>,
        at: OffsetDateTime,
    },
    SessionStarted {
        session_id: SessionId,
        planned_duration_secs: u64,
        at: OffsetDateTime,
    },
    SessionPaused {
        session_id: SessionId,
        remaining_secs: u64,
        at: OffsetDateTime,
    },
    SessionResumed {
        session_id: SessionId,
        at: OffsetDateTime,
    },
    SessionCompleted {
        session_id: SessionId,
        at: OffsetDateTime,
    },
    SessionAborted {
        session_id: SessionId,
        reason: AbortReason,
        at: OffsetDateTime,
    },
    CrashRecovered {
        session_id: SessionId,
        recovered_remaining_secs: u64,
        at: OffsetDateTime,
    },
    TamperDetected {
        kind: TamperKind,
        detail: Option<String>,
        at: OffsetDateTime,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TamperKind {
    HelperStopped,
    PermissionRevoked { permission: String },
    ClockRollback,
    UpdateMismatch,
    RulesDiverged,
    BinaryReplaced,
}

impl DomainEvent {
    pub fn occurred_at(&self) -> OffsetDateTime {
        match self {
            DomainEvent::SessionRequested { at, .. } => *at,
            DomainEvent::SessionPreparationStarted { at, .. } => *at,
            DomainEvent::EnforcementApplied { at, .. } => *at,
            DomainEvent::SessionStarted { at, .. } => *at,
            DomainEvent::SessionPaused { at, .. } => *at,
            DomainEvent::SessionResumed { at, .. } => *at,
            DomainEvent::SessionCompleted { at, .. } => *at,
            DomainEvent::SessionAborted { at, .. } => *at,
            DomainEvent::CrashRecovered { at, .. } => *at,
            DomainEvent::TamperDetected { at, .. } => *at,
        }
    }
}
