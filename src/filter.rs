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

    // Drop outliers if requested (remove top 0.1% by length)
    if settings.drop_outliers && !filtered.is_empty() {
        filtered = drop_outliers(filtered);
    }

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

/// Drop outlier reads (top 0.1% by length)
fn drop_outliers(mut reads: Vec<ReadMetrics>) -> Vec<ReadMetrics> {
    if reads.is_empty() {
        return reads;
    }

    // Sort by length to find the cutoff
    let mut lengths: Vec<u32> = reads.iter().map(|r| r.length).collect();
    lengths.sort_unstable();

    // Calculate 99.9th percentile
    let idx = (lengths.len() as f64 * 0.999) as usize;
    let cutoff = lengths.get(idx.min(lengths.len() - 1)).copied().unwrap_or(u32::MAX);

    let before = reads.len();
    reads.retain(|r| r.length <= cutoff);
    let after = reads.len();

    if before > after {
        info!(
            "Dropped {} outlier reads with length > {}",
            before - after,
            cutoff
        );
    }

    reads
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
