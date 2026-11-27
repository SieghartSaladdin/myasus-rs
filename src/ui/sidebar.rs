use rune::tok::motion;
use rune::{Color, El, Tx, col, el, row, spacer, text};

use crate::ui::app::MyAsusState;
use crate::ui::theme::{
    asus_blue, bg_canvas, bg_elevated, bg_sidebar, border_subtle, divider, font_mono, font_style,
    oled_teal, status_ok, status_warn, text_muted, text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    OledCare,
    Dashboard,
    HardwarePower,
    DiagnosisUpdates,
    SystemSettings,
}

impl NavPage {
    pub fn items() -> &'static [(NavPage, &'static str, &'static str, &'static str)] {
        // Each glyph names the page: half-lit circle (dimming), house (overview), bolt (power),
        // circular arrow (re-check and update), gear (settings).
        &[
            (NavPage::OledCare, "◐", "OLED Care", "Dimming, Shift & Saver"),
            (NavPage::Dashboard, "⌂", "Dashboard", "Live Telemetry"),
            (NavPage::HardwarePower, "⚡", "Hardware & Power", "Battery, Fan & Audio"),
            (NavPage::DiagnosisUpdates, "⟳", "Diagnosis & Updates", "Checkup & LVFS/APT"),
            (NavPage::SystemSettings, "⚙", "System & Settings", "Specs, Daemon & Export"),
        ]
    }
}

pub fn render_sidebar(s: &MyAsusState) -> El {
    // No logo file exists yet, so the product name stands in as a wordmark.
    let header = col()
        .w_full()
        .gap(2.0)
        .px(16.0)
        .py(8.0)
        .child(
            text("MyASUS Linux")
                .font(font_style(17.0, 750.0))
                .color(text_primary()),
        )
        .child(
            text("Unofficial, open source")
                .font(font_style(10.5, 500.0))
                .color(text_muted()),
        );

    let mut nav = col().w_full().px(10.0).gap(4.0);
    for &(page, glyph, label, hint) in NavPage::items() {
        nav = nav.child(nav_item(page, glyph, label, hint, s.active_page == page));
    }

    let wmi_color = if s.device.asus_wmi_loaded {
        status_ok()
    } else {
        status_warn()
    };

    let footer = col()
        .w_full()
        .px(12.0)
        .pb(14.0)
        .gap(8.0)
        .child(
            col()
                .w_full()
                .gap(5.0)
                .padding(11.0)
                .radius(8.0)
                .fill(bg_canvas())
                .border(1.0, border_subtle())
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .justify_between()
                        .child(
                            text("OLED Guard")
                                .font(font_style(11.0, 650.0))
                                .color(oled_teal()),
                        )
                        .child(
                            text(format!("{}%", s.display_prefs.flicker_free_dimming))
                                .font(font_mono(11.0, 700.0))
                                .color(text_primary()),
                        ),
                )
                .child(
                    text(format!(
                        "PWM Lock {} • Shift {}",
                        if s.display_prefs.pwm_safe_lock { "On" } else { "Off" },
                        if s.display_prefs.pixel_shift_enabled {
                            format!("({:+},{:+})", s.pixel_shift_offset.0, s.pixel_shift_offset.1)
                        } else {
                            "Off".into()
                        }
                    ))
                    .font(font_mono(10.5, 450.0))
                    .color(text_muted()),
                )
                .child(divider())
                .child(
                    row()
                        .center_y()
                        .gap(6.0)
                        .child(el().size(7.0, 7.0).radius(3.5).fill(wmi_color))
                        .child(
                            text(if s.device.asus_wmi_loaded {
                                format!("{} • WMI", s.device.os_name)
                            } else {
                                s.device.os_name.clone()
                            })
                            .font(font_style(10.5, 550.0))
                            .color(text_primary()),
                        ),
                ),
        );

    col()
        .w(244.0)
        .h_full()
        .pt(14.0)
        .gap(12.0)
        .fill(bg_sidebar())
        .border_sides([0.0, 1.0, 0.0, 0.0], border_subtle())
        .child(header)
        .child(el().px(14.0).child(divider()))
        .child(nav)
        .child(spacer())
        .child(footer)
}

fn nav_item(
    page: NavPage,
    glyph: &'static str,
    label: &'static str,
    hint: &'static str,
    active: bool,
) -> El {
    let accent = if page == NavPage::OledCare {
        oled_teal()
    } else {
        asus_blue()
    };
    let bg = if active {
        accent.alpha(0.18)
    } else {
        Color::TRANSPARENT
    };
    let border = if active {
        accent.alpha(0.55)
    } else {
        Color::TRANSPARENT
    };
    let hover_bg = if active {
        accent.alpha(0.22)
    } else {
        bg_elevated()
    };

    row()
        .key(format!("nav-{label}"))
        .role("button")
        .label(label)
        .focusable(true)
        .w_full()
        .px(12.0)
        .py(9.0)
        .gap(10.0)
        .radius(8.0)
        .center_y()
        .fill(bg)
        .border(1.0, border)
        .hover_fill(hover_bg)
        .motion(motion::SNAPPY)
        .on_press_tx(move |s: &mut MyAsusState, tx: &mut Tx| {
            if s.active_page != page {
                s.active_page = page;
                // A new page starts at its top instead of springing from the old scroll offset.
                tx.scroll_to_instant("main-scroll", 0.0, 0.0);
            }
        })
        .child(
            el()
                .size(28.0, 28.0)
                .radius(7.0)
                .fill(if active { accent } else { bg_elevated() })
                .center()
                .child(
                    text(glyph)
                        .font(font_style(13.5, 700.0))
                        .color(if active { bg_canvas() } else { text_muted() }),
                ),
        )
        .child(
            col()
                .gap(1.0)
                .grow(1.0)
                .shrink()
                .child(
                    text(label)
                        .font(font_style(13.0, if active { 700.0 } else { 550.0 }))
                        .color(if active { Color::WHITE } else { text_primary() }),
                )
                .child(
                    text(hint)
                        .font(font_style(10.5, 420.0))
                        .color(if active {
                            text_primary().alpha(0.75)
                        } else {
                            text_muted()
                        }),
                ),
        )
}
