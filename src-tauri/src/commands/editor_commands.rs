use crate::state::AppState;
use tauri::State;
use tbb_core::editor::{
    delete_file, diff_texts, is_protected, list_tree, read_file, read_file_at_head, rename_file,
    validate_yaml, write_file, DiffLine, FileContent, FileTreeEntry, SaveError, SaveResult,
};

fn project_root(state: &State<AppState>, project_id: &str) -> Result<std::path::PathBuf, String> {
    let projects = state
        .projects
        .lock()
        .map_err(|_| "internal state lock poisoned".to_string())?;
    projects
        .get(project_id)
        .map(|p| p.root_path.clone())
        .ok_or_else(|| "unknown project id".to_string())
}

#[tauri::command]
pub fn list_file_tree(
    project_id: String,
    state: State<AppState>,
) -> Result<Vec<FileTreeEntry>, String> {
    let root = project_root(&state, &project_id)?;
    list_tree(&root, 20_000).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_project_file(
    project_id: String,
    rel_path: String,
    state: State<AppState>,
) -> Result<FileContent, String> {
    let root = project_root(&state, &project_id)?;
    read_file(&root, &rel_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_project_file(
    project_id: String,
    rel_path: String,
    content: String,
    expected_mtime_ms: Option<i64>,
    force: bool,
    state: State<AppState>,
) -> Result<SaveResult, String> {
    let root = project_root(&state, &project_id)?;
    write_file(&root, &rel_path, &content, expected_mtime_ms, force).map_err(|e| match e {
        SaveError::ExternalChange { .. } => format!("EXTERNAL_CHANGE: {e}"),
        SaveError::Other(inner) => inner.to_string(),
    })
}

#[tauri::command]
pub fn delete_project_file(
    project_id: String,
    rel_path: String,
    state: State<AppState>,
) -> Result<(), String> {
    let root = project_root(&state, &project_id)?;
    delete_file(&root, &rel_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_project_file(
    project_id: String,
    from_rel: String,
    to_rel: String,
    state: State<AppState>,
) -> Result<(), String> {
    let root = project_root(&state, &project_id)?;
    rename_file(&root, &from_rel, &to_rel).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn diff_project_file_against_disk(
    project_id: String,
    rel_path: String,
    proposed_content: String,
    state: State<AppState>,
) -> Result<Vec<DiffLine>, String> {
    let root = project_root(&state, &project_id)?;
    let on_disk = read_file(&root, &rel_path).map_err(|e| e.to_string())?;
    Ok(diff_texts(&on_disk.content, &proposed_content))
}

#[tauri::command]
pub fn diff_project_file_against_head(
    project_id: String,
    rel_path: String,
    state: State<AppState>,
) -> Result<Option<Vec<DiffLine>>, String> {
    let root = project_root(&state, &project_id)?;
    let head_content = read_file_at_head(&root, &rel_path).map_err(|e| e.to_string())?;
    let Some(head_content) = head_content else {
        return Ok(None);
    };
    let current = read_file(&root, &rel_path).map_err(|e| e.to_string())?;
    Ok(Some(diff_texts(&head_content, &current.content)))
}

#[tauri::command]
pub fn validate_yaml_content(content: String) -> Result<(), String> {
    validate_yaml(&content)
}

#[tauri::command]
pub fn is_protected_path(rel_path: String) -> bool {
    is_protected(&rel_path)
}
