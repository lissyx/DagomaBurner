# Hacking on DagomApp

## Setup

You need:

- [Rust](https://rustup.rs).
- The [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/).
  On Debian based distributions:

      sudo apt install libwebkit2gtk-4.1-dev libudev-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev

Then install the Tauri CLI:

    cargo install tauri-cli --version "^2" --locked

You don't need to install Node.js or run `npm install`. The frontend is built
with Node.js and npm packages kept in `target/`, set up on first use:

- `target/node/`: Node.js (the version pinned in `tools/frontend/src/lib.rs`),
  downloaded from nodejs.org and checked against its SHA-256. This needs
  network access once.
- `target/frontend/`: the npm dependencies, installed from
  `package-lock.json`, and the built frontend.

To use your own Node.js instead (offline builds, distribution packaging), set
`DAGOMAPP_NODE` to its `node` binary; it must be 20.19, or 22.12 and later.

## Run a local dev build

    cargo tauri dev

This starts the Vite dev server on http://localhost:1420 and opens the app on
it. Changes to the frontend (`src/`) are picked up live; changes to the Rust
code (`src-tauri/`) rebuild and restart the app.

- `NODE_ENV=debug cargo tauri dev` opens a fullscreen, resizable window with
  the devtools. The devtools are also in the context menu of debug builds.
- A plain `cargo build` (or `cargo run`) gives a standalone debug app with the
  frontend embedded, without the dev server.

## Test

    cargo test

runs the Rust unit tests: Intel HEX parsing, and the STK500 bootloader
protocols against simulated bootloaders.

## Package

    cargo tauri build

builds the installers for the current OS into `target/release/bundle/`. To
build the Windows and macOS apps from Linux, see
[CROSS_COMPILING.md](CROSS_COMPILING.md).

## Layout

- `src/`: the frontend (Vite root).
  - `index.html`: the app window.
  - `controllers/`, `manager/`, `i18n/`: the app logic; `views/`: the
    Handlebars page templates.
  - `lib/`: browser replacements for the Node/Electron APIs the app used
    (events, serial port, application menu).
  - `public/static/`: CSS, images and vendored libraries, served as-is.
  - `config.json`: the supported printers.
- `src-tauri/`: the Rust backend.
  - `build.rs`: builds the frontend for non-dev builds.
  - `serial.rs`: serial port listing and I/O.
  - `flasher/`: firmware upload (STK500v1 for Melzi, STK500v2 for Mega 2560).
  - `windows.rs`: the window, ported from the Electron main process.
- `tools/frontend/`: installs Node.js into `target/node/` and runs
  `scripts/frontend.mjs` with it, for `build.rs` and `cargo tauri dev`.
- `scripts/frontend.mjs`: installs the npm dependencies into
  `target/frontend/` and runs Vite.
- `utils/`: firmware files and icons; `ressources/`: design sources.
