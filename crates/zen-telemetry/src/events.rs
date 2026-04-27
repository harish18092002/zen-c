use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum TelemetryEvent {
    AppStarted { cold_start_ms: u64 },
    SessionStarted { mode: String, duration_secs: u64 },
    SessionCompleted { actual_duration_secs: u64 },
    SessionAborted { reason: String },
    EnforcementFailed { layer: String, code: String },
    CrashRecovered,
    PermissionDenied { permission: String },
}
