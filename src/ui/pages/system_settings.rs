use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl, Switch};

use crate::services::display_prefs::DisplayPrefs;
use crate::ui::app::MyAsusState;
use crate::ui::theme::{
    card_heading, font_mono, font_style, kv_row, page_title, status_ok, status_pill,
    status_warn, surface_card, text_muted, text_primary,
};

pub fn render_system_settings(s: &MyAsusState, k: &Kit) -> El {
    let d = &s.device;
    let prefs_path = DisplayPrefs::path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/myasus-rs/display.json".into());

    let interval_idx = match s.config.refresh_interval_secs {
        1 => 0,
        2 => 1,
        5 => 2,
        _ => 3,
    };

    let prefs_card = surface_card(vec![
        card_heading(
            "App & OLED Daemon Preferences",
            Some("App settings live in config.json; OLED Care state lives in display.json and is maintained by `myasus-rs oled-daemon`."),
        ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(
                text("Telemetry Refresh Interval")
                    .font(font_style(13.0, 600.0))
                    .color(text_primary()),
            )
            .child(
                SegmentedControl::new(&["1s", "2s", "5s", "10s"], Some(interval_idx))
                    .width(Some(220.0))
                    .on_pick(|s: &mut MyAsusState, idx, _| {
                        s.config.refresh_interval_secs = match idx {
                            0 => 1,
                            1 => 2,
                            2 => 5,
                            _ => 10,
                        };
                    })
                    .build(k),
            ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(
                text("Temperature Unit")
                    .font(font_style(13.0, 600.0))
                    .color(text_primary()),
            )
            .child(
                Switch::new(s.config.use_fahrenheit)
                    .label(if s.config.use_fahrenheit {
                        "Fahrenheit (°F)"
                    } else {
                        "Celsius (°C)"
                    })
                    .on_toggle(|s: &mut MyAsusState| {
                        s.config.use_fahrenheit = !s.config.use_fahrenheit;
                    })
                    .build(k),
            ),
        row()
            .pt(4.0)
            .gap(10.0)
            .child(
                Button::new("Save Preferences")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut MyAsusState| {
                        let res = s
                            .config
                            .save()
                            .map(|_| "Saved application preferences".to_string());
                        s.record_action_result("Settings", res);
                    })
                    .build(k),
            )
            .child(
                Button::new("Export Diagnostic JSON")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut MyAsusState| s.export_support_report())
                    .build(k),
            ),
    ]);

    let specs_card = surface_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(card_heading("Hardware, OLED Panel & Linux Status", None))
            .child(status_pill(
                if s.udev_writable {
                    "udev Writable"
                } else {
                    "Polkit / pkexec"
                },
                if s.udev_writable {
                    status_ok()
                } else {
                    status_warn()
                },
            )),
        kv_row("Product", format!("{} ({})", d.product_name, d.product_family)),
        kv_row("OLED Panel (EDID)", format!("{}, {}", s.panel_info.panel_model, s.panel_info.resolution)),
        kv_row("BIOS / Mainboard", format!("{} • {}", d.bios_version, d.board_name)),
        kv_row("OS & Kernel", format!("{} ({})", d.os_name, d.kernel_version)),
        kv_row("UI Renderer", "Rune Native GPU (wgpu)"),
        kv_row("OLED Prefs File", prefs_path),
        kv_row("Autostart Daemon", "/etc/xdg/autostart/myasus-rs-restore.desktop"),
    ]);

    let cli_card = surface_card(vec![
        card_heading("Linux CLI & Autostart Commands", None),
        text("myasus-rs dimming 70                     # Set & persist OLED Flicker-Free Dimming")
            .font(font_mono(11.5, 450.0))
            .color(text_primary()),
        text("myasus-rs oled-daemon --delay 5          # Background daemon: restore dimming + Pixel Shift")
            .font(font_mono(11.5, 450.0))
            .color(text_primary()),
        text("myasus-rs screensaver --style orbit      # Fullscreen true-black animated OLED screensaver")
            .font(font_mono(11.5, 450.0))
            .color(text_primary()),
        text("myasus-rs battery-limit 80               # Limit battery charge to 80%")
            .font(font_mono(11.5, 450.0))
            .color(text_muted()),
    ]);

    col()
        .w_full()
        .gap(14.0)
        .child(page_title(
            "System & Settings",
            "Hardware specs, EDID panel identity, autostart OLED daemon, and diagnostic export.",
        ))
        .child(prefs_card)
        .child(specs_card)
        .child(cli_card)
}
