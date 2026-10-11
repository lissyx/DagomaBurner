DagomApp is a desktop app to set up Dagoma 3D printers over USB: printer
detection, Z-offset calibration, PID tuning, factory reset and firmware
flashing. It is built with [Tauri](https://tauri.app).

## Build

Install [Rust](https://rustup.rs) and the
[Tauri system dependencies](https://v2.tauri.app/start/prerequisites/), then:

    cargo build --release

The app is `target/release/dagomapp`. To develop, see [HACKING.md](HACKING.md).

## TODO

- graisser les axes,
- test de premiere couche
- m119 pour l'état des endstops
- m114 position que marlin pense être
- m115 l'UUID,...
