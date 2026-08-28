//! The hashed half of the D17 timing model: the fixed-60Hz simulation
//! bucket, its 300Hz microstep satellite scheduler, the state hash,
//! snapshot and restore.
//!
//! SKELETON ONLY (P3): no game logic lives here yet. The deliverable is
//! the determinism framework — fixed tick, exactly-one-entropy-draw-per-
//! tick, the 300Hz microstep scheduler of docs/DESIGN-RENDER.md sec 6,
//! FNV-1a state hash in a pinned field order, versioned snapshot bytes —
//! exercised by `tests/determinism.rs`. The per-host-frame (non-hashed)
//! half of D17 lives in `crate::frame`.

use crate::hash::{Fnv1a64, StateHash};
use crate::input::InputFrame;
use crate::mode::ModeConfig;
use crate::rng::Pcg32;
use crate::time::{Tick, TimeBase};
use crate::{CoreError, FORMAT_VERSION};

/// File magic for the snapshot format: `"BDLS"`.
pub const SNAPSHOT_MAGIC: [u8; 4] = *b"BDLS";

/// PCG stream used by the simulation itself. Distinct constants keep
/// future subsystems (spawn tables, audio shuffles, ...) on independent
/// streams from the same seed.
const STREAM_SIM: u64 = 1;

/// Microsteps per 60Hz sim tick: 5 (docs/DESIGN-RENDER.md sec 6, D17).
/// The sim runs a 300Hz service clock inside each tick —
/// 300 = lcm(60, 100, 50, 12.5) — and the three satellite events are
/// divisibility tests on ONE global microstep counter: %3 fires the 100Hz
/// service event, %6 the 50Hz fade step (while fading), %24 the 12.5Hz
/// palette bank cycle. All integer, all hashed.
///
/// Deliberately distinct from the HOST-side 240Hz sub-tick grid
/// (`crate::frame::SUBTICKS_PER_TICK`): that clock quantizes host dt
/// into whole 60Hz ticks and serves the display rates 60/120/240Hz
/// (D12); this 300Hz clock schedules the satellites INSIDE each tick at
/// the original service rates (100/50/12.5Hz). The two clocks never mix,
/// and neither is a float.
pub const MICROSTEPS_PER_TICK: u32 = 5;
/// 100Hz service event: every 3rd microstep of the 300Hz clock
/// (EXW TimerCallback@0044de58 analog, docs/RE-EXW-TICK.md,
/// docs/DESIGN-RENDER.md sec 6).
const SERVICE_DIVISOR: u64 = 3;
/// 50Hz palette-fade stepper FadeStep@00425901, active only while fading
/// (docs/RE-EXW-TICK.md, D15): every 6th microstep — every 2nd service
/// event, matching bit0 of the original 100Hz divider @004edbc8
/// [verified, docs/DESIGN-RENDER.md sec 6].
const FADE_DIVISOR: u64 = 6;
/// 12.5Hz palette bank cycle, operating range 0x90..0x97
/// (docs/RE-EXW-TICK.md): every 24th microstep — every 8th service
/// event, matching (ctr & 7) == 0 [verified].
const PAL_DIVISOR: u64 = 24;

/// Placeholder actor clamp in 640x480 game space (grid coords).
const ACTOR_MAX_X: i32 = 639;

/// Configuration a simulation is created from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimConfig {
    /// PRNG seed (see [`Pcg32::new`]).
    pub seed: u64,
    /// Ticks per second (D16: nominal 60 Hz for parity).
    pub time_base: TimeBase,
    /// The mode this simulation runs under (P6 seam, D200/D201):
    /// ONE immutable [`ModeConfig`] injected at sim construction and
    /// never mutated mid-run — a mode change is a new sim, i.e. a new
    /// `SimConfig`. Default = modern.
    pub mode: ModeConfig,
}

impl Default for SimConfig {
    /// Seed 0 is the documented neutral skeleton default; the parity
    /// harness pins real per-mission seeds later. Time base is
    /// [`TimeBase::NOMINAL`]; mode is [`ModeConfig::MODERN`] (the
    /// plan default).
    fn default() -> Self {
        SimConfig {
            seed: 0,
            time_base: TimeBase::NOMINAL,
            mode: ModeConfig::default(),
        }
    }
}

