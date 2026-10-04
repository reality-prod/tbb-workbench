import { useState } from "react";
import type { ReactNode } from "react";
import styles from "./Tabs.module.css";

export interface TabDef {
  id: string;
  label: string;
  icon?: ReactNode;
  badge?: ReactNode;
}

export function Tabs({
  tabs,
  active,
  onChange,
}: {
  tabs: TabDef[];
  active: string;
  onChange: (id: string) => void;
}): JSX.Element {
  return (
    <div className={styles.tabList} role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={tab.id === active}
          className={tab.id === active ? `${styles.tab} ${styles.active}` : styles.tab}
          onClick={() => onChange(tab.id)}
        >
          {tab.icon}
          {tab.label}
          {tab.badge}
        </button>
      ))}
    </div>
  );
}

export function useTabs(tabIds: string[], initial?: string): [string, (id: string) => void] {
  const [active, setActive] = useState(initial ?? tabIds[0] ?? "");
  return [active, setActive];
}
