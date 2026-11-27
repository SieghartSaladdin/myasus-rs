use std::path::PathBuf;

use chrono::Utc;
use clap::{Parser, Subcommand};

use crate::models::hardware::{FanProfile, SplendidMode};
use crate::platform::create_backend;
use crate::services::{
    diagnostics::run_full_diagnosis,
    report::{export_support_bundle, SupportBundle},
    telemetry::TelemetryCollector,
};

#[derive(Parser, Debug)]
#[command(
    name = "myasus-rs",
    version,
    about = "Complete MyASUS-inspired Rust hardware control & diagnostics utility for Windows & Linux (Ubuntu/Mint)"
)]
pub struct CliArgs {
    /// Initial GUI page (home | device-settings | audio-visual | diagnosis | devices)
    #[arg(long)]
    pub page: Option<String>,

    /// Capture a PNG screenshot of the GUI after rendering and exit
    #[arg(long)]
    pub screenshot: Option<PathBuf>,

    /// Pass the remaining arguments to Rune's headless tooling (debug builds), e.g.
    /// `--rune --script nav.json --frames 30 --out frames/`
    #[arg(long = "rune", num_args = 1.., allow_hyphen_values = true, value_name = "RUNE_ARGS")]
    pub rune_args: Vec<String>,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Print live hardware telemetry, device identity, and capabilities
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run one-click 6-category hardware diagnosis
    Diagnose {
        #[arg(long)]
        json: bool,
    },
    /// List connected USB, Bluetooth, Display, Audio & ASUS peripheral devices
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Set battery health charging threshold (40-100)
    BatteryLimit { percent: u8 },
    /// Set fan/thermal profile (whisper | standard | performance)
    FanMode { mode: String },
    /// Set ASUS keyboard backlight level (0-3)
    KbdLight { level: u8 },
    /// Toggle Fn Lock (on | off)
    FnLock { state: String },
    /// Set Splendid display color mode (normal | vivid | eyecare | manual)
    Splendid {
        mode: String,
        #[arg(long, default_value_t = 3)]
        level: u8,
    },
    /// Check firmware (fwupdmgr) and OS/package updates (apt / winget)
    Updates {
        #[arg(long)]
        json: bool,
    },
    /// Set OLED Flicker-Free Dimming level (30-100) and persist it across reboots
    Dimming { percent: u8 },
    /// Re-apply saved display preferences (OLED dimming + Splendid). One-shot.
    RestoreDisplay {
        /// Seconds to wait before applying (lets the desktop finish initialising)
        #[arg(long, default_value_t = 0)]
        delay: u64,
    },
    /// Run background OLED Care daemon (restores dimming, runs periodic Pixel Shift, guards gamma)
    OledDaemon {
        /// Seconds to wait before initial restore at session login
        #[arg(long, default_value_t = 5)]
        delay: u64,
    },
    /// Launch the fullscreen animated dark OLED screensaver (orbit | refresh | clock)
    Screensaver {
        #[arg(long, default_value = "orbit")]
        style: String,
    },
    /// Export full JSON hardware & diagnostic support report
    ExportReport {
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

pub fn run_cli_command(cmd: CliCommand) -> Result<(), String> {
    // Lightweight display commands: no need to probe the whole hardware backend.
    match cmd {
        CliCommand::RestoreDisplay { delay } => {
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_secs(delay));
            }
            println!("{}", crate::platform::restore_saved_display_prefs()?);
            return Ok(());
        }
        CliCommand::Dimming { percent } => {
            let pct = percent.clamp(30, 100);
            let prefs = crate::services::display_prefs::DisplayPrefs::save_dimming(pct)?;
            let msg = crate::platform::apply_display_pipeline(&prefs)?;
            println!("{msg} (saved)");
            return Ok(());
        }
        CliCommand::OledDaemon { delay } => {
            #[cfg(target_os = "linux")]
            {
                return crate::platform::linux_oled::run_linux_oled_daemon(delay);
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = delay;
                println!("{}", crate::platform::restore_saved_display_prefs()?);
                return Ok(());
            }
        }
        CliCommand::Screensaver { style } => {
            let parsed = crate::services::display_prefs::ScreensaverStyle::from_str_opt(&style);
            crate::ui::app::run_oled_screensaver_standalone(parsed);
            return Ok(());
        }
        _ => {}
    }

    let backend = create_backend();
    let mut collector = TelemetryCollector::new();
    let device = collector.collect_device_info(backend.as_ref());
    let telemetry = collector.sample(backend.as_ref());

