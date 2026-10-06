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

impl SessionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionMode::Focus => "Focus",
            SessionMode::ShortBreak => "ShortBreak",
            SessionMode::LongBreak => "LongBreak",
            SessionMode::Strict => "Strict",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Focus" => Some(SessionMode::Focus),
            "ShortBreak" => Some(SessionMode::ShortBreak),
            "LongBreak" => Some(SessionMode::LongBreak),
            "Strict" => Some(SessionMode::Strict),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AbortReason {
    UserRequested,
    PermissionRevoked,
    SystemShutdown,
    TimerCorrupted,
}

impl AbortReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            AbortReason::UserRequested => "UserRequested",
            AbortReason::PermissionRevoked => "PermissionRevoked",
            AbortReason::SystemShutdown => "SystemShutdown",
            AbortReason::TimerCorrupted => "TimerCorrupted",
        }
    }
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

impl SessionState {
    pub fn discriminant(&self) -> &'static str {
        match self {
            SessionState::Idle => "Idle",
            SessionState::Preparing => "Preparing",
            SessionState::Active { .. } => "Active",
            SessionState::Paused { .. } => "Paused",
            SessionState::Completing => "Completing",
            SessionState::Completed => "Completed",
            SessionState::Aborted { .. } => "Aborted",
            SessionState::Recovering => "Recovering",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, SessionState::Completed | SessionState::Aborted { .. })
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            SessionState::Preparing
                | SessionState::Active { .. }
                | SessionState::Paused { .. }
                | SessionState::Completing
                | SessionState::Recovering
        )
    }
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

impl BlockRuleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockRuleKind::App => "App",
            BlockRuleKind::Domain => "Domain",
            BlockRuleKind::Category => "Category",
            BlockRuleKind::Network => "Network",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "App" => Some(BlockRuleKind::App),
            "Domain" => Some(BlockRuleKind::Domain),
            "Category" => Some(BlockRuleKind::Category),
            "Network" => Some(BlockRuleKind::Network),
            _ => None,
        }
    }
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

impl PermissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionStatus::Healthy => "Healthy",
            PermissionStatus::Degraded => "Degraded",
            PermissionStatus::Unavailable => "Unavailable",
            PermissionStatus::PermissionDenied => "PermissionDenied",
        }
    }
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
