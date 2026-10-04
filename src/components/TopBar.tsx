import { useNavigate } from "react-router-dom";
import { MagnifyingGlass, Moon, Sun, Desktop, GitBranch, WarningCircle } from "@phosphor-icons/react";
import { useProjectStore } from "@/features/projects/projectStore";
import { useSettingsStore } from "@/features/settings/settingsStore";
import { Button } from "@/components/ui/Button";
import styles from "./TopBar.module.css";

export function TopBar(): JSX.Element {
  const { currentProject, repositoryStatus } = useProjectStore();
  const { settings, setTheme, effectiveTheme } = useSettingsStore();
  const navigate = useNavigate();

  function cycleTheme(): void {
    if (!settings) return;
    const order: Array<typeof settings.theme> = ["system", "light", "dark"];
    const currentIndex = order.indexOf(settings.theme);
    const next = order[(currentIndex + 1) % order.length] ?? "system";
    void setTheme(next);
  }

  const themeIcon =
    settings?.theme === "light" ? <Sun size={18} /> : settings?.theme === "dark" ? <Moon size={18} /> : <Desktop size={18} />;

  return (
    <header className={styles.bar}>
      <div className={styles.left}>
        {currentProject ? (
          <>
            <span className={styles.projectName}>{currentProject.name}</span>
            {repositoryStatus?.current_branch && (
              <span className={styles.commitSummary}>
                <GitBranch size={14} />
                {repositoryStatus.current_branch}
                {repositoryStatus.current_commit &&
                  ` @ ${repositoryStatus.current_commit.slice(0, 8)}`}
                {repositoryStatus.is_dirty && (
                  <span className={styles.dirtyBadge} title="Working tree has local changes">
                    <WarningCircle size={12} weight="fill" />
                    {repositoryStatus.changed_file_count} changed
                  </span>
                )}
              </span>
            )}
          </>
        ) : (
          <span className={styles.noProject}>No project open</span>
        )}
      </div>
      <div className={styles.right}>
        <button
          className={styles.iconButton}
          type="button"
          aria-label={`Theme: ${settings?.theme ?? "system"} (click to change)`}
          title={`Theme: ${settings?.theme ?? "system"} (${effectiveTheme})`}
          onClick={cycleTheme}
        >
          {themeIcon}
        </button>
        <button
          className={styles.paletteButton}
          type="button"
          aria-label="Open command palette"
          title="Command palette (Ctrl/Cmd+K)"
        >
          <MagnifyingGlass size={14} />
          Ctrl+K
        </button>
        <Button
          variant="primary"
          disabled={!currentProject}
          onClick={() => navigate("/build")}
        >
          Start Build
        </Button>
      </div>
    </header>
  );
}
