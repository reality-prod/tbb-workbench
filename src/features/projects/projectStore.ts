import { create } from "zustand";
import {
  getRepositoryStatus,
  openProject,
  type ManagedProject,
  type RepositoryStatus,
} from "@/lib/api";

export type CheckoutValidity =
  | "unknown"
  | "not_a_git_repo"
  | "not_rbm_like"
  | "valid_rbm_checkout";

/** Pure function over the real RepositoryStatus \u2014 never guesses, never
 * invents a status the backend didn't actually report. */
export function classifyCheckout(status: RepositoryStatus | null): CheckoutValidity {
  if (!status) return "unknown";
  if (!status.is_git_repo) return "not_a_git_repo";
  if (!status.looks_like_tor_browser_build) return "not_rbm_like";
  return "valid_rbm_checkout";
}

interface ProjectStoreState {
  currentProject: ManagedProject | null;
  repositoryStatus: RepositoryStatus | null;
  isLoading: boolean;
  error: string | null;
  /** Set to true exactly once per `openProjectAtPath` call that resulted in
   * a non-ideal checkout, so the UI can show a one-time modal rather than
   * nagging on every re-render. Cleared by `acknowledgeCheckoutWarning`. */
  pendingCheckoutWarning: CheckoutValidity | null;
  openProjectAtPath: (path: string) => Promise<void>;
  refreshRepositoryStatus: () => Promise<void>;
  acknowledgeCheckoutWarning: () => void;
}

export const useProjectStore = create<ProjectStoreState>((set, get) => ({
  currentProject: null,
  repositoryStatus: null,
  isLoading: false,
  error: null,
  pendingCheckoutWarning: null,

  openProjectAtPath: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      const project = await openProject(path);
      const status = await getRepositoryStatus(project.id);
      const validity = classifyCheckout(status);
      set({
        currentProject: project,
        repositoryStatus: status,
        isLoading: false,
        pendingCheckoutWarning: validity === "valid_rbm_checkout" ? null : validity,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  refreshRepositoryStatus: async () => {
    const project = get().currentProject;
    if (!project) return;
    try {
      const status = await getRepositoryStatus(project.id);
      set({ repositoryStatus: status });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  acknowledgeCheckoutWarning: () => set({ pendingCheckoutWarning: null }),
}));
