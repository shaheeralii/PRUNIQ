use serde::{Deserialize, Serialize};

use crate::core::package_models::{ConfiguredPackageState, EffectivePackageState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageAction {
    Disable,
    Enable,
    KeepCurrent,
    RestoreDefaultState,
    RestoreOriginalState,
    UndoLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionReason {
    ImmutableSystemComponent,
    ActiveDefaultLauncher,
    OnlyUsableLauncher,
    ActiveDefaultKeyboard,
    OnlyUsableKeyboard,
    LauncherProbeFailedConservative,
    KeyboardProbeFailedConservative,
}

impl ProtectionReason {
    pub const fn explanation(self) -> &'static str {
        match self {
            Self::ImmutableSystemComponent => {
                "Protected immutable core Android system component. Disabling may render the device unusable."
            }
            Self::ActiveDefaultLauncher => {
                "Currently active default home launcher. Disabling would leave the device without an active home screen."
            }
            Self::OnlyUsableLauncher => {
                "Only detected usable launcher on this device. Install and set an alternative launcher before disabling."
            }
            Self::ActiveDefaultKeyboard => {
                "Currently active default input method (keyboard). Disabling could prevent text entry and PIN unlock."
            }
            Self::OnlyUsableKeyboard => {
                "Only detected enabled keyboard on this device. Enable an alternative keyboard before disabling."
            }
            Self::LauncherProbeFailedConservative => {
                "Launcher detection failed on this device; conservatively blocking packages matching launcher heuristics."
            }
            Self::KeyboardProbeFailedConservative => {
                "Keyboard detection failed on this device; conservatively blocking packages matching input-method heuristics."
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProtectionProfile {
    pub serial: String,
    pub immutable_packages: Vec<String>,
    pub active_launcher: Option<String>,
    pub usable_launchers: Vec<String>,
    pub active_keyboard: Option<String>,
    pub enabled_keyboards: Vec<String>,
    pub launcher_detection_uncertain: bool,
    pub keyboard_detection_uncertain: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    InitialBaseline,
    PreOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedPackageState {
    pub effective_state: EffectivePackageState,
    pub configured_state: ConfiguredPackageState,
    pub configured_state_code: Option<i32>,
    pub installed_for_user: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageStateSnapshot {
    pub snapshot_id: String,
    pub serial: String,
    pub user_id: u32,
    pub package_name: String,
    pub snapshot_type: SnapshotType,
    pub effective_state: EffectivePackageState,
    pub configured_state: ConfiguredPackageState,
    pub configured_state_code: Option<i32>,
    pub android_version: Option<String>,
    pub captured_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSnapshotStatus {
    pub package_name: String,
    pub baseline: Option<PackageStateSnapshot>,
    pub latest_pre_operation: Option<PackageStateSnapshot>,
    pub pre_operation_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Succeeded,
    NoOp,
    BlockedByProtection,
    Unsupported,
    UncertainTimeout,
    VerificationMismatch,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorationFidelity {
    NotApplicable,
    Exact,
    EffectiveMatchOnly,
    Unverified,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageActionRequest {
    pub package_name: String,
    pub action: PackageAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityFlags {
    pub pm_default_state_supported: bool,
    pub pm_disable_until_used_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOperationsRequest {
    pub serial: String,
    pub user_id: u32,
    pub items: Vec<PackageActionRequest>,
    pub capabilities: CapabilityFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteOperationsRequest {
    pub serial: String,
    pub user_id: u32,
    pub items: Vec<PackageActionRequest>,
    pub confirmed: bool,
    pub android_version: Option<String>,
    pub capabilities: CapabilityFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedPackageAction {
    pub package_name: String,
    pub action: PackageAction,
    pub allowed: bool,
    pub is_noop: bool,
    pub blocked_reasons: Vec<ProtectionReason>,
    pub messages: Vec<String>,
    pub has_baseline_snapshot: bool,
    pub has_pre_op_snapshot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationPreview {
    pub serial: String,
    pub user_id: u32,
    pub total_requested: usize,
    pub executable_count: usize,
    pub blocked_count: usize,
    pub noop_count: usize,
    pub items: Vec<PlannedPackageAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageOperationResult {
    pub operation_id: String,
    pub serial: String,
    pub user_id: u32,
    pub package_name: String,
    pub action: PackageAction,
    pub status: OperationStatus,
    pub previous_state: Option<CapturedPackageState>,
    pub verified_state: Option<CapturedPackageState>,
    pub restoration_fidelity: RestorationFidelity,
    pub blocked_reasons: Vec<ProtectionReason>,
    pub limitation_note: Option<String>,
    pub error_message: Option<String>,
    pub executed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationResult {
    pub serial: String,
    pub user_id: u32,
    pub results: Vec<PackageOperationResult>,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub uncertain_count: usize,
    pub blocked_count: usize,
    pub noop_count: usize,
    pub aborted_early_reason: Option<String>,
}
