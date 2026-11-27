use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::models::{
    hardware::{
        ColorGamut, FanProfile, HardwareCapabilities, HardwareSettingsState, OledCareState,
        SplendidMode, TaskFirstMode,
    },
    peripherals::{ConnectedDevice, MonitorInfo, NetworkSummary},
    system::{BatteryMetrics, DeviceInfo, FanMetrics},
    updates::{UpdateItem, UpdateReport, UpdateSource},
};
use crate::platform::windows_atk::{
    AtkAcpiClient, DEVID_BATTERY_LIMIT, DEVID_CPU_FAN, DEVID_DGPU_DISABLE, DEVID_FN_LOCK,
    DEVID_GPU_FAN, DEVID_KBD_BACKLIGHT, DEVID_PANEL_OD, DEVID_TOUCHPAD,
};
use crate::platform::{windows_extra, windows_oled, PlatformBackend};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(lp_system_power_status: *mut SystemPowerStatus) -> i32;
}

struct CachedWindowsSensors {
    cpu_temp: Option<f32>,
    last_temp_poll: Option<Instant>,
    polling_temp: bool,
    battery_charge_limit: u8,
    splendid_mode: SplendidMode,
    eye_care_level: u8,
    brightness_percent: u8,
    volume_booster: bool,
    oled_care: OledCareState,
}

pub struct WindowsAsusBackend {
    atk_available: bool,
    device_cache: DeviceInfo,
    sensors: Arc<Mutex<CachedWindowsSensors>>,
}

impl WindowsAsusBackend {
    pub fn new() -> Self {
        let atk_available = AtkAcpiClient::is_available();
        let device_cache = Self::probe_device_info(atk_available);
        let initial_temp = Self::query_thermal_zone_celsius();
        let initial_brightness = Self::query_brightness().unwrap_or(75);
        let mut initial_oled = windows_oled::read_windows_oled_care();
        initial_oled.flicker_free_dimming =
            crate::services::display_prefs::DisplayPrefs::load().flicker_free_dimming;

        Self {
            atk_available,
            device_cache,
            sensors: Arc::new(Mutex::new(CachedWindowsSensors {
                cpu_temp: initial_temp,
                last_temp_poll: Some(Instant::now()),
                polling_temp: false,
                battery_charge_limit: 80,
                splendid_mode: SplendidMode::Normal,
                eye_care_level: 2,
                brightness_percent: initial_brightness,
                volume_booster: false,
                oled_care: initial_oled,
            })),
        }
    }

    fn run_powershell(script: &str) -> Option<String> {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn probe_device_info(atk_available: bool) -> DeviceInfo {
        let script = r#"
$cs = Get-CimInstance Win32_ComputerSystemProduct -ErrorAction SilentlyContinue
$bios = Get-CimInstance Win32_BIOS -ErrorAction SilentlyContinue
$bb = Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue
$os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
$reg = Get-ItemProperty 'HKLM:\HARDWARE\DESCRIPTION\System\BIOS' -ErrorAction SilentlyContinue
[PSCustomObject]@{
  product = if ($cs.Name) { $cs.Name } else { $reg.SystemProductName }
  family = if ($reg.SystemFamily) { $reg.SystemFamily } else { 'ASUS' }
  serial = if ($cs.IdentifyingNumber) { $cs.IdentifyingNumber } else { 'Unknown' }
  board = if ($bb.Product) { $bb.Product } else { $reg.BaseBoardProduct }
  bios_ver = if ($bios.SMBIOSBIOSVersion) { $bios.SMBIOSBIOSVersion } else { $reg.BIOSVersion }
  bios_date = if ($reg.BIOSReleaseDate) { $reg.BIOSReleaseDate } else { '' }
  os = if ($os.Caption) { $os.Caption } else { 'Windows' }
  kernel = if ($os.Version) { $os.Version } else { 'NT' }
} | ConvertTo-Json -Compress
"#;

        let mut info = DeviceInfo {
            asus_wmi_loaded: atk_available,
            os_name: "Windows".to_string(),
            ..DeviceInfo::default()
        };

        if let Some(raw) = Self::run_powershell(script) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                let get_s = |k: &str| {
                    v.get(k)
                        .and_then(|x| x.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                };
                if let Some(s) = get_s("product") {
                    info.product_name = s;
                }
                if let Some(s) = get_s("family") {
                    info.product_family = s;
                }
                if let Some(s) = get_s("serial") {
                    info.serial_number = s;
                }
                if let Some(s) = get_s("board") {
                    info.board_name = s;
                }
                if let Some(s) = get_s("bios_ver") {
                    info.bios_version = s;
                }
                if let Some(s) = get_s("bios_date") {
                    info.bios_date = s;
                }
                if let Some(s) = get_s("os") {
                    info.os_name = s;
                }
                if let Some(s) = get_s("kernel") {
                    info.kernel_version = format!("Windows NT {s}");
                }
            }
        }
        info
    }

