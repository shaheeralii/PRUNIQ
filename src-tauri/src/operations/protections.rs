use std::path::Path;

use crate::adb::command_specs::AdbReadOnlyCommand;
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::adb::validation::validate_package_name;
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;
use crate::operations::models::{DeviceProtectionProfile, ProtectionReason};

/// Immutable core Android components blocked from disable operations.
/// Explicitly non-exhaustive: dynamic launcher/keyboard checks and conservative
/// recommendations supplement this baseline guard.
pub const IMMUTABLE_PROTECTED_PACKAGES: &[&str] = &[
    "android",
    "com.android.systemui",
    "com.android.settings",
    "com.android.phone",
    "com.android.providers.telephony",
    "com.android.server.telecom",
    "com.android.packageinstaller",
    "com.google.android.packageinstaller",
    "com.android.permissioncontroller",
    "com.google.android.permissioncontroller",
    "com.android.providers.settings",
    "com.android.inputdevices",
    "com.android.keychain",
    "com.android.shell",
];

pub fn is_immutable_protected(package_name: &str) -> bool {
    IMMUTABLE_PROTECTED_PACKAGES.contains(&package_name)
}

/// Probes live active/usable launchers and keyboards using strictly read-only ADB queries.
pub async fn probe_device_protections<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
) -> Result<DeviceProtectionProfile, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let mut warnings = Vec::new();

    let active_launcher = match run_read_only_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::PmResolveHome {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_resolved_package(&out),
        None => None,
    };

    let mut usable_launchers = match run_read_only_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::QueryLaunchers {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_component_list_packages(&out),
        None => Vec::new(),
    };

    if let Some(ref active) = active_launcher {
        if !usable_launchers.iter().any(|p| p == active) {
            usable_launchers.push(active.clone());
        }
    }
    usable_launchers.sort();
    usable_launchers.dedup();

    let launcher_detection_uncertain = active_launcher.is_none() && usable_launchers.is_empty();
    if launcher_detection_uncertain {
        warnings.push(
            "Could not verify active or installed launchers; conservative launcher protection fallback is active."
                .to_string(),
        );
    }

    let active_keyboard = match run_read_only_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::GetDefaultInputMethod {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_default_ime_package(&out),
        None => None,
    };

    let mut enabled_keyboards = match run_read_only_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::ListEnabledInputMethods {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_component_list_packages(&out),
        None => Vec::new(),
    };

    if let Some(ref active) = active_keyboard {
        if !enabled_keyboards.iter().any(|p| p == active) {
            enabled_keyboards.push(active.clone());
        }
    }
    enabled_keyboards.sort();
    enabled_keyboards.dedup();

    let keyboard_detection_uncertain = active_keyboard.is_none() && enabled_keyboards.is_empty();
    if keyboard_detection_uncertain {
        warnings.push(
            "Could not verify active or enabled keyboards; conservative keyboard protection fallback is active."
                .to_string(),
        );
    }

    Ok(DeviceProtectionProfile {
        serial: target_serial,
        immutable_packages: IMMUTABLE_PROTECTED_PACKAGES
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        active_launcher,
        usable_launchers,
        active_keyboard,
        enabled_keyboards,
        launcher_detection_uncertain,
        keyboard_detection_uncertain,
        warnings,
    })
}

/// Evaluates whether disabling `package_name` must be blocked.
/// Enabling and restoring packages are never blocked by disable-protections.
pub fn check_disable_protections(
    package_name: &str,
    profile: &DeviceProtectionProfile,
) -> Vec<ProtectionReason> {
    let mut reasons = Vec::new();

    if is_immutable_protected(package_name) {
        reasons.push(ProtectionReason::ImmutableSystemComponent);
    }

    if profile.active_launcher.as_deref() == Some(package_name) {
        reasons.push(ProtectionReason::ActiveDefaultLauncher);
    }

    if profile.usable_launchers.len() == 1
        && profile.usable_launchers[0] == package_name
    {
        reasons.push(ProtectionReason::OnlyUsableLauncher);
    }

    if profile.active_keyboard.as_deref() == Some(package_name) {
        reasons.push(ProtectionReason::ActiveDefaultKeyboard);
    }

    if profile.enabled_keyboards.len() == 1
        && profile.enabled_keyboards[0] == package_name
    {
        reasons.push(ProtectionReason::OnlyUsableKeyboard);
    }

    if profile.launcher_detection_uncertain && matches_launcher_fallback(package_name) {
        reasons.push(ProtectionReason::LauncherProbeFailedConservative);
    }

    if profile.keyboard_detection_uncertain && matches_keyboard_fallback(package_name) {
        reasons.push(ProtectionReason::KeyboardProbeFailedConservative);
    }

    reasons
}

pub fn parse_resolved_package(stdout: &str) -> Option<String> {
    for raw_line in stdout.lines().rev() {
        let line = raw_line.trim();
        if let Some(pkg) = extract_package_from_component(line) {
            return Some(pkg);
        }
    }
    None
}

pub fn parse_default_ime_package(stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return None;
    }
    extract_package_from_component(trimmed)
}

