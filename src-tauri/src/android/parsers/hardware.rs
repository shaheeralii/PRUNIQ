use crate::core::device_models::{BatteryInfo, CpuInfo, DisplayInfo, MemoryInfo, StorageInfo};

pub fn parse_meminfo(stdout: &str) -> MemoryInfo {
    let mut info = MemoryInfo::default();

    for raw_line in stdout.lines() {
        let Some((key, rest)) = raw_line.split_once(':') else {
            continue;
        };
        let value_kb = rest
            .split_whitespace()
            .next()
            .and_then(|token| token.parse::<u64>().ok());

        match key.trim() {
            "MemTotal" => info.total_kb = value_kb,
            "MemAvailable" => info.available_kb = value_kb,
            "MemFree" => info.free_kb = value_kb,
            "Cached" => info.cached_kb = value_kb,
            _ => {}
        }
    }

    info
}

pub fn parse_cpuinfo(stdout: &str) -> CpuInfo {
    let mut core_count = 0u32;
    let mut hardware_model: Option<String> = None;
    let mut fallback_model: Option<String> = None;

    for raw_line in stdout.lines() {
        let Some((key, value)) = raw_line.split_once(':') else {
            continue;
        };
        let key_trimmed = key.trim();
        let val_trimmed = value.trim();

        if key_trimmed == "processor" && val_trimmed.parse::<u32>().is_ok() {
            core_count = core_count.saturating_add(1);
        } else if key_trimmed == "Hardware" && !val_trimmed.is_empty() {
            hardware_model = Some(val_trimmed.to_string());
        } else if (key_trimmed == "model name" || key_trimmed == "Processor")
            && fallback_model.is_none()
            && !val_trimmed.is_empty()
        {
            fallback_model = Some(val_trimmed.to_string());
        }
    }

    CpuInfo {
        architecture: None,
        supported_abis: Vec::new(),
        model: hardware_model.or(fallback_model),
        core_count: if core_count > 0 { Some(core_count) } else { None },
    }
}

/// Parses `df -k /data` across Toybox/POSIX (1-line or wrapped 2-line) and legacy Toolbox formats.
pub fn parse_df_data(stdout: &str) -> StorageInfo {
    let mut info = StorageInfo {
        mount_point: "/data".to_string(),
        ..StorageInfo::default()
    };

    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return info;
    }

    // Legacy Toolbox format: "/data: 12345678K total, 4567890K used, 7777788K available"
    for line in &lines {
        if let Some(rest) = line.strip_prefix("/data:") {
            for part in rest.split(',') {
                let tokens: Vec<&str> = part.split_whitespace().collect();
                if tokens.len() >= 2 {
                    let parsed = parse_size_kb_token(tokens[0]);
                    match tokens[1] {
                        "total" => info.total_kb = parsed,
                        "used" => info.used_kb = parsed,
                        "available" | "free" => info.available_kb = parsed,
                        _ => {}
                    }
                }
            }
            if let (Some(total), Some(used)) = (info.total_kb, info.used_kb) {
                if total > 0 {
                    let pct = (used.saturating_mul(100)) / total;
                    info.use_percent = u8::try_from(pct.min(100)).ok();
                }
            }
            return info;
        }
    }

    // Skip header line ("Filesystem 1K-blocks Used Available Use% Mounted on") and tokenize remainder.
    let body_tokens: Vec<&str> = lines
        .iter()
        .skip(1)
        .flat_map(|line| line.split_whitespace())
        .collect();

    // Expect at least: filesystem, total_kb, used_kb, avail_kb, use_pct, mount_point
    if body_tokens.len() >= 6 {
        let len = body_tokens.len();
        let mount = body_tokens[len - 1];
        let pct_str = body_tokens[len - 2].trim_end_matches('%');
        let avail_kb = body_tokens[len - 3].parse::<u64>().ok();
        let used_kb = body_tokens[len - 4].parse::<u64>().ok();
        let total_kb = body_tokens[len - 5].parse::<u64>().ok();

        if total_kb.is_some() {
            info.mount_point = mount.to_string();
            info.total_kb = total_kb;
            info.used_kb = used_kb;
            info.available_kb = avail_kb;
            info.use_percent = pct_str.parse::<u8>().ok();
        }
    }

    info
}

fn parse_size_kb_token(token: &str) -> Option<u64> {
    let trimmed = token.trim();
    if let Some(num) = trimmed.strip_suffix('K').or_else(|| trimmed.strip_suffix('k')) {
        return num.parse::<u64>().ok();
    }
    trimmed.parse::<u64>().ok()
}

pub fn parse_wm_size(stdout: &str, display: &mut DisplayInfo) {
    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if let Some(val) = line.strip_prefix("Physical size:") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                display.physical_size = Some(trimmed.to_string());
            }
        } else if let Some(val) = line.strip_prefix("Override size:") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                display.override_size = Some(trimmed.to_string());
            }
        }
    }
}

pub fn parse_wm_density(stdout: &str, display: &mut DisplayInfo) {
    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if let Some(val) = line.strip_prefix("Physical density:") {
            display.physical_density = val.trim().parse::<u32>().ok();
        } else if let Some(val) = line.strip_prefix("Override density:") {
            display.override_density = val.trim().parse::<u32>().ok();
        }
    }
}

