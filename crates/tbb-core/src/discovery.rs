//! "Discover supported commands": inspects a checkout's Makefile(s) and
//! local docs for real, static evidence of build targets — never executes
//! anything to discover it, and never claims a target exists without having
//! actually found it in the checkout.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTarget {
    pub name: String,
    /// Comment immediately preceding or trailing the target rule, if any
    /// (common `## help text` or `# comment` convention).
    pub help_text: Option<String>,
    pub source_file: String,
    pub line: usize,
    /// Never claims official/upstream-supported status — only that a rule
    /// with this name exists in this file at this revision.
    pub availability_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredScript {
    pub rel_path: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryReport {
    pub targets: Vec<DiscoveredTarget>,
    pub wrapper_scripts: Vec<DiscoveredScript>,
    pub doc_files_found: Vec<String>,
}

// Rust's `regex` crate has no lookaround support, so instead of `:(?!=)` we
// match a colon not immediately followed by `=` by allowing either the line
// to end right after the colon, or the next character to be anything other
// than `=` (which also rules out `:=` and `::=` variable-assignment forms).
static TARGET_RULE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z0-9][A-Za-z0-9_.\-/%]*)\s*:(=?)").unwrap());
static PHONY_LIST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.PHONY\s*:\s*(.+)$").unwrap());
static HELP_COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*##\s?(.*)$").unwrap());

/// Parses a single Makefile's text for target rules and adjacent `##` help
/// comments (a very common, though not universal, self-documenting Make
/// convention). Rules with a leading `.` other than a small allowlist
/// (`.PHONY` etc.) and pattern/implicit rules with only `%` are skipped as
/// noise rather than presented as user-runnable targets.
pub fn parse_makefile_targets(content: &str, source_file: &str) -> Vec<DiscoveredTarget> {
    let mut targets = Vec::new();
    let mut phony: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut pending_help: Option<String> = None;

    for (idx, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim_end();

        if let Some(caps) = PHONY_LIST_RE.captures(line) {
            for t in caps[1].split_whitespace() {
                phony.insert(t.to_string());
            }
            continue;
        }

        if let Some(caps) = HELP_COMMENT_RE.captures(line) {
            pending_help = Some(caps[1].trim().to_string());
            continue;
        }

        if line.trim_start().starts_with('#') {
            // Non-help comment: doesn't reset pending_help, but doesn't
            // extend it either.
            continue;
        }

        if line.starts_with('\t') || line.trim().is_empty() {
            continue;
        }

        if let Some(caps) = TARGET_RULE_RE.captures(line) {
            let name = caps[1].to_string();
            let is_assignment = &caps[2] == "=";
            if is_assignment || name.starts_with('.') || name.contains('%') {
                pending_help = None;
                continue;
            }
            let availability_note = if phony.contains(&name) {
                "declared as .PHONY in this Makefile at the currently checked-out revision"
                    .to_string()
            } else {
                "target rule found in this Makefile at the currently checked-out revision \
                 (not confirmed .PHONY; may also represent a file)"
                    .to_string()
            };
            targets.push(DiscoveredTarget {
                name,
                help_text: pending_help.take(),
                source_file: source_file.to_string(),
                line: idx + 1,
                availability_note,
            });
        } else {
            pending_help = None;
        }
    }

    targets
}

const DOC_CANDIDATES: &[&str] = &[
    "README",
    "README.md",
    "README.rst",
    "HACKING",
    "HACKING.md",
    "CONTRIBUTING.md",
    "doc/",
    "docs/",
];

const MAKEFILE_CANDIDATES: &[&str] = &["Makefile", "makefile", "GNUmakefile"];

/// Runs static discovery over the checkout root: real file reads only, no
/// process execution.
pub fn discover(root: &Path) -> anyhow::Result<DiscoveryReport> {
    let mut report = DiscoveryReport::default();

    for candidate in MAKEFILE_CANDIDATES {
        let path = root.join(candidate);
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                report
                    .targets
                    .extend(parse_makefile_targets(&content, candidate));
            }
        }
    }

    // Also check one level into tools/ and rbm/ for auxiliary Makefiles,
    // since some checkouts split targets across subdirectories.
    for subdir in ["tools", "rbm"] {
        let dir = root.join(subdir);
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if MAKEFILE_CANDIDATES.contains(&name.as_str()) {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            let rel = format!("{subdir}/{name}");
                            report
                                .targets
                                .extend(parse_makefile_targets(&content, &rel));
                        }
                    }
                }
            }
        }
    }

    for candidate in DOC_CANDIDATES {
        let path = root.join(candidate);
        if path.exists() {
            report.doc_files_found.push(candidate.to_string());
        }
    }

    if let Ok(entries) = std::fs::read_dir(root.join("tools")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let executable = is_executable(&path);
                if executable {
                    if let Some(rel) = crate::security::path_guard::to_relative(root, &path) {
                        report.wrapper_scripts.push(DiscoveredScript {
                            rel_path: rel,
                            executable,
                        });
                    }
                }
            }
        }
    }

    Ok(report)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets_with_help_comments_and_phony() {
        let makefile = "\
.PHONY: all clean nightly-linux

## Build the default release target\nall: prep\n\t@echo building\n\n## Remove build outputs\nclean:\n\t@rm -rf out\n\nnightly-linux: all\n\t@echo nightly\n\nprep:\n\t@echo prep\n";
        let targets = parse_makefile_targets(makefile, "Makefile");
        let names: Vec<_> = targets.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"all"));
        assert!(names.contains(&"clean"));
        assert!(names.contains(&"nightly-linux"));
        assert!(names.contains(&"prep"));

        let all = targets.iter().find(|t| t.name == "all").unwrap();
        assert_eq!(
            all.help_text.as_deref(),
            Some("Build the default release target")
        );
        assert!(all.availability_note.contains("PHONY"));

        let prep = targets.iter().find(|t| t.name == "prep").unwrap();
        assert!(prep.availability_note.contains("not confirmed"));
    }

    #[test]
    fn skips_pattern_rules_and_dot_targets() {
        let makefile =
            "%.o: %.c\n\t@cc -c $<\n\n.DEFAULT_GOAL := all\n\nreal_target:\n\t@echo hi\n";
        let targets = parse_makefile_targets(makefile, "Makefile");
        let names: Vec<_> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["real_target"]);
    }

    #[test]
    fn full_discovery_over_fixture_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Makefile"),
            "## default build\nall:\n\t@true\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("README.md"), "readme").unwrap();
        std::fs::create_dir_all(tmp.path().join("tools")).unwrap();
        let script = tmp.path().join("tools/gen-keyring.sh");
        std::fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let report = discover(tmp.path()).unwrap();
        assert!(report.targets.iter().any(|t| t.name == "all"));
        assert!(report.doc_files_found.contains(&"README.md".to_string()));
        #[cfg(unix)]
        assert!(report
            .wrapper_scripts
            .iter()
            .any(|s| s.rel_path == "tools/gen-keyring.sh"));
    }
}
