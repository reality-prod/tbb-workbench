import type { ReactNode } from "react";
import { Info, Warning, XCircle, CheckCircle } from "@phosphor-icons/react";
import styles from "./Alert.module.css";

type Tone = "info" | "warning" | "danger" | "success";

const ICONS: Record<Tone, ReactNode> = {
  info: <Info size={20} weight="regular" />,
  warning: <Warning size={20} weight="regular" />,
  danger: <XCircle size={20} weight="regular" />,
  success: <CheckCircle size={20} weight="regular" />,
};

export function Alert({
  tone = "info",
  title,
  children,
}: {
  tone?: Tone;
  title?: string;
  children: ReactNode;
}): JSX.Element {
  return (
    <div className={`${styles.alert} ${styles[tone]}`} role={tone === "danger" ? "alert" : "status"}>
      <span className={styles.icon}>{ICONS[tone]}</span>
      <div>
        {title && <p className={styles.title}>{title}</p>}
        <div className={styles.body}>{children}</div>
      </div>
    </div>
  );
}
