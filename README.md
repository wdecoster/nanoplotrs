# NanoPlot

Plotting and statistics for long-read sequencing data. Rust implementation of [NanoPlot](https://github.com/wdecoster/NanoPlot).

## Installation

Pre-built binaries are attached to each [release](https://github.com/wdecoster/nanoplotrs/releases) for Linux (x86\_64) and macOS. Download the binary, make it executable, and move it to any directory on your `PATH` — `~/.local/bin` works without root, `/usr/local/bin` if you have it.

### Linux (dynamically linked, glibc)

```bash
curl -L -o nanoplot https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-linux
chmod +x nanoplot
mv nanoplot ~/.local/bin/
```

### Linux (static, musl)

Portable build with no glibc dependency — useful on older systems and minimal
containers (when available; see notes below).

```bash
curl -L -o nanoplot https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-linux-musl
chmod +x nanoplot
mv nanoplot ~/.local/bin/
```

### macOS

```bash
curl -L -o nanoplot https://github.com/wdecoster/nanoplotrs/releases/latest/download/nanoplot-macos
chmod +x nanoplot
mv nanoplot ~/.local/bin/
```

Windows is not shipped as a binary; use WSL (the Linux binary) or build from source.

### Build from source

Requires Rust 1.70+ and system libraries: `libclang`, `libbz2`, `liblzma`, `zlib`, `libcurl`, `libssl`.

```bash
git clone https://github.com/wdecoster/nanoplotrs
cd nanoplotrs
cargo build --release
# binary is at target/release/nanoplot
```

### Development

After cloning, install the git hooks once. They run `cargo fmt`, `clippy`, and
the tests on commit/push, which catches the formatting and lint issues that are
the most common cause of CI failures.

```bash
make install-hooks
```

Run `make help` for the other available targets (`fmt`, `clippy`, `test`, `ci`, …).

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

For aligned BAM/CRAM, each read is counted once: secondary alignments are always
excluded, and supplementary alignments are excluded by default (use
`--use_supplementary` to include them). See [docs/internals.md](docs/internals.md)
for why.

A "rich" FASTQ — one whose headers carry MinKNOW/albacore metadata (`ch=`,
`start_time=`, … or the newer SAM-style `ch:i:`/`st:Z:` fields) — is detected
automatically, and the per-read channel and timestamp it contains unlock the
time-resolved plots (cumulative yield, reads over time, active pores, and length
and quality over time), just like a sequencing summary.

### Output options

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --outdir DIR` | `.` | Output directory |
| `-p, --prefix STRING` | | Prefix for all output filenames |
| `-f, --format FORMAT` | `svg` | Plot format: `svg`, `png`, or `pdf` |
| `--dpi NUM` | `300` | DPI for PNG output |
| `--raw` | | Export raw read data as `NanoPlot-data.tsv` |
| `--json` | | Also write statistics as JSON (`NanoStats.json`) alongside the TSV |

### Filtering options

| Flag | Description |
|------|-------------|
| `--minlength NUM` | Discard reads shorter than NUM bases |
| `--maxlength NUM` | Discard reads longer than NUM bases |
| `--minqual NUM` | Discard reads with mean quality below NUM |
| `--downsample NUM` | Randomly subsample to NUM reads |
| `--use_supplementary` | Include supplementary alignments from BAM/CRAM (excluded by default) |

These filters apply to both the statistics and the plots. To exclude extreme
read lengths from the plots *only* (without affecting statistics), use
`--percentile` under [Plot options](#plot-options).

### Plot options

| Flag | Default | Description |
|------|---------|-------------|
| `-c, --color HEX` | `#4CB391` | Plot color (hex or color name) |
| `--colormap NAME` | `viridis` | Colormap for 2D density plots: `viridis`, `inferno`, `turbo`, or `grayscale` |
| `--dots` | | Use dot scatter plots instead of the default 2D density plots |
| `--title STRING` | | Custom title for all plots |
| `--N50` | | Mark N50 on length histograms |
| `--loglength` | | Additionally show log-transformed read length in scatter plots |
| `--percentile NUM` | `99` | Read-length percentile shown in plots. Clips the top `(100 - NUM)%` longest reads from the **plots only** — statistics are always computed on all (filtered) reads. Set `100` to show all reads. |

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
| `NanoStats.json` | Summary statistics in JSON format (only with `--json`) |
| `NanoPlot-report.html` | Interactive HTML report with embedded plots |
| `NanoPlot-data.tsv` | Raw read data per read (only with `--raw`) |

`NanoStats.tsv` is a two-column `Metric`/`Value` table. Alongside the summary
metrics (read count, N50, mean/median length and quality, etc.) it lists how many
reads, what fraction, and how many megabases fall above a set of quality and read-
length cutoffs (e.g. `Reads >Q10 (count)`, `Reads >10kb (Mb)`). The HTML report
renders the same cutoffs as a table.

**Plots always generated:**

- `Non_weightedHistogramReadlength` — read count histogram
- `WeightedHistogramReadlength` — bases-weighted length histogram
- `Non_weightedLogTransformed_HistogramReadlength` — log-scaled count histogram
- `WeightedLogTransformed_HistogramReadlength` — log-scaled weighted histogram
- `Yield_By_Length` — cumulative yield vs minimum read length

**With basecall quality** (FASTQ, unaligned BAM, summary):

- `LengthvsQualityScatterPlot` — read length vs mean quality

Aligned BAM/CRAM input is summarised by alignment identity rather than basecall
quality (see below).

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

# Export raw data; clip the longest 5% of reads from the plots only
nanoplot -i data.fastq.gz \
  --percentile 95 --raw \
  -o output/

# Read from stdin
samtools view -b aligned.bam | nanoplot -i - -o output/
```

---

## How this was built

In the interest of transparency: this Rust port was not written by hand. The
code was written and tested by [Claude](https://claude.com/claude-code),
Anthropic's coding agent, working under the supervision of
[Wouter De Coster](https://github.com/wdecoster) — who directed the design,
decided what the tool should do and which features to add, and reviewed the
result.

## Further reading

Implementation details — input format auto-detection, how alignment records are
counted (secondary/supplementary handling), the statistics and threshold
definitions, and how each plot is generated — are documented in
[docs/internals.md](docs/internals.md). None of this is required to use NanoPlot.

## License

MIT
