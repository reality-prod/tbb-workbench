//! Best-effort parsing of subprocess output into warning/error counts, phase
//! markers, and a progress estimate. Every parsed signal is heuristic and
//! labeled as such in `RunStats.progress.source` / phase names — this module
//! never claims certainty the underlying command doesn't provide.
//!
//! Raw output is never discarded or altered by this parsing; it only reads
//! lines to update counters alongside the untouched raw stream.

use crate::models::{LineLevel, PhaseMark, ProgressInfo, RunStats};
use regex::Regex;
use std::sync::LazyLock;

static WARNING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bwarning\b[:\]]").unwrap());
static ERROR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(error|fatal|fatal error|panic)\b[:\]]").unwrap());
static FRACTION_PROGRESS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[?\s*(\d{1,7})\s*/\s*(\d{1,7})\s*\]?").unwrap());
static DOWNLOAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(downloading|fetching|cloning into)\b").unwrap());

/// Generic phase markers recognizable across make/RBM-driven builds without
/// assuming any specific project's exact wording.
const PHASE_PATTERNS: &[(&str, &str)] = &[
    (r"(?i)entering directory", "Entering subdirectory"),
    (r"(?i)^cloning into", "Fetching source"),
    (
        r"(?i)\bconfigure\b.*\bstarted\b|\brunning configure\b",
        "Configuring",
    ),
    (r"(?i)\bcompiling\b|\bbuilding\b", "Compiling"),
    (r"(?i)\blinking\b", "Linking"),
    (
        r"(?i)\bpackaging\b|\bcreating package\b|\bcreating archive\b",
        "Packaging",
    ),
    (r"(?i)\bverify(ing)?\b|\bchecksum\b", "Verifying"),
    (
        r"(?i)\bsuccessfully built\b|\bbuild (complete|finished|succeeded)\b",
        "Build complete",
    ),
];

struct PhaseMatcher {
    label: &'static str,
    re: Regex,
}

static PHASE_MATCHERS: LazyLock<Vec<PhaseMatcher>> = LazyLock::new(|| {
    PHASE_PATTERNS
        .iter()
        .map(|(pat, label)| PhaseMatcher {
            label,
            re: Regex::new(pat).unwrap(),
        })
        .collect()
});

pub fn classify_line(text: &str) -> LineLevel {
    if ERROR_RE.is_match(text) {
        LineLevel::Error
    } else if WARNING_RE.is_match(text) {
        LineLevel::Warning
    } else {
        LineLevel::Info
    }
}

/// Updates `stats` in place based on one new output line. Called once per
/// line as it streams in; O(1) amortized regex work per line.
pub fn observe_line(stats: &mut RunStats, text: &str, level: LineLevel, at_ms: i64) {
    stats.lines_total += 1;
    match level {
        LineLevel::Warning => stats.warnings += 1,
        LineLevel::Error => {
            stats.errors += 1;
            if stats.first_fatal.is_none() {
                stats.first_fatal = Some(text.to_string());
            }
        }
        LineLevel::Info => {}
    }

    if DOWNLOAD_RE.is_match(text) {
        stats.downloads += 1;
    }

    for matcher in PHASE_MATCHERS.iter() {
        if matcher.re.is_match(text) {
            if stats.current_phase.as_deref() != Some(matcher.label) {
                stats.current_phase = Some(matcher.label.to_string());
                stats.phases.push(PhaseMark {
                    name: matcher.label.to_string(),
                    at_ms,
                });
            }
            break;
        }
    }

    if let Some(caps) = FRACTION_PROGRESS_RE.captures(text) {
        if let (Ok(current), Ok(total)) = (caps[1].parse::<u32>(), caps[2].parse::<u32>()) {
            if total > 0 && current <= total {
                stats.progress = Some(ProgressInfo {
                    current,
                    total,
                    percent: (current as f32 / total as f32) * 100.0,
                    source:
                        "parsed heuristically from an 'N/M' pattern in output; not authoritative"
                            .to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_warnings_and_errors() {
        assert_eq!(
            classify_line("warning: unused variable `x`"),
            LineLevel::Warning
        );
        assert_eq!(
            classify_line("error: cannot find value `y`"),
            LineLevel::Error
        );
        assert_eq!(
            classify_line("fatal: not a git repository"),
            LineLevel::Error
        );
        assert_eq!(classify_line("Compiling foo v0.1.0"), LineLevel::Info);
    }

    #[test]
    fn counts_accumulate_across_lines() {
        let mut stats = RunStats::default();
        for (line, ms) in [
            ("Compiling core", 0),
            ("warning: deprecated api", 1),
            ("error: build failed", 2),
        ] {
            let level = classify_line(line);
            observe_line(&mut stats, line, level, ms);
        }
        assert_eq!(stats.warnings, 1);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.lines_total, 3);
        assert_eq!(stats.first_fatal.as_deref(), Some("error: build failed"));
    }

    #[test]
    fn detects_phase_transitions_once() {
        let mut stats = RunStats::default();
        for line in [
            "Entering directory 'foo'",
            "Entering directory 'foo'",
            "Linking target",
        ] {
            let level = classify_line(line);
            observe_line(&mut stats, line, level, 0);
        }
        assert_eq!(stats.phases.len(), 2);
        assert_eq!(stats.current_phase.as_deref(), Some("Linking"));
    }

    #[test]
    fn progress_is_labeled_heuristic() {
        let mut stats = RunStats::default();
        observe_line(
            &mut stats,
            "[42/100] Building object foo.o",
            LineLevel::Info,
            0,
        );
        let progress = stats.progress.expect("progress detected");
        assert_eq!(progress.current, 42);
        assert_eq!(progress.total, 100);
        assert!(progress.source.contains("heuristic"));
    }

    #[test]
    fn ignores_malformed_fraction() {
        let mut stats = RunStats::default();
        observe_line(
            &mut stats,
            "ratio was 100/42 which is invalid",
            LineLevel::Info,
            0,
        );
        assert!(stats.progress.is_none());
    }
}
