//! Statistics computation for sequencing reads

use crate::error::Result;
use nanoget_rs::ReadMetrics;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Statistics summary for a collection of reads
#[derive(Debug, Clone)]
pub struct Stats {
    pub num_reads: usize,
    pub total_bases: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub stdev_length: f64,
    pub min_length: u32,
    pub max_length: u32,
    pub n50: u64,
    pub mean_quality: Option<f64>,
    pub median_quality: Option<f64>,
    // Alignment stats (for BAM/CRAM)
    pub total_aligned_bases: Option<u64>,
    pub mean_percent_identity: Option<f64>,
    pub median_percent_identity: Option<f64>,
}

impl Stats {
    /// Compute statistics from a collection of reads
    pub fn compute(reads: &[ReadMetrics]) -> Self {
        if reads.is_empty() {
            return Self::empty();
        }

        let num_reads = reads.len();
        let lengths: Vec<u32> = reads.iter().map(|r| r.length).collect();
        let total_bases: u64 = lengths.iter().map(|&l| l as u64).sum();

        // Length statistics
        let mean_length = total_bases as f64 / num_reads as f64;
        let median_length = median(&lengths);
        let stdev_length = std_dev(&lengths, mean_length);
        let min_length = *lengths.iter().min().unwrap_or(&0);
        let max_length = *lengths.iter().max().unwrap_or(&0);
        let n50 = calculate_n50(&lengths, total_bases);

        // Quality statistics
        let qualities: Vec<f64> = reads.iter().filter_map(|r| r.quality).collect();
        let (mean_quality, median_quality) = if !qualities.is_empty() {
            (
                Some(qualities.iter().sum::<f64>() / qualities.len() as f64),
                Some(median_f64(&qualities)),
            )
        } else {
            (None, None)
        };

        // Alignment statistics
        let aligned_lengths: Vec<u32> = reads.iter().filter_map(|r| r.aligned_length).collect();
        let total_aligned_bases = if !aligned_lengths.is_empty() {
            Some(aligned_lengths.iter().map(|&l| l as u64).sum())
        } else {
            None
        };

        let percent_identities: Vec<f64> =
            reads.iter().filter_map(|r| r.percent_identity).collect();
        let (mean_percent_identity, median_percent_identity) = if !percent_identities.is_empty() {
            (
                Some(percent_identities.iter().sum::<f64>() / percent_identities.len() as f64),
                Some(median_f64(&percent_identities)),
            )
        } else {
            (None, None)
        };

        Self {
            num_reads,
            total_bases,
            mean_length,
            median_length,
            stdev_length,
            min_length,
            max_length,
            n50,
            mean_quality,
            median_quality,
            total_aligned_bases,
            mean_percent_identity,
            median_percent_identity,
        }
    }

    fn empty() -> Self {
        Self {
            num_reads: 0,
            total_bases: 0,
            mean_length: 0.0,
            median_length: 0.0,
            stdev_length: 0.0,
            min_length: 0,
            max_length: 0,
            n50: 0,
            mean_quality: None,
            median_quality: None,
            total_aligned_bases: None,
            mean_percent_identity: None,
            median_percent_identity: None,
        }
    }

    /// Format statistics as a human-readable string
    pub fn to_string_report(&self) -> String {
        let mut output = String::new();

        writeln!(output, "General summary:").unwrap();
        writeln!(
            output,
            "Number of reads:          {:>15}",
            format_number(self.num_reads as u64)
        )
        .unwrap();
        writeln!(
            output,
            "Total bases:              {:>15}",
            format_number(self.total_bases)
        )
        .unwrap();
        writeln!(
            output,
            "Median read length:       {:>15.1}",
            self.median_length
        )
        .unwrap();
        writeln!(
            output,
            "Mean read length:         {:>15.1}",
            self.mean_length
        )
        .unwrap();
        writeln!(
            output,
            "STDEV read length:        {:>15.1}",
            self.stdev_length
        )
        .unwrap();
        writeln!(output, "Min read length:          {:>15}", self.min_length).unwrap();
        writeln!(output, "Max read length:          {:>15}", self.max_length).unwrap();
        writeln!(
            output,
            "Read length N50:          {:>15}",
            format_number(self.n50)
        )
        .unwrap();

        if let Some(mean_q) = self.mean_quality {
            writeln!(output, "Mean read quality:        {:>15.1}", mean_q).unwrap();
        }
        if let Some(median_q) = self.median_quality {
            writeln!(output, "Median read quality:      {:>15.1}", median_q).unwrap();
        }

        if let Some(aligned) = self.total_aligned_bases {
            writeln!(output).unwrap();
            writeln!(output, "Alignment summary:").unwrap();
            writeln!(
                output,
                "Total aligned bases:      {:>15}",
                format_number(aligned)
            )
            .unwrap();
        }
        if let Some(mean_pi) = self.mean_percent_identity {
            writeln!(output, "Mean percent identity:    {:>15.2}%", mean_pi).unwrap();
        }
        if let Some(median_pi) = self.median_percent_identity {
            writeln!(output, "Median percent identity:  {:>15.2}%", median_pi).unwrap();
        }

        output
    }

