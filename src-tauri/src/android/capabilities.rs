use std::path::Path;

use crate::adb::command_specs::{AdbReadOnlyCommand, AllowedDeviceProperty};
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::android::parsers::capabilities::{
    classify_support_tier, detect_subcommand_in_help, parse_current_user, parse_pm_list_users,
};
use crate::core::device_models::{CommandSupport, DeviceCapabilities};
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;

/// Probes device capabilities using strictly read-only help and metadata commands.
/// Never executes mutating commands to test availability.
pub async fn probe_device_capabilities<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
) -> Result<DeviceCapabilities, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let mut warnings = Vec::new();

    let android_version = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::GetProp {
            serial: target_serial.clone(),
            property: AllowedDeviceProperty::AndroidVersion,
        },
        &mut warnings,
    )
    .await?
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());

    let api_level = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::GetProp {
            serial: target_serial.clone(),
            property: AllowedDeviceProperty::ApiLevel,
        },
        &mut warnings,
    )
    .await?
    .and_then(|s| s.trim().parse::<u32>().ok());

    let pm_help = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::PmHelp {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?;

    let cmd_pkg_help = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::CmdPackageHelp {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?;

    let am_help = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::AmHelp {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?;

    let current_user_out = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::AmGetCurrentUser {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?;

    let users_out = run_optional_stdout(
        service,
        adb_path,
        &AdbReadOnlyCommand::PmListUsers {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?;

    let active_user_id = current_user_out.as_deref().and_then(parse_current_user);
    let users = users_out
        .as_deref()
        .map(parse_pm_list_users)
        .unwrap_or_default();

    // Never silently assume user 0 if user detection fails and multiple profiles exist.
    let user_detection_warning = if active_user_id.is_none() {
        if users.len() > 1 {
            Some(
                "Could not detect active Android user while multiple user profiles exist. Explicitly choose a user ID before future package operations."
                    .to_string(),
            )
        } else {
            Some(
                "Could not detect active Android user via 'am get-current-user'; user scope is unverified."
                    .to_string(),
            )
        }
    } else {
        None
    };

    let combined_pkg_help = match (&pm_help, &cmd_pkg_help) {
        (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    };

    let am_get_current_user = if active_user_id.is_some() {
        CommandSupport::Supported
    } else {
        detect_subcommand_in_help(am_help.as_deref(), "get-current-user")
    };

    Ok(DeviceCapabilities {
        serial: target_serial,
        android_version,
        api_level,
        support_tier: classify_support_tier(api_level),
        active_user_id,
        users,
        user_detection_warning,
        pm_list_packages_flags: detect_subcommand_in_help(pm_help.as_deref(), "list packages"),
        pm_disable_user: detect_subcommand_in_help(combined_pkg_help.as_deref(), "disable-user"),
        pm_disable: detect_subcommand_in_help(combined_pkg_help.as_deref(), "disable "),
        pm_disable_until_used: detect_subcommand_in_help(
            combined_pkg_help.as_deref(),
            "disable-until-used",
        ),
        pm_enable: detect_subcommand_in_help(combined_pkg_help.as_deref(), "enable"),
        pm_default_state: detect_subcommand_in_help(combined_pkg_help.as_deref(), "default-state"),
        cmd_package_install_existing: detect_subcommand_in_help(
            cmd_pkg_help.as_deref(),
            "install-existing",
        ),
        am_get_current_user,
        warnings,
    })
}

async fn run_optional_stdout<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    command: &AdbReadOnlyCommand,
    warnings: &mut Vec<String>,
) -> Result<Option<String>, AdtError> {
    match service.execute_read_only(adb_path, command).await {
        Ok(out) => {
            if !out.stdout.trim().is_empty() {
                Ok(Some(out.stdout))
            } else if !out.stderr.trim().is_empty() {
                // Some Android releases print `pm help` / `am help` to stderr even with exit 0.
                Ok(Some(out.stderr))
            } else {
                Ok(None)
            }
        }
        Err(err) if err.is_connection_loss() => Err(err),
        Err(AdtError::ProcessFailed { stderr, .. })
            if matches!(
                command,
                AdbReadOnlyCommand::PmHelp { .. }
                    | AdbReadOnlyCommand::CmdPackageHelp { .. }
                    | AdbReadOnlyCommand::AmHelp { .. }
            ) && (stderr.contains("usage:") || stderr.contains("Package manager")) =>
        {
            // Legacy Android versions often exit non-zero when printing `pm help` or `am help`.
            Ok(Some(stderr))
        }
        Err(err) => {
            warnings.push(format!("{}: {err}", command.label()));
            Ok(None)
        }
    }
}
