use tauri::State;

use crate::android::{capabilities, device_specs};
use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::core::device_models::{DeviceCapabilities, DeviceSpecs};
use crate::core::errors::AdtError;
use crate::AppState;

#[tauri::command]
pub async fn get_device_specs(
    serial: String,
    state: State<'_, AppState>,
) -> Result<DeviceSpecs, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    device_specs::get_device_specs(&state.service, &adb_path, &serial, &discovered_devices).await
}

#[tauri::command]
pub async fn get_device_capabilities(
    serial: String,
    state: State<'_, AppState>,
) -> Result<DeviceCapabilities, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    capabilities::probe_device_capabilities(&state.service, &adb_path, &serial, &discovered_devices)
        .await
}