pub fn parse_battery_dumpsys(stdout: &str) -> BatteryInfo {
    let mut info = BatteryInfo::default();
    let mut ac_powered = false;
    let mut usb_powered = false;
    let mut wireless_powered = false;

    for raw_line in stdout.lines() {
        let Some((key, val)) = raw_line.split_once(':') else {
            continue;
        };
        let key_trimmed = key.trim();
        let val_trimmed = val.trim();

        match key_trimmed {
            "level" => info.level = val_trimmed.parse::<u8>().ok(),
            "scale" => info.scale = val_trimmed.parse::<u8>().ok(),
            "temperature" => {
                if let Ok(tenths) = val_trimmed.parse::<i32>() {
                    info.temperature_c = Some(tenths as f32 / 10.0);
                }
            }
            "status" => {
                info.status = Some(
                    match val_trimmed {
                        "2" => "charging",
                        "3" => "discharging",
                        "4" => "not_charging",
                        "5" => "full",
                        _ => "unknown",
                    }
                    .to_string(),
                );
            }
            "AC powered" => ac_powered = val_trimmed.eq_ignore_ascii_case("true"),
            "USB powered" => usb_powered = val_trimmed.eq_ignore_ascii_case("true"),
            "Wireless powered" => wireless_powered = val_trimmed.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    info.charging_source = if ac_powered {
        Some("ac".to_string())
    } else if usb_powered {
        Some("usb".to_string())
    } else if wireless_powered {
        Some("wireless".to_string())
    } else if info.status.is_some() {
        Some("none".to_string())
    } else {
        None
    };

    info
}

pub fn parse_kernel_version(stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_meminfo_and_cpuinfo() {
        let mem = "\
MemTotal:        5734400 kB\n\
MemFree:          312000 kB\n\
MemAvailable:    2410000 kB\n\
Cached:          1850000 kB\n";
        let parsed_mem = parse_meminfo(mem);
        assert_eq!(parsed_mem.total_kb, Some(5_734_400));
        assert_eq!(parsed_mem.available_kb, Some(2_410_000));
        assert_eq!(parsed_mem.free_kb, Some(312_000));
        assert_eq!(parsed_mem.cached_kb, Some(1_850_000));

        let cpu = "\
processor\t: 0\n\
processor\t: 1\n\
processor\t: 2\n\
processor\t: 3\n\
Hardware\t: Qualcomm Technologies, Inc SM7325\n";
        let parsed_cpu = parse_cpuinfo(cpu);
        assert_eq!(parsed_cpu.core_count, Some(4));
        assert_eq!(
            parsed_cpu.model.as_deref(),
            Some("Qualcomm Technologies, Inc SM7325")
        );
    }

    #[test]
    fn parses_toybox_wrapped_and_legacy_df_outputs() {
        let toybox = "\
Filesystem      1K-blocks     Used Available Use% Mounted on
/dev/block/dm-5 115343360 48234120  67109240  42% /data
";
        let info = parse_df_data(toybox);
        assert_eq!(info.total_kb, Some(115_343_360));
        assert_eq!(info.used_kb, Some(48_234_120));
        assert_eq!(info.available_kb, Some(67_109_240));
        assert_eq!(info.use_percent, Some(42));

        let wrapped = "\
Filesystem      1K-blocks     Used Available Use% Mounted on
/dev/block/platform/soc/1d84000.ufshc/by-name/userdata
                 57671680 28835840  28835840  50% /data
";
        let wrapped_info = parse_df_data(wrapped);
        assert_eq!(wrapped_info.total_kb, Some(57_671_680));
        assert_eq!(wrapped_info.use_percent, Some(50));

        let legacy = "/data: 12000000K total, 6000000K used, 6000000K available\n";
        let legacy_info = parse_df_data(legacy);
        assert_eq!(legacy_info.total_kb, Some(12_000_000));
        assert_eq!(legacy_info.used_kb, Some(6_000_000));
        assert_eq!(legacy_info.available_kb, Some(6_000_000));
        assert_eq!(legacy_info.use_percent, Some(50));
    }

    #[test]
    fn parses_display_and_battery_outputs() {
        let mut display = DisplayInfo::default();
        parse_wm_size("Physical size: 1080x2400\nOverride size: 720x1600\n", &mut display);
        parse_wm_density("Physical density: 420\nOverride density: 360\n", &mut display);
        assert_eq!(display.physical_size.as_deref(), Some("1080x2400"));
        assert_eq!(display.override_size.as_deref(), Some("720x1600"));
        assert_eq!(display.physical_density, Some(420));
        assert_eq!(display.override_density, Some(360));

        let batt = "\
Current Battery Service state:
  AC powered: false
  USB powered: true
  Wireless powered: false
  status: 2
  level: 82
  scale: 100
  temperature: 315
";
        let parsed_batt = parse_battery_dumpsys(batt);
        assert_eq!(parsed_batt.level, Some(82));
        assert_eq!(parsed_batt.scale, Some(100));
        assert_eq!(parsed_batt.temperature_c, Some(31.5));
        assert_eq!(parsed_batt.status.as_deref(), Some("charging"));
        assert_eq!(parsed_batt.charging_source.as_deref(), Some("usb"));
    }
}
