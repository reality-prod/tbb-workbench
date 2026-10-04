//! Detects whether a directory looks like a tor-browser-build style
//! checkout, and reads real git state via `git2` (read-only).
//!
//! Detection is signal-based: markers are individually optional evidence,
//! never a hardcoded required layout.

use crate::models::{ChangedFile, RemoteInfo, RepositoryStatus};
use crate::security::redact::redact_credential_urls;
use git2::{Repository, StatusOptions};
use std::path::Path;

const CANDIDATE_MARKERS: &[&str] = &[
    "rbm.conf",
    "rbm",
    "projects",
    "config",
    "conf",
    "tools",
    "keyring",
    "container",
    "Makefile",
];

fn status_label(s: git2::Status) -> (&'static str, bool) {
    use git2::Status as St;
    if s.intersects(St::CONFLICTED) {
        ("conflicted", false)
    } else if s.intersects(St::WT_NEW) || s.intersects(St::INDEX_NEW) {
        (
            "untracked",
            s.intersects(St::INDEX_NEW) && !s.intersects(St::WT_NEW),
        )
    } else if s.intersects(St::WT_DELETED) || s.intersects(St::INDEX_DELETED) {
        ("deleted", s.intersects(St::INDEX_DELETED))
    } else if s.intersects(St::WT_RENAMED) || s.intersects(St::INDEX_RENAMED) {
        ("renamed", s.intersects(St::INDEX_RENAMED))
    } else if s.intersects(St::WT_TYPECHANGE) || s.intersects(St::INDEX_TYPECHANGE) {
        ("typechange", s.intersects(St::INDEX_TYPECHANGE))
    } else {
        ("modified", s.intersects(St::INDEX_MODIFIED))
    }
}

