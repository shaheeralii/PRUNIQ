use std::path::Path;

use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;
use crate::core::package_models::{ConfiguredPackageState, EffectivePackageState};
use crate::operations::executor::capture_live_package_state;
use crate::operations::locks::DeviceLockManager;
use crate::operations::models::{
    OperationStatus, PackageAction, PackageOperationResult, RestorationFidelity,
};
use crate::storage::db::Storage;

/// Reconciles an interrupted, timed-out, or unverified operation strictly via
/// read-only live device inspection (never automatically retries mutating ADB commands).
pub async fn reconcile_uncertain_operation<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    discovered_devices: &[AdbDevice],
    lock_manager: &DeviceLockManager,
    storage: &Storage,
    serial: &str,
    operation_id: &str,
) -> Result<PackageOperationResult, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();

    let (op, target_token) = storage
        .get_operation_by_id(operation_id)?
        .ok_or_else(|| AdtError::OperationNotFound {
            operation_id: operation_id.to_string(),
        })?;

    let _device_guard = lock_manager.try_lock_device(&target_serial)?;

    let live_state = capture_live_package_state(
        service,
        adb_path,
        &target_serial,
        op.user_id,
        &op.package_name,
    )
    .await?;

    let (expected_eff, expected_cfg) = parse_target_token(target_token.as_deref(), op.action);
    let is_restoration = matches!(
        op.action,
        PackageAction::RestoreDefaultState
            | PackageAction::RestoreOriginalState
            | PackageAction::UndoLast
    );

    let configured_matches = expected_cfg.is_some_and(|c| live_state.configured_state == c);
    let effective_matches = expected_eff == live_state.effective_state;

    let (new_status, new_fidelity, note, err_msg) = if configured_matches || effective_matches {
        let fidelity = if !is_restoration {
            RestorationFidelity::NotApplicable
        } else if configured_matches {
            RestorationFidelity::Exact
        } else {
            RestorationFidelity::EffectiveMatchOnly
        };
        (
            OperationStatus::Succeeded,
            fidelity,
            Some(
                "Reconciled after timeout/disconnect: live device inspection confirmed the target state is active."
                    .to_string(),
            ),
            None,
        )
    } else {
        let fidelity = if is_restoration {
            RestorationFidelity::Failed
        } else {
            RestorationFidelity::NotApplicable
        };
        (
            OperationStatus::VerificationMismatch,
            fidelity,
            Some(format!(
                "Reconciled after timeout/disconnect: package is currently {:?} ({:?}), not {:?}. Safe to re-run manually if desired.",
                live_state.effective_state, live_state.configured_state, expected_eff
            )),
            Some("Live device state does not match the interrupted operation target.".to_string()),
        )
    };

    storage.update_reconciled_operation(
        operation_id,
        new_status,
        &live_state,
        new_fidelity,
        note.as_deref(),
        err_msg.as_deref(),
    )
}

fn parse_target_token(
    token: Option<&str>,
    action: PackageAction,
) -> (EffectivePackageState, Option<ConfiguredPackageState>) {
    if let Some(raw) = token {
        if let Some((eff_str, cfg_str)) = raw.split_once(':') {
            let eff = match eff_str {
                "enabled" => EffectivePackageState::Enabled,
                "disabled" => EffectivePackageState::Disabled,
                _ => EffectivePackageState::Unknown,
            };
            let cfg = match cfg_str {
                "default" => Some(ConfiguredPackageState::Default),
                "enabled" => Some(ConfiguredPackageState::Enabled),
                "disabled" => Some(ConfiguredPackageState::Disabled),
                "disabled_user" => Some(ConfiguredPackageState::DisabledUser),
                "disabled_until_used" => Some(ConfiguredPackageState::DisabledUntilUsed),
                _ => None,
            };
            if eff != EffectivePackageState::Unknown {
                return (eff, cfg);
            }
        }
    }

    match action {
        PackageAction::Disable => (
            EffectivePackageState::Disabled,
            Some(ConfiguredPackageState::DisabledUser),
        ),
        PackageAction::Enable => (
            EffectivePackageState::Enabled,
            Some(ConfiguredPackageState::Enabled),
        ),
        PackageAction::RestoreDefaultState => (
            EffectivePackageState::Enabled,
            Some(ConfiguredPackageState::Default),
        ),
        _ => (EffectivePackageState::Unknown, None),
    }
}
