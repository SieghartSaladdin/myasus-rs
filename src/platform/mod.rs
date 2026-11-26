pub mod fallback_hw;
#[cfg(target_os = "linux")]
pub mod linux_extra;
#[cfg(target_os = "linux")]
pub mod linux_hw;
#[cfg(target_os = "linux")]
pub mod linux_oled;
#[cfg(target_os = "linux")]
pub mod linux_updates;
pub mod sysfs;
#[cfg(target_os = "windows")]
pub mod windows_atk;
#[cfg(target_os = "windows")]
pub mod windows_extra;
#[cfg(target_os = "windows")]
pub mod windows_hw;
#[cfg(target_os = "windows")]
pub mod windows_oled;

use crate::models::{
    hardware::{FanProfile, HardwareSettingsState, SplendidMode},
    peripherals::{ConnectedDevice, MonitorInfo, NetworkSummary},
    system::{BatteryMetrics, DeviceInfo, FanMetrics},
    updates::UpdateReport,
};

pub trait PlatformBackend: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn read_device_info(&self) -> DeviceInfo;
    fn read_temperatures(&self) -> (Option<f32>, Option<f32>);
    fn read_fans(&self) -> FanMetrics;
    fn read_battery(&self) -> BatteryMetrics;
    fn read_hardware_settings(&self) -> HardwareSettingsState;

    fn set_battery_charge_limit(&self, limit: u8) -> Result<String, String>;
    fn set_fan_profile(&self, profile: FanProfile) -> Result<String, String>;
    fn set_kbd_backlight(&self, level: u8) -> Result<String, String>;
    fn set_fn_lock(&self, enabled: bool) -> Result<String, String>;
    fn set_touchpad(&self, enabled: bool) -> Result<String, String>;
    fn set_panel_overdrive(&self, enabled: bool) -> Result<String, String>;
    fn set_dgpu_disabled(&self, disabled: bool) -> Result<String, String>;
    fn set_splendid_mode(&self, mode: SplendidMode, eye_care_level: u8) -> Result<String, String>;
    fn set_display_brightness(&self, percent: u8) -> Result<String, String>;
    fn set_volume_booster(&self, enabled: bool) -> Result<String, String>;
    fn set_oled_pixel_refresh(&self, enabled: bool) -> Result<String, String>;
    fn set_oled_taskbar_autohide(&self, enabled: bool) -> Result<String, String>;
    fn set_oled_taskbar_transparency(&self, enabled: bool) -> Result<String, String>;
    fn set_oled_flicker_free_dimming(&self, percent: u8) -> Result<String, String>;

    fn read_cpu_governor(&self) -> String;
    fn set_cpu_governor(&self, governor: &str) -> Result<String, String>;
    fn list_connected_devices(&self) -> Vec<ConnectedDevice>;
    fn list_monitors(&self) -> Vec<MonitorInfo>;
    fn network_summary(&self) -> NetworkSummary;

    fn check_updates(&self) -> UpdateReport;
    fn wifi_signal_info(&self) -> (bool, Option<i32>, String);
    fn bluetooth_present(&self) -> bool;
    fn boot_time_summary(&self) -> Option<String>;
}

pub fn create_backend() -> Box<dyn PlatformBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux_hw::LinuxAsusBackend::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_hw::WindowsAsusBackend::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Box::new(fallback_hw::FallbackBackend::new())
    }
}
