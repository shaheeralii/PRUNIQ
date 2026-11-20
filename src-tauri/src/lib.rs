pub mod adb;
pub mod android;
pub mod commands;
pub mod core;
pub mod packages;
pub mod rules;

use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use adb::process_runner::TokioProcessRunner;
use adb::service::AdbService;
use core::models::AdbDevice;
use rules::engine::RuleEngine;

/// Maximum age for the cached read-only device discovery snapshot before
/// device-scoped commands trigger a fresh read-only `adb devices -l` check.
pub const DEVICE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct SessionCache {
    pub custom_adb_path: Option<PathBuf>,
    pub resolved_adb_path: Option<PathBuf>,
    pub discovered_devices: Vec<AdbDevice>,
    pub discovered_at: Option<Instant>,
}

pub struct AppState {
    pub service: AdbService<TokioProcessRunner>,
    pub cache: RwLock<SessionCache>,
    pub rule_engine: RuleEngine,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            service: AdbService::new(TokioProcessRunner),
            cache: RwLock::new(SessionCache::default()),
            rule_engine: RuleEngine::from_embedded()
                .expect("bundled package-rules/rules.json must be valid"),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Android Debloating Toolkit");
}
