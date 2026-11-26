use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use crate::models::{
    hardware::{
        ColorGamut, FanProfile, HardwareCapabilities, HardwareSettingsState, OledCareState,
        SplendidMode, TaskFirstMode,
    },
    peripherals::{ConnectedDevice, MonitorInfo, NetworkSummary},
    system::{BatteryMetrics, DeviceInfo, FanMetrics},
    updates::UpdateReport,
};
use crate::platform::{linux_extra, linux_oled, linux_updates, sysfs, PlatformBackend};

const ASUS_WMI_DIR: &str = "/sys/devices/platform/asus-nb-wmi";
const ACPI_PROFILE_PATH: &str = "/sys/firmware/acpi/platform_profile";
const KBD_LED_DIR: &str = "/sys/class/leds/asus::kbd_backlight";

pub struct LinuxAsusBackend {
    visual_state: Mutex<(SplendidMode, u8, bool, OledCareState)>,
}

impl LinuxAsusBackend {
    pub fn new() -> Self {
        Self {
            visual_state: Mutex::new((
                SplendidMode::Normal,
                2,
                false,
                linux_oled::read_linux_oled_care(),
            )),
        }
    }

    fn find_battery_dir() -> Option<PathBuf> {
        let dir = Path::new("/sys/class/power_supply");
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("BAT")
                || sysfs::read_trimmed(path.join("type")).as_deref() == Some("Battery")
            {
                return Some(path);
            }
        }
        None
    }

    fn find_ac_online() -> bool {
        let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
            return true;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if sysfs::read_trimmed(path.join("type")).as_deref() == Some("Mains") {
                if let Some(val) = sysfs::read_u32(path.join("online")) {
                    return val == 1;
                }
            }
        }
        true
    }

    fn find_backlight_dir() -> Option<PathBuf> {
        let entries = fs::read_dir("/sys/class/backlight").ok()?;
        entries.flatten().map(|e| e.path()).next()
    }

    fn is_asus_wmi_loaded() -> bool {
        if Path::new(ASUS_WMI_DIR).exists() {
            return true;
        }
        fs::read_to_string("/proc/modules")
            .map(|m| m.contains("asus_nb_wmi") || m.contains("asus_wmi"))
            .unwrap_or(false)
    }

    fn read_fan_profile(&self) -> (FanProfile, bool, String) {
        let acpi_path = Path::new(ACPI_PROFILE_PATH);
        if let Some(raw) = sysfs::read_trimmed(acpi_path) {
            if let Some(profile) = FanProfile::from_acpi_name(&raw) {
                return (profile, true, "ACPI platform_profile".to_string());
            }
        }

        let policy_path = Path::new(ASUS_WMI_DIR).join("throttle_thermal_policy");
        if let Some(code) = sysfs::read_u32(&policy_path) {
            if let Some(profile) = FanProfile::from_asus_policy_code(code as u8) {
                return (profile, true, "asus-nb-wmi thermal_policy".to_string());
            }
        }

        if let Ok(out) = Command::new("powerprofilesctl").arg("get").output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                if let Some(profile) = FanProfile::from_acpi_name(&s) {
                    return (profile, true, "power-profiles-daemon".to_string());
                }
            }
        }

        (FanProfile::Standard, false, "Unavailable".to_string())
    }

    fn read_display_brightness(&self) -> u8 {
        let Some(dir) = Self::find_backlight_dir() else {
            return 75;
        };
        let cur = sysfs::read_u32(dir.join("brightness")).unwrap_or(0);
        let max = sysfs::read_u32(dir.join("max_brightness")).unwrap_or(100);
        if max == 0 {
            75
        } else {
            (((cur as f32 / max as f32) * 100.0).round() as u8).clamp(5, 100)
        }
    }
}

