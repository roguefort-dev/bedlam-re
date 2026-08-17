//! Per-host-frame state and the accumulator driver — the D17 "bucket (b)"
//! half of the timing model (docs/DECISIONS.md D17).
//!
//! Everything in this module runs at HOST refresh rate with real dt
//! semantics: input polling, UI hit-tests, cooldowns, cursor, audio/video
//! knobs. That mirrors the original architecture (per-frame poll in the
//! present-paced loop, D16, plus the 100Hz service satellites,
//! docs/RE-EXW-TICK.md). None of it is hashed: the deterministic 60Hz
//! bucket lives in [`crate::sim`], and [`SimDriver`] is the only glue —
//! it quantizes host dt to whole sub-ticks and feeds whole 60Hz ticks to
//! the sim, so the sim never sees dt at all.

use crate::input::InputFrame;
use crate::sim::{Sim, SimConfig};

/// Sub-ticks per 60Hz sim tick: the 240Hz quantization grid (D12
/// high-refresh ceiling).
///
/// 240Hz host = exactly 1 sub-tick per frame, 60Hz = 4, 15Hz = 16. Hosts
/// quantize their dt to whole sub-ticks on this grid; the accumulator banks
/// the remainder so the long-run tick rate stays exact (no drift, no
/// rounding). dt NEVER enters sim math — it only counts whole 60Hz ticks.
///
/// Deliberately distinct from the SIM-side 300Hz microstep clock
/// (`crate::sim::MICROSTEPS_PER_TICK`, docs/DESIGN-RENDER.md sec 6):
/// this 240Hz grid is a HOST-display-oriented clock that quantizes host
/// dt into whole 60Hz ticks (it serves the display rates 60/120/240Hz),
/// while the 300Hz microstep clock schedules the satellites INSIDE each
/// already-quantized tick at the original service rates
/// (100/50/12.5Hz). The two clocks never mix, and neither is a float.
pub const SUBTICKS_PER_TICK: u32 = 4;

/// Game-space extents the cursor clamps into (640x480 canonical render
/// space). Clamp, not modulo — mirrors EXW scroll-clamp style; exact EXW
/// addresses TBD pending P2e input RE.
const CURSOR_MAX_X: i32 = 639;
const CURSOR_MAX_Y: i32 = 479;

/// Placeholder volume ceiling (doc nod: EXW music volume 0..100).
const VOLUME_MAX: i32 = 100;

/// Per-host-frame state — the NON-hashed bucket (D17 b).
///
/// These systems are frame-rate-driven: they run once per host frame with
/// the frame's dt (quantized to sub-ticks), which is exactly why they are
/// excluded from the sim state hash — same inputs at 15/60/240Hz host must
/// produce identical SIM hashes even though the cursor/latch trajectories
/// differ (see `frame_state_excluded_from_hash` in `tests/determinism.rs`).
///
/// PLACEHOLDER scaffolding throughout: the real input map, UI hit-test
/// state and cooldown wiring land with P2e input RE / P4+ game logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameState {
    /// Pointer X, integrated from `mouse_dx` each frame, clamped
    /// 0..=639.
    pub cursor_x: i32,
    /// Pointer Y, integrated from `mouse_dy` each frame, clamped
    /// 0..=479.
    pub cursor_y: i32,
    /// Edge latch on `buttons` bit 0: 1 exactly on the host frame where
    /// the button transitions released->pressed, 0 otherwise (press-edge
    /// pulse; true multi-event latching lands with P2e input RE).
    pub latch_primary: u32,
    /// Audio volume 0..=100 (EXW Up/Down volume hotkeys pending P2e input
    /// RE; there is no input path yet, the per-frame clamp below just
    /// keeps direct field writes in range).
    pub volume: i32,
    /// Cooldown display counter (D17: cooldowns are per-frame systems),
    /// decremented in whole sim ticks and saturating at 0. PLACEHOLDER:
    /// real cooldown displays land with P4+ game logic.
    pub cooldown_display: i32,
    /// Previous frame's `buttons` bit 0, for edge detection (private
    /// bookkeeping; not part of the public field set).
    prev_primary: bool,
}

