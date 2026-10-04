import type { DiffLine } from "@/lib/api";
import styles from "./DiffView.module.css";

export function DiffView({ lines }: { lines: DiffLine[] }): JSX.Element {
  if (lines.length === 0) {
    return <p className={styles.empty}>No differences.</p>;
  }
  return (
    <div className={styles.diff}>
      {lines.map((line, i) => (
        <div
          key={i}
          className={
            line.tag === "insert" ? styles.insert : line.tag === "delete" ? styles.delete : styles.context
          }
        >
          <span className={styles.marker} aria-hidden="true">
            {line.tag === "insert" ? "+" : line.tag === "delete" ? "-" : " "}
          </span>
          {line.text}
        </div>
      ))}
    </div>
  );
}
