use crate::core::errors::AdtError;

/// Upper bound on opaque ADB serial length (covers USB serials, IP:port, and mDNS service names).
pub const MAX_SERIAL_LEN: usize = 256;

/// Upper bound on Android package identifier length.
pub const MAX_PACKAGE_NAME_LEN: usize = 255;

/// Validates an ADB device serial as an opaque identifier:
/// - Must be non-empty.
/// - Must not exceed `MAX_SERIAL_LEN` bytes.
/// - Must not contain whitespace or control characters.
///
/// Authoritative authorization is enforced separately by matching against recently discovered devices.
pub fn validate_device_serial(serial: &str) -> Result<(), AdtError> {
    if serial.is_empty() {
        return Err(AdtError::InvalidDeviceSerial {
            reason: "device serial must not be empty".to_string(),
        });
    }

    if serial.len() > MAX_SERIAL_LEN {
        return Err(AdtError::InvalidDeviceSerial {
            reason: format!("device serial exceeds maximum length of {MAX_SERIAL_LEN} bytes"),
        });
    }

    if serial.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AdtError::InvalidDeviceSerial {
            reason: "device serial must not contain whitespace or control characters".to_string(),
        });
    }

    Ok(())
}

/// Hand-written validation for Android package identifiers without regex.
/// Explicitly accepts the special core framework identifier `android`.
pub fn validate_package_name(package_name: &str) -> Result<(), AdtError> {
    if package_name.is_empty() {
        return Err(AdtError::InvalidPackageName {
            reason: "package identifier must not be empty".to_string(),
        });
    }

    if package_name.len() > MAX_PACKAGE_NAME_LEN {
        return Err(AdtError::InvalidPackageName {
            reason: format!(
                "package identifier exceeds maximum length of {MAX_PACKAGE_NAME_LEN} bytes"
            ),
        });
    }

    if package_name == "android" {
        return Ok(());
    }

    let mut segment_count = 0usize;
    for segment in package_name.split('.') {
        segment_count += 1;
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return Err(AdtError::InvalidPackageName {
                reason: "package identifier contains an empty dot-separated segment".to_string(),
            });
        };

        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(AdtError::InvalidPackageName {
                reason: "each segment must start with an ASCII letter or underscore".to_string(),
            });
        }

        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AdtError::InvalidPackageName {
                reason: "segments may only contain ASCII letters, digits, and underscores"
                    .to_string(),
            });
        }
    }

    if segment_count < 2 {
        return Err(AdtError::InvalidPackageName {
            reason: "package identifier must have at least two dot-separated segments (except 'android')".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_opaque_adb_serial_formats() {
        let valid_serials = [
            "R58M91234AB",
            "emulator-5554",
            "192.168.1.42:5555",
            "[fe80::1]:5555",
            "adb-R58M91234AB-aBc123._adb-tls-connect._tcp",
            "0123456789ABCDEF",
        ];

        for serial in valid_serials {
            assert!(
                validate_device_serial(serial).is_ok(),
                "expected serial '{serial}' to be accepted"
            );
        }
    }

    #[test]
    fn accepts_serial_at_exact_max_length() {
        let exact = "a".repeat(MAX_SERIAL_LEN);
        assert!(validate_device_serial(&exact).is_ok());
    }

    #[test]
    fn rejects_empty_serial() {
        assert!(matches!(
            validate_device_serial(""),
            Err(AdtError::InvalidDeviceSerial { .. })
        ));
    }

    #[test]
    fn rejects_serial_exceeding_max_length() {
        let too_long = "a".repeat(MAX_SERIAL_LEN + 1);
        assert!(matches!(
            validate_device_serial(&too_long),
            Err(AdtError::InvalidDeviceSerial { .. })
        ));
    }

    #[test]
    fn rejects_whitespace_and_control_characters() {
        let invalid_serials = [
            "ABC 123",
            "ABC\t123",
            "ABC\n123",
            "ABC\r123",
            "ABC\x00123",
            "ABC\x1b123",
            " leading",
            "trailing ",
        ];

        for serial in invalid_serials {
            assert!(
                matches!(
                    validate_device_serial(serial),
                    Err(AdtError::InvalidDeviceSerial { .. })
                ),
                "expected serial '{serial:?}' to be rejected"
            );
        }
    }

    #[test]
    fn accepts_special_android_and_standard_package_names() {
        let valid = [
            "android",
            "com.android.systemui",
            "com.google.android.gms",
            "com.sec.android.app.launcher",
            "org.example._internal.app_2",
        ];
        for pkg in valid {
            assert!(
                validate_package_name(pkg).is_ok(),
                "expected package '{pkg}' to be valid"
            );
        }
    }

    #[test]
    fn rejects_invalid_or_injected_package_names() {
        let invalid = [
            "",
            "singleword",
            "com..empty",
            ".leading.dot",
            "trailing.dot.",
            "com.example.app;rm -rf /",
            "com.example.app --user 0",
            "-flag.like",
            "com.123numericstart.app",
            "com.example.dash-name",
        ];
        for pkg in invalid {
            assert!(
                matches!(
                    validate_package_name(pkg),
                    Err(AdtError::InvalidPackageName { .. })
                ),
                "expected package '{pkg}' to be rejected"
            );
        }
    }
}