    fn query_thermal_zone_celsius() -> Option<f32> {
        let script = r#"
$tz = Get-CimInstance Win32_PerfFormattedData_Counters_ThermalZoneInformation -ErrorAction SilentlyContinue | Select-Object -First 1
if ($tz -and $tz.HighPrecisionTemperature -gt 2000) {
  [math]::Round(($tz.HighPrecisionTemperature / 10.0) - 273.15, 1)
} elseif ($tz -and $tz.Temperature -gt 200) {
  [math]::Round($tz.Temperature - 273.15, 1)
}
"#;
        let out = Self::run_powershell(script)?;
        let val = out.trim().replace(',', ".").parse::<f32>().ok()?;
        if (0.0..=130.0).contains(&val) {
            Some(val)
        } else {
            None
        }
    }

    fn query_brightness() -> Option<u8> {
        let out = Self::run_powershell(
            "(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness",
        )?;
        out.trim().parse::<u8>().ok()
    }

    fn maybe_refresh_temp_async(&self) {
        let mut guard = self.sensors.lock().unwrap();
        let stale = guard
            .last_temp_poll
            .map(|t| t.elapsed() >= Duration::from_secs(4))
            .unwrap_or(true);
        if !stale || guard.polling_temp {
            return;
        }
        guard.polling_temp = true;
        let shared = Arc::clone(&self.sensors);
        std::thread::spawn(move || {
            let temp = Self::query_thermal_zone_celsius();
            let mut g = shared.lock().unwrap();
            if temp.is_some() {
                g.cpu_temp = temp;
            }
            g.last_temp_poll = Some(Instant::now());
            g.polling_temp = false;
        });
    }
}

