# AGENTS.md — myasus-rs

## Project Overview
`myasus-rs` is a modular, pure-Rust desktop & CLI hardware control utility reverse-engineered from MyASUS (`4.0.82.0`) for **Linux Mint / Ubuntu** (with cross-platform fallback on Windows).

## Architecture Rules
- Follow **Single Responsibility Principle (SRP)**: keep every source file focused on one domain and under ~250 lines.
- Separate concerns cleanly across:
  - `src/models/` — pure data types & enums (no I/O, no UI).
  - `src/platform/` — OS & hardware drivers (`sysfs`, `asus-nb-wmi`, `hwmon`, `ACPI platform_profile`, `fwupdmgr`, `apt`, and cross-platform fallback).
  - `src/services/` — live telemetry polling, diagnostics engine, config persistence.
  - `src/ui/` — `eframe`/`egui` MyASUS dark-theme shell, sidebar, quick settings, and modular pages.
- Privileged Linux writes MUST attempt direct `sysfs` write first (works with udev rules) and fall back safely to `pkexec` without storing credentials.

## Build & Verification Commands
- Native build & tests: `cargo test`
- Linux x86_64 target verification: `cargo check --target x86_64-unknown-linux-gnu`
- Release build (Ubuntu / Linux Mint): `cargo build --release`
- Package `.deb` on Ubuntu / Mint: `./packaging/build-deb.sh`
