use std::fs;
use std::path::Path;

use adt_lib::adb::process_runner::mock::MockProcessRunner;
use adt_lib::adb::process_runner::ProcessOutput;
use adt_lib::adb::service::AdbService;
use adt_lib::android::capabilities::probe_device_capabilities;
use adt_lib::android::device_specs::get_device_specs;
use adt_lib::core::device_models::{CommandSupport, SupportTier};
use adt_lib::core::errors::AdtError;
use adt_lib::core::package_models::{
    ConfiguredPackageState, EffectivePackageState, InstallPartition, LabelSource, PackageFilter,
    PackageOrigin,
};
use adt_lib::packages::inventory::filter_packages;
use adt_lib::packages::scanner::{inspect_package_details, scan_installed_packages};

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

#[tokio::test]
async fn full_device_specs_parses_all_probes_and_handles_missing_battery_gracefully() {
    let devices_out = read_fixture("adb_devices_multi.txt");
    let getprop_out = read_fixture("getprop_sample.txt");

    let runner = MockProcessRunner::new(vec![
        ok_out(devices_out),
        ok_out(getprop_out),
        ok_out("5.4.254-android12-9\n"),
        ok_out("processor\t: 0\nprocessor\t: 1\nHardware\t: Qualcomm SM7325\n"),
        ok_out("MemTotal: 6144000 kB\nMemAvailable: 2800000 kB\nMemFree: 400000 kB\nCached: 1900000 kB\n"),
        ok_out("Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/block/dm-5 115343360 48234120 67109240 42% /data\n"),
        ok_out("Physical size: 1080x2400\n"),
        ok_out("Physical density: 420\n"),
        err_exit("Can't find service: battery\n"),
    ]);

    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");
    let devices = service.list_devices(adb).await.expect("devices");

    let specs = get_device_specs(&service, adb, "SECOND_AUTH_DEV", &devices)
        .await
        .expect("specs should succeed even when one probe fails");

    assert_eq!(specs.serial, "SECOND_AUTH_DEV");
    assert_eq!(specs.identification.manufacturer.as_deref(), Some("samsung"));
    assert_eq!(specs.identification.codename.as_deref(), Some("a52sxq"));
    assert_eq!(specs.identification.api_level, Some(34));
    assert_eq!(
        specs.identification.kernel_version.as_deref(),
        Some("5.4.254-android12-9")
    );
    assert_eq!(specs.hardware.cpu.core_count, Some(2));
    assert_eq!(specs.hardware.memory.total_kb, Some(6_144_000));
    assert_eq!(specs.hardware.internal_storage.use_percent, Some(42));
    assert_eq!(specs.hardware.display.physical_size.as_deref(), Some("1080x2400"));
    assert_eq!(specs.hardware.display.physical_density, Some(420));
    assert_eq!(specs.hardware.battery.level, None);
    assert_eq!(specs.warnings.len(), 1);
}

#[tokio::test]
async fn capability_probe_reports_multi_user_warning_without_assuming_user_zero() {
    let devices_out = read_fixture("adb_devices_multi.txt");
    let pm_help = read_fixture("pm_help_android14.txt");

    let runner = MockProcessRunner::new(vec![
        ok_out(devices_out),
        ok_out("14\n"),
        ok_out("34\n"),
        ok_out(pm_help.clone()),
        ok_out(pm_help),
        ok_out("Activity manager commands:\n  get-current-user\n"),
        err_exit("Error: am get-current-user failed\n"),
        ok_out("Users:\n\tUserInfo{0:Owner:c13} running\n\tUserInfo{10:Work:30} running\n"),
    ]);

    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");
    let devices = service.list_devices(adb).await.expect("devices");

    let caps = probe_device_capabilities(&service, adb, "R58M91234AB", &devices)
        .await
        .expect("capabilities should succeed");

    assert_eq!(caps.support_tier, SupportTier::Primary);
    assert_eq!(caps.active_user_id, None);
    assert_eq!(caps.users.len(), 2);
    assert!(caps.user_detection_warning.is_some());
    assert_eq!(caps.pm_disable_user, CommandSupport::Supported);
    assert_eq!(caps.pm_default_state, CommandSupport::Supported);
    assert_eq!(caps.cmd_package_install_existing, CommandSupport::Supported);
}

