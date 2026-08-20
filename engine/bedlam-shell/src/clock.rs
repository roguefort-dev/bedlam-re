//! The fixed-step present clock (P4 step 1).
//!
//! Contract (PLAN sec 6 P6 "time-based simulation" + the Determinism
//! Charter, D17): the display pace NEVER enters the simulation. The
//! window host measures a frame delta, hands it to
//! [`FixedStepClock::advance`], and gets back how many 60 Hz host
//! pumps are due; every pump then runs the SAME fixed dt
//! ([`SUBTICKS_PER_PUMP`] sub-ticks on the bedlam-core 240 Hz grid),
//! so the hashed state is bit-identical no matter what the display
//! did. A 60 Hz monitor pumps 1:1; 240 Hz mostly presents zero-tick
//! frames; a stall executes at most `max_catchup` pumps and DISCARDS
//! the surplus banked time (drop-sim-time, never fast-forward - the
//! anti-spiral-of-death policy).
//!
//! The arithmetic is pure integer math (u128 rationals, no floats, no
//! clock reads) so the exact banking/clamping behavior is unit-pinned.

/// Host pump rate: 60 Hz (the D17 hashed tick grid; the FUN_0043d00b
/// host frame pace).
pub const HOST_HZ: u32 = 60;

/// Nanoseconds per second (the clock rational denominator).
const NS_PER_S: u64 = 1_000_000_000;

/// dt handed to [`bedlam_game::GameHost::pump_frame`] for every pump:
/// 4 sub-ticks on the 240 Hz bedlam-core grid = exactly one 60 Hz
/// tick, no banking inside the driver (the SHELL owns quantization).
pub const SUBTICKS_PER_PUMP: u32 = 4;

/// Default anti-spiral clamp: at most 4 catch-up pumps per presented
/// frame (~66 ms of sim per vsync) before surplus time is dropped.
pub const DEFAULT_MAX_CATCHUP: u32 = 4;

/// Fixed-step accumulator over integer nanoseconds.
///
/// `rate_num` ticks are due per `rate_den` nanoseconds (60 per
/// 1_000_000_000 for [`HOST_HZ`]). All state is u128 integers; after
/// each advance the bank is the exact integer remainder
/// `acc - due * rate_den / rate_num`, carried into the next frame.
#[derive(Debug, Clone)]
pub struct FixedStepClock {
    rate_num: u64,
    rate_den: u64,
    acc_ns: u128,
    max_catchup: u32,
    ticks_executed: u64,
    ticks_dropped: u64,
}

impl FixedStepClock {
    /// A clock at `hz` with the given anti-spiral clamp.
    pub fn new(hz: u32, max_catchup: u32) -> FixedStepClock {
        assert!(hz > 0, "clock rate must be positive");
        FixedStepClock {
            rate_num: u64::from(hz),
            rate_den: NS_PER_S,
            acc_ns: 0,
            max_catchup,
            ticks_executed: 0,
            ticks_dropped: 0,
        }
    }

    /// The 60 Hz host clock with [`DEFAULT_MAX_CATCHUP`].
    pub fn host() -> FixedStepClock {
        FixedStepClock::new(HOST_HZ, DEFAULT_MAX_CATCHUP)
    }

    /// Feed one measured frame delta; returns the pumps due now.
    ///
    /// `due = floor(acc * rate_num / rate_den)`; executed is
    /// `min(due, max_catchup)`; the bank ALWAYS resets to the integer
    /// remainder of the full `due` (clamped surplus time is DROPPED,
    /// recorded in `ticks_dropped` - the sim runs slow through a
    /// stall instead of fast-forwarding through history).
    pub fn advance(&mut self, delta_ns: u64) -> u32 {
        self.acc_ns += u128::from(delta_ns);
        let due = (self.acc_ns * u128::from(self.rate_num)) / u128::from(self.rate_den);
        self.acc_ns -= due * u128::from(self.rate_den) / u128::from(self.rate_num);
        let executed = due.min(u128::from(self.max_catchup));
        self.ticks_executed += executed as u64;
        self.ticks_dropped += (due - executed) as u64;
        executed as u32
    }

    /// Whole pumps executed so far.
    pub fn ticks_executed(&self) -> u64 {
        self.ticks_executed
    }

    /// Whole pumps DISCARDED by the anti-spiral clamp so far (a
    /// non-zero count in a report means the host stalled).
    pub fn ticks_dropped(&self) -> u64 {
        self.ticks_dropped
    }

