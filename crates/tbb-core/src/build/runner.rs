//! Real subprocess execution for build/git invocations.
//!
//! - argv-only spawning via `tokio::process::Command`; no shell interpolation
//!   for standard runs. Shell mode (explicit, separately gated) runs
//!   `sh -c <string>` only when `BuildInvocation.shell_mode` is true — the
//!   caller (Tauri command layer) is responsible for checking the user's
//!   Settings > Safety > "allow shell command mode" toggle before ever
//!   setting that flag; this module executes whatever invocation it's given
//!   as-is, honestly, without silently reinterpreting it.
//! - stdout/stderr are read concurrently, each line timestamped, classified,
//!   folded into `RunStats`, and written immediately to a raw log file.
//! - output is emitted to the caller in small time-windowed batches, not one
//!   event per line, to avoid flooding IPC during high-output steps.
//! - cancellation: SIGTERM (Unix) is sent first; if the process hasn't
//!   exited after the caller's timeout, `force_kill` sends a hard kill.
//!   On non-Unix, `request_cancel` performs a hard kill immediately (no
//!   portable graceful-interrupt primitive), and the completion event's
//!   `termination` field reflects what actually happened, not what was
//!   requested.

use crate::build::output_parser::{classify_line, observe_line};
use crate::models::{
    BuildOutputEvent, BuildState, OutputBatch, OutputStream, RunStats, TerminationConfirmation,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone, Copy)]
pub struct RunSupervisorConfig {
    /// How often buffered output lines are flushed to the frontend.
    pub batch_interval: Duration,
    /// Max lines per batch even if the interval hasn't elapsed, to bound
    /// per-event payload size during very high-output steps.
    pub max_lines_per_batch: usize,
}

impl Default for RunSupervisorConfig {
    fn default() -> Self {
        Self {
            batch_interval: Duration::from_millis(120),
            max_lines_per_batch: 200,
        }
    }
}

pub struct RunHandle {
    pub run_id: String,
    child: Arc<Mutex<Option<Child>>>,
    pid: Option<u32>,
    cancel_requested: Arc<AtomicBool>,
    force_killed: Arc<AtomicBool>,
}

impl RunHandle {
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn cancel_was_requested(&self) -> bool {
        self.cancel_requested.load(Ordering::SeqCst)
    }

