import { useCallback, useEffect, useState } from "react";
import { AlertBanner } from "./components/feedback/AlertBanner";
import { StatusBadge } from "./components/shared/StatusBadge";
import { AppManagerPanel } from "./features/app-manager/AppManagerPanel";
import { DeviceInfoPanel } from "./features/device-info/DeviceInfoPanel";
import { AdbDevicePanel } from "./features/devices/AdbDevicePanel";
import { detectAdb, listDevices, parseAdtError } from "./lib/tauri/adb";
import { getDeviceCapabilities, getDeviceSpecs } from "./lib/tauri/device";
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
  InstalledPackage,
  PackageDetails,
  PackageInventory,
} from "./types/packages";
import type {
  PackageRecommendation,
  RuleDatabaseSummary,
} from "./types/rules";

type ActiveTab = "devices" | "device-info" | "apps";

export function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("devices");
  const [customAdbPath, setCustomAdbPath] = useState("");
  const [adbStatus, setAdbStatus] = useState<AdbStatus | null>(null);
  const [devices, setDevices] = useState<AdbDevice[]>([]);
  const [selectedSerial, setSelectedSerial] = useState<string | null>(null);

  const [deviceSpecs, setDeviceSpecs] = useState<DeviceSpecs | null>(null);
  const [capabilities, setCapabilities] = useState<DeviceCapabilities | null>(null);
  const [selectedUserId, setSelectedUserId] = useState<number | null>(null);

  const [inventory, setInventory] = useState<PackageInventory | null>(null);
  const [ruleDbSummary, setRuleDbSummary] = useState<RuleDatabaseSummary | null>(null);
  const [recommendations, setRecommendations] = useState<
    Record<string, PackageRecommendation>
  >({});
  const [selectedPackageName, setSelectedPackageName] = useState<string | null>(null);
  const [packageDetails, setPackageDetails] = useState<PackageDetails | null>(null);

  const [loadingMessage, setLoadingMessage] = useState<string | null>(null);
  const [inspectingPackage, setInspectingPackage] = useState(false);
  const [error, setError] = useState<AdtErrorPayload | null>(null);

  const clearSelectedDeviceData = useCallback(() => {
    setDeviceSpecs(null);
    setCapabilities(null);
    setSelectedUserId(null);
    setInventory(null);
    setRecommendations({});
    setSelectedPackageName(null);
    setPackageDetails(null);
  }, []);

  const loadDeviceContext = useCallback(async (serial: string) => {
    setError(null);
    setLoadingMessage(`Reading specifications and capabilities for ${serial}...`);
    try {
      const [specs, caps] = await Promise.all([
        getDeviceSpecs(serial),
        getDeviceCapabilities(serial),
      ]);
      setDeviceSpecs(specs);
      setCapabilities(caps);
      setSelectedUserId(caps.activeUserId);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoadingMessage(null);
    }
  }, []);

  const handleCheckAdb = useCallback(
    async (pathOverride?: string) => {
      setError(null);
      setLoadingMessage("Checking ADB environment...");
      try {
        const [status, dbSummary] = await Promise.all([
          detectAdb(pathOverride),
          getRuleDatabaseSummary(),
        ]);
        setAdbStatus(status);
        setRuleDbSummary(dbSummary);
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
    },
    [clearSelectedDeviceData],
  );

  useEffect(() => {
    void handleCheckAdb();
  }, [handleCheckAdb]);

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
    await loadDeviceContext(device.serial);
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
    setLoadingMessage(`Scanning installed packages on ${selectedSerial}...`);
    try {
      const result = await scanInstalledPackages(selectedSerial, selectedUserId);
      setInventory(result);

      setLoadingMessage("Evaluating offline package recommendations...");
      const batch = await evaluateRecommendations(
        result.packages.map((p) => p.packageName),
        {
          manufacturer: deviceSpecs?.identification.manufacturer ?? null,
          model: deviceSpecs?.identification.model ?? null,
          codename: deviceSpecs?.identification.codename ?? null,
          apiLevel: deviceSpecs?.identification.apiLevel ?? null,
        },
      );
      setRuleDbSummary(batch.database);
      setRecommendations(batch.recommendations);
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

  const busy = loadingMessage !== null;

  return (
    <main className={styles.shell}>
      <header className={styles.header}>
        <div className={styles.titleGroup}>
          <h1 className={styles.title}>Android Debloating Toolkit</h1>
          <StatusBadge tone="neutral">Phase 3 â€” Offline Recommendation Engine</StatusBadge>
        </div>
        <div className={styles.statusBar}>
          <StatusBadge tone={adbStatus?.available ? "ok" : "warn"}>
            {adbStatus?.available
              ? `ADB ${adbStatus.adbVersion ?? "ready"}`
              : "ADB unavailable"}
          </StatusBadge>
          <StatusBadge tone={selectedSerial ? "accent" : "neutral"}>
            {selectedSerial ? `Target: ${selectedSerial}` : "No device selected"}
          </StatusBadge>
        </div>
      </header>

      <nav className={styles.tabs} aria-label="Workspace sections">
        <button
          type="button"
          className={`${styles.tabButton} ${
            activeTab === "devices" ? styles.tabButtonActive : ""
          }`}
          onClick={() => setActiveTab("devices")}
        >
          1. Devices & ADB ({devices.length})
        </button>
        <button
          type="button"
          className={`${styles.tabButton} ${
            activeTab === "device-info" ? styles.tabButtonActive : ""
          }`}
          disabled={!selectedSerial}
          onClick={() => setActiveTab("device-info")}
        >
          2. Device Info & Capabilities
        </button>
        <button
          type="button"
          className={`${styles.tabButton} ${
            activeTab === "apps" ? styles.tabButtonActive : ""
          }`}
          disabled={!selectedSerial}
          onClick={() => setActiveTab("apps")}
        >
          3. Installed Applications
          {inventory ? ` (${inventory.totalCount})` : ""}
        </button>
      </nav>

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
          }}
          busy={busy}
          onRefresh={() => {
            if (selectedSerial) {
              void loadDeviceContext(selectedSerial);
            }
          }}
        />
      )}

      {activeTab === "apps" && (
        <AppManagerPanel
          selectedSerial={selectedSerial}
          selectedUserId={selectedUserId}
          inventory={inventory}
          ruleDbSummary={ruleDbSummary}
          recommendations={recommendations}
          selectedPackageName={selectedPackageName}
          packageDetails={packageDetails}
          busy={busy}
          inspectingPackage={inspectingPackage}
          onScanPackages={() => void handleScanPackages()}
          onSelectPackage={handleSelectPackage}
          onInspectPackageDetails={(pkgName) =>
            void handleInspectPackageDetails(pkgName)
          }
        />
      )}
    </main>
  );
}

export default App;
