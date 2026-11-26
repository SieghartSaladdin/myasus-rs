use egui::{RichText, Ui};

use crate::models::{
    hardware::HardwareCapabilities,
    system::{DeviceInfo, TelemetrySnapshot},
};
use crate::ui::theme::{
    card_frame, section_heading, status_badge, STATUS_OK, STATUS_WARN, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn render_system_info(
    ui: &mut Ui,
    device: &DeviceInfo,
    telemetry: &TelemetrySnapshot,
    caps: &HardwareCapabilities,
    backend_name: &str,
) {
    section_heading(
        ui,
        "System Info & Hardware Interfaces",
        "DMI firmware identity, Linux kernel driver bindings, and detected sysfs controls",
    );

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Hardware & Firmware").color(TEXT_PRIMARY).strong().size(15.0));
        ui.add_space(8.0);
        let rows = [
            ("Product Model", device.product_name.as_str()),
            ("Product Family", device.product_family.as_str()),
            ("Serial Number", device.serial_number.as_str()),
            ("Motherboard", device.board_name.as_str()),
            ("BIOS / UEFI Version", device.bios_version.as_str()),
            ("BIOS Release Date", device.bios_date.as_str()),
            ("Processor", device.cpu_brand.as_str()),
            ("Operating System", device.os_name.as_str()),
            ("Kernel", device.kernel_version.as_str()),
            ("Active Backend", backend_name),
        ];
        egui::Grid::new("sysinfo_grid")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for (k, v) in rows {
                    ui.label(RichText::new(k).color(TEXT_MUTED).size(13.0));
                    ui.label(RichText::new(v).color(TEXT_PRIMARY).size(13.0));
                    ui.end_row();
                }
                ui.label(RichText::new("CPU Logical Cores").color(TEXT_MUTED).size(13.0));
                ui.label(RichText::new(format!("{}", device.cpu_cores)).color(TEXT_PRIMARY));
                ui.end_row();
                ui.label(RichText::new("Installed Memory").color(TEXT_MUTED).size(13.0));
                ui.label(
                    RichText::new(format!("{:.1} GB", device.total_memory_gb)).color(TEXT_PRIMARY),
                );
                ui.end_row();
                ui.label(RichText::new("System Uptime").color(TEXT_MUTED).size(13.0));
                let hrs = telemetry.uptime_seconds / 3600;
                let mins = (telemetry.uptime_seconds % 3600) / 60;
                ui.label(RichText::new(format!("{hrs}h {mins}m")).color(TEXT_PRIMARY));
                ui.end_row();
            });
    });

    ui.add_space(12.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Linux Kernel Control Capabilities")
                .color(TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(8.0);
        let features = [
            ("asus-nb-wmi / asus-wmi Driver", device.asus_wmi_loaded),
            ("Battery Charge Limit Threshold", caps.battery_threshold_supported),
            ("ACPI / Thermal Fan Profile", caps.fan_profile_supported),
            ("ASUS Keyboard Backlight LED", caps.kbd_backlight_supported),
            ("Fn Lock Control", caps.fn_lock_supported),
            ("Touchpad Hardware Toggle", caps.touchpad_toggle_supported),
            ("Panel Overdrive (panel_od)", caps.panel_overdrive_supported),
            ("dGPU Eco Disable", caps.dgpu_disable_supported),
            ("Passwordless udev sysfs Write", caps.udev_writable),
        ];
        for (name, ok) in features {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).color(TEXT_PRIMARY).size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(
                        ui,
                        if ok { "Available" } else { "Unavailable" },
                        if ok { STATUS_OK } else { STATUS_WARN },
                    );
                });
            });
            ui.add_space(4.0);
        }
    });
}
