//! Statistics computation for sequencing reads

use crate::error::Result;
use nanoget_rs::ReadMetrics;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Quality cutoffs (Phred) for the "reads above threshold" table.
const QUALITY_CUTOFFS: [f64; 5] = [5.0, 7.0, 10.0, 12.0, 15.0];
/// Read-length cutoffs (bases) for the "reads above threshold" table.
/// Cutoffs at or above the longest read are skipped (they would always be zero).
const LENGTH_CUTOFFS: [u64; 7] = [
    10_000, 25_000, 50_000, 100_000, 200_000, 500_000, 1_000_000,
];

/// Aggregate for one "reads above a cutoff" row: how many reads clear the cutoff,
/// what fraction of all reads that is, and how many megabases they account for.
#[derive(Debug, Clone)]
pub struct ThresholdStat {
    /// Quality (Phred) or length (bases) cutoff.
    pub cutoff: f64,
    pub reads: usize,
    pub percent: f64,
    pub megabases: f64,
}

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
    // Reads above quality/length cutoffs (quality is empty when no quality data)
    pub quality_thresholds: Vec<ThresholdStat>,
    pub length_thresholds: Vec<ThresholdStat>,
}

/// Format a length cutoff for display, e.g. 10_000 -> "10kb", 1_000_000 -> "1Mb".
pub fn fmt_length_cutoff(cutoff: u64) -> String {
    if cutoff >= 1_000_000 && cutoff.is_multiple_of(1_000_000) {
        format!("{}Mb", cutoff / 1_000_000)
    } else {
        format!("{}kb", cutoff / 1_000)
    }
}

/// Serialize a list of threshold aggregates to a JSON array.
fn thresholds_to_json(items: &[ThresholdStat]) -> String {
    let entries: Vec<String> = items
        .iter()
        .map(|t| {
            format!(
                "{{\"cutoff\": {}, \"reads\": {}, \"percent\": {:.1}, \"megabases\": {:.1}}}",
                t.cutoff, t.reads, t.percent, t.megabases
            )
        })
        .collect();
    format!("[{}]", entries.join(", "))
}

/// Count reads clearing each quality cutoff, with their fraction and megabases.
fn compute_quality_thresholds(reads: &[ReadMetrics], num_reads: usize) -> Vec<ThresholdStat> {
    let pairs: Vec<(f64, u64)> = reads
        .iter()
        .filter_map(|r| r.quality.map(|q| (q, r.length as u64)))
        .collect();
    if pairs.is_empty() {
        return Vec::new();
    }
    QUALITY_CUTOFFS
        .iter()
        .map(|&cutoff| {
            let bases: u64 = pairs.iter().filter(|(q, _)| *q >= cutoff).map(|(_, l)| *l).sum();
            let count = pairs.iter().filter(|(q, _)| *q >= cutoff).count();
            ThresholdStat {
                cutoff,
                reads: count,
                percent: count as f64 / num_reads as f64 * 100.0,
                megabases: bases as f64 / 1e6,
            }
        })
        .collect()
}

/// Count reads clearing each length cutoff (below the maximum), with fraction and megabases.
fn compute_length_thresholds(lengths: &[u32], num_reads: usize, max_length: u32) -> Vec<ThresholdStat> {
    LENGTH_CUTOFFS
        .iter()
        .filter(|&&cutoff| cutoff < max_length as u64)
        .map(|&cutoff| {
            let bases: u64 = lengths
                .iter()
                .map(|&l| l as u64)
                .filter(|&l| l >= cutoff)
                .sum();
            let count = lengths.iter().filter(|&&l| l as u64 >= cutoff).count();
            ThresholdStat {
                cutoff: cutoff as f64,
                reads: count,
                percent: count as f64 / num_reads as f64 * 100.0,
                megabases: bases as f64 / 1e6,
            }
        })
        .collect()
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

        let quality_thresholds = compute_quality_thresholds(reads, num_reads);
        let length_thresholds = compute_length_thresholds(&lengths, num_reads, max_length);

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
            quality_thresholds,
            length_thresholds,
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
            quality_thresholds: Vec::new(),
            length_thresholds: Vec::new(),
        }
    }

    fn to_tsv(&self) -> String {
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

        // Reads above thresholds: emitted as atomic (count / % / Mb) rows per cutoff
        // so the file stays a strict two-column Metric/Value table.
        for t in &self.quality_thresholds {
            writeln!(output, "Reads >Q{:.0} (count)\t{}", t.cutoff, t.reads).unwrap();
            writeln!(output, "Reads >Q{:.0} (%)\t{:.1}", t.cutoff, t.percent).unwrap();
            writeln!(output, "Reads >Q{:.0} (Mb)\t{:.1}", t.cutoff, t.megabases).unwrap();
        }
        for t in &self.length_thresholds {
            let label = fmt_length_cutoff(t.cutoff as u64);
            writeln!(output, "Reads >{} (count)\t{}", label, t.reads).unwrap();
            writeln!(output, "Reads >{} (%)\t{:.1}", label, t.percent).unwrap();
            writeln!(output, "Reads >{} (Mb)\t{:.1}", label, t.megabases).unwrap();
        }

        output
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.to_tsv().as_bytes())?;
        Ok(())
    }

    fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"num_reads\": {},\n", self.num_reads));
        s.push_str(&format!("  \"total_bases\": {},\n", self.total_bases));
        s.push_str(&format!("  \"mean_length\": {:.1},\n", self.mean_length));
        s.push_str(&format!("  \"median_length\": {:.1},\n", self.median_length));
        s.push_str(&format!("  \"stdev_length\": {:.1},\n", self.stdev_length));
        s.push_str(&format!("  \"min_length\": {},\n", self.min_length));
        s.push_str(&format!("  \"max_length\": {},\n", self.max_length));
        s.push_str(&format!("  \"n50\": {},\n", self.n50));
        match self.mean_quality {
            Some(v) => s.push_str(&format!("  \"mean_quality\": {:.1},\n", v)),
            None => s.push_str("  \"mean_quality\": null,\n"),
        }
        match self.median_quality {
            Some(v) => s.push_str(&format!("  \"median_quality\": {:.1},\n", v)),
            None => s.push_str("  \"median_quality\": null,\n"),
        }
        match self.total_aligned_bases {
            Some(v) => s.push_str(&format!("  \"total_aligned_bases\": {},\n", v)),
            None => s.push_str("  \"total_aligned_bases\": null,\n"),
        }
        match self.mean_percent_identity {
            Some(v) => s.push_str(&format!("  \"mean_percent_identity\": {:.2},\n", v)),
            None => s.push_str("  \"mean_percent_identity\": null,\n"),
        }
        match self.median_percent_identity {
            Some(v) => s.push_str(&format!("  \"median_percent_identity\": {:.2},\n", v)),
            None => s.push_str("  \"median_percent_identity\": null,\n"),
        }
        s.push_str(&format!(
            "  \"quality_thresholds\": {},\n",
            thresholds_to_json(&self.quality_thresholds)
        ));
        s.push_str(&format!(
            "  \"length_thresholds\": {}\n",
            thresholds_to_json(&self.length_thresholds)
        ));
        s.push('}');
        s
    }

    pub fn write_json_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.to_json().as_bytes())?;
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
    if sorted.len().is_multiple_of(2) {
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
    if sorted.len().is_multiple_of(2) {
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
}
