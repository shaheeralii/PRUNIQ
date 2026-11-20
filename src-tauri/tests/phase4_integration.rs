use std::path::Path;
use std::sync::RwLock;

use adt_lib::adb::device_discovery::parse_devices_long;
use adt_lib::adb::process_runner::mock::MockProcessRunner;
use adt_lib::adb::process_runner::ProcessOutput;
use adt_lib::adb::service::AdbService;
use adt_lib::core::errors::AdtError;
use adt_lib::core::models::AdbDevice;
use adt_lib::core::package_models::{ConfiguredPackageState, EffectivePackageState};
use adt_lib::operations::executor::execute_package_operations;
use adt_lib::operations::locks::DeviceLockManager;
use adt_lib::operations::models::{
    CapabilityFlags, ExecuteOperationsRequest, OperationStatus, PackageAction,
    PackageActionRequest, PreviewOperationsRequest, ProtectionReason, RestorationFidelity,
};
use adt_lib::operations::planner::preview_package_operations;
use adt_lib::operations::protections::probe_device_protections;
use adt_lib::operations::snapshots::SnapshotStore;

fn ok_out(stdout: impl Into<String>) -> Result<ProcessOutput, AdtError> {
    Ok(ProcessOutput {
        exit_code: Some(0),
        stdout: stdout.into(),
        stderr: String::new(),
    })
}

fn sample_devices() -> Vec<AdbDevice> {
    parse_devices_long("List of devices attached\nDEV01 device model:SM_A528B transport_id:1\n")
        .expect("devices fixture")
}

fn dumpsys_state(pkg: &str, enabled_code: i32) -> String {
    format!(
        "Packages:\n  Package [{pkg}] (111):\n    codePath=/system/priv-app/App\n    versionCode=10\n    versionName=1.0\n    pkgFlags=[ SYSTEM ]\n    User 0: installed=true hidden=false stopped=false enabled={enabled_code}\n"
    )
}

#[tokio::test]
async fn verified_disable_enable_and_exact_original_restoration_cycle() {
    let devices = sample_devices();
    let pkg = "com.example.bloat";
    let adb = Path::new("adb.exe");

    let runner = MockProcessRunner::new(vec![
        // Protection probes for Disable (4 read-only calls)
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        // 1. Disable: pre-state (DEFAULT=0) -> pm disable-user -> post-state (DISABLED_USER=3)
        ok_out(dumpsys_state(pkg, 0)),
        ok_out("Package com.example.bloat new state: disabled-user\n"),
        ok_out(dumpsys_state(pkg, 3)),
        // 2. Enable: pre-state (3) -> pm enable -> post-state (ENABLED=1)
        ok_out(dumpsys_state(pkg, 3)),
        ok_out("Package com.example.bloat new state: enabled\n"),
        ok_out(dumpsys_state(pkg, 1)),
        // 3. RestoreOriginalState (baseline was DEFAULT=0): pre-state (1) -> pm default-state -> post-state (0)
        ok_out(dumpsys_state(pkg, 1)),
        ok_out("Package com.example.bloat new state: default\n"),
        ok_out(dumpsys_state(pkg, 0)),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let store = RwLock::new(SnapshotStore::new());
    let caps = CapabilityFlags {
        pm_default_state_supported: true,
        pm_disable_until_used_supported: true,
    };

    // Disable
    let disable_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::Disable,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: caps.clone(),
        },
    )
    .await
    .expect("disable cycle");
    assert_eq!(disable_res.succeeded_count, 1);
    assert_eq!(disable_res.results[0].status, OperationStatus::Succeeded);
    assert_eq!(
        disable_res.results[0]
            .verified_state
            .as_ref()
            .expect("verified")
            .configured_state,
        ConfiguredPackageState::DisabledUser
    );

    // Enable
    let enable_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::Enable,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: caps.clone(),
        },
    )
    .await
    .expect("enable cycle");
    assert_eq!(enable_res.succeeded_count, 1);
    assert_eq!(
        enable_res.results[0]
            .verified_state
            .as_ref()
            .expect("verified")
            .configured_state,
        ConfiguredPackageState::Enabled
    );

    // Restore Original State (should restore immutable baseline DEFAULT=0 via pm default-state)
    let restore_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::RestoreOriginalState,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: caps,
        },
    )
    .await
    .expect("restore original cycle");
    assert_eq!(restore_res.succeeded_count, 1);
    assert_eq!(restore_res.results[0].status, OperationStatus::Succeeded);
    assert_eq!(
        restore_res.results[0].restoration_fidelity,
        RestorationFidelity::Exact
    );
    assert_eq!(
        restore_res.results[0]
            .verified_state
            .as_ref()
            .expect("verified")
            .configured_state,
        ConfiguredPackageState::Default
    );
}

