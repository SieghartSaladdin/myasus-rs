//! Real Linux OLED Care controls:
//! - Pixel Shift via X11 RandR integer translation (`xrandr --transform`)
//! - Atomic Flicker-Free Dimming + Splendid RGB gamma (`xrandr --gamma --brightness`)
//! - System Dark Mode (GTK + XDG Portal `color-scheme` on Cinnamon & GNOME)
//! - Panel Auto-Hide & Idle Screensaver (`gsettings`)
//! - Auto Display Off (`gsettings` sleep-display + `xset dpms`) & Away/Idle Auto-Dim
//! - Real EDID OLED panel identification & optional external ROG monitor DDC/CI probe

use std::process::Command;

use crate::models::hardware::OledCareState;
use crate::services::display_prefs::{DisplayPrefs, PixelShiftStrength};

/// 8-point pixel-shift orbit around `(0, 0)` so subpixel borders age uniformly.
const SHIFT_ORBIT: [(i32, i32); 8] = [
    (0, 0),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
];

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OledPanelInfo {
    pub output_name: String,
    pub panel_model: String,
    pub is_likely_oled: bool,
    pub resolution: String,
    pub external_rog_monitor: Option<String>,
    pub ddcutil_available: bool,
}

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

/// Detects the primary active X11/RandR display output (e.g. `eDP`, `eDP-1`).
///
/// Never re-probes outputs: a plain `xrandr` query makes the driver re-read the panel's EDID.
pub fn primary_xrandr_output() -> Option<String> {
    if let Some(name) = crate::platform::x11_gamma::primary_output_name() {
        return Some(name);
    }
    let out = Command::new("xrandr")
        .args(["--current", "--listmonitors"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .skip(1)
        .find_map(|line| line.split_whitespace().last().map(ToString::to_string))
}

/// Parses EDID hex from `xrandr --props` to extract real panel model (e.g. `ATNA56AC01-0` Samsung OLED)
/// and probes whether an external ASUS ROG monitor is connected (and whether `ddcutil` is installed).
pub fn detect_oled_panel_info() -> OledPanelInfo {
    let output_name = primary_xrandr_output().unwrap_or_else(|| "eDP".to_string());
    let mut panel_model = String::new();
    let mut resolution = String::new();
    let mut external_rog_monitor = None;

    if let Ok(out) = Command::new("xrandr").args(["--current", "--props"]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let mut current_output = String::new();
        let mut in_edid = false;
        let mut edid_hex = String::new();

        for line in text.lines() {
            if !line.starts_with('\t') && !line.starts_with(' ') && line.contains(" connected") {
                if !edid_hex.is_empty() {
                    if let Some(name) = parse_edid_model_name(&edid_hex) {
                        if current_output == output_name && panel_model.is_empty() {
                            panel_model = name.clone();
                        } else if name.to_uppercase().contains("PG")
                            || name.to_uppercase().contains("XG")
                            || name.to_uppercase().contains("ROG")
                        {
                            external_rog_monitor = Some(format!("{current_output}: {name}"));
                        }
                    }
                    edid_hex.clear();
                }
                in_edid = false;
                current_output = line.split_whitespace().next().unwrap_or("").to_string();
                if current_output == output_name {
                    for tok in line.split_whitespace() {
                        if tok.contains('x') && tok.contains('+') {
                            resolution = tok.split('+').next().unwrap_or("").to_string();
                            break;
                        }
                    }
                }
            } else if line.trim() == "EDID:" {
                in_edid = true;
                edid_hex.clear();
            } else if in_edid {
                let trimmed = line.trim();
                if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                    edid_hex.push_str(trimmed);
                } else {
                    in_edid = false;
                    if let Some(name) = parse_edid_model_name(&edid_hex) {
                        if current_output == output_name && panel_model.is_empty() {
                            panel_model = name.clone();
                        } else if name.to_uppercase().contains("PG")
                            || name.to_uppercase().contains("XG")
                            || name.to_uppercase().contains("ROG")
                        {
                            external_rog_monitor = Some(format!("{current_output}: {name}"));
                        }
                    }
                    edid_hex.clear();
                }
            }
        }
        if !edid_hex.is_empty() {
            if let Some(name) = parse_edid_model_name(&edid_hex) {
                if current_output == output_name && panel_model.is_empty() {
                    panel_model = name;
                }
            }
        }
    }

    let is_likely_oled = panel_model.starts_with("ATNA")
        || panel_model.starts_with("SDC")
        || panel_model.to_uppercase().contains("OLED")
        || panel_model.starts_with("B156")
        || panel_model.is_empty();

    let display_model = if panel_model.starts_with("ATNA") {
        format!("Samsung OLED ({panel_model})")
    } else if panel_model.is_empty() {
        "Built-in OLED Panel".to_string()
    } else {
        panel_model
    };

    let ddcutil_available = Command::new("which")
        .arg("ddcutil")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    OledPanelInfo {
        output_name,
        panel_model: display_model,
        is_likely_oled,
        resolution: if resolution.is_empty() {
            "Active".into()
        } else {
            resolution
        },
        external_rog_monitor,
        ddcutil_available,
    }
}

/// Extracts EDID monitor name (tag 0xFC) or alphanumeric string (tag 0xFE) from 128+ byte hex EDID.
fn parse_edid_model_name(hex: &str) -> Option<String> {
    if hex.len() < 256 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for chunk in hex.as_bytes().chunks_exact(2) {
        let s = std::str::from_utf8(chunk).ok()?;
        bytes.push(u8::from_str_radix(s, 16).ok()?);
    }
    // 4 descriptor blocks at offsets 54, 72, 90, 108 (18 bytes each)
    let mut fallback = None;
    for offset in [54usize, 72, 90, 108] {
        if offset + 18 > bytes.len() {
            break;
        }
        let blk = &bytes[offset..offset + 18];
        if blk[0] == 0 && blk[1] == 0 && blk[2] == 0 && (blk[3] == 0xFC || blk[3] == 0xFE) {
            let text: String = blk[5..18]
                .iter()
                .take_while(|&&b| b != 0x0A && b != 0x00)
                .filter(|&&b| b.is_ascii_graphic() || b == b' ')
                .map(|&b| b as char)
                .collect();
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                if blk[3] == 0xFC {
                    return Some(trimmed);
                }
                fallback = Some(trimmed);
            }
        }
    }
    fallback
}

