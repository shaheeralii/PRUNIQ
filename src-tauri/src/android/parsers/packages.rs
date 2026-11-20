use crate::adb::validation::validate_package_name;
use crate::android::app_metadata::resolve_label;
use crate::core::package_models::{ConfiguredPackageState, PackageDetails};
use crate::packages::categorization::detect_partition;

/// Parses a single line from `pm list packages` or `pm list packages -f`.
/// Uses `rsplit_once('=')` so Android 11+ `/data/app/~~random==/...==/base.apk=com.example`
/// paths with embedded `=` characters parse cleanly.
pub fn parse_pm_list_packages_line(raw_line: &str) -> Option<(String, Option<String>)> {
    let line = raw_line.trim();
    let rest = line.strip_prefix("package:")?;
    if rest.is_empty() {
        return None;
    }

    let (pkg, apk_path) = match rest.rsplit_once('=') {
        Some((path, pkg_name)) if !path.is_empty() => (pkg_name.trim(), Some(path.trim().to_string())),
        _ => (rest.trim(), None),
    };

    if validate_package_name(pkg).is_ok() {
        Some((pkg.to_string(), apk_path))
    } else {
        None
    }
}

pub fn parse_pm_list_packages(stdout: &str) -> Vec<(String, Option<String>)> {
    stdout
        .lines()
        .filter_map(parse_pm_list_packages_line)
        .collect()
}

/// Parses targeted single-package `dumpsys package <package_name>` output.
pub fn parse_dumpsys_package(
    package_name: &str,
    stdout: &str,
    target_user_id: Option<u32>,
) -> PackageDetails {
    let (label, label_source) = resolve_label(package_name);

    let mut version_name: Option<String> = None;
    let mut version_code: Option<i64> = None;
    let mut code_path: Option<String> = None;
    let mut is_system_flag: Option<bool> = None;
    let mut is_updated_system_flag: Option<bool> = None;
    let mut observed_user_id: Option<u32> = None;
    let mut installed_for_user: Option<bool> = None;
    let mut configured_state = ConfiguredPackageState::Unknown;
    let mut configured_state_code: Option<i32> = None;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();

        if version_code.is_none() {
            if let Some(after) = line.strip_prefix("versionCode=") {
                let token = after.split_whitespace().next().unwrap_or("");
                version_code = token.parse::<i64>().ok();
            }
        }

        if version_name.is_none() {
            if let Some(after) = line.strip_prefix("versionName=") {
                let trimmed = after.trim();
                if !trimmed.is_empty() {
                    version_name = Some(trimmed.to_string());
                }
            }
        }

        if code_path.is_none() {
            if let Some(after) = line.strip_prefix("codePath=") {
                let trimmed = after.trim();
                if !trimmed.is_empty() {
                    code_path = Some(trimmed.to_string());
                }
            }
        }

        if is_system_flag.is_none() && line.starts_with("pkgFlags=[") {
            is_system_flag = Some(line.contains("SYSTEM"));
            is_updated_system_flag = Some(line.contains("UPDATED_SYSTEM_APP"));
        }

        if let Some(after_user) = line.strip_prefix("User ") {
            if let Some((uid_str, rest)) = after_user.split_once(':') {
                if let Ok(uid) = uid_str.trim().parse::<u32>() {
                    let matches_target = target_user_id.map_or(observed_user_id.is_none(), |t| t == uid);
                    if matches_target {
                        observed_user_id = Some(uid);
                        if let Some(installed) = extract_key_value(rest, "installed=") {
                            installed_for_user = match installed {
                                "true" => Some(true),
                                "false" => Some(false),
                                _ => None,
                            };
                        }
                        if let Some(enabled_str) = extract_key_value(rest, "enabled=") {
                            if let Ok(code) = enabled_str.parse::<i32>() {
                                configured_state_code = Some(code);
                                configured_state = ConfiguredPackageState::from_code(code);
                            }
                        }
                    }
                }
            }
        }
    }

    let partition = detect_partition(code_path.as_deref());

    PackageDetails {
        package_name: package_name.to_string(),
        label,
        label_source,
        version_name,
        version_code,
        code_path,
        partition,
        is_system_flag,
        is_updated_system_flag,
        observed_user_id,
        installed_for_user,
        configured_state,
        configured_state_code,
    }
}

fn extract_key_value<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    for token in line.split_whitespace() {
        if let Some(val) = token.strip_prefix(prefix) {
            return Some(val.trim_end_matches(','));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_android_11_plus_paths_containing_equals_signs() {
        let sample = "\
package:/data/app/~~aBcD==/com.google.android.youtube-xYz==/base.apk=com.google.android.youtube\r\n\
package:/system/framework/framework-res.apk=android\r\n\
package:com.android.settings\r\n\
WARNING: ignoring unrelated line\r\n";

        let parsed = parse_pm_list_packages(sample);
        assert_eq!(parsed.len(), 3);
        assert_eq!(
            parsed[0],
            (
                "com.google.android.youtube".to_string(),
                Some("/data/app/~~aBcD==/com.google.android.youtube-xYz==/base.apk".to_string())
            )
        );
        assert_eq!(
            parsed[1],
            (
                "android".to_string(),
                Some("/system/framework/framework-res.apk".to_string())
            )
        );
        assert_eq!(parsed[2], ("com.android.settings".to_string(), None));
    }

    #[test]
    fn parses_dumpsys_package_metadata_and_per_user_state() {
        let sample = "\
Packages:
  Package [com.samsung.android.bixby.agent] (a1b2c3):
    codePath=/system/priv-app/Bixby
    versionCode=320010 minSdk=31 targetSdk=34
    versionName=3.2.10
    pkgFlags=[ SYSTEM HAS_CODE ALLOW_CLEAR_USER_DATA UPDATED_SYSTEM_APP ]
    User 0: ceDataInode=12345 installed=true hidden=false suspended=false stopped=true enabled=3
    User 10: ceDataInode=67890 installed=true hidden=false suspended=false stopped=false enabled=0
";

        let u0 = parse_dumpsys_package("com.samsung.android.bixby.agent", sample, Some(0));
        assert_eq!(u0.label, "Bixby Voice");
        assert_eq!(u0.version_code, Some(320_010));
        assert_eq!(u0.version_name.as_deref(), Some("3.2.10"));
        assert_eq!(u0.code_path.as_deref(), Some("/system/priv-app/Bixby"));
        assert_eq!(u0.is_system_flag, Some(true));
        assert_eq!(u0.is_updated_system_flag, Some(true));
        assert_eq!(u0.observed_user_id, Some(0));
        assert_eq!(u0.installed_for_user, Some(true));
        assert_eq!(u0.configured_state_code, Some(3));
        assert_eq!(u0.configured_state, ConfiguredPackageState::DisabledUser);

        let u10 = parse_dumpsys_package("com.samsung.android.bixby.agent", sample, Some(10));
        assert_eq!(u10.observed_user_id, Some(10));
        assert_eq!(u10.configured_state_code, Some(0));
        assert_eq!(u10.configured_state, ConfiguredPackageState::Default);
    }
}
