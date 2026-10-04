pub mod commands;
pub mod state;

use state::AppState;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;
use tbb_core::settings;
use tokio::sync::Mutex as AsyncMutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .expect("could not resolve app local data directory");

            std::fs::create_dir_all(&app_data_dir).ok();

            let settings_path = app_data_dir.join("settings.json");
            let loaded_settings = settings::load(&settings_path).unwrap_or_default();

            app.manage(AppState {
                projects: Mutex::new(HashMap::new()),
                active_runs: Mutex::new(HashMap::new()),
                settings: AsyncMutex::new(loaded_settings),
                app_data_dir,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // -----------------------------------------------------------------
            // Project
            // -----------------------------------------------------------------
            commands::project_commands::open_project,
            commands::project_commands::get_repository_status,
            commands::project_commands::list_projects,
            commands::project_commands::get_project,
            commands::project_commands::list_recent_projects,
            // -----------------------------------------------------------------
            // Run execution
            // -----------------------------------------------------------------
            commands::run_commands::preview_build_command,
            commands::run_commands::run_invocation,
            commands::run_commands::cancel_run,
            commands::run_commands::force_kill_run,
            commands::run_commands::has_active_run,
            commands::run_commands::reconcile_interrupted_runs,
            // -----------------------------------------------------------------
            // Git
            // -----------------------------------------------------------------
            commands::git_commands::validate_clone_destination,
            commands::git_commands::preview_clone_invocation,
            commands::git_commands::preview_checkout_invocation,
            commands::git_commands::preview_fetch_invocation,
            commands::git_commands::preview_submodule_update_invocation,
            commands::git_commands::preview_diff_patch_invocation,
            // -----------------------------------------------------------------
            // Editor
            // -----------------------------------------------------------------
            commands::editor_commands::list_file_tree,
            commands::editor_commands::read_project_file,
            commands::editor_commands::write_project_file,
            commands::editor_commands::delete_project_file,
            commands::editor_commands::rename_project_file,
            commands::editor_commands::diff_project_file_against_disk,
            commands::editor_commands::diff_project_file_against_head,
            commands::editor_commands::validate_yaml_content,
            commands::editor_commands::is_protected_path,
            // -----------------------------------------------------------------
            // RBM
            // -----------------------------------------------------------------
            commands::rbm_commands::inspect_rbm,
            // -----------------------------------------------------------------
            // Discovery
            // -----------------------------------------------------------------
            commands::discovery_commands::discover_supported_commands,
            // -----------------------------------------------------------------
            // Diagnostics
            // -----------------------------------------------------------------
            commands::diagnostics_commands::run_host_diagnostics,
            commands::diagnostics_commands::diagnostics_as_text,
            commands::diagnostics_commands::diagnostics_as_json,
            // -----------------------------------------------------------------
            // Logs
            // -----------------------------------------------------------------
            commands::logs_commands::list_run_history,
            commands::logs_commands::get_log_tail,
            commands::logs_commands::get_full_log,
            commands::logs_commands::get_log_file_path,
            commands::logs_commands::delete_run_record,
            commands::logs_commands::get_logs_disk_usage,
            commands::logs_commands::apply_log_retention,
            commands::logs_commands::build_diagnostic_bundle,
            // -----------------------------------------------------------------
            // Settings
            // -----------------------------------------------------------------
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            // -----------------------------------------------------------------
            // Preflight
            // -----------------------------------------------------------------
            commands::preflight_commands::get_preflight_report,
            commands::preflight_commands::get_perl_preflight,
            commands::preflight_commands::install_cpanm,
            commands::preflight_commands::install_rbm_perl_modules,
            // -----------------------------------------------------------------
            // Artifacts
            // -----------------------------------------------------------------
            commands::artifacts_commands::scan_project_artifacts,
            // -----------------------------------------------------------------
            // System integration
            // -----------------------------------------------------------------
            commands::system_commands::reveal_path_in_file_manager,
            commands::system_commands::open_terminal_here,
            commands::system_commands::run_diagnostic_tool_checks,
            commands::system_commands::save_diagnostics_json,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tbb-workbench");
}
