//! Time-based plot generation

use crate::config::Config;
use crate::error::Result;
use crate::plots::{save_plot, GeneratedPlot};
use chrono::{DateTime, Utc};
use kuva::backend::svg::SvgBackend;
use kuva::plot::LinePlot;
use kuva::plot::scatter::ScatterPlot;
use kuva::render::layout::Layout;
use kuva::render::plots::Plot;
use kuva::render::render::render_multiple;
use nanoget_rs::ReadMetrics;
use std::collections::HashMap;

/// Time interval for binning (in seconds)
const TIME_BIN_SECONDS: i64 = 600; // 10 minutes

/// Maximum points to plot
const MAX_TIME_POINTS: usize = 10_000;

/// Generate all time-based plots
pub fn generate_time_plots(
    reads: &[ReadMetrics],
    config: &Config,
) -> Result<Vec<GeneratedPlot>> {
    let mut plots = Vec::new();

    // Filter reads with valid start times
    let reads_with_time: Vec<&ReadMetrics> = reads
        .iter()
        .filter(|r| r.start_time.is_some())
        .collect();

    if reads_with_time.is_empty() {
        return Ok(plots);
    }

    // Get the earliest start time as reference
    let min_time = reads_with_time
        .iter()
        .filter_map(|r| r.start_time)
        .min()
        .unwrap();

    // Cumulative yield over time (Gigabases)
    plots.push(create_cumulative_yield_plot(&reads_with_time, min_time, config)?);

    // Cumulative read count over time
    plots.push(create_cumulative_reads_plot(&reads_with_time, min_time, config)?);

    // Number of reads over time (binned)
    plots.push(create_reads_over_time_plot(&reads_with_time, min_time, config)?);

    // Active pores over time (if channel data available)
    let reads_with_channel: Vec<&ReadMetrics> = reads_with_time
        .iter()
        .filter(|r| r.channel_id.is_some())
        .copied()
        .collect();

    if !reads_with_channel.is_empty() {
        plots.push(create_active_pores_plot(&reads_with_channel, min_time, config)?);
    }

    Ok(plots)
}

