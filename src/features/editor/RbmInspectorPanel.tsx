import { Alert } from "@/components/ui/Alert";
import { Badge } from "@/components/ui/Badge";
import type { RbmInspection } from "@/lib/api";
import styles from "./RbmInspectorPanel.module.css";

export function RbmInspectorPanel({ inspection }: { inspection: RbmInspection }): JSX.Element {
  if (!inspection.parsed_ok) {
    return (
      <Alert tone="warning" title="Could not parse as plain YAML">
        {inspection.parse_error}
        <p className={styles.note}>
          {inspection.notes[0] ??
            "This file may use RBM/Perl templating syntax that requires RBM itself to evaluate."}
        </p>
      </Alert>
    );
  }

  return (
    <div className={styles.wrap}>
      <section>
        <h4 className={styles.sectionTitle}>Top-level fields</h4>
        <ul className={styles.fieldList}>
          {inspection.top_level_fields.map((f) => (
            <li key={f.key}>
              <span className={styles.key}>{f.key}</span>
              <Badge tone="neutral">{f.kind}</Badge>
              <span className={styles.preview}>{f.value_preview}</span>
            </li>
          ))}
        </ul>
      </section>

      {inspection.variables.length > 0 && (
        <section>
          <h4 className={styles.sectionTitle}>Variables</h4>
          <ul className={styles.fieldList}>
            {inspection.variables.map((f) => (
              <li key={f.key}>
                <span className={styles.key}>{f.key}</span>
                <span className={styles.preview}>{f.value_preview}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {inspection.targets_or_platforms.length > 0 && (
        <section>
          <h4 className={styles.sectionTitle}>Targets / platforms</h4>
          <div className={styles.chipRow}>
            {inspection.targets_or_platforms.map((t) => (
              <Badge key={t} tone="accent">{t}</Badge>
            ))}
          </div>
        </section>
      )}

      {inspection.input_files.length > 0 && (
        <section>
          <h4 className={styles.sectionTitle}>Input files</h4>
          <ul className={styles.plainList}>
            {inspection.input_files.map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>
        </section>
      )}

      {inspection.includes.length > 0 && (
        <section>
          <h4 className={styles.sectionTitle}>Includes</h4>
          <ul className={styles.plainList}>
            {inspection.includes.map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>
        </section>
      )}

      <Alert tone="info">{inspection.notes[inspection.notes.length - 1]}</Alert>
    </div>
  );
}
