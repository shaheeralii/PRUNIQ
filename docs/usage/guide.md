# PRUNIQ Usage Guide

## 1. Enable Wired USB Debugging

1. On Android, open **Settings -> About phone** and tap **Build number** seven times to enable **Developer options**.
2. Open **Settings -> Developer options** and enable **USB debugging**.
3. Connect the phone to your Windows PC over USB, unlock the screen, and tap **Allow** on the RSA debugging authorization dialog.

> Wireless ADB debugging is not implemented; use a wired USB cable.

## 2. Connect & Select Your Target Device

1. Open **Devices & ADB**. PRUNIQ checks `PATH` (or your saved custom `adb.exe` path) and verifies `adb version`.
2. Click **Refresh Devices** to run `adb devices -l` and inspect each device state (`device`, `unauthorized`, `offline`, `no permissions`).
3. Click **Select Device** on an authorized (`device`) entry. Every device-scoped backend command explicitly validates and targets that serial.

## 3. Inspect Device Specs, Capabilities & User Scope

Open **Device Specs** to review hardware specs, the read-only capability matrix (`pm help`, `cmd package help`, `am help`), and detected Android user profiles (`am get-current-user`, `pm list users`).

- Select an explicit Android User ID (for example, `User 0`) in **Target User Scope** before modifying packages. PRUNIQ never silently assumes User 0 if detection fails.

## 4. Scan Applications & Review Recommendations

1. Open **Applications** and click **Scan Packages**. PRUNIQ runs up to five bulk `pm list packages` calls (never per-app `dumpsys` loops).
2. Search by label or package ID, and filter by risk, origin, effective state, partition, or updated-system flag.
3. Select a row to open **Package Inspector**. Click **Inspect dumpsys** to query live version, `codePath`, and per-user configured state (`0` DEFAULT through `4` DISABLED_UNTIL_USED).
4. Review **Recommendation & Evidence**. Unknown packages default to `Unknown / Unverified` and are never treated as safe to disable.

## 5. Guarded Package Operations & Restoration

1. Choose **Disable**, **Enable**, **Restore Original**, **Undo Last**, or **Restore Default** (shown only when `pm default-state` was detected as supported) for a single app or checked selection.
2. Inspect the **Review & Confirm Package Operations** dialog:
   - Immutable core components (14 guarded packages) and the active or sole detected launcher/keyboard are blocked from Disable automatically.
   - Verify the target serial and explicit Android User ID before clicking **Confirm & Execute**.
3. Before each mutation, PRUNIQ saves an immutable **Initial Toolkit Baseline** (once per device/user/package) and a **Pre-Operation** snapshot to local SQLite, then verifies live post-operation state via `dumpsys package` (with `pm list packages -d` fallback).
4. Review restoration fidelity honestly: PRUNIQ distinguishes `exact` restoration from `effective_match_only` when rootless ADB cannot reproduce an exact configured code (such as `DISABLED (2)` or `DEFAULT (0)` without `pm default-state`).

## 6. Timeout & Disconnect Recovery

PRUNIQ never auto-retries timed-out or disconnected mutations:

1. Open **History & Undo**. Interrupted entries are marked `uncertain_timeout`.
2. Reconnect and select the device, then click **Verify Live State** on the interrupted entry.
3. PRUNIQ runs a strictly read-only live check under the per-device lock and updates SQLite history to `succeeded` or `verification_mismatch`.

## 7. Read-Only Memory & Storage Diagnostics

Open **Memory & Storage** to:
- Capture session-scoped (in-memory) read-only RAM snapshots (`/proc/meminfo`, `dumpsys meminfo`, `ps -A` with legacy `ps` fallback) without killing or force-stopping processes, and compare any two snapshots. Memory fluctuates naturally and disabling apps does not guarantee higher free RAM.
- Inspect `/data` and mounted partitions via `df -k`. Disabling preinstalled system apps does **not** remove factory APKs from read-only `/system`, `/product`, or `/vendor` partitions.

## 8. Settings & Redacted Diagnostics

Open **Settings** to save a custom `adb.exe` path, toggle optional auto-scan on device select, view offline rule database counts, or generate and download a local privacy-redacted JSON diagnostic bundle.
