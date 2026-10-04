use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tbb_core::build::RunHandle;
use tbb_core::models::ManagedProject;
use tbb_core::settings::ApplicationSettings;
use tokio::sync::Mutex as AsyncMutex;

pub struct ActiveRun {
    pub handle: Arc<RunHandle>,
    pub run_id: String,
}

/// In-memory + on-disk-backed application state.
///
/// `projects` and `active_runs` are in-memory only, by design: an active
/// run cannot meaningfully survive an app restart (see
/// `BuildState::InterruptedByAppExit`), and opened projects are just a
/// convenience the user re-opens (or picks from `settings.recent_projects`,
/// which IS persisted).
pub struct AppState {
    pub projects: std::sync::Mutex<HashMap<String, ManagedProject>>,
    /// project_id -> active run. Presence of an entry enforces "no
    /// simultaneous builds in the same repository by default".
    pub active_runs: std::sync::Mutex<HashMap<String, ActiveRun>>,
    pub settings: AsyncMutex<ApplicationSettings>,
    pub app_data_dir: PathBuf,
}

impl AppState {
    pub fn logs_dir(&self) -> PathBuf {
        self.app_data_dir.join("logs")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.app_data_dir.join("settings.json")
    }
}
