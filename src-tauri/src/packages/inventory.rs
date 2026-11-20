use crate::core::package_models::{InstalledPackage, PackageFilter};

/// Deterministic search and filtering over a scanned package inventory.
pub fn filter_packages<'a>(
    packages: &'a [InstalledPackage],
    filter: &PackageFilter,
) -> Vec<&'a InstalledPackage> {
    let normalized_search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);

    packages
        .iter()
        .filter(|pkg| {
            if let Some(ref needle) = normalized_search {
                let matches_pkg = pkg.package_name.to_ascii_lowercase().contains(needle);
                let matches_label = pkg.label.to_ascii_lowercase().contains(needle);
                if !matches_pkg && !matches_label {
                    return false;
                }
            }
            if let Some(origin) = filter.origin {
                if pkg.origin != origin {
                    return false;
                }
            }
            if let Some(state) = filter.state {
                if pkg.effective_state != state {
                    return false;
                }
            }
            if let Some(partition) = filter.partition {
                if pkg.partition != partition {
                    return false;
                }
            }
            if filter.system_only && !pkg.is_system {
                return false;
            }
            if filter.third_party_only && !pkg.is_third_party {
                return false;
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::package_models::{
        EffectivePackageState, InstallPartition, LabelSource, PackageOrigin,
    };

    fn sample_inventory() -> Vec<InstalledPackage> {
        vec![
            InstalledPackage {
                package_name: "com.samsung.android.bixby.agent".to_string(),
                label: "Bixby Voice".to_string(),
                label_source: LabelSource::CuratedCatalog,
                apk_path: Some("/system/priv-app/Bixby/Bixby.apk".to_string()),
                effective_state: EffectivePackageState::Disabled,
                origin: PackageOrigin::Oem,
                partition: InstallPartition::System,
                is_system: true,
                is_third_party: false,
                is_updated_system: false,
                is_google: false,
                is_oem: true,
                user_id: Some(0),
            },
            InstalledPackage {
                package_name: "org.mozilla.firefox".to_string(),
                label: "org.mozilla.firefox".to_string(),
                label_source: LabelSource::PackageNameFallback,
                apk_path: Some("/data/app/org.mozilla.firefox/base.apk".to_string()),
                effective_state: EffectivePackageState::Enabled,
                origin: PackageOrigin::ThirdParty,
                partition: InstallPartition::Data,
                is_system: false,
                is_third_party: true,
                is_updated_system: false,
                is_google: false,
                is_oem: false,
                user_id: Some(0),
            },
        ]
    }

    #[test]
    fn filters_by_search_label_origin_and_state() {
        let items = sample_inventory();

        let by_label = filter_packages(
            &items,
            &PackageFilter {
                search: Some("bixby".to_string()),
                ..PackageFilter::default()
            },
        );
        assert_eq!(by_label.len(), 1);
        assert_eq!(by_label[0].package_name, "com.samsung.android.bixby.agent");

        let by_state = filter_packages(
            &items,
            &PackageFilter {
                state: Some(EffectivePackageState::Enabled),
                third_party_only: true,
                ..PackageFilter::default()
            },
        );
        assert_eq!(by_state.len(), 1);
        assert_eq!(by_state[0].package_name, "org.mozilla.firefox");
    }
}
