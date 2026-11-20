use crate::adb::validation::validate_device_serial;
use crate::core::errors::AdtError;

/// Closed allow-list of Android system properties permitted in Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedDeviceProperty {
    Manufacturer,
    Model,
    AndroidVersion,
    ApiLevel,
}

impl AllowedDeviceProperty {
    pub const fn prop_key(self) -> &'static str {
        match self {
            Self::Manufacturer => "ro.product.manufacturer",
            Self::Model => "ro.product.model",
            Self::AndroidVersion => "ro.build.version.release",
            Self::ApiLevel => "ro.build.version.sdk",
        }
    }
}

/// Closed, strictly read-only allow-list of ADB commands permitted in Phase 1.
/// No mutating operations or arbitrary shell strings can be constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdbReadOnlyCommand {
    Version,
    DevicesLong,
    GetProp {
        serial: String,
        property: AllowedDeviceProperty,
    },
}

impl AdbReadOnlyCommand {
    /// Human-readable label for logs and structured errors (never leaks sensitive payloads).
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Version => "adb version",
            Self::DevicesLong => "adb devices -l",
            Self::GetProp { property, .. } => match property {
                AllowedDeviceProperty::Manufacturer => "adb shell getprop ro.product.manufacturer",
                AllowedDeviceProperty::Model => "adb shell getprop ro.product.model",
                AllowedDeviceProperty::AndroidVersion => {
                    "adb shell getprop ro.build.version.release"
                }
                AllowedDeviceProperty::ApiLevel => "adb shell getprop ro.build.version.sdk",
            },
        }
    }

    /// Builds argv entries directly for process spawning without shell interpolation.
    pub fn build_args(&self) -> Result<Vec<String>, AdtError> {
        match self {
            Self::Version => Ok(vec!["version".to_string()]),
            Self::DevicesLong => Ok(vec!["devices".to_string(), "-l".to_string()]),
            Self::GetProp { serial, property } => {
                validate_device_serial(serial)?;
                Ok(vec![
                    "-s".to_string(),
                    serial.clone(),
                    "shell".to_string(),
                    "getprop".to_string(),
                    property.prop_key().to_string(),
                ])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_version_args() {
        let args = AdbReadOnlyCommand::Version
            .build_args()
            .expect("version args should build");
        assert_eq!(args, vec!["version"]);
    }

    #[test]
    fn builds_devices_long_args() {
        let args = AdbReadOnlyCommand::DevicesLong
            .build_args()
            .expect("devices args should build");
        assert_eq!(args, vec!["devices", "-l"]);
    }

    #[test]
    fn builds_allowlisted_getprop_args_with_explicit_serial() {
        let cases = [
            (AllowedDeviceProperty::Manufacturer, "ro.product.manufacturer"),
            (AllowedDeviceProperty::Model, "ro.product.model"),
            (AllowedDeviceProperty::AndroidVersion, "ro.build.version.release"),
            (AllowedDeviceProperty::ApiLevel, "ro.build.version.sdk"),
        ];

        for (property, expected_key) in cases {
            let cmd = AdbReadOnlyCommand::GetProp {
                serial: "emulator-5554".to_string(),
                property,
            };
            let args = cmd.build_args().expect("valid serial should build args");
            assert_eq!(args, vec!["-s", "emulator-5554", "shell", "getprop", expected_key]);
        }
    }

    #[test]
    fn rejects_invalid_serial_before_building_getprop_args() {
        let cmd = AdbReadOnlyCommand::GetProp {
            serial: "bad serial".to_string(),
            property: AllowedDeviceProperty::Model,
        };
        assert!(matches!(
            cmd.build_args(),
            Err(AdtError::InvalidDeviceSerial { .. })
        ));
    }
}
