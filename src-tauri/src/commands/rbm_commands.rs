use crate::state::AppState;
use tauri::State;
use tbb_core::rbm::{inspect_rbm_file, RbmInspection};

#[tauri::command]
pub fn inspect_rbm(
    project_id: String,
    rel_path: String,
    state: State<AppState>,
) -> Result<RbmInspection, String> {
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
    inspect_rbm_file(&root, &rel_path).map_err(|e| e.to_string())
}
