use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "zen-mode-tray";

/// Install the menu-bar / system-tray icon. The icon is loaded from the
/// bundle resources; menu items hand off to the main window or invoke
/// well-defined commands. This must be called once during `setup`.
pub fn install_tray(handle: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(handle, "open", "Open Zen Mode", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(handle, "pause", "Pause Session", true, None::<&str>)?;
    let resume_item = MenuItem::with_id(handle, "resume", "Resume Session", true, None::<&str>)?;
    let end_item = MenuItem::with_id(handle, "end", "End Session…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        handle,
        &[&open_item, &pause_item, &resume_item, &end_item, &quit_item],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(handle.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("no default window icon configured".into())
        })?)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "pause" => spawn_command(app, "pause_session"),
            "resume" => spawn_command(app, "resume_session"),
            "end" => spawn_command(app, "stop_session"),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(handle)?;

    Ok(())
}

fn show_main_window(handle: &AppHandle) {
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
    }
}

/// The tray menu items invoke the corresponding session command if a session
/// is running. We resolve the session_id from the current timer state and
/// dispatch through the Tauri runtime so the command runs with proper state.
fn spawn_command(handle: &AppHandle, command: &str) {
    let handle = handle.clone();
    let command = command.to_string();
    tauri::async_runtime::spawn(async move {
        use crate::state::AppState;
        let state = handle.state::<AppState>();
        let Some(session_id) = state.timer.current_session().await else {
            tracing::debug!("tray: no active session to act on");
            return;
        };
        let id_str = session_id.0.to_string();
        let result = match command.as_str() {
            "pause_session" => {
                let remaining = state.timer.pause().await;
                if let Some(rem) = remaining {
                    state
                        .service
                        .pause_session(&session_id, rem)
                        .await
                        .map(|_| ())
                } else {
                    Ok(())
                }
            }
            "resume_session" => state.service.resume_session(&session_id).await.map(|_| {
                let timer = state.timer.clone();
                tauri::async_runtime::spawn(async move {
                    timer.resume().await;
                });
            }),
            "stop_session" => state
                .service
                .stop_session(
                    &session_id,
                    zen_domain::entities::AbortReason::UserRequested,
                )
                .await
                .map(|_| {
                    let timer = state.timer.clone();
                    tauri::async_runtime::spawn(async move {
                        timer.stop().await;
                    });
                }),
            _ => Ok(()),
        };

        if let Err(e) = result {
            tracing::warn!(error = %e, command = %command, session_id = %id_str, "tray command failed");
        }
    });
}

/// Update the tray title to the given string. Called every tick from the
/// timer loop. macOS uses the title text directly; on Windows it appears in
/// the tooltip.
pub fn set_title(handle: &AppHandle, title: &str) {
    if let Some(tray) = handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(title));
    }
}
