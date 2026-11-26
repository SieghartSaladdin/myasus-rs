# myasus-rs — Complete MyASUS Port in Pure Rust (Windows & Linux Mint / Ubuntu x86_64)

Open-source, modular pure-Rust desktop & CLI hardware control utility reverse-engineered from **MyASUS (`4.0.82.0`)** using **REA (Reverse Engineer Anything)**. Runs natively with **real hardware control** on both **Windows x86_64** (`\\.\ATKACPI` + Win32/WMI) and **Linux x86_64 — Ubuntu 22.04/24.04 & Linux Mint 21/22** (`asus-nb-wmi`, `hwmon`, ACPI `platform_profile`, `fwupd`, `apt`).

See [docs/REA_REVERSE_ENGINEERING.md](docs/REA_REVERSE_ENGINEERING.md) for the complete static analysis ledger with REA Evidence IDs across all 14 functional modules.

## Complete Feature Set (9 GUI Pages + 10 CLI Commands)
1. **Home Dashboard**: Real-time CPU Load + Temperature with live sparkline graph, Memory, per-mount Hard Disk usage warnings, real CPU/GPU Fan RPM, Device Model & Serial Copy, and **Quick Setting** drawer.
2. **Device Settings (4 Tabs)**:
   - *Power & Performance*: Battery Health Charging limit (`60% / 80% / 100% / 40–100%`), Fan Profile (`Whisper / Standard / Performance`), CPU Scaling Governor / Power Plan.
   - *Audio & Visual*: Splendid & Eye Care hardware gamma LUT (`SetDeviceGammaRamp` on Windows, `xrandr --gamma` on Linux), Display Brightness, Panel Overdrive, and Connected Monitors list.
   - *Connectivity*: Active Wi-Fi SSID, Signal strength (`dBm`), Local IP, Router Gateway IP, and live RX/TX throughput (`KB/s`).
   - *Input & GPU*: Keyboard Backlight (`0–3`), Fn Lock, Precision Touchpad, Discrete GPU Eco Disable (`dgpu_disable`).
3. **System Diagnosis (2 Tabs)**:
   - *Automated 6-Subsystem Checkup*: Battery & Power, Thermal & Fans, RAM buffer integrity, Storage I/O & capacity, Wi-Fi/Bluetooth/DNS, and ASUS Driver (`ATKACPI` / `asus-nb-wmi`).
   - *Interactive Self-Test & Processes*: Dead-Pixel / Display uniformity tester, live Keyboard keypress verifier, Touchpad click/scroll tester, and Top Resource Processes monitor.
4. **System Update**: Checks **LVFS UEFI/BIOS firmware** (`fwupdmgr`) + **APT packages** on Ubuntu/Linux Mint, and `winget upgrade` on Windows.
5. **My ASUS Devices**: Scans USB, HID, Bluetooth, Audio & Display hardware with automatic **ASUS (`VID_0B05`)** highlighting.
6. **Message Center**: Live event feed for high-temperature alerts, low disk warnings, diagnostic runs, and applied hardware settings with unread counter.
7. **Customer Support**: One-click JSON Diagnostic Support Bundle export + system summary copy + Windows/Linux troubleshooting guide.
8. **System Info** & **Settings**: Complete DMI/BIOS/Hardware Capability Matrix, temperature unit (`°C / °F`), polling interval, and Linux udev helper status.

## Quick Run — Windows x86_64
```powershell
cargo run                     # Launch Native GUI (real \\.\ATKACPI + Win32 sensors)
cargo build --release         # Build standalone target\release\myasus-rs.exe
```

## Quick Install — Linux Mint / Ubuntu x86_64
```bash
chmod +x packaging/install-ubuntu-mint.sh
./packaging/install-ubuntu-mint.sh
```
Or build a native `.deb` package:
```bash
chmod +x packaging/build-deb.sh
./packaging/build-deb.sh
sudo apt install ./target/deb/myasus-rs_0.1.0_amd64.deb
```

## CLI Commands (Windows & Linux)
```bash
myasus-rs                              # Launch GUI
myasus-rs status [--json]              # Live CPU, Temp, RAM, Fan RPM, Device Info
myasus-rs diagnose [--json]            # 6-category hardware diagnosis
myasus-rs devices [--json]             # Connected USB, BT, Displays & ASUS devices
myasus-rs battery-limit 80             # Set charging threshold (40-100)
myasus-rs fan-mode performance         # whisper | standard | performance
myasus-rs kbd-light 2                  # Keyboard backlight (0-3)
myasus-rs fn-lock on                   # on | off
myasus-rs splendid eyecare --level 3   # normal | vivid | eyecare | manual
myasus-rs updates [--json]             # Firmware & OS/package update check
myasus-rs export-report                # Export full JSON diagnostic bundle
```
