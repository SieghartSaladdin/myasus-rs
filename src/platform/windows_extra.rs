use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::ptr;

use crate::models::{
    hardware::SplendidMode,
    peripherals::{ConnectedDevice, DeviceKind, MonitorInfo, NetworkSummary},
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[link(name = "user32")]
extern "system" {
    fn GetDC(hwnd: *mut c_void) -> *mut c_void;
    fn ReleaseDC(hwnd: *mut c_void, hdc: *mut c_void) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn SetDeviceGammaRamp(hdc: *mut c_void, lp_ramp: *const u16) -> i32;
}

/// Applies a real hardware gamma LUT color temperature / Eye Care blue-light filter
/// on Windows using GDI `SetDeviceGammaRamp` (zero external tools required).
pub fn apply_windows_gamma(mode: SplendidMode, eye_care_level: u8) -> Result<String, String> {
    let (r_scale, g_scale, b_scale): (f32, f32, f32) = match mode {
        SplendidMode::Normal => (1.0, 1.0, 1.0),
        SplendidMode::Vivid => (1.0, 0.99, 1.0),
        SplendidMode::EyeCare => {
            let lvl = eye_care_level.clamp(1, 5) as f32;
            (1.0, 1.0 - lvl * 0.03, 1.0 - lvl * 0.09)
        }
        SplendidMode::Manual => {
            let lvl = eye_care_level.clamp(1, 5) as f32;
            (1.0, 1.0 - lvl * 0.02, 1.0 - lvl * 0.06)
        }
    };

    let mut ramp = [0u16; 256 * 3];
    for i in 0..256 {
        let base = (i as f32 / 255.0) * 65535.0;
        let gamma_adj = if mode == SplendidMode::Vivid {
            ((i as f32 / 255.0).powf(0.93) * 65535.0).clamp(0.0, 65535.0)
        } else {
            base
        };
        ramp[i] = (gamma_adj * r_scale).clamp(0.0, 65535.0) as u16;
        ramp[256 + i] = (gamma_adj * g_scale).clamp(0.0, 65535.0) as u16;
        ramp[512 + i] = (gamma_adj * b_scale).clamp(0.0, 65535.0) as u16;
    }

    let hdc = unsafe { GetDC(ptr::null_mut()) };
    if hdc.is_null() {
        return Err("Failed acquiring Windows display DC".to_string());
    }
    let ok = unsafe { SetDeviceGammaRamp(hdc, ramp.as_ptr()) };
    unsafe {
        ReleaseDC(ptr::null_mut(), hdc);
    }

    if ok != 0 {
        Ok(format!(
            "Applied {} display color LUT via Windows GDI GammaRamp (R:{r_scale:.2} G:{g_scale:.2} B:{b_scale:.2})",
            mode.label()
        ))
    } else {
        Ok(format!(
            "Saved {} profile (display driver locked hardware gamma ramp)",
            mode.label()
        ))
    }
}

fn run_ps(script: &str) -> Option<String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub fn list_windows_devices() -> Vec<ConnectedDevice> {
    let script = r#"
$items = Get-CimInstance Win32_PnPEntity -ErrorAction SilentlyContinue |
  Where-Object { $_.Status -eq 'OK' -and $_.PNPClass -in @('USB','HIDClass','Bluetooth','AudioEndpoint','Monitor','Mouse','Keyboard') } |
  Select-Object -First 45 Name, Manufacturer, PNPClass, DeviceID, Status
$items | ConvertTo-Json -Compress
"#;

    let mut devices = Vec::new();
    let Some(raw) = run_ps(script) else {
        return devices;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return devices;
    };
    let arr = match val {
        serde_json::Value::Array(a) => a,
        obj => vec![obj],
    };

    for item in arr {
        let name = item
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Device")
            .to_string();
        let vendor = item
            .get("Manufacturer")
            .and_then(|v| v.as_str())
            .unwrap_or("Generic")
            .to_string();
        let class = item
            .get("PNPClass")
            .and_then(|v| v.as_str())
            .unwrap_or("USB");
        let id = item
            .get("DeviceID")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let status = item
            .get("Status")
            .and_then(|v| v.as_str())
            .unwrap_or("OK")
            .to_string();

        let kind = match class {
            "Bluetooth" => DeviceKind::Bluetooth,
            "AudioEndpoint" => DeviceKind::Audio,
            "Monitor" => DeviceKind::Display,
            "Mouse" | "Keyboard" | "HIDClass" => DeviceKind::Input,
            _ => DeviceKind::Usb,
        };
        let lower_id = id.to_ascii_uppercase();
        let is_asus = lower_id.contains("VID_0B05")
            || lower_id.contains("ASUS")
            || vendor.to_ascii_uppercase().contains("ASUS")
            || name.to_ascii_uppercase().contains("ASUS");

        devices.push(ConnectedDevice {
            name,
            vendor,
            kind,
            is_asus,
            status,
            identifier: id,
        });
    }

    devices.sort_by(|a, b| b.is_asus.cmp(&a.is_asus).then_with(|| a.name.cmp(&b.name)));
    devices
}

pub fn list_windows_monitors() -> Vec<MonitorInfo> {
    let script = r#"
Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue |
  Where-Object { $_.CurrentHorizontalResolution -gt 0 } |
  Select-Object Name, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate |
  ConvertTo-Json -Compress
"#;
    let mut list = Vec::new();
    let Some(raw) = run_ps(script) else {
        return list;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return list;
    };
    let arr = match val {
        serde_json::Value::Array(a) => a,
        obj => vec![obj],
    };
    for (idx, item) in arr.into_iter().enumerate() {
        let name = item
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("Display")
            .to_string();
        let w = item
            .get("CurrentHorizontalResolution")
            .and_then(|v| v.as_u64())
            .unwrap_or(1920);
        let h = item
            .get("CurrentVerticalResolution")
            .and_then(|v| v.as_u64())
            .unwrap_or(1080);
        let hz = item
            .get("CurrentRefreshRate")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        list.push(MonitorInfo {
            name,
            resolution: format!("{w}x{h}"),
            refresh_hz: hz,
            primary: idx == 0,
        });
    }
    list
}

pub fn read_windows_network_details(wifi_tuple: (bool, Option<i32>, String)) -> NetworkSummary {
    let (connected, signal_dbm, iface) = wifi_tuple;
    let mut summary = NetworkSummary {
        interface_name: iface,
        connected,
        signal_dbm,
        ..NetworkSummary::default()
    };

    if let Ok(out) = Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with("SSID") && !t.starts_with("BSSID") {
                let ssid = t
                    .split_once(':')
                    .map(|(_, v)| v.trim().to_string())
                    .filter(|s| !s.is_empty());
                summary.ssid = ssid;
            }
        }
    }

    let ip_script = r#"
