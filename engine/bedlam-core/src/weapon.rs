//! The COMMAND-record weapon-fire producer family (P4.2/W12-S3-prep,
//! DESIGN-DIFFHARNESS §7 S3 row + §10-W12; RE-EXW-SIM §7j.37 — every
//! [verified] tag below cites §7j.37's re-verification of the
//! FUN_00409138 dispatch decompile + the FUN_00410823 tick).
//!
//! Scope: the E-side model of the two projectile banks + the command
//! ring + the consumer + the per-type tick. The banks are the S3 T2
//! watch surface (0x4c71f4 400×0x36 weapon records + 0x4cc654
//! 50×0x22 projectile records, lifecycle spawn/active/free) — they do
//! NOT enter `MissionSim::state_hash` (the W6 split: watched bank
//! rows are their own dump blobs; the scene hash stays the 31-leaf
//! robot model).
//!
//! NO-INJECT INVARIANT: with no staged COMMAND records the consumer
//! reduces to the recharge pass over zeroed slots and the ticks walk
//! all-free banks — `advance_frame` is byte-identical to the pre-S3
//! engine (pinned by the S0/S1/S2 canonical chains). No fire happens
//! without COMMAND records [asserted by tests].
//!
//! E-gaps (unmodeled, documented — S3.scen's differ findings will
//! name them): the remaining AI-order family internals (w2..4/0x18/0x19/
//! 0x21..0x28 spawn bodies), the mortar family FUN_0040a9ff (w0xE),
//! the impact APPLICATION (FUN_0041a894/0041bc1c need the
//! terrain-structure bank — S4 pairs it), the debris disbursers, the
//! SFX/message families (T4), the enemy-fire spawn producers (the
//! critter family), and the smoke-trail ring bank.

use crate::mission::{dist_octagonal, MissionSim};

/// Capacity of the 400×0x36 weapon-anim bank at 0x4c71f4 [7j.22/1].
pub const WEAPON_BANK_SLOTS: usize = 400;
/// Capacity of the 50×0x22 projectile bank at 0x4cc654 [7j.13/5].
pub const ENEMY_BANK_SLOTS: usize = 50;
/// Weapon slots per robot record (+0x36.., stride 8) [7j.37/1].
pub const WEAPON_SLOTS: usize = 7;

/// The weapon/projectile DAMAGE TABLE — FUN_00419aff's pure id
/// switch [verified, §7j.15/1 + the 7j.37 re-read]: the ONLY selector
/// beside the id is the global difficulty dword 0x46cbf8 (0..2); the
/// d==2 rows override the linear (d+1)·k with a flat 4·k via the
/// branchless ADD idiom. All unlisted ids return 1 (the cosmetic
/// ballistic set lands here). The 0x69-vs-table question stays OPEN.
pub fn weapon_damage(id: u16, difficulty: u32) -> i32 {
    match id {
        2 => 20,
        3 => 30,
        4 => 40,
        5 | 0x1A => 75,
        0xC => 5000,
        0xD => 312,
        0x24 => 400,
        0x29 => 250,
        0x65 => difficulty_scaled(difficulty, 50),
        0x66 => difficulty_scaled(difficulty, 300),
        0x67 | 0x68 => difficulty_scaled(difficulty, 75),
        _ => 1,
    }
}

/// The difficulty-scaled enemy-fire rows: (d+1)·k, plus k again when
/// d == 2 (the branchless flat override — d=2 → 4·k) [verified].
fn difficulty_scaled(difficulty: u32, k: i32) -> i32 {
    (difficulty as i32 + 1) * k + i32::from(difficulty == 2) * k
}

/// One robot weapon slot: the 8-byte group at robot +0x36+8k
/// {id u16@+0, ammo u16@+2, pad, cooldown u16@+6} [verified §7j.37/1].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeaponSlot {
    /// Weapon/stat id (0 = empty slot); id−2 ∈ 0..0x26 is the
    /// consumer's 39-case switch domain.
    pub id: u16,
    /// Ammo count (signed: the consumer decrements and clears the
    /// enable bit at 0).
    pub ammo: i16,
    /// Fire cooldown (frames); 0 = ready [the fire gate is
    /// mask ∧ cooldown==0 ∧ ammo≠0, verified].
    pub cooldown: u16,
}

/// One 0x36-stride weapon-anim record of the 400-slot bank at
/// 0x4c71f4 [layout verified 7j.22/1 + §7j.37]. kind 0 = free slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponRecord {
    /// Type/weapon-stat id u16@+0x00 (0 = free).
    pub kind: u16,
    /// Owner robot idx d@+0x02.
    pub owner: i32,
    /// Target selector d@+0x06 (homing 0x29 only: bit 0x1000 robot /
    /// bit 0x2000 structure / else critter).
    pub target: i32,
    /// Tick counter d@+0x0A (the notes' "ttl").
    pub tick: i32,
    /// Draw counter d@+0x0E (shell/artillery anim frames).
    pub draw_ctr: i32,
    /// Position Q13 d@+0x12/+0x16/+0x1A.
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Velocity d@+0x1E/+0x22 (x/y) and +0x26 (z, straight types).
    pub vx: i32,
    pub vy: i32,
    pub vz: i32,
    /// Class d@+0x2A: launch delay (0x24/0x29) or detonation cycle
    /// count (0xF/0x13) [7j.22/1].
    pub class: i32,
    /// Arc d@+0x2E: ballistic z-velocity (gravity −0x100/tick) or
    /// the heading byte (0x24 draw / 0x29 homing).
    pub arc: i32,
    /// Smoke-trail link d@+0x32 (−1 = none; the 0x4e66b8 ring bank).
    pub trail: i32,
}

impl Default for WeaponRecord {
    fn default() -> Self {
        WeaponRecord {
            kind: 0,
            owner: 0,
            target: 0,
            tick: 0,
            draw_ctr: 0,
            x: 0,
            y: 0,
            z: 0,
            vx: 0,
            vy: 0,
            vz: 0,
            class: 0,
            arc: 0,
            trail: -1,
        }
    }
}

impl WeaponRecord {
    /// True while the slot is occupied (kind ≠ 0 = active).
    pub fn active(&self) -> bool {
        self.kind != 0
    }
}

/// One 0x22-stride record of the 50-slot projectile bank at 0x4cc654
/// [7j.13/5]: word@+0 is the TYPE (0 = free; the enemy-fire family
/// 0x65..0x69 per the 7j.28 draw dispatch — the tick's "type 1/2/3"
/// branches are the −0x64 normalization [hypothesis, §7j.37 scope
/// note]), x/y/z @+2/+6/+0xA (z Q13, 0x2000 = 1 level),
/// vx/vy/vz @+0xE/+0x12/+0x16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnemyProjectile {
    pub kind: u16,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub vx: i32,
    pub vy: i32,
    pub vz: i32,
}

/// One COMMAND ring record — the W5 payload grammar shared with the
/// O1 injector [D82/D83]: byte@+0 marker, short@+1 robot id, short@+3
/// spot, byte@+5 flags, byte@+6 the builder FILLER (SP: rand&0xF; the
/// consumer never reads it), words@+7/+9/+0xB x/y/z — the SP consumer
/// offsets re-verified instruction-exact (FUN_00409138: the local_e0
/// word pointer is `rec+7`, bit0 reads [0]/[1] then bumps +2 shorts so
/// bit0∧bit1 triples read +0xB/+0xD/+0xF; the MP-only mask block bumps
/// the pointer first — SP never takes it). The words are RAW Q5 tile
/// units (tile·32) on the fire path: the mine/grenade spawn math
/// subtracts the muzzle in pos>>8 = Q5 [§7j.37]. 14 payload bytes
/// (byte 13 padding; the ring stride is host-side only — EXW 0x80,
/// the capgen append zero-extends whatever it writes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRecord {
    pub marker: u8,
    pub id: i16,
    pub spot: i16,
    pub flags: u8,
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl CommandRecord {
    /// Parse the 14-byte payload form. `None` = short payload (the
    /// canonical layer fails loud on it). The words +7/+9/+0xB
    /// (record bytes 7..13) are the triple; byte@+6 is the builder
    /// filler the consumer never reads; bytes 12/13 are the +0xD
    /// padding reachable in the original only through the
    /// bit0∧bit1 pointer-bump quirk (see `consume_commands`).
    pub fn from_payload(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 14 {
            return None;
        }
        Some(CommandRecord {
            marker: bytes[0],
            id: i16::from_le_bytes([bytes[1], bytes[2]]),
            spot: i16::from_le_bytes([bytes[3], bytes[4]]),
            flags: bytes[5],
            x: i16::from_le_bytes([bytes[7], bytes[8]]),
            y: i16::from_le_bytes([bytes[9], bytes[10]]),
            z: i16::from_le_bytes([bytes[11], bytes[12]]),
        })
    }
}

