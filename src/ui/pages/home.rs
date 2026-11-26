use egui::{Color32, FontId, Pos2, Rect, RichText, Rounding, Stroke, Ui, Vec2};

use crate::models::{
    system::{DeviceInfo, TelemetrySnapshot},
    updates::UpdateReport,
};
use crate::services::config::AppConfig;
use crate::ui::icons::{draw_icon, Icon};
use crate::ui::sidebar::NavPage;
use crate::ui::theme::{
    blue_top_card, card_frame, pill_button, ASUS_BLUE, STATUS_DANGER, STATUS_OK, STATUS_WARN,
    TEXT_MUTED, TEXT_PRIMARY,
};

pub enum HomeAction {
    Navigate(NavPage),
    OpenQuickSettings,
    RunOneClickDiagnosis,
    CheckUpdates,
    CopySerial,
}

pub fn render_home(
    ui: &mut Ui,
    device: &DeviceInfo,
    telemetry: &TelemetrySnapshot,
    cpu_history: &[f32],
    updates: &UpdateReport,
    config: &AppConfig,
) -> Option<HomeAction> {
    let mut action = None;

    // Hero banner with subtle ASUS blue wave curves
    let banner_rect = Rect::from_min_size(
        ui.cursor().min,
        Vec2::new(ui.available_width(), 108.0),
    );
    draw_wave_backdrop(ui, banner_rect);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&device.product_name)
                    .font(FontId::proportional(28.0))
                    .color(TEXT_PRIMARY)
                    .strong(),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Serial number  {}", device.serial_number))
                        .color(TEXT_MUTED)
                        .size(13.0),
                );
                let copy_resp = ui
                    .allocate_response(Vec2::splat(20.0), egui::Sense::click())
                    .on_hover_text("Copy serial number");
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(copy_resp.rect), |ui| {
                    draw_icon(
                        ui,
                        Icon::Copy,
                        15.0,
                        if copy_resp.hovered() { TEXT_PRIMARY } else { TEXT_MUTED },
                    );
                });
                if copy_resp.clicked() {
                    action = Some(HomeAction::CopySerial);
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .link(RichText::new("System info").color(Color32::from_rgb(76, 166, 255)))
                    .clicked()
                {
                    action = Some(HomeAction::Navigate(NavPage::SystemInfo));
                }
                ui.label(RichText::new("|").color(TEXT_MUTED));
                let chip_label = if device.asus_wmi_loaded {
                    if cfg!(target_os = "windows") {
                        "ATKACPI Hardware Active"
                    } else {
                        "asus-nb-wmi Active"
                    }
                } else {
                    "Hardware Controls"
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(chip_label)
                                .color(Color32::from_rgb(100, 180, 255))
                                .size(12.0),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0_f32, ASUS_BLUE))
                        .rounding(Rounding::same(14.0)),
                    )
                    .clicked()
                {
                    action = Some(HomeAction::Navigate(NavPage::DeviceSettings));
                }
            });
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Quick setting").color(TEXT_PRIMARY).size(13.0))
                        .fill(Color32::from_rgb(82, 86, 98))
                        .rounding(Rounding::same(16.0))
                        .min_size(Vec2::new(118.0, 30.0)),
                )
                .clicked()
            {
                action = Some(HomeAction::OpenQuickSettings);
            }
        });
    });

    ui.add_space(16.0);

    // Middle split: Left = Laptop visual + Battery summary; Right = 2x2 Blue-Top Telemetry Cards
    ui.columns(2, |cols| {
        // Left column: Laptop illustration + battery/kernel summary card
        cols[0].vertical_centered(|ui| {
            ui.add_space(8.0);
            draw_laptop_illustration(ui, &device.product_family);
            ui.add_space(18.0);
        });

        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Battery & System Status")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(14.5),
            );
            ui.add_space(4.0);
            if telemetry.battery.present {
                let b = &telemetry.battery;
                ui.label(
                    RichText::new(format!(
                        "Charge: {}% ({})  •  AC: {}",
                        b.percentage,
                        b.status,
                        if b.ac_online { "Plugged In" } else { "Discharging" }
                    ))
                    .color(TEXT_MUTED)
                    .size(12.5),
                );
                if let Some(health) = b.health_percent {
                    ui.label(
                        RichText::new(format!(
                            "Battery health: {:.1}%   •   Charge limit: {}%",
                            health,
                            b.charge_limit.unwrap_or(100)
                        ))
                        .color(TEXT_MUTED)
                        .size(12.0),
                    );
                }
            } else {
                ui.label(RichText::new("Desktop / AC power mode").color(TEXT_MUTED));
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("OS: {}  •  Kernel: {}", device.os_name, device.kernel_version))
                    .color(TEXT_MUTED)
                    .size(11.5),
            );
        });

        // Right column: 2x2 MyASUS Telemetry Grid
        cols[1].columns(2, |grid| {
            // Card 1: CPU Load + Temp + Live Sparkline
            blue_top_card(&mut grid[0], 128.0, |ui| {
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Cpu, 20.0, TEXT_PRIMARY);
                    ui.add_space(4.0);
                    ui.label(RichText::new("CPU load").color(TEXT_MUTED).size(13.0));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:.0}%", telemetry.cpu_usage_percent))
                            .font(FontId::proportional(26.0))
                            .color(TEXT_PRIMARY)
                            .strong(),
                    );
                    if let Some(temp) = telemetry.cpu_temp_celsius {
                        let temp_color = if temp >= 90.0 {
                            STATUS_DANGER
                        } else if temp >= 78.0 {
                            STATUS_WARN
                        } else {
                            TEXT_PRIMARY
                        };
                        ui.label(
                            RichText::new("|")
                                .font(FontId::proportional(20.0))
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(config.format_temp(temp))
                                .font(FontId::proportional(24.0))
                                .color(temp_color)
                                .strong(),
                        );
                    }
                });
                ui.add_space(6.0);
                draw_sparkline(ui, cpu_history, ASUS_BLUE);
            });

            // Card 2: Memory
            blue_top_card(&mut grid[1], 128.0, |ui| {
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Memory, 20.0, TEXT_PRIMARY);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Memory").color(TEXT_MUTED).size(13.0));
                });
                ui.add_space(14.0);
                ui.label(
                    RichText::new(format!("{:.0}%", telemetry.memory_usage_percent))
                        .font(FontId::proportional(28.0))
                        .color(TEXT_PRIMARY)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "{:.1} / {:.1} GB",
                        telemetry.memory_used_gb, telemetry.memory_total_gb
                    ))
                    .color(TEXT_MUTED)
                    .size(11.5),
                );
            });
        });

        cols[1].add_space(10.0);

        cols[1].columns(2, |grid| {
            // Card 3: Hard disk space
            blue_top_card(&mut grid[0], 128.0, |ui| {
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Disk, 20.0, TEXT_PRIMARY);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Hard disk space").color(TEXT_MUTED).size(13.0));
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(82.0).show(ui, |ui| {
                    for disk in telemetry.disks.iter().take(4) {
                        let color = if disk.usage_percent >= 88.0 {
                            STATUS_DANGER
                        } else if disk.usage_percent >= 75.0 {
                            STATUS_WARN
                        } else {
                            STATUS_OK
                        };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&disk.mount_point).color(TEXT_PRIMARY).size(12.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.0}%", disk.usage_percent))
                                        .color(color)
                                        .strong()
                                        .size(12.0),
                                );
                            });
                        });
                        ui.label(
                            RichText::new(format!("{:.0}/{:.0} GB", disk.used_gb, disk.total_gb))
                                .color(TEXT_MUTED)
                                .size(11.0),
                        );
                    }
                });
            });

            // Card 4: Fan RPM
            blue_top_card(&mut grid[1], 128.0, |ui| {
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Fan, 20.0, TEXT_PRIMARY);
                    ui.add_space(4.0);
                    ui.label(RichText::new("Fan").color(TEXT_MUTED).size(13.0));
                });
                ui.add_space(10.0);
                let cpu_fan = telemetry
                    .fans
                    .cpu_fan_rpm
                    .map(|r| format!("{r} RPM"))
                    .unwrap_or_else(|| "N/A".into());
                let gpu_fan = telemetry
                    .fans
                    .gpu_fan_rpm
                    .map(|r| format!("{r} RPM"))
                    .unwrap_or_else(|| "N/A".into());
                ui.label(RichText::new("CPU fan").color(TEXT_MUTED).size(11.5));
                ui.label(RichText::new(cpu_fan).color(TEXT_PRIMARY).strong().size(14.0));
                ui.add_space(6.0);
                ui.label(RichText::new("GPU fan").color(TEXT_MUTED).size(11.5));
                ui.label(RichText::new(gpu_fan).color(TEXT_PRIMARY).strong().size(14.0));
            });
        });
    });

    ui.add_space(16.0);

    // Bottom 2 Action Cards: System Diagnosis & System Update (matching screenshot!)
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.set_min_height(92.0);
            ui.set_width(ui.available_width());
            ui.label(RichText::new("System Diagnosis").color(TEXT_PRIMARY).strong().size(15.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new("You can check your device's health via this diagnosis tool.")
                    .color(TEXT_MUTED)
                    .size(12.5),
            );
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                if pill_button(ui, "One-click diagnosis", false).clicked() {
                    action = Some(HomeAction::RunOneClickDiagnosis);
                }
            });
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.set_min_height(92.0);
            ui.set_width(ui.available_width());
            ui.label(RichText::new("System Update").color(TEXT_PRIMARY).strong().size(15.0));
            ui.add_space(4.0);
            let update_summary = if updates.items.is_empty() {
                "Check UEFI/BIOS (LVFS fwupd) and Ubuntu/Mint APT packages.".to_string()
            } else {
                format!(
                    "{} item(s) available for update. Click \"Check update items\" for details.",
                    updates.items.len()
                )
            };
            ui.label(RichText::new(update_summary).color(TEXT_MUTED).size(12.5));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let checked = if updates.last_checked.is_empty() {
                    "Not checked yet".to_string()
                } else {
                    format!("Last checked: {}", updates.last_checked)
                };
                ui.label(RichText::new(checked).color(TEXT_MUTED).size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, "Check update items", false).clicked() {
                        action = Some(HomeAction::CheckUpdates);
                    }
                });
            });
        });
    });

    action
}

