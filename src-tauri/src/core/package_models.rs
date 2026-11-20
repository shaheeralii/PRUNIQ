use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageOrigin {
    System,
    Oem,
    Google,
    ThirdParty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPartition {
    System,
    SystemExt,
    Product,
    Vendor,
    Odm,
    Apex,
    Data,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectivePackageState {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfiguredPackageState {
    Default,
    Enabled,
    Disabled,
    DisabledUser,
    DisabledUntilUsed,
    Unknown,
}

impl ConfiguredPackageState {
    pub const fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Default,
            1 => Self::Enabled,
            2 => Self::Disabled,
            3 => Self::DisabledUser,
            4 => Self::DisabledUntilUsed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelSource {
    CuratedCatalog,
    PackageNameFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub package_name: String,
    pub label: String,
    pub label_source: LabelSource,
    pub apk_path: Option<String>,
    pub effective_state: EffectivePackageState,
    pub origin: PackageOrigin,
    pub partition: InstallPartition,
    pub is_system: bool,
    pub is_third_party: bool,
    pub is_updated_system: bool,
    pub is_google: bool,
    pub is_oem: bool,
    pub user_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDetails {
    pub package_name: String,
    pub label: String,
    pub label_source: LabelSource,
    pub version_name: Option<String>,
    pub version_code: Option<i64>,
    pub code_path: Option<String>,
    pub partition: InstallPartition,
    pub is_system_flag: Option<bool>,
    pub is_updated_system_flag: Option<bool>,
    pub observed_user_id: Option<u32>,
    pub installed_for_user: Option<bool>,
    pub configured_state: ConfiguredPackageState,
    pub configured_state_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInventory {
    pub serial: String,
    pub queried_user_id: Option<u32>,
    pub total_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub system_count: usize,
    pub third_party_count: usize,
    pub packages: Vec<InstalledPackage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageFilter {
    pub search: Option<String>,
    pub origin: Option<PackageOrigin>,
    pub state: Option<EffectivePackageState>,
    pub partition: Option<InstallPartition>,
    pub system_only: bool,
    pub third_party_only: bool,
}
