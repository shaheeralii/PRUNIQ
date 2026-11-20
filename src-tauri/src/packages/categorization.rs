use crate::core::package_models::{InstallPartition, PackageOrigin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageClassification {
    pub origin: PackageOrigin,
    pub partition: InstallPartition,
    pub is_system: bool,
    pub is_third_party: bool,
    pub is_updated_system: bool,
    pub is_google: bool,
    pub is_oem: bool,
}

const OEM_PREFIXES: &[&str] = &[
    "com.samsung.",
    "com.sec.",
    "com.miui.",
    "com.xiaomi.",
    "com.huawei.",
    "com.hihonor.",
    "com.oppo.",
    "com.coloros.",
    "com.oplus.",
    "com.oneplus.",
    "com.vivo.",
    "com.bbk.",
    "com.realme.",
    "com.motorola.",
    "com.lge.",
    "com.sonyericsson.",
    "com.sonymobile.",
    "com.asus.",
    "com.transsion.",
    "com.tecno.",
    "com.infinix.",
];

const GOOGLE_EXACT: &[&str] = &[
    "com.android.vending",
    "com.android.chrome",
];

pub fn detect_partition(apk_path: Option<&str>) -> InstallPartition {
    let Some(path) = apk_path else {
        return InstallPartition::Unknown;
    };

    if path.starts_with("/system_ext/") || path.starts_with("/system/system_ext/") {
        InstallPartition::SystemExt
    } else if path.starts_with("/product/") || path.starts_with("/system/product/") {
        InstallPartition::Product
    } else if path.starts_with("/vendor/") || path.starts_with("/system/vendor/") {
        InstallPartition::Vendor
    } else if path.starts_with("/odm/") || path.starts_with("/vendor/odm/") {
        InstallPartition::Odm
    } else if path.starts_with("/apex/") {
        InstallPartition::Apex
    } else if path.starts_with("/system/") {
        InstallPartition::System
    } else if path.starts_with("/data/") {
        InstallPartition::Data
    } else {
        InstallPartition::Other
    }
}

/// Multi-signal categorization combining `pm list packages -s/-3`, APK partition, and namespace.
/// Never relies exclusively on a single prefix or installation path.
pub fn categorize_package(
    package_name: &str,
    apk_path: Option<&str>,
    in_system_set: bool,
    in_third_party_set: bool,
) -> PackageClassification {
    let partition = detect_partition(apk_path);
    let partition_is_system = matches!(
        partition,
        InstallPartition::System
            | InstallPartition::SystemExt
            | InstallPartition::Product
            | InstallPartition::Vendor
            | InstallPartition::Odm
            | InstallPartition::Apex
    );

    let is_system = in_system_set || (!in_third_party_set && partition_is_system);
    let is_third_party = in_third_party_set && !in_system_set;
    let is_updated_system = in_system_set && partition == InstallPartition::Data;

    let is_google = package_name.starts_with("com.google.android.")
        || package_name == "com.google.android"
        || GOOGLE_EXACT.contains(&package_name);

    let is_oem = OEM_PREFIXES
        .iter()
        .any(|prefix| package_name.starts_with(prefix))
        || (is_system && matches!(partition, InstallPartition::Vendor | InstallPartition::Odm));

    let origin = if is_google {
        PackageOrigin::Google
    } else if is_oem {
        PackageOrigin::Oem
    } else if is_third_party {
        PackageOrigin::ThirdParty
    } else if is_system {
        PackageOrigin::System
    } else {
        PackageOrigin::Unknown
    };

    PackageClassification {
        origin,
        partition,
        is_system,
        is_third_party,
        is_updated_system,
        is_google,
        is_oem,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_system_updated_system_google_oem_and_third_party() {
        let updated_google = categorize_package(
            "com.google.android.youtube",
            Some("/data/app/~~abc==/com.google.android.youtube/base.apk"),
            true,
            false,
        );
        assert_eq!(updated_google.origin, PackageOrigin::Google);
        assert!(updated_google.is_system);
        assert!(updated_google.is_updated_system);
        assert!(updated_google.is_google);
        assert_eq!(updated_google.partition, InstallPartition::Data);

        let oem_app = categorize_package(
            "com.samsung.android.bixby.agent",
            Some("/system/priv-app/Bixby/Bixby.apk"),
            true,
            false,
        );
        assert_eq!(oem_app.origin, PackageOrigin::Oem);
        assert!(oem_app.is_oem);
        assert!(oem_app.is_system);
        assert!(!oem_app.is_updated_system);

        let user_app = categorize_package(
            "org.mozilla.firefox",
            Some("/data/app/~~def==/org.mozilla.firefox/base.apk"),
            false,
            true,
        );
        assert_eq!(user_app.origin, PackageOrigin::ThirdParty);
        assert!(user_app.is_third_party);
        assert!(!user_app.is_system);
    }
}
