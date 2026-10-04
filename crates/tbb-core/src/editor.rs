//! Real local text editing: file tree listing, read/write with mtime-based
//! external-change detection, and unified diffs against disk/HEAD. Every
//! path is passed through `security::path_guard` before touching disk.

use crate::security::path_guard::{resolve_relative, to_relative};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeEntry {
    pub name: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    /// True for paths that are part of the "protected" set the product
    /// brief calls out (rbm.conf, projects/, config/, etc.) — still
    /// editable, but the UI shows a stronger warning before saving.
    pub protected: bool,
}

const PROTECTED_PREFIXES: &[&str] = &[
    "rbm.conf",
    "rbm/",
    "projects/",
    "config/",
    "conf/",
    "keyring/",
    "container/",
    "Makefile",
];

pub fn is_protected(rel_path: &str) -> bool {
    PROTECTED_PREFIXES
        .iter()
        .any(|p| rel_path == *p || rel_path.starts_with(p))
}

/// Lists the repository-aware file tree honoring `.gitignore`, using the
/// `ignore` crate so the walk matches what git itself would track (plus
/// respecting explicit ignores) rather than a naive `walkdir` over
/// everything including build output and `.git` internals.
pub fn list_tree(root: &Path, max_entries: usize) -> Result<Vec<FileTreeEntry>> {
    let mut out = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .filter_entry(|e| e.file_name() != ".git")
        .build();

    for entry in walker {
        if out.len() >= max_entries {
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path == root {
            continue;
        }
        let Some(rel) = to_relative(root, path) else {
            continue;
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let size = if is_dir {
            None
        } else {
            entry.metadata().ok().map(|m| m.len())
        };
        out.push(FileTreeEntry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            protected: is_protected(&rel),
            rel_path: rel,
            is_dir,
            size,
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.rel_path.cmp(&b.rel_path),
    });
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub rel_path: String,
    pub content: String,
    pub mtime_ms: i64,
    pub protected: bool,
    pub size: u64,
}

pub fn read_file(root: &Path, rel_path: &str) -> Result<FileContent> {
    let abs = resolve_relative(root, rel_path)?;
    let meta = std::fs::metadata(&abs).with_context(|| format!("cannot stat {rel_path}"))?;
    if meta.is_dir() {
        bail!("'{rel_path}' is a directory, not a file");
    }
    let content = std::fs::read_to_string(&abs)
        .with_context(|| format!("cannot read {rel_path} (is it binary?)"))?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(FileContent {
        rel_path: rel_path.to_string(),
        content,
        mtime_ms,
        protected: is_protected(rel_path),
        size: meta.len(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResult {
    pub rel_path: String,
    pub mtime_ms: i64,
    pub bytes_written: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("file changed on disk since it was opened (expected mtime {expected_ms}, found {actual_ms}) — reload and re-apply your changes, or save-anyway to overwrite")]
    ExternalChange { expected_ms: i64, actual_ms: i64 },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Writes `content` to `rel_path`, refusing (by default) if the on-disk
/// mtime no longer matches `expected_mtime_ms` — i.e. the file changed on
/// disk while it was open in the editor. `force` bypasses this after the
/// user has explicitly confirmed they want to overwrite.
pub fn write_file(
    root: &Path,
    rel_path: &str,
    content: &str,
    expected_mtime_ms: Option<i64>,
    force: bool,
) -> Result<SaveResult, SaveError> {
    let abs = resolve_relative(root, rel_path).map_err(SaveError::Other)?;

    if !force {
        if let (Some(expected), Ok(meta)) = (expected_mtime_ms, std::fs::metadata(&abs)) {
            if let Some(actual) = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
            {
                if actual != expected {
                    return Err(SaveError::ExternalChange {
                        expected_ms: expected,
                        actual_ms: actual,
                    });
                }
            }
        }
    }

    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SaveError::Other(e.into()))?;
    }
    std::fs::write(&abs, content).map_err(|e| SaveError::Other(e.into()))?;
    let meta = std::fs::metadata(&abs).map_err(|e| SaveError::Other(e.into()))?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(SaveResult {
        rel_path: rel_path.to_string(),
        mtime_ms,
        bytes_written: content.len(),
    })
}

/// Create/rename/delete are advanced-mode-only operations gated by the
/// caller; this module only enforces path containment, never policy.
pub fn delete_file(root: &Path, rel_path: &str) -> Result<()> {
    let abs = resolve_relative(root, rel_path)?;
    if abs.is_dir() {
        std::fs::remove_dir_all(&abs)?;
    } else {
        std::fs::remove_file(&abs)?;
    }
    Ok(())
}

pub fn rename_file(root: &Path, from_rel: &str, to_rel: &str) -> Result<()> {
    let from_abs = resolve_relative(root, from_rel)?;
    let to_abs = resolve_relative(root, to_rel)?;
    if to_abs.exists() {
        bail!("target '{to_rel}' already exists");
    }
    if let Some(parent) = to_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&from_abs, &to_abs)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub tag: String, // "context" | "insert" | "delete"
    pub text: String,
}

/// Unified-diff-style line list between two texts, computed with `similar`
/// (a real diff algorithm, not a naive string comparison).
pub fn diff_texts(old: &str, new: &str) -> Vec<DiffLine> {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    diff.iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Equal => "context",
                ChangeTag::Insert => "insert",
                ChangeTag::Delete => "delete",
            };
            DiffLine {
                tag: tag.to_string(),
                text: change.to_string_lossy().trim_end_matches('\n').to_string(),
            }
        })
        .collect()
}

