//! Fixed-timestep time base for the deterministic simulation.

use crate::CoreError;

/// Simulation tick index: count of elapsed fixed timesteps since sim start.
pub type Tick = u64;

/// Nominal simulation rate: 60 ticks per second.
///
/// Decision D16: the original Bedlam EXW main loop is vsync-present-paced
/// (DirectDraw present-locked) with no software frame clock, so the parity
/// simulation runs a fixed 60 Hz time base and presentation pacing is
/// decoupled later. Per-zone tick rates, if the original varies them, are
/// carried as data by the replay/snapshot formats.
pub const NOMINAL_TICK_HZ: u32 = 60;

/// A simulation time base: how many fixed ticks elapse per real second.
///
/// Stored in replays and snapshots so a recording always names the rate it
/// was made at (data, not code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeBase {
    /// Ticks per second. Always > 0 (use [`TimeBase::new`] to validate).
    pub tick_hz: u32,
}

impl TimeBase {
    /// The D16 parity time base: 60 Hz.
    pub const NOMINAL: TimeBase = TimeBase {
        tick_hz: NOMINAL_TICK_HZ,
    };

    /// Construct a time base.
    ///
    /// Returns [`CoreError::BadTickHz`] if `hz == 0`.
    pub fn new(hz: u32) -> Result<Self, CoreError> {
        if hz == 0 {
            Err(CoreError::BadTickHz(hz))
        } else {
            Ok(TimeBase { tick_hz: hz })
        }
    }
}

impl Default for TimeBase {
    fn default() -> Self {
        TimeBase::NOMINAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_is_60hz() {
        assert_eq!(NOMINAL_TICK_HZ, 60);
        assert_eq!(TimeBase::NOMINAL.tick_hz, 60);
        assert_eq!(TimeBase::default().tick_hz, 60);
    }

    #[test]
    fn zero_hz_rejected() {
        assert_eq!(TimeBase::new(0), Err(CoreError::BadTickHz(0)));
        assert!(TimeBase::new(1).is_ok());
        assert_eq!(TimeBase::new(120).unwrap().tick_hz, 120);
    }
}
