use std::fs;
use std::path::Path;
use std::process::Command;

use crate::models::peripherals::{ConnectedDevice, DeviceKind, MonitorInfo, NetworkSummary};
use crate::platform::sysfs;

pub fn list_linux_devices() -> Vec<ConnectedDevice> {
    let mut devices = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let p = entry.path();
            let Some(vendor_id) = sysfs::read_trimmed(p.join("idVendor")) else {
                continue;
            };
            let product_id = sysfs::read_trimmed(p.join("idProduct")).unwrap_or_default();
            let product = sysfs::read_trimmed(p.join("product"))
                .unwrap_or_else(|| format!("USB Device {vendor_id}:{product_id}"));
            let manufacturer =
                sysfs::read_trimmed(p.join("manufacturer")).unwrap_or_else(|| vendor_id.clone());
            let is_asus = vendor_id.eq_ignore_ascii_case("0b05")
                || manufacturer.to_ascii_uppercase().contains("ASUS")
                || product.to_ascii_uppercase().contains("ASUS");

            devices.push(ConnectedDevice {
                name: product,
                vendor: manufacturer,
                kind: DeviceKind::Usb,
                is_asus,
                status: "Connected".to_string(),
                identifier: format!("USB\\VID_{vendor_id}&PID_{product_id}"),
            });
        }
    }

    if let Ok(entries) = fs::read_dir("/sys/class/bluetooth") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            devices.push(ConnectedDevice {
                name: format!("Bluetooth Controller ({name})"),
                vendor: "Linux BlueZ".to_string(),
                kind: DeviceKind::Bluetooth,
                is_asus: false,
                status: "Active".to_string(),
                identifier: name,
            });
        }
    }

    devices.sort_by(|a, b| b.is_asus.cmp(&a.is_asus).then_with(|| a.name.cmp(&b.name)));
    devices
}

pub fn list_linux_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    let Ok(out) = Command::new("xrandr").arg("--current").output() else {
        return monitors;
    };
    if !out.status.success() {
        return monitors;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut current: Option<MonitorInfo> = None;

    for line in text.lines() {
        if line.contains(" connected") {
            if let Some(m) = current.take() {
                monitors.push(m);
            }
            let name = line.split_whitespace().next().unwrap_or("Display").to_string();
            let primary = line.contains(" primary ");
            current = Some(MonitorInfo {
                name,
                resolution: "Auto".to_string(),
                refresh_hz: None,
                primary,
            });
        } else if line.contains('*') {
            if let Some(m) = current.as_mut() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(res) = parts.first() {
                    m.resolution = (*res).to_string();
                }
                for tok in parts.iter().skip(1) {
                    if tok.contains('*') {
                        let cleaned = tok.replace(['*', '+'], "");
                        if let Ok(hz) = cleaned.parse::<f32>() {
                            m.refresh_hz = Some(hz.round() as u32);
                        }
                    }
                }
            }
        }
    }
    if let Some(m) = current {
        monitors.push(m);
    }
    monitors
}

pub fn read_linux_network_details(wifi_tuple: (bool, Option<i32>, String)) -> NetworkSummary {
    let (connected, signal_dbm, iface) = wifi_tuple;
    let mut summary = NetworkSummary {
        interface_name: iface,
        connected,
        signal_dbm,
        ..NetworkSummary::default()
    };

    if let Ok(out) = Command::new("iwgetid").arg("-r").output() {
        if out.status.success() {
            let ssid = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !ssid.is_empty() {
                summary.ssid = Some(ssid);
            }
        }
    }

    if let Ok(out) = Command::new("ip").args(["route", "show", "default"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = text.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pos) = parts.iter().position(|&p| p == "via") {
                summary.gateway_ip = parts.get(pos + 1).map(|s| (*s).to_string());
                summary.connected = true;
            }
            if let Some(pos) = parts.iter().position(|&p| p == "dev") {
                if let Some(dev) = parts.get(pos + 1) {
                    if summary.interface_name.contains("No active") {
                        summary.interface_name = (*dev).to_string();
                    }
                }
            }
        }
    }

    if let Ok(out) = Command::new("hostname").arg("-I").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        summary.local_ip = text.split_whitespace().next().map(String::from);
    }

    summary
}

pub fn read_linux_cpu_governor() -> String {
    sysfs::read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .unwrap_or_else(|| "schedutil".to_string())
}

pub fn set_linux_cpu_governor(governor: &str) -> Result<String, String> {
    let path = Path::new("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
    if !path.exists() {
        return Err("cpufreq scaling_governor sysfs not exposed on this kernel".to_string());
    }
    sysfs::write_sysfs_privileged(path, governor)
}