/// The deterministic simulation — the HASHED, fixed-60Hz bucket of D17 (a).
///
/// Everything here advances in whole ticks with integer math only; dt never
/// enters. All state is private and hashed in a pinned order (see
/// [`Sim::state_hash`]). A `Sim` never performs I/O, never reads the
/// clock, and contains no floats. Per-host-frame state (cursor, latches,
/// volume, cooldown displays) deliberately does NOT live here — see
/// `crate::frame::FrameState`.
#[derive(Debug, PartialEq, Eq)]
pub struct Sim {
    tick: Tick,
    rng: Pcg32,
    time_base: TimeBase,
    /// The immutable mode this sim was constructed under (P6 seam,
    /// D201). CONFIG, not state: deliberately not hashed and not
    /// serialized (like the seed); read it with [`Sim::mode`]. A mode
    /// change is a new sim, so there is no setter — ever.
    mode: ModeConfig,
    initial_state_hash: u64,
    /// Per-tick entropy slot: exactly one `u32` draw per tick, recorded in
    /// state so any divergence in RNG consumption order shifts the hash.
    last_draw: u32,

    // Satellite scheduler (D17 c, docs/DESIGN-RENDER.md sec 6): one
    // global 300Hz microstep counter, events are divisibility tests.
    /// Global 300Hz microstep counter: incremented
    /// [`MICROSTEPS_PER_TICK`] times per tick and tested with
    /// %`SERVICE_DIVISOR`/%`FADE_DIVISOR`/%`PAL_DIVISOR`. Zeroed at
    /// construction, mirroring the original counter being zeroed at
    /// boot release (FUN_0041e19d zeroes divider 004edbc8
    /// [verified, docs/DESIGN-RENDER.md sec 6]).
    microstep: u64,
    /// Total 100Hz service events elapsed.
    service_ticks: u64,
    /// Whether the 50Hz fade stepper is armed (EXW: fade countdown
    /// nonzero).
    fading: bool,
    /// Total 50Hz fade steps elapsed.
    fade_steps: u64,
    /// Total 12.5Hz palette cycles elapsed.
    pal_cycles: u64,

    // PLACEHOLDER scaffolding - replaced by real sim state in P4+:
    /// Placeholder actor grid X (only `buttons` bit 0 reaches it in this
    /// skeleton).
    actor_x: i32,
    /// Placeholder actor grid Y (no input path yet).
    actor_y: i32,
}

/// Serialized simulation state.
///
/// Layout (little-endian, fixed order):
/// magic `"BDLS"` @0, version u16 @4, flags u16 @6 (=0), tick_hz u32 @8,
/// tick u64 @12, stored_hash u64 @20, state_len u32 @28, then `state_len`
/// state bytes. The state bytes are the canonical fields below, and
/// `stored_hash` is [`Sim::state_hash`] recomputed over the tick plus those
/// fields (see `Sim::snapshot`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Format version the bytes were written with.
    pub version: u16,
    /// Ticks per second the snapshot was taken at.
    pub tick_hz: u32,
    /// Tick index the snapshot was taken at.
    pub tick: Tick,
    /// [`Sim::state_hash`] at snapshot time; verified on restore.
    pub stored_hash: u64,
    /// Canonical state bytes (private; serialization is via `to_bytes`).
    state_bytes: Vec<u8>,
}

/// Header size of [`Snapshot::to_bytes`].
const SNAPSHOT_HEADER_LEN: usize = 4 + 2 + 2 + 4 + 8 + 8 + 4;

/// Version-1 state-region layout (little-endian, fixed order):
/// rng_state u64 @0, rng_stream u64 @8, last_draw u32 @16, microstep u64
/// @20, service_ticks u64 @28, fading u8 @36 (+3 pad bytes @37..40),
/// fade_steps u64 @40, pal_cycles u64 @48, actor_x i32 @56, actor_y i32
/// @60, initial_state_hash u64 @64.
const STATE_LEN: usize = 72;

