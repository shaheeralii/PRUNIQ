use std::sync::PoisonError;

use tauri::State;

use crate::adb::validation::validate_device_serial;
use crate::android::diagnostics;
use crate::commands::adb::resolve_adb_and_recent_devices;
use crate::core::diagnostics_models::{
    MemoryComparison, MemorySnapshot, PerformanceMetrics, StorageDashboard,
};
use crate::core::errors::AdtError;
use crate::AppState;

const MAX_SESSION_MEMORY_SNAPSHOTS: usize = 20;

#[tauri::command]
pub async fn capture_memory_snapshot(
    serial: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<MemorySnapshot, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    let snapshot = diagnostics::capture_memory_snapshot(
        &state.service,
        &adb_path,
        &serial,
        &discovered_devices,
        label.as_deref(),
    )
    .await?;

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        let list = cache.memory_snapshots.entry(serial.clone()).or_default();
        list.push(snapshot.clone());
        if list.len() > MAX_SESSION_MEMORY_SNAPSHOTS {
            let drain_count = list.len() - MAX_SESSION_MEMORY_SNAPSHOTS;
            list.drain(0..drain_count);
        }

        let perf = cache
            .performance_by_serial
            .entry(serial.clone())
            .or_insert_with(|| PerformanceMetrics {
                serial: serial.clone(),
                ..PerformanceMetrics::default()
            });
        perf.last_memory_probe_ms = Some(snapshot.capture_duration_ms);
    }

    Ok(snapshot)
}

#[tauri::command]
pub fn list_memory_snapshots(
    serial: String,
    state: State<'_, AppState>,
) -> Result<Vec<MemorySnapshot>, AdtError> {
    validate_device_serial(&serial)?;
    let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
    Ok(cache
        .memory_snapshots
        .get(&serial)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
pub fn compare_memory_snapshots(
    serial: String,
    before_id: String,
    after_id: String,
    state: State<'_, AppState>,
) -> Result<MemoryComparison, AdtError> {
    validate_device_serial(&serial)?;
    let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
    let snapshots = cache
        .memory_snapshots
        .get(&serial)
        .ok_or_else(|| AdtError::OperationNotFound {
            operation_id: before_id.clone(),
        })?;

    let before = snapshots
        .iter()
        .find(|s| s.snapshot_id == before_id)
        .ok_or_else(|| AdtError::OperationNotFound {
            operation_id: before_id.clone(),
        })?;
    let after = snapshots
        .iter()
        .find(|s| s.snapshot_id == after_id)
        .ok_or_else(|| AdtError::OperationNotFound {
            operation_id: after_id.clone(),
        })?;

    Ok(diagnostics::compare_memory_snapshots(before, after))
}

#[tauri::command]
pub async fn get_storage_diagnostics(
    serial: String,
    state: State<'_, AppState>,
) -> Result<StorageDashboard, AdtError> {
    let (adb_path, discovered_devices) = resolve_adb_and_recent_devices(&serial, &state).await?;
    let dashboard = diagnostics::get_storage_diagnostics(
        &state.service,
        &adb_path,
        &serial,
        &discovered_devices,
    )
    .await?;

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        let perf = cache
            .performance_by_serial
            .entry(serial.clone())
            .or_insert_with(|| PerformanceMetrics {
                serial: serial.clone(),
                ..PerformanceMetrics::default()
            });
        perf.last_storage_probe_ms = Some(dashboard.capture_duration_ms);
    }

    Ok(dashboard)
}

#[tauri::command]
pub fn get_performance_metrics(
    serial: String,
    state: State<'_, AppState>,
) -> Result<PerformanceMetrics, AdtError> {
    validate_device_serial(&serial)?;
    let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
    Ok(cache
        .performance_by_serial
        .get(&serial)
        .cloned()
        .unwrap_or(PerformanceMetrics {
            serial,
            ..PerformanceMetrics::default()
        }))
}
