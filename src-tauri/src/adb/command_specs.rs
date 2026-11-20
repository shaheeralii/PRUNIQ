use crate::adb::validation::{validate_device_serial, validate_package_name};
use crate::core::errors::AdtError;

/// Closed allow-list of single Android system properties permitted for targeted `getprop`.
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

/// Strictly typed filter flags for `pm list packages`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageListFilter {
    All,
    System,
    ThirdParty,
    Enabled,
    Disabled,
}

impl PackageListFilter {
    pub const fn flag(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::System => Some("-s"),
            Self::ThirdParty => Some("-3"),
            Self::Enabled => Some("-e"),
            Self::Disabled => Some("-d"),
        }
    }
}

/// Closed, strictly read-only allow-list of ADB commands.
/// No mutating operations or arbitrary shell strings can be constructed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdbReadOnlyCommand {
    Version,
    DevicesLong,
    GetProp {
        serial: String,
        property: AllowedDeviceProperty,
    },
    GetAllProps {
        serial: String,
    },
    ReadProcMeminfo {
        serial: String,
    },
    ReadProcCpuinfo {
        serial: String,
    },
    DataStorageDf {
        serial: String,
    },
    StoragePartitionsDf {
        serial: String,
    },
    DumpsysMeminfo {
        serial: String,
    },
    ProcessList {
        serial: String,
    },
    ProcessListLegacy {
        serial: String,
    },
    WindowSize {
        serial: String,
    },
    WindowDensity {
        serial: String,
    },
    BatteryDumpsys {
        serial: String,
    },
    KernelVersion {
        serial: String,
    },
    PmHelp {
        serial: String,
    },
    CmdPackageHelp {
        serial: String,
    },
    AmHelp {
        serial: String,
    },
    AmGetCurrentUser {
        serial: String,
    },
    PmListUsers {
        serial: String,
    },
    PmListPackages {
        serial: String,
        filter: PackageListFilter,
        include_paths: bool,
        user_id: Option<u32>,
    },
    DumpsysPackage {
        serial: String,
        package_name: String,
    },
    PmResolveHome {
        serial: String,
    },
    QueryLaunchers {
        serial: String,
    },
    GetDefaultInputMethod {
        serial: String,
    },
    ListEnabledInputMethods {
        serial: String,
    },
}

impl AdbReadOnlyCommand {
    pub fn target_serial(&self) -> Option<&str> {
        match self {
            Self::Version | Self::DevicesLong => None,
            Self::GetProp { serial, .. }
            | Self::GetAllProps { serial }
            | Self::ReadProcMeminfo { serial }
            | Self::ReadProcCpuinfo { serial }
            | Self::DataStorageDf { serial }
            | Self::StoragePartitionsDf { serial }
            | Self::DumpsysMeminfo { serial }
            | Self::ProcessList { serial }
            | Self::ProcessListLegacy { serial }
            | Self::WindowSize { serial }
            | Self::WindowDensity { serial }
            | Self::BatteryDumpsys { serial }
            | Self::KernelVersion { serial }
            | Self::PmHelp { serial }
            | Self::CmdPackageHelp { serial }
            | Self::AmHelp { serial }
            | Self::AmGetCurrentUser { serial }
            | Self::PmListUsers { serial }
            | Self::PmListPackages { serial, .. }
            | Self::DumpsysPackage { serial, .. }
            | Self::PmResolveHome { serial }
            | Self::QueryLaunchers { serial }
            | Self::GetDefaultInputMethod { serial }
            | Self::ListEnabledInputMethods { serial } => Some(serial.as_str()),
        }
    }

