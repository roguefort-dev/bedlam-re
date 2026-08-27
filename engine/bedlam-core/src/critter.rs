//! The critter-actor family (P4.2/W12-S8, DESIGN-DIFFHARNESS §7 S8
//! row + §10-W12; RE-EXW-SIM §7j.42 — every [verified] tag below
//! cites §7j.42's instruction-walk of FUN_00412f34 + the §7j.17/
//! §7j.18/§7j.23/§7j.24/§7j.29 lanes the controller reaches).
//!
//! Scope: the E-side model of the 0x4cff98 critter bank (0x7E
//! stride, count cell 0x46cc2c) + the .NME staging host seam + the
//! controller subset for the CORPUS KINDS — 1 (the wanderers,
//! §7j.71), 4 (seek steppers) and 5/6 (the shared mixed-AI body;
//! §7j.72 landed the S6 staging — 26 corpus missions host it). The
//! kind 2/3/7 controller bodies are documented E-gaps:
//! `stage_critters` REFUSES an .NME hosting them (fail loud — never
//! spawn a critter whose brain is missing).
//!
//! Coordinate scales (§7j.23/2 + §7j.42's probe reads): x/y are
//! Q13 for kinds 2/3/5/6/7 but RAW px (= Q5 counts) for kinds 1/4;
//! z is Q5 (32/tile) for every kind — the projectile spawn's
//! `(z+0x10)<<8` and the walker's `|rz − pz>>8| < 0x20` box pin it.
//!
//! The bank does NOT enter `MissionSim::state_hash` (the W6 split —
//! its dump row is the T2 `critter-bank` watch, an E-ONLY coverage
//! row: no EXD alias exists, so the differ reports it as a coverage
//! finding, never fabricated on O1). What IS aliased: the RNG
//! stream (T0 — every controller draw), the robot bank (T1 — the
//! stun/knock + damage lanes), the projectile bank (T2 — the 0x68
//! fire cycle), and the score (T0 — the §7j.24 death bounty).
//!
//! NO-INJECT INVARIANT: with the family unarmed (the grammar
//! `critters = 1` key) the controller never runs and
//! `advance_frame` is byte-identical to the pre-S8 engine — pinned
//! by the S0..S7 canonical chains. The ORIGINAL runs the controller
//! every mission (it loads .NME natively; MissionShell 0x447fe1 is
//! ungated): the per-frame draws on unarmed paths are the recorded
//! stream gap (§7j.42/5, the D113 pattern).
//!
//! E-gaps inside the modeled kinds (documented, corpus-dead on the
//! S8 staging tiles or presentation-only): the 8-sample walk probe
//! family (FUN_0041e6a8 + the 0x4543e4/0x454404 pointer cells —
//! blocked-path fidelity; open flat ground probes pass on both
//! channels), the blocked-step climb ladder (FUN_00415ff2's
//! alternatives), the epilogue presence-mark arena (0x46af58) and
//! 8-corner z-settle (no stream draws), the SFX families (BEAMIN,
//! BIOFIRE, the juice/death trios — T4), and the aim-angle byte
//! (FUN_0041eb7d/ebc1 vs the engine's 32-sector table — positions
//! only, E-only rows).

use crate::mission::{dist_octagonal, MissionSim, FACING_NONE};
use crate::weapon::EnemyProjectile;

/// Critter bank capacity: 0xAC44 bytes / 0x7E stride at 0x4cff98.
pub const CRITTER_SLOTS: usize = 0x2AC;

/// The kind-4/5/6 respawn-delay table DAT_00454edc (DGROUP,
/// file-extract §7j.42/7): difficulty 0/1/2 → 1500/900/600.
pub const RESPAWN_DELAYS: [i32; 3] = [1500, 900, 600];

/// The 0x4cec38 effect-row bank capacity [§7j.24/5]: 80 × 0x20.
pub const EFFECT_ROWS: usize = 80;

/// One 0x4cec38 effect row [§7j.24/5 — the FUN_0041a14f spawn
/// grammar]. Presentation-side T3 surface (E-ONLY row: no EXD
/// alias); NEVER enters `state_hash` (the W6 split).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectRow {
    /// Age word w@+0: incremented on EVERY row once per spawn
    /// call (FUN_0041a494), zeroed on allocation.
    pub age: u16,
    /// Position d@+2/+6/+0xA (caller's scale — Q13 x/y, z·0x100).
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Velocity components d@+0xE/+0x12: cos/sin of a RandA angle
    /// `>> 8` [modeled as the engine sine words on the low byte —
    /// the exact FUN_0041eb65/77 scale is E-only row content].
    pub cos: i32,
    pub sin: i32,
    /// d@+0x16 := (RandA&7)·0x10 + 0x80 (the ttl/size word).
    pub ttl: i32,
    /// Sprite/variant id w@+0x1A: i for i < 8, else
    /// FUN_0041ec1c(5,0)+3 (one draw per overflow row).
    pub id: u16,
}

/// The seek/range-attack distance gate (0x1F4 px, §7j.42/2).
const RANGE_GATE: i32 = 0x1F4;
/// The engage close-band floor (0x60 px, §7j.42/3).
const CLOSE_BAND: i32 = 0x60;
/// The engage hold-band ceiling (0x80 px): inside it the mode-8 →
/// mode-2 transition fires (§7j.42/7).
const HOLD_BAND: i32 = 0x80;

/// One critter record — the modeled subset of the 0x7E-stride
/// frame [§7j.17 item 1, field names per §7j.42/1].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CritterRecord {
    /// Kind w@+0x00 — the .NME section state {1..7}; the dispatch
    /// key (table 0x412f18, §7j.42/1).
    pub kind: u16,
    /// Species w@+0x02 — for the mixed kinds the SUBSTEPS-PER-FRAME
    /// count (§7j.42/2); the death handlers re-stamp it to 1.
    pub species: u16,
    /// Attacker u16@+0x04 — the owner of the last robot-inflicted
    /// hit (−1 = environment; the bounty gate reads it, §7j.24/2).
    pub attacker: i16,
    /// HP s16@+0x06 — `hp ≤ 0` is the death test (§7j.23/1).
    pub hp: i16,
    /// Mode w@+0xC — the runtime AI state: 0xB dormant, 9 seek,
    /// 2 range-attack, 8 engage, 3 chase, 7 dying, 6 ballistic
    /// (the death dive), 5 stun/rise, 0xA pause.
    pub mode: u16,
    /// Anim counter w@+0xE (the FUN_0041642d wrap counter).
    pub anim: u16,
    /// Heading d@+0x10 — DUAL-PURPOSE (§7j.29): the wander/aim
    /// heading 0..255, or the 2-bit SEEK direction 0..3 in mode 9.
    pub heading: i32,
    /// Presence w@+0x24 — 0 skips the critter whole (main loop).
    pub presence: bool,
    /// The attack-target triple d@+0x2A/+0x2E/+0x32 — the mode-8→2
    /// transition stages the nearest robot's (x Q13, y Q13, z Q5)
    /// (§7j.42/7).
    pub target_x: i32,
    pub target_y: i32,
    pub target_z: i32,
    /// Impact x/y d@+0x1C/+0x20 — the stun/knock destination the
    /// mode-6 dive steers at (§7j.42/2).
    pub impact_x: i32,
    pub impact_y: i32,
    /// World x/y — Q13 for kind 5/6, RAW px for kind 4 (§7j.23/2).
    pub x: i32,
    pub y: i32,
    /// World z, Q5 (8·32 per level; §7j.42's spawn `<<8` reading).
    pub z: i32,
    /// Home x/y d@+0x42/+0x46 (spawn x/y).
    pub home_x: i32,
    pub home_y: i32,
    /// Multiplexed countdown w@+0x56 — dormant timer / seek pause /
    /// fire cadence / chase count, per mode (§7j.42/2); for kind 1
    /// the wander countdown (§7j.71/3).
    pub countdown: i32,
    /// DIR w@+0x58 — the kind-1 wander direction: −1 = idle,
    /// 0..3 = walking {−y, +x, +y, −x} (§7j.71/3; the S2 loader
    /// seeds −1).
    pub dir: i16,
    /// Frame word w@+0x5A — the kind-1 DIR mirror, written on every
    /// direction pick (§7j.71/3).
    pub frame: u16,
    /// z-restore d@+0x4E — the kind-1 standing level z, restored on
    /// the idle squash + the bounds re-pick (§7j.71/1,/3).
    pub z_restore: i32,
    /// Dying counter d@+0x52 (mode 7 runs 0x28 frames).
    pub death_ctr: i32,
    /// Target robot w@+0x7A (the mode-2 fire victim; −1 = none).
    pub target_robot: i16,
    /// Fuse / hit-flash w@+0x7C — the main loop decrements it every
    /// frame BEFORE the dispatch (§7j.42/1); a hit sets 1.
    pub fuse: u16,
    /// Facing word w@+0x72 (the idle 1/32 drift writes ±0xF).
    pub facing: u16,
}

impl Default for CritterRecord {
    fn default() -> Self {
        CritterRecord {
            kind: 0,
            species: 0,
            attacker: -1,
            hp: 0,
            mode: 0,
            anim: 0,
            heading: 0,
            presence: false,
            target_x: 0,
            target_y: 0,
            target_z: 0,
            impact_x: 0,
            impact_y: 0,
            x: 0,
            y: 0,
            z: 0,
            home_x: 0,
            home_y: 0,
            countdown: 0,
            dir: -1,
            frame: 0,
            z_restore: 0,
            death_ctr: 0,
            target_robot: -1,
            fuse: 0,
            facing: 0,
        }
    }
}

