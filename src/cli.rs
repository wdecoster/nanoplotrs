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
    // Input options (mutually exclusive in groups)
    /// Input FASTQ file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub fastq: Option<Vec<PathBuf>>,

    /// Input FASTA file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub fasta: Option<Vec<PathBuf>>,

    /// Input BAM file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub bam: Option<Vec<PathBuf>>,

    /// Input CRAM file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub cram: Option<Vec<PathBuf>>,

    /// Input unaligned BAM file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub ubam: Option<Vec<PathBuf>>,

    /// Input sequencing summary TSV file(s)
    #[arg(long, value_name = "FILE", num_args = 1..)]
    pub summary: Option<Vec<PathBuf>>,

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

    /// Drop outlier reads with extreme lengths
    #[arg(long)]
    pub drop_outliers: bool,

    /// Additionally show log-transformed length in scatter plots
    #[arg(long)]
    pub loglength: bool,

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

    /// Output stats in TSV format
    #[arg(long)]
    pub tsv_stats: bool,

    /// Verbose output
    #[arg(long)]
    pub verbose: bool,
}

impl Cli {
    /// Get the input file type and files
    pub fn get_input(&self) -> Option<(nanoget_rs::FileType, Vec<PathBuf>)> {
        if let Some(files) = &self.fastq {
            return Some((nanoget_rs::FileType::Fastq, files.clone()));
        }
        if let Some(files) = &self.fasta {
            return Some((nanoget_rs::FileType::Fasta, files.clone()));
        }
        if let Some(files) = &self.bam {
            return Some((nanoget_rs::FileType::Bam, files.clone()));
        }
        if let Some(files) = &self.cram {
            return Some((nanoget_rs::FileType::Cram, files.clone()));
        }
        if let Some(files) = &self.ubam {
            return Some((nanoget_rs::FileType::Ubam, files.clone()));
        }
        if let Some(files) = &self.summary {
            return Some((nanoget_rs::FileType::Summary, files.clone()));
        }
        None
    }

    /// Check if we have alignment data (BAM/CRAM)
    pub fn has_alignment_data(&self) -> bool {
        self.bam.is_some() || self.cram.is_some()
    }
}
