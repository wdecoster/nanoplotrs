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
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

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
    info!("Generating plots for {} reads", reads.len());

    // Pre-compute shared data; wrap in Arc so closures can share without cloning the data.
    let lengths = Arc::new(reads.iter().map(|r| r.length as f64).collect::<Vec<_>>());
    let log_lengths: Vec<f64> = lengths
        .iter()
        .filter(|&&l| l > 0.0)
        .map(|&l| l.log10())
        .collect();
    let n50_marker = if config.show_n50 {
        Some(stats.n50 as f64)
    } else {
        None
    };

    type Task = Box<dyn FnOnce() -> Result<GeneratedPlot> + Send>;
    let mut tasks: Vec<Task> = Vec::new();

    // Non-weighted histogram
    {
        let (lengths, config) = (Arc::clone(&lengths), config.clone());
        tasks.push(Box::new(move || {
            histogram::create_histogram(
                &lengths,
                "Non_weightedHistogramReadlength",
                "Non-weighted histogram of read lengths",
                "Read length",
                "Number of reads",
                None,
                &config,
                n50_marker,
            )
        }));
    }

    // Weighted histogram
    {
        let (lengths, config) = (Arc::clone(&lengths), config.clone());
        tasks.push(Box::new(move || {
            histogram::create_weighted_histogram(
                &lengths,
                "WeightedHistogramReadlength",
                "Weighted histogram of read lengths",
                "Read length",
                "Number of bases",
                &config,
                n50_marker,
            )
        }));
    }

    // Log-transformed histograms
    if !log_lengths.is_empty() {
        let log_lengths = Arc::new(log_lengths);
        {
            let (log_lengths, config) = (Arc::clone(&log_lengths), config.clone());
            tasks.push(Box::new(move || {
                histogram::create_log_histogram(
                    &log_lengths,
                    "Non_weightedLogTransformed_HistogramReadlength",
                    "Non-weighted histogram of read lengths after log transformation",
                    "Read length",
                    "Number of reads",
                    &config,
                )
            }));
        }
        {
            let (lengths, config) = (Arc::clone(&lengths), config.clone());
            tasks.push(Box::new(move || {
                histogram::create_log_weighted_histogram(
                    &lengths,
                    "WeightedLogTransformed_HistogramReadlength",
                    "Weighted histogram of read lengths after log transformation",
                    "Read length",
                    "Number of bases (log scale)",
                    &config,
                )
            }));
        }
    }

    // Yield by length
    {
        let (lengths, config) = (Arc::clone(&lengths), config.clone());
        tasks.push(Box::new(move || {
            yield_plot::create_yield_by_length(&lengths, "Yield_By_Length", &config)
        }));
    }

    // Length vs Quality scatter
    let reads_with_qual: Vec<_> = reads.iter().filter(|r| r.quality.is_some()).collect();
    if !reads_with_qual.is_empty() {
        let lq = Arc::new(
            reads_with_qual
                .iter()
                .map(|r| r.length as f64)
                .collect::<Vec<_>>(),
        );
        let qq = Arc::new(
            reads_with_qual
                .iter()
                .filter_map(|r| r.quality)
                .collect::<Vec<_>>(),
        );
        {
            let (lq, qq, config) = (Arc::clone(&lq), Arc::clone(&qq), config.clone());
            tasks.push(Box::new(move || {
                if config.dots {
                    scatter::create_scatter(&lq, &qq, "LengthvsQualityScatterPlot", "Read length vs Average read quality", "Read length", "Average read quality", &config)
                } else {
                    scatter::create_density2d(&lq, &qq, "LengthvsQualityDensityPlot", "Read length vs Average read quality", "Read length", "Average read quality", &config)
                }
            }));
        }
        if config.loglength {
            let (lq, qq, config) = (Arc::clone(&lq), Arc::clone(&qq), config.clone());
            tasks.push(Box::new(move || {
                if config.dots {
                    scatter::create_log_scatter(&lq, &qq, "LengthvsQualityScatterPlot_loglength", "Read length vs Average read quality (log transformed)", "Read length", "Average read quality", &config)
                } else {
                    scatter::create_log_density2d(&lq, &qq, "LengthvsQualityDensityPlot_loglength", "Read length vs Average read quality (log transformed)", "Read length", "Average read quality", &config)
                }
            }));
        }
    }

    // Alignment-specific plots
    if config.has_alignment {
        let reads_with_mapq: Vec<_> = reads
            .iter()
            .filter(|r| r.mapping_quality.is_some())
            .collect();
        if !reads_with_mapq.is_empty() {
            let lm = Arc::new(
                reads_with_mapq
                    .iter()
                    .map(|r| r.length as f64)
                    .collect::<Vec<_>>(),
            );
            let mq = Arc::new(
                reads_with_mapq
                    .iter()
                    .filter_map(|r| r.mapping_quality.map(|q| q as f64))
                    .collect::<Vec<_>>(),
            );
            {
                let (lm, mq, config) = (Arc::clone(&lm), Arc::clone(&mq), config.clone());
                tasks.push(Box::new(move || {
                    if config.dots {
                        scatter::create_scatter(&lm, &mq, "LengthvsMappingQualityScatterPlot", "Read length vs Mapping quality", "Read length", "Mapping quality", &config)
                    } else {
                        scatter::create_density2d(&lm, &mq, "LengthvsMappingQualityDensityPlot", "Read length vs Mapping quality", "Read length", "Mapping quality", &config)
                    }
                }));
            }
            if config.loglength {
                let (lm, mq, config) = (Arc::clone(&lm), Arc::clone(&mq), config.clone());
                tasks.push(Box::new(move || {
                    if config.dots {
                        scatter::create_log_scatter(&lm, &mq, "LengthvsMappingQualityScatterPlot_loglength", "Read length vs Mapping quality (log transformed)", "Read length", "Mapping quality", &config)
                    } else {
                        scatter::create_log_density2d(&lm, &mq, "LengthvsMappingQualityDensityPlot_loglength", "Read length vs Mapping quality (log transformed)", "Read length", "Mapping quality", &config)
                    }
                }));
            }
        }

        // Aligned read length vs sequenced read length
        let reads_with_aln: Vec<_> = reads.iter().filter(|r| r.aligned_length.is_some()).collect();
        if !reads_with_aln.is_empty() {
            let seq_len = Arc::new(reads_with_aln.iter().map(|r| r.length as f64).collect::<Vec<_>>());
            let aln_len = Arc::new(reads_with_aln.iter().filter_map(|r| r.aligned_length.map(|l| l as f64)).collect::<Vec<_>>());
            let (seq_len, aln_len, config) = (Arc::clone(&seq_len), Arc::clone(&aln_len), config.clone());
            tasks.push(Box::new(move || {
                if config.dots {
                    scatter::create_scatter(&seq_len, &aln_len, "AlignedReadlengthvsSequencedReadLength", "Aligned read length vs sequenced read length", "Sequenced read length", "Aligned read length", &config)
                } else {
                    scatter::create_density2d(&seq_len, &aln_len, "AlignedReadlengthvsSequencedReadLength", "Aligned read length vs sequenced read length", "Sequenced read length", "Aligned read length", &config)
                }
            }));
        }

        let percent_ids: Vec<f64> = reads.iter().filter_map(|r| r.percent_identity).collect();
        if !percent_ids.is_empty() {
            let lp = Arc::new(
                reads
                    .iter()
                    .filter(|r| r.percent_identity.is_some())
                    .map(|r| r.length as f64)
                    .collect::<Vec<_>>(),
            );
            let pi = Arc::new(percent_ids);
            {
                let (pi, config) = (Arc::clone(&pi), config.clone());
                tasks.push(Box::new(move || {
                    histogram::create_identity_histogram(
                        &pi,
                        "PercentIdentityHistogram",
                        "Histogram of percent identity",
                        &config,
                    )
                }));
            }
            {
                let (pi, config) = (Arc::clone(&pi), config.clone());
                tasks.push(Box::new(move || {
                    histogram::create_phred_histogram(
                        &pi,
                        "PhredScoreHistogram",
                        "Histogram of accuracy (Phred scale)",
                        &config,
                    )
                }));
            }
            {
                let (lp, pi, config) = (Arc::clone(&lp), Arc::clone(&pi), config.clone());
                tasks.push(Box::new(move || {
                    if config.dots {
                        scatter::create_scatter(&lp, &pi, "LengthvsPercentIdentityScatterPlot", "Read length vs Percent identity", "Read length", "Percent identity", &config)
                    } else {
                        scatter::create_density2d(&lp, &pi, "LengthvsPercentIdentityDensityPlot", "Read length vs Percent identity", "Read length", "Percent identity", &config)
                    }
                }));
            }
            if config.loglength {
                let (lp, pi, config) = (Arc::clone(&lp), Arc::clone(&pi), config.clone());
                tasks.push(Box::new(move || {
                    if config.dots {
                        scatter::create_log_scatter(&lp, &pi, "LengthvsPercentIdentityScatterPlot_loglength", "Read length vs Percent identity (log transformed)", "Read length", "Percent identity", &config)
                    } else {
                        scatter::create_log_density2d(&lp, &pi, "LengthvsPercentIdentityDensityPlot_loglength", "Read length vs Percent identity (log transformed)", "Read length", "Percent identity", &config)
                    }
                }));
            }
        }
    }

    // Run all tasks in parallel
    let mut plots: Vec<GeneratedPlot> = tasks
        .into_par_iter()
        .map(|task| task())
        .collect::<Result<Vec<_>>>()?;

    // Time plots are appended after — they return multiple plots and are rarely present
    if config.has_time_data {
        plots.extend(time_plots::generate_time_plots(reads, config)?);
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
