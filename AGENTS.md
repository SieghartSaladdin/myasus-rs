# AGENTS.md: myasus-rs

## Project Overview
`myasus-rs` is a modular, pure-Rust desktop & CLI hardware control utility reverse-engineered from MyASUS (`4.0.82.0`) for **Linux Mint / Ubuntu** (with cross-platform fallback on Windows).

## Architecture Rules
- Keep every source file focused on one domain (single responsibility). `cargo test` rejects files over 250 lines; files already over are frozen and may only shrink (`tests/source_file_size.rs`).
- Separate concerns cleanly across:
  - `src/models/`: pure data types & enums (no I/O, no UI).
  - `src/platform/`: OS & hardware drivers (`sysfs`, `asus-nb-wmi`, `hwmon`, `ACPI platform_profile`, `fwupdmgr`, `apt`, and cross-platform fallback).
  - `src/services/`: live telemetry polling, diagnostics engine, config persistence.
  - `src/ui/`: Rune (GPU UI toolkit) MyASUS dark-theme shell, sidebar, and one module per page in `src/ui/pages/`.
- Privileged Linux writes MUST attempt direct `sysfs` write first (works with udev rules) and fall back safely to `pkexec` without storing credentials.

## Build & Verification Commands
- Native build & tests: `cargo test` (also runs the source guards in `tests/`; each failure message names the sanctioned fix)
- UI verification (needs `cargo build`): `python3 scripts/verify.py run --all`; headless and sandboxed, one feature per entry in `feature-map.json`
- Script tests and the dimming-flow model check: `python3 -m unittest discover -s scripts` and `python3 scripts/model_dimming.py` (see `docs/dimming-flow.md`)
- Linux x86_64 target verification: `cargo check --target x86_64-unknown-linux-gnu`
- Release build (Ubuntu / Linux Mint): `cargo build --release`
- Package `.deb` on Ubuntu / Mint: `./packaging/build-deb.sh`
