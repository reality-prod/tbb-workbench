import type { HTMLAttributes, ReactNode } from "react";
import styles from "./Card.module.css";

interface CardProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  title?: ReactNode;
  icon?: ReactNode;
  actions?: ReactNode;
  tone?: "default" | "warning" | "danger";
}

export function Card({ title, icon, actions, tone = "default", className, children, ...rest }: CardProps): JSX.Element {
  const classes = [styles.card, styles[tone], className].filter(Boolean).join(" ");
  return (
    <div className={classes} {...rest}>
      {(title || actions) && (
        <div className={styles.header}>
          <div className={styles.titleRow}>
            {icon && <span className={styles.icon}>{icon}</span>}
            {title && <h3 className={styles.title}>{title}</h3>}
          </div>
          {actions && <div className={styles.actions}>{actions}</div>}
        </div>
      )}
      <div className={styles.body}>{children}</div>
    </div>
  );
}
