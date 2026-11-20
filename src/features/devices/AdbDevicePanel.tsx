import { AlertBanner } from "../../components/feedback/AlertBanner";
import { StatusBadge } from "../../components/shared/StatusBadge";
import type { AdbDevice, AdbStatus } from "../../types/adb";
import styles from "./AdbDevicePanel.module.css";

interface AdbDevicePanelProps {
  customAdbPath: string;
  onCustomAdbPathChange: (value: string) => void;
  adbStatus: AdbStatus | null;
  devices: AdbDevice[];
  selectedSerial: string | null;
  busy: boolean;
  onCheckAdb: () => void;
  onRefreshDevices: () => void;
  onSelectDevice: (device: AdbDevice) => void;
}

export function AdbDevicePanel({
  customAdbPath,
  onCustomAdbPathChange,
  adbStatus,
  devices,
  selectedSerial,
  busy,
  onCheckAdb,
  onRefreshDevices,
  onSelectDevice,
}: AdbDevicePanelProps) {
  return (
    <div className={styles.stack}>
      <section className={styles.card}>
        <div className={styles.cardHeader}>
          <h2 className={styles.heading}>ADB Environment</h2>
          <div className={styles.row}>
            {adbStatus && (
              <StatusBadge tone={adbStatus.available ? "ok" : "warn"}>
                {adbStatus.available ? `Available (${adbStatus.source})` : "Not found"}
              </StatusBadge>
            )}
            <button
              type="button"
              className={styles.button}
              disabled={busy}
              onClick={onCheckAdb}
            >
              Check ADB
            </button>
          </div>
        </div>

        <div className={styles.row}>
          <input
            type="text"
            className={styles.input}
            placeholder="Optional custom path to adb.exe (leave blank to use system PATH)"
            value={customAdbPath}
            onChange={(e) => onCustomAdbPathChange(e.target.value)}
            aria-label="Custom ADB executable path"
          />
        </div>

        {adbStatus && adbStatus.available ? (
          <dl className={styles.kvGrid}>
            <dt>Executable</dt>
            <dd>{adbStatus.executablePath ?? "Unavailable"}</dd>
            <dt>ADB Version</dt>
            <dd>{adbStatus.adbVersion ?? "Unavailable"}</dd>
            <dt>Platform-Tools</dt>
            <dd>{adbStatus.platformToolsVersion ?? "Unavailable"}</dd>
          </dl>
        ) : adbStatus ? (
          <AlertBanner
            tone="warn"
            title="ADB executable not found."
            detail={adbStatus.guidance}
          />
        ) : null}
      </section>

      <section className={styles.card}>
        <div className={styles.cardHeader}>
          <h2 className={styles.heading}>Connected Devices ({devices.length})</h2>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy || !adbStatus?.available}
            onClick={onRefreshDevices}
          >
            Refresh Devices
          </button>
        </div>

        {devices.length === 0 ? (
          <p className={styles.muted}>
            No devices detected. Enable USB debugging on your Android device, connect via USB, and click Refresh Devices.
          </p>
        ) : (
          <ul className={styles.deviceList}>
            {devices.map((device) => {
              const isSelected = device.serial === selectedSerial;
              const isAuthorized = device.state === "authorized";
              const metadata = [
                device.model,
                device.device,
                device.transportId ? `transport ${device.transportId}` : null,
              ]
                .filter(Boolean)
                .join(" Â· ");

              return (
                <li
                  key={device.serial}
                  className={`${styles.deviceItem} ${isSelected ? styles.deviceSelected : ""}`}
                >
                  <div className={styles.deviceMeta}>
                    <span className={styles.serial}>{device.serial}</span>
                    <span className={styles.muted}>
                      {metadata || "No extended transport metadata"}
                    </span>
                    {device.guidance && <span className={styles.muted}>{device.guidance}</span>}
                  </div>

                  <div className={styles.row}>
                    <StatusBadge tone={isAuthorized ? "ok" : "warn"}>
                      {device.rawState}
                    </StatusBadge>
                    <button
                      type="button"
                      className={`${styles.button} ${isSelected ? styles.buttonPrimary : ""}`}
                      disabled={busy || !isAuthorized}
                      onClick={() => onSelectDevice(device)}
                    >
                      {isSelected ? "Selected" : "Select Device"}
                    </button>
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
