use egui::{RichText, Ui};

use crate::models::peripherals::{EventSeverity, SystemEvent};
use crate::ui::icons::draw_status_dot;
use crate::ui::theme::{
    card_frame, pill_button, section_heading, status_badge, ASUS_BLUE, STATUS_DANGER, STATUS_OK,
    STATUS_WARN, TEXT_MUTED, TEXT_PRIMARY,
};

pub enum MessageAction {
    MarkAllRead,
    ClearAll,
}

pub fn render_messages(ui: &mut Ui, events: &[SystemEvent]) -> Option<MessageAction> {
    let mut action = None;

    section_heading(
        ui,
        "Message Center",
        "Hardware alerts, thermal warnings, battery threshold changes, and diagnostic history",
    );

    ui.horizontal(|ui| {
        if pill_button(ui, "Mark All Read", false).clicked() {
            action = Some(MessageAction::MarkAllRead);
        }
        if pill_button(ui, "Clear History", false).clicked() {
            action = Some(MessageAction::ClearAll);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let unread = events.iter().filter(|e| !e.read).count();
            ui.label(
                RichText::new(format!("{} total  •  {} unread", events.len(), unread))
                    .color(TEXT_MUTED)
                    .size(12.5),
            );
        });
    });

    ui.add_space(12.0);

    if events.is_empty() {
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("No hardware messages or alerts yet.").color(TEXT_MUTED));
        });
        return action;
    }

    for ev in events.iter().rev() {
        let (badge, color) = match ev.severity {
            EventSeverity::Info => ("Info", ASUS_BLUE),
            EventSeverity::Success => ("OK", STATUS_OK),
            EventSeverity::Warning => ("Warning", STATUS_WARN),
            EventSeverity::Alert => ("Alert", STATUS_DANGER),
        };
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if !ev.read {
                    draw_status_dot(ui, 4.0, ASUS_BLUE);
                }
                let title = if ev.read {
                    RichText::new(&ev.title).color(TEXT_PRIMARY).size(14.0)
                } else {
                    RichText::new(&ev.title)
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(14.0)
                };
                ui.label(title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_badge(ui, badge, color);
                    ui.label(RichText::new(&ev.timestamp).color(TEXT_MUTED).size(11.5));
                });
            });
            ui.add_space(4.0);
            ui.label(RichText::new(&ev.detail).color(TEXT_MUTED).size(12.5));
        });
        ui.add_space(8.0);
    }

    action
}
