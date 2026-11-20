use std::path::Path;

use crate::adb::command_specs::AdbReadOnlyCommand;
use crate::adb::device_discovery::verify_authorized_device;
use crate::adb::process_runner::ProcessRunner;
use crate::adb::service::AdbService;
use crate::android::parsers::getprop::{parse_abi_list, parse_getprop_all, prop_first};
use crate::android::parsers::hardware::{
    parse_battery_dumpsys, parse_cpuinfo, parse_df_data, parse_kernel_version, parse_meminfo,
    parse_wm_density, parse_wm_size,
};
use crate::core::device_models::{
    BatteryInfo, CpuInfo, DeviceHardware, DeviceIdentification, DeviceSpecs, DisplayInfo,
    MemoryInfo, StorageInfo,
};
use crate::core::errors::AdtError;
use crate::core::models::AdbDevice;

/// Collects full read-only device identification and hardware specifications for an
/// explicitly verified, recently discovered authorized device serial.
pub async fn get_device_specs<R: ProcessRunner>(
    service: &AdbService<R>,
    adb_path: &Path,
    serial: &str,
    discovered_devices: &[AdbDevice],
) -> Result<DeviceSpecs, AdtError> {
    let verified = verify_authorized_device(serial, discovered_devices)?;
    let target_serial = verified.serial.clone();
    let mut warnings = Vec::new();

    let props_output = service
        .execute_read_only(
            adb_path,
            &AdbReadOnlyCommand::GetAllProps {
                serial: target_serial.clone(),
            },
        )
        .await?;
    let props = parse_getprop_all(&props_output.stdout);

    let raw_api_level = prop_first(&props, &["ro.build.version.sdk"]);
    let api_level = raw_api_level
        .as_deref()
        .and_then(|v| v.parse::<u32>().ok());

    let kernel_version = match run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::KernelVersion {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_kernel_version(&out),
        None => None,
    };

    let identification = DeviceIdentification {
        manufacturer: prop_first(&props, &["ro.product.manufacturer", "ro.product.vendor.manufacturer"]),
        model: prop_first(&props, &["ro.product.model", "ro.product.vendor.model"]),
        codename: prop_first(&props, &["ro.product.device", "ro.build.product"]),
        product: prop_first(&props, &["ro.product.name", "ro.build.product"]),
        android_version: prop_first(&props, &["ro.build.version.release"]),
        api_level,
        raw_api_level,
        build_number: prop_first(&props, &["ro.build.display.id", "ro.build.id"]),
        build_fingerprint: prop_first(&props, &["ro.build.fingerprint"]),
        security_patch: prop_first(&props, &["ro.build.version.security_patch"]),
        kernel_version,
    };

    let mut cpu = match run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::ReadProcCpuinfo {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_cpuinfo(&out),
        None => CpuInfo::default(),
    };
    cpu.supported_abis = parse_abi_list(&props);
    cpu.architecture = cpu
        .supported_abis
        .first()
        .cloned()
        .or_else(|| prop_first(&props, &["ro.product.cpu.abi"]));
    if cpu.model.is_none() {
        cpu.model = prop_first(&props, &["ro.soc.model", "ro.board.platform", "ro.hardware"]);
    }

    let memory = match run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::ReadProcMeminfo {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_meminfo(&out),
        None => MemoryInfo::default(),
    };

    let internal_storage = match run_optional_probe(
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
        None => StorageInfo {
            mount_point: "/data".to_string(),
            ..StorageInfo::default()
        },
    };

    let mut display = DisplayInfo::default();
    if let Some(out) = run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::WindowSize {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        parse_wm_size(&out, &mut display);
    }
    if let Some(out) = run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::WindowDensity {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        parse_wm_density(&out, &mut display);
    }

    let battery = match run_optional_probe(
        service,
        adb_path,
        &AdbReadOnlyCommand::BatteryDumpsys {
            serial: target_serial.clone(),
        },
        &mut warnings,
    )
    .await?
    {
        Some(out) => parse_battery_dumpsys(&out),
        None => BatteryInfo::default(),
    };

    Ok(DeviceSpecs {
        serial: target_serial,
        identification,
        hardware: DeviceHardware {
            cpu,
            memory,
            internal_storage,
            display,
            battery,
        },
        warnings,
    })
}

async fn run_optional_probe<R: ProcessRunner>(
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
