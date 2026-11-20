import { AlertBanner } from "../../components/feedback/AlertBanner";
import { IconCheck, IconRefresh } from "../../components/shared/Icons";
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
              <StatusBadge
                tone={adbStatus.available ? "ok" : "warn"}
                icon={adbStatus.available ? <IconCheck size={12} /> : undefined}
              >
                {adbStatus.available
                  ? `Available (${adbStatus.source})`
                  : "Not found"}
              </StatusBadge>
            )}
            <button
              type="button"
              className={styles.button}
              disabled={busy}
              onClick={onCheckAdb}
            >
              <IconRefresh size={13} />
              <span>Check ADB</span>
            </button>
          </div>
        </div>

        <div className={styles.row}>
          <input
            type="text"
            className={styles.input}
            placeholder="Optional custom path to adb.exe (leave blank to detect via system PATH)"
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
            title="ADB executable not found on PATH."
            detail={adbStatus.guidance}
          />
        ) : null}
      </section>

      <section className={styles.card}>
        <div className={styles.cardHeader}>
          <h2 className={styles.heading}>
            Connected Devices ({devices.length})
          </h2>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy || !adbStatus?.available}
            onClick={onRefreshDevices}
          >
            <IconRefresh size={13} />
            <span>Refresh Devices</span>
          </button>
        </div>

        {devices.length === 0 ? (
          <div className={styles.onboarding}>
            <h3 className={styles.onboardingTitle}>
              Ready to connect an Android device via USB
            </h3>
            <ol className={styles.checklist}>
              <li>
                Enable <strong>Developer options</strong> and{" "}
                <strong>USB debugging</strong> in Android Settings.
              </li>
              <li>
                Connect your phone using a data-capable USB cable and switch USB
                mode to File Transfer (MTP) if needed.
              </li>
              <li>
                Unlock the phone screen and tap <strong>Allow</strong> on the
                RSA debugging authorization dialog.
              </li>
              <li>
                Click <strong>Refresh Devices</strong> above, then select your
                authorized device.
              </li>
            </ol>
          </div>
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
                .join(" · ");

              return (
                <li
                  key={device.serial}
                  className={`${styles.deviceItem} ${
                    isSelected ? styles.deviceSelected : ""
                  }`}
                >
                  <div className={styles.deviceMeta}>
                    <span className={styles.serial}>{device.serial}</span>
                    <span className={styles.muted}>
                      {metadata || "No extended transport metadata"}
                    </span>
                    {device.guidance && (
                      <span className={styles.muted}>{device.guidance}</span>
                    )}
                  </div>

                  <div className={styles.row}>
                    <StatusBadge tone={isAuthorized ? "ok" : "warn"}>
                      {device.rawState}
                    </StatusBadge>
                    <button
                      type="button"
                      className={`${styles.button} ${
                        isSelected ? styles.buttonPrimary : ""
                      }`}
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
