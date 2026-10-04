use crate::state::AppState;
use tauri::State;
use tbb_core::discovery::{discover, DiscoveryReport};

#[tauri::command]
pub fn discover_supported_commands(
    project_id: String,
    state: State<AppState>,
) -> Result<DiscoveryReport, String> {
    let root = {
        let projects = state
            .projects
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        projects
            .get(&project_id)
            .map(|p| p.root_path.clone())
            .ok_or_else(|| "unknown project id".to_string())?
    };
    discover(&root).map_err(|e| e.to_string())
}
