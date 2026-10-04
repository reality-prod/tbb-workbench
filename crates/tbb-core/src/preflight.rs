//! Builds the preflight summary shown before every build: repo state,
//! resolved command, disk-space risk, and blocking/non-blocking issues.
//! This module only reads state and computes a report; it never executes
//! anything and never decides on its own to proceed.

use crate::models::{AppIssue, BuildInvocation, RepositoryStatus, Severity};
use crate::repository::inspect_repository;
use crate::system::disk_info_for;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub repository: RepositoryStatus,
    pub resolved_command: String,
    pub working_dir: String,
    pub available_disk_bytes: Option<u64>,
    /// A conservative, clearly-labeled-as-rough threshold check, not a
    /// prediction of exact space the build will need (which this app has
    /// no way to know without RBM/the project's own accounting).
    pub low_disk_space_warning: bool,
    pub issues: Vec<AppIssue>,
    pub requires_dirty_confirmation: bool,
    pub blocked: bool,
}

/// Below this we warn; this is deliberately conservative and explicitly
/// not a promise the build will fit above it either.
const LOW_DISK_THRESHOLD_BYTES: u64 = 20 * 1024 * 1024 * 1024; // 20 GiB

pub fn build_preflight(
    project_root: &Path,
    invocation: &BuildInvocation,
    shell_mode_allowed: bool,
) -> PreflightReport {
    let repository = inspect_repository(project_root);
    let disk = disk_info_for(project_root);
    let mut issues = Vec::new();

    if !repository.is_git_repo {
        issues.push(
            AppIssue::new(
                Severity::Error,
                "not-a-git-checkout",
                "This directory is not a Git checkout",
            )
            .details(repository.validation_errors.join("; "))
            .action("Open a valid tor-browser-build checkout, or clone one first."),
        );
    }

    if invocation.shell_mode && !shell_mode_allowed {
        issues.push(
            AppIssue::new(
                Severity::Error,
                "shell-mode-disabled",
                "Shell command mode is disabled in Settings",
            )
            .action("Enable 'allow shell command mode' in Settings > Safety, or use argv mode."),
        );
    }

    let low_disk_space_warning = disk
        .as_ref()
        .map(|d| d.available_bytes < LOW_DISK_THRESHOLD_BYTES)
        .unwrap_or(false);
    if low_disk_space_warning {
        issues.push(
            AppIssue::new(
                Severity::Warning,
                "low-disk-space",
                "Available disk space looks low for a full build",
            )
            .details(format!(
                "{} bytes free at {}",
                disk.as_ref().map(|d| d.available_bytes).unwrap_or(0),
                disk.as_ref()
                    .map(|d| d.mount_point.clone())
                    .unwrap_or_default()
            ))
            .overridable(),
        );
    }

    if !project_root.exists() {
        issues.push(AppIssue::new(
            Severity::Error,
            "missing-root",
            "Project root no longer exists on disk",
        ));
    }

    let requires_dirty_confirmation = repository.is_dirty;
    let blocked = issues
        .iter()
        .any(|i| i.severity == Severity::Error && !i.overridable);

    PreflightReport {
        resolved_command: crate::util::command_display::display_command(
            &invocation.executable,
            &invocation.args,
        ),
        working_dir: invocation.working_dir.to_string_lossy().to_string(),
        available_disk_bytes: disk.map(|d| d.available_bytes),
        low_disk_space_warning,
        repository,
        issues,
        requires_dirty_confirmation,
        blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvironmentOverride, RunKind};

    fn invocation(root: &Path) -> BuildInvocation {
        BuildInvocation {
            executable: "make".into(),
            args: vec!["all".into()],
            working_dir: root.to_path_buf(),
            env_overrides: Vec::<EnvironmentOverride>::new(),
            label: "test".into(),
            shell_mode: false,
            expected_output_dirs: vec![],
            preset_id: None,
            kind: RunKind::Build,
        }
    }

    #[test]
    fn blocks_on_non_git_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let report = build_preflight(tmp.path(), &invocation(tmp.path()), false);
        assert!(report.blocked);
        assert!(report.issues.iter().any(|i| i.code == "not-a-git-checkout"));
    }

    #[test]
    fn flags_dirty_checkout_for_confirmation_without_blocking() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("projects")).unwrap();
        std::fs::write(tmp.path().join("rbm.conf"), "").unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n").unwrap();
        let out = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        std::process::Command::new("git")
            .args(["config", "user.email", "[email protected]"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n\t@echo changed\n").unwrap();

        let report = build_preflight(tmp.path(), &invocation(tmp.path()), false);
        assert!(!report.blocked);
        assert!(report.requires_dirty_confirmation);
    }

    #[test]
    fn blocks_shell_mode_when_disallowed() {
        let tmp = tempfile::tempdir().unwrap();
        let mut inv = invocation(tmp.path());
        inv.shell_mode = true;
        let report = build_preflight(tmp.path(), &inv, false);
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "shell-mode-disabled"));
    }
}
