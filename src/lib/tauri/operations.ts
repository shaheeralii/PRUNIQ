import { invoke } from "@tauri-apps/api/core";
import type {
  BulkOperationPreview,
  BulkOperationResult,
  DeviceProtectionProfile,
  ExecuteOperationsRequest,
  PackageOperationResult,
  PackageSnapshotStatus,
  PreviewOperationsRequest,
} from "../../types/operations";

export async function getDeviceProtections(
  serial: string,
): Promise<DeviceProtectionProfile> {
  return invoke<DeviceProtectionProfile>("get_device_protections", { serial });
}

export async function previewPackageOperations(
  request: PreviewOperationsRequest,
): Promise<BulkOperationPreview> {
  return invoke<BulkOperationPreview>("preview_package_operations", { request });
}

export async function executePackageOperations(
  request: ExecuteOperationsRequest,
): Promise<BulkOperationResult> {
  return invoke<BulkOperationResult>("execute_package_operations", { request });
}

export async function getPackageSnapshots(
  serial: string,
  userId: number,
): Promise<Record<string, PackageSnapshotStatus>> {
  return invoke<Record<string, PackageSnapshotStatus>>("get_package_snapshots", {
    serial,
    userId,
  });
}

export async function getSessionOperations(
  serial: string,
): Promise<PackageOperationResult[]> {
  return invoke<PackageOperationResult[]>("get_session_operations", { serial });
}
