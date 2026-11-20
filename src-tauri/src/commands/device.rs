use std::sync::PoisonError;

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
    let specs =
        device_specs::get_device_specs(&state.service, &adb_path, &serial, &discovered_devices)
            .await?;

    let existing_id = {
        let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
        cache.serial_to_device_id.get(&serial).cloned()
    };

    let record = state.storage.upsert_device_profile(
        existing_id.as_deref(),
        specs.identification.manufacturer.as_deref(),
        specs.identification.model.as_deref(),
        specs.identification.codename.as_deref(),
        specs.identification.android_version.as_deref(),
        specs.identification.api_level,
    )?;

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        cache.serial_to_device_id.insert(serial, record.device_id);
    }

    Ok(specs)
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
