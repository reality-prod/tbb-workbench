import { useEffect, useState } from "react";
import {
  ArrowsClockwise,
  Copy,
  DownloadSimple,
  CheckCircle,
  WarningCircle,
  XCircle,
  Question,
  MinusCircle,
} from "@phosphor-icons/react";
import { save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "@/features/projects/projectStore";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Alert } from "@/components/ui/Alert";
import {
  diagnosticsAsText,
  runHostDiagnostics,
  saveDiagnosticsJson,
  type CapabilityStatus,
  type DiagnosticsReport,
} from "@/lib/api";
import styles from "./DiagnosticsPage.module.css";

const STATUS_ICON: Record<CapabilityStatus, JSX.Element> = {
  passed: <CheckCircle size={16} weight="fill" className={styles.iconSuccess} />,
  warning: <WarningCircle size={16} weight="fill" className={styles.iconWarning} />,
  missing: <XCircle size={16} weight="fill" className={styles.iconDanger} />,
  unknown: <Question size={16} className={styles.iconNeutral} />,
  not_applicable: <MinusCircle size={16} className={styles.iconNeutral} />,
};

const STATUS_LABEL: Record<CapabilityStatus, string> = {
  passed: "Passed",
  warning: "Warning",
  missing: "Missing",
  unknown: "Unknown",
  not_applicable: "Not applicable",
};

function formatBytes(bytes: number): string {
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GiB`;
}

export function DiagnosticsPage(): JSX.Element {
  const { currentProject } = useProjectStore();
  const [report, setReport] = useState<DiagnosticsReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function runChecks(): Promise<void> {
    setLoading(true);
    setError(null);
    try {
      const r = await runHostDiagnostics(currentProject?.id ?? null);
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void runChecks();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentProject]);

  async function copyReport(): Promise<void> {
    if (!report) return;
    const text = await diagnosticsAsText(report);
    await navigator.clipboard.writeText(text);
  }

  async function exportJson(): Promise<void> {
    if (!report) return;
    const destination = await save({
      defaultPath: "tbb-workbench-diagnostics.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!destination) return;
    await saveDiagnosticsJson(report, destination);
  }

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h2>Diagnostics</h2>
        <div className={styles.headerActions}>
          <Button variant="secondary" icon={<ArrowsClockwise size={14} />} onClick={() => void runChecks()} disabled={loading}>
            Run checks
          </Button>
          <Button variant="secondary" icon={<Copy size={14} />} onClick={() => void copyReport()} disabled={!report}>
            Copy diagnostics
          </Button>
          <Button variant="secondary" icon={<DownloadSimple size={14} />} onClick={() => void exportJson()} disabled={!report}>
            Export diagnostics JSON
          </Button>
        </div>
      </div>

      {error && <Alert tone="danger">{error}</Alert>}

      {report && (
        <>
          <Card title="Host profile">
            <dl className={styles.dl}>
              <dt>OS</dt>
              <dd>{report.host.os} {report.host.os_version}</dd>
              <dt>Kernel</dt>
              <dd>{report.host.kernel_version ?? "unknown"}</dd>
              <dt>Architecture</dt>
              <dd>{report.host.arch}</dd>
              <dt>CPU cores</dt>
              <dd>{report.host.cpu_cores}</dd>
              <dt>Memory</dt>
              <dd>
                {formatBytes(report.host.available_memory_bytes)} available of{" "}
                {formatBytes(report.host.total_memory_bytes)}
              </dd>
              {report.disk && (
                <>
                  <dt>Disk ({report.disk.mount_point})</dt>
                  <dd>
                    {formatBytes(report.disk.available_bytes)} available of{" "}
                    {formatBytes(report.disk.total_bytes)}
                  </dd>
                </>
              )}
            </dl>
          </Card>

          <Card title="Tool checks">
            <div className={styles.toolGrid}>
              {report.tools.map((t) => (
                <div key={t.name} className={styles.toolRow}>
                  {STATUS_ICON[t.status]}
                  <span className={styles.toolName}>{t.name}</span>
                  <Badge
                    tone={
                      t.status === "passed"
                        ? "success"
                        : t.status === "warning"
                          ? "warning"
                          : t.status === "missing"
                            ? "danger"
                            : "neutral"
                    }
                  >
                    {STATUS_LABEL[t.status]}
                  </Badge>
                  {t.version && <span className={styles.toolVersion}>{t.version}</span>}
                </div>
              ))}
            </div>
          </Card>

          {report.repository && (
            <Card title="Repository health">
              <dl className={styles.dl}>
                <dt>Git checkout</dt>
                <dd>{report.repository.is_git_repo ? "Yes" : "No"}</dd>
                <dt>tor-browser-build style</dt>
                <dd>{report.repository.looks_like_tor_browser_build ? "Yes" : "Not confirmed"}</dd>
                <dt>Detected markers</dt>
                <dd>{report.repository.detected_markers.join(", ") || "None"}</dd>
                <dt>Submodules</dt>
                <dd>{report.repository.submodule_count}</dd>
              </dl>
              {report.repository.validation_errors.length > 0 && (
                <Alert tone="warning">
                  <ul className={styles.issueList}>
                    {report.repository.validation_errors.map((e, i) => (
                      <li key={i}>{e}</li>
                    ))}
                  </ul>
                </Alert>
              )}
            </Card>
          )}

          <Alert tone="info" title="Build environment note">
            {report.readiness_note}
          </Alert>
        </>
      )}
    </div>
  );
}
