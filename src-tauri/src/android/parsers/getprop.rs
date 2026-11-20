use std::collections::BTreeMap;

/// Parses bulk `adb shell getprop` output (`[key]: [value]` lines).
pub fn parse_getprop_all(stdout: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if !line.starts_with('[') {
            continue;
        }
        let Some((key_part, val_part)) = line.split_once("]: [") else {
            continue;
        };
        let key = &key_part[1..];
        if key.is_empty() {
            continue;
        }
        let value = val_part.strip_suffix(']').unwrap_or(val_part).trim();
        if !value.is_empty() {
            map.insert(key.to_string(), value.to_string());
        }
    }

    map
}

pub fn prop_first(props: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = props.get(*key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub fn parse_abi_list(props: &BTreeMap<String, String>) -> Vec<String> {
    let mut abis = Vec::new();
    if let Some(raw) = prop_first(props, &["ro.product.cpu.abilist"]) {
        for item in raw.split(',') {
            let trimmed = item.trim();
            if !trimmed.is_empty() {
                abis.push(trimmed.to_string());
            }
        }
    } else {
        for key in ["ro.product.cpu.abi", "ro.product.cpu.abi2"] {
            if let Some(val) = props.get(key) {
                let trimmed = val.trim();
                if !trimmed.is_empty() && !abis.iter().any(|existing| existing == trimmed) {
                    abis.push(trimmed.to_string());
                }
            }
        }
    }
    abis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_getprop_output_and_skips_empty_values() {
        let sample = "\
[ro.product.manufacturer]: [samsung]\r\n\
[ro.product.model]: [SM-A528B]\r\n\
[ro.product.device]: [a52sxq]\r\n\
[ro.build.version.release]: [14]\r\n\
[ro.build.version.sdk]: [34]\r\n\
[ro.product.cpu.abilist]: [arm64-v8a,armeabi-v7a,armeabi]\r\n\
[ro.empty.prop]: []\r\n";

        let map = parse_getprop_all(sample);
        assert_eq!(map.get("ro.product.manufacturer").map(String::as_str), Some("samsung"));
        assert_eq!(map.get("ro.product.model").map(String::as_str), Some("SM-A528B"));
        assert_eq!(map.get("ro.empty.prop"), None);
        assert_eq!(
            parse_abi_list(&map),
            vec!["arm64-v8a", "armeabi-v7a", "armeabi"]
        );
    }
}
