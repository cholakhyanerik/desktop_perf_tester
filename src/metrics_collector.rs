use serde::{Deserialize, Serialize};
use std::time::Instant;
use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    pub time_sec: u64,
    pub cpu_usage: f32,
    pub ram_usage: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub gpu_usage: f32,
    pub network_usage: u64,
}

pub struct MetricsCollector {
    sys: System,
    start_time: Instant,
    pid1: Pid,
    pid2: Pid,
    pub history_app1: Vec<AppMetrics>,
    pub history_app2: Vec<AppMetrics>,
}

impl MetricsCollector {
    pub fn new(pid1: u32, pid2: u32) -> Self {
        Self {
            sys: System::new(),
            start_time: Instant::now(),
            pid1: Pid::from_u32(pid1),
            pid2: Pid::from_u32(pid2),
            history_app1: vec![],
            history_app2: vec![],
        }
    }

    pub fn collect(&mut self) {
        self.sys.refresh_processes(
            ProcessesToUpdate::Some(&[self.pid1, self.pid2]),
            true,
        );

        let elapsed = self.start_time.elapsed().as_secs();

        let gpu1 = get_gpu_usage(self.pid1.as_u32());
        let gpu2 = get_gpu_usage(self.pid2.as_u32());

        let net1 = get_network_usage(self.pid1.as_u32());
        let net2 = get_network_usage(self.pid2.as_u32());

        if let Some(p1) = self.sys.process(self.pid1) {
            let d = p1.disk_usage();
            self.history_app1.push(AppMetrics {
                time_sec: elapsed,
                cpu_usage: p1.cpu_usage(),
                ram_usage: p1.memory() / 1024 / 1024,
                disk_read: d.read_bytes,
                disk_write: d.written_bytes,
                gpu_usage: gpu1,
                network_usage: net1,
            });
        }

        if let Some(p2) = self.sys.process(self.pid2) {
            let d = p2.disk_usage();
            self.history_app2.push(AppMetrics {
                time_sec: elapsed,
                cpu_usage: p2.cpu_usage(),
                ram_usage: p2.memory() / 1024 / 1024,
                disk_read: d.read_bytes,
                disk_write: d.written_bytes,
                gpu_usage: gpu2,
                network_usage: net2,
            });
        }
    }
}

//
// ================= GPU =================
//

#[cfg(target_os = "windows")]
fn get_gpu_usage(pid: u32) -> f32 {
    use std::process::Command;

    let query = format!(
        "((Get-Counter '\\GPU Engine(pid_{}_*)\\Utilization Percentage').CounterSamples | Measure-Object -Property CookedValue -Sum).Sum",
        pid
    );

    Command::new("powershell")
        .args(["-NoProfile", "-Command", &query])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().replace(',', ".").parse::<f32>().ok())
        .unwrap_or(0.0)
}

#[cfg(target_os = "linux")]
fn get_gpu_usage(_pid: u32) -> f32 {
    use std::process::Command;

    Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next()?.parse::<f32>().ok())
        .unwrap_or(0.0)
}

#[cfg(target_os = "macos")]
fn get_gpu_usage(_pid: u32) -> f32 {
    0.0
}

//
// ================= NETWORK =================
//

#[cfg(target_os = "windows")]
fn get_network_usage(pid: u32) -> u64 {
    use std::process::Command;

    let script = format!(
        "(Get-Process -Id {}).IOReadBytes + (Get-Process -Id {}).IOWriteBytes",
        pid, pid
    );

    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(not(target_os = "windows"))]
fn get_network_usage(_pid: u32) -> u64 {
    0
}