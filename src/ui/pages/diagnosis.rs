use egui::{Color32, RichText, Ui};

use crate::models::{
    diagnosis::{DiagnosisReport, DiagnosticCategory, DiagnosticStatus},
    peripherals::ProcessEntry,
};
use crate::ui::pages::interactive_diag::{render_interactive_tests, InteractiveTestState};
use crate::ui::theme::{
    card_frame, pill_button, section_heading, status_badge, ASUS_BLUE, STATUS_DANGER, STATUS_OK,
    STATUS_WARN, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosisTab {
    AutomatedChecks,
    InteractiveSuite,
}

pub enum DiagnosisAction {
    RunAll,
    RunCategory(DiagnosticCategory),
    RefreshProcesses,
}

pub fn render_diagnosis(
    ui: &mut Ui,
    tab: &mut DiagnosisTab,
    report: &DiagnosisReport,
    interactive: &mut InteractiveTestState,
    top_processes: &[ProcessEntry],
) -> Option<DiagnosisAction> {
    let mut action = None;

    section_heading(
        ui,
        "System Diagnosis",
        "Automated 6-subsystem checkup + Interactive Display, Keyboard, Touchpad & Process Diagnostics",
    );

    ui.horizontal(|ui| {
        for (t, label) in [
            (DiagnosisTab::AutomatedChecks, "Automated Hardware Checkup"),
            (DiagnosisTab::InteractiveSuite, "Interactive Self-Test & Processes"),
        ] {
            let selected = *tab == t;
            let btn = egui::Button::new(
                RichText::new(label).color(if selected { Color32::WHITE } else { TEXT_MUTED }),
            )
            .fill(if selected { ASUS_BLUE } else { Color32::from_rgb(35, 38, 46) });
            if ui.add(btn).clicked() {
                *tab = t;
            }
        }
    });
    ui.add_space(12.0);

    if *tab == DiagnosisTab::InteractiveSuite {
        if render_interactive_tests(ui, interactive, top_processes) {
            action = Some(DiagnosisAction::RefreshProcesses);
        }
        return action;
    }

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("One-Click Hardware Diagnosis")
                        .color(TEXT_PRIMARY)
                        .strong()
                        .size(16.0),
                );
                let ts = if report.timestamp.is_empty() {
                    "Run a comprehensive check of all 6 subsystems.".to_string()
                } else {
                    format!(
                        "Last run: {}  •  Healthy: {}  •  Attention: {}  •  Issues: {}",
                        report.timestamp,
                        report.count_by_status(DiagnosticStatus::Passed),
                        report.count_by_status(DiagnosticStatus::Warning),
                        report.count_by_status(DiagnosticStatus::Issue),
                    )
                };
                ui.label(RichText::new(ts).color(TEXT_MUTED).size(12.5));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, "Run All Checks", true).clicked() {
                    action = Some(DiagnosisAction::RunAll);
                }
            });
        });
    });

    ui.add_space(12.0);

    for &category in DiagnosticCategory::all() {
        let existing = report.items.iter().find(|i| i.category == category);
        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(category.title()).color(TEXT_PRIMARY).strong().size(14.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Check").clicked() {
                        action = Some(DiagnosisAction::RunCategory(category));
                    }
                    if let Some(item) = existing {
                        let color = match item.status {
                            DiagnosticStatus::Passed => STATUS_OK,
                            DiagnosticStatus::Warning => STATUS_WARN,
                            DiagnosticStatus::Issue => STATUS_DANGER,
                            DiagnosticStatus::Skipped => TEXT_MUTED,
                        };
                        status_badge(ui, item.status.label(), color);
                    }
                });
            });

            if let Some(item) = existing {
                ui.add_space(4.0);
                ui.label(RichText::new(&item.summary).color(TEXT_PRIMARY).size(13.0));
                for d in &item.details {
                    ui.label(RichText::new(format!("• {d}")).color(TEXT_MUTED).size(12.0));
                }
                if let Some(rec) = &item.recommendation {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Tip: {rec}"))
                            .color(STATUS_WARN)
                            .size(12.0),
                    );
                }
            } else {
                ui.label(RichText::new("Not tested yet.").color(TEXT_MUTED).size(12.0));
            }
        });
        ui.add_space(8.0);
    }

    action
}
