use std::path::Path;

use crate::adb::command_specs::{AdbMutatingCommand, AdbReadOnlyCommand, PackageListFilter};
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::adb::validation::validate_package_name;
use crate::android::parsers::packages::{parse_dumpsys_package, parse_pm_list_packages};
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;
use crate::core::package_models::{ConfiguredPackageState, EffectivePackageState};
use crate::diagnostics::redaction::redact_text;
use crate::operations::locks::DeviceLockManager;
use crate::operations::models::{
    BulkOperationResult, CapabilityFlags, CapturedPackageState, ExecuteOperationsRequest,
    OperationStatus, PackageAction, PackageOperationResult, PackageStateSnapshot,
    RestorationFidelity,
};
use crate::operations::protections::{check_disable_protections, probe_device_protections};
use crate::operations::snapshots::{next_id, now_epoch_ms};
use crate::storage::db::Storage;

struct ResolvedMutationPlan {
    command: AdbMutatingCommand,
    expected_effective: EffectivePackageState,
    expected_configured: Option<ConfiguredPackageState>,
    exact_possible: bool,
    limitation_note: Option<String>,
}

/// Executes confirmed single or bulk package operations under a per-device Tokio lock,
/// persisting immutable baselines, pre-operation snapshots, and sanitized history to SQLite.
pub async fn execute_package_operations<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    discovered_devices: &[AdbDevice],
    lock_manager: &DeviceLockManager,
    storage: &Storage,
    device_id: &str,
    request: &ExecuteOperationsRequest,
) -> Result<BulkOperationResult, AdtError> {
    if !request.confirmed {
        return Err(AdtError::ConfirmationRequired);
    }

    let verified = verify_authorized_device(&request.serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let redact_serials = vec![target_serial.clone()];

    for item in &request.items {
        validate_package_name(&item.package_name)?;
    }

    let _device_guard = lock_manager.try_lock_device(&target_serial)?;

    let has_disable_action = request
        .items
        .iter()
        .any(|i| i.action == PackageAction::Disable);

    let protection_profile = if has_disable_action {
        Some(
            probe_device_protections(service, adb_path, &target_serial, discovered_devices).await?,
        )
    } else {
        None
    };

    let mut results = Vec::with_capacity(request.items.len());
    let mut succeeded_count = 0usize;
    let mut failed_count = 0usize;
    let mut uncertain_count = 0usize;
    let mut blocked_count = 0usize;
    let mut noop_count = 0usize;
    let mut aborted_early_reason: Option<String> = None;

    for item in &request.items {
        let op_id = next_id("op");

        if item.action == PackageAction::KeepCurrent {
            noop_count += 1;
            let res = PackageOperationResult {
                operation_id: op_id,
                device_id: device_id.to_string(),
                user_id: request.user_id,
                package_name: item.package_name.clone(),
                action: item.action,
                status: OperationStatus::NoOp,
                previous_state: None,
                verified_state: None,
                restoration_fidelity: RestorationFidelity::NotApplicable,
                blocked_reasons: Vec::new(),
                limitation_note: None,
                error_message: None,
                executed_at_ms: now_epoch_ms(),
            };
            storage.record_operation_result(&res, None)?;
            results.push(res);
            continue;
        }

        if item.action == PackageAction::Disable {
            if let Some(ref profile) = protection_profile {
                let blocked_reasons = check_disable_protections(&item.package_name, profile);
                if !blocked_reasons.is_empty() {
                    blocked_count += 1;
                    let res = PackageOperationResult {
                        operation_id: op_id,
                        device_id: device_id.to_string(),
                        user_id: request.user_id,
                        package_name: item.package_name.clone(),
                        action: item.action,
                        status: OperationStatus::BlockedByProtection,
                        previous_state: None,
                        verified_state: None,
                        restoration_fidelity: RestorationFidelity::NotApplicable,
                        blocked_reasons,
                        limitation_note: None,
                        error_message: Some(
                            "Operation blocked by ADT package safety protections.".to_string(),
                        ),
                        executed_at_ms: now_epoch_ms(),
                    };
                    storage.record_operation_result(&res, Some("blocked"))?;
                    results.push(res);
                    continue;
                }
            }
        }

        let target_snapshot_opt: Option<PackageStateSnapshot> = match item.action {
            PackageAction::RestoreOriginalState => {
                storage.get_baseline(device_id, request.user_id, &item.package_name)?
            }
            PackageAction::UndoLast => {
                storage.get_latest_pre_operation(device_id, request.user_id, &item.package_name)?
            }
            _ => None,
        };

        let plan = match resolve_mutation_plan(
            &target_serial,
            request.user_id,
            &item.package_name,
            item.action,
            &request.capabilities,
            target_snapshot_opt.as_ref(),
        ) {
            Ok(plan) => plan,
            Err(reason) => {
                failed_count += 1;
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::Unsupported,
                    previous_state: None,
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Failed,
                    blocked_reasons: Vec::new(),
                    limitation_note: Some(reason.clone()),
                    error_message: Some(reason),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, None)?;
                results.push(res);
                continue;
            }
        };

        let target_label = target_state_token(plan.expected_effective, plan.expected_configured);

        let previous_state = match capture_live_package_state(
            service,
            adb_path,
            &target_serial,
            request.user_id,
            &item.package_name,
        )
        .await
        {
            Ok(state) => state,
            Err(err) if err.is_connection_loss() => {
                failed_count += 1;
                let sanitized = redact_text(&err.to_string(), &redact_serials);
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::Failed,
                    previous_state: None,
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Failed,
                    blocked_reasons: Vec::new(),
                    limitation_note: None,
                    error_message: Some(sanitized.clone()),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                aborted_early_reason = Some(format!(
                    "Aborted remaining items after connection loss: {sanitized}"
                ));
                break;
            }
            Err(err) => {
                failed_count += 1;
                let sanitized = redact_text(&err.to_string(), &redact_serials);
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::Failed,
                    previous_state: None,
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Failed,
                    blocked_reasons: Vec::new(),
                    limitation_note: None,
                    error_message: Some(format!(
                        "Failed to capture pre-operation state: {sanitized}"
                    )),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                continue;
            }
        };

        // Persist immutable baseline (if absent) and pre-operation snapshot to SQLite.
        storage.capture_baseline_if_absent(
            device_id,
            request.user_id,
            &item.package_name,
            &previous_state,
            request.android_version.as_deref(),
        )?;
        storage.record_pre_operation(
            device_id,
            request.user_id,
            &item.package_name,
            &previous_state,
            request.android_version.as_deref(),
        )?;

        match service.execute_mutating(adb_path, &plan.command).await {
            Ok(_) => {}
            Err(AdtError::Timeout {
                command,
                timeout_ms,
            }) => {
                uncertain_count += 1;
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::UncertainTimeout,
                    previous_state: Some(previous_state),
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Unverified,
                    blocked_reasons: Vec::new(),
                    limitation_note: Some(
                        "Timed out waiting for local ADB client. The device may still have applied the change; use Verify Live State in History & Recovery before retrying."
                            .to_string(),
                    ),
                    error_message: Some(format!(
                        "Command '{command}' timed out after {timeout_ms} ms (no automatic retry performed)."
                    )),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                continue;
            }
            Err(err) if err.is_connection_loss() => {
                uncertain_count += 1;
                let sanitized = redact_text(&err.to_string(), &redact_serials);
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::UncertainTimeout,
                    previous_state: Some(previous_state),
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Unverified,
                    blocked_reasons: Vec::new(),
                    limitation_note: Some(
                        "Device disconnected while command was in flight. Reconnect and run Verify Live State in History & Recovery."
                            .to_string(),
                    ),
                    error_message: Some(sanitized.clone()),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                aborted_early_reason = Some(format!(
                    "Aborted remaining items after connection loss: {sanitized}"
                ));
                break;
            }
            Err(err) => {
                failed_count += 1;
                let sanitized = redact_text(&err.to_string(), &redact_serials);
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::Failed,
                    previous_state: Some(previous_state),
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Failed,
                    blocked_reasons: Vec::new(),
                    limitation_note: plan.limitation_note,
                    error_message: Some(sanitized),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                continue;
            }
        }

        let verified_state = match capture_live_package_state(
            service,
            adb_path,
            &target_serial,
            request.user_id,
            &item.package_name,
        )
        .await
        {
            Ok(state) => state,
            Err(err) => {
                uncertain_count += 1;
                let sanitized = redact_text(&err.to_string(), &redact_serials);
                let res = PackageOperationResult {
                    operation_id: op_id,
                    device_id: device_id.to_string(),
                    user_id: request.user_id,
                    package_name: item.package_name.clone(),
                    action: item.action,
                    status: OperationStatus::UncertainTimeout,
                    previous_state: Some(previous_state),
                    verified_state: None,
                    restoration_fidelity: RestorationFidelity::Unverified,
                    blocked_reasons: Vec::new(),
                    limitation_note: Some(
                        "Command exited 0, but post-operation verification did not complete. Run Verify Live State in History & Recovery."
                            .to_string(),
                    ),
                    error_message: Some(format!("Verification failed: {sanitized}")),
                    executed_at_ms: now_epoch_ms(),
                };
                storage.record_operation_result(&res, Some(&target_label))?;
                results.push(res);
                if err.is_connection_loss() {
                    aborted_early_reason = Some(format!(
                        "Aborted remaining items after connection loss during verification: {sanitized}"
                    ));
                    break;
                }
                continue;
            }
        };

        let (status, fidelity, extra_note) =
            evaluate_verification(item.action, &plan, &verified_state);

        match status {
            OperationStatus::Succeeded => succeeded_count += 1,
            _ => failed_count += 1,
        }

        let limitation_note = merge_notes(plan.limitation_note, extra_note);

        let res = PackageOperationResult {
            operation_id: op_id,
            device_id: device_id.to_string(),
            user_id: request.user_id,
            package_name: item.package_name.clone(),
            action: item.action,
            status,
            previous_state: Some(previous_state),
            verified_state: Some(verified_state),
            restoration_fidelity: fidelity,
            blocked_reasons: Vec::new(),
            limitation_note,
            error_message: if status == OperationStatus::VerificationMismatch {
                Some(
                    "Post-operation verification showed the package did not reach the expected state."
                        .to_string(),
                )
            } else {
                None
            },
            executed_at_ms: now_epoch_ms(),
        };
        storage.record_operation_result(&res, Some(&target_label))?;
        results.push(res);
    }

    Ok(BulkOperationResult {
        serial: target_serial,
        device_id: device_id.to_string(),
        user_id: request.user_id,
        results,
        succeeded_count,
        failed_count,
        uncertain_count,
        blocked_count,
        noop_count,
        aborted_early_reason,
    })
}

