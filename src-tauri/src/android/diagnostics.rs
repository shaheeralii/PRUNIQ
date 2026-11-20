use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use crate::adb::command_specs::AdbReadOnlyCommand;
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::{AdbService, BULK_SCAN_ADB_TIMEOUT};
use crate::android::parsers::diagnostics::{
    parse_df_partitions, parse_dumpsys_meminfo, parse_extended_meminfo, parse_ps_output,
    DumpsysMeminfoSummary, ExtendedMeminfo,
};
use crate::android::parsers::hardware::parse_df_data;
use crate::core::diagnostics_models::{
    MemoryComparison, MemorySnapshot, ProcessMemoryDelta, StorageDashboard, StoragePartitionEntry,
};
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;
use crate::operations::snapshots::{next_id, now_epoch_ms};

const MAX_PSS_PROCESSES: usize = 25;
const MAX_PS_PROCESSES: usize = 40;
const MAX_DELTA_PROCESSES: usize = 15;

/// Captures a strictly read-only memory and background-process snapshot.
/// Never kills or force-stops any Android process.
pub async fn capture_memory_snapshot<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
    label: Option<&str>,
) -> Result<MemorySnapshot, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let started = Instant::now();
    let mut warnings = Vec::new();

    let meminfo = match run_optional_read_only(
        service,
        adb_path,
        &AdbReadOnlyCommand::ReadProcMeminfo {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_extended_meminfo(&out),
        None => ExtendedMeminfo::default(),
    };

    let dumpsys = match service
        .execute_read_only_with_timeout(
            adb_path,
            &AdbReadOnlyCommand::DumpsysMeminfo {
                serial: target_serial.clone(),
            },
            BULK_SCAN_ADB_TIMEOUT,
        )
        .await
    {
        Ok(out) => parse_dumpsys_meminfo(&out.stdout, MAX_PSS_PROCESSES),
        Err(err) if err.is_connection_loss() => return Err(err),
        Err(err) => {
            warnings.push(format!("adb shell dumpsys meminfo: {err}"));
            DumpsysMeminfoSummary::default()
        }
    };

    let ps_output = match service
        .execute_read_only(
            adb_path,
            &AdbReadOnlyCommand::ProcessList {
                serial: target_serial.clone(),
            },
        )
        .await
    {
        Ok(out) => Some(out.stdout),
        Err(err) if err.is_connection_loss() => return Err(err),
        Err(_) => {
            // Fallback for older Android releases (API 19-25) whose Toolbox `ps` rejects `-A`.
            run_optional_read_only(
                service,
                adb_path,
                &AdbReadOnlyCommand::ProcessListLegacy {
                    serial: target_serial.clone(),
                },
                &mut warnings,
            )
            .await?
        }
    };

    let (running_process_count, running_processes) = match ps_output {
        Some(out) => {
            let (count, list) = parse_ps_output(&out, MAX_PS_PROCESSES);
            (Some(count), list)
        }
        None => (None, Vec::new()),
    };

    let used_estimated_kb = match (meminfo.total_kb, meminfo.available_kb) {
        (Some(total), Some(avail)) => Some(total.saturating_sub(avail)),
        _ => None,
    };

    let normalized_label = label
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "Memory Snapshot".to_string());

    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    Ok(MemorySnapshot {
        snapshot_id: next_id("mem"),
        serial: target_serial,
        label: normalized_label,
        captured_at_ms: now_epoch_ms(),
        capture_duration_ms: duration_ms,
        total_kb: meminfo.total_kb,
        available_kb: meminfo.available_kb,
        free_kb: meminfo.free_kb,
        cached_kb: meminfo.cached_kb,
        buffers_kb: meminfo.buffers_kb,
        swap_total_kb: meminfo.swap_total_kb,
        swap_free_kb: meminfo.swap_free_kb,
        used_estimated_kb,
        dumpsys_total_ram_kb: dumpsys.total_ram_kb,
        dumpsys_free_ram_kb: dumpsys.free_ram_kb,
        dumpsys_used_ram_kb: dumpsys.used_ram_kb,
        dumpsys_lost_ram_kb: dumpsys.lost_ram_kb,
        top_processes: dumpsys.top_processes,
        running_process_count,
        running_processes,
        warnings,
    })
}

