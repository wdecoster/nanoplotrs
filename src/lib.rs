//! # NanoPlot
//!
//! A Rust tool for creating plots and statistics from long read sequencing data.
//!
//! This library provides functionality to:
//! - Extract read metrics from various sequencing formats (FASTQ, FASTA, BAM, CRAM, etc.)
//! - Compute statistics (N50, mean, median, etc.)
//! - Generate publication-quality plots
//! - Create HTML reports with embedded visualizations
//!
//! ## Example
//!
//! ```rust,no_run
//! use nanoplot::{run, cli::Cli};
//! use clap::Parser;
//!
//! fn main() -> anyhow::Result<()> {
//!     let cli = Cli::parse();
//!     run(cli)?;
//!     Ok(())
//! }
//! ```

pub mod cli;
pub mod config;
pub mod error;
pub mod filter;
pub mod plots;
pub mod report;
pub mod stats;

use crate::cli::Cli;
use crate::config::{Config, FilterSettings};
use crate::error::{NanoPlotError, Result};
use crate::filter::filter_reads;
use crate::plots::generate_plots;
use crate::report::generate_html_report;
use crate::stats::{write_raw_data, Stats};
use log::info;
use nanoget_rs::{extract_metrics, ExtractArgs, MetricsCollection};
use std::fs;

/// Run NanoPlot with the given CLI arguments
pub fn run(cli: Cli) -> Result<()> {
    // Validate input
    let (file_type, files) = cli.get_input().ok_or(NanoPlotError::NoInputFiles)?;

    info!("Processing {} {:?} file(s)", files.len(), file_type);

    // Create output directory
    fs::create_dir_all(&cli.outdir)
        .map_err(|e| NanoPlotError::OutputDirError(format!("{}: {}", cli.outdir.display(), e)))?;

    // Build configuration
    let mut config = Config::from_cli(&cli);
    let filter_settings = FilterSettings::from_cli(&cli);

    // Extract metrics using nanoget-rs
    let extract_args = ExtractArgs {
        files,
        file_type,
        threads: cli.threads,
        output_format: "json".to_string(),
        output: None,
        read_type: "1D".to_string(),
        barcoded: false,
        keep_supplementary: true,
        combine: "simple".to_string(),
        names: None,
    };

    let metrics: MetricsCollection = extract_metrics(&extract_args)?;
    info!("Extracted metrics for {} reads", metrics.reads.len());

    if metrics.reads.is_empty() {
        return Err(NanoPlotError::NoReadsAfterFilter);
    }

    // Update config based on data availability
    config.has_quality = metrics.reads.iter().any(|r| r.quality.is_some());
    config.has_time_data = metrics.reads.iter().any(|r| r.start_time.is_some());

    // Calculate stats before filtering
    let stats_before = if filter_settings.has_filters() {
        Some(Stats::compute(&metrics.reads))
    } else {
        None
    };

    // Apply filters
    let filtered_reads = filter_reads(metrics.reads, &filter_settings);

    if filtered_reads.is_empty() {
        return Err(NanoPlotError::NoReadsAfterFilter);
    }

    // Calculate final statistics
    let stats = Stats::compute(&filtered_reads);

    // Write stats file
    let stats_filename = if config.tsv_stats {
        "NanoStats.tsv"
    } else {
        "NanoStats.txt"
    };
    stats.write_to_file(&config.output_path(stats_filename), config.tsv_stats)?;
    info!("Wrote statistics to {}", stats_filename);

    // Write raw data if requested
    if config.raw {
        write_raw_data(&filtered_reads, &config.output_path("NanoPlot-data.tsv"))?;
        info!("Wrote raw data to NanoPlot-data.tsv");
    }

    // Generate plots
    let plots = generate_plots(&filtered_reads, &stats, &config)?;

    // Generate HTML report
    generate_html_report(&plots, &stats, stats_before.as_ref(), &config)?;
    info!("Generated HTML report: NanoPlot-report.html");

    info!(
        "NanoPlot finished. Output written to {}",
        cli.outdir.display()
    );

    Ok(())
}