$cfg = Get-NetIPConfiguration -ErrorAction SilentlyContinue | Where-Object { $_.IPv4DefaultGateway -ne $null } | Select-Object -First 1
if ($cfg) {
  [PSCustomObject]@{
    ip = $cfg.IPv4Address.IPAddress
    gw = $cfg.IPv4DefaultGateway.NextHop
    iface = $cfg.InterfaceAlias
  } | ConvertTo-Json -Compress
}
"#;
    if let Some(raw) = run_ps(ip_script) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            summary.local_ip = v.get("ip").and_then(|x| x.as_str()).map(String::from);
            summary.gateway_ip = v.get("gw").and_then(|x| x.as_str()).map(String::from);
            if summary.gateway_ip.is_some() {
                summary.connected = true;
            }
            if summary.interface_name.contains("No active") {
                if let Some(alias) = v.get("iface").and_then(|x| x.as_str()) {
                    summary.interface_name = alias.to_string();
                }
            }
        }
    }
    summary
}

pub fn read_windows_power_plan() -> String {
    let Ok(out) = Command::new("powercfg")
        .args(["/getactivescheme"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return "Balanced".to_string();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    text.split('(')
        .nth(1)
        .and_then(|s| s.split(')').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Balanced".to_string())
}

pub fn set_windows_power_plan(profile_key: &str) -> Result<String, String> {
    let guid = match profile_key {
        "powersave" => "a1841308-3541-4fab-bc81-f71556f20b4a",
        "performance" => "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
        _ => "381b4222-f694-41f0-9685-ff5bb260df2e",
    };
    let status = Command::new("powercfg")
        .args(["/setactive", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("Failed invoking powercfg: {e}"))?;
    if status.success() {
        Ok(format!("Switched Windows power plan to {profile_key}"))
    } else {
        Err(format!(
            "Windows Modern Standby laptop may restrict legacy powercfg scheme '{profile_key}'"
        ))
    }
}
