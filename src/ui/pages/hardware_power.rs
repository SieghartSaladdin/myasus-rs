use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, SegmentedControl, Slider, Switch};

use crate::models::hardware::{BatteryChargeMode, FanProfile};
use crate::ui::app::MyAsusState;
use crate::ui::theme::{
    asus_blue_soft, card_heading, font_mono, font_style, page_title, status_ok, surface_card,
    text_muted, text_primary, value_pill,
};

pub fn render_hardware_power(s: &MyAsusState, k: &Kit) -> El {
    let modes = [
        BatteryChargeMode::FullCapacity,
        BatteryChargeMode::Balanced,
        BatteryChargeMode::MaxLifespan,
    ];
    let mut mode_row = row().gap(8.0);
    for mode in modes {
        let active = s.hw.battery_charge_limit == mode.threshold();
        mode_row = mode_row.child(
            Button::new(mode.label())
                .variant(if active {
                    ButtonVariant::Contrast
                } else {
                    ButtonVariant::Neutral
                })
                .on_press(move |s: &mut MyAsusState| {
                    let target = mode.threshold();
                    s.hw.battery_charge_limit = target;
                    let res = s.backend.set_battery_charge_limit(target);
                    s.record_action_result("Battery Charge Limit", res);
                })
                .build(k),
        );
    }

    let battery_card = surface_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(card_heading(
                "Battery Health Charging",
                Some("Writes /sys/class/power_supply/BAT*/charge_control_end_threshold."),
            ))
            .child(value_pill(
                format!("Limit: {}%", s.hw.battery_charge_limit),
                asus_blue_soft(),
            )),
        mode_row,
        row()
            .w_full()
            .center_y()
            .gap(12.0)
            .child(
                text("Custom Limit:")
                    .font(font_style(12.5, 500.0))
                    .color(text_muted()),
            )
            .child(
                Slider::new("hw-bat-limit", s.hw.battery_charge_limit as f32)
                    .range(40.0..=100.0)
                    .step(1.0)
                    .width(240.0)
                    .label("Battery Limit")
                    .hide_label()
                    .on_change(|s: &mut MyAsusState, val, _| {
                        s.hw.battery_charge_limit = val.round().clamp(40.0, 100.0) as u8;
                    })
                    .build(k),
            )
            .child(
                text(format!("{}%", s.hw.battery_charge_limit))
                    .font(font_mono(12.5, 650.0))
                    .color(text_primary()),
            )
            .child(
                Button::new("Apply")
                    .variant(ButtonVariant::Contrast)
                    .control_size(ControlSize::Small)
                    .on_press(|s: &mut MyAsusState| {
                        let target = s.hw.battery_charge_limit;
                        let res = s.backend.set_battery_charge_limit(target);
                        s.record_action_result("Battery Charge Limit", res);
                    })
                    .build(k),
            ),
    ]);

    let fan_idx = match s.hw.fan_profile {
        FanProfile::Whisper => 0,
        FanProfile::Standard => 1,
        FanProfile::Performance => 2,
    };

    let thermal_card = surface_card(vec![
        card_heading(
            "Fan & Thermal Profile + CPU Governor",
            Some("Controls /sys/firmware/acpi/platform_profile and Linux cpufreq scaling_governor."),
        ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                SegmentedControl::new(&["Whisper", "Standard", "Performance"], Some(fan_idx))
                    .width(Some(340.0))
                    .on_pick(|s: &mut MyAsusState, idx, _| {
                        let profile = match idx {
                            0 => FanProfile::Whisper,
                            2 => FanProfile::Performance,
                            _ => FanProfile::Standard,
                        };
                        s.hw.fan_profile = profile;
                        let res = s.backend.set_fan_profile(profile);
                        s.record_action_result("Fan Profile", res);
                    })
                    .build(k),
            )
            .child(value_pill(
                format!("Governor: {}", s.cpu_governor),
                status_ok(),
            )),
        row()
            .gap(8.0)
            .child(gov_btn("powersave", s.cpu_governor == "powersave", k))
            .child(gov_btn("schedutil", s.cpu_governor == "schedutil", k))
            .child(gov_btn("performance", s.cpu_governor == "performance", k)),
    ]);

    let kbd_audio_card = surface_card(vec![
        card_heading(
            "Keyboard Backlight & PulseAudio Booster",
            Some("Controls /sys/class/leds/asus::kbd_backlight and PulseAudio/PipeWire over-amplification."),
        ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                text("Keyboard Backlight")
                    .font(font_style(13.0, 600.0))
                    .color(text_primary()),
            )
            .child(
                SegmentedControl::new(
                    &["Off", "Level 1", "Level 2", "Level 3"],
                    Some((s.hw.kbd_backlight_level as usize).min(3)),
                )
                .width(Some(320.0))
                .on_pick(|s: &mut MyAsusState, idx, _| {
                    let lvl = idx as u8;
                    s.hw.kbd_backlight_level = lvl;
                    let res = s.backend.set_kbd_backlight(lvl);
                    s.record_action_result("Keyboard Backlight", res);
                })
                .build(k),
            ),
        row()
            .w_full()
            .center_y()
            .justify_between()
            .gap(12.0)
            .child(
                col()
                    .gap(2.0)
                    .child(
                        text("Speaker Volume Booster (130%)")
                            .font(font_style(13.0, 600.0))
                            .color(text_primary()),
                    )
                    .child(
                        text("Boosts @DEFAULT_SINK@ via pactl + desktop sound over-amplification.")
                            .font(font_style(11.5, 400.0))
                            .color(text_muted()),
                    ),
            )
            .child(
                Switch::new(s.hw.volume_booster_enabled)
                    .label(if s.hw.volume_booster_enabled { "On" } else { "Off" })
                    .on_toggle(|s: &mut MyAsusState| {
                        let next = !s.hw.volume_booster_enabled;
                        s.hw.volume_booster_enabled = next;
                        let res = s.backend.set_volume_booster(next);
                        s.record_action_result("Volume Booster", res);
                    })
                    .build(k),
            ),
    ]);

    let mut page = col()
        .w_full()
        .gap(14.0)
        .child(page_title(
            "Hardware & Power",
            "Only controls backed by real sysfs, ACPI platform_profile, LED, and PulseAudio interfaces on this machine.",
        ))
        .child(battery_card)
        .child(thermal_card)
        .child(kbd_audio_card);

    // Conditionally show extra ACPI controls ONLY when actually exposed by this laptop's kernel driver!
    let caps = &s.hw.capabilities;
    if caps.fn_lock_supported || caps.touchpad_toggle_supported || caps.dgpu_disable_supported {
        let mut extras = vec![card_heading(
            "Model-Specific ACPI Controls",
            Some("Detected in /sys/devices/platform/asus-nb-wmi on this model."),
        )];
        if caps.fn_lock_supported {
            extras.push(
                row()
                    .w_full()
                    .center_y()
                    .justify_between()
                    .child(
                        text("Function Key Lock (Fn Lock)")
                            .font(font_style(13.0, 600.0))
                            .color(text_primary()),
                    )
                    .child(
                        Switch::new(s.hw.fn_lock_enabled)
                            .label(if s.hw.fn_lock_enabled { "On" } else { "Off" })
                            .on_toggle(|s: &mut MyAsusState| {
                                let next = !s.hw.fn_lock_enabled;
                                s.hw.fn_lock_enabled = next;
                                let res = s.backend.set_fn_lock(next);
                                s.record_action_result("Fn Lock", res);
                            })
                            .build(k),
                    ),
            );
        }
        if caps.touchpad_toggle_supported {
            extras.push(
                row()
                    .w_full()
                    .center_y()
                    .justify_between()
                    .child(
                        text("Precision Touchpad")
                            .font(font_style(13.0, 600.0))
                            .color(text_primary()),
                    )
                    .child(
                        Switch::new(s.hw.touchpad_enabled)
                            .label(if s.hw.touchpad_enabled { "On" } else { "Off" })
                            .on_toggle(|s: &mut MyAsusState| {
                                let next = !s.hw.touchpad_enabled;
                                s.hw.touchpad_enabled = next;
                                let res = s.backend.set_touchpad(next);
                                s.record_action_result("Touchpad", res);
                            })
                            .build(k),
                    ),
            );
        }
        if caps.dgpu_disable_supported {
            extras.push(
                row()
                    .w_full()
                    .center_y()
                    .justify_between()
                    .child(
                        text("Discrete GPU Eco Mode (dGPU Disable)")
                            .font(font_style(13.0, 600.0))
                            .color(text_primary()),
                    )
                    .child(
                        Switch::new(s.hw.dgpu_disabled)
                            .label(if s.hw.dgpu_disabled { "On" } else { "Off" })
                            .on_toggle(|s: &mut MyAsusState| {
                                let next = !s.hw.dgpu_disabled;
                                s.hw.dgpu_disabled = next;
                                let res = s.backend.set_dgpu_disabled(next);
                                s.record_action_result("dGPU Eco Mode", res);
                            })
                            .build(k),
                    ),
            );
        }
        page = page.child(surface_card(extras));
    }

    page
}

fn gov_btn(name: &'static str, active: bool, k: &Kit) -> El {
    Button::new(name)
        .variant(if active {
            ButtonVariant::Contrast
        } else {
            ButtonVariant::Neutral
        })
        .control_size(ControlSize::Small)
        .on_press(move |s: &mut MyAsusState| {
            let res = s.backend.set_cpu_governor(name);
            s.cpu_governor = s.backend.read_cpu_governor();
            s.record_action_result("CPU Governor", res);
        })
        .build(k)
}