/// What one firing slot did — the dispatch-routing surface the tests
/// pin (the family cases are routed but their spawn internals are
/// the documented E-gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireDispatch {
    /// No case: gates failed (mask/cooldown/ammo) or id outside
    /// 2..=0x28.
    NoFire,
    /// Inline spawn(s) into the weapon bank: (count, type).
    Spawn(usize, u16),
    /// The AI-order family routing: (family, count-arg). The spawn
    /// body is the E-gap; the bookkeeping DOES run [hypothesis:
    /// mirrors the inline cases].
    Family(&'static str, i32),
}

impl MissionSim {
    /// Stage one COMMAND record payload into the ring (the W5 seam —
    /// the SAME grammar the O1 capgen injector appends at the ring).
    /// Returns false on a short payload (< 14 B).
    pub fn stage_command(&mut self, payload: &[u8]) -> bool {
        match CommandRecord::from_payload(payload) {
            Some(rec) => {
                self.commands.push(rec);
                true
            }
            None => false,
        }
    }

    /// Stage one already-parsed COMMAND record (the host/test seam
    /// behind [`MissionSim::stage_command`]).
    pub fn stage_command_record(&mut self, rec: CommandRecord) {
        self.commands.push(rec);
    }

    /// Pending (unconsumed) command records.
    pub fn pending_commands(&self) -> usize {
        self.commands.len()
    }

    /// The weapon-anim bank (T2 watch surface; slot index = the
    /// original record order).
    pub fn weapon_bank(&self) -> &[WeaponRecord] {
        &self.weapon_bank
    }

    /// Mutable bank access — the host/test STAGING seam (the
    /// robots_mut pattern): normal gameplay stages records only
    /// through the COMMAND dispatch.
    pub fn weapon_bank_mut(&mut self) -> &mut [WeaponRecord] {
        &mut self.weapon_bank
    }

    /// The 50-slot projectile bank (T2 watch surface).
    pub fn enemy_bank(&self) -> &[EnemyProjectile] {
        &self.enemy_bank
    }

    /// Mutable projectile-bank access (host/test staging seam).
    pub fn enemy_bank_mut(&mut self) -> &mut [EnemyProjectile] {
        &mut self.enemy_bank
    }

    /// The ORDER-target triple 0x4dd484/88/8c (bit1 records write
    /// it; the weapon dispatch aims at it).
    pub fn command_order_target(&self) -> (i32, i32, i32) {
        self.order_target
    }

    /// The order-active flag 0x4dc6bc (bit1 sets it).
    pub fn command_order_active(&self) -> bool {
        self.order_flag
    }

    /// Stage one robot's weapon slots + enable mask (the +0x36.. /
    /// +0x6E robot-record fields). [design] The original fills them
    /// at spawn via the session-table stats copy; the host seam
    /// stages them post-spawn (the D51 set_weapon_loadout pattern —
    /// the sidebar table stays presentation-side).
    pub fn stage_robot_weapons(&mut self, idx: usize, slots: [WeaponSlot; 7], mask: u16) -> bool {
        if let Some(r) = self.robots.get_mut(idx) {
            r.weapons = slots;
            r.weapon_mask = mask;
            true
        } else {
            false
        }
    }

    /// Stage one enemy projectile directly (test/future-producer
    /// seam into the 0x4cc654 bank — the real producers are the
    /// critter/turret fire family, an E-gap). Returns the slot.
    pub fn stage_enemy_projectile(&mut self, rec: EnemyProjectile) -> Option<usize> {
        let slot = self.enemy_free_slot()?;
        self.enemy_bank[slot] = rec;
        Some(slot)
    }

    /// FUN_0041286f: first free slot of the 50×0x22 bank (the
    /// critter fire family's producer seam, W12-S8).
    pub(crate) fn enemy_free_slot(&self) -> Option<usize> {
        self.enemy_bank.iter().position(|r| r.kind == 0)
    }

    /// FUN_00412848: first free slot of the 400×0x36 bank.
    fn weapon_free_slot(&self) -> Option<usize> {
        self.weapon_bank.iter().position(|r| r.kind == 0)
    }

    /// The COMMAND-record consumer — FUN_00409138's modeled subset
    /// [verified §7j.37/1]. MissionShell runs it after the click
    /// dispatcher + builder, before the six robot phases; the
    /// RECHARGE PASS runs once per frame for every robot × 7 slots
    /// (enabled ∧ cooldown≠0 → −−) even with an empty ring.
    ///
    /// SP model: robot = the record's id field (per-player id bases
    /// are MP-only, D89); the spot writes (cursor angle 0x4dc678 +
    /// robot +0x14 frame-base) are presentation — unmodeled; bit4
    /// (FUN_00449c82/0041c9f0) is unpinned — unmodeled; the MP mask
    /// word write is unmodeled; the idle AI ticks (deploy-delay ≠ 0
    /// ∧ frame&3==0) are the unpinned families — unmodeled.
    pub fn consume_commands(&mut self) {
        let records = std::mem::take(&mut self.commands);
        for rec in records {
            self.apply_command_record(rec);
        }
        // The recharge pass (loop-exit tail, every frame): enabled
        // slots with nonzero cooldown decrement [verified — no id
        // gate in the decompile].
        for r in &mut self.robots {
            for k in 0..WEAPON_SLOTS {
                if r.weapon_mask & (1 << k) != 0 && r.weapons[k].cooldown != 0 {
                    r.weapons[k].cooldown -= 1;
                }
            }
        }
    }

    fn apply_command_record(&mut self, rec: CommandRecord) {
        let robot = i32::from(rec.id);
        // SP id base 0 (D89); an out-of-range id is a staging error —
        // the original would index OOB memory here, E skips [design
        // guard, fail-loud upstream at the canonical layer is not
        // possible: the record grammar carries raw ids].
        if robot < 0 || robot as usize >= self.robots.len() {
            return;
        }
        let idx = robot as usize;
        self.hints.command(rec.flags);

        // flags bit0 SELECT [verified]: move-target words (raw Q5)
        // when state ∉ {3,4,5}; auto-arm state := 1 + stop 10^6 when
        // alive ∧ state ∉ {2,3,4,5}.
        if rec.flags & 1 != 0 {
            let r = &mut self.robots[idx];
            if !matches!(r.state, 3..=5) {
                r.target = Some((i32::from(rec.x), i32::from(rec.y)));
            }
            if r.alive && !matches!(r.state, 2..=5) {
                r.state = 1;
                r.stop_dist = 1_000_000;
            }
        }
        // flags bit1 ORDER [verified]: the triple + the fire arm.
        // QUIRK [decompile-faithful]: with bit0 also set the
        // original's word pointer has advanced +2 shorts, so the
        // triple reads the record's +0xB/+0xD/+0xF words — the two
        // extra payload words E does not model as fields; the quirk
        // therefore shifts the triple SOURCE but E can only stage
        // the +7/+9/+0xB words (documented divergence: scenarios
        // must not set bit0∧bit1 on one record to stay inside the
        // modeled grammar).
        if rec.flags & 2 != 0 {
            self.order_target = (i32::from(rec.x), i32::from(rec.y), i32::from(rec.z));
            self.order_flag = true;
            let (alive, state, kind) = {
                let r = &self.robots[idx];
                (r.alive, r.state, r.kind)
            };
            // Fire gates: alive ∧ deploy-delay(+0xA0, unmodeled = 0)
            // ∧ state ≠ 2; player-kind → sidebar redraw (presentation).
            if alive && state != 2 {
                let _ = kind; // the redraw countdown is presentation-side
                self.fire_robot_weapons(idx);
            }
        }
    }

