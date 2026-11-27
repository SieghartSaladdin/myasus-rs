use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, SegmentedControl, Slider, Switch};

use crate::models::hardware::SplendidMode;
use crate::services::display_prefs::{PixelShiftStrength, ScreensaverStyle};
use crate::ui::app::MyAsusState;
use crate::ui::theme::{
    asus_blue, bg_card, bg_elevated, border_subtle, card_heading, divider, font_mono, font_style,
    left_blue_card, oled_notice_red, oled_teal, oled_teal_card, page_title, rgb, status_ok,
    status_pill, status_warn, surface_card, text_muted, text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OledSubTab {
    DimmingAndCarePro,
    PixelRefreshAndShift,
    SystemAndScreensaver,
}

impl OledSubTab {
    fn index(self) -> usize {
        match self {
            Self::DimmingAndCarePro => 0,
            Self::PixelRefreshAndShift => 1,
            Self::SystemAndScreensaver => 2,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            1 => Self::PixelRefreshAndShift,
            2 => Self::SystemAndScreensaver,
            _ => Self::DimmingAndCarePro,
        }
    }
}

pub fn render_oled_care(s: &MyAsusState, k: &Kit) -> El {
    let effective_pct = ((s.hw.display_brightness_percent as f32 / 100.0)
        * (s.display_prefs.flicker_free_dimming as f32 / 100.0)
        * 100.0)
        .round() as u8;

    // Top OLED Panel Status Hero
    let panel_hero = col()
        .w_full()
        .gap(12.0)
        .padding(18.0)
        .radius(12.0)
        .gradient(rgb(14, 34, 46), bg_card())
        .border(1.0, oled_teal().alpha(0.38))
        .child(
            row()
                .w_full()
                .center_y()
                .justify_between()
                .gap(12.0)
                .child(
                    col()
                        .grow(1.0)
                        .shrink()
                        .gap(4.0)
                        .child(
                            row()
                                .center_y()
                                .gap(8.0)
                                .child(
                                    text(s.panel_info.panel_model.clone())
                                        .font(font_style(18.0, 750.0))
                                        .color(text_primary()),
                                )
                                .child(status_pill(
                                    format!(
                                        "{} • {}",
                                        s.panel_info.output_name, s.panel_info.resolution
                                    ),
                                    oled_teal(),
                                )),
                        )
                        .child(
                            text(format!(
                                "HW Backlight {}%  •  Gamma Dimming {}%  •  Effective ~{}%  •  Splendid {} ({})  •  Pixel Shift ({:+},{:+})px",
                                s.hw.display_brightness_percent,
                                s.display_prefs.flicker_free_dimming,
                                effective_pct,
                                s.display_prefs.splendid_mode.label(),
                                s.display_prefs.splendid_gamma_string(),
                                s.pixel_shift_offset.0,
                                s.pixel_shift_offset.1
                            ))
                            .font(font_mono(11.5, 500.0))
                            .color(text_muted())
                            .wrap()
                            .w_full(),
                        ),
                )
                .child(
                    row()
                        .gap(8.0)
                        .child(
                            Button::new("Screensaver")
                                .variant(ButtonVariant::Contrast)
                                .control_size(ControlSize::Regular)
                                .on_press(|s: &mut MyAsusState| {
                                    let style = s.display_prefs.screensaver_style;
                                    s.screensaver.start_screensaver(style, s.current_clock);
                                })
                                .build(k),
                        )
                        .child(
                            Button::new("Pixel Refresh")
                                .variant(ButtonVariant::Neutral)
                                .control_size(ControlSize::Regular)
                                .on_press(|s: &mut MyAsusState| {
                                    s.screensaver.start_pixel_refresh(60.0, s.current_clock);
                                })
                                .build(k),
                        )
                        .child(
                            Button::new("Screen Off")
                                .variant(ButtonVariant::Outline)
                                .control_size(ControlSize::Regular)
                                .on_press(|s: &mut MyAsusState| {
                                    #[cfg(target_os = "linux")]
                                    {
                                        let res =
                                            crate::platform::linux_oled::turn_off_display_now();
                                        s.record_action_result("DPMS Screen Off", res);
                                    }
                                })
                                .build(k),
                        ),
                ),
        );

    let sub_tabs = SegmentedControl::new(
        &[
            "Dimming & Care Pro",
            "Pixel Refresh & Shift",
            "System & Screensaver",
        ],
        Some(s.oled_tab.index()),
    )
    .width(None)
    .on_pick(|s: &mut MyAsusState, idx, _| {
        s.oled_tab = OledSubTab::from_index(idx);
    })
    .build(k);

    let tab_body = match s.oled_tab {
        OledSubTab::DimmingAndCarePro => render_dimming_and_care_pro(s, k),
        OledSubTab::PixelRefreshAndShift => render_pixel_refresh_and_shift(s, k),
        OledSubTab::SystemAndScreensaver => render_system_and_screensaver(s, k),
    };
    let tab_body = crate::ui::theme::swap_view(format!("oled-tab-{}", s.oled_tab.index()), tab_body);

    col()
        .w_full()
        .gap(14.0)
        .child(page_title(
            "ASUS OLED Care (Linux Native)",
            "Only real, verified Linux OLED protections: PWM-free dimming, Anti-ABL, xrandr Pixel Shift, Dark Mode, DPMS & GPU Screensaver.",
        ))
        .child(panel_hero)
        .child(sub_tabs)
        .child(tab_body)
}

fn render_dimming_and_care_pro(s: &MyAsusState, k: &Kit) -> El {
    let p = &s.display_prefs;

    // 1. Flicker-Free Dimming + PWM-Safe Backlight Lock + Uniform Brightness (Anti-ABL)
    let dimming_card = oled_teal_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(card_heading(
                "OLED Flicker-Free Dimming & PWM Guard",
                Some("Holds hardware backlight above PWM flicker threshold and dims smoothly via CRTC gamma. Persists across reboots."),
            ))
            .child(
                el()
                    .px(11.0)
                    .py(4.0)
                    .radius(6.0)
                    .fill(oled_teal().alpha(0.18))
                    .border(1.0, oled_teal().alpha(0.5))
                    .child(
                        text(format!("{}%", p.flicker_free_dimming))
                            .font(font_mono(13.5, 700.0))
                            .color(text_primary()),
                    ),
            ),
        row()
            .w_full()
            .center_y()
            .gap(10.0)
            .child(
                Slider::new("oled-dim-slider", p.flicker_free_dimming as f32)
                    .range(30.0..=100.0)
                    .step(1.0)
                    .width(290.0)
                    .label("OLED Flicker-Free Dimming")
                    .hide_label()
                    .value_text(format!("{}%", p.flicker_free_dimming))
                    .on_change(|s: &mut MyAsusState, val, done| {
                        let pct = val.round().clamp(30.0, 100.0) as u8;
                        s.display_prefs.flicker_free_dimming = pct;
                        s.hw.oled_care.flicker_free_dimming = pct;
                        if done {
                            let res = s.backend.set_oled_flicker_free_dimming(pct);
                            s.refresh_hardware();
                            s.record_action_result("OLED Dimming", res);
                        }
                    })
                    .build(k),
            )
            .child(dim_preset("50%", 50, p.flicker_free_dimming == 50, k))
            .child(dim_preset("70%", 70, p.flicker_free_dimming == 70, k))
            .child(dim_preset("85%", 85, p.flicker_free_dimming == 85, k))
            .child(dim_preset("100%", 100, p.flicker_free_dimming == 100, k)),
        divider(),
        toggle_row(
            "PWM-Free Hardware Backlight Lock",
            "Keeps panel backlight >= PWM floor so lowering Screen Brightness shifts to gamma instead of low-Hz PWM.",
            p.pwm_safe_lock,
            |s: &mut MyAsusState| {
                let next = !s.display_prefs.pwm_safe_lock;
                let floor = s.display_prefs.pwm_safe_floor;
                s.display_prefs.pwm_safe_lock = next;
                let res = s.backend.set_oled_pwm_safe_lock(next, floor);
                s.refresh_hardware();
                s.record_action_result("PWM-Free Lock", res);
            },
            k,
        ),
        row()
            .w_full()
            .center_y()
            .gap(12.0)
            .child(
                text("Hardware PWM Floor:")
                    .font(font_style(12.0, 500.0))
                    .color(text_muted()),
            )
            .child(
                Slider::new("pwm-floor-slider", p.pwm_safe_floor as f32)
                    .range(40.0..=100.0)
                    .step(5.0)
                    .width(200.0)
                    .disabled(!p.pwm_safe_lock)
                    .label("PWM Floor")
                    .hide_label()
                    .on_change(|s: &mut MyAsusState, val, done| {
                        let floor = val.round().clamp(40.0, 100.0) as u8;
                        s.display_prefs.pwm_safe_floor = floor;
                        if done {
                            let res = s
                                .backend
                                .set_oled_pwm_safe_lock(s.display_prefs.pwm_safe_lock, floor);
                            s.refresh_hardware();
                            s.record_action_result("PWM Floor", res);
                        }
                    })
                    .build(k),
            )
            .child(
                text(format!("{}%", p.pwm_safe_floor))
                    .font(font_mono(12.0, 650.0))
                    .color(text_primary()),
            ),
        divider(),
        toggle_row(
            "Uniform Brightness (Anti-ABL Limiter)",
            "Caps peak hardware backlight so OLED Automatic Brightness Limiter (ABL) doesn't pump between dark and bright windows.",
            p.uniform_brightness_enabled,
            |s: &mut MyAsusState| {
                let next = !s.display_prefs.uniform_brightness_enabled;
                s.display_prefs.uniform_brightness_enabled = next;
                let cap = s.display_prefs.uniform_brightness_cap;
                let _ = s.display_prefs.save();
                if next && s.hw.display_brightness_percent > cap {
                    let res = s.backend.set_display_brightness(cap);
                    s.refresh_hardware();
                    s.record_action_result("Uniform Brightness", res);
                } else {
                    s.status_banner = Some((
                        format!(
                            "Uniform Brightness (Anti-ABL) {} (cap {}%)",
                            if next { "enabled" } else { "disabled" },
                            cap
                        ),
                        false,
                    ));
                }
            },
            k,
        ),
        row()
            .w_full()
            .center_y()
            .gap(12.0)
            .child(
                text("Uniform Peak Cap:")
                    .font(font_style(12.0, 500.0))
                    .color(text_muted()),
            )
            .child(
                Slider::new("uniform-cap-slider", p.uniform_brightness_cap as f32)
                    .range(55.0..=95.0)
                    .step(5.0)
                    .width(200.0)
                    .disabled(!p.uniform_brightness_enabled)
                    .label("Uniform Brightness Cap")
                    .hide_label()
                    .on_change(|s: &mut MyAsusState, val, done| {
                        let cap = val.round().clamp(55.0, 95.0) as u8;
                        s.display_prefs.uniform_brightness_cap = cap;
                        if done {
                            let _ = s.display_prefs.save();
                            if s.display_prefs.uniform_brightness_enabled
                                && s.hw.display_brightness_percent > cap
                            {
                                let res = s.backend.set_display_brightness(cap);
                                s.refresh_hardware();
                                s.record_action_result("Uniform Brightness Cap", res);
                            } else {
                                s.status_banner = Some((
                                    format!("Uniform Brightness peak cap saved ({cap}%)"),
                                    false,
                                ));
                            }
                        }
                    })
                    .build(k),
            )
            .child(
                text(format!("{}%", p.uniform_brightness_cap))
                    .font(font_mono(12.0, 650.0))
                    .color(text_primary()),
            ),
        divider(),
        row()
            .w_full()
            .center_y()
            .gap(12.0)
            .child(
                text("Unified Screen Brightness:")
                    .font(font_style(12.5, 600.0))
                    .color(text_primary()),
            )
            .child(
                Slider::new("screen-brightness-slider", s.hw.display_brightness_percent as f32)
                    .range(10.0..=100.0)
                    .step(1.0)
                    .width(220.0)
                    .label("Screen Brightness")
                    .hide_label()
                    .on_change(|s: &mut MyAsusState, val, _| {
                        s.hw.display_brightness_percent = val.round().clamp(10.0, 100.0) as u8;
                    })
                    .build(k),
            )
            .child(
                text(format!("{}%", s.hw.display_brightness_percent))
                    .font(font_mono(12.5, 650.0))
                    .color(text_primary()),
            )
            .child(
                Button::new("Apply")
                    .variant(ButtonVariant::Contrast)
                    .control_size(ControlSize::Small)
                    .on_press(|s: &mut MyAsusState| {
                        let pct = s.hw.display_brightness_percent;
                        let res = s.backend.set_display_brightness(pct);
                        s.refresh_hardware();
                        s.record_action_result("Screen Brightness", res);
                    })
                    .build(k),
            ),
    ]);

    // 2. Splendid & Eye Care (Atomically combined with OLED Dimming!)
    let splendid_idx = match p.splendid_mode {
        SplendidMode::Normal => 0,
        SplendidMode::Vivid => 1,
        SplendidMode::EyeCare => 2,
        SplendidMode::Manual => 3,
    };

    let splendid_card = left_blue_card(vec![
        card_heading(
            "Splendid Color & Eye Care",
            Some("Applies RGB gamma atomically alongside OLED Dimming so neither resets the other."),
        ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                SegmentedControl::new(&["Normal", "Vivid", "Eye Care", "Manual"], Some(splendid_idx))
                    .width(Some(360.0))
                    .on_pick(|s: &mut MyAsusState, idx, _| {
                        let mode = match idx {
                            1 => SplendidMode::Vivid,
                            2 => SplendidMode::EyeCare,
                            3 => SplendidMode::Manual,
                            _ => SplendidMode::Normal,
                        };
                        s.hw.splendid_mode = mode;
                        s.display_prefs.splendid_mode = mode;
                        let lvl = s.hw.eye_care_level;
                        let res = s.backend.set_splendid_mode(mode, lvl);
                        s.refresh_hardware();
                        s.record_action_result("Splendid Profile", res);
                    })
                    .build(k),
            )
            .child(
                row()
                    .center_y()
                    .gap(8.0)
                    .child(
                        text("Eye Care:")
                            .font(font_style(12.0, 500.0))
                            .color(text_muted()),
                    )
                    .child(
                        Slider::new("eyecare-slider", p.eye_care_level as f32)
                            .range(1.0..=5.0)
                            .step(1.0)
                            .width(120.0)
                            .label("Eye Care Level")
                            .hide_label()
                            .on_change(|s: &mut MyAsusState, val, done| {
                                let lvl = val.round().clamp(1.0, 5.0) as u8;
                                s.hw.eye_care_level = lvl;
                                s.display_prefs.eye_care_level = lvl;
                                if done {
                                    s.hw.splendid_mode = SplendidMode::EyeCare;
                                    let res = s.backend.set_splendid_mode(SplendidMode::EyeCare, lvl);
                                    s.refresh_hardware();
                                    s.record_action_result("Eye Care", res);
                                }
                            })
                            .build(k),
                    )
                    .child(
                        text(format!("L{}", p.eye_care_level))
                            .font(font_mono(12.0, 650.0))
                            .color(text_primary()),
                    ),
            ),
    ]);

    // 3. Away / Idle Auto-Dim & External ROG OLED Care Pro Monitor Status
    let away_idx = match p.away_dim_secs {
        0..=45 => 0,
        46..=75 => 1,
        76..=150 => 2,
        _ => 3,
    };

    let rog_status_pill = match &s.panel_info.external_rog_monitor {
        Some(name) => status_pill(format!("ROG Detected: {name}"), status_ok()),
        None => status_pill("Laptop OLED (No External ROG)", asus_blue()),
    };

    let away_and_rog_card = surface_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(card_heading(
                "Away Auto-Dim & ROG OLED Care Pro",
                Some("Automatically dims the OLED when idle (software proximity fallback on laptops) and probes external ROG monitors."),
            ))
            .child(rog_status_pill),
        toggle_row(
            "Idle / Away Auto-Dimming",
            "Automatically lowers display brightness when you step away (Cinnamon/GNOME idle-dim-ac & idle-dim-battery).",
            p.away_dim_enabled,
            |s: &mut MyAsusState| {
                let next = !s.display_prefs.away_dim_enabled;
                s.display_prefs.away_dim_enabled = next;
                #[cfg(target_os = "linux")]
                {
                    let res = crate::platform::linux_oled::set_linux_away_dim(
                        next,
                        s.display_prefs.away_dim_secs,
                        s.display_prefs.away_dim_level,
                    );
                    s.record_action_result("Away Auto-Dim", res);
                }
            },
            k,
        ),
        row()
            .w_full()
            .center_y()
            .gap(12.0)
            .child(
                text("Dim After Idle:")
                    .font(font_style(12.0, 500.0))
                    .color(text_muted()),
            )
            .child(
                SegmentedControl::new(&["30s", "60s", "2 min", "5 min"], Some(away_idx))
                    .width(Some(280.0))
                    .on_pick(|s: &mut MyAsusState, idx, _| {
                        let secs = match idx {
                            0 => 30,
                            1 => 60,
                            2 => 120,
                            _ => 300,
                        };
                        s.display_prefs.away_dim_secs = secs;
                        #[cfg(target_os = "linux")]
                        {
                            let res = crate::platform::linux_oled::set_linux_away_dim(
                                s.display_prefs.away_dim_enabled,
                                secs,
                                s.display_prefs.away_dim_level,
                            );
                            s.record_action_result("Away Auto-Dim", res);
                        }
                    })
                    .build(k),
            ),
        el()
            .w_full()
            .px(12.0)
            .py(9.0)
            .radius(7.0)
            .fill(bg_elevated())
            .border(1.0, border_subtle())
            .child(
                text(match &s.panel_info.external_rog_monitor {
                    Some(name) if s.panel_info.ddcutil_available => format!(
                        "External ROG OLED ({name}) connected with ddcutil ready — hardware OLED Care Pro controls available via DDC/CI."
                    ),
                    Some(name) => format!(
                        "External ROG OLED ({name}) detected! Install `sudo apt install ddcutil` to control hardware Neo Proximity / Logo Dimming via DDC/CI."
                    ),
                    None => "Neo Proximity ToF Sensor & hardware Logo Detection are monitor-firmware features on external ROG OLED monitors (e.g. PG27UCDM). On built-in laptop OLEDs, Idle Auto-Dim + Pixel Shift + Auto-Hide Taskbar provide equivalent Linux burn-in protection.".to_string(),
                })
                .font(font_style(11.5, 450.0))
                .color(text_muted())
                .wrap()
                .w_full(),
            ),
        text("Notice: Avoid displaying static high-contrast images at maximum brightness for extended periods to prevent OLED burn-in.")
            .font(font_style(11.5, 450.0))
            .color(oled_notice_red())
            .wrap()
            .w_full(),
    ]);

    col()
        .w_full()
        .gap(12.0)
        .child(dimming_card)
        .child(splendid_card)
        .child(away_and_rog_card)
}

