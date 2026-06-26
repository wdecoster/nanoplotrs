# NanoPlot internals

This document describes how NanoPlot (Rust) ingests data, computes statistics,
and generates plots. It is aimed at people who want to understand or modify the
tool beyond what the [README](../README.md) describes. None of this is required
to use NanoPlot.

Read metrics are extracted with [nanoget-rs](https://github.com/wdecoster/nanoget-rs);
plots are rendered with [kuva](https://psy-fer.github.io/kuva/).

## Input format auto-detection

A single `--input` may be FASTQ, FASTA, BAM, CRAM, unaligned BAM, or a Nanopore
sequencing-summary TSV. The format is sniffed from file content (not the
extension) by `FileType::sniff`. When several files are passed they must all
sniff to the same type, otherwise the run aborts with a `MixedFileTypes` error.
For stdin (`-`) the type is detected from the leading bytes inside the extractor.

Which metrics are available depends on the source:

| Source | Length | Basecall quality | Alignment identity | Time / channel |
|--------|:------:|:----------------:|:------------------:|:--------------:|
| FASTA | ✓ | | | |
| FASTQ | ✓ | ✓ | | |
| unaligned BAM | ✓ | ✓ | | |
| aligned BAM/CRAM | ✓ | | ✓ | |
| sequencing summary | ✓ | ✓ | | ✓ |

Aligned BAM/CRAM is deliberately summarised by **alignment percent identity and
mapping quality**, not by per-base basecall quality — for aligned data, identity
against the reference is the more informative signal, and the per-read mean
qscore is left to the unaligned/FASTQ paths.

## Counting alignment records once per read

A long read can produce several records in a BAM/CRAM file: one **primary**
alignment plus any number of **secondary** (flag `0x100`) and **supplementary**
(flag `0x800`, chimeric/split) alignments. NanoPlot reports per-read statistics,
so each read must be counted exactly once.

The subtlety is in how record length is measured: `length = record.seq().len()`,
the length of the `SEQ` field of that record.

- A **primary** alignment is soft-clipped — its `SEQ` contains the entire read,
  so its length is the true read length.
- A **supplementary** alignment is hard-clipped (the SAM standard) — its `SEQ`
  holds only the aligned sub-segment, not the read.
- A **secondary** alignment has `SEQ = *` (`seq().len() == 0`) or is hard-clipped.

Including these extra records therefore corrupts the length and yield
distributions, not just the read count. For a read of length `L` split into a
primary (full `L`) and one supplementary segment `b`, summing record lengths
gives `L + b` — the read is counted twice and the segment `b` is counted twice
in the yield. Secondary records can inject phantom length-0 reads.

Accordingly:

- **Secondary alignments are always excluded** (in nanoget-rs, independently of
  any flag).
- **Supplementary alignments are excluded by default**; `--use_supplementary`
  includes them for callers who specifically want split-alignment segments
  (accepting the inflated read count and yield that implies).

## Statistics

Statistics are computed in `src/stats.rs` over the filtered read set, *before*
any plotting-only percentile clipping (see below), so the numbers always reflect
all reads that passed `--minlength` / `--maxlength` / `--minqual` / `--downsample`.

- **N50** — reads sorted by length descending; the length at which the running
  sum of bases first reaches half of the total.
- **Length** — mean, median, standard deviation, min, max, total bases.
- **Quality** — mean and median, only when basecall quality is present.
- **Alignment** — total aligned bases and mean/median percent identity, only for
  aligned BAM/CRAM.

### Reads-above-threshold table

To keep `NanoStats.tsv` a strict two-column `Metric`/`Value` table with atomic
(numeric) values, the "reads above a cutoff" aggregates are emitted as three
rows per cutoff — count, percentage of reads, and megabases — rather than a
single composite string:

```
Reads >Q10 (count)   12345
Reads >Q10 (%)       34.5
Reads >Q10 (Mb)      678.9
```

- **Quality cutoffs**: Phred `5, 7, 10, 12, 15`, emitted only when quality data
  is present. A read clears a cutoff when its mean quality is `>=` the cutoff.
- **Length cutoffs**: `10kb, 25kb, 50kb, 100kb, 200kb, 500kb, 1Mb`, restricted to
  cutoffs strictly below the longest read so no always-zero rows are produced.

The same aggregates appear as nested arrays in `NanoStats.json` (which has no
two-column constraint) and as a rendered table in the HTML report.

Per-read "top N longest / highest quality" lists are intentionally not produced:
a single very long read is rarely informative.

## Plots

Plots are generated in `src/plots/`. Length histograms (count and
bases-weighted, each with a log-transformed variant) and the yield-by-length
curve are always produced. Length-vs-quality is added when quality is present;
percent-identity and mapping-quality plots when alignment data is present.

By default 2D scatter relationships are drawn as density (`hist2d`) plots;
`--dots` switches to a plain dot scatter and `--colormap` selects the density
colormap. `--loglength` adds a log-scaled length axis variant.

### Time plots (sequencing summary)

When reads carry start times (sequencing-summary input), `src/plots/time_plots.rs`
adds cumulative yield and read-count curves, a binned reads-over-time plot
(10-minute bins), violin plots of length and quality over time (3-hour bins,
capped at 24 groups to avoid label crowding), and — when channel identifiers are
present — an active-pores-over-time plot. Long series are downsampled to a few
thousand points for rendering while preserving shape.

## Plotting-only percentile clipping

`--percentile` (default 99) clips the longest `(100 - p)%` of reads from the
**plot data only**, via `clip_to_percentile_for_plots`. This keeps a handful of
ultra-long reads from compressing the informative part of the histograms. It
never affects the statistics, which are computed on the full filtered set. Set
`--percentile 100` to disable clipping.
