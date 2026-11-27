use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rune::{
    App, AppHandle, AppSpec, ColorScheme, Cx, El, Key, KeyEvent, ScrollSpec, Tx, col, el, row,
    text,
};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit};

use crate::models::{
    diagnosis::DiagnosisReport,
    hardware::HardwareSettingsState,
    peripherals::{ConnectedDevice, MonitorInfo, NetworkSummary, ProcessEntry},
    system::{DeviceInfo, TelemetrySnapshot},
    updates::UpdateReport,
};
use crate::platform::{create_backend, PlatformBackend};
use crate::services::{
    config::AppConfig,
    diagnostics::run_full_diagnosis,
    display_prefs::{DisplayPrefs, ScreensaverStyle},
    report::{export_support_bundle, SupportBundle},
    telemetry::TelemetryCollector,
};
use crate::ui::{
    pages::{
        diagnosis::{render_diagnosis_updates, DiagSubTab, InteractiveTestState},
        hardware_power::render_hardware_power,
        home::render_dashboard,
        oled_care::{render_oled_care, OledSubTab},
        system_settings::render_system_settings,
    },
    screensaver::{render_screensaver_canvas, ScreensaverSession},
    sidebar::{render_sidebar, NavPage},
    theme::{
        bg_canvas, border_subtle, font_style, myasus_kit, status_danger, status_ok, swap_view,
        text_muted, text_primary,
    },
};

#[cfg(target_os = "linux")]
use crate::platform::linux_oled::OledPanelInfo;

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone)]
pub struct OledPanelInfo {
    pub output_name: String,
    pub panel_model: String,
    pub is_likely_oled: bool,
    pub resolution: String,
    pub external_rog_monitor: Option<String>,
    pub ddcutil_available: bool,
}

pub struct MyAsusState {
    pub backend: Arc<dyn PlatformBackend>,
    pub collector: TelemetryCollector,
    pub config: AppConfig,
    pub display_prefs: DisplayPrefs,
    pub panel_info: OledPanelInfo,
    pub system_dark_mode: bool,
    pub screen_off_mins: u32,
    pub pixel_shift_step: usize,
    pub pixel_shift_offset: (i32, i32),

    pub device: DeviceInfo,
    pub telemetry: TelemetrySnapshot,
    pub hw: HardwareSettingsState,
    pub cpu_governor: String,
    pub monitors: Vec<MonitorInfo>,
    pub devices: Vec<ConnectedDevice>,
    pub network: NetworkSummary,
    pub top_processes: Vec<ProcessEntry>,
    pub cpu_history: VecDeque<f32>,
    pub diagnosis: Option<DiagnosisReport>,
    pub updates: UpdateReport,

    pub active_page: NavPage,
    pub oled_tab: OledSubTab,
    pub diag_tab: DiagSubTab,
    pub interactive_diag: InteractiveTestState,
    pub screensaver: ScreensaverSession,

    pub status_banner: Option<(String, bool)>,
    pub current_clock: f64,
    pub udev_writable: bool,
}

