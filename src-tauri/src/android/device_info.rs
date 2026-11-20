use std::path::Path;

use crate::adb::command_specs::{AdbReadOnlyCommand, AllowedDeviceProperty};
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::core::errors::AdtError;
use crate::core::models::{AdbDevice, BasicDeviceInfo};

/// Retrieves only the four Phase 1 allow-listed device properties for an explicitly
/// validated, recently discovered authorized device serial.
pub async fn get_basic_device_info<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
) -> Result<BasicDeviceInfo, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();

    let manufacturer = read_property(
        service,
        adb_path,
        &target_serial,
        AllowedDeviceProperty::Manufacturer,
    )
    .await?;
    let model = read_property(
        service,
        adb_path,
        &target_serial,
        AllowedDeviceProperty::Model,
    )
    .await?;
    let android_version = read_property(
        service,
        adb_path,
        &target_serial,
        AllowedDeviceProperty::AndroidVersion,
    )
    .await?;
    let raw_api_level = read_property(
        service,
        adb_path,
        &target_serial,
        AllowedDeviceProperty::ApiLevel,
    )
    .await?;

    let api_level = raw_api_level
        .as_deref()
        .and_then(|raw| raw.parse::<u32>().ok());

    Ok(BasicDeviceInfo {
        serial: target_serial,
        manufacturer,
        model,
        android_version,
        api_level,
        raw_api_level,
    })
}

async fn read_property<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    property: AllowedDeviceProperty,
) -> Result<Option<String>, AdtError> {
    let cmd = AdbReadOnlyCommand::GetProp {
        serial: serial.to_string(),
        property,
    };
    let output = service.execute_read_only(adb_path, &cmd).await?;
    let value = output.stdout.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::adb::device_discovery::parse_devices_long;
    use crate::adb::process_runner::mock::MockProcessRunner;
    use crate::adb::process_runner::ProcessOutput;

    fn ok_output(stdout: &str) -> Result<ProcessOutput, AdtError> {
        Ok(ProcessOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    }

    fn sample_devices() -> Vec<AdbDevice> {
        parse_devices_long(
            "List of devices attached\n\
             R58M91234AB device model:SM_G973F transport_id:1\n\
             UNAUTH999 unauthorized transport_id:2\n",
        )
        .expect("fixture devices should parse")
    }

    #[tokio::test]
    async fn queries_only_the_four_allowlisted_properties_for_authorized_serial() {
        let runner = MockProcessRunner::new(vec![
            ok_output("samsung\r\n"),
            ok_output("SM-G973F\r\n"),
            ok_output("12\r\n"),
            ok_output("31\r\n"),
        ]);
        let service = AdbService::new(runner);
        let devices = sample_devices();

        let info = get_basic_device_info(&service, Path::new("adb.exe"), "R58M91234AB", &devices)
            .await
            .expect("basic device info should succeed");

        assert_eq!(info.serial, "R58M91234AB");
        assert_eq!(info.manufacturer.as_deref(), Some("samsung"));
        assert_eq!(info.model.as_deref(), Some("SM-G973F"));
        assert_eq!(info.android_version.as_deref(), Some("12"));
        assert_eq!(info.api_level, Some(31));
        assert_eq!(info.raw_api_level.as_deref(), Some("31"));
    }

    #[tokio::test]
    async fn handles_missing_properties_and_non_numeric_sdk_gracefully() {
        let runner = MockProcessRunner::new(vec![
            ok_output("\r\n"),
            ok_output("LegacyPhone\n"),
            ok_output(""),
            ok_output("unknown-sdk\n"),
        ]);
        let service = AdbService::new(runner);
        let devices = sample_devices();

        let info = get_basic_device_info(&service, Path::new("adb.exe"), "R58M91234AB", &devices)
            .await
            .expect("missing fields should still succeed gracefully");

        assert_eq!(info.manufacturer, None);
        assert_eq!(info.model.as_deref(), Some("LegacyPhone"));
        assert_eq!(info.android_version, None);
        assert_eq!(info.api_level, None);
        assert_eq!(info.raw_api_level.as_deref(), Some("unknown-sdk"));
    }

    #[tokio::test]
    async fn rejects_unauthorized_device_before_spawning_any_getprop() {
        let runner = MockProcessRunner::new(vec![]);
        let service = AdbService::new(runner);
        let devices = sample_devices();

        let err = get_basic_device_info(&service, Path::new("adb.exe"), "UNAUTH999", &devices)
            .await
            .expect_err("unauthorized device must be rejected");

        assert!(matches!(err, AdtError::DeviceUnauthorized { .. }));
        assert!(service.runner().invocations().is_empty());
    }

    #[tokio::test]
    async fn maps_device_disconnection_mid_query() {
        let runner = MockProcessRunner::new(vec![
            ok_output("Google\n"),
            Ok(ProcessOutput {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "adb.exe: device 'R58M91234AB' not found\r\n".to_string(),
            }),
        ]);
        let service = AdbService::new(runner);
        let devices = sample_devices();

        let err = get_basic_device_info(&service, Path::new("adb.exe"), "R58M91234AB", &devices)
            .await
            .expect_err("disconnect mid-query should fail");

        assert!(matches!(err, AdtError::DeviceDisconnected { .. }));
    }
}
