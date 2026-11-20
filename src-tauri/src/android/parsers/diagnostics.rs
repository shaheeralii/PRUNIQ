use crate::core::diagnostics_models::{
    ProcessMemoryEntry, RunningProcessEntry, StoragePartitionEntry,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtendedMeminfo {
    pub total_kb: Option<u64>,
    pub available_kb: Option<u64>,
    pub free_kb: Option<u64>,
    pub cached_kb: Option<u64>,
    pub buffers_kb: Option<u64>,
    pub swap_total_kb: Option<u64>,
    pub swap_free_kb: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DumpsysMeminfoSummary {
    pub total_ram_kb: Option<u64>,
    pub free_ram_kb: Option<u64>,
    pub used_ram_kb: Option<u64>,
    pub lost_ram_kb: Option<u64>,
    pub top_processes: Vec<ProcessMemoryEntry>,
}

pub fn parse_extended_meminfo(stdout: &str) -> ExtendedMeminfo {
    let mut info = ExtendedMeminfo::default();

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
            "Buffers" => info.buffers_kb = value_kb,
            "SwapTotal" => info.swap_total_kb = value_kb,
            "SwapFree" => info.swap_free_kb = value_kb,
            _ => {}
        }
    }

    info
}

/// Parses read-only `dumpsys meminfo` summary lines and the "Total PSS by process" table.
/// Supports both modern `245,120K: com.example (pid 123)` and older `145200 kB: com.example (pid 123)` formats.
pub fn parse_dumpsys_meminfo(stdout: &str, max_processes: usize) -> DumpsysMeminfoSummary {
    let mut summary = DumpsysMeminfoSummary::default();
    let mut in_process_section = false;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();

        if line.starts_with("Total PSS by process:") {
            in_process_section = true;
            continue;
        }

        if in_process_section {
            if line.is_empty()
                || line.starts_with("Total PSS by ")
                || line.starts_with("Total RAM:")
            {
                in_process_section = false;
            } else if let Some(entry) = parse_pss_process_line(line) {
                summary.top_processes.push(entry);
                continue;
            }
        }

        if let Some(rest) = line.strip_prefix("Total RAM:") {
            summary.total_ram_kb = extract_first_kb(rest);
        } else if let Some(rest) = line.strip_prefix("Free RAM:") {
            summary.free_ram_kb = extract_first_kb(rest);
        } else if let Some(rest) = line.strip_prefix("Used RAM:") {
            summary.used_ram_kb = extract_first_kb(rest);
        } else if let Some(rest) = line.strip_prefix("Lost RAM:") {
            summary.lost_ram_kb = extract_first_kb(rest);
        }
    }

    summary
        .top_processes
        .sort_by(|a, b| b.pss_kb.cmp(&a.pss_kb).then_with(|| a.process_name.cmp(&b.process_name)));
    summary.top_processes.truncate(max_processes);
    summary
}

fn parse_pss_process_line(line: &str) -> Option<ProcessMemoryEntry> {
    let (kb_part, rest) = line.split_once(':')?;
    let pss_kb = parse_kb_token(kb_part)?;
    let rest_trimmed = rest.trim();
    if rest_trimmed.is_empty() {
        return None;
    }

    let (name, pid) = if let Some((before_paren, inside_paren)) = rest_trimmed.split_once('(') {
        let parsed_pid = inside_paren
            .trim_start()
            .strip_prefix("pid ")
            .and_then(|after_pid| {
                after_pid
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
                    .and_then(|digits| digits.parse::<u32>().ok())
            });
        (before_paren.trim().to_string(), parsed_pid)
    } else {
        (rest_trimmed.to_string(), None)
    };

    if name.is_empty() {
        return None;
    }

    Some(ProcessMemoryEntry {
        process_name: name,
        pid,
        pss_kb,
    })
}

fn extract_first_kb(rest: &str) -> Option<u64> {
    let trimmed = rest.trim();
    let first = trimmed.split_whitespace().next()?;
    parse_kb_token(first)
}

fn parse_kb_token(raw: &str) -> Option<u64> {
    let cleaned = raw
        .trim()
        .replace(',', "")
        .trim_end_matches("kB")
        .trim_end_matches("KB")
        .trim_end_matches('K')
        .trim_end_matches('k')
        .trim()
        .to_string();
    cleaned.parse::<u64>().ok()
}

