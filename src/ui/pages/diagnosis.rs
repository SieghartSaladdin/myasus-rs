use rune::{Color, El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl};

use crate::models::diagnosis::DiagnosticStatus;
use crate::ui::app::MyAsusState;
use crate::ui::theme::{
    asus_blue, card_heading, font_mono, font_style, page_title, rgb, status_danger, status_ok,
    status_pill, status_warn, surface_card, text_muted, text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagSubTab {
    HardwareCheckup,
    OledDisplayTests,
    Updates,
}

impl DiagSubTab {
    fn index(self) -> usize {
        match self {
            Self::HardwareCheckup => 0,
            Self::OledDisplayTests => 1,
            Self::Updates => 2,
        }
    }

    fn from_index(i: usize) -> Self {
        match i {
            1 => Self::OledDisplayTests,
            2 => Self::Updates,
            _ => Self::HardwareCheckup,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InteractiveTestState {
    pub display_test_step: Option<usize>,
    pub speaker_test_note: Option<String>,
}

impl InteractiveTestState {
    const COLORS: [(&'static str, (u8, u8, u8)); 6] = [
        ("Pure Red (Subpixel & Burn-In)", (255, 0, 0)),
        ("Pure Green (Subpixel & Burn-In)", (0, 255, 0)),
        ("Pure Blue (OLED Blue Subpixel Wear)", (0, 0, 255)),
        ("5% Dark Gray (OLED Banding / Uniformity)", (14, 14, 14)),
        ("Pure White (Uniformity Check)", (255, 255, 255)),
        ("Pure Black (OLED True-Black Check)", (0, 0, 0)),
    ];

    pub fn active_display_test_color(&self) -> Option<Color> {
        let idx = self.display_test_step?;
        let (_, (r, g, b)) = *Self::COLORS.get(idx)?;
        Some(rgb(r, g, b))
    }

    pub fn active_color_name(&self) -> &'static str {
        self.display_test_step
            .and_then(|i| Self::COLORS.get(i))
            .map(|(n, _)| *n)
            .unwrap_or("")
    }

    pub fn advance_display_test(&mut self) {
        match self.display_test_step {
            None => self.display_test_step = Some(0),
            Some(i) if i + 1 < Self::COLORS.len() => self.display_test_step = Some(i + 1),
            Some(_) => self.display_test_step = None,
        }
    }
}

pub fn render_diagnosis_updates(s: &MyAsusState, k: &Kit) -> El {
    let header = row()
        .w_full()
        .center_y()
        .justify_between()
        .child(page_title(
            "Diagnosis & Updates",
            "Hardware health checkup, OLED uniformity/burn-in inspection, and LVFS/APT updates.",
        ))
        .child(
            row()
                .gap(8.0)
                .child(
                    Button::new("Run Diagnosis")
                        .variant(ButtonVariant::Contrast)
                        .on_press(|s: &mut MyAsusState| {
                            s.diag_tab = DiagSubTab::HardwareCheckup;
                            s.run_diagnosis_now();
                        })
                        .build(k),
                )
                .child(
                    Button::new("Check Updates")
                        .variant(ButtonVariant::Outline)
                        .on_press(|s: &mut MyAsusState| {
                            s.diag_tab = DiagSubTab::Updates;
                            s.check_updates_now();
                        })
                        .build(k),
                ),
        );

    let tabs = SegmentedControl::new(
        &[
            "Hardware Checkup",
            "OLED Uniformity Test",
            "Firmware & APT Updates",
        ],
        Some(s.diag_tab.index()),
    )
    .width(None)
    .on_pick(|s: &mut MyAsusState, idx, _| {
        s.diag_tab = DiagSubTab::from_index(idx);
    })
    .build(k);

    let body = match s.diag_tab {
        DiagSubTab::HardwareCheckup => render_checkup(s, k),
        DiagSubTab::OledDisplayTests => render_oled_tests(s, k),
        DiagSubTab::Updates => render_updates(s, k),
    };
    let body = crate::ui::theme::swap_view(format!("diag-tab-{}", s.diag_tab.index()), body);

    col()
        .w_full()
        .gap(14.0)
        .child(header)
        .child(tabs)
        .child(body)
}

fn render_checkup(s: &MyAsusState, k: &Kit) -> El {
    let Some(report) = &s.diagnosis else {
        return surface_card(vec![
            card_heading(
                "Ready for 6-Category Linux Hardware Checkup",
                Some("Inspects Battery & Power, Thermal & Fans, RAM, NVMe/SSD, Wi-Fi/BT, and asus-nb-wmi kernel driver."),
            ),
            Button::new("Start Diagnosis")
                .variant(ButtonVariant::Contrast)
                .on_press(|s: &mut MyAsusState| s.run_diagnosis_now())
                .build(k),
        ]);
    };

    let summary = row()
        .gap(10.0)
        .center_y()
        .child(status_pill(
            format!("{} Healthy", report.count_by_status(DiagnosticStatus::Passed)),
            status_ok(),
        ))
        .child(status_pill(
            format!(
                "{} Attention",
                report.count_by_status(DiagnosticStatus::Warning)
            ),
            status_warn(),
        ))
        .child(status_pill(
            format!("{} Issues", report.count_by_status(DiagnosticStatus::Issue)),
            status_danger(),
        ))
        .child(
            text(format!("Run: {}", report.timestamp))
                .font(font_mono(11.5, 450.0))
                .color(text_muted()),
        );

    let mut list = col().w_full().gap(10.0).child(summary);
    for item in &report.items {
        let color = match item.status {
            DiagnosticStatus::Passed => status_ok(),
            DiagnosticStatus::Warning => status_warn(),
            DiagnosticStatus::Issue => status_danger(),
            DiagnosticStatus::Skipped => asus_blue(),
        };
        let mut children = vec![
            row()
                .w_full()
                .center_y()
                .justify_between()
                .child(
                    text(item.category.title())
                        .font(font_style(14.0, 650.0))
                        .color(text_primary()),
                )
                .child(status_pill(item.status.label(), color)),
            text(item.summary.clone())
                .font(font_style(12.5, 500.0))
                .color(text_primary())
                .wrap()
                .w_full(),
        ];
        for d in &item.details {
            children.push(
                text(format!("• {d}"))
                    .font(font_style(11.5, 400.0))
                    .color(text_muted())
                    .wrap()
                    .w_full(),
            );
        }
        if let Some(rec) = &item.recommendation {
            children.push(
                text(format!("Tip: {rec}"))
                    .font(font_style(11.5, 550.0))
                    .color(status_warn())
                    .wrap()
                    .w_full(),
            );
        }
        list = list.child(surface_card(children));
    }
    list
}

fn render_oled_tests(s: &MyAsusState, k: &Kit) -> El {
    let oled_test_card = surface_card(vec![
        card_heading(
            "OLED Burn-In, Subpixel & 5% Gray Uniformity Check",
            Some("Cycles Red, Green, Blue, 5% Dark Gray (for OLED vertical banding), White, and True Black full-screen frames."),
        ),
        row()
            .gap(10.0)
            .child(
                Button::new("Start Full-Screen Pattern Check")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut MyAsusState| s.interactive_diag.display_test_step = Some(0))
                    .build(k),
            )
            .child(
                Button::new("Run 1-Min Pixel Refresh Sweep")
                    .variant(ButtonVariant::Neutral)
                    .on_press(|s: &mut MyAsusState| {
                        s.screensaver.start_pixel_refresh(60.0, s.current_clock);
                    })
                    .build(k),
            ),
    ]);

    let mut speaker_children = vec![
        card_heading(
            "Stereo Speaker Sweep",
            Some("Runs a quick 2-channel sine sweep via `speaker-test`."),
        ),
        Button::new("Run Stereo Test")
            .variant(ButtonVariant::Outline)
            .on_press(|s: &mut MyAsusState| {
                let ok = std::process::Command::new("speaker-test")
                    .args(["-c", "2", "-t", "sine", "-l", "1"])
                    .spawn()
                    .is_ok();
                s.interactive_diag.speaker_test_note = Some(if ok {
                    "Triggered 2-channel sine sweep via speaker-test.".into()
                } else {
                    "speaker-test not installed (install alsa-utils).".into()
                });
            })
            .build(k),
    ];
    if let Some(note) = &s.interactive_diag.speaker_test_note {
        speaker_children.push(
            text(note.clone())
                .font(font_style(12.0, 500.0))
                .color(text_muted()),
        );
    }

    col()
        .w_full()
        .gap(12.0)
        .child(oled_test_card)
        .child(surface_card(speaker_children))
}

fn render_updates(s: &MyAsusState, k: &Kit) -> El {
    let report = &s.updates;
    let providers = row()
        .gap(10.0)
        .center_y()
        .child(status_pill(
            if report.fwupd_available {
                "fwupdmgr Ready"
            } else {
                "fwupdmgr Unavailable"
            },
            if report.fwupd_available {
                status_ok()
            } else {
                status_warn()
            },
        ))
        .child(status_pill(
            if report.apt_available {
                "APT Ready"
            } else {
                "APT Unavailable"
            },
            if report.apt_available {
                status_ok()
            } else {
                status_warn()
            },
        ))
        .child(
            text(if report.last_checked.is_empty() {
                "Not checked yet".to_string()
            } else {
                format!("Checked: {}", report.last_checked)
            })
            .font(font_mono(11.5, 450.0))
            .color(text_muted()),
        );

    let mut body = col().w_full().gap(10.0).child(providers);

    if report.items.is_empty() {
        body = body.child(surface_card(vec![
            card_heading(
                if report.last_checked.is_empty() {
                    "Click 'Check Updates' to scan LVFS firmware & APT packages"
                } else {
                    "All checked packages & firmware are up to date"
                },
                None,
            ),
            Button::new("Scan Now")
                .variant(ButtonVariant::Contrast)
                .on_press(|s: &mut MyAsusState| s.check_updates_now())
                .build(k),
        ]));
    } else {
        for item in &report.items {
            body = body.child(surface_card(vec![
                row()
                    .w_full()
                    .center_y()
                    .justify_between()
                    .child(
                        text(item.name.clone())
                            .font(font_style(14.0, 650.0))
                            .color(text_primary()),
                    )
                    .child(status_pill(
                        if item.critical {
                            "Critical"
                        } else {
                            item.source.label()
                        },
                        if item.critical {
                            status_danger()
                        } else {
                            asus_blue()
                        },
                    )),
                text(format!("{} → {}", item.current_version, item.available_version))
                    .font(font_mono(12.0, 600.0))
                    .color(status_ok()),
                text(item.summary.clone())
                    .font(font_style(12.0, 400.0))
                    .color(text_muted())
                    .wrap()
                    .w_full(),
            ]));
        }
    }

    body
}
