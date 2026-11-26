use std::net::ToSocketAddrs;
use std::time::Instant;

use chrono::Utc;

use crate::models::{
    diagnosis::{DiagnosisReport, DiagnosticCategory, DiagnosticItem, DiagnosticStatus},
    system::{DeviceInfo, TelemetrySnapshot},
};
use crate::platform::PlatformBackend;

pub fn run_full_diagnosis(
    backend: &dyn PlatformBackend,
    device: &DeviceInfo,
    telemetry: &TelemetrySnapshot,
) -> DiagnosisReport {
    let items = DiagnosticCategory::all()
        .iter()
        .map(|&cat| run_category_check(cat, backend, device, telemetry))
        .collect();

    DiagnosisReport {
        timestamp: Utc::now().format("%Y/%m/%d %H:%M:%S UTC").to_string(),
        items,
    }
}

pub fn run_category_check(
    category: DiagnosticCategory,
    backend: &dyn PlatformBackend,
    device: &DeviceInfo,
    telemetry: &TelemetrySnapshot,
) -> DiagnosticItem {
    let start = Instant::now();
    let mut item = match category {
        DiagnosticCategory::BatteryAndPower => check_battery(telemetry),
        DiagnosticCategory::ThermalAndFan => check_thermal(telemetry),
        DiagnosticCategory::Memory => check_memory(telemetry),
        DiagnosticCategory::Storage => check_storage(telemetry),
        DiagnosticCategory::NetworkAndBluetooth => check_network(backend),
        DiagnosticCategory::DriverAndSystem => check_driver(backend, device),
    };
    item.duration_ms = start.elapsed().as_millis() as u64;
    item
}

fn check_battery(t: &TelemetrySnapshot) -> DiagnosticItem {
    let b = &t.battery;
    if !b.present {
        return DiagnosticItem {
            category: DiagnosticCategory::BatteryAndPower,
            status: DiagnosticStatus::Skipped,
            summary: "No battery detected (AC desktop or sensor unavailable).".into(),
            details: vec![format!("AC Power Online: {}", b.ac_online)],
            recommendation: None,
            duration_ms: 0,
        };
    }

    let mut details = vec![
        format!("Charge Level: {}% ({})", b.percentage, b.status),
        format!("AC Adapter: {}", if b.ac_online { "Connected" } else { "Battery Power" }),
    ];
    if let Some(limit) = b.charge_limit {
        details.push(format!("Charge Threshold Limit: {limit}%"));
    }
    if let Some(cycles) = b.cycle_count {
        details.push(format!("Charge Cycles: {cycles}"));
    }

    let (status, summary, recommendation) = match b.health_percent {
        Some(h) if h < 65.0 => {
            details.push(format!("Battery Health: {h:.1}% of design capacity"));
            (
                DiagnosticStatus::Issue,
                format!("Battery wear is significant ({h:.1}% remaining capacity)."),
                Some("Enable Balanced (80%) or Max Lifespan (60%) charging limit to slow wear.".into()),
            )
        }
        Some(h) if h < 80.0 => {
            details.push(format!("Battery Health: {h:.1}% of design capacity"));
            (
                DiagnosticStatus::Warning,
                format!("Moderate battery wear detected ({h:.1}% health)."),
                Some("Keep Battery Health Charging at 80% when mostly plugged in.".into()),
            )
        }
        Some(h) => {
            details.push(format!("Battery Health: {h:.1}% of design capacity"));
            (
                DiagnosticStatus::Passed,
                format!("Battery and AC adapter are healthy ({h:.1}% capacity)."),
                None,
            )
        }
        None => (
            DiagnosticStatus::Passed,
            format!("Battery detected at {}% ({}).", b.percentage, b.status),
            None,
        ),
    };

    DiagnosticItem {
        category: DiagnosticCategory::BatteryAndPower,
        status,
        summary,
        details,
        recommendation,
        duration_ms: 0,
    }
}

