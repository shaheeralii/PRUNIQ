import { invoke } from "@tauri-apps/api/core";
import type { PackageDetails, PackageInventory } from "../../types/packages";

export async function scanInstalledPackages(
  serial: string,
  userId?: number | null,
): Promise<PackageInventory> {
  return invoke<PackageInventory>("scan_installed_packages", {
    serial,
    userId: userId ?? null,
  });
}

export async function inspectPackageDetails(
  serial: string,
  packageName: string,
  userId?: number | null,
): Promise<PackageDetails> {
  return invoke<PackageDetails>("inspect_package_details", {
    serial,
    packageName,
    userId: userId ?? null,
  });
}
