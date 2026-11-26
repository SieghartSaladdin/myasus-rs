use std::sync::Mutex;

use chrono::Utc;

use crate::models::{
    hardware::{FanProfile, HardwareCapabilities, HardwareSettingsState, SplendidMode},
    peripherals::{ConnectedDevice, DeviceKind, MonitorInfo, NetworkSummary},
    system::{BatteryMetrics, DeviceInfo, FanMetrics},
    updates::{UpdateItem, UpdateReport, UpdateSource},
};
use crate::platform::PlatformBackend;

/// Cross-platform fallback backend (Windows / macOS) so the Rust desktop app
/// compiles, runs, and previews MyASUS features cleanly outside Linux while
/// still collecting real CPU, RAM, and Disk telemetry via `sysinfo`.
#[allow(dead_code)]
pub struct FallbackBackend {
    state: Mutex<HardwareSettingsState>,
}

impl FallbackBackend {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let caps = HardwareCapabilities {
            battery_threshold_supported: true,
            battery_threshold_path: Some("Simulation / Non-Linux Preview".to_string()),
            fan_profile_supported: true,
            fan_profile_backend: "Simulation (Run on Linux Mint/Ubuntu for asus-nb-wmi)".to_string(),
            kbd_backlight_supported: true,
            kbd_backlight_max: 3,
            fn_lock_supported: true,
            touchpad_toggle_supported: true,
            panel_overdrive_supported: true,
            dgpu_disable_supported: false,
            udev_writable: true,
        };
        let state = HardwareSettingsState {
            capabilities: caps,
            ..HardwareSettingsState::default()
        };
        Self {
            state: Mutex::new(state),
        }
    }
}

