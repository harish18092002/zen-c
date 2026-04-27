mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::stop_session,
            commands::get_session_state,
            commands::list_profiles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
