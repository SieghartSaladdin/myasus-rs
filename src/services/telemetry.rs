use std::time::Instant;

use sysinfo::{Disks, Networks, System};

use crate::models::{
    peripherals::ProcessEntry,
    system::{DeviceInfo, DiskMetrics, TelemetrySnapshot},
};
use crate::platform::PlatformBackend;

pub struct TelemetryCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_net_sample: Instant,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys,
            disks,
            networks,
            last_net_sample: Instant::now(),
        }
    }

    pub fn collect_device_info(&mut self, backend: &dyn PlatformBackend) -> DeviceInfo {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();

        let mut info = backend.read_device_info();
        if let Some(cpu) = self.sys.cpus().first() {
            info.cpu_brand = cpu.brand().trim().to_string();
        }
        info.cpu_cores = self.sys.cpus().len();
        info.total_memory_gb = self.sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
        info
    }

    pub fn sample(&mut self, backend: &dyn PlatformBackend) -> TelemetrySnapshot {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.disks.refresh(true);

        let cpu_usage_percent = self.sys.global_cpu_usage();
        let total_mem = self.sys.total_memory().max(1);
        let used_mem = self.sys.used_memory();
        let memory_usage_percent = (used_mem as f64 / total_mem as f64 * 100.0) as f32;
        let memory_used_gb = used_mem as f64 / (1024.0 * 1024.0 * 1024.0);
        let memory_total_gb = total_mem as f64 / (1024.0 * 1024.0 * 1024.0);

        let total_swap = self.sys.total_swap();
        let swap_usage_percent = if total_swap > 0 {
            (self.sys.used_swap() as f64 / total_swap as f64 * 100.0) as f32
        } else {
            0.0
        };

        let mut disks: Vec<DiskMetrics> = self
            .disks
            .list()
            .iter()
            .filter(|d| d.total_space() > 512 * 1024 * 1024)
            .filter(|d| {
                let mp = d.mount_point().to_string_lossy();
                !mp.starts_with("/snap") && !mp.starts_with("/boot/efi")
            })
            .map(|d| {
                let total = d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let avail = d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = (total - avail).max(0.0);
                let pct = if total > 0.0 {
                    (used / total * 100.0) as f32
                } else {
                    0.0
                };
                DiskMetrics {
                    mount_point: d.mount_point().to_string_lossy().to_string(),
                    name: d.name().to_string_lossy().to_string(),
                    used_gb: used,
                    total_gb: total,
                    usage_percent: pct,
                }
            })
            .collect();

        disks.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
        disks.dedup_by(|a, b| a.mount_point == b.mount_point);

        let (cpu_temp_celsius, gpu_temp_celsius) = backend.read_temperatures();
        let fans = backend.read_fans();
        let battery = backend.read_battery();

        TelemetrySnapshot {
            cpu_usage_percent,
            cpu_temp_celsius,
            gpu_temp_celsius,
            memory_usage_percent,
            memory_used_gb,
            memory_total_gb,
            swap_usage_percent,
            disks,
            fans,
            battery,
            uptime_seconds: System::uptime(),
        }
    }

    /// Returns live `(rx_kbps, tx_kbps)` across all network interfaces.
    pub fn sample_network_throughput(&mut self) -> (f64, f64) {
        self.networks.refresh(true);
        let dt = self.last_net_sample.elapsed().as_secs_f64().max(0.1);
        self.last_net_sample = Instant::now();

        let mut rx_bytes = 0u64;
        let mut tx_bytes = 0u64;
        for (_, data) in &self.networks {
            rx_bytes = rx_bytes.saturating_add(data.received());
            tx_bytes = tx_bytes.saturating_add(data.transmitted());
        }
        (
            (rx_bytes as f64 / 1024.0) / dt,
            (tx_bytes as f64 / 1024.0) / dt,
        )
    }

    /// Collects top processes by CPU/memory usage (mirrors `ClientProcessPerfEnum`).
    pub fn top_processes(&mut self, limit: usize) -> Vec<ProcessEntry> {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let mut list: Vec<ProcessEntry> = self
            .sys
            .processes()
            .values()
            .map(|p| ProcessEntry {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_mb: p.memory() as f64 / (1024.0 * 1024.0),
            })
            .collect();
        list.sort_by(|a, b| {
            b.memory_mb
                .partial_cmp(&a.memory_mb)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list.truncate(limit);
        list
    }
}