impl Sim {
    /// Create a simulation: PRNG on [`STREAM_SIM`], the microstep
    /// counter at 0 (mirroring the boot-release counter zeroing,
    /// FUN_0041e19d), satellite event totals and placeholder payload at
    /// their zeroed defaults, `fading` off, and `initial_state_hash`
    /// recorded from the tick-0 state hash.
    pub fn new(config: &SimConfig) -> Sim {
        let mut sim = Sim {
            tick: 0,
            rng: Pcg32::new(config.seed, STREAM_SIM),
            time_base: config.time_base,
            mode: config.mode,
            initial_state_hash: 0,
            last_draw: 0,
            microstep: 0,
            service_ticks: 0,
            fading: false,
            fade_steps: 0,
            pal_cycles: 0,
            actor_x: 0,
            actor_y: 0,
        };
        sim.initial_state_hash = sim.state_hash().0;
        sim
    }

    /// Advance exactly one fixed timestep (1/60 s at the nominal base).
    ///
    /// Order (each step is part of the determinism contract):
    /// 1. exactly ONE PRNG draw per tick (the entropy slot), stored in
    ///    `last_draw`;
    /// 2. the 300Hz microstep scheduler runs
    ///    [`MICROSTEPS_PER_TICK`] microsteps of one shared counter
    ///    (docs/DESIGN-RENDER.md sec 6, D17 c); within EACH microstep the
    ///    event tests are evaluated in the FIXED order service -> fade ->
    ///    palette: %3 fires the 100Hz service event (always), %6 the 50Hz
    ///    fade step (only while `fading`), %24 the 12.5Hz palette bank
    ///    cycle (always);
    /// 3. placeholder payload: `buttons` bit 0 held advances `actor_x`
    ///    by 1, clamped at 639 — nothing else reaches the sim in this
    ///    skeleton (mouse deltas are per-frame data, `crate::frame`);
    /// 4. the tick counter increments last.
    pub fn tick(&mut self, input: &InputFrame) {
        self.last_draw = self.rng.next_u32();

        // 300Hz microstep scheduler: 5 microsteps per tick, one global
        // counter, fixed service -> fade -> palette test order (the
        // %3/%6/%24 tests below are written is_multiple_of: identical
        // integer semantics, clippy-clean).
        for _ in 0..MICROSTEPS_PER_TICK {
            self.microstep += 1;
            // 100Hz service event.
            if self.microstep.is_multiple_of(SERVICE_DIVISOR) {
                self.service_ticks += 1;
            }
            // 50Hz fade stepper, armed while `fading`.
            if self.fading && self.microstep.is_multiple_of(FADE_DIVISOR) {
                self.fade_steps += 1;
            }
            // 12.5Hz palette bank cycle.
            if self.microstep.is_multiple_of(PAL_DIVISOR) {
                self.pal_cycles += 1;
            }
        }

        // PLACEHOLDER payload update.
        if input.buttons & 1 != 0 {
            self.actor_x = (self.actor_x + 1).min(ACTOR_MAX_X);
        }

        self.tick += 1;
    }

    /// Current tick index (ticks elapsed since creation).
    pub fn tick_index(&self) -> Tick {
        self.tick
    }

    /// The time base this sim runs at.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// The immutable [`ModeConfig`] this sim was constructed under
    /// (P6 seam, D201). There is no mode setter: a mode change is a
    /// new sim constructed from a new [`SimConfig`].
    pub fn mode(&self) -> ModeConfig {
        self.mode
    }

    /// The most recent per-tick entropy draw (the entropy slot value).
    pub fn last_draw(&self) -> u32 {
        self.last_draw
    }

    /// `state_hash()` at tick 0, recorded at creation and carried across
    /// snapshots/replays (this is the `initial_state_hash` replay field).
    pub fn initial_state_hash(&self) -> u64 {
        self.initial_state_hash
    }

    /// The global 300Hz microstep counter (5 per tick, zeroed at boot
    /// release per DESIGN-RENDER sec 6; satellite events are the %3/%6/%24
    /// divisibility tests on it).
    pub fn microstep(&self) -> u64 {
        self.microstep
    }

    /// 100Hz service satellite: total events elapsed (fires on every
    /// 3rd microstep).
    pub fn service_ticks(&self) -> u64 {
        self.service_ticks
    }

    /// Whether the 50Hz fade stepper is armed.
    pub fn fading(&self) -> bool {
        self.fading
    }

