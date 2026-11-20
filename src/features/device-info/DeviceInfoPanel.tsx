import { AlertBanner } from "../../components/feedback/AlertBanner";
import { IconRefresh } from "../../components/shared/Icons";
import { StatusBadge, type BadgeTone } from "../../components/shared/StatusBadge";
import {
  formatCommandSupport,
  formatKb,
  formatSupportTier,
} from "../../lib/format";
import type {
  CommandSupport,
  DeviceCapabilities,
  DeviceSpecs,
} from "../../types/device";
import styles from "./DeviceInfoPanel.module.css";

interface DeviceInfoPanelProps {
  selectedSerial: string | null;
  specs: DeviceSpecs | null;
  capabilities: DeviceCapabilities | null;
  selectedUserId: number | null;
  onSelectUserId: (userId: number | null) => void;
  busy: boolean;
  onRefresh: () => void;
}

function supportTone(support: CommandSupport): BadgeTone {
  switch (support) {
    case "supported":
      return "ok";
    case "unsupported":
      return "warn";
    case "unknown":
      return "neutral";
  }
}

export function DeviceInfoPanel({
  selectedSerial,
  specs,
  capabilities,
  selectedUserId,
  onSelectUserId,
  busy,
  onRefresh,
}: DeviceInfoPanelProps) {
  if (!selectedSerial) {
    return (
      <section className={styles.card}>
        <h2 className={styles.heading}>Device Information & Capabilities</h2>
        <p className={styles.muted}>
          Select an authorized device from Devices & ADB to load read-only hardware specifications and capability probes.
        </p>
      </section>
    );
  }

  const allWarnings = [
    ...(specs?.warnings ?? []),
    ...(capabilities?.warnings ?? []),
  ];

  const ramUsedPct =
    specs?.hardware.memory.totalKb &&
    specs.hardware.memory.availableKb !== null &&
    specs.hardware.memory.totalKb > 0
      ? Math.min(
          100,
          Math.round(
            ((specs.hardware.memory.totalKb - specs.hardware.memory.availableKb) /
              specs.hardware.memory.totalKb) *
              100,
          ),
        )
      : null;

  const storagePct = specs?.hardware.internalStorage.usePercent ?? null;

  return (
    <div className={styles.stack}>
      <div className={styles.toolbar}>
        <p className={styles.muted}>
          Targeted device:{" "}
          <strong className={styles.mono}>{selectedSerial}</strong>
        </p>
        <button
          type="button"
          className={styles.button}
          disabled={busy}
          onClick={onRefresh}
        >
          <IconRefresh size={13} />
          <span>Refresh Specs & Capabilities</span>
        </button>
      </div>

      {capabilities?.userDetectionWarning && (
        <AlertBanner
          tone="warn"
          title="Android user profile detection notice"
          detail={capabilities.userDetectionWarning}
        />
      )}

      {allWarnings.length > 0 && (
        <AlertBanner
          tone="warn"
          title="Some read-only probes returned partial data"
          items={allWarnings}
        />
      )}

      {specs && (
        <div className={styles.grid}>
          <section className={styles.card}>
            <h2 className={styles.heading}>Identification</h2>
            <dl className={styles.kvGrid}>
              <dt>Manufacturer</dt>
              <dd>{specs.identification.manufacturer ?? "Unavailable"}</dd>
              <dt>Model</dt>
              <dd>{specs.identification.model ?? "Unavailable"}</dd>
              <dt>Codename</dt>
              <dd>{specs.identification.codename ?? "Unavailable"}</dd>
              <dt>Product</dt>
              <dd>{specs.identification.product ?? "Unavailable"}</dd>
              <dt>Android Version</dt>
              <dd>{specs.identification.androidVersion ?? "Unavailable"}</dd>
              <dt>API Level</dt>
              <dd>
                {specs.identification.apiLevel ??
                  specs.identification.rawApiLevel ??
                  "Unavailable"}
              </dd>
              <dt>Security Patch</dt>
              <dd>{specs.identification.securityPatch ?? "Unavailable"}</dd>
              <dt>Build Number</dt>
              <dd>{specs.identification.buildNumber ?? "Unavailable"}</dd>
              <dt>Kernel</dt>
              <dd>{specs.identification.kernelVersion ?? "Unavailable"}</dd>
              <dt>Fingerprint</dt>
              <dd>{specs.identification.buildFingerprint ?? "Unavailable"}</dd>
            </dl>
          </section>

          <section className={styles.card}>
            <h2 className={styles.heading}>Hardware, Memory & Storage</h2>

            {ramUsedPct !== null && (
              <div className={styles.usageBlock}>
                <div className={styles.usageRow}>
                  <span>RAM in use (estimated)</span>
                  <span className={styles.mono}>
                    {ramUsedPct}% · {formatKb(specs.hardware.memory.availableKb)} avail of{" "}
                    {formatKb(specs.hardware.memory.totalKb)}
                  </span>
                </div>
                <div className={styles.usageTrack}>
                  <div
                    className={styles.usageFill}
                    style={{ width: `${ramUsedPct}%` }}
                  />
                </div>
              </div>
            )}

            {storagePct !== null && (
              <div className={styles.usageBlock}>
                <div className={styles.usageRow}>
                  <span>Internal storage ({specs.hardware.internalStorage.mountPoint})</span>
                  <span className={styles.mono}>
                    {storagePct}% · {formatKb(specs.hardware.internalStorage.availableKb)} free of{" "}
                    {formatKb(specs.hardware.internalStorage.totalKb)}
                  </span>
                </div>
                <div className={styles.usageTrack}>
                  <div
                    className={styles.usageFill}
                    style={{ width: `${Math.min(100, storagePct)}%` }}
                  />
                </div>
              </div>
            )}

            <dl className={styles.kvGrid}>
              <dt>CPU Architecture</dt>
              <dd>{specs.hardware.cpu.architecture ?? "Unavailable"}</dd>
              <dt>Supported ABIs</dt>
              <dd>
                {specs.hardware.cpu.supportedAbis.length > 0
                  ? specs.hardware.cpu.supportedAbis.join(", ")
                  : "Unavailable"}
              </dd>
              <dt>CPU Model</dt>
              <dd>{specs.hardware.cpu.model ?? "Unavailable"}</dd>
              <dt>CPU Cores</dt>
              <dd>{specs.hardware.cpu.coreCount ?? "Unavailable"}</dd>
              <dt>RAM (Free / Cached)</dt>
              <dd>
                {formatKb(specs.hardware.memory.freeKb)} /{" "}
                {formatKb(specs.hardware.memory.cachedKb)}
              </dd>
              <dt>Display Size</dt>
              <dd>
                {specs.hardware.display.physicalSize ?? "Unavailable"}
                {specs.hardware.display.overrideSize
                  ? ` (override: ${specs.hardware.display.overrideSize})`
                  : ""}
              </dd>
              <dt>Display Density</dt>
              <dd>
                {specs.hardware.display.physicalDensity !== null
                  ? `${specs.hardware.display.physicalDensity} dpi`
                  : "Unavailable"}
                {specs.hardware.display.overrideDensity !== null
                  ? ` (override: ${specs.hardware.display.overrideDensity} dpi)`
                  : ""}
              </dd>
              <dt>Battery</dt>
              <dd>
                {specs.hardware.battery.level !== null
                  ? `${specs.hardware.battery.level}%`
                  : "Unavailable"}
                {specs.hardware.battery.temperatureC !== null
                  ? ` · ${specs.hardware.battery.temperatureC.toFixed(1)} °C`
                  : ""}
                {specs.hardware.battery.status
                  ? ` · ${specs.hardware.battery.status}`
                  : ""}
                {specs.hardware.battery.chargingSource
                  ? ` (${specs.hardware.battery.chargingSource})`
                  : ""}
              </dd>
            </dl>
          </section>
        </div>
      )}

      {capabilities && (
        <div className={styles.grid}>
          <section className={styles.card}>
            <h2 className={styles.heading}>Android User Profiles & Support Tier</h2>
            <dl className={styles.kvGrid}>
              <dt>Provisional Tier</dt>
              <dd>{formatSupportTier(capabilities.supportTier)}</dd>
              <dt>Detected Active User</dt>
              <dd>
                {capabilities.activeUserId !== null
                  ? `User ${capabilities.activeUserId}`
                  : "Undetected"}
              </dd>
              <dt>Target User Scope</dt>
              <dd>
                <select
                  className={styles.select}
                  aria-label="Android user profile scope for package operations"
                  value={selectedUserId === null ? "unscoped" : String(selectedUserId)}
                  onChange={(e) => {
                    const val = e.target.value;
                    onSelectUserId(val === "unscoped" ? null : Number(val));
                  }}
                >
                  <option value="unscoped">Device default (read-only scan only)</option>
                  {capabilities.users.map((u) => (
                    <option key={u.id} value={String(u.id)}>
                      User {u.id}: {u.name} {u.running ? "(running)" : ""}
                    </option>
                  ))}
                  {capabilities.activeUserId !== null &&
                    !capabilities.users.some((u) => u.id === capabilities.activeUserId) && (
                      <option value={String(capabilities.activeUserId)}>
                        User {capabilities.activeUserId} (active)
                      </option>
                    )}
                </select>
              </dd>
            </dl>
          </section>

          <section className={styles.card}>
            <h2 className={styles.heading}>Read-Only Capability Matrix</h2>
            <div>
              <div className={styles.capRow}>
                <span className={styles.mono}>pm list packages flags</span>
                <StatusBadge tone={supportTone(capabilities.pmListPackagesFlags)}>
                  {formatCommandSupport(capabilities.pmListPackagesFlags)}
                </StatusBadge>
              </div>
              <div className={styles.capRow}>
                <span className={styles.mono}>am get-current-user</span>
                <StatusBadge tone={supportTone(capabilities.amGetCurrentUser)}>
                  {formatCommandSupport(capabilities.amGetCurrentUser)}
                </StatusBadge>
              </div>
              <div className={styles.capRow}>
                <span className={styles.mono}>pm disable-user</span>
                <StatusBadge tone={supportTone(capabilities.pmDisableUser)}>
                  {formatCommandSupport(capabilities.pmDisableUser)}
                </StatusBadge>
              </div>
              <div className={styles.capRow}>
                <span className={styles.mono}>pm enable</span>
                <StatusBadge tone={supportTone(capabilities.pmEnable)}>
                  {formatCommandSupport(capabilities.pmEnable)}
                </StatusBadge>
              </div>
              <div className={styles.capRow}>
                <span className={styles.mono}>pm default-state</span>
                <StatusBadge tone={supportTone(capabilities.pmDefaultState)}>
                  {formatCommandSupport(capabilities.pmDefaultState)}
                </StatusBadge>
              </div>
              <div className={styles.capRow}>
                <span className={styles.mono}>cmd package install-existing</span>
                <StatusBadge tone={supportTone(capabilities.cmdPackageInstallExisting)}>
                  {formatCommandSupport(capabilities.cmdPackageInstallExisting)}
                </StatusBadge>
              </div>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
