use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub product_name: String,
    pub product_family: String,
    pub serial_number: String,
    pub board_name: String,
    pub bios_version: String,
    pub bios_date: String,
    pub os_name: String,
    pub kernel_version: String,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub total_memory_gb: f64,
    pub asus_wmi_loaded: bool,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            product_name: "ASUS Laptop".to_string(),
            product_family: "Vivobook".to_string(),
            serial_number: "Unavailable".to_string(),
            board_name: "Unknown".to_string(),
            bios_version: "Unknown".to_string(),
            bios_date: "Unknown".to_string(),
            os_name: std::env::consts::OS.to_string(),
            kernel_version: "Unknown".to_string(),
            cpu_brand: "Unknown CPU".to_string(),
            cpu_cores: 0,
            total_memory_gb: 0.0,
            asus_wmi_loaded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub mount_point: String,
    pub name: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FanMetrics {
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
    pub extra_fans: Vec<(String, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryMetrics {
    pub present: bool,
    pub percentage: u8,
    pub status: String,
    pub ac_online: bool,
    pub health_percent: Option<f32>,
    pub power_watts: Option<f32>,
    pub cycle_count: Option<u32>,
    pub charge_limit: Option<u8>,
}

impl Default for BatteryMetrics {
    fn default() -> Self {
        Self {
            present: false,
            percentage: 0,
            status: "Unknown".to_string(),
            ac_online: true,
            health_percent: None,
            power_watts: None,
            cycle_count: None,
            charge_limit: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub cpu_usage_percent: f32,
    pub cpu_temp_celsius: Option<f32>,
    pub gpu_temp_celsius: Option<f32>,
    pub memory_usage_percent: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub swap_usage_percent: f32,
    pub disks: Vec<DiskMetrics>,
    pub fans: FanMetrics,
    pub battery: BatteryMetrics,
    pub uptime_seconds: u64,
}
