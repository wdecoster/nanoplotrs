//! Plotting modules for NanoPlot

pub mod histogram;
pub mod scatter;
pub mod time_plots;
pub mod yield_plot;

use crate::config::Config;
use crate::error::Result;
use crate::stats::Stats;
use log::info;
use nanoget_rs::ReadMetrics;
use std::fs;
use std::path::PathBuf;

// Re-export for internal use
use resvg;
use tiny_skia;
use usvg;

/// A plot that has been generated
#[derive(Debug, Clone)]
pub struct GeneratedPlot {
    /// Title of the plot
    pub title: String,
    /// Path to the saved file
    pub path: PathBuf,
    /// SVG content (for embedding in HTML)
    pub svg_content: String,
}

/// Generate all plots for the given reads
pub fn generate_plots(
    reads: &[ReadMetrics],
    stats: &Stats,
    config: &Config,
) -> Result<Vec<GeneratedPlot>> {
    let mut plots = Vec::new();

    info!("Generating plots for {} reads", reads.len());

    // Length histograms
    let lengths: Vec<f64> = reads.iter().map(|r| r.length as f64).collect();

    // Non-weighted histogram (read counts)
    plots.push(histogram::create_histogram(
        &lengths,
        "Non_weightedHistogramReadlength",
        "Non-weighted histogram of read lengths",
        "Read length",
        "Number of reads",
        None, // no weights
        config,
        if config.show_n50 {
            Some(stats.n50 as f64)
        } else {
            None
        },
    )?);

    // Weighted histogram (base counts)
    plots.push(histogram::create_weighted_histogram(
        &lengths,
        "WeightedHistogramReadlength",
        "Weighted histogram of read lengths",
        "Read length",
        "Number of bases",
        config,
        if config.show_n50 {
            Some(stats.n50 as f64)
        } else {
            None
        },
    )?);

    // Log-transformed histograms
    let log_lengths: Vec<f64> = lengths
        .iter()
        .filter(|&&l| l > 0.0)
        .map(|&l| l.log10())
        .collect();

    if !log_lengths.is_empty() {
        plots.push(histogram::create_log_histogram(
            &log_lengths,
            "Non_weightedLogTransformed_HistogramReadlength",
            "Non-weighted histogram of read lengths after log transformation",
            "Read length",
            "Number of reads",
            config,
        )?);

        plots.push(histogram::create_log_weighted_histogram(
            &lengths,
            "WeightedLogTransformed_HistogramReadlength",
            "Weighted histogram of read lengths after log transformation",
            "Read length",
            "Number of bases (log scale)",
            config,
        )?);
    }

    // Yield by length plot
    plots.push(yield_plot::create_yield_by_length(
        &lengths,
        "Yield_By_Length",
        config,
    )?);

    // Length vs Quality scatter (if quality data available)
    let reads_with_qual: Vec<_> = reads.iter().filter(|r| r.quality.is_some()).collect();
    if !reads_with_qual.is_empty() {
        let lengths_with_qual: Vec<f64> = reads_with_qual.iter().map(|r| r.length as f64).collect();
        let qualities: Vec<f64> = reads_with_qual.iter().filter_map(|r| r.quality).collect();

        plots.push(scatter::create_scatter(
            &lengths_with_qual,
            &qualities,
            "LengthvsQualityScatterPlot",
            "Read length vs Average read quality",
            "Read length",
            "Average read quality",
            config,
        )?);

        // Log-transformed version if --loglength is set
        if config.loglength {
            plots.push(scatter::create_log_scatter(
                &lengths_with_qual,
                &qualities,
                "LengthvsQualityScatterPlot_loglength",
                "Read length vs Average read quality (log transformed)",
                "Read length",
                "Average read quality",
                config,
            )?);
        }
    }

    // Alignment-specific plots (Phase 2)
    if config.has_alignment {
        // Mapping quality scatter
        let reads_with_mapq: Vec<_> = reads
            .iter()
            .filter(|r| r.mapping_quality.is_some())
            .collect();
        if !reads_with_mapq.is_empty() {
            let lengths_with_mapq: Vec<f64> =
                reads_with_mapq.iter().map(|r| r.length as f64).collect();
            let mapping_quals: Vec<f64> = reads_with_mapq
                .iter()
                .filter_map(|r| r.mapping_quality.map(|q| q as f64))
                .collect();

            plots.push(scatter::create_scatter(
                &lengths_with_mapq,
                &mapping_quals,
                "LengthvsMappingQualityScatterPlot",
                "Read length vs Mapping quality",
                "Read length",
                "Mapping quality",
                config,
            )?);

            // Log-transformed version if --loglength is set
            if config.loglength {
                plots.push(scatter::create_log_scatter(
                    &lengths_with_mapq,
                    &mapping_quals,
                    "LengthvsMappingQualityScatterPlot_loglength",
                    "Read length vs Mapping quality (log transformed)",
                    "Read length",
                    "Mapping quality",
                    config,
                )?);
            }
        }

        // Percent identity histogram
        let percent_ids: Vec<f64> = reads.iter().filter_map(|r| r.percent_identity).collect();

        if !percent_ids.is_empty() {
            plots.push(histogram::create_histogram(
                &percent_ids,
                "PercentIdentityHistogram",
                "Histogram of percent identity",
                "Percent identity",
                "Number of reads",
                None,
                config,
                None,
            )?);

            // Length vs Percent Identity scatter
            let reads_with_pi: Vec<_> = reads
                .iter()
                .filter(|r| r.percent_identity.is_some())
                .collect();
            let lengths_with_pi: Vec<f64> = reads_with_pi.iter().map(|r| r.length as f64).collect();

            plots.push(scatter::create_scatter(
                &lengths_with_pi,
                &percent_ids,
                "LengthvsPercentIdentityScatterPlot",
                "Read length vs Percent identity",
                "Read length",
                "Percent identity",
                config,
            )?);

            // Log-transformed version if --loglength is set
            if config.loglength {
                plots.push(scatter::create_log_scatter(
                    &lengths_with_pi,
                    &percent_ids,
                    "LengthvsPercentIdentityScatterPlot_loglength",
                    "Read length vs Percent identity (log transformed)",
                    "Read length",
                    "Percent identity",
                    config,
                )?);
            }
        }
    }

    // Time-based plots (if time data available)
    if config.has_time_data {
        let time_plot_results = time_plots::generate_time_plots(reads, config)?;
        plots.extend(time_plot_results);
    }

    info!("Generated {} plots", plots.len());
    Ok(plots)
}