#[tokio::test]
async fn blocks_protected_packages_and_requires_explicit_confirmation() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");

    let runner = MockProcessRunner::new(vec![
        // Protection probes for preview
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        // Protection probes for execute
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let store = RwLock::new(SnapshotStore::new());
    let caps = CapabilityFlags {
        pm_default_state_supported: false,
        pm_disable_until_used_supported: false,
    };

    // Unconfirmed execution must be rejected immediately.
    let unconfirmed = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: "com.example.safe".to_string(),
                action: PackageAction::Disable,
            }],
            confirmed: false,
            android_version: None,
            capabilities: caps.clone(),
        },
    )
    .await
    .expect_err("unconfirmed must fail");
    assert!(matches!(unconfirmed, AdtError::ConfirmationRequired));

    let profile = probe_device_protections(&service, adb, "DEV01", &devices)
        .await
        .expect("probe protections");
    let preview = {
        let guard = store.read().expect("read store");
        preview_package_operations(
            &PreviewOperationsRequest {
                serial: "DEV01".to_string(),
                user_id: 0,
                items: vec![
                    PackageActionRequest {
                        package_name: "com.android.systemui".to_string(),
                        action: PackageAction::Disable,
                    },
                    PackageActionRequest {
                        package_name: "com.sec.android.app.launcher".to_string(),
                        action: PackageAction::Disable,
                    },
                    PackageActionRequest {
                        package_name: "com.google.android.inputmethod.latin".to_string(),
                        action: PackageAction::Disable,
                    },
                    PackageActionRequest {
                        package_name: "com.example.keep".to_string(),
                        action: PackageAction::KeepCurrent,
                    },
                ],
                capabilities: caps.clone(),
            },
            &profile,
            &guard,
        )
        .expect("preview")
    };
    assert_eq!(preview.blocked_count, 3);
    assert_eq!(preview.noop_count, 1);
    assert_eq!(preview.executable_count, 0);

    // Defense-in-depth: execute blocks all 3 and issues zero mutating ADB calls.
    let exec_res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![
                PackageActionRequest {
                    package_name: "com.android.systemui".to_string(),
                    action: PackageAction::Disable,
                },
                PackageActionRequest {
                    package_name: "com.sec.android.app.launcher".to_string(),
                    action: PackageAction::Disable,
                },
                PackageActionRequest {
                    package_name: "com.google.android.inputmethod.latin".to_string(),
                    action: PackageAction::Disable,
                },
            ],
            confirmed: true,
            android_version: None,
            capabilities: caps,
        },
    )
    .await
    .expect("blocked items return structured result");

    assert_eq!(exec_res.blocked_count, 3);
    assert!(exec_res.results[0]
        .blocked_reasons
        .contains(&ProtectionReason::ImmutableSystemComponent));
    assert!(exec_res.results[1]
        .blocked_reasons
        .contains(&ProtectionReason::ActiveDefaultLauncher));
    assert!(exec_res.results[2]
        .blocked_reasons
        .contains(&ProtectionReason::ActiveDefaultKeyboard));
}