/// Deterministic comparison between two captured read-only memory snapshots.
pub fn compare_memory_snapshots(
    before: &MemorySnapshot,
    after: &MemorySnapshot,
) -> MemoryComparison {
    let mut before_by_name: HashMap<&str, u64> = HashMap::new();
    for proc_entry in &before.top_processes {
        *before_by_name
            .entry(proc_entry.process_name.as_str())
            .or_insert(0) += proc_entry.pss_kb;
    }

    let mut after_by_name: HashMap<&str, u64> = HashMap::new();
    for proc_entry in &after.top_processes {
        *after_by_name
            .entry(proc_entry.process_name.as_str())
            .or_insert(0) += proc_entry.pss_kb;
    }

    let mut seen_names: Vec<&str> = before_by_name
        .keys()
        .chain(after_by_name.keys())
        .copied()
        .collect();
    seen_names.sort_unstable();
    seen_names.dedup();

    let mut top_process_deltas: Vec<ProcessMemoryDelta> = seen_names
        .into_iter()
        .filter_map(|name| {
            let b = before_by_name.get(name).copied();
            let a = after_by_name.get(name).copied();
            let delta = delta_i64(Some(b.unwrap_or(0)), Some(a.unwrap_or(0)))?;
            if delta == 0 {
                return None;
            }
            Some(ProcessMemoryDelta {
                process_name: name.to_string(),
                before_pss_kb: b,
                after_pss_kb: a,
                delta_kb: delta,
            })
        })
        .collect();

    top_process_deltas.sort_by(|x, y| {
        y.delta_kb
            .unsigned_abs()
            .cmp(&x.delta_kb.unsigned_abs())
            .then_with(|| x.process_name.cmp(&y.process_name))
    });
    top_process_deltas.truncate(MAX_DELTA_PROCESSES);

    let elapsed_ms = delta_i64(Some(before.captured_at_ms), Some(after.captured_at_ms))
        .unwrap_or(0);

    let process_count_delta = delta_i64(
        before.running_process_count.map(|c| c as u64),
        after.running_process_count.map(|c| c as u64),
    );

    MemoryComparison {
        before_snapshot_id: before.snapshot_id.clone(),
        before_label: before.label.clone(),
        before_captured_at_ms: before.captured_at_ms,
        after_snapshot_id: after.snapshot_id.clone(),
        after_label: after.label.clone(),
        after_captured_at_ms: after.captured_at_ms,
        elapsed_ms,
        available_delta_kb: delta_i64(before.available_kb, after.available_kb),
        free_delta_kb: delta_i64(before.free_kb, after.free_kb),
        cached_delta_kb: delta_i64(before.cached_kb, after.cached_kb),
        used_estimated_delta_kb: delta_i64(before.used_estimated_kb, after.used_estimated_kb),
        process_count_delta,
        top_process_deltas,
        interpretation_notes: vec![
            "Android memory usage fluctuates naturally with background jobs, caching, and screen state."
                .to_string(),
            "Android intentionally keeps recently used data in RAM cache; lower free RAM does not automatically indicate worse performance."
                .to_string(),
            "Disabling applications may reduce unwanted background activity, but does not guarantee a permanent increase in free RAM."
                .to_string(),
        ],
    }
}

/// Collects read-only internal storage and partition diagnostics.
pub async fn get_storage_diagnostics<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
) -> Result<StorageDashboard, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let started = Instant::now();
    let mut warnings = Vec::new();

    let data_info = match run_optional_read_only(
        service,
        adb_path,
        &AdbReadOnlyCommand::DataStorageDf {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_df_data(&out),
        None => Default::default(),
    };

    let partitions = match run_optional_read_only(
        service,
        adb_path,
        &AdbReadOnlyCommand::StoragePartitionsDf {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_df_partitions(&out),
        None => Vec::new(),
    };

    let data_partition = StoragePartitionEntry {
        filesystem: partitions
            .iter()
            .find(|p| p.mount_point == "/data")
            .map_or_else(|| "/data".to_string(), |p| p.filesystem.clone()),
        mount_point: if data_info.mount_point.is_empty() {
            "/data".to_string()
        } else {
            data_info.mount_point
        },
        total_kb: data_info.total_kb,
        used_kb: data_info.used_kb,
        available_kb: data_info.available_kb,
        use_percent: data_info.use_percent,
        is_read_only_system_partition: false,
    };

    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    Ok(StorageDashboard {
        serial: target_serial,
        captured_at_ms: now_epoch_ms(),
        capture_duration_ms: duration_ms,
        data_partition,
        partitions,
        disabling_storage_note: "Disabling a preinstalled system application stops it from running for the selected user, but does NOT delete its factory APK from the read-only /system, /product, or /vendor partition. User storage (/data) is mainly freed by app data, caches, or uninstalling app updates.".to_string(),
        advanced_removal_note: "User-scoped uninstallation (pm uninstall --user) is intentionally excluded from standard disable actions because it can delete app data, does not erase read-only factory APKs, and may face OEM restoration restrictions.".to_string(),
        warnings,
    })
}

fn delta_i64(before: Option<u64>, after: Option<u64>) -> Option<i64> {
    let b = i64::try_from(before?).ok()?;
    let a = i64::try_from(after?).ok()?;
    a.checked_sub(b)
}

async fn run_optional_read_only<R: ProcessRunner>(
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
