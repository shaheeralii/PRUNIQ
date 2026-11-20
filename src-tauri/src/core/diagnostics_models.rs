use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMemoryEntry {
    pub process_name: String,
    pub pid: Option<u32>,
    pub pss_kb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningProcessEntry {
    pub user: String,
    pub pid: u32,
    pub rss_kb: Option<u64>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    pub snapshot_id: String,
    pub serial: String,
    pub label: String,
    pub captured_at_ms: u64,
    pub capture_duration_ms: u64,
    pub total_kb: Option<u64>,
    pub available_kb: Option<u64>,
    pub free_kb: Option<u64>,
    pub cached_kb: Option<u64>,
    pub buffers_kb: Option<u64>,
    pub swap_total_kb: Option<u64>,
    pub swap_free_kb: Option<u64>,
    pub used_estimated_kb: Option<u64>,
    pub dumpsys_total_ram_kb: Option<u64>,
    pub dumpsys_free_ram_kb: Option<u64>,
    pub dumpsys_used_ram_kb: Option<u64>,
    pub dumpsys_lost_ram_kb: Option<u64>,
    pub top_processes: Vec<ProcessMemoryEntry>,
    pub running_process_count: Option<usize>,
    pub running_processes: Vec<RunningProcessEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMemoryDelta {
    pub process_name: String,
    pub before_pss_kb: Option<u64>,
    pub after_pss_kb: Option<u64>,
    pub delta_kb: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryComparison {
    pub before_snapshot_id: String,
    pub before_label: String,
    pub before_captured_at_ms: u64,
    pub after_snapshot_id: String,
    pub after_label: String,
    pub after_captured_at_ms: u64,
    pub elapsed_ms: i64,
    pub available_delta_kb: Option<i64>,
    pub free_delta_kb: Option<i64>,
    pub cached_delta_kb: Option<i64>,
    pub used_estimated_delta_kb: Option<i64>,
    pub process_count_delta: Option<i64>,
    pub top_process_deltas: Vec<ProcessMemoryDelta>,
    pub interpretation_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePartitionEntry {
    pub filesystem: String,
    pub mount_point: String,
    pub total_kb: Option<u64>,
    pub used_kb: Option<u64>,
    pub available_kb: Option<u64>,
    pub use_percent: Option<u8>,
    pub is_read_only_system_partition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDashboard {
    pub serial: String,
    pub captured_at_ms: u64,
    pub capture_duration_ms: u64,
    pub data_partition: StoragePartitionEntry,
    pub partitions: Vec<StoragePartitionEntry>,
    pub disabling_storage_note: String,
    pub advanced_removal_note: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    pub serial: String,
    pub last_package_scan_ms: Option<u64>,
    pub last_package_count: Option<usize>,
    pub last_memory_probe_ms: Option<u64>,
    pub last_storage_probe_ms: Option<u64>,
}