fn render_pixel_refresh_and_shift(s: &MyAsusState, k: &Kit) -> El {
    let p = &s.display_prefs;
    let interval_idx = match p.pixel_shift_interval_mins {
        0..=44 => 0,
        45..=90 => 1,
        _ => 2,
    };

    // 1. Real xrandr Pixel Shift
    let shift_card = oled_teal_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(card_heading(
                "Pixel Shift (Real X11 RandR Translation)",
                Some("Subtly translates the image by integer pixels (1:1 sharp, zero blur) in an 8-point orbit so edges & icons age evenly."),
            ))
            .child(status_pill(
                format!(
                    "Offset ({:+}, {:+}) px",
                    s.pixel_shift_offset.0, s.pixel_shift_offset.1
                ),
                oled_teal(),
            )),
        toggle_row(
            "Enable Automatic Pixel Shift",
            "Run by the background login daemon (`myasus-rs oled-daemon`). Each shift blanks the screen for about a second on X11, so it runs rarely.",
            p.pixel_shift_enabled,
            |s: &mut MyAsusState| {
                let next = !s.display_prefs.pixel_shift_enabled;
                s.display_prefs.pixel_shift_enabled = next;
                let _ = s.display_prefs.save();
                if !next {
                    s.recenter_pixels();
                } else {
                    s.shift_pixels_next();
                }
            },
            k,
        ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                col()
                    .gap(5.0)
                    .child(
                        text("Shift Radius")
                            .font(font_style(12.0, 550.0))
                            .color(text_muted()),
                    )
                    .child(
                        SegmentedControl::new(
                            &[
                                PixelShiftStrength::Subtle.label(),
                                PixelShiftStrength::Standard.label(),
                                PixelShiftStrength::Strong.label(),
                            ],
                            Some(p.pixel_shift_strength.index()),
                        )
                        .fit()
                        .on_pick(|s: &mut MyAsusState, idx, _| {
                            s.display_prefs.pixel_shift_strength =
                                PixelShiftStrength::from_index(idx);
                            let _ = s.display_prefs.save();
                            if s.display_prefs.pixel_shift_enabled {
                                s.shift_pixels_next();
                            }
                        })
                        .build(k),
                    ),
            )
            .child(
                col()
                    .gap(5.0)
                    .child(
                        text("Orbit Interval")
                            .font(font_style(12.0, 550.0))
                            .color(text_muted()),
                    )
                    .child(
                        SegmentedControl::new(&["30 min", "60 min", "2 hours"], Some(interval_idx))
                            .width(Some(210.0))
                            .on_pick(|s: &mut MyAsusState, idx, _| {
                                s.display_prefs.pixel_shift_interval_mins = match idx {
                                    0 => 30,
                                    1 => 60,
                                    _ => 120,
                                };
                                let _ = s.display_prefs.save();
                                s.status_banner = Some((
                                    format!(
                                        "Pixel Shift interval saved ({} min)",
                                        s.display_prefs.pixel_shift_interval_mins
                                    ),
                                    false,
                                ));
                            })
                            .build(k),
                    ),
            ),
        row()
            .gap(8.0)
            .child(
                Button::new("Shift Now (Next Orbit Step)")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut MyAsusState| s.shift_pixels_next())
                    .build(k),
            )
            .child(
                Button::new("Recenter (0, 0)")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut MyAsusState| s.recenter_pixels())
                    .build(k),
            ),
    ]);

    // 2. Active OLED Pixel Refresh Calibration
    let refresh_card = left_blue_card(vec![
        card_heading(
            "OLED Pixel Refresh & Wear Calibration",
            Some("Runs moving low-intensity RGB + luminance homogenization sweeps over pure #000000 OLED black to clear temporary image retention."),
        ),
        row()
            .gap(10.0)
            .child(
                Button::new("Quick Refresh (1 min)")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut MyAsusState| {
                        s.screensaver.start_pixel_refresh(60.0, s.current_clock);
                    })
                    .build(k),
            )
            .child(
                Button::new("Standard Calibration (5 min)")
                    .variant(ButtonVariant::Neutral)
                    .on_press(|s: &mut MyAsusState| {
                        s.screensaver.start_pixel_refresh(300.0, s.current_clock);
                    })
                    .build(k),
            )
            .child(
                Button::new("Deep Refresh (15 min)")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut MyAsusState| {
                        s.screensaver.start_pixel_refresh(900.0, s.current_clock);
                    })
                    .build(k),
            ),
        divider(),
        toggle_row(
            "Auto-Activate Idle Screensaver",
            "Configures Cinnamon / GNOME desktop idle timeout (`idle-delay`) so static desktops automatically trigger screensaver.",
            s.hw.oled_care.pixel_refresh_enabled,
            |s: &mut MyAsusState| {
                let next = !s.hw.oled_care.pixel_refresh_enabled;
                s.hw.oled_care.pixel_refresh_enabled = next;
                let res = s.backend.set_oled_pixel_refresh(next);
                s.record_action_result("Idle Screensaver", res);
            },
            k,
        ),
    ]);

    col().w_full().gap(12.0).child(shift_card).child(refresh_card)
}