/// Reads OLED Care & real Linux desktop settings (Cinnamon / GNOME).
pub fn read_linux_oled_care() -> OledCareState {
    let prefs = DisplayPrefs::load();
    let mut state = OledCareState::default();

    state.flicker_free_dimming = prefs.flicker_free_dimming;
    state.pwm_safe_lock = prefs.pwm_safe_lock;
    state.pwm_safe_floor = prefs.pwm_safe_floor;
    state.pixel_shift_enabled = prefs.pixel_shift_enabled;

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
    } else if let Some(dock) =
        gsettings_get("org.gnome.shell.extensions.dash-to-dock", "dock-fixed")
    {
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

/// Reads whether the Linux desktop (Cinnamon / GNOME / XDG portal) is currently in Dark Mode.
pub fn read_linux_dark_mode() -> bool {
    if let Some(scheme) = gsettings_get("org.gnome.desktop.interface", "color-scheme") {
        if scheme.contains("prefer-dark") {
            return true;
        }
    }
    if let Some(theme) = gsettings_get("org.cinnamon.desktop.interface", "gtk-theme")
        .or_else(|| gsettings_get("org.gnome.desktop.interface", "gtk-theme"))
    {
        return theme.to_lowercase().contains("dark");
    }
    true
}

/// Switches BOTH XDG color-scheme (`prefer-dark` / `default`) and GTK theme (`-Dark` ↔ Light)
/// on Linux Mint Cinnamon and Ubuntu GNOME so apps and panels truly switch to OLED Dark Mode.
pub fn set_linux_dark_mode(enabled: bool) -> Result<String, String> {
    let scheme = if enabled { "'prefer-dark'" } else { "'default'" };
    let mut changed = gsettings_set("org.gnome.desktop.interface", "color-scheme", scheme);

    for schema in [
        "org.cinnamon.desktop.interface",
        "org.gnome.desktop.interface",
    ] {
        if let Some(raw) = gsettings_get(schema, "gtk-theme") {
            let current = raw.trim_matches('\'');
            let next = toggle_theme_dark_name(current, enabled);
            if next != current && gsettings_set(schema, "gtk-theme", &format!("'{next}'")) {
                changed = true;
            }
        }
    }

    if changed {
        Ok(format!(
            "System Dark Mode {} (GTK + XDG Portal color-scheme)",
            if enabled { "enabled" } else { "disabled" }
        ))
    } else {
        Err("Could not update desktop dark mode via gsettings".to_string())
    }
}

fn toggle_theme_dark_name(theme: &str, dark: bool) -> String {
    if dark {
        if theme.to_lowercase().contains("dark") {
            theme.to_string()
        } else if let Some(rest) = theme.strip_prefix("Mint-Y-") {
            format!("Mint-Y-Dark-{rest}")
        } else if theme == "Mint-Y" {
            "Mint-Y-Dark".to_string()
        } else if theme == "Adwaita" {
            "Adwaita-dark".to_string()
        } else {
            format!("{theme}-Dark")
        }
    } else if let Some(rest) = theme.strip_prefix("Mint-Y-Dark-") {
        format!("Mint-Y-{rest}")
    } else if theme == "Mint-Y-Dark" {
        "Mint-Y".to_string()
    } else if let Some(base) = theme
        .strip_suffix("-dark")
        .or_else(|| theme.strip_suffix("-Dark"))
    {
        base.to_string()
    } else {
        theme.to_string()
    }
}

/// Reads display auto-off timeout in minutes from Cinnamon/GNOME power settings.
pub fn read_linux_screen_off_mins() -> u32 {
    if let Some(val) = gsettings_get(
        "org.cinnamon.settings-daemon.plugins.power",
        "sleep-display-ac",
    ) {
        if let Ok(secs) = val.parse::<u32>() {
            return secs / 60;
        }
    }
    DisplayPrefs::load().screen_off_mins
}

/// Sets automatic screen-off timer via Cinnamon power settings AND X11 DPMS (`xset dpms`).
pub fn set_linux_screen_off_mins(mins: u32) -> Result<String, String> {
    let secs = mins * 60;
    let sec_str = secs.to_string();
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "sleep-display-ac",
        &sec_str,
    );
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "sleep-display-battery",
        &sec_str,
    );
    let dpms_ok = Command::new("xset")
        .args(["dpms", &sec_str, &sec_str, &sec_str])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let mut prefs = DisplayPrefs::load();
    prefs.screen_off_mins = mins;
    let _ = prefs.save();

    if mins == 0 {
        Ok("Auto Screen Off disabled (DPMS timeout cleared)".to_string())
    } else if dpms_ok {
        Ok(format!(
            "Auto Screen Off set to {mins} min (Cinnamon Power + X11 DPMS)"
        ))
    } else {
        Ok(format!("Auto Screen Off set to {mins} min (Cinnamon Power)"))
    }
}

