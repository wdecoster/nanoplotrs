//! NanoPlot - Plotting and statistics for long read sequencing data
//!
//! A Rust implementation of NanoPlot for creating visualizations
//! and statistics from Nanopore sequencing data.

use anyhow::Result;
use clap::Parser;
use log::error;
use nanoplot::cli::Cli;

fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_secs()
        .init();

    // Run NanoPlot
    if let Err(e) = nanoplot::run(cli) {
        error!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
