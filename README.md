# UnSleep

A lightweight, minimalist cross-platform desktop anti-sleep app.

- Desktop: Uses Tauri to invoke system-level anti-sleep capabilities, which continue to work even when you switch to other apps.
- Browser preview: For UI debugging only, not released as a PWA.

## Prerequisites

You need to install:

- Node.js
- Rust / Cargo
- Tauri system dependencies for your platform

Install Rust on macOS:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

If Xcode Command Line Tools are not installed on macOS:

```bash
xcode-select --install
```

Install project dependencies:

```bash
npm install
```

## Development

```bash
npm run dev
```

This launches the Tauri desktop app. Click "Keep Awake" to enable system-level anti-sleep.

## Build for Current Platform

By default, Tauri builds for the current system platform. That means:

- Run on macOS → builds a macOS app
- Run on Windows → builds a Windows app
- Run on Linux → builds a Linux app

General build command:

```bash
npm run build
```

Equivalent to:

```bash
npx tauri build
```

## macOS Build

Run on macOS:

```bash
npm run build
```

To support both Apple Silicon and Intel Macs, first install the Intel Rust target:

```bash
rustup target add x86_64-apple-darwin
```

Then build the Universal macOS app:

```bash
npm run build:mac:universal
```

Or build separately:

```bash
npm run build:mac:apple
npm run build:mac:intel
```

Common output locations:

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

Usually generates `.app` and `.dmg`.

## Windows Build

Run on Windows:

```powershell
npm install
npm run build
```

Common output locations:

```text
src-tauri\target\release\bundle\nsis\
src-tauri\target\release\bundle\msi\
```

Usually generates `.exe` installers, depending on Tauri bundler configuration and local environment.

## Linux Build

Run on Linux:

```bash
npm install
npm run build
```

Common output locations:

```text
src-tauri/target/release/bundle/appimage/
src-tauri/target/release/bundle/deb/
src-tauri/target/release/bundle/rpm/
```

Usually generates `.AppImage`, `.deb`, or `.rpm`, depending on system dependencies and Tauri configuration.

## Cross-Platform Packaging Notes

Tauri does not recommend compiling all platform installers on a single machine. The most stable approach is:

- macOS package: build on macOS
- Windows package: build on Windows
- Linux package: build on Linux

For automated releases, use GitHub Actions on `macos-latest`, `windows-latest`, and `ubuntu-latest`:

```bash
npm install
npm run build
```

## Browser Preview

To preview the UI only:

```bash
npm run preview
```

Then open:

```text
http://localhost:5173
```

Browser preview is for UI debugging only. The actual anti-sleep capability is provided by the Tauri client.

## Anti-Sleep Implementation

- macOS: calls `caffeinate`
- Windows: calls `SetThreadExecutionState`
- Linux: calls `systemd-inhibit`
- Browser preview: falls back to `Screen Wake Lock API` for debugging

## Menu Bar / Tray

The desktop client docks in the system menu bar or tray.

The app starts hidden: no window is opened on launch, and only the menu bar / tray icon stays visible. On macOS it runs as a pure menu-bar app (no Dock icon or Cmd+Tab entry); on Windows and Linux the window is simply hidden on startup.

Clicking the window close button (X) only hides the window — it does not quit the app. The only way to fully quit is the "Quit" item in the menu bar / tray icon menu.

Menu items:

- Show Window
- Keep Awake / Allow Sleep
- Quit

On macOS it appears in the top menu bar; on Windows and Linux it appears in the system tray. iOS does not have a system tray concept, so this entry is not supported.

## iOS Build

iOS commands can only be run on macOS, and require Xcode, the iOS target, and Apple signing environment.

Initialize the iOS project for the first time:

```bash
npm run tauri ios init
```

Or use the project script:

```bash
npm run ios:init
```

Develop on simulator or device:

```bash
npm run tauri ios dev
```

Or:

```bash
npm run ios:dev
```

Build iOS release:

```bash
npm run tauri ios build
```

Or:

```bash
npm run ios:build
```

Open the Xcode project for archiving, signing, and release:

```bash
npm run tauri ios build -- --open
```

Or:

```bash
npm run ios:open
```

App Store Connect export:

```bash
npm run tauri ios build -- --export-method app-store-connect
```

Generated IPA is usually at:

```text
src-tauri/gen/apple/build/arm64/UnSleep.ipa
```

Note: The current iOS version can be compiled as a client framework, but the native anti-sleep implementation still needs additional integration with iOS's `idleTimerDisabled` capability.

## License

MIT