/// Reads the file's content at HEAD via git2, for the "compare to HEAD"
/// affordance. Returns `None` if the file is untracked or the repo has no
/// commits yet, rather than erroring — that's a legitimate, expected state.
pub fn read_file_at_head(root: &Path, rel_path: &str) -> Result<Option<String>> {
    let repo = git2::Repository::open(root)?;
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };
    let tree = head.peel_to_tree()?;
    let entry = match tree.get_path(Path::new(rel_path)) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    let object = entry.to_object(&repo)?;
    let blob = match object.as_blob() {
        Some(b) => b,
        None => return Ok(None),
    };
    Ok(Some(String::from_utf8_lossy(blob.content()).to_string()))
}

/// Best-effort YAML validation: returns a human-readable error location if
/// parsing fails. Never mutates the source — the app must not silently
/// reformat YAML.
pub fn validate_yaml(content: &str) -> Result<(), String> {
    yaml_rust2::YamlLoader::load_from_str(content)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("projects")).unwrap();
        fs::write(
            tmp.path().join("projects/tor-browser.var"),
            "var:\n  x: 1\n",
        )
        .unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n").unwrap();
        fs::write(tmp.path().join("README.md"), "# hi\n").unwrap();
        tmp
    }

    #[test]
    fn lists_tree_and_flags_protected_paths() {
        let tmp = fixture();
        let entries = list_tree(tmp.path(), 100).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.rel_path.clone()).collect();
        assert!(names.contains(&"Makefile".to_string()));
        assert!(names.contains(&"projects/tor-browser.var".to_string()));
        let makefile = entries.iter().find(|e| e.rel_path == "Makefile").unwrap();
        assert!(makefile.protected);
        let readme = entries.iter().find(|e| e.rel_path == "README.md").unwrap();
        assert!(!readme.protected);
    }

    #[test]
    fn round_trips_read_write() {
        let tmp = fixture();
        let f = read_file(tmp.path(), "README.md").unwrap();
        assert_eq!(f.content, "# hi\n");
        let saved = write_file(
            tmp.path(),
            "README.md",
            "# hi there\n",
            Some(f.mtime_ms),
            false,
        )
        .unwrap();
        assert!(saved.bytes_written > 0);
        let f2 = read_file(tmp.path(), "README.md").unwrap();
        assert_eq!(f2.content, "# hi there\n");
    }

    #[test]
    fn detects_external_change_before_overwrite() {
        let tmp = fixture();
        let f = read_file(tmp.path(), "README.md").unwrap();
        // Simulate an external edit with a distinct mtime.
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(tmp.path().join("README.md"), "# changed externally\n").unwrap();

        let result = write_file(
            tmp.path(),
            "README.md",
            "# my edit\n",
            Some(f.mtime_ms),
            false,
        );
        assert!(matches!(result, Err(SaveError::ExternalChange { .. })));

        // force=true bypasses the check.
        let forced = write_file(
            tmp.path(),
            "README.md",
            "# my edit\n",
            Some(f.mtime_ms),
            true,
        );
        assert!(forced.is_ok());
    }

    #[test]
    fn write_rejects_path_traversal() {
        let tmp = fixture();
        let result = write_file(tmp.path(), "../outside.txt", "x", None, true);
        assert!(result.is_err());
    }

    #[test]
    fn diff_reports_inserted_and_deleted_lines() {
        let diff = diff_texts("a\nb\nc\n", "a\nX\nc\n");
        assert!(diff.iter().any(|l| l.tag == "delete" && l.text == "b"));
        assert!(diff.iter().any(|l| l.tag == "insert" && l.text == "X"));
        assert!(diff.iter().any(|l| l.tag == "context" && l.text == "a"));
    }

    #[test]
    fn yaml_validation_catches_real_errors() {
        assert!(validate_yaml("key: value\n").is_ok());
        assert!(validate_yaml("key: [unterminated\n").is_err());
    }

    #[test]
    fn rename_refuses_existing_target() {
        let tmp = fixture();
        fs::write(tmp.path().join("b.txt"), "b").unwrap();
        fs::write(tmp.path().join("a.txt"), "a").unwrap();
        assert!(rename_file(tmp.path(), "a.txt", "b.txt").is_err());
        assert!(rename_file(tmp.path(), "a.txt", "c.txt").is_ok());
        assert!(tmp.path().join("c.txt").exists());
    }
}
