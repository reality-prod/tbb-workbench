use crate::state::{ActiveRun, AppState};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tbb_core::build::{spawn_run, RunSupervisorConfig};
use tbb_core::models::{
    BuildInvocation, BuildState, CompletionEvent, EnvironmentOverride, RunRecord,
};
use tbb_core::security::redact::{key_looks_secret, redact_env, valid_env_key};

/// Resolves the exact argv for an invocation and returns it for preview
/// WITHOUT executing anything — the same struct `run_invocation` later
/// executes. Applies honest `likely_secret` flagging based on key name.
#[tauri::command]
pub fn preview_build_command(invocation: BuildInvocation) -> BuildInvocation {
    let mut inv = invocation;
    for ov in &mut inv.env_overrides {
        if !ov.likely_secret {
            ov.likely_secret = key_looks_secret(&ov.key);
        }
    }
    inv
}

fn validate_invocation(inv: &BuildInvocation) -> Result<(), String> {
    if inv.executable.trim().is_empty() {
        return Err("executable must not be empty".to_string());
    }
    for ov in &inv.env_overrides {
        if !valid_env_key(&ov.key) {
            return Err(format!("invalid environment variable name: '{}'", ov.key));
        }
    }
    if !inv.working_dir.exists() {
        return Err(format!(
            "working directory does not exist: {}",
            inv.working_dir.display()
        ));
    }
    Ok(())
}

/// Executes any invocation (build preset/custom command, git clone/fetch/
/// checkout/submodule update, diff generation) through the same real
/// subprocess supervisor. Enforces: single active run per project, and that
/// shell_mode invocations are only accepted when the user has explicitly
/// enabled shell command mode in Settings.
#[tauri::command]
pub async fn run_invocation(
    app: AppHandle,
    project_id: String,
    invocation: BuildInvocation,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_invocation(&invocation)?;

    if invocation.shell_mode {
        let settings = state.settings.lock().await;
        if !settings.safety.allow_shell_command_mode {
            return Err(
                "shell command mode is disabled in Settings > Safety; enable it explicitly first"
                    .to_string(),
            );
        }
    }

    {
        let active = state
            .active_runs
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        if active.contains_key(&project_id) {
            return Err(
                "a run is already active for this project; cancel it first or wait for it to finish"
                    .to_string(),
            );
        }
    }

    let (project_name, project_root, repo_branch, repo_commit, repo_dirty) = {
        let projects = state
            .projects
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        match projects.get(&project_id) {
            Some(p) => {
                let status = tbb_core::repository::inspect_repository(&p.root_path);
                (
                    p.name.clone(),
                    Some(p.root_path.clone()),
                    status.current_branch,
                    status.current_commit,
                    status.is_dirty,
                )
            }
            None => (project_id.clone(), None, None, None, false),
        }
    };

    let run_id = uuid::Uuid::new_v4().to_string();
    let log_dir = state.logs_dir();
    let env: Vec<(String, String)> = invocation
        .env_overrides
        .iter()
        .map(|o| (o.key.clone(), o.value.clone()))
        .collect();

    let mut spawned = spawn_run(
        run_id.clone(),
        &invocation.executable,
        &invocation.args,
        &invocation.working_dir,
        &env,
        &log_dir,
        RunSupervisorConfig::default(),
    )
    .await
    .map_err(|e| format!("failed to start run: {e}"))?;

    {
        let mut active = state
            .active_runs
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        active.insert(
            project_id.clone(),
            ActiveRun {
                handle: spawned.handle.clone(),
                run_id: run_id.clone(),
            },
        );
    }

    let started_at_ms = tbb_core::util::time::now_ms();
    let pid = spawned.handle.pid();

    let app_for_output = app.clone();
    let run_id_for_output = run_id.clone();
    tokio::spawn(async move {
        while let Some(batch) = spawned.output_rx.recv().await {
            let _ = app_for_output.emit(&format!("run-output://{run_id_for_output}"), &batch);
        }
    });

    let app_for_completion = app.clone();
    let project_id_for_completion = project_id.clone();
    let run_id_for_completion = run_id.clone();
    let invocation_for_record = invocation.clone();
    let log_dir_for_completion = log_dir.clone();
    let log_file_path = spawned.log_file_path.clone();

    tauri::async_runtime::spawn(async move {
        if let Ok(info) = spawned.completion_rx.await {
            let finished_at_ms = tbb_core::util::time::now_ms();
            let redacted_env: Vec<EnvironmentOverride> =
                redact_env(&invocation_for_record.env_overrides, false);
            let env_keys: Vec<String> = invocation_for_record
                .env_overrides
                .iter()
                .map(|e| e.key.clone())
                .collect();

            let record = RunRecord {
                id: run_id_for_completion.clone(),
                kind: invocation_for_record.kind,
                project_id: project_id_for_completion.clone(),
                project_name: project_name.clone(),
                project_root: project_root.clone(),
                label: invocation_for_record.label.clone(),
                preset_id: invocation_for_record.preset_id.clone(),
                executable: invocation_for_record.executable.clone(),
                args: invocation_for_record.args.clone(),
                working_dir: invocation_for_record.working_dir.clone(),
                env_keys,
                env_redacted: redacted_env,
                shell_mode: invocation_for_record.shell_mode,
                state: info.state,
                started_at_ms,
                finished_at_ms: Some(finished_at_ms),
                duration_ms: Some(finished_at_ms - started_at_ms),
                exit_code: info.exit_code,
                signal: info.signal,
                cancel_requested: info.cancel_requested,
                termination: info.termination,
                repo_branch,
                repo_commit,
                repo_dirty,
                pid,
                log_file: log_file_path.to_string_lossy().to_string(),
                stats: info.stats.clone(),
                expected_output_dirs: invocation_for_record.expected_output_dirs.clone(),
            };
            let _ = tbb_core::logs::save_record(&log_dir_for_completion, &record);

            let completion = CompletionEvent {
                run_id: run_id_for_completion.clone(),
                state: info.state,
                exit_code: info.exit_code,
                signal: info.signal,
                duration_ms: finished_at_ms - started_at_ms,
                cancel_requested: info.cancel_requested,
                termination: info.termination,
                stats: info.stats,
                log_file: record.log_file.clone(),
            };
            let _ = app_for_completion.emit(
                &format!("run-complete://{run_id_for_completion}"),
                &completion,
            );

            if let Some(app_state) = app_for_completion.try_state::<AppState>() {
                if let Ok(mut active) = app_state.active_runs.lock() {
                    active.remove(&project_id_for_completion);
                }
            }
        }
    });

    Ok(run_id)
}

