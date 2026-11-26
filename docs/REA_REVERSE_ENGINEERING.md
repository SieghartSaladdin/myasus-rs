# Complete Reverse Engineering Report: MyASUS (`4.0.82.0`) -> `myasus-rs` (Windows & Linux x86_64)

This document records static reverse-engineering evidence collected using **REA (Reverse Engineer Anything)** across all functional modules of **MyASUS `4.0.82.0`** (`B9ECED6F.ASUSPCAssistant_4.0.82.0_x64__qmba6cd70vzyy`), and maps every observed subsystem to native **Windows (`\\.\ATKACPI` / Win32 / WMI)** and **Linux x86_64 (Ubuntu / Linux Mint `asus-nb-wmi` / `hwmon` / ACPI / `fwupd` / `apt`)** implementations in `myasus-rs`.

> **Method & Boundaries**: Static artifact analysis only (`inspect_desktop_application`, `inspect_managed_artifact`, `inspect_managed_members`, `inspect_managed_native_boundaries`, `inspect_native_exports`, `inspect_pe_imports`). Commercial ad/trial modules (`McAfee`, `AppDeals`, `PromotionDialog`, `News`, `Preload`, `AsusExclusive`) were intentionally excluded from the clean-room Rust port.

---

## 1. Complete REA Evidence Ledger (All Analyzed MyASUS Modules)

| Module / Artifact | Runtime / Key Observations | SHA-256 (prefix) | REA Evidence ID |
|---|---|---|---|
| `B9ECED6F.ASUSPCAssistant_4.0.82.0_x64__qmba6cd70vzyy` | MSIX directory: 3,137 files, 478 managed PE/CLI assemblies | `0b11f128...` | `ev_51566ef470395e93a1d10dc28c0275da16953e28e2998c61651da9124d1e46dc` |
| `MyASUS\AsusMyASUS.exe` | `.NETFramework v4.8` WPF Shell (216 types, 1,026 methods) | `17a8510b...` | `ev_ba0795257e13063bc57ff7ba16ad8a96a648cfc9beb89165ee20d78e8e57d7fa` |
| `MyASUS\ASUSPluginInterface.dll` | `.NETFramework v4.7.1` Plugin SDK (113 types) | `3f08c4c1...` | `ev_9c7ffaccde3b466c444e24c396d40361811a4ab2a61e481648c09722339c7c39` |
| `ModuleDll\Home\AsusHome.dll` | Home dashboard: HID, WLAN, `ASHWTestWin32` RPC (332 types) | `4af6ce2e...` | `ev_ed711a6c4b4448f83f13145057742276938b10b25d2235962fb968babef779ba` |
| `ModuleDll\HWSettings\AsusCustomization.dll` | 191 P/Invokes: `AsusCustomizationRpcClient`, `AsusSplendidRpcClientEx`, `AsusSystemAnalysisRpcClient` | `ea8a26df...` | `ev_7063dffdb45e88ed144b8ad26b8215c9a09478f5c0a3680c1f80230b27305226` |
| `ModuleDll\SystemDiagnosis\AsusSystemDiagnosis.dll` | 126 P/Invokes: `ASHWTestWin32` (Battery, Fan, Disk, RAM, Wi-Fi, BT, Ping, Process Perf) | `79862e57...` | `ev_90eabae7707655e3aa6185b5c20691bd39aa7f1a4166b1180ad1d427e65de296` |
| `ModuleDll\SystemDiagnosis\AsusSystemInteractiveDiagnosis.dll` | Interactive Display/Keyboard/RawInput/USB tests (`ASHWTestWin32`, `user32`, `Shcore`) | — | `ev_3cc1ffecd71e5de63d2a66c9eff77923c45a68a8130f3f23cde03e1fab941050` |
| `ModuleDll\Display\AsusDisplay.dll` | External monitor DDC/VCP, Splendid, Blue Light, Brightness (`AsusMonitorHelp.dll`) | — | `ev_ee0c6a87081e479316073ce49dbee084592002b207c4c014469c8941b1a32553` |
| `ModuleDll\AsusLumina\AsusColorKernel.dll` | Native color engine exports: `CalDeltaE`, `GamutMappingFlow`, `GetGammaLut`, `LutMapping8bit` | `5347a70a...` | `ev_bee254e3492eee9ac9048553389f82d56f41b6f82d07367fb70103a8c0bae807` |
| `ModuleDll\LiveUpdate\AsusLiveUpdate.dll` + `RPCInterface.idl` | Managed update client + MIDL `ALU_RPCInterface` (`A6FB5F6C-A70C-468E-9CB2-21667D31C5CD`) | `8b8e4042...` | `ev_4e2bab56f9cb060064c3467e9943457e515543057335afab0930a93f9286b099` |
| `ModuleDll\Settings\AsusSettings.dll` | App settings & preferences (91 types, pure managed) | `917d0057...` | `ev_638871ebfcf65514869f94ca30d7fb9c3e8034607910731a3bf5797168b09c95` |
| `ModuleDll\ASUSOne\AsusOne.dll` | Connected ASUS device registry (25 types, pure managed) | `70e5ce49...` | `ev_c8203ea47ba849d253ed2396828934eff9505d5f28960f871bf20b6fe0e3b2c2` |
| `ModuleDll\Mouse\AsusMouse.dll` | Mouse DPI/Battery/Scroll via `ASUSMouseHelper.dll` | — | `ev_621186c433b579098b27ccda194b7e7f8e232753aa8374ef569595954596ad4f` |
| `ModuleDll\Router\AsusRouter.dll` | Router/Wi-Fi telemetry & AiMesh discovery via `DUTUtil.dll` | — | `ev_ec0e11a715aaac4c5676dc51f9d3d694917735e80d9951addfd3315b765a4b1a` |
| `ModuleDll\Switch\AsusSwitch.dll` | Data migration & encrypted transfer via `AsusSwitchRpcClient.dll` | — | `ev_3edab5b900ea96e4ceeb72977b500cc24386728afb3b65cf79a97706a15bdd94` |
| `ModuleDll\MessageCenter\AsusMessageCenter.dll` | Notifications & bulletin event center (67 types) | `9eb4d9ac...` | `ev_f661325205f079479b13c1c0508e1ac5e4a67dc57d3786f73b0182eeab948c70` |
| `ModuleDll\CustomerSupport\AsusCustomerSupport.dll` | Device support, HID probe & diagnostic log export | — | `ev_e803a53f01168d9fa0d4bf78f4118943bd07931b2a68d46ff42ebdc74293a0a8` |

