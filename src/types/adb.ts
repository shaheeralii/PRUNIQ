export type AdbSource = "custom" | "path" | "not_found";

export interface AdbStatus {
  available: boolean;
  executablePath: string | null;
  source: AdbSource;
  adbVersion: string | null;
  platformToolsVersion: string | null;
  installedAs: string | null;
  guidance: string | null;
}

export type DeviceConnectionState =
  | "authorized"
  | "unauthorized"
  | "offline"
  | "no_permissions"
  | "other";

export interface AdbDevice {
  serial: string;
  state: DeviceConnectionState;
  rawState: string;
  product: string | null;
  model: string | null;
  device: string | null;
  transportId: string | null;
  usb: string | null;
  guidance: string | null;
}

export interface BasicDeviceInfo {
  serial: string;
  manufacturer: string | null;
  model: string | null;
  androidVersion: string | null;
  apiLevel: number | null;
  rawApiLevel: string | null;
}

export interface AdtErrorPayload {
  code: string;
  message: string;
  guidance: string | null;
}
