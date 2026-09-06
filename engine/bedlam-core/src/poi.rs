//! The S8 PERSONNEL/POI family (P5/G2, the LAST census class; RE-EXW-SIM
//! §7j.77 — every [verified] tag below cites §7j.77's objdump walk of the
//! loader block 0x416f6e..0x417094 + the controller FUN_00412a98
//! 0x412a98..0x412f21 + the walker FUN_00415b6c + the damage lane
//! FUN_0040dc1b).
//!
//! Scope: the E-side model of the 0x4dabdc POI bank (0x1E stride, count
//! cell 0x46cbf0, 0xF00 B = 128 slots, whole-bank memset-0 at .NME
//! load) + the .NME section-8 staging (four POIs per 8-B record) + the
//! controller subset — idle 1, settle 2, walk-out 3, flee 4, ESCAPE 5,
//! panic 6/7 — + the FUN_0040dc1b damage lane [all verified §7j.77].
//!
//! The rescue fiction: idle personnel wander; an OPEN elevator pad
//! within reach sends them fleeing to it; arrival = the escape award
//! (the counter + 5000 pts); weapon blasts near them deal proximity
//! damage and a dead POI runs the 6→7 panic tail.
//!
//! E-GAPS (documented, deliberately unlanded): the RandB death-SOUND
//! pick (the second stream, presentation), both FUN_0043a48e SFX, the
//! FUN_00420608 death effect spawn, the MissionShell [0x4eba10] banner
//! countdown, the animator (0x405186 family — reads state/heading,
//! draws sprites), and the blast-debris CALLER of the damage lane (the
//! 0x40e158 sweep — the engine's debris bank is a T3 dump surface with
//! no behavior tick; `poi_damage` is the transcribed seam).
//!
//! Coordinate scales (§7j.77/1): x/y Q13 (0x2000 per tile), z Q5
//! (32/tile); the exit-slot x/y are RAW px (the §7j.19 activator's
//! pad·0x20+0xF stamps — the same scale the octile scans compare
//! against poi.x>>8).
//!
//! The bank does NOT enter `MissionSim::state_hash` (the W6 split, the
//! critter-bank precedent): the T2 `poi-bank` watch row is the capture-
//! plan side (EXW 0x4dabdc / EXD 0x971d4, alias-complete since D162).

use crate::mission::{dist_octagonal, MissionSim};

/// POI bank capacity — the 0xF00-byte arena at 0x4dabdc / the 0x1E
/// stride [verified §7j.77/1].
pub const POI_SLOTS: usize = 0xF00 / 0x1E;

/// The exit-slot bank stride family — five 0x1C slots @0x4e662c
/// (§7j.19); the controller-read subset {active, PHASE, x, y, dwell}.
pub const EXIT_SLOTS: usize = 5;

/// One personnel/POI record — the modeled subset of the 0x1E-stride
/// frame [§7j.77/1; field names per the sar-16 idiom table].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoiRecord {
    /// ACTIVE w@+0x00 — 0 skips the POI whole (the loop head) and is
    /// the escape-complete tombstone.
    pub active: bool,
    /// HP w@+0x02 — seeded 0x32 (50); the ONE .NME bank with no
    /// base+(base·m)/27 scaling (no imul site in the loader block).
    /// Decremented by [`MissionSim::poi_damage`] (FUN_0040dc1b).
    pub hp: i16,
    /// STATE w@+0x04 — {1 idle, 2 settle, 3 walk-out, 4 flee, 5
    /// ESCAPE, 6/7 panic}; seeded 1 (personnel spawn IDLE — §7j.77/2
    /// retired §7j.18's transposed "+4 5" reading).
    pub state: u16,
    /// HEADING word w@+0x08 — seeded RandA()&7 RAW; the aims stamp
    /// sector<<5 (the 32-sector quantized octile direction); the
    /// walker tail copies the draw word's quadrant. The cos/sin walk
    /// reads THIS word.
    pub heading: i32,
    /// TIMER w@+0x0A — multiplexed per state (idle counter / settle
    /// counter / walk budget / flee budget / escape countdown / panic
    /// counter); seeded 0 by the load memset.
    pub timer: i32,
    /// EXIT SLOT w@+0x0C — the flee-begin stash; the escape-complete
    /// resets that slot's dwell (the multi-POI elevator reset).
    pub exit_slot: usize,
    /// World x d@+0x0E — Q13.
    pub x: i32,
    /// World y d@+0x12 — Q13.
    pub y: i32,
    /// World z d@+0x16 — Q5, re-settled by the per-frame floor probe.
    pub z: i32,
    /// DRAW word d@+0x1A — the walker's sprite quadrant
    /// (angle+0x20)&0xC0 ∈ {0, 0x40, 0x80, 0xC0}; the quadrant
    /// ladder's dispatch key.
    pub draw_word: i32,
}

impl PoiRecord {
    /// The §7j.77/2 spawn stamp — everything the loader block writes.
    fn spawn(x: i32, y: i32, z: i32, heading: i32) -> Self {
        PoiRecord {
            active: true,
            hp: 0x32,
            state: 1,
            heading,
            timer: 0,
            exit_slot: 0,
            x,
            y,
            z,
            draw_word: 0,
        }
    }
}

