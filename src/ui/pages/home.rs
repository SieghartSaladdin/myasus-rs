use rune::{El, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::ui::app::MyAsusState;
use crate::ui::sidebar::NavPage;
use crate::ui::theme::{
    asus_blue, asus_blue_soft, bg_card, bg_elevated, font_mono, font_style, meter_bar,
    oled_teal, rgb, status_danger, status_ok, status_pill, status_warn, surface_card,
    text_muted, text_primary, value_pill,
};

pub fn render_dashboard(s: &MyAsusState, k: &Kit) -> El {
    let serial = s.device.serial_number.clone();

    let hero = row()
        .w_full()
        .items_stretch()
        .radius(12.0)
        .padding(18.0)
        .gap(16.0)
        .gradient(rgb(16, 30, 50), bg_card())
        .border(1.0, asus_blue().alpha(0.35))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(6.0)
                .child(
                    row()
                        .center_y()
                        .gap(8.0)
                        .child(
                            text(s.device.product_name.clone())
                                .font(font_style(20.0, 750.0))
                                .color(text_primary()),
                        )
                        .child(value_pill(s.device.product_family.clone(), asus_blue_soft())),
                )
                .child(
                    text(format!(
                        "{}  •  BIOS {}  •  {}",
                        s.panel_info.panel_model, s.device.bios_version, s.device.os_name
                    ))
                    .font(font_style(12.5, 450.0))
                    .color(text_muted()),
                )
                .child(
                    text(format!(
                        "{} ({} cores)  •  {:.1} GB RAM",
                        s.device.cpu_brand, s.device.cpu_cores, s.device.total_memory_gb
                    ))
                    .font(font_style(12.0, 400.0))
                    .color(text_muted()),
                )
                .child(
                    row()
                        .pt(6.0)
                        .gap(8.0)
                        .child(
                            Button::new("OLED Care")
                                .variant(ButtonVariant::Contrast)
                                .on_press(|s: &mut MyAsusState| s.active_page = NavPage::OledCare)
                                .build(k),
                        )
                        .child(
                            Button::new("Hardware & Power")
                                .variant(ButtonVariant::Neutral)
                                .on_press(|s: &mut MyAsusState| {
                                    s.active_page = NavPage::HardwarePower
                                })
                                .build(k),
                        )
                        .child(
                            Button::new("Copy Serial")
                                .variant(ButtonVariant::Outline)
                                .on_press_tx(move |s: &mut MyAsusState, tx: &mut Tx| {
                                    tx.copy(serial.clone());
                                    s.status_banner = Some((
                                        format!("Copied serial: {}", s.device.serial_number),
                                        false,
                                    ));
                                })
                                .build(k),
                        ),
                ),
        );

    let cpu_temp_str = s
        .telemetry
        .cpu_temp_celsius
        .map(|t| s.config.format_temp(t))
        .unwrap_or_else(|| "N/A".to_string());
    let gpu_temp_str = s
        .telemetry
        .gpu_temp_celsius
        .map(|t| s.config.format_temp(t))
        .unwrap_or_else(|| "N/A".to_string());
    let cpu_color = if s.telemetry.cpu_usage_percent > 85.0 {
        status_danger()
    } else if s.telemetry.cpu_usage_percent > 60.0 {
        status_warn()
    } else {
        asus_blue()
    };

    let mut spark = row().w_full().h(26.0).gap(2.0).items_end();
    for &val in &s.cpu_history {
        let h = ((val / 100.0).clamp(0.08, 1.0) * 26.0).round();
        spark = spark.child(
            el()
                .grow(1.0)
                .shrink()
                .h(h)
                .radius(2.0)
                .fill(asus_blue().alpha(0.75)),
        );
    }

    let cpu_card = surface_card(vec![
        row()
            .w_full()
            .justify_between()
            .center_y()
            .child(
                text("CPU & GPU")
                    .font(font_style(13.5, 680.0))
                    .color(text_primary()),
            )
            .child(text(cpu_temp_str).font(font_mono(13.0, 700.0)).color(cpu_color)),
        text(format!(
            "{:.1}% load  •  GPU {}",
            s.telemetry.cpu_usage_percent, gpu_temp_str
        ))
        .font(font_style(12.0, 450.0))
        .color(text_muted()),
        meter_bar(s.telemetry.cpu_usage_percent / 100.0, cpu_color),
        spark,
        text(format!("CPU load, last {} s", s.cpu_history.len() as u64 * s.config.refresh_interval_secs))
            .font(font_style(11.0, 450.0))
            .color(text_muted()),
    ])
    .grow(1.0)
    .shrink();

    let mem_card = surface_card(vec![
        row()
            .w_full()
            .justify_between()
            .center_y()
            .child(
                text("Memory & Fans")
                    .font(font_style(13.5, 680.0))
                    .color(text_primary()),
            )
            .child(
                text(format!("{:.0}%", s.telemetry.memory_usage_percent))
                    .font(font_mono(13.0, 700.0))
                    .color(status_ok()),
            ),
        text(format!(
            "{:.1} / {:.1} GB RAM  •  Swap {:.0}%",
            s.telemetry.memory_used_gb,
            s.telemetry.memory_total_gb,
            s.telemetry.swap_usage_percent
        ))
        .font(font_style(12.0, 450.0))
        .color(text_muted()),
        meter_bar(s.telemetry.memory_usage_percent / 100.0, status_ok()),
        text(format!(
            "CPU Fan: {}  •  GPU Fan: {}",
            s.telemetry.fans.cpu_fan_rpm.map_or("Auto".into(), |r| format!("{r} RPM")),
            s.telemetry.fans.gpu_fan_rpm.map_or("Auto".into(), |r| format!("{r} RPM")),
        ))
        .font(font_mono(11.5, 500.0))
        .color(text_muted()).wrap().w_full(),
    ])
    .grow(1.0)
    .shrink();

    let bat = &s.telemetry.battery;
    let bat_color = if bat.percentage < 20 && !bat.ac_online {
        status_danger()
    } else if bat.ac_online {
        status_ok()
    } else {
        asus_blue()
    };
    let bat_card = surface_card(vec![
        row()
            .w_full()
            .justify_between()
            .center_y()
            .child(
                text("Battery & Power")
                    .font(font_style(13.5, 680.0))
                    .color(text_primary()),
            )
            .child(status_pill(
                if bat.ac_online { "AC Plugged" } else { "Battery" },
                bat_color,
            )),
        text(format!(
            "{}% ({})  •  Charge Limit {}%",
            bat.percentage,
            bat.status,
            bat.charge_limit.unwrap_or(s.hw.battery_charge_limit)
        ))
        .font(font_style(12.0, 500.0))
        .color(text_primary()),
        meter_bar(bat.percentage as f32 / 100.0, bat_color),
        text(format!(
            "Power: {}  •  Health: {}",
            bat.power_watts
                .map(|w| format!("{w:.1} W"))
                .unwrap_or_else(|| "N/A".into()),
            bat.health_percent
                .map(|h| format!("{h:.1}%"))
                .unwrap_or_else(|| "N/A".into()),
        ))
        .font(font_mono(11.5, 450.0))
        .color(text_muted()),
    ])
    .grow(1.0)
    .shrink();

    let oled_strip = surface_card(vec![row()
        .w_full()
        .center_y()
        .justify_between()
        .gap(12.0)
        .child(
            col()
                .grow(1.0)
                .shrink()
                .gap(3.0)
                .child(
                    text("Active OLED Care Pipeline")
                        .font(font_style(14.0, 680.0))
                        .color(text_primary()),
                )
                .child(
                    text(format!(
                        "Dimming {}%  •  PWM Lock {} (floor {}%)  •  Anti-ABL {}  •  Pixel Shift {}  •  Dark Mode {}",
                        s.display_prefs.flicker_free_dimming,
                        if s.display_prefs.pwm_safe_lock { "On" } else { "Off" },
                        s.display_prefs.pwm_safe_floor,
                        if s.display_prefs.uniform_brightness_enabled { "On" } else { "Off" },
                        if s.display_prefs.pixel_shift_enabled { "On" } else { "Off" },
                        if s.system_dark_mode { "On" } else { "Off" },
                    ))
                    .font(font_mono(11.5, 450.0))
                    .color(text_muted())
                    .wrap()
                    .w_full(),
                ),
        )
        .child(
            Button::new("Configure")
                .variant(ButtonVariant::Neutral)
                .control_size(ControlSize::Small)
                .on_press(|s: &mut MyAsusState| s.active_page = NavPage::OledCare)
                .build(k),
        )]);

    // Storage & Network live row
    let mut disk_col = col().gap(8.0);
    for disk in s.telemetry.disks.iter().take(3) {
        let color = if disk.usage_percent >= 90.0 {
            status_danger()
        } else if disk.usage_percent >= 75.0 {
            status_warn()
        } else {
            asus_blue()
        };
        disk_col = disk_col.child(
            col()
                .gap(4.0)
                .child(
                    row()
                        .w_full()
                        .justify_between()
                        .child(
                            text(format!("{} ({})", disk.mount_point, disk.name))
                                .font(font_style(12.0, 500.0))
                                .color(text_primary()),
                        )
                        .child(
                            text(format!(
                                "{:.0} / {:.0} GB ({:.0}%)",
                                disk.used_gb, disk.total_gb, disk.usage_percent
                            ))
                            .font(font_mono(11.5, 500.0))
                            .color(text_muted()),
                        ),
                )
                .child(meter_bar(disk.usage_percent / 100.0, color)),
        );
    }

    let storage_card = surface_card(vec![
        text("Storage Overview")
            .font(font_style(14.0, 680.0))
            .color(text_primary()),
        disk_col,
    ])
    .grow(1.0)
    .shrink();

    let net = &s.network;
    let net_card = surface_card(vec![
        row()
            .w_full()
            .center_y()
            .justify_between()
            .child(
                text("Live Network & Power Profile")
                    .font(font_style(14.0, 680.0))
                    .color(text_primary()),
            )
            .child(status_pill(
                if net.connected { "Online" } else { "Offline" },
                if net.connected {
                    status_ok()
                } else {
                    status_warn()
                },
            )),
        text(format!(
            "Interface: {}  •  SSID: {}  •  IP: {}",
            net.interface_name,
            net.ssid.as_deref().unwrap_or("Wired / N/A"),
            net.local_ip.as_deref().unwrap_or("-"),
        ))
        .font(font_style(12.0, 450.0))
        .color(text_muted()),
        text(format!(
            "Throughput: ↓ {:.1} KB/s  •  ↑ {:.1} KB/s",
            net.rx_kbps, net.tx_kbps
        ))
        .font(font_mono(11.5, 550.0))
        .color(oled_teal()),
        row()
            .pt(4.0)
            .gap(8.0)
            .child(
                el()
                    .px(10.0)
                    .py(4.0)
                    .radius(6.0)
                    .fill(bg_elevated())
                    .child(
                        text(format!("Fan: {}", s.hw.fan_profile.label()))
                            .font(font_style(11.5, 550.0))
                            .color(text_primary()),
                    ),
            )
            .child(
                el()
                    .px(10.0)
                    .py(4.0)
                    .radius(6.0)
                    .fill(bg_elevated())
                    .child(
                        text(format!("Governor: {}", s.cpu_governor))
                            .font(font_style(11.5, 550.0))
                            .color(text_primary()),
                    ),
            ),
    ])
    .grow(1.0)
    .shrink();

    col()
        .w_full()
        .gap(14.0)
        .child(hero)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .items_stretch()
                .child(cpu_card)
                .child(mem_card)
                .child(bat_card),
        )
        .child(oled_strip)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .items_stretch()
                .child(storage_card)
                .child(net_card),
        )
}