/// Create cumulative yield over time plot
fn create_cumulative_yield_plot(
    reads: &[&ReadMetrics],
    min_time: DateTime<Utc>,
    config: &Config,
) -> Result<GeneratedPlot> {
    let title = "Cumulative yield over time";

    // Sort reads by start time
    let mut sorted_reads: Vec<_> = reads.iter().collect();
    sorted_reads.sort_by_key(|r| r.start_time.unwrap());

    // Calculate cumulative yield
    let mut data: Vec<(f64, f64)> = Vec::with_capacity(sorted_reads.len());
    let mut cum_yield = 0.0;

    for read in sorted_reads {
        let time_hours = (read.start_time.unwrap() - min_time).num_seconds() as f64 / 3600.0;
        cum_yield += read.length as f64 / 1e9; // Convert to Gb
        data.push((time_hours, cum_yield));
    }

    // Downsample if too many points
    let data = downsample_time_data(data, MAX_TIME_POINTS);

    let plot = LinePlot::new()
        .with_data(data)
        .with_color(&config.color);

    let plots = vec![Plot::Line(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(config.title.as_deref().unwrap_or(title))
        .with_x_label("Time (hours)")
        .with_y_label("Cumulative yield (Gb)");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, "CumulativeYieldPlot_Gigabases", config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create cumulative read count over time plot
fn create_cumulative_reads_plot(
    reads: &[&ReadMetrics],
    min_time: DateTime<Utc>,
    config: &Config,
) -> Result<GeneratedPlot> {
    let title = "Cumulative read count over time";

    // Sort reads by start time
    let mut sorted_reads: Vec<_> = reads.iter().collect();
    sorted_reads.sort_by_key(|r| r.start_time.unwrap());

    // Calculate cumulative count
    let mut data: Vec<(f64, f64)> = Vec::with_capacity(sorted_reads.len());

    for (i, read) in sorted_reads.iter().enumerate() {
        let time_hours = (read.start_time.unwrap() - min_time).num_seconds() as f64 / 3600.0;
        data.push((time_hours, (i + 1) as f64));
    }

    // Downsample if too many points
    let data = downsample_time_data(data, MAX_TIME_POINTS);

    let plot = LinePlot::new()
        .with_data(data)
        .with_color(&config.color);

    let plots = vec![Plot::Line(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(config.title.as_deref().unwrap_or(title))
        .with_x_label("Time (hours)")
        .with_y_label("Cumulative read count");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, "CumulativeYieldPlot_NumberOfReads", config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create reads over time plot (binned by 10-minute intervals)
fn create_reads_over_time_plot(
    reads: &[&ReadMetrics],
    min_time: DateTime<Utc>,
    config: &Config,
) -> Result<GeneratedPlot> {
    let title = "Number of reads over time";

    // Bin reads by time interval
    let mut bins: HashMap<i64, usize> = HashMap::new();

    for read in reads {
        let time_seconds = (read.start_time.unwrap() - min_time).num_seconds();
        let bin = time_seconds / TIME_BIN_SECONDS;
        *bins.entry(bin).or_insert(0) += 1;
    }

    // Convert to sorted data points
    let mut data: Vec<(f64, f64)> = bins
        .iter()
        .map(|(&bin, &count)| {
            let time_hours = (bin * TIME_BIN_SECONDS) as f64 / 3600.0;
            (time_hours, count as f64)
        })
        .collect();
    data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let plot = ScatterPlot::new()
        .with_data(data)
        .with_color(&config.color)
        .with_size(5.0);

    let plots = vec![Plot::Scatter(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(config.title.as_deref().unwrap_or(title))
        .with_x_label("Time (hours)")
        .with_y_label("Reads per 10 minutes");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, "NumberOfReads_Over_Time", config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Create active pores over time plot
fn create_active_pores_plot(
    reads: &[&ReadMetrics],
    min_time: DateTime<Utc>,
    config: &Config,
) -> Result<GeneratedPlot> {
    let title = "Active pores over time";

    // Track unique channels per time bin
    let mut bins: HashMap<i64, std::collections::HashSet<u16>> = HashMap::new();

    for read in reads {
        let time_seconds = (read.start_time.unwrap() - min_time).num_seconds();
        let bin = time_seconds / TIME_BIN_SECONDS;
        if let Some(channel) = read.channel_id {
            bins.entry(bin).or_default().insert(channel);
        }
    }

    // Convert to sorted data points (count of unique channels per bin)
    let mut data: Vec<(f64, f64)> = bins
        .iter()
        .map(|(&bin, channels)| {
            let time_hours = (bin * TIME_BIN_SECONDS) as f64 / 3600.0;
            (time_hours, channels.len() as f64)
        })
        .collect();
    data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let plot = ScatterPlot::new()
        .with_data(data)
        .with_color(&config.color)
        .with_size(5.0);

    let plots = vec![Plot::Scatter(plot)];
    let layout = Layout::auto_from_plots(&plots)
        .with_title(config.title.as_deref().unwrap_or(title))
        .with_x_label("Time (hours)")
        .with_y_label("Active pores per 10 minutes");

    let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
    let path = save_plot(&svg, "ActivePoresOverTime", config)?;

    Ok(GeneratedPlot {
        title: title.to_string(),
        path,
        svg_content: svg,
    })
}

/// Downsample time series data while preserving shape
fn downsample_time_data(data: Vec<(f64, f64)>, max_points: usize) -> Vec<(f64, f64)> {
    if data.len() <= max_points {
        return data;
    }

    let step = data.len() as f64 / max_points as f64;
    let mut result = Vec::with_capacity(max_points);

    for i in 0..max_points {
        let idx = (i as f64 * step) as usize;
        if idx < data.len() {
            result.push(data[idx]);
        }
    }

    // Always include the last point
    if let Some(last) = data.last() {
        if result.last() != Some(last) {
            result.push(*last);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downsample_time_data() {
        let data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64 * 2.0)).collect();
        let downsampled = downsample_time_data(data, 100);

        assert!(downsampled.len() <= 101); // max_points + possibly last point
        assert!(downsampled.len() >= 100);
    }
}