    /// Format statistics as TSV
    pub fn to_tsv(&self) -> String {
        let mut output = String::new();

        writeln!(output, "Metric\tValue").unwrap();
        writeln!(output, "Number of reads\t{}", self.num_reads).unwrap();
        writeln!(output, "Total bases\t{}", self.total_bases).unwrap();
        writeln!(output, "Median read length\t{:.1}", self.median_length).unwrap();
        writeln!(output, "Mean read length\t{:.1}", self.mean_length).unwrap();
        writeln!(output, "STDEV read length\t{:.1}", self.stdev_length).unwrap();
        writeln!(output, "Min read length\t{}", self.min_length).unwrap();
        writeln!(output, "Max read length\t{}", self.max_length).unwrap();
        writeln!(output, "Read length N50\t{}", self.n50).unwrap();

        if let Some(mean_q) = self.mean_quality {
            writeln!(output, "Mean read quality\t{:.1}", mean_q).unwrap();
        }
        if let Some(median_q) = self.median_quality {
            writeln!(output, "Median read quality\t{:.1}", median_q).unwrap();
        }
        if let Some(aligned) = self.total_aligned_bases {
            writeln!(output, "Total aligned bases\t{}", aligned).unwrap();
        }
        if let Some(mean_pi) = self.mean_percent_identity {
            writeln!(output, "Mean percent identity\t{:.2}", mean_pi).unwrap();
        }
        if let Some(median_pi) = self.median_percent_identity {
            writeln!(output, "Median percent identity\t{:.2}", median_pi).unwrap();
        }

        output
    }

    /// Write statistics to file
    pub fn write_to_file(&self, path: &Path, as_tsv: bool) -> Result<()> {
        let content = if as_tsv {
            self.to_tsv()
        } else {
            self.to_string_report()
        };

        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

/// Calculate N50 from read lengths
fn calculate_n50(lengths: &[u32], total_bases: u64) -> u64 {
    if lengths.is_empty() {
        return 0;
    }

    let mut sorted_lengths: Vec<u32> = lengths.to_vec();
    sorted_lengths.sort_unstable_by(|a, b| b.cmp(a)); // Sort descending

    let half_total = total_bases / 2;
    let mut cumsum: u64 = 0;

    for &len in &sorted_lengths {
        cumsum += len as u64;
        if cumsum >= half_total {
            return len as u64;
        }
    }

    sorted_lengths[0] as u64
}

/// Calculate median of u32 values
fn median(values: &[u32]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted: Vec<u32> = values.to_vec();
    sorted.sort_unstable();

    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0
    } else {
        sorted[mid] as f64
    }
}

/// Calculate median of f64 values
fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Calculate standard deviation
fn std_dev(values: &[u32], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let variance: f64 = values
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;

    variance.sqrt()
}

/// Format large numbers with commas
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Write raw read data to TSV file
pub fn write_raw_data(reads: &[ReadMetrics], path: &Path) -> Result<()> {
    let mut file = File::create(path)?;

    // Header
    writeln!(
        file,
        "read_id\tlength\tquality\taligned_length\tmapping_quality\tpercent_identity\tchannel_id\tstart_time\tbarcode"
    )?;

    // Data rows
    for read in reads {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            read.read_id.as_deref().unwrap_or(""),
            read.length,
            read.quality
                .map(|q| format!("{:.2}", q))
                .unwrap_or_default(),
            read.aligned_length
                .map(|l| l.to_string())
                .unwrap_or_default(),
            read.mapping_quality
                .map(|q| q.to_string())
                .unwrap_or_default(),
            read.percent_identity
                .map(|p| format!("{:.2}", p))
                .unwrap_or_default(),
            read.channel_id.map(|c| c.to_string()).unwrap_or_default(),
            read.start_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            read.barcode.as_deref().unwrap_or(""),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reads(lengths: &[u32]) -> Vec<ReadMetrics> {
        lengths.iter().map(|&l| ReadMetrics::new(None, l)).collect()
    }

    #[test]
    fn test_n50_calculation() {
        // Example: lengths 10, 20, 30, 40, 50 -> total 150, half = 75
        // Sorted desc: 50, 40, 30, 20, 10
        // 50 -> cum 50, 50+40=90 >= 75, so N50 = 40
        let lengths = vec![10, 20, 30, 40, 50];
        let total: u64 = lengths.iter().map(|&l| l as u64).sum();
        assert_eq!(calculate_n50(&lengths, total), 40);
    }

    #[test]
    fn test_stats_compute() {
        let reads = make_reads(&[100, 200, 300, 400, 500]);
        let stats = Stats::compute(&reads);

        assert_eq!(stats.num_reads, 5);
        assert_eq!(stats.total_bases, 1500);
        assert_eq!(stats.mean_length, 300.0);
        assert_eq!(stats.median_length, 300.0);
        assert_eq!(stats.min_length, 100);
        assert_eq!(stats.max_length, 500);
    }

    #[test]
    fn test_median() {
        assert_eq!(median(&[1, 2, 3, 4, 5]), 3.0);
        assert_eq!(median(&[1, 2, 3, 4]), 2.5);
        assert_eq!(median(&[5]), 5.0);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(123), "123");
    }
}
