use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use directories::UserDirs;
use serde::Serialize;

use crate::models::{
    diagnosis::DiagnosisReport,
    hardware::HardwareSettingsState,
    peripherals::{ConnectedDevice, MonitorInfo, NetworkSummary},
    system::{DeviceInfo, TelemetrySnapshot},
    updates::UpdateReport,
};

#[derive(Serialize)]
pub struct SupportBundle<'a> {
    pub generated_at: String,
    pub app_version: &'static str,
    pub backend: &'a str,
    pub device: &'a DeviceInfo,
    pub telemetry: &'a TelemetrySnapshot,
    pub hardware: &'a HardwareSettingsState,
    pub diagnosis: &'a DiagnosisReport,
    pub updates: &'a UpdateReport,
    pub network: &'a NetworkSummary,
    pub monitors: &'a [MonitorInfo],
    pub connected_devices: &'a [ConnectedDevice],
}

pub fn default_export_path() -> PathBuf {
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = format!("myasus-rs-support-report_{ts}.json");
    if let Some(dirs) = UserDirs::new() {
        if let Some(desktop) = dirs.desktop_dir() {
            return desktop.join(&file_name);
        }
        return dirs.home_dir().join(&file_name);
    }
    std::env::temp_dir().join(file_name)
}

pub fn export_support_bundle(
    target_path: Option<PathBuf>,
    bundle: &SupportBundle<'_>,
) -> Result<PathBuf, String> {
    let path = target_path.unwrap_or_else(default_export_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed creating output directory: {e}"))?;
        }
    }
    let json = serde_json::to_string_pretty(bundle)
        .map_err(|e| format!("Failed serializing support report: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed writing report to {}: {e}", path.display()))?;
    Ok(path)
}
