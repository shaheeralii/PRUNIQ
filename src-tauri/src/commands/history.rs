use std::sync::PoisonError;

use tauri::State;

use crate::commands::operations::ensure_device_id_for_serial;
use crate::core::errors::AdtError;
use crate::diagnostics::export::{build_redacted_diagnostic_bundle, DiagnosticBundle};
use crate::operations::models::PackageOperationResult;
use crate::storage::models::AppSettings;
use crate::AppState;

#[tauri::command]
pub fn get_operation_history(
    serial: Option<String>,
    status_filter: Option<String>,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<PackageOperationResult>, AdtError> {
    let device_id = match serial.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(ensure_device_id_for_serial(s, &state)?),
        None => None,
    };

    state.storage.list_operation_history(
        device_id.as_deref(),
        status_filter.as_deref(),
        search.as_deref(),
        500,
    )
}

#[tauri::command]
pub fn clear_operation_history(
    serial: Option<String>,
    state: State<'_, AppState>,
) -> Result<usize, AdtError> {
    let device_id = match serial.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(ensure_device_id_for_serial(s, &state)?),
        None => None,
    };
    state.storage.clear_operation_history(device_id.as_deref())
}

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, AdtError> {
    state.storage.get_settings()
}

#[tauri::command]
pub fn save_app_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, AdtError> {
    let saved = state.storage.save_settings(&settings)?;
    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        cache.custom_adb_path = saved.custom_adb_path.as_deref().map(std::path::PathBuf::from);
        cache.resolved_adb_path = None;
    }
    Ok(saved)
}

#[tauri::command]
pub fn export_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticBundle, AdtError> {
    let (resolved_adb, active_serials) = {
        let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
        let resolved = cache
            .resolved_adb_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());
        let mut serials: Vec<String> = cache
            .discovered_devices
            .iter()
            .map(|d| d.serial.clone())
            .collect();
        for key in cache.serial_to_device_id.keys() {
            if !serials.contains(key) {
                serials.push(key.clone());
            }
        }
        (resolved, serials)
    };

    build_redacted_diagnostic_bundle(
        &state.storage,
        state.rule_engine.summary(),
        resolved_adb.as_deref(),
        &active_serials,
    )
}
