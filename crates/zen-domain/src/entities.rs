use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub Uuid);

impl ProfileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionMode {
    Focus,
    ShortBreak,
    LongBreak,
    Strict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AbortReason {
    UserRequested,
    PermissionRevoked,
    SystemShutdown,
    TimerCorrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Preparing,
    Active {
        started_at: OffsetDateTime,
        ends_at: OffsetDateTime,
    },
    Paused {
        remaining_secs: u64,
    },
    Completing,
    Completed,
    Aborted {
        reason: AbortReason,
    },
    Recovering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub profile_id: ProfileId,
    pub mode: SessionMode,
    pub state: SessionState,
    pub planned_duration_secs: u64,
    pub created_at: OffsetDateTime,
}

impl Session {
    pub fn new(profile_id: ProfileId, mode: SessionMode, planned_duration_secs: u64) -> Self {
        Self {
            id: SessionId::new(),
            profile_id,
            mode,
            state: SessionState::Idle,
            planned_duration_secs,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockRuleKind {
    App,
    Domain,
    Category,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRule {
    pub id: Uuid,
    pub profile_id: ProfileId,
    pub kind: BlockRuleKind,
    pub target: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub revision: i64,
    pub block_rules: Vec<BlockRule>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionStatus {
    Healthy,
    Degraded,
    Unavailable,
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionState {
    pub name: String,
    pub status: PermissionStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockAction {
    Block,
    Allow,
    Redirect { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strictness {
    Normal,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub bundle_id: Option<String>,
    pub executable_path: Option<String>,
    pub action: BlockAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRule {
    pub pattern: String,
    pub normalized: String,
    pub action: BlockAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    pub cidr: String,
    pub action: BlockAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementPlan {
    pub revision: i64,
    pub session_id: SessionId,
    pub app_rules: Vec<AppRule>,
    pub domain_rules: Vec<DomainRule>,
    pub network_rules: Vec<NetworkRule>,
    pub strictness: Strictness,
    pub plan_hash: String,
}
