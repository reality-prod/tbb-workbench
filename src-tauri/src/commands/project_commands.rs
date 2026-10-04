use crate::state::AppState;
use tauri::State;
use tbb_core::models::{ManagedProject, RepositoryStatus};
use tbb_core::repository::inspect_repository;
use tbb_core::settings;

#[tauri::command]
pub async fn open_project(
    path: String,
    state: State<'_, AppState>,
) -> Result<ManagedProject, String> {
    let root_path = std::path::PathBuf::from(&path);
    if !root_path.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {path}"));
    }
    let canonical = root_path.canonicalize().map_err(|e| e.to_string())?;

    let name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let project = ManagedProject {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        root_path: canonical.clone(),
        opened_at: chrono::Utc::now(),
    };

    state
        .projects
        .lock()
        .map_err(|_| "internal state lock poisoned".to_string())?
        .insert(project.id.clone(), project.clone());

    {
        let mut settings = state.settings.lock().await;
        settings::remember_project(&mut settings, canonical, name);
        let _ = settings::save(&state.settings_path(), &settings);
    }

    Ok(project)
}

#[tauri::command]
pub fn get_repository_status(
    project_id: String,
    state: State<AppState>,
) -> Result<RepositoryStatus, String> {
    let root_path = {
        let projects = state
            .projects
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        projects
            .get(&project_id)
            .map(|p| p.root_path.clone())
            .ok_or_else(|| "unknown project id".to_string())?
    };
    Ok(inspect_repository(&root_path))
}

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<ManagedProject>, String> {
    let projects = state
        .projects
        .lock()
        .map_err(|_| "internal state lock poisoned".to_string())?;
    Ok(projects.values().cloned().collect())
}

#[tauri::command]
pub fn get_project(project_id: String, state: State<AppState>) -> Result<ManagedProject, String> {
    let projects = state
        .projects
        .lock()
        .map_err(|_| "internal state lock poisoned".to_string())?;
    projects
        .get(&project_id)
        .cloned()
        .ok_or_else(|| "unknown project id".to_string())
}

#[tauri::command]
pub async fn list_recent_projects(
    state: State<'_, AppState>,
) -> Result<Vec<settings::RecentProject>, String> {
    let settings = state.settings.lock().await;
    Ok(settings.recent_projects.clone())
}
