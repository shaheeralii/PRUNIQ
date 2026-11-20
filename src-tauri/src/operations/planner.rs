use crate::adb::validation::validate_package_name;
use crate::core::errors::AdtError;
use crate::operations::models::{
    BulkOperationPreview, DeviceProtectionProfile, PackageAction, PlannedPackageAction,
    PreviewOperationsRequest,
};
use crate::operations::protections::check_disable_protections;
use crate::storage::db::Storage;

/// Builds a deterministic, read-only preview of planned single or bulk package actions
/// backed by persisted SQLite snapshots before the user explicitly confirms execution.
pub fn preview_package_operations(
    request: &PreviewOperationsRequest,
    device_id: &str,
    profile: &DeviceProtectionProfile,
    storage: &Storage,
) -> Result<BulkOperationPreview, AdtError> {
    let mut planned = Vec::with_capacity(request.items.len());
    let mut executable_count = 0usize;
    let mut blocked_count = 0usize;
    let mut noop_count = 0usize;

    for item in &request.items {
        validate_package_name(&item.package_name)?;

        let baseline = storage.get_baseline(device_id, request.user_id, &item.package_name)?;
        let pre_op =
            storage.get_latest_pre_operation(device_id, request.user_id, &item.package_name)?;

        let mut blocked_reasons = Vec::new();
        let mut messages = Vec::new();
        let mut allowed = true;
        let mut is_noop = false;

        match item.action {
            PackageAction::KeepCurrent => {
                is_noop = true;
                messages.push("No modification will be made.".to_string());
            }
            PackageAction::Disable => {
                blocked_reasons = check_disable_protections(&item.package_name, profile);
                if !blocked_reasons.is_empty() {
                    allowed = false;
                    for reason in &blocked_reasons {
                        messages.push(reason.explanation().to_string());
                    }
                } else {
                    messages.push(format!(
                        "Will disable package for Android User {} via pm disable-user and verify live state.",
                        request.user_id
                    ));
                }
            }
            PackageAction::Enable => {
                messages.push(format!(
                    "Will enable package for Android User {} via pm enable and verify live state.",
                    request.user_id
                ));
            }
            PackageAction::RestoreDefaultState => {
                if !request.capabilities.pm_default_state_supported {
                    allowed = false;
                    messages.push(
                        "pm default-state is not reported as supported on this device.".to_string(),
                    );
                } else {
                    messages.push(format!(
                        "Will restore factory default component state for User {} via pm default-state.",
                        request.user_id
                    ));
                }
            }
            PackageAction::RestoreOriginalState => {
                if let Some(ref snap) = baseline {
                    messages.push(format!(
                        "Will restore initial toolkit baseline state ({:?}, code {:?}) captured before first modification.",
                        snap.configured_state, snap.configured_state_code
                    ));
                } else {
                    allowed = false;
                    messages.push(
                        "No initial toolkit baseline snapshot exists for this package on this user."
                            .to_string(),
                    );
                }
            }
            PackageAction::UndoLast => {
                if let Some(ref snap) = pre_op {
                    messages.push(format!(
                        "Will undo last change back to pre-operation state ({:?}, code {:?}).",
                        snap.configured_state, snap.configured_state_code
                    ));
                } else {
                    allowed = false;
                    messages.push(
                        "No pre-operation snapshot exists to undo for this package on this user."
                            .to_string(),
                    );
                }
            }
        }

        if is_noop {
            noop_count += 1;
        } else if allowed {
            executable_count += 1;
        } else {
            blocked_count += 1;
        }

        planned.push(PlannedPackageAction {
            package_name: item.package_name.clone(),
            action: item.action,
            allowed,
            is_noop,
            blocked_reasons,
            messages,
            has_baseline_snapshot: baseline.is_some(),
            has_pre_op_snapshot: pre_op.is_some(),
        });
    }

    Ok(BulkOperationPreview {
        serial: request.serial.clone(),
        user_id: request.user_id,
        total_requested: request.items.len(),
        executable_count,
        blocked_count,
        noop_count,
        items: planned,
        warnings: profile.warnings.clone(),
    })
}