/// Captures live configured + effective package state for `(serial, user_id, package_name)`.
pub async fn capture_live_package_state<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    user_id: u32,
    package_name: &str,
) -> Result<CapturedPackageState, AdtError> {
    let dump_cmd = AdbReadOnlyCommand::DumpsysPackage {
        serial: serial.to_string(),
        package_name: package_name.to_string(),
    };
    let dump_out = service.execute_read_only(adb_path, &dump_cmd).await?;
    let details = parse_dumpsys_package(package_name, &dump_out.stdout, Some(user_id));

    let effective_from_configured = match details.configured_state {
        ConfiguredPackageState::Default | ConfiguredPackageState::Enabled => {
            if details.installed_for_user == Some(false) {
                EffectivePackageState::Unknown
            } else {
                EffectivePackageState::Enabled
            }
        }
        ConfiguredPackageState::Disabled
        | ConfiguredPackageState::DisabledUser
        | ConfiguredPackageState::DisabledUntilUsed => EffectivePackageState::Disabled,
        ConfiguredPackageState::Unknown => EffectivePackageState::Unknown,
    };

    let effective_state = if effective_from_configured != EffectivePackageState::Unknown {
        effective_from_configured
    } else {
        let disabled_cmd = AdbReadOnlyCommand::PmListPackages {
            serial: serial.to_string(),
            filter: PackageListFilter::Disabled,
            include_paths: false,
            user_id: Some(user_id),
        };
        let disabled_out = service.execute_read_only(adb_path, &disabled_cmd).await?;
        let is_disabled = parse_pm_list_packages(&disabled_out.stdout)
            .into_iter()
            .any(|(pkg, _)| pkg == package_name);
        if is_disabled {
            EffectivePackageState::Disabled
        } else if details.installed_for_user == Some(true) {
            EffectivePackageState::Enabled
        } else {
            EffectivePackageState::Unknown
        }
    };

    Ok(CapturedPackageState {
        effective_state,
        configured_state: details.configured_state,
        configured_state_code: details.configured_state_code,
        installed_for_user: details.installed_for_user,
    })
}

