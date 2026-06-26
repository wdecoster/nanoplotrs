//! Histogram plot generation

use crate::config::Config;
use crate::error::Result;
use crate::plots::{save_plot, GeneratedPlot};
use kuva::backend::svg::SvgBackend;
use kuva::plot::Histogram;
use kuva::render::layout::{Layout, TickFormat};
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;
use std::sync::Arc;

/// Compute a human-readable tick step for a given data range and target tick count.
/// e.g. range=165277, target=5 → step=50000; range=750000, target=5 → step=200000
fn nice_tick_step(range: f64, target_ticks: usize) -> f64 {
    let rough = range / target_ticks as f64;
    let mag = 10f64.powf(rough.log10().floor());
    let norm = rough / mag;
    let nice = if norm <= 1.5 {
        1.0
    } else if norm <= 3.5 {
        2.0
    } else if norm <= 7.5 {
        5.0
    } else {
        10.0
    };
    nice * mag
}

/// Create a standard histogram
#[allow(clippy::too_many_arguments)]
pub fn create_histogram(
    data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    _weights: Option<&[f64]>,
    config: &Config,
    _n50_marker: Option<f64>,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Start from 0 for non-negative data; scale bins like Python: max(round(max/500), 10)
    let range_min = if min_val >= 0.0 { 0.0 } else { min_val };
    let range_max = max_val + (max_val - range_min) * 0.01;
    let num_bins = ((max_val / 500.0).round() as usize).max(10).min(200);
    let tick_step = nice_tick_step(range_max - range_min, 5);

    let hist = Histogram::new()
        .with_data(data.to_vec())
        .with_bins(num_bins)
        .with_range((range_min, range_max))
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label)
        .with_x_tick_format(TickFormat::Integer)
        .with_x_tick_step(tick_step);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a weighted histogram where each read contributes its length to the bin count
/// (y-axis = number of bases, not number of reads)
pub fn create_weighted_histogram(
    data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
    _n50_marker: Option<f64>,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range_min = if min_val >= 0.0 { 0.0 } else { min_val };
    let range_max = max_val + (max_val - range_min) * 0.01;
    let num_bins = ((max_val / 500.0).round() as usize).max(10).min(200);
    let tick_step = nice_tick_step(range_max - range_min, 5);

    let bin_width = (range_max - range_min) / num_bins as f64;
    let edges: Vec<f64> = (0..=num_bins)
        .map(|i| range_min + i as f64 * bin_width)
        .collect();

    // Each read contributes its own length (in bases) to its bin
    let mut counts = vec![0.0f64; num_bins];
    for &val in data {
        if val < range_min || val > range_max {
            continue;
        }
        let bin_idx = ((val - range_min) / bin_width) as usize;
        counts[bin_idx.min(num_bins - 1)] += val;
    }

    let hist = Histogram::from_bins(edges, counts).with_color(&config.color);

    let base_fmt: TickFormat = TickFormat::Custom(Arc::new(|v| {
        if v >= 1_000_000_000.0 {
            format!("{:.1}G", v / 1_000_000_000.0)
        } else if v >= 1_000_000.0 {
            format!("{:.0}M", v / 1_000_000.0)
        } else if v >= 1_000.0 {
            format!("{:.0}k", v / 1_000.0)
        } else {
            format!("{}", v as u64)
        }
    }));

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label)
        .with_x_tick_format(TickFormat::Integer)
        .with_x_tick_step(tick_step)
        .with_y_tick_format(base_fmt);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a log-transformed histogram
pub fn create_log_histogram(
    log_data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    let min_val = log_data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = log_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (min_val.floor(), max_val.ceil());
    // Bins based on the log range (each unit = one order of magnitude)
    let num_bins = ((max_val - min_val) * 30.0).round() as usize;
    let num_bins = num_bins.max(10).min(200);

    let hist = Histogram::new()
        .with_data(log_data.to_vec())
        .with_bins(num_bins)
        .with_range(range)
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    // X-axis is stored in log10 space but labels show actual values (powers of 10)
    let log_fmt: TickFormat = TickFormat::Custom(Arc::new(|v| {
        let actual = 10f64.powf(v);
        if actual >= 1_000_000.0 {
            format!("{}M", (actual / 1_000_000.0).round() as u64)
        } else if actual >= 1_000.0 {
            format!("{}k", (actual / 1_000.0).round() as u64)
        } else {
            format!("{}", actual.round() as u64)
        }
    }));
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label)
        .with_x_tick_step(1.0)
        .with_x_tick_format(log_fmt);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a log-transformed weighted histogram (y-axis = bases, x-axis = log10 read length)
