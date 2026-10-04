//! Git operations that mutate state (clone, fetch) use the system `git`
//! binary so behaviour is transparent and matches the user's own shell.
//! Every operation here goes through the same subprocess runner as builds,
//! so it gets the same live-streaming, cancellation, and raw-log guarantees.
//! Nothing in this module runs automatically; every call is user-triggered.

use crate::models::{BuildInvocation, EnvironmentOverride, RunKind};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    DefaultBranch,
    Branch,
    Tag,
    Commit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CloneRequest {
    pub source_url: String,
    pub destination: PathBuf,
    pub shallow: bool,
    /// If set, checked out after clone via a second, explicit invocation
    /// rather than folded silently into the clone command.
    pub git_ref: Option<String>,
    pub ref_kind: Option<RefKind>,
}

/// Builds the exact `git clone` invocation the app will run — shown to the
/// user before anything executes.
pub fn build_clone_invocation(req: &CloneRequest) -> BuildInvocation {
    let mut args = vec!["clone".to_string(), "--progress".to_string()];
    if req.shallow {
        args.push("--depth".to_string());
        args.push("1".to_string());
        // Shallow clones only fetch the tip of the default branch by
        // default; if the user picked shallow AND a specific ref we still
        // pass --branch so git resolves it directly during clone when
        // possible (falls back to full history only if that ref isn't a
        // branch/tag at the remote tip, in which case the follow-up
        // checkout step below will surface a clear failure instead of
        // silently succeeding on the wrong ref).
        if let (Some(r), Some(RefKind::Branch | RefKind::Tag)) = (&req.git_ref, &req.ref_kind) {
            args.push("--branch".to_string());
            args.push(r.clone());
        }
    }
    args.push(req.source_url.clone());
    args.push(req.destination.to_string_lossy().to_string());

    let parent = req
        .destination
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    BuildInvocation {
        executable: "git".to_string(),
        args,
        working_dir: parent,
        env_overrides: vec![EnvironmentOverride {
            key: "GIT_TERMINAL_PROMPT".to_string(),
            value: "0".to_string(),
            likely_secret: false,
        }],
        label: format!("Clone {}", req.source_url),
        shell_mode: false,
        expected_output_dirs: vec![req.destination.to_string_lossy().to_string()],
        preset_id: None,
        kind: RunKind::GitClone,
    }
}

/// Explicit follow-up checkout after a non-shallow clone with a specific
/// ref, or any ref that couldn't be resolved during `git clone --branch`.
pub fn build_checkout_invocation(repo_root: &Path, git_ref: &str) -> BuildInvocation {
    BuildInvocation {
        executable: "git".to_string(),
        args: vec!["checkout".to_string(), git_ref.to_string()],
        working_dir: repo_root.to_path_buf(),
        env_overrides: vec![],
        label: format!("Checkout {git_ref}"),
        shell_mode: false,
        expected_output_dirs: vec![],
        preset_id: None,
        kind: RunKind::GitCheckout,
    }
}

pub fn build_fetch_invocation(repo_root: &Path, remote: &str) -> BuildInvocation {
    BuildInvocation {
        executable: "git".to_string(),
        args: vec![
            "fetch".to_string(),
            "--progress".to_string(),
            remote.to_string(),
        ],
        working_dir: repo_root.to_path_buf(),
        env_overrides: vec![EnvironmentOverride {
            key: "GIT_TERMINAL_PROMPT".to_string(),
            value: "0".to_string(),
            likely_secret: false,
        }],
        label: format!("Fetch {remote}"),
        shell_mode: false,
        expected_output_dirs: vec![],
        preset_id: None,
        kind: RunKind::GitFetch,
    }
}

pub fn build_submodule_update_invocation(repo_root: &Path) -> BuildInvocation {
    BuildInvocation {
        executable: "git".to_string(),
        args: vec![
            "submodule".to_string(),
            "update".to_string(),
            "--init".to_string(),
            "--recursive".to_string(),
            "--progress".to_string(),
        ],
        working_dir: repo_root.to_path_buf(),
        env_overrides: vec![],
        label: "Update submodules".to_string(),
        shell_mode: false,
        expected_output_dirs: vec![],
        preset_id: None,
        kind: RunKind::GitSubmodule,
    }
}

/// Creates a unified diff patch of the working tree via `git diff`, used by
/// "Create Git patch from changes". Read-only from the repo's perspective.
pub fn build_diff_patch_invocation(repo_root: &Path, staged_too: bool) -> BuildInvocation {
    let mut args = vec!["diff".to_string()];
    if staged_too {
        args.push("HEAD".to_string());
    }
    BuildInvocation {
        executable: "git".to_string(),
        args,
        working_dir: repo_root.to_path_buf(),
        env_overrides: vec![],
        label: "Generate diff patch".to_string(),
        shell_mode: false,
        expected_output_dirs: vec![],
        preset_id: None,
        kind: RunKind::Custom,
    }
}

pub fn validate_destination(destination: &Path) -> Result<(), String> {
    if destination.exists() {
        let is_empty = destination
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            return Err(format!(
                "destination '{}' already exists and is not empty",
                destination.display()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_invocation_is_argv_only_and_reproducible() {
        let req = CloneRequest {
            source_url: "https://example.org/tor-browser-build.git".into(),
            destination: PathBuf::from("/home/u/code/tbb"),
            shallow: true,
            git_ref: Some("main".into()),
            ref_kind: Some(RefKind::Branch),
        };
        let inv = build_clone_invocation(&req);
        assert_eq!(inv.executable, "git");
        assert!(inv.args.contains(&"--depth".to_string()));
        assert!(inv.args.contains(&"--branch".to_string()));
        assert!(!inv.shell_mode);
        assert_eq!(inv.working_dir, PathBuf::from("/home/u/code"));
    }

    #[test]
    fn destination_must_be_empty_or_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("checkout");
        assert!(validate_destination(&dest).is_ok());
        std::fs::create_dir_all(&dest).unwrap();
        assert!(
            validate_destination(&dest).is_ok(),
            "empty existing dir is fine"
        );
        std::fs::write(dest.join("file"), "x").unwrap();
        assert!(validate_destination(&dest).is_err());
    }
}
