use anyhow::Result;
use chrono::Local;
use plotters::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::Path; // Добавлено!

use crate::metrics_collector::AppMetrics;

#[derive(Serialize)]
struct FullReport<'a> {
    run_id: &'a str,
    app1_metrics: &'a [AppMetrics],
    app2_metrics: &'a [AppMetrics],
}

pub struct ReportGenerator {
    run_id: String,
}

impl ReportGenerator {
    pub fn new() -> Self {
        let run_id = Local::now().format("%Y%m%d_%H%M%S").to_string();
        Self { run_id }
    }

    pub fn prepare_directories(&self) -> Result<()> {
        let _ = fs::remove_dir_all("reports");
        fs::create_dir_all("reports")?;
        fs::create_dir_all(format!("report_history/{}", self.run_id))?;
        Ok(())
    }

    pub fn generate_reports(&self, data1: &[AppMetrics], data2: &[AppMetrics]) -> Result<()> {
        self.generate_markdown("reports/app1_report.md", "App 1", data1)?;
        self.generate_markdown("reports/app2_report.md", "App 2", data2)?;
        self.generate_comparison("reports/comparison.md", data1, data2)?;
        self.generate_json("reports/full_report.json", data1, data2)?;

        // ГЕНЕРАЦИЯ ВСЕХ 6 ГРАФИКОВ (Добавлены GPU и Network)
        self.draw_metric_chart("reports/cpu_comparison.png", "CPU Usage (%)", data1, data2, |m| m.cpu_usage, Some(100.0))?;
        self.draw_metric_chart("reports/ram_comparison.png", "RAM Usage (MB)", data1, data2, |m| m.ram_usage as f32, None)?;
        self.draw_metric_chart("reports/disk_read_comparison.png", "Disk Read (B/s)", data1, data2, |m| m.disk_read as f32, None)?;
        self.draw_metric_chart("reports/disk_write_comparison.png", "Disk Write (B/s)", data1, data2, |m| m.disk_write as f32, None)?;
        self.draw_metric_chart("reports/gpu_comparison.png", "GPU Usage (%)", data1, data2, |m| m.gpu_usage, Some(100.0))?;
        self.draw_metric_chart("reports/network_comparison.png", "Network Usage (B/s)", data1, data2, |m| m.network_usage as f32, None)?;

        // Копируем всё в историю
        let history_dir = format!("report_history/{}", self.run_id);
        let files_to_copy = [
            "app1_report.md", "app2_report.md", "comparison.md", "full_report.json",
            "cpu_comparison.png", "ram_comparison.png", "disk_read_comparison.png", 
            "disk_write_comparison.png", "gpu_comparison.png", "network_comparison.png"
        ];

        for file in files_to_copy {
            let src = format!("reports/{}", file);
            if Path::new(&src).exists() {
                fs::copy(&src, format!("{}/{}", history_dir, file))?;
            }
        }
        
        println!("📂 All Reports and 6 smart charts saved in 'reports/'");
        Ok(())
    }

    // Вспомогательные методы markdown и json скрыл для экономии места, 
    // они остаются такими же, только добавь туда вывод GPU и Network, если хочешь.
    fn generate_markdown(&self, path: &str, app_name: &str, data: &[AppMetrics]) -> Result<()> {
        if data.is_empty() { return Ok(()); }
        let avg_cpu: f32 = data.iter().map(|d| d.cpu_usage).sum::<f32>() / data.len() as f32;
        let avg_ram: u64 = data.iter().map(|d| d.ram_usage).sum::<u64>() / data.len() as u64;
        let content = format!("# Report: {}\n- CPU: {:.2}%\n- RAM: {} MB", app_name, avg_cpu, avg_ram);
        fs::write(path, content)?;
        Ok(())
    }

    fn generate_comparison(&self, path: &str, data1: &[AppMetrics], data2: &[AppMetrics]) -> Result<()> {
        if data1.is_empty() || data2.is_empty() {
             fs::write(path, "# Comparison Report\nNo sufficient data to compare.")?;
             return Ok(());
        }

        let avg_cpu1: f32 = data1.iter().map(|d| d.cpu_usage).sum::<f32>() / data1.len() as f32;
        let avg_cpu2: f32 = data2.iter().map(|d| d.cpu_usage).sum::<f32>() / data2.len() as f32;
        
        let avg_ram1: f32 = data1.iter().map(|d| d.ram_usage as f32).sum::<f32>() / data1.len() as f32;
        let avg_ram2: f32 = data2.iter().map(|d| d.ram_usage as f32).sum::<f32>() / data2.len() as f32;
        
        // Form textual descriptions
        let cpu_winner = if avg_cpu1 < avg_cpu2 { "App 1" } else { "App 2" };
        let ram_winner = if avg_ram1 < avg_ram2 { "App 1" } else { "App 2" };

        let content = format!(
            "# Detailed Textual Comparison Report\n\n\
            This report contains the raw text descriptions comparing the metrics of both applications.\n\n\
            ## CPU Usage\n\
            - **App 1 Avg**: {:.2}%\n\
            - **App 2 Avg**: {:.2}%\n\
            - **Winner (Lower is better)**: {}\n\n\
            ## RAM Usage\n\
            - **App 1 Avg**: {:.2} MB\n\
            - **App 2 Avg**: {:.2} MB\n\
            - **Winner (Lower is better)**: {}\n\n\
            *Note: Read generated PNG charts for disk, network, and GPU comparison visualizations.*",
            avg_cpu1, avg_cpu2, cpu_winner,
            avg_ram1, avg_ram2, ram_winner
        );

        fs::write(path, content)?;
        Ok(())
    }

