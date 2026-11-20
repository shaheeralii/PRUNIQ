use std::sync::PoisonError;
use std::time::Instant;

use tauri::State;

use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::commands::operations::ensure_device_id_for_serial;
use crate::core::diagnostics_models::PerformanceMetrics;
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
    let started = Instant::now();
    let inventory = scanner::scan_installed_packages(
        &state.service,
        &adb_path,
        &serial,
        &discovered_devices,
        user_id,
    )
    .await?;
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        let perf = cache
            .performance_by_serial
            .entry(serial.clone())
            .or_insert_with(|| PerformanceMetrics {
                serial: serial.clone(),
                ..PerformanceMetrics::default()
            });
        perf.last_package_scan_ms = Some(duration_ms);
        perf.last_package_count = Some(inventory.total_count);
    }

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
