import { invoke } from "@tauri-apps/api/core";
import type {
  MemoryComparison,
  MemorySnapshot,
  PerformanceMetrics,
  StorageDashboard,
} from "../../types/diagnostics";

export async function captureMemorySnapshot(
  serial: string,
  label?: string,
): Promise<MemorySnapshot> {
  return invoke<MemorySnapshot>("capture_memory_snapshot", {
    serial,
    label: label?.trim() ? label.trim() : null,
  });
}

export async function listMemorySnapshots(
  serial: string,
): Promise<MemorySnapshot[]> {
  return invoke<MemorySnapshot[]>("list_memory_snapshots", { serial });
}

export async function compareMemorySnapshots(
  serial: string,
  beforeId: string,
  afterId: string,
): Promise<MemoryComparison> {
  return invoke<MemoryComparison>("compare_memory_snapshots", {
    serial,
    beforeId,
    afterId,
  });
}

export async function getStorageDiagnostics(
  serial: string,
): Promise<StorageDashboard> {
  return invoke<StorageDashboard>("get_storage_diagnostics", { serial });
}

export async function getPerformanceMetrics(
  serial: string,
): Promise<PerformanceMetrics> {
  return invoke<PerformanceMetrics>("get_performance_metrics", { serial });
}