/// Immediately puts the OLED display to sleep using X11 DPMS.
pub fn turn_off_display_now() -> Result<String, String> {
    let ok = Command::new("xset")
        .args(["dpms", "force", "off"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok("Display turned off via X11 DPMS".to_string())
    } else {
        Err("Failed to turn off display (xset dpms unavailable)".to_string())
    }
}

/// Configures Cinnamon/GNOME idle auto-dimming (software away dimmer when user steps away).
pub fn set_linux_away_dim(enabled: bool, secs: u32, level: u8) -> Result<String, String> {
    let flag = if enabled { "true" } else { "false" };
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "idle-dim-ac",
        flag,
    );
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "idle-dim-battery",
        flag,
    );
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "idle-dim-time",
        &secs.to_string(),
    );
    let _ = gsettings_set(
        "org.cinnamon.settings-daemon.plugins.power",
        "idle-brightness",
        &level.clamp(20, 70).to_string(),
    );

    let mut prefs = DisplayPrefs::load();
    prefs.away_dim_enabled = enabled;
    prefs.away_dim_secs = secs;
    prefs.away_dim_level = level;
    let _ = prefs.save();

    Ok(format!(
        "Away / Idle Auto-Dim {} (after {secs}s → {}%)",
        if enabled { "enabled" } else { "disabled" },
        level.clamp(20, 70)
    ))
}

