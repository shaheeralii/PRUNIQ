import { AlertBanner } from "../feedback/AlertBanner";
import { StatusBadge } from "../shared/StatusBadge";
import type { BulkOperationPreview, PackageAction } from "../../types/operations";
import styles from "./OperationPreviewDialog.module.css";

interface OperationPreviewDialogProps {
  preview: BulkOperationPreview;
  executing: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function formatActionName(action: PackageAction): string {
  switch (action) {
    case "disable":
      return "Disable (pm disable-user)";
    case "enable":
      return "Enable (pm enable)";
    case "keep_current":
      return "Keep current";
    case "restore_default_state":
      return "Restore default state";
    case "restore_original_state":
      return "Restore original state";
    case "undo_last":
      return "Undo last change";
  }
}

export function OperationPreviewDialog({
  preview,
  executing,
  onCancel,
  onConfirm,
}: OperationPreviewDialogProps) {
  return (
    <div className={styles.backdrop} role="dialog" aria-modal="true" aria-labelledby="op-preview-title">
      <div className={styles.dialog}>
        <header className={styles.header}>
          <h2 id="op-preview-title" className={styles.title}>
            Review & Confirm Package Operations
          </h2>
          <StatusBadge tone="accent">
            User {preview.userId} Â· {preview.serial}
          </StatusBadge>
        </header>

        <div className={styles.body}>
          <p className={styles.meta}>
            Planned: <strong>{preview.executableCount}</strong> executable,{" "}
            <strong>{preview.blockedCount}</strong> blocked,{" "}
            <strong>{preview.noopCount}</strong> no-op. Initial baseline and pre-operation snapshots are captured automatically before modifying each package.
          </p>

          {preview.warnings.length > 0 && (
            <AlertBanner
              tone="warn"
              title="Safety probe warnings"
              items={preview.warnings}
            />
          )}

          <ul className={styles.list}>
            {preview.items.map((item) => (
              <li
                key={`${item.packageName}-${item.action}`}
                className={`${styles.item} ${!item.allowed ? styles.itemBlocked : ""}`}
              >
                <div className={styles.itemHeader}>
                  <span className={styles.mono}>{item.packageName}</span>
                  <div>
                    <StatusBadge
                      tone={
                        item.isNoop
                          ? "neutral"
                          : item.allowed
                            ? "accent"
                            : "error"
                      }
                    >
                      {item.isNoop
                        ? "No-Op"
                        : item.allowed
                          ? formatActionName(item.action)
                          : "Blocked"}
                    </StatusBadge>
                  </div>
                </div>
                {item.messages.map((msg, idx) => (
                  <p key={`${idx}-${msg}`} className={styles.note}>
                    {msg}
                  </p>
                ))}
              </li>
            ))}
          </ul>
        </div>

        <footer className={styles.footer}>
          <button
            type="button"
            className={styles.button}
            disabled={executing}
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={executing || preview.executableCount === 0}
            onClick={onConfirm}
          >
            {executing
              ? "Executing & Verifying..."
              : `Confirm & Execute (${preview.executableCount})`}
          </button>
        </footer>
      </div>
    </div>
  );
}
