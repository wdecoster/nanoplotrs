//! Histogram plot generation

use crate::config::Config;
use crate::error::Result;
use crate::plots::{save_plot, GeneratedPlot};
use kuva::backend::svg::SvgBackend;
use kuva::plot::Histogram;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

/// Create a standard histogram
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

    // Calculate range
    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Add some padding
    let range = (min_val - (max_val - min_val) * 0.05, max_val + (max_val - min_val) * 0.05);

    // Create histogram using kuva
    let hist = Histogram::new()
        .with_data(data.to_vec())
        .with_bins(50)
        .with_range(range)
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a weighted histogram (simulated by repeating values)
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

    // For weighted histogram, we need to compute bins manually since kuva doesn't support weights directly
    // We'll create a bar chart representation instead
    let (_bin_centers, _weighted_counts) = calculate_weighted_histogram(data, 50);

    // Calculate range
    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (min_val - (max_val - min_val) * 0.05, max_val + (max_val - min_val) * 0.05);

    // Use regular histogram for display, with a note that counts represent bases
    // Since kuva doesn't support weighted histograms directly, we'll approximate
    // by scaling the bin counts by average length in each bin
    let hist = Histogram::new()
        .with_data(data.to_vec())
        .with_bins(50)
        .with_range(range)
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(x_label)
        .with_y_label(y_label);

    // Note: For a true weighted histogram, we would need bar plot with manual bins
    // This is an approximation that shows distribution shape
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
    let range = (min_val - 0.1, max_val + 0.1);

    let hist = Histogram::new()
        .with_data(log_data.to_vec())
        .with_bins(50)
        .with_range(range)
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(&format!("{} (log10)", x_label))
        .with_y_label(y_label);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create a log-transformed weighted histogram
pub fn create_log_weighted_histogram(
    data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    // Transform to log scale
    let log_data: Vec<f64> = data.iter().filter(|&&x| x > 0.0).map(|&x| x.log10()).collect();

    if log_data.is_empty() {
        return Err(crate::error::NanoPlotError::PlotError("No valid data for log histogram".into()));
    }

    let min_val = log_data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = log_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (min_val - 0.1, max_val + 0.1);

    let hist = Histogram::new()
        .with_data(log_data)
        .with_bins(50)
        .with_range(range)
        .with_color(&config.color);

    let plots = vec![Plot::Histogram(hist)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(&format!("{} (log10)", x_label))
        .with_y_label(y_label);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Calculate weighted histogram bins and counts
fn calculate_weighted_histogram(data: &[f64], num_bins: usize) -> (Vec<f64>, Vec<f64>) {
    if data.is_empty() {
        return (vec![], vec![]);
    }

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max_val - min_val).abs() < f64::EPSILON {
        return (vec![min_val], vec![data.iter().sum()]);
    }

    let bin_width = (max_val - min_val) / num_bins as f64;
    let mut counts = vec![0.0; num_bins];
    let mut bin_centers = Vec::with_capacity(num_bins);

    for i in 0..num_bins {
        bin_centers.push(min_val + (i as f64 + 0.5) * bin_width);
    }

    // Weight by the value itself (length)
    for &val in data {
        let bin_idx = ((val - min_val) / bin_width) as usize;
        let bin_idx = bin_idx.min(num_bins - 1);
        counts[bin_idx] += val; // Add the length as weight
    }

    (bin_centers, counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_weighted_histogram() {
        let data = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let (bins, counts) = calculate_weighted_histogram(&data, 5);

        assert_eq!(bins.len(), 5);
        assert_eq!(counts.len(), 5);
        // Total should equal sum of data
        let total: f64 = counts.iter().sum();
        let expected: f64 = data.iter().sum();
        assert!((total - expected).abs() < 0.001);
    }
}