/// Computes the `(dx, dy)` pixel offset for a given orbit step and strength.
pub fn pixel_shift_offset(step: usize, strength: PixelShiftStrength) -> (i32, i32) {
    let (ux, uy) = SHIFT_ORBIT[step % SHIFT_ORBIT.len()];
    let s = strength.pixels();
    (ux * s, uy * s)
}

/// Applies real integer-pixel translation via `xrandr --output <out> --transform 1,0,dx,0,1,dy,0,0,1`.
/// Integer offsets keep every pixel 1:1 sharp (no subpixel blur) while shifting static UI edges.
pub fn apply_linux_pixel_shift(
    step: usize,
    strength: PixelShiftStrength,
) -> Result<(i32, i32, String), String> {
    let Some(output_name) = primary_xrandr_output() else {
        return Err("xrandr output not detected".to_string());
    };
    let (dx, dy) = pixel_shift_offset(step, strength);
    let status = if dx == 0 && dy == 0 {
        Command::new("xrandr")
            .args(["--current", "--output", &output_name, "--transform", "none"])
            .status()
    } else {
        let matrix = format!("1,0,{dx},0,1,{dy},0,0,1");
        Command::new("xrandr")
            .args(["--current", "--output", &output_name, "--transform", &matrix])
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            schedule_display_pipeline_guard();
            Ok((
                dx,
                dy,
                format!("Pixel Shift applied on {output_name}: ({dx:+}, {dy:+}) px"),
            ))
        }
        _ => Err(format!("xrandr --transform failed on {output_name}")),
    }
}

/// Resets `xrandr --transform` back to `(0, 0)` center.
pub fn reset_linux_pixel_shift() -> Result<String, String> {
    let Some(output_name) = primary_xrandr_output() else {
        return Ok("Pixel Shift centered (0, 0)".to_string());
    };
    let ok = Command::new("xrandr")
        .args(["--current", "--output", &output_name, "--transform", "none"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        schedule_display_pipeline_guard();
        Ok(format!("Pixel Shift recentered to (0, 0) on {output_name}"))
    } else {
        Err(format!("Failed to reset transform on {output_name}"))
    }
}

/// Registers `myasus-rs screensaver` as Cinnamon's custom screensaver command (optional).
pub fn register_cinnamon_oled_screensaver(style: &str) -> Result<String, String> {
    let exe = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/usr/local/bin/myasus-rs".to_string());
    let cmd = format!("'{exe} screensaver --style {style}'");
    let ok1 = gsettings_set(
        "org.cinnamon.desktop.screensaver",
        "custom-screensaver-command",
        &cmd,
    );
    let ok2 = gsettings_set(
        "org.cinnamon.desktop.screensaver",
        "idle-activation-enabled",
        "true",
    );
    if ok1 || ok2 {
        Ok(format!(
            "Registered OLED dark screensaver in Cinnamon ({style})"
        ))
    } else {
        Err("Cinnamon screensaver gsettings schema not found".to_string())
    }
}

pub fn set_linux_pixel_refresh(enabled: bool) -> Result<String, String> {
    let prefs = DisplayPrefs::load();
    let delay_secs = if enabled {
        prefs.screensaver_idle_mins.max(5) * 60
    } else {
        0
    };
    let delay = delay_secs.to_string();
    let ok = gsettings_set("org.cinnamon.desktop.session", "idle-delay", &delay)
        || gsettings_set("org.gnome.desktop.session", "idle-delay", &delay);
    let _ = gsettings_set(
        "org.cinnamon.desktop.screensaver",
        "idle-activation-enabled",
        if enabled { "true" } else { "false" },
    );
    if ok {
        Ok(format!(
            "OLED Idle Screensaver {} ({} min)",
            if enabled { "enabled" } else { "disabled" },
            delay_secs / 60
        ))
    } else {
        Ok(format!(
            "Saved OLED Idle Screensaver preference ({})",
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
    if gsettings_set(
        "org.gnome.shell.extensions.dash-to-dock",
        "dock-fixed",
        dock_fixed,
    ) {
        return Ok(format!(
            "Ubuntu GNOME dock auto-hide {}",
            if enabled { "enabled" } else { "disabled" }
        ));
    }

    Err("Panel auto-hide schema (Cinnamon / GNOME Dash-to-Dock) not detected".to_string())
}

#[allow(dead_code)]
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
    Err("Taskbar transparency requires GNOME Dash-to-Dock".to_string())
}

/// Real PulseAudio / PipeWire Volume Booster on Ubuntu & Linux Mint (over-amplification up to 130%).
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
            "Volume Booster {} in desktop sound settings",
            if enabled { "enabled" } else { "disabled" }
        ))
    }
}

