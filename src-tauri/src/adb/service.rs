use std::path::Path;
use std::time::Duration;

use crate::adb::command_specs::AdbReadOnlyCommand;
use crate::adb::detection::parse_adb_version;
use crate::adb::device_discovery::parse_devices_long;
use crate::adb::process_runner::{ProcessOutput, ProcessRunner};
use crate::core::errors::AdtError;
use crate::core::models::{AdbDevice, AdbVersionInfo};

pub const DEFAULT_ADB_TIMEOUT: Duration = Duration::from_secs(10);
pub const BULK_SCAN_ADB_TIMEOUT: Duration = Duration::from_secs(25);

/// Stateless, read-only ADB service generic over `ProcessRunner`.
pub struct AdbService<R: ProcessRunner> {
    runner: R,
    timeout: Duration,
}

impl<R: ProcessRunner> AdbService<R> {
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            timeout: DEFAULT_ADB_TIMEOUT,
        }
    }

    pub fn with_timeout(runner: R, timeout: Duration) -> Self {
        Self { runner, timeout }
    }

    pub fn runner(&self) -> &R {
        &self.runner
    }

    pub async fn execute_read_only(
        &self,
        adb_path: &Path,
        command: &AdbReadOnlyCommand,
    ) -> Result<ProcessOutput, AdtError> {
        self.execute_read_only_with_timeout(adb_path, command, self.timeout)
            .await
    }

    pub async fn execute_read_only_with_timeout(
        &self,
        adb_path: &Path,
        command: &AdbReadOnlyCommand,
        timeout: Duration,
    ) -> Result<ProcessOutput, AdtError> {
        let args = command.build_args()?;
        let output = self
            .runner
            .run(adb_path, &args, command.label(), timeout)
            .await?;

        if !output.success() {
            return Err(classify_process_failure(command, &output));
        }

        Ok(output)
    }

    pub async fn get_version(&self, adb_path: &Path) -> Result<AdbVersionInfo, AdtError> {
        let output = self
            .execute_read_only(adb_path, &AdbReadOnlyCommand::Version)
            .await?;
        parse_adb_version(&output.stdout)
    }

    pub async fn list_devices(&self, adb_path: &Path) -> Result<Vec<AdbDevice>, AdtError> {
        let output = self
            .execute_read_only(adb_path, &AdbReadOnlyCommand::DevicesLong)
            .await?;
        parse_devices_long(&output.stdout)
    }
}

fn classify_process_failure(command: &AdbReadOnlyCommand, output: &ProcessOutput) -> AdtError {
    let stderr_trimmed = output.stderr.trim();
    let stderr_lower = stderr_trimmed.to_ascii_lowercase();

    if let Some(serial) = command.target_serial() {
        if stderr_lower.contains("unauthorized") {
            return AdtError::DeviceUnauthorized {
                serial: serial.to_string(),
            };
        }
        if stderr_lower.contains("device offline") || stderr_lower.contains("offline") {
            return AdtError::DeviceOffline {
                serial: serial.to_string(),
            };
        }
        if stderr_lower.contains("not found") || stderr_lower.contains("closed") {
            return AdtError::DeviceDisconnected {
                serial: serial.to_string(),
            };
        }
    }

    AdtError::ProcessFailed {
        command: command.label().to_string(),
        exit_code: output.exit_code,
        stderr: if stderr_trimmed.is_empty() {
            "no error details provided by ADB".to_string()
        } else {
            stderr_trimmed.to_string()
        },
    }
}