impl MyAsusState {
    pub fn new(start_page: NavPage, start_oled_tab: OledSubTab) -> Self {
        let backend: Arc<dyn PlatformBackend> = Arc::from(create_backend());
        crate::services::failures::log_if_err(
            "restore saved display preferences",
            crate::platform::restore_saved_display_prefs(),
        );

        let mut collector = TelemetryCollector::new();
        let config = AppConfig::load();
        let display_prefs = DisplayPrefs::load();
        let device = collector.collect_device_info(backend.as_ref());
        let telemetry = collector.sample(backend.as_ref());
        let hw = backend.read_hardware_settings();
        let cpu_governor = backend.read_cpu_governor();
        let monitors = backend.list_monitors();
        let devices = backend.list_connected_devices();
        let network = backend.network_summary();
        let top_processes = collector.top_processes(8);

        #[cfg(target_os = "linux")]
        let (panel_info, system_dark_mode, screen_off_mins) = (
            crate::platform::linux_oled::detect_oled_panel_info(),
            crate::platform::linux_oled::read_linux_dark_mode(),
            crate::platform::linux_oled::read_linux_screen_off_mins(),
        );

        #[cfg(not(target_os = "linux"))]
        let (panel_info, system_dark_mode, screen_off_mins) = (
            OledPanelInfo {
                output_name: "eDP".into(),
                panel_model: "Built-in OLED Panel".into(),
                is_likely_oled: true,
                resolution: "Active".into(),
                external_rog_monitor: None,
                ddcutil_available: false,
            },
            true,
            display_prefs.screen_off_mins,
        );

        let mut cpu_history = VecDeque::with_capacity(28);
        cpu_history.push_back(telemetry.cpu_usage_percent);

        let udev_writable = std::path::Path::new("/sys/firmware/acpi/platform_profile")
            .metadata()
            .map(|_| crate::platform::sysfs::is_path_writable("/sys/firmware/acpi/platform_profile"))
            .unwrap_or(true);

        Self {
            backend,
            collector,
            config,
            display_prefs,
            panel_info,
            system_dark_mode,
            screen_off_mins,
            pixel_shift_step: 0,
            pixel_shift_offset: (0, 0),
            device,
            telemetry,
            hw,
            cpu_governor,
            monitors,
            devices,
            network,
            top_processes,
            cpu_history,
            diagnosis: None,
            updates: UpdateReport::default(),
            active_page: start_page,
            oled_tab: start_oled_tab,
            diag_tab: DiagSubTab::HardwareCheckup,
            interactive_diag: InteractiveTestState::default(),
            screensaver: ScreensaverSession::default(),
            status_banner: None,
            current_clock: 0.0,
            udev_writable,
        }
    }

    /// Samples telemetry synchronously on the UI thread. Only for one-off user actions
    /// (e.g. "Run diagnosis"); the periodic refresh runs on [`spawn_telemetry_worker`].
    pub fn refresh_telemetry(&mut self) {
        let update = TelemetryUpdate::collect(&mut self.collector, self.backend.as_ref());
        self.apply_telemetry(update);
    }

    fn apply_telemetry(&mut self, update: TelemetryUpdate) {
        self.telemetry = update.telemetry;
        self.cpu_history.push_back(self.telemetry.cpu_usage_percent);
        while self.cpu_history.len() > 28 {
            self.cpu_history.pop_front();
        }
        self.top_processes = update.top_processes;
        self.network = update.network;
    }

    pub fn refresh_hardware(&mut self) {
        self.hw = self.backend.read_hardware_settings();
        self.display_prefs = DisplayPrefs::load();
        self.cpu_governor = self.backend.read_cpu_governor();
    }

    pub fn record_action_result(&mut self, title: &str, result: Result<String, String>) {
        match result {
            Ok(msg) => self.status_banner = Some((msg, false)),
            Err(err) => self.status_banner = Some((format!("{title}: {err}"), true)),
        }
    }

    pub fn shift_pixels_next(&mut self) {
        self.pixel_shift_step = (self.pixel_shift_step + 1) % 8;
        #[cfg(target_os = "linux")]
        {
            match crate::platform::linux_oled::apply_linux_pixel_shift(
                self.pixel_shift_step,
                self.display_prefs.pixel_shift_strength,
            ) {
                Ok((dx, dy, msg)) => {
                    self.pixel_shift_offset = (dx, dy);
                    self.status_banner = Some((msg, false));
                }
                Err(e) => self.status_banner = Some((e, true)),
            }
        }
    }

    pub fn recenter_pixels(&mut self) {
        self.pixel_shift_step = 0;
        self.pixel_shift_offset = (0, 0);
        #[cfg(target_os = "linux")]
        {
            let res = crate::platform::linux_oled::reset_linux_pixel_shift();
            self.record_action_result("Pixel Shift", res);
        }
    }