impl MissionSim {
    /// Stage the mission's .NME through the FUN_00416458 spawn
    /// schedule (§7j.18) — the `critters = 1` grammar host seam
    /// (D114). The ORIGINAL loads the file natively at mission
    /// load; E stages the identical bytes. Only the sections whose
    /// kinds the E controller models may spawn (S2 → kind 1,
    /// S3 → kind 5, S4 → kind 4, S6 → kind 6); any other NON-EMPTY
    /// section is REFUSED (fail loud — never spawn a brain the
    /// engine does not carry; ZONEA/MISSION1 hosts exactly S3+S4).
    ///
    /// Spawn schedule [verified §7j.18 + §7j.71/1 + §7j.72]: S2
    /// (state 1, 10-B recs, w3/w4 = x/y tile): `d+3` each — the z
    /// SEARCH walks the DAT volume down from level 6 for the first
    /// RAW tile ∈ 1..3 with air above (skip the spawn when none);
    /// x = w3·0x20+0x10, y = w4·0x20+0x10 (RAW px), z = z-restore =
    /// L·0x20+0x1F, DIR −1, species 1, countdown =
    /// FUN_0041ec1c(10)+10 (the section's ONLY stream draw, one per
    /// spawned critter), hp = 200+(200·m)/27. S3 (state 5, 8-B
    /// recs, w1 = probe level, w2/w3 = x/y tile): `max(d,1)` each —
    /// x = w2·0x2000+0xF00, y = w3·0x2000+0xF00 (Q13), z = the
    /// floor probe at level w1, mode 8, species 3, anim 5, heading
    /// 0x72, hp base 0x96. S4 (state 4, 8-B recs): `(d>>1)+2` each
    /// — x = w2·0x20+0xF, y = w3·0x20+0xF (RAW px), z = the floor
    /// probe at level w1, mode 9, species 6, heading = RandA()&3
    /// (the loader's only stream draw for these two sections), hp
    /// base 0xC8. S6 (state 6, 8-B recs): ONE each at EVERY
    /// difficulty — the S3 stamps verbatim with kind 6 (mode 8,
    /// species 3, anim 5, heading 0x72, the w1-level floor probe,
    /// hp base 0x96) and NO stream draws (§7j.72/1). **EVERY
    /// section's hp scalar = the LINEAR MISSION m [0x46ae8c]
    /// (§7j.64/D153, §7j.71/1, §7j.72/2) — hp = base+(base·m)/27;
    /// the engine reads `MissionSim::linear`** (staged by the
    /// destroy family BEFORE critters in the canonical order; 0
    /// when unstaged — the S8 chain re-baseline decision, §7j.72/4:
    /// S8 stages no destroy, so its m = 0 and the pinned T2
    /// critter-bank bytes moved 155/207 → 150/200 deliberately).
    /// presence := 1; home := spawn. Returns the staged count, or
    /// `None` when the file hosts an unmodeled kind (the caller
    /// fails loud).
    pub fn stage_critters(&mut self, nme: &[u8], difficulty: u32) -> Option<usize> {
        const WIDTHS: [usize; 8] = [10, 10, 8, 8, 10, 8, 6, 8];
        let mut p = 0usize;
        let mut counts = [0usize; 8];
        let mut sections: [Vec<Vec<u16>>; 8] = Default::default();
        for (si, w) in WIDTHS.iter().enumerate() {
            let count = if p + 2 <= nme.len() {
                u16::from_le_bytes([nme[p], nme[p + 1]]) as usize
            } else {
                0
            };
            p += 2;
            let mut recs = Vec::new();
            for i in 0..count {
                let mut words = vec![0u16; w / 2];
                for (k, word) in words.iter_mut().enumerate() {
                    let off = p + i * w + k * 2;
                    if off + 2 <= nme.len() {
                        *word = u16::from_le_bytes([nme[off], nme[off + 1]]);
                    }
                }
                recs.push(words);
            }
            p += count * w;
            counts[si] = count;
            sections[si] = recs;
        }
        // S8 (personnel/POI) is a separate bank — the poi-bank T2
        // row's own unit. Any other unmodeled section refuses.
        for (si, &n) in counts.iter().enumerate() {
            if n != 0 && si != 1 && si != 2 && si != 3 && si != 5 {
                return None;
            }
        }
        let d = difficulty as i32;
        // The hp scalar [0x46ae8c] = the linear mission m (§7j.71/1).
        let m = self.linear as i32;
        let mut staged = 0usize;
        for rec in &sections[1] {
            // S2 (§7j.71/1): d+3 each; the z search is draw-free and
            // runs per spawn iteration; ONE bounded pick per critter
            // that clears the stand gate.
            let spawns = d + 3;
            for _ in 0..spawns {
                if self.critters.len() >= CRITTER_SLOTS {
                    break;
                }
                let tx = rec[3] as i32;
                let ty = rec[4] as i32;
                let Some(level) = self.wander_stand_level(tx, ty) else {
                    continue;
                };
                let x = tx * 0x20 + 0x10;
                let y = ty * 0x20 + 0x10;
                let z = level * 0x20 + 0x1F;
                let countdown = self.bounded_pick(10) + 10;
                let base = 0xC8i32;
                self.critters.push(CritterRecord {
                    kind: 1,
                    species: 1,
                    hp: (base + base * m / 27) as i16,
                    dir: -1,
                    frame: 0,
                    z_restore: z,
                    x,
                    y,
                    z,
                    countdown,
                    presence: true,
                    ..Default::default()
                });
                staged += 1;
            }
        }
        for rec in &sections[2] {
            let spawns = match d {
                1 => (self.rand_a() & 1) as i32 + 1,
                _ => d.max(1),
            };
            for _ in 0..spawns {
                if self.critters.len() >= CRITTER_SLOTS {
                    break;
                }
                let x = rec[2] as i32 * 0x2000 + 0xF00;
                let y = rec[3] as i32 * 0x2000 + 0xF00;
                let base = 0x96i32;
                let z = self.terrain.floor_z(x >> 8, y >> 8, rec[1] as i32 * 32);
                self.critters.push(CritterRecord {
                    kind: 5,
                    species: 3,
                    // §7j.72/2 + /4: the scalar is the linear
                    // mission m [0x46ae8c] — the §7j.71/1 hold
                    // retired by the D179 rider (every section's
                    // imul site reads the same cell).
                    hp: (base + base * m / 27) as i16,
                    mode: 8,
                    anim: 5,
                    heading: 0x72,
                    presence: true,
                    x,
                    y,
                    z,
                    home_x: x,
                    home_y: y,
                    ..Default::default()
                });
                staged += 1;
            }
        }
        for rec in &sections[3] {
            let spawns = (d >> 1) + 2;
            for _ in 0..spawns {
                if self.critters.len() >= CRITTER_SLOTS {
                    break;
                }
                let x = rec[2] as i32 * 0x20 + 0xF;
                let y = rec[3] as i32 * 0x20 + 0xF;
                let base = 0xC8i32;
                // RAW px = Q5 — the probe reads the spawn tile itself.
                let z = self.terrain.floor_z(x, y, rec[1] as i32 * 32);
                let heading = (self.rand_a() & 3) as i32;
                self.critters.push(CritterRecord {
                    kind: 4,
                    species: 6,
                    hp: (base + base * m / 27) as i16, // §7j.72/4 — see the S3 note
                    mode: 9,
                    heading,
                    presence: true,
                    x,
                    y,
                    z,
                    home_x: x,
                    home_y: y,
                    ..Default::default()
                });
                staged += 1;
            }
        }
        for rec in &sections[5] {
            // S6 (§7j.72/1): ONE each at EVERY difficulty — no inner
            // spawn loop, NO stream draws (the decompile's block has
            // zero RandA/FUN_0041ec1c sites); the S3 stamps verbatim
            // with the kind word 6 (the shared k5/6 mixed body).
            if self.critters.len() >= CRITTER_SLOTS {
                break;
            }
            let x = rec[2] as i32 * 0x2000 + 0xF00;
            let y = rec[3] as i32 * 0x2000 + 0xF00;
            let base = 0x96i32;
            let z = self.terrain.floor_z(x >> 8, y >> 8, rec[1] as i32 * 32);
            self.critters.push(CritterRecord {
                kind: 6,
                species: 3,
                hp: (base + base * m / 27) as i16, // §7j.72/2 — the m cell, as every section
                mode: 8,
                anim: 5,
                heading: 0x72,
                presence: true,
                x,
                y,
                z,
                home_x: x,
                home_y: y,
                ..Default::default()
            });
            staged += 1;
        }
        Some(staged)
    }

    /// Arm the critter family (grammar `critters = 1`): the
    /// ORIGINAL runs the controller every mission from boot; E arms
    /// it per scenario so the S0..S7 chains stay byte-identical
    /// (the per-frame draws on unarmed paths are the recorded
    /// stream gap, §7j.42/5).
    pub fn arm_critter_family(&mut self) {
        self.critter_family_armed = true;
    }

    /// The critter bank (read view for the canonical dump row).
    pub fn critters(&self) -> &[CritterRecord] {
        &self.critters
    }

    /// The 0x4cec38 effect-row bank (read view for the E-only T3
    /// dump row).
    pub fn effect_rows(&self) -> &[EffectRow] {
        &self.effect_rows
    }

    /// FUN_0041a14f(x Q13, y Q13, z Q13, count) — the effect-row
    /// SPAWNER [§7j.24/5, verified]: per row the allocator
    /// FUN_0041a494 ages EVERY row's w@+0 then returns the
    /// MAX-age row (always-evict LRU); the row gets {age 0, the
    /// position, cos/sin of a RandA angle >>8, ttl :=
    /// (RandA&7)·0x10+0x80, id := i (<8) else
    /// FUN_0041ec1c(5,0)+3}. THREE draws per row + one per
    /// overflow id row — the stream-relevant part; the cos/sin
    /// values are the modeled sine-word lookup.
    pub fn stage_effect_rows(&mut self, x: i32, y: i32, z: i32, count: usize) {
        for i in 0..count {
            for r in self.effect_rows.iter_mut() {
                r.age = r.age.wrapping_add(1);
            }
            let mut slot = 0usize;
            let mut max_age = -1i32;
            for (si, r) in self.effect_rows.iter().enumerate() {
                if r.age as i32 > max_age {
                    max_age = r.age as i32;
                    slot = si;
                }
            }
            let ca = (self.rand_a() & 0xFF) as u16;
            let sa = (self.rand_a() & 0xFF) as u16;
            let cos = match self.angles.sine_word(ca) {
                Some(c) => (c as i16 as i32) >> 8,
                None => 0,
            };
            let sin = match self.angles.sine_word(((sa as i32 - 0x40) & 0xFF) as u16) {
                Some(s) => (s as i16 as i32) >> 8,
                None => 0,
            };
            let ttl = ((self.rand_a() & 7) as i32) * 0x10 + 0x80;
            let id = if i < 8 {
                i as u16
            } else {
                (self.bounded_pick(5) + 3) as u16
            };
            self.effect_rows[slot] = EffectRow {
                age: 0,
                x,
                y,
                z,
                cos,
                sin,
                ttl,
                id,
            };
        }
    }

    /// The CRITTER CONTROLLER — FUN_00412f34's modeled subset
    /// [§7j.42/1]. MissionShell calls it at 0x447fe1, BEFORE the
    /// click/command/robot machinery (0x448021+), so it runs at the
    /// head of `advance_frame`. Per critter: presence gate → fuse
    /// decrement → kind dispatch. The epilogue's presence-mark
    /// write, 8-corner z-settle and trap re-probe carry no stream
    /// draws (E-gaps, module doc).
    pub fn critter_tick(&mut self) {
        let respawn = RESPAWN_DELAYS[self.difficulty.min(2) as usize];
        let leash = (self.difficulty as i32 + 1) * 0x40 + 0x258;
        for idx in 0..self.critters.len() {
            if !self.critters[idx].presence {
                continue;
            }
            if self.critters[idx].fuse != 0 {
                self.critters[idx].fuse -= 1;
            }
            match self.critters[idx].kind {
                1 => self.critter_wander(idx),
                4 => self.critter_state4(idx, respawn),
                5 | 6 => self.critter_mixed(idx, respawn, leash),
                // Staging refuses the other kinds (module doc).
                _ => {}
            }
        }
    }

