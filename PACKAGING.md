# Packaging DagomApp

`cargo tauri build` builds the app and its installers. It needs the setup from
[HACKING.md](HACKING.md), including the Tauri CLI
(`cargo install tauri-cli --version "^2" --locked`).

Each build only makes the packages of its target platform:

| Platform      | Packages                  | Build on                 |
|---------------|---------------------------|--------------------------|
| Linux x86_64  | `.deb`, `.rpm`            | Linux                    |
| Windows x64   | NSIS installer (`.exe`)   | Windows, or Linux        |
| macOS         | `.dmg` holding the `.app` | macOS                    |

They are written to `target/release/bundle/<type>/`, or
`target/<target>/release/bundle/<type>/` with `--target`.

The version comes from `package.json`; keep `src-tauri/Cargo.toml` in sync.

## Linux

    cargo tauri build

makes `deb/DagomApp_<version>_amd64.deb` and
`rpm/DagomApp-<version>-1.x86_64.rpm`, without needing Debian or RPM tools.
Build on the architecture you target. The package is named `dagom-app`:

    sudo apt install ./DagomApp_<version>_amd64.deb
    sudo dnf install ./DagomApp-<version>-1.x86_64.rpm

Serial ports belong to the `dialout` group, which desktop users are usually
not in. So the packages install a udev rule,
`/usr/lib/udev/rules.d/70-dagomapp.rules` (from `src-tauri/linux/`), that gives
the logged-in user access to the printers' FTDI and Silicon Labs USB serial
adapters, and reload udev on install and removal.

## Windows

The installer installs DagomApp for the current user, without administrator
rights, in English or French depending on the Windows language. If the
Microsoft Edge WebView2 Runtime is missing (it comes with Windows 11 and up to
date Windows 10), the installer downloads and runs Microsoft's installer for
it, which needs Internet access.

### On Windows

1. Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#windows):
   the Microsoft C++ Build Tools ("Desktop development with C++") and Rust
   with its default MSVC toolchain.
2. Build:

       cargo tauri build

The installer is `target\release\bundle\nsis\DagomApp_<version>_x64-setup.exe`.
Tauri downloads NSIS itself.

### From Linux

1. Set up MinGW-w64 as described in [CROSS_COMPILING.md](CROSS_COMPILING.md).
2. Install NSIS, which provides `makensis`:

   - Debian/Ubuntu: `sudo apt install nsis`
   - Fedora: `sudo dnf install mingw32-nsis`

3. Build:

       cargo tauri build --target x86_64-pc-windows-gnu

The installer is
`target/x86_64-pc-windows-gnu/release/bundle/nsis/DagomApp_<version>_x64-setup.exe`;
it includes the `WebView2Loader.dll` that MinGW builds need. Tauri downloads
an NSIS plugin (`nsis_tauri_utils.dll`) from GitHub while building.

### Signing

Unsigned installers trigger a Windows SmartScreen warning. To sign them, see
[Tauri's Windows code signing guide](https://v2.tauri.app/distribute/sign/windows/):
on Windows with a certificate in `bundle.windows`, or from Linux with a custom
`bundle.windows.signCommand` (e.g. using `osslsigncode`).

## macOS

Tauri only makes `.app` bundles and `.dmg` images on macOS: build them on a Mac
(or a macOS CI runner).

1. Install the Xcode Command Line Tools (`xcode-select --install`) and Rust.
2. To support both Intel and Apple Silicon Macs with a universal app, add both
   Rust targets:

       rustup target add x86_64-apple-darwin aarch64-apple-darwin

3. Build:

       cargo tauri build --target universal-apple-darwin

   or `cargo tauri build` for the Mac's own architecture only.

The image is
`target/universal-apple-darwin/release/bundle/dmg/DagomApp_<version>_universal.dmg`.

### Signing and notarization

macOS blocks apps that aren't signed and notarized by a registered Apple
developer. To distribute DagomApp, sign it with a Developer ID certificate and
notarize it, by setting these variables when building (see
[Tauri's macOS code signing guide](https://v2.tauri.app/distribute/sign/macos/)):

- `APPLE_SIGNING_IDENTITY`: the certificate name, e.g.
  `Developer ID Application: Dagoma (TEAMID)`, from the keychain.
- `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID`,
  for notarization.

For testing, `APPLE_SIGNING_IDENTITY=-` signs ad-hoc: the app then runs on
Apple Silicon Macs, but users have to allow it in System Settings, Privacy &
Security, the first time.