/// Atomically applies BOTH Splendid RGB gamma and OLED Flicker-Free software dimming in a
/// single `xrandr` call so they never overwrite each other's CRTC gamma ramp.
pub fn apply_linux_display_pipeline(prefs: &DisplayPrefs) -> Result<String, String> {
    let pct = prefs.flicker_free_dimming.clamp(30, 100);
    let brightness = format!("{:.2}", pct as f32 / 100.0);
    let gamma = prefs.splendid_gamma_string();
    let Some(output_name) = primary_xrandr_output() else {
        return Ok(format!(
            "Saved display pipeline: {} ({gamma}), dimming {pct}% (xrandr unavailable)",
            prefs.splendid_mode.label()
        ));
    };
    // Prefer setting only the CRTC gamma ramp: `xrandr --brightness` also re-sends the CRTC
    // config, which (with Pixel Shift active) fires a RandR event that csd-color answers by
    // resetting the ramp to full brightness.
    if crate::platform::x11_gamma::set_output_gamma(
        &output_name,
        prefs.splendid_gamma_rgb(),
        pct as f32 / 100.0,
    )
    .is_ok()
    {
        return Ok(format!(
            "Applied on {output_name}: OLED Dimming {pct}% + {} ({gamma})",
            prefs.splendid_mode.label()
        ));
    }
    let ok = Command::new("xrandr")
        .args([
            "--current",
            "--output",
            &output_name,
            "--gamma",
            &gamma,
            "--brightness",
            &brightness,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(format!(
            "Applied on {output_name}: OLED Dimming {pct}% + {} ({gamma})",
            prefs.splendid_mode.label()
        ))
    } else {
        Ok(format!(
            "Saved display pipeline: OLED Dimming {pct}% + {} ({gamma})",
            prefs.splendid_mode.label()
        ))
    }
}

/// Re-applies Flicker-Free Dimming + Splendid gamma when something reset the CRTC gamma ramp.
///
/// On Cinnamon, `csd-color` reloads the default (identity) ramp about a second after every RandR
/// change: each Pixel Shift `--transform`, resume from suspend, unlock, DPMS wake or hotplug.
/// That wiped the dimming and left the panel at full brightness until something re-applied it.
/// Returns the apply message when it had to re-apply.
pub fn reassert_display_pipeline_if_reset() -> Option<String> {
    let prefs = DisplayPrefs::load();
    let neutral = prefs.flicker_free_dimming >= 100
        && prefs.splendid_gamma_rgb() == (1.0, 1.0, 1.0);
    if neutral {
        // Nothing of ours to keep on the ramp; leave other tools (night light) alone.
        return None;
    }
    let output = primary_xrandr_output()?;
    let brightness = prefs.flicker_free_dimming.clamp(30, 100) as f32 / 100.0;
    let intact = crate::platform::x11_gamma::output_gamma_matches(
        &output,
        prefs.splendid_gamma_rgb(),
        brightness,
    )?;
    if intact {
        return None;
    }
    apply_linux_display_pipeline(&prefs).ok()
}

/// After our own RandR change (Pixel Shift), `csd-color` resets the ramp asynchronously, so
/// check a few times over the next seconds and restore the dimming as soon as it is wiped.
fn schedule_display_pipeline_guard() {
    let _ = std::thread::Builder::new()
        .name("myasus-gamma-guard".into())
        .spawn(|| {
            for delay_ms in [1_200u64, 2_500, 5_000] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                let _ = reassert_display_pipeline_if_reset();
            }
        });
}

