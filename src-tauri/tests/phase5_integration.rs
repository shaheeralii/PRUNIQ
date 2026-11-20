use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use adt_lib::adb::device_discovery::parse_devices_long;
use adt_lib::adb::process_runner::mock::MockProcessRunner;
use adt_lib::adb::process_runner::ProcessOutput;
use adt_lib::adb::service::AdbService;
use adt_lib::core::errors::AdtError;
use adt_lib::core::models::AdbDevice;
use adt_lib::core::package_models::{ConfiguredPackageState, EffectivePackageState};
use adt_lib::diagnostics::export::build_redacted_diagnostic_bundle;
use adt_lib::operations::executor::execute_package_operations;
use adt_lib::operations::locks::DeviceLockManager;
use adt_lib::operations::models::{
    CapabilityFlags, CapturedPackageState, ExecuteOperationsRequest, OperationStatus,
    PackageAction, PackageActionRequest, PackageOperationResult, RestorationFidelity,
};
use adt_lib::operations::recovery::reconcile_uncertain_operation;
use adt_lib::rules::engine::RuleEngine;
use adt_lib::storage::db::Storage;
use adt_lib::storage::models::AppSettings;

fn ok_out(stdout: impl Into<String>) -> Result<ProcessOutput, AdtError> {
    Ok(ProcessOutput {
        exit_code: Some(0),
        stdout: stdout.into(),
        stderr: String::new(),
    })
}

fn sample_devices() -> Vec<AdbDevice> {
    parse_devices_long("List of devices attached\nR58M91234AB device model:SM_A528B transport_id:1\n")
        .expect("devices fixture")
}

fn dumpsys_state(pkg: &str, enabled_code: i32) -> String {
    format!(
        "Packages:\n  Package [{pkg}] (111):\n    codePath=/system/priv-app/App\n    versionCode=10\n    versionName=1.0\n    pkgFlags=[ SYSTEM ]\n    User 0: installed=true hidden=false stopped=false enabled={enabled_code}\n"
    )
}

