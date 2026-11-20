import { useState } from "react";
import { AlertBanner } from "../../components/feedback/AlertBanner";
import { IconRefresh } from "../../components/shared/Icons";
import { StatusBadge, type BadgeTone } from "../../components/shared/StatusBadge";
import { formatConfiguredState, formatEffectiveState } from "../../lib/format";
import type {
  OperationStatus,
  PackageActionRequest,
  PackageOperationResult,
} from "../../types/operations";
import styles from "./HistoryPanel.module.css";

interface HistoryPanelProps {
  selectedSerial: string | null;
  selectedUserId: number | null;
  history: PackageOperationResult[];
  statusFilter: string;
  searchQuery: string;
  scopeAllDevices: boolean;
  busy: boolean;
  reconcilingId: string | null;
  onStatusFilterChange: (value: string) => void;
  onSearchChange: (value: string) => void;
  onScopeChange: (allDevices: boolean) => void;
  onRefresh: () => void;
  onReconcile: (operationId: string) => void;
  onRequestPackageAction: (items: PackageActionRequest[]) => void;
  onClearHistory: () => void;
}

function statusTone(status: OperationStatus): BadgeTone {
  switch (status) {
    case "succeeded":
      return "ok";
    case "no_op":
      return "neutral";
    case "uncertain_timeout":
    case "blocked_by_protection":
    case "unsupported":
      return "warn";
    case "failed":
    case "verification_mismatch":
      return "error";
  }
}

function formatTimestamp(ms: number): string {
  if (!ms) {
    return "Unknown time";
  }
  return new Date(ms).toLocaleString();
}

