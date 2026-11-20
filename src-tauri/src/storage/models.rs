use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub device_id: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub codename: Option<String>,
    pub android_version: Option<String>,
    pub api_level: Option<u32>,
    pub last_connection_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub custom_adb_path: Option<String>,
    pub auto_scan_on_select: bool,
    pub log_level: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            custom_adb_path: None,
            auto_scan_on_select: false,
            log_level: "standard".to_string(),
        }
    }
}