    pub fn run_diagnosis_now(&mut self) {
        self.refresh_telemetry();
        let report = run_full_diagnosis(self.backend.as_ref(), &self.device, &self.telemetry);
        let issues = report.count_by_status(crate::models::diagnosis::DiagnosticStatus::Issue);
        let warnings = report.count_by_status(crate::models::diagnosis::DiagnosticStatus::Warning);
        self.diagnosis = Some(report);
        self.record_action_result(
            "Diagnosis",
            Ok(format!(
                "6-category checkup finished ({issues} issues, {warnings} warnings)"
            )),
        );
    }

    pub fn check_updates_now(&mut self) {
        self.updates = self.backend.check_updates();
        let count = self.updates.items.len();
        self.record_action_result(
            "Updates",
            Ok(format!("Checked LVFS & APT: {count} updates available")),
        );
    }

    pub fn export_support_report(&mut self) {
        let diag = self.diagnosis.clone().unwrap_or_else(|| {
            run_full_diagnosis(self.backend.as_ref(), &self.device, &self.telemetry)
        });
        let bundle = SupportBundle {
            generated_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            app_version: env!("CARGO_PKG_VERSION"),
            backend: self.backend.backend_name(),
            device: &self.device,
            telemetry: &self.telemetry,
            hardware: &self.hw,
            diagnosis: &diag,
            updates: &self.updates,
            network: &self.network,
            monitors: &self.monitors,
            connected_devices: &self.devices,
        };
        let res = export_support_bundle(None, &bundle)
            .map(|p| format!("Support bundle exported to {}", p.display()));
        self.record_action_result("Export", res);
    }
}

fn poll_app(s: &mut MyAsusState, clock: f64, tx: &mut Tx) -> bool {
    s.current_clock = clock;

    // Animate the OLED dark screensaver / Pixel Refresh calibration at the display's own
    // refresh rate (60/90/120/144/165 Hz...). `wake_after(ZERO)` asks Rune for the next
    // vsync-paced frame instead of a fixed 30fps timer, and the animation is driven from the
    // frame clock so motion speed stays identical on every refresh rate.
    if s.screensaver.active {
        s.screensaver.elapsed_secs = (clock - s.screensaver.started_clock).max(0.0);
        if let Some(total) = s.screensaver.calibration_total_secs {
            if s.screensaver.elapsed_secs >= total {
                s.screensaver.stop();
                s.status_banner = Some((
                    "OLED Pixel Refresh calibration cycle completed cleanly.".to_string(),
                    false,
                ));
            }
        }
        if s.screensaver.active {
            tx.wake_after(Duration::ZERO);
        }
        return true;
    }

    // Live telemetry is sampled on a background thread (see `spawn_telemetry_worker`), so the
    // UI thread never blocks on sysinfo / `ip` / `iwgetid` / `hostname` and animations keep
    // hitting every vsync.

    // Automatic Pixel Shift runs only in `oled-daemon`. The app used to run its own orbit too,
    // so the two shifted independently and each shift (a CRTC modeset) blanked the screen.

    false
}

/// One telemetry sample, gathered off the UI thread and applied in one step.
struct TelemetryUpdate {
    telemetry: TelemetrySnapshot,
    top_processes: Vec<ProcessEntry>,
    network: NetworkSummary,
}

impl TelemetryUpdate {
    fn collect(collector: &mut TelemetryCollector, backend: &dyn PlatformBackend) -> Self {
        let telemetry = collector.sample(backend);
        let top_processes = collector.top_processes(8);
        let mut network = backend.network_summary();
        let (rx, tx) = collector.sample_network_throughput();
        network.rx_kbps = rx;
        network.tx_kbps = tx;
        Self { telemetry, top_processes, network }
    }
}