    /// The 7-slot weapon loop of the bit1 arm [verified §7j.37/1]:
    /// per slot, fire iff (enable mask bit) ∧ cooldown == 0 ∧ ammo ≠
    /// 0; then the auto-rearm when the mask emptied this pass.
    fn fire_robot_weapons(&mut self, idx: usize) {
        let mut fired_any = false;
        for k in 0..WEAPON_SLOTS {
            let bit = 1u16 << k;
            let (id, ammo, cd, enabled) = {
                let r = &self.robots[idx];
                let s = r.weapons[k];
                (s.id, s.ammo, s.cooldown, r.weapon_mask & bit != 0)
            };
            if !enabled || cd != 0 || ammo == 0 {
                continue;
            }
            if !(2..=0x28).contains(&id) {
                // Gates passed, id outside the switch: still marks
                // the pass as fired (bVar3) [verified].
                fired_any = true;
                continue;
            }
            fired_any = true;
            self.fire_slot(idx, k, id);
        }
        // AUTO-REARM [verified]: mask emptied ∧ something fired →
        // first slot with id≠0 ∧ ammo≠0 gets its bit (the SFX
        // message family 0x1C..0x21 is T4 — unmodeled).
        if self.robots[idx].weapon_mask == 0 && fired_any {
            for k in 0..WEAPON_SLOTS {
                let r = &self.robots[idx];
                if r.weapons[k].id != 0 && r.weapons[k].ammo != 0 {
                    self.robots[idx].weapon_mask |= 1 << k;
                    break;
                }
            }
        }
    }

    /// One firing slot: the inline spawn cases verbatim from the
    /// dispatch decompile; the AI-order families routed only.
    fn fire_slot(&mut self, idx: usize, k: usize, id: u16) -> FireDispatch {
        match id {
            // w9..0xB ARTILLERY: 1× type = id, no velocity, cooldown
            // 0, mask bit cleared UNCONDITIONALLY [verified].
            9..=0xB => {
                if let Some(slot) = self.weapon_free_slot() {
                    let (px, py, pz) = self.muzzle(idx);
                    self.weapon_bank[slot] = WeaponRecord {
                        kind: id,
                        owner: idx as i32,
                        x: px + 0x100,
                        y: py + 0x100,
                        z: (pz + 0x15) * 0x100,
                        ..WeaponRecord::default()
                    };
                    self.spend_ammo(idx, k, 0, true);
                } else {
                    // No free slot: the original still ran its
                    // bookkeeping? The free-slot −1 branch skips the
                    // record writes AND the bookkeeping [verified:
                    // `if ((int)uVar19 != -1)` wraps everything].
                    return FireDispatch::NoFire;
                }
                FireDispatch::Spawn(1, id)
            }
            // w0x10..0x12 PROXIMITY MINES: 2/4/6× type 0xF, vel>>2.
            0x10..=0x12 => {
                let n = match id {
                    0x10 => 2,
                    0x11 => 4,
                    _ => 6,
                };
                self.spawn_mine_burst(idx, k, n, 0xF, 2);
                FireDispatch::Spawn(n, 0xF)
            }
            // w0x14..0x16 PRESSURE MINES: 2/4/6× type 0x13, vel>>1.
            0x14..=0x16 => {
                let n = match id {
                    0x14 => 2,
                    0x15 => 4,
                    _ => 6,
                };
                self.spawn_mine_burst(idx, k, n, 0x13, 1);
                FireDispatch::Spawn(n, 0x13)
            }
            // w0x1B/0x1C BOUNCY GRENADES: 4/6× type 0x1A.
            0x1B | 0x1C => {
                let n = if id == 0x1B { 4 } else { 6 };
                self.spawn_grenade_burst(idx, k, n, 0x1A);
                FireDispatch::Spawn(n, 0x1A)
            }
            // w0x1D/0x1E STICKY GRENADES: 4/6× type 0x1F.
            0x1D | 0x1E => {
                let n = if id == 0x1D { 4 } else { 6 };
                self.spawn_grenade_burst(idx, k, n, 0x1F);
                FireDispatch::Spawn(n, 0x1F)
            }
            // w0x20 ROCKET PACK X1: 1× type 0x24, no jitter,
            // cooldown 5, arc = the angle pair [verified].
            0x20 => {
                if let Some(slot) = self.spawn_straight(idx, 0x24) {
                    let arc = {
                        let r = &self.robots[idx];
                        let (tx, ty, _) = self.order_target;
                        let dx = (tx * 0x100) - (r.pos_x & !0xFF);
                        let dy = (ty * 0x100) - (r.pos_y & !0xFF);
                        self.angles.angle_byte(dx, dy) as i32
                    };
                    self.weapon_bank[slot].arc = arc;
                    self.spend_ammo(idx, k, 5, false);
                }
                FireDispatch::Spawn(1, 0x24)
            }
            // The needler family: routed, spawn internals are the
            // documented E-gap. Bookkeeping runs [hypothesis:
            // mirrors the inline cases — the family bodies are
            // unpinned; the differ's cadence watch will arbitrate].
            2..=4 => {
                let n = match id {
                    2 => 3,
                    3 => 2,
                    _ => 1,
                };
                self.spend_ammo(idx, k, 8, false);
                FireDispatch::Family("FUN_0040b615", n)
            }
            6..=8 => FireDispatch::Spawn(self.spawn_plasma_burst(idx, k, usize::from(id - 5)), 5),
            0xE => {
                self.spend_ammo(idx, k, 8, false);
                FireDispatch::Family("FUN_0040a9ff", 1)
            }
            0x18 | 0x19 => {
                let n = if id == 0x18 { 1 } else { 2 };
                self.spend_ammo(idx, k, 8, false);
                FireDispatch::Family("FUN_0040a56f", n)
            }
            0x21..=0x23 => {
                let n = match id {
                    0x21 => 3,
                    0x22 => 6,
                    _ => 9,
                };
                self.spend_ammo(idx, k, 8, false);
                FireDispatch::Family("FUN_0040ace8", n)
            }
            0x25..=0x28 => {
                let n = match id {
                    0x25 => 1,
                    0x26 => 2,
                    0x27 => 4,
                    _ => 6,
                };
                self.spend_ammo(idx, k, 8, false);
                FireDispatch::Family("FUN_0040a7a1", n)
            }
            _ => FireDispatch::NoFire,
        }
    }

