//! The hermetic simulation: deterministic tick, state hash, snapshot and
//! restore.
//!
//! SKELETON ONLY (P3): no game logic lives here yet. The deliverable is the
//! determinism framework — fixed tick, exactly-one-entropy-draw-per-tick,
//! FNV-1a state hash in a pinned field order, versioned snapshot bytes —
//! exercised by `tests/determinism.rs`.

use crate::hash::{Fnv1a64, StateHash};
use crate::input::InputFrame;
use crate::rng::Pcg32;
use crate::time::{Tick, TimeBase};
use crate::{CoreError, FORMAT_VERSION};

/// File magic for the snapshot format: `"BDLS"`.
pub const SNAPSHOT_MAGIC: [u8; 4] = *b"BDLS";

/// PCG stream used by the simulation itself. Distinct constants keep
/// future subsystems (spawn tables, audio shuffles, ...) on independent
/// streams from the same seed.
const STREAM_SIM: u64 = 1;

/// Game-space extents the placeholder cursor clamps into (640x480 canonical
/// render space).
const CURSOR_MAX_X: i32 = 639;
const CURSOR_MAX_Y: i32 = 479;

/// Configuration a simulation is created from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimConfig {
    /// PRNG seed (see [`Pcg32::new`]).
    pub seed: u64,
    /// Ticks per second (D16: nominal 60 Hz for parity).
    pub time_base: TimeBase,
}

impl Default for SimConfig {
    /// Seed 0 is the documented neutral skeleton default; the parity
    /// harness pins real per-mission seeds later. Time base is
    /// [`TimeBase::NOMINAL`].
    fn default() -> Self {
        SimConfig {
            seed: 0,
            time_base: TimeBase::NOMINAL,
        }
    }
}

/// The deterministic simulation.
///
/// All state is private and hashed in a pinned order (see [`Sim::state_hash`]).
/// A `Sim` never performs I/O, never reads the clock, and contains no floats.
#[derive(Debug, PartialEq, Eq)]
pub struct Sim {
    tick: Tick,
    rng: Pcg32,
    time_base: TimeBase,
    initial_state_hash: u64,
    /// Per-tick entropy slot: exactly one `u32` draw per tick, recorded in
    /// state so any divergence in RNG consumption order shifts the hash.
    last_draw: u32,

    // PLACEHOLDER scaffolding - replaced by real sim state in P4+:
    /// Placeholder pointer position, clamped to 640x480 game space (clamp,
    /// not modulo — mirrors EXW scroll-clamp style; exact EXW addresses TBD
    /// pending P2e input RE).
    cursor_x: i32,
    /// Placeholder pointer Y (see `cursor_x`).
    cursor_y: i32,
    /// Placeholder edge latch on `buttons` bit 0 (press sets 1, release
    /// clears; true multi-tick latching lands with P2e input RE).
    latch_primary: u32,
    /// Placeholder audio volume 0..=100 (doc nod: EXW music volume 0..100).
    volume: i32,
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
/// rng_state u64 @0, rng_stream u64 @8, cursor_x i32 @16, cursor_y i32 @20,
/// latch_primary u32 @24, volume i32 @28, last_draw u32 @32,
/// initial_state_hash u64 @36.
const STATE_LEN: usize = 44;

impl Sim {
    /// Create a simulation: PRNG on [`STREAM_SIM`], placeholder state at its
    /// documented defaults, and `initial_state_hash` recorded from the
    /// tick-0 state hash.
    pub fn new(config: &SimConfig) -> Sim {
        let mut sim = Sim {
            tick: 0,
            rng: Pcg32::new(config.seed, STREAM_SIM),
            time_base: config.time_base,
            initial_state_hash: 0,
            last_draw: 0,
            cursor_x: 0,
            cursor_y: 0,
            latch_primary: 0,
            volume: 50,
        };
        sim.initial_state_hash = sim.state_hash().0;
        sim
    }

    /// Advance exactly one fixed timestep.
    ///
    /// Determinism rules baked in here:
    /// - exactly ONE PRNG draw per tick (the entropy slot), and it is stored;
    /// - cursor integrates this tick's mouse deltas then CLAMPS to the
    ///   640x480 game space (clamp, not modulo — divergence-safe and
    ///   EXW-style);
    /// - `buttons` bit 0 press latches 1, release clears 0.
    pub fn tick(&mut self, input: &InputFrame) {
        self.tick += 1;
        self.last_draw = self.rng.next_u32();
        self.cursor_x = (self.cursor_x + i32::from(input.mouse_dx)).clamp(0, CURSOR_MAX_X);
        self.cursor_y = (self.cursor_y + i32::from(input.mouse_dy)).clamp(0, CURSOR_MAX_Y);
        if input.buttons & 1 != 0 {
            self.latch_primary = 1;
        } else {
            self.latch_primary = 0;
        }
        // `volume` has no input path yet (keyboard volume hotkeys are
        // pending P2e input RE); it stays a hashed placeholder knob.
    }

    /// Current tick index (ticks elapsed since creation).
    pub fn tick_index(&self) -> Tick {
        self.tick
    }

    /// The time base this sim runs at.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// The most recent per-tick entropy draw (the entropy slot value).
    pub fn last_draw(&self) -> u32 {
        self.last_draw
    }

    /// Placeholder cursor position `(x, y)` in 640x480 game space.
    pub fn cursor(&self) -> (i32, i32) {
        (self.cursor_x, self.cursor_y)
    }

