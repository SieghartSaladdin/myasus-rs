use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub refresh_interval_secs: u64,
    pub use_fahrenheit: bool,
    pub show_quick_settings_on_start: bool,
    pub confirm_critical_changes: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 2,
            use_fahrenheit: false,
            show_quick_settings_on_start: false,
            confirm_critical_changes: false,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("org", "myasus-rs", "myasus-rs")?;
        Some(dirs.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()
            .ok_or_else(|| "Unable to determine XDG config directory".to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed creating config dir: {e}"))?;
        }
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed serializing config: {e}"))?;
        fs::write(path, body).map_err(|e| format!("Failed writing config: {e}"))
    }

    pub fn format_temp(&self, celsius: f32) -> String {
        if self.use_fahrenheit {
            format!("{:.0}°F", celsius * 9.0 / 5.0 + 32.0)
        } else {
            format!("{:.0}°C", celsius)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_celsius_and_fahrenheit() {
        let mut cfg = AppConfig::default();
        assert_eq!(cfg.format_temp(93.0), "93°C");
        cfg.use_fahrenheit = true;
        assert_eq!(cfg.format_temp(100.0), "212°F");
    }
}
