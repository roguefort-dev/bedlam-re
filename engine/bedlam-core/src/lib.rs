//! bedlam-core: hermetic deterministic simulation core for Bedlam (1996).
//!
//! Charter (docs/PLAN.md sec 7, "Determinism Charter"; phase P3;
//! decisions D16 + D17):
//! - Hermetic: no I/O, no threads, no wall clock (`std::fs` / `std::time` /
//!   `std::thread` are forbidden in this crate) and no ambient randomness.
//!   Everything that enters the simulation arrives as bytes or arguments.
//! - No floats in sim state: simulation math is integer / fixed-point only
//!   (Q16.16, see [`fx`]).
//! - Fixed timestep = 1 original frame; default time base is 60 Hz nominal
//!   (D16: the original is vsync-present-paced with no software frame clock,
//!   so the parity sim runs fixed 60 Hz and presentation is decoupled later).
//!   The replay/snapshot formats record the time base as data, not code.
//! - Timing model (D17, hybrid): (a) sim/physics = FIXED 60 Hz timestep
//!   accumulator and NEVER dt, so replay + state hash stay exact; (b) input
//!   polling, UI hit-tests, cooldowns, cursor, audio/video run per-frame at
//!   host refresh with dt ([`frame::FrameState`], deliberately excluded
//!   from the state hash) — mirroring the original architecture (per-frame
//!   poll in the present-paced loop + 100 Hz service satellites); (c) the
//!   satellite clocks are integer substeps of the sim tick (100 Hz service
//!   = 5 per 3 ticks, 50 Hz fade while fading, 12.5 Hz palette cycle) inside
//!   the hashed [`sim::Sim`].
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

pub mod claim_rects;
pub mod critter;
pub mod destroy;
pub mod frame;
pub mod fx;
pub mod hash;
pub mod input;
pub mod mission;
pub mod mode;
pub mod poi;
pub mod replay;
pub mod rng;
pub mod sim;
pub mod time;
pub mod weapon;

/// Version of the replay/snapshot serialization formats written by this
/// crate. Bumped on any breaking layout change; readers reject other
/// values. PRE-RELEASE: these skeleton formats may still change freely
/// until the first release — no compatibility obligation yet.
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