fn draw_wave_backdrop(ui: &mut Ui, rect: Rect) {
    let painter = ui.painter();
    for i in 0..5 {
        let offset = i as f32 * 6.0;
        let alpha = 28u8.saturating_sub(i as u8 * 4);
        let color = Color32::from_rgba_unmultiplied(0, 120, 240, alpha);
        let mut pts = Vec::with_capacity(24);
        for step in 0..=23 {
            let t = step as f32 / 23.0;
            let x = rect.left() + t * rect.width();
            let y = rect.center().y + (t * std::f32::consts::TAU * 1.1 + i as f32 * 0.4).sin() * 18.0
                + offset;
            pts.push(Pos2::new(x, y));
        }
        painter.add(egui::Shape::line(pts, Stroke::new(2.0_f32, color)));
    }
}

fn draw_laptop_illustration(ui: &mut Ui, family: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(220.0, 126.0), egui::Sense::hover());
    let p = ui.painter();

    let screen_outer = Rect::from_min_max(
        Pos2::new(rect.left() + 26.0, rect.top() + 6.0),
        Pos2::new(rect.right() - 26.0, rect.bottom() - 24.0),
    );
    p.rect_filled(screen_outer, 6.0, Color32::from_rgb(20, 22, 28));
    p.rect_stroke(screen_outer, 6.0, Stroke::new(2.0_f32, Color32::from_rgb(130, 138, 152)));

    let inner = screen_outer.shrink(6.0);
    p.rect_filled(inner, 3.0, Color32::from_rgb(18, 56, 118));
    let glow = Rect::from_min_max(
        Pos2::new(inner.left(), inner.center().y - 14.0),
        inner.right_bottom(),
    );
    p.rect_filled(glow, 3.0, Color32::from_rgba_unmultiplied(0, 120, 240, 140));
    p.text(
        inner.center(),
        egui::Align2::CENTER_CENTER,
        family,
        FontId::proportional(13.0),
        Color32::WHITE,
    );

    // Base chassis
    let base = Rect::from_min_max(
        Pos2::new(rect.left() + 8.0, rect.bottom() - 22.0),
        Pos2::new(rect.right() - 8.0, rect.bottom() - 10.0),
    );
    p.rect_filled(base, 4.0, Color32::from_rgb(186, 192, 204));
    p.rect_filled(
        Rect::from_center_size(Pos2::new(base.center().x, base.top() + 3.0), Vec2::new(34.0, 4.0)),
        2.0,
        Color32::from_rgb(90, 96, 108),
    );
}

fn draw_sparkline(ui: &mut Ui, values: &[f32], color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), egui::Sense::hover());
    if values.len() < 2 {
        return;
    }
    let step = rect.width() / (values.len() - 1) as f32;
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.left() + i as f32 * step;
            let norm = (v / 100.0).clamp(0.0, 1.0);
            let y = rect.bottom() - norm * rect.height();
            Pos2::new(x, y)
        })
        .collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.8_f32, color)));
}
