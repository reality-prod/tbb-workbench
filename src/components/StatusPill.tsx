import type { BuildState } from "@/lib/api";
import styles from "./StatusPill.module.css";

const LABELS: Record<BuildState, string> = {
  idle: "Idle",
  preparing: "Preparing",
  checking_prerequisites: "Checking prerequisites",
  fetching: "Fetching",
  building: "Building",
  packaging: "Packaging",
  verifying: "Verifying",
  running: "Running",
  cancelling: "Cancelling\u2026",
  cancelled: "Cancelled",
  failed: "Failed",
  succeeded: "Completed",
  interrupted_by_app_exit: "Interrupted by app exit",
};

// Every state also gets a non-color glyph so meaning never depends on color
// alone, per the accessibility requirement.
const GLYPHS: Record<BuildState, string> = {
  idle: "\u25CB",
  preparing: "\u25D0",
  checking_prerequisites: "\u25D0",
  fetching: "\u25B6",
  building: "\u25B6",
  packaging: "\u25B6",
  verifying: "\u25B6",
  running: "\u25B6",
  cancelling: "\u25A0",
  cancelled: "\u25A0",
  failed: "\u2715",
  succeeded: "\u2713",
  interrupted_by_app_exit: "\u26A0",
};

const TONE: Record<BuildState, "neutral" | "info" | "success" | "danger" | "warning"> = {
  idle: "neutral",
  preparing: "info",
  checking_prerequisites: "info",
  fetching: "info",
  building: "info",
  packaging: "info",
  verifying: "info",
  running: "info",
  cancelling: "warning",
  cancelled: "neutral",
  failed: "danger",
  succeeded: "success",
  interrupted_by_app_exit: "warning",
};

export function StatusPill({ state }: { state: BuildState }): JSX.Element {
  const tone = TONE[state];
  return (
    <span className={`${styles.pill} ${styles[tone]}`} role="status">
      <span aria-hidden="true" className={styles.glyph}>
        {GLYPHS[state]}
      </span>
      {LABELS[state]}
    </span>
  );
}
