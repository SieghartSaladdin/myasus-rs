use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    BatteryAndPower,
    ThermalAndFan,
    Memory,
    Storage,
    NetworkAndBluetooth,
    DriverAndSystem,
}

impl DiagnosticCategory {
    pub fn title(&self) -> &'static str {
        match self {
            Self::BatteryAndPower => "Battery & Power Adapter",
            Self::ThermalAndFan => "Thermal & Cooling Fans",
            Self::Memory => "System Memory (RAM)",
            Self::Storage => "Storage & SSD Health",
            Self::NetworkAndBluetooth => "Wi-Fi & Bluetooth",
            Self::DriverAndSystem => "ASUS Kernel Driver & Boot",
        }
    }

    pub fn all() -> &'static [DiagnosticCategory] {
        &[
            Self::BatteryAndPower,
            Self::ThermalAndFan,
            Self::Memory,
            Self::Storage,
            Self::NetworkAndBluetooth,
            Self::DriverAndSystem,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Passed,
    Warning,
    Issue,
    Skipped,
}

impl DiagnosticStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Passed => "Healthy",
            Self::Warning => "Attention",
            Self::Issue => "Issue Found",
            Self::Skipped => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub category: DiagnosticCategory,
    pub status: DiagnosticStatus,
    pub summary: String,
    pub details: Vec<String>,
    pub recommendation: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosisReport {
    pub timestamp: String,
    pub items: Vec<DiagnosticItem>,
}

impl DiagnosisReport {
    pub fn count_by_status(&self, status: DiagnosticStatus) -> usize {
        self.items.iter().filter(|item| item.status == status).count()
    }
}
