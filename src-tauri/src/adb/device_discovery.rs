use crate::adb::validation::validate_device_serial;
use crate::core::errors::AdtError;
use crate::core::models::{AdbDevice, DeviceConnectionState};

/// Parses stdout from `adb devices -l` into structured device entries.
pub fn parse_devices_long(stdout: &str) -> Result<Vec<AdbDevice>, AdtError> {
    let mut devices = Vec::new();

    for (line_idx, raw_line) in stdout.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.eq_ignore_ascii_case("List of devices attached")
            || line.starts_with("* ")
        {
            continue;
        }

        let (serial, rest) = split_first_token(line).ok_or_else(|| AdtError::ParseError {
            context: "adb devices -l".to_string(),
            message: format!("line {} is missing a device serial", line_idx + 1),
        })?;

        validate_device_serial(serial).map_err(|err| AdtError::ParseError {
            context: "adb devices -l".to_string(),
            message: format!("line {} has invalid serial: {err}", line_idx + 1),
        })?;

        let rest = rest.trim_start();
        if rest.is_empty() {
            return Err(AdtError::ParseError {
                context: "adb devices -l".to_string(),
                message: format!(
                    "line {} for '{serial}' is missing a device state",
                    line_idx + 1
                ),
            });
        }

        let (state, raw_state, metadata_str) = parse_state_and_remainder(rest);

        let mut product = None;
        let mut model = None;
        let mut device = None;
        let mut transport_id = None;
        let mut usb = None;

        for token in metadata_str.split_whitespace() {
            if let Some((key, value)) = token.split_once(':') {
                if value.is_empty() {
                    continue;
                }
                match key {
                    "product" => product = Some(value.to_string()),
                    "model" => model = Some(value.to_string()),
                    "device" => device = Some(value.to_string()),
                    "transport_id" => transport_id = Some(value.to_string()),
                    "usb" => usb = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        let guidance = guidance_for_state(&state).map(str::to_string);

        devices.push(AdbDevice {
            serial: serial.to_string(),
            state,
            raw_state,
            product,
            model,
            device,
            transport_id,
            usb,
            guidance,
        });
    }

    Ok(devices)
}

/// Validates that an explicitly provided opaque serial matches an authorized device
/// in the provided recently discovered device list.
pub fn verify_authorized_device<'a>(
    serial: &str,
    discovered_devices: &'a [AdbDevice],
) -> Result<&'a AdbDevice, AdtError> {
    validate_device_serial(serial)?;

    let device = discovered_devices
        .iter()
        .find(|d| d.serial == serial)
        .ok_or_else(|| AdtError::DeviceNotFound {
            serial: serial.to_string(),
        })?;

    match device.state {
        DeviceConnectionState::Authorized => Ok(device),
        DeviceConnectionState::Unauthorized => Err(AdtError::DeviceUnauthorized {
            serial: serial.to_string(),
        }),
        DeviceConnectionState::Offline => Err(AdtError::DeviceOffline {
            serial: serial.to_string(),
        }),
        DeviceConnectionState::NoPermissions => Err(AdtError::DeviceNoPermissions {
            serial: serial.to_string(),
        }),
        DeviceConnectionState::Other => Err(AdtError::DeviceNotReady {
            serial: serial.to_string(),
            state: device.raw_state.clone(),
        }),
    }
}

fn split_first_token(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.find(char::is_whitespace) {
        Some(idx) => Some((&trimmed[..idx], &trimmed[idx..])),
        None => Some((trimmed, "")),
    }
}

fn parse_state_and_remainder(rest: &str) -> (DeviceConnectionState, String, &str) {
    if let Some(after_no_perm) = rest.strip_prefix("no permissions") {
        let is_boundary =
            after_no_perm.is_empty() || after_no_perm.starts_with(char::is_whitespace);
        if is_boundary {
            return (
                DeviceConnectionState::NoPermissions,
                "no permissions".to_string(),
                after_no_perm.trim_start(),
            );
        }
    }

    let (state_token, tail) = split_first_token(rest).unwrap_or((rest, ""));
    let state = match state_token {
        "device" => DeviceConnectionState::Authorized,
        "unauthorized" => DeviceConnectionState::Unauthorized,
        "offline" => DeviceConnectionState::Offline,
        _ => DeviceConnectionState::Other,
    };

    (state, state_token.to_string(), tail.trim_start())
}

