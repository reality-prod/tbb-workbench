import { lazy, Suspense } from "react";
import { HashRouter, Route, Routes } from "react-router-dom";
import { AppShell } from "@/app/AppShell";
import { OnboardingPage } from "@/routes/OnboardingPage";
import { ClonePage } from "@/routes/ClonePage";
import { DashboardPage } from "@/routes/DashboardPage";
import { BuildPage } from "@/routes/BuildPage";
import { DiagnosticsPage } from "@/routes/DiagnosticsPage";
import { LogsPage } from "@/routes/LogsPage";
import { ArtifactsPage } from "@/routes/ArtifactsPage";
import { SettingsPage } from "@/routes/SettingsPage";

// The Config Editor pulls in CodeMirror + legacy-modes, which is the bulk
// of the app's JS weight. Splitting it into its own chunk means every other
// screen (including first launch) loads without paying for it.
const EditorPage = lazy(() =>
  import("@/routes/EditorPage").then((m) => ({ default: m.EditorPage })),
);

function RouteFallback(): JSX.Element {
  return <p style={{ color: "var(--text-secondary)" }}>Loading…</p>;
}

export function App(): JSX.Element {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<OnboardingPage />} />
          <Route path="/clone" element={<ClonePage />} />
          <Route path="/projects" element={<DashboardPage />} />
          <Route path="/build" element={<BuildPage />} />
          <Route path="/logs" element={<LogsPage />} />
          <Route
            path="/editor"
            element={
              <Suspense fallback={<RouteFallback />}>
                <EditorPage />
              </Suspense>
            }
          />
          <Route path="/diagnostics" element={<DiagnosticsPage />} />
          <Route path="/artifacts" element={<ArtifactsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}
