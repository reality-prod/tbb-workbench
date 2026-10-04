import { forwardRef } from "react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import styles from "./Button.module.css";

type Variant = "primary" | "secondary" | "outline" | "danger" | "ghost";
type Size = "sm" | "md";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: ReactNode;
  iconTrailing?: ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", icon, iconTrailing, className, children, ...rest },
  ref,
) {
  const classes = [styles.btn, styles[variant], styles[size], className].filter(Boolean).join(" ");
  return (
    <button ref={ref} className={classes} {...rest}>
      {icon && <span className={styles.iconSlot}>{icon}</span>}
      {children}
      {iconTrailing && <span className={styles.iconSlot}>{iconTrailing}</span>}
    </button>
  );
});