/// One exit-slot record — the controller-read subset of the 0x1C
/// frame @0x4e662c [§7j.19 + §7j.77/3]: {active d@+0, PHASE d@+4
/// (1 descend / 2 landed-OPEN / 3 depart), x d@+8, y d@+0xC (RAW px
/// — the FUN_0041fa51 activator's pad·0x20+0xF stamps), dwell d@+0x18
/// (reset to 0 by the POI escape-complete)}.
///
/// Host-staged through [`MissionSim::stage_poi_exit`]: the §7j.19
/// producer side (the pad-trigger activator + the dropship animator
/// that advances PHASE) is deliberately unlanded — an unstaged bank
/// reads all-inactive, so personnel idle/walk and never flee, exactly
/// like a mission whose elevators never open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExitSlot {
    pub active: bool,
    pub phase: i32,
    pub x: i32,
    pub y: i32,
    pub dwell: i32,
}

/// The escape-lane constants [verified §7j.77/3].
/// The head-lane distance gate (any open exit within 384 octile-px).
pub const FLEE_DIST: i32 = 0x180;
/// The idle setlle/walk-out split (192).
pub const SETTLE_DIST: i32 = 0xC0;
/// The flee-arrival gate (the escape begins within 16 octile-px).
pub const ARRIVE_DIST: i32 = 0x10;
/// The escape-complete panic timer stamp ([0x4eba10] := 50).
pub const PANIC_TIMER: i32 = 0x32;
/// The never-expire flee sentinel (the flee walk is unbounded).
pub const FLEE_FOREVER: i32 = 0x2710;
/// The escape award (FUN_00448b80(0x1388)).
pub const ESCAPE_AWARD: i32 = 5000;

impl MissionSim {
    /// The .NME SECTION-8 staging (§7j.77/2) — called by
    /// `stage_critters` for the eighth parsed section. Per 8-B record
    /// (w1 = z level, w2 = x tile, w3 = y tile) exactly FOUR POIs;
    /// three RandA draws per POI in the asm's order — x, y, then the
    /// heading (the z probe between is draw-free):
    /// x = ((RandA&0x1F) + w2<<5)·0x100 (a RANDOM in-tile offset
    /// 0..0x1F00 — not the S3/S5/S6 fixed +0xF00), y likewise on w3,
    /// z = the floor probe at level w1 (the FUN_0041e411 family),
    /// heading = RandA&7 RAW. NO difficulty term, NO count draw, NO
    /// hp scaling (the literal 0x32).
    pub(crate) fn stage_pois(&mut self, recs: &[Vec<u16>]) -> usize {
        let mut staged = 0usize;
        for rec in recs {
            for _ in 0..4 {
                if self.pois.len() >= POI_SLOTS {
                    break;
                }
                let x = ((self.rand_a() as i32 & 0x1F) + ((rec[2] as i32) << 5)) * 0x100;
                let y = ((self.rand_a() as i32 & 0x1F) + ((rec[3] as i32) << 5)) * 0x100;
                let z = self.terrain.floor_z(x >> 8, y >> 8, (rec[1] as i32) << 5);
                let heading = (self.rand_a() & 7) as i32;
                self.pois.push(PoiRecord::spawn(x, y, z, heading));
                staged += 1;
            }
        }
        staged
    }

    /// Read-only view of the POI bank (the canonical dump / census
    /// seam; NOT hashed — the W6 split).
    pub fn pois(&self) -> &[PoiRecord] {
        &self.pois
    }

    /// The escape counter [0x4eba0c] — MissionShell resets it at boot
    /// and the HUD tail reads it; only the POI escape-complete
    /// increments it.
    pub fn poi_escape_count(&self) -> i32 {
        self.poi_escapes
    }

    /// The panic timer [0x4eba10] — stamped 0x32 by every escape
    /// (the MissionShell banner countdown decrements it host-side).
    pub fn poi_panic_timer(&self) -> i32 {
        self.poi_panic
    }

    /// Host seam for the §7j.19 exit-slot family — the
    /// controller-read subset only. `phase` 2 = landed-OPEN (the only
    /// phase the flee/escape lanes act on).
    pub fn stage_poi_exit(&mut self, slot: usize, x: i32, y: i32, phase: i32) {
        if let Some(e) = self.poi_exits.get_mut(slot) {
            *e = ExitSlot {
                active: true,
                phase,
                x,
                y,
                dwell: 0,
            };
        }
    }

    /// The FUN_0040dc1b DAMAGE LANE [verified §7j.77/5]: hp −= dmg;
    /// hp ≤ 0 → the POI death — state := 6 (PANIC-1), timer := 0 (the
    /// controller's 6→7 tail takes it from there; the dead POI stays
    /// ACTIVE). The RandB sound pick, the death SFX, and the
    /// FUN_00420608 effect spawn are the documented E-gaps. The
    /// original's caller is the weapon-blast debris sweep
    /// (0x40e158: octile dist < 0x30 ∧ |dz| < 0x20 → dmg =
    /// (0x40 − dist)>>2) — no debris behavior tick exists engine-side
    /// yet, so this seam awaits that family.
    pub fn poi_damage(&mut self, idx: usize, dmg: i32) {
        let Some(p) = self.pois.get_mut(idx) else {
            return;
        };
        p.hp -= dmg as i16;
        if p.hp <= 0 {
            p.state = 6;
            p.timer = 0;
        }
    }

