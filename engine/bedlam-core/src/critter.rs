//! The critter-actor family (P4.2/W12-S8, DESIGN-DIFFHARNESS §7 S8
//! row + §10-W12; RE-EXW-SIM §7j.42 — every [verified] tag below
//! cites §7j.42's instruction-walk of FUN_00412f34 + the §7j.17/
//! §7j.18/§7j.23/§7j.24/§7j.29 lanes the controller reaches).
//!
//! Scope: the E-side model of the 0x4cff98 critter bank (0x7E
//! stride, count cell 0x46cc2c) + the .NME staging host seam + the
//! controller subset for the CORPUS KINDS — 1 (the wanderers,
//! §7j.71), 2 (the sine-walk shooters, §7j.74), 3 (the chasers,
//! §7j.75), 4 (seek steppers), 5/6 (the shared mixed-AI body;
//! §7j.72 landed the S6 staging — 26 corpus missions host it) and
//! 7 (the close-combat beamers, §7j.76). The kind-8+ bodies and
//! the S8 POI bank are the documented E-gaps: `stage_critters`
//! REFUSES an .NME hosting them (fail loud — never spawn a critter
//! whose brain is missing).
//!
//! Coordinate scales (§7j.23/2 + §7j.42's probe reads + §7j.74/4):
//! x/y are Q13 for kinds 2/3/5/6/7 but RAW px (= Q5 counts) for
//! kinds 1/4; z is Q5 (32/tile) for kinds 1/4/5/6 — the projectile
//! spawn's `(z+0x10)<<8` and the walker's `|rz − pz>>8| < 0x20` box
//! pin it — but Q13 for kind 2 (the S1 stamp 0xC000 = 6 levels,
//! passed through raw to the 0x65 projectile; §7j.74/4).
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
/// The kind-3 mode-3/0xA walk-pattern dword table 0x454b48
/// (§7j.75/6 — raw DGROUP bytes): indexed by the live countdown
/// 1..9 (0 is never read — the aim sets 9 first); step on
/// {2,3,7,8,9} = 6 steps per 10 frames.
const CHASER_WALK_TABLE: [i32; 10] = [0, 0, 1, 1, 0, 0, 0, 1, 1, 1];

/// FUN_00412a19(aim, heading) — the kind-7 STEER helper
/// [§7j.76/3, 0x412a19..0x412a49]: equal → 0; else δ :=
/// wrap8(aim − heading) ∈ [1, 0xFF] (the +0x100/−0x100 wrap pair),
/// and δ ≥ 0x80 → −1 else +1 — the ±1-per-substep turn toward the
/// aim by the shorter arc (the 0x80 tie turns clockwise).
fn closecombat_steer(aim: i32, heading: i32) -> i32 {
    if aim == heading {
        return 0;
    }
    let mut d = aim - heading;
    if d < 0 {
        d += 0x100;
    }
    if d > 0xFF {
        d -= 0x100;
    }
    if d >= 0x80 {
        -1
    } else {
        1
    }
}

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
    /// The wake-heading cell d@+0x14 — the kind-3 preserved spawn
    /// heading (S5's w1<<6; the dormant teleport restores it to
    /// heading 20 frames before waking, §7j.75/2b). NOT
    /// serialized.
    pub spawn_heading: i32,
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
    /// Home z d@+0x4A — S5 is the ONE section that stamps it
    /// (§7j.75/1; the dormant teleport restores z from it); the
    /// other kinds leave it 0. NOT serialized (the §7j.71
    /// dir/frame/z_restore convention).
    pub home_z: i32,
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
    /// The pathfinder wall-follow sector w@+0x5E (kind 3,
    /// §7j.75/4): 0x00 = −y, 0x40 = +x, 0x80 = +y, 0xC0 = −x;
    /// every blocked-path exit copies it into heading. NOT
    /// serialized.
    pub seek_sector: u16,
    /// z-restore d@+0x4E — the kind-1 standing level z, restored on
    /// the idle squash + the bounds re-pick (§7j.71/1,/3).
    pub z_restore: i32,
    /// Dying counter d@+0x52 (mode 7 runs 0x28 frames).
    pub death_ctr: i32,
    /// Variant d@+0x18 — the kind-2 heading PRECESSION rate
    /// (FUN_0041ec1c(4)+3, negated when the record's w2 flag ≠ 0;
    /// §7j.74/1). NOT serialized in the canonical bank blob (the
    /// §7j.71 dir/frame/z_restore convention).
    pub variant: i32,
    /// Target robot w@+0x7A (the mode-2 fire victim; −1 = none).
    pub target_robot: i16,
    /// Fuse / hit-flash w@+0x7C — the main loop decrements it every
    /// frame BEFORE the dispatch (§7j.42/1); a hit sets 1.
    pub fuse: u16,
    /// Facing word w@+0x72 (the idle 1/32 drift writes ±0xF).
    pub facing: u16,
    /// Knock vx w@+0x74 — the kind-7 in-record knock vector (cos of
    /// the away heading >>6, §7j.76/4); integrates ×2/frame in
    /// modes 5/6. NOT serialized.
    pub knock_vx: i32,
    /// Knock vy w@+0x76 — the kind-7 vector's sin half. NOT
    /// serialized.
    pub knock_vy: i32,
    /// Fall-rate ramp w@+0x78 — the kind-7 ballistic mode's
    /// +2/frame gravity counter (capped 0x18, §7j.76/2); z −= it
    /// each mode-6 substep. NOT serialized.
    pub fall_rate: i32,
    /// Sticky nearest-robot scan idx — the controller's [esp+0x2c]
    /// stack cell, which persists across frames (re-read by the
    /// engage tail after a mode-5 flip — §7j.76/2). NOT
    /// serialized.
    pub scan_robot: i32,
    /// Sticky scan dist (px octile) — the [esp+0x28] cell; the
    /// sentinel 10_000_000 before the first default-path substep.
    /// NOT serialized.
    pub scan_dist: i32,
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
            home_z: 0,
            countdown: 0,
            dir: -1,
            frame: 0,
            seek_sector: 0,
            z_restore: 0,
            death_ctr: 0,
            variant: 0,
            target_robot: -1,
            fuse: 0,
            facing: 0,
            spawn_heading: 0,
            knock_vx: 0,
            knock_vy: 0,
            fall_rate: 0,
            scan_robot: 0,
            scan_dist: 10_000_000,
        }
    }
}

