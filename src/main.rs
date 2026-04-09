mod process_manager;
mod metrics_collector;
mod report_generator;

use anyhow::Result;
use dotenvy::dotenv;
use std::env;
use std::thread;
use std::time::Duration;

use metrics_collector::MetricsCollector;
use process_manager::ProcessManager;
use report_generator::ReportGenerator;

fn main() -> Result<()> {
    dotenv().ok();

    let app1_path = env::var("APP_1_PATH").ok().filter(|s| !s.trim().is_empty());
    let app2_path = env::var("APP_2_PATH").ok().filter(|s| !s.trim().is_empty());

    let mut paths = vec![];
    let mut names = vec![];

    if let Some(p) = app1_path { paths.push(p); names.push("App1"); }
    if let Some(p) = app2_path { paths.push(p); names.push("App2"); }

    if paths.len() < 2 {
        anyhow::bail!("Provide at least 2 apps");
    }

    let reporter = ReportGenerator::new();
    reporter.prepare_directories()?;

    let mut pm = ProcessManager::start_apps(&paths[0], &paths[1])?;

    let pid1 = pm.app1.id();
    let pid2 = pm.app2.id();

    let mut collector = MetricsCollector::new(pid1, pid2);

    while pm.are_both_running() {
        collector.collect();
        thread::sleep(Duration::from_secs(1));
    }

    pm.kill_all();

    reporter.generate_reports(
        names[0],
        names[1],
        &collector.history_app1,
        &collector.history_app2,
    )?;

    Ok(())
}