fn target_state_token(
    effective: EffectivePackageState,
    configured: Option<ConfiguredPackageState>,
) -> String {
    let eff = match effective {
        EffectivePackageState::Enabled => "enabled",
        EffectivePackageState::Disabled => "disabled",
        EffectivePackageState::Unknown => "unknown",
    };
    let cfg = match configured {
        Some(ConfiguredPackageState::Default) => "default",
        Some(ConfiguredPackageState::Enabled) => "enabled",
        Some(ConfiguredPackageState::Disabled) => "disabled",
        Some(ConfiguredPackageState::DisabledUser) => "disabled_user",
        Some(ConfiguredPackageState::DisabledUntilUsed) => "disabled_until_used",
        Some(ConfiguredPackageState::Unknown) | None => "any",
    };
    format!("{eff}:{cfg}")
}

fn resolve_mutation_plan(
    serial: &str,
    user_id: u32,
    package_name: &str,
    action: PackageAction,
    capabilities: &CapabilityFlags,
    snapshot: Option<&PackageStateSnapshot>,
) -> Result<ResolvedMutationPlan, String> {
    match action {
        PackageAction::Disable => Ok(ResolvedMutationPlan {
            command: AdbMutatingCommand::PmDisableUser {
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
            },
            expected_effective: EffectivePackageState::Disabled,
            expected_configured: Some(ConfiguredPackageState::DisabledUser),
            exact_possible: true,
            limitation_note: None,
        }),
        PackageAction::Enable => Ok(ResolvedMutationPlan {
            command: AdbMutatingCommand::PmEnable {
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
            },
            expected_effective: EffectivePackageState::Enabled,
            expected_configured: Some(ConfiguredPackageState::Enabled),
            exact_possible: true,
            limitation_note: None,
        }),
        PackageAction::RestoreDefaultState => {
            if !capabilities.pm_default_state_supported {
                return Err(
                    "pm default-state is not supported on this device; operation was not attempted."
                        .to_string(),
                );
            }
            Ok(ResolvedMutationPlan {
                command: AdbMutatingCommand::PmDefaultState {
                    serial: serial.to_string(),
                    user_id,
                    package_name: package_name.to_string(),
                },
                expected_effective: EffectivePackageState::Enabled,
                expected_configured: Some(ConfiguredPackageState::Default),
                exact_possible: true,
                limitation_note: None,
            })
        }
        PackageAction::RestoreOriginalState | PackageAction::UndoLast => {
            let Some(snap) = snapshot else {
                return Err(match action {
                    PackageAction::RestoreOriginalState => {
                        "No initial toolkit baseline snapshot exists for this package.".to_string()
                    }
                    _ => "No pre-operation snapshot exists to undo for this package.".to_string(),
                });
            };
            plan_snapshot_restoration(serial, user_id, package_name, snap, capabilities)
        }
        PackageAction::KeepCurrent => {
            Err("KeepCurrent should not invoke mutation planning".to_string())
        }
    }
}