fn render_system_and_screensaver(s: &MyAsusState, k: &Kit) -> El {
    let screen_off_idx = match s.screen_off_mins {
        1..=3 => 0,
        4..=7 => 1,
        8..=12 => 2,
        13..=20 => 3,
        21..=60 => 4,
        _ => 5,
    };

    // 1. System & Display Protection (Real gsettings + DPMS!)
    let system_card = oled_teal_card(vec![
        card_heading(
            "System & Display Burn-In Protection",
            Some("Directly configures Linux Mint Cinnamon / Ubuntu GNOME themes, panel auto-hide, and X11 DPMS display sleep."),
        ),
        toggle_row(
            "System & App Dark Mode (GTK + XDG Portal)",
            "Switches `color-scheme` to `prefer-dark` and toggles your GTK theme (`-Dark` variant) to minimize OLED pixel wear and power draw.",
            s.system_dark_mode,
            |s: &mut MyAsusState| {
                let next = !s.system_dark_mode;
                #[cfg(target_os = "linux")]
                {
                    let res = crate::platform::linux_oled::set_linux_dark_mode(next);
                    if res.is_ok() {
                        s.system_dark_mode = next;
                    }
                    s.record_action_result("System Dark Mode", res);
                }
            },
            k,
        ),
        toggle_row(
            "Automatically Hide Desktop Taskbar / Panel",
            "Hides the Cinnamon panel / GNOME dock when not in use so persistent taskbar icons cannot burn into the OLED edge.",
            s.hw.oled_care.auto_hide_taskbar,
            |s: &mut MyAsusState| {
                let next = !s.hw.oled_care.auto_hide_taskbar;
                s.hw.oled_care.auto_hide_taskbar = next;
                let res = s.backend.set_oled_taskbar_autohide(next);
                s.record_action_result("Taskbar Auto-Hide", res);
            },
            k,
        ),
        divider(),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                col()
                    .gap(2.0)
                    .child(
                        text("Automatic Screen Off (Power + X11 DPMS)")
                            .font(font_style(13.5, 620.0))
                            .color(text_primary()),
                    )
                    .child(
                        text("Completely powers off the OLED panel after inactivity.")
                            .font(font_style(11.5, 400.0))
                            .color(text_muted()),
                    ),
            )
            .child(status_pill(
                if s.screen_off_mins == 0 {
                    "Never".into()
                } else {
                    format!("{} min", s.screen_off_mins)
                },
                if s.screen_off_mins > 0 && s.screen_off_mins <= 15 {
                    status_ok()
                } else {
                    status_warn()
                },
            )),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(10.0)
            .child(
                SegmentedControl::new(
                    &["2 min", "5 min", "10 min", "15 min", "30 min", "Never"],
                    Some(screen_off_idx),
                )
                .width(Some(420.0))
                .on_pick(|s: &mut MyAsusState, idx, _| {
                    let mins = match idx {
                        0 => 2,
                        1 => 5,
                        2 => 10,
                        3 => 15,
                        4 => 30,
                        _ => 0,
                    };
                    s.screen_off_mins = mins;
                    s.display_prefs.screen_off_mins = mins;
                    #[cfg(target_os = "linux")]
                    {
                        let res = crate::platform::linux_oled::set_linux_screen_off_mins(mins);
                        s.record_action_result("Auto Screen Off", res);
                    }
                })
                .build(k),
            )
            .child(
                Button::new("Turn Off Display Now")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut MyAsusState| {
                        #[cfg(target_os = "linux")]
                        {
                            let res = crate::platform::linux_oled::turn_off_display_now();
                            s.record_action_result("DPMS Screen Off", res);
                        }
                    })
                    .build(k),
            ),
    ]);

    // 2. Built-in GPU Animated Dark OLED Screensaver
    let saver_card = left_blue_card(vec![
        card_heading(
            "Dedicated OLED Dark Screensaver (GPU Animated)",
            Some("Pure #000000 black background (>92% pixels fully off) with slow-drifting low-luminance motion to prevent static image retention."),
        ),
        SegmentedControl::new(
            &[
                ScreensaverStyle::AsusOrbit.label(),
                ScreensaverStyle::PixelRefresh.label(),
                ScreensaverStyle::MinimalClock.label(),
            ],
            Some(s.display_prefs.screensaver_style.index()),
        )
        .width(Some(460.0))
        .on_pick(|s: &mut MyAsusState, idx, _| {
            s.display_prefs.screensaver_style = ScreensaverStyle::from_index(idx);
            let _ = s.display_prefs.save();
            s.status_banner = Some((
                format!(
                    "OLED Screensaver style set to {}",
                    s.display_prefs.screensaver_style.label()
                ),
                false,
            ));
        })
        .build(k),
        row()
            .gap(10.0)
            .child(
                Button::new("Launch OLED Screensaver Now")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut MyAsusState| {
                        let style = s.display_prefs.screensaver_style;
                        s.screensaver.start_screensaver(style, s.current_clock);
                    })
                    .build(k),
            )
            .child(
                Button::new("Register in Cinnamon Screensaver")
                    .variant(ButtonVariant::Neutral)
                    .on_press(|s: &mut MyAsusState| {
                        let style = s.display_prefs.screensaver_style.cli_name();
                        #[cfg(target_os = "linux")]
                        {
                            let res =
                                crate::platform::linux_oled::register_cinnamon_oled_screensaver(
                                    style,
                                );
                            s.record_action_result("Cinnamon Screensaver", res);
                        }
                    })
                    .build(k),
            ),
        text("Tip: You can also launch standalone from any shortcut or lock hook with `myasus-rs screensaver --style orbit`.")
            .font(font_mono(11.0, 450.0))
            .color(text_muted()),
    ]);

    col().w_full().gap(12.0).child(system_card).child(saver_card)
}