    /// The nearest-exit scan FUN_00417c64 [verified §7j.77/3]: the
    /// five slots, skip inactive, octile dist on RAW-px slot x/y vs
    /// poi.x>>8 (Q5), best-fit, sentinel 0x989680 → (idx, dist).
    fn poi_nearest_exit(&self, idx: usize) -> (i32, i32) {
        let p = &self.pois[idx];
        let mut best_idx = -1i32;
        let mut best = 0x989680i32;
        for (slot, e) in self.poi_exits.iter().enumerate() {
            if !e.active {
                continue;
            }
            let dx = e.x - (p.x >> 8);
            let dy = e.y - (p.y >> 8);
            let d = dist_octagonal(dx, dy);
            if d < best {
                best_idx = slot as i32;
                best = d;
            }
        }
        (best_idx, best)
    }

    /// The 32-sector quantized octile aim — ((dir + 0xF) & 0xFF) >> 5
    /// & 7 << 5 over the FUN_00425498 direction byte (the engine's
    /// `angle_byte` transcription) [verified §7j.77/3].
    fn poi_sector_aim(&self, dx: i32, dy: i32) -> i32 {
        let dir = self.angles.angle_byte(dx, dy) as i32;
        // The asm's sequential quantizer (sar 5 / and 7 / shl 5) —
        // parenthesized: Rust's shift precedence would otherwise
        // fold `& 7 << 5` into a single `& 0xE0`.
        ((((dir + 0xF) & 0xFF) >> 5) & 7) << 5
    }

    /// The POI move gate — FUN_0040cc5e/0x41e859 [verified §7j.77/4]:
    /// floor := floor_z at the TARGET px (the FUN_0041e231 family —
    /// the same probe the staging uses), pass iff |floor − z| ≤ 4
    /// (NOT the critter walk_gate's 3), and z := floor on pass.
    fn poi_gate(&mut self, idx: usize, dx: i32, dy: i32) -> bool {
        let (x, y, z) = {
            let p = &self.pois[idx];
            (p.x, p.y, p.z)
        };
        let floor = self.terrain.floor_z((x + dx) >> 8, (y + dy) >> 8, z);
        if (floor - z).abs() <= 4 {
            self.pois[idx].z = floor;
            true
        } else {
            false
        }
    }

