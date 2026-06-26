//! Read filtering functions

use crate::config::FilterSettings;
use log::info;
use nanoget_rs::ReadMetrics;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Apply filters to a collection of reads
pub fn filter_reads(reads: Vec<ReadMetrics>, settings: &FilterSettings) -> Vec<ReadMetrics> {
    let initial_count = reads.len();

    let mut filtered: Vec<ReadMetrics> = reads
        .into_iter()
        .filter(|r| passes_length_filter(r, settings))
        .filter(|r| passes_quality_filter(r, settings))
        .collect();

    // Downsample if requested
    if let Some(n) = settings.downsample {
        filtered = downsample(filtered, n);
    }

    let final_count = filtered.len();
    if final_count < initial_count {
        info!(
            "Filtered {} reads to {} reads ({:.1}% retained)",
            initial_count,
            final_count,
            (final_count as f64 / initial_count as f64) * 100.0
        );
    }

    filtered
}

/// Check if read passes length filters
fn passes_length_filter(read: &ReadMetrics, settings: &FilterSettings) -> bool {
    if let Some(min) = settings.min_length {
        if read.length < min {
            return false;
        }
    }
    if let Some(max) = settings.max_length {
        if read.length > max {
            return false;
        }
    }
    true
}

/// Check if read passes quality filter
fn passes_quality_filter(read: &ReadMetrics, settings: &FilterSettings) -> bool {
    if let Some(min_qual) = settings.min_quality {
        if let Some(qual) = read.quality {
            if qual < min_qual {
                return false;
            }
        }
    }
    true
}

/// Clip reads to the given length percentile for plotting.
/// Intended for plot data only — stats are always computed on the full filtered set.
/// A percentile of 100.0 is a no-op.
pub fn clip_to_percentile_for_plots(reads: &[ReadMetrics], percentile: f64) -> Vec<ReadMetrics> {
    if reads.is_empty() || percentile >= 100.0 {
        return reads.to_vec();
    }

    let mut lengths: Vec<u32> = reads.iter().map(|r| r.length).collect();
    lengths.sort_unstable();

    let idx = ((percentile / 100.0) * lengths.len() as f64) as usize;
    let cutoff = lengths[idx.min(lengths.len() - 1)];

    let before = reads.len();
    let clipped: Vec<ReadMetrics> = reads.iter().filter(|r| r.length <= cutoff).cloned().collect();
    let after = clipped.len();

    if before > after {
        info!(
            "Clipped {} reads above the {:.0}th percentile (>{} bp) from plots",
            before - after,
            percentile,
            cutoff
        );
    }

    clipped
}

/// Randomly downsample reads to N
fn downsample(mut reads: Vec<ReadMetrics>, n: usize) -> Vec<ReadMetrics> {
    if reads.len() <= n {
        return reads;
    }

    let mut rng = thread_rng();
    reads.shuffle(&mut rng);
    reads.truncate(n);

    info!("Downsampled to {} reads", n);
    reads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_read(length: u32, quality: Option<f64>) -> ReadMetrics {
        let mut read = ReadMetrics::new(None, length);
        read.quality = quality;
        read
    }

    #[test]
    fn test_length_filter() {
        let reads = vec![
            make_read(100, None),
            make_read(500, None),
            make_read(1000, None),
            make_read(5000, None),
        ];

        let settings = FilterSettings {
            min_length: Some(200),
            max_length: Some(2000),
            ..Default::default()
        };

        let filtered = filter_reads(reads, &settings);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.length >= 200 && r.length <= 2000));
    }

    #[test]
    fn test_quality_filter() {
        let reads = vec![
            make_read(1000, Some(5.0)),
            make_read(1000, Some(10.0)),
            make_read(1000, Some(15.0)),
            make_read(1000, None),
        ];

        let settings = FilterSettings {
            min_quality: Some(8.0),
            ..Default::default()
        };

        let filtered = filter_reads(reads, &settings);
        // Reads with quality >= 8 or no quality should pass
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_downsample() {
        let reads: Vec<ReadMetrics> = (0..1000).map(|i| make_read(i * 10, None)).collect();

        let settings = FilterSettings {
            downsample: Some(100),
            ..Default::default()
        };

        let filtered = filter_reads(reads, &settings);
        assert_eq!(filtered.len(), 100);
    }
}
