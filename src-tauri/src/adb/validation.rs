use crate::core::errors::AdtError;

/// Upper bound on opaque ADB serial length (covers USB serials, IP:port, and mDNS service names).
pub const MAX_SERIAL_LEN: usize = 256;

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
            "ABC\0123",
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
}
