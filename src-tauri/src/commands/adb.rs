use std::path::PathBuf;
use std::sync::PoisonError;
use std::time::Instant;

use tauri::State;

use crate::adb::detection::resolve_adb_executable;
use crate::adb::validation::validate_device_serial;
use crate::android::device_info;
use crate::core::errors::AdtError;
use crate::core::models::{AdbDevice, AdbSource, AdbStatus, BasicDeviceInfo};
use crate::{AppState, DEVICE_CACHE_TTL};

/// Detects ADB via an optional user-configured executable path or the system PATH,
/// verifies availability by running `adb version`, and returns structured status.
#[tauri::command]
pub async fn detect_adb(
    custom_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<AdbStatus, AdtError> {
    let normalized_custom = custom_path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);

    let resolved = match resolve_adb_executable(normalized_custom.as_deref()) {
        Ok(pair) => pair,
        Err(AdtError::AdbNotFound) => {
            let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
            cache.custom_adb_path = None;
            cache.resolved_adb_path = None;
            return Ok(AdbStatus {
                available: false,
                executable_path: None,
                source: AdbSource::NotFound,
                adb_version: None,
                platform_tools_version: None,
                installed_as: None,
                guidance: AdtError::AdbNotFound.guidance().map(str::to_string),
            });
        }
        Err(err) => return Err(err),
    };

    let (adb_path, source) = resolved;
    let version_info = state.service.get_version(&adb_path).await?;

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        cache.custom_adb_path = normalized_custom;
        cache.resolved_adb_path = Some(adb_path.clone());
    }

    Ok(AdbStatus {
        available: true,
        executable_path: Some(adb_path.to_string_lossy().into_owned()),
        source,
        adb_version: Some(version_info.adb_version),
        platform_tools_version: version_info.platform_tools_version,
        installed_as: version_info.installed_as,
        guidance: None,
    })
}

/// Lists connected ADB devices via `adb devices -l` and refreshes the in-memory
/// discovered-device snapshot used to verify explicit device serials.
#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<AdbDevice>, AdtError> {
    let adb_path = resolve_cached_or_path_adb(&state)?;
    let devices = state.service.list_devices(&adb_path).await?;

    {
        let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
        cache.resolved_adb_path = Some(adb_path);
        cache.discovered_devices = devices.clone();
        cache.discovered_at = Some(Instant::now());
    }

    Ok(devices)
}

/// Retrieves basic device identification using only the four Phase 1 allow-listed
/// `getprop` queries. Requires an explicit opaque serial verified against a recently
/// discovered authorized device.
#[tauri::command]
pub async fn get_basic_device_info(
    serial: String,
    state: State<'_, AppState>,
) -> Result<BasicDeviceInfo, AdtError> {
    validate_device_serial(&serial)?;
    let adb_path = resolve_cached_or_path_adb(&state)?;

    let recent_devices = {
        let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
        let is_fresh = cache
            .discovered_at
            .is_some_and(|t| t.elapsed() <= DEVICE_CACHE_TTL);
        if is_fresh {
            Some(cache.discovered_devices.clone())
        } else {
            None
        }
    };

    let discovered_devices = match recent_devices {
        Some(list) => list,
        None => {
            let refreshed = state.service.list_devices(&adb_path).await?;
            let mut cache = state.cache.write().unwrap_or_else(PoisonError::into_inner);
            cache.discovered_devices = refreshed.clone();
            cache.discovered_at = Some(Instant::now());
            refreshed
        }
    };

    device_info::get_basic_device_info(&state.service, &adb_path, &serial, &discovered_devices)
        .await
}

fn resolve_cached_or_path_adb(state: &State<'_, AppState>) -> Result<PathBuf, AdtError> {
    let custom = {
        let cache = state.cache.read().unwrap_or_else(PoisonError::into_inner);
        if let Some(resolved) = cache.resolved_adb_path.as_ref().filter(|p| p.is_file()) {
            return Ok(resolved.clone());
        }
        cache.custom_adb_path.clone()
    };

    let (adb_path, _) = resolve_adb_executable(custom.as_deref())?;
    Ok(adb_path)
}
