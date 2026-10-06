mod commands;
mod dto;
mod permissions;
mod setup;
mod state;
mod tray;

use std::sync::Arc;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // tracing initialization is best-effort — if the user has already
    // installed a global subscriber we don't want to panic.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // Resolve the app data dir on this thread (it doesn't need async)
            // and then do the rest of the boot inside a blocking-on-async
            // call so the rest of `setup` stays synchronous as Tauri expects.
            let data_dir = handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("resolve app_data_dir: {e}"))?;

            let state = tauri::async_runtime::block_on(setup::bootstrap(data_dir))?;
            let timer = Arc::clone(&state.timer);
            let service = Arc::clone(&state.service);

            // Spawn the timer loop. It emits a `session://tick` event every
            // second whenever a session is running.
            let handle_clone = handle.clone();
            let timer_for_loop = timer.clone();
            let service_for_loop = service.clone();
            tauri::async_runtime::spawn(setup::run_timer_loop(
                timer_for_loop,
                service_for_loop,
                handle_clone,
            ));

            // Build the tray icon. Errors here are non-fatal — the app still
            // works without a tray; we just log and move on.
            if let Err(e) = tray::install_tray(&handle) {
                tracing::warn!(error = %e, "tray install failed");
            }

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::stop_session,
            commands::pause_session,
            commands::resume_session,
            commands::get_session_state,
            commands::get_active_session,
            commands::list_profiles,
            commands::get_profile,
            commands::upsert_profile,
            commands::delete_profile,
            commands::probe_permissions,
            commands::list_recent_tamper,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