fn dim_preset(label: &'static str, pct: u8, active: bool, k: &Kit) -> El {
    Button::new(label)
        .variant(if active {
            ButtonVariant::Contrast
        } else {
            ButtonVariant::Neutral
        })
        .control_size(ControlSize::Small)
        .on_press(move |s: &mut MyAsusState| {
            s.display_prefs.flicker_free_dimming = pct;
            s.hw.oled_care.flicker_free_dimming = pct;
            let res = s.backend.set_oled_flicker_free_dimming(pct);
            s.refresh_hardware();
            s.record_action_result("OLED Dimming", res);
        })
        .build(k)
}

fn toggle_row(
    title: &'static str,
    subtitle: &'static str,
    checked: bool,
    on_toggle: impl Fn(&mut MyAsusState) + 'static,
    k: &Kit,
) -> El {
    row()
        .w_full()
        .center_y()
        .justify_between()
        .gap(14.0)
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(
                    text(title)
                        .font(font_style(13.5, 620.0))
                        .color(text_primary()),
                )
                .child(
                    text(subtitle)
                        .font(font_style(11.5, 400.0))
                        .color(text_muted())
                        .wrap()
                        .w_full(),
                ),
        )
        .child(
            Switch::new(checked)
                .label(if checked { "On" } else { "Off" })
                .on_toggle(on_toggle)
                .build(k),
        )
}
