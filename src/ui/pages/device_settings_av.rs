use egui::{Color32, RichText, Slider, Ui};

use crate::models::{
    hardware::{ColorGamut, HardwareSettingsState, SplendidMode},
    peripherals::MonitorInfo,
};
use crate::ui::pages::device_settings::DeviceSettingAction;
use crate::ui::theme::{
    left_blue_card, toggle_switch_labeled, ASUS_BLUE, OLED_NOTICE_RED, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn render_audio_and_visual(
    ui: &mut Ui,
    hw: &mut HardwareSettingsState,
    monitors: &[MonitorInfo],
) -> Option<DeviceSettingAction> {
    let mut action = None;

    // 1. Volume Booster (matching user's MyASUS screenshot!)
    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("Volume Booster")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Switch on to enable Volume Booster.")
                .color(TEXT_MUTED)
                .size(12.5),
        );
        ui.add_space(8.0);
        let mut vb = hw.volume_booster_enabled;
        if toggle_switch_labeled(ui, &mut vb).changed() {
            hw.volume_booster_enabled = vb;
            action = Some(DeviceSettingAction::SetVolumeBooster(vb));
        }
    });

    ui.add_space(12.0);

    // 2. ASUS OLED Care (matching user's MyASUS screenshot!)
    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("ASUS OLED Care")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(10.0);

        // Pixel refresh
        ui.label(RichText::new("Pixel refresh").color(TEXT_PRIMARY).strong().size(14.0));
        ui.label(
            RichText::new(
                "Launch a special screen saver after being idle for 30 minutes. This screen saver refreshes your screen pixels and optimizes picture quality.",
            )
            .color(TEXT_MUTED)
            .size(12.0),
        );
        ui.label(
            RichText::new(
                "*To prevent permanent burn-in damage to your OLED display, we strongly suggest that you always leave this feature enabled.",
            )
            .color(OLED_NOTICE_RED)
            .size(12.0),
        );
        ui.add_space(6.0);
        let mut pr = hw.oled_care.pixel_refresh_enabled;
        if toggle_switch_labeled(ui, &mut pr).changed() {
            hw.oled_care.pixel_refresh_enabled = pr;
            action = Some(DeviceSettingAction::SetOledPixelRefresh(pr));
        }

        ui.add_space(12.0);

        // Pixel shift
        ui.label(RichText::new("Pixel shift").color(TEXT_PRIMARY).strong().size(14.0));
        ui.label(
            RichText::new(
                "Displayed pixels will slightly move to avoid consistently showing static images on your desktop.",
            )
            .color(TEXT_MUTED)
            .size(12.0),
        );
        ui.add_space(6.0);
        let mut ps = hw.oled_care.pixel_shift_enabled;
        if toggle_switch_labeled(ui, &mut ps).changed() {
            hw.oled_care.pixel_shift_enabled = ps;
            action = Some(DeviceSettingAction::SetOledPixelShift(ps));
        }

        ui.add_space(12.0);

        // Automatically hide taskbar in desktop mode
        let os_tb = if cfg!(target_os = "windows") {
            "Automatically hide Windows taskbar in desktop mode"
        } else {
            "Automatically hide desktop taskbar / panel in desktop mode"
        };
        ui.label(RichText::new(os_tb).color(TEXT_PRIMARY).strong().size(14.0));
        ui.label(
            RichText::new(
                "Automatically hide taskbar in desktop mode when not in use to avoid consistently showing static images.",
            )
            .color(TEXT_MUTED)
            .size(12.0),
        );
        ui.add_space(6.0);
        let mut ah = hw.oled_care.auto_hide_taskbar;
        if toggle_switch_labeled(ui, &mut ah).changed() {
            hw.oled_care.auto_hide_taskbar = ah;
            action = Some(DeviceSettingAction::SetOledTaskbarAutohide(ah));
        }

        ui.add_space(12.0);

        // Taskbar transparency effect
        let tb_tr = if cfg!(target_os = "windows") {
            "Windows taskbar transparency effect"
        } else {
            "Desktop taskbar transparency effect"
        };
        ui.label(RichText::new(tb_tr).color(TEXT_PRIMARY).strong().size(14.0));
        ui.label(
            RichText::new("Enable a semi-transparent effect on taskbar.")
                .color(TEXT_MUTED)
                .size(12.0),
        );
        ui.add_space(6.0);
        let mut tr = hw.oled_care.taskbar_transparency;
        if toggle_switch_labeled(ui, &mut tr).changed() {
            hw.oled_care.taskbar_transparency = tr;
            action = Some(DeviceSettingAction::SetOledTaskbarTransparency(tr));
        }

        ui.add_space(12.0);

        // OLED Flicker-Free Dimming
        ui.label(
            RichText::new("OLED Flicker-Free Dimming")
                .color(TEXT_PRIMARY)
                .strong()
                .size(14.0),
        );
        ui.horizontal(|ui| {
            let mut dim = hw.oled_care.flicker_free_dimming;
            if ui.add(Slider::new(&mut dim, 30..=100).suffix("%")).changed() {
                hw.oled_care.flicker_free_dimming = dim;
            }
            if ui.button("Apply").clicked() {
                action = Some(DeviceSettingAction::SetOledFlickerFreeDimming(
                    hw.oled_care.flicker_free_dimming,
                ));
            }
        });

        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "Notice: Avoid displaying static images at maximum brightness for extended periods; otherwise, it might cause permanent burn-in damage to your OLED display.",
            )
            .color(OLED_NOTICE_RED)
            .size(11.5),
        );
    });

    ui.add_space(12.0);

    // 3. AI Noise-Canceling Audio
    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("AI Noise-Canceling Audio (ClearVoice)")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("AI Noise-Canceling Microphone")
                .color(TEXT_PRIMARY)
                .strong()
                .size(13.5),
        );
        let mut mic = hw.clearvoice_mic_enabled;
        if toggle_switch_labeled(ui, &mut mic).changed() {
            hw.clearvoice_mic_enabled = mic;
            action = Some(DeviceSettingAction::SetClearVoiceMic(mic));
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new("AI Noise-Canceling Speaker")
                .color(TEXT_PRIMARY)
                .strong()
                .size(13.5),
        );
        let mut spk = hw.clearvoice_speaker_enabled;
        if toggle_switch_labeled(ui, &mut spk).changed() {
            hw.clearvoice_speaker_enabled = spk;
            action = Some(DeviceSettingAction::SetClearVoiceSpeaker(spk));
        }
    });

    ui.add_space(12.0);

    // 4. Splendid, Color Gamut, Brightness & Refresh Rate
    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("Splendid & Display Visuals")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for mode in [
                SplendidMode::Normal,
                SplendidMode::Vivid,
                SplendidMode::EyeCare,
                SplendidMode::Manual,
            ] {
                let active = hw.splendid_mode == mode;
                let btn = egui::Button::new(mode.label()).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.splendid_mode = mode;
                    action = Some(DeviceSettingAction::SetSplendid(mode, hw.eye_care_level));
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Color Gamut:").color(TEXT_MUTED));
            for gamut in [
                ColorGamut::Native,
                ColorGamut::SRgb,
                ColorGamut::DciP3,
                ColorGamut::DisplayP3,
            ] {
                let active = hw.color_gamut == gamut;
                let btn = egui::Button::new(gamut.label()).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.color_gamut = gamut;
                    action = Some(DeviceSettingAction::SetColorGamut(gamut));
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Eye Care Blue-Light Level:").color(TEXT_MUTED));
            let mut lvl = hw.eye_care_level;
            if ui.add(Slider::new(&mut lvl, 1..=5)).changed() {
                hw.eye_care_level = lvl;
                hw.splendid_mode = SplendidMode::EyeCare;
                action = Some(DeviceSettingAction::SetSplendid(SplendidMode::EyeCare, lvl));
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Screen Brightness:").color(TEXT_MUTED));
            let mut bright = hw.display_brightness_percent;
            ui.add(Slider::new(&mut bright, 5..=100).suffix("%"));
            if bright != hw.display_brightness_percent {
                hw.display_brightness_percent = bright;
            }
            if ui.button("Apply").clicked() {
                action = Some(DeviceSettingAction::SetBrightness(
                    hw.display_brightness_percent,
                ));
            }
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new("Panel Overdrive (High-Refresh LCD)")
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
        let mut od = hw.panel_overdrive_enabled;
        if toggle_switch_labeled(ui, &mut od).changed() {
            hw.panel_overdrive_enabled = od;
            action = Some(DeviceSettingAction::SetPanelOverdrive(od));
        }

        if !monitors.is_empty() {
            ui.add_space(8.0);
            for m in monitors {
                ui.label(
                    RichText::new(format!(
                        "Active Display: {} — {} ({} Hz)",
                        m.name,
                        m.resolution,
                        m.refresh_hz.map(|h| h.to_string()).unwrap_or_else(|| "?".into())
                    ))
                    .color(TEXT_MUTED)
                    .size(12.0),
                );
            }
        }
    });

    action
}
