import { useMemo, useState } from "react";
import { AlertBanner } from "../../components/feedback/AlertBanner";
import { StatusBadge, type BadgeTone } from "../../components/shared/StatusBadge";
import {
  filterInstalledPackages,
  formatConfidence,
  formatConfiguredState,
  formatEffectiveState,
  formatMatchScope,
  formatOrigin,
  formatPartition,
  formatRisk,
  formatVerificationStatus,
  riskTone,
  type ClientPackageFilter,
} from "../../lib/format";
import type {
  BulkOperationResult,
  DeviceProtectionProfile,
  OperationStatus,
  PackageAction,
  PackageActionRequest,
  PackageSnapshotStatus,
  RestorationFidelity,
} from "../../types/operations";
import type {
  EffectivePackageState,
  InstallPartition,
  InstalledPackage,
  PackageDetails,
  PackageInventory,
  PackageOrigin,
} from "../../types/packages";
import type {
  PackageRecommendation,
  RiskClassification,
  RuleDatabaseSummary,
} from "../../types/rules";
import styles from "./AppManagerPanel.module.css";

interface AppManagerPanelProps {
  selectedSerial: string | null;
  selectedUserId: number | null;
  defaultStateSupported: boolean;
  inventory: PackageInventory | null;
  ruleDbSummary: RuleDatabaseSummary | null;
  recommendations: Record<string, PackageRecommendation>;
  protections: DeviceProtectionProfile | null;
  snapshots: Record<string, PackageSnapshotStatus>;
  lastOperationResult: BulkOperationResult | null;
  selectedPackageName: string | null;
  packageDetails: PackageDetails | null;
  busy: boolean;
  inspectingPackage: boolean;
  onScanPackages: () => void;
  onSelectPackage: (pkg: InstalledPackage) => void;
  onInspectPackageDetails: (packageName: string) => void;
  onRequestOperations: (items: PackageActionRequest[]) => void;
  onDismissLastResult: () => void;
}

const DEFAULT_FILTER: ClientPackageFilter = {
  search: "",
  origin: "all",
  state: "all",
  partition: "all",
  risk: "all",
  updatedSystemOnly: false,
};