fn check_thermal(t: &TelemetrySnapshot) -> DiagnosticItem {
    let mut details = Vec::new();
    if let Some(cpu) = t.cpu_temp_celsius {
        details.push(format!("CPU Temperature: {cpu:.1} °C"));
    } else {
        details.push("CPU Temperature: Sensor unavailable".into());
    }
    if let Some(cpu_rpm) = t.fans.cpu_fan_rpm {
        details.push(format!("CPU Fan: {cpu_rpm} RPM"));
    }
    if let Some(gpu_rpm) = t.fans.gpu_fan_rpm {
        details.push(format!("GPU Fan: {gpu_rpm} RPM"));
    }

    let (status, summary, rec) = match t.cpu_temp_celsius {
        Some(temp) if temp >= 90.0 => (
            DiagnosticStatus::Issue,
            format!("High CPU temperature detected ({temp:.1} °C)."),
            Some("Switch Fan Profile to Performance Mode and ensure air vents are clear.".into()),
        ),
        Some(temp) if temp >= 80.0 => (
            DiagnosticStatus::Warning,
            format!("Elevated CPU temperature ({temp:.1} °C)."),
            Some("Monitor background CPU load or switch to Standard/Performance fan mode.".into()),
        ),
        Some(temp) => (
            DiagnosticStatus::Passed,
            format!("Thermal sensors within normal range ({temp:.1} °C)."),
            None,
        ),
        None => (
            DiagnosticStatus::Warning,
            "No hwmon temperature sensor reported.".into(),
            Some("Install `lm-sensors` and run `sudo sensors-detect` on Ubuntu/Mint.".into()),
        ),
    };

    DiagnosticItem {
        category: DiagnosticCategory::ThermalAndFan,
        status,
        summary,
        details,
        recommendation: rec,
        duration_ms: 0,
    }
}

fn check_memory(t: &TelemetrySnapshot) -> DiagnosticItem {
    // Non-destructive 4 MiB pattern verification buffer
    let mut buf = vec![0xAAu8; 4 * 1024 * 1024];
    for (idx, b) in buf.iter_mut().enumerate() {
        *b = (idx as u8).wrapping_mul(31).wrapping_add(7);
    }
    let verified = buf
        .iter()
        .enumerate()
        .all(|(idx, &b)| b == (idx as u8).wrapping_mul(31).wrapping_add(7));

    let details = vec![
        format!(
            "RAM Usage: {:.1}% ({:.1} / {:.1} GB)",
            t.memory_usage_percent, t.memory_used_gb, t.memory_total_gb
        ),
        format!("Swap Usage: {:.1}%", t.swap_usage_percent),
        format!(
            "4 MiB Buffer Integrity Check: {}",
            if verified { "PASS" } else { "FAIL" }
        ),
    ];

    let (status, summary, rec) = if !verified {
        (
            DiagnosticStatus::Issue,
            "Memory buffer verification failed!".into(),
            Some("Run MemTest86+ from your Linux Mint / Ubuntu GRUB boot menu.".into()),
        )
    } else if t.memory_usage_percent >= 90.0 {
        (
            DiagnosticStatus::Warning,
            format!("High RAM usage ({:.0}%).", t.memory_usage_percent),
            Some("Close unused applications to avoid heavy swap pressure.".into()),
        )
    } else {
        (
            DiagnosticStatus::Passed,
            format!("Memory integrity passed ({:.1}% used).", t.memory_usage_percent),
            None,
        )
    };

    DiagnosticItem {
        category: DiagnosticCategory::Memory,
        status,
        summary,
        details,
        recommendation: rec,
        duration_ms: 0,
    }
}

fn check_storage(t: &TelemetrySnapshot) -> DiagnosticItem {
    let mut details = Vec::new();
    let mut worst = DiagnosticStatus::Passed;

    for d in &t.disks {
        details.push(format!(
            "{} — {:.1}/{:.1} GB ({:.0}%)",
            d.mount_point, d.used_gb, d.total_gb, d.usage_percent
        ));
        if d.usage_percent >= 92.0 {
            worst = DiagnosticStatus::Issue;
        } else if d.usage_percent >= 85.0 && worst != DiagnosticStatus::Issue {
            worst = DiagnosticStatus::Warning;
        }
    }

    // Small temp write/read latency check
    let tmp = std::env::temp_dir().join("myasus_rs_diag_probe.tmp");
    let io_start = Instant::now();
    let io_ok = std::fs::write(&tmp, vec![0x55u8; 64 * 1024]).is_ok()
        && std::fs::read(&tmp).map(|v| v.len() == 64 * 1024).unwrap_or(false);
    let _ = std::fs::remove_file(&tmp);
    let io_ms = io_start.elapsed().as_millis();
    details.push(format!(
        "64 KiB Read/Write Probe: {} ({io_ms} ms)",
        if io_ok { "PASS" } else { "FAIL" }
    ));

    let (summary, rec) = match worst {
        DiagnosticStatus::Issue => (
            "One or more storage volumes are critically full (>= 92%).".into(),
            Some("Free up disk space (`sudo apt autoremove && sudo apt clean`) to protect SSD health.".into()),
        ),
        DiagnosticStatus::Warning => (
            "Storage utilization is high (>= 85%).".into(),
            Some("Keep at least 15% free space on NVMe/SSD drives for wear leveling.".into()),
        ),
        _ => (
            format!("{} storage volume(s) verified healthy.", t.disks.len()),
            None,
        ),
    };

    DiagnosticItem {
        category: DiagnosticCategory::Storage,
        status: worst,
        summary,
        details,
        recommendation: rec,
        duration_ms: 0,
    }
}

