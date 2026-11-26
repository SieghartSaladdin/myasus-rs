use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateSource {
    FirmwareLvfs,
    AptSystem,
    KernelDriver,
}

impl UpdateSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FirmwareLvfs => "BIOS / UEFI Firmware (LVFS)",
            Self::AptSystem => "Ubuntu / Mint Package (APT)",
            Self::KernelDriver => "Linux Kernel / asus-wmi",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItem {
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    pub source: UpdateSource,
    pub summary: String,
    pub critical: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateReport {
    pub last_checked: String,
    pub items: Vec<UpdateItem>,
    pub fwupd_available: bool,
    pub apt_available: bool,
    pub notes: Vec<String>,
}