    /// The kind-1 WANDERER body (0x414c96) [§7j.71/2..3]. Entry
    /// sequence: the door-tile gate FUN_004186fc is the documented
    /// E-gap (no door-claim/variant bank mirror engine-side —
    /// byte[0x4796d5+30·tile] of the §7j.12 type-DB rows); then the
    /// suicide-bomb trigger; then the substep machine. Species ≡ 1
    /// for S2 wanderers and nothing re-stamps it.
    fn critter_wander(&mut self, idx: usize) {
        // FUN_00417e2f [§7j.71/2]: nearest robot within 0x30 px →
        // explode (presence := 0, 8 iterations of {3 jitter draws +
        // 1× debris KIND 1 + two bounded-pick(3) splash tile draws +
        // the splash row}, delay = counter>>1) and SKIP the body —
        // the explicit `mov eax,1` return convention.
        let (x, y, z) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        let (_, dist) = self.nearest_robot(x, y);
        if dist < 0x30 {
            self.critters[idx].presence = false;
            for i in 0..8 {
                let jz = z + (self.rand_a() & 0xF) as i32;
                let jy = y + (self.rand_a() & 0x3F) as i32 - 0x1F;
                let jx = x + (self.rand_a() & 0x3F) as i32 - 0x1F;
                let _ = self.stage_debris(jx, jy, jz, 1, i >> 1, -1);
                let sy = self.bounded_pick(3) + (y >> 5) - 1;
                let sx = self.bounded_pick(3) + (x >> 5) - 1;
                let sz = (z >> 5) + 1;
                let _ = self.stage_splash(sx, sy, sz, (i >> 1) as u16);
            }
            return;
        }
        let species = self.critters[idx].species as i32;
        let mut substep = 0i32;
        while substep < species {
            // HEAD (0x414f5f): the countdown decrements FIRST.
            self.critters[idx].countdown -= 1;
            if self.critters[idx].countdown > 0 {
                if self.critters[idx].dir < 0 {
                    // The IDLE SQUASH (0x4151a5): the pause between
                    // walks lasts exactly ONE substep after the dec —
                    // the 8..15/12..27 re-pick constants are squashed
                    // to 1 here (§7j.71/3).
                    let zr = self.critters[idx].z_restore;
                    let c = &mut self.critters[idx];
                    c.countdown = 1;
                    c.z = zr;
                } else {
                    // WALK: the DIR table 0x412f08 (0x4151e8).
                    let dir = self.critters[idx].dir as i32;
                    self.wander_step(idx, dir);
                }
            } else if self.critters[idx].dir >= 0 {
                // WALK-END (0x414d30): one draw.
                let pause = (self.rand_a() & 7) as i32 + 8;
                let c = &mut self.critters[idx];
                c.dir = -1;
                c.countdown = pause;
            } else {
                // The PICK (0x414f89): countdown := (RandA&0xF)+0xA
                // (draw 1); the 25% gate (draw 2) → random 4-way
                // (draw 3) else toward the nearest robot (no draws);
                // then the frame/anim mirror (both paths).
                let pause = (self.rand_a() & 0xF) as i32 + 0xA;
                let gate = self.rand_a();
                let dir = if gate & 3 == 0 {
                    (self.rand_a() & 3) as i32
                } else {
                    self.wander_toward(idx)
                };
                let c = &mut self.critters[idx];
                c.countdown = pause;
                c.dir = dir as i16;
                c.frame = dir as u16;
            }
            substep += 1;
        }
    }