    /// The walker FUN_00415b6c(i, dx, dy, angle) [verified §7j.77/4]:
    /// free move if the gate passes (x/y += the cos/sin steps, draw
    /// word := (angle+0x20)&0xC0); else the QUADRANT LADDER on the
    /// CURRENT draw word — {0 and 0x80: the y-axis first, x fallback
    /// picked by the ANGLE (≥0x80 → −x first); 0x40 and 0xC0: the
    /// x-axis first, y fallback picked by the ORIGINAL dx arg (>0x80
    /// → +y first)} — each axis attempt gate-tested (±0x200), first
    /// pass applies the step and rewrites the draw word; the tail
    /// copies the draw word into the heading word.
    fn poi_walk(&mut self, idx: usize, dx: i32, dy: i32) {
        let angle = self.pois[idx].heading;
        let saved_z = self.pois[idx].z;
        if self.poi_gate(idx, dx, dy) {
            let p = &mut self.pois[idx];
            p.x += dx;
            p.y += dy;
            p.draw_word = (angle + 0x20) & 0xC0;
        } else {
            // The blocked path restores the entry z (defensive — a
            // failed gate never wrote it) and walks the ladder.
            self.pois[idx].z = saved_z;
            let dw = self.pois[idx].draw_word;
            match dw {
                0 => {
                    // Ladder A: y−0x200 (word stays 0); else x by angle.
                    if self.poi_gate(idx, 0, -0x200) {
                        self.pois[idx].y -= 0x200;
                    } else if angle < 0x80 {
                        if self.poi_gate(idx, 0x200, 0) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0x40;
                            p.x += 0x200;
                        } else if self.poi_gate(idx, -0x200, 0) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0xC0;
                            p.x -= 0x200;
                        }
                    } else if self.poi_gate(idx, -0x200, 0) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0xC0;
                        p.x -= 0x200;
                    } else if self.poi_gate(idx, 0x200, 0) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0x40;
                        p.x += 0x200;
                    }
                }
                0x40 => {
                    // Ladder B: x+0x200 (word stays 0x40); else y by
                    // the original dx arg.
                    if self.poi_gate(idx, 0x200, 0) {
                        self.pois[idx].x += 0x200;
                    } else if dx > 0x80 {
                        if self.poi_gate(idx, 0, 0x200) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0x80;
                            p.y += 0x200;
                        } else if self.poi_gate(idx, 0, -0x200) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0;
                            p.y -= 0x200;
                        }
                    } else if self.poi_gate(idx, 0, -0x200) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0;
                        p.y -= 0x200;
                    } else if self.poi_gate(idx, 0, 0x200) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0x80;
                        p.y += 0x200;
                    }
                }
                0x80 => {
                    // Ladder C: y+0x200 (word stays 0x80); else x by
                    // angle.
                    if self.poi_gate(idx, 0, 0x200) {
                        self.pois[idx].y += 0x200;
                    } else if angle < 0x80 {
                        if self.poi_gate(idx, 0x200, 0) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0x40;
                            p.x += 0x200;
                        } else if self.poi_gate(idx, -0x200, 0) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0xC0;
                            p.x -= 0x200;
                        }
                    } else if self.poi_gate(idx, -0x200, 0) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0xC0;
                        p.x -= 0x200;
                    } else if self.poi_gate(idx, 0x200, 0) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0x40;
                        p.x += 0x200;
                    }
                }
                0xC0 => {
                    // Ladder D: x−0x200 (word stays 0xC0); else y by
                    // the original dx arg.
                    if self.poi_gate(idx, -0x200, 0) {
                        self.pois[idx].x -= 0x200;
                    } else if dx > 0x80 {
                        if self.poi_gate(idx, 0, 0x200) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0x80;
                            p.y += 0x200;
                        } else if self.poi_gate(idx, 0, -0x200) {
                            let p = &mut self.pois[idx];
                            p.draw_word = 0;
                            p.y -= 0x200;
                        }
                    } else if self.poi_gate(idx, 0, -0x200) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0;
                        p.y -= 0x200;
                    } else if self.poi_gate(idx, 0, 0x200) {
                        let p = &mut self.pois[idx];
                        p.draw_word = 0x80;
                        p.y += 0x200;
                    }
                }
                _ => {}
            }
        }
        // The tail: heading := the draw word's low word.
        let dw = self.pois[idx].draw_word;
        self.pois[idx].heading = dw;
    }

    /// The cos/sin walk steps — cos(heading)>>6, sin(heading)>>6 over
    /// the [0x46cbd0] tables (FUN_0041eb65/0x41eb77; sin(a) =
    /// cos(a−0x40)) [verified §7j.77/3]. Returns (dx, dy).
    fn poi_walk_vector(&self, heading: i32) -> (i32, i32) {
        let h = (heading & 0xFF) as u16;
        match (
            self.angles.sine_word(h),
            self.angles.sine_word(((h as i32 - 0x40) & 0xFF) as u16),
        ) {
            // The arg order at the call site: edx = the FIRST lookup
            // (cos), ebx = the SECOND (sin).
            (Some(cos), Some(sin)) => ((cos as i16 as i32) >> 6, (sin as i16 as i32) >> 6),
            _ => (0, 0),
        }
    }

    /// The POI controller FUN_00412a98 [verified whole §7j.77/3] —
    /// runs after the critter controller (MissionShell 0x447fe6,
    /// right after 0x447fe1), armed with the same `critters = 1`
    /// family arm (the .NME loader stages both banks).
    pub(crate) fn poi_tick(&mut self) {
        for i in 0..self.pois.len() {
            if !self.pois[i].active {
                continue;
            }
            // The per-frame prologue: the z re-settle + the exit scan.
            let (x, y, z) = {
                let p = &self.pois[i];
                (p.x, p.y, p.z)
            };
            let z2 = self.terrain.floor_z(x >> 8, y >> 8, z);
            self.pois[i].z = z2;
            let (scan_idx, scan_dist) = self.poi_nearest_exit(i);
            let state = self.pois[i].state as i32;
            match state {
                1..=3 => {
                    // The head lane: ONE draw, then the two gates.
                    let roll = (self.rand_a() & 0xF) as i32;
                    if roll == 0
                        && scan_dist < FLEE_DIST
                        && scan_idx >= 0
                        && self.poi_exits[scan_idx as usize].phase == 2
                    {
                        let p = &mut self.pois[i];
                        p.state = 4;
                        p.exit_slot = scan_idx as usize;
                    } else {
                        self.poi_idle_machine(i, scan_dist);
                    }
                }
                4 => {
                    // The flee fast lane: arrival at ANY exit (the
                    // scan's best distance) with the STASHED slot's
                    // phase still 2 → ESCAPE-BEGIN.
                    let slot = self.pois[i].exit_slot;
                    if scan_dist < ARRIVE_DIST && self.poi_exits[slot].phase == 2 {
                        let p = &mut self.pois[i];
                        p.state = 5;
                        p.timer = -1;
                    } else {
                        self.poi_flee(i);
                    }
                }
                5 => {
                    let p = &mut self.pois[i];
                    p.timer += 1;
                    if p.timer >= 10 {
                        // ESCAPE-COMPLETE [§7j.77/3]: the tombstone,
                        // the counter, the panic cell, the elevator
                        // dwell reset, the award.
                        p.active = false;
                        let slot = p.exit_slot;
                        self.poi_escapes += 1;
                        self.poi_panic = PANIC_TIMER;
                        if let Some(e) = self.poi_exits.get_mut(slot) {
                            e.dwell = 0;
                        }
                        self.score_pending += ESCAPE_AWARD;
                        self.objective_notification(5000);
                    }
                }
                6 => {
                    let p = &mut self.pois[i];
                    p.timer += 1;
                    if p.timer > 5 {
                        p.state = 7;
                    }
                }
                7 => {
                    // PANIC-2: inert (the corpse stays ACTIVE).
                    self.pois[i].timer = 0;
                }
                _ => {}
            }
        }
    }

    /// The states 1/2/3 machine [verified §7j.77/3]. Draw budget: one
    /// head-gate draw in `poi_tick` plus at most one gate draw + two
    /// stamp draws here on a transition frame.
    fn poi_idle_machine(&mut self, i: usize, exit_dist: i32) {
        match self.pois[i].state {
            1 => {
                if self.pois[i].timer <= 10 {
                    self.pois[i].timer += 1;
                    return;
                }
                // timer > 10: settle (exit close ∧ 1/16) vs walk-out.
                if exit_dist >= SETTLE_DIST || (self.rand_a() & 0xF) as i32 != 0 {
                    self.poi_walk_out(i);
                } else {
                    // SETTLE-BEGIN: face the nearest alive robot. The
                    // FUN_00417c00 distance write into the frame's
                    // [esp] scratch is transient (nothing reads it
                    // before the next frame's exit scan) — modeled as
                    // the aim only.
                    let (px, py) = {
                        let p = &self.pois[i];
                        (p.x, p.y)
                    };
                    let mut best = -1i32;
                    let mut best_d = 0x989680i32;
                    for (ri, r) in self.robots.iter().enumerate() {
                        if !r.alive {
                            continue;
                        }
                        let d = dist_octagonal(r.pos_x - px, r.pos_y - py);
                        if d < best_d {
                            best = ri as i32;
                            best_d = d;
                        }
                    }
                    let aim = (best >= 0).then(|| {
                        let r = &self.robots[best as usize];
                        self.poi_sector_aim(r.pos_x - px, r.pos_y - py)
                    });
                    let p = &mut self.pois[i];
                    p.state = 2;
                    p.timer = 0;
                    if let Some(a) = aim {
                        p.heading = a;
                    }
                }
            }
            2 => {
                let p = &mut self.pois[i];
                p.timer += 1;
                if p.timer > 8 {
                    p.timer = 0;
                    p.state = 1;
                }
            }
            3 => {
                if self.pois[i].timer == 0 {
                    self.pois[i].state = 1;
                } else {
                    self.pois[i].timer -= 1;
                    let h = self.pois[i].heading;
                    let (dx, dy) = self.poi_walk_vector(h);
                    self.poi_walk(i, dx, dy);
                }
            }
            _ => {}
        }
    }

    /// The state-1 walk-out gate [verified §7j.77/3]: a 1/16 draw,
    /// then state 3 + the 10..25-frame budget + a random 8-way
    /// heading (two more draws).
    fn poi_walk_out(&mut self, i: usize) {
        if (self.rand_a() & 0xF) as i32 != 0 {
            return;
        }
        let timer = ((self.rand_a() & 0xF) as i32) + 10;
        let heading = ((self.rand_a() & 7) as i32) << 5;
        let p = &mut self.pois[i];
        p.state = 3;
        p.timer = timer;
        p.heading = heading;
    }

    /// The state-4 FLEE body [verified §7j.77/3]: abort to idle when
    /// the stashed exit closes; else re-aim at the exit, walk, and
    /// decrement the budget (negative → the 10000 never-expire
    /// sentinel).
    fn poi_flee(&mut self, i: usize) {
        let slot = self.pois[i].exit_slot;
        let exit_ok = self
            .poi_exits
            .get(slot)
            .map(|e| e.active && e.phase == 2)
            .unwrap_or(false);
        if !exit_ok {
            let p = &mut self.pois[i];
            p.timer = 0;
            p.state = 1;
            return;
        }
        let (px, py) = {
            let p = &self.pois[i];
            (p.x, p.y)
        };
        let (ex, ey) = {
            let e = &self.poi_exits[slot];
            // The aim compares the RAW-px exit against poi.x>>8 (Q5);
            // shift the exit up to Q13 for `angle_byte` (the original
            // shl <<8 inside FUN_0042548).
            (e.x << 8, e.y << 8)
        };
        let aim = self.poi_sector_aim(ex - px, ey - py);
        self.pois[i].heading = aim;
        let h = self.pois[i].heading;
        let (dx, dy) = self.poi_walk_vector(h);
        self.poi_walk(i, dx, dy);
        let p = &mut self.pois[i];
        p.timer -= 1;
        if p.timer < 0 {
            p.timer = FLEE_FOREVER;
        }
    }
}

