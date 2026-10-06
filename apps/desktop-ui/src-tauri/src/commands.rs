use std::str::FromStr;

use tauri::State;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;
use uuid::Uuid;
use zen_domain::entities::{AbortReason, BlockRule, Profile, ProfileId, SessionId};
use zen_domain::errors::DomainError;

use crate::dto::{
    block_rule_from_input, parse_session_mode, parse_strictness, profile_to_dto, session_to_dto,
    PermissionDto, ProfileDto, ProfileUpsertRequest, SessionDto, StartSessionRequest,
    StartSessionResult, TamperRecordDto,
};
use crate::permissions::probe_all;
use crate::state::AppState;

/// Translate domain errors into the string Tauri ships to JS. We keep messages
/// human-readable; full structured errors live in the tracing logs.
fn map_err(err: DomainError) -> String {
    let s = err.to_string();
    error!(error = %s, "command failed");
    s
}

fn parse_uuid(label: &str, value: &str) -> Result<Uuid, String> {
    Uuid::from_str(value).map_err(|e| format!("invalid {label}: {e}"))
}

#[tauri::command]
pub async fn start_session(
    state: State<'_, AppState>,
    payload: StartSessionRequest,
) -> Result<StartSessionResult, String> {
    let profile_uuid = parse_uuid("profileId", &payload.profile_id)?;
    let mode = parse_session_mode(&payload.mode)?;
    let strictness = parse_strictness(payload.strictness.as_deref())?;

    let started = state
        .service
        .start_session(
            ProfileId(profile_uuid),
            payload.duration_secs,
            mode,
            strictness,
        )
        .await
        .map_err(map_err)?;

    state
        .timer
        .start(started.session_id.clone(), payload.duration_secs)
        .await;

    Ok(StartSessionResult {
        session_id: started.session_id.0.to_string(),
        started_at: started.started_at.format(&Rfc3339).unwrap_or_default(),
        ends_at: started.ends_at.format(&Rfc3339).unwrap_or_default(),
        plan_revision: started.plan.revision,
        plan_hash: started.plan.plan_hash,
    })
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    let id = SessionId(parse_uuid("sessionId", &session_id)?);
    state
        .service
        .stop_session(&id, AbortReason::UserRequested)
        .await
        .map_err(map_err)?;
    state.timer.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn pause_session(state: State<'_, AppState>, session_id: String) -> Result<u64, String> {
    let id = SessionId(parse_uuid("sessionId", &session_id)?);
    let remaining = state.timer.pause().await.ok_or("no active timer")?;
    state
        .service
        .pause_session(&id, remaining)
        .await
        .map_err(map_err)?;
    Ok(remaining)
}

#[tauri::command]
pub async fn resume_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, String> {
    let id = SessionId(parse_uuid("sessionId", &session_id)?);
    let ends_at = state.service.resume_session(&id).await.map_err(map_err)?;
    state.timer.resume().await;
    Ok(ends_at.format(&Rfc3339).unwrap_or_default())
}

#[tauri::command]
pub async fn get_session_state(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<SessionDto>, String> {
    let id = SessionId(parse_uuid("sessionId", &session_id)?);
    let session = state.service.get_session(&id).await.map_err(map_err)?;
    Ok(session.as_ref().map(session_to_dto))
}

#[tauri::command]
pub async fn get_active_session(state: State<'_, AppState>) -> Result<Option<SessionDto>, String> {
    let session = state.service.get_active_session().await.map_err(map_err)?;
    Ok(session.as_ref().map(session_to_dto))
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileDto>, String> {
    let profiles = state.service.list_profiles().await.map_err(map_err)?;
    Ok(profiles.iter().map(profile_to_dto).collect())
}

#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Option<ProfileDto>, String> {
    let id = ProfileId(parse_uuid("profileId", &profile_id)?);
    let profile = state.service.get_profile(&id).await.map_err(map_err)?;
    Ok(profile.as_ref().map(profile_to_dto))
}

#[tauri::command]
pub async fn upsert_profile(
    state: State<'_, AppState>,
    payload: ProfileUpsertRequest,
) -> Result<ProfileDto, String> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err("profile name cannot be empty".into());
    }

    let now = OffsetDateTime::now_utc();
    let (profile_id, created_at, revision) = match &payload.id {
        Some(id_str) => {
            let pid = ProfileId(parse_uuid("profileId", id_str)?);
            let existing = state.service.get_profile(&pid).await.map_err(map_err)?;
            match existing {
                Some(existing) => (pid, existing.created_at, existing.revision + 1),
                None => (pid, now, 1),
            }
        }
        None => (ProfileId::new(), now, 1),
    };

    let block_rules: Result<Vec<BlockRule>, String> = payload
        .block_rules
        .into_iter()
        .map(|input| block_rule_from_input(profile_id.clone(), input))
        .collect();
    let block_rules = block_rules?;

    let profile = Profile {
        id: profile_id.clone(),
        name,
        revision,
        block_rules,
        created_at,
        updated_at: now,
    };

    state
        .service
        .save_profile(&profile)
        .await
        .map_err(map_err)?;

    let stored = state
        .service
        .get_profile(&profile_id)
        .await
        .map_err(map_err)?
        .ok_or("profile vanished after save")?;
    Ok(profile_to_dto(&stored))
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<bool, String> {
    // Refuse if the profile is referenced by the active session.
    if let Some(active) = state.service.get_active_session().await.map_err(map_err)? {
        if active.profile_id.0.to_string() == profile_id {
            return Err("cannot delete a profile used by the active session".into());
        }
    }
    let pid = ProfileId(parse_uuid("profileId", &profile_id)?);
    state.profile_repo().delete(&pid).await.map_err(map_err)
}

#[tauri::command]
pub async fn probe_permissions() -> Result<Vec<PermissionDto>, String> {
    Ok(probe_all())
}

#[tauri::command]
pub async fn list_recent_tamper(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<TamperRecordDto>, String> {
    let records = state
        .tamper
        .list_recent(limit.unwrap_or(20))
        .await
        .map_err(map_err)?;
    Ok(records
        .into_iter()
        .map(|r| TamperRecordDto {
            event_type: r.event_type,
            detail: r.detail,
            detected_at: r.detected_at,
        })
        .collect())
}
