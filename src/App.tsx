import { useCallback, useEffect, useState } from "react";
import { OperationPreviewDialog } from "./components/dialogs/OperationPreviewDialog";
import { AlertBanner } from "./components/feedback/AlertBanner";
import {
  IconActivity,
  IconApps,
  IconCpu,
  IconDevices,
  IconHistory,
  IconSettings,
  IconShield,
} from "./components/shared/Icons";
import { StatusBadge } from "./components/shared/StatusBadge";
import { AppManagerPanel } from "./features/app-manager/AppManagerPanel";
import { DeviceInfoPanel } from "./features/device-info/DeviceInfoPanel";
import { AdbDevicePanel } from "./features/devices/AdbDevicePanel";
import { DiagnosticsPanel } from "./features/diagnostics/DiagnosticsPanel";
import { HistoryPanel } from "./features/history/HistoryPanel";
import { SettingsPanel } from "./features/settings/SettingsPanel";
import { detectAdb, listDevices, parseAdtError } from "./lib/tauri/adb";
import { getDeviceCapabilities, getDeviceSpecs } from "./lib/tauri/device";
import {
  captureMemorySnapshot,
  compareMemorySnapshots,
  getPerformanceMetrics,
  getStorageDiagnostics,
  listMemorySnapshots,
} from "./lib/tauri/diagnostics";
import {
  clearOperationHistory,
  exportDiagnostics,
  getAppSettings,
  getOperationHistory,
  reconcileUncertainOperation,
  saveAppSettings,
} from "./lib/tauri/history";
import {
  executePackageOperations,
  getDeviceProtections,
  getPackageSnapshots,
  previewPackageOperations,
} from "./lib/tauri/operations";
import {
  inspectPackageDetails,
  scanInstalledPackages,
} from "./lib/tauri/packages";
import {
  evaluateRecommendations,
  getRuleDatabaseSummary,
} from "./lib/tauri/rules";
import styles from "./styles/app.module.css";
import type { AdbDevice, AdbStatus, AdtErrorPayload } from "./types/adb";
import type { DeviceCapabilities, DeviceSpecs } from "./types/device";
import type {
  MemoryComparison,
  MemorySnapshot,
  PerformanceMetrics,
  StorageDashboard,
} from "./types/diagnostics";
import type { AppSettings, DiagnosticBundle } from "./types/history";
import type {
  BulkOperationPreview,
  BulkOperationResult,
  DeviceProtectionProfile,
  PackageActionRequest,
  PackageOperationResult,
  PackageSnapshotStatus,
} from "./types/operations";
import type {
  InstalledPackage,
  PackageDetails,
  PackageInventory,
} from "./types/packages";
import type {
  PackageRecommendation,
  RuleDatabaseSummary,
} from "./types/rules";

type ActiveTab =
  | "devices"
  | "device-info"
  | "apps"
  | "diagnostics"
  | "history"
  | "settings";

const DEFAULT_SETTINGS: AppSettings = {
  customAdbPath: null,
  autoScanOnSelect: false,
  logLevel: "standard",
};

const SECTION_TITLES: Record<ActiveTab, string> = {
  devices: "Devices & ADB Connection",
  "device-info": "Device Specifications & Capabilities",
  apps: "Installed Applications",
  diagnostics: "Memory & Storage Diagnostics",
  history: "Operation History & Recovery",
  settings: "Settings & Redacted Diagnostics",
};