#[tokio::test]
async fn timeout_marks_uncertain_without_retrying_and_verification_catches_silent_failure() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let pkg_timeout = "com.example.slow";
    let pkg_silent = "com.example.stubborn";

    let runner = MockProcessRunner::new(vec![
        // Protection probes
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.sec.android.app.launcher/.Launcher\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        ok_out("com.google.android.inputmethod.latin/.LatinIME\n"),
        // Item 1: pre-state -> timeout (must NOT retry!)
        ok_out(dumpsys_state(pkg_timeout, 0)),
        Err(AdtError::Timeout {
            command: "adb shell pm disable-user".to_string(),
            timeout_ms: 15_000,
        }),
        // Item 2: pre-state -> exit 0 -> post-state STILL enabled (0) -> VerificationMismatch!
        ok_out(dumpsys_state(pkg_silent, 0)),
        ok_out("Success\n"),
        ok_out(dumpsys_state(pkg_silent, 0)),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let store = RwLock::new(SnapshotStore::new());

    let res = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![
                PackageActionRequest {
                    package_name: pkg_timeout.to_string(),
                    action: PackageAction::Disable,
                },
                PackageActionRequest {
                    package_name: pkg_silent.to_string(),
                    action: PackageAction::Disable,
                },
            ],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: CapabilityFlags {
                pm_default_state_supported: false,
                pm_disable_until_used_supported: false,
            },
        },
    )
    .await
    .expect("bulk result should return cleanly");

    assert_eq!(res.uncertain_count, 1);
    assert_eq!(res.failed_count, 1);
    assert_eq!(res.succeeded_count, 0);
    assert_eq!(res.results[0].status, OperationStatus::UncertainTimeout);
    assert_eq!(
        res.results[0].restoration_fidelity,
        RestorationFidelity::Unverified
    );
    assert_eq!(res.results[1].status, OperationStatus::VerificationMismatch);

    // Exactly 9 calls total -> zero retries occurred on timeout.
    assert_eq!(service.runner().invocations().len(), 9);
}

#[tokio::test]
async fn reports_effective_match_only_when_exact_restoration_is_not_supported() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let pkg = "com.example.oemdisabled";

    let runner = MockProcessRunner::new(vec![
        // Enable package whose baseline is DISABLED (2): pre-state(2) -> pm enable -> post-state(1)
        ok_out(dumpsys_state(pkg, 2)),
        ok_out("Package com.example.oemdisabled new state: enabled\n"),
        ok_out(dumpsys_state(pkg, 1)),
        // Restore original (2): pre-state(1) -> pm disable-user (since rootless cannot set 2) -> post-state(3)
        ok_out(dumpsys_state(pkg, 1)),
        ok_out("Package com.example.oemdisabled new state: disabled-user\n"),
        ok_out(dumpsys_state(pkg, 3)),
    ]);

    let service = AdbService::new(runner);
    let locks = DeviceLockManager::new();
    let store = RwLock::new(SnapshotStore::new());
    let caps = CapabilityFlags {
        pm_default_state_supported: false,
        pm_disable_until_used_supported: false,
    };

    let _ = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::Enable,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: caps.clone(),
        },
    )
    .await
    .expect("enable");

    let restore = execute_package_operations(
        &service,
        adb,
        &devices,
        &locks,
        &store,
        &ExecuteOperationsRequest {
            serial: "DEV01".to_string(),
            user_id: 0,
            items: vec![PackageActionRequest {
                package_name: pkg.to_string(),
                action: PackageAction::RestoreOriginalState,
            }],
            confirmed: true,
            android_version: Some("14".to_string()),
            capabilities: caps,
        },
    )
    .await
    .expect("restore");

    assert_eq!(restore.succeeded_count, 1);
    assert_eq!(restore.results[0].status, OperationStatus::Succeeded);
    assert_eq!(
        restore.results[0].restoration_fidelity,
        RestorationFidelity::EffectiveMatchOnly
    );
    assert!(restore.results[0].limitation_note.is_some());
    assert_eq!(
        restore.results[0]
            .verified_state
            .as_ref()
            .expect("verified")
            .effective_state,
        EffectivePackageState::Disabled
    );
}
