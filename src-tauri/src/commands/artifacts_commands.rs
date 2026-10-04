use crate::state::AppState;
use tauri::State;
use tbb_core::artifacts::{scan_artifacts, ArtifactScanResult};

#[tauri::command]
pub fn scan_project_artifacts(
    project_id: String,
    expected_dirs: Vec<String>,
    compute_hashes: bool,
    state: State<AppState>,
) -> Result<ArtifactScanResult, String> {
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
    Ok(scan_artifacts(&root, &expected_dirs, compute_hashes))
}