---

## 2. Dual-OS Hardware Mapping (`Windows x86_64` & `Linux Mint / Ubuntu x86_64`)

| MyASUS Feature | Windows Native Backend (`windows_atk.rs` / `windows_hw.rs` / `windows_extra.rs`) | Linux x86_64 Backend (`linux_hw.rs` / `linux_extra.rs` / `linux_updates.rs`) |
|---|---|---|
| **Device Identity & BIOS** | `Win32_ComputerSystemProduct`, `Win32_BIOS`, `HKLM\HARDWARE\DESCRIPTION\System\BIOS` | `/sys/class/dmi/id/{product_name,product_family,product_serial,bios_version,board_name}` + `/etc/os-release` |
| **CPU / RAM / Disk / Net Throughput** | `sysinfo` + live 30-sample sparkline + `Top Processes` | `sysinfo` + live 30-sample sparkline + `Top Processes` |
| **CPU Temperature** | `Win32_PerfFormattedData_Counters_ThermalZoneInformation` (`\_TZ.THRM`) | `/sys/class/hwmon/hwmon*/temp*_input` (`coretemp`/`k10temp`/`zenpower`/`asus`) + `thermal_zone0` |
| **CPU & GPU Fan RPM** | `\\.\ATKACPI` IOCTL `0x0022240C` `DSTS(0x00110013)` & `DSTS(0x00110014)` | `/sys/class/hwmon/hwmon*/fan{1,2}_input` |
| **Battery Health Charging** | `\\.\ATKACPI` `DEVS(0x00120057, 40..100)` + `GetSystemPowerStatus` | `/sys/class/power_supply/BAT*/charge_control_end_threshold` (direct or `pkexec`) |
| **Fan / Thermal Profile** | `\\.\ATKACPI` `0x00110019` (Vivobook) & `0x00120075` (ROG/Zenbook) | `/sys/firmware/acpi/platform_profile` -> `throttle_thermal_policy` -> `powerprofilesctl` |
| **CPU Governor / Power Plan** | `powercfg /getactivescheme` & `/setactive` | `/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor` |
| **Splendid / Eye Care LUT** | Win32 GDI `SetDeviceGammaRamp` (`gdi32.dll`) hardware LUT | `xrandr --gamma` warm blue-light filter |
| **Display Brightness & Monitors** | `WmiMonitorBrightnessMethods` + `Win32_VideoController` | `/sys/class/backlight/*/brightness` + `xrandr --current` |
| **Keyboard Backlight / Fn Lock / Touchpad / dGPU / Panel OD** | `\\.\ATKACPI` `DEVS(0x00050021 / 0x00100023 / 0x00100011 / 0x00090020 / 0x00050019)` | `/sys/class/leds/asus::kbd_backlight/brightness` & `/sys/devices/platform/asus-nb-wmi/*` |
| **My ASUS Devices (USB/BT/HID)** | `Win32_PnPEntity` (highlights ASUS `VID_0B05`) | `/sys/bus/usb/devices` (`idVendor == 0b05`) & `/sys/class/bluetooth` |
| **Connectivity / Wi-Fi / Router Gateway** | `netsh wlan show interfaces` + `Get-NetIPConfiguration` + live RX/TX | `/proc/net/wireless` + `iwgetid` + `ip route` + live RX/TX |
| **System Diagnosis & Interactive Tests** | 6 automated checks + Interactive Dead-Pixel / Keyboard / Touchpad / Top Processes | Same 6 automated checks + Interactive Self-Test Suite |
| **System Update** | `winget upgrade` | `fwupdmgr get-updates --json` (LVFS BIOS/UEFI) + `apt list --upgradable` |
| **Message Center & Support Export** | Auto thermal/disk/hardware alerts + JSON Support Bundle export | Auto thermal/disk/hardware alerts + JSON Support Bundle export |
