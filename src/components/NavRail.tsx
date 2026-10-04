import { NavLink } from "react-router-dom";
import {
  SquaresFour,
  FolderOpen,
  Hammer,
  ListBullets,
  FileCode,
  Stethoscope,
  Package,
  GearSix,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import styles from "./NavRail.module.css";

const ROUTES: Array<{ to: string; label: string; icon: ReactNode }> = [
  { to: "/", label: "Overview", icon: <SquaresFour size={20} /> },
  { to: "/projects", label: "Projects", icon: <FolderOpen size={20} /> },
  { to: "/build", label: "Build", icon: <Hammer size={20} /> },
  { to: "/logs", label: "Logs", icon: <ListBullets size={20} /> },
  { to: "/editor", label: "Config Editor", icon: <FileCode size={20} /> },
  { to: "/diagnostics", label: "Diagnostics", icon: <Stethoscope size={20} /> },
  { to: "/artifacts", label: "Artifacts", icon: <Package size={20} /> },
  { to: "/settings", label: "Settings", icon: <GearSix size={20} /> },
];

export function NavRail(): JSX.Element {
  return (
    <nav className={styles.rail} aria-label="Primary">
      <div className={styles.brand}>
        <span className={styles.brandMark} aria-hidden="true" />
        <span className={styles.brandName}>TBB Workbench</span>
      </div>
      <ul className={styles.list}>
        {ROUTES.map((route) => (
          <li key={route.to}>
            <NavLink
              to={route.to}
              className={({ isActive }) =>
                isActive ? `${styles.link} ${styles.active}` : styles.link
              }
              end={route.to === "/"}
            >
              <span className={styles.icon} aria-hidden="true">
                {route.icon}
              </span>
              {route.label}
            </NavLink>
          </li>
        ))}
      </ul>
      <p className={styles.disclaimer}>
        Not affiliated with or endorsed by The Tor Project.
      </p>
    </nav>
  );
}
