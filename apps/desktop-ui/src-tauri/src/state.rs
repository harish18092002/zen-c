use std::sync::Arc;

use sqlx::SqlitePool;
use zen_core::{SessionService, TimerEngine};
use zen_db::{SqliteProfileRepository, SqliteTamperRepository};

/// Shared, cloneable handle to all long-lived backend services.
///
/// Tauri commands receive `tauri::State<'_, AppState>` and clone the inner
/// `Arc`s they need. Cloning `AppState` itself is cheap because every field is
/// an `Arc` or a shallow handle.
#[derive(Clone)]
pub struct AppState {
    pub service: Arc<SessionService>,
    pub timer: Arc<TimerEngine>,
    pub tamper: Arc<SqliteTamperRepository>,
    pub pool: SqlitePool,
}

impl AppState {
    pub fn profile_repo(&self) -> SqliteProfileRepository {
        SqliteProfileRepository::new(self.pool.clone())
    }
}
