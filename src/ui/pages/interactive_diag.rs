use egui::{Color32, RichText, Rounding, Stroke, Ui, Vec2};

use crate::models::peripherals::ProcessEntry;
use crate::ui::theme::{card_frame, ASUS_BLUE, STATUS_OK, TEXT_MUTED, TEXT_PRIMARY};

pub struct InteractiveTestState {
    pub display_color_idx: usize,
    pub last_key_pressed: String,
    pub keys_tested: Vec<String>,
    pub left_clicks: u32,
    pub right_clicks: u32,
    pub scroll_events: u32,
}

impl Default for InteractiveTestState {
    fn default() -> Self {
        Self {
            display_color_idx: 0,
            last_key_pressed: "Press any key...".to_string(),
            keys_tested: Vec::new(),
            left_clicks: 0,
            right_clicks: 0,
            scroll_events: 0,
        }
    }
}

pub fn render_interactive_tests(
    ui: &mut Ui,
    state: &mut InteractiveTestState,
    top_processes: &[ProcessEntry],
) -> bool {
    let mut refresh_procs = false;

    // 1. Dead-Pixel / Display Uniformity Test (mirrors AsusSystemInteractiveDiagnosis)
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Interactive Display & Dead-Pixel Test")
                .color(TEXT_PRIMARY)
                .strong()
                .size(14.5),
        );
        ui.label(
            RichText::new("Click the color swatches or canvas to cycle Red / Green / Blue / White / Black uniformity.")
                .color(TEXT_MUTED)
                .size(12.0),
        );
        ui.add_space(6.0);

        let palette: [(&str, Color32); 5] = [
            ("Red", Color32::from_rgb(235, 45, 45)),
            ("Green", Color32::from_rgb(40, 210, 80)),
            ("Blue", Color32::from_rgb(35, 110, 245)),
            ("White", Color32::WHITE),
            ("Black", Color32::BLACK),
        ];

        ui.horizontal(|ui| {
            for (idx, (label, color)) in palette.iter().enumerate() {
                let active = state.display_color_idx == idx;
                let btn = egui::Button::new(*label)
                    .fill(if active { ASUS_BLUE } else { Color32::from_rgb(44, 48, 58) });
                if ui.add(btn).clicked() {
                    state.display_color_idx = idx;
                }
                let _ = color;
            }
        });
        ui.add_space(6.0);
        let (_, active_color) = palette[state.display_color_idx % palette.len()];
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 54.0),
            egui::Sense::click(),
        );
        ui.painter().rect_filled(rect, Rounding::same(6.0), active_color);
        ui.painter()
            .rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, Color32::GRAY));
        if resp.clicked() {
            state.display_color_idx = (state.display_color_idx + 1) % palette.len();
        }
    });

    ui.add_space(10.0);

    // 2. Keyboard & Touchpad Live Input Tester
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Keyboard Live Input Test")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(14.0),
            );
            ui.input(|i| {
                for ev in &i.events {
                    if let egui::Event::Key {
                        key, pressed: true, ..
                    } = ev
                    {
                        let name = format!("{key:?}");
                        state.last_key_pressed = name.clone();
                        if !state.keys_tested.contains(&name) {
                            state.keys_tested.push(name);
                        }
                    }
                }
            });
            ui.label(
                RichText::new(format!("Last Key: {}", state.last_key_pressed))
                    .color(STATUS_OK)
                    .strong(),
            );
            ui.label(
                RichText::new(format!(
                    "Unique keys verified: {} ({})",
                    state.keys_tested.len(),
                    state
                        .keys_tested
                        .iter()
                        .rev()
                        .take(8)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .color(TEXT_MUTED)
                .size(11.5),
            );
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Touchpad & Pointer Test")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(14.0),
            );
            let (pad, resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), egui::Sense::click());
            ui.painter()
                .rect_filled(pad, Rounding::same(6.0), Color32::from_rgb(26, 29, 36));
            ui.painter()
                .rect_stroke(pad, Rounding::same(6.0), Stroke::new(1.0_f32, ASUS_BLUE));
            ui.painter().text(
                pad.center(),
                egui::Align2::CENTER_CENTER,
                "Click / Right-Click / Scroll Here",
                egui::FontId::proportional(12.0),
                TEXT_MUTED,
            );
            if resp.clicked() {
                state.left_clicks += 1;
            }
            if resp.secondary_clicked() {
                state.right_clicks += 1;
            }
            if resp.hovered() && ui.input(|i| i.raw_scroll_delta.y.abs() > 0.1) {
                state.scroll_events += 1;
            }
            ui.label(
                RichText::new(format!(
                    "Left: {}  •  Right: {}  •  Scroll: {}",
                    state.left_clicks, state.right_clicks, state.scroll_events
                ))
                .color(TEXT_MUTED)
                .size(11.5),
            );
        });
    });

    ui.add_space(10.0);

    // 3. Top Resource Processes (mirrors ClientProcessPerfEnum)
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Top Resource Processes (Memory / CPU)")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(14.5),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Refresh Processes").clicked() {
                    refresh_procs = true;
                }
            });
        });
        ui.add_space(6.0);
        egui::Grid::new("proc_perf_grid")
            .num_columns(4)
            .spacing([18.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("PID").color(TEXT_MUTED).strong());
                ui.label(RichText::new("Process").color(TEXT_MUTED).strong());
                ui.label(RichText::new("Memory").color(TEXT_MUTED).strong());
                ui.label(RichText::new("CPU").color(TEXT_MUTED).strong());
                ui.end_row();
                for p in top_processes.iter().take(8) {
                    ui.label(RichText::new(format!("{}", p.pid)).color(TEXT_MUTED).size(12.0));
                    ui.label(RichText::new(&p.name).color(TEXT_PRIMARY).size(12.0));
                    ui.label(
                        RichText::new(format!("{:.0} MB", p.memory_mb))
                            .color(ASUS_BLUE)
                            .size(12.0),
                    );
                    ui.label(
                        RichText::new(format!("{:.1}%", p.cpu_percent))
                            .color(TEXT_MUTED)
                            .size(12.0),
                    );
                    ui.end_row();
                }
            });
    });

    refresh_procs
}
