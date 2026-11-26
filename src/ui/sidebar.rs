use egui::{Color32, Margin, Rounding, Stroke, Ui, Vec2};

use crate::ui::icons::{draw_icon, draw_status_dot, Icon};
use crate::ui::theme::{
    ASUS_BLUE, BG_SIDEBAR, BORDER_SUBTLE, STATUS_DANGER, STATUS_OK, STATUS_WARN, TEXT_MUTED,
    TEXT_PRIMARY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Home,
    DeviceSettings,
    SystemDiagnosis,
    SystemUpdate,
    MyAsusDevices,
    MessageCenter,
    CustomerSupport,
    SystemInfo,
    Settings,
}

impl NavPage {
    pub fn primary_items() -> &'static [(NavPage, Icon, &'static str)] {
        &[
            (NavPage::Home, Icon::Home, "Home"),
            (NavPage::DeviceSettings, Icon::Sliders, "Device Settings"),
            (NavPage::SystemDiagnosis, Icon::Wrench, "System Diagnosis"),
            (NavPage::SystemUpdate, Icon::Update, "System Update"),
        ]
    }

    pub fn ecosystem_items() -> &'static [(NavPage, Icon, &'static str)] {
        &[
            (NavPage::MyAsusDevices, Icon::Devices, "My ASUS Devices"),
            (NavPage::MessageCenter, Icon::Message, "Message Center"),
        ]
    }

    pub fn support_items() -> &'static [(NavPage, Icon, &'static str)] {
        &[
            (NavPage::CustomerSupport, Icon::Support, "Customer Support"),
            (NavPage::SystemInfo, Icon::Info, "System Info"),
            (NavPage::Settings, Icon::Gear, "Settings"),
        ]
    }
}

pub fn render_sidebar(
    ui: &mut Ui,
    active_page: &mut NavPage,
    os_label: &str,
    asus_wmi_loaded: bool,
    unread_messages: usize,
) {
    egui::Frame::none()
        .fill(BG_SIDEBAR)
        .inner_margin(Margin::symmetric(0.0, 12.0))
        .show(ui, |ui| {
            ui.set_min_height(ui.available_height());

            ui.horizontal(|ui| {
                ui.add_space(14.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(rect.center(), 17.0, Color32::from_rgb(56, 62, 74));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "A",
                    egui::FontId::proportional(16.0),
                    TEXT_PRIMARY,
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("MyASUS Rust")
                            .color(TEXT_PRIMARY)
                            .strong()
                            .size(14.0),
                    );
                    ui.label(
                        egui::RichText::new(if cfg!(target_os = "windows") {
                            "Windows Native"
                        } else {
                            "Linux Mint / Ubuntu"
                        })
                        .color(TEXT_MUTED)
                        .size(11.0),
                    );
                });
            });

            ui.add_space(14.0);

            for &(page, icon, label) in NavPage::primary_items() {
                nav_item(ui, active_page, page, icon, label, 0);
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            for &(page, icon, label) in NavPage::ecosystem_items() {
                let badge = if page == NavPage::MessageCenter {
                    unread_messages
                } else {
                    0
                };
                nav_item(ui, active_page, page, icon, label, badge);
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            for &(page, icon, label) in NavPage::support_items() {
                nav_item(ui, active_page, page, icon, label, 0);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgb(22, 24, 29))
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(10.0))
                    .outer_margin(Margin::symmetric(12.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let (dot_color, drv_text) = if asus_wmi_loaded {
                            let label = if cfg!(target_os = "windows") {
                                "ATKACPI Active"
                            } else {
                                "asus-nb-wmi Active"
                            };
                            (STATUS_OK, label)
                        } else {
                            (STATUS_WARN, "Generic Hardware")
                        };
                        ui.horizontal(|ui| {
                            draw_status_dot(ui, 4.0, dot_color);
                            ui.label(egui::RichText::new(drv_text).color(TEXT_PRIMARY).size(11.5));
                        });
                        ui.label(egui::RichText::new(os_label).color(TEXT_MUTED).size(10.5));
                    });
            });
        });
}

fn nav_item(
    ui: &mut Ui,
    active: &mut NavPage,
    target: NavPage,
    icon: Icon,
    label: &str,
    badge_count: usize,
) {
    let selected = *active == target;
    let fill = if selected {
        ASUS_BLUE
    } else {
        Color32::TRANSPARENT
    };
    let text_color = if selected { Color32::WHITE } else { TEXT_PRIMARY };

    let resp = egui::Frame::none()
        .fill(fill)
        .inner_margin(Margin::symmetric(16.0, 8.5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                draw_icon(ui, icon, 16.0, text_color);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(label).color(text_color).size(13.0));
                if badge_count > 0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::Frame::none()
                            .fill(STATUS_DANGER)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(6.0, 1.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{badge_count}"))
                                        .color(Color32::WHITE)
                                        .size(10.5)
                                        .strong(),
                                );
                            });
                    });
                }
            });
        })
        .response
        .interact(egui::Sense::click());

    if resp.hovered() && !selected {
        ui.painter()
            .rect_filled(resp.rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 12));
    }
    if resp.clicked() {
        *active = target;
    }
}
