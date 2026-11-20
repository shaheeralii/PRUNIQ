use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentification {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub codename: Option<String>,
    pub product: Option<String>,
    pub android_version: Option<String>,
    pub api_level: Option<u32>,
    pub raw_api_level: Option<String>,
    pub build_number: Option<String>,
    pub build_fingerprint: Option<String>,
    pub security_patch: Option<String>,
    pub kernel_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub architecture: Option<String>,
    pub supported_abis: Vec<String>,
    pub model: Option<String>,
    pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_kb: Option<u64>,
    pub available_kb: Option<u64>,
    pub free_kb: Option<u64>,
    pub cached_kb: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub mount_point: String,
    pub total_kb: Option<u64>,
    pub used_kb: Option<u64>,
    pub available_kb: Option<u64>,
    pub use_percent: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub physical_size: Option<String>,
    pub override_size: Option<String>,
    pub physical_density: Option<u32>,
    pub override_density: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub level: Option<u8>,
    pub scale: Option<u8>,
    pub temperature_c: Option<f32>,
    pub status: Option<String>,
    pub charging_source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHardware {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub internal_storage: StorageInfo,
    pub display: DisplayInfo,
    pub battery: BatteryInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSpecs {
    pub serial: String,
    pub identification: DeviceIdentification,
    pub hardware: DeviceHardware,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Primary,
    ExploratoryLegacy,
    BelowExploratory,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSupport {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidUser {
    pub id: u32,
    pub name: String,
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCapabilities {
    pub serial: String,
    pub android_version: Option<String>,
    pub api_level: Option<u32>,
    pub support_tier: SupportTier,
    pub active_user_id: Option<u32>,
    pub users: Vec<AndroidUser>,
    pub user_detection_warning: Option<String>,
    pub pm_list_packages_flags: CommandSupport,
    pub pm_disable_user: CommandSupport,
    pub pm_disable: CommandSupport,
    pub pm_disable_until_used: CommandSupport,
    pub pm_enable: CommandSupport,
    pub pm_default_state: CommandSupport,
    pub cmd_package_install_existing: CommandSupport,
    pub am_get_current_user: CommandSupport,
    pub warnings: Vec<String>,
}
