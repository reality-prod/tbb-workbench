import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import {
  FolderOpen,
  Trash,
  DownloadSimple,
  ClipboardText,
} from "@phosphor-icons/react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Alert } from "@/components/ui/Alert";
import { StatusPill } from "@/components/StatusPill";
import {
  applyLogRetention,
  buildDiagnosticBundle,
  deleteRunRecord,
  getFullLog,
  getLogsDiskUsage,
  listRunHistory,
  revealPathInFileManager,
  type DiskUsage,
  type RunRecord,
} from "@/lib/api";
import styles from "./LogsPage.module.css";

function formatDuration(ms: number | null): string {
  if (ms === null) return "—";
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m ${s % 60}s`;
}

function formatBytes(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
  return `${(bytes / 1024).toFixed(0)} KiB`;
}

export function LogsPage(): JSX.Element {
  const [records, setRecords] = useState<RunRecord[]>([]);
  const [selected, setSelected] = useState<RunRecord | null>(null);
  const [fullLog, setFullLog] = useState<string>("");
  const [logFilter, setLogFilter] = useState("");
  const [wrap, setWrap] = useState(true);
  const [diskUsage, setDiskUsage] = useState<DiskUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh(): Promise<void> {
    try {
      const [r, usage] = await Promise.all([listRunHistory(), getLogsDiskUsage()]);
      setRecords(r);
      setDiskUsage(usage);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function openRecord(record: RunRecord): Promise<void> {
    setSelected(record);
    try {
      const text = await getFullLog(record.id);
      setFullLog(text);
    } catch (e) {
      setFullLog(`(could not load log: ${String(e)})`);
    }
  }

  async function deleteSelected(): Promise<void> {
    if (!selected) return;
    await deleteRunRecord(selected.id);
    setSelected(null);
    setFullLog("");
    await refresh();
  }

  async function exportLog(kind: "combined" | "metadata"): Promise<void> {
    if (!selected) return;
    const destination = await save({
      defaultPath: kind === "combined" ? `${selected.id}.log` : `${selected.id}-metadata.json`,
    });
    if (!destination) return;
    if (kind === "combined") {
      await writeTextFile(destination, fullLog);
    } else {
      const bundle = await buildDiagnosticBundle(selected.id);
      await writeTextFile(destination, bundle);
    }
  }

  async function copyBundle(): Promise<void> {
    if (!selected) return;
    const bundle = await buildDiagnosticBundle(selected.id);
    await navigator.clipboard.writeText(bundle);
  }

  async function applyRetentionNow(): Promise<void> {
    await applyLogRetention();
    await refresh();
  }

  const filteredLog = logFilter.trim()
    ? fullLog.split("\n").filter((l) => l.toLowerCase().includes(logFilter.toLowerCase())).join("\n")
    : fullLog;

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h2>Logs</h2>
        {diskUsage && (
          <span className={styles.diskUsage}>
            {diskUsage.file_count} file(s), {formatBytes(diskUsage.total_bytes)} on disk
          </span>
        )}
        <div className={styles.spacer} />
        <Button variant="secondary" size="sm" onClick={() => void applyRetentionNow()}>
          Apply retention now
        </Button>
      </div>

      {error && <Alert tone="danger">{error}</Alert>}

      <div className={styles.layout}>
        <div className={styles.historyColumn}>
          {records.length === 0 && <p className={styles.empty}>No runs yet.</p>}
          {records.map((r) => (
            <button
              key={r.id}
              className={selected?.id === r.id ? `${styles.historyRow} ${styles.selected}` : styles.historyRow}
              onClick={() => void openRecord(r)}
            >
              <div className={styles.historyTop}>
                <span className={styles.historyLabel}>{r.label}</span>
                <StatusPill state={r.state} />
              </div>
              <div className={styles.historyMeta}>
                <span>{r.project_name}</span>
                <span>{formatDuration(r.duration_ms)}</span>
                {r.repo_commit && <span className={styles.mono}>{r.repo_commit.slice(0, 8)}</span>}
                {r.repo_dirty && <Badge tone="warning">dirty</Badge>}
              </div>
            </button>
          ))}
        </div>

        <div className={styles.viewerColumn}>
          {!selected && <p className={styles.empty}>Select a run to view its log.</p>}
          {selected && (
            <Card
              title={selected.label}
              actions={
                <>
                  <Button size="sm" variant="ghost" icon={<FolderOpen size={14} />} onClick={() => void revealPathInFileManager(selected.log_file)}>
                    Reveal Log File
                  </Button>
                  <Button size="sm" variant="ghost" icon={<ClipboardText size={14} />} onClick={() => void copyBundle()}>
                    Copy diagnostic bundle
                  </Button>
                  <Button size="sm" variant="ghost" icon={<DownloadSimple size={14} />} onClick={() => void exportLog("combined")}>
                    Export log
                  </Button>
                  <Button size="sm" variant="danger" icon={<Trash size={14} />} onClick={() => void deleteSelected()}>
                    Delete
                  </Button>
                </>
              }
            >
              <dl className={styles.metaDl}>
                <dt>Command</dt>
                <dd className={styles.mono}>{selected.executable} {selected.args.join(" ")}</dd>
                <dt>Working dir</dt>
                <dd className={styles.mono}>{selected.working_dir}</dd>
                <dt>Exit code</dt>
                <dd>{selected.exit_code ?? "—"} {selected.signal !== null && `(signal ${selected.signal})`}</dd>
                <dt>Warnings / Errors</dt>
                <dd>{selected.stats.warnings} / {selected.stats.errors}</dd>
              </dl>

              <div className={styles.logToolbar}>
                <input
                  className={styles.logFilter}
                  placeholder="Filter log lines"
                  value={logFilter}
                  onChange={(e) => setLogFilter(e.target.value)}
                />
                <label className={styles.wrapToggle}>
                  <input type="checkbox" checked={wrap} onChange={(e) => setWrap(e.target.checked)} />
                  Wrap
                </label>
                <button
                  className={styles.copyAll}
                  onClick={() => void navigator.clipboard.writeText(fullLog)}
                >
                  Copy all
                </button>
              </div>
              <pre className={wrap ? styles.logBoxWrap : styles.logBox}>{filteredLog}</pre>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

async function writeTextFile(path: string, content: string): Promise<void> {
  const { writeTextFile: write } = await import("@tauri-apps/plugin-fs");
  await write(path, content);
}