export function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("devices");
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [customAdbPath, setCustomAdbPath] = useState("");
  const [adbStatus, setAdbStatus] = useState<AdbStatus | null>(null);
  const [devices, setDevices] = useState<AdbDevice[]>([]);
  const [selectedSerial, setSelectedSerial] = useState<string | null>(null);

  const [deviceSpecs, setDeviceSpecs] = useState<DeviceSpecs | null>(null);
  const [capabilities, setCapabilities] = useState<DeviceCapabilities | null>(null);
  const [protections, setProtections] = useState<DeviceProtectionProfile | null>(null);
  const [selectedUserId, setSelectedUserId] = useState<number | null>(null);

  const [inventory, setInventory] = useState<PackageInventory | null>(null);
  const [ruleDbSummary, setRuleDbSummary] = useState<RuleDatabaseSummary | null>(null);
  const [recommendations, setRecommendations] = useState<
    Record<string, PackageRecommendation>
  >({});
  const [snapshots, setSnapshots] = useState<Record<string, PackageSnapshotStatus>>({});
  const [pendingItems, setPendingItems] = useState<PackageActionRequest[]>([]);
  const [operationPreview, setOperationPreview] = useState<BulkOperationPreview | null>(
    null,
  );
  const [lastOperationResult, setLastOperationResult] =
    useState<BulkOperationResult | null>(null);

  const [memorySnapshots, setMemorySnapshots] = useState<MemorySnapshot[]>([]);
  const [memoryComparison, setMemoryComparison] = useState<MemoryComparison | null>(
    null,
  );
  const [storageDashboard, setStorageDashboard] = useState<StorageDashboard | null>(
    null,
  );
  const [performanceMetrics, setPerformanceMetrics] =
    useState<PerformanceMetrics | null>(null);

  const [history, setHistory] = useState<PackageOperationResult[]>([]);
  const [historyStatusFilter, setHistoryStatusFilter] = useState("all");
  const [historySearch, setHistorySearch] = useState("");
  const [historyAllDevices, setHistoryAllDevices] = useState(true);
  const [reconcilingId, setReconcilingId] = useState<string | null>(null);
  const [diagnosticBundle, setDiagnosticBundle] = useState<DiagnosticBundle | null>(
    null,
  );

  const [selectedPackageName, setSelectedPackageName] = useState<string | null>(null);
  const [packageDetails, setPackageDetails] = useState<PackageDetails | null>(null);

  const [loadingMessage, setLoadingMessage] = useState<string | null>(null);
  const [inspectingPackage, setInspectingPackage] = useState(false);
  const [executingOperations, setExecutingOperations] = useState(false);
  const [error, setError] = useState<AdtErrorPayload | null>(null);

  const clearSelectedDeviceData = useCallback(() => {
    setDeviceSpecs(null);
    setCapabilities(null);
    setProtections(null);
    setSelectedUserId(null);
    setInventory(null);
    setRecommendations({});
    setSnapshots({});
    setMemorySnapshots([]);
    setMemoryComparison(null);
    setStorageDashboard(null);
    setPerformanceMetrics(null);
    setPendingItems([]);
    setOperationPreview(null);
    setLastOperationResult(null);
    setSelectedPackageName(null);
    setPackageDetails(null);
  }, []);

  const refreshHistory = useCallback(
    async (opts?: {
      serial?: string | null;
      allDevices?: boolean;
      statusFilter?: string;
      search?: string;
    }) => {
      const useAll = opts?.allDevices ?? historyAllDevices;
      const targetSerial = useAll ? null : (opts?.serial ?? selectedSerial);
      const rows = await getOperationHistory({
        serial: targetSerial,
        statusFilter: opts?.statusFilter ?? historyStatusFilter,
        search: opts?.search ?? historySearch,
      });
      setHistory(rows);
    },
    [historyAllDevices, historySearch, historyStatusFilter, selectedSerial],
  );

  const runScanAndEvaluate = useCallback(
    async (
      serial: string,
      userId: number | null,
      specsOverride?: DeviceSpecs | null,
    ) => {
      const activeSpecs = specsOverride ?? deviceSpecs;
      setLoadingMessage(`Scanning installed packages on ${serial}...`);
      const result = await scanInstalledPackages(serial, userId);
      setInventory(result);

      setLoadingMessage("Evaluating offline package recommendations...");
      const batch = await evaluateRecommendations(
        result.packages.map((p) => p.packageName),
        {
          manufacturer: activeSpecs?.identification.manufacturer ?? null,
          model: activeSpecs?.identification.model ?? null,
          codename: activeSpecs?.identification.codename ?? null,
          apiLevel: activeSpecs?.identification.apiLevel ?? null,
        },
      );
      setRuleDbSummary(batch.database);
      setRecommendations(batch.recommendations);

      const perf = await getPerformanceMetrics(serial);
      setPerformanceMetrics(perf);

      if (userId !== null) {
        const snapMap = await getPackageSnapshots(serial, userId);
        setSnapshots(snapMap);
      }
    },
    [deviceSpecs],
  );

  const loadDeviceContext = useCallback(
    async (serial: string, shouldAutoScan: boolean) => {
      setError(null);
      setLoadingMessage(
        `Reading specifications, capabilities, and safety protections for ${serial}...`,
      );
      try {
        const [specs, caps, prot, memSnaps, perf] = await Promise.all([
          getDeviceSpecs(serial),
          getDeviceCapabilities(serial),
          getDeviceProtections(serial),
          listMemorySnapshots(serial),
          getPerformanceMetrics(serial),
        ]);
        setDeviceSpecs(specs);
        setCapabilities(caps);
        setProtections(prot);
        setMemorySnapshots(memSnaps);
        setPerformanceMetrics(perf);
        setSelectedUserId(caps.activeUserId);
        if (caps.activeUserId !== null) {
          const snapMap = await getPackageSnapshots(serial, caps.activeUserId);
          setSnapshots(snapMap);
        }
        await refreshHistory({ serial, allDevices: false });
        setHistoryAllDevices(false);

        if (shouldAutoScan) {
          await runScanAndEvaluate(serial, caps.activeUserId, specs);
        }
      } catch (err) {
        setError(parseAdtError(err));
      } finally {
        setLoadingMessage(null);
      }
    },
    [refreshHistory, runScanAndEvaluate],
  );

  const initializeApp = useCallback(async () => {
    setError(null);
    setLoadingMessage("Loading local settings and checking ADB environment...");
    try {
      const [loadedSettings, dbSummary] = await Promise.all([
        getAppSettings(),
        getRuleDatabaseSummary(),
      ]);
      setSettings(loadedSettings);
      setCustomAdbPath(loadedSettings.customAdbPath ?? "");
      setRuleDbSummary(dbSummary);

      const status = await detectAdb(loadedSettings.customAdbPath ?? undefined);
      setAdbStatus(status);
      if (status.available) {
        const found = await listDevices();
        setDevices(found);
      }

      const initialHistory = await getOperationHistory({});
      setHistory(initialHistory);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  }, []);

  useEffect(() => {
    void initializeApp();
  }, [initializeApp]);

  const handleCheckAdb = async (pathOverride?: string) => {
    setError(null);
    setLoadingMessage("Checking ADB environment...");
    try {
      const status = await detectAdb(pathOverride);
      setAdbStatus(status);
      if (!status.available) {
        setDevices([]);
        setSelectedSerial(null);
        clearSelectedDeviceData();
        return;
      }
      const found = await listDevices();
      setDevices(found);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleRefreshDevices = async () => {
    setError(null);
    setLoadingMessage("Scanning connected ADB devices...");
    try {
      const found = await listDevices();
      setDevices(found);
      if (
        selectedSerial &&
        !found.some(
          (d) => d.serial === selectedSerial && d.state === "authorized",
        )
      ) {
        setSelectedSerial(null);
        clearSelectedDeviceData();
      }
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleSelectDevice = async (device: AdbDevice) => {
    if (device.state !== "authorized") {
      return;
    }
    setSelectedSerial(device.serial);
    clearSelectedDeviceData();
    await loadDeviceContext(device.serial, settings.autoScanOnSelect);
    setActiveTab("device-info");
  };

  const handleScanPackages = async () => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    setSelectedPackageName(null);
    setPackageDetails(null);
    setRecommendations({});
    try {
      await runScanAndEvaluate(selectedSerial, selectedUserId);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleSelectPackage = (pkg: InstalledPackage) => {
    setSelectedPackageName(pkg.packageName);
    if (packageDetails && packageDetails.packageName !== pkg.packageName) {
      setPackageDetails(null);
    }
  };

  const handleInspectPackageDetails = async (packageName: string) => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    setInspectingPackage(true);
    try {
      const details = await inspectPackageDetails(
        selectedSerial,
        packageName,
        selectedUserId,
      );
      setPackageDetails(details);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setInspectingPackage(false);
    }
  };

  const capabilityFlags = {
    pmDefaultStateSupported: capabilities?.pmDefaultState === "supported",
    pmDisableUntilUsedSupported: capabilities?.pmDisableUntilUsed === "supported",
  };

  const handleRequestOperations = async (items: PackageActionRequest[]) => {
    if (!selectedSerial || selectedUserId === null || items.length === 0) {
      return;
    }
    setError(null);
    setLoadingMessage("Preparing safety preview for planned package operations...");
    try {
      const preview = await previewPackageOperations({
        serial: selectedSerial,
        userId: selectedUserId,
        items,
        capabilities: capabilityFlags,
      });
      setPendingItems(items);
      setOperationPreview(preview);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleConfirmOperations = async () => {
    if (!selectedSerial || selectedUserId === null || pendingItems.length === 0) {
      return;
    }
    setError(null);
    setExecutingOperations(true);
    try {
      const result = await executePackageOperations({
        serial: selectedSerial,
        userId: selectedUserId,
        items: pendingItems,
        confirmed: true,
        androidVersion: deviceSpecs?.identification.androidVersion ?? null,
        capabilities: capabilityFlags,
      });
      setLastOperationResult(result);
      setOperationPreview(null);
      setPendingItems([]);
      setPackageDetails(null);
      await refreshHistory();
      if (!result.abortedEarlyReason) {
        await runScanAndEvaluate(selectedSerial, selectedUserId);
      }
    } catch (err) {
      setError(parseAdtError(err));
      setOperationPreview(null);
    } finally {
      setExecutingOperations(false);
      setLoadingMessage(null);
    }
  };

  const handleCaptureMemory = async (label: string) => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    setLoadingMessage(`Capturing read-only memory snapshot '${label}'...`);
    try {
      await captureMemorySnapshot(selectedSerial, label);
      const [snaps, perf] = await Promise.all([
        listMemorySnapshots(selectedSerial),
        getPerformanceMetrics(selectedSerial),
      ]);
      setMemorySnapshots(snaps);
      setPerformanceMetrics(perf);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleCompareMemory = async (beforeId: string, afterId: string) => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    try {
      const cmp = await compareMemorySnapshots(selectedSerial, beforeId, afterId);
      setMemoryComparison(cmp);
    } catch (err) {
      setError(parseAdtError(err));
    }
  };

  const handleRefreshStorage = async () => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    setLoadingMessage("Reading storage partitions and /data usage...");
    try {
      const dash = await getStorageDiagnostics(selectedSerial);
      setStorageDashboard(dash);
      const perf = await getPerformanceMetrics(selectedSerial);
      setPerformanceMetrics(perf);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleReconcileOperation = async (operationId: string) => {
    if (!selectedSerial) {
      return;
    }
    setError(null);
    setReconcilingId(operationId);
    try {
      await reconcileUncertainOperation(selectedSerial, operationId);
      await refreshHistory();
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setReconcilingId(null);
    }
  };

  const handleClearHistory = async () => {
    setError(null);
    setLoadingMessage("Clearing operation history (preserving baseline snapshots)...");
    try {
      await clearOperationHistory(historyAllDevices ? null : selectedSerial);
      await refreshHistory();
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleSaveSettings = async (next: AppSettings) => {
    setError(null);
    setLoadingMessage("Saving settings to local SQLite database...");
    try {
      const saved = await saveAppSettings(next);
      setSettings(saved);
      setCustomAdbPath(saved.customAdbPath ?? "");
      const status = await detectAdb(saved.customAdbPath ?? undefined);
      setAdbStatus(status);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const handleGenerateDiagnostics = async () => {
    setError(null);
    setLoadingMessage("Generating privacy-redacted diagnostic bundle...");
    try {
      const bundle = await exportDiagnostics();
      setDiagnosticBundle(bundle);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  };

  const busy = loadingMessage !== null || executingOperations;
  const uncertainHistoryCount = history.filter(
    (op) => op.status === "uncertain_timeout",
  ).length;
  const targetModelLabel =
    [deviceSpecs?.identification.manufacturer, deviceSpecs?.identification.model]
      .filter(Boolean)
      .join(" ") || selectedSerial;

  return (
    <div className={styles.appFrame}>
      <header className={styles.topBar}>
        <div className={styles.brandGroup}>
          <span className={styles.brandMark}>ADT</span>
          <h1 className={styles.brandTitle}>Android Debloating Toolkit</h1>
          <span className={styles.topDivider} aria-hidden="true" />
          <span className={styles.activeSectionLabel}>
            {SECTION_TITLES[activeTab]}
          </span>
        </div>

        <div className={styles.topStatusGroup}>
          <StatusBadge tone={adbStatus?.available ? "ok" : "warn"}>
            {adbStatus?.available
              ? `ADB ${adbStatus.adbVersion ?? "ready"}`
              : "ADB unavailable"}
          </StatusBadge>
          {selectedSerial ? (
            <StatusBadge tone="accent" icon={<IconShield size={12} />}>
              {targetModelLabel}
              {deviceSpecs?.identification.androidVersion
                ? ` · Android ${deviceSpecs.identification.androidVersion}`
                : ""}
              {selectedUserId !== null ? ` · User ${selectedUserId}` : ""}
            </StatusBadge>
          ) : (
            <StatusBadge tone="neutral">No device selected</StatusBadge>
          )}
        </div>
      </header>

      <aside className={styles.sidebar} aria-label="Workspace navigation">
        <div>
          <p className={styles.navSectionLabel}>Workspace</p>
          <nav className={styles.navList}>
            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "devices" ? styles.navButtonActive : ""
              }`}
              onClick={() => setActiveTab("devices")}
            >
              <span className={styles.navLabelGroup}>
                <IconDevices />
                <span>Devices & ADB</span>
              </span>
              <span className={styles.navCount}>{devices.length}</span>
            </button>

            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "device-info" ? styles.navButtonActive : ""
              }`}
              disabled={!selectedSerial}
              onClick={() => setActiveTab("device-info")}
            >
              <span className={styles.navLabelGroup}>
                <IconCpu />
                <span>Device Specs</span>
              </span>
            </button>

            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "apps" ? styles.navButtonActive : ""
              }`}
              disabled={!selectedSerial}
              onClick={() => setActiveTab("apps")}
            >
              <span className={styles.navLabelGroup}>
                <IconApps />
                <span>Applications</span>
              </span>
              {inventory && (
                <span className={styles.navCount}>{inventory.totalCount}</span>
              )}
            </button>

            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "diagnostics" ? styles.navButtonActive : ""
              }`}
              disabled={!selectedSerial}
              onClick={() => setActiveTab("diagnostics")}
            >
              <span className={styles.navLabelGroup}>
                <IconActivity />
                <span>Memory & Storage</span>
              </span>
              {memorySnapshots.length > 0 && (
                <span className={styles.navCount}>{memorySnapshots.length}</span>
              )}
            </button>

            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "history" ? styles.navButtonActive : ""
              }`}
              onClick={() => {
                setActiveTab("history");
                void refreshHistory();
              }}
            >
              <span className={styles.navLabelGroup}>
                <IconHistory />
                <span>History & Undo</span>
              </span>
              <span
                className={`${styles.navCount} ${
                  uncertainHistoryCount > 0 ? styles.navCountWarn : ""
                }`}
              >
                {uncertainHistoryCount > 0
                  ? `${uncertainHistoryCount}!`
                  : history.length}
              </span>
            </button>

            <button
              type="button"
              className={`${styles.navButton} ${
                activeTab === "settings" ? styles.navButtonActive : ""
              }`}
              onClick={() => setActiveTab("settings")}
            >
              <span className={styles.navLabelGroup}>
                <IconSettings />
                <span>Settings</span>
              </span>
            </button>
          </nav>
        </div>

        <section className={styles.targetCard} aria-label="Active target summary">
          <div className={styles.targetCardHeader}>
            <span className={styles.targetHeading}>Active Target</span>
            <span
              className={`${styles.targetDot} ${
                selectedSerial ? styles.targetDotActive : ""
              }`}
              aria-hidden="true"
            />
          </div>
          {selectedSerial ? (
            <>
              <span className={styles.targetModel}>{targetModelLabel}</span>
              <span className={styles.targetMono}>{selectedSerial}</span>
              <span className={styles.targetMeta}>
                Scope:{" "}
                {selectedUserId !== null
                  ? `Android User ${selectedUserId}`
                  : "User unselected"}
              </span>
              <span className={styles.targetMeta}>
                Safety guard:{" "}
                {protections
                  ? protections.launcherDetectionUncertain ||
                    protections.keyboardDetectionUncertain
                    ? "Conservative fallback"
                    : "Active"
                  : "Pending"}
              </span>
            </>
          ) : (
            <span className={styles.targetMeta}>
              Connect and select an authorized Android device to begin.
            </span>
          )}
        </section>
      </aside>

      <main className={styles.workspace}>
        {loadingMessage && (
          <div className={styles.loadingBar} role="status">
            {loadingMessage}
          </div>
        )}

        {error && (
          <AlertBanner
            tone="error"
            title={error.message}
            detail={error.guidance}
            onDismiss={() => setError(null)}
          />
        )}

        {activeTab === "devices" && (
          <AdbDevicePanel
            customAdbPath={customAdbPath}
            onCustomAdbPathChange={setCustomAdbPath}
            adbStatus={adbStatus}
            devices={devices}
            selectedSerial={selectedSerial}
            busy={busy}
            onCheckAdb={() => void handleCheckAdb(customAdbPath)}
            onRefreshDevices={() => void handleRefreshDevices()}
            onSelectDevice={(device) => void handleSelectDevice(device)}
          />
        )}

        {activeTab === "device-info" && (
          <DeviceInfoPanel
            selectedSerial={selectedSerial}
            specs={deviceSpecs}
            capabilities={capabilities}
            selectedUserId={selectedUserId}
            onSelectUserId={(uid) => {
              setSelectedUserId(uid);
              setInventory(null);
              setRecommendations({});
              setSelectedPackageName(null);
              setPackageDetails(null);
              if (selectedSerial && uid !== null) {
                void getPackageSnapshots(selectedSerial, uid).then(setSnapshots);
              } else {
                setSnapshots({});
              }
            }}
            busy={busy}
            onRefresh={() => {
              if (selectedSerial) {
                void loadDeviceContext(selectedSerial, false);
              }
            }}
          />
        )}

        {activeTab === "apps" && (
          <AppManagerPanel
            selectedSerial={selectedSerial}
            selectedUserId={selectedUserId}
            defaultStateSupported={capabilityFlags.pmDefaultStateSupported}
            inventory={inventory}
            ruleDbSummary={ruleDbSummary}
            recommendations={recommendations}
            protections={protections}
            snapshots={snapshots}
            lastOperationResult={lastOperationResult}
            selectedPackageName={selectedPackageName}
            packageDetails={packageDetails}
            busy={busy}
            inspectingPackage={inspectingPackage}
            onScanPackages={() => void handleScanPackages()}
            onSelectPackage={handleSelectPackage}
            onInspectPackageDetails={(pkgName) =>
              void handleInspectPackageDetails(pkgName)
            }
            onRequestOperations={(items) => void handleRequestOperations(items)}
            onDismissLastResult={() => setLastOperationResult(null)}
          />
        )}

        {activeTab === "diagnostics" && (
          <DiagnosticsPanel
            selectedSerial={selectedSerial}
            snapshots={memorySnapshots}
            comparison={memoryComparison}
            storageDashboard={storageDashboard}
            performanceMetrics={performanceMetrics}
            busy={busy}
            onCaptureMemory={(label) => void handleCaptureMemory(label)}
            onCompareSnapshots={(b, a) => void handleCompareMemory(b, a)}
            onRefreshStorage={() => void handleRefreshStorage()}
          />
        )}

        {activeTab === "history" && (
          <HistoryPanel
            selectedSerial={selectedSerial}
            selectedUserId={selectedUserId}
            history={history}
            statusFilter={historyStatusFilter}
            searchQuery={historySearch}
            scopeAllDevices={historyAllDevices}
            busy={busy}
            reconcilingId={reconcilingId}
            onStatusFilterChange={(val) => {
              setHistoryStatusFilter(val);
              void refreshHistory({ statusFilter: val });
            }}
            onSearchChange={(val) => {
              setHistorySearch(val);
              void refreshHistory({ search: val });
            }}
            onScopeChange={(allDevices) => {
              setHistoryAllDevices(allDevices);
              void refreshHistory({ allDevices });
            }}
            onRefresh={() => void refreshHistory()}
            onReconcile={(opId) => void handleReconcileOperation(opId)}
            onRequestPackageAction={(items) => void handleRequestOperations(items)}
            onClearHistory={() => void handleClearHistory()}
          />
        )}

        {activeTab === "settings" && (
          <SettingsPanel
            settings={settings}
            ruleDbSummary={ruleDbSummary}
            diagnosticBundle={diagnosticBundle}
            busy={busy}
            onSaveSettings={(next) => void handleSaveSettings(next)}
            onGenerateDiagnostics={() => void handleGenerateDiagnostics()}
          />
        )}
      </main>

      <footer className={styles.statusBar}>
        <div className={styles.statusCluster}>
          <span>
            ADB: {adbStatus?.available ? adbStatus.adbVersion ?? "ready" : "missing"}
          </span>
          <span className={styles.statusSep}>|</span>
          <span>Storage: Local SQLite (Offline-First)</span>
          {ruleDbSummary && (
            <>
              <span className={styles.statusSep}>|</span>
              <span>
                Rules: {ruleDbSummary.databaseVersion} ({ruleDbSummary.verifiedRules}{" "}
                verified)
              </span>
            </>
          )}
        </div>
        <div className={styles.statusCluster}>
          {performanceMetrics?.lastPackageScanMs !== null &&
          performanceMetrics?.lastPackageScanMs !== undefined ? (
            <span>
              Last scan: {performanceMetrics.lastPackageScanMs} ms (
              {performanceMetrics.lastPackageCount ?? 0} pkgs)
            </span>
          ) : (
            <span>No scan measured yet</span>
          )}
        </div>
      </footer>

      {operationPreview && (
        <OperationPreviewDialog
          preview={operationPreview}
          executing={executingOperations}
          onCancel={() => {
            setOperationPreview(null);
            setPendingItems([]);
          }}
          onConfirm={() => void handleConfirmOperations()}
        />
      )}
    </div>
  );
}

export default App;
