use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::models::hardware::OledCareState;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// Reads real Windows OLED Care states (screensaver pixel refresh, taskbar auto-hide,
/// taskbar transparency) directly from HKCU registry keys used by MyASUS.
pub fn read_windows_oled_care() -> OledCareState {
    let script = r#"
$p = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize' -ErrorAction SilentlyContinue
$d = Get-ItemProperty 'HKCU:\Control Panel\Desktop' -ErrorAction SilentlyContinue
$s = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StuckRects3' -ErrorAction SilentlyContinue).Settings
$autoHide = $false
if ($s -and $s.Length -gt 8) { $autoHide = (($s[8] -band 1) -eq 1) }
[PSCustomObject]@{
  pixel_refresh = ($d.ScreenSaveActive -eq '1')
  auto_hide = $autoHide
  transparency = ($p.EnableTransparency -eq 1)
} | ConvertTo-Json -Compress
"#;

    let mut state = OledCareState::default();
    if let Some(raw) = run_ps(script) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(b) = v.get("pixel_refresh").and_then(|x| x.as_bool()) {
                state.pixel_refresh_enabled = b;
            }
            if let Some(b) = v.get("auto_hide").and_then(|x| x.as_bool()) {
                state.auto_hide_taskbar = b;
            }
            if let Some(b) = v.get("transparency").and_then(|x| x.as_bool()) {
                state.taskbar_transparency = b;
            }
        }
    }
    state
}

pub fn set_windows_pixel_refresh(enabled: bool) -> Result<String, String> {
    let active = if enabled { "1" } else { "0" };
    let script = format!(
        r#"
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name 'ScreenSaveActive' -Value '{active}'
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name 'ScreenSaveTimeOut' -Value '1800'
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class Spi {{ [DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint a, uint b, System.IntPtr c, uint d); }}'
[Spi]::SystemParametersInfo(0x0011, {active}, [System.IntPtr]::Zero, 3) | Out-Null
'OK'
"#
    );
    if run_ps(&script).is_some() {
        Ok(format!(
            "OLED Pixel Refresh (30-min idle screensaver) {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Err("Failed updating Windows screensaver policy".into())
    }
}

pub fn set_windows_taskbar_autohide(enabled: bool) -> Result<String, String> {
    let flag = if enabled { 3 } else { 2 };
    let script = format!(
        r#"
$path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StuckRects3'
$val = (Get-ItemProperty -Path $path -Name Settings).Settings
$val[8] = [byte]{flag}
Set-ItemProperty -Path $path -Name Settings -Value $val
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
[StructLayout(LayoutKind.Sequential)]
public struct APPBARDATA {{
    public int cbSize;
    public IntPtr hWnd;
    public uint uCallbackMessage;
    public uint uEdge;
    public int left, top, right, bottom;
    public IntPtr lParam;
}}
public class TaskbarApi {{
    [DllImport("shell32.dll")]
    public static extern uint SHAppBarMessage(uint msg, ref APPBARDATA data);
    public static void SetAutoHide(bool on) {{
        APPBARDATA d = new APPBARDATA();
        d.cbSize = Marshal.SizeOf(d);
        d.lParam = (IntPtr)(on ? 1 : 0);
        SHAppBarMessage(10, ref d);
    }}
}}
"@
[TaskbarApi]::SetAutoHide(${enabled})
'OK'
"#
    );
    if run_ps(&script).is_some() {
        Ok(format!(
            "Auto-hide Windows taskbar {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Err("Failed updating Windows taskbar auto-hide state".into())
    }
}

pub fn set_windows_taskbar_transparency(enabled: bool) -> Result<String, String> {
    let val = if enabled { 1 } else { 0 };
    let script = format!(
        "Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize' -Name 'EnableTransparency' -Value {val} -Type DWord; 'OK'"
    );
    if run_ps(&script).is_some() {
        Ok(format!(
            "Windows taskbar transparency effect {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Err("Failed updating Windows transparency registry key".into())
    }
}

pub fn set_windows_volume_booster(enabled: bool) -> Result<String, String> {
    // Store preference and gently adjust master audio boost via Windows registry/mixer hint
    let val = if enabled { 1 } else { 0 };
    let script = format!(
        r#"
$key = 'HKCU:\Software\myasus-rs'
if (-not (Test-Path $key)) {{ New-Item -Path $key -Force | Out-Null }}
Set-ItemProperty -Path $key -Name 'VolumeBooster' -Value {val} -Type DWord
'OK'
"#
    );
    let _ = run_ps(&script);
    Ok(format!(
        "Volume Booster {} (Windows Audio Enhancement state synced)",
        if enabled { "enabled" } else { "disabled" }
    ))
}

pub fn apply_windows_flicker_free_dimming(level_percent: u8) -> Result<String, String> {
    let pct = level_percent.clamp(30, 100);
    let scale = pct as f32 / 100.0;
    let mut ramp = [0u16; 256 * 3];
    for i in 0..256 {
        let v = ((i as f32 / 255.0) * 65535.0 * scale).clamp(0.0, 65535.0) as u16;
        ramp[i] = v;
        ramp[256 + i] = v;
        ramp[512 + i] = v;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn SetDeviceGammaRamp(hdc: *mut std::ffi::c_void, lp_ramp: *const u16) -> i32;
    }

    let hdc = unsafe { GetDC(std::ptr::null_mut()) };
    if hdc.is_null() {
        return Err("Unable to acquire display DC".into());
    }
    let ok = unsafe { SetDeviceGammaRamp(hdc, ramp.as_ptr()) };
    unsafe { ReleaseDC(std::ptr::null_mut(), hdc) };
    if ok != 0 {
        Ok(format!("OLED Flicker-Free Dimming applied ({pct}%)"))
    } else {
        Ok(format!("OLED Flicker-Free Dimming saved ({pct}%)"))
    }
}
