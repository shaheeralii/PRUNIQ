import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, DiagnosticBundle } from "../../types/history";
import type { PackageOperationResult } from "../../types/operations";

export async function getOperationHistory(params: {
  serial?: string | null;
  statusFilter?: string | null;
  search?: string | null;
}): Promise<PackageOperationResult[]> {
  return invoke<PackageOperationResult[]>("get_operation_history", {
    serial: params.serial ?? null,
    statusFilter: params.statusFilter ?? null,
    search: params.search ?? null,
  });
}

export async function reconcileUncertainOperation(
  serial: string,
  operationId: string,
): Promise<PackageOperationResult> {
  return invoke<PackageOperationResult>("reconcile_uncertain_operation", {
    serial,
    operationId,
  });
}

export async function clearOperationHistory(
  serial?: string | null,
): Promise<number> {
  return invoke<number>("clear_operation_history", {
    serial: serial ?? null,
  });
}

export async function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_app_settings");
}

export async function saveAppSettings(
  settings: AppSettings,
): Promise<AppSettings> {
  return invoke<AppSettings>("save_app_settings", { settings });
}

export async function exportDiagnostics(): Promise<DiagnosticBundle> {
  return invoke<DiagnosticBundle>("export_diagnostics");
}