function isProtectedInProfile(
  packageName: string,
  profile: DeviceProtectionProfile | null,
): boolean {
  if (!profile) {
    return false;
  }
  if (profile.immutablePackages.includes(packageName)) {
    return true;
  }
  if (profile.activeLauncher === packageName) {
    return true;
  }
  if (profile.usableLaunchers.length === 1 && profile.usableLaunchers[0] === packageName) {
    return true;
  }
  if (profile.activeKeyboard === packageName) {
    return true;
  }
  if (profile.enabledKeyboards.length === 1 && profile.enabledKeyboards[0] === packageName) {
    return true;
  }
  return false;
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

function formatFidelity(fidelity: RestorationFidelity): string {
  switch (fidelity) {
    case "not_applicable":
      return "N/A";
    case "exact":
      return "Exact restoration";
    case "effective_match_only":
      return "Effective state match only";
    case "unverified":
      return "Unverified";
    case "failed":
      return "Failed";
  }
}

export function AppManagerPanel({
  selectedSerial,
  selectedUserId,
  defaultStateSupported,
  inventory,
  ruleDbSummary,
  recommendations,
  protections,
  snapshots,
  lastOperationResult,
  selectedPackageName,
  packageDetails,
  busy,
  inspectingPackage,
  onScanPackages,
  onSelectPackage,
  onInspectPackageDetails,
  onRequestOperations,
  onDismissLastResult,
}: AppManagerPanelProps) {
  const [filter, setFilter] = useState<ClientPackageFilter>(DEFAULT_FILTER);
  const [checkedPackages, setCheckedPackages] = useState<string[]>([]);

  const filteredPackages = useMemo(() => {
    if (!inventory) {
      return [];
    }
    return filterInstalledPackages(inventory.packages, filter, recommendations);
  }, [inventory, filter, recommendations]);

  const checkedSet = useMemo(() => new Set(checkedPackages), [checkedPackages]);

  const selectedPackage = useMemo(() => {
    if (!inventory || !selectedPackageName) {
      return null;
    }
    return (
      inventory.packages.find((p) => p.packageName === selectedPackageName) ?? null
    );
  }, [inventory, selectedPackageName]);

  const selectedRecommendation = selectedPackage
    ? recommendations[selectedPackage.packageName] ?? null
    : null;
  const selectedSnapshotStatus = selectedPackage
    ? snapshots[selectedPackage.packageName] ?? null
    : null;
  const selectedProtected = selectedPackage
    ? isProtectedInProfile(selectedPackage.packageName, protections)
    : false;

  if (!selectedSerial) {
    return (
      <section className={styles.card}>
        <h2 className={styles.heading}>Installed Applications</h2>
        <p className={styles.muted}>
          Select an authorized device from the Devices & ADB tab before scanning or managing packages.
        </p>
      </section>
    );
  }

  const canMutate = selectedUserId !== null && !busy;

  const toggleChecked = (packageName: string) => {
    setCheckedPackages((prev) =>
      prev.includes(packageName)
        ? prev.filter((p) => p !== packageName)
        : [...prev, packageName],
    );
  };

  const triggerBulkAction = (action: PackageAction) => {
    if (checkedPackages.length === 0) {
      return;
    }
    onRequestOperations(
      checkedPackages.map((packageName) => ({ packageName, action })),
    );
  };

  const triggerSingleAction = (packageName: string, action: PackageAction) => {
    onRequestOperations([{ packageName, action }]);
  };

  const hasActiveFilters =
    filter.search.trim().length > 0 ||
    filter.origin !== "all" ||
    filter.state !== "all" ||
    filter.partition !== "all" ||
    filter.risk !== "all" ||
    filter.updatedSystemOnly;

  return (
    <div className={styles.stack}>
      <section className={styles.card}>
        <div className={styles.toolbar}>
          <div>
            <h2 className={styles.heading}>Installed Application Inventory</h2>
            <p className={styles.muted}>
              Target: <strong>{selectedSerial}</strong> Â· User scope:{" "}
              <strong>
                {selectedUserId !== null ? `User ${selectedUserId}` : "Not selected"}
              </strong>
            </p>
          </div>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy}
            onClick={() => {
              setCheckedPackages([]);
              onScanPackages();
            }}
          >
            {inventory ? "Rescan Packages" : "Scan Packages"}
          </button>
        </div>

        {selectedUserId === null && (
          <AlertBanner
            tone="warn"
            title="Explicit Android User ID required for package management"
            detail="Select a specific Android user profile in the Device Info & Capabilities tab before running Disable, Enable, or Restore operations. ADT never silently assumes User 0."
          />
        )}

        {ruleDbSummary && (
          <div className={styles.ruleDbBar}>
            Offline recommendation database (<strong>{ruleDbSummary.databaseVersion}</strong>):{" "}
            <strong>{ruleDbSummary.activeRules}</strong> active rules (
            <strong>{ruleDbSummary.verifiedRules}</strong> verified). Active protections:{" "}
            launcher <strong>{protections?.activeLauncher ?? "unverified"}</strong>, keyboard{" "}
            <strong>{protections?.activeKeyboard ?? "unverified"}</strong>.
          </div>
        )}

        {inventory && (
          <>
            <div className={styles.statsRow}>
              <span>
                Showing <strong>{filteredPackages.length}</strong> of{" "}
                <strong>{inventory.totalCount}</strong> packages
              </span>
              <span>
                Enabled: <strong>{inventory.enabledCount}</strong>
              </span>
              <span>
                Disabled: <strong>{inventory.disabledCount}</strong>
              </span>
              <span>
                System: <strong>{inventory.systemCount}</strong>
              </span>
              <span>
                Third-Party: <strong>{inventory.thirdPartyCount}</strong>
              </span>
            </div>

            <div className={styles.filtersRow}>
              <input
                type="search"
                className={styles.searchInput}
                placeholder="Search by label or package identifier..."
                value={filter.search}
                onChange={(e) =>
                  setFilter((prev) => ({ ...prev, search: e.target.value }))
                }
                aria-label="Search installed packages"
              />

              <select
                className={styles.select}
                aria-label="Filter by recommendation risk"
                value={filter.risk}
                onChange={(e) =>
                  setFilter((prev) => ({
                    ...prev,
                    risk: e.target.value as RiskClassification | "all",
                  }))
                }
              >
                <option value="all">All risks</option>
                <option value="recommended_to_disable">Recommended to Disable</option>
                <option value="caution">Caution</option>
                <option value="do_not_disable">Do Not Disable</option>
                <option value="unknown_unverified">Unknown / Unverified</option>
              </select>

              <select
                className={styles.select}
                aria-label="Filter by origin"
                value={filter.origin}
                onChange={(e) =>
                  setFilter((prev) => ({
                    ...prev,
                    origin: e.target.value as PackageOrigin | "all",
                  }))
                }
              >
                <option value="all">All origins</option>
                <option value="system">System</option>
                <option value="oem">OEM</option>
                <option value="google">Google</option>
                <option value="third_party">Third-Party</option>
                <option value="unknown">Unknown</option>
              </select>

              <select
                className={styles.select}
                aria-label="Filter by effective state"
                value={filter.state}
                onChange={(e) =>
                  setFilter((prev) => ({
                    ...prev,
                    state: e.target.value as EffectivePackageState | "all",
                  }))
                }
              >
                <option value="all">All states</option>
                <option value="enabled">Enabled</option>
                <option value="disabled">Disabled</option>
                <option value="unknown">Unknown</option>
              </select>

              <select
                className={styles.select}
                aria-label="Filter by partition"
                value={filter.partition}
                onChange={(e) =>
                  setFilter((prev) => ({
                    ...prev,
                    partition: e.target.value as InstallPartition | "all",
                  }))
                }
              >
                <option value="all">All partitions</option>
                <option value="system">/system</option>
                <option value="system_ext">/system_ext</option>
                <option value="product">/product</option>
                <option value="vendor">/vendor</option>
                <option value="odm">/odm</option>
                <option value="apex">/apex</option>
                <option value="data">/data</option>
                <option value="other">Other</option>
                <option value="unknown">Unknown</option>
              </select>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={filter.updatedSystemOnly}
                  onChange={(e) =>
                    setFilter((prev) => ({
                      ...prev,
                      updatedSystemOnly: e.target.checked,
                    }))
                  }
                />
                Updated system only
              </label>

              {hasActiveFilters && (
                <button
                  type="button"
                  className={styles.button}
                  onClick={() => setFilter(DEFAULT_FILTER)}
                >
                  Clear Filters
                </button>
              )}
            </div>

            {checkedPackages.length > 0 && (
              <div className={styles.bulkBar}>
                <span>
                  <strong>{checkedPackages.length}</strong> package(s) selected for bulk preview
                </span>
                <div className={styles.actionRow}>
                  <button
                    type="button"
                    className={`${styles.button} ${styles.buttonDanger}`}
                    disabled={!canMutate}
                    onClick={() => triggerBulkAction("disable")}
                  >
                    Disable Selected
                  </button>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={!canMutate}
                    onClick={() => triggerBulkAction("enable")}
                  >
                    Enable Selected
                  </button>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={!canMutate}
                    onClick={() => triggerBulkAction("restore_original_state")}
                  >
                    Restore Original
                  </button>
                  {defaultStateSupported && (
                    <button
                      type="button"
                      className={styles.button}
                      disabled={!canMutate}
                      onClick={() => triggerBulkAction("restore_default_state")}
                    >
                      Restore Default
                    </button>
                  )}
                  <button
                    type="button"
                    className={styles.button}
                    onClick={() => setCheckedPackages([])}
                  >
                    Clear Selection
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </section>

      {lastOperationResult && (
        <section className={styles.card}>
          <div className={styles.toolbar}>
            <h3 className={styles.heading}>
              Last Operation Results ({lastOperationResult.succeededCount} succeeded,{" "}
              {lastOperationResult.blockedCount} blocked,{" "}
              {lastOperationResult.uncertainCount} uncertain,{" "}
              {lastOperationResult.failedCount} failed)
            </h3>
            <button
              type="button"
              className={styles.button}
              onClick={onDismissLastResult}
            >
              Dismiss
            </button>
          </div>

          {lastOperationResult.abortedEarlyReason && (
            <AlertBanner
              tone="error"
              title="Bulk execution stopped early"
              detail={lastOperationResult.abortedEarlyReason}
            />
          )}

          <ul className={styles.resultsList}>
            {lastOperationResult.results.map((r) => (
              <li key={r.operationId} className={styles.resultItem}>
                <div className={styles.toolbar}>
                  <strong>{r.packageName}</strong>
                  <div className={styles.actionRow}>
                    <StatusBadge tone={statusTone(r.status)}>{r.status}</StatusBadge>
                    {r.restorationFidelity !== "not_applicable" && (
                      <StatusBadge
                        tone={r.restorationFidelity === "exact" ? "ok" : "warn"}
                      >
                        {formatFidelity(r.restorationFidelity)}
                      </StatusBadge>
                    )}
                  </div>
                </div>
                {r.verifiedState && (
                  <p className={styles.muted}>
                    Verified live state:{" "}
                    {formatEffectiveState(r.verifiedState.effectiveState)} Â·{" "}
                    {formatConfiguredState(
                      r.verifiedState.configuredState,
                      r.verifiedState.configuredStateCode,
                    )}
                  </p>
                )}
                {r.limitationNote && <p className={styles.muted}>{r.limitationNote}</p>}
                {r.errorMessage && <p className={styles.muted}>{r.errorMessage}</p>}
              </li>
            ))}
          </ul>
        </section>
      )}

      {inventory && inventory.warnings.length > 0 && (
        <AlertBanner
          tone="warn"
          title="Bulk package scan notices"
          items={inventory.warnings}
        />
      )}

      {!inventory ? (
        <section className={styles.card}>
          <p className={styles.muted}>
            Click <strong>Scan Packages</strong> to enumerate installed packages, evaluate offline recommendations, and enable guarded package controls.
          </p>
        </section>
      ) : (
        <div className={styles.splitLayout}>
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th aria-label="Select for bulk action" />
                  <th>Application</th>
                  <th>Risk</th>
                  <th>Origin</th>
                  <th>Partition</th>
                  <th>State</th>
                </tr>
              </thead>
              <tbody>
                {filteredPackages.length === 0 ? (
                  <tr>
                    <td colSpan={6} className={styles.muted}>
                      No packages match the current search and filter criteria.
                    </td>
                  </tr>
                ) : (
                  filteredPackages.map((pkg) => {
                    const isSelected = pkg.packageName === selectedPackageName;
                    const rec = recommendations[pkg.packageName];
                    const risk: RiskClassification = rec?.risk ?? "unknown_unverified";
                    const isProtected = isProtectedInProfile(pkg.packageName, protections);
                    return (
                      <tr
                        key={pkg.packageName}
                        className={`${styles.rowSelectable} ${
                          isSelected ? styles.rowActive : ""
                        }`}
                        onClick={() => onSelectPackage(pkg)}
                      >
                        <td onClick={(e) => e.stopPropagation()}>
                          <input
                            type="checkbox"
                            aria-label={`Select ${pkg.packageName}`}
                            checked={checkedSet.has(pkg.packageName)}
                            onChange={() => toggleChecked(pkg.packageName)}
                          />
                        </td>
                        <td>
                          <span className={styles.pkgLabel}>
                            {pkg.label}
                            {isProtected && (
                              <StatusBadge tone="error">Protected</StatusBadge>
                            )}
                          </span>
                          <span className={styles.pkgId}>{pkg.packageName}</span>
                        </td>
                        <td>
                          <StatusBadge tone={riskTone(risk)}>
                            {formatRisk(risk)}
                          </StatusBadge>
                        </td>
                        <td>
                          <StatusBadge
                            tone={pkg.origin === "third_party" ? "accent" : "neutral"}
                          >
                            {formatOrigin(pkg.origin)}
                          </StatusBadge>
                        </td>
                        <td>{formatPartition(pkg.partition)}</td>
                        <td>
                          <StatusBadge
                            tone={
                              pkg.effectiveState === "enabled"
                                ? "ok"
                                : pkg.effectiveState === "disabled"
                                  ? "warn"
                                  : "neutral"
                            }
                          >
                            {formatEffectiveState(pkg.effectiveState)}
                          </StatusBadge>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>

          <aside className={styles.card}>
            <div className={styles.toolbar}>
              <h3 className={styles.heading}>Application Details</h3>
              {selectedPackage && (
                <button
                  type="button"
                  className={styles.button}
                  disabled={busy || inspectingPackage}
                  onClick={() => onInspectPackageDetails(selectedPackage.packageName)}
                >
                  {inspectingPackage
                    ? "Inspecting..."
                    : packageDetails?.packageName === selectedPackage.packageName
                      ? "Refresh Details"
                      : "Inspect (dumpsys)"}
                </button>
              )}
            </div>

            {!selectedPackage ? (
              <p className={styles.muted}>
                Select any row in the application table to inspect metadata, recommendation evidence, recovery snapshots, and package controls.
              </p>
            ) : (
              <>
                <h4 className={styles.subHeading}>Package Controls (User {selectedUserId ?? "â€”"})</h4>
                <div className={styles.actionRow}>
                  <button
                    type="button"
                    className={`${styles.button} ${styles.buttonDanger}`}
                    disabled={
                      !canMutate ||
                      selectedProtected ||
                      selectedPackage.effectiveState === "disabled"
                    }
                    onClick={() =>
                      triggerSingleAction(selectedPackage.packageName, "disable")
                    }
                  >
                    Disable
                  </button>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={
                      !canMutate ||
                      selectedPackage.effectiveState === "enabled"
                    }
                    onClick={() =>
                      triggerSingleAction(selectedPackage.packageName, "enable")
                    }
                  >
                    Enable
                  </button>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={!canMutate || !selectedSnapshotStatus?.baseline}
                    onClick={() =>
                      triggerSingleAction(
                        selectedPackage.packageName,
                        "restore_original_state",
                      )
                    }
                  >
                    Restore Original
                  </button>
                  <button
                    type="button"
                    className={styles.button}
                    disabled={!canMutate || !selectedSnapshotStatus?.latestPreOperation}
                    onClick={() =>
                      triggerSingleAction(selectedPackage.packageName, "undo_last")
                    }
                  >
                    Undo Last
                  </button>
                  {defaultStateSupported && (
                    <button
                      type="button"
                      className={styles.button}
                      disabled={!canMutate}
                      onClick={() =>
                        triggerSingleAction(
                          selectedPackage.packageName,
                          "restore_default_state",
                        )
                      }
                    >
                      Restore Default
                    </button>
                  )}
                </div>

                {selectedProtected && (
                  <AlertBanner
                    tone="error"
                    title="Disable blocked by ADT protection"
                    detail="This package is an immutable core component or the active/sole launcher or keyboard on this device."
                  />
                )}

                {selectedSnapshotStatus?.baseline && (
                  <p className={styles.muted}>
                    Initial toolkit baseline:{" "}
                    <strong>
                      {formatConfiguredState(
                        selectedSnapshotStatus.baseline.configuredState,
                        selectedSnapshotStatus.baseline.configuredStateCode,
                      )}
                    </strong>{" "}
                    (pre-op history: {selectedSnapshotStatus.preOperationCount})
                  </p>
                )}

                <dl className={styles.kvGrid}>
                  <dt>Label</dt>
                  <dd>{selectedPackage.label}</dd>
                  <dt>Package ID</dt>
                  <dd>{selectedPackage.packageName}</dd>
                  <dt>Effective State</dt>
                  <dd>{formatEffectiveState(selectedPackage.effectiveState)}</dd>
                  <dt>Origin</dt>
                  <dd>{formatOrigin(selectedPackage.origin)}</dd>
                  <dt>Partition</dt>
                  <dd>{formatPartition(selectedPackage.partition)}</dd>
                  <dt>APK Path</dt>
                  <dd>{selectedPackage.apkPath ?? "Unavailable"}</dd>
                </dl>

                <h4 className={styles.subHeading}>Recommendation & Evidence</h4>
                {selectedRecommendation ? (
                  <>
                    <div>
                      <StatusBadge tone={riskTone(selectedRecommendation.risk)}>
                        {formatRisk(selectedRecommendation.risk)}
                      </StatusBadge>
                    </div>
                    <p className={styles.muted}>{selectedRecommendation.explanation}</p>
                    <dl className={styles.kvGrid}>
                      <dt>Match Scope</dt>
                      <dd>{formatMatchScope(selectedRecommendation.matchScope)}</dd>
                      <dt>Verification</dt>
                      <dd>
                        {formatVerificationStatus(
                          selectedRecommendation.verificationStatus,
                        )}
                      </dd>
                      <dt>Confidence</dt>
                      <dd>{formatConfidence(selectedRecommendation.confidence)}</dd>
                      {selectedRecommendation.purpose && (
                        <>
                          <dt>Purpose</dt>
                          <dd>{selectedRecommendation.purpose}</dd>
                        </>
                      )}
                      {selectedRecommendation.knownConsequences && (
                        <>
                          <dt>Consequences</dt>
                          <dd>{selectedRecommendation.knownConsequences}</dd>
                        </>
                      )}
                    </dl>
                  </>
                ) : (
                  <p className={styles.muted}>
                    No recommendation evaluation available for this package yet.
                  </p>
                )}

                {packageDetails &&
                  packageDetails.packageName === selectedPackage.packageName && (
                    <>
                      <h4 className={styles.subHeading}>Targeted dumpsys Inspection</h4>
                      <dl className={styles.kvGrid}>
                        <dt>Version Name</dt>
                        <dd>{packageDetails.versionName ?? "Unavailable"}</dd>
                        <dt>Version Code</dt>
                        <dd>{packageDetails.versionCode ?? "Unavailable"}</dd>
                        <dt>Code Path</dt>
                        <dd>{packageDetails.codePath ?? "Unavailable"}</dd>
                        <dt>Observed User</dt>
                        <dd>
                          {packageDetails.observedUserId !== null
                            ? `User ${packageDetails.observedUserId}`
                            : "Unavailable"}
                        </dd>
                        <dt>Configured State</dt>
                        <dd>
                          {formatConfiguredState(
                            packageDetails.configuredState,
                            packageDetails.configuredStateCode,
                          )}
                        </dd>
                      </dl>
                    </>
                  )}
              </>
            )}
          </aside>
        </div>
      )}
    </div>
  );
}
