use crate::state::AppState;
use tauri::State;
use tbb_core::diagnostics::{run_diagnostics, to_copyable_text, to_json_pretty, DiagnosticsReport};

#[tauri::command]
pub fn run_host_diagnostics(
    project_id: Option<String>,
    state: State<AppState>,
) -> Result<DiagnosticsReport, String> {
    let root = match project_id {
        Some(id) => {
            let projects = state
                .projects
                .lock()
                .map_err(|_| "internal state lock poisoned".to_string())?;
            projects.get(&id).map(|p| p.root_path.clone())
        }
        None => None,
    };
    Ok(run_diagnostics(root.as_deref()))
}

#[tauri::command]
pub fn diagnostics_as_text(report: DiagnosticsReport) -> String {
    to_copyable_text(&report)
}

#[tauri::command]
pub fn diagnostics_as_json(report: DiagnosticsReport) -> Result<String, String> {
    to_json_pretty(&report).map_err(|e| e.to_string())
}
