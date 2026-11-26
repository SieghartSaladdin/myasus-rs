use std::f32::consts::TAU;

use egui::{pos2, vec2, Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::ui::theme::ASUS_BLUE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Home,
    Sliders,
    Wrench,
    Update,
    Devices,
    Message,
    Support,
    Info,
    Gear,
    Cpu,
    Memory,
    Disk,
    Fan,
    Copy,
}

/// Draws a razor-sharp vector icon via `egui::Painter` so icons render identically
/// on Windows, Ubuntu, and Linux Mint with zero missing-glyph boxes.
pub fn draw_icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    let c = rect.center();
    let s = size * 0.5;
    let stroke = Stroke::new((size * 0.095).clamp(1.3, 2.2), color);

    match icon {
        Icon::Home => {
            let roof = vec![
                pos2(c.x - s * 0.85, c.y - s * 0.05),
                pos2(c.x, c.y - s * 0.82),
                pos2(c.x + s * 0.85, c.y - s * 0.05),
            ];
            p.add(egui::Shape::line(roof, stroke));
            let body = vec![
                pos2(c.x - s * 0.62, c.y - s * 0.15),
                pos2(c.x - s * 0.62, c.y + s * 0.78),
                pos2(c.x + s * 0.62, c.y + s * 0.78),
                pos2(c.x + s * 0.62, c.y - s * 0.15),
            ];
            p.add(egui::Shape::line(body, stroke));
            let door = Rect::from_min_max(
                pos2(c.x - s * 0.2, c.y + s * 0.2),
                pos2(c.x + s * 0.2, c.y + s * 0.78),
            );
            p.rect_stroke(door, 1.0, stroke);
        }
        Icon::Sliders => {
            for (idx, knob_y) in [(-0.55_f32, -0.25_f32), (0.0, 0.28), (0.55, -0.1)] {
                let x = c.x + s * idx;
                p.line_segment([pos2(x, c.y - s * 0.75), pos2(x, c.y + s * 0.75)], stroke);
                p.circle_filled(pos2(x, c.y + s * knob_y), size * 0.12, color);
            }
        }
        Icon::Wrench => {
            p.line_segment(
                [
                    pos2(c.x - s * 0.65, c.y + s * 0.65),
                    pos2(c.x + s * 0.35, c.y - s * 0.35),
                ],
                Stroke::new((size * 0.14).clamp(1.8, 2.8), color),
            );
            p.circle_stroke(pos2(c.x + s * 0.42, c.y - s * 0.42), s * 0.38, stroke);
            p.circle_filled(
                pos2(c.x + s * 0.56, c.y - s * 0.56),
                s * 0.18,
                ui.visuals().panel_fill,
            );
        }
        Icon::Update => {
            draw_arc(p, c, s * 0.7, 0.25, 2.75, stroke);
            draw_arc(p, c, s * 0.7, 3.4, 5.9, stroke);
            let a1 = pos2(c.x + s * 0.68, c.y - s * 0.2);
            p.line_segment([a1, pos2(a1.x - s * 0.3, a1.y - s * 0.22)], stroke);
            p.line_segment([a1, pos2(a1.x + s * 0.08, a1.y - s * 0.35)], stroke);
            let a2 = pos2(c.x - s * 0.68, c.y + s * 0.2);
            p.line_segment([a2, pos2(a2.x + s * 0.3, a2.y + s * 0.22)], stroke);
            p.line_segment([a2, pos2(a2.x - s * 0.08, a2.y + s * 0.35)], stroke);
        }
        Icon::Devices => {
            let screen = Rect::from_min_max(
                pos2(c.x - s * 0.75, c.y - s * 0.55),
                pos2(c.x + s * 0.35, c.y + s * 0.25),
            );
            p.rect_stroke(screen, 1.5, stroke);
            p.line_segment(
                [
                    pos2(c.x - s * 0.85, c.y + s * 0.45),
                    pos2(c.x + s * 0.2, c.y + s * 0.45),
                ],
                stroke,
            );
            let phone = Rect::from_min_max(
                pos2(c.x + s * 0.25, c.y - s * 0.15),
                pos2(c.x + s * 0.82, c.y + s * 0.72),
            );
            p.rect_filled(phone, 1.5, ui.visuals().panel_fill);
            p.rect_stroke(phone, 1.5, stroke);
        }
        Icon::Message => {
            let box_r = Rect::from_min_max(
                pos2(c.x - s * 0.78, c.y - s * 0.62),
                pos2(c.x + s * 0.78, c.y + s * 0.38),
            );
            p.rect_stroke(box_r, 2.5, stroke);
            p.line_segment(
                [pos2(c.x - s * 0.3, c.y + s * 0.38), pos2(c.x - s * 0.48, c.y + s * 0.76)],
                stroke,
            );
            p.line_segment(
                [pos2(c.x - s * 0.48, c.y + s * 0.76), pos2(c.x + s * 0.05, c.y + s * 0.38)],
                stroke,
            );
            p.line_segment(
                [pos2(c.x - s * 0.42, c.y - s * 0.2), pos2(c.x + s * 0.42, c.y - s * 0.2)],
                stroke,
            );
            p.line_segment(
                [pos2(c.x - s * 0.42, c.y + s * 0.08), pos2(c.x + s * 0.18, c.y + s * 0.08)],
                stroke,
            );
        }
        Icon::Support => {
            draw_arc(p, pos2(c.x, c.y + s * 0.05), s * 0.72, std::f32::consts::PI, TAU, stroke);
            let left_ear = Rect::from_min_max(
                pos2(c.x - s * 0.85, c.y - s * 0.02),
                pos2(c.x - s * 0.52, c.y + s * 0.55),
            );
            let right_ear = Rect::from_min_max(
                pos2(c.x + s * 0.52, c.y - s * 0.02),
                pos2(c.x + s * 0.85, c.y + s * 0.55),
            );
            p.rect_stroke(left_ear, 2.0, stroke);
            p.rect_stroke(right_ear, 2.0, stroke);
            p.line_segment(
                [pos2(c.x + s * 0.65, c.y + s * 0.55), pos2(c.x + s * 0.18, c.y + s * 0.78)],
                stroke,
            );
        }
        Icon::Info => {
            p.circle_stroke(c, s * 0.78, stroke);
            p.circle_filled(pos2(c.x, c.y - s * 0.34), size * 0.075, color);
            p.line_segment(
                [pos2(c.x, c.y - s * 0.08), pos2(c.x, c.y + s * 0.42)],
                stroke,
            );
        }
        Icon::Gear => {
            p.circle_stroke(c, s * 0.34, stroke);
            p.circle_stroke(c, s * 0.64, stroke);
            for i in 0..8 {
                let ang = (i as f32 / 8.0) * TAU;
                let p1 = pos2(c.x + ang.cos() * s * 0.62, c.y + ang.sin() * s * 0.62);
                let p2 = pos2(c.x + ang.cos() * s * 0.88, c.y + ang.sin() * s * 0.88);
                p.line_segment([p1, p2], stroke);
            }
        }
        Icon::Cpu => {
            let screen = Rect::from_min_max(
                pos2(c.x - s * 0.78, c.y - s * 0.65),
                pos2(c.x + s * 0.78, c.y + s * 0.35),
            );
            p.rect_filled(screen.shrink(2.0), 1.5, Color32::from_rgba_unmultiplied(0, 120, 240, 120));
            p.rect_stroke(screen, 2.0, stroke);
            p.line_segment(
                [pos2(c.x - s * 0.92, c.y + s * 0.6), pos2(c.x + s * 0.92, c.y + s * 0.6)],
                stroke,
            );
        }
        Icon::Memory => {
            p.circle_filled(c, s * 0.76, Color32::from_rgba_unmultiplied(0, 120, 240, 150));
            p.circle_stroke(c, s * 0.78, stroke);
            let slice = vec![
                c,
                pos2(c.x, c.y - s * 0.76),
                pos2(c.x + s * 0.76, c.y),
            ];
            p.add(egui::Shape::convex_polygon(slice, ASUS_BLUE, Stroke::NONE));
            p.line_segment([c, pos2(c.x, c.y - s * 0.76)], stroke);
            p.line_segment([c, pos2(c.x + s * 0.76, c.y)], stroke);
        }
        Icon::Disk => {
            let chassis = Rect::from_min_max(
                pos2(c.x - s * 0.8, c.y - s * 0.58),
                pos2(c.x + s * 0.8, c.y + s * 0.58),
            );
            p.rect_stroke(chassis, 2.5, stroke);
            p.line_segment(
                [pos2(c.x - s * 0.78, c.y + s * 0.14), pos2(c.x + s * 0.78, c.y + s * 0.14)],
                stroke,
            );
            p.circle_filled(pos2(c.x + s * 0.45, c.y + s * 0.36), size * 0.08, ASUS_BLUE);
        }
        Icon::Fan => {
            p.circle_filled(c, s * 0.76, Color32::from_rgba_unmultiplied(0, 120, 240, 130));
            p.circle_stroke(c, s * 0.78, stroke);
            for i in 0..4 {
                let ang = (i as f32 / 4.0) * TAU + 0.4;
                let tip = pos2(c.x + ang.cos() * s * 0.58, c.y + ang.sin() * s * 0.58);
                let ctrl = pos2(
                    c.x + (ang + 0.55).cos() * s * 0.46,
                    c.y + (ang + 0.55).sin() * s * 0.46,
                );
                p.add(egui::Shape::convex_polygon(
                    vec![c, tip, ctrl],
                    Color32::from_rgb(215, 232, 255),
                    Stroke::NONE,
                ));
            }
            p.circle_filled(c, size * 0.12, color);
        }
        Icon::Copy => {
            let back = Rect::from_min_size(pos2(c.x - s * 0.65, c.y - s * 0.7), vec2(s * 0.95, s * 1.1));
            let front = Rect::from_min_size(pos2(c.x - s * 0.25, c.y - s * 0.35), vec2(s * 0.95, s * 1.1));
            p.rect_stroke(back, Rounding::same(1.5), stroke);
            p.rect_filled(front, Rounding::same(1.5), ui.visuals().panel_fill);
            p.rect_stroke(front, Rounding::same(1.5), stroke);
        }
    }
}

pub fn draw_status_dot(ui: &mut Ui, radius: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.4), Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

fn draw_arc(
    p: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_rad: f32,
    end_rad: f32,
    stroke: Stroke,
) {
    let steps = 16;
    let pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let a = start_rad + t * (end_rad - start_rad);
            pos2(center.x + a.cos() * radius, center.y + a.sin() * radius)
        })
        .collect();
    p.add(egui::Shape::line(pts, stroke));
}
