//! Boot Camp hint lifetime, EXW 0x424a6f/0x425010 and command dismissal.
//! Text layout and rendering are separate from the show-once mission state.

const LIFETIME: u32 = 65_000;
const LAST_PAD: [usize; 15] = [
    0x15, 0x1d, 0x2b, 0x33, 0x38, 0x3d, 0x42, 0x48, 0x4b, 0x4f, 0x51, 0x55, 0x66, 0x69, 0x71,
];

/// Zone-A/M1 dispatcher ranges (EXW 0x433d05 and sibling branches).
pub fn hint_for_pad(slot: usize) -> Option<usize> {
    if slot < 0x11 {
        return None;
    }
    LAST_PAD.iter().position(|&last| slot <= last)
}

/// One mission's latches and active box timer. Construct anew at mission load.
#[derive(Debug, Default)]
pub struct HintState {
    seen: [bool; 15],
    current: Option<usize>,
    remaining: u32,
}

impl HintState {
    /// Returns the newly shown id, allowing the caller to start its text/SFX.
    /// Nonzero network modes suppress hints, including cooperative mode.
    pub fn probe(&mut self, zone: u8, mission: u8, network_mode: u8, pad: usize) -> Option<usize> {
        if zone != 1 || mission != 1 || network_mode != 0 {
            return None;
        }
        let id = hint_for_pad(pad)?;
        if self.seen[id] {
            return None;
        }
        self.seen[id] = true;
        self.current = Some(id);
        self.remaining = LIFETIME;
        Some(id)
    }

    pub fn active(&self) -> Option<usize> {
        self.current.filter(|_| self.remaining != 0)
    }

    pub fn remaining(&self) -> u32 {
        self.remaining
    }

    /// Called at the original hint ticker position, once per mission frame.
    pub fn tick(&mut self) {
        self.remaining = self.remaining.saturating_sub(1);
    }

    /// Command flags, not raw buttons: movement bit 0 / firing bit 1.
    /// Strict signed comparisons in EXW 0x40a2bc and 0x40a396.
    pub fn command(&mut self, flags: u8) {
        if (flags & 1 != 0 && self.remaining < 0xfde0)
            || (flags & 2 != 0 && self.remaining < 0xfdd4)
        {
            self.remaining = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dispatch_ranges_cover_exactly_the_97_hint_pads() {
        let counts = [5, 8, 14, 8, 5, 5, 5, 6, 3, 4, 2, 4, 17, 3, 8];
        let mut slot = 17;
        for (id, count) in counts.into_iter().enumerate() {
            for _ in 0..count {
                assert_eq!(hint_for_pad(slot), Some(id));
                slot += 1;
            }
        }
        assert_eq!(slot, 114);
        assert_eq!(hint_for_pad(16), None);
        assert_eq!(hint_for_pad(114), None);
    }

    #[test]
    fn scope_show_once_and_replacement_match_the_original_latches() {
        let mut hints = HintState::default();
        for scope in [(2, 1, 0), (1, 2, 0), (1, 1, 1), (1, 1, 2)] {
            assert_eq!(hints.probe(scope.0, scope.1, scope.2, 17), None);
        }
        assert_eq!(hints.probe(1, 1, 0, 17), Some(0));
        hints.tick();
        assert_eq!(hints.probe(1, 1, 0, 21), None);
        assert_eq!(
            hints.remaining(),
            64_999,
            "revisiting never restarts the timer"
        );
        assert_eq!(hints.probe(1, 1, 0, 22), Some(1));
        assert_eq!(hints.active(), Some(1));
        assert_eq!(hints.probe(1, 1, 0, 17), None);
    }

    #[test]
    fn dismissal_is_strict_and_does_not_reset_the_seen_latch() {
        for (flags, age) in [(1, 8), (2, 20), (3, 8)] {
            let mut hints = HintState::default();
            hints.probe(1, 1, 0, 17);
            for _ in 0..age {
                hints.tick();
            }
            hints.command(flags);
            assert_eq!(hints.active(), Some(0), "equality must not dismiss");
            hints.tick();
            hints.command(flags);
            assert_eq!(hints.active(), None);
            assert_eq!(hints.probe(1, 1, 0, 17), None);
        }
        let mut hints = HintState::default();
        hints.probe(1, 1, 0, 17);
        for _ in 0..LIFETIME {
            hints.tick();
        }
        assert_eq!(hints.active(), None);
        hints.tick();
        assert_eq!(hints.remaining(), 0);
    }
}

mod panel;
pub use panel::HintPanel;