    /// Normal command entry of FUN_0040af98. Weapon IDs 6/7/8
    /// emit 1/2/3 type-5 shots (RE-EXW-PLASMA.md).
    fn spawn_plasma_burst(&mut self, idx: usize, k: usize, count: usize) -> usize {
        let mut spawned = 0;
        for shot in 0..count {
            let Some(slot) = self.weapon_free_slot() else {
                break;
            };
            if shot != 0 && self.robots[idx].weapons[k].ammo <= 0 {
                break;
            }
            self.spend_ammo(idx, k, 2, false);
            let robot = &self.robots[idx];
            let (x, y, z) = (robot.pos_x, robot.pos_y, (robot.z + 21) << 8);
            let (mut tx, mut ty, tz) = self.order_target;
            if shot != 0 {
                tx += (self.rand_a() & 31) as i32 - 16;
                ty += (self.rand_a() & 31) as i32 - 16;
            }
            let dx = tx - (x >> 8);
            let dy = ty - (y >> 8);
            let den = (dist_octagonal(dx << 8, dy << 8) / 8).max(1);
            let vx = (dx << 16) / den;
            let vy = (dy << 16) / den;
            let vz = if tz == 0 {
                0
            } else {
                ((tz - (z >> 8)) << 16) / den
            };
            if vx == 0 && vy == 0 {
                break;
            }
            // The guest leaves unrelated fields from the prior occupant intact.
            let rec = &mut self.weapon_bank[slot];
            rec.kind = 5;
            rec.owner = idx as i32;
            rec.tick = 0;
            rec.draw_ctr = 0;
            rec.x = x;
            rec.y = y;
            rec.z = z;
            rec.vx = vx;
            rec.vy = vy;
            rec.vz = vz;
            spawned += 1;
        }
        spawned
    }

    /// Muzzle position of a robot (Q13 pos, Q5 z).
    fn muzzle(&self, idx: usize) -> (i32, i32, i32) {
        let r = &self.robots[idx];
        (r.pos_x, r.pos_y, r.z)
    }

    /// The inline ammo/cooldown/mask bookkeeping [verified]:
    /// ammo−1; 0 → mask bit clear; cooldown := cd.
    fn spend_ammo(&mut self, idx: usize, k: usize, cd: u16, unconditional_disarm: bool) {
        let bit = 1u16 << k;
        let r = &mut self.robots[idx];
        r.weapons[k].ammo -= 1;
        if r.weapons[k].ammo == 0 || unconditional_disarm {
            r.weapon_mask &= !bit;
        }
        r.weapons[k].cooldown = cd;
    }

    /// The mine spawn burst (w0x10..0x16) — per record: the free-slot
    /// gate, then ammo>0, then the bookkeeping, then the aim [the
    /// decompile order: `free ≠ −1 ∧ ammo > 0` guards everything
    /// inside the loop]; 2× RandA jitter ±0x20 on the order target,
    /// octile>>3 normalization, vel>>shift, ttl RandA&0xF+1, arc
    /// 0x900−RandA&0x2FF, class 4 [verified §7j.37/1; 4 RandA draws
    /// per record]. A degenerate aim (vx==vy==0) skips the record
    /// AFTER the bookkeeping and ends the burst.
    fn spawn_mine_burst(&mut self, idx: usize, k: usize, count: usize, kind: u16, shift: u32) {
        for _ in 0..count {
            if self.robots[idx].weapons[k].ammo <= 0 {
                break;
            }
            if self.weapon_free_slot().is_none() {
                break;
            }
            self.spend_ammo(idx, k, 8, false);
            if self
                .spawn_aimed(idx, kind, shift, true, Some(0x900))
                .is_none()
            {
                break;
            }
        }
    }

    /// The grenade burst (w0x1B..0x1E): 3D velocity from the order
    /// z, ttl 0x32∓/＋RandA&0xF, arc 0xB00− (0x1A) / 0x900−
    /// (0x1F) RandA&0x2FF, class 0, trail := 0 [verified].
    fn spawn_grenade_burst(&mut self, idx: usize, k: usize, count: usize, kind: u16) {
        for _ in 0..count {
            if self.robots[idx].weapons[k].ammo <= 0 {
                break;
            }
            if self.weapon_free_slot().is_none() {
                break;
            }
            self.spend_ammo(idx, k, 8, false);
            let arc_base = if kind == 0x1A { 0xB00 } else { 0x900 };
            match self.spawn_aimed(idx, kind, 0, false, Some(arc_base)) {
                Some(slot) => self.weapon_bank[slot].trail = 0,
                None => break,
            }
        }
    }

    /// One aimed spawn at the (optionally jittered) order target —
    /// the shared velocity math of the inline spawners [verified
    /// §7j.37/1]. `arc_rng` = the mine/grenade arc draw range base
    /// (0x900/0xB00); the rocket passes `None` (its arc is the angle
    /// pair, no draw). Returns the bank slot (None = degenerate aim:
    /// vx==vy==0 → no record).
    fn spawn_aimed(
        &mut self,
        idx: usize,
        kind: u16,
        shift: u32,
        jitter: bool,
        arc_rng: Option<i32>,
    ) -> Option<usize> {
        let (px, py, pz) = self.muzzle(idx);
        let (tx0, ty0, tz) = self.order_target;
        let (jx, jy) = if jitter {
            (
                (self.rand_a() & 0x3F) as i32 - 0x20 + tx0,
                (self.rand_a() & 0x3F) as i32 - 0x20 + ty0,
            )
        } else {
            (tx0, ty0)
        };
        // Octile over the Q13 delta; floor-div 8, clamped ≥ 1.
        let dx_q13 = (jx - (px >> 8)) * 0x100;
        let dy_q13 = (jy - (py >> 8)) * 0x100;
        let mut den = dist_octagonal(dx_q13, dy_q13);
        if den == 0 {
            den = 1;
        }
        den >>= 3;
        if den == 0 {
            den = 1;
        }
        let mut vx = ((jx - (px >> 8)) * 0x10000) / den;
        let mut vy = ((jy - (py >> 8)) * 0x10000) / den;
        if shift > 0 {
            vx >>= shift;
            vy >>= shift;
        }
        if vx == 0 && vy == 0 {
            return None;
        }
        // vz: 3D only for the grenade/rocket set (order z ≠ 0).
        let vz = if tz != 0 {
            ((tz - (pz + 0x15)) * 0x10000) / den
        } else {
            0
        };
        let ttl = match kind {
            0x1A => 0x32 - (self.rand_a() & 0xF) as i32,
            0x1F => 0x32 + (self.rand_a() & 0xF) as i32,
            // The rocket spawns with tick 0 and NO RandA draw (its
            // arc is the angle pair, not the jitter family).
            0x24 => 0,
            _ => (self.rand_a() & 0xF) as i32 + 1,
        };
        let arc = match arc_rng {
            Some(base) => base - (self.rand_a() & 0x2FF) as i32,
            None => 0,
        };
        let class = if matches!(kind, 0xF | 0x13) { 4 } else { 0 };
        let slot = self.weapon_free_slot()?;
        self.weapon_bank[slot] = WeaponRecord {
            kind,
            owner: idx as i32,
            x: px,
            y: py,
            z: (pz + 0x15) * 0x100,
            vx,
            vy,
            vz,
            tick: ttl,
            arc,
            class,
            ..WeaponRecord::default()
        };
        Some(slot)
    }

    /// The straight-velocity spawn of the rocket (no jitter, no arc
    /// draw, ttl 0 — the caller stamps the angle-pair arc).
    fn spawn_straight(&mut self, idx: usize, kind: u16) -> Option<usize> {
        self.spawn_aimed(idx, kind, 0, false, None)
    }

    /// The WEAPON-ANIM TICK — FUN_00410823's modeled subset
    /// [verified §7j.37 items 3..6 + 7j.22]. One walk over the whole
    /// bank per call; MissionShell calls it 4×/frame with phase
    /// args 0..3 (types 9..0xB tick on phase 0 only; the actor
    /// hit-test lanes run on odd phases — E has no actors: the
    /// critter bank is an E-gap and the robot lane is MP-only, so
    /// the lanes are structural no-ops in the SP model).
    pub fn weapon_tick(&mut self, phase: i32) {
        let (map_w, map_h) = self.terrain.size();
        for i in 0..WEAPON_BANK_SLOTS {
            if !self.weapon_bank[i].active() {
                continue;
            }
            let kind = self.weapon_bank[i].kind;
            match kind {
                2..=4 => self.tick_bullet(i, map_w, map_h, phase),
                5 => self.tick_shell(i, map_w, map_h, phase),
                9..=0xB => {
                    if phase == 0 {
                        self.tick_artillery(i, kind);
                    }
                }
                0xE | 0xF | 0x13 | 0x17 | 0x1A | 0x1F => self.tick_ballistic(i, map_w, map_h),
                0x24 => self.tick_rocket(i, map_w, map_h),
                0x29 => self.tick_homing(i, map_w, map_h, phase),
                _ => {} // no tick in this function [7j.22/6]
            }
        }
    }

