use egui::{RichText, Ui};

use crate::models::updates::UpdateReport;
use crate::ui::icons::{draw_icon, Icon};
use crate::ui::theme::{
    card_frame, pill_button, section_heading, status_badge, ASUS_BLUE, STATUS_OK, STATUS_WARN,
    TEXT_MUTED, TEXT_PRIMARY,
};

pub fn render_updates(ui: &mut Ui, report: &UpdateReport) -> bool {
    let mut trigger_refresh = false;

    section_heading(
        ui,
        "System Update",
        "Ubuntu / Linux Mint firmware (LVFS fwupd) and kernel / APT driver updates",
    );

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("{} Available Update(s)", report.items.len()))
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(16.0),
                );
                let checked = if report.last_checked.is_empty() {
                    "Click Check Now to query fwupdmgr & apt.".to_string()
                } else {
                    format!("Last checked: {}", report.last_checked)
                };
                ui.label(RichText::new(checked).color(TEXT_MUTED).size(12.0));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, "Check Now", true).clicked() {
                    trigger_refresh = true;
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            status_badge(
                ui,
                if report.fwupd_available {
                    "fwupdmgr Ready"
                } else {
                    "fwupdmgr Not Found"
                },
                if report.fwupd_available {
                    STATUS_OK
                } else {
                    STATUS_WARN
                },
            );
            status_badge(
                ui,
                if report.apt_available {
                    "APT Ready"
                } else {
                    "APT Not Found"
                },
                if report.apt_available {
                    STATUS_OK
                } else {
                    STATUS_WARN
                },
            );
        });
    });

    ui.add_space(12.0);

    for note in &report.notes {
        ui.horizontal(|ui| {
            draw_icon(ui, Icon::Info, 14.0, ASUS_BLUE);
            ui.label(RichText::new(note).color(TEXT_MUTED).size(12.0));
        });
    }

    if report.items.is_empty() {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Your system firmware and packages are up to date (or press Check Now).")
                    .color(TEXT_MUTED),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Linux Mint / Ubuntu manual apply commands:\n  sudo fwupdmgr refresh --force && sudo fwupdmgr update\n  sudo apt update && sudo apt upgrade",
                )
                .color(TEXT_MUTED)
                .monospace()
                .size(11.5),
            );
        });
    } else {
        for item in &report.items {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.name).color(TEXT_PRIMARY).strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_badge(
                            ui,
                            item.source.label(),
                            if item.critical { STATUS_WARN } else { ASUS_BLUE },
                        );
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "{}  →  {}",
                        item.current_version, item.available_version
                    ))
                    .color(ASUS_BLUE)
                    .size(12.5),
                );
                ui.label(RichText::new(&item.summary).color(TEXT_MUTED).size(12.0));
            });
            ui.add_space(8.0);
        }
    }

    trigger_refresh
}