pub fn inspect_repository(root: &Path) -> RepositoryStatus {
    let mut errors = Vec::new();
    let mut detected_markers = Vec::new();

    for marker in CANDIDATE_MARKERS {
        if root.join(marker).exists() {
            detected_markers.push((*marker).to_string());
        }
    }

    let looks_like_tor_browser_build = detected_markers
        .iter()
        .any(|m| m == "rbm.conf" || m == "rbm")
        && detected_markers.iter().any(|m| m == "projects")
        && detected_markers.iter().any(|m| m == "Makefile");

    let repo = match Repository::open(root) {
        Ok(repo) => Some(repo),
        Err(e) => {
            if root.join(".git").exists() {
                errors.push(format!("found .git but could not open repository: {e}"));
            }
            None
        }
    };

    let mut status = RepositoryStatus {
        is_git_repo: repo.is_some(),
        looks_like_tor_browser_build,
        detected_markers,
        ..Default::default()
    };

    if let Some(repo) = &repo {
        let mut remotes = Vec::new();
        if let Ok(names) = repo.remotes() {
            for name in names.iter().flatten() {
                if let Ok(remote) = repo.find_remote(name) {
                    if let Some(url) = remote.url() {
                        remotes.push(RemoteInfo {
                            name: name.to_string(),
                            url: redact_credential_urls(url),
                        });
                    }
                }
            }
        }
        status.remote_origin_url = remotes
            .iter()
            .find(|r| r.name == "origin")
            .map(|r| r.url.clone());
        status.remotes = remotes;

        match repo.head() {
            Ok(head) => {
                status.head_detached = !head.is_branch();
                status.current_branch = head.shorthand().map(|s| s.to_string());
                status.current_commit = head.target().map(|oid| oid.to_string());

                if !status.head_detached {
                    if let (Some(branch_name), Ok(local)) = (
                        head.shorthand(),
                        repo.find_branch(head.shorthand().unwrap_or(""), git2::BranchType::Local),
                    ) {
                        let _ = branch_name;
                        if let Ok(upstream) = local.upstream() {
                            if let (Some(local_oid), Some(upstream_oid)) =
                                (local.get().target(), upstream.get().target())
                            {
                                if let Ok((ahead, behind)) =
                                    repo.graph_ahead_behind(local_oid, upstream_oid)
                                {
                                    status.ahead = Some(ahead);
                                    status.behind = Some(behind);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("could not resolve HEAD: {e}")),
        }

        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        match repo.statuses(Some(&mut opts)) {
            Ok(statuses) => {
                for entry in statuses.iter() {
                    if let Some(path) = entry.path() {
                        let (label, staged) = status_label(entry.status());
                        status.changed_files.push(ChangedFile {
                            path: path.to_string(),
                            status: label.to_string(),
                            staged,
                        });
                    }
                }
                status.changed_file_count = status.changed_files.len();
                status.is_dirty = status.changed_file_count > 0;
            }
            Err(e) => errors.push(format!("could not read working tree status: {e}")),
        }

        if let Ok(tag_names) = repo.tag_names(None) {
            status.tags = tag_names.iter().flatten().map(|s| s.to_string()).collect();
        }

        if let Ok(submodules) = repo.submodules() {
            status.submodules = submodules
                .iter()
                .map(|s| s.path().to_string_lossy().to_string())
                .collect();
        }

        if let Ok(fetch_head) = root.join(".git/FETCH_HEAD").metadata() {
            if let Ok(modified) = fetch_head.modified() {
                status.last_fetch_ms = Some(crate::util::time::system_time_ms(modified));
            }
        }
    } else if !root.exists() {
        errors.push("selected path does not exist".to_string());
    } else if !root.join(".git").exists() {
        errors.push("selected path is not a git checkout (no .git found)".to_string());
    }

    status.validation_errors = errors;
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_fixture_repo(dir: &Path) {
        std::fs::create_dir_all(dir.join("projects")).unwrap();
        std::fs::write(dir.join("rbm.conf"), "# fixture rbm.conf\n").unwrap();
        std::fs::write(dir.join("Makefile"), "all:\n\techo build\n").unwrap();
        run(dir, &["init", "-q", "-b", "main"]);
        run(dir, &["config", "user.email", "[email protected]"]);
        run(dir, &["config", "user.name", "Fixture"]);
        run(dir, &["add", "."]);
        run(dir, &["commit", "-q", "-m", "init fixture"]);
    }

    #[test]
    fn detects_valid_fixture_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        init_fixture_repo(tmp.path());
        let status = inspect_repository(tmp.path());
        assert!(status.is_git_repo);
        assert!(status.looks_like_tor_browser_build);
        assert!(!status.is_dirty);
        assert!(status.current_commit.is_some());
        assert_eq!(status.current_branch.as_deref(), Some("main"));
    }

    #[test]
    fn detects_dirty_working_tree_with_statuses() {
        let tmp = tempfile::tempdir().unwrap();
        init_fixture_repo(tmp.path());
        std::fs::write(tmp.path().join("projects/new-file.yaml"), "key: value\n").unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n\techo changed\n").unwrap();
        let status = inspect_repository(tmp.path());
        assert!(status.is_dirty);
        assert_eq!(status.changed_file_count, 2);
        assert!(status
            .changed_files
            .iter()
            .any(|f| f.path == "projects/new-file.yaml" && f.status == "untracked"));
        assert!(status
            .changed_files
            .iter()
            .any(|f| f.path == "Makefile" && f.status == "modified"));
    }

    #[test]
    fn reports_non_git_directory_honestly() {
        let tmp = tempfile::tempdir().unwrap();
        let status = inspect_repository(tmp.path());
        assert!(!status.is_git_repo);
        assert!(!status.validation_errors.is_empty());
    }

    #[test]
    fn reports_missing_path_honestly() {
        let status = inspect_repository(Path::new("/definitely/does/not/exist/xyz"));
        assert!(!status.is_git_repo);
        assert!(status
            .validation_errors
            .iter()
            .any(|e| e.contains("does not exist")));
    }

    #[test]
    fn detects_tags() {
        let tmp = tempfile::tempdir().unwrap();
        init_fixture_repo(tmp.path());
        run(tmp.path(), &["tag", "v1.0.0"]);
        let status = inspect_repository(tmp.path());
        assert_eq!(status.tags, vec!["v1.0.0".to_string()]);
    }
}
