import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { GitBranch } from "@phosphor-icons/react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { CommandPreview } from "@/components/CommandPreview";
import { useRunStore } from "@/features/builds/runStore";
import { useProjectStore } from "@/features/projects/projectStore";
import {
  previewCloneInvocation,
  validateCloneDestination,
  isTerminalState,
  type BuildInvocation,
  type RefKind,
} from "@/lib/api";
import styles from "./ClonePage.module.css";

const DEFAULT_UPSTREAM = "https://gitlab.torproject.org/tpo/applications/tor-browser-build.git";

export function ClonePage(): JSX.Element {
  const navigate = useNavigate();
  const { openProjectAtPath } = useProjectStore();
  const { runId, state, lines, exitCode, start, clear } = useRunStore();

  const [sourceUrl, setSourceUrl] = useState(DEFAULT_UPSTREAM);
  const [destination, setDestination] = useState("");
  const [shallow, setShallow] = useState(true);
  const [refKind, setRefKind] = useState<RefKind>("default_branch");
  const [gitRef, setGitRef] = useState("");
  const [destinationError, setDestinationError] = useState<string | null>(null);
  const [invocation, setInvocation] = useState<BuildInvocation | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  useEffect(() => {
    clear();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function pickDestination(): Promise<void> {
    const selected = await open({ directory: true, multiple: false });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) setDestination(`${path}/tor-browser-build`);
  }

  async function refreshPreview(): Promise<void> {
    setDestinationError(null);
    setPreviewError(null);
    if (!destination.trim()) return;
    try {
      await validateCloneDestination(destination.trim());
    } catch (e) {
      setDestinationError(String(e));
      return;
    }
    try {
      const inv = await previewCloneInvocation({
        source_url: sourceUrl.trim(),
        destination: destination.trim(),
        shallow,
        git_ref: refKind === "default_branch" ? null : gitRef.trim() || null,
        ref_kind: refKind === "default_branch" ? null : refKind,
      });
      setInvocation(inv);
    } catch (e) {
      setPreviewError(String(e));
    }
  }

  useEffect(() => {
    void refreshPreview();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceUrl, destination, shallow, refKind, gitRef]);

  const isRunning = !isTerminalState(state) && state !== "idle";
  const canStart = !!invocation && !destinationError && !isRunning && destination.trim() !== "";

  async function startClone(): Promise<void> {
    if (!invocation) return;
    // Clone runs are not tied to an existing project yet, so we use a
    // synthetic "no project" key; the runner still enforces one active run
    // per key, which is sufficient here since only one clone wizard exists.
    await start("clone-wizard", invocation);
  }

  async function onCloneSucceeded(): Promise<void> {
    await openProjectAtPath(destination.trim());
    navigate("/projects");
  }

  const succeeded = state === "succeeded";

  return (
    <div className={styles.page}>
      <h2>Clone upstream build repository</h2>
      <p className={styles.lede}>
        This clones a real Git repository using your system <code>git</code>. Nothing happens
        until you press Start Clone below. The default URL points at the upstream
        tor-browser-build project; verify it yourself before proceeding — this app does not
        pin or vendor upstream URLs authoritatively.
      </p>

      <Card title="Source" icon={<GitBranch size={20} />}>
        <label className={styles.field}>
          <span>Upstream URL</span>
          <input value={sourceUrl} onChange={(e) => setSourceUrl(e.target.value)} />
        </label>

        <div className={styles.refRow}>
          <label className={styles.field}>
            <span>Reference</span>
            <select value={refKind} onChange={(e) => setRefKind(e.target.value as RefKind)}>
              <option value="default_branch">Default branch</option>
              <option value="branch">Specific branch (release/maintenance)</option>
              <option value="tag">Tag</option>
              <option value="commit">Commit</option>
            </select>
          </label>
          {refKind !== "default_branch" && (
            <label className={styles.field}>
              <span>
                {refKind === "branch" ? "Branch name" : refKind === "tag" ? "Tag name" : "Commit SHA"}
              </span>
              <input value={gitRef} onChange={(e) => setGitRef(e.target.value)} />
            </label>
          )}
        </div>

        <label className={styles.checkboxRow}>
          <input type="checkbox" checked={shallow} onChange={(e) => setShallow(e.target.checked)} />
          Shallow clone (faster, but may affect workflows that need full history or tags)
        </label>
      </Card>

      <Card title="Destination">
        <div className={styles.destRow}>
          <input
            className={styles.destInput}
            value={destination}
            onChange={(e) => setDestination(e.target.value)}
            placeholder="/home/you/code/tor-browser-build"
          />
          <Button variant="secondary" onClick={() => void pickDestination()}>
            Choose folder\u2026
          </Button>
        </div>
        {destinationError && <Alert tone="danger">{destinationError}</Alert>}
      </Card>

      {previewError && <Alert tone="danger">{previewError}</Alert>}
      {invocation && (
        <Card title="Command preview">
          <CommandPreview invocation={invocation} />
          <Button variant="primary" disabled={!canStart} onClick={() => void startClone()}>
            Start Clone
          </Button>
        </Card>
      )}

      {runId && (
        <Card title="Clone output">
          <div className={styles.logPane} role="log" aria-live="polite">
            {lines.map((line) => (
              <div key={line.seq}>{line.text}</div>
            ))}
          </div>
          {exitCode !== null && (
            <p className={styles.exitSummary}>
              {succeeded
                ? "Clone completed successfully."
                : `Clone process exited with code ${exitCode}.`}
            </p>
          )}
          {succeeded && (
            <Button variant="primary" onClick={() => void onCloneSucceeded()}>
              Open this checkout
            </Button>
          )}
        </Card>
      )}
    </div>
  );
}
