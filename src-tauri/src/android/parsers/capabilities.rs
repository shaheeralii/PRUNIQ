use crate::core::device_models::{AndroidUser, CommandSupport, SupportTier};

pub const fn classify_support_tier(api_level: Option<u32>) -> SupportTier {
    match api_level {
        Some(api) if api >= 21 => SupportTier::Primary,
        Some(19..=20) => SupportTier::ExploratoryLegacy,
        Some(_) => SupportTier::BelowExploratory,
        None => SupportTier::Unknown,
    }
}

/// Parses `pm list users` output such as:
/// `UserInfo{0:Owner:c13} running`
/// `UserInfo{10:Work profile:30}`
pub fn parse_pm_list_users(stdout: &str) -> Vec<AndroidUser> {
    let mut users = Vec::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        let Some(start) = line.find("UserInfo{") else {
            continue;
        };
        let after_prefix = &line[start + "UserInfo{".len()..];
        let Some(end) = after_prefix.find('}') else {
            continue;
        };
        let inside = &after_prefix[..end];
        let trailing = &after_prefix[end + 1..];

        let mut parts = inside.splitn(3, ':');
        let Some(id_str) = parts.next() else {
            continue;
        };
        let Ok(id) = id_str.trim().parse::<u32>() else {
            continue;
        };
        let name = parts.next().unwrap_or("User").trim().to_string();
        let running = trailing.contains("running");

        users.push(AndroidUser { id, name, running });
    }

    users
}

pub fn parse_current_user(stdout: &str) -> Option<u32> {
    stdout.trim().parse::<u32>().ok()
}

pub fn detect_subcommand_in_help(help_output: Option<&str>, needle: &str) -> CommandSupport {
    match help_output {
        Some(text) if !text.trim().is_empty() => {
            if text.contains(needle) {
                CommandSupport::Supported
            } else {
                CommandSupport::Unsupported
            }
        }
        _ => CommandSupport::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_provisional_support_tiers() {
        assert_eq!(classify_support_tier(Some(34)), SupportTier::Primary);
        assert_eq!(classify_support_tier(Some(21)), SupportTier::Primary);
        assert_eq!(classify_support_tier(Some(19)), SupportTier::ExploratoryLegacy);
        assert_eq!(classify_support_tier(Some(18)), SupportTier::BelowExploratory);
        assert_eq!(classify_support_tier(None), SupportTier::Unknown);
    }

    #[test]
    fn parses_multiple_android_users() {
        let sample = "\
Users:
\tUserInfo{0:Owner:c13} running
\tUserInfo{10:Work profile:30} running
\tUserInfo{95:DualApp:2000010}
";
        let users = parse_pm_list_users(sample);
        assert_eq!(users.len(), 3);
        assert_eq!(users[0], AndroidUser { id: 0, name: "Owner".to_string(), running: true });
        assert_eq!(users[1], AndroidUser { id: 10, name: "Work profile".to_string(), running: true });
        assert_eq!(users[2], AndroidUser { id: 95, name: "DualApp".to_string(), running: false });
    }

    #[test]
    fn detects_help_capabilities_conservatively() {
        let pm_help = "usage: pm list packages [-f] [-d] [-e] [-s] [-3]\npm disable-user [--user USER_ID] PACKAGE\npm enable PACKAGE\npm default-state PACKAGE\n";
        assert_eq!(detect_subcommand_in_help(Some(pm_help), "disable-user"), CommandSupport::Supported);
        assert_eq!(detect_subcommand_in_help(Some(pm_help), "disable-until-used"), CommandSupport::Unsupported);
        assert_eq!(detect_subcommand_in_help(None, "disable-user"), CommandSupport::Unknown);
    }
}
