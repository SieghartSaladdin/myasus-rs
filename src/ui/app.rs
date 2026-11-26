use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Utc;
use egui::{Color32, Margin, RichText};

use crate::models::{
    diagnosis::DiagnosisReport,
    hardware::HardwareSettingsState,
    peripherals::{ConnectedDevice, EventSeverity, MonitorInfo, NetworkSummary, ProcessEntry, SystemEvent},
    system::{DeviceInfo, TelemetrySnapshot},
    updates::UpdateReport,
};
use crate::platform::{create_backend, PlatformBackend};
use crate::services::{
    config::AppConfig,
    diagnostics::{run_category_check, run_full_diagnosis},
    report::{export_support_bundle, SupportBundle},
    telemetry::TelemetryCollector,
};
use crate::ui::{
    pages::{
        device_settings::{render_device_settings, DeviceSettingAction, SettingsTab},
        devices::render_devices,
        diagnosis::{render_diagnosis, DiagnosisAction, DiagnosisTab},
        home::{render_home, HomeAction},
        interactive_diag::InteractiveTestState,
        messages::{render_messages, MessageAction},
        settings::render_settings,
        support::{render_support, SupportAction},
        system_info::render_system_info,
        updates::render_updates,
    },
    quick_settings::{render_quick_settings, QuickAction},
    sidebar::{render_sidebar, NavPage},
    theme::{apply_myasus_theme, BG_WINDOW, STATUS_DANGER, STATUS_OK},
};

pub struct MyAsusApp {
    backend: Box<dyn PlatformBackend>,
    collector: TelemetryCollector,
    config: AppConfig,
    device: DeviceInfo,
    telemetry: TelemetrySnapshot,
    cpu_history: VecDeque<f32>,
    hw_settings: HardwareSettingsState,
    cpu_governor: String,
    monitors: Vec<MonitorInfo>,
    connected_devices: Vec<ConnectedDevice>,
    filter_asus_only: bool,
    network: NetworkSummary,
    top_processes: Vec<ProcessEntry>,
    diagnosis: DiagnosisReport,
    diag_tab: DiagnosisTab,
    interactive_diag: InteractiveTestState,
    updates: UpdateReport,
    events: Vec<SystemEvent>,
    last_export_path: Option<String>,
    active_page: NavPage,
    settings_tab: SettingsTab,
    quick_settings_open: bool,
    last_refresh: Instant,
    toast: Option<(String, bool, Instant)>,
    thermal_alert_logged: bool,
    screenshot_target: Option<PathBuf>,
    screenshot_frames: u8,
}

impl MyAsusApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        start_page: NavPage,
        start_settings_tab: SettingsTab,
        screenshot_target: Option<PathBuf>,
    ) -> Self {
        apply_myasus_theme(&cc.egui_ctx);

        let backend = create_backend();
        let mut collector = TelemetryCollector::new();
        let config = AppConfig::load();
        let device = collector.collect_device_info(backend.as_ref());
        let telemetry = collector.sample(backend.as_ref());
        let hw_settings = backend.read_hardware_settings();
        let cpu_governor = backend.read_cpu_governor();
        let monitors = backend.list_monitors();
        let network = backend.network_summary();
        let top_processes = collector.top_processes(10);

        let mut cpu_history = VecDeque::with_capacity(32);
        cpu_history.push_back(telemetry.cpu_usage_percent);

        let mut events = vec![SystemEvent {
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            title: "Hardware Backend Initialized".into(),
            detail: format!("{} — {}", backend.backend_name(), device.product_name),
            severity: EventSeverity::Info,
            read: false,
        }];

        if telemetry.disks.iter().any(|d| d.usage_percent >= 90.0) {
            events.push(SystemEvent {
                timestamp: Utc::now().format("%H:%M:%S").to_string(),
                title: "Low Disk Space Warning".into(),
                detail: "One or more storage drives are above 90% capacity.".into(),
                severity: EventSeverity::Alert,
                read: false,
            });
        }

        Self {
            backend,
            collector,
            quick_settings_open: config.show_quick_settings_on_start,
            config,
            device,
            telemetry,
            cpu_history,
            hw_settings,
            cpu_governor,
            monitors,
            connected_devices: Vec::new(),
            filter_asus_only: false,
            network,
            top_processes,
            diagnosis: DiagnosisReport::default(),
            diag_tab: DiagnosisTab::AutomatedChecks,
            interactive_diag: InteractiveTestState::default(),
            updates: UpdateReport::default(),
            events,
            last_export_path: None,
            active_page: start_page,
            settings_tab: start_settings_tab,
            last_refresh: Instant::now(),
            toast: None,
            thermal_alert_logged: false,
            screenshot_target,
            screenshot_frames: 0,
        }
    }

    fn log_event(&mut self, title: impl Into<String>, detail: impl Into<String>, severity: EventSeverity) {
        self.events.push(SystemEvent {
            timestamp: Utc::now().format("%H:%M:%S").to_string(),
            title: title.into(),
            detail: detail.into(),
            severity,
            read: false,
        });
        if self.events.len() > 60 {
            self.events.remove(0);
        }
    }

    fn push_toast(&mut self, result: Result<String, String>) {
        match result {
            Ok(msg) => {
                self.log_event("Hardware Setting Applied", &msg, EventSeverity::Success);
                self.toast = Some((msg, true, Instant::now()));
            }
            Err(err) => {
                self.log_event("Hardware Setting Error", &err, EventSeverity::Warning);
                self.toast = Some((err, false, Instant::now()));
            }
        }
        self.hw_settings = self.backend.read_hardware_settings();
        self.cpu_governor = self.backend.read_cpu_governor();
        self.telemetry = self.collector.sample(self.backend.as_ref());
    }

    fn do_export_support_bundle(&mut self) {
        if self.connected_devices.is_empty() {
            self.connected_devices = self.backend.list_connected_devices();
        }
        let bundle = SupportBundle {
            generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            app_version: env!("CARGO_PKG_VERSION"),
            backend: self.backend.backend_name(),
            device: &self.device,
            telemetry: &self.telemetry,
            hardware: &self.hw_settings,
            diagnosis: &self.diagnosis,
            updates: &self.updates,
            network: &self.network,
            monitors: &self.monitors,
            connected_devices: &self.connected_devices,
        };
        match export_support_bundle(None, &bundle) {
            Ok(path) => {
                let p = path.display().to_string();
                self.last_export_path = Some(p.clone());
                self.push_toast(Ok(format!("Exported diagnostic support report to {p}")));
            }
            Err(err) => self.push_toast(Err(err)),
        }
    }
}