impl PlatformBackend for WindowsAsusBackend {
    fn backend_name(&self) -> &'static str {
        if self.atk_available {
            "Windows Native (\\\\.\\ATKACPI + Win32 Sensors)"
        } else {
            "Windows Native (Win32 / WMI)"
        }
    }

    fn read_device_info(&self) -> DeviceInfo {
        self.device_cache.clone()
    }

    fn read_temperatures(&self) -> (Option<f32>, Option<f32>) {
        self.maybe_refresh_temp_async();
        let temp = self.sensors.lock().unwrap().cpu_temp;
        (temp, None)
    }

    fn read_fans(&self) -> FanMetrics {
        if !self.atk_available {
            return FanMetrics::default();
        }
        FanMetrics {
            cpu_fan_rpm: AtkAcpiClient::read_fan_rpm(DEVID_CPU_FAN),
            gpu_fan_rpm: AtkAcpiClient::read_fan_rpm(DEVID_GPU_FAN),
            extra_fans: Vec::new(),
        }
    }

    fn read_battery(&self) -> BatteryMetrics {
        let mut status = SystemPowerStatus {
            ac_line_status: 255,
            battery_flag: 255,
            battery_life_percent: 255,
            system_status_flag: 0,
            battery_life_time: 0,
            battery_full_life_time: 0,
        };
        let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
        let ac_online = ok && status.ac_line_status == 1;
        if !ok || status.battery_flag == 128 || status.battery_life_percent > 100 {
            return BatteryMetrics {
                present: false,
                ac_online,
                status: if ac_online {
                    "AC Power (No battery reading)".to_string()
                } else {
                    "Unknown".to_string()
                },
                charge_limit: if AtkAcpiClient::is_present(DEVID_BATTERY_LIMIT) {
                    Some(self.sensors.lock().unwrap().battery_charge_limit)
                } else {
                    None
                },
                ..BatteryMetrics::default()
            };
        }

        let percentage = status.battery_life_percent;
        let charge_state = if (status.battery_flag & 8) != 0 {
            "Charging"
        } else if ac_online && percentage >= 99 {
            "Full"
        } else if ac_online {
            "Plugged in (Not charging)"
        } else {
            "Discharging"
        };

        let limit = self.sensors.lock().unwrap().battery_charge_limit;
        BatteryMetrics {
            present: true,
            percentage,
            status: charge_state.to_string(),
            ac_online,
            health_percent: None,
            power_watts: None,
            cycle_count: None,
            charge_limit: if AtkAcpiClient::is_present(DEVID_BATTERY_LIMIT) {
                Some(limit)
            } else {
                None
            },
        }
    }

    fn read_hardware_settings(&self) -> HardwareSettingsState {
        let (fan_profile, fan_supported, fan_backend) = match AtkAcpiClient::read_fan_profile() {
            Some((p, name)) => (p, true, name.to_string()),
            None => (FanProfile::Standard, false, "Unavailable".to_string()),
        };

        let bat_supported = AtkAcpiClient::is_present(DEVID_BATTERY_LIMIT);
        let kbd_raw = AtkAcpiClient::dsts(DEVID_KBD_BACKLIGHT);
        let kbd_supported = kbd_raw.is_some();
        let kbd_level = kbd_raw.map(|v| (v & 0x03) as u8).unwrap_or(0);

        let fn_raw = AtkAcpiClient::dsts(DEVID_FN_LOCK);
        let tp_raw = AtkAcpiClient::dsts(DEVID_TOUCHPAD);
        let od_raw = AtkAcpiClient::dsts(DEVID_PANEL_OD);
        let dgpu_raw = AtkAcpiClient::dsts(DEVID_DGPU_DISABLE);

        let g = self.sensors.lock().unwrap();

        HardwareSettingsState {
            capabilities: HardwareCapabilities {
                battery_threshold_supported: bat_supported,
                battery_threshold_path: bat_supported
                    .then(|| "\\\\.\\ATKACPI DEVS(0x00120057)".to_string()),
                fan_profile_supported: fan_supported,
                fan_profile_backend: fan_backend,
                kbd_backlight_supported: kbd_supported,
                kbd_backlight_max: 3,
                fn_lock_supported: fn_raw.is_some(),
                touchpad_toggle_supported: tp_raw.is_some(),
                panel_overdrive_supported: od_raw.is_some(),
                dgpu_disable_supported: dgpu_raw.is_some(),
                udev_writable: self.atk_available,
            },
            battery_charge_limit: g.battery_charge_limit,
            fan_profile,
            kbd_backlight_level: kbd_level,
            fn_lock_enabled: fn_raw.map(|v| (v & 1) == 1).unwrap_or(false),
            touchpad_enabled: tp_raw.map(|v| (v & 1) == 1).unwrap_or(true),
            panel_overdrive_enabled: od_raw.map(|v| (v & 1) == 1).unwrap_or(false),
            dgpu_disabled: dgpu_raw.map(|v| (v & 1) == 1).unwrap_or(false),
            splendid_mode: g.splendid_mode,
            color_gamut: ColorGamut::Native,
            eye_care_level: g.eye_care_level,
            display_brightness_percent: g.brightness_percent,
            volume_booster_enabled: g.volume_booster,
            clearvoice_mic_enabled: true,
            clearvoice_speaker_enabled: false,
            wifi_smartconnect_enabled: true,
            taskfirst_mode: TaskFirstMode::Normal,
            oled_care: g.oled_care.clone(),
        }
    }

    fn set_battery_charge_limit(&self, limit: u8) -> Result<String, String> {
        let clamped = limit.clamp(40, 100);
        AtkAcpiClient::devs(DEVID_BATTERY_LIMIT, clamped as u32)?;
        self.sensors.lock().unwrap().battery_charge_limit = clamped;
        Ok(format!(
            "Battery charge limit set to {clamped}% via \\\\.\\ATKACPI (0x00120057)"
        ))
    }

    fn set_fan_profile(&self, profile: FanProfile) -> Result<String, String> {
        AtkAcpiClient::write_fan_profile(profile)
    }

    fn set_kbd_backlight(&self, level: u8) -> Result<String, String> {
        let clamped = level.min(3);
        let payload = if clamped == 0 {
            0x00u32
        } else {
            0x80u32 | (clamped as u32)
        };
        AtkAcpiClient::devs(DEVID_KBD_BACKLIGHT, payload)?;
        Ok(format!(
            "Keyboard backlight set to level {clamped} via \\\\.\\ATKACPI"
        ))
    }

    fn set_fn_lock(&self, enabled: bool) -> Result<String, String> {
        AtkAcpiClient::devs(DEVID_FN_LOCK, if enabled { 1 } else { 0 })?;
        Ok(format!(
            "Fn Lock {} via \\\\.\\ATKACPI",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_touchpad(&self, enabled: bool) -> Result<String, String> {
        AtkAcpiClient::devs(DEVID_TOUCHPAD, if enabled { 1 } else { 0 })?;
        Ok(format!(
            "Touchpad {} via \\\\.\\ATKACPI",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_panel_overdrive(&self, enabled: bool) -> Result<String, String> {
        AtkAcpiClient::devs(DEVID_PANEL_OD, if enabled { 1 } else { 0 })?;
        Ok(format!(
            "Panel Overdrive {} via \\\\.\\ATKACPI",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_dgpu_disabled(&self, disabled: bool) -> Result<String, String> {
        AtkAcpiClient::devs(DEVID_DGPU_DISABLE, if disabled { 1 } else { 0 })?;
        Ok(format!(
            "dGPU {} via \\\\.\\ATKACPI",
            if disabled { "disabled" } else { "enabled" }
        ))
    }

    fn set_splendid_mode(&self, mode: SplendidMode, eye_care_level: u8) -> Result<String, String> {
        let mut g = self.sensors.lock().unwrap();
        g.splendid_mode = mode;
        g.eye_care_level = eye_care_level;
        drop(g);
        windows_extra::apply_windows_gamma(mode, eye_care_level)
    }

    fn read_cpu_governor(&self) -> String {
        windows_extra::read_windows_power_plan()
    }

    fn set_cpu_governor(&self, governor: &str) -> Result<String, String> {
        windows_extra::set_windows_power_plan(governor)
    }

    fn list_connected_devices(&self) -> Vec<ConnectedDevice> {
        windows_extra::list_windows_devices()
    }

    fn list_monitors(&self) -> Vec<MonitorInfo> {
        windows_extra::list_windows_monitors()
    }

    fn network_summary(&self) -> NetworkSummary {
        windows_extra::read_windows_network_details(self.wifi_signal_info())
    }

    fn set_display_brightness(&self, percent: u8) -> Result<String, String> {
        let clamped = percent.clamp(5, 100);
        let cmd = format!(
            "(Get-CimInstance -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Select-Object -First 1) | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{ Timeout = 1; Brightness = {clamped} }} | Out-Null; 'OK'"
        );
        if Self::run_powershell(&cmd).is_some() {
            self.sensors.lock().unwrap().brightness_percent = clamped;
            Ok(format!("Display brightness set to {clamped}% via WMI"))
        } else {
            Err("WmiMonitorBrightnessMethods unavailable on current display".to_string())
        }
    }

    fn set_volume_booster(&self, enabled: bool) -> Result<String, String> {
        self.sensors.lock().unwrap().volume_booster = enabled;
        windows_oled::set_windows_volume_booster(enabled)
    }

    fn set_oled_pixel_refresh(&self, enabled: bool) -> Result<String, String> {
        self.sensors.lock().unwrap().oled_care.pixel_refresh_enabled = enabled;
        windows_oled::set_windows_pixel_refresh(enabled)
    }

    fn set_oled_taskbar_autohide(&self, enabled: bool) -> Result<String, String> {
        self.sensors.lock().unwrap().oled_care.auto_hide_taskbar = enabled;
        windows_oled::set_windows_taskbar_autohide(enabled)
    }

    fn set_oled_taskbar_transparency(&self, enabled: bool) -> Result<String, String> {
        self.sensors.lock().unwrap().oled_care.taskbar_transparency = enabled;
        windows_oled::set_windows_taskbar_transparency(enabled)
    }

    fn set_oled_flicker_free_dimming(&self, percent: u8) -> Result<String, String> {
        let clamped = percent.clamp(30, 100);
        self.sensors.lock().unwrap().oled_care.flicker_free_dimming = clamped;
        let msg = windows_oled::apply_windows_flicker_free_dimming(clamped)?;
        match crate::services::display_prefs::DisplayPrefs::save_dimming(clamped) {
            Ok(_) => Ok(msg),
            Err(e) => Ok(format!("{msg} (warning: not persisted: {e})")),
        }
    }

    fn check_updates(&self) -> UpdateReport {
        let mut report = UpdateReport {
            last_checked: Utc::now().format("%Y/%m/%d %H:%M UTC").to_string(),
            items: Vec::new(),
            fwupd_available: false,
            apt_available: false,
            notes: Vec::new(),
        };

        let out = Command::new("winget")
            .args(["upgrade", "--accept-source-agreements"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match out {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut seen_sep = false;
                for line in text.lines() {
                    if line.starts_with("---") {
                        seen_sep = true;
                        continue;
                    }
                    if !seen_sep {
                        continue;
                    }
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if cols.len() >= 4 && !line.contains("upgrades available") {
                        let name = cols[0].to_string();
                        let available = cols[cols.len() - 2].to_string();
                        let current = cols[cols.len() - 3].to_string();
                        report.items.push(UpdateItem {
                            name,
                            current_version: current,
                            available_version: available,
                            source: UpdateSource::AptSystem,
                            summary: "Available Windows package upgrade via winget".to_string(),
                            critical: false,
                        });
                        if report.items.len() >= 15 {
                            break;
                        }
                    }
                }
                report.notes.push(
                    "Windows update scan uses `winget upgrade`. On Linux Mint / Ubuntu, scans use `fwupdmgr` + `apt`."
                        .into(),
                );
            }
            _ => {
                report.notes.push(
                    "winget not available; check Windows Update or run on Linux Mint / Ubuntu for fwupd + APT."
                        .into(),
                );
            }
        }
        report
    }

    fn wifi_signal_info(&self) -> (bool, Option<i32>, String) {
        let out = Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if let Ok(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut iface = String::new();
            let mut connected = false;
            let mut signal_dbm = None;
            for line in text.lines() {
                let t = line.trim();
                if let Some(v) = t.strip_prefix("Name") {
                    iface = v.trim_start_matches([' ', ':']).to_string();
                } else if t.starts_with("State") && t.contains("connected") && !t.contains("disconnected") {
                    connected = true;
                } else if let Some(v) = t.strip_prefix("Signal") {
                    if let Ok(pct) = v.trim_start_matches([' ', ':']).trim_end_matches('%').trim().parse::<i32>() {
                        // Approximate RSSI dBm from Windows percentage (0..100 -> -100..-50 dBm)
                        signal_dbm = Some((pct / 2) - 100);
                    }
                }
            }
            if !iface.is_empty() {
                return (connected, signal_dbm, iface);
            }
        }
        (false, None, "No active Wi-Fi interface".to_string())
    }

    fn bluetooth_present(&self) -> bool {
        Self::run_powershell(
            "(Get-CimInstance Win32_PnPEntity -ErrorAction SilentlyContinue | Where-Object { $_.PNPClass -eq 'Bluetooth' } | Select-Object -First 1).Name",
        )
        .is_some()
    }

    fn boot_time_summary(&self) -> Option<String> {
        Self::run_powershell(
            "$os = Get-CimInstance Win32_OperatingSystem; \"Last boot: $($os.LastBootUpTime.ToString('yyyy/MM/dd HH:mm'))\"",
        )
    }
}
