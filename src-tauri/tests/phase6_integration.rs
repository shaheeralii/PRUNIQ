use std::fs;
use std::path::Path;

use adt_lib::adb::device_discovery::parse_devices_long;
use adt_lib::adb::process_runner::mock::MockProcessRunner;
use adt_lib::adb::process_runner::ProcessOutput;
use adt_lib::adb::service::AdbService;
use adt_lib::android::diagnostics::{
    capture_memory_snapshot, compare_memory_snapshots, get_storage_diagnostics,
};
use adt_lib::core::errors::AdtError;
use adt_lib::core::models::AdbDevice;

fn read_fixture(filename: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture '{}': {e}", path.display()))
}

fn ok_out(stdout: impl Into<String>) -> Result<ProcessOutput, AdtError> {
    Ok(ProcessOutput {
        exit_code: Some(0),
        stdout: stdout.into(),
        stderr: String::new(),
    })
}

fn err_exit(stderr: impl Into<String>) -> Result<ProcessOutput, AdtError> {
    Ok(ProcessOutput {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: stderr.into(),
    })
}

fn sample_devices() -> Vec<AdbDevice> {
    parse_devices_long("List of devices attached\nDEV01 device model:SM_A528B transport_id:1\n")
        .expect("devices fixture")
}

#[tokio::test]
async fn captures_memory_snapshots_with_legacy_ps_fallback_and_computes_deltas() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let dumpsys_before = read_fixture("dumpsys_meminfo_sample.txt");
    let ps_before = read_fixture("ps_sample.txt");

    let dumpsys_after = "\
Total PSS by process:
    245,120K: system (pid 1520)
    198,432K: com.android.systemui (pid 2104 / activities)
     84,210K: com.google.android.gms (pid 3412)

Total RAM: 5,734,400K (status normal)
 Free RAM: 2,522,600K
 Used RAM: 2,867,700K
 Lost RAM:   344,100K
";

    let runner = MockProcessRunner::new(vec![
        // Snapshot 1 (Before): /proc/meminfo, dumpsys meminfo, ps -A fails -> falls back to legacy ps
        ok_out("MemTotal: 5734400 kB\nMemFree: 312000 kB\nMemAvailable: 2410000 kB\nCached: 1850000 kB\n"),
        ok_out(dumpsys_before),
        err_exit("ps: unknown option -A\n"),
        ok_out(ps_before),
        // Snapshot 2 (After): /proc/meminfo, dumpsys meminfo, ps -A succeeds
        ok_out("MemTotal: 5734400 kB\nMemFree: 400000 kB\nMemAvailable: 2560000 kB\nCached: 1900000 kB\n"),
        ok_out(dumpsys_after),
        ok_out("USER PID PPID VSZ RSS WCHAN ADDR S NAME\nroot 1 0 1000 500 0 0 S init\nsystem 1520 812 18000 245000 0 0 S system_server\n"),
    ]);

    let service = AdbService::new(runner);

    let before = capture_memory_snapshot(&service, adb, "DEV01", &devices, Some("Baseline"))
        .await
        .expect("before snapshot");
    let after = capture_memory_snapshot(&service, adb, "DEV01", &devices, Some("After test"))
        .await
        .expect("after snapshot");

    assert_eq!(before.label, "Baseline");
    assert_eq!(before.running_process_count, Some(5));
    assert_eq!(before.top_processes.len(), 4);

    let cmp = compare_memory_snapshots(&before, &after);
    assert_eq!(cmp.available_delta_kb, Some(150_000));
    assert_eq!(cmp.free_delta_kb, Some(88_000));
    assert_eq!(cmp.cached_delta_kb, Some(50_000));
    assert_eq!(cmp.used_estimated_delta_kb, Some(-150_000));
    assert_eq!(cmp.process_count_delta, Some(-3));
    assert_eq!(cmp.top_process_deltas[0].process_name, "com.example.backgroundapp");
    assert_eq!(cmp.top_process_deltas[0].delta_kb, -112_500);
    assert!(!cmp.interpretation_notes.is_empty());

    // Audit strictly read-only invocations: never runs force-stop or kill.
    for inv in service.runner().invocations() {
        assert!(!inv.args.iter().any(|a| a.contains("force-stop") || a == "kill"));
    }
}

#[tokio::test]
async fn captures_read_only_storage_dashboard_with_honest_system_partition_notes() {
    let devices = sample_devices();
    let adb = Path::new("adb.exe");
    let df_all = read_fixture("df_partitions_sample.txt");

    let runner = MockProcessRunner::new(vec![
        ok_out("Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/block/dm-5 115343360 48234120 67109240 42% /data\n"),
        ok_out(df_all),
    ]);

    let service = AdbService::new(runner);
    let dash = get_storage_diagnostics(&service, adb, "DEV01", &devices)
        .await
        .expect("storage dashboard");

    assert_eq!(dash.data_partition.mount_point, "/data");
    assert_eq!(dash.data_partition.use_percent, Some(42));
    assert_eq!(dash.partitions.len(), 5);
    assert!(dash.disabling_storage_note.contains("does NOT delete"));
    assert!(dash.advanced_removal_note.contains("pm uninstall"));
}
