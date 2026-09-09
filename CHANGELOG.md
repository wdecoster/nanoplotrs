# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-09-09

Results from earlier versions are not comparable to this one. Two long-standing bugs in
nanoget-rs, both fixed in its 0.2.0, changed what NanoPlot reports for FASTQ input.

### Fixed

- **Read qualities were wrong for every FASTQ input.** nanoget-rs averaged the raw ASCII
  quality line as if it were already Phred, so every quality was 33 too high and anything
  above ~Q27 saturated at the cap. On the nanotest data the mean read quality was reported
  as 43.1 where it should be 10.1, and `Reads >Q15` read 371 (100%) where the true answer
  is 0. Every quality histogram, length-vs-quality plot and quality threshold table
  produced from FASTQ was affected. Summary files and unaligned BAM were not.
- **Time plots were missing for gzipped FASTQ.** `.fastq.gz` was detected as plain FASTQ
  rather than rich FASTQ, so channel, start time and run id were dropped and the
  time-based plots never appeared. A run over `reads.fastq.gz` now produces 12 plots where
  it produced 8.
- **bzip2 input and gzipped sequencing summaries were rejected** by format detection.

### Changed

- Read metrics are now stored columnar (one array per field) rather than as a struct per
  read. Extraction of a 2M-read FASTQ peaks at roughly 175 MB against 554 MB before.
  NanoPlot's own plotting buffers are now the larger term; `--downsample` reduces them.
- Malformed input is a hard error rather than a silently wrong answer: a FASTQ record
  whose sequence and quality lines differ in length, or a record in a rich FASTQ with no
  metadata in its header. A file mixing rich and plain headers, which previously degraded
  quietly, now fails with a message naming `--file-type fastq` as the way to read it.
- Zero-length reads are dropped from all input formats, matching python nanoget.
- Percent identity is gap-compressed rather than BLAST-style, so it is not directly
  comparable to python NanoPlot; see the nanoget-rs README for the details.

## [0.1.2] and earlier

Not documented here.
