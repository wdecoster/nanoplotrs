//! Scatter plot generation

use crate::config::Config;
use crate::error::Result;
use crate::plots::{save_plot, GeneratedPlot};
use kuva::backend::svg::SvgBackend;
use kuva::plot::scatter::ScatterPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Maximum number of points to plot (for performance)
const MAX_SCATTER_POINTS: usize = 10_000;

/// Create a scatter plot
pub fn create_scatter(
    x_data: &[f64],
    y_data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    // Downsample if too many points
    let (x_sampled, y_sampled) = if x_data.len() > MAX_SCATTER_POINTS {
        downsample_points(x_data, y_data, MAX_SCATTER_POINTS)
    } else {
        (x_data.to_vec(), y_data.to_vec())
    };

    // Convert to kuva format
    let data: Vec<(f64, f64)> = x_sampled
        .iter()
        .zip(y_sampled.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let plot = ScatterPlot::new()
        .with_data(data)
        .with_color(&config.color)
        .with_size(3.0);

    let plots = vec![Plot::Scatter(plot)];
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

/// Create a scatter plot with log-transformed x-axis
pub fn create_log_scatter(
    x_data: &[f64],
    y_data: &[f64],
    filename: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let plot_title = config.title.as_deref().unwrap_or(title);

    // Filter out zero/negative x values and log transform
    let valid_pairs: Vec<(f64, f64)> = x_data
        .iter()
        .zip(y_data.iter())
        .filter(|(&x, _)| x > 0.0)
        .map(|(&x, &y)| (x.log10(), y))
        .collect();

    if valid_pairs.is_empty() {
        return Err(crate::error::NanoPlotError::PlotError(
            "No valid data points for log scatter plot".into(),
        ));
    }

    // Downsample if too many points
    let data = if valid_pairs.len() > MAX_SCATTER_POINTS {
        let mut rng = thread_rng();
        let mut pairs = valid_pairs;
        pairs.shuffle(&mut rng);
        pairs.truncate(MAX_SCATTER_POINTS);
        pairs
    } else {
        valid_pairs
    };

    let plot = ScatterPlot::new()
        .with_data(data)
        .with_color(&config.color)
        .with_size(3.0);

    let plots = vec![Plot::Scatter(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(plot_title)
        .with_x_label(format!("{} (log10)", x_label))
        .with_y_label(y_label);

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Randomly downsample paired data points
fn downsample_points(x: &[f64], y: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut rng = thread_rng();
    let mut indices: Vec<usize> = (0..x.len()).collect();
    indices.shuffle(&mut rng);
    indices.truncate(n);

    let x_sampled: Vec<f64> = indices.iter().map(|&i| x[i]).collect();
    let y_sampled: Vec<f64> = indices.iter().map(|&i| y[i]).collect();

    (x_sampled, y_sampled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downsample_points() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| i as f64 * 2.0).collect();

        let (x_s, y_s) = downsample_points(&x, &y, 10);

        assert_eq!(x_s.len(), 10);
        assert_eq!(y_s.len(), 10);
    }
}