    /// One kind-1 walk step (the DIR table cases 0x414fb9/0x414d56/
    /// 0x414e40/0x4150af) [§7j.71/3]: ±6 RAW px on one axis, the
    /// z-band death gate, the stepped-axis map-bounds gate, then the
    /// 8-sample wall probe at the stepped cell. The death path does
    /// NOT stop the case (the substep loop never re-checks presence).
    fn wander_step(&mut self, idx: usize, dir: i32) {
        let (x, y, z, zr) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z, c.z_restore)
        };
        // The z-band gate (z < 0 ∨ z ≥ 0x100 → FUN_00418250).
        if !(0..0x100).contains(&z) {
            self.wander_die(idx);
        }
        let (nx, ny) = match dir {
            0 => (x, y - 6),
            1 => (x + 6, y),
            2 => (x, y + 6),
            _ => (x - 6, y),
        };
        // The stepped axis carries the map bound ([0x4eddec]·0x20
        // for x, [0x4eddf0]·0x20 for y — §7j.71/3); the other axis
        // is NOT checked on this path, faithful.
        let (w, h) = self.terrain.size();
        let in_bounds = if dir == 1 || dir == 3 {
            (0..w * 0x20).contains(&nx)
        } else {
            (0..h * 0x20).contains(&ny)
        };
        if !in_bounds {
            // Out of map bounds (0x414da2): re-pick + z restore.
            let pause = (self.rand_a() & 0xF) as i32 + 0xC;
            let c = &mut self.critters[idx];
            c.dir = -1;
            c.countdown = pause;
            c.z = zr;
            return;
        }
        if self.wander_probe(nx, ny, zr) {
            // Commit (0x414e00/0x414ef9/0x415078/0x415171): the
            // stepped coordinate only; z unchanged.
            let c = &mut self.critters[idx];
            if dir == 1 || dir == 3 {
                c.x = nx;
            } else {
                c.y = ny;
            }
        } else {
            // Probe blocked (0x414e0f): re-pick, NO z restore.
            let pause = (self.rand_a() & 0xF) as i32 + 0xC;
            let c = &mut self.critters[idx];
            c.dir = -1;
            c.countdown = pause;
        }
    }

    /// FUN_00418250 — the kind-1 death path [§7j.71/6]: mode 7 +
    /// presence 0 ALWAYS; the debris row only when the RAW-px x/y
    /// clear the TILE-width/height bounds (the asm compares px vs
    /// [0x4eddec]/[0x4eddf0] — a near-dead quirk that almost never
    /// fires), z forced 0xFF by the original's <<8 clamp.
    fn wander_die(&mut self, idx: usize) {
        let (w, h) = self.terrain.size();
        let (x, y, z) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        {
            let c = &mut self.critters[idx];
            c.mode = 7;
            c.presence = false;
        }
        if (0..w).contains(&x) && (0..h).contains(&y) && (0..8).contains(&z) {
            let _ = self.stage_debris(x, y, 0xFF, 1, 0, -1);
        }
    }

    /// FUN_00417af2 — the toward-robot 4-way picker [§7j.71/4]:
    /// nearest ALIVE robot in px; the y-axis wins ties (DX ≤ DY);
    /// the cy==ry ∧ on-top degenerate lands on 0. No draws, no
    /// difficulty.
    fn wander_toward(&self, idx: usize) -> i32 {
        let (cx, cy) = {
            let c = &self.critters[idx];
            (c.x, c.y)
        };
        let mut rx = 0i32;
        let mut ry = 0i32;
        let mut best = 10_000_000i32;
        for r in self.robots.iter() {
            if !r.alive {
                continue;
            }
            let d = dist_octagonal((r.pos_x >> 8) - cx, (r.pos_y >> 8) - cy);
            if d < best {
                best = d;
                rx = r.pos_x >> 8;
                ry = r.pos_y >> 8;
            }
        }
        let dx = (cx - rx).abs();
        let dy = (cy - ry).abs();
        if cy >= ry && dx <= dy {
            0
        } else if cy <= ry && dx <= dy {
            2
        } else if cx > rx {
            3
        } else {
            1
        }
    }

    /// FUN_0041f8f9 — the kind-1 wall probe [§7j.71/5]: the 8-sample
    /// footprint (the 3×3 box minus the center, offsets ±11/+12);
    /// per sample the bounds, the floor probe == z exactly
    /// (FUN_0041e231 = the engine floor_z), and the RAW DAT tile
    /// ≤ 3 (air or standable — 0xFF pads FAIL, the raw read).
    fn wander_probe(&mut self, x: i32, y: i32, z: i32) -> bool {
        const SAMPLES: [(i32, i32); 8] = [
            (-11, -11),
            (-11, 12),
            (12, -11),
            (12, 12),
            (0, -11),
            (0, 12),
            (-11, 0),
            (12, 0),
        ];
        let (w, h) = self.terrain.size();
        for &(ox, oy) in SAMPLES.iter() {
            let sx = x + ox;
            let sy = y + oy;
            if sx < 0 || sy < 0 || sx >> 5 >= w || sy >> 5 >= h {
                return false;
            }
            if self.terrain.floor_z(sx, sy, z) != z {
                return false;
            }
            let tile = self
                .terrain
                .raw_dat_byte(sx >> 5, sy >> 5, z >> 5)
                .unwrap_or(0);
            if tile > 3 {
                return false;
            }
        }
        true
    }

    /// The S2 loader's standing-level search [§7j.71/1]: scan the
    /// RAW DAT volume DOWN from level 6; skip air; the first tile
    /// ∈ 1..3 stops the scan (a >3 tile keeps scanning); the stand
    /// gate requires the found level's tile ∈ 1..3 AND air above.
    /// RAW bytes (0xFF pad decks are NOT standable here — no
    /// dat_type remap).
    fn wander_stand_level(&self, tx: i32, ty: i32) -> Option<i32> {
        let raw = |z: i32| self.terrain.raw_dat_byte(tx, ty, z).unwrap_or(0);
        let mut found = -1i32;
        let mut level = 6i32;
        while level >= 0 {
            let t = raw(level);
            if t == 0 {
                level -= 1;
                continue;
            }
            found = level;
            if t <= 3 {
                break;
            }
            level -= 1;
        }
        if found < 0 {
            return None;
        }
        let t = raw(found);
        if t == 0 || t > 3 {
            return None;
        }
        if raw(found + 1) != 0 {
            return None;
        }
        Some(found)
    }

    /// FUN_00417c00 — the nearest-ALIVE-robot probe [§7j.42/3]:
    /// raw octile on px deltas, sentinel (0, 10_000_000) when none.
    fn nearest_robot(&self, px: i32, py: i32) -> (usize, i32) {
        let mut best = 0usize;
        let mut best_d = 10_000_000i32;
        for (i, r) in self.robots.iter().enumerate() {
            if !r.alive {
                continue;
            }
            let d = dist_octagonal((r.pos_x >> 8) - px, (r.pos_y >> 8) - py);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        (best, best_d)
    }

    /// The kind-4 body (the seek steppers, 0x414079) [§7j.42/2]:
    /// `species` substeps per frame; the mode ladder
    /// {0xB dormant, 7 dying, 6 ballistic, 5 stun-rise} then the
    /// shared nearest-robot tail for modes 9 (seek walk) and 2
    /// (range-attack).
    fn critter_state4(&mut self, idx: usize, respawn: i32) {
        let species = self.critters[idx].species as i32;
        let mut substep = 0i32;
        while substep < species {
            match self.critters[idx].mode {
                0xB => {
                    if self.critters[idx].countdown < respawn {
                        if substep == 0 {
                            self.critters[idx].countdown += 1;
                        }
                        substep += 1;
                        continue;
                    }
                    // WAKE (0x4143b9): anim 0, countdown 0, seek
                    // dir RandA&3 (one draw), mode 9, species 6,
                    // hp 0xC8; the same substep falls into the
                    // mode-9 tail. BEAMIN is the T4 E-gap.
                    let dir = (self.rand_a() & 3) as i32;
                    let c = &mut self.critters[idx];
                    c.anim = 0;
                    c.countdown = 0;
                    c.heading = dir;
                    c.mode = 9;
                    c.species = 6;
                    c.hp = 0xC8;
                }
                7 => {
                    let c = &mut self.critters[idx];
                    c.anim = 0;
                    c.hp = 0;
                    c.death_ctr += 1;
                    if c.death_ctr >= 0x28 {
                        c.mode = 0xB;
                        c.countdown = 0;
                    }
                    substep += 1;
                    continue;
                }
                6 => {
                    // Ballistic dive (0x41412c): the (x<<8)−impact
                    // leash in Q13; inside ±0x8000 the knockback
                    // step toward the away-from-impact heading
                    // (mult = max(countdown, 2) — the modeled
                    // core; the asm's direct cos·mult mover is
                    // the documented approximation surface).
                    // ALWAYS after the move attempt: substep-0 →
                    // countdown−−; countdown==0 → mode 7 + counter
                    // reset (0x414225..0x414263 — the ONLY
                    // mode-7 transition; no anim path here).
                    let dx = (self.critters[idx].x << 8) - self.critters[idx].impact_x;
                    let dy = (self.critters[idx].y << 8) - self.critters[idx].impact_y;
                    if dx.abs() < 0x8000 && dy.abs() < 0x8000 {
                        let h = self.angles.angle_byte(dx, dy) as i32;
                        self.critters[idx].heading = h;
                        let mult = self.critters[idx].countdown.max(2);
                        self.critter_dive_step(idx, mult);
                    }
                    if substep == 0 {
                        self.critters[idx].countdown -= 1;
                        if self.critters[idx].countdown <= 0 {
                            self.critters[idx].mode = 7;
                            self.critters[idx].death_ctr = 0;
                        }
                    }
                    substep += 1;
                    continue;
                }
                5 => {
                    // Stun-rise (0x414265): countdown ≥ 2 → mode 9;
                    // substep 0 → countdown++.
                    let c = &mut self.critters[idx];
                    if c.countdown >= 2 {
                        c.mode = 9;
                        c.countdown = 0;
                    } else if substep == 0 {
                        c.countdown += 1;
                    }
                    // The mode flip reaches the mode-9 tail below
                    // the same substep only via the fallthrough —
                    // the asm jumps to 0x4142bd (the tail), so a
                    // fresh mode 9 DOES gate on dist this substep.
                }
                _ => {}
            }
            // The shared tail (0x4142bd) — modes 9/2, gated by the
            // nearest-robot distance (probed per substep at
            // 0x4142a4 for every mode outside the ladder).
            let mode = self.critters[idx].mode;
            if mode != 9 && mode != 2 {
                substep += 1;
                continue;
            }
            let (x, y) = (self.critters[idx].x, self.critters[idx].y);
            let (nearest, dist) = self.nearest_robot(x, y);
            if mode == 9 && dist < RANGE_GATE {
                if self.critters[idx].countdown == 0 {
                    // The re-picker (0x4142fb): gate draw; 25%
                    // random dir (second draw) else the
                    // dominant-axis probe; then the pause word
                    // (third draw).
                    let gate = self.rand_a();
                    let dir = if gate & 3 == 0 {
                        (self.rand_a() & 3) as i32
                    } else {
                        self.dominant_axis(idx, nearest)
                    };
                    let pause = (self.rand_a() & 0x3F) as i32 + 0x20;
                    let c = &mut self.critters[idx];
                    c.heading = dir;
                    c.countdown = pause;
                    substep += 1;
                    continue;
                }
                // The walk (0x4144d7): countdown−−, the 4-way
                // stepper ±1 raw unit (px for kind 4); blocked →
                // countdown 0. The stepper probe is the
                // modeled-approximation (module doc).
                let dir = self.critters[idx].heading & 0xFFFF;
                self.critters[idx].countdown -= 1;
                if dir <= 3 {
                    let (cx, cy, cz) = {
                        let c = &self.critters[idx];
                        (c.x, c.y, c.z)
                    };
                    let (ok, dx, dy) = match dir {
                        0 => (self.step_probe(cx, cy - 1, cz), 0, -1),
                        1 => (self.step_probe(cx + 1, cy, cz), 1, 0),
                        2 => (self.step_probe(cx, cy + 1, cz), 0, 1),
                        _ => (self.step_probe(cx - 1, cy, cz), -1, 0),
                    };
                    if ok {
                        self.critters[idx].x += dx;
                        self.critters[idx].y += dy;
                        self.critter_acquire(idx, dir as u8);
                    } else {
                        self.critters[idx].countdown = 0;
                    }
                }
                substep += 1;
                continue;
            }
            if mode == 2 && dist < RANGE_GATE {
                let cd = self.critters[idx].countdown;
                if cd == 4 {
                    // Re-seek (0x414534).
                    self.critters[idx].mode = 9;
                } else {
                    // FIRE (0x414549): FUN_0040db9e(target, 2,
                    // heading<<6, 1, −1) [§7j.42/4] — damage 1 with
                    // owner dword@0x476fb4 (≡ 0: .bss, sole ref the
                    // gate read, no writer — so robot 0 is the
                    // credited owner) + the stun/knock half: the
                    // 0xFFFF facing write FIRST (outside the state
                    // gate), then state ∉ {3,5} → dir := seed, the
                    // walk-probe gate, the move (cos/sin·2>>7 —
                    // Q13 units, ≤2 px), facing := −1 on pass.
                    let victim = self.critters[idx].target_robot;
                    let seed = (self.critters[idx].heading & 0xFF) << 6;
                    if victim >= 0 {
                        let v = victim as usize;
                        if self.robots.get(v).is_some() {
                            self.apply_damage(v, 1, 0);
                            let state = self.robots[v].state;
                            self.robots[v].facing = 0xFFFF;
                            if state != 3 && state != 5 {
                                let (vx, vy) = self.knock_velocity(seed);
                                self.robots[v].dir_byte = (seed & 0xFFFF) as u16;
                                let (px, py) = {
                                    let r = &self.robots[v];
                                    (r.pos_x + vx, r.pos_y + vy)
                                };
                                if self.move_possible(v, px, py) {
                                    let r = &mut self.robots[v];
                                    r.pos_x = px;
                                    r.pos_y = py;
                                    r.facing = FACING_NONE;
                                }
                            }
                        }
                    }
                    if substep == 0 {
                        self.critters[idx].countdown = cd + 1;
                    }
                }
            }
            substep += 1;
        }
    }

    /// The shared kind-5/6 body (0x41367c) [§7j.42/3 + /7]: the
    /// 1/32 idle facing gate at entry (one draw every frame, a
    /// second on the hit), then `species` substeps of the mode
    /// ladder {0xB dormant, 0xA pause, 7 dying, 6 ballistic,
    /// 5 rise, 8 engage, 2 fire, 3 chase}.
    fn critter_mixed(&mut self, idx: usize, respawn: i32, leash: i32) {
        if self.rand_a() & 0x1F == 0 {
            let f = (self.rand_a() & 0x1F) as i32 - 0xF;
            self.critters[idx].facing = f as u16;
        }
        let species = self.critters[idx].species as i32;
        let mut substep = 0i32;
        while substep < species {
            match self.critters[idx].mode {
                0xB => {
                    if self.critters[idx].countdown < respawn {
                        if substep == 0 {
                            // The BEAMIN pre-wake at table−9 is the
                            // T4 E-gap; the counter write is the
                            // observable (0x4136b1).
                            self.critters[idx].countdown += 1;
                        }
                        substep += 1;
                        continue;
                    }
                    // WAKE (0x413a43): heading = FUN_0041ec1c(0xFF)
                    // (one draw), mode 8, species 3, hp 0x96.
                    // asm 0x413a93: jmp 0x4136fc — the wake
                    // RE-DISPATCHES THE SAME SUBSTEP into ENGAGE
                    // (no substep burn).
                    let heading = self.bounded_pick(0xFF);
                    let c = &mut self.critters[idx];
                    c.anim = 0;
                    c.countdown = 0;
                    c.presence = true;
                    c.heading = heading;
                    c.species = 3;
                    c.mode = 8;
                    c.hp = 0x96;
                    continue;
                }
                0xA => {
                    if self.critters[idx].countdown == 0 {
                        let c = &mut self.critters[idx];
                        c.mode = 8;
                        c.anim = 2;
                    } else {
                        // The pause tail (0x4139bd): countdown−−,
                        // heading := countdown (the shared-word
                        // quirk), step; substep-0 → the anim-6
                        // wrap (FUN_0041642d).
                        self.critters[idx].countdown -= 1;
                        let h = self.critters[idx].countdown;
                        self.critters[idx].heading = h;
                        self.critter_step(idx);
                        if substep == 0 {
                            let a = self.critters[idx].anim;
                            self.critters[idx].anim = (a + 1) % 6;
                        }
                    }
                    substep += 1;
                    continue;
                }
                7 => {
                    let c = &mut self.critters[idx];
                    c.anim = 0;
                    c.hp = 0;
                    c.death_ctr += 1;
                    if c.death_ctr >= 0x28 {
                        c.mode = 0xB;
                        c.countdown = 0;
                    }
                    substep += 1;
                    continue;
                }
                6 => {
                    // Ballistic dive (0x413793): aim AT the impact
                    // (angle of impact−critter), heading := the
                    // aim, step the REVERSED heading ((aim+0x80)
                    // &0xFF — the knockback away); anim++ at
                    // substep 0 ONLY; anim ≥ 8 → mode 7 + counter
                    // 0. No countdown traffic (asm 0x41378a..
                    // 0x413838).
                    let h = self.angles.angle_byte(
                        self.critters[idx].impact_x - self.critters[idx].x,
                        self.critters[idx].impact_y - self.critters[idx].y,
                    ) as i32;
                    self.critters[idx].heading = h;
                    let rev = (h + 0x80) & 0xFF;
                    self.critter_step_heading(idx, rev);
                    if substep == 0 {
                        self.critters[idx].anim += 1;
                    }
                    if self.critters[idx].anim >= 8 {
                        let c = &mut self.critters[idx];
                        c.mode = 7;
                        c.death_ctr = 0;
                    }
                    substep += 1;
                    continue;
                }
                5 => {
                    // Rise (0x413846): anim > 1 → mode 8 + anim 2
                    // and the flip RE-DISPATCHES ENGAGE the same
                    // substep (asm 0x413854..0x41386c: jmp
                    // 0x4136fc); else heading := the at-impact aim
                    // ONLY (no step — asm 0x413871..0x4138da),
                    // anim++ at substep 0, substep burn.
                    if self.critters[idx].anim > 1 {
                        let c = &mut self.critters[idx];
                        c.mode = 8;
                        c.anim = 2;
                        continue;
                    }
                    let h = self.angles.angle_byte(
                        self.critters[idx].impact_x - self.critters[idx].x,
                        self.critters[idx].impact_y - self.critters[idx].y,
                    ) as i32;
                    self.critters[idx].heading = h;
                    if substep == 0 {
                        self.critters[idx].anim += 1;
                    }
                    substep += 1;
                    continue;
                }
                8 => {
                    // ENGAGE (0x413a98): the SP gate [0x4dd410] ≡ 0
                    // (sole text ref, no writer). Band geometry
                    // [asm 0x413ad1..0x413cfb, exact]:
                    // dist < 0x60 → POINT-BLANK RETREAT (aim at
                    // the robot + 0x80 + facing, step, the
                    // substep-0 anim-6 wrap); 0x60 ≤ dist ≤ 0x80
                    // → the TRANSITION to mode 2 (anim 0,
                    // countdown (RandA&0x1F)+0xA — one draw, the
                    // aim heading, the robot triple staged at
                    // +0x2A/+0x2E/+0x32); 0x80 < dist < leash →
                    // the 1/128 juice roll (draw ALWAYS consumed)
                    // + heading := aim + facing, step, anim wrap;
                    // dist ≥ leash → the quiet skip.
                    let (x, y) = (self.critters[idx].x, self.critters[idx].y);
                    let (nearest, dist) = self.nearest_robot(x >> 8, y >> 8);
                    if dist < CLOSE_BAND {
                        let aim = self.aim_angle(nearest, idx);
                        let facing = self.critters[idx].facing as i32;
                        self.critters[idx].heading = (aim + 0x80 + facing) & 0xFF;
                        self.critter_step(idx);
                        if substep == 0 {
                            let a = self.critters[idx].anim;
                            self.critters[idx].anim = (a + 1) % 6;
                        }
                    } else if dist <= HOLD_BAND {
                        // The TRANSITION (0x413c43): one draw.
                        let cd = (self.rand_a() & 0x1F) as i32 + 0xA;
                        let aim = self.aim_angle(nearest, idx);
                        let (tx, ty, tz) = self
                            .robots
                            .get(nearest)
                            .map(|r| (r.pos_x, r.pos_y, r.z))
                            .unwrap_or((0, 0, 0));
                        let c = &mut self.critters[idx];
                        c.mode = 2;
                        c.anim = 0;
                        c.countdown = cd;
                        c.heading = aim;
                        c.target_robot = nearest as i16;
                        c.target_x = tx;
                        c.target_y = ty;
                        c.target_z = tz;
                    } else if dist < leash {
                        // The approach band (0x413b85): the juice
                        // roll is ALWAYS consumed; heading := the
                        // plain aim + facing (NO +0x80).
                        let _juice = self.rand_a();
                        let aim = self.aim_angle(nearest, idx);
                        let facing = self.critters[idx].facing as i32;
                        self.critters[idx].heading = (aim + facing) & 0xFF;
                        self.critter_step(idx);
                        if substep == 0 {
                            let a = self.critters[idx].anim;
                            self.critters[idx].anim = (a + 1) % 6;
                        }
                    }
                    // Beyond the leash: the quiet skip.
                    substep += 1;
                    continue;
                }
                2 => {
                    // FIRE (0x413d00) [§7j.42/7]: substep 0 →
                    // anim++; anim ≤ 1 → tail; else anim := 0 and
                    // the 0x68 spawn with the full 3-D aim.
                    let mut attempted = false;
                    let mut slot_ok = false;
                    if substep == 0 {
                        self.critters[idx].anim += 1;
                    }
                    if self.critters[idx].anim > 1 {
                        self.critters[idx].anim = 0;
                        attempted = true;
                        slot_ok = self.spawn_critter_projectile(idx);
                    }
                    // Tail (0x413e58): countdown−−; break when it
                    // hits 0 or the spawn failed (edi == −1);
                    // otherwise the roll — d=0 breaks on
                    // (RandA&7)==0, d=1 on (RandA&0xF)==0, and
                    // d=2 NEVER rolls (asm 0x413e81: jne straight
                    // to the substep burn — no draw) [corrects
                    // the §7j.42/7 "always" gloss].
                    self.critters[idx].countdown -= 1;
                    let breakout = if self.critters[idx].countdown == 0 || (attempted && !slot_ok) {
                        true
                    } else {
                        match self.difficulty {
                            0 => self.rand_a() & 7 == 0,
                            1 => self.rand_a() & 0xF == 0,
                            _ => false,
                        }
                    };
                    if breakout {
                        // Mode 3 chase + the strafe jitter (0x413e9b):
                        // ONE roll on EVERY break path (the d=2
                        // countdown/slot breaks included); (RandA&1)
                        // == 0 → heading −= facing+0x40 else +=.
                        let facing = self.critters[idx].facing as i32;
                        let delta = facing + 0x40;
                        let add = if self.rand_a() & 1 == 0 {
                            -delta
                        } else {
                            delta
                        };
                        let c = &mut self.critters[idx];
                        c.mode = 3;
                        c.anim = 2;
                        c.countdown = 6;
                        c.heading += add;
                    }
                    substep += 1;
                    continue;
                }
                3 => {
                    // CHASE (0x413f06): the 1/128 juice draw is
                    // ALWAYS consumed; step the current heading;
                    // substep 0 → anim wrap 6; countdown → mode 8.
                    let _juice = self.rand_a();
                    self.critter_step(idx);
                    if substep == 0 {
                        let c = &mut self.critters[idx];
                        c.anim = (c.anim + 1) % 6;
                    }
                    self.critters[idx].countdown -= 1;
                    if self.critters[idx].countdown == 0 {
                        let c = &mut self.critters[idx];
                        c.mode = 8;
                        c.anim = 2;
                    }
                    substep += 1;
                    continue;
                }
                _ => {
                    substep += 1;
                }
            }
        }
    }

    /// The mode-2 projectile spawn (0x413d35..0x413e53): a
    /// type-0x68 record into the 50×0x22 bank at the critter's
    /// (x, y, (z+0x10)<<8) with the octile-normalized 3-D velocity
    /// at the STAGED target [§7j.42/7 — exact integer math; the
    /// owner word is NOT written (stale-slot faithful)].
    fn spawn_critter_projectile(&mut self, idx: usize) -> bool {
        let Some(slot) = self.enemy_free_slot() else {
            return false;
        };
        let c = &self.critters[idx];
        let dx = (c.target_x >> 8) - (c.x >> 8);
        let dy = (c.target_y >> 8) - (c.y >> 8);
        let dz = (c.target_z + 4) - (c.z + 0x10);
        let dist = dist_octagonal(dx, dy);
        let dist = if dist == 0 { 1 } else { dist };
        let vx = dx * 0x800 / dist;
        let vy = dy * 0x800 / dist;
        let den2 = {
            let d0 = dist * 0x10;
            let d1 = dz * 0x10;
            let d = dist_octagonal(d0, d1);
            if d == 0 {
                1
            } else {
                d
            }
        };
        let vz = dz * 0x8000 / den2;
        self.enemy_bank[slot] = EnemyProjectile {
            kind: 0x68,
            x: c.x,
            y: c.y,
            z: (c.z + 0x10) << 8,
            vx,
            vy,
            vz,
        };
        true
    }

    /// FUN_004181bd's core — the dominant-axis direction toward a
    /// robot (0 = −y, 1 = +x, 2 = +y, 3 = −x) [§7j.17/6]. The
    /// deltas are in the ASKING critter's own scale (Q13 vs the
    /// robot Q13 for kind 5/6; raw px vs robot px for kind 4 —
    /// the ratio is scale-invariant).
    fn dominant_axis(&self, idx: usize, robot: usize) -> i32 {
        let (Some(c), Some(r)) = (self.critters.get(idx), self.robots.get(robot)) else {
            return 0;
        };
        // The asking critter's scale: kind 4 is raw px (= Q5),
        // the mixed kinds Q13.
        let (cx, cy) = if c.kind == 4 {
            (c.x, c.y)
        } else {
            (c.x >> 8, c.y >> 8)
        };
        let dx = (r.pos_x >> 8) - cx;
        let dy = (r.pos_y >> 8) - cy;
        if dx.abs() > dy.abs() {
            if dx >= 0 {
                1
            } else {
                3
            }
        } else if dy >= 0 {
            2
        } else {
            0
        }
    }

    /// FUN_00415490's modeled core (§7j.29): the directional
    /// forward-acquisition probe — the walk-axis window (−4, 0xF]
    /// against every robot, |Δ| < 0x18 crossing + z. On hit:
    /// target := the robot, mode := 2, countdown := 0.
    fn critter_acquire(&mut self, idx: usize, dir: u8) {
        let (cx, cy, cz) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        for (i, r) in self.robots.iter().enumerate() {
            if !r.alive {
                continue;
            }
            let rx = r.pos_x >> 8;
            let ry = r.pos_y >> 8;
            let hit = match dir {
                0 => (cx - rx) > -4 && (cx - rx) <= 0xF && (cy - ry).abs() < 0x18,
                1 => (rx - cx) > -4 && (rx - cx) <= 0xF && (ry - cy).abs() < 0x18,
                2 => (ry - cy) > -4 && (ry - cy) <= 0xF && (rx - cx).abs() < 0x18,
                _ => (cx - rx) > -4 && (cx - rx) <= 0xF && (ry - cy).abs() < 0x18,
            };
            if hit && (cz - r.z).abs() < 0x18 {
                let c = &mut self.critters[idx];
                c.target_robot = i as i16;
                c.mode = 2;
                c.countdown = 0;
                return;
            }
        }
    }

    /// The mode-9 seek-stepper probe (FUN_00417f2c family,
    /// head-decoded §7j.42/2): the target must stay in bounds with
    /// the floor height within 0xF Q5 of the critter's z
    /// [modeled-approximation — the full 8-sample probe ladder is
    /// the refinement surface; open flat ground passes on both
    /// channels]. The kind-4 callers pass RAW px (= Q5); the
    /// bounds/probe scales match.
    fn step_probe(&mut self, x: i32, y: i32, z: i32) -> bool {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >> 5 >= w || y >> 5 >= h {
            return false;
        }
        let floor = self.terrain.floor_z(x, y, z);
        (z - floor).abs() < 0xF
    }

    /// FUN_00415ff2's modeled core — the heading step mover
    /// [§7j.42 dumps: x += cos(heading)>>6, y += sin(heading)>>6
    /// behind the FUN_0040cc27 probe; modeled-approximation for the
    /// blocked-path climb ladder — module doc]. The movement is in
    /// the record's own x/y scale (Q13 for kind 5/6). The heading
    /// is an ARGUMENT: several callers step a REVERSED heading
    /// while the record's heading field keeps the aim (the kind-5
    /// mode-6 dive, asm 0x4137e3..0x413804).
    fn critter_step_heading(&mut self, idx: usize, heading: i32) {
        let heading = (heading & 0xFF) as u16;
        let (x, y, z) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        // SIGNED word reads (the table is i16; the u16 view loses
        // the sign for headings past 0x80/0xC0).
        let (dx, dy) = match (
            self.angles.sine_word(heading),
            self.angles
                .sine_word(((heading as i32 - 0x40) & 0xFF) as u16),
        ) {
            (Some(c), Some(s)) => ((c as i16 as i32) >> 6, (s as i16 as i32) >> 6),
            _ => return,
        };
        let nx = x + dx;
        let ny = y + dy;
        let (w, h) = self.terrain.size();
        if nx < 0 || ny < 0 || nx >> 13 >= w || ny >> 13 >= h {
            return;
        }
        let floor = self.terrain.floor_z(nx >> 8, ny >> 8, z);
        if (z - floor).abs() > 3 {
            return;
        }
        let c = &mut self.critters[idx];
        c.x = nx;
        c.y = ny;
    }

    /// Step along the record's own heading field.
    fn critter_step(&mut self, idx: usize) {
        let h = self.critters[idx].heading;
        self.critter_step_heading(idx, h);
    }

    /// The kind-4 mode-6 dive mover [asm 0x4141c2..0x41421f,
    /// modeled]: step = (cos·mult)>>16 / (sin·mult)>>16 in the
    /// record's RAW-px scale (mult = max(countdown, 2)), gated by
    /// the walk probe. The original multiplies through FUN_0041eb65/
    /// 77 whose exact scale this models as the engine sine words.
    fn critter_dive_step(&mut self, idx: usize, mult: i32) {
        let heading = (self.critters[idx].heading & 0xFF) as u16;
        let (x, y, z) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        let (dx, dy) = match (
            self.angles.sine_word(heading),
            self.angles
                .sine_word(((heading as i32 - 0x40) & 0xFF) as u16),
        ) {
            (Some(c), Some(s)) => (
                ((c as i16 as i32) * mult) >> 16,
                ((s as i16 as i32) * mult) >> 16,
            ),
            _ => return,
        };
        let nx = x + dx;
        let ny = y + dy;
        let (w, h) = self.terrain.size();
        if nx < 0 || ny < 0 || nx >> 5 >= w || ny >> 5 >= h {
            return;
        }
        let floor = self.terrain.floor_z(nx, ny, z);
        if (z - floor).abs() > 3 {
            return;
        }
        let c = &mut self.critters[idx];
        c.x = nx;
        c.y = ny;
    }

    /// The aim angle at a robot — the octile-normalized atan2 pair
    /// (FUN_0041eb7d/ebc1) modeled as the engine's 32-sector angle
    /// byte [approximation, positions only — E-only rows; module
    /// doc].
    fn aim_angle(&self, robot: usize, idx: usize) -> i32 {
        let (Some(c), Some(r)) = (self.critters.get(idx), self.robots.get(robot)) else {
            return 0;
        };
        self.angles.angle_byte(r.pos_x - c.x, r.pos_y - c.y) as i32
    }

    /// FUN_0041ec1c(n) — the bounded RandA pick [§7j.42/3]:
    /// `RandA()&0x7FFF` bucketed by 0x8000/n, clamp n−1 (one draw).
    fn bounded_pick(&mut self, n: i32) -> i32 {
        let r = (self.rand_a() & 0x7FFF) as i32;
        let n = n.max(1);
        let v = r / (0x8000 / n);
        v.min(n - 1)
    }

    /// The FUN_0040c536 knock velocity pair — cos/sin of the seed
    /// ·2 >>7 [§7j.42/4]; the caller shifts into Q13.
    fn knock_velocity(&self, seed: i32) -> (i32, i32) {
        let h = (seed & 0xFF) as u16;
        match (
            self.angles.sine_word(h),
            self.angles.sine_word(((h as i32 - 0x40) & 0xFF) as u16),
        ) {
            (Some(c), Some(s)) => (((c as i16 as i32) * 2) >> 7, ((s as i16 as i32) * 2) >> 7),
            _ => (0, 0),
        }
    }

    /// FUN_004197d4 — the odd-pass robot-hit walker [§7j.42
    /// dumps]: every ALIVE robot × every 0x22-bank record of type
    /// {0x65, 0x67, 0x68}: the px box |dx|<0x10 ∧ |dy|<0x10 ∧
    /// |dz|<0x20 (robot z Q5 vs record z>>8) → disburser the
    /// record, FUN_0040e230(robot, weapon_damage(type), owner −1).
    /// Ungated in the shell (an empty bank is a structural no-op —
    /// no draws), so it runs every odd enemy pass like the
    /// original.
    pub fn critter_projectile_walker(&mut self) {
        for ri in 0..self.robots.len() {
            if !self.robots[ri].alive {
                continue;
            }
            let (rx, ry, rz) = {
                let r = &self.robots[ri];
                (r.pos_x >> 8, r.pos_y >> 8, r.z)
            };
            for slot in 0..self.enemy_bank.len() {
                let kind = self.enemy_bank[slot].kind;
                if !matches!(kind, 0x65 | 0x67 | 0x68) {
                    continue;
                }
                let p = &self.enemy_bank[slot];
                let dx = (p.x >> 8) - rx;
                let dy = (p.y >> 8) - ry;
                let dz = (p.z >> 8) - rz;
                if dx.abs() < 0x10 && dy.abs() < 0x10 && dz.abs() < 0x20 {
                    let dmg = crate::weapon::weapon_damage(kind, self.difficulty);
                    self.projectile_disburser(slot);
                    self.apply_damage(ri, dmg, -1);
                }
            }
        }
    }

    /// FUN_004190bc's modeled core — the WEAPON→CRITTER hit
    /// applier (§7j.23/1..3, mode 2): per present critter — the px
    /// octile box < 0x20 on x/y (the record's own scale per kind),
    /// the z box < 0x20 (0x24 kind 3, 0x40 kind 7); kinds 3..7
    /// immune while mode ∈ {6,7,0xB}. On hit: hp −=
    /// weapon_damage(weapon) (per-WEAPON, kind-independent),
    /// attacker := owner, fuse := 1, kinds 4..7 mode := 5 +
    /// impact := (x<<8, y<<8); death (hp ≤ 0) → the per-kind
    /// handlers (k4 FUN_00418ca4, k5/6 FUN_00418e26 — §7j.24/1..2).
    /// Caller passes px (= Q5 counts) x/y and the weapon's Q5 z.
    pub fn critter_hit_test(&mut self, x: i32, y: i32, z: i32, weapon: u16, owner: i32) {
        for idx in 0..self.critters.len() {
            let c = &self.critters[idx];
            if !c.presence {
                continue;
            }
            if matches!(c.kind, 3..=7) && matches!(c.mode, 6 | 7 | 0xB) {
                continue;
            }
            let (cx, cy) = if c.kind == 4 {
                (c.x, c.y)
            } else {
                (c.x >> 8, c.y >> 8)
            };
            if dist_octagonal(x - cx, y - cy) >= 0x20 {
                continue;
            }
            let zwin = match c.kind {
                3 => 0x24,
                7 => 0x40,
                _ => 0x20,
            };
            if (z - c.z).abs() >= zwin {
                continue;
            }
            let dmg = crate::weapon::weapon_damage(weapon, self.difficulty);
            let (ckind, was_mode, wx, wy, wz) = {
                let c = &mut self.critters[idx];
                c.hp -= dmg as i16;
                c.attacker = owner as i16;
                c.fuse = 1;
                if matches!(c.kind, 4..=7) {
                    c.mode = 5;
                    c.impact_x = x << 8;
                    c.impact_y = y << 8;
                }
                (c.kind, c.mode, c.x, c.y, c.z)
            };
            let _ = was_mode;
            if self.critters[idx].hp <= 0 {
                self.critter_death(idx, ckind, weapon, wx, wy, wz);
            }
        }
    }

    /// The §7j.24 death handlers for the modeled kinds —
    /// k4 FUN_00418ca4 (substeps := 1, hp 0, mode 6 dive,
    /// countdown 6) and k5/6 FUN_00418e26 (substeps := 1, hp 0,
    /// mode 6, anim 0); both stage 1× kind-7 debris at the
    /// critter's tile (delay 1) + the weapon-gated {0x24,0x29,0xC}
    /// extras (3× k7 + the 8/12-row splash — the splash bank rows
    /// are the destroy-view T3 surface) and the BOUNTY gate
    /// (§7j.24/2): attacker ≠ −1 ∧ robots[attacker].kind ==
    /// player_type → score += 75 (k4) / 150 (k5/6) + strip_arm.
    /// The SFX trios are the T4 E-gap.
    pub(crate) fn critter_death(
        &mut self,
        idx: usize,
        kind: u16,
        weapon: u16,
        wx: i32,
        wy: i32,
        wz: i32,
    ) {
        {
            let c = &mut self.critters[idx];
            c.species = 1;
            c.hp = 0;
            c.mode = 6;
            c.death_ctr = 0;
            c.countdown = 0;
            if kind == 4 {
                c.countdown = 6;
            } else {
                c.anim = 0;
            }
        }
        // The debris: kind-4 records are px-scale already; the
        // others shift Q13 → Q5. z is Q5 for every kind.
        let (dx, dy) = if kind == 4 {
            (wx, wy)
        } else {
            (wx >> 8, wy >> 8)
        };
        let _ = self.stage_debris(dx, dy, wz, 7, 1, 0);
        if matches!(weapon, 0x24 | 0x29 | 0xC) {
            for k in 0..3 {
                let jx = dx + ((self.rand_a() & 0x1F) as i32 - 0xF);
                let jy = dy + ((self.rand_a() & 0x1F) as i32 - 0xF);
                let jz = wz + ((self.rand_a() & 0xF) as i32 - 7);
                let _ = self.stage_debris(jx, jy, jz, 7, k + 1, 0);
            }
            // The weapon-gated effect rows [§7j.24/1, /5]: k4 = 8,
            // k5/6 = 12; the args are ((x<<8), (y<<8)) for k4
            // (px→Q13) and the RAW Q13 x/y for k5/6; z :=
            // (z+0x15)·0x100 for both.
            let (rx, ry) = if kind == 4 {
                ((wx << 8), (wy << 8))
            } else {
                (wx, wy)
            };
            let rows = if kind == 4 { 8 } else { 12 };
            self.stage_effect_rows(rx, ry, (wz + 0x15) * 0x100, rows);
        }
        // The bounty gate.
        let attacker = self.critters[idx].attacker;
        if attacker >= 0 {
            let player_type = self.player_type_word();
            if self
                .robots
                .get(attacker as usize)
                .map(|r| r.kind == player_type)
                .unwrap_or(false)
            {
                let bounty = if kind == 4 { 75 } else { 150 };
                self.score_pending += bounty;
                self.strip_arm = true;
            }
        }
    }

    /// The player TYPE word [0x4edb90] (0 in SP — gates the bounty
    /// + the case-4 pickup).
    fn player_type_word(&self) -> u16 {
        self.player_type
    }
}