    /// Types 2..4 BULLETS [verified §7j.37/3]: up to 2 tested
    /// sub-steps per call, then a one-step rollback; hits re-add the
    /// step; the record frees ONLY at tick > 99 (impacts do not kill
    /// bullets — the impact pair + disburser applications are the
    /// S4/T4 E-gaps). The CRITTER actor lane (FUN_004190bc) runs on
    /// the odd phases at the substep test position (W12-S8): a hit
    /// applies the per-weapon damage/death lane; the record itself
    /// keeps flying exactly like the original (the lane does not
    /// free it).
    fn tick_bullet(&mut self, i: usize, map_w: i32, map_h: i32, phase: i32) {
        const NO_RESULT: i32 = 0;
        const BOUNDS: i32 = 1;
        const TERRAIN: i32 = 2;
        const DONE: i32 = 5;
        let (vx, vy, vz) = {
            let r = &self.weapon_bank[i];
            (r.vx, r.vy, r.vz)
        };
        let mut x = self.weapon_bank[i].x;
        let mut y = self.weapon_bank[i].y;
        let mut z = self.weapon_bank[i].z;
        let mut result = NO_RESULT;
        let mut substeps = 0;
        while result == NO_RESULT && self.weapon_bank[i].tick < 100 {
            x += vx;
            self.weapon_bank[i].tick += 2;
            y += vy;
            z += vz;
            if substeps < 2 {
                if x < 0
                    || y < 0
                    || x >> 13 >= map_w
                    || y >> 13 >= map_h
                    || z >> 13 > 7
                    || z < 0
                    || self.weapon_bank[i].tick > 99
                {
                    result = BOUNDS;
                } else {
                    // Actor lanes: the critter lane (FUN_004190bc,
                    // odd phases — the modeled W12-S8 surface) at the
                    // substep test position; the MP robot lane stays
                    // the SP-model no-op.
                    let floor = self.terrain.floor_z(x >> 8, y >> 8, z >> 8);
                    if z >> 8 < floor {
                        result = TERRAIN;
                    } else if phase & 1 == 1 {
                        let (wk, wo) = (self.weapon_bank[i].kind, self.weapon_bank[i].owner as i16);
                        self.critter_hit_test(x >> 8, y >> 8, z >> 8, wk, wo as i32);
                    }
                }
            } else {
                result = DONE;
            }
            substeps += 1;
        }
        // Rollback one step; hit paths re-add it.
        x -= vx;
        y -= vy;
        z -= vz;
        match result {
            BOUNDS => {
                self.weapon_bank[i].kind = 0;
                return;
            }
            TERRAIN => {
                // The impact pair + the disburser [§7j.37/3 +
                // §7j.39/2, raw asm 0x410ab9..0x4114eb]:
                // FUN_0041a894 (0x410ae0) then FUN_0041bc1c
                // (0x410af9), damage = FUN_00419aff(kind), score
                // armed (`push 1`); then the re-add commits the
                // position and the disburser stages K2 (2 jitter
                // draws) and FREES the record — the §7j.37/3
                // "no free on impact" gloss is corrected by the
                // raw asm (kind := 0 inside the disburser).
                let (hx, hy) = (x + vx, y + vy);
                let hz = z + vz;
                let dmg = weapon_damage(self.weapon_bank[i].kind, self.difficulty);
                self.resolve_object_impact(hx, hy, 0, dmg, true);
                self.resolve_structure_impact(hx, hy, dmg);
                self.weapon_bank[i].x = hx;
                self.weapon_bank[i].y = hy;
                self.weapon_bank[i].z = hz;
                self.weapon_disburser(i);
                return;
            }
            _ => {}
        }
        self.weapon_bank[i].x = x;
        self.weapon_bank[i].y = y;
        self.weapon_bank[i].z = z;
    }

    /// Type 5 SHELL [verified §7j.37/4]: one move; free on bounds/
    /// tick>100/z-OOB; floor hit → impact(75) + free; else commit +
    /// tick++. The critter-lane trail body is a structural no-op.
    fn tick_shell(&mut self, i: usize, map_w: i32, map_h: i32, phase: i32) {
        let (x, y, z, tick) = {
            let r = &mut self.weapon_bank[i];
            (r.x + r.vx, r.y + r.vy, r.z + r.vz, r.tick)
        };
        if x < 0
            || y < 0
            || x >= map_w * 0x2000
            || y >= map_h * 0x2000
            || tick > 100
            || !(0..=0xFFFF).contains(&z)
        {
            self.weapon_bank[i].kind = 0;
            return;
        }
        let _ = phase; // the odd-phase lane is the critter lane (E-gap)
        let floor = self.terrain.floor_z(x >> 8, y >> 8, z >> 8);
        if z >> 8 < floor {
            // The floor hit → impact pair (75) + disburser + FREE
            // [§7j.37/4 + §7j.39/2]: the resolvers run at the
            // UNCOMMITTED hit position; the disburser reads the
            // record's stored (one-step-back) position and frees.
            let dmg = weapon_damage(5, self.difficulty);
            self.resolve_object_impact(x, y, 0, dmg, true);
            self.resolve_structure_impact(x, y, dmg);
            self.weapon_bank[i].kind = 0;
            self.weapon_disburser(i);
            return;
        }
        let r = &mut self.weapon_bank[i];
        r.x = x;
        r.y = y;
        r.z = z;
        r.tick += 1;
    }

    /// Artillery burst durations indexed BY TYPE at 0x456c78+4·type
    /// [verified §7j.37/5]: w9→2, w0xA→4, w0xB→7 frames.
    fn artillery_duration(kind: u16) -> i32 {
        match kind {
            9 => 2,
            0xA => 4,
            _ => 7,
        }
    }

    /// Types 9..0xB ARTILLERY [verified §7j.37/5 + §7j.39/7]: fall
    /// 0x200/tick to the FUN_0041e411 settle; the burst window
    /// tick−0x20 < duration walks the scripted pair lists — per
    /// pair the SCRIPT BLAST FUN_004244a1 (5000 damage, flag 1 +
    /// its k6 1-in-8 gate draws) + the 50% RandA k11 gate (the
    /// staged k11 drawing its own SFX-gate draw); past the window
    /// → tick ≤ 0x22 exits silently (w9's exact-0x22 end), else
    /// the disburser tail (kind := 0 for 9..0xB).
    fn tick_artillery(&mut self, i: usize, kind: u16) {
        let r = &mut self.weapon_bank[i];
        r.tick += 1;
        let x = r.x;
        let y = r.y;
        let z = r.z;
        let floor = self.terrain.floor_z(x >> 8, y >> 8, z >> 8);
        if floor < z >> 8 {
            r.z = z - 0x200;
        } else {
            r.z = floor << 8;
        }
        let tick = r.tick;
        let owner = r.owner;
        if tick < 0x20 {
            // tick==0x18 ∧ player-kind → the wall-strip redraw
            // FUN_004245c9 (presentation — unmodeled).
            return;
        }
        if tick - 0x20 < Self::artillery_duration(kind) {
            // The burst pair walk [§7j.39/7]: list
            // PTR[0x456bf0+4·(tick−0x20)], (Δy,Δx) until the 500
            // sentinel; the blast at (x_tile+Δx, y_tile+Δy,
            // z_level) then the 50% k11 gate.
            let f = (tick - 0x20) as usize;
            let (xt, yt) = (x >> 13, y >> 13);
            let z_level = (((z >> 8) + 0x3F) >> 5).min(7);
            if let Some(pairs) = crate::destroy::ARTILLERY_PAIRS.get(f) {
                for &(dy, dx) in pairs.iter() {
                    self.script_blast(xt + dx as i32, yt + dy as i32, z_level);
                    if self.rand_a() & 1 != 0 {
                        self.stage_debris(
                            (xt << 5) + 0xF,
                            (yt << 5) + 0xF,
                            (z_level << 5) + 0x10,
                            11,
                            0,
                            owner,
                        );
                    }
                }
            }
        } else {
            // Past the window: tick ≤ 0x22 → the silent exit; the
            // disburser (which frees the record) runs late.
            if tick > 0x22 {
                self.weapon_disburser(i);
            }
        }
    }

