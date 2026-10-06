use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tracing::{info, warn};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use zen_core::NoopBlockingAdapter;
use zen_core::{SessionService, TimerEngine, TimerSnapshot, TimerState};
use zen_db::{
    connection, migrations, SqliteProfileRepository, SqliteSessionRepository,
    SqliteTamperRepository,
};
use zen_domain::entities::{AbortReason, SessionId};

use crate::dto::TimerTickDto;
use crate::state::AppState;

const DB_FILENAME: &str = "zen.db";
const TICK_EVENT: &str = "session://tick";
const COMPLETED_EVENT: &str = "session://completed";
const TAMPER_EVENT: &str = "session://tamper";

/// Open the database, run migrations, wire repositories, and assemble the
/// `SessionService`. Also performs crash recovery — if an active session is
/// present in the DB, re-applies its enforcement plan and primes the timer.
pub async fn bootstrap(data_dir: PathBuf) -> Result<AppState, String> {
    info!(?data_dir, "bootstrap: starting");

    let db_path = data_dir.join(DB_FILENAME);
    let pool = connection::connect_file(&db_path)
        .await
        .map_err(|e| format!("connect db: {e}"))?;

    migrations::run(&pool)
        .await
        .map_err(|e| format!("run migrations: {e}"))?;

    let session_repo = Arc::new(SqliteSessionRepository::new(pool.clone()));
    let profile_repo = Arc::new(SqliteProfileRepository::new(pool.clone()));
    let tamper_repo = Arc::new(SqliteTamperRepository::new(pool.clone()));

    let blocker = build_platform_blocker();
    let service = Arc::new(SessionService::new(
        session_repo.clone(),
        profile_repo.clone(),
        blocker,
    ));

    let timer = Arc::new(TimerEngine::new());

    // Crash recovery — must run before the UI shows so the user never sees a
    // brief "no active session" state when one is actually still enforced.
    match service.recover_active_session().await {
        Ok(Some(recovered)) => {
            info!(
                session_id = %recovered.session.id.0,
                remaining_secs = recovered.remaining_secs,
                clock_rollback = recovered.clock_rollback_suspected,
                "recovered active session"
            );
            timer
                .start(recovered.session.id.clone(), recovered.remaining_secs)
                .await;
        }
        Ok(None) => info!("no active session to recover"),
        Err(e) => warn!(error = %e, "recovery failed"),
    }

    Ok(AppState {
        service,
        timer,
        tamper: tamper_repo,
        pool,
    })
}

#[cfg(target_os = "macos")]
fn build_platform_blocker() -> Arc<dyn zen_core::ports::BlockingAdapter> {
    Arc::new(zen_macos::adapter::MacOsBlockingAdapter::new())
}

#[cfg(target_os = "windows")]
fn build_platform_blocker() -> Arc<dyn zen_core::ports::BlockingAdapter> {
    Arc::new(zen_windows::adapter::WindowsBlockingAdapter::new())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn build_platform_blocker() -> Arc<dyn zen_core::ports::BlockingAdapter> {
    Arc::new(NoopBlockingAdapter::new())
}

/// 1-Hz timer loop. Each tick is emitted to the frontend; on completion it
/// drives the session FSM to `Completed` and notifies the UI.
pub async fn run_timer_loop(
    timer: Arc<TimerEngine>,
    service: Arc<SessionService>,
    handle: AppHandle,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let Some(snapshot) = timer.tick().await else {
            continue;
        };

        let dto = snapshot_to_dto(&snapshot);
        if let Err(e) = handle.emit(TICK_EVENT, &dto) {
            warn!(error = %e, "emit tick failed");
        }

        // Update tray title with a short countdown — keeps progress visible
        // even when the main window is closed.
        update_tray_title(&handle, &snapshot);

        if snapshot.state == TimerState::Completed {
            let session_id = snapshot.session_id.clone();
            handle_completion(timer.clone(), service.clone(), handle.clone(), session_id).await;
        }
    }
}

async fn handle_completion(
    timer: Arc<TimerEngine>,
    service: Arc<SessionService>,
    handle: AppHandle,
    session_id: SessionId,
) {
    timer.stop().await;
    let id_str = session_id.0.to_string();
    match service.complete_session(&session_id).await {
        Ok(_) => {
            info!(session_id = %id_str, "session completed by timer");
            let _ = handle.emit(COMPLETED_EVENT, &id_str);
        }
        Err(e) => {
            warn!(error = %e, "complete_session failed; falling back to abort");
            let _ = service
                .stop_session(&session_id, AbortReason::TimerCorrupted)
                .await;
            let _ = handle.emit(TAMPER_EVENT, format!("complete failed: {e}"));
        }
    }
}

fn snapshot_to_dto(snapshot: &TimerSnapshot) -> TimerTickDto {
    TimerTickDto {
        session_id: snapshot.session_id.0.to_string(),
        remaining_secs: snapshot.remaining_secs,
        elapsed_secs: snapshot.elapsed_secs,
        planned_secs: snapshot.planned_secs,
        state: match snapshot.state {
            TimerState::Running => "Running",
            TimerState::Paused => "Paused",
            TimerState::Completed => "Completed",
            TimerState::Idle => "Idle",
        }
        .into(),
    }
}

fn update_tray_title(handle: &AppHandle, snapshot: &TimerSnapshot) {
    let mins = snapshot.remaining_secs / 60;
    let secs = snapshot.remaining_secs % 60;
    let title = format!("{:02}:{:02}", mins, secs);
    crate::tray::set_title(handle, &title);
}
