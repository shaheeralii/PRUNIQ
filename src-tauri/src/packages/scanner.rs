use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::adb::command_specs::{AdbReadOnlyCommand, PackageListFilter};
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::{AdbService, BULK_SCAN_ADB_TIMEOUT};
use crate::adb::validation::validate_package_name;
use crate::android::app_metadata::resolve_label;
use crate::android::parsers::packages::{parse_dumpsys_package, parse_pm_list_packages};
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;
use crate::core::package_models::{
    EffectivePackageState, InstalledPackage, PackageDetails, PackageInventory,
};
use crate::packages::categorization::categorize_package;

/// Performs a fast bulk read-only package scan (at most 5 ADB commands total).
/// Never runs `dumpsys package` in a loop across all installed packages.
pub async fn scan_installed_packages<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
    user_id: Option<u32>,
) -> Result<PackageInventory, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let mut warnings = Vec::new();

    // 1. Primary bulk discovery with APK paths (-f), falling back to plain `pm list packages`.
    let primary_cmd = AdbReadOnlyCommand::PmListPackages {
        serial: target_serial.clone(),
        filter: PackageListFilter::All,
        include_paths: true,
        user_id,
    };

    let primary_entries = match service
        .execute_read_only_with_timeout(adb_path, &primary_cmd, BULK_SCAN_ADB_TIMEOUT)
        .await
    {
        Ok(out) => parse_pm_list_packages(&out.stdout),
        Err(err) if err.is_connection_loss() => return Err(err),
        Err(err) => {
            warnings.push(format!(
                "pm list packages -f unavailable ({err}); retrying without APK paths."
            ));
            let fallback_cmd = AdbReadOnlyCommand::PmListPackages {
                serial: target_serial.clone(),
                filter: PackageListFilter::All,
                include_paths: false,
                user_id,
            };
            let out = service
                .execute_read_only_with_timeout(adb_path, &fallback_cmd, BULK_SCAN_ADB_TIMEOUT)
                .await?;
            parse_pm_list_packages(&out.stdout)
        }
    };

    // 2. Bulk category & effective-state sets (-s, -3, -e, -d).
    let system_set = fetch_package_set(
        service,
        adb_path,
        &target_serial,
        PackageListFilter::System,
        user_id,
        &mut warnings,
    )
    .await?;

    let third_party_set = fetch_package_set(
        service,
        adb_path,
        &target_serial,
        PackageListFilter::ThirdParty,
        user_id,
        &mut warnings,
    )
    .await?;

    let enabled_set = fetch_package_set(
        service,
        adb_path,
        &target_serial,
        PackageListFilter::Enabled,
        user_id,
        &mut warnings,
    )
    .await?;

    let disabled_set = fetch_package_set(
        service,
        adb_path,
        &target_serial,
        PackageListFilter::Disabled,
        user_id,
        &mut warnings,
    )
    .await?;

    // Merge into a deterministic sorted map keyed by package name.
    let mut apk_by_pkg: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (pkg, path) in primary_entries {
        apk_by_pkg.insert(pkg, path);
    }
    for extra_set in [&system_set, &third_party_set, &enabled_set, &disabled_set] {
        for pkg in extra_set {
            apk_by_pkg.entry(pkg.clone()).or_insert(None);
        }
    }

    let mut packages = Vec::with_capacity(apk_by_pkg.len());
    let mut enabled_count = 0usize;
    let mut disabled_count = 0usize;
    let mut system_count = 0usize;
    let mut third_party_count = 0usize;

    for (package_name, apk_path) in apk_by_pkg {
        let in_system = system_set.contains(&package_name);
        let in_third_party = third_party_set.contains(&package_name);
        let classification =
            categorize_package(&package_name, apk_path.as_deref(), in_system, in_third_party);

        let effective_state = if disabled_set.contains(&package_name) {
            disabled_count += 1;
            EffectivePackageState::Disabled
        } else if enabled_set.contains(&package_name) {
            enabled_count += 1;
            EffectivePackageState::Enabled
        } else {
            EffectivePackageState::Unknown
        };

        if classification.is_system {
            system_count += 1;
        }
        if classification.is_third_party {
            third_party_count += 1;
        }

        let (label, label_source) = resolve_label(&package_name);

        packages.push(InstalledPackage {
            package_name,
            label,
            label_source,
            apk_path,
            effective_state,
            origin: classification.origin,
            partition: classification.partition,
            is_system: classification.is_system,
            is_third_party: classification.is_third_party,
            is_updated_system: classification.is_updated_system,
            is_google: classification.is_google,
            is_oem: classification.is_oem,
            user_id,
        });
    }

    Ok(PackageInventory {
        serial: target_serial,
        queried_user_id: user_id,
        total_count: packages.len(),
        enabled_count,
        disabled_count,
        system_count,
        third_party_count,
        packages,
        warnings,
    })
}

/// Targeted read-only inspection of a single validated package via `dumpsys package`.
pub async fn inspect_package_details<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
    package_name: &str,
    user_id: Option<u32>,
) -> Result<PackageDetails, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    validate_package_name(package_name)?;

    let cmd = AdbReadOnlyCommand::DumpsysPackage {
        serial: verified.serial.clone(),
        package_name: package_name.to_string(),
    };
    let out = service.execute_read_only(adb_path, &cmd).await?;
    Ok(parse_dumpsys_package(package_name, &out.stdout, user_id))
}

async fn fetch_package_set<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    filter: PackageListFilter,
    user_id: Option<u32>,
    warnings: &mut Vec<String>,
) -> Result<HashSet<String>, AdtError> {
    let cmd = AdbReadOnlyCommand::PmListPackages {
        serial: serial.to_string(),
        filter,
        include_paths: false,
        user_id,
    };

    match service
        .execute_read_only_with_timeout(adb_path, &cmd, BULK_SCAN_ADB_TIMEOUT)
        .await
    {
        Ok(out) => Ok(parse_pm_list_packages(&out.stdout)
            .into_iter()
            .map(|(pkg, _)| pkg)
            .collect()),
        Err(err) if err.is_connection_loss() => Err(err),
        Err(err) => {
            warnings.push(format!(
                "pm list packages {:?} unavailable: {err}",
                filter.flag().unwrap_or("all")
            ));
            Ok(HashSet::new())
        }
    }
}
