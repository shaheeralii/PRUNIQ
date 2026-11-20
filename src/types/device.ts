export interface DeviceIdentification {
  manufacturer: string | null;
  model: string | null;
  codename: string | null;
  product: string | null;
  androidVersion: string | null;
  apiLevel: number | null;
  rawApiLevel: string | null;
  buildNumber: string | null;
  buildFingerprint: string | null;
  securityPatch: string | null;
  kernelVersion: string | null;
}

export interface CpuInfo {
  architecture: string | null;
  supportedAbis: string[];
  model: string | null;
  coreCount: number | null;
}

export interface MemoryInfo {
  totalKb: number | null;
  availableKb: number | null;
  freeKb: number | null;
  cachedKb: number | null;
}

export interface StorageInfo {
  mountPoint: string;
  totalKb: number | null;
  usedKb: number | null;
  availableKb: number | null;
  usePercent: number | null;
}

export interface DisplayInfo {
  physicalSize: string | null;
  overrideSize: string | null;
  physicalDensity: number | null;
  overrideDensity: number | null;
}

export interface BatteryInfo {
  level: number | null;
  scale: number | null;
  temperatureC: number | null;
  status: string | null;
  chargingSource: string | null;
}

export interface DeviceHardware {
  cpu: CpuInfo;
  memory: MemoryInfo;
  internalStorage: StorageInfo;
  display: DisplayInfo;
  battery: BatteryInfo;
}

export interface DeviceSpecs {
  serial: string;
  identification: DeviceIdentification;
  hardware: DeviceHardware;
  warnings: string[];
}

export type SupportTier = "primary" | "exploratory_legacy" | "below_exploratory" | "unknown";
export type CommandSupport = "supported" | "unsupported" | "unknown";

export interface AndroidUser {
  id: number;
  name: string;
  running: boolean;
}

export interface DeviceCapabilities {
  serial: string;
  androidVersion: string | null;
  apiLevel: number | null;
  supportTier: SupportTier;
  activeUserId: number | null;
  users: AndroidUser[];
  userDetectionWarning: string | null;
  pmListPackagesFlags: CommandSupport;
  pmDisableUser: CommandSupport;
  pmDisable: CommandSupport;
  pmDisableUntilUsed: CommandSupport;
  pmEnable: CommandSupport;
  pmDefaultState: CommandSupport;
  cmdPackageInstallExisting: CommandSupport;
  amGetCurrentUser: CommandSupport;
  warnings: string[];
}