export function HistoryPanel({
  selectedSerial,
  selectedUserId,
  history,
  statusFilter,
  searchQuery,
  scopeAllDevices,
  busy,
  reconcilingId,
  onStatusFilterChange,
  onSearchChange,
  onScopeChange,
  onRefresh,
  onReconcile,
  onRequestPackageAction,
  onClearHistory,
}: HistoryPanelProps) {
  const [confirmClear, setConfirmClear] = useState(false);

  const uncertainCount = history.filter(
    (h) => h.status === "uncertain_timeout",
  ).length;

  return (
    <div className={styles.stack}>
      <section className={styles.card}>
        <div className={styles.toolbar}>
          <div>
            <h2 className={styles.heading}>Operation History & Recovery</h2>
            <p className={styles.muted}>
              Persisted locally in SQLite without raw hardware serials or raw shell strings. Clearing history preserves immutable Initial Toolkit Baselines.
            </p>
          </div>
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.button}
              disabled={busy}
              onClick={onRefresh}
            >
              <IconRefresh size={13} />
              <span>Refresh</span>
            </button>
            {!confirmClear ? (
              <button
                type="button"
                className={`${styles.button} ${styles.buttonDanger}`}
                disabled={busy || history.length === 0}
                onClick={() => setConfirmClear(true)}
              >
                Clear History
              </button>
            ) : (
              <>
                <button
                  type="button"
                  className={`${styles.button} ${styles.buttonDanger}`}
                  disabled={busy}
                  onClick={() => {
                    setConfirmClear(false);
                    onClearHistory();
                  }}
                >
                  Confirm Clear
                </button>
                <button
                  type="button"
                  className={styles.button}
                  onClick={() => setConfirmClear(false)}
                >
                  Cancel
                </button>
              </>
            )}
          </div>
        </div>

        <div className={styles.filters}>
          <input
            type="search"
            className={styles.input}
            placeholder="Filter history by package name or note..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            aria-label="Search operation history"
          />

          <select
            className={styles.select}
            aria-label="Filter by operation status"
            value={statusFilter}
            onChange={(e) => onStatusFilterChange(e.target.value)}
          >
            <option value="all">All statuses</option>
            <option value="uncertain_timeout">Uncertain / Timed out</option>
            <option value="verification_mismatch">Verification mismatch</option>
            <option value="succeeded">Succeeded</option>
            <option value="blocked_by_protection">Blocked by protection</option>
            <option value="failed">Failed</option>
          </select>

          <select
            className={styles.select}
            aria-label="History device scope"
            value={scopeAllDevices ? "all" : "current"}
            onChange={(e) => onScopeChange(e.target.value === "all")}
          >
            <option value="current" disabled={!selectedSerial}>
              Current device
            </option>
            <option value="all">All persisted devices</option>
          </select>
        </div>
      </section>

      {uncertainCount > 0 && (
        <AlertBanner
          tone="warn"
          title={`${uncertainCount} operation(s) require live-state verification`}
          detail="A timeout or USB disconnect occurred while an operation was in flight. Connect the device and click 'Verify Live State' to inspect the actual state without automatically retrying mutations."
        />
      )}

      <section className={styles.card}>
        {history.length === 0 ? (
          <p className={styles.muted}>
            No persisted operation history matches the current filters.
          </p>
        ) : (
          <ul className={styles.list}>
            {history.map((op) => {
              const needsVerify =
                op.status === "uncertain_timeout" ||
                op.status === "verification_mismatch";
              const canOperateOnSelected =
                selectedSerial !== null &&
                selectedUserId !== null &&
                selectedUserId === op.userId;

              return (
                <li
                  key={op.operationId}
                  className={`${styles.item} ${
                    op.status === "uncertain_timeout" ? styles.itemUncertain : ""
                  }`}
                >
                  <div className={styles.row}>
                    <div>
                      <span className={styles.mono}>{op.packageName}</span>{" "}
                      <span className={styles.muted}>
                        · {op.action} · User {op.userId} ·{" "}
                        {formatTimestamp(op.executedAtMs)}
                      </span>
                    </div>
                    <div className={styles.actions}>
                      <StatusBadge tone={statusTone(op.status)}>
                        {op.status}
                      </StatusBadge>
                      {op.restorationFidelity !== "not_applicable" && (
                        <StatusBadge
                          tone={
                            op.restorationFidelity === "exact" ? "ok" : "warn"
                          }
                        >
                          {op.restorationFidelity}
                        </StatusBadge>
                      )}
                    </div>
                  </div>

                  {op.previousState && (
                    <div className={styles.stateTransition}>
                      <span className={styles.stateChip}>
                        {formatEffectiveState(op.previousState.effectiveState)} (
                        {formatConfiguredState(
                          op.previousState.configuredState,
                          op.previousState.configuredStateCode,
                        )}
                        )
                      </span>
                      <span aria-hidden="true">→</span>
                      <span className={styles.stateChip}>
                        {op.verifiedState
                          ? `${formatEffectiveState(
                              op.verifiedState.effectiveState,
                            )} (${formatConfiguredState(
                              op.verifiedState.configuredState,
                              op.verifiedState.configuredStateCode,
                            )})`
                          : "Verification pending"}
                      </span>
                    </div>
                  )}

                  {op.limitationNote && (
                    <p className={styles.muted}>{op.limitationNote}</p>
                  )}
                  {op.errorMessage && (
                    <p className={styles.muted}>Error: {op.errorMessage}</p>
                  )}

                  <div className={styles.actions}>
                    {needsVerify && (
                      <button
                        type="button"
                        className={`${styles.button} ${styles.buttonPrimary}`}
                        disabled={
                          busy ||
                          !selectedSerial ||
                          reconcilingId === op.operationId
                        }
                        onClick={() => onReconcile(op.operationId)}
                      >
                        {reconcilingId === op.operationId
                          ? "Verifying..."
                          : "Verify Live State"}
                      </button>
                    )}
                    {canOperateOnSelected && (
                      <>
                        <button
                          type="button"
                          className={styles.button}
                          disabled={busy}
                          onClick={() =>
                            onRequestPackageAction([
                              {
                                packageName: op.packageName,
                                action: "undo_last",
                              },
                            ])
                          }
                        >
                          Undo Last
                        </button>
                        <button
                          type="button"
                          className={styles.button}
                          disabled={busy}
                          onClick={() =>
                            onRequestPackageAction([
                              {
                                packageName: op.packageName,
                                action: "restore_original_state",
                              },
                            ])
                          }
                        >
                          Restore Original
                        </button>
                      </>
                    )}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
