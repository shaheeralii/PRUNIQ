# Changelog

All notable changes to **PRUNIQ — Android Debloating Toolkit** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Current synchronized development version: `0.1.0` (unreleased).

### Added
- **ADB Foundation (Phase 1):** External ADB discovery (`PATH` + custom path), `adb version` parsing, timeout-enforced async `TokioProcessRunner` + `MockProcessRunner`, opaque serial validation, multi-device state parsing, and structured serializable errors.
- **Device Info & Bulk Package Scanner (Phase 2):** Full read-only device identification and hardware specs, read-only capability and multi-user profile probing, bulk `pm list packages` scanning, embedded offline app-label catalog with raw package-ID fallback, multi-signal categorization, and targeted single-package `dumpsys package` inspection.
- **Offline Recommendation Engine (Phase 3):** Version-controlled JSON rule schema (`data/package-rules/schema.json`), evidence-enforcing loader, deterministic specificity precedence with restrictive conflict resolution, and conservative `unknown_unverified` fallback (`0` initial verified rules).
- **Guarded Package Management & Recovery (Phase 4):** Separated reversible mutating command allow-list (`pm disable-user`, `pm enable`, `pm default-state`, `pm disable-until-used`), per-device Tokio mutex locks, immutable + live launcher/keyboard protections with conservative fallbacks, immutable Initial Toolkit Baseline and Pre-Operation snapshots, preview/confirmation dialog, post-operation verification, and honest restoration-fidelity reporting.
- **SQLite Persistence, Recovery & Diagnostics (Phase 5):** Local SQLite storage (`rusqlite` bundled) with migration `0001_initial.sql`, random local device UUIDs, persistent settings and sanitized operation history, read-only live-state reconciliation for timed-out/disconnected operations, and on-demand privacy-redacted JSON diagnostic export.
- **Read-Only Memory & Storage Diagnostics (Phase 6):** Session-scoped RAM snapshots (`/proc/meminfo`, `dumpsys meminfo`, `ps -A` with legacy `ps` fallback), before/after signed delta comparison, multi-partition storage dashboard (`df -k`), and measured session probe durations.
- **Desktop GUI (Phase 7):** Obsidian Emerald dark workstation layout using CSS Modules and CSS variables with fixed sidebar navigation, pinned Active Target card, master-detail package inspector, and non-color-only status badges.
- **Release Preparation (Phase 8):** Dual-license governance (`Apache-2.0` code + `CC-BY-4.0` original dataset material), PRUNIQ Obsidian Emerald icon assets, Vitest 3 frontend tests, Windows NSIS/MSI Tauri bundle configuration, GitHub Actions Windows CI/CD with independently diagnosable NSIS and MSI packaging jobs, and public documentation.

### Known Scope & Limitations
- Windows 10/11 (64-bit) is the initial target OS; Linux and macOS distribution are deferred.
- External official Android SDK Platform-Tools (`adb.exe`) is required and not bundled.
- Offline rule database starts with `0` verified rules pending independently verified hardware evidence.
- Physical hardware verification across diverse OEM skins and legacy API 19—20 devices remains incomplete and explicitly documented as untested.
