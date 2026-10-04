import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  Hammer,
  Stethoscope,
  GitDiff,
  FileCode,
  ListBullets,
  GitBranch,
} from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { Badge } from "@/components/ui/Badge";
import styles from "./DashboardPage.module.css";

export function DashboardPage(): JSX.Element {
  const { currentProject, repositoryStatus, refreshRepositoryStatus, error } = useProjectStore();
  const navigate = useNavigate();

  useEffect(() => {
    if (currentProject) {
      void refreshRepositoryStatus();
    }
  }, [currentProject, refreshRepositoryStatus]);

  if (!currentProject) {
    return (
      <div className={styles.empty}>
        <p>No project open yet. Go to Overview to open an existing checkout.</p>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <div>
        <h2 className={styles.heading}>{currentProject.name}</h2>
        <p className={styles.path}>{currentProject.root_path}</p>
      </div>

      {error && <Alert tone="danger">{error}</Alert>}

      <div className={styles.quickActions}>
        <Button variant="primary" icon={<Hammer size={16} />} onClick={() => navigate("/build")}>
          Start Build
        </Button>
        <Button variant="secondary" icon={<Stethoscope size={16} />} onClick={() => navigate("/diagnostics")}>
          Check Prerequisites
        </Button>
        <Button variant="secondary" icon={<GitDiff size={16} />} onClick={() => navigate("/editor")}>
          Inspect Changes
        </Button>
        <Button variant="secondary" icon={<FileCode size={16} />} onClick={() => navigate("/editor")}>
          Edit Configuration
        </Button>
        <Button variant="secondary" icon={<ListBullets size={16} />} onClick={() => navigate("/logs")}>
          View Latest Logs
        </Button>
      </div>

      {repositoryStatus && (
        <div className={styles.grid}>
          <Card title="Repository" icon={<GitBranch size={20} />}>
            <dl className={styles.dl}>
              <dt>Is git checkout</dt>
              <dd>{repositoryStatus.is_git_repo ? "Yes" : "No"}</dd>
              <dt>Remote origin</dt>
              <dd>{repositoryStatus.remote_origin_url ?? "Not detected"}</dd>
              <dt>Branch</dt>
              <dd>
                {repositoryStatus.current_branch ?? (repositoryStatus.head_detached ? "Detached HEAD" : "unknown")}
              </dd>
              <dt>Commit</dt>
              <dd className={styles.mono}>{repositoryStatus.current_commit ?? "Unknown"}</dd>
              <dt>Ahead / behind upstream</dt>
              <dd>
                {repositoryStatus.ahead !== null && repositoryStatus.behind !== null
                  ? `${repositoryStatus.ahead} ahead, ${repositoryStatus.behind} behind`
                  : "No upstream tracked"}
              </dd>
              <dt>Working tree</dt>
              <dd>
                {repositoryStatus.is_dirty ? (
                  <Badge tone="warning">{repositoryStatus.changed_file_count} changed file(s)</Badge>
                ) : (
                  <Badge tone="success">Clean</Badge>
                )}
              </dd>
              <dt>Tags</dt>
              <dd>{repositoryStatus.tags.length}</dd>
              <dt>Submodules</dt>
              <dd>{repositoryStatus.submodules.length}</dd>
            </dl>
          </Card>

          <Card title="Checkout detection">
            <p className={styles.cardNote}>
              Detected markers found on disk (not a required fixed layout):
            </p>
            <div className={styles.markerRow}>
              {repositoryStatus.detected_markers.length > 0 ? (
                repositoryStatus.detected_markers.map((m) => (
                  <Badge key={m} tone="accent">{m}</Badge>
                ))
              ) : (
                <span className={styles.cardNote}>None detected</span>
              )}
            </div>
            <p className={styles.cardNote}>
              {repositoryStatus.looks_like_tor_browser_build
                ? "Looks like a tor-browser-build style checkout (rbm.conf/rbm + projects/ + Makefile present)."
                : "Does not clearly match a tor-browser-build style checkout yet — you can still browse and edit it."}
            </p>
          </Card>

          {repositoryStatus.validation_errors.length > 0 && (
            <Card title="Needs attention" tone="warning">
              <ul className={styles.errorList}>
                {repositoryStatus.validation_errors.map((e, i) => (
                  <li key={i}>{e}</li>
                ))}
              </ul>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}
