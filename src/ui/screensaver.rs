//! Built-in GPU-animated OLED Dark Screensaver & Active Pixel Refresh Calibration.
//!
//! Renders over a pure `#000000` OLED black canvas (where >92% of pixels are completely off)
//! and continuously moves every lit element so zero static pixels remain lit.

use chrono::Local;
use rune::{Color, El, col, el, row, text};

use crate::services::display_prefs::ScreensaverStyle;
use crate::ui::theme::{asus_blue, font_mono, font_style, oled_teal, rgb};

#[derive(Debug, Clone)]
pub struct ScreensaverSession {
    pub active: bool,
    pub style: ScreensaverStyle,
    /// If `Some(total_secs)`, runs as a timed Pixel Refresh calibration cycle.
    pub calibration_total_secs: Option<f64>,
    pub started_clock: f64,
    pub elapsed_secs: f64,
    pub standalone_exit_on_dismiss: bool,
}

impl Default for ScreensaverSession {
    fn default() -> Self {
        Self {
            active: false,
            style: ScreensaverStyle::AsusOrbit,
            calibration_total_secs: None,
            started_clock: 0.0,
            elapsed_secs: 0.0,
            standalone_exit_on_dismiss: false,
        }
    }
}

impl ScreensaverSession {
    pub fn start_screensaver(&mut self, style: ScreensaverStyle, clock: f64) {
        self.active = true;
        self.style = style;
        self.calibration_total_secs = None;
        self.started_clock = clock;
        self.elapsed_secs = 0.0;
    }

    pub fn start_pixel_refresh(&mut self, duration_secs: f64, clock: f64) {
        self.active = true;
        self.style = ScreensaverStyle::PixelRefresh;
        self.calibration_total_secs = Some(duration_secs.max(15.0));
        self.started_clock = clock;
        self.elapsed_secs = 0.0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.calibration_total_secs = None;
    }
}