#[tokio::test]
async fn bulk_package_scan_categorizes_merges_and_filters_with_five_adb_calls() {
    let devices_out = read_fixture("adb_devices_multi.txt");
    let all_f = read_fixture("pm_list_packages_f.txt");
    let sys_list = "\
package:android
package:com.android.systemui
package:com.google.android.youtube
package:com.samsung.android.bixby.agent
package:com.vendor.oemdiag
";
    let third_list = "package:org.mozilla.firefox\n";
    let enabled_list = "\
package:android
package:com.android.systemui
package:com.google.android.youtube
package:org.mozilla.firefox
package:com.vendor.oemdiag
";
    let disabled_list = "package:com.samsung.android.bixby.agent\n";

    let runner = MockProcessRunner::new(vec![
        ok_out(devices_out),
        ok_out(all_f),
        ok_out(sys_list),
        ok_out(third_list),
        ok_out(enabled_list),
        ok_out(disabled_list),
    ]);

    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");
    let devices = service.list_devices(adb).await.expect("devices");

    let inv = scan_installed_packages(&service, adb, "R58M91234AB", &devices, Some(0))
        .await
        .expect("bulk scan should succeed");

    assert_eq!(inv.total_count, 6);
    assert_eq!(inv.enabled_count, 5);
    assert_eq!(inv.disabled_count, 1);
    assert_eq!(inv.system_count, 5);
    assert_eq!(inv.third_party_count, 1);

    // Verify bulk efficiency: 1 devices call + 5 pm list calls = 6 total (zero per-app dumpsys).
    assert_eq!(service.runner().invocations().len(), 6);

    let yt = inv
        .packages
        .iter()
        .find(|p| p.package_name == "com.google.android.youtube")
        .expect("youtube present");
    assert_eq!(yt.label, "YouTube");
    assert_eq!(yt.label_source, LabelSource::CuratedCatalog);
    assert_eq!(yt.origin, PackageOrigin::Google);
    assert!(yt.is_updated_system);
    assert_eq!(yt.partition, InstallPartition::Data);

    let vendor_diag = inv
        .packages
        .iter()
        .find(|p| p.package_name == "com.vendor.oemdiag")
        .expect("vendor diag present");
    assert_eq!(vendor_diag.label, "com.vendor.oemdiag");
    assert_eq!(vendor_diag.label_source, LabelSource::PackageNameFallback);
    assert_eq!(vendor_diag.origin, PackageOrigin::Oem);
    assert_eq!(vendor_diag.partition, InstallPartition::Vendor);

    let disabled_oem = filter_packages(
        &inv.packages,
        &PackageFilter {
            origin: Some(PackageOrigin::Oem),
            state: Some(EffectivePackageState::Disabled),
            ..PackageFilter::default()
        },
    );
    assert_eq!(disabled_oem.len(), 1);
    assert_eq!(disabled_oem[0].package_name, "com.samsung.android.bixby.agent");
}

#[tokio::test]
async fn targeted_dumpsys_package_details_parses_configured_state_and_rejects_bad_names() {
    let devices_out = read_fixture("adb_devices_multi.txt");
    let dump_out = read_fixture("dumpsys_package_bixby.txt");

    let runner = MockProcessRunner::new(vec![ok_out(devices_out), ok_out(dump_out)]);
    let service = AdbService::new(runner);
    let adb = Path::new("adb.exe");
    let devices = service.list_devices(adb).await.expect("devices");

    let details = inspect_package_details(
        &service,
        adb,
        "R58M91234AB",
        &devices,
        "com.samsung.android.bixby.agent",
        Some(0),
    )
    .await
    .expect("targeted inspection should succeed");

    assert_eq!(details.version_name.as_deref(), Some("3.2.10"));
    assert_eq!(details.configured_state, ConfiguredPackageState::DisabledUser);
    assert_eq!(details.configured_state_code, Some(3));

    let bad = inspect_package_details(
        &service,
        adb,
        "R58M91234AB",
        &devices,
        "com.bad;pm disable",
        Some(0),
    )
    .await
    .expect_err("invalid package identifier must be rejected");
    assert!(matches!(bad, AdtError::InvalidPackageName { .. }));
}
