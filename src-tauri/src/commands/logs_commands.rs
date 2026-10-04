use crate::state::AppState;
use tauri::State;
use tbb_core::logs::{
    apply_retention, delete_record, disk_usage, list_records, read_raw_log_tail, DiskUsage,
};
use tbb_core::models::RunRecord;
use tbb_core::settings::RetentionSettings;

#[tauri::command]
pub fn list_run_history(state: State<AppState>) -> Vec<RunRecord> {
    list_records(&state.logs_dir())
}

#[tauri::command]
pub fn get_log_tail(
    run_id: String,
    max_lines: usize,
    state: State<AppState>,
) -> Result<Vec<String>, String> {
    let path = state.logs_dir().join(format!("{run_id}.log"));
    read_raw_log_tail(&path, max_lines).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_full_log(run_id: String, state: State<AppState>) -> Result<String, String> {
    let path = state.logs_dir().join(format!("{run_id}.log"));
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_log_file_path(run_id: String, state: State<AppState>) -> String {
    state
        .logs_dir()
        .join(format!("{run_id}.log"))
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn delete_run_record(run_id: String, state: State<AppState>) -> Result<(), String> {
    delete_record(&state.logs_dir(), &run_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_logs_disk_usage(state: State<AppState>) -> DiskUsage {
    disk_usage(&state.logs_dir())
}

#[tauri::command]
pub async fn apply_log_retention(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let retention: RetentionSettings = {
        let settings = state.settings.lock().await;
        settings.retention.clone()
    };
    apply_retention(&state.logs_dir(), &retention).map_err(|e| e.to_string())
}

/// Copyable diagnostic bundle for a failed/completed run: the record plus
/// the final N lines of raw (already-redaction-safe-by-construction) log.
#[tauri::command]
pub fn build_diagnostic_bundle(run_id: String, state: State<AppState>) -> Result<String, String> {
    let logs_dir = state.logs_dir();
    let record = tbb_core::logs::load_record(&logs_dir, &run_id).map_err(|e| e.to_string())?;
    let tail = read_raw_log_tail(&logs_dir.join(format!("{run_id}.log")), 200).unwrap_or_default();

    let mut bundle = serde_json::json!({
        "record": record,
        "log_tail": tail,
    });
    if let Some(obj) = bundle.as_object_mut() {
        obj.insert(
            "note".to_string(),
            serde_json::Value::String(
                "Environment values here are already redacted for likely secrets. \
                 Working directory and command are shown exactly as executed."
                    .to_string(),
            ),
        );
    }
    serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())
}
