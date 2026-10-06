use std::str::FromStr;

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;
use zen_core::ports::SessionRepository;
use zen_domain::{
    entities::{AbortReason, Session, SessionId, SessionMode, SessionState},
    errors::DomainError,
    events::DomainEvent,
};

use crate::error::{map_serde, map_sqlx, map_time_format, map_time_parse, map_uuid};
use crate::tamper_repo::insert_tamper_event;

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
    async fn save(&self, session: &Session) -> Result<(), DomainError> {
        let id_str = session.id.0.to_string();
        let profile_id_str = session.profile_id.0.to_string();
        let mode_str = session.mode.as_str();
        let state_kind = session.state.discriminant();
        let created_at = session
            .created_at
            .format(&Rfc3339)
            .map_err(map_time_format)?;
        let now_str = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(map_time_format)?;

        let (started_at_opt, ends_at_opt, completed_at_opt, paused_secs, aborted_reason) =
            extract_state_columns(&session.state)?;

        sqlx::query(
            "INSERT INTO sessions (id, profile_id, mode, state, planned_duration_secs,
                                   started_at, ends_at, paused_remaining_secs, aborted_reason,
                                   completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                profile_id            = excluded.profile_id,
                mode                  = excluded.mode,
                state                 = excluded.state,
                planned_duration_secs = excluded.planned_duration_secs,
                started_at            = COALESCE(excluded.started_at, sessions.started_at),
                ends_at               = excluded.ends_at,
                paused_remaining_secs = excluded.paused_remaining_secs,
                aborted_reason        = COALESCE(excluded.aborted_reason, sessions.aborted_reason),
                completed_at          = COALESCE(excluded.completed_at, sessions.completed_at),
                updated_at            = excluded.updated_at",
        )
        .bind(&id_str)
        .bind(&profile_id_str)
        .bind(mode_str)
        .bind(state_kind)
        .bind(session.planned_duration_secs as i64)
        .bind(&started_at_opt)
        .bind(&ends_at_opt)
        .bind(paused_secs)
        .bind(aborted_reason)
        .bind(&completed_at_opt)
        .bind(&created_at)
        .bind(&now_str)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(())
    }

    async fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>, DomainError> {
        let id_str = id.0.to_string();
        let row_opt = sqlx::query(SESSION_SELECT)
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;

        match row_opt {
            Some(row) => Ok(Some(row_to_session(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_active(&self) -> Result<Option<Session>, DomainError> {
        let row_opt = sqlx::query(
            "SELECT id, profile_id, mode, state, planned_duration_secs,
                    started_at, ends_at, paused_remaining_secs, aborted_reason,
                    completed_at, created_at
             FROM sessions
             WHERE state IN ('Preparing', 'Active', 'Paused', 'Completing', 'Recovering')
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        match row_opt {
            Some(row) => Ok(Some(row_to_session(&row)?)),
            None => Ok(None),
        }
    }

    async fn append_event(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let payload = serde_json::to_string(event).map_err(map_serde)?;
        let event_type = event.event_type();
        let at = event
            .occurred_at()
            .format(&Rfc3339)
            .map_err(map_time_format)?;

        match event {
            DomainEvent::TamperDetected { kind, detail, .. } => {
                insert_tamper_event(&self.pool, kind, detail.as_deref(), &at).await?;
            }
            _ => {
                let session_id = event.session_id().map(|s| s.0.to_string()).ok_or_else(|| {
                    DomainError::Persistence {
                        reason: format!("event {event_type} has no session_id"),
                    }
                })?;

                sqlx::query(
                    "INSERT INTO session_events (session_id, event_type, payload, occurred_at)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .bind(&session_id)
                .bind(event_type)
                .bind(&payload)
                .bind(&at)
                .execute(&self.pool)
                .await
                .map_err(map_sqlx)?;

                // Mirror EnforcementApplied into enforcement_state for observability.
                if let DomainEvent::EnforcementApplied { plan, .. } = event {
                    sqlx::query(
                        "INSERT INTO enforcement_state (session_id, plan_revision, plan_hash, applied_at, status)
                         VALUES (?1, ?2, ?3, ?4, 'Applied')",
                    )
                    .bind(&session_id)
                    .bind(plan.revision)
                    .bind(&plan.plan_hash)
                    .bind(&at)
                    .execute(&self.pool)
                    .await
                    .map_err(map_sqlx)?;
                }
            }
        }

        Ok(())
    }
}

const SESSION_SELECT: &str = "SELECT id, profile_id, mode, state, planned_duration_secs,
            started_at, ends_at, paused_remaining_secs, aborted_reason,
            completed_at, created_at
     FROM sessions
     WHERE id = ?1";

/// One row's worth of state-column values, in `sessions` schema order.
/// Tuple form: `(started_at, ends_at, completed_at, paused_remaining_secs, aborted_reason)`.
type StateColumns = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<&'static str>,
);

fn extract_state_columns(state: &SessionState) -> Result<StateColumns, DomainError> {
    Ok(match state {
        SessionState::Active {
            started_at,
            ends_at,
        } => (
            Some(started_at.format(&Rfc3339).map_err(map_time_format)?),
            Some(ends_at.format(&Rfc3339).map_err(map_time_format)?),
            None,
            None,
            None,
        ),
        SessionState::Paused { remaining_secs } => {
            (None, None, None, Some(*remaining_secs as i64), None)
        }
        SessionState::Completed => (
            None,
            None,
            Some(
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .map_err(map_time_format)?,
            ),
            None,
            None,
        ),
        SessionState::Aborted { reason } => (
            None,
            None,
            Some(
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .map_err(map_time_format)?,
            ),
            None,
            Some(reason.as_str()),
        ),
        SessionState::Idle
        | SessionState::Preparing
        | SessionState::Completing
        | SessionState::Recovering => (None, None, None, None, None),
    })
}

fn row_to_session(row: &sqlx::sqlite::SqliteRow) -> Result<Session, DomainError> {
    let id_str: String = row.try_get("id").map_err(map_sqlx)?;
    let profile_id_str: String = row.try_get("profile_id").map_err(map_sqlx)?;
    let mode_str: String = row.try_get("mode").map_err(map_sqlx)?;
    let state_kind: String = row.try_get("state").map_err(map_sqlx)?;
    let planned: i64 = row.try_get("planned_duration_secs").map_err(map_sqlx)?;
    let started_at: Option<String> = row.try_get("started_at").map_err(map_sqlx)?;
    let ends_at: Option<String> = row.try_get("ends_at").map_err(map_sqlx)?;
    let paused_remaining: Option<i64> = row.try_get("paused_remaining_secs").map_err(map_sqlx)?;
    let aborted_reason: Option<String> = row.try_get("aborted_reason").map_err(map_sqlx)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_sqlx)?;

    let mode = SessionMode::parse(&mode_str).ok_or_else(|| DomainError::Persistence {
        reason: format!("unknown session mode: {mode_str}"),
    })?;
    let state = rebuild_state(
        &state_kind,
        started_at,
        ends_at,
        paused_remaining,
        aborted_reason,
    )?;
    let id = Uuid::from_str(&id_str).map_err(map_uuid)?;
    let profile_id = Uuid::from_str(&profile_id_str).map_err(map_uuid)?;
    let created_at = parse_datetime(&created_at_str)?;

    Ok(Session {
        id: SessionId(id),
        profile_id: zen_domain::entities::ProfileId(profile_id),
        mode,
        state,
        planned_duration_secs: planned as u64,
        created_at,
    })
}

fn rebuild_state(
    kind: &str,
    started_at: Option<String>,
    ends_at: Option<String>,
    paused_remaining: Option<i64>,
    aborted_reason: Option<String>,
) -> Result<SessionState, DomainError> {
    match kind {
        "Idle" => Ok(SessionState::Idle),
        "Preparing" => Ok(SessionState::Preparing),
        "Completing" => Ok(SessionState::Completing),
        "Completed" => Ok(SessionState::Completed),
        "Recovering" => Ok(SessionState::Recovering),
        "Active" => {
            let s = started_at.ok_or_else(|| DomainError::Persistence {
                reason: "Active row missing started_at".into(),
            })?;
            let e = ends_at.ok_or_else(|| DomainError::Persistence {
                reason: "Active row missing ends_at".into(),
            })?;
            Ok(SessionState::Active {
                started_at: parse_datetime(&s)?,
                ends_at: parse_datetime(&e)?,
            })
        }
        "Paused" => {
            let remaining = paused_remaining.ok_or_else(|| DomainError::Persistence {
                reason: "Paused row missing paused_remaining_secs".into(),
            })?;
            Ok(SessionState::Paused {
                remaining_secs: remaining.max(0) as u64,
            })
        }
        "Aborted" => {
            let reason_str = aborted_reason.ok_or_else(|| DomainError::Persistence {
                reason: "Aborted row missing aborted_reason".into(),
            })?;
            let reason = match reason_str.as_str() {
                "UserRequested" => AbortReason::UserRequested,
                "PermissionRevoked" => AbortReason::PermissionRevoked,
                "SystemShutdown" => AbortReason::SystemShutdown,
                "TimerCorrupted" => AbortReason::TimerCorrupted,
                other => {
                    return Err(DomainError::Persistence {
                        reason: format!("unknown abort reason: {other}"),
                    })
                }
            };
            Ok(SessionState::Aborted { reason })
        }
        other => Err(DomainError::Persistence {
            reason: format!("unknown session state: {other}"),
        }),
    }
}

fn parse_datetime(s: &str) -> Result<OffsetDateTime, DomainError> {
    if let Ok(t) = OffsetDateTime::parse(s, &Rfc3339) {
        return Ok(t);
    }
    let fmt = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let pdt = time::PrimitiveDateTime::parse(s, &fmt).map_err(map_time_parse)?;
    Ok(pdt.assume_utc())
}