/// Parses read-only `ps -A` or legacy `ps` process listings.
/// Returns `(total_parsed_count, capped_relevant_entries)`.
pub fn parse_ps_output(stdout: &str, max_entries: usize) -> (usize, Vec<RunningProcessEntry>) {
    let mut entries = Vec::new();
    let mut total_count = 0usize;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("USER") {
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 8 {
            continue;
        }

        let Ok(pid) = tokens[1].parse::<u32>() else {
            continue;
        };

        let user = tokens[0].to_string();
        let rss_kb = tokens[4].parse::<u64>().ok();
        let name = tokens[tokens.len() - 1].to_string();

        total_count += 1;
        // Skip kernel bracket threads like [kworker/0:1] from the user-facing overview list.
        if !name.starts_with('[') {
            entries.push(RunningProcessEntry {
                user,
                pid,
                rss_kb,
                name,
            });
        }
    }

    entries.sort_by(|a, b| {
        b.rss_kb
            .unwrap_or(0)
            .cmp(&a.rss_kb.unwrap_or(0))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries.truncate(max_entries);
    (total_count, entries)
}

/// Parses multi-partition `df -k` output (including wrapped 2-line Toybox entries).
pub fn parse_df_partitions(stdout: &str) -> Vec<StoragePartitionEntry> {
    let mut partitions = Vec::new();
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx];
        idx += 1;

        if line.starts_with("Filesystem") {
            continue;
        }

        let mut tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() == 1 && idx < lines.len() {
            let next_tokens: Vec<&str> = lines[idx].split_whitespace().collect();
            if next_tokens.len() >= 5 {
                tokens.extend(next_tokens);
                idx += 1;
            }
        }

        if tokens.len() < 6 {
            continue;
        }

        let len = tokens.len();
        let filesystem = tokens[0].to_string();
        let mount_point = tokens[len - 1].to_string();
        let use_percent = tokens[len - 2].trim_end_matches('%').parse::<u8>().ok();
        let available_kb = tokens[len - 3].parse::<u64>().ok();
        let used_kb = tokens[len - 4].parse::<u64>().ok();
        let total_kb = tokens[len - 5].parse::<u64>().ok();

        if total_kb.is_none_or(|t| t == 0) {
            continue;
        }

        let is_read_only_system_partition = matches!(
            mount_point.as_str(),
            "/" | "/system" | "/system_ext" | "/product" | "/vendor" | "/odm" | "/apex"
        ) || mount_point.starts_with("/apex/");

        partitions.push(StoragePartitionEntry {
            filesystem,
            mount_point,
            total_kb,
            used_kb,
            available_kb,
            use_percent,
            is_read_only_system_partition,
        });
    }

    partitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extended_meminfo_including_buffers_and_swap() {
        let sample = "\
MemTotal:        5734400 kB
MemFree:          312000 kB
MemAvailable:    2410000 kB
Buffers:           64000 kB
Cached:          1850000 kB
SwapTotal:       2097152 kB
SwapFree:        1500000 kB
";
        let info = parse_extended_meminfo(sample);
        assert_eq!(info.total_kb, Some(5_734_400));
        assert_eq!(info.buffers_kb, Some(64_000));
        assert_eq!(info.swap_total_kb, Some(2_097_152));
        assert_eq!(info.swap_free_kb, Some(1_500_000));
    }

    #[test]
    fn parses_modern_and_legacy_dumpsys_meminfo_formats() {
        let modern = "\
Total PSS by process:
    245,120K: system (pid 1520)
    198,432K: com.android.systemui (pid 2104 / activities)
     84,210K: com.google.android.gms (pid 3412)

Total PSS by OOM adjustment:
    500,000K: Native

Total RAM: 5,734,400K (status normal)
 Free RAM: 2,410,100K (  420,000K cached pss + 1,680,100K cached kernel +   310,000K free)
 Used RAM: 2,980,200K (2,650,000K used pss +   330,200K kernel)
 Lost RAM:   344,100K
";
        let parsed = parse_dumpsys_meminfo(modern, 10);
        assert_eq!(parsed.total_ram_kb, Some(5_734_400));
        assert_eq!(parsed.free_ram_kb, Some(2_410_100));
        assert_eq!(parsed.used_ram_kb, Some(2_980_200));
        assert_eq!(parsed.lost_ram_kb, Some(344_100));
        assert_eq!(parsed.top_processes.len(), 3);
        assert_eq!(parsed.top_processes[0].process_name, "system");
        assert_eq!(parsed.top_processes[0].pid, Some(1520));
        assert_eq!(parsed.top_processes[1].pss_kb, 198_432);

        let legacy = "\
Total PSS by process:
    145200 kB: system (pid 1012)
Total RAM: 1,920,000 kB (status normal)
 Free RAM:   612,000 kB
";
        let parsed_legacy = parse_dumpsys_meminfo(legacy, 10);
        assert_eq!(parsed_legacy.total_ram_kb, Some(1_920_000));
        assert_eq!(parsed_legacy.free_ram_kb, Some(612_000));
        assert_eq!(parsed_legacy.top_processes[0].pss_kb, 145_200);
    }

    #[test]
    fn parses_ps_and_filters_kernel_threads_from_overview() {
        let sample = "\
USER           PID  PPID        VSZ    RSS WCHAN            ADDR S NAME
root             1     0   10834120   5120 0                   0 S init
root            42     2          0      0 0                   0 S [kworker/0:1]
u0_a145       4821   812   15423100 182400 0                   0 S com.android.chrome
system        1520   812   18423100 245000 0                   0 S system_server
";
        let (total, entries) = parse_ps_output(sample, 10);
        assert_eq!(total, 4);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "system_server");
        assert_eq!(entries[0].rss_kb, Some(245_000));
        assert_eq!(entries[1].name, "com.android.chrome");
    }

    #[test]
    fn parses_wrapped_multi_partition_df_output() {
        let sample = "\
Filesystem      1K-blocks     Used Available Use% Mounted on
tmpfs             2867200        0   2867200   0% /dev
/dev/block/dm-0   3145728  2980000    165728  95% /
/dev/block/platform/soc/1d84000.ufshc/by-name/userdata
                115343360 48234120  67109240  42% /data
/dev/block/dm-2   1572864  1410000    162864  90% /vendor
";
        let parts = parse_df_partitions(sample);
        assert_eq!(parts.len(), 4);
        let data = parts
            .iter()
            .find(|p| p.mount_point == "/data")
            .expect("/data present");
        assert_eq!(data.total_kb, Some(115_343_360));
        assert!(!data.is_read_only_system_partition);

        let vendor = parts
            .iter()
            .find(|p| p.mount_point == "/vendor")
            .expect("/vendor present");
        assert!(vendor.is_read_only_system_partition);
    }
}
