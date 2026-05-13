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
use rayon::ThreadPoolBuilder;
use std::fs;

/// Run NanoPlot with the given CLI arguments
pub fn run(cli: Cli) -> Result<()> {
    let files = cli.get_input();
    if files.is_empty() {
        return Err(NanoPlotError::NoInputFiles);
    }

    let is_stdin = files.len() == 1 && files[0].as_os_str() == "-";

    // For real files, sniff format and verify all files are the same type.
    // For stdin, extract_metrics handles detection internally.
    let file_type = if is_stdin {
        nanoget_rs::FileType::Fastq // placeholder; overridden inside extract_metrics
    } else {
        let ft = nanoget_rs::FileType::sniff(&files[0])?;
        for f in &files[1..] {
            let fft = nanoget_rs::FileType::sniff(f)?;
            if fft != ft {
                return Err(NanoPlotError::MixedFileTypes(
                    format!("{:?}", ft),
                    format!("{:?}", fft),
                ));
            }
        }
        ft
    };
    let files = files.to_vec();

    info!("Processing {} file(s)", files.len());

    // Create output directory
    fs::create_dir_all(&cli.outdir)
        .map_err(|e| NanoPlotError::OutputDirError(format!("{}: {}", cli.outdir.display(), e)))?;

    // Build configuration
    let mut config = Config::from_cli(&cli);
    let filter_settings = FilterSettings::from_cli(&cli);

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

    // One pool shared by extraction (file- and chromosome-level) and plot generation.
    let pool = ThreadPoolBuilder::new()
        .num_threads(cli.threads)
        .build()
        .map_err(|e| NanoPlotError::PlotError(e.to_string()))?;

    pool.install(|| -> Result<()> {
        let metrics: MetricsCollection = extract_metrics(&extract_args)?;
        info!("Extracted metrics for {} reads", metrics.reads.len());

        if metrics.reads.is_empty() {
            return Err(NanoPlotError::NoReadsAfterFilter);
        }

        // Determine which data types are present from the actual metrics.
        for r in &metrics.reads {
            config.has_quality |= r.quality.is_some();
            config.has_time_data |= r.start_time.is_some();
            config.has_alignment |= r.aligned_length.is_some();
            if config.has_quality && config.has_time_data && config.has_alignment {
                break;
            }
        }

        let stats_before = if filter_settings.has_filters() {
            Some(Stats::compute(&metrics.reads))
        } else {
            None
        };

        let filtered_reads = filter_reads(metrics.reads, &filter_settings);

        if filtered_reads.is_empty() {
            return Err(NanoPlotError::NoReadsAfterFilter);
        }

        let stats = Stats::compute(&filtered_reads);

        stats.write_to_file(&config.output_path("NanoStats.tsv"))?;
        info!("Wrote statistics to NanoStats.tsv");

        if config.raw {
            write_raw_data(&filtered_reads, &config.output_path("NanoPlot-data.tsv"))?;
            info!("Wrote raw data to NanoPlot-data.tsv");
        }

        let plots = generate_plots(&filtered_reads, &stats, &config)?;

        generate_html_report(&plots, &stats, stats_before.as_ref(), &config)?;
        info!("Generated HTML report: NanoPlot-report.html");

        info!(
            "NanoPlot finished. Output written to {}",
            cli.outdir.display()
        );

        Ok(())
    })
}