    /// Arm or disarm the 50Hz fade stepper. Sim-side control for the
    /// skeleton (the real arming is EXW FadeSetup/FadeCancel, P4+);
    /// disarming mid-fade freezes `fade_steps` where it stands — it
    /// stays hashed either way.
    pub fn set_fading(&mut self, on: bool) {
        self.fading = on;
    }

    /// 50Hz fade satellite: total steps elapsed (fires on every 6th
    /// microstep while `fading`).
    pub fn fade_steps(&self) -> u64 {
        self.fade_steps
    }

    /// 12.5Hz palette-cycle satellite: total cycles elapsed (fires on
    /// every 24th microstep).
    pub fn pal_cycles(&self) -> u64 {
        self.pal_cycles
    }

    /// Placeholder actor grid coords `(x, y)` in 640x480 game space.
    pub fn actor(&self) -> (i32, i32) {
        (self.actor_x, self.actor_y)
    }

    /// FNV-1a 64 over the canonical serialization of ALL hashed state
    /// fields, in this FIXED order: `tick` u64, rng state u64, rng stream
    /// u64, `last_draw` u32, `microstep` u64, `service_ticks` u64,
    /// `fading` as u8, `fade_steps` u64, `pal_cycles` u64, `actor_x` i32,
    /// `actor_y` i32.
    ///
    /// The order is a stability contract; appending new fields (P4+) goes
    /// at the END of this list with a `FORMAT_VERSION` bump.
    /// `FrameState` (crate::frame) is NEVER hashed — D17 excludes
    /// frame-rate-driven systems from this value.
    pub fn state_hash(&self) -> StateHash {
        let mut h = Fnv1a64::new();
        h.write_u64(self.tick);
        h.write_u64(self.rng.state());
        h.write_u64(self.rng.stream());
        h.write_u32(self.last_draw);
        h.write_u64(self.microstep);
        h.write_u64(self.service_ticks);
        h.write_u8(u8::from(self.fading));
        h.write_u64(self.fade_steps);
        h.write_u64(self.pal_cycles);
        h.write_i32(self.actor_x);
        h.write_i32(self.actor_y);
        StateHash(h.finish())
    }

    /// Capture a snapshot of the hashed sim bucket. The non-hashed
    /// `FrameState` is deliberately NOT serialized — hosts rebuild it
    /// from their own UI state.
    pub fn snapshot(&self) -> Snapshot {
        let mut state_bytes = Vec::with_capacity(STATE_LEN);
        state_bytes.extend_from_slice(&self.rng.state().to_le_bytes());
        state_bytes.extend_from_slice(&self.rng.stream().to_le_bytes());
        state_bytes.extend_from_slice(&self.last_draw.to_le_bytes());
        state_bytes.extend_from_slice(&self.microstep.to_le_bytes());
        state_bytes.extend_from_slice(&self.service_ticks.to_le_bytes());
        state_bytes.push(u8::from(self.fading));
        state_bytes.extend_from_slice(&[0u8; 3]); // pad @37..40
        state_bytes.extend_from_slice(&self.fade_steps.to_le_bytes());
        state_bytes.extend_from_slice(&self.pal_cycles.to_le_bytes());
        state_bytes.extend_from_slice(&self.actor_x.to_le_bytes());
        state_bytes.extend_from_slice(&self.actor_y.to_le_bytes());
        state_bytes.extend_from_slice(&self.initial_state_hash.to_le_bytes());
        debug_assert_eq!(state_bytes.len(), STATE_LEN);
        Snapshot {
            version: FORMAT_VERSION,
            tick_hz: self.time_base.tick_hz,
            tick: self.tick,
            stored_hash: self.state_hash().0,
            state_bytes,
        }
    }

