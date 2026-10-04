import { Outlet } from "react-router-dom";
import { NavRail } from "@/components/NavRail";
import { TopBar } from "@/components/TopBar";
import { CheckoutWarningModal } from "@/features/projects/CheckoutWarningModal";
import styles from "./AppShell.module.css";

export function AppShell(): JSX.Element {
  return (
    <div className={styles.shell}>
      <NavRail />
      <div className={styles.mainColumn}>
        <TopBar />
        <main className={styles.content}>
          <Outlet />
        </main>
      </div>
      <CheckoutWarningModal />
    </div>
  );
}
