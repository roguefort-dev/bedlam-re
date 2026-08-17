//! bedlam-core: hermetic deterministic simulation core for Bedlam (1996).
//!
//! Charter (docs/PLAN.md sec 7, "Determinism Charter"; phase P3; decision D16):
//! - Hermetic: no I/O, no threads, no wall clock (`std::fs` / `std::time` /
//!   `std::thread` are forbidden in this crate) and no ambient randomness.
//!   Everything that enters the simulation arrives as bytes or arguments.
//! - No floats in sim state: simulation math is integer / fixed-point only
//!   (Q16.16, see [`fx`]).
//! - Fixed timestep = 1 original frame; default time base is 60 Hz nominal
//!   (D16: the original is vsync-present-paced with no software frame clock,
//!   so the parity sim runs fixed 60 Hz and presentation is decoupled later).
//!   The replay/snapshot formats record the time base as data, not code.
//! - Seeded PRNG ([`rng::Pcg32`]): statistically matched to the original
//!   later; the original bit-stream is deliberately NOT mirrored (parity
//!   tier T3).
//! - Per-tick state hash ([`hash`], in-crate FNV-1a 64) is stable across OSes
//!   and Rust versions. `std`'s `DefaultHasher` and hash crates are not used.
//! - No unordered iteration may influence sim state: no HashMap/HashSet in
//!   any sim-state path.
//! - All serialization is little-endian, fixed field order, versioned
//!   ([`FORMAT_VERSION`]).
//! - Panic = engine bug. Replay/snapshot BYTES are user-supplied: parsing
//!   returns typed [`CoreError`]s and never panics. Error `Display` strings
//!   are a stable contract.

#![forbid(unsafe_code)]

pub mod fx;
pub mod hash;
pub mod input;
pub mod replay;
pub mod rng;
pub mod sim;
pub mod time;

/// Version of the replay/snapshot serialization formats written by this
/// crate. Bumped on any breaking layout change; readers reject other values.
pub const FORMAT_VERSION: u16 = 1;

/// Core parse/validation failures. `Display` strings are a stable contract:
/// the parity harnesses embed them verbatim in divergence reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// First 4 bytes do not match the expected format magic.
    #[error("bad magic")]
    BadMagic,
    /// Format version field is not one this build can read.
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u16),
    /// Buffer shorter than the format's required size for its declared
    /// counts/lengths.
    #[error("truncated: need {needed}B, have {have}B")]
    Truncated { needed: usize, have: usize },
    /// Declared tick count does not match the number of input frames
    /// actually present.
    #[error("tick count mismatch: declared {declared}, actual {actual}")]
    TickCountMismatch { declared: u32, actual: usize },
    /// Stored hash does not match the hash recomputed from the bytes.
    #[error("hash mismatch: stored {stored:#x}, computed {computed:#x}")]
    HashMismatch { stored: u64, computed: u64 },
    /// Time base with a zero (or otherwise unusable) tick rate.
    #[error("bad tick rate: {0}Hz")]
    BadTickHz(u32),
}
