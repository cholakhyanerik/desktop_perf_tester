mod process_manager;
mod metrics_collector;
mod report_generator;

use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;
use std::thread;
use std::time::Duration;

use metrics_collector::MetricsCollector;
use process_manager::ProcessManager;
use report_generator::ReportGenerator;

fn main() -> Result<()> {
    // 1. Читаем .env файл (игнорируем ошибку, если файла нет, но переменные заданы в системе)
    let _ = dotenv();
    
    // Используем anyhow::Context для понятных сообщений об ошибках
    let app1_path = env::var("APP_1_PATH").context("APP_1_PATH not found in .env or environment variables")?;
    let app2_path = env::var("APP_2_PATH").context("APP_2_PATH not found in .env or environment variables")?;

    println!("🚀 Starting Performance Test...");
    
    // 2. Инициализируем генератор отчетов и чистим папки
    let reporter = ReportGenerator::new();
    reporter.prepare_directories()?;

    // 3. Запускаем процессы
    let mut pm = ProcessManager::start_apps(&app1_path, &app2_path)?;
    
    let pid1 = pm.app1.id();
    let pid2 = pm.app2.id();

    // 4. Инициализируем коллектор метрик
    let mut collector = MetricsCollector::new(pid1, pid2);

    println!("👀 Monitoring started. Close any of the apps to finish the test.");

    // 5. Жизненный цикл (Main Loop)
    while pm.are_both_running() {
        collector.collect();
        // Интервал сбора данных - 1 секунда
        thread::sleep(Duration::from_secs(1)); 
    }

    // 6. Завершение теста
    pm.kill_all();

    // 7. Анализ и генерация отчетов
    println!("📊 Generating reports...");
    reporter.generate_reports(&collector.history_app1, &collector.history_app2)?;

    println!("✅ Test fully completed!");
    Ok(())
}