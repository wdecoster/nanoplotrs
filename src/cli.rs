//! Command-line argument parsing for NanoPlot

use clap::Parser;
use std::path::PathBuf;

/// NanoPlot: Plotting and statistics for long read sequencing data
#[derive(Parser, Debug)]
#[command(name = "nanoplot")]
#[command(author = "Wouter De Coster")]
#[command(version)]
#[command(about = "Creates various plots for long read sequencing data", long_about = None)]
pub struct Cli {
    /// Input file(s) — format auto-detected from content (FASTQ, FASTA, BAM, CRAM, uBAM, summary TSV)
    #[arg(short = 'i', long = "input", value_name = "FILE", num_args = 1..)]
    pub input: Vec<PathBuf>,

    // Output options
    /// Output directory
    #[arg(short = 'o', long, default_value = ".")]
    pub outdir: PathBuf,

    /// Output file prefix
    #[arg(short = 'p', long, default_value = "")]
    pub prefix: String,

    // Processing options
    /// Number of threads to use
    #[arg(short = 't', long, default_value = "4")]
    pub threads: usize,

    // Filtering options
    /// Minimum read length filter
    #[arg(long)]
    pub minlength: Option<u32>,

    /// Maximum read length filter
    #[arg(long)]
    pub maxlength: Option<u32>,

    /// Minimum average read quality filter
    #[arg(long)]
    pub minqual: Option<f64>,

    /// Downsample to N reads
    #[arg(long)]
    pub downsample: Option<usize>,

    /// Include supplementary alignments from BAM/CRAM (excluded by default).
    /// Secondary alignments are always excluded.
    #[arg(long)]
    pub use_supplementary: bool,

    /// Percentile of read lengths to show in plots (default 99; set 100 to show all)
    #[arg(long, default_value = "99")]
    pub percentile: f64,

    /// Additionally show log-transformed length in scatter plots
    #[arg(long)]
    pub loglength: bool,

    /// Use dot scatter plots instead of the default 2D density plots
    #[arg(long)]
    pub dots: bool,

    /// Colormap for 2D density plots
    #[arg(long, default_value = "viridis", value_parser = ["viridis", "inferno", "turbo", "grayscale"])]
    pub colormap: String,

    /// Also write statistics as JSON (alongside the default TSV)
    #[arg(long)]
    pub json: bool,

    // Visual options
    /// Plot color (hex format like #4CB391 or color name)
    #[arg(short = 'c', long, default_value = "#4CB391")]
    pub color: String,

    /// Plot title
    #[arg(long)]
    pub title: Option<String>,

    /// Output format for plots
    #[arg(short = 'f', long, default_value = "svg", value_parser = ["svg", "png", "pdf"])]
    pub format: String,

    /// DPI for PNG output
    #[arg(long, default_value = "300")]
    pub dpi: u32,

    /// Show N50 marker on histograms
    #[arg(long = "N50")]
    pub n50_marker: bool,

    /// Export raw data as TSV
    #[arg(long)]
    pub raw: bool,

    /// Verbose output
    #[arg(long)]
    pub verbose: bool,
}

impl Cli {
    /// Return the list of input files.
    pub fn get_input(&self) -> &[PathBuf] {
        &self.input
    }
}