    /// Validate snapshot bytes and rebuild the `Sim` positioned to continue
    /// from the snapshot's tick. Never panics on user bytes.
    ///
    /// Checks, in order: length >= header, magic, version, declared state
    /// length (truncation), then the restored sim's recomputed
    /// [`Sim::state_hash`] must equal the stored hash
    /// ([`CoreError::HashMismatch`] otherwise), and the snapshot's time
    /// base must equal `expected_config.time_base`
    /// ([`CoreError::BadTickHz`] otherwise).
    ///
    /// The seed is deliberately NOT re-checked: continuity is carried by the
    /// live PRNG state inside the snapshot.
    pub fn restore(bytes: &[u8], expected_config: &SimConfig) -> Result<Sim, CoreError> {
        let have = bytes.len();
        if have < SNAPSHOT_HEADER_LEN {
            return Err(CoreError::Truncated {
                needed: SNAPSHOT_HEADER_LEN,
                have,
            });
        }
        if bytes[0..4] != SNAPSHOT_MAGIC {
            return Err(CoreError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FORMAT_VERSION {
            return Err(CoreError::UnsupportedVersion(version));
        }
        // bytes[6..8]: flags, reserved — value ignored.
        let tick_hz = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let tick = u64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19],
        ]);
        let stored_hash = u64::from_le_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23], bytes[24], bytes[25], bytes[26], bytes[27],
        ]);
        let state_len = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;

        let total = SNAPSHOT_HEADER_LEN + state_len;
        if have < total {
            return Err(CoreError::Truncated {
                needed: total,
                have,
            });
        }
        if state_len != STATE_LEN {
            // Version-1 snapshots have a fixed-size state region.
            return Err(CoreError::Truncated {
                needed: SNAPSHOT_HEADER_LEN + STATE_LEN,
                have,
            });
        }
        let s = &bytes[SNAPSHOT_HEADER_LEN..total];

        let rng_state = u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);
        let rng_stream = u64::from_le_bytes([s[8], s[9], s[10], s[11], s[12], s[13], s[14], s[15]]);
        let last_draw = u32::from_le_bytes([s[16], s[17], s[18], s[19]]);
        let microstep =
            u64::from_le_bytes([s[20], s[21], s[22], s[23], s[24], s[25], s[26], s[27]]);
        let service_ticks =
            u64::from_le_bytes([s[28], s[29], s[30], s[31], s[32], s[33], s[34], s[35]]);
        let fading = s[36] != 0;
        // s[37..40]: pad, ignored.
        let fade_steps =
            u64::from_le_bytes([s[40], s[41], s[42], s[43], s[44], s[45], s[46], s[47]]);
        let pal_cycles =
            u64::from_le_bytes([s[48], s[49], s[50], s[51], s[52], s[53], s[54], s[55]]);
        let actor_x = i32::from_le_bytes([s[56], s[57], s[58], s[59]]);
        let actor_y = i32::from_le_bytes([s[60], s[61], s[62], s[63]]);
        let initial_state_hash =
            u64::from_le_bytes([s[64], s[65], s[66], s[67], s[68], s[69], s[70], s[71]]);

        let sim = Sim {
            tick,
            rng: Pcg32::from_raw_parts(rng_state, rng_stream),
            time_base: TimeBase { tick_hz },
            // The mode is config, not state: it is not in the
            // snapshot bytes, so a restore ADOPTS the mode of the
            // SimConfig it is restored under (restoring is
            // constructing a new sim — D201).
            mode: expected_config.mode,
            initial_state_hash,
            last_draw,
            microstep,
            service_ticks,
            fading,
            fade_steps,
            pal_cycles,
            actor_x,
            actor_y,
        };
        // Re-hash the RESTORED object itself: the canonical form is what
        // must match the stored hash, so the rebuilt sim is always
        // self-consistent (e.g. a crafted `fading` byte of 2 cannot pass).
        if sim.state_hash().0 != stored_hash {
            return Err(CoreError::HashMismatch {
                stored: stored_hash,
                computed: sim.state_hash().0,
            });
        }

        if tick_hz != expected_config.time_base.tick_hz {
            return Err(CoreError::BadTickHz(tick_hz));
        }

        Ok(sim)
    }
}

