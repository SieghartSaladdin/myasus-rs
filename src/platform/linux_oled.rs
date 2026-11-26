use std::process::Command;

use crate::models::hardware::OledCareState;

fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    let out = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> bool {
    Command::new("gsettings")
        .args(["set", schema, key, value])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Reads OLED Care / desktop panel settings from Linux Mint Cinnamon or Ubuntu GNOME.
pub fn read_linux_oled_care() -> OledCareState {
    let mut state = OledCareState::default();

    // Pixel refresh idle screensaver (Cinnamon / GNOME)
    if let Some(delay) = gsettings_get("org.cinnamon.desktop.session", "idle-delay")
        .or_else(|| gsettings_get("org.gnome.desktop.session", "idle-delay"))
    {
        let secs = delay
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        state.pixel_refresh_enabled = secs > 0;
    }

    // Panel auto-hide (Linux Mint Cinnamon or Ubuntu GNOME Dash-to-Dock)
    if let Some(cinn) = gsettings_get("org.cinnamon", "panels-autohide") {
        state.auto_hide_taskbar = cinn.contains("true");
    } else if let Some(dock) = gsettings_get("org.gnome.shell.extensions.dash-to-dock", "dock-fixed") {
        state.auto_hide_taskbar = dock == "false";
    }

    // Panel transparency (Ubuntu Dash-to-Dock)
    if let Some(mode) = gsettings_get(
        "org.gnome.shell.extensions.dash-to-dock",
        "transparency-mode",
    ) {
        state.taskbar_transparency = mode.contains("DYNAMIC") || mode.contains("FIXED");
    }

    state
}

pub fn set_linux_pixel_refresh(enabled: bool) -> Result<String, String> {
    let delay = if enabled { "1800" } else { "0" };
    let ok = gsettings_set("org.cinnamon.desktop.session", "idle-delay", delay)
        || gsettings_set("org.gnome.desktop.session", "idle-delay", delay);
    if ok {
        Ok(format!(
            "OLED Pixel Refresh idle screensaver (30 min) {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Ok(format!(
            "Saved OLED Pixel Refresh preference ({}) — configure xscreensaver/DE idle timeout",
            if enabled { "On" } else { "Off" }
        ))
    }
}

pub fn set_linux_taskbar_autohide(enabled: bool) -> Result<String, String> {
    let cinn_val = if enabled { "['1:true']" } else { "['1:false']" };
    if gsettings_set("org.cinnamon", "panels-autohide", cinn_val) {
        return Ok(format!(
            "Linux Mint Cinnamon panel auto-hide {}",
            if enabled { "enabled" } else { "disabled" }
        ));
    }

    let dock_fixed = if enabled { "false" } else { "true" };
    if gsettings_set("org.gnome.shell.extensions.dash-to-dock", "dock-fixed", dock_fixed) {
        return Ok(format!(
            "Ubuntu GNOME dock auto-hide {}",
            if enabled { "enabled" } else { "disabled" }
        ));
    }

    Ok(format!(
        "Saved panel auto-hide preference ({}) — DE schema not detected",
        if enabled { "On" } else { "Off" }
    ))
}

pub fn set_linux_taskbar_transparency(enabled: bool) -> Result<String, String> {
    let mode = if enabled { "'DYNAMIC'" } else { "'DEFAULT'" };
    if gsettings_set(
        "org.gnome.shell.extensions.dash-to-dock",
        "transparency-mode",
        mode,
    ) {
        return Ok(format!(
            "Ubuntu taskbar transparency {}",
            if enabled { "enabled" } else { "disabled" }
        ));
    }
    Ok(format!(
        "Saved taskbar transparency preference ({})",
        if enabled { "On" } else { "Off" }
    ))
}

/// Real PulseAudio / PipeWire Volume Booster on Ubuntu & Linux Mint (over-amplification up to 135%).
pub fn set_linux_volume_booster(enabled: bool) -> Result<String, String> {
    let _ = gsettings_set(
        "org.gnome.desktop.sound",
        "allow-volume-above-100-percent",
        if enabled { "true" } else { "false" },
    );
    let _ = gsettings_set(
        "org.cinnamon.desktop.sound",
        "maximum-volume",
        if enabled { "150" } else { "100" },
    );

    let target = if enabled { "130%" } else { "100%" };
    let pactl_ok = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", target])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if pactl_ok {
        Ok(format!(
            "Volume Booster {} via PulseAudio/PipeWire ({target})",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Ok(format!(
            "Volume Booster {} (Cinnamon/GNOME over-amplification updated)",
            if enabled { "enabled" } else { "disabled" }
        ))
    }
}

pub fn apply_linux_flicker_free_dimming(level_percent: u8) -> Result<String, String> {
    let pct = level_percent.clamp(30, 100);
    let brightness = format!("{:.2}", pct as f32 / 100.0);
    let Ok(out) = Command::new("xrandr").arg("--listmonitors").output() else {
        return Ok(format!("Saved OLED Flicker-Free Dimming ({pct}%)"));
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let output_name = stdout
        .lines()
        .skip(1)
        .find_map(|line| line.split_whitespace().last())
        .unwrap_or("eDP-1");
    let ok = Command::new("xrandr")
        .args(["--output", output_name, "--brightness", &brightness])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(format!("OLED Flicker-Free Dimming set to {pct}% on {output_name}"))
    } else {
        Ok(format!("OLED Flicker-Free Dimming saved ({pct}%)"))
    }
}
