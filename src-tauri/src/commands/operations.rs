use std::collections::HashMap;
use std::sync::PoisonError;

use tauri::State;

use crate::adb::validation::validate_device_serial;
use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::core::errors::AdtError;
use crate::operations::models::{
    BulkOperationPreview, BulkOperationResult, DeviceProtectionProfile, ExecuteOperationsRequest,
    PackageOperationResult, PackageSnapshotStatus, PreviewOperationsRequest,
};
use crate::operations::{executor, planner, protections, recovery};
use crate::AppState;

pub(crate) fn ensure_device_id_for_serial(
    serial: &str,
    state: &State<'_, AppState>,
) -> Result<String, AdtError> {
    validate_device_serial(serial)?;
    {
        let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = cache.serial_to_device_id.get(serial) {
            return Ok(existing.clone());
        }
    }

    let record = state
        .storage
        .upsert_device_profile(None, None, None, None, None, None)?;
    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        cache
            .serial_to_device_id
            .insert(serial.to_string(), record.device_id.clone());
    }
    Ok(record.device_id)
}

#[tauri::command]
pub async fn get_device_protections(
    serial: String,
    state: State<'_, AppState>,
) -> Result<DeviceProtectionProfile, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    protections::probe_device_protections(&state.service, &adb_path, &serial, &discovered_devices)
        .await
}

#[tauri::command]
pub async fn preview_package_operations(
    request: PreviewOperationsRequest,
    state: State<'_, AppState>,
) -> Result<BulkOperationPreview, AdtError> {
    let (adb_path, discovered_devices) =
        resolve_adb_and_recent_devices(&request.serial, &state).await?;
    let device_id = ensure_device_id_for_serial(&request.serial, &state)?;
    let profile = protections::probe_device_protections(
        &state.service,
        &adb_path,
        &request.serial,
        &discovered_devices,
    )
    .await?;

    planner::preview_package_operations(&request, &device_id, &profile, &state.storage)
}

#[tauri::command]
pub async fn execute_package_operations(
    request: ExecuteOperationsRequest,
    state: State<'_, AppState>,
) -> Result<BulkOperationResult, AdtError> {
    if !request.confirmed {
        return Err(AdtError::ConfirmationRequired);
    }

    let (adb_path, discovered_devices) =
        resolve_adb_and_recent_devices(&request.serial, &state).await?;
    let device_id = ensure_device_id_for_serial(&request.serial, &state)?;

    executor::execute_package_operations(
        &state.service,
        &adb_path,
        &discovered_devices,
        &state.device_locks,
        &state.storage,
        &device_id,
        &request,
    )
    .await
}

#[tauri::command]
pub fn get_package_snapshots(
    serial: String,
    user_id: u32,
    state: State<'_, AppState>,
) -> Result<HashMap<String, PackageSnapshotStatus>, AdtError> {
    let device_id = ensure_device_id_for_serial(&serial, &state)?;
    state
        .storage
        .list_snapshot_statuses_for_device(&device_id, user_id)
}

#[tauri::command]
pub fn get_session_operations(
    serial: String,
    state: State<'_, AppState>,
) -> Result<Vec<PackageOperationResult>, AdtError> {
    let device_id = ensure_device_id_for_serial(&serial, &state)?;
    state
        .storage
        .list_operation_history(Some(&device_id), None, None, 200)
}

#[tauri::command]
pub async fn reconcile_uncertain_operation(
    serial: String,
    operation_id: String,
    state: State<'_, AppState>,
) -> Result<PackageOperationResult, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    recovery::reconcile_uncertain_operation(
        &state.service,
        &adb_path,
        &discovered_devices,
        &state.device_locks,
        &state.storage,
        &serial,
        &operation_id,
    )
    .await
}
