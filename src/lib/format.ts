import type { CommandSupport, SupportTier } from "../types/device";
import type {
  ConfiguredPackageState,
  EffectivePackageState,
  InstallPartition,
  InstalledPackage,
  PackageOrigin,
} from "../types/packages";

export function formatKb(kb: number | null | undefined): string {
  if (kb === null || kb === undefined || Number.isNaN(kb)) {
    return "Unavailable";
  }
  if (kb >= 1024 * 1024) {
    return `${(kb / (1024 * 1024)).toFixed(2)} GB`;
  }
  if (kb >= 1024) {
    return `${(kb / 1024).toFixed(1)} MB`;
  }
  return `${kb} KB`;
}

export function formatOrigin(origin: PackageOrigin): string {
  switch (origin) {
    case "system":
      return "System";
    case "oem":
      return "OEM";
    case "google":
      return "Google";
    case "third_party":
      return "Third-Party";
    case "unknown":
      return "Unknown";
  }
}

export function formatPartition(partition: InstallPartition): string {
  switch (partition) {
    case "system":
      return "/system";
    case "system_ext":
      return "/system_ext";
    case "product":
      return "/product";
    case "vendor":
      return "/vendor";
    case "odm":
      return "/odm";
    case "apex":
      return "/apex";
    case "data":
      return "/data";
    case "other":
      return "Other";
    case "unknown":
      return "Unknown";
  }
}

export function formatEffectiveState(state: EffectivePackageState): string {
  switch (state) {
    case "enabled":
      return "Enabled";
    case "disabled":
      return "Disabled";
    case "unknown":
      return "Unknown";
  }
}

export function formatConfiguredState(
  state: ConfiguredPackageState,
  code: number | null,
): string {
  const name = (() => {
    switch (state) {
      case "default":
        return "DEFAULT";
      case "enabled":
        return "ENABLED";
      case "disabled":
        return "DISABLED";
      case "disabled_user":
        return "DISABLED_USER";
      case "disabled_until_used":
        return "DISABLED_UNTIL_USED";
      case "unknown":
        return "UNKNOWN";
    }
  })();
  return code !== null ? `${name} (${code})` : name;
}

export function formatSupportTier(tier: SupportTier): string {
  switch (tier) {
    case "primary":
      return "Primary range (API 21+)";
    case "exploratory_legacy":
      return "Exploratory legacy range (API 19-20)";
    case "below_exploratory":
      return "Below exploratory range (< API 19)";
    case "unknown":
      return "Unknown API level";
  }
}

export function formatCommandSupport(support: CommandSupport): string {
  switch (support) {
    case "supported":
      return "Supported";
    case "unsupported":
      return "Not listed";
    case "unknown":
      return "Unknown";
  }
}

export interface ClientPackageFilter {
  search: string;
  origin: PackageOrigin | "all";
  state: EffectivePackageState | "all";
  partition: InstallPartition | "all";
  updatedSystemOnly: boolean;
}

export function filterInstalledPackages(
  packages: InstalledPackage[],
  filter: ClientPackageFilter,
): InstalledPackage[] {
  const needle = filter.search.trim().toLowerCase();

  return packages.filter((pkg) => {
    if (needle.length > 0) {
      const matchName = pkg.packageName.toLowerCase().includes(needle);
      const matchLabel = pkg.label.toLowerCase().includes(needle);
      if (!matchName && !matchLabel) {
        return false;
      }
    }
    if (filter.origin !== "all" && pkg.origin !== filter.origin) {
      return false;
    }
    if (filter.state !== "all" && pkg.effectiveState !== filter.state) {
      return false;
    }
    if (filter.partition !== "all" && pkg.partition !== filter.partition) {
      return false;
    }
    if (filter.updatedSystemOnly && !pkg.isUpdatedSystem) {
      return false;
    }
    return true;
  });
}
