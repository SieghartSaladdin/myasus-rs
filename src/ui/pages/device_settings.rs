use egui::{Color32, FontId, Pos2, RichText, Slider, Stroke, Ui};

use crate::models::{
    hardware::{
        BatteryChargeMode, ColorGamut, FanProfile, HardwareSettingsState, SplendidMode,
        TaskFirstMode,
    },
    peripherals::{MonitorInfo, NetworkSummary},
};
use crate::ui::pages::device_settings_av::render_audio_and_visual;
use crate::ui::theme::{
    card_frame, left_blue_card, pill_button, status_badge, toggle_switch_labeled, ASUS_BLUE,
    STATUS_OK, STATUS_WARN, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    PowerAndPerformance,
    AudioAndVisual,
    Connectivity,
    InputAndGpu,
}

pub enum DeviceSettingAction {
    OpenQuickSettings,
    SetBatteryLimit(u8),
    SetFanProfile(FanProfile),
    SetCpuGovernor(String),
    SetKbdBacklight(u8),
    SetFnLock(bool),
    SetTouchpad(bool),
    SetPanelOverdrive(bool),
    SetDgpuDisabled(bool),
    SetSplendid(SplendidMode, u8),
    SetColorGamut(ColorGamut),
    SetBrightness(u8),
    SetVolumeBooster(bool),
    SetOledPixelRefresh(bool),
    SetOledPixelShift(bool),
    SetOledTaskbarAutohide(bool),
    SetOledTaskbarTransparency(bool),
    SetOledFlickerFreeDimming(u8),
    SetClearVoiceMic(bool),
    SetClearVoiceSpeaker(bool),
    SetTaskFirst(TaskFirstMode),
    SetWifiSmartConnect(bool),
    RefreshNetwork,
}

pub fn render_device_settings(
    ui: &mut Ui,
    tab: &mut SettingsTab,
    hw: &mut HardwareSettingsState,
    cpu_governor: &str,
    monitors: &[MonitorInfo],
    network: &NetworkSummary,
) -> Option<DeviceSettingAction> {
    let mut action = None;

    // Header matching MyASUS Device Settings screenshot
    ui.label(
        RichText::new("Device Settings")
            .font(FontId::proportional(24.0))
            .color(TEXT_PRIMARY)
            .strong(),
    );
    ui.add_space(10.0);

    // "General" blue-underlined tab header
    let gen_resp = ui.label(
        RichText::new("   General   ")
            .color(Color32::from_rgb(56, 156, 255))
            .strong()
            .size(14.0),
    );
    let underline_y = gen_resp.rect.bottom() + 4.0;
    ui.painter().line_segment(
        [
            Pos2::new(gen_resp.rect.left(), underline_y),
            Pos2::new(gen_resp.rect.left() + 140.0, underline_y),
        ],
        Stroke::new(2.5_f32, ASUS_BLUE),
    );
    ui.add_space(12.0);

    // Quick setting banner card
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Quick setting")
                .color(TEXT_PRIMARY)
                .strong()
                .size(14.5),
        );
        ui.add_space(8.0);
        if pill_button(ui, "Quick setting", false).clicked() {
            action = Some(DeviceSettingAction::OpenQuickSettings);
        }
    });

    ui.add_space(14.0);

    // Horizontal MyASUS category bar: Power & Performance | Audio & Visual | Connectivity | Input Device Settings
    ui.horizontal(|ui| {
        let tabs = [
            (SettingsTab::PowerAndPerformance, "Power & Performance"),
            (SettingsTab::AudioAndVisual, "Audio & Visual"),
            (SettingsTab::Connectivity, "Connectivity"),
            (SettingsTab::InputAndGpu, "Input Device Settings"),
        ];
        for (idx, (t, label)) in tabs.iter().enumerate() {
            let selected = *tab == *t;
            let color = if selected {
                Color32::from_rgb(56, 156, 255)
            } else {
                TEXT_PRIMARY
            };
            let btn = egui::Button::new(RichText::new(*label).color(color).size(13.5))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE);
            if ui.add(btn).clicked() {
                *tab = *t;
            }
            if idx + 1 < tabs.len() {
                ui.label(RichText::new("|").color(TEXT_MUTED));
            }
        }
    });
    ui.add_space(14.0);

    let sub_action = match *tab {
        SettingsTab::PowerAndPerformance => render_power_tab(ui, hw, cpu_governor),
        SettingsTab::AudioAndVisual => render_audio_and_visual(ui, hw, monitors),
        SettingsTab::Connectivity => render_connectivity_tab(ui, hw, network),
        SettingsTab::InputAndGpu => render_input_tab(ui, hw),
    };

    action.or(sub_action)
}