impl PlatformBackend for FallbackBackend {
    fn backend_name(&self) -> &'static str {
        "Cross-Platform Preview (Linux sysfs active on Ubuntu/Mint)"
    }

    fn read_device_info(&self) -> DeviceInfo {
        DeviceInfo {
            product_name: "ASUS Vivobook".to_string(),
            product_family: "Vivobook Series".to_string(),
            serial_number: "PREVIEW-NON-LINUX".to_string(),
            board_name: "ASUS System Board".to_string(),
            bios_version: "UEFI (Preview)".to_string(),
            bios_date: "2024/08/22".to_string(),
            os_name: format!("{} (Preview Mode)", std::env::consts::OS),
            kernel_version: std::env::consts::ARCH.to_string(),
            cpu_brand: String::new(),
            cpu_cores: 0,
            total_memory_gb: 0.0,
            asus_wmi_loaded: false,
        }
    }

    fn read_temperatures(&self) -> (Option<f32>, Option<f32>) {
        (Some(58.0), Some(52.0))
    }

    fn read_fans(&self) -> FanMetrics {
        FanMetrics {
            cpu_fan_rpm: Some(3200),
            gpu_fan_rpm: Some(2400),
            extra_fans: Vec::new(),
        }
    }

    fn read_battery(&self) -> BatteryMetrics {
        let limit = self.state.lock().unwrap().battery_charge_limit;
        BatteryMetrics {
            present: true,
            percentage: limit.min(80),
            status: "Charging (Preview)".to_string(),
            ac_online: true,
            health_percent: Some(91.4),
            power_watts: Some(18.5),
            cycle_count: Some(112),
            charge_limit: Some(limit),
        }
    }

    fn read_hardware_settings(&self) -> HardwareSettingsState {
        self.state.lock().unwrap().clone()
    }

    fn set_battery_charge_limit(&self, limit: u8) -> Result<String, String> {
        let clamped = limit.clamp(40, 100);
        self.state.lock().unwrap().battery_charge_limit = clamped;
        Ok(format!(
            "Preview: Battery charge threshold set to {clamped}% (writes /sys/class/power_supply/BAT*/charge_control_end_threshold on Linux)"
        ))
    }

    fn set_fan_profile(&self, profile: FanProfile) -> Result<String, String> {
        self.state.lock().unwrap().fan_profile = profile;
        Ok(format!(
            "Preview: Fan profile set to {} (writes /sys/firmware/acpi/platform_profile on Linux)",
            profile.label()
        ))
    }

    fn set_kbd_backlight(&self, level: u8) -> Result<String, String> {
        let clamped = level.min(3);
        self.state.lock().unwrap().kbd_backlight_level = clamped;
        Ok(format!("Preview: Keyboard backlight set to level {clamped}"))
    }

    fn set_fn_lock(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().fn_lock_enabled = enabled;
        Ok(format!(
            "Preview: Fn Lock {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_touchpad(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().touchpad_enabled = enabled;
        Ok(format!(
            "Preview: Touchpad {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_panel_overdrive(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().panel_overdrive_enabled = enabled;
        Ok(format!(
            "Preview: Panel Overdrive {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    }

    fn set_dgpu_disabled(&self, disabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().dgpu_disabled = disabled;
        Ok(format!(
            "Preview: dGPU {}",
            if disabled { "disabled" } else { "enabled" }
        ))
    }

    fn set_splendid_mode(&self, mode: SplendidMode, eye_care_level: u8) -> Result<String, String> {
        let mut s = self.state.lock().unwrap();
        s.splendid_mode = mode;
        s.eye_care_level = eye_care_level;
        Ok(format!("Preview: Splendid mode set to {}", mode.label()))
    }

    fn set_display_brightness(&self, percent: u8) -> Result<String, String> {
        let clamped = percent.clamp(5, 100);
        self.state.lock().unwrap().display_brightness_percent = clamped;
        Ok(format!("Preview: Display brightness set to {clamped}%"))
    }

    fn set_volume_booster(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().volume_booster_enabled = enabled;
        Ok(format!("Preview: Volume Booster {}", if enabled { "On" } else { "Off" }))
    }

    fn set_oled_pixel_refresh(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().oled_care.pixel_refresh_enabled = enabled;
        Ok(format!("Preview: OLED Pixel Refresh {}", if enabled { "On" } else { "Off" }))
    }

    fn set_oled_taskbar_autohide(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().oled_care.auto_hide_taskbar = enabled;
        Ok(format!("Preview: Taskbar Auto-Hide {}", if enabled { "On" } else { "Off" }))
    }

    fn set_oled_taskbar_transparency(&self, enabled: bool) -> Result<String, String> {
        self.state.lock().unwrap().oled_care.taskbar_transparency = enabled;
        Ok(format!("Preview: Taskbar Transparency {}", if enabled { "On" } else { "Off" }))
    }

    fn set_oled_flicker_free_dimming(&self, percent: u8) -> Result<String, String> {
        let clamped = percent.clamp(30, 100);
        self.state.lock().unwrap().oled_care.flicker_free_dimming = clamped;
        Ok(format!("Preview: OLED Flicker-Free Dimming {clamped}%"))
    }

    fn read_cpu_governor(&self) -> String {
        "balanced".to_string()
    }

    fn set_cpu_governor(&self, governor: &str) -> Result<String, String> {
        Ok(format!("Preview: CPU governor set to {governor}"))
    }

    fn list_connected_devices(&self) -> Vec<ConnectedDevice> {
        vec![ConnectedDevice {
            name: "ASUS Vivobook Internal Keyboard".into(),
            vendor: "ASUSTeK".into(),
            kind: DeviceKind::Input,
            is_asus: true,
            status: "OK".into(),
            identifier: "VID_0B05".into(),
        }]
    }

    fn list_monitors(&self) -> Vec<MonitorInfo> {
        vec![MonitorInfo {
            name: "Built-in Display".into(),
            resolution: "1920x1080".into(),
            refresh_hz: Some(60),
            primary: true,
        }]
    }

    fn network_summary(&self) -> NetworkSummary {
        NetworkSummary {
            interface_name: "wlan0".into(),
            ssid: Some("Preview-WiFi".into()),
            connected: true,
            signal_dbm: Some(-54),
            local_ip: Some("192.168.1.100".into()),
            gateway_ip: Some("192.168.1.1".into()),
            rx_kbps: 0.0,
            tx_kbps: 0.0,
        }
    }

    fn check_updates(&self) -> UpdateReport {
        UpdateReport {
            last_checked: Utc::now().format("%Y/%m/%d %H:%M UTC").to_string(),
            items: vec![
                UpdateItem {
                    name: "linux-firmware (Preview)".to_string(),
                    current_version: "2024.08".to_string(),
                    available_version: "2024.11".to_string(),
                    source: UpdateSource::KernelDriver,
                    summary: "On Ubuntu / Linux Mint, real checks run via `apt` and `fwupdmgr`."
                        .to_string(),
                    critical: false,
                },
            ],
            fwupd_available: false,
            apt_available: false,
            notes: vec![
                "Running in non-Linux preview mode. Build on Ubuntu / Linux Mint for live fwupd + APT checks."
                    .to_string(),
            ],
        }
    }

    fn wifi_signal_info(&self) -> (bool, Option<i32>, String) {
        (true, Some(-54), "wlan0 (preview)".to_string())
    }

    fn bluetooth_present(&self) -> bool {
        true
    }

    fn boot_time_summary(&self) -> Option<String> {
        Some("Startup finished in 4.2s (kernel) + 7.1s (userspace) [preview]".to_string())
    }
}
