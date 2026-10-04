import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { useSettingsStore } from "@/features/settings/settingsStore";
import { reconcileInterruptedRuns } from "@/lib/api";
import "./styles/global.css";

void useSettingsStore.getState().load();
void reconcileInterruptedRuns();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