impl FrameState {
    /// Zeroed state (`volume` starts 0 — hosts restore it from saved
    /// settings once the EXW volume wiring is RE'd).
    pub fn new() -> FrameState {
        FrameState {
            cursor_x: 0,
            cursor_y: 0,
            latch_primary: 0,
            volume: 0,
            cooldown_display: 0,
            prev_primary: false,
        }
    }

    /// Run one host frame of the frame-rate systems.
    ///
    /// `dt_subticks` is this frame's elapsed time quantized to whole
    /// sub-ticks (see [`SUBTICKS_PER_TICK`]). Behavior:
    /// - cursor integrates this frame's mouse deltas, then clamps into
    ///   640x480 game space;
    /// - `latch_primary` pulses 1 on the press edge of `buttons` bit 0
    ///   (rising edge only — held frames read 0);
    /// - `volume` is clamped back into 0..=100 (no input path yet);
    /// - `cooldown_display` decrements by `dt_subticks / 4` — whole 60Hz
    ///   ticks only, saturating at 0. PLACEHOLDER: simple and documented;
    ///   real cooldown displays replace it in P4+.
    pub fn advance_frame(&mut self, dt_subticks: u32, input: &InputFrame) {
        self.cursor_x = (self.cursor_x + i32::from(input.mouse_dx)).clamp(0, CURSOR_MAX_X);
        self.cursor_y = (self.cursor_y + i32::from(input.mouse_dy)).clamp(0, CURSOR_MAX_Y);
        let primary = input.buttons & 1 != 0;
        self.latch_primary = u32::from(primary && !self.prev_primary);
        self.prev_primary = primary;
        self.volume = self.volume.clamp(0, VOLUME_MAX);
        // Whole-tick decrement with a hard floor at 0: `saturating_sub`
        // alone would still run negative on i32 (it only pins at MIN).
        let whole_ticks = (dt_subticks / SUBTICKS_PER_TICK) as i32;
        self.cooldown_display = self.cooldown_display.saturating_sub(whole_ticks).max(0);
    }
}

/// Host-rate driver: the hashed fixed-step [`Sim`], the non-hashed
/// [`FrameState`], and the sub-tick accumulator that converts host frames
/// into whole 60Hz ticks (D17 a+b).
///
/// The accumulator is frame-rate-driven bookkeeping: it lives HERE, never
/// in `Sim`, and is therefore NEVER hashed and never snapshotted. Only the
/// sequence of (input, tick) pairs reaches the sim, which is what makes
/// the same input script produce the identical sim hash at 15/60/240Hz
/// host.
#[derive(Debug)]
pub struct SimDriver {
    sim: Sim,
    frame: FrameState,
    accumulator: u32,
    pending_input: InputFrame,
}

impl SimDriver {
    /// Create a driver around a fresh [`Sim`] built from `config`, with a
    /// zeroed frame state, empty accumulator and neutral pending input.
    pub fn new(config: &SimConfig) -> SimDriver {
        SimDriver {
            sim: Sim::new(config),
            frame: FrameState::new(),
            accumulator: 0,
            pending_input: InputFrame::default(),
        }
    }

    /// The hashed 60Hz simulation bucket.
    pub fn sim(&self) -> &Sim {
        &self.sim
    }

    /// Mutable sim access for explicit engine operations (snapshot
    /// capture, `set_fading`). Prefer routing gameplay through
    /// [`SimDriver::advance`] so the input log stays the whole truth.
    pub fn sim_mut(&mut self) -> &mut Sim {
        &mut self.sim
    }

    /// The non-hashed per-frame state.
    pub fn frame(&self) -> &FrameState {
        &self.frame
    }

    /// Mutable frame state for the host-side systems (UI hit-tests,
    /// volume writes, ...). Changes here can never touch the sim hash.
    pub fn frame_mut(&mut self) -> &mut FrameState {
        &mut self.frame
    }

