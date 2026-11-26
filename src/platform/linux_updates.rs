use std::process::Command;

use chrono::Utc;

use crate::models::updates::{UpdateItem, UpdateReport, UpdateSource};

pub fn check_linux_updates() -> UpdateReport {
    let mut report = UpdateReport {
        last_checked: Utc::now().format("%Y/%m/%d %H:%M UTC").to_string(),
        items: Vec::new(),
        fwupd_available: false,
        apt_available: false,
        notes: Vec::new(),
    };

    check_fwupd(&mut report);
    check_apt(&mut report);
    report
}

fn check_fwupd(report: &mut UpdateReport) {
    let Ok(output) = Command::new("fwupdmgr")
        .args(["get-updates", "--json"])
        .output()
    else {
        report
            .notes
            .push("fwupdmgr not installed (install `fwupd` for ASUS UEFI/BIOS LVFS updates).".into());
        return;
    };

    report.fwupd_available = true;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(devices) = json.get("Devices").and_then(|d| d.as_array()) {
            for dev in devices {
                let name = dev
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Firmware Device")
                    .to_string();
                let current = dev
                    .get("Version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(releases) = dev.get("Releases").and_then(|r| r.as_array()) {
                    if let Some(rel) = releases.first() {
                        let available = rel
                            .get("Version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("newer")
                            .to_string();
                        let summary = rel
                            .get("Summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Firmware capsule update via LVFS")
                            .to_string();
                        report.items.push(UpdateItem {
                            name,
                            current_version: current,
                            available_version: available,
                            source: UpdateSource::FirmwareLvfs,
                            summary,
                            critical: true,
                        });
                    }
                }
            }
        }
    }
}

fn check_apt(report: &mut UpdateReport) {
    let Ok(output) = Command::new("apt").args(["list", "--upgradable"]).output() else {
        report
            .notes
            .push("apt not detected (non-Debian/Ubuntu/Mint environment).".into());
        return;
    };

    report.apt_available = true;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1).take(25) {
        if let Some(item) = parse_apt_upgradable_line(line) {
            report.items.push(item);
        }
    }
}

pub fn parse_apt_upgradable_line(line: &str) -> Option<UpdateItem> {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.contains('/') {
        return None;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let name = parts[0].split('/').next()?.to_string();
    let available = parts[1].to_string();
    let current = trimmed
        .split("[upgradable from: ")
        .nth(1)
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or("installed")
        .to_string();

    let is_kernel = name.starts_with("linux-image")
        || name.starts_with("linux-firmware")
        || name.contains("asus");

    Some(UpdateItem {
        name,
        current_version: current,
        available_version: available,
        source: if is_kernel {
            UpdateSource::KernelDriver
        } else {
            UpdateSource::AptSystem
        },
        summary: if is_kernel {
            "Kernel or firmware driver package".to_string()
        } else {
            "System package update available via APT".to_string()
        },
        critical: is_kernel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ubuntu_mint_apt_upgradable_line() {
        let line = "linux-firmware/noble-updates 20240318.git3b128b60-0ubuntu2.9 amd64 [upgradable from: 20240318.git3b128b60-0ubuntu2.8]";
        let item = parse_apt_upgradable_line(line).unwrap();
        assert_eq!(item.name, "linux-firmware");
        assert_eq!(item.source, UpdateSource::KernelDriver);
        assert!(item.critical);
        assert_eq!(item.current_version, "20240318.git3b128b60-0ubuntu2.8");
    }
}
