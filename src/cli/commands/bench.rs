use crate::benchmark;
use anyhow::Result;
use std::path::Path;

pub fn handle_bench(db_path: &Path, heavy: bool) -> Result<()> {
    println!("Running performance benchmarks on {}...", db_path.display());
    let report = benchmark::run_benchmark(db_path, heavy)?;

    println!("\n==========================================================================================");
    println!(
        "                      DATABASE PERFORMANCE BENCHMARK REPORT                              "
    );
    println!("==========================================================================================");
    println!("Database:     {}", report.db_path);
    println!("Format:       {}", report.format);
    println!("Total Games:  {}", report.total_games);
    if report.total_players > 0 {
        println!(
            "Entities:     {} players, {} events, {} sites",
            report.total_players, report.total_events, report.total_sites
        );
    }
    println!("Disk Size:    {:.2} MB", report.file_size_mb);
    println!("------------------------------------------------------------------------------------------");
    println!(
        "{:<22} | {:<42} | {:>10} | {:<20}",
        "Category", "Benchmark Operation", "Time (ms)", "Details / Speed"
    );
    println!("{:-<22}-+-{:-<42}-+-{:-<10}-+-{:-<20}", "", "", "", "");

    for item in &report.results {
        println!(
            "{:<22} | {:<42} | {:>10.2} | {:<20}",
            item.category, item.name, item.elapsed_ms, item.notes
        );
    }
    println!("==========================================================================================");
    println!(
        "Overall Benchmark Duration: {:.2} ms ({:.2} s)\n",
        report.total_time_ms,
        report.total_time_ms / 1000.0
    );
    Ok(())
}