fn plan_snapshot_restoration(
    serial: &str,
    user_id: u32,
    package_name: &str,
    snapshot: &PackageStateSnapshot,
    capabilities: &CapabilityFlags,
) -> Result<ResolvedMutationPlan, String> {
    match snapshot.configured_state {
        ConfiguredPackageState::Default => {
            if capabilities.pm_default_state_supported {
                Ok(ResolvedMutationPlan {
                    command: AdbMutatingCommand::PmDefaultState {
                        serial: serial.to_string(),
                        user_id,
                        package_name: package_name.to_string(),
                    },
                    expected_effective: EffectivePackageState::Enabled,
                    expected_configured: Some(ConfiguredPackageState::Default),
                    exact_possible: true,
                    limitation_note: None,
                })
            } else {
                Ok(ResolvedMutationPlan {
                    command: AdbMutatingCommand::PmEnable {
                        serial: serial.to_string(),
                        user_id,
                        package_name: package_name.to_string(),
                    },
                    expected_effective: EffectivePackageState::Enabled,
                    expected_configured: Some(ConfiguredPackageState::Enabled),
                    exact_possible: false,
                    limitation_note: Some(
                        "pm default-state is not supported on this device; restored effective enabled state via pm enable (ENABLED instead of DEFAULT)."
                            .to_string(),
                    ),
                })
            }
        }
        ConfiguredPackageState::Enabled => Ok(ResolvedMutationPlan {
            command: AdbMutatingCommand::PmEnable {
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
            },
            expected_effective: EffectivePackageState::Enabled,
            expected_configured: Some(ConfiguredPackageState::Enabled),
            exact_possible: true,
            limitation_note: None,
        }),
        ConfiguredPackageState::DisabledUser => Ok(ResolvedMutationPlan {
            command: AdbMutatingCommand::PmDisableUser {
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
            },
            expected_effective: EffectivePackageState::Disabled,
            expected_configured: Some(ConfiguredPackageState::DisabledUser),
            exact_possible: true,
            limitation_note: None,
        }),
        ConfiguredPackageState::DisabledUntilUsed => {
            if capabilities.pm_disable_until_used_supported {
                Ok(ResolvedMutationPlan {
                    command: AdbMutatingCommand::PmDisableUntilUsed {
                        serial: serial.to_string(),
                        user_id,
                        package_name: package_name.to_string(),
                    },
                    expected_effective: EffectivePackageState::Disabled,
                    expected_configured: Some(ConfiguredPackageState::DisabledUntilUsed),
                    exact_possible: true,
                    limitation_note: None,
                })
            } else {
                Ok(ResolvedMutationPlan {
                    command: AdbMutatingCommand::PmDisableUser {
                        serial: serial.to_string(),
                        user_id,
                        package_name: package_name.to_string(),
                    },
                    expected_effective: EffectivePackageState::Disabled,
                    expected_configured: Some(ConfiguredPackageState::DisabledUser),
                    exact_possible: false,
                    limitation_note: Some(
                        "pm disable-until-used is not supported on this device; restored effectively disabled state as DISABLED_USER (3)."
                            .to_string(),
                    ),
                })
            }
        }
        ConfiguredPackageState::Disabled => Ok(ResolvedMutationPlan {
            command: AdbMutatingCommand::PmDisableUser {
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
            },
            expected_effective: EffectivePackageState::Disabled,
            expected_configured: Some(ConfiguredPackageState::DisabledUser),
            exact_possible: false,
            limitation_note: Some(
                "Exact restoration to DISABLED (2) typically requires root; restored effectively disabled state as DISABLED_USER (3)."
                    .to_string(),
            ),
        }),
        ConfiguredPackageState::Unknown => match snapshot.effective_state {
            EffectivePackageState::Enabled => Ok(ResolvedMutationPlan {
                command: AdbMutatingCommand::PmEnable {
                    serial: serial.to_string(),
                    user_id,
                    package_name: package_name.to_string(),
                },
                expected_effective: EffectivePackageState::Enabled,
                expected_configured: None,
                exact_possible: false,
                limitation_note: Some(
                    "Original configured component state code was unavailable; restored effective enabled state via pm enable."
                        .to_string(),
                ),
            }),
            EffectivePackageState::Disabled => Ok(ResolvedMutationPlan {
                command: AdbMutatingCommand::PmDisableUser {
                    serial: serial.to_string(),
                    user_id,
                    package_name: package_name.to_string(),
                },
                expected_effective: EffectivePackageState::Disabled,
                expected_configured: None,
                exact_possible: false,
                limitation_note: Some(
                    "Original configured component state code was unavailable; restored effective disabled state via pm disable-user."
                        .to_string(),
                ),
            }),
            EffectivePackageState::Unknown => Err(
                "Captured snapshot did not contain a verifiable state to restore.".to_string(),
            ),
        },
    }
}