    /// `state_hash()` at tick 0, recorded at creation and carried across
    /// snapshots/replays (this is the `initial_state_hash` replay field).
    pub fn initial_state_hash(&self) -> u64 {
        self.initial_state_hash
    }

    /// FNV-1a 64 over the canonical serialization of ALL state fields, in
    /// this FIXED order: `tick` u64, rng state u64, rng stream u64,
    /// `cursor_x` i32, `cursor_y` i32, `latch_primary` u32, `volume` i32,
    /// `last_draw` u32. The order is a stability contract; appending new
    /// fields (P4+) goes at the END of this list with a `FORMAT_VERSION`
    /// bump.
    pub fn state_hash(&self) -> StateHash {
        let mut h = Fnv1a64::new();
        h.write_u64(self.tick);
        h.write_u64(self.rng.state());
        h.write_u64(self.rng.stream());
        h.write_i32(self.cursor_x);
        h.write_i32(self.cursor_y);
        h.write_u32(self.latch_primary);
        h.write_i32(self.volume);
        h.write_u32(self.last_draw);
        StateHash(h.finish())
    }

    /// Capture a snapshot of the full state.
    pub fn snapshot(&self) -> Snapshot {
        let mut state_bytes = Vec::with_capacity(STATE_LEN);
        state_bytes.extend_from_slice(&self.rng.state().to_le_bytes());
        state_bytes.extend_from_slice(&self.rng.stream().to_le_bytes());
        state_bytes.extend_from_slice(&self.cursor_x.to_le_bytes());
        state_bytes.extend_from_slice(&self.cursor_y.to_le_bytes());
        state_bytes.extend_from_slice(&self.latch_primary.to_le_bytes());
        state_bytes.extend_from_slice(&self.volume.to_le_bytes());
        state_bytes.extend_from_slice(&self.last_draw.to_le_bytes());
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
    /// length (truncation), then the stored hash is recomputed from the
    /// parsed fields (canonical order, see [`Sim::state_hash`]) and must
    /// match, and the snapshot's time base must equal
    /// `expected_config.time_base` ([`CoreError::BadTickHz`] otherwise).
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
        let cursor_x = i32::from_le_bytes([s[16], s[17], s[18], s[19]]);
        let cursor_y = i32::from_le_bytes([s[20], s[21], s[22], s[23]]);
        let latch_primary = u32::from_le_bytes([s[24], s[25], s[26], s[27]]);
        let volume = i32::from_le_bytes([s[28], s[29], s[30], s[31]]);
        let last_draw = u32::from_le_bytes([s[32], s[33], s[34], s[35]]);
        let initial_state_hash =
            u64::from_le_bytes([s[36], s[37], s[38], s[39], s[40], s[41], s[42], s[43]]);

        // Re-hash in the canonical field order (tick comes from the header).
        let mut h = Fnv1a64::new();
        h.write_u64(tick);
        h.write_u64(rng_state);
        h.write_u64(rng_stream);
        h.write_i32(cursor_x);
        h.write_i32(cursor_y);
        h.write_u32(latch_primary);
        h.write_i32(volume);
        h.write_u32(last_draw);
        let computed = h.finish();
        if computed != stored_hash {
            return Err(CoreError::HashMismatch {
                stored: stored_hash,
                computed,
            });
        }

        if tick_hz != expected_config.time_base.tick_hz {
            return Err(CoreError::BadTickHz(tick_hz));
        }

        Ok(Sim {
            tick,
            rng: Pcg32::from_raw_parts(rng_state, rng_stream),
            time_base: TimeBase { tick_hz },
            initial_state_hash,
            last_draw,
            cursor_x,
            cursor_y,
            latch_primary,
            volume,
        })
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
    fn cursor_clamps_into_game_space() {
        let mut sim = Sim::new(&SimConfig::default());
        for _ in 0..8 {
            sim.tick(&InputFrame {
                mouse_dx: 200,
                mouse_dy: -100,
                ..InputFrame::default()
            });
        }
        assert_eq!(sim.cursor(), (639, 0));
    }

    #[test]
    fn latch_tracks_buttons_bit0() {
        let config = SimConfig::default();
        let press = InputFrame {
            buttons: 1,
            ..InputFrame::default()
        };
        // Held vs released on tick 2: identical RNG/cursor trajectory, the
        // latch is the only difference => different state hashes.
        let mut held = Sim::new(&config);
        held.tick(&press);
        held.tick(&press);
        let mut released = Sim::new(&config);
        released.tick(&press);
        released.tick(&InputFrame::default());
        assert_ne!(held.state_hash(), released.state_hash());

        // Buttons OUTSIDE bit 0 have no placeholder effect: same trajectory.
        let mut other_bit = Sim::new(&config);
        other_bit.tick(&InputFrame {
            buttons: 0b1110,
            ..InputFrame::default()
        });
        let mut no_buttons = Sim::new(&config);
        no_buttons.tick(&InputFrame::default());
        assert_eq!(other_bit.state_hash(), no_buttons.state_hash());
    }

    #[test]
    fn one_draw_per_tick() {
        // Tick with identical inputs: only the PRNG state advances, and it
        // advances by exactly one draw per tick (hash changes every tick).
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
        };
        let mut sim = Sim::new(&config);
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
        };
        assert_eq!(Sim::restore(&bytes, &other), Err(CoreError::BadTickHz(60)));
    }
}