fn render_power_tab(
    ui: &mut Ui,
    hw: &mut HardwareSettingsState,
    cpu_governor: &str,
) -> Option<DeviceSettingAction> {
    let mut action = None;

    left_blue_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Battery Care Mode")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(16.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(
                    ui,
                    if hw.capabilities.battery_threshold_supported {
                        "Hardware Ready"
                    } else {
                        "No EC Limit"
                    },
                    if hw.capabilities.battery_threshold_supported {
                        STATUS_OK
                    } else {
                        STATUS_WARN
                    },
                );
            });
        });
        ui.label(
            RichText::new(
                "Limits maximum charging to extend battery lifespan (80% recommended when mostly plugged in).",
            )
            .color(TEXT_MUTED)
            .size(12.5),
        );
        ui.add_space(8.0);
        let mut care_on = hw.battery_charge_limit <= 80;
        if toggle_switch_labeled(ui, &mut care_on).changed() {
            let target = if care_on { 80 } else { 100 };
            hw.battery_charge_limit = target;
            action = Some(DeviceSettingAction::SetBatteryLimit(target));
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for mode in [
                BatteryChargeMode::FullCapacity,
                BatteryChargeMode::Balanced,
                BatteryChargeMode::MaxLifespan,
            ] {
                let active = hw.battery_charge_limit == mode.threshold();
                let btn = egui::Button::new(mode.label()).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.battery_charge_limit = mode.threshold();
                    action = Some(DeviceSettingAction::SetBatteryLimit(mode.threshold()));
                }
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Custom Threshold:").color(TEXT_MUTED));
            let mut custom = hw.battery_charge_limit;
            if ui.add(Slider::new(&mut custom, 40..=100).suffix("%")).changed() {
                hw.battery_charge_limit = custom;
            }
            if ui.button("Apply").clicked() {
                action = Some(DeviceSettingAction::SetBatteryLimit(hw.battery_charge_limit));
            }
        });
    });

    ui.add_space(12.0);

    left_blue_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Fan Profile").color(TEXT_PRIMARY).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(
                    ui,
                    &hw.capabilities.fan_profile_backend,
                    if hw.capabilities.fan_profile_supported {
                        STATUS_OK
                    } else {
                        STATUS_WARN
                    },
                );
            });
        });
        ui.label(
            RichText::new("Adjust thermal acoustics and cooling fan curve.")
                .color(TEXT_MUTED)
                .size(12.5),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for profile in [
                FanProfile::Whisper,
                FanProfile::Standard,
                FanProfile::Performance,
            ] {
                let active = hw.fan_profile == profile;
                let btn = egui::Button::new(profile.label()).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.fan_profile = profile;
                    action = Some(DeviceSettingAction::SetFanProfile(profile));
                }
            }
        });
    });

    ui.add_space(12.0);

    left_blue_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("CPU Scaling Governor / Power Plan")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(16.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_badge(ui, cpu_governor, ASUS_BLUE);
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (key, label) in [
                ("powersave", "Power Saver"),
                ("schedutil", "Balanced"),
                ("performance", "High Performance"),
            ] {
                if ui
                    .add(egui::Button::new(label).fill(Color32::from_rgb(44, 48, 58)))
                    .clicked()
                {
                    action = Some(DeviceSettingAction::SetCpuGovernor(key.to_string()));
                }
            }
        });
    });

    action
}