#[cfg(test)]
mod wanderer_tests {
    //! The kind-1 lane (§7j.71): the S2 loader walk, the substep
    //! machine's squash/pick/walk cycle, the wall probe, the
    //! toward-robot picker, and the suicide trigger's draw budget.

    use super::*;

    /// Flat level-2 world: plane 2 solid type-1 everywhere, height
    /// byte 0x1F everywhere (floor_z == L·0x20+0x1F at every cell —
    /// the wander probe's equality contract holds by construction).
    fn sim_flat(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let angles = crate::mission::AngleTable::from_thresholds(&[0u16; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5; // m = 5 → hp = 200 + 1000/27 = 237
        sim
    }

    /// An .NME hosting exactly one S2 record (w3/w4 = the x/y tile);
    /// every other section empty.
    fn s2_nme(w3: u16, w4: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0u16.to_le_bytes()); // S1 count
        b.extend_from_slice(&1u16.to_le_bytes()); // S2 count
        for w in [1u16, 0, 0, w3, w4] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        for _ in 0..6 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S3..S8 counts
        }
        b
    }

    fn push_wanderer(sim: &mut MissionSim, x: i32, y: i32, countdown: i32, dir: i16) {
        sim.critters.push(CritterRecord {
            kind: 1,
            species: 1,
            hp: 200,
            dir,
            countdown,
            z: 0x5F,
            z_restore: 0x5F,
            x,
            y,
            presence: true,
            ..Default::default()
        });
    }

