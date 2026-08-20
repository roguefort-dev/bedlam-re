//! bedlam-assets: pure, deterministic decoders for Bedlam (1996) asset formats.
//!
//! Library rules (docs/PLAN.md P3):
//! - buffer-in / buffer-out only: no `std::fs`, no environment access, no wall
//!   clock, no renderer dependencies.
//! - parsers must never panic on user-supplied bytes: all reads are bounds
//!   checked; a panic here is an engine bug.
//! - status classification ("parsed" / "heuristic-failed" / ...) is a CLI
//!   concern; this crate returns typed `Ok(struct)` / `Err(AssetsError)`.
#![forbid(unsafe_code)]

pub mod audio;
pub mod bdl;

pub mod codecs;
pub mod language;
pub mod misc;
pub mod mission;
pub mod music;
pub mod pal;
pub mod smk;
pub mod sprites;
pub mod tiles;
pub mod trn;

pub use pal::Palette;

/// Lowercase hex string of the first `n` bytes of `data` (fewer if short).
pub fn hex_head(data: &[u8], n: usize) -> String {
    data.iter()
        .take(n)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

/// Codec-level decode failures. `Display` strings are stable: the inspect CLI
/// embeds them verbatim in its per-image `codec` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    #[error("rle16 stream overrun")]
    Rle16Overrun,
    #[error("literal overrun")]
    LiteralOverrun,
    #[error("raw overrun")]
    RawOverrun,
    #[error("byterle incomplete")]
    ByterleIncomplete,
}

/// Format-level parse failures. The inspect CLI maps these to the exact
/// status/detail strings it has always emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AssetsError {
    /// Buffer shorter than the format's minimum header.
    #[error("too small: {len}B")]
    TooSmall { len: usize },
    /// Buffer is not the format's exact required size.
    #[error("wrong size: {len}B")]
    WrongSize { len: usize },
    /// Declared record count overruns the buffer.
    #[error("count {count} overruns {len}B")]
    CountOverruns { count: usize, len: usize },
    /// Declared record count disagrees with the buffer size.
    #[error("count {count} mismatch")]
    CountMismatch { count: usize },
    /// Buffer length is not a multiple of the record size.
    #[error("len {len}B is not a multiple of the record size")]
    NotMultiple { len: usize },
    /// Grid file size does not match the w/h formula.
    #[error("size {len}B != formula {expected} (w={w} h={h})")]
    SizeFormula {
        len: usize,
        expected: usize,
        w: usize,
        h: usize,
    },
    /// Magic / signature mismatch.
    #[error("bad magic")]
    BadMagic,
    /// A text section heading the format requires is not in the buffer.
    #[error("section heading not found")]
    SectionNotFound,
    /// .MRS size disagrees with the header-table layout formula
    /// (data_off + sum(sizes)). True for all shipped files.
    #[error("mrs layout: {len}B != data_off+sizes {expected}B")]
    MrsLayout { len: usize, expected: usize },
    /// SMK container/codec decode failure (vendored decoder rejected
    /// the data; message text is stable).
    #[error("smk decode: {0}")]
    SmkDecode(&'static str),
    /// SMK byte stream ended before the structure it declares.
    #[error("smk stream truncated")]
    SmkTruncated,
    /// SMK structure violates the format (bad tree, bad record sizes).
    #[error("smk stream invalid")]
    SmkInvalid,
    #[error(transparent)]
    Codec(#[from] CodecError),
}

pub(crate) fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

pub(crate) fn i16le(d: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([d[o], d[o + 1]])
}

pub(crate) fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
