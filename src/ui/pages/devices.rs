use egui::{Color32, RichText, Ui};

use crate::models::peripherals::{ConnectedDevice, DeviceKind, MonitorInfo};
use crate::ui::theme::{
    card_frame, pill_button, section_heading, status_badge, ASUS_BLUE, STATUS_OK, TEXT_MUTED,
    TEXT_PRIMARY,
};

pub fn render_devices(
    ui: &mut Ui,
    devices: &[ConnectedDevice],
    monitors: &[MonitorInfo],
    filter_asus_only: &mut bool,
) -> bool {
    let mut refresh = false;

    section_heading(
        ui,
        "My ASUS Devices & Peripherals",
        "Connected USB, HID, Bluetooth, Audio, and Display hardware (ASUS VID 0x0B05 highlighted)",
    );

    ui.horizontal(|ui| {
        if pill_button(ui, "Scan Devices", true).clicked() {
            refresh = true;
        }
        ui.add_space(12.0);
        ui.checkbox(filter_asus_only, "Show ASUS Devices Only");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let asus_count = devices.iter().filter(|d| d.is_asus).count();
            ui.label(
                RichText::new(format!(
                    "Total: {}  •  ASUS Hardware: {}",
                    devices.len(),
                    asus_count
                ))
                .color(TEXT_MUTED)
                .size(12.5),
            );
        });
    });

    ui.add_space(12.0);

    // Displays card (matching AsusDisplay.dll)
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Connected Displays")
                .color(TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(6.0);
        if monitors.is_empty() {
            ui.label(RichText::new("Click Scan Devices to detect active monitors.").color(TEXT_MUTED));
        } else {
            for m in monitors {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&m.name).color(TEXT_PRIMARY).strong());
                    ui.label(RichText::new(&m.resolution).color(ASUS_BLUE));
                    if let Some(hz) = m.refresh_hz {
                        ui.label(RichText::new(format!("{hz} Hz")).color(TEXT_MUTED));
                    }
                    if m.primary {
                        status_badge(ui, "Primary", STATUS_OK);
                    }
                });
            }
        }
    });

    ui.add_space(12.0);

    let filtered: Vec<&ConnectedDevice> = devices
        .iter()
        .filter(|d| !*filter_asus_only || d.is_asus)
        .collect();

    if filtered.is_empty() {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("No matching devices found. Click Scan Devices to refresh.")
                    .color(TEXT_MUTED),
            );
        });
    } else {
        for dev in filtered {
            card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&dev.name).color(TEXT_PRIMARY).strong().size(13.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let kind_color = match dev.kind {
                            DeviceKind::Bluetooth => Color32::from_rgb(80, 165, 255),
                            DeviceKind::Audio => Color32::from_rgb(168, 130, 255),
                            DeviceKind::Display => STATUS_OK,
                            _ => TEXT_MUTED,
                        };
                        status_badge(ui, dev.kind.label(), kind_color);
                        if dev.is_asus {
                            status_badge(ui, "ASUS", ASUS_BLUE);
                        }
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "Vendor: {}  •  Status: {}  •  ID: {}",
                        dev.vendor, dev.status, dev.identifier
                    ))
                    .color(TEXT_MUTED)
                    .size(11.5),
                );
            });
            ui.add_space(6.0);
        }
    }

    refresh
}