    #[test]
    fn s2_staging_spawns_and_seeds() {
        let mut sim = sim_flat(0xC0FFEE);
        let staged = sim.stage_critters(&s2_nme(10, 10), 1).expect("S2 staged");
        // difficulty 1 → d+3 = 4 wanderers on the flat level-2 world.
        assert_eq!(staged, 4);
        assert_eq!(sim.critters.len(), 4);
        for c in sim.critters() {
            assert_eq!(c.kind, 1);
            assert_eq!(c.species, 1);
            assert_eq!(c.dir, -1, "the S2 DIR seed");
            assert_eq!(c.frame, 0);
            assert_eq!(c.z, 2 * 0x20 + 0x1F);
            assert_eq!(c.z_restore, c.z);
            assert_eq!(c.x, 10 * 0x20 + 0x10);
            assert_eq!(c.y, 10 * 0x20 + 0x10);
            assert_eq!(c.hp, 200 + 200 * 5 / 27, "hp = 200+(200·m)/27, m=5");
            assert!((10..=19).contains(&c.countdown));
            assert!(c.presence);
        }
    }

    #[test]
    fn s2_staging_draw_budget_one_per_critter() {
        let mut a = sim_flat(7);
        let mut b = sim_flat(7);
        a.stage_critters(&s2_nme(4, 4), 2).expect("staged");
        // d=2 → 5 critters → exactly 5 bounded_pick(10) draws.
        for _ in 0..5 {
            b.bounded_pick(10);
        }
        assert_eq!(a.rand_a_state(), b.rand_a_state());
    }

