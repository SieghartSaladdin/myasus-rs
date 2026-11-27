//! Persistent OLED Care & Display preferences for Linux (and cross-platform fallback).
//!
//! Stored at `~/.config/myasus-rs/display.json` and automatically restored/maintained at login
//! by `myasus-rs oled-daemon` (or one-shot `myasus-rs restore-display`).

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::models::hardware::SplendidMode;

pub const DIMMING_MIN: u8 = 30;
pub const DIMMING_MAX: u8 = 100;
pub const PWM_FLOOR_MIN: u8 = 40;
pub const PWM_FLOOR_MAX: u8 = 100;
pub const DEFAULT_PWM_FLOOR: u8 = 60;
pub const UNIFORM_CAP_MIN: u8 = 55;
pub const UNIFORM_CAP_MAX: u8 = 95;
pub const DEFAULT_UNIFORM_CAP: u8 = 75;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PixelShiftStrength {
    Subtle, // ±2 px
    #[default]
    Standard, // ±4 px
    Strong, // ±6 px
}

impl PixelShiftStrength {
    pub fn pixels(self) -> i32 {
        match self {
            Self::Subtle => 2,
            Self::Standard => 4,
            Self::Strong => 6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Subtle => "Subtle (±2px)",
            Self::Standard => "Standard (±4px)",
            Self::Strong => "Strong (±6px)",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Subtle => 0,
            Self::Standard => 1,
            Self::Strong => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Self::Subtle,
            2 => Self::Strong,
            _ => Self::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScreensaverStyle {
    #[default]
    AsusOrbit,
    PixelRefresh,
    MinimalClock,
}

impl ScreensaverStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::AsusOrbit => "ASUS Orbit",
            Self::PixelRefresh => "Pixel Refresh Sweep",
            Self::MinimalClock => "Minimal Drift Clock",
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::AsusOrbit => "orbit",
            Self::PixelRefresh => "refresh",
            Self::MinimalClock => "clock",
        }
    }

