import { useEffect, useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { CommandPreview } from "@/components/CommandPreview";
import {
  getPerlPreflight,
  getPreflightReport,
  installCpanm,
  installRbmPerlModules,
  type BuildInvocation,
  type PerlPreflightReport,
  type PreflightReport,
} from "@/lib/api";
import styles from "./PreflightModal.module.css";

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "unknown";
  const gib = bytes / 1024 / 1024 / 1024;
  return `${gib.toFixed(1)} GiB`;
}

function moduleTone(status: string): "info" | "warning" | "danger" {
  if (status === "passed") return "info";
  if (status === "warning") return "warning";
  return "danger";
}

export function PreflightModal({
  projectId,
  invocation,
  onClose,
  onConfirm,
}: {
  projectId: string;
  invocation: BuildInvocation;
  onClose: () => void;
  onConfirm: () => void;
}): JSX.Element {
  const [report, setReport] = useState<PreflightReport | null>(null);
  const [perl, setPerl] = useState<PerlPreflightReport | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [cpanmError, setCpanmError] = useState<string | null>(null);
  const [cpanmOutput, setCpanmOutput] = useState<string | null>(null);
  const [moduleError, setModuleError] = useState<string | null>(null);
  const [moduleOutput, setModuleOutput] = useState<string | null>(null);
  const [installingCpanm, setInstallingCpanm] = useState<string | null>(null);
  const [installingModules, setInstallingModules] = useState(false);
  const [understood, setUnderstood] = useState(false);
  const [dirtyAck, setDirtyAck] = useState(false);

  const loadPreflight = async (): Promise<void> => {
    setLoadError(null);
    const [nextReport, nextPerl] = await Promise.all([
      getPreflightReport(projectId, invocation),
      getPerlPreflight(),
    ]);
    setReport(nextReport);
    setPerl(nextPerl);
  };

  useEffect(() => {
    let cancelled = false;
    loadPreflight().catch((error) => {
      if (!cancelled) setLoadError(String(error));
    });
    return () => {
      cancelled = true;
    };
  }, [projectId, invocation]);

  const perlReady =
    !!perl && perl.blocking_modules.length === 0 && perl.checks.every((check) => check.status === "passed");

  const canProceed =
    !!report &&
    !report.blocked &&
    perlReady &&
    understood &&
    (!report.requires_dirty_confirmation || dirtyAck);

  const handleInstallCpanm = async (method: string): Promise<void> => {
    setInstallingCpanm(method);
    setCpanmError(null);
    setCpanmOutput(null);

    try {
      const output = await installCpanm(method);
      setCpanmOutput(output);
      await loadPreflight();
    } catch (error) {
      setCpanmError(String(error));
    } finally {
      setInstallingCpanm(null);
    }
  };

  const handleCopy = async (command: string): Promise<void> => {
    try {
      await navigator.clipboard.writeText(command);
      setCpanmError(null);
      setCpanmOutput(`Copied to clipboard:\n${command}`);
    } catch (error) {
      setCpanmError(`Could not copy the command: ${String(error)}`);
    }
  };

  const handleInstallModules = async (): Promise<void> => {
    setInstallingModules(true);
    setModuleError(null);
    setModuleOutput(null);

    try {
      const result = await installRbmPerlModules();
      setModuleOutput(result.output);
      await loadPreflight();
    } catch (error) {
      setModuleError(String(error));
    } finally {
      setInstallingModules(false);
    }
  };

  const busy = installingCpanm !== null || installingModules;
  const canInstallModules = !!perl && perl.cpanm_available && perl.missing_modules.length > 0;

  return (
    <Modal
      title="Preflight check"
      onClose={onClose}
      wide
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={busy}>
            Cancel
          </Button>

          {perl && perl.missing_modules.length > 0 && !perl.cpanm_available && (
            <div className={styles.footerActions}>
              {perl.platform.cpanm_install_options.map((option) => (
                <Button
                  key={option.id}
                  variant="primary"
                  disabled={busy}
                  onClick={() => {
                    if (option.can_run_in_app) {
                      void handleInstallCpanm(option.id);
                    } else {
                      void handleCopy(option.display_command);
                    }
                  }}
                >
                  {installingCpanm === option.id
                    ? "Installing cpanm..."
                    : option.can_run_in_app
                      ? option.label
                      : `Copy ${option.package_manager} command`}
                </Button>
              ))}
            </div>
          )}

          {canInstallModules && (
            <Button variant="primary" disabled={busy} onClick={() => void handleInstallModules()}>
              {installingModules ? "Installing Perl modules..." : "Install Perl modules"}
            </Button>
          )}

          <Button
            variant="secondary"
            disabled={busy}
            onClick={() => {
              void loadPreflight();
            }}
          >
            Re-check
          </Button>

          <Button variant="primary" disabled={!canProceed || busy} onClick={onConfirm}>
            Start Build
          </Button>
        </>
      }
    >
      {loadError && <Alert tone="danger">{loadError}</Alert>}
      {!report && !loadError && <p>Checking repository, Perl, package managers, and RBM modules...</p>}

      {report && (
        <div className={styles.body}>
          <CommandPreview invocation={invocation} />

          <section className={styles.moduleSection}>
            <div>
              <h3>RBM Perl dependencies</h3>
              <p>
                These are the external Perl modules imported by the RBM package. Perl core modules
                are not included.
              </p>
            </div>

            {perl && (
              <>
                <dl className={styles.platformSummary}>
                  <dt>Platform</dt>
                  <dd>
                    {perl.platform.os}
                    {perl.platform.os_version ? ` ${perl.platform.os_version}` : ""}
                  </dd>
                  <dt>Architecture</dt>
                  <dd>{perl.platform.arch}</dd>
                  <dt>Perl</dt>
                  <dd>{perl.platform.perl_path ?? "not found"}</dd>
                  <dt>Package managers</dt>
                  <dd>
                    {perl.platform.package_managers.length > 0
                      ? perl.platform.package_managers
                          .map((manager) => `${manager.name} (${manager.path})`)
                          .join(", ")
                      : "none detected"}
                  </dd>
                  <dt>cpanm</dt>
                  <dd>{perl.cpanm_available ? perl.cpanm_path : "not installed"}</dd>
                  <dt>Local Perl library</dt>
                  <dd>{perl.local_lib_root ?? "unavailable"}</dd>
                </dl>

                <div className={styles.moduleList}>
                  {perl.checks.map((check) => (
                    <div className={styles.moduleRow} key={check.module}>
                      <span className={styles.moduleName}>{check.module}</span>
                      <Alert tone={moduleTone(check.status)}>
                        {check.status === "passed" ? "Installed" : check.detail}
                      </Alert>
                    </div>
                  ))}
                </div>

                {perl.missing_modules.length > 0 && !perl.cpanm_available && (
                  <Alert tone="warning" title="cpanm is required for the missing modules">
                    <p>
                      No cpanm executable is available to the app. The detected package managers are
                      shown above.
                    </p>
                    {perl.platform.cpanm_install_options.length === 0 && (
                      <p>
                        No supported package manager was detected. Install cpanm manually, then use
                        Re-check.
                      </p>
                    )}
                    {perl.platform.cpanm_install_options.map((option) => (
                      <div key={option.id} className={styles.installOption}>
                        <strong>{option.label}</strong>
                        <code>{option.display_command}</code>
                        <span>{option.auth_note}</span>
                      </div>
                    ))}
                  </Alert>
                )}

                {cpanmError && <Alert tone="danger" title="cpanm installation failed">{cpanmError}</Alert>}
                {cpanmOutput && (
                  <Alert tone="info" title="cpanm installation result">
                    <pre className={styles.output}>{cpanmOutput}</pre>
                  </Alert>
                )}

                {canInstallModules && perl.install_command && (
                  <Alert tone="info" title="Module installation">
                    <p>Missing modules will be installed into the user-local Perl library.</p>
                    <p className={styles.mono}>{perl.install_command}</p>
                  </Alert>
                )}

                {moduleError && (
                  <Alert tone="danger" title="Perl module installation failed">
                    {moduleError}
                  </Alert>
                )}
                {moduleOutput && (
                  <Alert tone="info" title="Perl module installation result">
                    <pre className={styles.output}>{moduleOutput}</pre>
                  </Alert>
                )}

                {perlReady && (
                  <Alert tone="info" title="Perl preflight passed">
                    All required external RBM modules are loadable by the Perl executable used for
                    the check.
                  </Alert>
                )}
              </>
            )}
          </section>

          <dl className={styles.summary}>
            <dt>Working directory</dt>
            <dd className={styles.mono}>{report.working_dir}</dd>
            <dt>Branch</dt>
            <dd>{report.repository.current_branch ?? "unknown"}</dd>
            <dt>Commit</dt>
            <dd className={styles.mono}>{report.repository.current_commit ?? "unknown"}</dd>
            <dt>Working tree</dt>
            <dd>
              {report.repository.is_dirty
                ? `${report.repository.changed_file_count} changed file(s)`
                : "Clean"}
            </dd>
            <dt>Available disk space</dt>
            <dd>{formatBytes(report.available_disk_bytes)}</dd>
          </dl>

          {report.issues.map((issue) => (
            <Alert
              key={issue.code}
              tone={
                issue.severity === "error"
                  ? "danger"
                  : issue.severity === "warning"
                    ? "warning"
                    : "info"
              }
              title={issue.summary}
            >
              {issue.details && <p>{issue.details}</p>}
              {issue.suggested_action && (
                <p>
                  <strong>Suggested:</strong> {issue.suggested_action}
                </p>
              )}
            </Alert>
          ))}

          {report.requires_dirty_confirmation && (
            <label className={styles.checkboxRow}>
              <input
                type="checkbox"
                checked={dirtyAck}
                onChange={(event) => setDirtyAck(event.target.checked)}
              />
              I understand this checkout has local modifications and I want to build with
              uncommitted changes.
            </label>
          )}

          <label className={styles.checkboxRow}>
            <input
              type="checkbox"
              checked={understood}
              onChange={(event) => setUnderstood(event.target.checked)}
            />
            I understand this may build modified artifacts and executes the checked-out project&apos;s
            own build scripts on my machine.
          </label>
        </div>
      )}
    </Modal>
  );
}