/// Save plot to file based on config format
pub fn save_plot(svg: &str, base_name: &str, config: &Config) -> Result<PathBuf> {
    let extension = match config.format.as_str() {
        "png" => "png",
        "pdf" => "pdf",
        _ => "svg",
    };
    let path = config.output_path(&format!("{}.{}", base_name, extension));

    // Ensure output directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match config.format.as_str() {
        "png" => {
            let png_data = svg_to_png(svg, config.dpi)?;
            fs::write(&path, png_data)?;
        }
        "pdf" => {
            let pdf_data = svg_to_pdf(svg)?;
            fs::write(&path, pdf_data)?;
        }
        _ => {
            // Default to SVG
            fs::write(&path, svg)?;
        }
    }

    Ok(path)
}

/// Convert SVG string to PNG bytes
fn svg_to_png(svg: &str, dpi: u32) -> Result<Vec<u8>> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| {
        crate::error::NanoPlotError::PlotError(format!("Failed to parse SVG: {}", e))
    })?;

    let size = tree.size();
    let scale = dpi as f32 / 96.0; // Base DPI is 96
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| crate::error::NanoPlotError::PlotError("Failed to create pixmap".into()))?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| crate::error::NanoPlotError::PlotError(format!("Failed to encode PNG: {}", e)))
}

/// Convert SVG string to PDF bytes
fn svg_to_pdf(svg: &str) -> Result<Vec<u8>> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| {
        crate::error::NanoPlotError::PlotError(format!("Failed to parse SVG: {}", e))
    })?;

    let pdf = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| {
        crate::error::NanoPlotError::PlotError(format!("Failed to convert to PDF: {}", e))
    })?;

    Ok(pdf)
}
