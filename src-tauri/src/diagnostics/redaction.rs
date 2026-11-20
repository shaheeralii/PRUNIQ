/// Redacts raw hardware ADB serials and OS user profile directory names from strings
/// prior to SQLite storage or explicit diagnostic export.
pub fn redact_text(input: &str, active_serials: &[String]) -> String {
    let mut out = input.to_string();

    for serial in active_serials {
        let trimmed = serial.trim();
        if !trimmed.is_empty() {
            out = out.replace(trimmed, "[REDACTED_SERIAL]");
        }
    }

    out = redact_path_segment(&out, "\\Users\\", '\\');
    out = redact_path_segment(&out, "\\users\\", '\\');
    out = redact_path_segment(&out, "/Users/", '/');
    out = redact_path_segment(&out, "/users/", '/');
    out = redact_path_segment(&out, "/home/", '/');
    out
}

fn redact_path_segment(input: &str, marker: &str, sep: char) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(idx) = rest.find(marker) {
        let prefix_end = idx + marker.len();
        result.push_str(&rest[..prefix_end]);
        let after = &rest[prefix_end..];

        let seg_end = after
            .find(|c: char| c == sep || c == '/' || c == '\\' || c.is_whitespace() || c == '\'' || c == '"')
            .unwrap_or(after.len());

        if seg_end > 0 {
            result.push_str("[REDACTED]");
            rest = &after[seg_end..];
        } else {
            rest = after;
        }
    }

    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_active_serials_and_windows_or_posix_user_paths() {
        let serials = vec!["R58M91234AB".to_string(), "192.168.1.42:5555".to_string()];
        let raw = "ADB at C:\\Users\\Shaheer\\Tools\\adb.exe failed for device 'R58M91234AB' and /home/alice/log.txt on 192.168.1.42:5555";
        let clean = redact_text(raw, &serials);

        assert!(!clean.contains("R58M91234AB"));
        assert!(!clean.contains("192.168.1.42:5555"));
        assert!(!clean.contains("Shaheer"));
        assert!(!clean.contains("alice"));
        assert!(clean.contains("[REDACTED_SERIAL]"));
        assert!(clean.contains("C:\\Users\\[REDACTED]\\Tools\\adb.exe"));
        assert!(clean.contains("/home/[REDACTED]/log.txt"));
    }
}