#[cfg(test)]
mod tests {
    //! The S8 personnel/POI lane (§7j.77): the loader walk's exact
    //! seeds + draw budget, the idle/settle/walk-out machine, the
    //! flee→escape award lane over the exit seam, the panic tail,
    //! and the walker's gate/ladder.

    use super::*;

    /// Flat level-2 world (the wanderer-test fixture): plane 2
    /// solid type-1 everywhere, height byte 0x1F everywhere →
    /// floor_z == 2·0x20+0x1F == 0x5F at every cell.
    fn sim_flat(seed: u64) -> MissionSim {
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let mut words = vec![0i16; 256];
        for (a, w) in words.iter_mut().enumerate() {
            *w = ((a as f64 * std::f64::consts::PI / 128.0).sin() * 32767.0).round() as i16;
        }
        let angles = crate::mission::AngleTable::from_sintable_words(&words).unwrap();
        MissionSim::new(terrain, angles, seed)
    }

    /// An .NME hosting exactly one S8 record (w1 = z level, w2/w3 =
    /// the x/y tile); every other section empty.
    fn s8_nme(w1: u16, w2: u16, w3: u16) -> Vec<u8> {
        let mut b = Vec::new();
        for _ in 0..7 {
            b.extend_from_slice(&0u16.to_le_bytes()); // S1..S7 counts
        }
        b.extend_from_slice(&1u16.to_le_bytes()); // S8 count
        for w in [1u16, w1, w2, w3] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b
    }

