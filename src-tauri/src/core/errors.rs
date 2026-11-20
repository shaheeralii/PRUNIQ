use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdtError {
    #[error("ADB executable was not found on the system PATH.")]
    AdbNotFound,

    #[error("Configured ADB path is invalid or does not point to a file: {path}")]
    InvalidAdbPath { path: String },

    #[error("Invalid device serial: {reason}")]
    InvalidDeviceSerial { reason: String },

    #[error("Invalid Android package identifier: {reason}")]
    InvalidPackageName { reason: String },

    #[error("Recommendation rule database error: {message}")]
    RuleValidation { message: String },

    #[error("Another package operation is already running on device '{serial}'. Wait for it to finish before starting a new operation.")]
    OperationInProgress { serial: String },

    #[error("Explicit user confirmation is required before executing package modifications.")]
    ConfirmationRequired,

    #[error("An explicit Android user ID must be selected before executing package modifications.")]
    UserIdRequired,

    #[error("Device '{serial}' was not found in the list of connected devices.")]
    DeviceNotFound { serial: String },

    #[error("Device '{serial}' is unauthorized. Accept the USB debugging RSA prompt on the phone.")]
    DeviceUnauthorized { serial: String },

    #[error("Device '{serial}' is offline. Reconnect the USB cable or toggle USB debugging.")]
    DeviceOffline { serial: String },

    #[error("Insufficient permissions to access device '{serial}'.")]
    DeviceNoPermissions { serial: String },

    #[error("Device '{serial}' is not ready (current state: '{state}').")]
    DeviceNotReady { serial: String, state: String },

    #[error("Device '{serial}' disconnected during the operation.")]
    DeviceDisconnected { serial: String },

    #[error("ADB command '{command}' timed out after {timeout_ms} ms.")]
    Timeout { command: String, timeout_ms: u64 },

    #[error("Failed to start ADB process: {message}")]
    ProcessSpawnFailed { message: String },

    #[error("ADB command '{command}' failed (exit code: {exit_code:?}): {stderr}")]
    ProcessFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Failed to parse ADB output for '{context}': {message}")]
    ParseError { context: String, message: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdtErrorPayload<'a> {
    code: &'static str,
    message: String,
    guidance: Option<&'a str>,
}

impl AdtError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::AdbNotFound => "adb_not_found",
            Self::InvalidAdbPath { .. } => "invalid_adb_path",
            Self::InvalidDeviceSerial { .. } => "invalid_device_serial",
            Self::InvalidPackageName { .. } => "invalid_package_name",
            Self::RuleValidation { .. } => "rule_validation",
            Self::OperationInProgress { .. } => "operation_in_progress",
            Self::ConfirmationRequired => "confirmation_required",
            Self::UserIdRequired => "user_id_required",
            Self::DeviceNotFound { .. } => "device_not_found",
            Self::DeviceUnauthorized { .. } => "device_unauthorized",
            Self::DeviceOffline { .. } => "device_offline",
            Self::DeviceNoPermissions { .. } => "device_no_permissions",
            Self::DeviceNotReady { .. } => "device_not_ready",
            Self::DeviceDisconnected { .. } => "device_disconnected",
            Self::Timeout { .. } => "timeout",
            Self::ProcessSpawnFailed { .. } => "process_spawn_failed",
            Self::ProcessFailed { .. } => "process_failed",
            Self::ParseError { .. } => "parse_error",
        }
    }

    pub const fn guidance(&self) -> Option<&'static str> {
        match self {
            Self::AdbNotFound => Some(
                "Install Android SDK Platform-Tools and add it to PATH, or provide the full path to adb.exe.",
            ),
            Self::InvalidAdbPath { .. } => Some(
                "Verify the path points directly to the adb executable file (for example, C:\\platform-tools\\adb.exe).",
            ),
            Self::InvalidPackageName { .. } => Some(
                "Provide a valid Android package identifier such as com.example.app or android.",
            ),
            Self::RuleValidation { .. } => Some(
                "Verify the package-rule JSON conforms to data/package-rules/schema.json and includes evidence for verified rules.",
            ),
            Self::OperationInProgress { .. } => Some(
                "Conflicting operations cannot run simultaneously on the same device. Wait for the active operation to complete.",
            ),
            Self::ConfirmationRequired => Some(
                "Preview the planned package actions and confirm explicitly before execution.",
            ),
            Self::UserIdRequired => Some(
                "Select an Android user profile in the Device Info & Capabilities tab before modifying packages.",
            ),
            Self::DeviceNotFound { .. } => Some(
                "Refresh the device list and ensure USB debugging is enabled and the cable is connected.",
            ),
            Self::DeviceUnauthorized { .. } => Some(
                "Unlock the Android device and tap 'Allow' on the USB debugging authorization dialog.",
            ),
            Self::DeviceOffline { .. } => Some(
                "Unplug and reconnect the USB cable, try another port, or toggle USB debugging off and on.",
            ),
            Self::DeviceNoPermissions { .. } => Some(
                "Check USB driver installation or operating system USB permissions for this device.",
            ),
            Self::DeviceDisconnected { .. } => Some(
                "Reconnect the device, refresh the device list, and verify actual package state before retrying.",
            ),
            Self::Timeout { .. } => Some(
                "Terminating the local ADB process does not prove the device stopped executing. Verify actual device state before retrying.",
            ),
            _ => None,
        }
    }

    pub const fn is_connection_loss(&self) -> bool {
        matches!(
            self,
            Self::DeviceDisconnected { .. }
                | Self::DeviceOffline { .. }
                | Self::DeviceUnauthorized { .. }
                | Self::DeviceNoPermissions { .. }
        )
    }
}

impl Serialize for AdtError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let payload = AdtErrorPayload {
            code: self.code(),
            message: self.to_string(),
            guidance: self.guidance(),
        };
        payload.serialize(serializer)
    }
}
