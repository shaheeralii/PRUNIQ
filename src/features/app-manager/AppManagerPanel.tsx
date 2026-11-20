import { useMemo, useState } from "react";
import { AlertBanner } from "../../components/feedback/AlertBanner";
import { StatusBadge } from "../../components/shared/StatusBadge";
import {
  filterInstalledPackages,
  formatConfiguredState,
  formatEffectiveState,
  formatOrigin,
  formatPartition,
  type ClientPackageFilter,
} from "../../lib/format";
import type {
  EffectivePackageState,
  InstallPartition,
  InstalledPackage,
  PackageDetails,
  PackageInventory,
  PackageOrigin,
} from "../../types/packages";
import styles from "./AppManagerPanel.module.css";

interface AppManagerPanelProps {
  selectedSerial: string | null;
  selectedUserId: number | null;
  inventory: PackageInventory | null;
  selectedPackageName: string | null;
  packageDetails: PackageDetails | null;
  busy: boolean;
  inspectingPackage: boolean;
  onScanPackages: () => void;
  onSelectPackage: (pkg: InstalledPackage) => void;
  onInspectPackageDetails: (packageName: string) => void;
}

const DEFAULT_FILTER: ClientPackageFilter = {
  search: "",
  origin: "all",
  state: "all",
  partition: "all",
  updatedSystemOnly: false,
};

export function AppManagerPanel({
  selectedSerial,
  selectedUserId,
  inventory,
  selectedPackageName,
  packageDetails,
  busy,
  inspectingPackage,
  onScanPackages,
  onSelectPackage,
  onInspectPackageDetails,
}: AppManagerPanelProps) {
  const [filter, setFilter] = useState<ClientPackageFilter>(DEFAULT_FILTER);

  const filteredPackages = useMemo(() => {
    if (!inventory) {
      return [];
    }
    return filterInstalledPackages(inventory.packages, filter);
  }, [inventory, filter]);

  const selectedPackage = useMemo(() => {
    if (!inventory || !selectedPackageName) {
      return null;
    }
    return (
      inventory.packages.find((p) => p.packageName === selectedPackageName) ?? null
    );
  }, [inventory, selectedPackageName]);

  if (!selectedSerial) {
    return (
      <section className={styles.card}>
        <h2 className={styles.heading}>Installed Applications</h2>
        <p className={styles.muted}>
          Select an authorized device from the Devices & ADB tab before scanning installed packages.
        </p>
      </section>
    );
  }

  const hasActiveFilters =
    filter.search.trim().length > 0 ||
    filter.origin !== "all" ||
    filter.state !== "all" ||
    filter.partition !== "all" ||
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
                {selectedUserId !== null ? `User ${selectedUserId}` : "Device default"}
              </strong>
            </p>
          </div>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy}
            onClick={onScanPackages}
          >
            {inventory ? "Rescan Packages" : "Scan Packages"}
          </button>
        </div>

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
          </>
        )}
      </section>

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
            Click <strong>Scan Packages</strong> to run a fast bulk read-only inventory (uses at most 5 bulk{" "}
            <code>pm list packages</code> calls without per-app <code>dumpsys</code> loops).
          </p>
        </section>
      ) : (
        <div className={styles.splitLayout}>
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>Application</th>
                  <th>Origin</th>
                  <th>Partition</th>
                  <th>State</th>
                </tr>
              </thead>
              <tbody>
                {filteredPackages.length === 0 ? (
                  <tr>
                    <td colSpan={4} className={styles.muted}>
                      No packages match the current search and filter criteria.
                    </td>
                  </tr>
                ) : (
                  filteredPackages.map((pkg) => {
                    const isSelected = pkg.packageName === selectedPackageName;
                    return (
                      <tr
                        key={pkg.packageName}
                        className={`${styles.rowSelectable} ${
                          isSelected ? styles.rowActive : ""
                        }`}
                        onClick={() => onSelectPackage(pkg)}
                      >
                        <td>
                          <span className={styles.pkgLabel}>{pkg.label}</span>
                          <span className={styles.pkgId}>{pkg.packageName}</span>
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
                Select any row in the application table to inspect its metadata, installation partition, and targeted{" "}
                <code>dumpsys package</code> details.
              </p>
            ) : (
              <>
                <dl className={styles.kvGrid}>
                  <dt>Label</dt>
                  <dd>{selectedPackage.label}</dd>
                  <dt>Label Source</dt>
                  <dd>
                    {selectedPackage.labelSource === "curated_catalog"
                      ? "Curated offline catalog"
                      : "Raw package ID fallback"}
                  </dd>
                  <dt>Package ID</dt>
                  <dd>{selectedPackage.packageName}</dd>
                  <dt>Effective State</dt>
                  <dd>{formatEffectiveState(selectedPackage.effectiveState)}</dd>
                  <dt>Origin</dt>
                  <dd>{formatOrigin(selectedPackage.origin)}</dd>
                  <dt>Partition</dt>
                  <dd>{formatPartition(selectedPackage.partition)}</dd>
                  <dt>Flags</dt>
                  <dd>
                    {[
                      selectedPackage.isSystem ? "system" : null,
                      selectedPackage.isThirdParty ? "third-party" : null,
                      selectedPackage.isUpdatedSystem ? "updated-system" : null,
                      selectedPackage.isGoogle ? "google" : null,
                      selectedPackage.isOem ? "oem" : null,
                    ]
                      .filter(Boolean)
                      .join(", ") || "None"}
                  </dd>
                  <dt>APK Path</dt>
                  <dd>{selectedPackage.apkPath ?? "Unavailable"}</dd>
                </dl>

                {packageDetails &&
                  packageDetails.packageName === selectedPackage.packageName && (
                    <>
                      <h4 className={styles.heading}>Targeted dumpsys Inspection</h4>
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
                        <dt>Installed for User</dt>
                        <dd>
                          {packageDetails.installedForUser === null
                            ? "Unknown"
                            : packageDetails.installedForUser
                              ? "Yes"
                              : "No"}
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

                <p className={styles.readOnlyNote}>
                  Phase 2.5 is strictly read-only. Package disable/enable and recovery snapshot controls are scheduled for Phase 4.
                </p>
              </>
            )}
          </aside>
        </div>
      )}
    </div>
  );
}
