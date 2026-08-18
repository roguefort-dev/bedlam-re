//! bedlam-audio - hermetic integer mix graph for the Bedlam reimplementation
//! (P3 skeleton; docs/DESIGN-AUDIO.md is the design note this implements).
//!
//! Determinism boundary (DESIGN-AUDIO sec 4 / DECISIONS D17 bucket b): the
//! mix graph is PURE hermetic integer math - no I/O, no clock, no threads,
//! no floats, no unsafe. The DEVICE half (cpal-or-similar stream, hardware
//! rate conversion) is deferred to P4 (open question Q1). Audio state is
//! never hashed (D17 b); the audio determinism gate is byte-identity of the
//! mix stream under any host buffer chunking (tests/determinism.rs).
//!
//! RE anchors live in the design note; the short version:
//! - 11025 Hz 8-bit unsigned mono source waves, both builds (EXW
//!   WAVEFORMATEX + SetFrequency 0x2b11 base; B2 IRQ0-shared PCM driver);
//! - 16.16 pitch ratios straight from RATIO_TABLE (bedlam-assets music.rs);
//! - per-instrument 4 sub-voices, lowest-free probing, note-off releases
//!   the BASE sub-voice only (RE-EXW-MUSIC sec 6 quirk);
//! - master 0..=127 over the EXW (master * vol) / 48 product domain,
//!   linearized to Q8 gains (DECISIONS D25).

#![forbid(unsafe_code)]

pub mod mixer;
pub mod script;

pub use mixer::{Mixer, VoiceRef, MAX_VOICES, SAMPLE_RATE, SUB_VOICES_PER_INST};
pub use script::{tick_pos_q16, MusicCommand, MusicScript, TICKS_PER_SECOND, TICK_Q16};

use thiserror::Error;

/// Errors for the audio crate. thiserror only; host misuse returns Err,
/// never panics (panic = engine bug, PLAN P3 error policy).
#[derive(Debug, Error)]
pub enum AudioError {
    /// A script push arrived with a tick earlier than the previous event.
    /// The .MRS walk yields events in stream order, so this is a caller bug.
    #[error("script tick {tick} precedes the previous event tick {last}")]
    ScriptOutOfOrder { tick: u32, last: u32 },
    /// render interleaves stereo frames, so the buffer needs an even length.
    #[error("render buffer of {len} i16 values is not a whole number of stereo frames")]
    OddBufferLength { len: usize },
    /// A wave must contain at least one sample.
    #[error("wave for instrument {instrument} is empty")]
    EmptyWave { instrument: u16 },
}
