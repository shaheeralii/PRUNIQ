import { invoke } from "@tauri-apps/api/core";
import type { AdbDevice, AdbStatus, AdtErrorPayload, BasicDeviceInfo } from "../../types/adb";

export function parseAdtError(err: unknown): AdtErrorPayload {
  if (typeof err === "object" && err !== null && "message" in err && "code" in err) {
    const candidate = err as Partial<AdtErrorPayload>;
    return {
      code: String(candidate.code ?? "unknown_error"),
      message: String(candidate.message ?? "An unexpected error occurred."),
      guidance: typeof candidate.guidance === "string" ? candidate.guidance : null,
    };
  }
  return {
    code: "unknown_error",
    message: err instanceof Error ? err.message : String(err),
    guidance: null,
  };
}

export async function detectAdb(customPath?: string): Promise<AdbStatus> {
  const trimmed = customPath?.trim();
  return invoke<AdbStatus>("detect_adb", {
    customPath: trimmed ? trimmed : null,
  });
}

export async function listDevices(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>("list_devices");
}

export async function getBasicDeviceInfo(serial: string): Promise<BasicDeviceInfo> {
  return invoke<BasicDeviceInfo>("get_basic_device_info", { serial });
}
