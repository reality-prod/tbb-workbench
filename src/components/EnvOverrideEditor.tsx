import { Plus, Trash, Eye, EyeSlash } from "@phosphor-icons/react";
import { useState } from "react";
import type { EnvironmentOverride } from "@/lib/api";
import styles from "./EnvOverrideEditor.module.css";

const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

export function EnvOverrideEditor({
  value,
  onChange,
  revealValues,
}: {
  value: EnvironmentOverride[];
  onChange: (next: EnvironmentOverride[]) => void;
  revealValues: boolean;
}): JSX.Element {
  const [showSecrets, setShowSecrets] = useState(revealValues);

  function update(idx: number, patch: Partial<EnvironmentOverride>): void {
    const next = value.map((e, i) => (i === idx ? { ...e, ...patch } : e));
    onChange(next);
  }

  function remove(idx: number): void {
    onChange(value.filter((_, i) => i !== idx));
  }

  function add(): void {
    onChange([...value, { key: "", value: "", likely_secret: false }]);
  }

  return (
    <div className={styles.wrap}>
      <div className={styles.headerRow}>
        <span className={styles.label}>Environment overrides</span>
        <button
          type="button"
          className={styles.toggleReveal}
          onClick={() => setShowSecrets((s) => !s)}
        >
          {showSecrets ? <EyeSlash size={14} /> : <Eye size={14} />}
          {showSecrets ? "Hide values" : "Show values"}
        </button>
      </div>
      {value.length === 0 && <p className={styles.empty}>No overrides. The run inherits your normal environment.</p>}
      {value.map((env, idx) => {
        const keyValid = env.key === "" || ENV_KEY_RE.test(env.key);
        return (
          <div key={idx} className={styles.row}>
            <input
              className={`${styles.keyInput} ${!keyValid ? styles.invalid : ""}`}
              placeholder="VAR_NAME"
              value={env.key}
              onChange={(e) => update(idx, { key: e.target.value })}
              aria-invalid={!keyValid}
            />
            <input
              className={styles.valueInput}
              type={env.likely_secret && !showSecrets ? "password" : "text"}
              placeholder="value"
              value={env.value}
              onChange={(e) => update(idx, { value: e.target.value })}
            />
            <label className={styles.secretToggle}>
              <input
                type="checkbox"
                checked={env.likely_secret}
                onChange={(e) => update(idx, { likely_secret: e.target.checked })}
              />
              secret
            </label>
            <button
              type="button"
              className={styles.removeButton}
              onClick={() => remove(idx)}
              aria-label={`Remove ${env.key || "override"}`}
            >
              <Trash size={14} />
            </button>
          </div>
        );
      })}
      <button type="button" className={styles.addButton} onClick={add}>
        <Plus size={14} />
        Add override
      </button>
    </div>
  );
}