impl PlatformBackend for LinuxAsusBackend {
    fn backend_name(&self) -> &'static str {
        "Linux (asus-nb-wmi / hwmon / ACPI)"
    }

    fn read_device_info(&self) -> DeviceInfo {
        let dmi = Path::new("/sys/class/dmi/id");
        let product_name = sysfs::read_trimmed(dmi.join("product_name"))
            .unwrap_or_else(|| "ASUS Laptop".to_string());
        let product_family = sysfs::read_trimmed(dmi.join("product_family"))
            .unwrap_or_else(|| "Vivobook".to_string());
        let serial_number = sysfs::read_trimmed(dmi.join("product_serial"))
            .unwrap_or_else(|| "Protected (root/DMI)".to_string());
        let board_name =
            sysfs::read_trimmed(dmi.join("board_name")).unwrap_or_else(|| "Unknown".to_string());
        let bios_version =
            sysfs::read_trimmed(dmi.join("bios_version")).unwrap_or_else(|| "Unknown".to_string());
        let bios_date =
            sysfs::read_trimmed(dmi.join("bios_date")).unwrap_or_else(|| "Unknown".to_string());
        let os_name = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|c| sysfs::parse_os_release_pretty_name(&c))
            .unwrap_or_else(|| "Linux".to_string());
        let kernel_version = sysfs::read_trimmed("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|| "Unknown".to_string());

        DeviceInfo {
            product_name,
            product_family,
            serial_number,
            board_name,
            bios_version,
            bios_date,
            os_name,
            kernel_version,
            cpu_brand: String::new(),
            cpu_cores: 0,
            total_memory_gb: 0.0,
            asus_wmi_loaded: Self::is_asus_wmi_loaded(),
        }
    }

    fn read_temperatures(&self) -> (Option<f32>, Option<f32>) {
        let mut cpu_temp: Option<f32> = None;
        let mut gpu_temp: Option<f32> = None;

        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = sysfs::read_trimmed(path.join("name"))
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let temp = sysfs::read_trimmed(path.join("temp1_input"))
                    .and_then(|s| s.parse::<i64>().ok())
                    .and_then(sysfs::millidegrees_to_celsius);

                if let Some(c) = temp {
                    if matches!(
                        name.as_str(),
                        "coretemp" | "k10temp" | "zenpower" | "asus" | "acpitz"
                    ) && cpu_temp.is_none()
                    {
                        cpu_temp = Some(c);
                    } else if matches!(name.as_str(), "amdgpu" | "nouveau" | "nvidia")
                        && gpu_temp.is_none()
                    {
                        gpu_temp = Some(c);
                    }
                }
            }
        }

        if cpu_temp.is_none() {
            cpu_temp = sysfs::read_trimmed("/sys/class/thermal/thermal_zone0/temp")
                .and_then(|s| s.parse::<i64>().ok())
                .and_then(sysfs::millidegrees_to_celsius);
        }

        (cpu_temp, gpu_temp)
    }

    fn read_fans(&self) -> FanMetrics {
        let mut metrics = FanMetrics::default();
        let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
            return metrics;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let hwmon_name = sysfs::read_trimmed(path.join("name")).unwrap_or_default();
            for idx in 1..=4 {
                let fan_path = path.join(format!("fan{idx}_input"));
                if let Some(rpm) = sysfs::read_u32(&fan_path) {
                    let label = sysfs::read_trimmed(path.join(format!("fan{idx}_label")))
                        .unwrap_or_else(|| format!("{hwmon_name} fan{idx}"));
                    let lower = label.to_ascii_lowercase();
                    if (lower.contains("cpu") || idx == 1) && metrics.cpu_fan_rpm.is_none() {
                        metrics.cpu_fan_rpm = Some(rpm);
                    } else if (lower.contains("gpu") || idx == 2) && metrics.gpu_fan_rpm.is_none()
                    {
                        metrics.gpu_fan_rpm = Some(rpm);
                    } else {
                        metrics.extra_fans.push((label, rpm));
                    }
                }
            }
        }
        metrics
    }

    fn read_battery(&self) -> BatteryMetrics {
        let Some(bat) = Self::find_battery_dir() else {
            return BatteryMetrics::default();
        };
        let percentage = sysfs::read_u32(bat.join("capacity"))
            .unwrap_or(0)
            .min(100) as u8;
        let status =
            sysfs::read_trimmed(bat.join("status")).unwrap_or_else(|| "Unknown".to_string());
        let ac_online = Self::find_ac_online();

        let full = sysfs::read_u64(bat.join("energy_full"))
            .or_else(|| sysfs::read_u64(bat.join("charge_full")))
            .unwrap_or(0);
        let design = sysfs::read_u64(bat.join("energy_full_design"))
            .or_else(|| sysfs::read_u64(bat.join("charge_full_design")))
            .unwrap_or(0);
        let health_percent = sysfs::compute_battery_health(full, design);

        let power_watts = sysfs::read_u64(bat.join("power_now"))
            .map(|uw| uw as f32 / 1_000_000.0)
            .or_else(|| {
                let ua = sysfs::read_u64(bat.join("current_now"))?;
                let uv = sysfs::read_u64(bat.join("voltage_now"))?;
                Some((ua as f64 * uv as f64 / 1e12) as f32)
            });

        let cycle_count = sysfs::read_u32(bat.join("cycle_count"));
        let charge_limit = sysfs::read_u32(bat.join("charge_control_end_threshold"))
            .map(|v| (v as u8).clamp(40, 100));

        BatteryMetrics {
            present: true,
            percentage,
            status,
            ac_online,
            health_percent,
            power_watts,
            cycle_count,
            charge_limit,
        }
    }

    fn read_hardware_settings(&self) -> HardwareSettingsState {
        let bat_thresh_path = Self::find_battery_dir()
            .map(|d| d.join("charge_control_end_threshold"))
            .filter(|p| p.exists());
        let battery_charge_limit = bat_thresh_path
            .as_ref()
            .and_then(sysfs::read_u32)
            .map(|v| (v as u8).clamp(40, 100))
            .unwrap_or(80);

        let (fan_profile, fan_profile_supported, fan_profile_backend) = self.read_fan_profile();

        let kbd_path = Path::new(KBD_LED_DIR).join("brightness");
        let kbd_backlight_supported = kbd_path.exists();
        let kbd_backlight_level = sysfs::read_u32(&kbd_path).unwrap_or(0) as u8;
        let kbd_backlight_max =
            sysfs::read_u32(Path::new(KBD_LED_DIR).join("max_brightness")).unwrap_or(3) as u8;

        let fn_lock_path = Path::new(ASUS_WMI_DIR).join("fn_lock");
        let touchpad_path = Path::new(ASUS_WMI_DIR).join("touchpad");
        let panel_od_path = Path::new(ASUS_WMI_DIR).join("panel_od");
        let dgpu_path = Path::new(ASUS_WMI_DIR).join("dgpu_disable");

        let udev_writable = bat_thresh_path
            .as_ref()
            .map(sysfs::is_path_writable)
            .unwrap_or(false)
            || sysfs::is_path_writable(ACPI_PROFILE_PATH);

        let (splendid_mode, eye_care_level, volume_booster_enabled, oled_care) =
            self.visual_state.lock().unwrap().clone();

        HardwareSettingsState {
            capabilities: HardwareCapabilities {
                battery_threshold_supported: bat_thresh_path.is_some(),
                battery_threshold_path: bat_thresh_path.map(|p| p.display().to_string()),
                fan_profile_supported,
                fan_profile_backend,
                kbd_backlight_supported,
                kbd_backlight_max,
                fn_lock_supported: fn_lock_path.exists(),
                touchpad_toggle_supported: touchpad_path.exists(),
                panel_overdrive_supported: panel_od_path.exists(),
                dgpu_disable_supported: dgpu_path.exists(),
                udev_writable,
            },
            battery_charge_limit,
            fan_profile,
            kbd_backlight_level,
            fn_lock_enabled: sysfs::read_u32(&fn_lock_path).unwrap_or(0) == 1,
            touchpad_enabled: sysfs::read_u32(&touchpad_path).unwrap_or(1) == 1,
            panel_overdrive_enabled: sysfs::read_u32(&panel_od_path).unwrap_or(0) == 1,
            dgpu_disabled: sysfs::read_u32(&dgpu_path).unwrap_or(0) == 1,
            splendid_mode,
            color_gamut: ColorGamut::Native,
            eye_care_level,
            display_brightness_percent: self.read_display_brightness(),
            volume_booster_enabled,
            clearvoice_mic_enabled: true,
            clearvoice_speaker_enabled: false,
            wifi_smartconnect_enabled: true,
            taskfirst_mode: TaskFirstMode::Normal,
            oled_care,
        }
    }

    fn set_battery_charge_limit(&self, limit: u8) -> Result<String, String> {
        let clamped = limit.clamp(40, 100);
        let path = Self::find_battery_dir()
            .map(|d| d.join("charge_control_end_threshold"))
            .filter(|p| p.exists())
            .ok_or_else(|| {
                "charge_control_end_threshold not exposed (requires Linux kernel >= 5.4 with asus-nb-wmi)"
                    .to_string()
            })?;
        sysfs::write_sysfs_privileged(&path, &clamped.to_string())
    }

    fn set_fan_profile(&self, profile: FanProfile) -> Result<String, String> {
        let acpi_path = Path::new(ACPI_PROFILE_PATH);
        if acpi_path.exists() {
            return sysfs::write_sysfs_privileged(acpi_path, profile.acpi_name());
        }

        let policy_path = Path::new(ASUS_WMI_DIR).join("throttle_thermal_policy");
        if policy_path.exists() {
            return sysfs::write_sysfs_privileged(
                &policy_path,
                &profile.asus_policy_code().to_string(),
            );
        }

        let ppd_name = match profile {
            FanProfile::Whisper => "power-saver",
            FanProfile::Standard => "balanced",
            FanProfile::Performance => "performance",
        };
        let status = Command::new("powerprofilesctl")
            .args(["set", ppd_name])
            .status()
            .map_err(|e| format!("No ACPI platform_profile or powerprofilesctl available: {e}"))?;
        if status.success() {
            Ok(format!("Set power-profiles-daemon to {ppd_name}"))
        } else {
            Err(format!("powerprofilesctl failed setting {ppd_name}"))
        }
    }

    fn set_kbd_backlight(&self, level: u8) -> Result<String, String> {
        let path = Path::new(KBD_LED_DIR).join("brightness");
        if !path.exists() {
            return Err("ASUS keyboard backlight LED sysfs not detected".to_string());
        }
        sysfs::write_sysfs_privileged(&path, &level.to_string())
    }

    fn set_fn_lock(&self, enabled: bool) -> Result<String, String> {
        let path = Path::new(ASUS_WMI_DIR).join("fn_lock");
        if !path.exists() {
            return Err("asus-nb-wmi fn_lock attribute not exposed on this laptop".to_string());
        }
        sysfs::write_sysfs_privileged(&path, if enabled { "1" } else { "0" })
    }

    fn set_touchpad(&self, enabled: bool) -> Result<String, String> {
        let path = Path::new(ASUS_WMI_DIR).join("touchpad");
        if !path.exists() {
            return Err("asus-nb-wmi touchpad attribute not exposed".to_string());
        }
        sysfs::write_sysfs_privileged(&path, if enabled { "1" } else { "0" })
    }

    fn set_panel_overdrive(&self, enabled: bool) -> Result<String, String> {
        let path = Path::new(ASUS_WMI_DIR).join("panel_od");
        if !path.exists() {
            return Err("asus-nb-wmi panel_od attribute not exposed".to_string());
        }
        sysfs::write_sysfs_privileged(&path, if enabled { "1" } else { "0" })
    }

    fn set_dgpu_disabled(&self, disabled: bool) -> Result<String, String> {
        let path = Path::new(ASUS_WMI_DIR).join("dgpu_disable");
        if !path.exists() {
            return Err("asus-nb-wmi dgpu_disable attribute not exposed".to_string());
        }
        sysfs::write_sysfs_privileged(&path, if disabled { "1" } else { "0" })
    }

    fn set_splendid_mode(&self, mode: SplendidMode, eye_care_level: u8) -> Result<String, String> {
        {
            let mut g = self.visual_state.lock().unwrap();
            g.0 = mode;
            g.1 = eye_care_level;
        }
        let gamma = match mode {
            SplendidMode::Normal => "1.0:1.0:1.0".to_string(),
            SplendidMode::Vivid => "1.05:1.02:1.08".to_string(),
            SplendidMode::EyeCare => {
                let blue = (1.0 - (eye_care_level.clamp(1, 5) as f32 * 0.08)).clamp(0.55, 0.95);
                format!("1.0:0.96:{blue:.2}")
            }
            SplendidMode::Manual => {
                let blue = (1.05 - (eye_care_level.clamp(1, 5) as f32 * 0.06)).clamp(0.65, 1.05);
                format!("1.0:0.98:{blue:.2}")
            }
        };

        let Ok(out) = Command::new("xrandr").arg("--listmonitors").output() else {
            return Ok(format!(
                "Saved {} profile (xrandr unavailable on Wayland; use Cinnamon/GNOME Night Light)",
                mode.label()
            ));
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        let output_name = stdout
            .lines()
            .skip(1)
            .find_map(|line| line.split_whitespace().last())
            .unwrap_or("eDP-1");

        let status = Command::new("xrandr")
            .args(["--output", output_name, "--gamma", &gamma])
            .status();
        match status {
            Ok(s) if s.success() => Ok(format!(
                "Applied {} ({gamma}) on {output_name}",
                mode.label()
            )),
            _ => Ok(format!(
                "Profile set to {} (gamma control skipped on current display server)",
                mode.label()
            )),
        }
    }

    fn set_display_brightness(&self, percent: u8) -> Result<String, String> {
        let dir = Self::find_backlight_dir()
            .ok_or_else(|| "No /sys/class/backlight device detected".to_string())?;
        let max = sysfs::read_u32(dir.join("max_brightness")).unwrap_or(100);
        let raw = ((percent.clamp(5, 100) as f32 / 100.0) * max as f32).round() as u32;
        sysfs::write_sysfs_privileged(&dir.join("brightness"), &raw.to_string())
    }

    fn set_volume_booster(&self, enabled: bool) -> Result<String, String> {
        self.visual_state.lock().unwrap().2 = enabled;
        linux_oled::set_linux_volume_booster(enabled)
    }

    fn set_oled_pixel_refresh(&self, enabled: bool) -> Result<String, String> {
        self.visual_state.lock().unwrap().3.pixel_refresh_enabled = enabled;
        linux_oled::set_linux_pixel_refresh(enabled)
    }

    fn set_oled_taskbar_autohide(&self, enabled: bool) -> Result<String, String> {
        self.visual_state.lock().unwrap().3.auto_hide_taskbar = enabled;
        linux_oled::set_linux_taskbar_autohide(enabled)
    }

    fn set_oled_taskbar_transparency(&self, enabled: bool) -> Result<String, String> {
        self.visual_state.lock().unwrap().3.taskbar_transparency = enabled;
        linux_oled::set_linux_taskbar_transparency(enabled)
    }

    fn set_oled_flicker_free_dimming(&self, percent: u8) -> Result<String, String> {
        let clamped = percent.clamp(30, 100);
        self.visual_state.lock().unwrap().3.flicker_free_dimming = clamped;
        linux_oled::apply_linux_flicker_free_dimming(clamped)
    }

    fn read_cpu_governor(&self) -> String {
        linux_extra::read_linux_cpu_governor()
    }

    fn set_cpu_governor(&self, governor: &str) -> Result<String, String> {
        linux_extra::set_linux_cpu_governor(governor)
    }

    fn list_connected_devices(&self) -> Vec<ConnectedDevice> {
        linux_extra::list_linux_devices()
    }

    fn list_monitors(&self) -> Vec<MonitorInfo> {
        linux_extra::list_linux_monitors()
    }

    fn network_summary(&self) -> NetworkSummary {
        linux_extra::read_linux_network_details(self.wifi_signal_info())
    }

    fn check_updates(&self) -> UpdateReport {
        linux_updates::check_linux_updates()
    }

    fn wifi_signal_info(&self) -> (bool, Option<i32>, String) {
        if let Ok(content) = fs::read_to_string("/proc/net/wireless") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let iface = parts[0].trim_end_matches(':').to_string();
                    let level = parts[3].trim_end_matches('.').parse::<i32>().ok();
                    return (true, level, iface);
                }
            }
        }
        (false, None, "No active Wi-Fi interface".to_string())
    }

    fn bluetooth_present(&self) -> bool {
        fs::read_dir("/sys/class/bluetooth")
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    }

    fn boot_time_summary(&self) -> Option<String> {
        let out = Command::new("systemd-analyze").output().ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .map(|l| l.trim().to_string())
    }
}
