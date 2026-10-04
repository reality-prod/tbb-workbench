//! Domain models shared between core, the Tauri shell and (via serde) the
//! TypeScript frontend. Field changes are breaking for the UI.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedProject {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,
    pub opened_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteInfo {
    pub name: String,
    /// Credentials embedded in URLs are redacted before they reach the UI.
    pub url: String,
}

/// Everything here is either directly observed or explicitly empty.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryStatus {
    pub is_git_repo: bool,
    pub looks_like_tor_browser_build: bool,
    pub detected_markers: Vec<String>,
    pub remote_origin_url: Option<String>,
    pub remotes: Vec<RemoteInfo>,
    pub current_branch: Option<String>,
    pub head_detached: bool,
    pub current_commit: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub is_dirty: bool,
    pub changed_file_count: usize,
    pub changed_files: Vec<ChangedFile>,
    pub tags: Vec<String>,
    pub submodules: Vec<String>,
    pub last_fetch_ms: Option<i64>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: String,
    /// One of: modified, added, deleted, renamed, untracked, conflicted, typechange
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentOverride {
    pub key: String,
    pub value: String,
    pub likely_secret: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    #[default]
    Build,
    GitClone,
    GitFetch,
    GitCheckout,
    GitSubmodule,
    Custom,
}

/// A resolved, ready-to-spawn command: exactly what is previewed and run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInvocation {
    pub executable: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    #[serde(default)]
    pub env_overrides: Vec<EnvironmentOverride>,
    pub label: String,
    /// When true, `executable`/`args` describe a `sh -c` style invocation and
    /// the run is only permitted if the user enabled shell mode in settings.
    #[serde(default)]
    pub shell_mode: bool,
    #[serde(default)]
    pub expected_output_dirs: Vec<String>,
    #[serde(default)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub kind: RunKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildState {
    Idle,
    Preparing,
    CheckingPrerequisites,
    Fetching,
    Building,
    Packaging,
    Verifying,
    Running,
    Cancelling,
    Cancelled,
    Failed,
    Succeeded,
    InterruptedByAppExit,
}

impl BuildState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            BuildState::Cancelled
                | BuildState::Failed
                | BuildState::Succeeded
                | BuildState::InterruptedByAppExit
        )
    }

    /// Legal transitions of the run state machine. Terminal states never
    /// transition; `Cancelling` may only end in a terminal state.
    pub fn can_transition_to(self, next: BuildState) -> bool {
        use BuildState::*;
        if self.is_terminal() {
            return false;
        }
        if self == next {
            return true;
        }
        match self {
            Idle => true,
            Cancelling => next.is_terminal(),
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LineLevel {
    #[default]
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOutputEvent {
    pub run_id: String,
    pub seq: u64,
    pub stream: OutputStream,
    pub text: String,
    pub timestamp_ms: i64,
    pub level: LineLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhaseMark {
    pub name: String,
    pub at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub current: u32,
    pub total: u32,
    pub percent: f32,
    /// Always states where the number came from. Never inferred silently.
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunStats {
    pub warnings: u64,
    pub errors: u64,
    pub downloads: u64,
    pub lines_total: u64,
    pub current_phase: Option<String>,
    pub phases: Vec<PhaseMark>,
    pub progress: Option<ProgressInfo>,
    pub first_fatal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputBatch {
    pub run_id: String,
    pub events: Vec<BuildOutputEvent>,
    pub stats: RunStats,
    pub state: BuildState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationConfirmation {
    NotRequested,
    Confirmed,
    Unconfirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionEvent {
    pub run_id: String,
    pub state: BuildState,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub duration_ms: i64,
    pub cancel_requested: bool,
    pub termination: TerminationConfirmation,
    pub stats: RunStats,
    pub log_file: String,
}

/// Persisted (redacted) record of a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub kind: RunKind,
    pub project_id: String,
    pub project_name: String,
    pub project_root: Option<PathBuf>,
    pub label: String,
    pub preset_id: Option<String>,
    pub executable: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env_keys: Vec<String>,
    pub env_redacted: Vec<EnvironmentOverride>,
    pub shell_mode: bool,
    pub state: BuildState,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub cancel_requested: bool,
    pub termination: TerminationConfirmation,
    pub repo_branch: Option<String>,
    pub repo_commit: Option<String>,
    pub repo_dirty: bool,
    pub pid: Option<u32>,
    pub log_file: String,
    pub stats: RunStats,
    pub expected_output_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub path: String,
    pub size: u64,
    pub modified_ms: i64,
    pub kind: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Operational error presented to the user with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIssue {
    pub severity: Severity,
    pub code: String,
    pub summary: String,
    pub details: Option<String>,
    pub suggested_action: Option<String>,
    /// Blockers with `overridable = true` may be bypassed by the user after
    /// explicit acknowledgement.
    pub overridable: bool,
}

impl AppIssue {
    pub fn new(sev: Severity, code: &str, summary: impl Into<String>) -> Self {
        Self {
            severity: sev,
            code: code.to_string(),
            summary: summary.into(),
            details: None,
            suggested_action: None,
            overridable: false,
        }
    }
    pub fn details(mut self, d: impl Into<String>) -> Self {
        self.details = Some(d.into());
        self
    }
    pub fn action(mut self, a: impl Into<String>) -> Self {
        self.suggested_action = Some(a.into());
        self
    }
    pub fn overridable(mut self) -> Self {
        self.overridable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_do_not_transition() {
        assert!(!BuildState::Succeeded.can_transition_to(BuildState::Running));
        assert!(!BuildState::Failed.can_transition_to(BuildState::Building));
    }

    #[test]
    fn cancelling_only_ends_terminal() {
        assert!(BuildState::Cancelling.can_transition_to(BuildState::Cancelled));
        assert!(!BuildState::Cancelling.can_transition_to(BuildState::Building));
    }

    #[test]
    fn running_can_progress_through_phases() {
        assert!(BuildState::Running.can_transition_to(BuildState::Packaging));
        assert!(BuildState::Preparing.can_transition_to(BuildState::Running));
    }
}
