# Bundled font

`DejaVuSans.ttf` is embedded in the `nanoplot` binary (via `include_bytes!` in
`src/plots/mod.rs`) so that PNG and PDF rasterisation always has a font and never
depends on fonts being installed on the user's system — keeping the single,
dependency-free binary promise.

The font is unmodified and distributed under the **DejaVu Fonts License** (a
permissive, Bitstream Vera-derived license that allows redistribution and
bundling). See <https://dejavu-fonts.github.io/License.html> for the full text.
