use crate::state::AppState;
use tauri::State;
use tbb_core::settings::{self, ApplicationSettings};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<ApplicationSettings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
pub async fn update_settings(
    new_settings: ApplicationSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    settings::save(&state.settings_path(), &new_settings).map_err(|e| e.to_string())?;
    let mut settings = state.settings.lock().await;
    *settings = new_settings;
    Ok(())
}