fn evaluate_verification(
    action: PackageAction,
    plan: &ResolvedMutationPlan,
    verified: &CapturedPackageState,
) -> (OperationStatus, RestorationFidelity, Option<String>) {
    let is_restoration = matches!(
        action,
        PackageAction::RestoreDefaultState
            | PackageAction::RestoreOriginalState
            | PackageAction::UndoLast
    );

    let configured_exact = plan
        .expected_configured
        .is_some_and(|expected| verified.configured_state == expected);
    let effective_matches = verified.effective_state == plan.expected_effective;

    if !is_restoration {
        if configured_exact || effective_matches {
            return (
                OperationStatus::Succeeded,
                RestorationFidelity::NotApplicable,
                None,
            );
        }
        return (
            OperationStatus::VerificationMismatch,
            RestorationFidelity::NotApplicable,
            Some(format!(
                "Expected effective state {:?}, but observed {:?} (configured {:?}).",
                plan.expected_effective, verified.effective_state, verified.configured_state
            )),
        );
    }

    if plan.exact_possible && configured_exact {
        (OperationStatus::Succeeded, RestorationFidelity::Exact, None)
    } else if effective_matches {
        let note = if plan.exact_possible {
            Some(format!(
                "Effective state matches ({:?}), but observed configured state {:?} differs from exact target {:?}.",
                verified.effective_state, verified.configured_state, plan.expected_configured
            ))
        } else {
            None
        };
        (
            OperationStatus::Succeeded,
            RestorationFidelity::EffectiveMatchOnly,
            note,
        )
    } else {
        (
            OperationStatus::VerificationMismatch,
            RestorationFidelity::Failed,
            Some(format!(
                "Restoration target was not verified: observed effective {:?}, configured {:?}.",
                verified.effective_state, verified.configured_state
            )),
        )
    }
}

fn merge_notes(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(format!("{x} {y}")),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}