    pub fn from_str_opt(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "refresh" | "pixel-refresh" | "sweep" => Self::PixelRefresh,
            "clock" | "minimal" => Self::MinimalClock,
            _ => Self::AsusOrbit,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::AsusOrbit => 0,
            Self::PixelRefresh => 1,
            Self::MinimalClock => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::PixelRefresh,
            2 => Self::MinimalClock,
            _ => Self::AsusOrbit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayPrefs {
    // Flicker-Free Dimming + PWM-Safe Hardware Lock
    pub flicker_free_dimming: u8,
    pub pwm_safe_lock: bool,
    pub pwm_safe_floor: u8,
    // Uniform Brightness (Anti-ABL ceiling)
    pub uniform_brightness_enabled: bool,
    pub uniform_brightness_cap: u8,
    // Splendid RGB gamma (atomically composed with dimming)
    pub splendid_mode: SplendidMode,
    pub eye_care_level: u8,
    // Pixel Shift (real xrandr integer translation orbit)
    pub pixel_shift_enabled: bool,
    pub pixel_shift_strength: PixelShiftStrength,
    pub pixel_shift_interval_mins: u32,
    // OLED Screensaver & Away / Idle Auto-Dim
    pub screensaver_style: ScreensaverStyle,
    pub screensaver_idle_mins: u32,
    pub away_dim_enabled: bool,
    pub away_dim_secs: u32,
    pub away_dim_level: u8,
    // Display Auto-Off (DPMS + DE sleep-display)
    pub screen_off_mins: u32,
}

impl Default for DisplayPrefs {
    fn default() -> Self {
        Self {
            flicker_free_dimming: DIMMING_MAX,
            pwm_safe_lock: true,
            pwm_safe_floor: DEFAULT_PWM_FLOOR,
            uniform_brightness_enabled: false,
            uniform_brightness_cap: DEFAULT_UNIFORM_CAP,
            splendid_mode: SplendidMode::Normal,
            eye_care_level: 2,
            pixel_shift_enabled: true,
            pixel_shift_strength: PixelShiftStrength::Standard,
            pixel_shift_interval_mins: 5,
            screensaver_style: ScreensaverStyle::AsusOrbit,
            screensaver_idle_mins: 15,
            away_dim_enabled: true,
            away_dim_secs: 90,
            away_dim_level: 35,
            screen_off_mins: 10,
        }
    }
}

impl DisplayPrefs {
    pub fn path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("org", "myasus-rs", "myasus-rs")?;
        Some(dirs.config_dir().join("display.json"))
    }

    pub fn load() -> Self {
        let mut prefs: Self = Self::path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        prefs.clamp_in_place();
        prefs
    }

    pub fn clamp_in_place(&mut self) {
        self.flicker_free_dimming = self.flicker_free_dimming.clamp(DIMMING_MIN, DIMMING_MAX);
        self.pwm_safe_floor = self.pwm_safe_floor.clamp(PWM_FLOOR_MIN, PWM_FLOOR_MAX);
        self.uniform_brightness_cap = self
            .uniform_brightness_cap
            .clamp(UNIFORM_CAP_MIN, UNIFORM_CAP_MAX);
        self.eye_care_level = self.eye_care_level.clamp(1, 5);
        self.pixel_shift_interval_mins = self.pixel_shift_interval_mins.clamp(1, 60);
        self.away_dim_secs = self.away_dim_secs.clamp(15, 600);
        self.away_dim_level = self.away_dim_level.clamp(20, 70);
    }

    pub fn save(&self) -> Result<(), String> {
        let mut clamped = self.clone();
        clamped.clamp_in_place();
        let path = Self::path()
            .ok_or_else(|| "Unable to determine XDG config directory".to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed creating config dir: {e}"))?;
        }
        let body = serde_json::to_string_pretty(&clamped)
            .map_err(|e| format!("Failed serializing display prefs: {e}"))?;
        fs::write(path, body).map_err(|e| format!("Failed writing display prefs: {e}"))
    }

    /// Update and persist only the flicker-free dimming level while preserving all other prefs.
    pub fn save_dimming(percent: u8) -> Result<Self, String> {
        let mut prefs = Self::load();
        prefs.flicker_free_dimming = percent.clamp(DIMMING_MIN, DIMMING_MAX);
        prefs.save()?;
        Ok(prefs)
    }

    /// Update and persist PWM-safe lock toggle and floor percent.
    pub fn save_pwm_lock(enabled: bool, floor: u8) -> Result<Self, String> {
        let mut prefs = Self::load();
        prefs.pwm_safe_lock = enabled;
        prefs.pwm_safe_floor = floor.clamp(PWM_FLOOR_MIN, PWM_FLOOR_MAX);
        prefs.save()?;
        Ok(prefs)
    }

    /// Computes `(hardware_backlight_pct, software_dimming_pct)` for a requested screen brightness.
    ///
    /// - If `uniform_brightness_enabled`, caps requested hardware brightness to `uniform_brightness_cap`
    ///   (anti-ABL peak limiter).
    /// - If `pwm_safe_lock` is enabled and `target < pwm_safe_floor`, holds hardware backlight at
    ///   `pwm_safe_floor` (above OLED PWM threshold) and shifts extra dimming into CRTC gamma.
    pub fn split_brightness(&self, requested: u8) -> (u8, u8) {
        let mut req = requested.clamp(5, 100);
        if self.uniform_brightness_enabled {
            req = req.min(self.uniform_brightness_cap);
        }
        if !self.pwm_safe_lock || req >= self.pwm_safe_floor {
            (req, self.flicker_free_dimming)
        } else {
            let floor = self.pwm_safe_floor.max(PWM_FLOOR_MIN);
            let ratio = (req as f32 / floor as f32) * 100.0;
            let sw = (ratio.round() as u8).clamp(DIMMING_MIN, DIMMING_MAX);
            (floor, sw)
        }
    }

    /// Computes `(r, g, b)` CRTC gamma multipliers for the configured Splendid profile.
    pub fn splendid_gamma_rgb(&self) -> (f32, f32, f32) {
        match self.splendid_mode {
            SplendidMode::Normal => (1.00, 1.00, 1.00),
            SplendidMode::Vivid => (1.05, 1.02, 1.08),
            SplendidMode::EyeCare => {
                let blue =
                    (1.0 - (self.eye_care_level.clamp(1, 5) as f32 * 0.08)).clamp(0.55, 0.95);
                (1.00, 0.96, blue)
            }
            SplendidMode::Manual => {
                let blue =
                    (1.05 - (self.eye_care_level.clamp(1, 5) as f32 * 0.06)).clamp(0.65, 1.05);
                (1.05, 0.98, blue)
            }
        }
    }

    pub fn splendid_gamma_string(&self) -> String {
        let (r, g, b) = self.splendid_gamma_rgb();
        format!("{r:.2}:{g:.2}:{b:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_low_brightness_into_pwm_safe_floor_and_gamma_dimming() {
        let prefs = DisplayPrefs {
            flicker_free_dimming: 100,
            pwm_safe_lock: true,
            pwm_safe_floor: 60,
            ..Default::default()
        };
        assert_eq!(prefs.split_brightness(30), (60, 50));
        assert_eq!(prefs.split_brightness(42), (60, 70));
        assert_eq!(prefs.split_brightness(80), (80, 100));
    }

    #[test]
    fn uniform_brightness_caps_peak_hardware_luminance() {
        let prefs = DisplayPrefs {
            flicker_free_dimming: 100,
            pwm_safe_lock: true,
            pwm_safe_floor: 60,
            uniform_brightness_enabled: true,
            uniform_brightness_cap: 75,
            ..Default::default()
        };
        assert_eq!(prefs.split_brightness(100), (75, 100));
        assert_eq!(prefs.split_brightness(30), (60, 50));
    }

    #[test]
    fn bypasses_split_when_pwm_lock_disabled() {
        let prefs = DisplayPrefs {
            flicker_free_dimming: 70,
            pwm_safe_lock: false,
            pwm_safe_floor: 60,
            ..Default::default()
        };
        assert_eq!(prefs.split_brightness(30), (30, 70));
    }
}
