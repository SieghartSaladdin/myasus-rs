//! Direct RandR CRTC gamma control (no `xrandr` CLI).
//!
//! `xrandr --output eDP --gamma .. --brightness ..` also re-sends the output's CRTC config, and
//! while Pixel Shift's `--transform` is active that grows the screen (e.g. 2880x1620 → 2880x1624).
//! The resulting RandR change event makes Cinnamon's `csd-color` reload the default gamma ramp
//! within a second, so every dimming apply undid itself and the panel went back to full
//! brightness. `SetCrtcGamma` only replaces the ramp: no config change, no event, no reset.

use x11rb::connection::Connection;
use x11rb::protocol::randr::{self, ConnectionExt as _};

/// Builds one gamma ramp channel exactly like `xrandr --gamma <g> --brightness <b>`:
/// `min((i / (size - 1)) ^ (1 / g) * b, 1) * 65535`.
pub fn ramp(size: usize, gamma: f32, brightness: f32) -> Vec<u16> {
    let size = size.max(2);
    let exponent = 1.0 / f64::from(gamma.max(0.1));
    let brightness = f64::from(brightness.clamp(0.0, 1.0));
    (0..size)
        .map(|i| {
            let x = i as f64 / (size - 1) as f64;
            ((x.powf(exponent) * brightness).min(1.0) * 65535.0).round() as u16
        })
        .collect()
}

/// Whether a live ramp is (within ~1.5%) the one we would set.
pub fn ramp_matches(live: &[u16], want: &[u16]) -> bool {
    const TOLERANCE: i32 = 1000;
    live.len() == want.len()
        && live
            .iter()
            .zip(want)
            .all(|(&a, &b)| (i32::from(a) - i32::from(b)).abs() <= TOLERANCE)
}

struct Target<C: Connection> {
    conn: C,
    crtc: randr::Crtc,
    size: usize,
}

fn open_crtc(output: &str) -> Result<Target<impl Connection>, String> {
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| format!("X11 connection failed: {e}"))?;
    let root = conn.setup().roots[screen_num].root;
    let res = conn
        .randr_get_screen_resources_current(root)
        .and_then(|c| Ok(c.reply()))
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("RandR unavailable: {e}"))?;
    for out in &res.outputs {
        let Ok(Ok(info)) = conn
            .randr_get_output_info(*out, res.config_timestamp)
            .map(|c| c.reply())
        else {
            continue;
        };
        if info.crtc == 0 || String::from_utf8_lossy(&info.name) != output {
            continue;
        }
        let size = conn
            .randr_get_crtc_gamma_size(info.crtc)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .size as usize;
        return Ok(Target { conn, crtc: info.crtc, size });
    }
    Err(format!("RandR output {output} has no active CRTC"))
}

/// The primary RandR output's name (e.g. `eDP`), else the first output driving a CRTC.
///
/// Uses `GetScreenResourcesCurrent`, which never re-probes outputs. A plain `xrandr` call
/// (`--listmonitors`, `--props`, ...) re-reads every panel's EDID over DDC/AUX; done every few
/// seconds by the dimming guard, that was ~2 probes a second on the eDP panel.
pub fn primary_output_name() -> Option<String> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let res = conn.randr_get_screen_resources_current(root).ok()?.reply().ok()?;
    let primary = conn
        .randr_get_output_primary(root)
        .ok()
        .and_then(|c| c.reply().ok())
        .map_or(0, |r| r.output);
    let mut first_active = None;
    for out in &res.outputs {
        let Ok(Ok(info)) = conn
            .randr_get_output_info(*out, res.config_timestamp)
            .map(|c| c.reply())
        else {
            continue;
        };
        if info.crtc == 0 {
            continue;
        }
        let name = String::from_utf8_lossy(&info.name).into_owned();
        if *out == primary {
            return Some(name);
        }
        first_active.get_or_insert(name);
    }
    first_active
}

fn ramps(size: usize, rgb: (f32, f32, f32), brightness: f32) -> [Vec<u16>; 3] {
    [
        ramp(size, rgb.0, brightness),
        ramp(size, rgb.1, brightness),
        ramp(size, rgb.2, brightness),
    ]
}

/// Sets `output`'s CRTC gamma ramp to Splendid `rgb` gamma at `brightness` (0..=1).
pub fn set_output_gamma(output: &str, rgb: (f32, f32, f32), brightness: f32) -> Result<(), String> {
    let t = open_crtc(output)?;
    let [r, g, b] = ramps(t.size, rgb, brightness);
    let result = t
        .conn
        .randr_set_crtc_gamma(t.crtc, &r, &g, &b)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| format!("SetCrtcGamma failed: {e}"));
    result
}

/// Whether `output`'s live CRTC ramp still is the one `set_output_gamma` would set.
/// `None` when it can't be read (no X11 / RandR).
pub fn output_gamma_matches(output: &str, rgb: (f32, f32, f32), brightness: f32) -> Option<bool> {
    let t = open_crtc(output).ok()?;
    let live = t.conn.randr_get_crtc_gamma(t.crtc).ok()?.reply().ok()?;
    let [r, g, b] = ramps(t.size, rgb, brightness);
    Some(ramp_matches(&live.red, &r) && ramp_matches(&live.green, &g) && ramp_matches(&live.blue, &b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_xrandr_compatible_ramps() {
        let identity = ramp(256, 1.0, 1.0);
        assert_eq!(identity[0], 0);
        assert_eq!(identity[255], 65535);
        assert_eq!(identity[128], (128.0 / 255.0 * 65535.0_f64).round() as u16);

        let dimmed = ramp(256, 1.0, 0.54);
        assert_eq!(dimmed[255], (0.54 * 65535.0_f64).round() as u16);

        // `--gamma 0.84` (Eye Care blue) lowers mid-tones: exponent 1/0.84 > 1.
        assert!(ramp(256, 0.84, 1.0)[128] < identity[128]);
    }

    #[test]
    fn detects_a_ramp_reset_to_identity() {
        let ours = ramp(1024, 1.0, 0.54);
        assert!(ramp_matches(&ours, &ramp(1024, 1.0, 0.54)));
        assert!(!ramp_matches(&ramp(1024, 1.0, 1.0), &ours));
        assert!(!ramp_matches(&ours[..512], &ours));
    }
}