#[test]
fn migrations_are_idempotent_and_persist_across_reopen() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let temp_dir = env::temp_dir().join(format!("adt_phase5_db_{nanos}"));
    let db_path = temp_dir.join("adt.db");

    let device_id = {
        let storage = Storage::open(&db_path).expect("open db 1");
        assert_eq!(storage.current_schema_version().expect("ver 1"), 1);

        let dev = storage
            .upsert_device_profile(
                None,
                Some("samsung"),
                Some("SM-A528B"),
                Some("a52sxq"),
                Some("14"),
                Some(34),
            )
            .expect("upsert device");

        let state = CapturedPackageState {
            effective_state: EffectivePackageState::Enabled,
            configured_state: ConfiguredPackageState::Default,
            configured_state_code: Some(0),
            installed_for_user: Some(true),
        };
        storage
            .capture_baseline_if_absent(&dev.device_id, 0, "com.example.persist", &state, Some("14"))
            .expect("baseline");

        storage
            .save_settings(&AppSettings {
                custom_adb_path: Some("C:\\Tools\\platform-tools\\adb.exe".to_string()),
                auto_scan_on_select: true,
                log_level: "minimal".to_string(),
            })
            .expect("save settings");

        dev.device_id
    };

    // Reopen same database file (simulates full application restart + migration re-check).
    {
        let reopened = Storage::open(&db_path).expect("reopen db");
        assert_eq!(reopened.current_schema_version().expect("ver 2"), 1);

        let base = reopened
            .get_baseline(&device_id, 0, "com.example.persist")
            .expect("query baseline")
            .expect("baseline persisted");
        assert_eq!(base.configured_state, ConfiguredPackageState::Default);

        let settings = reopened.get_settings().expect("load settings");
        assert_eq!(
            settings.custom_adb_path.as_deref(),
            Some("C:\\Tools\\platform-tools\\adb.exe")
        );
        assert!(settings.auto_scan_on_select);
        assert_eq!(settings.log_level, "minimal");
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn timeout_fault_injection_persists_uncertain_and_reconciles_to_succeeded_via_read_only_probe() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let pkg = "com.example.timeout";

    let runner = MockProcessRunner::new(vec![
        // Protection probes
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        // Pre-state capture -> Timeout on disable-user
        ok_out(dumpsys_state(pkg, 0)),
        Err(AdtError::Timeout {
            command: "adb shell pm disable-user".to_string(),
            timeout_ms: 15_000,
        }),
        // Later reconciliation read-only check: device DID apply DISABLED_USER (3)
        ok_out(dumpsys_state(pkg, 3)),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let storage = Storage::open_in_memory().expect("db");
    let dev = storage
        .upsert_device_profile(None, Some("samsung"), Some("SM-A528B"), Some("a52sxq"), Some("14"), Some(34))
        .expect("dev");

    let exec_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &storage,
        &dev.device_id,
        &ExecuteOperationsRequest {
            serial: "R58M91234AB".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::Disable,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: CapabilityFlags {
                pm_default_state_supported: false,
                pm_disable_until_used_supported: false,
            },
        },
    )
    .await
    .expect("execute");

    let op_id = exec_res.results[0].operation_id.clone();
    assert_eq!(exec_res.results[0].status, OperationStatus::UncertainTimeout);

    // Reconcile strictly via read-only dumpsys inspection (no retry mutation!)
    let reconciled = reconcile_uncertain_operation(
        &service,
        adb,
        &devices,
        &locks,
        &storage,
        "R58M91234AB",
        &op_id,
    )
    .await
    .expect("reconcile");

    assert_eq!(reconciled.status, OperationStatus::Succeeded);
    assert_eq!(
        reconciled
            .verified_state
            .as_ref()
            .expect("verified")
            .configured_state,
        ConfiguredPackageState::DisabledUser
    );

    // Confirm persisted history reflects the reconciled state.
    let history = storage
        .list_operation_history(Some(&dev.device_id), None, None, 10)
        .expect("history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, OperationStatus::Succeeded);
    assert_eq!(service.runner().invocations().len(), 7);
}

#[tokio::test]
async fn disconnect_fault_injection_reconciles_as_mismatch_when_device_unchanged() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let pkg = "com.example.disconnect";

    let runner = MockProcessRunner::new(vec![
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out(dumpsys_state(pkg, 0)),
        Err(AdtError::DeviceDisconnected {
            serial: "R58M91234AB".to_string(),
        }),
        // After reconnect, device still reports DEFAULT (0)
        ok_out(dumpsys_state(pkg, 0)),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let storage = Storage::open_in_memory().expect("db");
    let dev = storage
        .upsert_device_profile(None, Some("samsung"), Some("SM-A528B"), Some("a52sxq"), Some("14"), Some(34))
        .expect("dev");

    let exec_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &storage,
        &dev.device_id,
        &ExecuteOperationsRequest {
            serial: "R58M91234AB".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::Disable,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: CapabilityFlags {
                pm_default_state_supported: false,
                pm_disable_until_used_supported: false,
            },
        },
    )
    .await
    .expect("execute");

    assert!(exec_res.aborted_early_reason.is_some());
    assert_eq!(exec_res.results[0].status, OperationStatus::UncertainTimeout);
    // Sanitized error in DB must never contain the raw serial!
    assert!(!exec_res.results[0]
        .error_message
        .as_deref()
        .unwrap_or("")
        .contains("R58M91234AB"));

    let reconciled = reconcile_uncertain_operation(
        &service,
        adb,
        &devices,
        &locks,
        &storage,
        "R58M91234AB",
        &exec_res.results[0].operation_id,
    )
    .await
    .expect("reconcile");

    assert_eq!(reconciled.status, OperationStatus::VerificationMismatch);
}

#[test]
fn diagnostic_export_redacts_serials_and_user_paths() {
    let storage = Storage::open_in_memory().expect("db");
    let dev = storage
        .upsert_device_profile(None, Some("Google"), Some("Pixel 7"), Some("panther"), Some("14"), Some(34))
        .expect("dev");

    storage
        .save_settings(&AppSettings {
            custom_adb_path: Some("C:\\Users\\Shaheer\\platform-tools\\adb.exe".to_string()),
            auto_scan_on_select: false,
            log_level: "standard".to_string(),
        })
        .expect("settings");

    storage
        .record_operation_result(
            &PackageOperationResult {
                operation_id: "op-diag-1".to_string(),
                device_id: dev.device_id.clone(),
                user_id: 0,
                package_name: "com.example.app".to_string(),
                action: PackageAction::Disable,
                status: OperationStatus::Failed,
                previous_state: None,
                verified_state: None,
                restoration_fidelity: RestorationFidelity::Failed,
                blocked_reasons: Vec::new(),
                limitation_note: None,
                error_message: Some(
                    "Error on R58M91234AB using C:\\Users\\Shaheer\\platform-tools\\adb.exe"
                        .to_string(),
                ),
                executed_at_ms: 1_700_000_000_000,
            },
            Some("disabled:disabled_user"),
        )
        .expect("record op");

    let engine = RuleEngine::from_embedded().expect("engine");
    let bundle = build_redacted_diagnostic_bundle(
        &storage,
        engine.summary(),
        Some("C:\\Users\\Shaheer\\platform-tools\\adb.exe"),
        &["R58M91234AB".to_string()],
    )
    .expect("bundle");

    let json = serde_json::to_string_pretty(&bundle).expect("serialize bundle");
    assert!(!json.contains("R58M91234AB"));
    assert!(!json.contains("Shaheer"));
    assert!(json.contains("[REDACTED_SERIAL]"));
    assert!(json.contains("[REDACTED]"));
}
