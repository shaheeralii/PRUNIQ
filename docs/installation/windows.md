# Windows 10/11 (64-bit) Installation Guide

## System Requirements

- **Operating System:** Windows 10 (64-bit) or Windows 11 (64-bit)
- **WebView2 Runtime:** Microsoft Edge WebView2 Runtime (preinstalled on Windows 11 and most updated Windows 10 systems)
- **External ADB:** Official Google Android SDK Platform-Tools (`adb.exe`), installed separately. PRUNIQ does **not** bundle or automatically download Platform-Tools.

## 1. Install PRUNIQ (Prebuilt Installers)

When published installers are available for a release:
- **NSIS Installer (`*x64-setup.exe`):** Standard Windows setup executable.
- **MSI Installer (`*x64*.msi`):** Standard Windows Installer package.

> **Unsigned Build Notice:** Unsigned open-source builds may trigger a Windows Defender SmartScreen "Unknown publisher" prompt. Only run installers built from verified sources (`https://github.com/shaheeralii/PRUNIQ`).

## 2. Install Official Android SDK Platform-Tools (External ADB)

- **Option A — Official `winget` package:**
  ```powershell
  winget install -e --id Google.PlatformTools
  ```
- **Option B — Official Google ZIP:**
  1. Download Platform-Tools from `https://developer.android.com/tools/releases/platform-tools`.
  2. Extract to a local folder (for example, `C:\Tools\platform-tools`).
  3. Either add that directory to your Windows User `PATH` or set the full path to `adb.exe` inside PRUNIQ (**Devices & ADB** or **Settings**).

Verify ADB in a new PowerShell window:

```powershell
adb version
```

## 3. Building Windows Installers From Source

Build prerequisites on Windows 10/11 (64-bit):
- Rust (`stable-msvc`) and Cargo (`rustfmt`, `clippy`)
- Node.js (v24) and npm
- Microsoft Visual Studio C++ Build Tools ("Desktop development with C++")
- Configured PRUNIQ icon assets present at every path in `src-tauri/tauri.conf.json` (`icons/32x32.png`, `icons/128x128.png`, `icons/icon.ico`)
- **For MSI packaging only:** Windows `VBSCRIPT` optional feature (required by Tauri 2's WiX MSI bundler). Check locally without modifying Windows:
  ```powershell
  Get-WindowsCapability -Online -Name "VBSCRIPT*"
  ```

Reproducible build commands (using the locally installed locked `@tauri-apps/cli`):

```powershell
npm ci

# Debug binary validation (no installer)
npm run tauri -- build --debug --no-bundle -- --locked

# NSIS installer only
npm run tauri -- build --bundles nsis -- --locked

# MSI installer only (requires VBSCRIPT)
npm run tauri -- build --bundles msi -- --locked
```

Outputs are written under `src-tauri/target/release/bundle/nsis/` and `src-tauri/target/release/bundle/msi/`.
