//! On-disk run history: one JSON record per run plus its raw log file.
//! Records are already redacted (env values) before being written, per the
//! "no build secrets or credentials in plaintext" requirement.

use crate::models::RunRecord;
use crate::settings::RetentionSettings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RECORD_EXT: &str = ".record.json";

pub fn record_path(logs_dir: &Path, run_id: &str) -> PathBuf {
    logs_dir.join(format!("{run_id}{RECORD_EXT}"))
}

pub fn save_record(logs_dir: &Path, record: &RunRecord) -> anyhow::Result<()> {
    std::fs::create_dir_all(logs_dir)?;
    let path = record_path(logs_dir, &record.id);
    let text = serde_json::to_string_pretty(record)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn load_record(logs_dir: &Path, run_id: &str) -> anyhow::Result<RunRecord> {
    let text = std::fs::read_to_string(record_path(logs_dir, run_id))?;
    Ok(serde_json::from_str(&text)?)
}

/// Lists all run records, newest first. Corrupt/unreadable records are
/// skipped rather than failing the whole listing — one bad file shouldn't
/// hide the rest of the user's history.
pub fn list_records(logs_dir: &Path) -> Vec<RunRecord> {
    let mut records = Vec::new();
    let Ok(entries) = std::fs::read_dir(logs_dir) else {
        return records;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(RECORD_EXT))
            .unwrap_or(false)
        {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<RunRecord>(&text) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.started_at_ms.cmp(&a.started_at_ms));
    records
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total_bytes: u64,
    pub file_count: usize,
}

pub fn disk_usage(logs_dir: &Path) -> DiskUsage {
    let mut total = 0u64;
    let mut count = 0usize;
    if let Ok(entries) = std::fs::read_dir(logs_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                    count += 1;
                }
            }
        }
    }
    DiskUsage {
        total_bytes: total,
        file_count: count,
    }
}

/// Deletes records/log files beyond `retention` limits. Returns the ids
/// removed. This is only ever invoked by an explicit user action or a
/// user-configured automatic retention setting — never silently beyond
/// what the user configured.
pub fn apply_retention(
    logs_dir: &Path,
    retention: &RetentionSettings,
) -> anyhow::Result<Vec<String>> {
    let mut records = list_records(logs_dir);
    let mut removed = Vec::new();
    let now = crate::util::time::now_ms();
    let max_age_ms = retention.max_age_days as i64 * 24 * 60 * 60 * 1000;

    records.retain(|r| {
        let too_old = max_age_ms > 0 && (now - r.started_at_ms) > max_age_ms;
        if too_old {
            removed.push(r.id.clone());
            false
        } else {
            true
        }
    });

    if records.len() > retention.max_logs {
        for r in records.split_off(retention.max_logs) {
            removed.push(r.id);
        }
    }

    for id in &removed {
        let _ = std::fs::remove_file(record_path(logs_dir, id));
        let _ = std::fs::remove_file(logs_dir.join(format!("{id}.log")));
    }

    Ok(removed)
}

pub fn delete_record(logs_dir: &Path, run_id: &str) -> anyhow::Result<()> {
    std::fs::remove_file(record_path(logs_dir, run_id)).ok();
    std::fs::remove_file(logs_dir.join(format!("{run_id}.log"))).ok();
    Ok(())
}

pub fn read_raw_log_tail(log_path: &Path, max_lines: usize) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(log_path)?;
    let all: Vec<&str> = content.lines().collect();
    let start = all.len().saturating_sub(max_lines);
    Ok(all[start..].iter().map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BuildState, RunKind, RunStats, TerminationConfirmation};

    fn sample_record(id: &str, started_at_ms: i64) -> RunRecord {
        RunRecord {
            id: id.to_string(),
            kind: RunKind::Build,
            project_id: "p1".into(),
            project_name: "tor-browser-build".into(),
            project_root: None,
            label: "test build".into(),
            preset_id: None,
            executable: "make".into(),
            args: vec![],
            working_dir: PathBuf::from("/tmp"),
            env_keys: vec![],
            env_redacted: vec![],
            shell_mode: false,
            state: BuildState::Succeeded,
            started_at_ms,
            finished_at_ms: Some(started_at_ms + 1000),
            duration_ms: Some(1000),
            exit_code: Some(0),
            signal: None,
            cancel_requested: false,
            termination: TerminationConfirmation::NotRequested,
            repo_branch: None,
            repo_commit: None,
            repo_dirty: false,
            pid: None,
            log_file: format!("{id}.log"),
            stats: RunStats::default(),
            expected_output_dirs: vec![],
        }
    }

    #[test]
    fn saves_and_lists_records_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        save_record(tmp.path(), &sample_record("run-a", 1000)).unwrap();
        save_record(tmp.path(), &sample_record("run-b", 2000)).unwrap();
        let records = list_records(tmp.path());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "run-b");
    }

    #[test]
    fn retention_caps_by_count() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            save_record(tmp.path(), &sample_record(&format!("run-{i}"), i as i64)).unwrap();
            std::fs::write(tmp.path().join(format!("run-{i}.log")), "log").unwrap();
        }
        let removed = apply_retention(
            tmp.path(),
            &RetentionSettings {
                max_logs: 2,
                max_age_days: 0,
            },
        )
        .unwrap();
        assert_eq!(removed.len(), 3);
        assert_eq!(list_records(tmp.path()).len(), 2);
    }

    #[test]
    fn retention_removes_old_records_by_age() {
        let tmp = tempfile::tempdir().unwrap();
        let old_ts = crate::util::time::now_ms() - 100 * 24 * 60 * 60 * 1000;
        save_record(tmp.path(), &sample_record("run-old", old_ts)).unwrap();
        save_record(
            tmp.path(),
            &sample_record("run-new", crate::util::time::now_ms()),
        )
        .unwrap();
        let removed = apply_retention(
            tmp.path(),
            &RetentionSettings {
                max_logs: 100,
                max_age_days: 30,
            },
        )
        .unwrap();
        assert_eq!(removed, vec!["run-old".to_string()]);
    }

    #[test]
    fn corrupt_record_file_is_skipped_not_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        save_record(tmp.path(), &sample_record("run-good", 1)).unwrap();
        std::fs::write(tmp.path().join("broken.record.json"), "{not json").unwrap();
        let records = list_records(tmp.path());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "run-good");
    }

    #[test]
    fn tail_reads_last_n_lines_only() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.log");
        std::fs::write(&path, "1\n2\n3\n4\n5\n").unwrap();
        let tail = read_raw_log_tail(&path, 2).unwrap();
        assert_eq!(tail, vec!["4".to_string(), "5".to_string()]);
    }
}
