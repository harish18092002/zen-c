//! Serde-friendly DTOs that cross the Tauri IPC boundary.
//!
//! Domain types use `OffsetDateTime` and `Uuid`, which would serialize
//! correctly but with a format the frontend can't always parse cleanly. The
//! DTOs in this module normalize everything to ISO-8601 strings and string
//! IDs, and apply camelCase for the JS side via `#[serde(rename_all)]`.

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use zen_domain::entities::{
    BlockRule, BlockRuleKind, PermissionStatus, Profile, Session, SessionMode, SessionState,
    Strictness,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub profile_id: String,
    pub duration_secs: u64,
    pub mode: String,
    #[serde(default)]
    pub strictness: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub profile_id: String,
    pub mode: String,
    pub state: SessionStateDto,
    pub planned_duration_secs: u64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SessionStateDto {
    Idle,
    Preparing,
    Active { started_at: String, ends_at: String },
    Paused { remaining_secs: u64 },
    Completing,
    Completed,
    Aborted { reason: String },
    Recovering,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDto {
    pub id: String,
    pub name: String,
    pub revision: i64,
    pub block_rules: Vec<BlockRuleDto>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRuleDto {
    pub id: String,
    pub kind: String,
    pub target: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpsertRequest {
    pub id: Option<String>,
    pub name: String,
    pub block_rules: Vec<BlockRuleInput>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRuleInput {
    pub kind: String,
    pub target: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerTickDto {
    pub session_id: String,
    pub remaining_secs: u64,
    pub elapsed_secs: u64,
    pub planned_secs: u64,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResult {
    pub session_id: String,
    pub started_at: String,
    pub ends_at: String,
    pub plan_revision: i64,
    pub plan_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDto {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TamperRecordDto {
    pub event_type: String,
    pub detail: Option<String>,
    pub detected_at: String,
}

// ---------- Conversion helpers ----------

pub fn session_state_to_dto(state: &SessionState) -> SessionStateDto {
    match state {
        SessionState::Idle => SessionStateDto::Idle,
        SessionState::Preparing => SessionStateDto::Preparing,
        SessionState::Active {
            started_at,
            ends_at,
        } => SessionStateDto::Active {
            started_at: started_at.format(&Rfc3339).unwrap_or_default(),
            ends_at: ends_at.format(&Rfc3339).unwrap_or_default(),
        },
        SessionState::Paused { remaining_secs } => SessionStateDto::Paused {
            remaining_secs: *remaining_secs,
        },
        SessionState::Completing => SessionStateDto::Completing,
        SessionState::Completed => SessionStateDto::Completed,
        SessionState::Aborted { reason } => SessionStateDto::Aborted {
            reason: reason.as_str().to_string(),
        },
        SessionState::Recovering => SessionStateDto::Recovering,
    }
}

pub fn session_to_dto(session: &Session) -> SessionDto {
    SessionDto {
        id: session.id.0.to_string(),
        profile_id: session.profile_id.0.to_string(),
        mode: session.mode.as_str().to_string(),
        state: session_state_to_dto(&session.state),
        planned_duration_secs: session.planned_duration_secs,
        created_at: session.created_at.format(&Rfc3339).unwrap_or_default(),
    }
}

pub fn profile_to_dto(profile: &Profile) -> ProfileDto {
    ProfileDto {
        id: profile.id.0.to_string(),
        name: profile.name.clone(),
        revision: profile.revision,
        block_rules: profile
            .block_rules
            .iter()
            .map(|r| BlockRuleDto {
                id: r.id.to_string(),
                kind: r.kind.as_str().to_string(),
                target: r.target.clone(),
                enabled: r.enabled,
            })
            .collect(),
        created_at: profile.created_at.format(&Rfc3339).unwrap_or_default(),
        updated_at: profile.updated_at.format(&Rfc3339).unwrap_or_default(),
    }
}

pub fn parse_session_mode(s: &str) -> Result<SessionMode, String> {
    SessionMode::parse(s).ok_or_else(|| format!("unknown session mode: {s}"))
}

pub fn parse_strictness(s: Option<&str>) -> Result<Strictness, String> {
    match s {
        None | Some("Normal") => Ok(Strictness::Normal),
        Some("Strict") => Ok(Strictness::Strict),
        Some(other) => Err(format!("unknown strictness: {other}")),
    }
}

pub fn parse_block_rule_kind(s: &str) -> Result<BlockRuleKind, String> {
    BlockRuleKind::parse(s).ok_or_else(|| format!("unknown block rule kind: {s}"))
}

pub fn permission_to_dto(
    name: &str,
    status: PermissionStatus,
    detail: Option<String>,
) -> PermissionDto {
    PermissionDto {
        name: name.to_string(),
        status: status.as_str().to_string(),
        detail,
    }
}

pub fn block_rule_from_input(
    profile_id: zen_domain::entities::ProfileId,
    input: BlockRuleInput,
) -> Result<BlockRule, String> {
    let kind = parse_block_rule_kind(&input.kind)?;
    let target = input.target.trim().to_string();
    if target.is_empty() {
        return Err("block rule target cannot be empty".into());
    }
    Ok(BlockRule {
        id: uuid::Uuid::new_v4(),
        profile_id,
        kind,
        target,
        enabled: input.enabled,
    })
}
