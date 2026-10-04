//! Discovers build output artifacts under a set of expected output
//! directories. Never invents a location — if nothing is found, that's
//! reported honestly ("Output location not detected"), not papered over.

use crate::models::BuildArtifact;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactScanResult {
    pub searched_dirs: Vec<String>,
    pub artifacts: Vec<BuildArtifact>,
    pub any_dir_existed: bool,
}

const ARTIFACT_EXTENSIONS: &[&str] = &[
    "tar.gz", "tar.xz", "tar.bz2", "zip", "dmg", "exe", "AppImage", "mar", "deb", "rpm",
];

fn kind_for(path: &Path) -> String {
    let name = path.to_string_lossy().to_lowercase();
    for ext in ARTIFACT_EXTENSIONS {
        if name.ends_with(&format!(".{}", ext.to_lowercase())) {
            return (*ext).to_string();
        }
    }
    "other".to_string()
}

/// Scans `expected_dirs` (relative or absolute; relative ones are resolved
/// against `working_dir`) for files. `compute_hashes` is opt-in since
/// hashing large release artifacts can be slow — the UI should default it
/// off and let the user request it.
pub fn scan_artifacts(
    working_dir: &Path,
    expected_dirs: &[String],
    compute_hashes: bool,
) -> ArtifactScanResult {
    let mut result = ArtifactScanResult {
        searched_dirs: vec![],
        artifacts: vec![],
        any_dir_existed: false,
    };

    for raw in expected_dirs {
        let dir = if Path::new(raw).is_absolute() {
            Path::new(raw).to_path_buf()
        } else {
            working_dir.join(raw)
        };
        result.searched_dirs.push(dir.to_string_lossy().to_string());
        if !dir.is_dir() {
            continue;
        }
        result.any_dir_existed = true;

        let walker = ignore::WalkBuilder::new(&dir).hidden(false).build();
        for entry in walker.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified_ms = meta
                .modified()
                .ok()
                .map(crate::util::time::system_time_ms)
                .unwrap_or(0);
            let sha256 = if compute_hashes {
                hash_file(path).ok()
            } else {
                None
            };
            result.artifacts.push(BuildArtifact {
                path: path.to_string_lossy().to_string(),
                size: meta.len(),
                modified_ms,
                kind: kind_for(path),
                sha256,
            });
        }
    }

    result
        .artifacts
        .sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    result
}

fn hash_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_artifacts_in_expected_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::write(out_dir.join("tor-browser-linux64.tar.xz"), b"fake").unwrap();
        std::fs::write(out_dir.join("notes.txt"), b"not an artifact ext").unwrap();

        let result = scan_artifacts(tmp.path(), &["out".to_string()], true);
        assert!(result.any_dir_existed);
        assert_eq!(result.artifacts.len(), 2);
        let tarball = result
            .artifacts
            .iter()
            .find(|a| a.path.ends_with("tor-browser-linux64.tar.xz"))
            .unwrap();
        assert_eq!(tarball.kind, "tar.xz");
        assert!(tarball.sha256.is_some());
    }

    #[test]
    fn honestly_reports_when_output_dir_never_existed() {
        let tmp = tempfile::tempdir().unwrap();
        let result = scan_artifacts(tmp.path(), &["never-created".to_string()], false);
        assert!(!result.any_dir_existed);
        assert!(result.artifacts.is_empty());
    }

    #[test]
    fn hashing_is_opt_in() {
        let tmp = tempfile::tempdir().unwrap();
        let out_dir = tmp.path().join("out");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::write(out_dir.join("a.zip"), b"data").unwrap();
        let result = scan_artifacts(tmp.path(), &["out".to_string()], false);
        assert!(result.artifacts[0].sha256.is_none());
    }
}
