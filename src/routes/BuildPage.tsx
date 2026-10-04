import { useEffect, useMemo, useState } from "react";
import { Play, Stop, HandPalm, Terminal as TerminalIcon, MagnifyingGlass } from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { useRunStore } from "@/features/builds/runStore";
import { PreflightModal } from "@/features/builds/PreflightModal";
import { StatusPill } from "@/components/StatusPill";
import { CommandPreview } from "@/components/CommandPreview";
import { EnvOverrideEditor } from "@/components/EnvOverrideEditor";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Alert } from "@/components/ui/Alert";
import { Badge } from "@/components/ui/Badge";
import { useSettingsStore } from "@/features/settings/settingsStore";
import {
  discoverSupportedCommands,
  isTerminalState,
  openTerminalHere,
  scanProjectArtifacts,
  type BuildInvocation,
  type DiscoveredTarget,
  type EnvironmentOverride,
} from "@/lib/api";
import styles from "./BuildPage.module.css";

type Mode = "preset" | "custom";

export function BuildPage(): JSX.Element {
  const { currentProject } = useProjectStore();
  const { settings } = useSettingsStore();
  const { runId, state, lines, stats, exitCode, signal, logFile, cancel, forceKill, start } =
    useRunStore();

  const [mode, setMode] = useState<Mode>("preset");
  const [targets, setTargets] = useState<DiscoveredTarget[]>([]);
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);
  const [selectedTarget, setSelectedTarget] = useState<string>("");

  const [executable, setExecutable] = useState("make");
  const [argsText, setArgsText] = useState("");
  const [envOverrides, setEnvOverrides] = useState<EnvironmentOverride[]>([]);
  const [expectedOutputDirs, setExpectedOutputDirs] = useState("out");

  const [showPreflight, setShowPreflight] = useState(false);
  const [artifactCount, setArtifactCount] = useState<number | null>(null);

  useEffect(() => {
    if (!currentProject) return;
    discoverSupportedCommands(currentProject.id)
      .then((report) => setTargets(report.targets))
      .catch((e) => setDiscoveryError(String(e)));
  }, [currentProject]);

  const args = useMemo(() => {
    if (mode === "preset") {
      return selectedTarget ? [selectedTarget] : [];
    }
    return argsText.split(" ").filter(Boolean);
  }, [mode, selectedTarget, argsText]);

  const invocation: BuildInvocation | null = useMemo(() => {
    if (!currentProject) return null;
    return {
      executable: mode === "preset" ? "make" : executable,
      args,
      working_dir: currentProject.root_path,
      env_overrides: envOverrides,
      label: mode === "preset" ? `make ${selectedTarget}` : "Custom command",
      shell_mode: false,
      expected_output_dirs: expectedOutputDirs.split(",").map((s) => s.trim()).filter(Boolean),
      preset_id: mode === "preset" ? selectedTarget : null,
      kind: "build",
    };
  }, [currentProject, mode, executable, args, envOverrides, expectedOutputDirs, selectedTarget]);

  const isRunning = !isTerminalState(state) && state !== "idle";

  useEffect(() => {
    if (state === "succeeded" && currentProject && invocation) {
      scanProjectArtifacts(currentProject.id, invocation.expected_output_dirs, false)
        .then((r) => setArtifactCount(r.artifacts.length))
        .catch(() => setArtifactCount(null));
    }
  }, [state, currentProject, invocation]);

  if (!currentProject) {
    return <p className={styles.empty}>Open a project first (see Overview).</p>;
  }

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h2>Build</h2>
        <StatusPill state={state} />
        {stats?.current_phase && <Badge tone="info">{stats.current_phase}</Badge>}
      </div>

      <Alert tone="warning" title="Tor Browser builds can be long-running and resource-intensive">
        Linking and packaging steps in particular may consume significant RAM and disk. This app
        cannot guarantee those stages can always be constrained safely — keep an eye on your
        system while a full build runs.
      </Alert>

      <Card title="Command" actions={
        <div className={styles.modeToggle}>
          <button
            className={mode === "preset" ? styles.modeActive : styles.modeInactive}
            onClick={() => setMode("preset")}
          >
            Detected targets
          </button>
          <button
            className={mode === "custom" ? styles.modeActive : styles.modeInactive}
            onClick={() => setMode("custom")}
          >
            Custom / advanced
          </button>
        </div>
      }>
        {mode === "preset" ? (
          <div className={styles.presetArea}>
            {discoveryError && <Alert tone="warning">{discoveryError}</Alert>}
            {targets.length === 0 && !discoveryError && (
              <p className={styles.panelNote}>
                No Makefile targets detected in this checkout yet. Switch to Custom / advanced, or
                verify this is a tor-browser-build style checkout on the Overview page.
              </p>
            )}
            <div className={styles.targetGrid}>
              {targets.map((t) => (
                <label
                  key={`${t.source_file}:${t.name}`}
                  className={selectedTarget === t.name ? styles.targetCardActive : styles.targetCard}
                >
                  <input
                    type="radio"
                    name="target"
                    checked={selectedTarget === t.name}
                    onChange={() => setSelectedTarget(t.name)}
                  />
                  <div>
                    <div className={styles.targetName}>{t.name}</div>
                    {t.help_text && <div className={styles.targetHelp}>{t.help_text}</div>}
                    <div className={styles.targetMeta}>
                      {t.source_file}:{t.line} — {t.availability_note}
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>
        ) : (
          <div className={styles.commandRow}>
            <label className={styles.field}>
              <span>Executable</span>
              <input value={executable} onChange={(e) => setExecutable(e.target.value)} />
            </label>
            <label className={styles.field}>
              <span>Arguments (space-separated)</span>
              <input value={argsText} onChange={(e) => setArgsText(e.target.value)} />
            </label>
          </div>
        )}

        <label className={styles.field}>
          <span>Expected output directories (comma-separated, relative to project root)</span>
          <input value={expectedOutputDirs} onChange={(e) => setExpectedOutputDirs(e.target.value)} />
        </label>

        <EnvOverrideEditor
          value={envOverrides}
          onChange={setEnvOverrides}
          revealValues={settings?.safety.reveal_env_values ?? false}
        />

        {invocation && <CommandPreview invocation={invocation} />}

        <div className={styles.controls}>
          <Button
            variant="primary"
            icon={<Play size={16} weight="fill" />}
            disabled={isRunning || !invocation || (mode === "preset" && !selectedTarget)}
            onClick={() => setShowPreflight(true)}
          >
            Run Preflight &amp; Start
          </Button>
          {isRunning && (
            <>
              <Button
                variant="secondary"
                icon={<HandPalm size={16} />}
                onClick={() => void cancel(currentProject.id)}
              >
                Request Graceful Interrupt
              </Button>
              <Button
                variant="danger"
                icon={<Stop size={16} weight="fill" />}
                onClick={() => void forceKill(currentProject.id)}
              >
                Force Stop
              </Button>
            </>
          )}
          <Button
            variant="ghost"
            icon={<TerminalIcon size={16} />}
            onClick={() => void openTerminalHere(currentProject.root_path)}
          >
            Open Terminal Here
          </Button>
        </div>
      </Card>

      <Card
        title="Live output"
        icon={<MagnifyingGlass size={18} />}
        actions={runId ? <span className={styles.runId}>run {runId.slice(0, 8)}</span> : undefined}
      >
        <div className={styles.statsRow}>
          {stats && (
            <>
              <Badge tone={stats.warnings > 0 ? "warning" : "neutral"}>{stats.warnings} warnings</Badge>
              <Badge tone={stats.errors > 0 ? "danger" : "neutral"}>{stats.errors} errors</Badge>
              {stats.downloads > 0 && <Badge tone="info">{stats.downloads} downloads seen</Badge>}
              {stats.progress && (
                <Badge tone="accent">
                  {stats.progress.current}/{stats.progress.total} ({stats.progress.percent.toFixed(0)}%,
                  heuristic)
                </Badge>
              )}
            </>
          )}
        </div>
        <div className={styles.logPane} role="log" aria-live="polite">
          {lines.length === 0 && (
            <p className={styles.logEmpty}>
              No output yet. This pane streams the real subprocess stdout/stderr, batched for
              performance; the complete raw log is always written to disk.
            </p>
          )}
          {lines.map((line) => (
            <div
              key={line.seq}
              className={
                line.level === "error"
                  ? styles.errorLine
                  : line.level === "warning"
                    ? styles.warningLine
                    : line.stream === "stderr"
                      ? styles.stderrLine
                      : styles.stdoutLine
              }
            >
              <span className={styles.streamTag} aria-hidden="true">
                {line.stream === "stderr" ? "ERR" : "OUT"}
              </span>
              {line.text}
            </div>
          ))}
        </div>
        {exitCode !== null && (
          <p className={styles.exitSummary}>
            Process exited with code {exitCode}
            {signal !== null ? ` (signal ${signal})` : ""}.
            {logFile && <> Full log: <span className={styles.mono}>{logFile}</span></>}
            {artifactCount !== null && <> {artifactCount} artifact(s) found in expected output directories.</>}
          </p>
        )}
      </Card>

      {showPreflight && invocation && (
        <PreflightModal
          projectId={currentProject.id}
          invocation={invocation}
          onClose={() => setShowPreflight(false)}
          onConfirm={() => {
            setShowPreflight(false);
            void start(currentProject.id, invocation);
          }}
        />
      )}
    </div>
  );
}
