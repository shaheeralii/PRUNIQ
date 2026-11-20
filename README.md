# PRUNIQ — Android Debloating Toolkit (ADT)

**PRUNIQ** (Android Debloating Toolkit) is a local-first Windows desktop utility built with **Tauri 2**, **Rust**, **React**, and **TypeScript** for inspecting connected Android devices, scanning installed applications, reviewing conservative evidence-based package recommendations, and performing reversible package management over **Android Debug Bridge (ADB)** without root access.

- **Repository:** `https://github.com/shaheeralii/PRUNIQ`
- **Target OS:** Windows 10/11 (64-bit)
- **Design:** Offline-first local utility. No accounts, no telemetry, no advertising, no cloud uploads, and no bundled Google Platform-Tools binaries.

## Implemented Functionality

- **ADB Discovery & Explicit Device Targeting:** Detects external `adb.exe` via `PATH` or a user-configured path, verifies `adb version`, distinguishes `authorized`, `unauthorized`, `offline`, `no permissions`, and other states, and validates an explicit opaque device serial against recently discovered authorized devices on every device-scoped command.
- **Device Specs & Read-Only Capability Probing:** Collects device identification and hardware details (`getprop`, `/proc/meminfo`, `/proc/cpuinfo`, `df -k /data`, `wm size`, `wm density`, `dumpsys battery`, `uname -r`) and probes read-only Package Manager and multi-user capabilities (`pm help`, `cmd package help`, `am help`, `am get-current-user`, `pm list users`) without executing mutating commands to test support.
- **Bulk Package Scanner & Targeted Inspection:** Enumerates installed packages using at most five bulk `pm list packages` calls (never looping `dumpsys package` over every app), resolves labels from a small embedded catalog (`data/app-catalog/common-labels.json`) with raw package-ID fallbacks, categorizes origin/partition/updated-system flags, and supports targeted single-package `dumpsys package` inspection.
- **Offline Recommendation Engine:** Evaluates version-controlled JSON rules (`data/package-rules/rules.json`) with deterministic specificity precedence (exact device > manufacturer > Android API range > general Android > `unknown_unverified`) and restrictive conflict resolution. Rules marked `verified` are rejected at load time unless complete evidence, `lastVerifiedDate`, and `reviewer` are present. Ships initially with `0` verified rules; unknown packages are never treated as safe to disable.
- **Guarded Reversible Package Operations:** Supports `disable-user`, `enable`, `default-state` (only when detected as supported), `restore original state`, and `undo last` under per-device Tokio locks (`tokio::sync::Mutex`), immutable core system guards (14 packages, non-exhaustive), live active/sole launcher and keyboard probes with conservative name-based fallbacks, mandatory preview/confirmation dialogs, and post-operation `dumpsys package` verification.
- **SQLite Persistence & Timeout Reconciliation:** Persists random local device UUIDs (never raw ADB serials), immutable Initial Toolkit Baselines, pre-operation snapshots, sanitized operation history (never raw shell strings), and settings in local SQLite (`rusqlite`). Interrupted or timed-out operations are never auto-retried; they can be reconciled via read-only live-state inspection.
- **Read-Only Memory & Storage Diagnostics:** Captures session-scoped (in-memory) RAM snapshots (`/proc/meminfo`, `dumpsys meminfo`, `ps -A` with legacy `ps` fallback) for before/after comparison, inspects `/data` and mounted partitions via `df -k`, and displays actual measured session probe durations without killing processes or promising RAM/storage gains.
- **Privacy-Redacted Diagnostic Export:** On-demand local JSON diagnostic bundle scrubbing active session ADB serials (`[REDACTED_SERIAL]`) and OS user profile directory names (`[REDACTED]`).

## External ADB Requirement

PRUNIQ does **not** bundle or download Google Android SDK Platform-Tools. Install official Platform-Tools separately and add `adb.exe` to `PATH` or configure its path in PRUNIQ. See [docs/installation/windows.md](docs/installation/windows.md).

## Local Development & Verification

Prerequisites on Windows 10/11 (64-bit): Rust (`stable-msvc`), Cargo (`rustfmt`, `clippy`), Node.js, npm, Microsoft C++ Build Tools, WebView2 Runtime, configured icons in `src-tauri/icons/`, and optional external `adb.exe`.

```powershell
# Reproducible frontend install and checks
npm ci
npm run typecheck
npm run test
npm run build

# Rust formatting, strict Clippy, and unit + mock integration tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked

# Run desktop app in development mode
npm run tauri -- dev
```

## Documentation

- [Windows Installation Guide](docs/installation/windows.md)
- [Usage Guide](docs/usage/guide.md)
- [Android Compatibility Policy & Matrix](docs/compatibility/matrix.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Screenshots Capture Guide](docs/screenshots/README.md)
- [Data Governance & CC-BY-4.0 Rules](data/README.md)
- [Contributing](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## Licensing & Attribution

Copyright © 2026 Syed Shaheer Ali.

- **Source code, configuration, build scripts, and general documentation:** [Apache License, Version 2.0](LICENSE) (`Apache-2.0`).
- **Original project-authored package rules and application catalog data (`data/package-rules/`, `data/app-catalog/`):** [Creative Commons Attribution 4.0 International](data/LICENSE) (`CC-BY-4.0`). See [data/README.md](data/README.md).