    /// The BALLISTIC family {0xE, 0xF, 0x13, 0x17, 0x1A, 0x1F}
    /// [verified §7j.37 + the decompile walk]: axis-wall bounces,
    /// gravity arc, floor-contact semantics (roll / damped bounce /
    /// the 0x17 3-clone split / the 0xE scripted detonation), the
    /// scenery settle, the 0xE smoke-trail append (E-gap bank), and
    /// the class-cycle expiry.
    fn tick_ballistic(&mut self, i: usize, map_w: i32, map_h: i32) {
        let kind = self.weapon_bank[i].kind;
        let (vx, vy, arc, z, x, y) = {
            let r = &self.weapon_bank[i];
            (r.vx, r.vy, r.arc, r.z, r.x, r.y)
        };
        let new_x = x + vx;
        let new_y = y + vy;
        if new_x < 0
            || new_y < 0
            || new_x >= map_w * 0x2000
            || new_y >= map_h * 0x2000
            || !(0..=0xFFFF).contains(&z)
        {
            self.weapon_bank[i].kind = 0;
            // The bounds path clears the trail-ring slot (E-gap bank
            // — the clear is noted for the S3 trail row).
            return;
        }
        if self.weapon_bank[i].tick >= 0x65 {
            // EXPIRY [§7j.39/3, raw asm 0x410d5c..0x410eb2]:
            // 0xF/0x13 → tick := 0, class−−; class ≠ 0 → the
            // cycle re-arms; class == 0 → the disburser FIRST,
            // then 4× FUN_0041a894 at (x±0x1000, y±0x1000) in the
            // quadrant order (+,+),(+,−),(−,+),(−,−) with damage
            // FUN_00419aff(0x1a) score-armed (the 0x1a damage even
            // when the dying record is 0xF/0x13), then 4×
            // FUN_0041bc1c over the same quadrants, then the
            // trail-ring slot clear. The 0xF disburser is a raw-asm
            // NO-OP (the §7j.14 map corrected) — a 0xF mine does
            // NOT free here; the mine-proximity family (the
            // FUN_00410823 tail checks, §7j.39/8) re-opens it.
            if kind == 0xF || kind == 0x13 {
                self.weapon_bank[i].tick = 0;
                self.weapon_bank[i].class -= 1;
            }
            if self.weapon_bank[i].class == 0 {
                let x = self.weapon_bank[i].x;
                let y = self.weapon_bank[i].y;
                let dmg = weapon_damage(0x1a, self.difficulty);
                self.weapon_disburser(i);
                for (sx, sy) in [
                    (0x1000i32, 0x1000i32),
                    (0x1000, -0x1000),
                    (-0x1000, 0x1000),
                    (-0x1000, -0x1000),
                ] {
                    self.resolve_object_impact(x + sx, y + sy, 0, dmg, true);
                }
                for (sx, sy) in [
                    (0x1000i32, 0x1000i32),
                    (0x1000, -0x1000),
                    (-0x1000, 0x1000),
                    (-0x1000, -0x1000),
                ] {
                    self.resolve_structure_impact(x + sx, y + sy, dmg);
                }
                // The trail-ring slot clear writes the 0x4e66b8
                // ring (an E-gap bank); the record's own trail
                // word stays [faithful — the record is freed for
                // every kind but 0xF].
            }
            return;
        }
        // X WALL: floor(new_x, OLD y) > z → per-type bounce on X.
        let mut cx = new_x;
        if z < 0x10000 {
            let floor = self.terrain.floor_z(new_x >> 8, y >> 8, z >> 8);
            if z >> 8 < floor {
                cx = x; // not committed
                match kind {
                    0xE | 0x17 => self.weapon_bank[i].vx = -vx,
                    0xF | 0x13 | 0x1F => self.weapon_bank[i].full_stop(),
                    0x1A => self.weapon_bank[i].vx = -(vx >> 1),
                    _ => {}
                }
            }
        }
        self.weapon_bank[i].x = cx;
        // Y WALL: floor(committed x, new_y) > z → per-type on Y.
        let mut cy = new_y;
        if z < 0x10000 {
            let floor = self.terrain.floor_z(cx >> 8, new_y >> 8, z >> 8);
            if z >> 8 < floor {
                cy = y;
                match kind {
                    0xE | 0x17 => self.weapon_bank[i].vy = -vy,
                    0xF | 0x13 | 0x1F => self.weapon_bank[i].full_stop(),
                    0x1A => self.weapon_bank[i].vy = -(vy >> 1),
                    _ => {}
                }
            }
        }
        self.weapon_bank[i].y = cy;
        // Gravity + the z move.
        let new_arc = arc - 0x100;
        self.weapon_bank[i].arc = new_arc;
        let new_z = z + new_arc;
        if new_z < 0 {
            self.weapon_bank[i].kind = 0;
            return;
        }
        if new_z < 0x10000 {
            let (fx, fy) = (self.weapon_bank[i].x, self.weapon_bank[i].y);
            let floor = self.terrain.floor_z(fx >> 8, fy >> 8, new_z >> 8);
            if new_z >> 8 < floor {
                // FLOOR CONTACT. (The scenery settle — z := floor<<8
                // when the type-DB mirror byte ≠ 0 — needs the
                // 0x4796d5 mirror rows: E's Terrain has no
                // per-tile variant byte staged; unmodeled [E-gap,
                // noted for the S5 mirror-row pairing].)
                match kind {
                    0xF | 0x13 => {
                        // Damped roll.
                        self.weapon_bank[i].vx >>= 1;
                        self.weapon_bank[i].vy >>= 1;
                        self.weapon_bank[i].arc >>= 2;
                    }
                    0xE | 0x17 | 0x1A | 0x1F => {
                        // Vertical bounce: arc := −arc, then damped
                        // (−arc − (−arc>>1)) unless 0xE; horizontal
                        // halving unless 0x17.
                        let mut a = -self.weapon_bank[i].arc;
                        if kind != 0xE {
                            a -= a >> 1;
                        }
                        self.weapon_bank[i].arc = a;
                        if kind == 0x17 {
                            // 3-CLONE SPLIT: damped v rotated
                            // (vy,−vx)/(−vy,vx)/(−vx,−vy) into free
                            // slots, type 0x17, class 0 — the parent
                            // tick++ FIRST, the clones copy the
                            // incremented record [verified].
                            self.weapon_bank[i].tick += 1;
                            let base = self.weapon_bank[i];
                            let dvx = base.vx;
                            let dvy = base.vy;
                            let svx = dvx - (dvx >> 1);
                            let svy = dvy - (dvy >> 1);
                            self.weapon_bank[i].vx = svx;
                            self.weapon_bank[i].vy = svy;
                            for (cvx, cvy) in [(svy, -svx), (-svy, svx), (-svx, -svy)] {
                                if let Some(slot) = self.weapon_free_slot() {
                                    self.weapon_bank[slot] = WeaponRecord {
                                        kind: 0x17,
                                        vx: cvx,
                                        vy: cvy,
                                        class: 0,
                                        ..base
                                    };
                                }
                            }
                        } else {
                            self.weapon_bank[i].vx >>= 1;
                            self.weapon_bank[i].vy >>= 1;
                        }
                        if kind == 0xE {
                            // The 3-cell scripted detonation
                            // [§7j.39/4, raw asm 0x411298..0x411338]:
                            // at the committed post-wall position
                            // with the POST-halving velocities —
                            // (x,y), (x−vx·4, y−vy·4), (x−vx·4,
                            // y+vy·4), all >>13, z = z>>13.
                            let (bx, by, bz) = (
                                self.weapon_bank[i].x,
                                self.weapon_bank[i].y,
                                self.weapon_bank[i].z,
                            );
                            let vx4 = self.weapon_bank[i].vx * 4;
                            let vy4 = self.weapon_bank[i].vy * 4;
                            self.script_blast(bx >> 13, by >> 13, bz >> 13);
                            self.script_blast((bx - vx4) >> 13, (by - vy4) >> 13, bz >> 13);
                            self.script_blast((bx - vx4) >> 13, (by + vy4) >> 13, bz >> 13);
                        }
                    }
                    _ => {}
                }
                // z stays (the pre-arc value) on contact.
            } else {
                self.weapon_bank[i].z = new_z;
            }
        } else {
            self.weapon_bank[i].z = new_z;
        }
        // COMMON TAIL: tick++ (the 0xE trail append doubles it —
        // the ring bank is an E-gap, but the DOUBLE INCREMENT is
        // state the tick rows watch [verified decompile]).
        self.weapon_bank[i].tick += 1;
        if kind == 0xE && self.weapon_bank[i].trail != -1 && self.weapon_bank[i].tick & 1 != 0 {
            self.weapon_bank[i].tick += 1;
        }
    }