/// How often the daemon checks that the dimming gamma ramp is still applied.
const GAMMA_GUARD_SECS: u64 = 3;

/// Runs the lightweight background OLED Care daemon (launched at XDG session login):
/// 1. Restores persisted Flicker-Free Dimming + Splendid gamma + Uniform/PWM backlight guard.
/// 2. Periodically shifts pixels in an 8-point integer orbit when Pixel Shift is enabled.
/// 3. Every few seconds, re-applies the gamma pipeline if a desktop daemon (`csd-color`),
///    suspend/resume or a RandR change reset it.
pub fn run_linux_oled_daemon(initial_delay_secs: u64) -> Result<(), String> {
    use std::time::{Duration, Instant};

    if initial_delay_secs > 0 {
        std::thread::sleep(Duration::from_secs(initial_delay_secs));
    }

    crate::services::failures::log_if_err(
        "restore saved display preferences",
        crate::platform::restore_saved_display_prefs(),
    );
    let mut step: usize = 0;
    let mut last_shift = Instant::now();

    loop {
        let prefs = DisplayPrefs::load();
        let shift_every = Duration::from_secs((prefs.pixel_shift_interval_mins.max(1) as u64) * 60);

        if prefs.pixel_shift_enabled && last_shift.elapsed() >= shift_every {
            last_shift = Instant::now();
            step = (step + 1) % SHIFT_ORBIT.len();
            crate::services::failures::log_if_err(
                "apply Pixel Shift",
                apply_linux_pixel_shift(step, prefs.pixel_shift_strength),
            );
        } else if !prefs.pixel_shift_enabled && step != 0 {
            step = 0;
            crate::services::failures::log_if_err("recenter Pixel Shift", reset_linux_pixel_shift());
        }

        let _ = reassert_display_pipeline_if_reset();

        std::thread::sleep(Duration::from_secs(GAMMA_GUARD_SECS));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_mint_dark_and_light_theme_names() {
        assert_eq!(
            toggle_theme_dark_name("Mint-Y-Dark-Aqua", false),
            "Mint-Y-Aqua"
        );
        assert_eq!(
            toggle_theme_dark_name("Mint-Y-Aqua", true),
            "Mint-Y-Dark-Aqua"
        );
        assert_eq!(toggle_theme_dark_name("Adwaita", true), "Adwaita-dark");
        assert_eq!(toggle_theme_dark_name("Adwaita-dark", false), "Adwaita");
    }

    #[test]
    fn computes_pixel_shift_orbit_offsets() {
        assert_eq!(pixel_shift_offset(0, PixelShiftStrength::Standard), (0, 0));
        assert_eq!(pixel_shift_offset(1, PixelShiftStrength::Standard), (4, 0));
        assert_eq!(pixel_shift_offset(2, PixelShiftStrength::Subtle), (2, 2));
        assert_eq!(pixel_shift_offset(5, PixelShiftStrength::Strong), (-6, 0));
    }
}
