//! Error types for NanoPlot

use thiserror::Error;

/// Main error type for NanoPlot operations
#[derive(Error, Debug)]
pub enum NanoPlotError {
    #[error("No input files provided")]
    NoInputFiles,

    #[error("Mixed file formats: cannot combine {0} and {1} in a single run")]
    MixedFileTypes(String, String),

    #[error("Failed to extract metrics: {0}")]
    ExtractionError(#[from] nanoget_rs::NanogetError),

    #[error("No reads remaining after filtering")]
    NoReadsAfterFilter,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to create output directory: {0}")]
    OutputDirError(String),

    #[error("Invalid color format: {0}")]
    InvalidColor(String),

    #[error("Plotting error: {0}")]
    PlotError(String),
}

pub type Result<T> = std::result::Result<T, NanoPlotError>;