    /// Banked integer remainder nanoseconds carried into the next
    /// frame (always < one pump period).
    pub fn banked_ns(&self) -> u128 {
        self.acc_ns
    }

    /// The anti-spiral clamp in pumps per presented frame.
    pub fn max_catchup(&self) -> u32 {
        self.max_catchup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact 60 Hz banking against a 16_666_666 ns vsync cadence
    /// (0.99999996 tick per frame): the first frame banks, every
    /// later frame executes exactly one pump, 600 frames total 599.
    #[test]
    fn six_hz_cadence_banks_exactly() {
        let mut c = FixedStepClock::host();
        // 16_666_666 * 60 = 999_999_960 < 1e9: not due, banked whole.
        assert_eq!(c.advance(16_666_666), 0);
        assert_eq!(c.banked_ns(), 16_666_666);
        // 33_333_332 * 60 = 1_999_999_920: due 1, consumed
        // floor(1e9/60) = 16_666_666, bank 16_666_666 again.
        assert_eq!(c.advance(16_666_666), 1);
        assert_eq!(c.banked_ns(), 16_666_666);
        // Steady state: one pump per frame from here on.
        assert_eq!(c.advance(16_666_666), 1);
        assert_eq!(c.banked_ns(), 16_666_666);
        // Long run: 600 vsyncs = 599 pumps, none dropped, the same
        // one-frame bank carried at the end.
        let mut c = FixedStepClock::host();
        let mut pumps = 0u64;
        for _ in 0..600 {
            pumps += u64::from(c.advance(16_666_666));
        }
        assert_eq!(pumps, 599);
        assert_eq!(c.ticks_executed(), 599);
        assert_eq!(c.ticks_dropped(), 0);
        assert_eq!(c.banked_ns(), 16_666_666);
    }

    /// A 240 Hz display (4_166_666 ns vsync): four vsyncs per pump
    /// period, so 240 frames execute exactly 59 pumps and 181 frames
    /// are zero-tick presents (hand-derived: ticks at frames
    /// 5, 9, 13, ..., 237 - the bank sheds 2 ns per tick because the
    /// consumed floor is 16_666_666 while 4 vsyncs add 16_666_664).
    #[test]
    fn hz240_display_mostly_zero_tick_frames() {
        let mut c = FixedStepClock::host();
        let mut pumps = 0u64;
        let mut zero_frames = 0u32;
        for _ in 0..240 {
            let n = c.advance(4_166_666);
            if n == 0 {
                zero_frames += 1;
            }
            pumps += u64::from(n);
        }
        assert_eq!(pumps, 59);
        assert_eq!(c.ticks_dropped(), 0);
        assert_eq!(zero_frames, 181);
    }

    /// A stall (10 s in one delta) executes at most the clamp and
    /// DISCARDS the surplus: the next normal frames are not owed
    /// hundreds of catch-up pumps.
    #[test]
    fn stall_clamps_and_drops() {
        let mut c = FixedStepClock::host();
        assert_eq!(c.advance(10_000_000_000), DEFAULT_MAX_CATCHUP);
        assert_eq!(c.ticks_dropped(), 596);
        // The bank is the remainder of the FULL due (600 ticks over
        // exactly 10 s = 1e9/60 * 600 consumed to the ns): empty.
        assert_eq!(c.banked_ns(), 0);
        // The next normal vsyncs resume the ordinary cadence with no
        // debt: bank, then one pump per frame.
        assert_eq!(c.advance(16_666_666), 0);
        assert_eq!(c.advance(16_666_666), 1);
        assert_eq!(c.advance(16_666_666), 1);
        assert_eq!(c.ticks_dropped(), 596);
    }

    /// Zero deltas are idle (no pump, nothing banked) - a paused or
    /// throttled present loop cannot manufacture sim time.
    #[test]
    fn zero_delta_is_idle() {
        let mut c = FixedStepClock::host();
        assert_eq!(c.advance(0), 0);
        assert_eq!(c.advance(0), 0);
        assert_eq!(c.ticks_executed(), 0);
        assert_eq!(c.banked_ns(), 0);
    }

    /// The pump dt constant: 4 sub-ticks = one 60 Hz tick on the
    /// 240 Hz bedlam-core grid (the fixed dt is what keeps display
    /// timing out of the hashed state).
    #[test]
    fn pump_dt_is_one_tick() {
        assert_eq!(SUBTICKS_PER_PUMP, 4);
        assert_eq!(HOST_HZ * SUBTICKS_PER_PUMP, 240);
    }
}
