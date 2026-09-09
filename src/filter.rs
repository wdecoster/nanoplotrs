//! Read filtering functions
//!
//! Reads are stored columnar, so filtering picks indices and gathers once rather than
//! moving whole records around. The predicates still read row-wise, against a borrowed
//! [`ReadView`].

use crate::config::FilterSettings;
use log::info;
use nanoget_rs::{MetricsCollection, ReadView};
use rand::rng;
use rand::seq::SliceRandom;

/// Apply filters to a collection of reads
pub fn filter_reads(reads: MetricsCollection, settings: &FilterSettings) -> MetricsCollection {
    let initial_count = reads.len();

    let mut indices: Vec<usize> = reads
        .iter()
        .filter(|r| passes_length_filter(r, settings))
        .filter(|r| passes_quality_filter(r, settings))
        .map(|r| r.index())
        .collect();

    // Downsample if requested
    if let Some(n) = settings.downsample {
        downsample(&mut indices, n);
    }

    let final_count = indices.len();
    if final_count == initial_count {
        // Nothing was removed, so there is no point gathering a copy.
        return reads;
    }

    info!(
        "Filtered {} reads to {} reads ({:.1}% retained)",
        initial_count,
        final_count,
        (final_count as f64 / initial_count as f64) * 100.0
    );

    reads.select(&indices)
}

/// Check if read passes length filters
fn passes_length_filter(read: &ReadView<'_>, settings: &FilterSettings) -> bool {
    if let Some(min) = settings.min_length {
        if read.length() < min {
            return false;
        }
    }
    if let Some(max) = settings.max_length {
        if read.length() > max {
            return false;
        }
    }
    true
}

/// Check if read passes quality filter
fn passes_quality_filter(read: &ReadView<'_>, settings: &FilterSettings) -> bool {
    if let Some(min_qual) = settings.min_quality {
        if let Some(qual) = read.quality() {
            if qual < min_qual {
                return false;
            }
        }
    }
    true
}

/// Clip reads to the given length percentile for plotting.
///
/// Intended for plot data only — stats are always computed on the full filtered set.
/// Returns `None` when nothing would be clipped, so callers can keep using the
/// unclipped collection instead of gathering an identical copy of it.
pub fn clip_to_percentile_for_plots(
    reads: &MetricsCollection,
    percentile: f64,
) -> Option<MetricsCollection> {
    if reads.is_empty() || percentile >= 100.0 {
        return None;
    }

    // The length column is borrowed, not projected out of a sequence of structs.
    let mut lengths: Vec<u32> = reads.reads.lengths().to_vec();
    lengths.sort_unstable();

    let idx = ((percentile / 100.0) * lengths.len() as f64) as usize;
    let cutoff = lengths[idx.min(lengths.len() - 1)];

    let indices: Vec<usize> = reads
        .reads
        .lengths()
        .iter()
        .enumerate()
        .filter(|(_, &l)| l <= cutoff)
        .map(|(i, _)| i)
        .collect();

    let before = reads.len();
    let after = indices.len();
    if before == after {
        return None;
    }

    info!(
        "Clipped {} reads above the {:.0}th percentile (>{} bp) from plots",
        before - after,
        percentile,
        cutoff
    );

    Some(reads.select(&indices))
}

/// Randomly downsample to N reads, in place over the selected indices.
fn downsample(indices: &mut Vec<usize>, n: usize) {
    if indices.len() <= n {
        return;
    }

    let mut rng = rng();
    indices.shuffle(&mut rng);
    indices.truncate(n);
    // Gather in index order. No plot depends on it — the time plots sort by start time
    // themselves — but it keeps the `--raw` export in file order and makes the gather
    // sequential over the columns.
    indices.sort_unstable();

    info!("Downsampled to {} reads", n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use nanoget_rs::{ReadColumnsBuilder, ReadMetrics};

    fn collection(reads: Vec<(u32, Option<f64>)>) -> MetricsCollection {
        let mut builder = ReadColumnsBuilder::with_capacity(reads.len());
        for (length, quality) in reads {
            let mut read = ReadMetrics::new(None, length);
            read.quality = quality;
            builder.push(read);
        }
        MetricsCollection::new(builder.finish())
    }

    #[test]
    fn test_length_filter() {
        let reads = collection(vec![(100, None), (500, None), (1000, None), (5000, None)]);

        let settings = FilterSettings {
            min_length: Some(200),
            max_length: Some(2000),
            ..Default::default()
        };

        let filtered = filter_reads(reads, &settings);
        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .all(|r| r.length() >= 200 && r.length() <= 2000));
    }

    #[test]
    fn test_quality_filter() {
        let reads = collection(vec![
            (1000, Some(5.0)),
            (1000, Some(10.0)),
            (1000, Some(15.0)),
            (1000, None),
        ]);

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
        let reads = collection((0..1000).map(|i| (i * 10, None)).collect());

        let settings = FilterSettings {
            downsample: Some(100),
            ..Default::default()
        };

        let filtered = filter_reads(reads, &settings);
        assert_eq!(filtered.len(), 100);
    }

    #[test]
    fn test_clip_to_percentile() {
        let reads = collection((1..=100).map(|i| (i * 100, None)).collect());
        // The cutoff is `lengths[(p/100) * len]` and the comparison is inclusive, so the
        // 90th percentile of 100 reads keeps 91. Pre-existing behaviour, preserved here.
        let clipped = clip_to_percentile_for_plots(&reads, 90.0).expect("clipped");
        assert_eq!(clipped.len(), 91);
        assert!(clipped.iter().all(|r| r.length() <= 9100));

        // 100th percentile is a no-op and must not copy.
        assert!(clip_to_percentile_for_plots(&reads, 100.0).is_none());
    }
}