/// Sends a graceful interrupt (SIGTERM on Unix). The completion event later
/// reports whether the process actually exited (`termination: confirmed`)
/// or not — this command only reports that the *request* was sent.
#[tauri::command]
pub async fn cancel_run(project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let handle = {
        let active = state
            .active_runs
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        active.get(&project_id).map(|r| r.handle.clone())
    };
    match handle {
        Some(h) => h
            .request_cancel()
            .await
            .map_err(|e| format!("failed to request cancellation: {e}")),
        None => Err("no active run for this project".to_string()),
    }
}

#[tauri::command]
pub async fn force_kill_run(project_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let handle: Option<Arc<tbb_core::build::RunHandle>> = {
        let active = state
            .active_runs
            .lock()
            .map_err(|_| "internal state lock poisoned".to_string())?;
        active.get(&project_id).map(|r| r.handle.clone())
    };
    match handle {
        Some(h) => h
            .force_kill()
            .await
            .map_err(|e| format!("failed to force-kill: {e}")),
        None => Err("no active run for this project".to_string()),
    }
}

#[tauri::command]
pub fn has_active_run(project_id: String, state: State<AppState>) -> Result<bool, String> {
    let active = state
        .active_runs
        .lock()
        .map_err(|_| "internal state lock poisoned".to_string())?;
    Ok(active.contains_key(&project_id))
}

/// Emits a synthetic completion for any run that never got to finish
/// because the app was closed mid-run, and records it as such. Called once
/// at startup for any run that was active in a previous session — detected
/// via presence of a `.log` file with no matching `.record.json`, which
/// `run_invocation` always writes on a normal completion path.
#[tauri::command]
pub fn reconcile_interrupted_runs(state: State<AppState>) -> Result<Vec<String>, String> {
    let log_dir = state.logs_dir();
    let mut reconciled = Vec::new();
    let Ok(entries) = std::fs::read_dir(&log_dir) else {
        return Ok(reconciled);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".log") {
            continue;
        }
        let run_id = name.trim_end_matches(".log");
        let record_path = log_dir.join(format!("{run_id}.record.json"));
        if !record_path.exists() {
            // A raw log with no completion record means the app exited (or
            // crashed) mid-run in a previous session. We can't fabricate an
            // exit code we never observed, so we record this honestly as
            // BuildState::InterruptedByAppExit with everything else unknown.
            let record = RunRecord {
                id: run_id.to_string(),
                kind: tbb_core::models::RunKind::Custom,
                project_id: "unknown".to_string(),
                project_name: "unknown (from previous session)".to_string(),
                project_root: None,
                label: "Interrupted run recovered on startup".to_string(),
                preset_id: None,
                executable: "unknown".to_string(),
                args: vec![],
                working_dir: std::path::PathBuf::from("."),
                env_keys: vec![],
                env_redacted: vec![],
                shell_mode: false,
                state: BuildState::InterruptedByAppExit,
                started_at_ms: 0,
                finished_at_ms: None,
                duration_ms: None,
                exit_code: None,
                signal: None,
                cancel_requested: false,
                termination: tbb_core::models::TerminationConfirmation::Unconfirmed,
                repo_branch: None,
                repo_commit: None,
                repo_dirty: false,
                pid: None,
                log_file: path.to_string_lossy().to_string(),
                stats: Default::default(),
                expected_output_dirs: vec![],
            };
            if tbb_core::logs::save_record(&log_dir, &record).is_ok() {
                reconciled.push(run_id.to_string());
            }
        }
    }
    Ok(reconciled)
}
