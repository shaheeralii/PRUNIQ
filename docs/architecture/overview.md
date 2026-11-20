# Architecture Overview

PRUNIQ is a local-first modular monolithic desktop application built with **Tauri 2**, **Rust**, **React**, **TypeScript**, and **SQLite (`rusqlite`)**.

## Layer 1 — Presentation (`src/`)

- **Application Shell (`src/App.tsx`, `src/styles/`):** Obsidian Emerald dark desktop layout built strictly with CSS Modules and CSS variables (no UI component framework).
- **Feature Views (`src/features/`):**
  - `devices/AdbDevicePanel.tsx` — external ADB detection and explicit device selection
  - `device-info/DeviceInfoPanel.tsx` — specs, RAM/storage usage bars, capability matrix, explicit User ID selector
  - `app-manager/AppManagerPanel.tsx` — bulk package inventory, filters, docked Package Inspector, guarded single/bulk controls
  - `diagnostics/DiagnosticsPanel.tsx` — read-only RAM snapshots, before/after comparison, storage partitions, measured session durations
  - `history/HistoryPanel.tsx` — persisted operation history, uncertain-operation reconciliation, undo/restore actions
  - `settings/SettingsPanel.tsx` — persisted local settings, rule DB summary, on-demand redacted diagnostic export
- **Typed IPC Wrappers (`src/lib/tauri/`, `src/types/`):** Explicit `invoke()` wrappers (`adb.ts`, `device.ts`, `packages.ts`, `rules.ts`, `operations.ts`, `history.ts`, `diagnostics.ts`).

## Layer 2 — Tauri IPC Commands (`src-tauri/src/commands/`)

Thin command handlers (`adb.rs`, `device.rs`, `packages.rs`, `rules.rs`, `operations.rs`, `history.rs`, `diagnostics.rs`) validate inputs, resolve ADB and device context, invoke Rust domain modules, and serialize `AdtError` payloads with user guidance.

## Layer 3 — Rust Backend Modules (`src-tauri/src/`)

| Module | Implemented Responsibilities |
|---|---|
| `core/` | Domain structs (`models.rs`, `device_models.rs`, `package_models.rs`, `diagnostics_models.rs`) and serializable `AdtError` (`errors.rs`). |
| `adb/` | External ADB path lookup & version parsing (`detection.rs`), hand-written serial/package validation (`validation.rs`), closed `AdbReadOnlyCommand` & `AdbMutatingCommand` allow-lists (`command_specs.rs`), `TokioProcessRunner` + `MockProcessRunner` (`process_runner.rs`), `adb devices -l` parser (`device_discovery.rs`), and `AdbService` (`service.rs`). |
| `android/` | Basic/full specs (`device_info.rs`, `device_specs.rs`), read-only capability probing (`capabilities.rs`), RAM/storage collectors & delta comparator (`diagnostics.rs`), offline app-label catalog lookup (`app_metadata.rs`), and output parsers (`parsers/`). |
| `packages/` | Bulk `pm list packages` scanner & targeted `dumpsys package` inspector (`scanner.rs`), multi-signal categorization (`categorization.rs`), and filtering (`inventory.rs`). |
| `rules/` | Rule/evidence models (`models.rs`), schema & evidence-enforcing loader (`loader.rs`), and deterministic precedence/conflict engine (`engine.rs`). |
| `operations/` | Per-device `tokio::sync::Mutex` lock manager (`locks.rs`), immutable + live launcher/keyboard protections (`protections.rs`), preview planner (`planner.rs`), verified executor (`executor.rs`), ID/timestamp helpers (`snapshots.rs`), and read-only timeout/disconnect reconciliation (`recovery.rs`). |
| `storage/` | Local SQLite persistence (`db.rs`, `models.rs`) with WAL mode, foreign keys, and versioned migration (`src-tauri/migrations/0001_initial.sql`). |
| `diagnostics/` | Serial and OS user-path redactor (`redaction.rs`) and local JSON diagnostic bundle builder (`export.rs`). |

## Persistence & In-Memory State Boundaries

- **Persisted in SQLite (`%APPDATA%\org.adt.android-debloating-toolkit\adt.db`):** `schema_migrations`, `devices` (random UUID v4, no raw serials), `package_cache`, `state_snapshots` (partial unique index enforces immutable `initial_baseline` alongside `pre_operation` rows), `operation_history` (structured actions and redacted error strings, never raw shell commands), and `settings`.
- **In-Memory Session Only (`SessionCache`):** Active ADB serials, discovered device list, serial-to-UUID map, session memory snapshots (capped at 20 per serial), and measured session latency metrics.