pub fn create_log_weighted_histogram(
    data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    // Keep (log_length, original_length) pairs so we can weight by original length
    let pairs: Vec<(f64, f64)> = data
        .iter()
        .filter(|&&x| x > 0.0)
        .map(|&x| (x.log10(), x))
        .collect();

    if pairs.is_empty() {
        return Err(crate::error::NanoPlotError::PlotError(
            "No valid data for log histogram".into(),
        ));
    }

    let min_val = pairs.iter().map(|(lx, _)| *lx).fold(f64::INFINITY, f64::min);
    let max_val = pairs.iter().map(|(lx, _)| *lx).fold(f64::NEG_INFINITY, f64::max);
    let range = (min_val.floor(), max_val.ceil());
    let num_bins = ((max_val - min_val) * 30.0).round() as usize;
    let num_bins = num_bins.max(10).min(200);

    let bin_width = (range.1 - range.0) / num_bins as f64;
    let edges: Vec<f64> = (0..=num_bins)
        .map(|i| range.0 + i as f64 * bin_width)
        .collect();

    // Weight each read by its original length (number of bases)
    let mut counts = vec![0.0f64; num_bins];
    for (log_len, orig_len) in &pairs {
        let bin_idx = ((log_len - range.0) / bin_width) as usize;
        counts[bin_idx.min(num_bins - 1)] += orig_len;
    }

    let base_fmt_y: TickFormat = TickFormat::Custom(Arc::new(|v| {
        if v >= 1_000_000_000.0 {
            format!("{:.1}G", v / 1_000_000_000.0)
        } else if v >= 1_000_000.0 {
            format!("{:.0}M", v / 1_000_000.0)
        } else if v >= 1_000.0 {
            format!("{:.0}k", v / 1_000.0)
        } else {
            format!("{}", v as u64)
        }
    }));

    let hist = Histogram::from_bins(edges, counts).with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let log_fmt_x: TickFormat = TickFormat::Custom(Arc::new(|v| {
        let actual = 10f64.powf(v);
        if actual >= 1_000_000.0 {
            format!("{}M", (actual / 1_000_000.0).round() as u64)
        } else if actual >= 1_000.0 {
            format!("{}k", (actual / 1_000.0).round() as u64)
        } else {
            format!("{}", actual.round() as u64)
        }
    }));
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label)
        .with_x_tick_step(1.0)
        .with_x_tick_format(log_fmt_x)
        .with_y_tick_format(base_fmt_y);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a percent identity histogram with axis clamped to [floor(min), 100]
pub fn create_identity_histogram(
    data: &[f64],
    filename: &str,
    title: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);

    // Clamp range to [floor(min), 100] — values above 100% are physically impossible
    let range_min = min_val.floor().max(0.0);
    let range_max = 100.0;
    // 0.5% per bin over the actual data range
    let num_bins = (((range_max - range_min) * 2.0).round() as usize).max(10).min(200);

    let hist = Histogram::new()
        .with_data(data.to_vec())
        .with_bins(num_bins)
        .with_range((range_min, range_max))
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label("Percent identity")
        .with_y_label("Number of reads")
        .with_x_tick_format(TickFormat::Fixed(1))
        .with_x_axis_max(100.0);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a Phred-scale accuracy histogram from percent identity values.
///
/// Converts: phred = -10 * log10(1 - percent_identity / 100)
/// e.g. 90% → Q10, 99% → Q20, 99.9% → Q30
pub fn create_phred_histogram(
    percent_identity: &[f64],
    filename: &str,
    title: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    // Convert to phred; skip values at exactly 100% (would be infinity)
    let phred_data: Vec<f64> = percent_identity
        .iter()
        .filter(|&&p| p < 100.0 && p >= 0.0)
        .map(|&p| -10.0 * (1.0 - p / 100.0).log10())
        .collect();

    if phred_data.is_empty() {
        return Err(crate::error::NanoPlotError::PlotError(
            "No valid data for phred histogram".into(),
        ));
    }

    let min_val = phred_data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = phred_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Start from the nearest integer below min; 0.5 phred per bin
    let range_min = min_val.floor().max(0.0);
    let range_max = max_val.ceil() + 1.0;
    let num_bins = (((range_max - range_min) * 2.0).round() as usize).max(10).min(200);

    let hist = Histogram::new()
        .with_data(phred_data)
        .with_bins(num_bins)
        .with_range((range_min, range_max))
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label("Accuracy (Phred scale)")
        .with_y_label("Number of reads")
        .with_x_tick_format(TickFormat::Fixed(0))
        .with_x_tick_step(5.0); // ticks at Q5, Q10, Q15, Q20...

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

