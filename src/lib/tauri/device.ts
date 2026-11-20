import { invoke } from "@tauri-apps/api/core";
import type { DeviceCapabilities, DeviceSpecs } from "../../types/device";

export async function getDeviceSpecs(serial: string): Promise<DeviceSpecs> {
  return invoke<DeviceSpecs>("get_device_specs", { serial });
}

export async function getDeviceCapabilities(serial: string): Promise<DeviceCapabilities> {
  return invoke<DeviceCapabilities>("get_device_capabilities", { serial });
}
