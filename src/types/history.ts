import type { PackageOperationResult } from "./operations";
import type { RuleDatabaseSummary } from "./rules";

export interface DeviceRecord {
  deviceId: string;
  manufacturer: string | null;
  model: string | null;
  codename: string | null;
  androidVersion: string | null;
  apiLevel: number | null;
  lastConnectionMs: number;
}

export interface AppSettings {
  customAdbPath: string | null;
  autoScanOnSelect: boolean;
  logLevel: "minimal" | "standard";
}

export interface DiagnosticAdbSummary {
  customPathConfigured: boolean;
  resolvedExecutableRedacted: string | null;
}

export interface DiagnosticBundle {
  schemaVersion: number;
  exportedAtMs: number;
  appVersion: string;
  operatingSystem: string;
  architecture: string;
  privacyGuarantees: string[];
  adb: DiagnosticAdbSummary;
  settings: AppSettings;
  ruleDatabase: RuleDatabaseSummary;
  devices: DeviceRecord[];
  recentOperations: PackageOperationResult[];
}