pub fn parse_component_list_packages(stdout: &str) -> Vec<String> {
    let mut packages = Vec::new();
    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if let Some(pkg) = extract_package_from_component(line) {
            if !packages.iter().any(|existing| existing == &pkg) {
                packages.push(pkg);
            }
        }
    }
    packages
}

fn extract_package_from_component(line: &str) -> Option<String> {
    for token in line.split_whitespace() {
        if let Some((pkg_part, component_part)) = token.split_once('/') {
            if !component_part.is_empty() && validate_package_name(pkg_part).is_ok() {
                return Some(pkg_part.to_string());
            }
        }
    }
    None
}

fn matches_launcher_fallback(package_name: &str) -> bool {
    let lower = package_name.to_ascii_lowercase();
    lower.contains("launcher")
        || lower.ends_with(".home")
        || lower.contains("trebuchet")
        || lower.contains("quickstep")
}

fn matches_keyboard_fallback(package_name: &str) -> bool {
    let lower = package_name.to_ascii_lowercase();
    lower.contains("inputmethod")
        || lower.contains("keyboard")
        || lower.contains("honeyboard")
        || lower.contains("gboard")
        || lower.ends_with(".ime")
}

async fn run_read_only_probe<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    command: &AdbReadOnlyCommand,
    warnings: &mut Vec<String>,
) -> Result<Option<String>, AdtError> {
    match service.execute_read_only(adb_path, command).await {
        Ok(out) => Ok(Some(out.stdout)),
        Err(err) if err.is_connection_loss() => Err(err),
        Err(err) => {
            warnings.push(format!("{}: {err}", command.label()));
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_launcher_and_ime_outputs() {
        let home_out = "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\ncom.sec.android.app.launcher/com.android.launcher3.Launcher\n";
        assert_eq!(
            parse_resolved_package(home_out).as_deref(),
            Some("com.sec.android.app.launcher")
        );

        let query_out = "  com.sec.android.app.launcher/.Launcher\n  com.teslacoilsw.launcher/.NovaLauncher\n";
        assert_eq!(
            parse_component_list_packages(query_out),
            vec!["com.sec.android.app.launcher", "com.teslacoilsw.launcher"]
        );

        assert_eq!(
            parse_default_ime_package("com.google.android.inputmethod.latin/com.android.inputmethod.latin.LatinIME\n")
                .as_deref(),
            Some("com.google.android.inputmethod.latin")
        );
        assert_eq!(parse_default_ime_package("null\n"), None);

        let ime_list = "com.samsung.android.honeyboard/.service.HoneyBoardService\ncom.google.android.inputmethod.latin/com.android.inputmethod.latin.LatinIME\n";
        assert_eq!(
            parse_component_list_packages(ime_list),
            vec![
                "com.samsung.android.honeyboard",
                "com.google.android.inputmethod.latin"
            ]
        );
    }

    #[test]
    fn blocks_immutable_active_and_sole_launcher_and_keyboard() {
        let profile = DeviceProtectionProfile {
            serial: "DEV01".to_string(),
            immutable_packages: IMMUTABLE_PROTECTED_PACKAGES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            active_launcher: Some("com.sec.android.app.launcher".to_string()),
            usable_launchers: vec!["com.sec.android.app.launcher".to_string()],
            active_keyboard: Some("com.google.android.inputmethod.latin".to_string()),
            enabled_keyboards: vec!["com.google.android.inputmethod.latin".to_string()],
            launcher_detection_uncertain: false,
            keyboard_detection_uncertain: false,
            warnings: Vec::new(),
        };

        assert!(check_disable_protections("android", &profile)
            .contains(&ProtectionReason::ImmutableSystemComponent));
        assert!(check_disable_protections("com.android.systemui", &profile)
            .contains(&ProtectionReason::ImmutableSystemComponent));

        let launcher_reasons = check_disable_protections("com.sec.android.app.launcher", &profile);
        assert!(launcher_reasons.contains(&ProtectionReason::ActiveDefaultLauncher));
        assert!(launcher_reasons.contains(&ProtectionReason::OnlyUsableLauncher));

        let kb_reasons =
            check_disable_protections("com.google.android.inputmethod.latin", &profile);
        assert!(kb_reasons.contains(&ProtectionReason::ActiveDefaultKeyboard));
        assert!(kb_reasons.contains(&ProtectionReason::OnlyUsableKeyboard));

        assert!(check_disable_protections("com.example.optionalapp", &profile).is_empty());
    }

    #[test]
    fn applies_conservative_fallbacks_when_dynamic_probes_fail() {
        let uncertain_profile = DeviceProtectionProfile {
            serial: "DEV01".to_string(),
            immutable_packages: Vec::new(),
            active_launcher: None,
            usable_launchers: Vec::new(),
            active_keyboard: None,
            enabled_keyboards: Vec::new(),
            launcher_detection_uncertain: true,
            keyboard_detection_uncertain: true,
            warnings: vec!["probes failed".to_string()],
        };

        assert!(check_disable_protections("com.miui.home", &uncertain_profile)
            .contains(&ProtectionReason::LauncherProbeFailedConservative));
        assert!(
            check_disable_protections("com.samsung.android.honeyboard", &uncertain_profile)
                .contains(&ProtectionReason::KeyboardProbeFailedConservative)
        );
    }
}
