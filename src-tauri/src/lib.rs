pub mod adb;
pub mod android;
pub mod commands;
pub mod core;
pub mod diagnostics;
pub mod operations;
pub mod packages;
pub mod rules;
pub mod storage;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use tauri::Manager;

use adb::process_runner::TokioProcessRunner;
use adb::service::AdbService;
use core::models::AdbDevice;
use operations::locks::DeviceLockManager;
use rules::engine::RuleEngine;
use storage::db::Storage;

/// Maximum age for the cached read-only device discovery snapshot before
/// device-scoped commands trigger a fresh read-only `adb devices -l` check.
pub const DEVICE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct SessionCache {
    pub custom_adb_path: Option<PathBuf>,
    pub resolved_adb_path: Option<PathBuf>,
    pub discovered_devices: Vec<AdbDevice>,
    pub discovered_at: Option<Instant>,
    /// In-memory only mapping from active ADB serial to local random SQLite device UUID.
    pub serial_to_device_id: HashMap<String, String>,
}

pub struct AppState {
    pub service: AdbService<TokioProcessRunner>,
    pub cache: RwLock<SessionCache>,
    pub rule_engine: RuleEngine,
    pub device_locks: DeviceLockManager,
    pub storage: Storage,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        let initial_custom_adb = storage
            .get_settings()
            .ok()
            .and_then(|s| s.custom_adb_path)
            .map(PathBuf::from);

        let cache = SessionCache {
            custom_adb_path: initial_custom_adb,
            ..SessionCache::default()
        };

        Self {
            service: AdbService::new(TokioProcessRunner),
            cache: RwLock::new(cache),
            rule_engine: RuleEngine::from_embedded()
                .expect("bundled package-rules/rules.json must be valid"),
            device_locks: DeviceLockManager::new(),
            storage,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("adt.db");
            let storage = Storage::open(&db_path)
                .map_err(|err| std::io::Error::other(err.to_string()))?;
            app.manage(AppState::new(storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::adb::detect_adb,
            commands::adb::list_devices,
            commands::adb::get_basic_device_info,
            commands::device::get_device_specs,
            commands::device::get_device_capabilities,
            commands::packages::scan_installed_packages,
            commands::packages::inspect_package_details,
            commands::rules::get_rule_database_summary,
            commands::rules::evaluate_recommendations,
            commands::operations::get_device_protections,
            commands::operations::preview_package_operations,
            commands::operations::execute_package_operations,
            commands::operations::get_package_snapshots,
            commands::operations::get_session_operations,
            commands::operations::reconcile_uncertain_operation,
            commands::history::get_operation_history,
            commands::history::clear_operation_history,
            commands::history::get_app_settings,
            commands::history::save_app_settings,
            commands::history::export_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Android Debloating Toolkit");
}