    fn generate_json(&self, path: &str, data1: &[AppMetrics], data2: &[AppMetrics]) -> Result<()> {
        let report = FullReport { run_id: &self.run_id, app1_metrics: data1, app2_metrics: data2 };
        fs::write(path, serde_json::to_string_pretty(&report)?)?;
        Ok(())
    }

    // УМНАЯ ОТРИСОВКА ГРАФИКОВ (СГРУППИРОВАННЫЕ СТОЛБЦЫ)
    fn draw_metric_chart<F>(
        &self, 
        file_name: &str, 
        base_title: &str, 
        data1: &[AppMetrics], 
        data2: &[AppMetrics], 
        metric_extractor: F,
        max_y_override: Option<f32>
    ) -> Result<()> 
    where 
        F: Fn(&AppMetrics) -> f32 
    {
        if data1.is_empty() || data2.is_empty() { return Ok(()); }

        let avg1: f32 = data1.iter().map(&metric_extractor).sum::<f32>() / data1.len() as f32;
        let avg2: f32 = data2.iter().map(&metric_extractor).sum::<f32>() / data2.len() as f32;
        let max1: f32 = data1.iter().map(&metric_extractor).fold(0.0_f32, f32::max);
        let max2: f32 = data2.iter().map(&metric_extractor).fold(0.0_f32, f32::max);
        
        // Аналитика: кто победил? (У кого среднее значение меньше, тот эффективнее)
        let winner_text = if avg1 == 0.0 && avg2 == 0.0 {
            "No Data".to_string()
        } else if avg1 < avg2 {
            format!("Winner: App 1 (Avg {:.2} vs {:.2})", avg1, avg2)
        } else if avg2 < avg1 {
            format!("Winner: App 2 (Avg {:.2} vs {:.2})", avg2, avg1)
        } else {
            "Draw (Equal resource usage)".to_string()
        };

        let full_title = format!("{} | {}", base_title, winner_text);

        let root = BitMapBackend::new(file_name, (900, 600)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut max_y = f32::max(max1, max2) * 1.2; // 20% отступа сверху
        if max_y <= 0.0 { max_y = 10.0; } 
        if let Some(override_val) = max_y_override { max_y = override_val; }

        let mut chart = ChartBuilder::on(&root)
            .caption(full_title, ("sans-serif", 24).into_font().color(&BLACK))
            .margin(25)
            .x_label_area_size(40)
            .y_label_area_size(80)
            // Range 0 to 2 allows us to group: Average (0.0 - 1.0) and Peak (1.0 - 2.0)
            .build_cartesian_2d(0f32..2.0f32, 0f32..max_y)?;

        chart.configure_mesh()
            .x_desc("Average (Left) vs Peak (Right)")
            .y_desc(base_title)
            .disable_x_mesh()
            .x_labels(2)
            .x_label_formatter(&|v| {
                if *v == 0.5 { "Average".to_string() }
                else if *v == 1.5 { "Peak".to_string() }
                else { "".to_string() }
            })
            .draw()?;

        // Цвета из прикрепленного скриншота пользователя: Синий (App 1) и Серый (App 2)
        let color_app1 = RGBColor(79, 129, 189); 
        let color_app2 = RGBColor(217, 217, 217);

        // Отрисовка Average баров (App 1 и App 2)
        chart.draw_series(std::iter::once(PathElement::new(vec![], &color_app1)))?
            .label("App 1 (Start)")
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 15, y + 10)], color_app1.filled()));
            
        chart.draw_series(std::iter::once(PathElement::new(vec![], &color_app2)))?
            .label("App 2 (End)")
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 15, y + 10)], color_app2.filled()));

        // Group 1: Average [Ось X: 0.1 до 0.45 для App1, 0.55 до 0.9 для App2]
        chart.draw_series(std::iter::once(Rectangle::new([(0.15, 0.0), (0.45, avg1)], color_app1.filled())))?;
        chart.draw_series(std::iter::once(Rectangle::new([(0.55, 0.0), (0.85, avg2)], color_app2.filled())))?;

        // Group 2: Peak [Ось X: 1.15 до 1.45 для App1, 1.55 до 1.85 для App2]
        chart.draw_series(std::iter::once(Rectangle::new([(1.15, 0.0), (1.45, max1)], color_app1.filled())))?;
        chart.draw_series(std::iter::once(Rectangle::new([(1.55, 0.0), (1.85, max2)], color_app2.filled())))?;

        chart.configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        root.present()?;
        Ok(())
    }
}