impl MissionSim {
    /// Stage the mission's .NME through the FUN_00416458 spawn
    /// schedule (§7j.18) — the `critters = 1` grammar host seam
    /// (D114). The ORIGINAL loads the file natively at mission
    /// load; E stages the identical bytes. Only the sections whose
    /// kinds the E controller models may spawn (S1 → kind 2, S2 →
    /// kind 1, S3 → kind 5, S4 → kind 4, S5 → kind 3, S6 →
    /// kind 6, S7 → kind 7); any other NON-EMPTY section is
    /// REFUSED (fail loud — never spawn a brain the engine does
    /// not carry; ZONEA/MISSION1 hosts exactly S3+S4).
    ///
    /// Spawn schedule [verified §7j.18 + §7j.71/1 + §7j.72 + §7j.74/1]:
    /// S1 (state 2, 10-B recs, w1 = spawn base, w2 = variant flag,
    /// w3/w4 = x/y tile): `w1+d` each clamped ≥ 1 — per attempt two
    /// scatter(5) draws set x/y = (tile + pick − 2)·0x2000 (Q13),
    /// the MAP-BOUNDS gate DROPS out-of-map attempts (2 draws
    /// consumed, no critter); on pass species 1, z 0xC000, heading
    /// 0, anim RandA&7, variant pick(4)+3 (negated when w2 ≠ 0),
    /// hp = 175+(175·m)/27 (the 0x4165db imul site), timer
    /// (RandA&0x1F)−0xF — 5 draws per landed critter. S2
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
    /// base 0xC8. S5 (state 3, 10-B recs, w1 = heading scalar,
    /// w2 = probe level, w3/w4 = x/y tile): ONE each at EVERY
    /// difficulty — x = w3·0x2000+0xF00, y = w4·0x2000+0xF00
    /// (Q13), z = the floor probe at level w2, home x/y/z staged
    /// (the ONE home-stamping section), heading = w1<<6 at +0x10
    /// AND the +0x14 wake-heading cell, species 8 (the spawn
    /// grace), MODE 0, target −1, hp base 0x5DC, NO stream draws
    /// (§7j.75/1). S6 (state 6, 8-B recs): ONE each at EVERY
    /// difficulty — the S3 stamps verbatim with kind 6 (mode 8,
    /// species 3, anim 5, heading 0x72, the w1-level floor probe,
    /// hp base 0x96) and NO stream draws (§7j.72/1). S7 (state 7,
    /// 6-B recs, w1/w2 = x/y tile): the d-cascade count {0→1,
    /// 1→(RandA&1)+1, 2→2, ≥3→1} — the roll is ONE SECTION-LEVEL
    /// draw at d=1 in the asm (§7j.76/1; the engine models it
    /// per-record, the landed-S3 convention — an empty section
    /// draws nothing) — x = w1·0x2000+0xF00, y = w2·0x2000+0xF00
    /// (Q13), z FIXED 0xDF (Q5 by value — no probe, no home
    /// stamps), anim 0, countdown 0, heading =
    /// FUN_0041ec1c(0xFF) (the section's ONLY per-critter draw),
    /// mode 3 (ACTIVE from frame 0), species 1, hp base 0x9C4
    /// (§7j.76/1). **EVERY
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
            if n != 0 && si != 0 && si != 1 && si != 2 && si != 3 && si != 4 && si != 5 && si != 6 {
                return None;
            }
        }
        let d = difficulty as i32;
        // The hp scalar [0x46ae8c] = the linear mission m (§7j.71/1).
        let m = self.linear as i32;
        let mut staged = 0usize;
        for rec in &sections[0] {
            // S1 (§7j.74/1): spawn base w1 + difficulty, clamped ≥ 1
            // (0x4164eb); per attempt TWO scatter(5) draws feed the
            // Q13 tile coords, then the MAP-BOUNDS DROP GATE discards
            // the attempt (draws consumed, NO critter, count not
            // incremented). On pass: anim = RandA&7, variant =
            // pick(4)+3 NEGATED by the w2 flag, hp base 0xAF, timer
            // word (RandA&0x1F)−0xF at +0x72 (a dead stamp for kind 2
            // — the body never reads it; the DRAW is stream-live),
            // z FIXED 0xC000 (Q13 — 6 levels; the kind-2 exception to
            // the record's Q5-z rule, §7j.74/4).
            let mut spawns = d + rec[1] as i32;
            if spawns < 1 {
                spawns = 1;
            }
            for _ in 0..spawns {
                if self.critters.len() >= CRITTER_SLOTS {
                    break;
                }
                let x = (rec[3] as i32 + self.bounded_pick(5) - 2) * 0x2000;
                let y = (rec[4] as i32 + self.bounded_pick(5) - 2) * 0x2000;
                let (w, h) = self.terrain.size();
                if x <= 0 || y <= 0 || x >> 13 >= w || y >> 13 >= h {
                    continue; // dropped — the 2 scatter draws stay consumed
                }
                let anim = (self.rand_a() & 7) as u16;
                let base = 0xaf_i32;
                let hp = base + base * m / 27;
                let variant = self.bounded_pick(4) + 3;
                let timer = ((self.rand_a() & 0x1F) as i32 - 0xF) as u16;
                let variant = if rec[2] != 0 { -variant } else { variant };
                self.critters.push(CritterRecord {
                    kind: 2,
                    species: 1,
                    hp: hp as i16,
                    anim,
                    heading: 0,
                    presence: true,
                    x,
                    y,
                    z: 0xC000,
                    variant,
                    facing: timer,
                    ..Default::default()
                });
                staged += 1;
            }
        }
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
        for rec in &sections[4] {
            // S5 (§7j.75/1): ONE each at EVERY difficulty — no
            // inner spawn loop, NO stream draws (zero RandA/
            // FUN_0041ec1c sites in the block); the ONE section
            // that stamps home (x/y AND z), the spawn heading
            // w1<<6 at BOTH heading d@+0x10 and the wake-heading
            // cell d@+0x14, species 8 (the spawn-grace counter —
            // NOT a substep count for kind 3), MODE 0
            // (awake-idle), target −1, hp base 0x5DC (1500).
            if self.critters.len() >= CRITTER_SLOTS {
                break;
            }
            let x = rec[3] as i32 * 0x2000 + 0xF00;
            let y = rec[4] as i32 * 0x2000 + 0xF00;
            let base = 0x5DCi32;
            let heading = (rec[1] as i32) << 6;
            let z = self.terrain.floor_z(x >> 8, y >> 8, rec[2] as i32 * 32);
            self.critters.push(CritterRecord {
                kind: 3,
                species: 8,
                hp: (base + base * m / 27) as i16, // §7j.75/1 — the m cell, as every section
                mode: 0,
                heading,
                spawn_heading: heading,
                presence: true,
                x,
                y,
                z,
                home_x: x,
                home_y: y,
                home_z: z,
                target_robot: -1,
                ..Default::default()
            });
            staged += 1;
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
        // S7 (§7j.76/1): the d-cascade count {0→1, 1→(RandA&1)+1,
        // 2→2, ≥3→1}; per critter: Q13 tile x/y, FIXED Q5 z 0xDF (NO
        // probe, NO home, NO bounds gate), anim/countdown 0, heading
        // = the ONE per-critter bounded pick(0xFF), mode 3 ACTIVE,
        // species 1 (the substep count), hp base 0x9C4 (2500).
        // ENGINE CONVENTION: the asm computes the cascade — and at
        // d=1 draws the roll — ONCE per SECTION, before the record
        // loop (0x416e36..0x416e80 — even an EMPTY section draws at
        // d=1); the engine models it PER-RECORD as the landed S3
        // does (§7j.72's staging precedent), so an empty S7 consumes
        // nothing and the canonical S8 chain stays byte-identical —
        // the deviation is the recorded S3-family convention.
        for rec in &sections[6] {
            let s7_spawns = match d {
                1 => (self.rand_a() & 1) as i32 + 1,
                2 => 2,
                _ => 1,
            };
            for _ in 0..s7_spawns {
                if self.critters.len() >= CRITTER_SLOTS {
                    break;
                }
                let x = rec[1] as i32 * 0x2000 + 0xF00;
                let y = rec[2] as i32 * 0x2000 + 0xF00;
                let base = 0x9C4i32;
                let heading = self.bounded_pick(0xFF);
                self.critters.push(CritterRecord {
                    kind: 7,
                    species: 1,
                    hp: (base + base * m / 27) as i16, // §7j.76/1 — the m cell, as every section
                    mode: 3,
                    anim: 0,
                    countdown: 0,
                    heading,
                    presence: true,
                    x,
                    y,
                    z: 0xDF,
                    ..Default::default()
                });
                staged += 1;
            }
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
                2 => self.critter_shooter(idx),
                3 => self.critter_chaser(idx, respawn),
                4 => self.critter_state4(idx, respawn),
                5 | 6 => self.critter_mixed(idx, respawn, leash),
                7 => self.critter_closecombat(idx),
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

    /// The kind-2 SINE-WALK SHOOTER body (0x415216..0x415466)
    /// [§7j.74/2]: `species` substeps per frame (≡ 1 for S1 —
    /// nothing re-stamps it). Per substep, in the original's exact
    /// order: anim wrap 0xF; heading precesses by the SIGNED
    /// variant (`(heading+variant)&0xFF` — the w2 flag's negation
    /// turns the curve the other way); the sine walk advances x/y
    /// by `(cos/sin word ·0x14)>>8` Q13 with NO bounds gate, NO
    /// wall probe and NO z change; then TWO always-consumed RandA
    /// gates — the 1/128 SQUAWK pulse (FUN_0043a48e with the
    /// [0x4edffc] voice base, x>>8/y>>8 — the play is the T4
    /// E-gap, the DRAW is stream-live) and the 1/4 fire chance
    /// (RandA&3 == 0 — a per-substep CHANCE, not "every 4th";
    /// corrects §7j.17). The fire arm: bounded-pick a robot SLOT
    /// over [0x46ccbc] (the 12-record 0xA8 bank), skip when the
    /// alive word +0x7C is 0; take the FIRST-FREE 0x4cc654 slot
    /// (FUN_0041286f; full → skip); aim at the robot with a
    /// ±0x1F00 Q13 jitter per axis (two more draws); the range
    /// gate `octile2D(dirx,diry)>>8 < 300−(2−d)·64` (the dz is
    /// DEAD for the gate — FUN_0041ebf8 never reads it); in range
    /// stamp projectile 0x65 at the critter position with the RAW
    /// direction >>5 velocity (NOT octile-normalized — closer
    /// targets fly slower bolts; unlike the mode-2 0x68 lane).
    /// Draw budget: 2 per substep always, 5 on the fire arm.
    fn critter_shooter(&mut self, idx: usize) {
        let species = self.critters[idx].species as i32;
        for _substep in 0..species {
            // anim := (anim+1)&0xF (0x41524e).
            let a = self.critters[idx].anim;
            self.critters[idx].anim = (a + 1) & 0xF;
            // heading := (heading + variant)&0xFF (0x415261).
            let variant = self.critters[idx].variant;
            self.critters[idx].heading = (self.critters[idx].heading + variant) & 0xFF;
            // The sine walk (0x41527a..0x4152ac): pure table reads.
            let heading = self.critters[idx].heading as u16;
            let (x, y) = {
                let c = &self.critters[idx];
                (c.x, c.y)
            };
            if let (Some(cw), Some(sw)) = (
                self.angles.sine_word(heading),
                self.angles
                    .sine_word(((heading as i32 - 0x40) & 0xFF) as u16),
            ) {
                let c = &mut self.critters[idx];
                c.x = x + (((cw as i16 as i32) * 0x14) >> 8);
                c.y = y + (((sw as i16 as i32) * 0x14) >> 8);
            }
            // Gate 1 — the SQUAWK pulse (draw always; play is T4).
            if self.rand_a() & 0x7F == 0 {
                // FUN_0043a48e([0x4edffc], x>>8, y>>8, 0, prio 2) —
                // the SFX family E-gap (module doc); no draws inside.
            }
            // Gate 2 — the 1/4 fire chance (draw always).
            if self.rand_a() & 3 != 0 {
                continue;
            }
            // The robot pick over the count cell (draw).
            let count = self.robots.len() as i32;
            let pick = self.bounded_pick(count) as usize;
            let Some(r) = self.robots.get(pick) else {
                continue;
            };
            let (rx, ry, rz, alive) = (r.pos_x, r.pos_y, r.z, r.alive);
            if !alive {
                continue;
            }
            // FUN_0041286f: the first-free 0x4cc654 slot (no draws).
            let Some(slot) = self.enemy_free_slot() else {
                continue;
            };
            let (x, y, z) = {
                let c = &self.critters[idx];
                (c.x, c.y, c.z)
            };
            // The jittered aim (two draws) — ±31/±32 px Q13 ≈ ±1 tile.
            let jx = (self.rand_a() & 0x3F) as i32 * 0x100 - 0x1F00;
            let dirx = rx - x + jx;
            let jy = (self.rand_a() & 0x3F) as i32 * 0x100 - 0x1F00;
            let diry = ry - y + jy;
            // robot z is Q5 (D123); the kind-2 z is Q13 (§7j.74/4).
            let dirz = (rz << 8) - z;
            // The 2-D octile range gate (0x4153d7): dz is dead here.
            let dist = {
                let d0 = dist_octagonal(dirx, diry);
                if d0 == 0 {
                    1
                } else {
                    d0
                }
            };
            let range = 0x12C - (2 - self.difficulty as i32) * 0x40;
            if dist >> 8 >= range {
                continue;
            }
            // The 0x65 stamp: the RAW direction >>5 velocity.
            self.enemy_bank[slot] = EnemyProjectile {
                kind: 0x65,
                x,
                y,
                z,
                vx: dirx >> 5,
                vy: diry >> 5,
                vz: dirz >> 5,
            };
        }
    }

    /// The kind-3 CHASER body (0x4145c1) [§7j.75/2] — runs ONCE
    /// per frame with NO substep loop (species is NOT a substep
    /// count for kind 3; its three roles: the 8-frame spawn grace
    /// (the R2 gate), the 0x20 return-home walk budget (R1/R4),
    /// and the wake clear — §7j.75/3). The whole chain is
    /// DRAW-FREE (§7j.75/8). Order: the target-liveness flip →
    /// the dormant/dying early exits → the species decrement →
    /// the 4-rule distance ladder → the mode bodies (a ladder
    /// flip runs the new body the SAME frame — the dispatch
    /// re-reads the mode word).
    fn critter_chaser(&mut self, idx: usize, respawn: i32) {
        // (a) Target liveness (0x4145c1): a set target whose robot
        // is dead → awake-idle (mode 8), countdown 0, target −1 —
        // BEFORE the mode dispatch (dormant/dying included).
        let target = self.critters[idx].target_robot;
        if target >= 0 {
            let dead = !self
                .robots
                .get(target as usize)
                .map(|r| r.alive)
                .unwrap_or(false);
            if dead {
                let c = &mut self.critters[idx];
                c.mode = 8;
                c.countdown = 0;
                c.target_robot = -1;
            }
        }
        match self.critters[idx].mode {
            0xB => {
                // (b) DORMANT (0x41460d): the difficulty delay
                // table; the TELEPORT-HOME block on the frame the
                // counter EXACTLY equals delay−0x14 (§7j.75/2b) —
                // heading := the +0x14 spawn heading, x/y/z :=
                // home (the re-materialize 20 frames before the
                // wake).
                if self.critters[idx].countdown < respawn {
                    self.critters[idx].countdown += 1;
                    if self.critters[idx].countdown == respawn - 0x14 {
                        let c = &mut self.critters[idx];
                        c.heading = c.spawn_heading;
                        c.x = c.home_x;
                        c.y = c.home_y;
                        c.z = c.home_z;
                    }
                    return;
                }
                // WAKE (0x414649): hp FLAT 1500 (no m scalar),
                // species cleared (can approach immediately).
                let c = &mut self.critters[idx];
                c.presence = true;
                c.countdown = 0;
                c.species = 0;
                c.mode = 8;
                c.hp = 0x5DC;
                return;
            }
            7 => {
                // (c) DYING (0x4146f3): 0x28 frames → dormant (the
                // death counter is not reset — the k5/6 shape).
                let c = &mut self.critters[idx];
                c.hp = 0;
                c.death_ctr += 1;
                if c.death_ctr >= 0x28 {
                    c.mode = 0xB;
                    c.countdown = 0;
                }
                return;
            }
            _ => {}
        }
        // (d) The species decrement (0x414731) — floor at 0.
        if self.critters[idx].species > 0 {
            self.critters[idx].species -= 1;
        }
        // (e) The distance ladder (0x41474e): ONE nearest-alive
        // probe (FUN_00417c00: idx 0 / dist 10_000_000 sentinel
        // when none) + the home leash (octile on the >>8'd home
        // deltas); four rules IN ORDER, each reading the LIVE
        // mode word.
        let (x, y) = (self.critters[idx].x, self.critters[idx].y);
        let (robot, dist) = self.nearest_robot(x >> 8, y >> 8);
        let leash = dist_octagonal(
            (self.critters[idx].home_x >> 8) - (x >> 8),
            (self.critters[idx].home_y >> 8) - (y >> 8),
        );
        // R1 (0x4147ce): dist > 200 ∧ mode 2 → return home (the
        // 0x20 walk budget stamps SPECIES, not countdown —
        // §7j.75/2e).
        if dist > 0xC8 && self.critters[idx].mode == 2 {
            let c = &mut self.critters[idx];
            c.mode = 0xA;
            c.countdown = 0;
            c.species = 0x20;
            c.target_robot = -1;
        }
        // R2 (0x41481f): species == 0 (the spawn grace) ∧
        // dist < 200 ∧ leash < 400 ∧ mode ∉ {3,2} → approach.
        if self.critters[idx].species == 0
            && dist < 0xC8
            && leash < 0x190
            && self.critters[idx].mode != 3
            && self.critters[idx].mode != 2
        {
            let c = &mut self.critters[idx];
            c.mode = 3;
            c.target_robot = robot as i16;
            c.countdown = 0;
        }
        // R3 (0x41487e): dist < 100 ∧ mode ≠ 2 → attack.
        if dist < 0x64 && self.critters[idx].mode != 2 {
            let c = &mut self.critters[idx];
            c.mode = 2;
            c.target_robot = robot as i16;
            c.countdown = 0;
        }
        // R4 (0x4148c1): leash ≥ 400 ∧ mode ≠ 10 → return home
        // (a mode-3 chaser past the leash flips mid-chase).
        if leash >= 0x190 && self.critters[idx].mode != 0xA {
            let c = &mut self.critters[idx];
            c.mode = 0xA;
            c.countdown = 0;
            c.species = 0x20;
            c.target_robot = -1;
        }
        // (f..i) The mode bodies — the dispatch re-reads the mode.
        match self.critters[idx].mode {
            3 => {
                // APPROACH (0x41492b): re-aim every 9 frames (the
                // 8-sector snap at the LIVE robot position), step
                // on the walk table, countdown−− AFTER the read.
                if self.critters[idx].countdown == 0 {
                    self.critters[idx].countdown = 9;
                    self.critters[idx].heading = self.chaser_aim_robot(idx);
                }
                let cd = self.critters[idx].countdown.clamp(0, 9) as usize;
                if CHASER_WALK_TABLE[cd] != 0 {
                    self.chaser_step(idx);
                }
                self.critters[idx].countdown -= 1;
            }
            2 => {
                // ATTACK (0x4149eb): the 5-frame aim cycle
                // (0→1→2→3→4→0 wrap), then fire EVERY frame while
                // a 0x4cc654 slot is free (§7j.75/2g — the §7j.17
                // ">4 shots" gloss is this wrap, not a gate).
                if self.critters[idx].countdown == 0 {
                    self.critters[idx].heading = self.chaser_aim_robot(idx);
                }
                self.critters[idx].countdown += 1;
                if self.critters[idx].countdown > 3 {
                    self.critters[idx].countdown = 0;
                }
                if let Some(slot) = self.enemy_free_slot() {
                    self.spawn_chaser_projectile(idx, slot);
                }
            }
            0xA => {
                // RETURN-HOME (0x414bbc): the same 9-frame aim
                // cycle at HOME, the same walk table.
                if self.critters[idx].countdown == 0 {
                    self.critters[idx].countdown = 9;
                    self.critters[idx].heading = self.chaser_aim_home(idx);
                }
                let cd = self.critters[idx].countdown.clamp(0, 9) as usize;
                if CHASER_WALK_TABLE[cd] != 0 {
                    self.chaser_step(idx);
                }
                self.critters[idx].countdown -= 1;
            }
            // Modes 0/8 — awake-idle: only the ladder acts.
            _ => {}
        }
    }

    /// The 8-sector snap (§7j.75/2f): `((angle+0xF)&0xFF)>>5&7)<<5`
    /// — the +15 half-sector rounding; headings land on
    /// {0,0x20,…,0xE0}.
    fn chaser_snap(&self, angle: i32) -> i32 {
        ((((angle + 0xF) & 0xFF) >> 5) & 7) << 5
    }

    /// The mode-3/2 re-aim at the LIVE target-robot position
    /// (FUN_00425498 with ebx/ecx = robot x/y, §7j.75/2f). An
    /// unset/gone target keeps the heading (unreachable via the
    /// ladder invariant; defensive).
    fn chaser_aim_robot(&self, idx: usize) -> i32 {
        let t = self.critters[idx].target_robot;
        if t < 0 {
            return self.critters[idx].heading;
        }
        let (Some(c), Some(r)) = (self.critters.get(idx), self.robots.get(t as usize)) else {
            return self.critters[idx].heading;
        };
        self.chaser_snap(self.angles.angle_byte(r.pos_x - c.x, r.pos_y - c.y) as i32)
    }

    /// The mode-0xA re-aim at home (§7j.75/2h).
    fn chaser_aim_home(&self, idx: usize) -> i32 {
        let c = &self.critters[idx];
        self.chaser_snap(self.angles.angle_byte(c.home_x - c.x, c.home_y - c.y) as i32)
    }

    /// The mode-2 fire (§7j.75/2g): projectile 0x67 at the LIVE
    /// robot position with the FULL 3-D octile-normalized
    /// velocity — the 0x68 lane's exact math (§7j.42/7): dist =
    /// max(octile(dx,dy),1), vx/vy = d·0x800/dist, vz =
    /// dz·0x8000/max(octile(dist<<4,dz<<4),1); z stamp
    /// (z+0x10)<<8; no jitter, no range gate (the ladder owns
    /// the bands).
    fn spawn_chaser_projectile(&mut self, idx: usize, slot: usize) {
        let t = self.critters[idx].target_robot;
        if t < 0 {
            return;
        }
        let Some(r) = self.robots.get(t as usize) else {
            return;
        };
        let c = &self.critters[idx];
        let dx = (r.pos_x >> 8) - (c.x >> 8);
        let dy = (r.pos_y >> 8) - (c.y >> 8);
        let dz = (r.z + 4) - (c.z + 0x10);
        let dist = {
            let d0 = dist_octagonal(dx, dy);
            if d0 == 0 {
                1
            } else {
                d0
            }
        };
        let vx = dx * 0x800 / dist;
        let vy = dy * 0x800 / dist;
        let den2 = {
            let d = dist_octagonal(dist * 0x10, dz * 0x10);
            if d == 0 {
                1
            } else {
                d
            }
        };
        let vz = dz * 0x8000 / den2;
        let (x, y, z) = (c.x, c.y, c.z);
        self.enemy_bank[slot] = EnemyProjectile {
            kind: 0x67,
            x,
            y,
            z: (z + 0x10) << 8,
            vx,
            vy,
            vz,
        };
    }

    /// FUN_0041571c — the kind-3 pathfinder step (§7j.75/4). The
    /// open path: x/y += cos/sin(heading)>>5 behind the walk
    /// gate, the sector word := (heading+0x20)&0xC0 (heading
    /// UNCHANGED). Blocked: the WALL-FOLLOW ladder on the sector
    /// word — each arm retries its own ±0x200 Q13 axis move (no
    /// sector write on the keep), then the two perpendicular
    /// candidates in the original's key order (the −y/+y arms key
    /// on the HEADING arg ≥ 0x80; the +x/−x arms key on the sin
    /// component > 0x80); EVERY blocked exit copies sector →
    /// heading (0x415b44). The 8-sample gate + the FUN_0040f277
    /// z-settle tail are the documented no-draw E-gap family
    /// (module doc) — the modeled gate is the landed-kinds
    /// approximation (bounds + the center floor band).
    fn chaser_step(&mut self, idx: usize) {
        let heading = (self.critters[idx].heading & 0xFF) as u16;
        let (dx, dy) = match (
            self.angles.sine_word(heading),
            self.angles
                .sine_word(((heading as i32 - 0x40) & 0xFF) as u16),
        ) {
            (Some(c), Some(s)) => ((c as i16 as i32) >> 5, (s as i16 as i32) >> 5),
            _ => return,
        };
        if self.walk_gate(idx, dx, dy) {
            let c = &mut self.critters[idx];
            c.x += dx;
            c.y += dy;
            c.seek_sector = ((heading as i32 + 0x20) & 0xFF & 0xC0) as u16;
            return; // the open path keeps the aim heading
        }
        // The wall-follow ladder (0x415afa): per arm the KEEP move
        // (own axis, ±0x200 Q13, no sector write) + the two
        // PERPENDICULAR candidates in the original's key order —
        // the −y/+y arms key on the HEADING arg (≥ 0x80 → the −x
        // side first), the +x/−x arms on the SIN component
        // (> 0x80 → +y first) [the two key forms are literal asm].
        let sector = self.critters[idx].seek_sector;
        let h = heading as i32;
        let try_axis = |dx: i32, dy: i32, sector: Option<u16>, sim: &mut Self| -> bool {
            if sim.walk_gate(idx, dx, dy) {
                let c = &mut sim.critters[idx];
                c.x += dx;
                c.y += dy;
                if let Some(s) = sector {
                    c.seek_sector = s;
                }
                true
            } else {
                false
            }
        };
        type LadderMove = (i32, i32, u16);
        let ladder: Option<((i32, i32), LadderMove, LadderMove)> = match sector {
            0x00 => Some((
                (0, -0x200),
                if h >= 0x80 {
                    (-0x200, 0, 0xC0)
                } else {
                    (0x200, 0, 0x40)
                },
                if h >= 0x80 {
                    (0x200, 0, 0x40)
                } else {
                    (-0x200, 0, 0xC0)
                },
            )),
            0x40 => Some((
                (0x200, 0),
                if dy > 0x80 {
                    (0, 0x200, 0x80)
                } else {
                    (0, -0x200, 0x00)
                },
                if dy > 0x80 {
                    (0, -0x200, 0x00)
                } else {
                    (0, 0x200, 0x80)
                },
            )),
            0x80 => Some((
                (0, 0x200),
                if h >= 0x80 {
                    (-0x200, 0, 0xC0)
                } else {
                    (0x200, 0, 0x40)
                },
                if h >= 0x80 {
                    (0x200, 0, 0x40)
                } else {
                    (-0x200, 0, 0xC0)
                },
            )),
            0xC0 => Some((
                (-0x200, 0),
                if dy > 0x80 {
                    (0, 0x200, 0x80)
                } else {
                    (0, -0x200, 0x00)
                },
                if dy > 0x80 {
                    (0, -0x200, 0x00)
                } else {
                    (0, 0x200, 0x80)
                },
            )),
            // Any other sector value: no move (the dispatch gaps).
            _ => None,
        };
        if let Some(((kx, ky), p1, p2)) = ladder {
            if !try_axis(kx, ky, None, self) && !try_axis(p1.0, p1.1, Some(p1.2), self) {
                try_axis(p2.0, p2.1, Some(p2.2), self);
            }
        }
        // The 0x415b44 copy — every blocked exit.
        let s = self.critters[idx].seek_sector as i32;
        self.critters[idx].heading = s;
    }

    /// FUN_0040cc27(idx, dx, dy) — the shared TRY-MOVE gate,
    /// modeled as the documented 8-sample-probe E-gap
    /// approximation (§7j.75/5): map bounds + the center floor
    /// band at the candidate cell. The original probes the
    /// 8-sample footprint against the FIRST corner-z word w@+0x60
    /// with a |Δ| ≤ 4 band and SETTLES z to the center floor on
    /// pass — the landed kinds' model (bounds + ≤ 3 band, no
    /// z-settle); open flat ground passes on both channels.
    fn walk_gate(&mut self, idx: usize, dx: i32, dy: i32) -> bool {
        let (x, y, z) = {
            let c = &self.critters[idx];
            (c.x, c.y, c.z)
        };
        let nx = x + dx;
        let ny = y + dy;
        let (w, h) = self.terrain.size();
        if nx < 0 || ny < 0 || nx >> 13 >= w || ny >> 13 >= h {
            return false;
        }
        let floor = self.terrain.floor_z(nx >> 8, ny >> 8, z);
        (z - floor).abs() <= 3
    }

    /// The kind-7 CLOSE-COMBAT body (0x412f52..0x41367c) [§7j.76/2].
    /// `species` substeps per frame (≡ 1 for S7 — nothing
    /// re-stamps it). Mode machine: 7 dying (the FIFTH frame
    /// despawns — hp 0 + presence 0), 6 ballistic (the in-record
    /// knock triple ×2/frame + the +2/frame fall-rate ramp cap
    /// 0x18, the floor landing test, the 8-debris/5-splash/24-row
    /// landing effects), 5 knock drift (10 frames of the same ×2
    /// drift, then mode 3), and the DEFAULT scan for every other
    /// mode (a dormant k7 is inert). The post-mode tail re-reads
    /// the mode: mode 3 ∧ sticky scan-dist < 0x320 → the engage —
    /// a nonzero countdown only decrements; else the ±1 STEER at
    /// the live scan robot (low-byte-scrubbed critter side), the
    /// cos/sin>>6 move with the ≥1/edge clamps, and the
    /// two-conjunct fire gate (scan-dist < 0x50 ∧ the
    /// (frame+idx) modulo 0x1F/0xF/0x7 by difficulty, ≥3 never)
    /// stamping the stationary z=6 TTL-0x18 beam 0x69 and setting
    /// the 6-frame recharge. The whole approach/fire chain is
    /// DRAW-FREE (§7j.76/5); the 0x69 tick/impact is the
    /// enemy_tick E-gap (§7j.50).
    fn critter_closecombat(&mut self, idx: usize) {
        let species = self.critters[idx].species as i32;
        let mut substep = 0i32;
        while substep < species {
            match self.critters[idx].mode {
                7 => {
                    // DYING (0x413618): countdown++ then > 4 →
                    // hp 0 ∧ presence 0 (the substep loop runs on).
                    self.critters[idx].countdown += 1;
                    if self.critters[idx].countdown > 4 {
                        let c = &mut self.critters[idx];
                        c.hp = 0;
                        c.presence = false;
                    }
                }
                6 => {
                    // BALLISTIC (0x412f99): x/y += knock·2, z −= the
                    // fall rate (which ramps +2 to the 0x18 cap),
                    // the ≥1/edge clamps, the floor probe, the
                    // landing test.
                    let c = &self.critters[idx];
                    let mut nx = c.x + c.knock_vx * 2;
                    let mut ny = c.y + c.knock_vy * 2;
                    let mut nz = c.z - c.fall_rate;
                    let rate = if c.fall_rate < 0x18 {
                        c.fall_rate + 2
                    } else {
                        c.fall_rate
                    };
                    let (w, h) = self.terrain.size();
                    if nx < 1 {
                        nx = 1;
                    }
                    if ny < 1 {
                        ny = 1;
                    }
                    if nx >> 13 >= w {
                        nx = (w << 13) - 1;
                    }
                    if ny >> 13 >= h {
                        ny = (h << 13) - 1;
                    }
                    if nz < 1 {
                        nz = 1;
                    }
                    self.critters[idx].fall_rate = rate;
                    let floor = self.terrain.floor_z(nx >> 8, ny >> 8, nz);
                    if floor < nz && nz != 1 {
                        // The NO-LANDING path (0x413249): write the
                        // clamped triple, stay mode 6.
                        let c = &mut self.critters[idx];
                        c.x = nx;
                        c.y = ny;
                        c.z = nz;
                    } else {
                        // LAND (0x4130c5): z := the post-clamp floor,
                        // mode 7 + countdown 0, then the effects.
                        let z = floor.max(1);
                        {
                            let c = &mut self.critters[idx];
                            c.z = z;
                            c.mode = 7;
                            c.countdown = 0;
                        }
                        let (x, y) = (self.critters[idx].x, self.critters[idx].y);
                        // (a) 8 debris — 3 draws each (kind 6, the
                        // staggered delay = the loop counter).
                        for i in 1..=8 {
                            let jz = z + (self.rand_a() & 0xF) as i32;
                            let jy = (y >> 8) + (self.rand_a() & 0x3F) as i32 - 0x1F;
                            let jx = (x >> 8) + (self.rand_a() & 0x3F) as i32 - 0x1F;
                            let _ = self.stage_debris(jx, jy, jz, 6, i, -1);
                        }
                        // (b) 5 splash tiles — 2 draws each, the
                        // z level clamp (z>>5)+2 ≤ 7, delay = the
                        // counter.
                        let sz = ((z >> 5) + 2).min(7);
                        for i in 1..=5 {
                            let sy = (y >> 13) + (self.rand_a() & 3) as i32 - 2;
                            let sx = (x >> 13) + (self.rand_a() & 3) as i32 - 2;
                            let _ = self.stage_splash(sx, sy, sz, i as u16);
                        }
                        // (c) 24 effect rows (FUN_0041a14f).
                        self.stage_effect_rows(x, y, (z + 0x15) * 0x100, 0x18);
                    }
                }
                5 => {
                    // KNOCK DRIFT (0x413303): countdown++ FIRST;
                    // > 10 → mode 3 + countdown 0 (the tail engage
                    // runs this substep on the STALE scan cells);
                    // else the ×2 drift with the ≥1/edge clamps.
                    self.critters[idx].countdown += 1;
                    if self.critters[idx].countdown > 10 {
                        let c = &mut self.critters[idx];
                        c.mode = 3;
                        c.countdown = 0;
                    } else {
                        let c = &self.critters[idx];
                        let mut nx = c.x + c.knock_vx * 2;
                        let mut ny = c.y + c.knock_vy * 2;
                        let (w, h) = self.terrain.size();
                        if nx < 1 {
                            nx = 1;
                        }
                        if ny < 1 {
                            ny = 1;
                        }
                        if nx >> 13 >= w {
                            nx = (w << 13) - 1;
                        }
                        if ny >> 13 >= h {
                            ny = (h << 13) - 1;
                        }
                        let c = &mut self.critters[idx];
                        c.x = nx;
                        c.y = ny;
                    }
                }
                _ => {
                    // The DEFAULT (0x4133c0): the scan alone — the
                    // sticky cells mirror the original's stack
                    // frame (they survive into later frames).
                    let (x, y) = (self.critters[idx].x, self.critters[idx].y);
                    let (robot, dist) = self.nearest_robot(x >> 8, y >> 8);
                    let c = &mut self.critters[idx];
                    c.scan_robot = robot as i32;
                    c.scan_dist = dist;
                }
            }
            // The post-mode tail (0x4133e7): re-read the mode;
            // engage only mode 3 inside the flat 800-px gate.
            if self.critters[idx].mode == 3 && self.critters[idx].scan_dist < 0x320 {
                if self.critters[idx].countdown != 0 {
                    self.critters[idx].countdown -= 1;
                } else {
                    // (a) AIM + STEER at the LIVE scan robot (the
                    // critter side low-byte-scrubbed).
                    let t = self.critters[idx].scan_robot;
                    if let Some(r) = self.robots.get(t as usize) {
                        let c = &self.critters[idx];
                        let dx = r.pos_x - (c.x & !0xFF);
                        let dy = r.pos_y - (c.y & !0xFF);
                        let aim = self.angles.angle_byte(dx, dy) as i32;
                        let steer = closecombat_steer(aim, c.heading & 0xFF);
                        let heading = (c.heading + steer) & 0xFF;
                        // (b) MOVE — cos/sin>>6, no wall probe.
                        let (mx, my) = match (
                            self.angles.sine_word(heading as u16),
                            self.angles.sine_word(((heading - 0x40) & 0xFF) as u16),
                        ) {
                            (Some(cos), Some(sin)) => {
                                ((cos as i16 as i32) >> 6, (sin as i16 as i32) >> 6)
                            }
                            _ => (0, 0),
                        };
                        let mut nx = c.x + mx;
                        let mut ny = c.y + my;
                        let (w, h) = self.terrain.size();
                        if nx < 1 {
                            nx = 1;
                        }
                        if ny < 1 {
                            ny = 1;
                        }
                        if nx >> 13 >= w {
                            nx = (w << 13) - 1;
                        }
                        if ny >> 13 >= h {
                            ny = (h << 13) - 1;
                        }
                        let c = &mut self.critters[idx];
                        c.heading = heading;
                        c.x = nx;
                        c.y = ny;
                        // (c) THE FIRE GATE: point-blank ∧ the
                        // (frame+idx) modulo by difficulty (≥3
                        // never fires — 0x413575's fall-through).
                        let dist = self.critters[idx].scan_dist;
                        if dist < 0x50 {
                            let phase = self.frame() as i32 + idx as i32;
                            let fire_frame = match self.difficulty {
                                0 => phase & 0x1F == 0,
                                1 => phase & 0xF == 0,
                                2 => phase & 0x7 == 0,
                                _ => false,
                            };
                            if fire_frame {
                                // (d) THE STAMP: the stationary beam
                                // (z LITERAL 6, TTL 0x18, no
                                // velocity) + the 6-frame recharge.
                                if let Some(slot) = self.enemy_free_slot() {
                                    let c = &self.critters[idx];
                                    self.enemy_bank[slot] = EnemyProjectile {
                                        kind: 0x69,
                                        x: c.x,
                                        y: c.y,
                                        z: 6,
                                        vx: 0,
                                        vy: 0,
                                        vz: 0,
                                    };
                                    self.critters[idx].countdown = 6;
                                }
                            }
                        }
                    }
                }
            }
            substep += 1;
        }
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
        let (x, y) = {
            let c = &self.critters[idx];
            (c.x, c.y)
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
        if !self.walk_gate(idx, dx, dy) {
            return;
        }
        let c = &mut self.critters[idx];
        c.x = x + dx;
        c.y = y + dy;
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
                if c.kind == 7 {
                    // The kind-7 knock lane [§7j.76/4]: the impact
                    // pair stages the shooter (as k4/k5/6), then the
                    // AWAY heading + the in-record vector, then mode
                    // 5 + countdown 0 (no juice roll — k7 has none).
                    c.impact_x = x << 8;
                    c.impact_y = y << 8;
                    let away = (self.angles.angle_byte(c.x - (x << 8), c.y - (y << 8)) as i32
                        + 0x80)
                        & 0xFF;
                    let h = away as u16;
                    let (vx, vy) = match (
                        self.angles.sine_word(h),
                        self.angles.sine_word(((h as i32 - 0x40) & 0xFF) as u16),
                    ) {
                        (Some(cos), Some(sin)) => {
                            ((cos as i16 as i32) >> 6, (sin as i16 as i32) >> 6)
                        }
                        _ => (0, 0),
                    };
                    c.heading = away;
                    c.knock_vx = vx;
                    c.knock_vy = vy;
                    c.mode = 5;
                    c.countdown = 0;
                } else if matches!(c.kind, 4..=7) {
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
            let expected_z = sim.terrain.floor_z(
                (10 * 0x2000 + 0xF00) >> 8,
                (12 * 0x2000 + 0xF00) >> 8,
                2 * 32,
            );
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
        assert!(
            k5.iter().all(|&h| h == 177),
            "kind-5 hp m-scaled (got {k5:?})"
        );
        assert!(
            k4.iter().all(|&h| h == 237),
            "kind-4 hp m-scaled (got {k4:?})"
        );
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
            variant: 0,
            home_z: 0,
            spawn_heading: 0,
            seek_sector: 0,
            knock_vx: 0,
            knock_vy: 0,
            fall_rate: 0,
            scan_robot: 0,
            scan_dist: 10_000_000,
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

#[cfg(test)]
mod shooter_tests {
    //! The kind-2 lane (§7j.74): the S1 loader walk (the clamp, the
    //! scatter, the bounds-drop gate, the stamps + the exact draw
    //! budget) and the sine-walk shooter body (the heading
    //! precession, the two always-draw gates, the 1/4 aimed 0x65
    //! fire with the raw direction>>5 velocity).

    use super::*;

    /// Flat level-2 world (as `wanderer_tests::sim_flat`) WITH the
    /// 256-word sine ramp (word[a] = round(sin(a)·32767)) — the
    /// walk table the k2 body reads (FUN_0041eb65/77).
    fn sim_sine(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let mut words = vec![0i16; 256];
        for (a, w) in words.iter_mut().enumerate() {
            *w = ((a as f64 * core::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16;
        }
        let angles = crate::mission::AngleTable::from_sintable_words(&words).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5; // m = 5 → S1 hp = 175 + 875/27 = 207
        sim
    }

    /// An .NME hosting `n` S1 records (w1 = spawn base, w2 = the
    /// variant flag, w3/w4 = x/y tile); every other section empty.
    fn s1_nme(n: u16, w1: u16, w2: u16, w3: u16, w4: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&n.to_le_bytes()); // S1 count
        for _ in 0..n {
            for w in [1u16, w1, w2, w3, w4] {
                b.extend_from_slice(&w.to_le_bytes());
            }
        }
        for _ in 0..7 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S2..S8
        }
        b
    }

    fn push_shooter(sim: &mut MissionSim, x: i32, y: i32, variant: i32) {
        sim.critters.push(CritterRecord {
            kind: 2,
            species: 1,
            hp: 100,
            heading: 0,
            variant,
            z: 0xC000,
            x,
            y,
            presence: true,
            ..Default::default()
        });
    }

    /// A robot staged at Q13 (x, y), floor z Q5, alive — the exact
    /// field initializers of `spawn_robot` minus its one draw (the
    /// draw-budget test needs a draw-free staging).
    fn push_robot(sim: &mut MissionSim, x: i32, y: i32, z: i32, alive: bool) {
        sim.robots.push(crate::mission::Robot {
            pos_x: x,
            pos_y: y,
            z,
            state: 0,
            dir_byte: 0,
            facing: crate::mission::FACING_NONE,
            anim: 0,
            variant: 0,
            probe_z: [z as u16; 8],
            stop_dist: 0,
            target: None,
            alive,
            drop_countdown: 0,
            hp: 5000,
            armor: 0,
            hit_flash: 0,
            alarm: 0,
            alarm_ctr: 0,
            shield: 0,
            shield_charges: 0,
            shield_boost: 0,
            battery: 0,
            armor_pool: 0,
            kind: 0,
            death_flag: 0,
            weapons: [crate::weapon::WeaponSlot::default(); 7],
            weapon_mask: 0,
        });
    }

    #[test]
    fn s1_staging_spawns_and_seeds() {
        // w1 = 2, d = 1 → 3 attempts per record; the central tile of
        // the 32×32 world always passes the bounds gate (jitter −2..2
        // on tile 15 stays in [0,32)).
        let mut sim = sim_sine(0xC0FFEE);
        let staged = sim
            .stage_critters(&s1_nme(2, 2, 0, 15, 15), 1)
            .expect("S1 staged");
        assert_eq!(staged, 6);
        for c in sim.critters() {
            assert_eq!(c.kind, 2);
            assert_eq!(c.species, 1);
            assert_eq!(c.z, 0xC000, "the FIXED Q13 spawn z (6 levels)");
            assert_eq!(c.heading, 0);
            assert!(c.anim <= 7, "anim seed = RandA&7");
            assert!((3..=6).contains(&c.variant), "variant = pick(4)+3 ∈ [3,7)");
            // (RandA&0x1F)−0xF ∈ [−15,+15] stored as the u16 word.
            let timer = c.facing as i16 as i32;
            assert!((-15..=15).contains(&timer), "the +0x72 timer stamp");
            assert_eq!(c.hp, 175 + 175 * 5 / 27, "hp = 175+(175·m)/27, m=5");
            // The jittered tile: (w3 + pick − 2)·0x2000, pick ∈ [0,5).
            assert!((13..=17).contains(&(c.x >> 13)));
            assert!((13..=17).contains(&(c.y >> 13)));
            assert!(c.presence);
        }
    }

    #[test]
    fn s1_w2_flag_negates_the_variant() {
        let mut sim = sim_sine(0xBEEF);
        sim.stage_critters(&s1_nme(1, 1, 1, 15, 15), 0)
            .expect("staged");
        let c = &sim.critters()[0];
        assert!(c.variant <= -3, "flagged record: variant = −(pick(4)+3)");
    }

    #[test]
    fn s1_spawn_count_clamps_to_one() {
        // w1 = 0 ∧ d = 0 → sum 0 → CLAMPED to 1 attempt (§7j.74/1).
        let mut sim = sim_sine(0xABCD);
        let staged = sim
            .stage_critters(&s1_nme(1, 0, 0, 15, 15), 0)
            .expect("staged");
        assert_eq!(staged, 1);
    }

    #[test]
    fn s1_bounds_gate_drops_out_of_map_attempts() {
        // Tile 33 (beyond the 32-tile world): EVERY attempt drops —
        // 0 critters, and exactly 4×2 scatter draws consumed
        // (w1 = 3, d = 1 → 4 attempts).
        let mut sim = sim_sine(0x600D);
        let staged = sim
            .stage_critters(&s1_nme(1, 3, 0, 33, 33), 1)
            .expect("staged");
        assert_eq!(staged, 0, "all attempts out of map → dropped");
        let mut probe = sim_sine(0x600D);
        for _ in 0..4 {
            probe.bounded_pick(5);
            probe.bounded_pick(5);
        }
        assert_eq!(sim.rand_a_state(), probe.rand_a_state());
    }

    #[test]
    fn s1_draw_budget_five_per_landed_critter() {
        // Per landed critter: scatter x, scatter y, anim RandA,
        // variant pick(4), timer RandA = 5 draws.
        let mut a = sim_sine(7);
        let mut b = sim_sine(7);
        let staged = a
            .stage_critters(&s1_nme(1, 1, 0, 15, 15), 1)
            .expect("staged");
        assert_eq!(staged, 2);
        for _ in 0..2 {
            b.bounded_pick(5);
            b.bounded_pick(5);
            b.rand_a();
            b.bounded_pick(4);
            b.rand_a();
        }
        assert_eq!(a.rand_a_state(), b.rand_a_state());
    }

    #[test]
    fn sine_walk_precesses_heading_and_advances() {
        // variant +5: heading 0→5; the walk deltas are (word·0x14)>>8
        // in Q13 with word = the corpus convention sin(a·π/128)·
        // 32767: at heading 5 the x word ≈ 4011 → dx ≈ 313, and the
        // y word at (5−0x40)&0xFF ≈ −cos(5π/128)·32767 → dy ≈ −2539
        // (≈ −0.31 tile south-east drift).
        let mut sim = sim_sine(1);
        push_shooter(&mut sim, 15 * 0x2000, 15 * 0x2000, 5);
        let s0 = sim.rand_a_state();
        sim.critter_tick();
        assert_eq!(sim.critters()[0].heading, 5);
        assert_eq!(sim.critters()[0].anim, 1, "anim := (0+1)&0xF");
        // The two always-draw gates ran (squawk + fire chance).
        assert_ne!(sim.rand_a_state(), s0);
        let x0 = 15 * 0x2000;
        let dx = sim.critters()[0].x - x0;
        let dy = sim.critters()[0].y - x0;
        assert!((280..=340).contains(&dx), "the sine walk dx (got {dx})");
        assert!((-2600..=-2450).contains(&dy), "the sine walk dy (got {dy})");
        // Negative variant walks the heading the other way.
        let mut sim2 = sim_sine(1);
        push_shooter(&mut sim2, 15 * 0x2000, 15 * 0x2000, -3);
        sim2.critter_tick();
        assert_eq!(sim2.critters()[0].heading, 0xFD);
    }

    #[test]
    fn fire_arm_stamps_projectile_65_raw_velocity() {
        // A live robot 5 tiles east, same z-plane (robot z Q5 6·32
        // = 192; critter z Q13 0xC000): dirz = 192<<8 − 0xC000 = 0.
        // Difficulty 2 → range 300 px; the octile dist (5 tiles =
        // 5 px after >>8) passes; the velocity is the RAW
        // direction >>5 (NOT octile-normalized).
        let mut sim = sim_sine(0xFEED);
        sim.difficulty = 2;
        push_robot(&mut sim, 20 * 0x2000, 15 * 0x2000, 192, true);
        push_shooter(&mut sim, 15 * 0x2000, 15 * 0x2000, 0);
        let mut fired = false;
        for _ in 0..64 {
            sim.critter_tick();
            if let Some(slot) = sim.enemy_bank.iter().position(|p| p.kind == 0x65) {
                let p = &sim.enemy_bank[slot];
                let c = &sim.critters()[0];
                assert_eq!(p.x, c.x, "spawned at the critter's CURRENT position");
                assert_eq!(p.y, c.y);
                assert_eq!(p.z, 0xC000, "z passes through RAW (Q13)");
                // The velocity is the RAW direction >>5: dirx =
                // robot.x − p.x + jitter with jitter ∈ [−0x1F00,
                // 0x2000] per axis (the exact §7j.74/2 form).
                let dirx = 20 * 0x2000 - p.x;
                let jx = p.vx * 32 - dirx;
                assert!(
                    (-0x1F00..=0x2000).contains(&jx),
                    "vx = (dirx+jitter)>>5 (jitter {jx:#x})"
                );
                assert_eq!(p.vx * 32 - dirx, jx, "raw >>5, NOT normalized");
                let diry = 15 * 0x2000 - p.y;
                let jy = p.vy * 32 - diry;
                assert!(
                    (-0x1F00..=0x2000).contains(&jy),
                    "vy = (diry+jitter)>>5 (jitter {jy:#x})"
                );
                // dirz = robot.z<<8 − 0xC000 = 0 → vz 0 (no z jitter).
                assert_eq!(p.vz, 0, "same z-plane");
                fired = true;
                break;
            }
        }
        assert!(fired, "the 1/4 gate fired within 64 frames");
    }

    #[test]
    fn fire_arm_range_gate_and_dead_robot_skip() {
        // Difficulty 0 → range 172 px; a robot 60 tiles east
        // (60 px after >>8) is OUT of range → no stamp, ever.
        let mut sim = sim_sine(0xD00D);
        sim.difficulty = 0;
        push_robot(&mut sim, 75 * 0x2000, 15 * 0x2000, 192, true);
        push_shooter(&mut sim, 15 * 0x2000, 15 * 0x2000, 0);
        for _ in 0..128 {
            sim.critter_tick();
        }
        assert!(
            sim.enemy_bank.iter().all(|p| p.kind != 0x65),
            "out of range → never fires"
        );
        // A DEAD robot (alive word +0x7C == 0): the fire arm picks a
        // slot but skips before any stamp.
        let mut sim2 = sim_sine(0xD00D);
        sim2.difficulty = 2;
        push_robot(&mut sim2, 20 * 0x2000, 15 * 0x2000, 192, false);
        push_shooter(&mut sim2, 15 * 0x2000, 15 * 0x2000, 0);
        for _ in 0..128 {
            sim2.critter_tick();
        }
        assert!(sim2.enemy_bank.iter().all(|p| p.kind != 0x65));
    }

    #[test]
    fn fire_arm_consumes_five_draws_on_the_full_arm() {
        // Find a seed whose first five draws are: squawk gate ≠ 0,
        // fire gate &3 == 0 (fires), then pick(1) + two jitters.
        // The frame then consumes EXACTLY those five draws.
        let mut fired_seed = None;
        for seed in 0x1010u64..0x2010 {
            let mut probe = sim_sine(seed);
            let g1 = probe.rand_a() & 0x7F;
            let g2 = probe.rand_a() & 3;
            let _pick = probe.bounded_pick(1);
            let _jx = probe.rand_a();
            let _jy = probe.rand_a();
            if g1 != 0 && g2 == 0 {
                fired_seed = Some(seed);
                break;
            }
        }
        let Some(seed) = fired_seed else {
            panic!("no fire-arm seed found in the window");
        };
        let mut sim = sim_sine(seed);
        sim.difficulty = 2;
        push_robot(&mut sim, 20 * 0x2000, 15 * 0x2000, 192, true);
        push_shooter(&mut sim, 15 * 0x2000, 15 * 0x2000, 0);
        let mut probe = sim_sine(seed);
        // Exactly the frame's five draws (squawk, fire, pick(1),
        // jitter, jitter) — nothing else in the body draws.
        probe.rand_a();
        probe.rand_a();
        probe.bounded_pick(1);
        probe.rand_a();
        probe.rand_a();
        sim.critter_tick();
        assert_eq!(sim.rand_a_state(), probe.rand_a_state());
        assert!(sim.enemy_bank.iter().any(|p| p.kind == 0x65));
    }

    #[test]
    fn presence_gate_keeps_idle_shooter_draw_free() {
        // With the family armed but the shooter ABSENT (presence 0),
        // the controller consumes nothing (the preamble gate).
        let mut sim = sim_sine(3);
        push_shooter(&mut sim, 0, 0, 0);
        sim.critters[0].presence = false;
        let s0 = sim.rand_a_state();
        sim.critter_tick();
        assert_eq!(sim.rand_a_state(), s0, "presence 0 → no draws");
    }
}

#[cfg(test)]
mod chaser_tests {
    //! The kind-3 lane (§7j.75): the S5 loader walk (ONE each,
    //! draw-free, the home stamps, the dual heading stamp) and the
    //! chaser body (the species triple role, the 4-rule distance
    //! ladder, the 8-sector snap aim, the every-frame 0x67 fire,
    //! the walk table, the wall-follow ladder, the dormant
    //! teleport + wake, the dying wrap).

    use super::*;

    fn sim_flat(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let angles = crate::mission::AngleTable::from_thresholds(&[0u16; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5; // m = 5 → S5 hp = 1500 + 7500/27 = 1777
        sim
    }

    fn sim_sine(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let mut words = vec![0i16; 256];
        for (a, w) in words.iter_mut().enumerate() {
            *w = ((a as f64 * core::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16;
        }
        let angles = crate::mission::AngleTable::from_sintable_words(&words).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5;
        sim
    }

    fn push_robot(sim: &mut MissionSim, x: i32, y: i32, z: i32, alive: bool) {
        sim.robots.push(crate::mission::Robot {
            pos_x: x,
            pos_y: y,
            z,
            state: 0,
            dir_byte: 0,
            facing: crate::mission::FACING_NONE,
            anim: 0,
            variant: 0,
            probe_z: [z as u16; 8],
            stop_dist: 0,
            target: None,
            alive,
            drop_countdown: 0,
            hp: 5000,
            armor: 0,
            hit_flash: 0,
            alarm: 0,
            alarm_ctr: 0,
            shield: 0,
            shield_charges: 0,
            shield_boost: 0,
            battery: 0,
            armor_pool: 0,
            kind: 0,
            death_flag: 0,
            weapons: [crate::weapon::WeaponSlot::default(); 7],
            weapon_mask: 0,
        });
    }

    /// An .NME hosting `n` S5 records (w1 = heading scalar, w2 =
    /// probe level, w3/w4 = x/y tile); every other section empty.
    fn s5_nme(n: u16, w1: u16, w2: u16, w3: u16, w4: u16) -> Vec<u8> {
        let mut b = Vec::new();
        for _ in 0..4 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S1..S4
        }
        b.extend_from_slice(&n.to_le_bytes()); // S5 count
        for _ in 0..n {
            for w in [1u16, w1, w2, w3, w4] {
                b.extend_from_slice(&w.to_le_bytes());
            }
        }
        for _ in 0..3 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S6..S8
        }
        b
    }

    fn push_chaser(sim: &mut MissionSim, x: i32, y: i32, z: i32) {
        sim.critters.push(CritterRecord {
            kind: 3,
            species: 0,
            hp: 1500,
            mode: 0,
            x,
            y,
            z,
            home_x: x,
            home_y: y,
            home_z: z,
            heading: 0x40,
            spawn_heading: 0x80,
            presence: true,
            ..Default::default()
        });
    }

    #[test]
    fn s5_staging_one_each_draw_free_with_home_stamps() {
        // §7j.75/1: ONE per record at EVERY difficulty (no spawn
        // loop), ZERO stream draws, home x/y/z staged, the w1<<6
        // heading at BOTH +0x10 and the +0x14 cell, species 8,
        // MODE 0, hp 1500+(1500·m)/27.
        let mut sim = sim_flat(0x5DC);
        let staged = sim
            .stage_critters(&s5_nme(2, 3, 2, 10, 12), 3)
            .expect("S5 staged");
        assert_eq!(staged, 2, "one each — the difficulty cell is dead");
        for c in sim.critters() {
            assert_eq!(c.kind, 3);
            assert_eq!(c.species, 8, "the spawn-grace counter");
            assert_eq!(c.mode, 0, "awake-idle, NOT 8");
            assert_eq!(c.target_robot, -1);
            assert_eq!(c.x, 10 * 0x2000 + 0xF00);
            assert_eq!(c.y, 12 * 0x2000 + 0xF00);
            assert_eq!(c.home_x, c.x, "S5 is the ONE home-stamping section");
            assert_eq!(c.home_y, c.y);
            assert_eq!(c.home_z, c.z, "home z staged too");
            assert_eq!(c.heading, 3 << 6, "w1<<6");
            assert_eq!(c.spawn_heading, 3 << 6, "the +0x14 wake-heading cell");
            assert_eq!(c.hp, (1500 + 1500 * 5 / 27) as i16, "m = 5");
            assert!(c.presence);
            assert_eq!(c.countdown, 0);
        }
        // Draw-free: the stream did not move.
        let mut probe = sim_flat(0x5DC);
        probe
            .stage_critters(&s5_nme(2, 3, 2, 10, 12), 3)
            .expect("staged");
        assert_eq!(sim.rand_a_state(), probe.rand_a_state());
    }

    #[test]
    fn chaser_spawn_grace_then_approach() {
        // R2 is gated on species == 0 (§7j.75/2e): the fresh 8 does
        // 8 frames of awake-idle (mode 0), then the approach fires.
        let mut sim = sim_sine(9);
        // Robot 4 tiles east = 128 px: the 100..200 approach band.
        push_robot(
            &mut sim,
            14 * 0x2000 + 0xF00,
            10 * 0x2000 + 0xF00,
            0x5F,
            true,
        );
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].species = 8;
        for _ in 0..7 {
            sim.critter_tick();
            assert_eq!(sim.critters[0].mode, 0, "the grace");
        }
        sim.critter_tick(); // 8th: species hits 0 → R2 → mode 3
        let c = &sim.critters[0];
        assert_eq!(c.mode, 3);
        assert_eq!(c.target_robot, 0);
        assert_eq!(c.countdown, 8, "aim-set 9, stepped (table[9]), dec → 8");
        assert_eq!(c.heading, 0x40, "the east snap");
        assert_eq!(c.species, 0);
    }

    #[test]
    fn chaser_close_band_fires_067_every_frame() {
        // R3: dist < 100 → mode 2; the body fires 0x67 EVERY frame
        // with the LIVE-robot 3-D octile velocity (§7j.75/2g); the
        // countdown cycles 1,2,3,0 (the 5-frame re-aim cycle).
        let mut sim = sim_sine(11);
        // Robot 2 tiles east = 64 px.
        push_robot(
            &mut sim,
            12 * 0x2000 + 0xF00,
            10 * 0x2000 + 0xF00,
            0x5F,
            true,
        );
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        let mut countdowns = Vec::new();
        for k in 0..5 {
            sim.critter_tick();
            let c = &sim.critters[0];
            assert_eq!(c.mode, 2);
            countdowns.push(c.countdown);
            // exactly k+1 bolts of 0x67 after k+1 frames
            assert_eq!(
                sim.enemy_bank.iter().filter(|p| p.kind == 0x67).count(),
                k + 1
            );
        }
        assert_eq!(countdowns, vec![1, 2, 3, 0, 1], "the aim cycle wrap");
        let p = sim.enemy_bank.iter().find(|p| p.kind == 0x67).unwrap();
        assert_eq!(p.x, 10 * 0x2000 + 0xF00, "the critter x stamp");
        assert_eq!(p.y, 10 * 0x2000 + 0xF00);
        assert_eq!(p.z, (0x5F + 0x10) << 8, "the (z+0x10)<<8 stamp");
        // dx = 64 px east, dist = 64 → vx = 64·0x800/64 = 0x800.
        assert_eq!(p.vx, 0x800);
        assert_eq!(p.vy, 0);
        assert!(p.vz < 0, "the robot center offset: dz = (z+4)−(z+0x10) < 0");
    }

    #[test]
    fn chaser_target_death_flips_idle_before_all_else() {
        // The head check (§7j.75/2a) runs BEFORE the mode dispatch:
        // even a dying chaser whose target died goes awake-idle.
        let mut sim = sim_sine(12);
        push_robot(&mut sim, 30 * 0x2000, 30 * 0x2000, 0x5F, false); // dead, far
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].mode = 7;
        sim.critters[0].death_ctr = 10;
        sim.critters[0].target_robot = 0;
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.mode, 8, "the awake-idle flip");
        assert_eq!(c.target_robot, -1);
        assert_eq!(c.countdown, 0);
    }

    #[test]
    fn chaser_break_and_leash_flip_return_home() {
        // R1 (dist > 200 ∧ mode 2) and R4 (leash ≥ 400) both stamp
        // MODE 10 + species 0x20 (the walk budget) + target −1 +
        // countdown 0 (§7j.75/2e — the 32 is NOT the countdown).
        let mut sim = sim_sine(13);
        // Robot 7 tiles east = 224 px > 200.
        push_robot(
            &mut sim,
            17 * 0x2000 + 0xF00,
            10 * 0x2000 + 0xF00,
            0x5F,
            true,
        );
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].mode = 2;
        sim.critters[0].target_robot = 0;
        sim.critters[0].countdown = 2;
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.mode, 0xA);
        assert_eq!(c.species, 0x20, "the return walk budget");
        assert_eq!(
            c.countdown, 8,
            "aim-at-home fired the same frame: 0 → 9, step, dec"
        );
        assert_eq!(c.target_robot, -1);
        // R4: a mode-3 chaser past the leash flips home mid-chase.
        let mut sim2 = sim_sine(13);
        push_chaser(&mut sim2, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim2.critters[0].x = 25 * 0x2000; // 15 tiles east of home (465 px)
        sim2.critters[0].mode = 3;
        sim2.critters[0].countdown = 5;
        sim2.critter_tick();
        let c = &sim2.critters[0];
        assert_eq!(c.mode, 0xA);
        assert_eq!(c.species, 0x20);
        assert_eq!(c.target_robot, -1);
    }

    #[test]
    fn chaser_walk_cycle_six_steps_per_ten() {
        // The walk table [0,0,1,1,0,0,0,1,1,1] (§7j.75/6): steps on
        // countdown {9,8,7,3,2} — 6 open-path steps of
        // cos(0x40)>>5 = 32767>>5 = 1023 Q13 per 10 frames.
        let mut sim = sim_sine(14);
        push_robot(
            &mut sim,
            14 * 0x2000 + 0xF00,
            10 * 0x2000 + 0xF00,
            0x5F,
            true,
        );
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].mode = 3;
        sim.critters[0].countdown = 9;
        sim.critters[0].target_robot = 0;
        let x0 = sim.critters[0].x;
        for _ in 0..10 {
            sim.critter_tick();
            assert_eq!(sim.critters[0].mode, 3, "the 100..200 band holds");
        }
        assert_eq!(sim.critters[0].x - x0, 6 * 1023, "6 steps of 1023");
        assert_eq!(sim.critters[0].countdown, 8, "the cycle position");
        assert_eq!(sim.critters[0].heading, 0x40, "the east snap holds");
    }

    #[test]
    fn chaser_blocked_path_wall_follow_ladder() {
        // §7j.75/4: the open step west runs off the map edge (the
        // bounds gate) → the wall-follow ladder: sector 0xC0's keep
        // (−0x200, 0) is ALSO out of bounds → the perpendicular −y
        // candidate passes (dy == 0 ≤ 0x80 → −y first), the sector
        // word := 0x00, and heading := the sector (the 0x415b44
        // copy).
        let mut sim = sim_sine(15);
        push_chaser(&mut sim, 0x10, 10 * 0x2000 + 0xF00, 0x5F); // tile 0: west is out
        sim.critters[0].mode = 3;
        sim.critters[0].countdown = 9;
        sim.critters[0].heading = 0xC0; // west
        sim.critters[0].seek_sector = 0xC0;
        let (x0, y0) = (sim.critters[0].x, sim.critters[0].y);
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.x, x0, "no west move");
        assert_eq!(c.y, y0 - 0x200, "the −y perpendicular");
        assert_eq!(c.seek_sector, 0x00);
        assert_eq!(c.heading, 0x00, "heading := the sector on the blocked path");
    }

    #[test]
    fn chaser_open_path_updates_sector_not_heading() {
        // §7j.75/4: the open path stamps the sector word
        // (heading+0x20)&0xC0 and KEEPS the aim heading.
        let mut sim = sim_sine(16);
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].mode = 3;
        sim.critters[0].countdown = 9;
        sim.critters[0].heading = 0x40;
        sim.critters[0].seek_sector = 0xC0;
        sim.critters[0].x += 1023; // keep off the aim path's start
        let x0 = sim.critters[0].x;
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.x, x0 + 1023, "the open-path step");
        assert_eq!(c.seek_sector, 0x40, "(0x40+0x20)&0xC0");
        assert_eq!(c.heading, 0x40, "the aim is kept");
    }

    #[test]
    fn chaser_dormant_teleport_then_wake() {
        // §7j.75/2b: at EXACTLY delay−0x14 the dormant chaser
        // teleports home (heading := the +0x14 spawn heading); at
        // delay it wakes with hp FLAT 1500 and species cleared.
        let mut sim = sim_sine(17);
        sim.difficulty = 2; // respawn delay 600
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        let (hx, hy, hz) = (
            sim.critters[0].home_x,
            sim.critters[0].home_y,
            sim.critters[0].home_z,
        );
        let c = &mut sim.critters[0];
        c.mode = 0xB;
        c.countdown = 0;
        c.x = 20 * 0x2000; // displaced
        c.y = 20 * 0x2000;
        c.z = 0x10;
        c.heading = 0x00;
        for k in 1..=580 {
            sim.critter_tick();
            assert_eq!(sim.critters[0].countdown, k);
        }
        let c = &sim.critters[0];
        assert_eq!((c.x, c.y, c.z), (hx, hy, hz), "the teleport at delay−20");
        assert_eq!(c.heading, 0x80, "the spawn-heading restore");
        assert_eq!(c.mode, 0xB, "still dormant");
        for _ in 581..=601 {
            sim.critter_tick();
        }
        let c = &sim.critters[0];
        assert_eq!(c.mode, 8, "the wake");
        assert_eq!(c.hp, 0x5DC, "FLAT 1500 — no m scalar on wake");
        assert_eq!(c.species, 0);
        assert_eq!(c.countdown, 0);
    }

    #[test]
    fn chaser_dying_40_frames_then_dormant() {
        let mut sim = sim_sine(18);
        push_chaser(&mut sim, 10 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F);
        sim.critters[0].mode = 7;
        for _ in 0..39 {
            sim.critter_tick();
            assert_eq!(sim.critters[0].mode, 7);
            assert_eq!(sim.critters[0].hp, 0);
        }
        sim.critter_tick(); // 40th
        assert_eq!(sim.critters[0].mode, 0xB);
        assert_eq!(sim.critters[0].countdown, 0);
    }

    #[test]
    fn chaser_whole_chain_draw_free() {
        // §7j.75/8: the whole k3 chain consumes ZERO stream draws —
        // staging, ladder, walk, fire, dormancy alike.
        let mut a = sim_sine(19);
        a.stage_critters(&s5_nme(1, 3, 2, 10, 10), 2)
            .expect("staged");
        push_robot(&mut a, 12 * 0x2000 + 0xF00, 10 * 0x2000 + 0xF00, 0x5F, true);
        a.critters[0].species = 0; // skip the grace
        let s0 = a.rand_a_state();
        for _ in 0..20 {
            a.critter_tick();
        }
        assert_eq!(a.rand_a_state(), s0, "the k3 chain is draw-free");
        assert!(
            a.enemy_bank.iter().any(|p| p.kind == 0x67),
            "the chaser engaged"
        );
    }
}