    /// A staged POI pushed directly (controller tests bypass the
    /// loader's RNG consumption).
    fn push_poi(sim: &mut MissionSim, state: u16, timer: i32, x: i32, y: i32) -> usize {
        sim.pois.push(PoiRecord {
            active: true,
            hp: 0x32,
            state,
            heading: 0,
            timer,
            exit_slot: 0,
            x,
            y,
            z: 0x5F,
            draw_word: 0,
        });
        sim.pois.len() - 1
    }

    /// Advance frames with the family arm set.
    fn run(sim: &mut MissionSim, frames: usize) {
        sim.arm_critter_family();
        for _ in 0..frames {
            sim.advance_frame();
        }
    }

    #[test]
    fn s8_staging_four_per_record_with_exact_seeds() {
        let mut sim = sim_flat(11);
        let staged = sim
            .stage_critters(&s8_nme(2, 5, 7), 1)
            .expect("S8 accepted");
        // The POIs do NOT add to the critter count (DAT_0046cc2c
        // counts critters only).
        assert_eq!(staged, 0);
        assert_eq!(sim.pois().len(), 4, "exactly four POIs per record");
        for p in sim.pois() {
            assert!(p.active);
            assert_eq!(
                p.state, 1,
                "spawn IDLE (§7j.77/2 — the §7j.18 transposition retired)"
            );
            assert_eq!(p.hp, 0x32, "the literal 50 — NO m-scalar on this bank");
            assert_eq!(p.timer, 0, "the memset seed");
            assert_eq!(p.exit_slot, 0);
            assert_eq!(p.draw_word, 0);
            assert!((0..=7).contains(&p.heading), "heading = RandA&7 raw");
            // x = ((r&0x1F) + w2<<5)·0x100 ∈ [5·0x2000, 5·0x2000+0x1F00)
            assert!((5 * 0x2000..5 * 0x2000 + 0x1F00).contains(&p.x));
            assert!((7 * 0x2000..7 * 0x2000 + 0x1F00).contains(&p.y));
            assert_eq!(p.z, 0x5F, "the w1-level floor probe (flat level 2)");
        }
    }

    #[test]
    fn s8_staging_draw_budget_three_per_poi() {
        // 4 POIs × 3 draws (x, y, heading) = 12 stream draws; the z
        // probe draws nothing. A same-seed sim that stages the
        // counts-only file then draws 12 times lands on the identical
        // stream state.
        let mut a = sim_flat(7);
        a.stage_critters(&s8_nme(2, 5, 7), 1).expect("staged");
        assert_eq!(a.pois().len(), 4);
        let mut b = sim_flat(7);
        let mut counts_only = Vec::new();
        for _ in 0..8 {
            counts_only.extend_from_slice(&0u16.to_le_bytes());
        }
        b.stage_critters(&counts_only, 1).expect("staged");
        assert!(b.pois().is_empty());
        for _ in 0..12 {
            b.rand_a();
        }
        assert_eq!(
            a.rand_a_state(),
            b.rand_a_state(),
            "the S8 schedule advances exactly 12 draws"
        );
    }

    #[test]
    fn poi_idle_timer_ramp_then_walk_out() {
        let mut sim = sim_flat(21);
        let i = push_poi(&mut sim, 1, 0, 0x2000, 0x2000);
        // Frames 1..=11: timer ramps to 11 with the head gate
        // missing (15/16 per frame — seed 21's rolls all miss over
        // this window; the assertion below pins it).
        run(&mut sim, 11);
        assert_eq!(sim.pois[i].timer, 11);
        assert_eq!(sim.pois[i].state, 1);
    }