    #[test]
    fn s2_search_rejects_no_air_above() {
        // A >3 special byte sits at the level directly ABOVE the
        // first standable tile: the scan stops at level 2 (plane-2
        // solid), but the stand gate reads level 3 ≠ 0 → reject.
        let mut sim = sim_flat(0xABCD);
        sim.terrain.dat_write(10, 10, 3, 7);
        let probe = sim_flat(0xABCD);
        let staged = sim.stage_critters(&s2_nme(10, 10), 0).expect("staged");
        assert_eq!(staged, 0, "no air above → no spawn");
        assert_eq!(sim.rand_a_state(), probe.rand_a_state(), "no draws");
    }

    #[test]
    fn s2_search_continues_past_special_lands_below() {
        // The scan SKIPS a >3 tile (level 4) and keeps going: with
        // plane 2 cleared at the tile, it lands on a 1..3 tile at
        // level 1 with air above.
        let mut sim = sim_flat(0xBEEF);
        sim.terrain.dat_write(10, 10, 2, 0);
        sim.terrain.dat_write(10, 10, 4, 7);
        sim.terrain.dat_write(10, 10, 1, 2);
        let staged = sim.stage_critters(&s2_nme(10, 10), 0).expect("staged");
        assert_eq!(staged, 3, "d=0 → 3 spawns at level 1");
        assert_eq!(sim.critters()[0].z, 0x3F);
    }

    /// An .NME hosting `n` S6 records (w1 = probe level, w2/w3 =
    /// x/y tile); every other section empty.
    fn s6_nme(n: u16, w1: u16, w2: u16, w3: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0u16.to_le_bytes()); // S1
        b.extend_from_slice(&0u16.to_le_bytes()); // S2
        b.extend_from_slice(&0u16.to_le_bytes()); // S3
        b.extend_from_slice(&0u16.to_le_bytes()); // S4
        b.extend_from_slice(&0u16.to_le_bytes()); // S5
        b.extend_from_slice(&n.to_le_bytes()); // S6 count
        for _ in 0..n {
            for w in [1u16, w1, w2, w3] {
                b.extend_from_slice(&w.to_le_bytes());
            }
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // S7
        b.extend_from_slice(&0u16.to_le_bytes()); // S8
        b
    }

    #[test]
    fn s6_staging_one_each_draw_free_s3_stamps() {
        // §7j.72/1: ONE per record at EVERY difficulty, ZERO stream
        // draws, the S3 stamps verbatim with the kind word 6.
        for d in [0u32, 3] {
            let mut sim = sim_flat(0x5EED);
            let probe = sim_flat(0x5EED);
            let staged = sim
                .stage_critters(&s6_nme(3, 2, 10, 12), d)
                .expect("S6 staged");
            assert_eq!(staged, 3, "one each at d={d}");
            assert_eq!(sim.critters.len(), 3);
            assert_eq!(sim.rand_a_state(), probe.rand_a_state(), "no draws");
            let expected_z =
                sim.terrain
                    .floor_z((10 * 0x2000 + 0xF00) >> 8, (12 * 0x2000 + 0xF00) >> 8, 2 * 32);
            for c in sim.critters() {
                assert_eq!(c.kind, 6);
                assert_eq!(c.species, 3);
                assert_eq!(c.mode, 8);
                assert_eq!(c.anim, 5);
                assert_eq!(c.heading, 0x72);
                assert_eq!(c.x, 10 * 0x2000 + 0xF00);
                assert_eq!(c.y, 12 * 0x2000 + 0xF00);
                assert_eq!(c.home_x, c.x, "the E S3 home convention (§7j.72/3)");
                assert_eq!(c.home_y, c.y);
                assert_eq!(c.countdown, 0, "w@+0x56 = 0");
                // m = 5 (sim_flat) → hp = 150 + 750/27 = 177.
                assert_eq!(c.hp, 150 + 150 * 5 / 27);
                assert!(c.presence);
                // z = the floor probe at level w1 on the flat world.
                assert_eq!(c.z, expected_z);
            }
        }
    }

