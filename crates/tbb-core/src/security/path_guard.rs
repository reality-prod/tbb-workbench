//! Single enforcement point for "never operate outside the project root".
//! Tauri capability scopes are coarser than "this opened project", so every
//! filesystem operation on project content goes through these functions.

use anyhow::{bail, Result};
use std::path::{Component, Path, PathBuf};

/// Canonicalizes `candidate` (resolving symlinks) and verifies it lives inside
/// `root`. Handles paths that do not exist yet by resolving the nearest
/// existing ancestor.
pub fn ensure_within_root(root: &Path, candidate: &Path) -> Result<PathBuf> {
    let canon_root = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("project root does not exist or is inaccessible: {e}"))?;
    let (existing_base, remainder) = nearest_existing_ancestor(candidate)?;
    let canon_base = existing_base
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("cannot resolve path: {e}"))?;
    let mut resolved = canon_base;
    for comp in remainder.components() {
        match comp {
            Component::Normal(p) => resolved.push(p),
            Component::CurDir => {}
            _ => bail!(
                "path traversal rejected: unsupported component in '{}'",
                candidate.display()
            ),
        }
    }
    if !resolved.starts_with(&canon_root) {
        bail!(
            "path traversal rejected: '{}' resolves outside project root '{}'",
            candidate.display(),
            root.display()
        );
    }
    Ok(resolved)
}

/// Joins a project-relative path (as sent by the UI) to `root` after
/// rejecting absolute paths and `..` components lexically, then applies the
/// canonical containment check.
pub fn resolve_relative(root: &Path, rel: &str) -> Result<PathBuf> {
    if rel.contains('\0') {
        bail!("path traversal rejected: NUL byte in path");
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() || rel.starts_with('/') || rel.starts_with('\\') {
        bail!("path traversal rejected: absolute path '{rel}' is not allowed");
    }
    for c in rel_path.components() {
        match c {
            Component::ParentDir => bail!("path traversal rejected: '..' in '{rel}'"),
            Component::Prefix(_) | Component::RootDir => {
                bail!("path traversal rejected: absolute path '{rel}' is not allowed")
            }
            _ => {}
        }
    }
    ensure_within_root(root, &root.join(rel_path))
}

/// Path relative to the (canonical) root using '/' separators.
pub fn to_relative(root: &Path, abs: &Path) -> Option<String> {
    let canon_root = root.canonicalize().ok()?;
    let rel = abs
        .strip_prefix(&canon_root)
        .ok()
        .or_else(|| abs.strip_prefix(root).ok())?;
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn nearest_existing_ancestor(path: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut remainder_parts = vec![];
    let mut cur = path.to_path_buf();
    loop {
        if cur.exists() {
            let mut remainder = PathBuf::new();
            for part in remainder_parts.into_iter().rev() {
                remainder.push(part);
            }
            return Ok((cur, remainder));
        }
        let Some(file_name) = cur.file_name() else {
            bail!("path has no valid ancestor on disk: {}", path.display());
        };
        remainder_parts.push(file_name.to_owned());
        if !cur.pop() {
            bail!("path has no valid ancestor on disk: {}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn accepts_path_inside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let inner = tmp.path().join("projects/foo.conf");
        fs::create_dir_all(inner.parent().unwrap()).unwrap();
        fs::write(&inner, "x").unwrap();
        let result = ensure_within_root(tmp.path(), &inner).unwrap();
        assert!(result.starts_with(tmp.path().canonicalize().unwrap()));
    }

    #[test]
    fn rejects_dot_dot_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(ensure_within_root(tmp.path(), &tmp.path().join("../../etc/passwd")).is_err());
        assert!(resolve_relative(tmp.path(), "../x").is_err());
        assert!(resolve_relative(tmp.path(), "a/../../x").is_err());
    }

    #[test]
    fn rejects_absolute_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(resolve_relative(tmp.path(), "/etc/passwd").is_err());
        assert!(resolve_relative(tmp.path(), "a\0b").is_err());
    }

    #[test]
    fn allows_not_yet_created_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = resolve_relative(tmp.path(), "new/dir/file.yaml").unwrap();
        assert_eq!(
            p,
            tmp.path().canonicalize().unwrap().join("new/dir/file.yaml")
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "s").unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("link")).unwrap();
        assert!(resolve_relative(tmp.path(), "link/secret.txt").is_err());
        assert!(resolve_relative(tmp.path(), "link").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn allows_symlink_inside_root() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        fs::write(tmp.path().join("real/a.txt"), "a").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("real"), tmp.path().join("alias")).unwrap();
        assert!(resolve_relative(tmp.path(), "alias/a.txt").is_ok());
    }
}
