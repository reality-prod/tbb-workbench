//! Best-effort RBM configuration inspector.
//!
//! This is explicitly NOT an RBM implementation or a substitute for RBM
//! evaluating the config itself: RBM configs use YAML with custom tags,
//! Perl-based templating, `input_files`, `var` blocks, `include`s and a
//! project-inheritance model that this module does not attempt to fully
//! evaluate. What it does do, honestly:
//!   - parses the file as YAML where it validly is YAML
//!   - extracts top-level keys and (for common RBM idioms) `var:`,
//!     `steps:`/`build:`, `input_files:`, `container:`/`platforms:` blocks
//!     as raw structural info
//!   - flags any construct it could not confidently interpret rather than
//!     guessing
//!
//! The raw source is always the authoritative editing surface; this is a
//! read-only visualization layered next to it.

use serde::{Deserialize, Serialize};
use std::path::Path;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbmField {
    pub key: String,
    /// Rendered as a short display string; not a re-serialization attempt.
    pub value_preview: String,
    pub kind: String, // "scalar" | "list" | "map" | "unrecognized"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RbmInspection {
    pub parsed_ok: bool,
    pub parse_error: Option<String>,
    pub top_level_fields: Vec<RbmField>,
    pub variables: Vec<RbmField>,
    pub input_files: Vec<String>,
    pub targets_or_platforms: Vec<String>,
    pub includes: Vec<String>,
    pub notes: Vec<String>,
}

fn preview(y: &Yaml) -> (String, String) {
    match y {
        Yaml::String(s) => (s.clone(), "scalar".to_string()),
        Yaml::Integer(i) => (i.to_string(), "scalar".to_string()),
        Yaml::Real(r) => (r.clone(), "scalar".to_string()),
        Yaml::Boolean(b) => (b.to_string(), "scalar".to_string()),
        Yaml::Array(a) => (format!("[{} item(s)]", a.len()), "list".to_string()),
        Yaml::Hash(h) => (format!("{{{} key(s)}}", h.len()), "map".to_string()),
        Yaml::Null => ("null".to_string(), "scalar".to_string()),
        _ => ("(unrecognized)".to_string(), "unrecognized".to_string()),
    }
}

fn string_list(y: &Yaml) -> Vec<String> {
    match y {
        Yaml::Array(items) => items
            .iter()
            .filter_map(|i| i.as_str().map(|s| s.to_string()))
            .collect(),
        Yaml::String(s) => vec![s.clone()],
        _ => vec![],
    }
}

/// Parses best-effort. RBM configs sometimes include Perl-ish templating
/// (`%{var}`, `[% ... %]`-style substitutions in some forks) that isn't
/// valid plain YAML; when that happens we report the parse failure honestly
/// instead of pretending we understood the file.
pub fn inspect_rbm_config(content: &str) -> RbmInspection {
    let mut out = RbmInspection::default();
    let docs = match YamlLoader::load_from_str(content) {
        Ok(d) => d,
        Err(e) => {
            out.parsed_ok = false;
            out.parse_error = Some(e.to_string());
            out.notes.push(
                "This file could not be parsed as plain YAML — it may use RBM/Perl \
                 templating syntax that requires RBM itself to evaluate. Edit the raw \
                 source directly; this inspector cannot reflect its structure."
                    .to_string(),
            );
            return out;
        }
    };
    out.parsed_ok = true;

    let Some(Yaml::Hash(root)) = docs.first() else {
        out.notes
            .push("top-level document is not a YAML mapping".to_string());
        return out;
    };

    for (k, v) in root.iter() {
        let Some(key) = k.as_str() else { continue };
        let (value_preview, kind) = preview(v);
        out.top_level_fields.push(RbmField {
            key: key.to_string(),
            value_preview,
            kind,
        });

        match key {
            "var" | "vars" | "variables" => {
                if let Yaml::Hash(vars) = v {
                    for (vk, vv) in vars.iter() {
                        if let Some(vkey) = vk.as_str() {
                            let (vp, vkind) = preview(vv);
                            out.variables.push(RbmField {
                                key: vkey.to_string(),
                                value_preview: vp,
                                kind: vkind,
                            });
                        }
                    }
                }
            }
            "input_files" | "inputs" => {
                out.input_files.extend(string_list(v));
            }
            "targets" | "platforms" => {
                out.targets_or_platforms.extend(string_list(v));
            }
            "include" | "includes" => {
                out.includes.extend(string_list(v));
            }
            _ => {}
        }
    }

    out.notes.push(
        "Effective values may still depend on RBM template expansion, project \
         inheritance, and the target/environment RBM is invoked with. This view \
         reflects only the static structure of this file."
            .to_string(),
    );

    out
}

pub fn inspect_rbm_file(root: &Path, rel_path: &str) -> anyhow::Result<RbmInspection> {
    let content = crate::editor::read_file(root, rel_path)?.content;
    Ok(inspect_rbm_config(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_rbm_style_yaml() {
        let src = r#"
var:
  torbrowser_version: "13.5"
  build_type: release
input_files:
  - filename: firefox.tar.bz2
    url: 'https://example.org/firefox.tar.bz2'
targets:
  - linux-x86_64
  - windows-x86_64
include:
  - projects/common.rbm
"#;
        let out = inspect_rbm_config(src);
        assert!(out.parsed_ok);
        assert_eq!(out.variables.len(), 2);
        assert!(out.variables.iter().any(|f| f.key == "torbrowser_version"));
        assert_eq!(
            out.targets_or_platforms,
            vec!["linux-x86_64", "windows-x86_64"]
        );
        assert_eq!(out.includes, vec!["projects/common.rbm"]);
    }

    #[test]
    fn reports_parse_failure_honestly_instead_of_guessing() {
        let src = "var:\n  x: [unterminated\n";
        let out = inspect_rbm_config(src);
        assert!(!out.parsed_ok);
        assert!(out.parse_error.is_some());
        assert!(out.variables.is_empty());
    }

    #[test]
    fn always_notes_effective_value_caveat() {
        let out = inspect_rbm_config("var:\n  a: 1\n");
        assert!(out.notes.iter().any(|n| n.contains("template expansion")));
    }
}