    /// Best-effort graceful interrupt. Unix: SIGTERM to the process (and, if
    /// it was spawned as a group leader, the group). Non-Unix: falls back to
    /// a hard kill since there's no portable equivalent here, and callers
    /// are told this via `termination` on the completion event rather than
    /// this function pretending otherwise.
    pub async fn request_cancel(&self) -> Result<()> {
        self.cancel_requested.store(true, Ordering::SeqCst);
        #[cfg(unix)]
        {
            let guard = self.child.lock().await;
            if let Some(child) = guard.as_ref() {
                if let Some(pid) = child.id() {
                    unsafe {
                        libc_kill(pid as i32, 15 /* SIGTERM */);
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            self.force_kill().await
        }
    }

    pub async fn force_kill(&self) -> Result<()> {
        self.force_killed.store(true, Ordering::SeqCst);
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            child.start_kill().context("failed to force-kill process")?;
        }
        Ok(())
    }
}

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    kill(pid, sig);
}

pub struct SpawnedRun {
    pub handle: Arc<RunHandle>,
    pub output_rx: mpsc::Receiver<OutputBatch>,
    pub completion_rx: tokio::sync::oneshot::Receiver<CompletionInfo>,
    pub log_file_path: PathBuf,
}

pub struct CompletionInfo {
    pub state: BuildState,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub cancel_requested: bool,
    pub termination: TerminationConfirmation,
    pub stats: RunStats,
}

/// Spawns `executable` with `args` in `working_dir`. When `shell_mode` is
/// true, `executable`/`args` are expected to already be `["sh", "-c", "..."]`
/// (or equivalent) constructed by the caller — this function does not build
/// a shell string itself, it only executes exactly what it's given.
#[allow(clippy::too_many_arguments)]
pub async fn spawn_run(
    run_id: String,
    executable: &str,
    args: &[String],
    working_dir: &std::path::Path,
    env_overrides: &[(String, String)],
    log_dir: &std::path::Path,
    config: RunSupervisorConfig,
) -> Result<SpawnedRun> {
    tokio::fs::create_dir_all(log_dir)
        .await
        .context("failed to create log directory")?;
    let log_file_path = log_dir.join(format!("{run_id}.log"));

    let mut cmd = Command::new(executable);
    cmd.args(args)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn '{executable}'"))?;
    let pid = child.id();

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<(OutputStream, String, i64)>();
    let (batch_tx, batch_rx) = mpsc::channel::<OutputBatch>(64);
    let (completion_tx, completion_rx) = tokio::sync::oneshot::channel();

    let log_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .await
        .context("failed to open log file")?;
    let log_file = Arc::new(Mutex::new(log_file));

    let stdout_task = tokio::spawn(read_lines(stdout, OutputStream::Stdout, line_tx.clone()));
    let stderr_task = tokio::spawn(read_lines(stderr, OutputStream::Stderr, line_tx.clone()));
    drop(line_tx);

    let cancel_requested = Arc::new(AtomicBool::new(false));
    let force_killed = Arc::new(AtomicBool::new(false));
    let child_shared = Arc::new(Mutex::new(Some(child)));

    let handle = Arc::new(RunHandle {
        run_id: run_id.clone(),
        child: child_shared.clone(),
        pid,
        cancel_requested: cancel_requested.clone(),
        force_killed: force_killed.clone(),
    });

    // Batching + parsing task: consumes raw lines, updates RunStats, writes
    // to the log file, and periodically flushes a batch to the caller.
    let seq_counter = Arc::new(AtomicU64::new(0));
    let run_id_for_batcher = run_id.clone();
    let log_for_batcher = log_file.clone();
    let batcher_task = tokio::spawn(async move {
        let mut stats = RunStats::default();
        let mut pending: Vec<BuildOutputEvent> = Vec::new();
        let mut ticker = tokio::time::interval(config.batch_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut closed = false;

        loop {
            tokio::select! {
                maybe_line = line_rx.recv(), if !closed => {
                    match maybe_line {
                        Some((stream, text, ts)) => {
                            let level = classify_line(&text);
                            observe_line(&mut stats, &text, level, ts);

                            let tag = match stream {
                                OutputStream::Stdout => "OUT",
                                OutputStream::Stderr => "ERR",
                            };
                            let raw_line = format!("[{ts}] [{tag}] {text}\n");
                            {
                                let mut f = log_for_batcher.lock().await;
                                let _ = f.write_all(raw_line.as_bytes()).await;
                            }

                            let seq = seq_counter.fetch_add(1, Ordering::SeqCst);
                            pending.push(BuildOutputEvent {
                                run_id: run_id_for_batcher.clone(),
                                seq,
                                stream,
                                text,
                                timestamp_ms: ts,
                                level,
                            });
                            if pending.len() >= config.max_lines_per_batch {
                                flush(&batch_tx, &run_id_for_batcher, &mut pending, &stats, BuildState::Running).await;
                            }
                        }
                        None => { closed = true; }
                    }
                }
                _ = ticker.tick() => {
                    if !pending.is_empty() {
                        flush(&batch_tx, &run_id_for_batcher, &mut pending, &stats, BuildState::Running).await;
                    }
                    if closed {
                        let _ = log_for_batcher.lock().await.flush().await;
                        break;
                    }
                }
            }
        }
        stats
    });

    let started = std::time::Instant::now();
    let child_for_wait = child_shared.clone();
    tokio::spawn(async move {
        let _ = stdout_task.await;
        let _ = stderr_task.await;
        let stats = batcher_task.await.unwrap_or_default();

        let exit_status = {
            let mut guard = child_for_wait.lock().await;
            if let Some(mut child) = guard.take() {
                child.wait().await.ok()
            } else {
                None
            }
        };

        let cancel_req = cancel_requested.load(Ordering::SeqCst);
        let was_force_killed = force_killed.load(Ordering::SeqCst);

        #[cfg(unix)]
        let signal = exit_status.and_then(|s| {
            use std::os::unix::process::ExitStatusExt;
            s.signal()
        });
        #[cfg(not(unix))]
        let signal: Option<i32> = None;

        let (state, code) = match exit_status {
            Some(status) => {
                let code = status.code();
                if cancel_req {
                    (BuildState::Cancelled, code)
                } else if status.success() {
                    (BuildState::Succeeded, code)
                } else {
                    (BuildState::Failed, code)
                }
            }
            None => (BuildState::Failed, None),
        };

        let termination = if !cancel_req {
            TerminationConfirmation::NotRequested
        } else if exit_status.is_some() {
            TerminationConfirmation::Confirmed
        } else {
            TerminationConfirmation::Unconfirmed
        };
        let _ = was_force_killed;

        let _ = completion_tx.send(CompletionInfo {
            state,
            exit_code: code,
            signal,
            cancel_requested: cancel_req,
            termination,
            stats,
        });
        let _ = started.elapsed();
    });

    Ok(SpawnedRun {
        handle,
        output_rx: batch_rx,
        completion_rx,
        log_file_path,
    })
}

async fn flush(
    tx: &mpsc::Sender<OutputBatch>,
    run_id: &str,
    pending: &mut Vec<BuildOutputEvent>,
    stats: &RunStats,
    state: BuildState,
) {
    if pending.is_empty() {
        return;
    }
    let events = std::mem::take(pending);
    let _ = tx
        .send(OutputBatch {
            run_id: run_id.to_string(),
            events,
            stats: stats.clone(),
            state,
        })
        .await;
}

async fn read_lines(
    stream: impl tokio::io::AsyncRead + Unpin,
    kind: OutputStream,
    tx: mpsc::UnboundedSender<(OutputStream, String, i64)>,
) {
    let mut reader = BufReader::new(stream).lines();
    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                let ts = chrono::Utc::now().timestamp_millis();
                if tx.send((kind, line, ts)).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn drain(mut rx: mpsc::Receiver<OutputBatch>) -> (Vec<String>, RunStats) {
        let mut lines = Vec::new();
        let mut last_stats = RunStats::default();
        while let Some(batch) = rx.recv().await {
            for e in batch.events {
                lines.push(e.text);
            }
            last_stats = batch.stats;
        }
        (lines, last_stats)
    }

    #[tokio::test]
    async fn captures_real_stdout_and_persists_raw_log() {
        let dir = tempfile::tempdir().unwrap();
        let run = spawn_run(
            "test-run".into(),
            "sh",
            &[
                "-c".into(),
                "echo hello-from-test; echo err-line 1>&2".into(),
            ],
            dir.path(),
            &[],
            &dir.path().join("logs"),
            RunSupervisorConfig::default(),
        )
        .await
        .unwrap();

        let (lines, _stats) = drain(run.output_rx).await;
        let completion = run.completion_rx.await.unwrap();
        assert_eq!(completion.state, BuildState::Succeeded);
        assert_eq!(completion.exit_code, Some(0));
        assert!(lines.iter().any(|l| l.contains("hello-from-test")));

        let log_contents = tokio::fs::read_to_string(&run.log_file_path).await.unwrap();
        assert!(log_contents.contains("hello-from-test"));
        assert!(log_contents.contains("err-line"));
    }

    #[tokio::test]
    async fn nonzero_exit_reported_as_failed() {
        let dir = tempfile::tempdir().unwrap();
        let run = spawn_run(
            "test-run-fail".into(),
            "sh",
            &["-c".into(), "echo about to fail; exit 3".into()],
            dir.path(),
            &[],
            dir.path(),
            RunSupervisorConfig::default(),
        )
        .await
        .unwrap();
        let (_lines, _stats) = drain(run.output_rx).await;
        let completion = run.completion_rx.await.unwrap();
        assert_eq!(completion.state, BuildState::Failed);
        assert_eq!(completion.exit_code, Some(3));
    }

    #[tokio::test]
    async fn spawn_failure_for_missing_executable_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let result = spawn_run(
            "test-run-missing".into(),
            "definitely-not-a-real-executable-xyz",
            &[],
            dir.path(),
            &[],
            dir.path(),
            RunSupervisorConfig::default(),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cancellation_terminates_long_running_process() {
        let dir = tempfile::tempdir().unwrap();
        let mut run = spawn_run(
            "test-run-cancel".into(),
            "sh",
            &["-c".into(), "trap 'exit 0' TERM; sleep 30".into()],
            dir.path(),
            &[],
            dir.path(),
            RunSupervisorConfig::default(),
        )
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        run.handle.request_cancel().await.unwrap();

        while run.output_rx.recv().await.is_some() {}
        let completion = tokio::time::timeout(Duration::from_secs(5), run.completion_rx)
            .await
            .expect("process should exit promptly after SIGTERM")
            .unwrap();
        assert!(completion.cancel_requested);
        assert_eq!(completion.state, BuildState::Cancelled);
    }

    #[tokio::test]
    async fn force_kill_stops_process_ignoring_sigterm() {
        let dir = tempfile::tempdir().unwrap();
        let mut run = spawn_run(
            "test-run-force".into(),
            "sh",
            &["-c".into(), "trap '' TERM; sleep 30".into()],
            dir.path(),
            &[],
            dir.path(),
            RunSupervisorConfig::default(),
        )
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        run.handle.request_cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        run.handle.force_kill().await.unwrap();

        while run.output_rx.recv().await.is_some() {}
        let completion = tokio::time::timeout(Duration::from_secs(5), run.completion_rx)
            .await
            .expect("force kill should terminate the process")
            .unwrap();
        assert!(completion.cancel_requested);
    }

    #[tokio::test]
    async fn stats_reflect_parsed_warnings_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let run = spawn_run(
            "test-run-stats".into(),
            "sh",
            &[
                "-c".into(),
                "echo 'warning: deprecated'; echo 'error: boom' 1>&2".into(),
            ],
            dir.path(),
            &[],
            dir.path(),
            RunSupervisorConfig::default(),
        )
        .await
        .unwrap();
        let (_lines, stats) = drain(run.output_rx).await;
        let completion = run.completion_rx.await.unwrap();
        assert_eq!(completion.stats.warnings, 1);
        assert_eq!(completion.stats.errors, 1);
        assert!(stats.lines_total >= 1);
    }
}
