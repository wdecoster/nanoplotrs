//! Yield by length plot generation

use crate::config::Config;
use crate::error::Result;
use crate::plots::{save_plot, GeneratedPlot};
use kuva::backend::svg::SvgBackend;
use kuva::plot::scatter::ScatterPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;

/// Maximum points to plot
const MAX_YIELD_POINTS: usize = 10_000;

/// Create yield by length plot
///
/// This shows cumulative yield (in Gb) for reads >= each length
pub fn create_yield_by_length(
    lengths: &[f64],
    filename: &str,
    config: &Config,
) -> Result<GeneratedPlot> {
    let title = config.title.as_deref().unwrap_or("Yield by length");

    // Sort lengths in descending order
    let mut sorted_lengths = lengths.to_vec();
    sorted_lengths.sort_by(|a, b| b.partial_cmp(a).unwrap());

    // Calculate cumulative yield
    let mut cumulative_yield = Vec::with_capacity(sorted_lengths.len());
    let mut cum_sum = 0.0;

    for &len in &sorted_lengths {
        cum_sum += len;
        cumulative_yield.push(cum_sum / 1e9); // Convert to Gb
    }

    // Downsample for plotting if needed
    let (x_data, y_data) = if sorted_lengths.len() > MAX_YIELD_POINTS {
        downsample_yield(&sorted_lengths, &cumulative_yield, MAX_YIELD_POINTS)
    } else {
        (sorted_lengths, cumulative_yield)
    };

    // Create scatter plot
    let data: Vec<(f64, f64)> = x_data
        .iter()
        .zip(y_data.iter())
        .map(|(&x, &y)| (x, y))
        .collect();

    let plot = ScatterPlot::new()
        .with_data(data)
        .with_color(&config.color)
        .with_size(3.0);

    let plots = vec![Plot::Scatter(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(title)
        .with_x_label("Read length")
        .with_y_label("Cumulative yield for reads >= length [Gb]");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, filename, config)?;

    Ok(GeneratedPlot {
        title: "Yield by length".to_string(),
        path,
        svg_content: svg,
    })
}

/// Downsample yield data while preserving the shape of the curve
fn downsample_yield(lengths: &[f64], yields: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    if lengths.len() <= n {
        return (lengths.to_vec(), yields.to_vec());
    }

    // Sample evenly across the index range to preserve curve shape
    let step = lengths.len() as f64 / n as f64;
    let mut x_sampled = Vec::with_capacity(n);
    let mut y_sampled = Vec::with_capacity(n);

    for i in 0..n {
        let idx = (i as f64 * step) as usize;
        x_sampled.push(lengths[idx]);
        y_sampled.push(yields[idx]);
    }

    (x_sampled, y_sampled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downsample_yield() {
        let lengths: Vec<f64> = (0..1000).rev().map(|i| i as f64).collect();
        let yields: Vec<f64> = (0..1000).map(|i| i as f64).collect();

        let (x, y) = downsample_yield(&lengths, &yields, 100);

        assert_eq!(x.len(), 100);
        assert_eq!(y.len(), 100);
        // First should be included
        assert_eq!(x[0], 999.0);
    }
}