    /// Advance one host frame. Returns the number of sim ticks executed.
    ///
    /// Order (D17):
    /// 1. the latest host sample becomes the pending input — per-frame
    ///    polling feeds the next fixed step to execute (possibly still
    ///    within this very call);
    /// 2. the frame-rate systems run with this frame's dt;
    /// 3. accumulated sub-ticks pay for whole 60Hz ticks; any remainder
    ///    is banked for the next frame (see [`SUBTICKS_PER_TICK`]). If a
    ///    host frame covers several ticks, they all consume the same
    ///    pending input.
    pub fn advance(&mut self, dt_subticks: u32, input: &InputFrame) -> u32 {
        self.pending_input = *input;
        self.frame.advance_frame(dt_subticks, input);
        self.accumulator = self.accumulator.saturating_add(dt_subticks);
        let mut executed = 0;
        while self.accumulator >= SUBTICKS_PER_TICK {
            self.sim.tick(&self.pending_input);
            self.accumulator -= SUBTICKS_PER_TICK;
            executed += 1;
        }
        executed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zeroed() {
        let f = FrameState::new();
        assert_eq!(f, FrameState::default());
        assert_eq!((f.cursor_x, f.cursor_y), (0, 0));
        assert_eq!(f.latch_primary, 0);
        assert_eq!(f.volume, 0);
        assert_eq!(f.cooldown_display, 0);
    }

    #[test]
    fn cursor_integrates_then_clamps() {
        let mut f = FrameState::new();
        let input = InputFrame {
            mouse_dx: 100,
            mouse_dy: -100,
            ..InputFrame::default()
        };
        for _ in 0..10 {
            f.advance_frame(4, &input);
        }
        // 10 x (+100, -100) = (1000, -1000) clamps into 640x480.
        assert_eq!((f.cursor_x, f.cursor_y), (639, 0));
    }

    #[test]
    fn latch_pulses_on_press_edge_only() {
        let mut f = FrameState::new();
        let press = InputFrame {
            buttons: 1,
            ..InputFrame::default()
        };
        let release = InputFrame::default();
        f.advance_frame(4, &release);
        assert_eq!(f.latch_primary, 0, "idle");
        f.advance_frame(4, &press);
        assert_eq!(f.latch_primary, 1, "press edge");
        f.advance_frame(4, &press);
        assert_eq!(f.latch_primary, 0, "held: no new edge");
        f.advance_frame(4, &release);
        assert_eq!(f.latch_primary, 0, "released");
        f.advance_frame(4, &press);
        assert_eq!(f.latch_primary, 1, "re-press edges again");
    }

    #[test]
    fn volume_clamps_into_0_100() {
        let mut f = FrameState::new();
        f.volume = 500;
        f.advance_frame(0, &InputFrame::default());
        assert_eq!(f.volume, 100);
        f.volume = -3;
        f.advance_frame(0, &InputFrame::default());
        assert_eq!(f.volume, 0);
    }

    #[test]
    fn cooldown_decrements_in_whole_ticks_and_saturates() {
        let mut f = FrameState::new();
        f.cooldown_display = 10;
        f.advance_frame(3, &InputFrame::default()); // 3/4 tick: no decrement
        assert_eq!(f.cooldown_display, 10);
        f.advance_frame(4, &InputFrame::default()); // exactly 1 tick
        assert_eq!(f.cooldown_display, 9);
        f.advance_frame(8, &InputFrame::default()); // 2 ticks in one frame
        assert_eq!(f.cooldown_display, 7);
        f.cooldown_display = 1;
        f.advance_frame(16, &InputFrame::default()); // 4 ticks
        assert_eq!(f.cooldown_display, 0, "saturates, never negative");
    }

    #[test]
    fn advance_runs_frame_systems_and_sim_ticks() {
        let mut driver = SimDriver::new(&SimConfig::default());
        let input = InputFrame {
            mouse_dx: 10,
            ..InputFrame::default()
        };
        assert_eq!(driver.advance(4, &input), 1);
        assert_eq!(driver.sim().tick_index(), 1);
        assert_eq!(driver.frame().cursor_x, 10);
        assert_eq!(driver.advance(3, &input), 0);
        assert_eq!(driver.sim().tick_index(), 1, "3 sub-ticks bank, no tick");
    }
}