    /// Human-readable label for logs and structured errors (never leaks raw serials).
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
            Self::GetAllProps { .. } => "adb shell getprop",
            Self::ReadProcMeminfo { .. } => "adb shell cat /proc/meminfo",
            Self::ReadProcCpuinfo { .. } => "adb shell cat /proc/cpuinfo",
            Self::DataStorageDf { .. } => "adb shell df -k /data",
            Self::StoragePartitionsDf { .. } => "adb shell df -k",
            Self::DumpsysMeminfo { .. } => "adb shell dumpsys meminfo",
            Self::ProcessList { .. } => "adb shell ps -A",
            Self::ProcessListLegacy { .. } => "adb shell ps",
            Self::WindowSize { .. } => "adb shell wm size",
            Self::WindowDensity { .. } => "adb shell wm density",
            Self::BatteryDumpsys { .. } => "adb shell dumpsys battery",
            Self::KernelVersion { .. } => "adb shell uname -r",
            Self::PmHelp { .. } => "adb shell pm help",
            Self::CmdPackageHelp { .. } => "adb shell cmd package help",
            Self::AmHelp { .. } => "adb shell am help",
            Self::AmGetCurrentUser { .. } => "adb shell am get-current-user",
            Self::PmListUsers { .. } => "adb shell pm list users",
            Self::PmListPackages { .. } => "adb shell pm list packages",
            Self::DumpsysPackage { .. } => "adb shell dumpsys package",
            Self::PmResolveHome { .. } => "adb shell pm resolve-activity (HOME)",
            Self::QueryLaunchers { .. } => "adb shell cmd package query-activities (HOME)",
            Self::GetDefaultInputMethod { .. } => {
                "adb shell settings get secure default_input_method"
            }
            Self::ListEnabledInputMethods { .. } => "adb shell ime list -s",
        }
    }

    /// Builds argv entries directly for process spawning without shell interpolation.
    pub fn build_args(&self) -> Result<Vec<String>, AdtError> {
        if let Some(serial) = self.target_serial() {
            validate_device_serial(serial)?;
        }

        match self {
            Self::Version => Ok(vec!["version".to_string()]),
            Self::DevicesLong => Ok(vec!["devices".to_string(), "-l".to_string()]),
            Self::GetProp { serial, property } => Ok(vec![
                "-s".to_string(),
                serial.clone(),
                "shell".to_string(),
                "getprop".to_string(),
                property.prop_key().to_string(),
            ]),
            Self::GetAllProps { serial } => Ok(shell_args(serial, &["getprop"])),
            Self::ReadProcMeminfo { serial } => Ok(shell_args(serial, &["cat", "/proc/meminfo"])),
            Self::ReadProcCpuinfo { serial } => Ok(shell_args(serial, &["cat", "/proc/cpuinfo"])),
            Self::DataStorageDf { serial } => Ok(shell_args(serial, &["df", "-k", "/data"])),
            Self::StoragePartitionsDf { serial } => Ok(shell_args(serial, &["df", "-k"])),
            Self::DumpsysMeminfo { serial } => Ok(shell_args(serial, &["dumpsys", "meminfo"])),
            Self::ProcessList { serial } => Ok(shell_args(serial, &["ps", "-A"])),
            Self::ProcessListLegacy { serial } => Ok(shell_args(serial, &["ps"])),
            Self::WindowSize { serial } => Ok(shell_args(serial, &["wm", "size"])),
            Self::WindowDensity { serial } => Ok(shell_args(serial, &["wm", "density"])),
            Self::BatteryDumpsys { serial } => Ok(shell_args(serial, &["dumpsys", "battery"])),
            Self::KernelVersion { serial } => Ok(shell_args(serial, &["uname", "-r"])),
            Self::PmHelp { serial } => Ok(shell_args(serial, &["pm", "help"])),
            Self::CmdPackageHelp { serial } => Ok(shell_args(serial, &["cmd", "package", "help"])),
            Self::AmHelp { serial } => Ok(shell_args(serial, &["am", "help"])),
            Self::AmGetCurrentUser { serial } => {
                Ok(shell_args(serial, &["am", "get-current-user"]))
            }
            Self::PmListUsers { serial } => Ok(shell_args(serial, &["pm", "list", "users"])),
            Self::PmListPackages {
                serial,
                filter,
                include_paths,
                user_id,
            } => {
                let mut args = shell_args(serial, &["pm", "list", "packages"]);
                if *include_paths {
                    args.push("-f".to_string());
                }
                if let Some(flag) = filter.flag() {
                    args.push(flag.to_string());
                }
                if let Some(uid) = user_id {
                    args.push("--user".to_string());
                    args.push(uid.to_string());
                }
                Ok(args)
            }
            Self::DumpsysPackage {
                serial,
                package_name,
            } => {
                validate_package_name(package_name)?;
                Ok(shell_args(serial, &["dumpsys", "package", package_name]))
            }
            Self::PmResolveHome { serial } => Ok(shell_args(
                serial,
                &[
                    "pm",
                    "resolve-activity",
                    "--brief",
                    "-a",
                    "android.intent.action.MAIN",
                    "-c",
                    "android.intent.category.HOME",
                ],
            )),
            Self::QueryLaunchers { serial } => Ok(shell_args(
                serial,
                &[
                    "cmd",
                    "package",
                    "query-activities",
                    "--brief",
                    "-a",
                    "android.intent.action.MAIN",
                    "-c",
                    "android.intent.category.HOME",
                ],
            )),
            Self::GetDefaultInputMethod { serial } => Ok(shell_args(
                serial,
                &["settings", "get", "secure", "default_input_method"],
            )),
            Self::ListEnabledInputMethods { serial } => {
                Ok(shell_args(serial, &["ime", "list", "-s"]))
            }
        }
    }
}

/// Strictly separated, closed allow-list of reversible Phase 4 package-management commands.
/// Never includes `pm uninstall`, `pm clear`, `pm install`, `am force-stop`, or reboot commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdbMutatingCommand {
    PmDisableUser {
        serial: String,
        user_id: u32,
        package_name: String,
    },
    PmEnable {
        serial: String,
        user_id: u32,
        package_name: String,
    },
    PmDefaultState {
        serial: String,
        user_id: u32,
        package_name: String,
    },
    PmDisableUntilUsed {
        serial: String,
        user_id: u32,
        package_name: String,
    },
}

