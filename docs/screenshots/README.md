# Screenshots Capture Guide

No fabricated or placeholder screenshots are included in this repository. Capture real screenshots locally from the running Windows desktop application (`npm run tauri -- dev` or a built Windows release binary) before publishing a public release.

## Recommended Capture Set

Save PNG files in `docs/screenshots/` using consistent names:

1. `01-devices-adb.png` — **Devices & ADB** view with external ADB detected and an authorized device selected.
2. `02-device-specs.png` — **Device Specs** view showing hardware specs, RAM/storage bars, and capability matrix.
3. `03-applications-inspector.png` — **Applications** master-detail view with filters, risk/protection badges, and docked Package Inspector.
4. `04-operation-preview-dialog.png` — **Review & Confirm Package Operations** dialog showing target User ID and protection guards.
5. `05-memory-storage-diagnostics.png` — **Memory & Storage** view showing before/after RAM comparison and `/data` partition usage.
6. `06-history-recovery.png` — **History & Undo** view showing verified state-transition chips.

## Privacy Checklist Before Committing Screenshots

- Mask or crop real hardware ADB serial numbers if you do not want your device serial public.
- Confirm no personal Windows username appears in custom ADB paths.
- Capture authentic device output only (never altered or fabricated metrics).
