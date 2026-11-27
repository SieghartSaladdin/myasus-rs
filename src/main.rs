mod cli;
mod models;
mod platform;
mod services;
mod ui;

use clap::Parser;

use crate::cli::{run_cli_command, CliArgs};
use crate::ui::{app::run_rune_app, pages::oled_care::OledSubTab, sidebar::NavPage};

fn main() {
    let args = CliArgs::parse();
    if let Some(command) = args.command {
        if let Err(err) = run_cli_command(command) {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
        return;
    }

    let (start_page, start_oled_tab) = match args.page.as_deref() {
        Some("oled") | Some("audio-visual") | Some("dimming") => {
            (NavPage::OledCare, OledSubTab::DimmingAndCare)
        }
        Some("pixel") | Some("shift") | Some("refresh") => {
            (NavPage::OledCare, OledSubTab::PixelRefreshAndShift)
        }
        Some("screensaver") | Some("display") => {
            (NavPage::OledCare, OledSubTab::SystemAndScreensaver)
        }
        Some("home") | Some("dashboard") => (NavPage::Dashboard, OledSubTab::DimmingAndCare),
        Some("power") | Some("hardware") | Some("device-settings") => {
            (NavPage::HardwarePower, OledSubTab::DimmingAndCare)
        }
        Some("diagnosis") | Some("updates") => {
            (NavPage::DiagnosisUpdates, OledSubTab::DimmingAndCare)
        }
        Some("settings") | Some("system-info") | Some("system") => {
            (NavPage::SystemSettings, OledSubTab::DimmingAndCare)
        }
        _ => (NavPage::OledCare, OledSubTab::DimmingAndCare),
    };

    run_rune_app(start_page, start_oled_tab, args.screenshot, args.rune_args);
}
