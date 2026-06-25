# NanoPlot

Plotting and statistics for long-read sequencing data. Rust implementation of [NanoPlot](https://github.com/wdecoster/NanoPlot).

## Installation

Pre-built binaries are available on the [releases page](https://github.com/wdecoster/nanoplotrs/releases) for Linux (x86\_64 and ARM64), macOS (Intel and Apple Silicon), and Windows.

### Linux (x86\_64)

```bash
curl -LO https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-VERSION-x86_64-unknown-linux-gnu.tar.gz
tar xzf nanoplot-VERSION-x86_64-unknown-linux-gnu.tar.gz
sudo mv nanoplot /usr/local/bin/
```

### Linux (ARM64)

```bash
curl -LO https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-VERSION-aarch64-unknown-linux-gnu.tar.gz
tar xzf nanoplot-VERSION-aarch64-unknown-linux-gnu.tar.gz
sudo mv nanoplot /usr/local/bin/
```

### macOS (Intel)

```bash
curl -LO https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-VERSION-x86_64-apple-darwin.tar.gz
tar xzf nanoplot-VERSION-x86_64-apple-darwin.tar.gz
sudo mv nanoplot /usr/local/bin/
```

### macOS (Apple Silicon)

```bash
curl -LO https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-VERSION-aarch64-apple-darwin.tar.gz
tar xzf nanoplot-VERSION-aarch64-apple-darwin.tar.gz
sudo mv nanoplot /usr/local/bin/
```

### Windows

Download `nanoplot-VERSION-x86_64-pc-windows-msvc.zip` from the [releases page](https://github.com/wdecoster/nanoplotrs/releases), extract the zip, and add the directory to your `PATH`.

### Build from source

Requires Rust 1.70+ and system libraries: `libclang`, `libbz2`, `liblzma`, `zlib`, `libcurl`, `libssl`.

```bash
git clone https://github.com/wdecoster/nanoplotrs
cd nanoplotrs
cargo build --release
# binary is at target/release/nanoplot
```

---

## Usage

```
nanoplot [OPTIONS] --input <FILE>...
```

### Input

| Flag | Description |
|------|-------------|
| `-i, --input FILE...` | Input file(s) — format is auto-detected from content |

Supported formats: FASTQ, FASTA, BAM, CRAM, unaligned BAM, Nanopore sequencing summary TSV. FASTQ, FASTA, and summary files may be gzip-compressed (`.gz`). Multiple files are accepted and processed together; all files must be the same format. Pass `-` to read from stdin.

### Output options

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --outdir DIR` | `.` | Output directory |
| `-p, --prefix STRING` | | Prefix for all output filenames |
| `-f, --format FORMAT` | `svg` | Plot format: `svg`, `png`, or `pdf` |
| `--dpi NUM` | `300` | DPI for PNG output |
| `--raw` | | Export raw read data as `NanoPlot-data.tsv` |

### Filtering options

| Flag | Description |
|------|-------------|
| `--minlength NUM` | Discard reads shorter than NUM bases |
| `--maxlength NUM` | Discard reads longer than NUM bases |
| `--minqual NUM` | Discard reads with mean quality below NUM |
| `--downsample NUM` | Randomly subsample to NUM reads |
| `--drop-outliers` | Remove top 0.1% of reads by length |

### Plot options

| Flag | Default | Description |
|------|---------|-------------|
| `-c, --color HEX` | `#4CB391` | Plot color (hex or color name) |
| `--title STRING` | | Custom title for all plots |
| `--N50` | | Mark N50 on length histograms |
| `--loglength` | | Use log-transformed read length in scatter plots |

### Other options

| Flag | Default | Description |
|------|---------|-------------|
| `-t, --threads NUM` | `4` | Number of threads |
| `--verbose` | | Enable debug logging |

---

## Output files

All files are written to `--outdir` with optional `--prefix`.

| File | Description |
|------|-------------|
| `NanoStats.tsv` | Summary statistics in TSV format |
| `NanoPlot-report.html` | Interactive HTML report with embedded plots |
| `NanoPlot-data.tsv` | Raw read data per read (only with `--raw`) |

**Plots always generated:**

- `Non_weightedHistogramReadlength` — read count histogram
- `WeightedHistogramReadlength` — bases-weighted length histogram
- `Non_weightedLogTransformed_HistogramReadlength` — log-scaled count histogram
- `WeightedLogTransformed_HistogramReadlength` — log-scaled weighted histogram
- `Yield_By_Length` — cumulative yield vs minimum read length

**With quality data** (FASTQ, BAM, CRAM, summary):

- `LengthvsQualityScatterPlot` — read length vs mean quality

**With alignment data** (BAM, CRAM):

- `PercentIdentityHistogram`
- `LengthvsMappingQualityScatterPlot`
- `LengthvsPercentIdentityScatterPlot`

---

## Examples

```bash
# FASTQ input, results in ./output
nanoplot -i reads.fastq.gz -o output/

# Multiple FASTQ files with quality and length filters
nanoplot -i run1.fastq.gz run2.fastq.gz \
  --minlength 1000 --minqual 10 \
  -o results/

# BAM file with PDF output and N50 marker
nanoplot -i aligned.bam \
  --format pdf --N50 \
  -o plots/ --prefix experiment1_

# Sequencing summary with downsampling
nanoplot -i sequencing_summary.txt \
  --downsample 50000 --loglength \
  -o output/

# Export raw data and drop outliers
nanoplot -i data.fastq.gz \
  --drop-outliers --raw \
  -o output/

# Read from stdin
samtools view -b aligned.bam | nanoplot -i - -o output/
```

---

## License

MIT
