use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: OperationKind,
    pub payload_encrypted: Option<Vec<u8>>,
    pub lamport_ts: u64,
    pub device_id: String,
    pub created_at: OffsetDateTime,
    pub synced_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationKind {
    Upsert,
    Delete,
}

#[derive(Debug, Clone)]
pub struct SyncCursor {
    pub device_id: String,
    pub last_lamport_ts: u64,
}
