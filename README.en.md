# AppFlash

**loyran works.** · Make Ideas Work Beautifully

**Your apps, in a flash.**

English · [简体中文](README.md)

A focused Windows / macOS application switcher. Hold your chosen prefix and press one letter to launch, restore, cycle through or minimize an app. No search box, no unrelated features.

> In development: Windows builds are available locally; macOS still needs on-device verification. The same loyranworks/AppFlash repository is public. Signed updates are being tested through official Releases; this is not yet a stable release.

## Get started

1. Open AppFlash. No application bindings are preconfigured.
2. Select **Applications** and choose a prefix; the recommended default is **Alt**. Multiple checked modifiers must be held together.
3. Open **Applications**, import or add apps, and assign unique letters A–Z.
4. Click **Save & apply**, close Settings, and press your shortcut. For Chrome assigned C, use **Alt + C**.

Closing Settings keeps AppFlash running. The Windows tray / macOS menu bar offers Settings, pause / resume and Quit. On macOS, opening AppFlash again from Finder restores Settings, including a minimized window. Pausing releases registered shortcuts. Switching is suspended while Settings is focused to prevent accidental activation.

## Batch import

Import discovers applications **that currently have windows**, not every installed program.

1. Open the apps you want to add.
2. Click **Batch import** in Applications. Review each icon, name and full path.
3. Filter by name or path, select entries and enter letters. Entering a letter selects its entry automatically.
4. Click **Add to list**. This only updates your unsaved list.
5. Review it and click **Save & apply** once. Bindings update immediately; **no restart is required**.

Existing paths are skipped and cannot be selected again. Same names with different paths remain separate entries. Duplicate letters prevent importing. Use **Add** for undiscovered apps: enter the name and full exe path on Windows or a bundle path such as /Applications/App.app on macOS. Target-app launch arguments are not supported.

## Shortcuts and application management

Prefix supports Control, Alt, Shift and Windows (Command on macOS), individually or together. Select at least one. Letters are case-insensitive and unique. Leave a letter blank to keep an entry disabled. Up to 26 apps can be bound; up to 500 entries can be stored in the scrollable list.

Click an app or its circular letter badge to edit. Adding, deleting and reordering take effect after saving. Registration failures attempt to restore old bindings. Failed rollback explicitly asks you to quit and reopen AppFlash.

**Shortcut conflicts.** Alt is the default. If a binding overlaps a shortcut you use in another app or the system, change its letter or prefix. Exit old AHK scripts using the same bindings before testing.

## Single and repeated presses

| Application state | Behavior |
| --- | --- |
| No running windows | Launch the application |
| Background or minimized | Activate / restore its window |
| One foreground window | Press again to minimize; press once more to restore |
| Multiple windows | Cycle in stable order, then minimize all target windows |

Repeated presses mean pressing and releasing the letter, not holding it for keyboard auto-repeat. Switching apps or closing target windows updates the cycle state. Windows matches full process paths. Launchers, UWP hosts, elevated apps and macOS Accessibility restrictions still require real-device testing.

## Edge hint

Hidden normally. Hold the prefix for **500ms (0.5 seconds) by default** to reveal circular letters and real app icons. Release the prefix to hide. Fast switching never waits for the hint. Lines, slow flowing animation and target highlighting provide lightweight feedback without app-name labels.

Choose enable / disable, delay (0–10 seconds), left / right edge, letter background color, and alphabetical or manual order. Set manual order with ↑ ↓ in Applications and save. Alphabetical display does not overwrite manual order. Native frosted glass falls back to translucency when unavailable. Large lists use compact sizing; multi-monitor and DPI behavior need validation.

## General settings

The app mark uses the original lightning cutout in a rounded square. Its Settings preview follows the selected letter background color; saving updates the tray and window icon. Pausing switches the runtime mark to gray (#343840); resuming restores the saved color. Color remains editable when the edge hint is disabled. Installer, executable-file and macOS Dock icons use the static default color.

- **Interface language:** Simplified Chinese or English. Changes immediately update and save Settings, dialogs, status text and tray menus without discarding other edits. App names, paths, original release notes and third-party system errors are not forcibly translated.
- **Launch at login:** Off by default. Changes take effect immediately without Save. Starts quietly with Settings closed. Use a fixed installation path; moving a development exe requires reconfiguring the startup entry.
- **Updates:** Checked about three seconds after launch and every six hours, or manually. Installation requires confirmation and verifies the signature and signed version. Unsaved edits block updating. Saved settings are backed up to config.pre-update.json; failed backup or verification cancels installation.

**Private GitHub releases cannot be accessed anonymously by ordinary clients.** No GitHub token is embedded. Online updates will be validated after this same repository becomes public and a production Release exists. Normal code pushes do not release a version; version tags trigger builds. See [release instructions](docs/releases.md).

## Configuration and privacy

Find the actual path under **General → Configuration location**. Typical paths: %APPDATA%/local.appflash.desktop/config.json on Windows and ~/Library/Application Support/local.appflash.desktop/config.json on macOS.

Settings use a temporary file and atomic replacement. Unreadable configuration is not overwritten: back it up and repair it first. Older configurations default to Chinese; legacy launch arguments are ignored and removed on the next save. Original AHK / INI files are neither migrated nor modified. Configuration contains local names and paths; review it before sharing.

## Development

Install Node.js / npm, stable Rust and platform build tools. Windows requires MSVC C++ Build Tools and WebView2; macOS requires Xcode Command Line Tools, with macOS 12 as the configured minimum.

    npm install
    npm run dev
    npm test
    npm run check
    npm run build

Windows installers are produced in src-tauri/target/release/bundle/nsis/. Signed updater packages require the maintainer's signing key; consult the release instructions. Updater signatures are not OS code signing. macOS builds use an ad-hoc signature covering the complete app bundle and still require manual approval on first launch. Windows code signing and macOS Developer ID signing / notarization remain outstanding, so security warnings may appear.

ui/ contains framework-free Settings and hint UI. src-tauri/src/ contains configuration, transactional registration, window cycling and updates. Its platform/ implements Win32 and the native macOS Swift helper. design/ retains visual iterations.

## Open source and contributing

The same repository is public, retaining its existing MIT License. This project has its own Git repository, independent of its parent. Exclude private configuration, paths, credentials and signing keys from code and history before publication.

Use a lightweight GitHub Flow: main is the integration branch; short-lived feat/name and fix/name branches are reviewed and verified through pull requests, then merged and deleted. No long-lived develop branch is needed yet. Release tags use v followed by the version number and must point to a verified commit.

For bug reports, include your OS, keyboard layout, target app and reproduction steps, but no private configuration. Run tests before contributing. Passing tests do not establish macOS compatibility, full real-key switching, multi-monitor behavior, cross-version updating or performance parity with AHK.
