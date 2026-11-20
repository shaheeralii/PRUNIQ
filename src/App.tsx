import { useCallback, useEffect, useState } from "react";
import {
  detectAdb,
  getBasicDeviceInfo,
  listDevices,
  parseAdtError,
} from "./lib/tauri/adb";
import type {
  AdbDevice,
  AdbStatus,
  AdtErrorPayload,
  BasicDeviceInfo,
} from "./types/adb";
import styles from "./styles/app.module.css";

export function App() {
  const [customPath, setCustomPath] = useState("");
  const [adbStatus, setAdbStatus] = useState<AdbStatus | null>(null);
  const [devices, setDevices] = useState<AdbDevice[]>([]);
  const [selectedSerial, setSelectedSerial] = useState<string | null>(null);
  const [deviceInfo, setDeviceInfo] = useState<BasicDeviceInfo | null>(null);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<AdtErrorPayload | null>(null);

  const handleDetectAdb = useCallback(async (pathOverride?: string) => {
    setError(null);
    setLoading("Checking ADB availability...");
    try {
      const status = await detectAdb(pathOverride);
      setAdbStatus(status);
      if (!status.available) {
        setDevices([]);
        setSelectedSerial(null);
        setDeviceInfo(null);
      }
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoading(null);
    }
  }, []);

  useEffect(() => {
    void handleDetectAdb();
  }, [handleDetectAdb]);

  const handleRefreshDevices = async () => {
    setError(null);
    setLoading("Scanning connected ADB devices...");
    try {
      const found = await listDevices();
      setDevices(found);
      if (selectedSerial && !found.some((d) => d.serial === selectedSerial)) {
        setSelectedSerial(null);
        setDeviceInfo(null);
      }
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoading(null);
    }
  };

  const handleInspectDevice = async (serial: string) => {
    setError(null);
    setSelectedSerial(serial);
    setDeviceInfo(null);
    setLoading(`Reading basic device properties for ${serial}...`);
    try {
      const info = await getBasicDeviceInfo(serial);
      setDeviceInfo(info);
    } catch (err) {
      setError(parseAdtError(err));
    } finally {
      setLoading(null);
    }
  };

  return (
    <main className={styles.container}>
      <header className={styles.header}>
        <h1 className={styles.title}>Android Debloating Toolkit</h1>
        <span className={styles.badge}>Phase 1 â€” Read-Only ADB Foundation</span>
      </header>

      {error && (
        <div className={styles.alertError} role="alert">
          <strong>{error.message}</strong>
          {error.guidance && <p className={styles.muted}>{error.guidance}</p>}
        </div>
      )}

      <section className={styles.card}>
        <div className={styles.cardHeader}>
          <h2 className={styles.sectionTitle}>1. ADB Environment</h2>
          <button
            type="button"
            className={styles.button}
            disabled={loading !== null}
            onClick={() => void handleDetectAdb(customPath)}
          >
            Check ADB
          </button>
        </div>

        <div className={styles.row}>
          <input
            type="text"
            className={styles.input}
            placeholder="Optional custom path to adb.exe (leave empty to detect via PATH)"
            value={customPath}
            onChange={(e) => setCustomPath(e.target.value)}
            aria-label="Custom ADB executable path"
          />
        </div>

        {adbStatus && adbStatus.available ? (
          <dl className={styles.kvGrid}>
            <dt>Status</dt>
            <dd>Available ({adbStatus.source})</dd>
            <dt>Executable</dt>
            <dd>{adbStatus.executablePath ?? "â€”"}</dd>
            <dt>ADB Version</dt>
            <dd>{adbStatus.adbVersion ?? "â€”"}</dd>
            <dt>Platform-Tools</dt>
            <dd>{adbStatus.platformToolsVersion ?? "â€”"}</dd>
          </dl>
        ) : adbStatus ? (
          <div className={styles.alertWarn}>
            <strong>ADB executable not found.</strong>
            {adbStatus.guidance && <p className={styles.muted}>{adbStatus.guidance}</p>}
          </div>
        ) : null}
      </section>

      <section className={styles.card}>
        <div className={styles.cardHeader}>
          <h2 className={styles.sectionTitle}>2. Connected Devices</h2>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={loading !== null || !adbStatus?.available}
            onClick={() => void handleRefreshDevices()}
          >
            Refresh Devices
          </button>
        </div>

        {devices.length === 0 ? (
          <p className={styles.muted}>
            No devices listed yet. Connect an Android device with USB debugging enabled and click Refresh Devices.
          </p>
        ) : (
          <ul className={styles.deviceList}>
            {devices.map((device) => {
              const isSelected = device.serial === selectedSerial;
              const isAuthorized = device.state === "authorized";
              return (
                <li
                  key={device.serial}
                  className={`${styles.deviceItem} ${isSelected ? styles.deviceSelected : ""}`}
                >
                  <div className={styles.deviceMeta}>
                    <span className={styles.serial}>{device.serial}</span>
                    <span className={styles.muted}>
                      {[device.model, device.device, device.transportId ? `transport ${device.transportId}` : null]
                        .filter(Boolean)
                        .join(" Â· ") || "No extended metadata"}
                    </span>
                    {device.guidance && <span className={styles.muted}>{device.guidance}</span>}
                  </div>
                  <div className={styles.row}>
                    <span
                      className={`${styles.stateTag} ${
                        isAuthorized ? styles.stateAuthorized : styles.stateWarning
                      }`}
                    >
                      {device.rawState}
                    </span>
                    <button
                      type="button"
                      className={styles.button}
                      disabled={loading !== null || !isAuthorized}
                      onClick={() => void handleInspectDevice(device.serial)}
                    >
                      Inspect
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section className={styles.card}>
        <h2 className={styles.sectionTitle}>3. Basic Device Information (Read-Only)</h2>
        {deviceInfo ? (
          <dl className={styles.kvGrid}>
            <dt>Targeted Serial</dt>
            <dd>{deviceInfo.serial}</dd>
            <dt>Manufacturer</dt>
            <dd>{deviceInfo.manufacturer ?? "Unavailable"}</dd>
            <dt>Model</dt>
            <dd>{deviceInfo.model ?? "Unavailable"}</dd>
            <dt>Android Version</dt>
            <dd>{deviceInfo.androidVersion ?? "Unavailable"}</dd>
            <dt>API Level</dt>
            <dd>{deviceInfo.apiLevel ?? deviceInfo.rawApiLevel ?? "Unavailable"}</dd>
          </dl>
        ) : (
          <p className={styles.muted}>
            Select an authorized device and click Inspect to query manufacturer, model, Android version, and API level.
          </p>
        )}
      </section>

      {loading && <p className={styles.muted}>{loading}</p>}
    </main>
  );
}

export default App;