#[cfg(test)]
mod closecombat_tests {
    //! The kind-7 lane (§7j.76): the S7 loader walk (the d-cascade
    //! count, the FIXED z, the one heading draw) and the k7 body
    //! (the steer-aim-move engage, the two-conjunct 0x69 fire with
    //! the 6-frame recharge, the knock drift, the ballistic landing
    //! machine, the 5-frame dying despawn, the stale-scan flip).

    use super::*;

    fn sim_flat(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let angles = crate::mission::AngleTable::from_thresholds(&[0u16; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5; // m = 5 → S7 hp = 2500 + 12500/27 = 2962
        sim
    }

    fn sim_sine(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let mut words = vec![0i16; 256];
        for (a, w) in words.iter_mut().enumerate() {
            *w = ((a as f64 * core::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16;
        }
        let angles = crate::mission::AngleTable::from_sintable_words(&words).unwrap();
        let mut sim = MissionSim::new(terrain, angles, seed);
        sim.linear = 5;
        sim
    }

    fn push_robot(sim: &mut MissionSim, x: i32, y: i32, z: i32, alive: bool) {
        sim.robots.push(crate::mission::Robot {
            pos_x: x,
            pos_y: y,
            z,
            state: 0,
            dir_byte: 0,
            facing: crate::mission::FACING_NONE,
            anim: 0,
            variant: 0,
            probe_z: [z as u16; 8],
            stop_dist: 0,
            target: None,
            alive,
            armor: 0,
            hit_flash: 0,
            alarm: 0,
            alarm_ctr: 0,
            shield: 0,
            shield_charges: 0,
            shield_boost: 0,
            battery: 0,
            armor_pool: 0,
            kind: 0,
            death_flag: 0,
            hp: 5000,
            weapons: [crate::weapon::WeaponSlot::default(); 7],
            weapon_mask: 0,
            drop_countdown: 0,
        });
    }

    /// An .NME hosting `n` S7 records (w1/w2 = x/y tile); every
    /// other section empty.
    fn s7_nme(n: u16, w1: u16, w2: u16) -> Vec<u8> {
        let mut b = Vec::new();
        for _ in 0..6 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S1..S6
        }
        b.extend_from_slice(&n.to_le_bytes()); // S7 count
        for _ in 0..n {
            for w in [1u16, w1, w2] {
                b.extend_from_slice(&w.to_le_bytes());
            }
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // S8
        b
    }

    fn push_closecombat(sim: &mut MissionSim, x: i32, y: i32, z: i32) {
        sim.critters.push(CritterRecord {
            kind: 7,
            species: 1,
            hp: 2500,
            mode: 3,
            x,
            y,
            z,
            heading: 0,
            presence: true,
            ..Default::default()
        });
    }

    #[test]
    fn s7_staging_cascade_counts_and_stamps() {
        // §7j.76/1: the d-cascade {0→1, 1→1+RandA&1, 2→2, ≥3→1} —
        // NOT max(d,1) — with the FIXED Q5 z 0xDF, mode 3 ACTIVE,
        // species 1, hp 2500+(2500·m)/27, heading = the bounded
        // pick(0xFF) ∈ [0, 0xFE], anim/countdown 0, NO home.
        let mut sim = sim_flat(0x9C4);
        let staged = sim
            .stage_critters(&s7_nme(2, 10, 12), 2)
            .expect("S7 staged");
        assert_eq!(staged, 4, "d=2 → TWO each");
        assert_eq!(sim.critters.len(), 4);
        for c in sim.critters() {
            assert_eq!(c.kind, 7);
            assert_eq!(c.species, 1, "the substep count");
            assert_eq!(c.mode, 3, "ACTIVE approach — never dormant");
            assert_eq!(c.anim, 0);
            assert_eq!(c.countdown, 0);
            assert_eq!(c.x, 10 * 0x2000 + 0xF00);
            assert_eq!(c.y, 12 * 0x2000 + 0xF00);
            assert_eq!(c.z, 0xDF, "the FIXED level-6-top constant");
            assert_eq!(c.hp, (2500 + 2500 * 5 / 27) as i16, "m=5");
            assert!((0..=0xFE).contains(&c.heading), "bounded pick(0xFF)");
            assert_eq!(c.home_x, 0, "S7 stages NO home");
            assert!(c.presence);
        }
        // d=0 and d≥3 → ONE each.
        let mut sim = sim_flat(0x9C5);
        assert_eq!(sim.stage_critters(&s7_nme(1, 8, 8), 0), Some(1));
        let mut sim = sim_flat(0x9C6);
        assert_eq!(sim.stage_critters(&s7_nme(1, 8, 8), 4), Some(1));
    }

    #[test]
    fn s7_staging_draw_budget_roll_plus_heading() {
        // §7j.76/1: at d=1 the section consumes the ONE count roll
        // (RandA&1) then ONE heading draw per critter — in order.
        let mut a = sim_flat(0xBEA);
        let mut b = sim_flat(0xBEA);
        a.stage_critters(&s7_nme(1, 10, 10), 1).expect("staged");
        let roll = (b.rand_a() & 1) as i32 + 1;
        for _ in 0..roll {
            b.bounded_pick(0xFF);
        }
        assert_eq!(a.rand_a_state(), b.rand_a_state());
        assert_eq!(a.critters.len(), roll as usize);
        // At d≠1 NO roll draw: d=0 → exactly one heading draw.
        let mut a = sim_flat(0xBEB);
        let mut b = sim_flat(0xBEB);
        a.stage_critters(&s7_nme(1, 10, 10), 0).expect("staged");
        b.bounded_pick(0xFF);
        assert_eq!(a.rand_a_state(), b.rand_a_state());
    }

    #[test]
    fn s7_engage_steers_moves_and_fires_the_beam() {
        // §7j.76/2: the engage — steer ±1 toward the aim, the
        // cos/sin>>6 move, and at point-blank (<0x50) on a fire
        // frame the stationary 0x69 stamp {z LITERAL 6} + the
        // 6-frame recharge; recharging frames only decrement.
        let mut sim = sim_sine(0x69);
        sim.set_difficulty(2);
        // A robot 0x30 px east of the critter (Q13 pos).
        push_robot(
            &mut sim,
            10 * 0x2000 + 0x30 * 0x100,
            10 * 0x2000,
            0x5F,
            true,
        );
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x5F);
        let (x0, y0, h0) = (
            sim.critters[0].x,
            sim.critters[0].y,
            sim.critters[0].heading,
        );
        sim.critter_tick();
        let c = &sim.critters[0];
        // Aim east (0x40): heading steers +1 from 0.
        assert_eq!(c.heading, (h0 + 1) & 0xFF, "the ±1 steer toward the aim");
        // The move — the SAME table pair the engine reads (the
        // test's sine-indexed table swaps the cos/sin roles).
        let expected_dx = ((sim.angles.sine_word(1).unwrap() as i16) >> 6) as i32;
        let expected_dy = ((sim.angles.sine_word(0xC1).unwrap() as i16) >> 6) as i32;
        assert_eq!(c.x, x0 + expected_dx, "the eb65>>6 step");
        assert_eq!(c.y, y0 + expected_dy, "the eb77>>6 step");
        // phase = frame(0) + idx(0) = 0 → &7 == 0 at d=2 → FIRE.
        let p = &sim.enemy_bank[0];
        assert_eq!(p.kind, 0x69, "the BEAM");
        assert_eq!(p.x, c.x, "the post-move Q13 x");
        assert_eq!(p.y, c.y);
        assert_eq!(p.z, 6, "the LITERAL z — NOT Q13");
        assert_eq!((p.vx, p.vy, p.vz), (0, 0, 0), "the beam is stationary");
        assert_eq!(c.countdown, 6, "the fire recharge");
        // Recharge: 6 frames of countdown-only (no second beam —
        // the bank holds exactly one 0x69).
        for _ in 0..6 {
            sim.critter_tick();
        }
        assert_eq!(sim.enemy_bank.iter().filter(|p| p.kind == 0x69).count(), 1);
        assert_eq!(sim.critters[0].countdown, 0);
    }

    #[test]
    fn s7_fire_phase_modulo_and_the_d3_never_arm() {
        // §7j.76/2c: the gate keys on (frame + idx) — idx 1 at
        // phase 1 fires at NO difficulty 0..2; d≥3 never fires.
        let mut sim = sim_sine(0x6A);
        push_robot(&mut sim, 0x2000 + 0x30 * 0x100, 0x2000, 0x5F, true);
        push_closecombat(&mut sim, 0, 0, 0x5F); // idx 0 dummy
        push_closecombat(&mut sim, 0x2000, 0x2000, 0x5F); // idx 1
        sim.set_difficulty(2);
        for _ in 0..3 {
            sim.critter_tick();
        }
        assert!(
            !sim.enemy_bank.iter().any(|p| p.kind == 0x69),
            "phase 1 &7 ≠ 0 — no fire"
        );
        // The ≥3 arm: nothing ever fires (0x413575's fall-through).
        let mut sim = sim_sine(0x6B);
        push_robot(&mut sim, 0x2000 + 0x30 * 0x100, 0x2000, 0x5F, true);
        push_closecombat(&mut sim, 0x2000, 0x2000, 0x5F);
        sim.set_difficulty(3);
        for _ in 0..24 {
            sim.critter_tick();
        }
        assert!(!sim.enemy_bank.iter().any(|p| p.kind == 0x69));
        assert_eq!(sim.critters[0].countdown, 0, "no recharge staged either");
    }

    #[test]
    fn s7_engage_chain_draw_free() {
        // §7j.76/5: the approach/move/fire chain consumes ZERO
        // stream draws (the S7 staging alone draws).
        let mut sim = sim_sine(0x6C);
        sim.set_difficulty(0);
        push_robot(
            &mut sim,
            10 * 0x2000 + 0x30 * 0x100,
            10 * 0x2000,
            0x5F,
            true,
        );
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x5F);
        let s0 = sim.rand_a_state();
        for _ in 0..24 {
            sim.critter_tick();
        }
        assert_eq!(sim.rand_a_state(), s0, "the k7 engage chain is draw-free");
        assert!(sim.enemy_bank.iter().any(|p| p.kind == 0x69));
    }

    #[test]
    fn s7_knock_lane_away_heading_inrecord_vector_mode5() {
        // §7j.76/4: the weapon hit stamps the impact pair, the AWAY
        // heading, the in-record vx/vy = cos/sin>>6, mode 5 +
        // countdown 0 — then 10 drift frames at ×2 and the flip to
        // mode 3.
        let mut sim = sim_sine(0x6D);
        // The shooter 0x10 px WEST of the critter: aim(crit −
        // shooter) = east 0x40 → the away heading = WEST 0xC0.
        let cx = 10 * 0x2000;
        let cy = 10 * 0x2000;
        push_closecombat(&mut sim, cx, cy, 0xDF);
        let sx = (cx >> 8) - 0x10;
        let sy = cy >> 8;
        sim.critter_hit_test(sx, sy, 0xDF, 0x1, 0);
        let c = &sim.critters[0];
        assert!(c.hp > 0, "the hit is non-lethal (hp 2500)");
        assert_eq!(c.mode, 5, "the knock drift");
        assert_eq!(c.countdown, 0);
        // The away heading = angle(crit − shooter) + 0x80 — west on
        // the test table (the east aim lands 0x3F, one shy of the
        // 0x40 cardinal — the threshold resolution).
        let aim = sim.angles.angle_byte(cx - (sx << 8), cy - (sy << 8)) as i32;
        let away = (aim + 0x80) & 0xFF;
        assert_eq!(c.heading, away, "the AWAY heading");
        assert!((0x80..=0xC0).contains(&away), "pointing west");
        assert!(c.knock_vx < 0, "knocked west");
        assert!(c.knock_vy >= 0, "the test-table's south lean");
        assert_eq!(c.impact_x, sx << 8);
        assert_eq!(c.impact_y, sy << 8);
        let (x0, vx) = (c.x, c.knock_vx);
        // Drift frame: x += vx·2, countdown 1.
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.countdown, 1);
        assert_eq!(c.x, x0 + vx * 2);
        // TEN drift frames (countdown 1..10), the 11th flips.
        for _ in 0..10 {
            sim.critter_tick();
        }
        assert_eq!(sim.critters[0].mode, 3, "back to approach");
        assert_eq!(sim.critters[0].countdown, 0);
    }

    #[test]
    fn s7_stale_scan_cells_after_the_drift_flip() {
        // §7j.76/2: the mode-5→3 flip does NOT rescan — the tail
        // engages against the STICKY scan cells (the original's
        // stack-frame leftover); the sentinel dist means no engage.
        let mut sim = sim_sine(0x6E);
        sim.set_difficulty(2);
        push_robot(
            &mut sim,
            10 * 0x2000 + 0x30 * 0x100,
            10 * 0x2000,
            0x5F,
            true,
        );
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x5F);
        sim.critters[0].mode = 5;
        sim.critters[0].countdown = 10; // flips to 3 THIS tick
        sim.critters[0].scan_dist = 10_000_000; // never scanned
        sim.critter_tick();
        assert_eq!(sim.critters[0].mode, 3);
        assert!(
            !sim.enemy_bank.iter().any(|p| p.kind == 0x69),
            "the stale sentinel skips the engage"
        );
        // With a STALE close scan cell (a leftover from an earlier
        // frame): the flip substep fires on the stale dist.
        let mut sim = sim_sine(0x6F);
        sim.set_difficulty(2);
        push_robot(
            &mut sim,
            10 * 0x2000 + 0x30 * 0x100,
            10 * 0x2000,
            0x5F,
            true,
        );
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x5F);
        sim.critters[0].mode = 5;
        sim.critters[0].countdown = 10;
        sim.critters[0].scan_robot = 0;
        sim.critters[0].scan_dist = 0x30;
        sim.critter_tick();
        assert!(sim.enemy_bank.iter().any(|p| p.kind == 0x69));
    }

    #[test]
    fn s7_ballistic_landing_machine() {
        // §7j.76/2: mode 6 — z −= the fall rate (the +2 ramp capped
        // 0x18), the ×2 knock drift, the floor landing test, the
        // landing effects (8 debris + 5 splash + 24 effect rows),
        // then mode 7.
        let mut sim = sim_sine(0x70);
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x100);
        let c = &mut sim.critters[0];
        c.mode = 6;
        c.fall_rate = 0x18;
        c.knock_vx = 4;
        c.knock_vy = -2;
        let (x0, y0) = (c.x, c.y);
        sim.critter_tick();
        let c = &sim.critters[0];
        // The flat-world floor at every cell = 2·0x20+0x1F = 0x5F;
        // z 0x100 − 0x18 = 0xE8 > floor → the NO-LANDING path.
        assert_eq!(c.mode, 6, "still falling");
        assert_eq!(c.z, 0x100 - 0x18);
        assert_eq!(c.x, x0 + 8, "the ×2 knock drift");
        assert_eq!(c.y, y0 - 4);
        assert_eq!(c.fall_rate, 0x18, "the ramp cap");
        // Fall to the floor: 0xE8 − 4·0x18 = 0x78 > 0x5F (no land),
        // 0x60 > 0x5F (no land), 0x70 − 0x18 = 0x58 ≤ 0x5F → LAND.
        for _ in 0..5 {
            sim.critter_tick();
        }
        assert_eq!(sim.critters[0].z, 0x70);
        assert_eq!(sim.critters[0].mode, 6);
        sim.critter_tick();
        let c = &sim.critters[0];
        assert_eq!(c.mode, 7, "the landing flips to dying");
        assert_eq!(c.z, 0x5F, "settled on the floor");
        assert_eq!(c.countdown, 0);
        // The landing effects — verified by the DRAW BUDGET
        // (§7j.76/2,5): 8 debris × 3 draws + 5 splash × 2 (the
        // stagers themselves draw nothing for kind 6/splash) + 24
        // rows × 3 draws + 16 overflow-id picks (rows 8..23) = 122.
        assert_eq!(sim.debris.iter().filter(|d| d.active).count(), 8);
        assert_eq!(sim.splashes.iter().filter(|s| s.age != 0).count(), 5);
        let mut b = sim_sine(0x70);
        for _ in 0..122 {
            b.rand_a();
        }
        assert_eq!(
            sim.rand_a_state(),
            b.rand_a_state(),
            "8·3 + 5·2 + 24·3 + 16 — the landing's whole draw budget"
        );
    }

    #[test]
    fn s7_dying_despawns_on_the_fifth_frame() {
        // §7j.76/2: mode 7 — countdown++ and > 4 → hp 0 ∧ presence
        // 0 (the FIFTH dying frame).
        let mut sim = sim_flat(0x71);
        push_closecombat(&mut sim, 0x2000, 0x2000, 0x5F);
        sim.critters[0].mode = 7;
        for _ in 0..4 {
            sim.critter_tick();
            assert!(sim.critters[0].presence, "frames 1..4 hold");
        }
        sim.critter_tick();
        let c = &sim.critters[0];
        assert!(!c.presence, "the fifth frame despawns");
        assert_eq!(c.hp, 0);
        assert_eq!(c.countdown, 5);
    }

    #[test]
    fn s7_dormant_is_inert() {
        // §7j.76/2: every mode but 3/5/6/7 has NO body — a dormant
        // (0xB) k7 only runs the scan (no stamps, no draws).
        let mut sim = sim_sine(0x72);
        push_robot(
            &mut sim,
            10 * 0x2000 + 0x30 * 0x100,
            10 * 0x2000,
            0x5F,
            true,
        );
        push_closecombat(&mut sim, 10 * 0x2000, 10 * 0x2000, 0x5F);
        sim.critters[0].mode = 0xB;
        let s0 = sim.rand_a_state();
        let (x0, y0) = (sim.critters[0].x, sim.critters[0].y);
        for _ in 0..8 {
            sim.critter_tick();
        }
        let c = &sim.critters[0];
        assert_eq!((c.x, c.y), (x0, y0), "inert");
        assert_eq!(c.mode, 0xB);
        assert_eq!(sim.rand_a_state(), s0, "draw-free");
        assert_eq!(c.scan_dist, 0x30, "the scan still runs (sticky cells)");
    }

    #[test]
    fn steer_is_the_shortest_arc_pm1() {
        // §7j.76/3: FUN_00412a19 — equal → 0; wrap the delta into
        // [1, 0xFF]; ≥ 0x80 → −1 else +1.
        assert_eq!(closecombat_steer(0x40, 0x40), 0);
        assert_eq!(closecombat_steer(0x41, 0x40), 1);
        assert_eq!(closecombat_steer(0xBF, 0x40), 1, "0x7F short arc = +1");
        assert_eq!(closecombat_steer(0xC0, 0x40), -1, "the 0x80 tie turns −1");
        assert_eq!(closecombat_steer(0x00, 0x40), -1);
        assert_eq!(closecombat_steer(0x41, 0x00), 1);
        assert_eq!(closecombat_steer(0xFF, 0x00), -1);
        assert_eq!(closecombat_steer(0x01, 0x00), 1);
        assert_eq!(closecombat_steer(0x3F, 0x40), -1);
    }
}