    /// Type 0x24 ROCKET [verified 7j.22/7]: class countdown (launch
    /// delay) → straight flight (no gravity); z<0 → z=0x1000 +
    /// disburser + free; floor → impact(400) + disburser + free;
    /// ttl>0x64 or OOB → free.
    fn tick_rocket(&mut self, i: usize, map_w: i32, map_h: i32) {
        let r = &mut self.weapon_bank[i];
        if r.class > 0 {
            r.class -= 1;
            return;
        }
        r.x += r.vx;
        r.y += r.vy;
        r.z += r.vz;
        r.tick += 1;
        if r.x < 0 || r.y < 0 || r.x >= map_w * 0x2000 || r.y >= map_h * 0x2000 || r.tick > 0x64 {
            r.kind = 0;
            return;
        }
        if r.z < 0 {
            // The z-OOB path [raw asm 0x411787]: z := 0x1000,
            // kind := 0, then a disburser call that reads the
            // already-zeroed kind — a structural no-op [faithful].
            r.z = 0x1000;
            r.kind = 0;
            return;
        }
        let (x, y, z) = (r.x, r.y, r.z);
        let floor = self.terrain.floor_z(x >> 8, y >> 8, z >> 8);
        if z >> 8 < floor {
            // The floor impact [§7j.39/2, raw asm 0x411899..]: the
            // OBJECT resolver (0x4118ad) → the STRUCTURE resolver
            // (0x4118c2) → the disburser (K6 + free).
            let dmg = weapon_damage(0x24, self.difficulty);
            self.resolve_object_impact(x, y, 0, dmg, true);
            self.resolve_structure_impact(x, y, dmg);
            self.weapon_bank[i].kind = 0;
            self.weapon_disburser(i);
        }
    }

    /// Type 0x29 HOMING [verified §7j.37/6]: class launch delay; z
    /// easing + ground lift; heading steering over the target delta
    /// (turn = angle-diff ×4); velocity 2·(sine lookups >>4); the
    /// forward probe + the LEFT-first ±4-sector avoidance; dead-
    /// target gates; floor → impact(250) + free; ttl>0xC8 or bounds
    /// → free. Target classes: bit 0x1000 robot (modeled), bit
    /// 0x2000 structure / else critter (E-gaps — no steering target
    /// until those banks land).
    fn tick_homing(&mut self, i: usize, map_w: i32, map_h: i32, phase: i32) {
        const TTL_MAX: i32 = 0xC8;
        let r = &mut self.weapon_bank[i];
        if r.class > 0 {
            r.class -= 1;
            return;
        }
        // Target resolution: the aim point per class.
        let t = r.target;
        let mut aim: Option<(i32, i32, i32)> = None;
        if t & 0x1000 != 0 {
            let idx = (t & 0xFFF) as usize;
            if let Some(tr) = self.robots.get(idx.wrapping_sub(1)) {
                if idx >= 1 && tr.alive {
                    aim = Some((tr.pos_x, tr.pos_y, (tr.z + 0x15) << 8));
                } else {
                    // Target-dead gate: disburser + fizzle.
                    r.kind = 0;
                    return;
                }
            } else {
                r.kind = 0;
                return;
            }
        }
        // The 0x2000 (structure) and critter classes have no E banks
        // yet: no steering target, the record flies its heading
        // [E-gap documented].
        let (x, y, z) = (r.x, r.y, r.z);
        let mut nz = z;
        if let Some((_, _, tz)) = aim {
            // z easing ±0x200 toward the target z.
            nz += (tz - z).clamp(-0x200, 0x200);
        }
        nz = nz.clamp(0, 0xFF00);
        {
            let floor = self.terrain.floor_z(x >> 8, y >> 8, nz >> 8);
            if (nz >> 8) - 4 <= floor {
                nz += 0x200;
            }
        }
        nz = nz.clamp(0, 0xFF00);
        let mut heading = r.arc & 0xFF;
        let mut vx = r.vx;
        let mut vy = r.vy;
        if let Some((tx, ty, _)) = aim {
            let dx = tx - (x & !0xFF);
            let dy = ty - (y & !0xFF);
            let target_angle = self.angles.angle_byte(dx, dy) as i32;
            // FUN_00412a19: the signed byte-diff, then ×4.
            let mut diff = (target_angle - heading) & 0xFF;
            if diff > 0x7F {
                diff -= 0x100;
            }
            heading = (heading + diff * 4) & 0xFF;
            if let (Some(c), Some(s)) = (
                self.angles.sine_word(heading as u16),
                self.angles.sine_word(((heading - 0x40) & 0xFF) as u16),
            ) {
                vx = 2 * (c as i32 >> 4);
                vy = 2 * (s as i32 >> 4);
            }
        }
        // Forward probe + the avoidance loop.
        let mut new_x = x + vx * 2;
        let mut new_y = y + vy * 2;
        let blocked = new_x < 0 || new_y < 0 || new_x >> 13 >= map_w || new_y >> 13 >= map_h || {
            let f = self.terrain.floor_z(new_x >> 8, new_y >> 8, nz >> 8);
            (nz >> 8) <= f
        };
        if blocked {
            let mut chosen = false;
            let mut off = 0i32;
            while off < 0x40 && !chosen {
                for sign in [-1i32, 1] {
                    let cand = (heading + sign * off) & 0xFF;
                    let (cvx, cvy) = match (
                        self.angles.sine_word(cand as u16),
                        self.angles.sine_word(((cand - 0x40) & 0xFF) as u16),
                    ) {
                        (Some(c), Some(s)) => (2 * (c as i32 >> 4), 2 * (s as i32 >> 4)),
                        _ => (vx, vy),
                    };
                    let cx = x + cvx * 2;
                    let cy = y + cvy * 2;
                    let oob = cx < 0 || cy < 0 || cx >> 13 >= map_w || cy >> 13 >= map_h;
                    if oob {
                        if sign < 0 {
                            nz += 0x600; // the LEFT leg climbs [verified]
                        }
                        heading = cand;
                        new_x = cx;
                        new_y = cy;
                        chosen = true;
                        break;
                    }
                    let f = self.terrain.floor_z(cx >> 8, cy >> 8, nz >> 8);
                    if f < nz >> 8 {
                        heading = cand;
                        new_x = cx;
                        new_y = cy;
                        chosen = true;
                        break;
                    }
                }
                off += 4;
            }
        }
        r.arc = heading;
        r.vx = vx;
        r.vy = vy;
        r.x = new_x;
        r.y = new_y;
        r.z = nz;
        r.tick += 1;
        if r.tick > TTL_MAX
            || new_x < 0
            || new_y < 0
            || new_x >> 13 >= map_w
            || new_y >> 13 >= map_h
        {
            r.kind = 0;
            return;
        }
        let _ = phase; // the odd-phase lanes are E-gap no-ops
        let floor = self.terrain.floor_z(new_x >> 8, new_y >> 8, nz >> 8);
        if nz >> 8 < floor {
            // The floor impact [§7j.39/2 — the order REVERSED vs
            // 0x24, raw asm 0x411f0e..0x411f4f]: the STRUCTURE
            // resolver (0x411f24) → the OBJECT resolver (0x411f3f)
            // → the disburser (K9 + free).
            let dmg = weapon_damage(0x29, self.difficulty);
            self.resolve_structure_impact(new_x, new_y, dmg);
            self.resolve_object_impact(new_x, new_y, 0, dmg, true);
            self.weapon_bank[i].kind = 0;
            self.weapon_disburser(i);
        }
    }

