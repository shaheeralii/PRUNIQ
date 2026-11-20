use serde::{Deserialize, Serialize};

use crate::core::errors::AdtError;
use crate::diagnostics::redaction::redact_text;
use crate::operations::models::PackageOperationResult;
use crate::operations::snapshots::now_epoch_ms;
use crate::rules::models::RuleDatabaseSummary;
use crate::storage::db::Storage;
use crate::storage::models::{AppSettings, DeviceRecord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticAdbSummary {
    pub custom_path_configured: bool,
    pub resolved_executable_redacted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticBundle {
    pub schema_version: u32,
    pub exported_at_ms: u64,
    pub app_version: String,
    pub operating_system: String,
    pub architecture: String,
    pub privacy_guarantees: Vec<String>,
    pub adb: DiagnosticAdbSummary,
    pub settings: AppSettings,
    pub rule_database: RuleDatabaseSummary,
    pub devices: Vec<DeviceRecord>,
    pub recent_operations: Vec<PackageOperationResult>,
}

pub fn build_redacted_diagnostic_bundle(
    storage: &Storage,
    rule_database: RuleDatabaseSummary,
    resolved_adb_path: Option<&str>,
    active_serials: &[String],
) -> Result<DiagnosticBundle, AdtError> {
    let mut settings = storage.get_settings()?;
    if let Some(ref custom) = settings.custom_adb_path {
        settings.custom_adb_path = Some(redact_text(custom, active_serials));
    }

    let devices = storage.list_devices()?;
    let mut recent_operations = storage.list_operation_history(None, None, None, 200)?;

    for op in &mut recent_operations {
        if let Some(ref err) = op.error_message {
            op.error_message = Some(redact_text(err, active_serials));
        }
        if let Some(ref note) = op.limitation_note {
            op.limitation_note = Some(redact_text(note, active_serials));
        }
    }

    let resolved_executable_redacted =
        resolved_adb_path.map(|path| redact_text(path, active_serials));

    Ok(DiagnosticBundle {
        schema_version: 1,
        exported_at_ms: now_epoch_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        privacy_guarantees: vec![
            "Raw hardware ADB serial numbers are never persisted in SQLite and are redacted from this export."
                .to_string(),
            "OS user profile directory names in paths and error messages are redacted as [REDACTED]."
                .to_string(),
            "Raw shell command strings are never stored or exported; only structured action types are recorded."
                .to_string(),
            "Diagnostic export is strictly local and never uploaded automatically.".to_string(),
        ],
        adb: DiagnosticAdbSummary {
            custom_path_configured: settings.custom_adb_path.is_some(),
            resolved_executable_redacted,
        },
        settings,
        rule_database,
        devices,
        recent_operations,
    })
}
