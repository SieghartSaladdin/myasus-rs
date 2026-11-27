#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryChargeMode {
    FullCapacity,
    Balanced,
    MaxLifespan,
    Custom(u8),
}

impl BatteryChargeMode {
    pub fn threshold(&self) -> u8 {
        match self {
            Self::FullCapacity => 100,
            Self::Balanced => 80,
            Self::MaxLifespan => 60,
            Self::Custom(v) => (*v).clamp(40, 100),
        }
    }

    #[allow(dead_code)]
    pub fn from_threshold(v: u8) -> Self {
        match v {
            100 => Self::FullCapacity,
            80 => Self::Balanced,
            60 => Self::MaxLifespan,
            other => Self::Custom(other.clamp(40, 100)),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::FullCapacity => "Full Capacity (100%)",
            Self::Balanced => "Balanced Mode (80%)",
            Self::MaxLifespan => "Maximum Lifespan (60%)",
            Self::Custom(_) => "Custom Limit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanProfile {
    Whisper,
    Standard,
    Performance,
}

impl FanProfile {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Whisper => "Whisper Mode",
            Self::Standard => "Standard Mode",
            Self::Performance => "Performance Mode",
        }
    }

    #[allow(dead_code)]
    pub fn acpi_name(&self) -> &'static str {
        match self {
            Self::Whisper => "quiet",
            Self::Standard => "balanced",
            Self::Performance => "performance",
        }
    }

    pub fn from_acpi_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "quiet" | "low-power" | "silent" | "power-saver" | "whisper" => Some(Self::Whisper),
            "balanced" | "standard" => Some(Self::Standard),
            "performance" | "overboost" | "turbo" => Some(Self::Performance),
            _ => None,
        }
    }

    pub fn asus_policy_code(&self) -> u8 {
        match self {
            Self::Standard => 0,
            Self::Performance => 1,
            Self::Whisper => 2,
        }
    }

    pub fn from_asus_policy_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Standard),
            1 => Some(Self::Performance),
            2 => Some(Self::Whisper),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplendidMode {
    Normal,
    Vivid,
    EyeCare,
    Manual,
}

impl SplendidMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Vivid => "Vivid",
            Self::EyeCare => "Eye Care",
            Self::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorGamut {
    Native,
    SRgb,
    DciP3,
    DisplayP3,
}

impl ColorGamut {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::SRgb => "sRGB",
            Self::DciP3 => "DCI-P3",
            Self::DisplayP3 => "Display P3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskFirstMode {
    Normal,
    Productivity,
    Gaming,
    Streaming,
}

impl TaskFirstMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Productivity => "Productivity",
            Self::Gaming => "Gaming",
            Self::Streaming => "Media & Streaming",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OledCareState {
    pub pixel_refresh_enabled: bool,
    pub pixel_shift_enabled: bool,
    pub auto_hide_taskbar: bool,
    pub taskbar_transparency: bool,
    pub flicker_free_dimming: u8,
    pub pwm_safe_lock: bool,
    pub pwm_safe_floor: u8,
    pub target_mode_enabled: bool,
}

impl Default for OledCareState {
    fn default() -> Self {
        Self {
            pixel_refresh_enabled: true,
            pixel_shift_enabled: true,
            auto_hide_taskbar: false,
            taskbar_transparency: false,
            flicker_free_dimming: 100,
            pwm_safe_lock: true,
            pwm_safe_floor: 60,
            target_mode_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub battery_threshold_supported: bool,
    pub battery_threshold_path: Option<String>,
    pub fan_profile_supported: bool,
    pub fan_profile_backend: String,
    pub kbd_backlight_supported: bool,
    pub kbd_backlight_max: u8,
    pub fn_lock_supported: bool,
    pub touchpad_toggle_supported: bool,
    pub panel_overdrive_supported: bool,
    pub dgpu_disable_supported: bool,
    pub udev_writable: bool,
}

impl Default for HardwareCapabilities {
    fn default() -> Self {
        Self {
            battery_threshold_supported: false,
            battery_threshold_path: None,
            fan_profile_supported: false,
            fan_profile_backend: "Unavailable".to_string(),
            kbd_backlight_supported: false,
            kbd_backlight_max: 3,
            fn_lock_supported: false,
            touchpad_toggle_supported: false,
            panel_overdrive_supported: false,
            dgpu_disable_supported: false,
            udev_writable: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSettingsState {
    pub capabilities: HardwareCapabilities,
    pub battery_charge_limit: u8,
    pub fan_profile: FanProfile,
    pub kbd_backlight_level: u8,
    pub fn_lock_enabled: bool,
    pub touchpad_enabled: bool,
    pub panel_overdrive_enabled: bool,
    pub dgpu_disabled: bool,
    pub splendid_mode: SplendidMode,
    pub color_gamut: ColorGamut,
    pub eye_care_level: u8,
    pub display_brightness_percent: u8,
    pub volume_booster_enabled: bool,
    pub clearvoice_mic_enabled: bool,
    pub clearvoice_speaker_enabled: bool,
    pub wifi_smartconnect_enabled: bool,
    pub taskfirst_mode: TaskFirstMode,
    pub oled_care: OledCareState,
}

impl Default for HardwareSettingsState {
    fn default() -> Self {
        Self {
            capabilities: HardwareCapabilities::default(),
            battery_charge_limit: 80,
            fan_profile: FanProfile::Standard,
            kbd_backlight_level: 1,
            fn_lock_enabled: false,
            touchpad_enabled: true,
            panel_overdrive_enabled: false,
            dgpu_disabled: false,
            splendid_mode: SplendidMode::Normal,
            color_gamut: ColorGamut::Native,
            eye_care_level: 2,
            display_brightness_percent: 75,
            volume_booster_enabled: false,
            clearvoice_mic_enabled: true,
            clearvoice_speaker_enabled: false,
            wifi_smartconnect_enabled: true,
            taskfirst_mode: TaskFirstMode::Normal,
            oled_care: OledCareState::default(),
        }
    }
}
