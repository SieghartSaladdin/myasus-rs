use egui::{RichText, Ui};

use crate::models::system::DeviceInfo;
use crate::ui::theme::{
    card_frame, pill_button, section_heading, status_badge, ASUS_BLUE, STATUS_OK, TEXT_MUTED,
    TEXT_PRIMARY,
};

pub enum SupportAction {
    ExportReport,
    CopySystemSummary,
    RunDiagnosisAndExport,
}

pub fn render_support(
    ui: &mut Ui,
    device: &DeviceInfo,
    last_export_path: Option<&str>,
) -> Option<SupportAction> {
    let mut action = None;

    section_heading(
        ui,
        "Customer Support & Diagnostics Export",
        "Export full hardware/EC diagnostic bundles and quick troubleshooting tools",
    );

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Diagnostic Support Bundle (JSON)")
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(15.5),
                );
                ui.label(
                    RichText::new(
                        "Exports DMI model, serial, BIOS, live sensors, fan RPM, hardware capabilities, connected devices, and diagnosis report.",
                    )
                    .color(TEXT_MUTED)
                    .size(12.5),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, "Export Report", true).clicked() {
                    action = Some(SupportAction::ExportReport);
                }
            });
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if pill_button(ui, "Diagnose + Export", false).clicked() {
                action = Some(SupportAction::RunDiagnosisAndExport);
            }
            if pill_button(ui, "Copy System Info", false).clicked() {
                action = Some(SupportAction::CopySystemSummary);
            }
        });

        if let Some(path) = last_export_path {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                status_badge(ui, "Saved", STATUS_OK);
                ui.label(RichText::new(path).color(ASUS_BLUE).monospace().size(11.5));
            });
        }
    });

    ui.add_space(12.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Device Quick Reference")
                .color(TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Model: {}  •  Board: {}  •  BIOS: {} ({})\nSerial: {}  •  OS: {}",
                device.product_name,
                device.board_name,
                device.bios_version,
                device.bios_date,
                device.serial_number,
                device.os_name
            ))
            .color(TEXT_MUTED)
            .size(12.5),
        );
    });

    ui.add_space(12.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Cross-Platform Hardware Troubleshooting Guide")
                .color(TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(6.0);
        let tips = [
            "Ubuntu / Linux Mint: Ensure kernel >= 5.15 (`uname -r`) so `asus-nb-wmi` exposes battery threshold & fan policies.",
            "Ubuntu / Linux Mint: Run `sudo ./packaging/install-ubuntu-mint.sh` to grant passwordless `plugdev` udev access.",
            "Windows 10 / 11: Requires ASUS System Control Interface v3 (`asussci2.inf`) so `\\\\.\\ATKACPI` responds to fan RPM & battery limit.",
            "LVFS BIOS Updates (Linux): Run `sudo fwupdmgr refresh --force && sudo fwupdmgr update` to safely install UEFI capsules.",
        ];
        for tip in tips {
            ui.label(RichText::new(format!("• {tip}")).color(TEXT_MUTED).size(12.0));
        }
    });

    action
}
