import type { ReactNode } from "react";
import styles from "./Badge.module.css";

type Tone = "neutral" | "info" | "success" | "warning" | "danger" | "accent";

export function Badge({ tone = "neutral", children }: { tone?: Tone; children: ReactNode }): JSX.Element {
  return <span className={`${styles.badge} ${styles[tone]}`}>{children}</span>;
}
