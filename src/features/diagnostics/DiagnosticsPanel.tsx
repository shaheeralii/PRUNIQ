import { useEffect, useState } from "react";
import { AlertBanner } from "../../components/feedback/AlertBanner";
import { IconRefresh } from "../../components/shared/Icons";
import { StatusBadge } from "../../components/shared/StatusBadge";
import { formatKb } from "../../lib/format";
import type {
  MemoryComparison,
  MemorySnapshot,
  PerformanceMetrics,
  StorageDashboard,
} from "../../types/diagnostics";
import styles from "./DiagnosticsPanel.module.css";

interface DiagnosticsPanelProps {
  selectedSerial: string | null;
  snapshots: MemorySnapshot[];
  comparison: MemoryComparison | null;
  storageDashboard: StorageDashboard | null;
  performanceMetrics: PerformanceMetrics | null;
  busy: boolean;
  onCaptureMemory: (label: string) => void;
  onCompareSnapshots: (beforeId: string, afterId: string) => void;
  onRefreshStorage: () => void;
}

function formatSignedKb(delta: number | null): string {
  if (delta === null) {
    return "Unavailable";
  }
  const sign = delta > 0 ? "+" : delta < 0 ? "-" : "";
  return `${sign}${formatKb(Math.abs(delta))}`;
}

function formatMs(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) {
    return "Not measured yet";
  }
  return `${ms} ms`;
}