fn render_connectivity_tab(
    ui: &mut Ui,
    hw: &mut HardwareSettingsState,
    network: &NetworkSummary,
) -> Option<DeviceSettingAction> {
    let mut action = None;

    left_blue_card(ui, |ui| {
        ui.label(RichText::new("TaskFirst").color(TEXT_PRIMARY).strong().size(16.0));
        ui.label(
            RichText::new("Prioritize network bandwidth profile for your active workload.")
                .color(TEXT_MUTED)
                .size(12.5),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for mode in [
                TaskFirstMode::Normal,
                TaskFirstMode::Productivity,
                TaskFirstMode::Gaming,
                TaskFirstMode::Streaming,
            ] {
                let active = hw.taskfirst_mode == mode;
                let btn = egui::Button::new(mode.label()).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.taskfirst_mode = mode;
                    action = Some(DeviceSettingAction::SetTaskFirst(mode));
                }
            }
        });
    });

    ui.add_space(12.0);

    left_blue_card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("WiFi SmartConnect & Network Status")
                    .color(TEXT_PRIMARY)
                    .strong()
                    .size(16.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Refresh").clicked() {
                    action = Some(DeviceSettingAction::RefreshNetwork);
                }
                status_badge(
                    ui,
                    if network.connected { "Connected" } else { "Offline" },
                    if network.connected { STATUS_OK } else { STATUS_WARN },
                );
            });
        });
        ui.add_space(6.0);
        let mut sc = hw.wifi_smartconnect_enabled;
        if toggle_switch_labeled(ui, &mut sc).changed() {
            hw.wifi_smartconnect_enabled = sc;
            action = Some(DeviceSettingAction::SetWifiSmartConnect(sc));
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "Interface: {}  •  SSID: {}  •  Signal: {}",
                network.interface_name,
                network.ssid.as_deref().unwrap_or("Wired / N/A"),
                network
                    .signal_dbm
                    .map(|d| format!("{d} dBm"))
                    .unwrap_or_else(|| "N/A".into())
            ))
            .color(TEXT_MUTED)
            .size(12.5),
        );
        ui.label(
            RichText::new(format!(
                "Local IP: {}  •  Gateway: {}  •  Throughput: Down {:.1} KB/s | Up {:.1} KB/s",
                network.local_ip.as_deref().unwrap_or("-"),
                network.gateway_ip.as_deref().unwrap_or("-"),
                network.rx_kbps,
                network.tx_kbps
            ))
            .color(TEXT_MUTED)
            .size(12.0),
        );
    });

    action
}

fn render_input_tab(ui: &mut Ui, hw: &mut HardwareSettingsState) -> Option<DeviceSettingAction> {
    let mut action = None;

    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("Function Key Lock (Fn Lock)")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.label(
            RichText::new("Switch between standard F1-F12 keys and ASUS hotkeys.")
                .color(TEXT_MUTED)
                .size(12.5),
        );
        ui.add_space(8.0);
        let mut fn_lock = hw.fn_lock_enabled;
        if toggle_switch_labeled(ui, &mut fn_lock).changed() {
            hw.fn_lock_enabled = fn_lock;
            action = Some(DeviceSettingAction::SetFnLock(fn_lock));
        }
    });

    ui.add_space(12.0);

    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("Keyboard Backlight")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for lvl in 0..=hw.capabilities.kbd_backlight_max.min(3) {
                let active = hw.kbd_backlight_level == lvl;
                let lbl = if lvl == 0 {
                    "Off".to_string()
                } else {
                    format!("Level {lvl}")
                };
                let btn = egui::Button::new(lbl).fill(if active {
                    ASUS_BLUE
                } else {
                    Color32::from_rgb(44, 48, 58)
                });
                if ui.add(btn).clicked() {
                    hw.kbd_backlight_level = lvl;
                    action = Some(DeviceSettingAction::SetKbdBacklight(lvl));
                }
            }
        });
    });

    ui.add_space(12.0);

    left_blue_card(ui, |ui| {
        ui.label(
            RichText::new("Touchpad & Discrete GPU")
                .color(TEXT_PRIMARY)
                .strong()
                .size(16.0),
        );
        ui.add_space(8.0);
        ui.label(RichText::new("Precision Touchpad").color(TEXT_PRIMARY).size(13.5));
        let mut tp = hw.touchpad_enabled;
        if toggle_switch_labeled(ui, &mut tp).changed() {
            hw.touchpad_enabled = tp;
            action = Some(DeviceSettingAction::SetTouchpad(tp));
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new("Discrete GPU Eco Mode (dGPU Disable)")
                .color(TEXT_PRIMARY)
                .size(13.5),
        );
        if hw.capabilities.dgpu_disable_supported {
            let mut dgpu = hw.dgpu_disabled;
            if toggle_switch_labeled(ui, &mut dgpu).changed() {
                hw.dgpu_disabled = dgpu;
                action = Some(DeviceSettingAction::SetDgpuDisabled(dgpu));
            }
        } else {
            ui.label(
                RichText::new("Not exposed by this laptop's ACPI firmware.")
                    .color(TEXT_MUTED)
                    .size(12.0),
            );
        }
    });

    action
}