impl eframe::App for MyAsusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let interval = Duration::from_secs(self.config.refresh_interval_secs.max(1));
        if self.last_refresh.elapsed() >= interval {
            self.telemetry = self.collector.sample(self.backend.as_ref());
            let (rx, tx) = self.collector.sample_network_throughput();
            self.network.rx_kbps = rx;
            self.network.tx_kbps = tx;
            if self.cpu_history.len() >= 30 {
                self.cpu_history.pop_front();
            }
            self.cpu_history.push_back(self.telemetry.cpu_usage_percent);

            if let Some(temp) = self.telemetry.cpu_temp_celsius {
                if temp >= 90.0 && !self.thermal_alert_logged {
                    self.thermal_alert_logged = true;
                    self.log_event(
                        "High CPU Temperature Alert",
                        format!("CPU thermal zone reached {temp:.1}°C. Consider Performance fan mode."),
                        EventSeverity::Alert,
                    );
                } else if temp < 80.0 {
                    self.thermal_alert_logged = false;
                }
            }
            self.last_refresh = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_millis(500));

        let unread = self.events.iter().filter(|e| !e.read).count();
        egui::SidePanel::left("myasus_sidebar")
            .resizable(false)
            .exact_width(220.0)
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                render_sidebar(
                    ui,
                    &mut self.active_page,
                    &self.device.os_name,
                    self.device.asus_wmi_loaded,
                    unread,
                );
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG_WINDOW)
                    .inner_margin(Margin::symmetric(24.0, 20.0)),
            )
            .show(ctx, |ui| {
                if let Some((msg, ok, at)) = &self.toast {
                    if at.elapsed() < Duration::from_secs(6) {
                        let color = if *ok { STATUS_OK } else { STATUS_DANGER };
                        egui::Frame::none()
                            .fill(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 32))
                            .rounding(6.0)
                            .inner_margin(Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(msg).color(color).size(12.5));
                            });
                        ui.add_space(10.0);
                    }
                }

                let cpu_hist_vec: Vec<f32> = self.cpu_history.iter().copied().collect();
                egui::ScrollArea::vertical().show(ui, |ui| match self.active_page {
                    NavPage::Home => {
                        if let Some(action) = render_home(
                            ui,
                            &self.device,
                            &self.telemetry,
                            &cpu_hist_vec,
                            &self.updates,
                            &self.config,
                        ) {
                            match action {
                                HomeAction::Navigate(page) => self.active_page = page,
                                HomeAction::OpenQuickSettings => self.quick_settings_open = true,
                                HomeAction::RunOneClickDiagnosis => {
                                    self.diagnosis = run_full_diagnosis(
                                        self.backend.as_ref(),
                                        &self.device,
                                        &self.telemetry,
                                    );
                                    self.log_event(
                                        "One-Click Diagnosis Completed",
                                        "Checked all 6 hardware subsystems.",
                                        EventSeverity::Info,
                                    );
                                    self.active_page = NavPage::SystemDiagnosis;
                                }
                                HomeAction::CheckUpdates => {
                                    self.updates = self.backend.check_updates();
                                    self.active_page = NavPage::SystemUpdate;
                                }
                                HomeAction::CopySerial => {
                                    ui.output_mut(|o| o.copied_text = self.device.serial_number.clone());
                                    self.toast = Some(("Serial number copied".into(), true, Instant::now()));
                                }
                            }
                        }
                    }
                    NavPage::DeviceSettings => {
                        if let Some(act) = render_device_settings(
                            ui,
                            &mut self.settings_tab,
                            &mut self.hw_settings,
                            &self.cpu_governor,
                            &self.monitors,
                            &self.network,
                        ) {
                            match act {
                                DeviceSettingAction::OpenQuickSettings => {
                                    self.quick_settings_open = true;
                                }
                                DeviceSettingAction::RefreshNetwork => {
                                    self.network = self.backend.network_summary();
                                    self.toast = Some(("Refreshed network status".into(), true, Instant::now()));
                                }
                                other => {
                                    let res = match other {
                                        DeviceSettingAction::SetBatteryLimit(v) => self.backend.set_battery_charge_limit(v),
                                        DeviceSettingAction::SetFanProfile(p) => self.backend.set_fan_profile(p),
                                        DeviceSettingAction::SetCpuGovernor(g) => self.backend.set_cpu_governor(&g),
                                        DeviceSettingAction::SetKbdBacklight(l) => self.backend.set_kbd_backlight(l),
                                        DeviceSettingAction::SetFnLock(e) => self.backend.set_fn_lock(e),
                                        DeviceSettingAction::SetTouchpad(e) => self.backend.set_touchpad(e),
                                        DeviceSettingAction::SetPanelOverdrive(e) => self.backend.set_panel_overdrive(e),
                                        DeviceSettingAction::SetDgpuDisabled(d) => self.backend.set_dgpu_disabled(d),
                                        DeviceSettingAction::SetSplendid(m, lvl) => self.backend.set_splendid_mode(m, lvl),
                                        DeviceSettingAction::SetColorGamut(g) => Ok(format!("Color gamut set to {}", g.label())),
                                        DeviceSettingAction::SetBrightness(p) => self.backend.set_display_brightness(p),
                                        DeviceSettingAction::SetVolumeBooster(e) => self.backend.set_volume_booster(e),
                                        DeviceSettingAction::SetOledPixelRefresh(e) => self.backend.set_oled_pixel_refresh(e),
                                        DeviceSettingAction::SetOledPixelShift(e) => Ok(format!("OLED Pixel Shift {}", if e { "enabled" } else { "disabled" })),
                                        DeviceSettingAction::SetOledTaskbarAutohide(e) => self.backend.set_oled_taskbar_autohide(e),
                                        DeviceSettingAction::SetOledTaskbarTransparency(e) => self.backend.set_oled_taskbar_transparency(e),
                                        DeviceSettingAction::SetOledFlickerFreeDimming(v) => self.backend.set_oled_flicker_free_dimming(v),
                                        DeviceSettingAction::SetClearVoiceMic(e) => Ok(format!("AI Noise-Canceling Microphone {}", if e { "enabled" } else { "disabled" })),
                                        DeviceSettingAction::SetClearVoiceSpeaker(e) => Ok(format!("AI Noise-Canceling Speaker {}", if e { "enabled" } else { "disabled" })),
                                        DeviceSettingAction::SetTaskFirst(m) => Ok(format!("TaskFirst bandwidth profile set to {}", m.label())),
                                        DeviceSettingAction::SetWifiSmartConnect(e) => Ok(format!("WiFi SmartConnect {}", if e { "enabled" } else { "disabled" })),
                                        DeviceSettingAction::OpenQuickSettings
                                        | DeviceSettingAction::RefreshNetwork => unreachable!(),
                                    };
                                    self.push_toast(res);
                                }
                            }
                        }
                    }
                    NavPage::SystemDiagnosis => {
                        if let Some(act) = render_diagnosis(
                            ui,
                            &mut self.diag_tab,
                            &self.diagnosis,
                            &mut self.interactive_diag,
                            &self.top_processes,
                        ) {
                            match act {
                                DiagnosisAction::RunAll => {
                                    self.diagnosis = run_full_diagnosis(
                                        self.backend.as_ref(),
                                        &self.device,
                                        &self.telemetry,
                                    );
                                    self.log_event("System Diagnosis Complete", "6/6 categories finished.", EventSeverity::Success);
                                }
                                DiagnosisAction::RunCategory(cat) => {
                                    let item = run_category_check(cat, self.backend.as_ref(), &self.device, &self.telemetry);
                                    self.diagnosis.items.retain(|i| i.category != cat);
                                    self.diagnosis.items.push(item);
                                    self.diagnosis.timestamp = Utc::now().format("%Y/%m/%d %H:%M:%S UTC").to_string();
                                }
                                DiagnosisAction::RefreshProcesses => {
                                    self.top_processes = self.collector.top_processes(10);
                                }
                            }
                        }
                    }
                    NavPage::SystemUpdate => {
                        if render_updates(ui, &self.updates) {
                            self.updates = self.backend.check_updates();
                            self.log_event(
                                "System Update Scan",
                                format!("Found {} available update item(s).", self.updates.items.len()),
                                EventSeverity::Info,
                            );
                        }
                    }
                    NavPage::MyAsusDevices => {
                        if render_devices(
                            ui,
                            &self.connected_devices,
                            &self.monitors,
                            &mut self.filter_asus_only,
                        ) {
                            self.connected_devices = self.backend.list_connected_devices();
                            self.monitors = self.backend.list_monitors();
                            self.toast = Some((
                                format!("Detected {} hardware device(s)", self.connected_devices.len()),
                                true,
                                Instant::now(),
                            ));
                        }
                    }
                    NavPage::MessageCenter => {
                        if let Some(act) = render_messages(ui, &self.events) {
                            match act {
                                MessageAction::MarkAllRead => {
                                    for ev in &mut self.events {
                                        ev.read = true;
                                    }
                                }
                                MessageAction::ClearAll => self.events.clear(),
                            }
                        }
                    }
                    NavPage::CustomerSupport => {
                        if let Some(act) = render_support(ui, &self.device, self.last_export_path.as_deref()) {
                            match act {
                                SupportAction::ExportReport => self.do_export_support_bundle(),
                                SupportAction::RunDiagnosisAndExport => {
                                    self.diagnosis = run_full_diagnosis(
                                        self.backend.as_ref(),
                                        &self.device,
                                        &self.telemetry,
                                    );
                                    self.do_export_support_bundle();
                                }
                                SupportAction::CopySystemSummary => {
                                    let txt = format!(
                                        "{} | S/N: {} | BIOS: {} | OS: {} | CPU: {} | RAM: {:.1} GB",
                                        self.device.product_name,
                                        self.device.serial_number,
                                        self.device.bios_version,
                                        self.device.os_name,
                                        self.device.cpu_brand,
                                        self.device.total_memory_gb
                                    );
                                    ui.output_mut(|o| o.copied_text = txt);
                                    self.toast = Some(("System summary copied to clipboard".into(), true, Instant::now()));
                                }
                            }
                        }
                    }
                    NavPage::SystemInfo => {
                        render_system_info(
                            ui,
                            &self.device,
                            &self.telemetry,
                            &self.hw_settings.capabilities,
                            self.backend.backend_name(),
                        );
                    }
                    NavPage::Settings => {
                        if render_settings(ui, &mut self.config, self.hw_settings.capabilities.udev_writable) {
                            let _ = self.config.save();
                        }
                    }
                });
            });

        if let Some(act) = render_quick_settings(ctx, &mut self.quick_settings_open, &self.hw_settings) {
            let res = match act {
                QuickAction::SetBatteryLimit(v) => self.backend.set_battery_charge_limit(v),
                QuickAction::SetFanProfile(p) => self.backend.set_fan_profile(p),
                QuickAction::SetKbdBacklight(l) => self.backend.set_kbd_backlight(l),
                QuickAction::SetFnLock(e) => self.backend.set_fn_lock(e),
            };
            self.push_toast(res);
        }

        // Built-in GPU framebuffer screenshot capture when `--screenshot <path>` is passed
        if let Some(target_path) = self.screenshot_target.clone() {
            self.screenshot_frames = self.screenshot_frames.saturating_add(1);
            if self.screenshot_frames == 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let captured = ctx.input(|i| {
                i.events.iter().find_map(|ev| match ev {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = captured {
                let [w, h] = img.size;
                let mut rgba = Vec::with_capacity(w * h * 4);
                for px in &img.pixels {
                    rgba.extend_from_slice(&px.to_array());
                }
                let _ = image::save_buffer(
                    &target_path,
                    &rgba,
                    w as u32,
                    h as u32,
                    image::ColorType::Rgba8,
                );
                self.screenshot_target = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint();
            }
        }
    }
}
