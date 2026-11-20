import type { ConfiguredPackageState, EffectivePackageState } from "./packages";

export type PackageAction =
  | "disable"
  | "enable"
  | "keep_current"
  | "restore_default_state"
  | "restore_original_state"
  | "undo_last";

export type ProtectionReason =
  | "immutable_system_component"
  | "active_default_launcher"
  | "only_usable_launcher"
  | "active_default_keyboard"
  | "only_usable_keyboard"
  | "launcher_probe_failed_conservative"
  | "keyboard_probe_failed_conservative";

export interface DeviceProtectionProfile {
  serial: string;
  immutablePackages: string[];
  activeLauncher: string | null;
  usableLaunchers: string[];
  activeKeyboard: string | null;
  enabledKeyboards: string[];
  launcherDetectionUncertain: boolean;
  keyboardDetectionUncertain: boolean;
  warnings: string[];
}

export interface CapturedPackageState {
  effectiveState: EffectivePackageState;
  configuredState: ConfiguredPackageState;
  configuredStateCode: number | null;
  installedForUser: boolean | null;
}

export interface PackageStateSnapshot {
  snapshotId: string;
  deviceId: string;
  userId: number;
  packageName: string;
  snapshotType: "initial_baseline" | "pre_operation";
  effectiveState: EffectivePackageState;
  configuredState: ConfiguredPackageState;
  configuredStateCode: number | null;
  androidVersion: string | null;
  capturedAtMs: number;
}

export interface PackageSnapshotStatus {
  packageName: string;
  baseline: PackageStateSnapshot | null;
  latestPreOperation: PackageStateSnapshot | null;
  preOperationCount: number;
}

export type OperationStatus =
  | "succeeded"
  | "no_op"
  | "blocked_by_protection"
  | "unsupported"
  | "uncertain_timeout"
  | "verification_mismatch"
  | "failed";

export type RestorationFidelity =
  | "not_applicable"
  | "exact"
  | "effective_match_only"
  | "unverified"
  | "failed";

export interface PackageActionRequest {
  packageName: string;
  action: PackageAction;
}

export interface CapabilityFlags {
  pmDefaultStateSupported: boolean;
  pmDisableUntilUsedSupported: boolean;
}

export interface PreviewOperationsRequest {
  serial: string;
  userId: number;
  items: PackageActionRequest[];
  capabilities: CapabilityFlags;
}

export interface ExecuteOperationsRequest {
  serial: string;
  userId: number;
  items: PackageActionRequest[];
  confirmed: boolean;
  androidVersion: string | null;
  capabilities: CapabilityFlags;
}

export interface PlannedPackageAction {
  packageName: string;
  action: PackageAction;
  allowed: boolean;
  isNoop: boolean;
  blockedReasons: ProtectionReason[];
  messages: string[];
  hasBaselineSnapshot: boolean;
  hasPreOpSnapshot: boolean;
}

export interface BulkOperationPreview {
  serial: string;
  userId: number;
  totalRequested: number;
  executableCount: number;
  blockedCount: number;
  noopCount: number;
  items: PlannedPackageAction[];
  warnings: string[];
}

export interface PackageOperationResult {
  operationId: string;
  deviceId: string;
  userId: number;
  packageName: string;
  action: PackageAction;
  status: OperationStatus;
  previousState: CapturedPackageState | null;
  verifiedState: CapturedPackageState | null;
  restorationFidelity: RestorationFidelity;
  blockedReasons: ProtectionReason[];
  limitationNote: string | null;
  errorMessage: string | null;
  executedAtMs: number;
}

export interface BulkOperationResult {
  serial: string;
  deviceId: string;
  userId: number;
  results: PackageOperationResult[];
  succeededCount: number;
  failedCount: number;
  uncertainCount: number;
  blockedCount: number;
  noopCount: number;
  abortedEarlyReason: string | null;
}
