import { useEffect, useState } from "react";
import { Warning } from "@phosphor-icons/react";
import { Card } from "@/components/ui/Card";
import { Alert } from "@/components/ui/Alert";
import { useSettingsStore } from "@/features/settings/settingsStore";
import type { ApplicationSettings, ThemePreference } from "@/lib/api";
import styles from "./SettingsPage.module.css";

export function SettingsPage(): JSX.Element {
  const { settings, load, save } = useSettingsStore();
  const [draft, setDraft] = useState<ApplicationSettings | null>(settings);

  useEffect(() => {
    if (!settings) void load();
    else setDraft(settings);
  }, [settings, load]);

  if (!draft) {
    return <p className={styles.empty}>Loading settings\u2026</p>;
  }

  function set<K extends keyof ApplicationSettings>(key: K, value: ApplicationSettings[K]): void {
    const next = { ...draft!, [key]: value };
    setDraft(next);
    void save(next);
  }

  function setSafety<K extends keyof ApplicationSettings["safety"]>(
    key: K,
    value: ApplicationSettings["safety"][K],
  ): void {
    set("safety", { ...draft!.safety, [key]: value });
  }

  function setRetention<K extends keyof ApplicationSettings["retention"]>(
    key: K,
    value: ApplicationSettings["retention"][K],
  ): void {
    set("retention", { ...draft!.retention, [key]: value });
  }

  function setEditor<K extends keyof ApplicationSettings["editor"]>(
    key: K,
    value: ApplicationSettings["editor"][K],
  ): void {
    set("editor", { ...draft!.editor, [key]: value });
  }

  return (
    <div className={styles.page}>
      <h2>Settings</h2>

      <Card title="Theme">
        <div className={styles.radioRow}>
          {(["system", "light", "dark"] as ThemePreference[]).map((theme) => (
            <label key={theme} className={styles.radioOption}>
              <input
                type="radio"
                name="theme"
                checked={draft.theme === theme}
                onChange={() => set("theme", theme)}
              />
              {theme[0]!.toUpperCase() + theme.slice(1)}
            </label>
          ))}
        </div>
      </Card>

      <Card title="Editor">
        <label className={styles.fieldRow}>
          <span>Tab size</span>
          <input
            type="number"
            min={1}
            max={8}
            value={draft.editor.tab_size}
            onChange={(e) => setEditor("tab_size", Number(e.target.value))}
            className={styles.numberInput}
          />
        </label>
        <label className={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={draft.editor.insert_spaces}
            onChange={(e) => setEditor("insert_spaces", e.target.checked)}
          />
          Insert spaces instead of tabs
        </label>
        <label className={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={draft.editor.show_whitespace}
            onChange={(e) => setEditor("show_whitespace", e.target.checked)}
          />
          Show whitespace characters
        </label>
      </Card>

      <Card title="Log retention">
        <label className={styles.fieldRow}>
          <span>Maximum stored runs</span>
          <input
            type="number"
            min={1}
            value={draft.retention.max_logs}
            onChange={(e) => setRetention("max_logs", Number(e.target.value))}
            className={styles.numberInput}
          />
        </label>
        <label className={styles.fieldRow}>
          <span>Maximum age (days, 0 = unlimited)</span>
          <input
            type="number"
            min={0}
            value={draft.retention.max_age_days}
            onChange={(e) => setRetention("max_age_days", Number(e.target.value))}
            className={styles.numberInput}
          />
        </label>
      </Card>

      <Card title="Safety">
        <label className={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={draft.safety.allow_advanced_commands}
            onChange={(e) => setSafety("allow_advanced_commands", e.target.checked)}
          />
          Allow advanced / custom command mode
        </label>
        <label className={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={draft.safety.allow_shell_command_mode}
            onChange={(e) => setSafety("allow_shell_command_mode", e.target.checked)}
          />
          Allow shell command mode (runs a string through a shell instead of a plain argument
          array)
        </label>
        {draft.safety.allow_shell_command_mode && (
          <Alert tone="warning" title="Shell command mode is enabled">
            Commands run this way are interpreted by a shell. Only use this for commands you
            wrote and trust yourself.
          </Alert>
        )}
        <label className={styles.checkboxRow}>
          <input
            type="checkbox"
            checked={draft.safety.reveal_env_values}
            onChange={(e) => setSafety("reveal_env_values", e.target.checked)}
          />
          Show raw environment variable values in command previews (instead of [REDACTED] for
          likely secrets)
        </label>
      </Card>

      <Card title="About" icon={<Warning size={20} />}>
        <p className={styles.aboutText}>
          TBB Workbench is a local, independent tool for inspecting and driving an existing
          tor-browser-build checkout. It is <strong>not affiliated with or endorsed by The Tor
          Project</strong>. Tor and Tor Browser are trademarks of The Tor Project.
        </p>
        <p className={styles.aboutText}>
          Privacy: no telemetry, no account, no automatic remote requests beyond the git/build
          operations you explicitly trigger. All application data stays on your machine.
        </p>
      </Card>
    </div>
  );
}
