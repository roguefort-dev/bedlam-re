//! Versioned input-log replay format.
//!
//! A replay = the seed/config the run started from (time base + seed +
//! initial state hash) plus the exact ordered per-tick input frames.
//! Everything is little-endian, fixed field order.

use crate::input::{self, InputFrame};
use crate::{CoreError, FORMAT_VERSION};

/// File magic for the replay format: `"BDLR"`.
pub const REPLAY_MAGIC: [u8; 4] = *b"BDLR";

/// Header size: magic(4) + version(2) + flags(2) + tick_hz(4) + seed(8) +
/// initial_state_hash(8) + tick_count(4).
const HEADER_LEN: usize = 4 + 2 + 2 + 4 + 8 + 8 + 4;

/// A recorded run: enough to reconstruct a `Sim` bit-exactly (with the
/// matching `SimConfig`) and re-simulate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    /// Format version the bytes were written with.
    pub version: u16,
    /// Ticks per second the recording was made at (data, not code).
    pub tick_hz: u32,
    /// Seed the simulation was created with.
    pub seed: u64,
    /// `state_hash()` of the freshly created `Sim` at tick 0.
    pub initial_state_hash: u64,
    /// One input frame per tick, oldest first.
    pub inputs: Vec<InputFrame>,
}

impl Replay {
    /// Serialize: header, then `tick_count` x 12-byte frames.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.inputs.len() * input::ENCODED_LEN);
        out.extend_from_slice(&REPLAY_MAGIC);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // flags (reserved, 0)
        out.extend_from_slice(&self.tick_hz.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.initial_state_hash.to_le_bytes());
        out.extend_from_slice(&(self.inputs.len() as u32).to_le_bytes());
        for frame in &self.inputs {
            let mut buf = [0u8; input::ENCODED_LEN];
            frame.to_bytes(&mut buf);
            out.extend_from_slice(&buf);
        }
        out
    }

    /// Parse and fully validate a replay buffer. Never panics: every failure
    /// is a typed [`CoreError`], including single-byte truncation.
    pub fn parse(bytes: &[u8]) -> Result<Replay, CoreError> {
        let have = bytes.len();
        if have < HEADER_LEN {
            return Err(CoreError::Truncated {
                needed: HEADER_LEN,
                have,
            });
        }
        if bytes[0..4] != REPLAY_MAGIC {
            return Err(CoreError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FORMAT_VERSION {
            return Err(CoreError::UnsupportedVersion(version));
        }
        // bytes[6..8]: flags, reserved — must be present (covered above), value ignored.
        let tick_hz = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let seed = read_u64(bytes, 12);
        let initial_state_hash = read_u64(bytes, 20);
        let declared = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);

        let total = HEADER_LEN + declared as usize * input::ENCODED_LEN;
        if have < total {
            return Err(CoreError::Truncated {
                needed: total,
                have,
            });
        }
        let inputs = bytes[HEADER_LEN..total]
            .chunks_exact(input::ENCODED_LEN)
            .map(InputFrame::from_bytes)
            .collect();
        if have > total {
            // Leftover bytes: the declared count does not describe the buffer.
            let actual = (have - HEADER_LEN) / input::ENCODED_LEN;
            return Err(CoreError::TickCountMismatch { declared, actual });
        }
        Ok(Replay {
            version,
            tick_hz,
            seed,
            initial_state_hash,
            inputs,
        })
    }
}

/// Read a little-endian `u64` at `at` (caller guarantees the 8 bytes exist).
fn read_u64(bytes: &[u8], at: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(frames: usize) -> Replay {
        let mut inputs = Vec::with_capacity(frames);
        for i in 0..frames {
            inputs.push(InputFrame {
                buttons: i as u32 * 31 + 7,
                mouse_dx: (i as i16).wrapping_mul(3) - 500,
                mouse_dy: 500 - (i as i16).wrapping_mul(2),
                mouse_buttons: (i % 3) as u8,
            });
        }
        Replay {
            version: FORMAT_VERSION,
            tick_hz: 60,
            seed: 0xABCD_1234,
            initial_state_hash: 0x1111_2222_3333_4444,
            inputs,
        }
    }

    #[test]
    fn round_trip_empty() {
        let replay = sample(0);
        let parsed = Replay::parse(&replay.to_bytes()).unwrap();
        assert_eq!(parsed, replay);
        assert_eq!(replay.to_bytes().len(), HEADER_LEN);
    }

    #[test]
    fn round_trip_1000_frames() {
        let replay = sample(1000);
        let bytes = replay.to_bytes();
        assert_eq!(bytes.len(), HEADER_LEN + 1000 * input::ENCODED_LEN);
        let parsed = Replay::parse(&bytes).unwrap();
        assert_eq!(parsed, replay);
    }

    #[test]
    fn every_single_byte_truncation_fails() {
        let bytes = sample(3).to_bytes();
        for i in 0..bytes.len() {
            let err = Replay::parse(&bytes[..i]).unwrap_err();
            assert!(
                matches!(err, CoreError::Truncated { .. }),
                "truncation at {i} gave {err:?}"
            );
        }
    }

    #[test]
    fn flipped_magic_byte_is_bad_magic() {
        let mut bytes = sample(2).to_bytes();
        bytes[0] ^= 0xFF;
        assert_eq!(Replay::parse(&bytes), Err(CoreError::BadMagic));
    }

    #[test]
    fn version_two_is_unsupported() {
        let mut bytes = sample(2).to_bytes();
        bytes[4] = 2;
        bytes[5] = 0;
        assert_eq!(Replay::parse(&bytes), Err(CoreError::UnsupportedVersion(2)));
    }

    #[test]
    fn leftover_bytes_are_tick_count_mismatch() {
        let mut bytes = sample(2).to_bytes();
        bytes.extend_from_slice(&[0u8; 12]); // one extra frame's worth
        assert_eq!(
            Replay::parse(&bytes),
            Err(CoreError::TickCountMismatch {
                declared: 2,
                actual: 3
            })
        );
    }
}