impl Snapshot {
    /// Serialize (see the [`Snapshot`] layout docs).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(SNAPSHOT_HEADER_LEN + self.state_bytes.len());
        out.extend_from_slice(&SNAPSHOT_MAGIC);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // flags (reserved, 0)
        out.extend_from_slice(&self.tick_hz.to_le_bytes());
        out.extend_from_slice(&self.tick.to_le_bytes());
        out.extend_from_slice(&self.stored_hash.to_le_bytes());
        out.extend_from_slice(&(self.state_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.state_bytes);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_records_initial_hash_at_tick_zero() {
        let sim = Sim::new(&SimConfig::default());
        assert_eq!(sim.tick_index(), 0);
        assert_eq!(sim.initial_state_hash(), sim.state_hash().0);
        assert_eq!(sim.time_base(), TimeBase::NOMINAL);
    }

    #[test]
    fn satellite_counters_start_zeroed_and_fading_off() {
        let sim = Sim::new(&SimConfig::default());
        assert_eq!(sim.microstep(), 0);
        assert_eq!(sim.service_ticks(), 0);
        assert!(!sim.fading());
        assert_eq!(sim.fade_steps(), 0);
        assert_eq!(sim.pal_cycles(), 0);
    }

    #[test]
    fn actor_advances_on_primary_button_and_clamps() {
        let mut sim = Sim::new(&SimConfig::default());
        let press = InputFrame {
            buttons: 1,
            ..InputFrame::default()
        };
        for _ in 0..700 {
            sim.tick(&press);
        }
        assert_eq!(sim.actor(), (639, 0));
    }

    #[test]
    fn primary_button_bit0_is_the_only_placeholder_input_path() {
        let config = SimConfig::default();
        let press = InputFrame {
            buttons: 1,
            ..InputFrame::default()
        };
        // Held vs released on tick 2: identical RNG/satellite trajectory,
        // the actor is the only difference => different state hashes.
        let mut held = Sim::new(&config);
        held.tick(&press);
        held.tick(&press);
        let mut released = Sim::new(&config);
        released.tick(&press);
        released.tick(&InputFrame::default());
        assert_ne!(held.state_hash(), released.state_hash());

        // Buttons OUTSIDE bit 0 — and all mouse deltas — have no
        // placeholder effect on the sim: identical trajectory.
        let mut other_bits = Sim::new(&config);
        other_bits.tick(&InputFrame {
            buttons: 0b1110,
            mouse_dx: 100,
            mouse_dy: -100,
            ..InputFrame::default()
        });
        let mut quiet = Sim::new(&config);
        quiet.tick(&InputFrame::default());
        assert_eq!(other_bits.state_hash(), quiet.state_hash());
    }

    #[test]
    fn one_draw_per_tick() {
        // Tick with identical inputs: only the PRNG state and satellites
        // advance, and the PRNG advances by exactly one draw per tick
        // (hash changes every tick).
        let mut sim = Sim::new(&SimConfig::default());
        let input = InputFrame::default();
        let mut prev = sim.state_hash();
        for _ in 0..50 {
            sim.tick(&input);
            assert_ne!(sim.state_hash(), prev);
            prev = sim.state_hash();
        }
    }

    #[test]
    fn snapshot_restore_round_trip() {
        let config = SimConfig {
            seed: 77,
            time_base: TimeBase::NOMINAL,
            mode: ModeConfig::default(),
        };
        let mut sim = Sim::new(&config);
        sim.set_fading(true);
        let input = InputFrame {
            buttons: 1,
            mouse_dx: 5,
            mouse_dy: -9,
            mouse_buttons: 2,
        };
        for _ in 0..100 {
            sim.tick(&input);
        }
        let snapshot = sim.snapshot();
        let bytes = snapshot.to_bytes();
        let mut restored = Sim::restore(&bytes, &config).unwrap();
        assert_eq!(restored.tick_index(), sim.tick_index());
        assert_eq!(restored.state_hash(), sim.state_hash());
        assert_eq!(restored.fading(), sim.fading());

        // Continuing both stays identical.
        for i in 0..10 {
            let input = InputFrame {
                buttons: (i % 2) * 3,
                mouse_dx: i as i16,
                mouse_dy: -(i as i16),
                mouse_buttons: 1,
            };
            sim.tick(&input);
            restored.tick(&input);
            assert_eq!(sim.state_hash(), restored.state_hash());
        }
    }

    #[test]
    fn restore_rejects_garbage_without_panicking() {
        let config = SimConfig::default();
        let mut sim = Sim::new(&config);
        sim.tick(&InputFrame::default());
        let bytes = sim.snapshot().to_bytes();

        // Truncations.
        for i in 0..bytes.len() {
            assert!(Sim::restore(&bytes[..i], &config).is_err(), "len {i}");
        }
        // Wrong magic.
        let mut bad = bytes.clone();
        bad[0] ^= 0xFF;
        assert_eq!(Sim::restore(&bad, &config), Err(CoreError::BadMagic));
        // Wrong version.
        let mut bad = bytes.clone();
        bad[4] = 9;
        assert_eq!(
            Sim::restore(&bad, &config),
            Err(CoreError::UnsupportedVersion(9))
        );
        // Tampered state field (rng_state lives at state offset 0 => byte 32).
        let mut bad = bytes.clone();
        bad[32] ^= 0x01;
        assert!(matches!(
            Sim::restore(&bad, &config),
            Err(CoreError::HashMismatch { .. })
        ));
        // Wrong expected time base.
        let other = SimConfig {
            seed: config.seed,
            time_base: TimeBase { tick_hz: 120 },
            mode: ModeConfig::default(),
        };
        assert_eq!(Sim::restore(&bytes, &other), Err(CoreError::BadTickHz(60)));
    }

    #[test]
    fn mode_is_injected_at_construction_and_never_mutated() {
        // P6 seam (D201): the ONE immutable ModeConfig rides SimConfig
        // into Sim::new; the sim exposes it and offers no way to
        // change it (a mode change is a NEW sim). Both arms of both
        // plan-named axes construct cleanly.
        for config in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let sim = Sim::new(&SimConfig {
                seed: 7,
                time_base: TimeBase::NOMINAL,
                mode: config,
            });
            assert_eq!(sim.mode(), config);
        }
        // Default SimConfig = modern (the plan default).
        assert_eq!(Sim::new(&SimConfig::default()).mode(), ModeConfig::MODERN);
    }

