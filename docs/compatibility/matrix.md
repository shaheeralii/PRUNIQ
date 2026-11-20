# Android Compatibility Policy & Matrix

ADB connectivity and Package Manager subcommands vary significantly across Android versions and OEM ROMs. PRUNIQ does **not** claim universal Android compatibility.

## Provisional Support Ranges

| Android Range | API Levels | Provisional Tier | Notes |
|---|---|---|---|
| **Android 5.0 and later** | API 21+ | Primary testing range | Primary target for bulk `pm list packages`, `pm disable-user`, `pm enable`, and per-user `dumpsys package` parsing. |
| **Android 4.4** | API 19—20 | Exploratory legacy range | Legacy fallbacks exist in parsers (`df`, `ps`, `pm help`), but real hardware behavior remains largely **NOT TESTED**. |
| **Android 4.3 and earlier** | Below API 19 | Outside target range | Reported as `below_exploratory`. |

## Capability Probing & Restoration Limitations

- **Read-Only Probing Only:** PRUNIQ checks `pm help`, `cmd package help`, `am help`, `am get-current-user`, and `pm list users` instead of probing mutating commands.
- **`pm default-state`:** Exposed only when reported as supported.
- **Rootless Restoration Fidelity:** Restoring `DISABLED (2)` typically requires root, and restoring `DEFAULT (0)` requires `pm default-state`. When exact configured-state restoration is unavailable, PRUNIQ records and displays `effective_match_only` instead of claiming exact restoration.
- **Multi-User Profiles:** Secondary users, Work Profiles, and OEM Dual Apps can hold distinct package states. PRUNIQ requires explicit User ID selection before mutations.

## Hardware Verification Matrix

> Only independently executed physical-device checks may be recorded as verified. All other combinations remain explicitly **NOT TESTED**.

| OEM / Skin | Model / Codename | Android / API | Read-Only Scan | Guarded Disable / Restore Cycle | Verification Status |
|---|---|---|---|---|---|
| Unlisted OEMs | Various | Android 5.0+ (API 21+) | Automated mock/fixture tests only | Automated mock/fixture tests only | **NOT TESTED** on physical hardware until recorded per model. |
| Legacy Android | Various | Android 4.4 (API 19—20) | Parser fixtures only | Not tested | **NOT TESTED** — exploratory legacy range. |