const fn guidance_for_state(state: &DeviceConnectionState) -> Option<&'static str> {
    match state {
        DeviceConnectionState::Authorized => None,
        DeviceConnectionState::Unauthorized => {
            Some("Unlock the device and accept the 'Allow USB debugging' RSA prompt.")
        }
        DeviceConnectionState::Offline => {
            Some("Reconnect the USB cable or toggle USB debugging off and back on.")
        }
        DeviceConnectionState::NoPermissions => {
            Some("Install OEM USB drivers or grant host USB device permissions.")
        }
        DeviceConnectionState::Other => {
            Some("Boot the device normally into Android with USB debugging enabled.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_device_list() {
        let output = "List of devices attached\r\n\r\n";
        let devices = parse_devices_long(output).expect("empty list should parse");
        assert!(devices.is_empty());
    }

    #[test]
    fn parses_daemon_startup_lines_and_multiple_device_states() {
        let output = "\
* daemon not running; starting now at tcp:5037\r\n\
* daemon started successfully\r\n\
List of devices attached\r\n\
R58M91234AB            device product:beyond1ltexx model:SM_G973F device:beyond1 transport_id:1\r\n\
ABC12345               unauthorized usb:1-2 transport_id:2\r\n\
192.168.1.42:5555      offline transport_id:3\r\n\
XYZ999                 no permissions (missing udev rules); see [http://developer.android.com/tools/device.html] usb:1-4 transport_id:4\r\n\
REC001                 recovery usb:1-5 transport_id:5\r\n";

        let devices = parse_devices_long(output).expect("multi-device output should parse");
        assert_eq!(devices.len(), 5);

        assert_eq!(devices[0].serial, "R58M91234AB");
        assert_eq!(devices[0].state, DeviceConnectionState::Authorized);
        assert_eq!(devices[0].raw_state, "device");
        assert_eq!(devices[0].product.as_deref(), Some("beyond1ltexx"));
        assert_eq!(devices[0].model.as_deref(), Some("SM_G973F"));
        assert_eq!(devices[0].device.as_deref(), Some("beyond1"));
        assert_eq!(devices[0].transport_id.as_deref(), Some("1"));
        assert_eq!(devices[0].guidance, None);

        assert_eq!(devices[1].serial, "ABC12345");
        assert_eq!(devices[1].state, DeviceConnectionState::Unauthorized);
        assert_eq!(devices[1].usb.as_deref(), Some("1-2"));
        assert!(devices[1].guidance.is_some());

        assert_eq!(devices[2].serial, "192.168.1.42:5555");
        assert_eq!(devices[2].state, DeviceConnectionState::Offline);
        assert!(devices[2].guidance.is_some());

        assert_eq!(devices[3].serial, "XYZ999");
        assert_eq!(devices[3].state, DeviceConnectionState::NoPermissions);
        assert_eq!(devices[3].raw_state, "no permissions");
        assert_eq!(devices[3].transport_id.as_deref(), Some("4"));

        assert_eq!(devices[4].serial, "REC001");
        assert_eq!(devices[4].state, DeviceConnectionState::Other);
        assert_eq!(devices[4].raw_state, "recovery");
    }

    #[test]
    fn rejects_malformed_line_without_state() {
        let output = "List of devices attached\nORPHAN_SERIAL_ONLY\n";
        assert!(matches!(
            parse_devices_long(output),
            Err(AdtError::ParseError { .. })
        ));
    }

    #[test]
    fn verifies_explicit_serial_authorization_against_discovered_devices() {
        let output = "\
List of devices attached
AUTH1   device model:Pixel_6 transport_id:1
UNAUTH1 unauthorized transport_id:2
OFF1    offline transport_id:3
NOPERM1 no permissions transport_id:4
REC1    recovery transport_id:5
";
        let devices = parse_devices_long(output).expect("fixture should parse");

        assert!(verify_authorized_device("AUTH1", &devices).is_ok());
        assert!(matches!(
            verify_authorized_device("UNKNOWN", &devices),
            Err(AdtError::DeviceNotFound { .. })
        ));
        assert!(matches!(
            verify_authorized_device("UNAUTH1", &devices),
            Err(AdtError::DeviceUnauthorized { .. })
        ));
        assert!(matches!(
            verify_authorized_device("OFF1", &devices),
            Err(AdtError::DeviceOffline { .. })
        ));
        assert!(matches!(
            verify_authorized_device("NOPERM1", &devices),
            Err(AdtError::DeviceNoPermissions { .. })
        ));
        assert!(matches!(
            verify_authorized_device("REC1", &devices),
            Err(AdtError::DeviceNotReady { .. })
        ));
        assert!(matches!(
            verify_authorized_device("bad serial", &devices),
            Err(AdtError::InvalidDeviceSerial { .. })
        ));
    }
}
