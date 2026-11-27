#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Usb,
    Bluetooth,
    Display,
    Audio,
    Input,
}

impl DeviceKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Usb => "USB",
            Self::Bluetooth => "Bluetooth",
            Self::Display => "Display",
            Self::Audio => "Audio",
            Self::Input => "Input",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedDevice {
    pub name: String,
    pub vendor: String,
    pub kind: DeviceKind,
    pub is_asus: bool,
    pub status: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub name: String,
    pub resolution: String,
    pub refresh_hz: Option<u32>,
    pub primary: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub interface_name: String,
    pub ssid: Option<String>,
    pub connected: bool,
    pub signal_dbm: Option<i32>,
    pub local_ip: Option<String>,
    pub gateway_ip: Option<String>,
    pub rx_kbps: f64,
    pub tx_kbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Info,
    Success,
    Warning,
    Alert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub timestamp: String,
    pub title: String,
    pub detail: String,
    pub severity: EventSeverity,
    pub read: bool,
}