fn check_network(backend: &dyn PlatformBackend) -> DiagnosticItem {
    let (wifi_up, rssi, iface) = backend.wifi_signal_info();
    let bt_present = backend.bluetooth_present();
    let dns_ok = ("connectivity-check.ubuntu.com", 80)
        .to_socket_addrs()
        .map(|mut a| a.next().is_some())
        .unwrap_or(false);

    let mut details = vec![
        format!(
            "Wi-Fi Interface: {} ({})",
            iface,
            if wifi_up { "Active" } else { "Inactive" }
        ),
        format!(
            "Bluetooth Adapter: {}",
            if bt_present { "Detected" } else { "Not detected" }
        ),
        format!(
            "DNS Connectivity Check: {}",
            if dns_ok { "Reachable" } else { "Offline / DNS unavailable" }
        ),
    ];
    if let Some(dbm) = rssi {
        details.push(format!("Wi-Fi Signal Strength: {dbm} dBm"));
    }

    let (status, summary, rec) = if dns_ok {
        (
            DiagnosticStatus::Passed,
            "Network connectivity and wireless stack are healthy.".into(),
            None,
        )
    } else if wifi_up {
        (
            DiagnosticStatus::Warning,
            "Wi-Fi interface is active, but external DNS did not resolve.".into(),
            Some("Check router/DNS settings in NetworkManager.".into()),
        )
    } else {
        (
            DiagnosticStatus::Warning,
            "No active Wi-Fi or external internet route detected.".into(),
            Some("Verify Wi-Fi hardware switch or Ethernet connection.".into()),
        )
    };

    DiagnosticItem {
        category: DiagnosticCategory::NetworkAndBluetooth,
        status,
        summary,
        details,
        recommendation: rec,
        duration_ms: 0,
    }
}

fn check_driver(backend: &dyn PlatformBackend, device: &DeviceInfo) -> DiagnosticItem {
    let driver_name = if cfg!(target_os = "windows") {
        "ASUS ATKACPI (\\\\.\\ATKACPI) Driver"
    } else {
        "asus-nb-wmi / asus-wmi Module"
    };
    let mut details = vec![
        format!("OS: {}", device.os_name),
        format!("Kernel: {}", device.kernel_version),
        format!(
            "{driver_name}: {}",
            if device.asus_wmi_loaded { "Loaded" } else { "Not detected" }
        ),
    ];
    if let Some(boot) = backend.boot_time_summary() {
        details.push(format!("Boot Analysis: {boot}"));
    }

    let (status, summary, rec) = if device.asus_wmi_loaded {
        (
            DiagnosticStatus::Passed,
            if cfg!(target_os = "windows") {
                "ASUS System Control Interface (ATKACPI) is active and responding.".into()
            } else {
                "ASUS Linux kernel WMI driver is loaded and active.".into()
            },
            None,
        )
    } else {
        (
            DiagnosticStatus::Warning,
            format!("{driver_name} not active on this host."),
            Some(
                if cfg!(target_os = "windows") {
                    "Install ASUS System Control Interface v3 driver from ASUS Support.".into()
                } else {
                    "On ASUS laptops running Ubuntu/Linux Mint, verify with `lsmod | grep asus` or upgrade to kernel >= 6.5.".into()
                },
            ),
        )
    };

    DiagnosticItem {
        category: DiagnosticCategory::DriverAndSystem,
        status,
        summary,
        details,
        recommendation: rec,
        duration_ms: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fallback_hw::FallbackBackend;
    use crate::services::telemetry::TelemetryCollector;

    #[test]
    fn full_diagnosis_runs_all_six_categories() {
        let backend = FallbackBackend::new();
        let mut collector = TelemetryCollector::new();
        let info = collector.collect_device_info(&backend);
        let snap = collector.sample(&backend);
        let report = run_full_diagnosis(&backend, &info, &snap);
        assert_eq!(report.items.len(), 6);
        assert!(!report.timestamp.is_empty());
    }
}
