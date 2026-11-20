use std::collections::HashMap;
use std::sync::PoisonError;

use tauri::State;

use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::core::errors::AdtError;
use crate::operations::models::{
    BulkOperationPreview, BulkOperationResult, DeviceProtectionProfile, ExecuteOperationsRequest,
    PackageOperationResult, PackageSnapshotStatus, PreviewOperationsRequest,
};
use crate::operations::{executor, planner, protections};
use crate::AppState;

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
    let profile = protections::probe_device_protections(
        &state.service,
        &adb_path,
        &request.serial,
        &discovered_devices,
    )
    .await?;

    let snapshots = state
        .snapshots
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    planner::preview_package_operations(&request, &profile, &snapshots)
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

    executor::execute_package_operations(
        &state.service,
        &adb_path,
        &discovered_devices,
        &state.device_locks,
        &state.snapshots,
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
    crate::adb::validation::validate_device_serial(&serial)?;
    let store = state
        .snapshots
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    Ok(store.list_statuses_for_device(&serial, user_id))
}

#[tauri::command]
pub fn get_session_operations(
    serial: String,
    state: State<'_, AppState>,
) -> Result<Vec<PackageOperationResult>, AdtError> {
    crate::adb::validation::validate_device_serial(&serial)?;
    let store = state
        .snapshots
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    Ok(store.list_session_operations(&serial))
}
