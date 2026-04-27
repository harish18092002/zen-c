use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct StartSessionPayload {
    pub profile_id: String,
    pub duration_secs: u64,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub state: String,
}

#[tauri::command]
pub async fn start_session(payload: StartSessionPayload) -> Result<SessionResponse, String> {
    // TODO: Wire to SessionService with real DB and blocking adapter
    tracing::info!("start_session: {:?}", payload);
    Ok(SessionResponse {
        session_id: uuid::Uuid::new_v4().to_string(),
        state: "Active".to_string(),
    })
}

#[tauri::command]
pub async fn stop_session(session_id: String) -> Result<(), String> {
    tracing::info!("stop_session: {}", session_id);
    Ok(())
}

#[tauri::command]
pub async fn get_session_state(session_id: String) -> Result<SessionResponse, String> {
    Ok(SessionResponse {
        session_id,
        state: "Idle".to_string(),
    })
}

#[tauri::command]
pub async fn list_profiles() -> Result<Vec<serde_json::Value>, String> {
    // TODO: Query SqliteProfileRepository
    Ok(vec![serde_json::json!({
        "id": "default",
        "name": "Default Focus",
        "revision": 1,
        "blockRules": []
    })])
}
