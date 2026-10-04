import { useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import styles from "./ConfirmDangerButton.module.css";

/**
 * Destructive-action confirmation requiring the user to type the exact
 * repository/resource name, per the "advanced cleanup tool" requirement:
 * confirmation requiring the user to type the repository name.
 */
export function ConfirmDangerButton({
  label,
  confirmWord,
  description,
  fileList,
  onConfirm,
  buttonLabel,
}: {
  label: string;
  confirmWord: string;
  description: string;
  fileList?: string[];
  onConfirm: () => void;
  buttonLabel: string;
}): JSX.Element {
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState("");
  const matches = typed === confirmWord;

  return (
    <>
      <Button variant="danger" onClick={() => setOpen(true)}>
        {buttonLabel}
      </Button>
      {open && (
        <Modal
          title={label}
          onClose={() => {
            setOpen(false);
            setTyped("");
          }}
          footer={
            <>
              <Button variant="secondary" onClick={() => setOpen(false)}>
                Cancel
              </Button>
              <Button
                variant="danger"
                disabled={!matches}
                onClick={() => {
                  onConfirm();
                  setOpen(false);
                  setTyped("");
                }}
              >
                {buttonLabel}
              </Button>
            </>
          }
        >
          <p>{description}</p>
          {fileList && fileList.length > 0 && (
            <ul className={styles.fileList}>
              {fileList.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          )}
          <label className={styles.confirmLabel}>
            Type <strong>{confirmWord}</strong> to confirm
            <input
              className={styles.confirmInput}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              autoFocus
            />
          </label>
        </Modal>
      )}
    </>
  );
}