/// Samples telemetry every `refresh_interval_secs` on its own thread and hands each result to
/// the UI thread. The slow work (process scan, spawning `ip`/`iwgetid`/`hostname`) used to run
/// inside `poll`, stalling a frame every refresh and making motion stutter on high-Hz panels.
fn spawn_telemetry_worker(
    handle: AppHandle<MyAsusState>,
    backend: Arc<dyn PlatformBackend>,
    interval_secs: u64,
) {
    let interval = Arc::new(AtomicU64::new(interval_secs.max(1)));
    let _ = std::thread::Builder::new()
        .name("myasus-telemetry".into())
        .spawn(move || {
            let mut collector = TelemetryCollector::new();
            loop {
                std::thread::sleep(Duration::from_secs(interval.load(Ordering::Relaxed).max(1)));
                // Keep Flicker-Free Dimming applied even without the login daemon: csd-color
                // resets the gamma ramp after every RandR change (Pixel Shift, resume, unlock).
                #[cfg(target_os = "linux")]
                let _ = crate::platform::display_sync::reassert_display_pipeline_if_reset();
                let update = TelemetryUpdate::collect(&mut collector, backend.as_ref());
                let interval = Arc::clone(&interval);
                handle.update(move |s: &mut MyAsusState, _tx: &mut Tx| {
                    // Follow the interval chosen in System Settings from the next sample on.
                    interval.store(s.config.refresh_interval_secs.max(1), Ordering::Relaxed);
                    s.apply_telemetry(update);
                });
            }
        });
}

fn handle_key(s: &mut MyAsusState, key: &KeyEvent, _tx: &mut Tx) -> bool {
    if s.screensaver.active {
        if matches!(key.key, Key::Escape | Key::Enter) {
            if s.screensaver.standalone_exit_on_dismiss {
                std::process::exit(0);
            }
            s.screensaver.stop();
            return true;
        }
    }
    if s.interactive_diag.display_test_step.is_some() && matches!(key.key, Key::Escape) {
        s.interactive_diag.display_test_step = None;
        return true;
    }
    false
}

fn render_status_banner(s: &MyAsusState, k: &Kit) -> Option<El> {
    let (msg, is_err) = s.status_banner.as_ref()?;
    let accent = if *is_err { status_danger() } else { status_ok() };
    Some(
        row()
            .key("status-banner")
            .w_full()
            .center_y()
            .gap(10.0)
            .px(14.0)
            .py(8.0)
            .radius(8.0)
            .fill(accent.alpha(0.14))
            .border(1.0, accent.alpha(0.45))
            .child(el().size(8.0, 8.0).radius(4.0).fill(accent))
            .child(
                text(msg.clone())
                    .font(font_style(12.5, 500.0))
                    .color(text_primary())
                    .wrap()
                    .grow(1.0)
                    .shrink(),
            )
            .child(
                Button::new("Dismiss")
                    .variant(ButtonVariant::Ghost)
                    .control_size(ControlSize::Small)
                    .on_press(|s: &mut MyAsusState| s.status_banner = None)
                    .build(k),
            ),
    )
}

pub fn view(s: &MyAsusState, _cx: &mut Cx<MyAsusState>) -> El {
    let k = myasus_kit();

    let page_body = match s.active_page {
        NavPage::OledCare => render_oled_care(s, &k),
        NavPage::Dashboard => render_dashboard(s, &k),
        NavPage::HardwarePower => render_hardware_power(s, &k),
        NavPage::DiagnosisUpdates => render_diagnosis_updates(s, &k),
        NavPage::SystemSettings => render_system_settings(s, &k),
    };

    let page_body = swap_view(format!("page-{:?}", s.active_page), page_body);

    let mut content_inner = col().w_full().gap(14.0).padding(22.0);
    if let Some(banner) = render_status_banner(s, &k) {
        content_inner = content_inner.child(banner);
    }
    content_inner = content_inner.child(page_body);

    let main_scroll = col()
        .key("main-scroll")
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .h_full()
        .fill(bg_canvas())
        .scroll(ScrollSpec { stick_bottom: false })
        .child(content_inner);

    let shell = row()
        .key("app-shell")
        .w_full()
        .h_full()
        .fill(bg_canvas())
        .child(render_sidebar(s))
        .child(main_scroll);

    let mut overlays = Vec::new();

    if s.screensaver.active {
        overlays.push(render_screensaver_canvas(
            &s.screensaver,
            |s: &mut MyAsusState| {
                if s.screensaver.standalone_exit_on_dismiss {
                    std::process::exit(0);
                }
                s.screensaver.stop();
            },
        ));
    }

    if let Some(color) = s.interactive_diag.active_display_test_color() {
        let color_name = s.interactive_diag.active_color_name();
        overlays.push(
            col()
                .key("display-test-overlay")
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
                .fill(color)
                .center()
                .gap(10.0)
                .on_press(|s: &mut MyAsusState| s.interactive_diag.advance_display_test())
                .child(
                    col()
                        .padding(16.0)
                        .radius(10.0)
                        .fill(bg_canvas().alpha(0.86))
                        .border(1.0, border_subtle())
                        .center()
                        .gap(4.0)
                        .child(
                            text(format!("OLED Uniformity & Subpixel Test: {color_name}"))
                                .font(font_style(15.5, 650.0))
                                .color(text_primary()),
                        )
                        .child(
                            text("Click to cycle Red / Green / Blue / Gray 5% / White / True Black • Esc to exit")
                                .font(font_style(12.0, 450.0))
                                .color(text_muted()),
                        ),
                ),
        );
    }

    rune_kit::root(shell, overlays)
}

