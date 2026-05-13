//! Runtime configuration for NanoPlot

use crate::cli::Cli;
use std::path::PathBuf;

/// Runtime configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct Config {
    /// Output directory
    pub outdir: PathBuf,
    /// File prefix for output files
    pub prefix: String,
    /// Number of threads
    pub threads: usize,
    /// Plot color
    pub color: String,
    /// Plot title
    pub title: Option<String>,
    /// Output format (svg, png, pdf)
    pub format: String,
    /// DPI for raster output
    pub dpi: u32,
    /// Show N50 marker
    pub show_n50: bool,
    /// Whether we have quality data
    pub has_quality: bool,
    /// Whether we have alignment data
    pub has_alignment: bool,
    /// Whether we have time data
    pub has_time_data: bool,
    /// Export raw data
    pub raw: bool,
    /// Show log-transformed length in scatter plots
    pub loglength: bool,
}

impl Config {
    /// Create config from CLI arguments
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            outdir: cli.outdir.clone(),
            prefix: cli.prefix.clone(),
            threads: cli.threads,
            color: cli.color.clone(),
            title: cli.title.clone(),
            format: cli.format.clone(),
            dpi: cli.dpi,
            show_n50: cli.n50_marker,
            has_quality: false,   // determined after data extraction
            has_alignment: false, // determined after file type detection
            has_time_data: false, // determined after data extraction
            raw: cli.raw,
            loglength: cli.loglength,
        }
    }

    /// Get the output path for a given filename
    pub fn output_path(&self, filename: &str) -> PathBuf {
        self.outdir.join(format!("{}{}", self.prefix, filename))
    }

    /// Get the plot file extension
    pub fn plot_extension(&self) -> &str {
        &self.format
    }
}

/// Filter settings derived from CLI
#[derive(Debug, Clone, Default)]
pub struct FilterSettings {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub min_quality: Option<f64>,
    pub downsample: Option<usize>,
    pub drop_outliers: bool,
}

impl FilterSettings {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            min_length: cli.minlength,
            max_length: cli.maxlength,
            min_quality: cli.minqual,
            downsample: cli.downsample,
            drop_outliers: cli.drop_outliers,
        }
    }

    /// Check if any filters are active
    pub fn has_filters(&self) -> bool {
        self.min_length.is_some()
            || self.max_length.is_some()
            || self.min_quality.is_some()
            || self.downsample.is_some()
            || self.drop_outliers
    }
}
