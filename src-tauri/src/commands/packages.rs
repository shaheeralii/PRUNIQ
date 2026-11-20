use tauri::State;

use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::commands::operations::ensure_device_id_for_serial;
use crate::core::errors::AdtError;
use crate::core::package_models::{PackageDetails, PackageInventory};
use crate::packages::scanner;
use crate::AppState;

#[tauri::command]
pub async fn scan_installed_packages(
    serial: String,
    user_id: Option<u32>,
    state: State<'_, AppState>,
) -> Result<PackageInventory, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    let inventory = scanner::scan_installed_packages(
        &state.service,
        &adb_path,
        &serial,
        &discovered_devices,
        user_id,
    )
    .await?;

    if let Some(uid) = user_id {
        let device_id = ensure_device_id_for_serial(&serial, &state)?;
        let _ = state
            .storage
            .cache_scanned_packages(&device_id, uid, &inventory.packages);
    }

    Ok(inventory)
}

#[tauri::command]
pub async fn inspect_package_details(
    serial: String,
    package_name: String,
    user_id: Option<u32>,
    state: State<'_, AppState>,
) -> Result<PackageDetails, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    scanner::inspect_package_details(
        &state.service,
        &adb_path,
        &serial,
        &discovered_devices,
        &package_name,
        user_id,
    )
    .await
}
