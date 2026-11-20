# Security Policy

## Local-First Safety Architecture

**PRUNIQ — Android Debloating Toolkit** runs locally on Windows and communicates with connected Android devices over wired USB Debugging (ADB). The implemented backend enforces the following boundaries:

1. **Closed Typed Command Allow-Lists (`src-tauri/src/adb/command_specs.rs`):** Every ADB invocation is constructed from a closed Rust enum (`AdbReadOnlyCommand` for read-only queries, `AdbMutatingCommand` for reversible Phase 4 operations) and passed directly as process `argv` without shell string interpolation.
2. **Input Validation (`src-tauri/src/adb/validation.rs`):** Opaque device serials reject empty values, whitespace, control characters, and lengths over 256 bytes, and device-scoped commands verify the serial against recently discovered authorized devices. Package identifiers use hand-written segment validation (explicitly allowing `android`).
3. **Disable Protections & Confirmation (`src-tauri/src/operations/`):** Disabling 14 core immutable system components (non-exhaustive) and the active or sole detected launcher/keyboard is blocked in Rust, with conservative name-based fallbacks if dynamic probes fail. Mutations require an explicit Android User ID, `confirmed: true`, and an exclusive per-device Tokio mutex lock (`DeviceLockManager`).
4. **Reversible Operations Only:** Implemented mutating commands are restricted to `pm disable-user`, `pm enable`, `pm default-state`, and `pm disable-until-used`. Destructive commands (`pm uninstall`, `pm clear`, `pm install`, `am force-stop`, reboots) are not implemented.
5. **Local Privacy & Redaction (`src-tauri/src/storage/db.rs`, `src-tauri/src/diagnostics/`):** Active ADB serials remain in memory (`SessionCache`) and are mapped to random local UUID v4 records in SQLite (`devices`). Raw shell command strings are not stored. Error strings stored in SQLite and on-demand local diagnostic JSON exports pass through `redact_text`, replacing active serials with `[REDACTED_SERIAL]` and user directory names after `\Users\`, `\users\`, `/Users/`, `/users/`, and `/home/` with `[REDACTED]`.

## Reporting a Vulnerability

If you find a security issue, command-injection vector, protection bypass, or privacy leak:

1. Do **not** disclose exploit details in a public issue.
2. Report privately via GitHub Security Advisories at `https://github.com/shaheeralii/PRUNIQ/security/advisories/new` (once public) or contact `@shaheeralii` privately.
3. Provide affected version, reproduction steps, and impact details.