    #[test]
    fn mode_is_config_not_state_the_seam_lands_inert() {
        // The seam itself changes NO behavior: neither plan-named axis
        // has an in-sim consumer yet (timing lock is a host pacing
        // policy, control scheme is a host input mapping), so the
        // same seed + input stream produces the IDENTICAL hashed
        // trajectory in both arms — which is exactly why the
        // canonical S0-S8 chains cannot move under the modern default.
        // The mode is also not part of the state hash (config, not
        // state — like the seed). A later unit that gives an axis or a
        // catalog toggle an in-sim consumer is the unit that makes the
        // arms diverge THERE; this pin documents the seam unit alone.
        let input = InputFrame {
            buttons: 1,
            mouse_dx: 3,
            mouse_dy: -2,
            mouse_buttons: 1,
        };
        let mut modern = Sim::new(&SimConfig {
            seed: 0x0BEE_F00D,
            time_base: TimeBase::NOMINAL,
            mode: ModeConfig::MODERN,
        });
        let mut classic = Sim::new(&SimConfig {
            seed: 0x0BEE_F00D,
            time_base: TimeBase::NOMINAL,
            mode: ModeConfig::CLASSIC,
        });
        assert_eq!(
            modern.initial_state_hash(),
            classic.initial_state_hash(),
            "tick-0 hash is mode-independent"
        );
        for _ in 0..64 {
            modern.tick(&input);
            classic.tick(&input);
            assert_eq!(modern.state_hash(), classic.state_hash());
        }
    }

    #[test]
    fn restore_adopts_the_expected_config_mode() {
        // The mode is not serialized (config, not state — the snapshot
        // format is byte-stable), so a restored sim takes the mode of
        // the SimConfig it is restored under: restoring IS
        // constructing a new sim (D201). The stored hash still binds
        // the trajectory bytes, not the mode.
        let config = SimConfig {
            seed: 99,
            time_base: TimeBase::NOMINAL,
            mode: ModeConfig::MODERN,
        };
        let mut sim = Sim::new(&config);
        for _ in 0..10 {
            sim.tick(&InputFrame::default());
        }
        let bytes = sim.snapshot().to_bytes();

        let same_mode = Sim::restore(&bytes, &config).unwrap();
        assert_eq!(same_mode.mode(), ModeConfig::MODERN);
        assert_eq!(same_mode.state_hash(), sim.state_hash());

        let classic_config = SimConfig {
            seed: config.seed,
            time_base: config.time_base,
            mode: ModeConfig::CLASSIC,
        };
        let as_classic = Sim::restore(&bytes, &classic_config).unwrap();
        assert_eq!(as_classic.mode(), ModeConfig::CLASSIC);
        assert_eq!(
            as_classic.state_hash(),
            sim.state_hash(),
            "same bytes, same trajectory: the mode is not hashed"
        );
    }
}
