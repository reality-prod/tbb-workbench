import { Warning, FolderOpen, GitBranch } from "@phosphor-icons/react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Alert } from "@/components/ui/Alert";
import { useNavigate } from "react-router-dom";
import { useProjectStore, type CheckoutValidity } from "./projectStore";
import type { RepositoryStatus } from "@/lib/api";

const ALL_MARKERS = [
  "rbm.conf", "rbm", "projects", "config", "conf", "tools", "keyring", "container", "Makefile",
];

function Body({
  validity,
  status,
}: {
  validity: CheckoutValidity;
  status: RepositoryStatus;
}): JSX.Element {
  if (validity === "not_a_git_repo") {
    return (
      <>
        <Alert tone="danger" title="Not a Git checkout">
          {status.validation_errors.join(" ") ||
            "No .git directory was found at this path."}
        </Alert>
        <p>
          You can still browse and edit files here, but repository features
          (branch/commit info, diffs, git clone-based workflows) won&apos;t
          work until you open an actual Git checkout.
        </p>
      </>
    );
  }

  // not_rbm_like: it IS a real git repo, we just didn't find the specific
  // markers a tor-browser-build checkout has. Show exactly what we looked
  // for and what we actually found — real data, not a canned message.
  const found = new Set(status.detected_markers);
  return (
    <>
      <Alert tone="warning" title="Doesn't look like a tor-browser-build checkout">
        This is a valid Git repository, but none of the usual tor-browser-build
        markers were found together at this path.
      </Alert>
      <div>
        <p style={{ marginBottom: 8, color: "var(--text-secondary)", fontSize: "var(--text-body-tiny)" }}>
          Checked for (found markers in purple):
        </p>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {ALL_MARKERS.map((m) => (
            <span
              key={m}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: "var(--text-label-tiny)",
                padding: "2px 8px",
                borderRadius: "var(--radius-pill)",
                background: found.has(m) ? "var(--accent-soft)" : "var(--bg-subtle)",
                color: found.has(m) ? "var(--accent-primary)" : "var(--text-secondary)",
                fontWeight: found.has(m) ? 600 : 400,
              }}
            >
              {m}
            </span>
          ))}
        </div>
      </div>
      <p>
        Build presets, Makefile target discovery, and the RBM inspector
        assume this layout and may show nothing useful here. File editing,
        git status, and diffs still work normally.
      </p>
    </>
  );
}

export function CheckoutWarningModal(): JSX.Element | null {
  const navigate = useNavigate();
  const { pendingCheckoutWarning, repositoryStatus, acknowledgeCheckoutWarning, currentProject } =
    useProjectStore();

  if (!pendingCheckoutWarning || !repositoryStatus || !currentProject) return null;

  const title =
    pendingCheckoutWarning === "not_a_git_repo"
      ? "Not a Git repository"
      : "Checkout not recognized";

  return (
    <Modal
      title={title}
      onClose={acknowledgeCheckoutWarning}
      footer={
        <>
          <Button
            variant="secondary"
            icon={<FolderOpen size={14} />}
            onClick={() => {
              acknowledgeCheckoutWarning();
              navigate("/");
            }}
          >
            Choose a different folder
          </Button>
          <Button
            variant="primary"
            icon={pendingCheckoutWarning === "not_a_git_repo" ? <Warning size={14} /> : <GitBranch size={14} />}
            onClick={acknowledgeCheckoutWarning}
          >
            Continue anyway
          </Button>
        </>
      }
    >
      <Body validity={pendingCheckoutWarning} status={repositoryStatus} />
    </Modal>
  );
}
