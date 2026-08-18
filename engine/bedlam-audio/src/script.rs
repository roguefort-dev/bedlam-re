//! Neutral music script (DESIGN-AUDIO sec 7): the .MRS event stream after
//! the bedlam-assets walk, re-expressed as absolute-tick commands. The
//! bedlam-game MusicPump analog builds this list; the Mixer dispatches it
//! on its internal Q16 tick grid. This crate deliberately takes no
//! dependency on bedlam-assets (coupling rule, DESIGN-AUDIO sec 7).

use crate::AudioError;

/// One pump tick = one .MRS delta tick = 10 ms (RE-EXW-MUSIC sec 2b).
pub const TICKS_PER_SECOND: u32 = 100;

/// Samples per pump tick in Q16: 11025 / 100 = 441 / 4, exactly
/// representable as 441 << 14. The 10 ms grid never rounds - this is what
/// makes script dispatch chunking-invariant (DESIGN-AUDIO sec 5).
pub const TICK_Q16: u64 = 441 << 14;

/// Q16 mix-cursor position of a pump tick boundary.
pub const fn tick_pos_q16(tick: u32) -> u64 {
    tick as u64 * TICK_Q16
}

/// A dispatched music command. Field semantics mirror the .MRS grammar
/// (RE-EXW-MUSIC sec 2b): ratio is the 16.16 RATIO_TABLE value, volume is
/// the raw stream byte (the 0xFF note-off decodes into NoteOff upstream).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicCommand {
    /// Trigger the lowest free sub-voice of the instrument.
    NoteOn {
        instrument: u16,
        ratio: u32,
        volume: u8,
    },
    /// Release the BASE sub-voice of the instrument (sec 6 quirk).
    NoteOff { instrument: u16 },
}

/// Ordered (absolute tick, command) list; ticks non-decreasing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MusicScript {
    events: Vec<(u32, MusicCommand)>,
}

impl MusicScript {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append one command at an absolute tick. Ticks must be
    /// non-decreasing (the walk yields events in stream order).
    pub fn push(&mut self, tick: u32, cmd: MusicCommand) -> Result<(), AudioError> {
        if let Some(&(last, _)) = self.events.last() {
            if tick < last {
                return Err(AudioError::ScriptOutOfOrder { tick, last });
            }
        }
        self.events.push((tick, cmd));
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn events(&self) -> &[(u32, MusicCommand)] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 11025 Hz expressed on the same Q16 grid (1 s of mix cursor).
    const SAMPLES_PER_SECOND_Q16: u64 = 11025u64 << 16;

    #[test]
    fn tick_grid_is_exact() {
        // 4 ticks = 441 samples exactly; 1 tick = 110.25 samples in Q16.
        assert_eq!(tick_pos_q16(0), 0);
        assert_eq!(tick_pos_q16(4), 441u64 << 16);
        assert_eq!(tick_pos_q16(1), (110u64 << 16) + (1u64 << 14));
        assert_eq!(tick_pos_q16(100), SAMPLES_PER_SECOND_Q16);
    }

    #[test]
    fn pushes_must_be_ordered() {
        let mut s = MusicScript::new();
        s.push(10, MusicCommand::NoteOff { instrument: 0 }).unwrap();
        let err = s
            .push(9, MusicCommand::NoteOff { instrument: 0 })
            .unwrap_err();
        assert!(matches!(
            err,
            AudioError::ScriptOutOfOrder { tick: 9, last: 10 }
        ));
        // Equal ticks are fine (chords share a tick).
        s.push(10, MusicCommand::NoteOff { instrument: 1 }).unwrap();
        assert_eq!(s.len(), 2);
        assert!(!s.is_empty());
    }
}