    #[test]
    fn s6_mixed_with_modeled_sections_preserves_file_order() {
        // S2 → S3 → S4 → S6 in file order: an .NME with one record
        // each of S2/S3/S4/S6 stages the sections in order (d=1:
        // 4 wanderers, 1..2 kind-5, 2 kind-4, 1 kind-6).
        let mut b = Vec::new();
        b.extend_from_slice(&0u16.to_le_bytes()); // S1
        b.extend_from_slice(&1u16.to_le_bytes()); // S2
        for w in [1u16, 0, 0, 6, 6] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&1u16.to_le_bytes()); // S3
        for w in [1u16, 2, 10, 10] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&1u16.to_le_bytes()); // S4
        for w in [1u16, 2, 10, 10] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // S5
        b.extend_from_slice(&1u16.to_le_bytes()); // S6
        for w in [1u16, 2, 10, 10] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // S7
        b.extend_from_slice(&0u16.to_le_bytes()); // S8
        let mut sim = sim_flat(0x600D);
        let staged = sim.stage_critters(&b, 1).expect("staged");
        let kinds: Vec<u16> = sim.critters().iter().map(|c| c.kind).collect();
        let k5 = kinds.iter().filter(|&&k| k == 5).count();
        assert!((1..=2).contains(&k5), "d=1 → the S3 RandA&1+1 roll");
        let mut expected = vec![1u16; 4]; // S2: d+3 wanderers first
        expected.extend((0..k5).map(|_| 5)); // S3 after S2
        expected.extend([4, 4]); // S4 after S3
        expected.push(6); // S6 last (file order)
        assert_eq!(staged, expected.len());
        assert_eq!(kinds, expected);
    }

    #[test]
    fn s3_s4_hp_scalars_read_linear_m_not_difficulty() {
        // §7j.72/4 (the D179 rider): the S3/S4 hp scalar is the
        // linear mission m — NOT difficulty. d=3, m=5: hp
        // 150+150·5/27 = 177 and 200+200·5/27 = 237 (the difficulty
        // form would say 166/222).
        let mut b = Vec::new();
        b.extend_from_slice(&0u16.to_le_bytes()); // S1
        b.extend_from_slice(&0u16.to_le_bytes()); // S2
        b.extend_from_slice(&1u16.to_le_bytes()); // S3
        for w in [1u16, 2, 10, 10] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&1u16.to_le_bytes()); // S4
        for w in [1u16, 2, 10, 10] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        for _ in 0..4 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S5..S8
        }
        let mut sim = sim_flat(0x7A11);
        sim.stage_critters(&b, 3).expect("staged");
        // d=3 → S3 spawns max(3,1) = 3, S4 spawns (3>>1)+2 = 3.
        let k5: Vec<i16> = sim.critters()[..3].iter().map(|c| c.hp).collect();
        let k4: Vec<i16> = sim.critters()[3..].iter().map(|c| c.hp).collect();
        assert!(k5.iter().all(|&h| h == 177), "kind-5 hp m-scaled (got {k5:?})");
        assert!(k4.iter().all(|&h| h == 237), "kind-4 hp m-scaled (got {k4:?})");
        // Unstaged linear (m = 0 — the S8 scenario's deliberate
        // no-destroy class, §7j.72/4): the base exactly.
        let mut sim0 = sim_flat(0x7A11);
        sim0.linear = 0;
        sim0.stage_critters(&b, 3).expect("staged");
        assert_eq!(sim0.critters()[0].hp, 150, "m=0 → base hp (kind 5)");
        assert_eq!(sim0.critters()[3].hp, 200, "m=0 → base hp (kind 4)");
    }

    #[test]
    fn idle_squash_restores_z_and_burns_no_draws() {
        let mut sim = sim_flat(1);
        push_wanderer(&mut sim, 0x210, 0x210, 5, -1);
        sim.critters[0].z = 0x40; // drifted off the standing level
        let s0 = sim.rand_a_state();
        sim.critter_tick();
        assert_eq!(sim.rand_a_state(), s0, "the squash path is draw-free");
        let c = &sim.critters[0];
        assert_eq!(c.countdown, 1, "squashed to one substep");
        assert_eq!(c.z, 0x5F, "z := z_restore");
        assert_eq!(c.dir, -1);
    }

    #[test]
    fn pick_then_walk_cycle() {
        let mut sim = sim_flat(2);
        push_wanderer(&mut sim, 0x210, 0x210, 1, -1);
        // Tick 1: the PICK (2 or 3 draws, no move).
        sim.critter_tick();
        let c = &sim.critters[0];
        assert!((0..=3).contains(&(c.dir as i32)), "a walk direction");
        assert_eq!(c.frame as i32, c.dir as i32, "the anim mirror");
        assert!((10..=25).contains(&c.countdown));
        assert_eq!(c.x, 0x210);
        assert_eq!(c.y, 0x210);
        // Tick 2: the WALK commits ±6 RAW px on the dir axis (the
        // flat world passes the 8-sample probe everywhere).
        let dir = c.dir as i32;
        let (ex, ey) = match dir {
            0 => (0x210, 0x210 - 6),
            1 => (0x210 + 6, 0x210),
            2 => (0x210, 0x210 + 6),
            _ => (0x210 - 6, 0x210),
        };
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!((c.x, c.y), (ex, ey), "the ±6 stepper commit");
        assert_eq!(c.dir as i32, dir, "still walking");
    }

    #[test]
    fn walk_end_and_repick() {
        let mut sim = sim_flat(3);
        push_wanderer(&mut sim, 0x210, 0x210, 2, 1); // two substeps left
        sim.critter_tick(); // countdown−− → 1 > 0 → the walk step
        assert_eq!(sim.critters[0].x, 0x210 + 6);
        // Next tick: countdown ≤ 0 ∧ dir ≥ 0 → WALK-END (one draw):
        // dir := −1, countdown ∈ 8..15.
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.dir, -1);
        assert!((8..=15).contains(&c.countdown));
    }

    #[test]
    fn blocked_probe_repick_no_z_restore() {
        // A 0xFF deck one tile EAST of the wanderer: the probe's +12
        // x-offset samples read the raw 0xFF (>3) → blocked →
        // re-pick, and z stays wherever it was (NO restore on this
        // path). The wanderer sits at px 5·0x20+0x10 (tile 5); the
        // east step's samples reach tile 6.
        let mut sim = sim_flat(4);
        sim.terrain.dat_write(6, 5, 2, 0xFF);
        push_wanderer(&mut sim, 5 * 0x20 + 0x10, 5 * 0x20 + 0x10, 3, 1);
        sim.critters[0].z = 0x50;
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.x, 5 * 0x20 + 0x10, "the step did not commit");
        assert_eq!(c.dir, -1, "re-picked");
        assert_eq!(c.z, 0x50, "no z-restore on the blocked path");
    }

    #[test]
    fn wander_toward_tie_break_and_axes() {
        let mut sim = sim_flat(5);
        sim.spawn_robot((3, 3, 2)); // robot px ≈ (3·32+15, 3·32+15) = (111, 111)
                                    // Exactly diagonal (dx == dy), critter north-west → y-axis, +y.
        push_wanderer(&mut sim, 111 - 60, 111 - 60, 1, -1);
        assert_eq!(sim.wander_toward(0), 2);
        // x-dominant east → 1; x-dominant west → 3.
        let mut sim2 = sim_flat(5);
        sim2.spawn_robot((3, 3, 2));
        push_wanderer(&mut sim2, 111 - 60, 111, 1, -1);
        assert_eq!(sim2.wander_toward(0), 1);
        push_wanderer(&mut sim2, 111 + 60, 111, 1, -1);
        assert_eq!(sim2.wander_toward(1), 3);
        // y-dominant south → 0 (toward = −y).
        let mut sim3 = sim_flat(5);
        sim3.spawn_robot((3, 3, 2));
        push_wanderer(&mut sim3, 111, 111 + 60, 1, -1);
        assert_eq!(sim3.wander_toward(0), 0);
    }

    #[test]
    fn suicide_trigger_explodes_with_40_draws() {
        let mut a = sim_flat(6);
        let mut b = sim_flat(6);
        a.spawn_robot((3, 3, 2));
        b.spawn_robot((3, 3, 2)); // the spawn's variant draw, in lockstep
        push_wanderer(&mut a, 111, 111, 9, 1); // on top of the robot
        a.critter_tick();
        assert!(!a.critters[0].presence, "deactivated");
        for _ in 0..40 {
            b.rand_a();
        }
        assert_eq!(a.rand_a_state(), b.rand_a_state(), "5 draws × 8");
    }

    #[test]
    fn suicide_trigger_far_robot_wanders_on() {
        let mut sim = sim_flat(8);
        sim.spawn_robot((10, 10, 2)); // robot px (335, 335) — far
        push_wanderer(&mut sim, 111, 111, 5, -1);
        sim.critter_tick();
        assert!(sim.critters[0].presence);
        assert_eq!(sim.critters[0].countdown, 1, "the idle squash ran");
    }
}

#[cfg(test)]
mod debris_physics_tests {
    //! The critter lane of the FUN_0040de9c debris physics pass
    //! (§7j.44/4) — module-internal so the bank can be pushed
    //! directly.

    use super::*;

    fn sim_with_plane0(nonzero: bool) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        if nonzero {
            // Nonzero volume at the probe block rows 4..6, col 4
            // (the debris at tile (5,5) probes rows 4..6 at col 4).
            for row in 4..7 {
                planes[row * 32 + 4] = 3;
            }
        }
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, Vec::new()).unwrap();
        let angles = crate::mission::AngleTable::from_thresholds(&[0u16; 64]).unwrap();
        MissionSim::new(terrain, angles, 0xC0FFEE)
    }

    fn push_critter(sim: &mut MissionSim, kind: u16, mode: u16, hp: i16, x: i32, y: i32) {
        let z = 0x1F;
        sim.critters.push(CritterRecord {
            kind,
            species: 1,
            attacker: 0,
            hp,
            mode,
            anim: 0,
            heading: 0,
            presence: true,
            target_x: 0,
            target_y: 0,
            target_z: 0,
            impact_x: 0,
            impact_y: 0,
            x,
            y,
            z,
            home_x: x,
            home_y: y,
            countdown: 0,
            dir: -1,
            frame: 0,
            z_restore: z,
            death_ctr: 0,
            target_robot: -1,
            fuse: 0,
            facing: 0,
        });
    }

    /// A k12 chunk (phys 6, radius 96) at tile (5,5) center.
    fn stage_k12(sim: &mut MissionSim) {
        assert!(sim.stage_debris(5 * 0x20 + 0x10, 5 * 0x20 + 0x10, 0x20, 12, 0, -1));
    }

    #[test]
    fn terrain_gate_blocks_empty_ground() {
        // ALL-zero volume planes: the critter walk never runs —
        // no damage even point-blank (§7j.44/3).
        let mut sim = sim_with_plane0(false);
        push_critter(
            &mut sim,
            5,
            0xA,
            100,
            (5 * 0x20 + 0x10) << 8,
            (5 * 0x20 + 0x10) << 8,
        );
        stage_k12(&mut sim);
        for _ in 0..6 {
            sim.advance_frame();
        }
        assert_eq!(sim.critters()[0].hp, 100);
    }

    #[test]
    fn crush_damage_mag_and_falloff_gate() {
        // Gate open + point-blank: hp -= 25 per frame (kind 12),
        // six frames. The knock is (0,0) under the zeroed sine
        // table, and the move probe still gates the store.
        let mut sim = sim_with_plane0(true);
        let cx = (5 * 0x20 + 0x10 - 8) << 8;
        let cy = (5 * 0x20 + 0x10) << 8;
        push_critter(&mut sim, 5, 0xA, 500, cx, cy);
        stage_k12(&mut sim);
        for _ in 0..6 {
            sim.advance_frame();
        }
        assert_eq!(sim.critters()[0].hp, 500 - 6 * 25);
        // Falloff gate: at radius 96 the falloff ((96-1)-dist)>>3
        // drops to 2 at dist 71 — no damage beyond that.
        let mut far = sim_with_plane0(true);
        push_critter(
            &mut far,
            5,
            0xA,
            100,
            (5 * 0x20 + 0x10 - 72) << 8,
            (5 * 0x20 + 0x10) << 8,
        );
        stage_k12(&mut far);
        for _ in 0..6 {
            far.advance_frame();
        }
        assert_eq!(far.critters()[0].hp, 100, "falloff <= 2 -> no crush");
    }

    #[test]
    fn crush_mode_and_kind_gates() {
        // Modes 7/6/0xB are skipped by the walk; kind 2/7 by the
        // dispatcher.
        for mode in [7u16, 6, 0xB] {
            let mut sim = sim_with_plane0(true);
            push_critter(
                &mut sim,
                5,
                mode,
                100,
                (5 * 0x20 + 0x10 - 8) << 8,
                (5 * 0x20 + 0x10) << 8,
            );
            stage_k12(&mut sim);
            sim.advance_frame();
            assert_eq!(sim.critters()[0].hp, 100, "mode {mode} skipped");
        }
        let mut sim = sim_with_plane0(true);
        push_critter(
            &mut sim,
            2,
            0xA,
            100,
            (5 * 0x20 + 0x10 - 8) << 8,
            (5 * 0x20 + 0x10) << 8,
        );
        stage_k12(&mut sim);
        sim.advance_frame();
        assert_eq!(sim.critters()[0].hp, 100, "kind 2 guarded");
    }

    #[test]
    fn crush_kill_dispatches_environment_death() {
        // hp 10 point-blank: killed with attacker -1; the k5/6
        // handler stamps mode 6 + species 1 + stages the k7 chunk.
        let mut sim = sim_with_plane0(true);
        push_critter(
            &mut sim,
            5,
            0xA,
            10,
            (5 * 0x20 + 0x10 - 8) << 8,
            (5 * 0x20 + 0x10) << 8,
        );
        stage_k12(&mut sim);
        sim.advance_frame();
        assert!(sim.critters()[0].hp <= 0);
        assert_eq!(sim.critters()[0].attacker, -1);
        assert_eq!(sim.critters()[0].mode, 6, "the death-dive stamp");
        assert!(
            sim.debris_bank().iter().any(|r| r.active && r.kind == 7),
            "the death chunk staged"
        );
    }
}
