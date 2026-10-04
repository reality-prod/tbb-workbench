//! Application settings persistence: local JSON file only, no cloud, no
//! secrets. Safety toggles default to the safe/conservative choice.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySettings {
    /// Enables the advanced/custom-command build path in the UI at all.
    pub allow_advanced_commands: bool,
    /// Enables the separately-gated "Run shell command" mode. Off by
    /// default; the runner refuses shell_mode invocations unless this is
    /// explicitly true (checked by the Tauri command layer, not core).
    pub allow_shell_command_mode: bool,
    /// Whether command previews show raw env values instead of
    /// `[REDACTED]` for likely-secret keys. Off by default.
    pub reveal_env_values: bool,
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            allow_advanced_commands: true,
            allow_shell_command_mode: false,
            reveal_env_values: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub last_opened_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionSettings {
    pub max_logs: usize,
    pub max_age_days: u32,
}

impl Default for RetentionSettings {
    fn default() -> Self {
        Self {
            max_logs: 100,
            max_age_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub show_whitespace: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 2,
            insert_spaces: true,
            show_whitespace: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationSettings {
    pub theme: ThemePreference,
    pub safety: SafetySettings,
    pub retention: RetentionSettings,
    pub editor: EditorSettings,
    pub recent_projects: Vec<RecentProject>,
    pub default_clone_parent_dir: Option<PathBuf>,
}

pub fn load(path: &Path) -> anyhow::Result<ApplicationSettings> {
    if !path.exists() {
        return Ok(ApplicationSettings::default());
    }
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn save(path: &Path, settings: &ApplicationSettings) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn remember_project(settings: &mut ApplicationSettings, path: PathBuf, name: String) {
    settings.recent_projects.retain(|p| p.path != path);
    settings.recent_projects.insert(
        0,
        RecentProject {
            path,
            name,
            last_opened_ms: crate::util::time::now_ms(),
        },
    );
    settings.recent_projects.truncate(20);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let s = ApplicationSettings::default();
        assert!(!s.safety.allow_shell_command_mode);
        assert!(!s.safety.reveal_env_values);
    }

    #[test]
    fn round_trips_through_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let mut settings = ApplicationSettings::default();
        settings.safety.allow_shell_command_mode = true;
        save(&path, &settings).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.safety.allow_shell_command_mode);
    }

    #[test]
    fn missing_file_yields_defaults_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = load(&tmp.path().join("nope.json")).unwrap();
        assert!(!loaded.safety.allow_shell_command_mode);
    }

    #[test]
    fn remembering_project_dedupes_and_caps_list() {
        let mut s = ApplicationSettings::default();
        for i in 0..25 {
            remember_project(&mut s, PathBuf::from(format!("/p{i}")), format!("p{i}"));
        }
        assert_eq!(s.recent_projects.len(), 20);
        remember_project(&mut s, PathBuf::from("/p24"), "p24-renamed".to_string());
        assert_eq!(s.recent_projects.len(), 20);
        assert_eq!(s.recent_projects[0].name, "p24-renamed");
    }
}