impl AdbMutatingCommand {
    pub fn target_serial(&self) -> &str {
        match self {
            Self::PmDisableUser { serial, .. }
            | Self::PmEnable { serial, .. }
            | Self::PmDefaultState { serial, .. }
            | Self::PmDisableUntilUsed { serial, .. } => serial.as_str(),
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::PmDisableUser { .. } => "adb shell pm disable-user",
            Self::PmEnable { .. } => "adb shell pm enable",
            Self::PmDefaultState { .. } => "adb shell pm default-state",
            Self::PmDisableUntilUsed { .. } => "adb shell pm disable-until-used",
        }
    }

    pub fn build_args(&self) -> Result<Vec<String>, AdtError> {
        let (serial, subcmd, user_id, package_name) = match self {
            Self::PmDisableUser {
                serial,
                user_id,
                package_name,
            } => (serial, "disable-user", *user_id, package_name),
            Self::PmEnable {
                serial,
                user_id,
                package_name,
            } => (serial, "enable", *user_id, package_name),
            Self::PmDefaultState {
                serial,
                user_id,
                package_name,
            } => (serial, "default-state", *user_id, package_name),
            Self::PmDisableUntilUsed {
                serial,
                user_id,
                package_name,
            } => (serial, "disable-until-used", *user_id, package_name),
        };

        validate_device_serial(serial)?;
        validate_package_name(package_name)?;

        Ok(vec![
            "-s".to_string(),
            serial.clone(),
            "shell".to_string(),
            "pm".to_string(),
            subcmd.to_string(),
            "--user".to_string(),
            user_id.to_string(),
            package_name.clone(),
        ])
    }
}

fn shell_args(serial: &str, tail: &[&str]) -> Vec<String> {
    let mut args = Vec::with_capacity(3 + tail.len());
    args.push("-s".to_string());
    args.push(serial.to_string());
    args.push("shell".to_string());
    for part in tail {
        args.push((*part).to_string());
    }
    args
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
    fn builds_pm_list_packages_and_dumpsys_args_safely() {
        let list_cmd = AdbReadOnlyCommand::PmListPackages {
            serial: "DEV01".to_string(),
            filter: PackageListFilter::System,
            include_paths: true,
            user_id: Some(0),
        };
        assert_eq!(
            list_cmd.build_args().expect("list packages args"),
            vec!["-s", "DEV01", "shell", "pm", "list", "packages", "-f", "-s", "--user", "0"]
        );

        let dump_cmd = AdbReadOnlyCommand::DumpsysPackage {
            serial: "DEV01".to_string(),
            package_name: "android".to_string(),
        };
        assert_eq!(
            dump_cmd.build_args().expect("dumpsys args"),
            vec!["-s", "DEV01", "shell", "dumpsys", "package", "android"]
        );
    }

    #[test]
    fn builds_phase6_read_only_diagnostic_args() {
        assert_eq!(
            AdbReadOnlyCommand::DumpsysMeminfo {
                serial: "DEV01".to_string(),
            }
            .build_args()
            .expect("dumpsys meminfo"),
            vec!["-s", "DEV01", "shell", "dumpsys", "meminfo"]
        );
        assert_eq!(
            AdbReadOnlyCommand::ProcessList {
                serial: "DEV01".to_string(),
            }
            .build_args()
            .expect("ps -A"),
            vec!["-s", "DEV01", "shell", "ps", "-A"]
        );
        assert_eq!(
            AdbReadOnlyCommand::StoragePartitionsDf {
                serial: "DEV01".to_string(),
            }
            .build_args()
            .expect("df -k"),
            vec!["-s", "DEV01", "shell", "df", "-k"]
        );
    }

    #[test]
    fn builds_mutating_pm_commands_with_explicit_serial_and_user_id() {
        let disable = AdbMutatingCommand::PmDisableUser {
            serial: "DEV01".to_string(),
            user_id: 0,
            package_name: "com.example.app".to_string(),
        };
        assert_eq!(
            disable.build_args().expect("disable args"),
            vec![
                "-s",
                "DEV01",
                "shell",
                "pm",
                "disable-user",
                "--user",
                "0",
                "com.example.app"
            ]
        );

        let default_state = AdbMutatingCommand::PmDefaultState {
            serial: "DEV01".to_string(),
            user_id: 10,
            package_name: "com.example.app".to_string(),
        };
        assert_eq!(
            default_state.build_args().expect("default-state args"),
            vec![
                "-s",
                "DEV01",
                "shell",
                "pm",
                "default-state",
                "--user",
                "10",
                "com.example.app"
            ]
        );
    }

    #[test]
    fn rejects_invalid_serial_and_invalid_package_name() {
        let bad_serial = AdbReadOnlyCommand::GetProp {
            serial: "bad serial".to_string(),
            property: AllowedDeviceProperty::Model,
        };
        assert!(matches!(
            bad_serial.build_args(),
            Err(AdtError::InvalidDeviceSerial { .. })
        ));

        let bad_pkg = AdbMutatingCommand::PmDisableUser {
            serial: "DEV01".to_string(),
            user_id: 0,
            package_name: "com.bad;reboot".to_string(),
        };
        assert!(matches!(
            bad_pkg.build_args(),
            Err(AdtError::InvalidPackageName { .. })
        ));
    }
}
