import { create } from "zustand";
import { getSettings, updateSettings, type ApplicationSettings, type ThemePreference } from "@/lib/api";

interface SettingsStoreState {
  settings: ApplicationSettings | null;
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (next: ApplicationSettings) => Promise<void>;
  setTheme: (theme: ThemePreference) => Promise<void>;
  effectiveTheme: "light" | "dark";
}

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function resolveEffective(theme: ThemePreference | undefined): "light" | "dark" {
  if (theme === "dark") return "dark";
  if (theme === "light") return "light";
  return systemPrefersDark() ? "dark" : "light";
}

function applyThemeToDocument(effective: "light" | "dark"): void {
  document.documentElement.dataset.theme = effective;
}

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  settings: null,
  loaded: false,
  error: null,
  effectiveTheme: resolveEffective(undefined),

  load: async () => {
    try {
      const settings = await getSettings();
      const effective = resolveEffective(settings.theme);
      applyThemeToDocument(effective);
      set({ settings, loaded: true, effectiveTheme: effective });
    } catch (err) {
      set({ error: String(err), loaded: true });
    }
  },

  save: async (next: ApplicationSettings) => {
    try {
      await updateSettings(next);
      const effective = resolveEffective(next.theme);
      applyThemeToDocument(effective);
      set({ settings: next, effectiveTheme: effective });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setTheme: async (theme: ThemePreference) => {
    const current = get().settings;
    if (!current) return;
    await get().save({ ...current, theme });
  },
}));
