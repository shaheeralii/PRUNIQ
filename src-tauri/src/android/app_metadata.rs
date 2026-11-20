use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::core::package_models::LabelSource;

const EMBEDDED_CATALOG_JSON: &str = include_str!("../../../data/app-catalog/common-labels.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppCatalogFile {
    entries: Vec<AppCatalogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppCatalogEntry {
    package_name: String,
    label: String,
}

fn catalog_map() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let parsed: AppCatalogFile = serde_json::from_str(EMBEDDED_CATALOG_JSON)
            .expect("embedded common-labels.json must be valid JSON");
        parsed
            .entries
            .into_iter()
            .map(|entry| (entry.package_name, entry.label))
            .collect()
    })
}

/// Resolves a human-readable label from the curated offline catalog, falling back
/// honestly to the raw package identifier when no verified label is present.
pub fn resolve_label(package_name: &str) -> (String, LabelSource) {
    if let Some(label) = catalog_map().get(package_name) {
        (label.clone(), LabelSource::CuratedCatalog)
    } else {
        (package_name.to_string(), LabelSource::PackageNameFallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_catalog_entries_and_falls_back_honestly() {
        let (label, source) = resolve_label("com.android.systemui");
        assert_eq!(label, "System UI");
        assert_eq!(source, LabelSource::CuratedCatalog);

        let (unknown_label, unknown_source) = resolve_label("com.vendor.unknown.customapp");
        assert_eq!(unknown_label, "com.vendor.unknown.customapp");
        assert_eq!(unknown_source, LabelSource::PackageNameFallback);
    }
}
