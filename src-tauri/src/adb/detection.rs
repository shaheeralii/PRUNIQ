use std::env;
use std::path::{Path, PathBuf};

use crate::core::errors::AdtError;
use crate::core::models::{AdbSource, AdbVersionInfo};

/// Returns the OS-specific ADB executable filename.
#[cfg(windows)]
pub const fn adb_binary_name() -> &'static str {
    "adb.exe"
}

#[cfg(not(windows))]
pub const fn adb_binary_name() -> &'static str {
    "adb"
}

/// Resolves ADB executable using either an explicit user-configured path or system PATH.
/// Never downloads binaries or modifies environment variables.
pub fn resolve_adb_executable(
    custom_path: Option<&Path>,
) -> Result<(PathBuf, AdbSource), AdtError> {
    if let Some(path) = custom_path {
        let raw = path.to_string_lossy();
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_file() {
                return Ok((candidate, AdbSource::Custom));
            }
            return Err(AdtError::InvalidAdbPath {
                path: trimmed.to_string(),
            });
        }
    }

    let path_var = env::var_os("PATH").ok_or(AdtError::AdbNotFound)?;
    find_adb_in_search_dirs(env::split_paths(&path_var), adb_binary_name())
        .map(|found| (found, AdbSource::Path))
        .ok_or(AdtError::AdbNotFound)
}

/// Pure helper allowing deterministic unit testing of PATH lookup without modifying global PATH.
pub fn find_adb_in_search_dirs<I>(dirs: I, binary_name: &str) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    for dir in dirs {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Parses stdout from `adb version`.
pub fn parse_adb_version(stdout: &str) -> Result<AdbVersionInfo, AdtError> {
    let mut adb_version: Option<String> = None;
    let mut platform_tools_version: Option<String> = None;
    let mut installed_as: Option<String> = None;

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("Android Debug Bridge version ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                adb_version = Some(trimmed.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("Version ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                platform_tools_version = Some(trimmed.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("Installed as ") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                installed_as = Some(trimmed.to_string());
            }
        }
    }

    let adb_version = adb_version.ok_or_else(|| AdtError::ParseError {
        context: "adb version".to_string(),
        message: "expected 'Android Debug Bridge version <ver>' line in output".to_string(),
    })?;

    Ok(AdbVersionInfo {
        adb_version,
        platform_tools_version,
        installed_as,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("adt_test_{label}_{nanos}"));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn parses_standard_windows_adb_version_output() {
        let sample = "\
Android Debug Bridge version 1.0.41\r\n\
Version 35.0.2-12147458\r\n\
Installed as C:\\Tools\\platform-tools\\adb.exe\r\n\
Running on Windows 10.0.22631\r\n";

        let info = parse_adb_version(sample).expect("valid version output should parse");
        assert_eq!(info.adb_version, "1.0.41");
        assert_eq!(info.platform_tools_version.as_deref(), Some("35.0.2-12147458"));
        assert_eq!(info.installed_as.as_deref(), Some("C:\\Tools\\platform-tools\\adb.exe"));
    }

    #[test]
    fn parses_minimal_legacy_adb_version_output() {
        let sample = "Android Debug Bridge version 1.0.32\nRevision 09a0d98bebce-android\n";
        let info = parse_adb_version(sample).expect("minimal version output should parse");
        assert_eq!(info.adb_version, "1.0.32");
        assert_eq!(info.platform_tools_version, None);
        assert_eq!(info.installed_as, None);
    }

    #[test]
    fn rejects_unrecognized_version_output() {
        let sample = "not an adb binary\n";
        assert!(matches!(
            parse_adb_version(sample),
            Err(AdtError::ParseError { .. })
        ));
    }

    #[test]
    fn finds_adb_in_search_directories() {
        let dir1 = unique_temp_dir("empty");
        let dir2 = unique_temp_dir("with_adb");
        let fake_adb = dir2.join("adb.exe");
        fs::write(&fake_adb, b"placeholder").expect("write temp file");

        let found = find_adb_in_search_dirs(vec![dir1.clone(), dir2.clone()], "adb.exe");
        assert_eq!(found, Some(fake_adb));

        let _ = fs::remove_dir_all(&dir1);
        let _ = fs::remove_dir_all(&dir2);
    }

    #[test]
    fn resolves_valid_custom_path_and_rejects_missing_or_directory() {
        let dir = unique_temp_dir("custom");
        let fake_adb = dir.join("adb.exe");
        fs::write(&fake_adb, b"placeholder").expect("write temp file");

        let (resolved, source) =
            resolve_adb_executable(Some(&fake_adb)).expect("valid custom file should resolve");
        assert_eq!(resolved, fake_adb);
        assert_eq!(source, AdbSource::Custom);

        assert!(matches!(
            resolve_adb_executable(Some(&dir)),
            Err(AdtError::InvalidAdbPath { .. })
        ));

        let missing = dir.join("missing_adb.exe");
        assert!(matches!(
            resolve_adb_executable(Some(&missing)),
            Err(AdtError::InvalidAdbPath { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }
}
