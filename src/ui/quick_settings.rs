use egui::{Context, RichText, Window};

use crate::models::hardware::{BatteryChargeMode, FanProfile, HardwareSettingsState};
use crate::ui::theme::{toggle_switch, ASUS_BLUE, BG_CARD, TEXT_MUTED, TEXT_PRIMARY};

pub enum QuickAction {
    SetBatteryLimit(u8),
    SetFanProfile(FanProfile),
    SetKbdBacklight(u8),
    SetFnLock(bool),
}

pub fn render_quick_settings(
    ctx: &Context,
    open: &mut bool,
    hw: &HardwareSettingsState,
) -> Option<QuickAction> {
    if !*open {
        return None;
    }

    let mut action = None;
    Window::new("Quick Setting")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .default_width(320.0)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(BG_CARD)
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Battery Health Charging")
                    .color(TEXT_PRIMARY)
                    .strong(),
            );
            ui.label(
                RichText::new("Limit maximum charge to extend battery lifespan")
                    .color(TEXT_MUTED)
                    .size(11.5),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (label, mode) in [
                    ("60%", BatteryChargeMode::MaxLifespan),
                    ("80%", BatteryChargeMode::Balanced),
                    ("100%", BatteryChargeMode::FullCapacity),
                ] {
                    let active = hw.battery_charge_limit == mode.threshold();
                    let btn = egui::Button::new(label)
                        .fill(if active { ASUS_BLUE } else { egui::Color32::from_rgb(46, 50, 60) });
                    if ui.add(btn).clicked() {
                        action = Some(QuickAction::SetBatteryLimit(mode.threshold()));
                    }
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(RichText::new("Fan Profile").color(TEXT_PRIMARY).strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for profile in [
                    FanProfile::Whisper,
                    FanProfile::Standard,
                    FanProfile::Performance,
                ] {
                    let active = hw.fan_profile == profile;
                    let btn = egui::Button::new(profile.label().replace(" Mode", ""))
                        .fill(if active { ASUS_BLUE } else { egui::Color32::from_rgb(46, 50, 60) });
                    if ui.add(btn).clicked() {
                        action = Some(QuickAction::SetFanProfile(profile));
                    }
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Keyboard Backlight").color(TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for lvl in (0..=hw.capabilities.kbd_backlight_max.min(3)).rev() {
                        let active = hw.kbd_backlight_level == lvl;
                        let btn = egui::Button::new(format!("{lvl}")).fill(if active {
                            ASUS_BLUE
                        } else {
                            egui::Color32::from_rgb(46, 50, 60)
                        });
                        if ui.add(btn).clicked() {
                            action = Some(QuickAction::SetKbdBacklight(lvl));
                        }
                    }
                });
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Function Key Lock (Fn Lock)").color(TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut fn_lock = hw.fn_lock_enabled;
                    if toggle_switch(ui, &mut fn_lock).changed() {
                        action = Some(QuickAction::SetFnLock(fn_lock));
                    }
                });
            });
        });

    action
}
