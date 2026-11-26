use egui::{RichText, Slider, Ui};

use crate::services::config::AppConfig;
use crate::ui::theme::{card_frame, section_heading, toggle_switch, TEXT_MUTED, TEXT_PRIMARY};

pub fn render_settings(ui: &mut Ui, config: &mut AppConfig, udev_writable: bool) -> bool {
    let mut changed = false;

    section_heading(
        ui,
        "Settings",
        "Configure telemetry polling, temperature units, and Linux Mint / Ubuntu permissions",
    );

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("General Preferences").color(TEXT_PRIMARY).strong().size(15.0));
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Telemetry Refresh Interval (seconds)").color(TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Slider::new(&mut config.refresh_interval_secs, 1..=10).suffix("s"))
                    .changed()
                {
                    changed = true;
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Display Temperatures in Fahrenheit (°F)").color(TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut config.use_fahrenheit).changed() {
                    changed = true;
                }
            });
        });
    });

    ui.add_space(12.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Ubuntu / Linux Mint Passwordless Hardware Control")
                .color(TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(6.0);
        let status = if udev_writable {
            "udev rules active — Battery limit, Fan profile & LEDs can be updated without password prompts."
        } else {
            "Using Polkit (pkexec) fallback for sysfs writes. Install the bundled udev rules for instant passwordless control:"
        };
        ui.label(RichText::new(status).color(TEXT_MUTED).size(12.5));
        if !udev_writable {
            ui.add_space(6.0);
            ui.label(
                RichText::new("sudo ./packaging/install-ubuntu-mint.sh")
                    .monospace()
                    .color(TEXT_PRIMARY)
                    .size(12.0),
            );
        }
    });

    ui.add_space(12.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("About myasus-rs").color(TEXT_PRIMARY).strong().size(15.0));
        ui.label(
            RichText::new(format!(
                "Version {}  •  Pure Rust native GUI (eframe/egui)  •  Clean-room Linux Mint / Ubuntu port",
                env!("CARGO_PKG_VERSION")
            ))
            .color(TEXT_MUTED)
            .size(12.5),
        );
    });

    changed
}