    match cmd {
        CliCommand::Status { json } => {
            if json {
                let payload = serde_json::json!({
                    "backend": backend.backend_name(),
                    "device": device,
                    "telemetry": telemetry,
                    "hardware": backend.read_hardware_settings(),
                    "network": backend.network_summary(),
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
            } else {
                println!("=== {} ({}) ===", device.product_name, device.os_name);
                println!("Backend  : {}", backend.backend_name());
                println!("Serial   : {} | BIOS: {}", device.serial_number, device.bios_version);
                println!("Kernel   : {}", device.kernel_version);
                println!(
                    "CPU      : {:.1}% | Temp: {}",
                    telemetry.cpu_usage_percent,
                    telemetry
                        .cpu_temp_celsius
                        .map(|t| format!("{t:.1}°C"))
                        .unwrap_or_else(|| "N/A".into())
                );
                println!(
                    "Memory   : {:.1}% ({:.1} / {:.1} GB)",
                    telemetry.memory_usage_percent, telemetry.memory_used_gb, telemetry.memory_total_gb
                );
                println!(
                    "Fans     : CPU {} | GPU {}",
                    telemetry.fans.cpu_fan_rpm.map(|r| format!("{r} RPM")).unwrap_or_else(|| "N/A".into()),
                    telemetry.fans.gpu_fan_rpm.map(|r| format!("{r} RPM")).unwrap_or_else(|| "N/A".into())
                );
            }
            Ok(())
        }
        CliCommand::Diagnose { json } => {
            let report = run_full_diagnosis(backend.as_ref(), &device, &telemetry);
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            } else {
                println!("=== System Diagnosis ({}) ===", report.timestamp);
                for item in &report.items {
                    println!("[{:^11}] {}: {}", item.status.label(), item.category.title(), item.summary);
                }
            }
            Ok(())
        }
        CliCommand::Devices { json } => {
            let devices = backend.list_connected_devices();
            let monitors = backend.list_monitors();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "monitors": monitors,
                        "devices": devices
                    }))
                    .unwrap_or_default()
                );
            } else {
                println!("=== Connected Monitors ({}) ===", monitors.len());
                for m in &monitors {
                    println!(
                        " - {} {} ({} Hz)",
                        m.name,
                        m.resolution,
                        m.refresh_hz.map(|h| h.to_string()).unwrap_or_else(|| "?".into())
                    );
                }
                println!("=== Connected Devices ({}) ===", devices.len());
                for d in &devices {
                    let tag = if d.is_asus { "[ASUS] " } else { "" };
                    println!(" - {tag}[{}] {} ({})", d.kind.label(), d.name, d.vendor);
                }
            }
            Ok(())
        }
        CliCommand::BatteryLimit { percent } => {
            println!("{}", backend.set_battery_charge_limit(percent)?);
            Ok(())
        }
        CliCommand::FanMode { mode } => {
            let profile = FanProfile::from_acpi_name(&mode)
                .ok_or_else(|| format!("Unknown mode '{mode}'. Use: whisper, standard, performance"))?;
            println!("{}", backend.set_fan_profile(profile)?);
            Ok(())
        }
        CliCommand::KbdLight { level } => {
            println!("{}", backend.set_kbd_backlight(level)?);
            Ok(())
        }
        CliCommand::FnLock { state } => {
            let enabled = match state.to_ascii_lowercase().as_str() {
                "on" | "1" | "true" | "enable" => true,
                "off" | "0" | "false" | "disable" => false,
                _ => return Err("Use 'on' or 'off'".into()),
            };
            println!("{}", backend.set_fn_lock(enabled)?);
            Ok(())
        }
        CliCommand::Splendid { mode, level } => {
            let m = match mode.to_ascii_lowercase().as_str() {
                "normal" => SplendidMode::Normal,
                "vivid" => SplendidMode::Vivid,
                "eyecare" | "eye-care" => SplendidMode::EyeCare,
                "manual" => SplendidMode::Manual,
                _ => return Err("Use: normal, vivid, eyecare, manual".into()),
            };
            println!("{}", backend.set_splendid_mode(m, level)?);
            Ok(())
        }
        CliCommand::Updates { json } => {
            let report = backend.check_updates();
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            } else {
                println!("=== System Updates ({}) ===", report.last_checked);
                println!("Found {} update item(s).", report.items.len());
                for item in &report.items {
                    println!(" - [{}] {} ({} -> {})", item.source.label(), item.name, item.current_version, item.available_version);
                }
            }
            Ok(())
        }
        CliCommand::ExportReport { output } => {
            let diagnosis = run_full_diagnosis(backend.as_ref(), &device, &telemetry);
            let hw = backend.read_hardware_settings();
            let updates = backend.check_updates();
            let network = backend.network_summary();
            let monitors = backend.list_monitors();
            let devices = backend.list_connected_devices();
            let bundle = SupportBundle {
                generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                app_version: env!("CARGO_PKG_VERSION"),
                backend: backend.backend_name(),
                device: &device,
                telemetry: &telemetry,
                hardware: &hw,
                diagnosis: &diagnosis,
                updates: &updates,
                network: &network,
                monitors: &monitors,
                connected_devices: &devices,
            };
            let path = export_support_bundle(output, &bundle)?;
            println!("Support report exported to: {}", path.display());
            Ok(())
        }
        CliCommand::RestoreDisplay { .. }
        | CliCommand::Dimming { .. }
        | CliCommand::OledDaemon { .. }
        | CliCommand::Screensaver { .. } => unreachable!(),
    }
}