    #[test]
    fn poi_walk_out_transition_and_budget() {
        // timer > 10 + no exit in reach → the walk-out gate; find a
        // seed whose head-gate draw misses and whose walk-out roll
        // hits (1/16) — probe on a throwaway sim, then run a FRESH
        // same-seed sim (the probe burns the draws it checks).
        let mut seed = 1u64;
        loop {
            let mut probe = sim_flat(seed);
            let _ = push_poi(&mut probe, 1, 11, 0x2000, 0x2000);
            // burn the head-gate draw and require a miss, then the
            // walk-out roll must hit.
            if (probe.rand_a() & 0xF) != 0 && (probe.rand_a() & 0xF) == 0 {
                break;
            }
            seed += 1;
        }
        let mut sim = sim_flat(seed);
        let i = push_poi(&mut sim, 1, 11, 0x2000, 0x2000);
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].state, 3, "the walk-out fired");
        assert!((10..=25).contains(&sim.pois[i].timer), "(rand&0xF)+10");
        assert_eq!(sim.pois[i].heading & 0x1F, 0, "(rand&7)<<5");
        assert!((0..8).contains(&(sim.pois[i].heading >> 5)));
    }

    #[test]
    fn poi_walk_budget_decrement_and_return_to_idle() {
        let mut sim = sim_flat(33);
        let i = push_poi(&mut sim, 3, 2, 0x2000, 0x2000);
        sim.pois[i].heading = 0; // due north — the flat world walks it
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].timer, 1, "budget decrements");
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].timer, 0);
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].state, 1, "budget exhausted → idle");
    }

    #[test]
    fn poi_walker_moves_and_sets_draw_word() {
        let mut sim = sim_flat(33);
        let i = push_poi(&mut sim, 3, 5, 0x4000, 0x4000);
        sim.pois[i].heading = 0x40; // due east
        let x0 = sim.pois[i].x;
        run(&mut sim, 1);
        let p = &sim.pois[i];
        // cos(0x40)>>6 = 32767>>6 = 511 east; sin = 0.
        assert_eq!(p.x - x0, 511);
        assert_eq!(p.draw_word, (0x40 + 0x20) & 0xC0);
        assert_eq!(
            p.heading, p.draw_word,
            "the walker tail copies the draw word"
        );
    }

    #[test]
    fn poi_gate_tolerance_four() {
        // Same-level target passes (|diff| 0 ≤ 4) and updates z; a
        // target column whose floor sits one level higher (|diff|
        // 0x20) fails.
        let mut sim = sim_flat(5);
        let i = push_poi(&mut sim, 1, 0, 0x2000, 0x2000);
        assert!(sim.poi_gate(i, 0x100, 0));
        assert_eq!(sim.pois[i].z, 0x5F);
        // The wall: level 2 EMPTY at (16,16) + level 3 solid there →
        // the probe's up-walk lands floor 0x7F.
        let mut planes = vec![0u8; 8 * 32 * 32];
        for b in planes[2 * 32 * 32..3 * 32 * 32].iter_mut() {
            *b = 1;
        }
        planes[2 * 32 * 32 + 16 * 32 + 16] = 0;
        planes[3 * 32 * 32 + 16 * 32 + 16] = 1;
        let heights = vec![[0x1Fu8; 1024]];
        let terrain = crate::mission::Terrain::from_parts(32, 32, planes, heights).unwrap();
        let angles = sim_flat(5).angles;
        let mut sim2 = MissionSim::new(terrain, angles, 5);
        let j = push_poi(&mut sim2, 1, 0, 16 * 0x2000, 16 * 0x2000);
        assert!(
            !sim2.poi_gate(j, 0x100, 0),
            "the higher column blocks (|diff| > 4)"
        );
        assert_eq!(sim2.pois[j].z, 0x5F, "a failed gate never writes z");
    }

    #[test]
    fn poi_settle_faces_nearest_robot_then_returns() {
        let mut sim = sim_flat(9);
        let i = push_poi(&mut sim, 1, 11, 0x2000, 0x2000);
        // A robot due east of the POI + an ACTIVE exit nearby (the
        // settle lane needs nearest-exit dist < 0xC0; phase is NOT
        // read here — only the head flee lane checks phase 2).
        sim.robots.push(crate::mission::Robot {
            pos_x: 0x2000 + 0x4000,
            pos_y: 0x2000,
            z: 0x5F,
            state: 0,
            dir_byte: 0,
            facing: crate::mission::FACING_NONE,
            anim: 0,
            variant: 0,
            probe_z: [0x5F; 8],
            stop_dist: 0,
            target: None,
            alive: true,
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
        sim.stage_poi_exit(0, (0x2000 >> 8) + 0x40, 0x2000 >> 8, 1);
        // seed 9's stream: pin by behavior — run until settled.
        let mut settled = false;
        for _ in 0..200 {
            run(&mut sim, 1);
            if sim.pois[i].state == 2 {
                settled = true;
                break;
            }
            assert_eq!(sim.pois[i].state, 1, "idle while the gates miss");
        }
        assert!(settled, "the 1/16 settle lane fires within 200 frames");
        assert_eq!(sim.pois[i].timer, 0);
        assert_eq!(
            sim.pois[i].heading, 0x40,
            "sector<<5 aimed at the due-east robot"
        );
        // Nine settle frames (timer 1..9 > 8) then back to idle.
        let mut back = false;
        for _ in 0..12 {
            run(&mut sim, 1);
            if sim.pois[i].state == 1 && sim.pois[i].timer == 0 {
                back = true;
                break;
            }
        }
        assert!(back, "settle lasts ~9 frames then returns to idle");
    }

    #[test]
    fn poi_head_lane_flees_to_open_exit() {
        let mut sim = sim_flat(4);
        let i = push_poi(&mut sim, 1, 11, 0x2000, 0x2000);
        // An OPEN (phase 2) exit within 0x180 → the head lane.
        sim.stage_poi_exit(2, (0x2000 >> 8) + 0x40, 0x2000 >> 8, 2);
        let mut fled = false;
        for _ in 0..200 {
            run(&mut sim, 1);
            if sim.pois[i].state == 4 {
                fled = true;
                break;
            }
        }
        assert!(fled, "the 1/16 head lane fires within 200 frames");
        assert_eq!(sim.pois[i].exit_slot, 2, "the scan's slot stashed");
    }

    #[test]
    fn poi_flee_abort_when_exit_not_open() {
        let mut sim = sim_flat(6);
        let i = push_poi(&mut sim, 4, 7, 0x2000, 0x2000);
        // No exit staged → the stashed slot reads inactive.
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].state, 1);
        assert_eq!(sim.pois[i].timer, 0);
    }

    #[test]
    fn poi_flee_walks_toward_exit_and_never_expires() {
        let mut sim = sim_flat(8);
        let i = push_poi(&mut sim, 4, 1, 0x2000, 0x2000);
        // An open exit ~0x300 px east (beyond the 0x10 arrival).
        sim.stage_poi_exit(0, (0x2000 >> 8) + 0x300, 0x2000 >> 8, 2);
        run(&mut sim, 1);
        let p = &sim.pois[i];
        assert_eq!(p.state, 4);
        assert_eq!(p.timer, 0, "budget decrements");
        assert_eq!(p.heading & 0x1F, 0, "re-aimed sector<<5 at the exit");
        assert!(p.heading != 0, "an east exit gives a nonzero sector word");
        // The negative budget clamps to the 10000 sentinel.
        let mut sim2 = sim_flat(8);
        let j = push_poi(&mut sim2, 4, 0, 0x2000, 0x2000);
        sim2.stage_poi_exit(0, (0x2000 >> 8) + 0x300, 0x2000 >> 8, 2);
        sim2.pois[j].heading = 0; // stationary-aimed north
        run(&mut sim2, 1);
        assert_eq!(sim2.pois[j].timer, FLEE_FOREVER, "0−1 < 0 → the sentinel");
    }

    #[test]
    fn poi_escape_arrival_awards_and_tombstones() {
        let mut sim = sim_flat(10);
        let i = push_poi(&mut sim, 4, 0, 0x2000, 0x2000);
        // The exit AT the POI (dist 0 < 0x10), phase 2.
        sim.stage_poi_exit(3, 0x2000 >> 8, 0x2000 >> 8, 2);
        sim.poi_exits[3].dwell = 99;
        // Force the flee-begin stash to the right slot first.
        sim.pois[i].exit_slot = 3;
        let s0 = sim.rand_a_state();
        run(&mut sim, 1);
        assert_eq!(sim.pois[i].state, 5, "arrival begins ESCAPE");
        assert_eq!(sim.pois[i].timer, -1, "the 0xFFFF seed");
        // States 4/5 are draw-free (§7j.77/3): eleven ticks to the
        // award with ZERO stream movement.
        let s1 = sim.rand_a_state();
        for _ in 0..11 {
            run(&mut sim, 1);
        }
        assert_eq!(sim.rand_a_state(), s1, "the escape lane is draw-free");
        let _ = s0;
        let p = &sim.pois[i];
        assert!(!p.active, "the escape-complete tombstone");
        assert_eq!(sim.poi_escape_count(), 1, "[0x4eba0c]++");
        assert_eq!(sim.poi_panic_timer(), PANIC_TIMER, "[0x4eba10] := 0x32");
        assert_eq!(sim.poi_exits[3].dwell, 0, "the multi-POI elevator reset");
        assert_eq!(sim.score_pending, ESCAPE_AWARD, "FUN_00448b80(5000)");
    }

    #[test]
    fn poi_damage_lane_and_panic_tail() {
        let mut sim = sim_flat(12);
        let i = push_poi(&mut sim, 1, 0, 0x2000, 0x2000);
        sim.poi_damage(i, 10);
        assert_eq!(sim.pois[i].hp, 0x32 - 10);
        assert_eq!(sim.pois[i].state, 1, "partial damage does not panic");
        sim.poi_damage(i, 100);
        assert!(sim.pois[i].hp <= 0);
        assert_eq!(sim.pois[i].state, 6, "death → PANIC-1");
        assert_eq!(sim.pois[i].timer, 0);
        // Six ticks: timer 1..6 → > 5 → state 7; state 7 zeroes the
        // timer every frame and stays ACTIVE (the corpse).
        run(&mut sim, 6);
        assert_eq!(sim.pois[i].state, 7);
        run(&mut sim, 3);
        assert_eq!(sim.pois[i].state, 7);
        assert_eq!(sim.pois[i].timer, 0);
        assert!(sim.pois[i].active, "the dead POI stays present");
    }

    #[test]
    fn poi_damage_out_of_range_is_noop() {
        let mut sim = sim_flat(13);
        sim.poi_damage(9, 10); // no such record
        assert!(sim.pois().is_empty());
    }

    #[test]
    fn unarmed_family_never_ticks() {
        // The D114 arm discipline: an unarmed sim with staged POIs
        // consumes no draws and moves nothing.
        let mut sim = sim_flat(14);
        let i = push_poi(&mut sim, 3, 5, 0x2000, 0x2000);
        sim.pois[i].heading = 0x40;
        let s0 = sim.rand_a_state();
        for _ in 0..5 {
            sim.advance_frame();
        }
        assert_eq!(sim.rand_a_state(), s0, "no draws unarmed");
        assert_eq!(sim.pois[i].timer, 5, "no ticks unarmed");
    }
}