/// Renders the animated OLED dark screensaver / Pixel Refresh overlay on pure `#000000` black.
pub fn render_screensaver_canvas<S: 'static>(
    session: &ScreensaverSession,
    on_dismiss: impl Fn(&mut S) + 'static,
) -> El {
    let t = session.elapsed_secs as f32;
    let now_time = Local::now().format("%H:%M:%S").to_string();
    let now_date = Local::now().format("%A, %d %b %Y").to_string();

    let mut root = el()
        .key("oled-screensaver-root")
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)) // rune_kit::root lays overlays out beside the shell; pin covers the window
        .fill(Color::BLACK)
        .on_press(on_dismiss);

    match session.style {
        ScreensaverStyle::AsusOrbit => {
            // Smooth Lissajous drift across the true-black screen (ultra-low brightness)
            let cx = 420.0 + (t * 0.27).sin() * 260.0;
            let cy = 260.0 + (t * 0.19).cos() * 140.0;
            let ring2_x = 460.0 + (t * 0.17 + 1.4).cos() * 300.0;
            let ring2_y = 300.0 + (t * 0.23 + 0.8).sin() * 160.0;

            root = root
                .child(
                    el()
                        .key("orbit-ring-1")
                        .absolute(cx - 110.0, cy - 110.0)
                        .size(220.0, 220.0)
                        .radius(110.0)
                        .border(1.5, asus_blue().alpha(0.32)),
                )
                .child(
                    el()
                        .key("orbit-ring-2")
                        .absolute(ring2_x - 80.0, ring2_y - 80.0)
                        .size(160.0, 160.0)
                        .radius(80.0)
                        .border(1.5, oled_teal().alpha(0.28)),
                )
                .child(
                    col()
                        .key("orbit-clock")
                        .absolute(cx - 120.0, cy - 42.0)
                        .w(240.0)
                        .center()
                        .gap(4.0)
                        .child(
                            text(now_time)
                                .font(font_mono(34.0, 600.0))
                                .color(rgb(130, 175, 220).alpha(0.65)),
                        )
                        .child(
                            text(now_date)
                                .font(font_style(12.0, 450.0))
                                .color(rgb(140, 152, 170).alpha(0.85)),
                        )
                        .child(
                            text("ASUS OLED Care • Click or press Esc to wake")
                                .font(font_style(10.5, 450.0))
                                .color(oled_teal().alpha(0.67)),
                        ),
                );
        }
        ScreensaverStyle::PixelRefresh => {
            // Sweeping low-intensity R, G, B, Cyan, White wear-equalization bands across pure black
            let phase = (t * 0.18) % 1.0;
            let sweep_x = phase * 1160.0;
            let sweep_y = ((t * 0.13 + 0.3) % 1.0) * 760.0;

            let bands = [
                (0.0, rgb(180, 40, 40).alpha(0.42)),
                (28.0, rgb(40, 180, 70).alpha(0.42)),
                (56.0, rgb(40, 110, 210).alpha(0.45)),
                (84.0, oled_teal().alpha(0.35)),
            ];

            for (idx, (offset, col_c)) in bands.into_iter().enumerate() {
                let x = (sweep_x + offset) % 1160.0;
                root = root.child(
                    el()
                        .key(format!("v-band-{idx}"))
                        .absolute(x, 0.0)
                        .w(18.0)
                        .h_full()
                        .fill(col_c),
                );
            }

            root = root.child(
                el()
                    .key("h-band")
                    .absolute(0.0, sweep_y)
                    .w_full()
                    .h(14.0)
                    .fill(rgb(200, 215, 235).alpha(0.22)),
            );

            // Drifting calibration HUD
            let hud_x = 340.0 + (t * 0.22).sin() * 180.0;
            let hud_y = 260.0 + (t * 0.17).cos() * 110.0;
            let progress_label = if let Some(total) = session.calibration_total_secs {
                let pct = ((session.elapsed_secs / total) * 100.0).clamp(0.0, 100.0);
                let rem = (total - session.elapsed_secs).max(0.0).round() as u32;
                format!(
                    "Pixel Refresh Calibration: {:.0}% ({:02}:{:02} remaining)",
                    pct,
                    rem / 60,
                    rem % 60
                )
            } else {
                format!("Pixel Refresh Sweep Running ({:.0}s)", session.elapsed_secs)
            };

            root = root.child(
                col()
                    .key("refresh-hud")
                    .absolute(hud_x, hud_y)
                    .w(420.0)
                    .padding(14.0)
                    .radius(10.0)
                    .fill(Color::BLACK.alpha(0.75))
                    .border(1.0, oled_teal().alpha(0.35))
                    .gap(6.0)
                    .child(
                        row()
                            .center_y()
                            .gap(8.0)
                            .child(el().size(8.0, 8.0).radius(4.0).fill(oled_teal()))
                            .child(
                                text(progress_label)
                                    .font(font_mono(12.5, 650.0))
                                    .color(rgb(205, 225, 240).alpha(0.85)),
                            ),
                    )
                    .child(
                        text("Uniform RGB/luminance sweeps exercise subpixels evenly on true OLED black. Click or press Esc to exit.")
                            .font(font_style(11.0, 420.0))
                            .color(rgb(140, 155, 175).alpha(0.79))
                            .wrap()
                            .w_full(),
                    ),
            );
        }
        ScreensaverStyle::MinimalClock => {
            let cx = 420.0 + (t * 0.14).sin() * 280.0;
            let cy = 280.0 + (t * 0.11).cos() * 150.0;
            root = root.child(
                col()
                    .key("minimal-clock")
                    .absolute(cx - 120.0, cy - 36.0)
                    .w(240.0)
                    .center()
                    .gap(4.0)
                    .child(
                        text(now_time)
                            .font(font_mono(42.0, 500.0))
                            .color(rgb(120, 130, 145).alpha(0.75)),
                    )
                    .child(
                        text(now_date)
                            .font(font_style(12.0, 400.0))
                            .color(rgb(140, 152, 170).alpha(0.85)),
                    ),
            );
        }
    }

    root
}