export function DiagnosticsPanel({
  selectedSerial,
  snapshots,
  comparison,
  storageDashboard,
  performanceMetrics,
  busy,
  onCaptureMemory,
  onCompareSnapshots,
  onRefreshStorage,
}: DiagnosticsPanelProps) {
  const [labelInput, setLabelInput] = useState("");
  const [beforeId, setBeforeId] = useState("");
  const [afterId, setAfterId] = useState("");
  const [processFilter, setProcessFilter] = useState("");

  useEffect(() => {
    if (snapshots.length >= 2) {
      setBeforeId((prev) =>
        snapshots.some((s) => s.snapshotId === prev)
          ? prev
          : snapshots[0].snapshotId,
      );
      setAfterId((prev) =>
        snapshots.some((s) => s.snapshotId === prev)
          ? prev
          : snapshots[snapshots.length - 1].snapshotId,
      );
    } else if (snapshots.length === 1) {
      setBeforeId(snapshots[0].snapshotId);
      setAfterId("");
    } else {
      setBeforeId("");
      setAfterId("");
    }
  }, [snapshots]);

  if (!selectedSerial) {
    return (
      <section className={styles.card}>
        <h2 className={styles.heading}>Memory & Storage Diagnostics</h2>
        <p className={styles.muted}>
          Select an authorized device from Devices & ADB to run read-only RAM snapshots, process inspection, and storage diagnostics.
        </p>
      </section>
    );
  }

  const latestSnapshot =
    snapshots.length > 0 ? snapshots[snapshots.length - 1] : null;
  const procNeedle = processFilter.trim().toLowerCase();
  const visibleTopProcesses = (latestSnapshot?.topProcesses ?? []).filter((p) =>
    procNeedle ? p.processName.toLowerCase().includes(procNeedle) : true,
  );

  return (
    <div className={styles.stack}>
      <section className={styles.card}>
        <div className={styles.toolbar}>
          <div>
            <h2 className={styles.heading}>
              Read-Only RAM Diagnostics & Snapshots
            </h2>
            <p className={styles.muted}>
              Captures <code>/proc/meminfo</code>, <code>dumpsys meminfo</code>, and process counts without killing or force-stopping any processes.
            </p>
          </div>
          <StatusBadge tone="accent">Read-Only</StatusBadge>
        </div>

        <AlertBanner
          tone="info"
          title="Interpreting Android RAM accurately"
          detail="Android intentionally uses free RAM for caching so apps resume faster. Memory usage fluctuates naturally, lower free RAM does not automatically indicate slower performance, and disabling packages does not guarantee higher free RAM."
        />

        <div className={styles.row}>
          <input
            type="text"
            className={styles.input}
            placeholder={`Optional label (e.g., Snapshot ${snapshots.length + 1} — Baseline)`}
            value={labelInput}
            onChange={(e) => setLabelInput(e.target.value)}
            aria-label="Memory snapshot label"
          />
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy}
            onClick={() => {
              const label =
                labelInput.trim() || `Snapshot ${snapshots.length + 1}`;
              onCaptureMemory(label);
              setLabelInput("");
            }}
          >
            Capture Memory Snapshot
          </button>
        </div>

        {latestSnapshot && (
          <>
            <p className={styles.muted}>
              Latest snapshot: <strong>{latestSnapshot.label}</strong> ·{" "}
              {new Date(latestSnapshot.capturedAtMs).toLocaleTimeString()} ·
              probe time {latestSnapshot.captureDurationMs} ms
            </p>

            {latestSnapshot.warnings.length > 0 && (
              <AlertBanner
                tone="warn"
                title="Partial memory probe warnings"
                items={latestSnapshot.warnings}
              />
            )}

            <div className={styles.metricsGrid}>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Total RAM</span>
                <span className={styles.metricValue}>
                  {formatKb(latestSnapshot.totalKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Available RAM</span>
                <span className={styles.metricValue}>
                  {formatKb(latestSnapshot.availableKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Free RAM</span>
                <span className={styles.metricValue}>
                  {formatKb(latestSnapshot.freeKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Cached Memory</span>
                <span className={styles.metricValue}>
                  {formatKb(latestSnapshot.cachedKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Swap (Free / Total)</span>
                <span className={styles.metricValue}>
                  {formatKb(latestSnapshot.swapFreeKb)} /{" "}
                  {formatKb(latestSnapshot.swapTotalKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>Running Processes</span>
                <span className={styles.metricValue}>
                  {latestSnapshot.runningProcessCount ?? "Unavailable"}
                </span>
              </div>
            </div>
          </>
        )}
      </section>

      {snapshots.length >= 2 && (
        <section className={styles.card}>
          <h2 className={styles.heading}>Before / After Memory Comparison</h2>
          <div className={styles.row}>
            <select
              className={styles.select}
              aria-label="Before memory snapshot"
              value={beforeId}
              onChange={(e) => setBeforeId(e.target.value)}
            >
              {snapshots.map((s) => (
                <option key={s.snapshotId} value={s.snapshotId}>
                  Before: {s.label} ({new Date(s.capturedAtMs).toLocaleTimeString()})
                </option>
              ))}
            </select>
            <select
              className={styles.select}
              aria-label="After memory snapshot"
              value={afterId}
              onChange={(e) => setAfterId(e.target.value)}
            >
              {snapshots.map((s) => (
                <option key={s.snapshotId} value={s.snapshotId}>
                  After: {s.label} ({new Date(s.capturedAtMs).toLocaleTimeString()})
                </option>
              ))}
            </select>
            <button
              type="button"
              className={styles.button}
              disabled={busy || !beforeId || !afterId || beforeId === afterId}
              onClick={() => onCompareSnapshots(beforeId, afterId)}
            >
              Compare Snapshots
            </button>
          </div>

          {comparison && (
            <>
              <div className={styles.metricsGrid}>
                <div className={styles.metricBox}>
                  <span className={styles.metricLabel}>Available Delta</span>
                  <span
                    className={`${styles.metricValue} ${
                      comparison.availableDeltaKb !== null &&
                      comparison.availableDeltaKb > 0
                        ? styles.deltaPositive
                        : comparison.availableDeltaKb !== null &&
                            comparison.availableDeltaKb < 0
                          ? styles.deltaNegative
                          : ""
                    }`}
                  >
                    {formatSignedKb(comparison.availableDeltaKb)}
                  </span>
                </div>
                <div className={styles.metricBox}>
                  <span className={styles.metricLabel}>Free Delta</span>
                  <span className={styles.metricValue}>
                    {formatSignedKb(comparison.freeDeltaKb)}
                  </span>
                </div>
                <div className={styles.metricBox}>
                  <span className={styles.metricLabel}>Cached Delta</span>
                  <span className={styles.metricValue}>
                    {formatSignedKb(comparison.cachedDeltaKb)}
                  </span>
                </div>
                <div className={styles.metricBox}>
                  <span className={styles.metricLabel}>Process Count Delta</span>
                  <span className={styles.metricValue}>
                    {comparison.processCountDelta === null
                      ? "Unavailable"
                      : comparison.processCountDelta > 0
                        ? `+${comparison.processCountDelta}`
                        : String(comparison.processCountDelta)}
                  </span>
                </div>
              </div>

              {comparison.topProcessDeltas.length > 0 && (
                <div className={styles.tableWrap}>
                  <table className={styles.table}>
                    <thead>
                      <tr>
                        <th>Process</th>
                        <th>Before PSS</th>
                        <th>After PSS</th>
                        <th>Delta</th>
                      </tr>
                    </thead>
                    <tbody>
                      {comparison.topProcessDeltas.map((d) => (
                        <tr key={d.processName}>
                          <td>{d.processName}</td>
                          <td>{formatKb(d.beforePssKb)}</td>
                          <td>{formatKb(d.afterPssKb)}</td>
                          <td
                            className={
                              d.deltaKb < 0
                                ? styles.deltaPositive
                                : d.deltaKb > 0
                                  ? styles.deltaNegative
                                  : ""
                            }
                          >
                            {formatSignedKb(d.deltaKb)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}

              <ul className={styles.notes}>
                {comparison.interpretationNotes.map((note, idx) => (
                  <li key={`${idx}-${note}`}>{note}</li>
                ))}
              </ul>
            </>
          )}
        </section>
      )}

      {latestSnapshot && latestSnapshot.topProcesses.length > 0 && (
        <section className={styles.card}>
          <div className={styles.toolbar}>
            <h2 className={styles.heading}>
              Background Process Overview (Top PSS)
            </h2>
            <input
              type="search"
              className={styles.input}
              placeholder="Filter processes by name..."
              value={processFilter}
              onChange={(e) => setProcessFilter(e.target.value)}
              aria-label="Filter processes"
            />
          </div>
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Process</th>
                  <th>PID</th>
                  <th>Total PSS</th>
                </tr>
              </thead>
              <tbody>
                {visibleTopProcesses.map((proc) => (
                  <tr key={`${proc.processName}-${proc.pid ?? "none"}`}>
                    <td>{proc.processName}</td>
                    <td>{proc.pid ?? "—"}</td>
                    <td>{formatKb(proc.pssKb)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}

      <section className={styles.card}>
        <div className={styles.toolbar}>
          <div>
            <h2 className={styles.heading}>Storage Diagnostics Dashboard</h2>
            <p className={styles.muted}>
              Inspects internal user storage (<code>/data</code>) and mounted partitions via read-only <code>df -k</code>.
            </p>
          </div>
          <button
            type="button"
            className={styles.button}
            disabled={busy}
            onClick={onRefreshStorage}
          >
            <IconRefresh size={13} />
            <span>
              {storageDashboard ? "Refresh Storage" : "Load Storage Diagnostics"}
            </span>
          </button>
        </div>

        {storageDashboard && (
          <>
            <AlertBanner
              tone="info"
              title="Why disabling preinstalled apps does not shrink /system"
              detail={storageDashboard.disablingStorageNote}
            />
            <p className={styles.muted}>
              {storageDashboard.advancedRemovalNote}
            </p>

            {storageDashboard.warnings.length > 0 && (
              <AlertBanner
                tone="warn"
                title="Storage probe notices"
                items={storageDashboard.warnings}
              />
            )}

            <div className={styles.metricsGrid}>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>/data Total</span>
                <span className={styles.metricValue}>
                  {formatKb(storageDashboard.dataPartition.totalKb)}
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>/data Used</span>
                <span className={styles.metricValue}>
                  {formatKb(storageDashboard.dataPartition.usedKb)} (
                  {storageDashboard.dataPartition.usePercent !== null
                    ? `${storageDashboard.dataPartition.usePercent}%`
                    : "—"}
                  )
                </span>
              </div>
              <div className={styles.metricBox}>
                <span className={styles.metricLabel}>/data Available</span>
                <span className={styles.metricValue}>
                  {formatKb(storageDashboard.dataPartition.availableKb)}
                </span>
              </div>
            </div>

            {storageDashboard.dataPartition.usePercent !== null && (
              <div
                className={styles.progressTrack}
                aria-label="Internal storage usage"
              >
                <div
                  className={styles.progressFill}
                  style={{
                    width: `${Math.min(100, storageDashboard.dataPartition.usePercent)}%`,
                  }}
                />
              </div>
            )}

            {storageDashboard.partitions.length > 0 && (
              <div className={styles.tableWrap}>
                <table className={styles.table}>
                  <thead>
                    <tr>
                      <th>Mount</th>
                      <th>Total</th>
                      <th>Used</th>
                      <th>Available</th>
                      <th>Use %</th>
                      <th>Scope</th>
                    </tr>
                  </thead>
                  <tbody>
                    {storageDashboard.partitions.map((p) => (
                      <tr key={`${p.filesystem}-${p.mountPoint}`}>
                        <td>{p.mountPoint}</td>
                        <td>{formatKb(p.totalKb)}</td>
                        <td>{formatKb(p.usedKb)}</td>
                        <td>{formatKb(p.availableKb)}</td>
                        <td>
                          {p.usePercent !== null ? `${p.usePercent}%` : "—"}
                        </td>
                        <td>
                          {p.isReadOnlySystemPartition
                            ? "Read-only system"
                            : "Writable / user"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </section>

      <section className={styles.card}>
        <h2 className={styles.heading}>Measured Session Performance</h2>
        <p className={styles.muted}>
          Actual measured ADB round-trip durations recorded on this device during the current session.
        </p>
        <div className={styles.metricsGrid}>
          <div className={styles.metricBox}>
            <span className={styles.metricLabel}>Last Bulk Package Scan</span>
            <span className={styles.metricValue}>
              {formatMs(performanceMetrics?.lastPackageScanMs)}
              {performanceMetrics?.lastPackageCount !== null &&
              performanceMetrics?.lastPackageCount !== undefined
                ? ` (${performanceMetrics.lastPackageCount} pkgs)`
                : ""}
            </span>
          </div>
          <div className={styles.metricBox}>
            <span className={styles.metricLabel}>Last Memory Probe</span>
            <span className={styles.metricValue}>
              {formatMs(performanceMetrics?.lastMemoryProbeMs)}
            </span>
          </div>
          <div className={styles.metricBox}>
            <span className={styles.metricLabel}>Last Storage Probe</span>
            <span className={styles.metricValue}>
              {formatMs(performanceMetrics?.lastStorageProbeMs)}
            </span>
          </div>
        </div>
      </section>
    </div>
  );
}