pub fn run_rune_app(
    start_page: NavPage,
    start_oled_tab: OledSubTab,
    screenshot: Option<PathBuf>,
    extra_rune_args: Vec<String>,
) {
    let spec = AppSpec::<MyAsusState> {
        min_size: Some((920.0, 600.0)),
        poll: Some(poll_app),
        on_key: Some(handle_key),
        scenes: &[
            ("oled", |s| {
                s.active_page = NavPage::OledCare;
                s.oled_tab = OledSubTab::DimmingAndCare;
            }),
            ("pixel", |s| {
                s.active_page = NavPage::OledCare;
                s.oled_tab = OledSubTab::PixelRefreshAndShift;
            }),
            ("saver", |s| {
                s.active_page = NavPage::OledCare;
                s.oled_tab = OledSubTab::SystemAndScreensaver;
            }),
            ("screensaver-live", |s| {
                s.screensaver.start_screensaver(ScreensaverStyle::AsusOrbit, 0.0);
                s.screensaver.elapsed_secs = 4.2;
            }),
            ("dashboard", |s| s.active_page = NavPage::Dashboard),
            ("power", |s| s.active_page = NavPage::HardwarePower),
        ],
        ..AppSpec::new(
            "MyASUS Linux",
            (1160.0, 760.0),
            || MyAsusState::new(NavPage::OledCare, OledSubTab::DimmingAndCare),
            view,
        )
    };

    let mut rune_args = Vec::new();
    if let Some(path) = screenshot {
        rune_args.push("--snap".to_string());
        rune_args.push(path.to_string_lossy().to_string());
    }
    rune_args.extend(extra_rune_args);

    let app = App::new(spec)
        .color_scheme(ColorScheme::Dark)
        .init(move || MyAsusState::new(start_page, start_oled_tab))
        .args(rune_args);

    spawn_telemetry_worker(
        app.handle(),
        Arc::from(create_backend()),
        AppConfig::load().refresh_interval_secs,
    );

    app.run();
}

/// Launches the fullscreen animated OLED dark screensaver standalone (`myasus-rs screensaver`).
pub fn run_oled_screensaver_standalone(style: ScreensaverStyle) {
    let spec = AppSpec::<MyAsusState> {
        poll: Some(poll_app),
        on_key: Some(handle_key),
        ..AppSpec::new(
            "ASUS OLED Care Screensaver",
            (1280.0, 720.0),
            || MyAsusState::new(NavPage::OledCare, OledSubTab::SystemAndScreensaver),
            view,
        )
    };

    App::new(spec)
        .color_scheme(ColorScheme::Dark)
        .full_screen(true)
        .init(move || {
            let mut st = MyAsusState::new(NavPage::OledCare, OledSubTab::SystemAndScreensaver);
            st.screensaver.start_screensaver(style, 0.0);
            st.screensaver.standalone_exit_on_dismiss = true;
            st
        })
        .args(Vec::new())
        .run();
}
