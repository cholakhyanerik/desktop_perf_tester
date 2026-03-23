use serde::{Deserialize, Serialize};
use std::time::Instant;
use sysinfo::{Pid, ProcessesToUpdate, System, Networks};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    pub time_sec: u64,
    pub cpu_usage: f32, // в процентах
    pub ram_usage: u64, // в мегабайтах
    pub disk_read: u64, // в байтах
    pub disk_write: u64, // в байтах
    pub gpu_usage: f32, // в процентах (Общесистемная метрика/через nvidia-smi)
    pub network_usage: u64, // в байтах (Общесистемная метрика)
}

pub struct MetricsCollector {
    sys: System,
    networks: Networks,
    start_time: Instant,
    pid1: Pid,
    pid2: Pid,
    pub history_app1: Vec<AppMetrics>,
    pub history_app2: Vec<AppMetrics>,
}

impl MetricsCollector {
    pub fn new(pid1: u32, pid2: u32) -> Self {
        Self {
            sys: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            start_time: Instant::now(),
            pid1: Pid::from_u32(pid1),
            pid2: Pid::from_u32(pid2),
            history_app1: Vec::new(),
            history_app2: Vec::new(),
        }
    }

    pub fn collect(&mut self) {
        self.sys.refresh_processes(ProcessesToUpdate::Some(&[self.pid1, self.pid2]), true);
        self.networks.refresh(true);
        
        let elapsed = self.start_time.elapsed().as_secs();

        // Считаем общесистемный трафик (так как попроцессный сбор кроссплатформенно сложен)
        let mut total_network = 0;
        for (_, network_data) in &self.networks {
            let rx = network_data.received();
            let tx = network_data.transmitted();
            let rx = if rx > 0 { rx } else { network_data.total_received() }; // fallback or actual rate
            let tx = if tx > 0 { tx } else { network_data.total_transmitted() };
            // Since we refresh per second, received() usually gives bytes since last refresh in sysinfo > 0.30
            total_network += rx + tx;
        }

        // Пытаемся получить общесистемный GPU универсально (AMD/Intel/NVIDIA)
        let gpu_usage = if cfg!(target_os = "windows") {
            // Универсальный метод для Windows через встроенные счетчики
            std::process::Command::new("powershell")
                .args(&["-NoProfile", "-Command", "((Get-Counter '\\GPU Engine(*)\\Utilization Percentage' -ErrorAction SilentlyContinue).CounterSamples | Measure-Object -Property CookedValue -Sum).Sum"])
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .and_then(|s| s.trim().replace(',', ".").parse::<f32>().ok())
                .unwrap_or(0.0)
        } else {
            // Fallback для Linux / Mac
            std::process::Command::new("nvidia-smi")
                .args(&["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
                .output()
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .and_then(|s| s.trim().split('\n').next()?.parse::<f32>().ok())
                .unwrap_or(0.0)
        };

        if let Some(process1) = self.sys.process(self.pid1) {
            let disk_usage = process1.disk_usage();
            self.history_app1.push(AppMetrics {
                time_sec: elapsed,
                cpu_usage: process1.cpu_usage(),
                ram_usage: process1.memory() / 1024 / 1024,
                disk_read: disk_usage.read_bytes,
                disk_write: disk_usage.written_bytes,
                gpu_usage,
                network_usage: total_network,
            });
        }

        if let Some(process2) = self.sys.process(self.pid2) {
            let disk_usage = process2.disk_usage();
            self.history_app2.push(AppMetrics {
                time_sec: elapsed,
                cpu_usage: process2.cpu_usage(),
                ram_usage: process2.memory() / 1024 / 1024,
                disk_read: disk_usage.read_bytes,
                disk_write: disk_usage.written_bytes,
                gpu_usage,
                network_usage: total_network,
            });
        }
    }
}