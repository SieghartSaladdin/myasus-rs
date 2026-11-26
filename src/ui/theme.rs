use egui::{Color32, FontId, Margin, Rect, Rounding, Sense, Stroke, Ui, Vec2};

pub const BG_WINDOW: Color32 = Color32::from_rgb(18, 19, 23);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(27, 29, 34);
pub const BG_CARD: Color32 = Color32::from_rgb(33, 35, 41);
pub const BG_CARD_ALT: Color32 = Color32::from_rgb(41, 44, 52);
pub const ASUS_BLUE: Color32 = Color32::from_rgb(0, 120, 240);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(242, 244, 248);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(154, 160, 172);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(54, 58, 68);
pub const STATUS_OK: Color32 = Color32::from_rgb(46, 204, 113);
pub const STATUS_WARN: Color32 = Color32::from_rgb(243, 156, 18);
pub const STATUS_DANGER: Color32 = Color32::from_rgb(235, 87, 87);
pub const OLED_NOTICE_RED: Color32 = Color32::from_rgb(224, 108, 98);

pub fn apply_myasus_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = BG_WINDOW;
    visuals.panel_fill = BG_WINDOW;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = Color32::from_rgb(12, 13, 16);
    visuals.selection.bg_fill = ASUS_BLUE;
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);

    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);

    visuals.widgets.inactive.bg_fill = BG_CARD_ALT;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 57, 68);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ASUS_BLUE);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = ASUS_BLUE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    ctx.set_visuals(visuals);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(14.0))
}

/// Renders a telemetry tile with MyASUS's signature 2px electric-blue top border.
pub fn blue_top_card(ui: &mut Ui, min_height: f32, add_contents: impl FnOnce(&mut Ui)) {
    let response = egui::Frame::none()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(16.0, 14.0))
        .show(ui, |ui| {
            ui.set_min_height(min_height);
            ui.set_width(ui.available_width());
            add_contents(ui);
        });

    let rect = response.response.rect;
    let bar = Rect::from_min_max(rect.left_top(), egui::pos2(rect.right(), rect.top() + 3.0));
    ui.painter().rect_filled(bar, 2.0, ASUS_BLUE);
}

/// MyASUS Device Settings signature card with a 3px electric-blue vertical left accent stripe.
pub fn left_blue_card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    let response = egui::Frame::none()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin {
            left: 24.0,
            right: 20.0,
            top: 18.0,
            bottom: 18.0,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });

    let rect = response.response.rect;
    let bar = Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + 3.0, rect.bottom()));
    ui.painter().rect_filled(bar, 1.0, ASUS_BLUE);
}

pub fn toggle_switch_labeled(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let mut resp = None;
    ui.horizontal(|ui| {
        resp = Some(toggle_switch(ui, on));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(if *on { "On" } else { "Off" })
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
    });
    resp.unwrap()
}

/// Outlined pill button matching MyASUS's "One-click diagnosis" / "Check update items" buttons.
pub fn pill_button(ui: &mut Ui, label: &str, filled: bool) -> egui::Response {
    let fill = if filled { ASUS_BLUE } else { Color32::TRANSPARENT };
    let stroke = Stroke::new(1.3_f32, ASUS_BLUE);
    let text_color = if filled { Color32::WHITE } else { Color32::from_rgb(100, 175, 255) };

    let btn = egui::Button::new(egui::RichText::new(label).color(text_color).size(13.0))
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(18.0))
        .min_size(Vec2::new(132.0, 32.0));
    ui.add(btn)
}

pub fn status_badge(ui: &mut Ui, text: &str, color: Color32) {
    let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 36);
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, color))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(11.5).strong());
        });
}

pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let desired_size = Vec2::new(42.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let radius = rect.height() * 0.5;
        let bg = if *on { ASUS_BLUE } else { Color32::from_rgb(72, 76, 88) };
        ui.painter().rect_filled(rect, radius, bg);
        let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        ui.painter().circle_filled(egui::pos2(cx, rect.center().y), radius - 3.0, Color32::WHITE);
    }
    response
}

pub fn section_heading(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .font(FontId::proportional(22.0))
            .color(TEXT_PRIMARY)
            .strong(),
    );
    ui.label(egui::RichText::new(subtitle).color(TEXT_MUTED).size(13.0));
    ui.add_space(10.0);
}
