import { create } from "zustand";
import {
  cancelRun,
  forceKillRun,
  runInvocation,
  subscribeToRunCompletion,
  subscribeToRunOutput,
  type BuildInvocation,
  type BuildState,
  type CompletionEvent,
  type OutputBatch,
  type RunStats,
} from "@/lib/api";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface LogLine {
  seq: number;
  stream: "stdout" | "stderr";
  text: string;
  timestampMs: number;
  level: "info" | "warning" | "error";
}

interface RunStoreState {
  runId: string | null;
  state: BuildState;
  lines: LogLine[];
  stats: RunStats | null;
  exitCode: number | null;
  signal: number | null;
  durationMs: number | null;
  logFile: string | null;
  startedAtMs: number | null;
  error: string | null;
  start: (projectId: string, invocation: BuildInvocation) => Promise<void>;
  cancel: (projectId: string) => Promise<void>;
  forceKill: (projectId: string) => Promise<void>;
  clear: () => void;
}

// Bounded in-memory buffer: the full, unabridged log always lives in the
// on-disk file written by the Rust backend (see Logs page / "Reveal Log
// File"). The UI only needs a recent window to stay responsive.
const MAX_BUFFERED_LINES = 5000;

let unlistenOutput: UnlistenFn | null = null;
let unlistenCompletion: UnlistenFn | null = null;

export const useRunStore = create<RunStoreState>((set) => ({
  runId: null,
  state: "idle",
  lines: [],
  stats: null,
  exitCode: null,
  signal: null,
  durationMs: null,
  logFile: null,
  startedAtMs: null,
  error: null,

  start: async (projectId: string, invocation: BuildInvocation) => {
    unlistenOutput?.();
    unlistenCompletion?.();
    set({
      lines: [],
      stats: null,
      exitCode: null,
      signal: null,
      durationMs: null,
      logFile: null,
      error: null,
      state: "preparing",
      startedAtMs: Date.now(),
    });

    try {
      const runId = await runInvocation(projectId, invocation);
      set({ runId, state: "running" });

      unlistenOutput = await subscribeToRunOutput(runId, (batch: OutputBatch) => {
        set((s) => {
          const newLines: LogLine[] = batch.events.map((e) => ({
            seq: e.seq,
            stream: e.stream,
            text: e.text,
            timestampMs: e.timestamp_ms,
            level: e.level,
          }));
          const next = [...s.lines, ...newLines];
          if (next.length > MAX_BUFFERED_LINES) {
            next.splice(0, next.length - MAX_BUFFERED_LINES);
          }
          return { lines: next, stats: batch.stats, state: batch.state };
        });
      });

      unlistenCompletion = await subscribeToRunCompletion(runId, (event: CompletionEvent) => {
        set({
          state: event.state,
          exitCode: event.exit_code,
          signal: event.signal,
          durationMs: event.duration_ms,
          stats: event.stats,
          logFile: event.log_file,
        });
      });
    } catch (err) {
      set({ error: String(err), state: "failed" });
    }
  },

  cancel: async (projectId: string) => {
    set({ state: "cancelling" });
    try {
      await cancelRun(projectId);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  forceKill: async (projectId: string) => {
    try {
      await forceKillRun(projectId);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  clear: () => {
    unlistenOutput?.();
    unlistenCompletion?.();
    set({
      runId: null,
      state: "idle",
      lines: [],
      stats: null,
      exitCode: null,
      signal: null,
      durationMs: null,
      logFile: null,
      startedAtMs: null,
      error: null,
    });
  },
}));
