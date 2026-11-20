use std::fs;
use std::path::Path;

use adt_lib::adb::process_runner::mock::MockProcessRunner;
use adt_lib::adb::process_runner::ProcessOutput;
use adt_lib::adb::service::AdbService;
use adt_lib::android::device_info::get_basic_device_info;
use adt_lib::core::errors::AdtError;
use adt_lib::core::models::DeviceConnectionState;

fn read_fixture(filename: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture '{}': {e}", path.display()))
}

fn ok_output(stdout: String) -> Result<ProcessOutput, AdtError> {
    Ok(ProcessOutput {
        exit_code: Some(0),
        stdout,
        stderr: String::new(),
    })
}

#[tokio::test]
async fn fixture_version_and_multi_device_targeted_inspection() {
    let version_out = read_fixture("adb_version_windows.txt");
    let devices_out = read_fixture("adb_devices_multi.txt");

    let runner = MockProcessRunner::new(vec![
        ok_output(version_out),
        ok_output(devices_out),
        ok_output("samsung\n".to_string()),
        ok_output("SM-A528B\n".to_string()),
        ok_output("14\n".to_string()),
        ok_output("34\n".to_string()),
    ]);

    let service = AdbService::new(runner);
    let adb = Path::new("C:\\Tools\\platform-tools\\adb.exe");

    let version = service.get_version(adb).await.expect("version should parse");
    assert_eq!(version.adb_version, "1.0.41");
    assert_eq!(version.platform_tools_version.as_deref(), Some("35.0.2-12147458"));

    let devices = service.list_devices(adb).await.expect("devices should parse");
    assert_eq!(devices.len(), 5);
    assert_eq!(devices[0].state, DeviceConnectionState::Authorized);
    assert_eq!(devices[1].state, DeviceConnectionState::Offline);
    assert_eq!(devices[2].state, DeviceConnectionState::Unauthorized);
    assert_eq!(devices[3].state, DeviceConnectionState::NoPermissions);
    assert_eq!(devices[4].state, DeviceConnectionState::Authorized);

    // Explicitly target the second authorized device (never an arbitrary first device).
    let info = get_basic_device_info(&service, adb, "SECOND_AUTH_DEV", &devices)
        .await
        .expect("targeted device info should succeed");

    assert_eq!(info.serial, "SECOND_AUTH_DEV");
    assert_eq!(info.manufacturer.as_deref(), Some("samsung"));
    assert_eq!(info.model.as_deref(), Some("SM-A528B"));
    assert_eq!(info.android_version.as_deref(), Some("14"));
    assert_eq!(info.api_level, Some(34));

    // Audit exact argv executed: only version, devices -l, and the 4 allow-listed getprop keys.
    let invocations = service.runner().invocations();
    assert_eq!(invocations.len(), 6);
    assert_eq!(invocations[0].args, vec!["version"]);
    assert_eq!(invocations[1].args, vec!["devices", "-l"]);
    assert_eq!(
        invocations[2].args,
        vec!["-s", "SECOND_AUTH_DEV", "shell", "getprop", "ro.product.manufacturer"]
    );
    assert_eq!(
        invocations[3].args,
        vec!["-s", "SECOND_AUTH_DEV", "shell", "getprop", "ro.product.model"]
    );
    assert_eq!(
        invocations[4].args,
        vec!["-s", "SECOND_AUTH_DEV", "shell", "getprop", "ro.build.version.release"]
    );
    assert_eq!(
        invocations[5].args,
        vec!["-s", "SECOND_AUTH_DEV", "shell", "getprop", "ro.build.version.sdk"]
    );
}

#[tokio::test]
async fn fixture_daemon_startup_and_empty_device_list() {
    let daemon_out = read_fixture("adb_devices_daemon_start.txt");
    let empty_out = read_fixture("adb_devices_empty.txt");

    let runner = MockProcessRunner::new(vec![ok_output(daemon_out), ok_output(empty_out)]);
    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");

    let devices = service.list_devices(adb).await.expect("daemon start should parse");
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].serial, "emulator-5554");
    assert_eq!(devices[0].state, DeviceConnectionState::Authorized);

    let empty = service.list_devices(adb).await.expect("empty list should parse");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn simulates_timeout_error_cleanly() {
    let runner = MockProcessRunner::new(vec![Err(AdtError::Timeout {
        command: "adb devices -l".to_string(),
        timeout_ms: 10_000,
    })]);
    let service = AdbService::new(runner);

    let err = service
        .list_devices(Path::new("adb.exe"))
        .await
        .expect_err("timeout must propagate");

    assert!(matches!(err, AdtError::Timeout { timeout_ms: 10_000, .. }));
}

#[tokio::test]
async fn simulates_missing_adb_binary_error() {
    let runner = MockProcessRunner::new(vec![Err(AdtError::AdbNotFound)]);
    let service = AdbService::new(runner);

    let err = service
        .get_version(Path::new("adb.exe"))
        .await
        .expect_err("missing adb must propagate");

    assert!(matches!(err, AdtError::AdbNotFound));
}

#[tokio::test]
async fn blocks_offline_and_unauthorized_devices_during_inspection() {
    let devices_out = read_fixture("adb_devices_multi.txt");
    let runner = MockProcessRunner::new(vec![ok_output(devices_out)]);
    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");

    let devices = service.list_devices(adb).await.expect("devices should parse");

    let offline_err = get_basic_device_info(&service, adb, "192.168.1.42:5555", &devices)
        .await
        .expect_err("offline device must be rejected");
    assert!(matches!(offline_err, AdtError::DeviceOffline { .. }));

    let unauth_err = get_basic_device_info(&service, adb, "PIXEL8_UNAUTH", &devices)
        .await
        .expect_err("unauthorized device must be rejected");
    assert!(matches!(unauth_err, AdtError::DeviceUnauthorized { .. }));
}
