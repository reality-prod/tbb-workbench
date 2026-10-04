//! Aggregates host profile, tool checks, and repository health into one
//! diagnostic report, plus JSON export for "Export diagnostics JSON".

use crate::repository::inspect_repository;
use crate::system::{
    disk_info_for, host_profile, run_tool_checks, DiskInfo, HostProfile, ToolCheck,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryHealth {
    pub is_git_repo: bool,
    pub looks_like_tor_browser_build: bool,
    pub detected_markers: Vec<String>,
    pub submodule_count: usize,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub generated_at_ms: i64,
    pub host: HostProfile,
    pub disk: Option<DiskInfo>,
    pub tools: Vec<ToolCheck>,
    pub repository: Option<RepositoryHealth>,
    pub readiness_note: String,
}

pub fn run_diagnostics(project_root: Option<&Path>) -> DiagnosticsReport {
    let host = host_profile();
    let disk = project_root.and_then(disk_info_for);
    let tools = run_tool_checks();
    let repository = project_root.map(|root| {
        let status = inspect_repository(root);
        RepositoryHealth {
            is_git_repo: status.is_git_repo,
            looks_like_tor_browser_build: status.looks_like_tor_browser_build,
            detected_markers: status.detected_markers,
            submodule_count: status.submodules.len(),
            validation_errors: status.validation_errors,
        }
    });

    DiagnosticsReport {
        generated_at_ms: crate::util::time::now_ms(),
        host,
        disk,
        tools,
        repository,
        readiness_note: crate::system::overall_linux_build_readiness_note().to_string(),
    }
}

pub fn to_json_pretty(report: &DiagnosticsReport) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

pub fn to_copyable_text(report: &DiagnosticsReport) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "TBB Workbench diagnostics ({}) \n",
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(report.generated_at_ms)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default()
    ));
    s.push_str(&format!(
        "Host: {} {} ({}), {} cores, {:.1} GiB RAM\n",
        report.host.os,
        report.host.os_version.clone().unwrap_or_default(),
        report.host.arch,
        report.host.cpu_cores,
        report.host.total_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0
    ));
    if let Some(disk) = &report.disk {
        s.push_str(&format!(
            "Disk at {}: {:.1} GiB free of {:.1} GiB\n",
            disk.mount_point,
            disk.available_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
            disk.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0
        ));
    }
    s.push_str("Tools:\n");
    for t in &report.tools {
        s.push_str(&format!(
            "  {:<12} {:?}  {}\n",
            t.name,
            t.status,
            t.version.clone().unwrap_or_default()
        ));
    }
    if let Some(repo) = &report.repository {
        s.push_str(&format!(
            "Repository: git={} tbb-like={} markers={:?}\n",
            repo.is_git_repo, repo.looks_like_tor_browser_build, repo.detected_markers
        ));
        if !repo.validation_errors.is_empty() {
            s.push_str(&format!("  issues: {:?}\n", repo.validation_errors));
        }
    }
    s.push_str(&format!("\n{}\n", report.readiness_note));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_without_a_project_root() {
        let report = run_diagnostics(None);
        assert!(report.repository.is_none());
        assert!(!report.tools.is_empty());
    }

    #[test]
    fn runs_with_a_real_fixture_repo() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("rbm.conf"), "").unwrap();
        std::fs::create_dir_all(tmp.path().join("projects")).unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n").unwrap();
        let report = run_diagnostics(Some(tmp.path()));
        let repo = report.repository.unwrap();
        assert!(repo.looks_like_tor_browser_build);
    }

    #[test]
    fn json_and_text_exports_are_well_formed() {
        let report = run_diagnostics(None);
        let json = to_json_pretty(&report).unwrap();
        assert!(json.contains("\"host\""));
        let text = to_copyable_text(&report);
        assert!(text.contains("Host:"));
    }
}
