mod cli;
mod models;
mod platform;
mod services;
mod ui;

use clap::Parser;

use crate::cli::{run_cli_command, CliArgs};
use crate::ui::{app::MyAsusApp, pages::device_settings::SettingsTab, sidebar::NavPage};

fn main() {
    let args = CliArgs::parse();
    if let Some(command) = args.command {
        if let Err(err) = run_cli_command(command) {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
        return;
    }

    let (start_page, start_tab) = match args.page.as_deref() {
        Some("device-settings") | Some("power") => {
            (NavPage::DeviceSettings, SettingsTab::PowerAndPerformance)
        }
        Some("audio-visual") | Some("oled") => {
            (NavPage::DeviceSettings, SettingsTab::AudioAndVisual)
        }
        Some("connectivity") => (NavPage::DeviceSettings, SettingsTab::Connectivity),
        Some("input") => (NavPage::DeviceSettings, SettingsTab::InputAndGpu),
        Some("diagnosis") => (NavPage::SystemDiagnosis, SettingsTab::PowerAndPerformance),
        Some("devices") => (NavPage::MyAsusDevices, SettingsTab::PowerAndPerformance),
        _ => (NavPage::Home, SettingsTab::PowerAndPerformance),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MyASUS (Rust Edition)")
            .with_inner_size([1140.0, 720.0])
            .with_min_inner_size([880.0, 560.0]),
        ..Default::default()
    };

    let screenshot_path = args.screenshot.clone();
    if let Err(err) = eframe::run_native(
        "myasus-rs",
        options,
        Box::new(move |cc| {
            Ok(Box::new(MyAsusApp::new(
                cc,
                start_page,
                start_tab,
                screenshot_path,
            )))
        }),
    ) {
        eprintln!("Failed to launch GUI: {err}");
        std::process::exit(1);
    }
}
