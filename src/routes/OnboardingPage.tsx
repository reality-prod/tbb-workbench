import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "react-router-dom";
import { Clock, FolderOpen, GitBranch, ArrowRight } from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { Button } from "@/components/ui/Button";
import { listRecentProjects, type RecentProject } from "@/lib/api";
import styles from "./OnboardingPage.module.css";

export function OnboardingPage(): JSX.Element {
  const navigate = useNavigate();
  const { openProjectAtPath, isLoading, error } = useProjectStore();
  const [manualPath, setManualPath] = useState("");
  const [recent, setRecent] = useState<RecentProject[]>([]);

  useEffect(() => {
    listRecentProjects()
      .then(setRecent)
      .catch(() => setRecent([]));
  }, []);

  async function handleOpenExisting(): Promise<void> {
    const selected = await open({ directory: true, multiple: false });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;
    await openProjectAtPath(path);
    navigate("/projects");
  }

  async function handleOpenManualPath(): Promise<void> {
    if (!manualPath.trim()) return;
    await openProjectAtPath(manualPath.trim());
    navigate("/projects");
  }

  async function handleOpenRecent(path: string): Promise<void> {
    await openProjectAtPath(path);
    navigate("/projects");
  }

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>Build workflows, made inspectable.</h1>
      <p className={styles.lede}>
        TBB Workbench is a local frontend for an existing tor-browser-build
        checkout on your machine. It does not download, clone, or run
        anything until you explicitly choose to. Tor and Tor Browser are
        trademarks of The Tor Project; this application is not affiliated
        with or endorsed by The Tor Project.
      </p>

      <div className={styles.actions}>
        <Button
          variant="primary"
          icon={<FolderOpen size={16} weight="fill" />}
          onClick={() => void handleOpenExisting()}
          disabled={isLoading}
        >
          Open Existing Checkout
        </Button>
        <Button variant="secondary" icon={<GitBranch size={16} />} onClick={() => navigate("/clone")}>
          Clone Upstream Build Repository
        </Button>
      </div>

      <div className={styles.manualEntry}>
        <label htmlFor="manual-path" className={styles.manualLabel}>
          Or paste a checkout path directly
        </label>
        <div className={styles.manualRow}>
          <input
            id="manual-path"
            className={styles.manualInput}
            type="text"
            placeholder="/home/you/code/tor-browser-build"
            value={manualPath}
            onChange={(e) => setManualPath(e.target.value)}
          />
          <Button
            variant="secondary"
            icon={<ArrowRight size={14} />}
            onClick={() => void handleOpenManualPath()}
            disabled={isLoading || !manualPath.trim()}
          >
            Open
          </Button>
        </div>
      </div>

      {recent.length > 0 && (
        <div className={styles.recentSection}>
          <p className={styles.manualLabel}>Recent</p>
          <ul className={styles.recentList}>
            {recent.slice(0, 6).map((p) => (
              <li key={p.path}>
                <button className={styles.recentItem} onClick={() => void handleOpenRecent(p.path)}>
                  <Clock size={14} />
                  <span className={styles.recentName}>{p.name}</span>
                  <span className={styles.recentPath}>{p.path}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {error && <p className={styles.error} role="alert">{error}</p>}
    </div>
  );
}
