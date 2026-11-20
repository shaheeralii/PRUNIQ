export type PackageOrigin = "system" | "oem" | "google" | "third_party" | "unknown";

export type InstallPartition =
  | "system"
  | "system_ext"
  | "product"
  | "vendor"
  | "odm"
  | "apex"
  | "data"
  | "other"
  | "unknown";

export type EffectivePackageState = "enabled" | "disabled" | "unknown";

export type ConfiguredPackageState =
  | "default"
  | "enabled"
  | "disabled"
  | "disabled_user"
  | "disabled_until_used"
  | "unknown";

export type LabelSource = "curated_catalog" | "package_name_fallback";

export interface InstalledPackage {
  packageName: string;
  label: string;
  labelSource: LabelSource;
  apkPath: string | null;
  effectiveState: EffectivePackageState;
  origin: PackageOrigin;
  partition: InstallPartition;
  isSystem: boolean;
  isThirdParty: boolean;
  isUpdatedSystem: boolean;
  isGoogle: boolean;
  isOem: boolean;
  userId: number | null;
}

export interface PackageDetails {
  packageName: string;
  label: string;
  labelSource: LabelSource;
  versionName: string | null;
  versionCode: number | null;
  codePath: string | null;
  partition: InstallPartition;
  isSystemFlag: boolean | null;
  isUpdatedSystemFlag: boolean | null;
  observedUserId: number | null;
  installedForUser: boolean | null;
  configuredState: ConfiguredPackageState;
  configuredStateCode: number | null;
}

export interface PackageInventory {
  serial: string;
  queriedUserId: number | null;
  totalCount: number;
  enabledCount: number;
  disabledCount: number;
  systemCount: number;
  thirdPartyCount: number;
  packages: InstalledPackage[];
  warnings: string[];
}