    /// The 50×0x22 PROJECTILE TICK — FUN_00412010's modeled subset
    /// [7j.13/5]: per call x+=vx, y+=vy, z+=vz; deactivate on bounds
    /// exit; terrain probe FUN_0041eaa1(z); the impact branches on
    /// the −0x64-normalized type (0x65 → damage(0x65) application —
    /// the S4 E-gap; 0x66 → + free; 0x67 → free) [the normalization
    /// is the §7j.37 hypothesis reconciling the 7j.13 "type 1/2/3"
    /// branches with the 7j.28 draw dispatch]. Called 4×/frame by
    /// the shell (the enemy pass).
    pub fn enemy_tick(&mut self) {
        let (map_w, map_h) = self.terrain.size();
        for i in 0..ENEMY_BANK_SLOTS {
            let r = &mut self.enemy_bank[i];
            if r.kind == 0 {
                continue;
            }
            r.x += r.vx;
            r.y += r.vy;
            r.z += r.vz;
            if r.x < 0 || r.y < 0 || r.x >= map_w * 0x2000 || r.y >= map_h * 0x2000 {
                r.kind = 0;
                continue;
            }
            // Terrain probe: hit when the projectile is below the
            // floor surface.
            let floor = self.terrain.floor_z(r.x >> 8, r.y >> 8, r.z >> 8);
            if r.z >> 8 < floor {
                // The impact branches [§7j.13/5 + §7j.14/4]: the
                // disburser FUN_004126dc FIRST (k20/k8/k4 — each
                // clears the type word), then the resolvers. The
                // score flag follows the weapon-site push-1
                // convention [hypothesis — no E producer covers
                // the 0x22 bank yet; a live record names it].
                let (x, y, kind) = (r.x, r.y, r.kind);
                match kind {
                    0x66 => {
                        self.projectile_disburser(i);
                        let dmg = weapon_damage(0x66, self.difficulty);
                        self.resolve_object_impact(x, y, 0, dmg, true);
                        self.resolve_structure_impact(x, y, dmg);
                    }
                    0x67 => {
                        self.projectile_disburser(i);
                    }
                    0x65 => {
                        self.projectile_disburser(i);
                        // type 0x65 does NOT deactivate on terrain
                        // [7j.13/5: no free on the 0x65 branch] —
                        // the disburser's clear makes this a
                        // presentational no-op for 0x65 [the
                        // FUN_004126dc kind map: 0x65 → K0x14 +
                        // clear; the "no deactivate" reading is the
                        // §7j.37 normalization hypothesis's open
                        // edge — S4+ coverage names it].
                        let dmg = weapon_damage(0x65, self.difficulty);
                        self.resolve_object_impact(x, y, 0, dmg, true);
                    }
                    _ => {}
                }
            }
        }
    }
}

trait WeaponRecordExt {
    fn full_stop(&mut self);
}

impl WeaponRecordExt for WeaponRecord {
    /// The 0xF/0x13/0x1F wall-contact stop: v = 0, arc = 0.
    fn full_stop(&mut self) {
        self.vx = 0;
        self.vy = 0;
        self.vz = 0;
        self.arc = 0;
    }
}

#[cfg(test)]
mod plasma_tests {
    use super::*;
    use crate::mission::{AngleTable, Terrain};

    fn armed(ammo: i16) -> MissionSim {
        let terrain = Terrain::from_parts(16, 16, vec![0; 8 * 16 * 16], vec![[0; 1024]]).unwrap();
        let angles = AngleTable::from_thresholds(&[0; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, 123);
        sim.spawn_robot((4, 4, 1));
        sim.robots[0].weapons[0] = WeaponSlot {
            id: 8,
            ammo,
            cooldown: 0,
        };
        sim.robots[0].weapon_mask = 1;
        let robot = &sim.robots[0];
        sim.order_target = ((robot.pos_x >> 8) + 32, robot.pos_y >> 8, 0);
        sim
    }

    #[test]
    fn plasma_burst_spawns_shells_and_spends_per_shot() {
        let mut sim = armed(10);
        assert_eq!(sim.fire_slot(0, 0, 8), FireDispatch::Spawn(3, 5));
        let shots: Vec<_> = sim.weapon_bank.iter().filter(|r| r.active()).collect();
        assert_eq!(shots.len(), 3);
        assert_eq!((shots[0].vx, shots[0].vy, shots[0].vz), (2048, 0, 0));
        assert_eq!(shots[0].z, (sim.robots[0].z + 21) << 8);
        assert!(shots
            .iter()
            .all(|r| r.kind == 5 && r.owner == 0 && r.tick == 0 && r.draw_ctr == 0));
        assert_eq!(sim.robots[0].weapons[0].ammo, 7);
        assert_eq!(sim.robots[0].weapons[0].cooldown, 2);
        let mut expected_rng = armed(10);
        for _ in 0..4 {
            expected_rng.rand_a();
        }
        assert_eq!(
            sim.rand_a_state(),
            expected_rng.rand_a_state(),
            "only extras draw x/y jitter"
        );
    }

    #[test]
    fn plasma_capacity_and_ammo_gates_do_not_invent_shots() {
        let mut sim = armed(1);
        assert_eq!(sim.fire_slot(0, 0, 8), FireDispatch::Spawn(1, 5));
        assert_eq!(sim.robots[0].weapons[0].ammo, 0);
        assert_eq!(sim.robots[0].weapon_mask, 0);
        let mut sim = armed(10);
        for shot in &mut sim.weapon_bank {
            shot.kind = 5;
        }
        let rng_before = sim.rand_a_state();
        assert_eq!(sim.fire_slot(0, 0, 8), FireDispatch::Spawn(0, 5));
        assert_eq!(sim.robots[0].weapons[0].ammo, 10);
        assert_eq!(sim.robots[0].weapons[0].cooldown, 0);
        assert_eq!(sim.rand_a_state(), rng_before);
    }

    #[test]
    fn plasma_degenerate_aim_spends_once_and_keeps_unwritten_slot_fields() {
        let mut sim = armed(10);
        let robot = &sim.robots[0];
        sim.order_target = (robot.pos_x >> 8, robot.pos_y >> 8, 0);
        assert_eq!(sim.fire_slot(0, 0, 8), FireDispatch::Spawn(0, 5));
        assert_eq!(sim.robots[0].weapons[0].ammo, 9);
        assert_eq!(sim.robots[0].weapons[0].cooldown, 2);
        sim.order_target.0 += 32;
        sim.weapon_bank[0].class = 99;
        sim.weapon_bank[0].trail = 17;
        assert_eq!(sim.fire_slot(0, 0, 6), FireDispatch::Spawn(1, 5));
        assert_eq!(
            (sim.weapon_bank[0].class, sim.weapon_bank[0].trail),
            (99, 17)
        );
    }
}
