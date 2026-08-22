//! The P2d mission-sim seam: one squad member's move, modeled from EXW.
//!
//! This module is the engine-side half of the P2d slice (docs/RE-EXW-SIM.md
//! is the RE half; every constant here carries its EXW anchor there).
//! It models exactly what the vertical slice needs — the walkability grid,
//! the order input path, and the per-sub-tick mover state — and nothing
//! more: no enemies, no combat, no reinforcement drops, no sidebar UI.
//! Those attach in later slices against the same seams.
//!
//! Architecture facts mirrored here (all [verified] in docs/RE-EXW-SIM.md):
//! - World position is Q13 (0x2000 units/tile); the walkability code works
//!   in Q5 (0x20 units/tile): candidate = `(pos + delta) >> 8`, tile =
//!   `candidate >> 5`.
//! - The frame loop runs the unit manager SIX times per frame (phases
//!   0..5, MissionShell@0044771c); movement strides are Q13 ±0x400
//!   (1/8 tile) per serviced sub-tick.
//! - Walkability = an 8-probe footprint test with a climb limit of 4
//!   z-units (move_is_possible@0041e897) over a height field built from
//!   the mission DAT type grid + the CGR 32×32 sub-tile height sprites
//!   (get_z_pos@0041e231 / get_from_dat_file@0041eb28). There is no
//!   separate solidity bit on this path — tall steps ARE the walls.
//! - One click-on-robot arms a single pending order (FUN_004247b5:
//!   window 0x197 frames, spread-claimed destination tiles); the unit
//!   manager then hands the move to every robot within 6 tiles
//!   (state 4 + stop distance 1000000) and walks them to their target
//!   with an octagonally-normalized velocity; arrival snaps state-4
//!   robots to the tile grid (robots@0040b9f6).
//!
//! Hermetic per the crate charter: bytes/args in, no I/O, integer math
//! only, no unordered state. The 64-entry angle threshold table is the
//! one runtime-data dependency ([inferred] SINTABLE.BIN words[2..66],
//! see [`AngleTable`]); it is injected so no asset bytes live in git.

use crate::hash::{Fnv1a64, StateHash};
use crate::rng::Pcg32;
use crate::weapon::{
    CommandRecord, EnemyProjectile, WeaponRecord, WeaponSlot, ENEMY_BANK_SLOTS, WEAPON_BANK_SLOTS,
};

/// Q13 units per tile (world position quantum) [0x2000, verified].
pub const Q13_PER_TILE: i32 = 0x2000;
/// Q5 units per tile (walkability-space quantum) [0x20, verified].
pub const Q5_PER_TILE: i32 = 0x20;
/// Robot spawn centering offset inside the tile [0xF00, verified].
pub const SPAWN_CENTER: i32 = 0xF00;
/// Per-sub-tick movement stride, Q13 [0x400 = 1/8 tile, verified].
pub const MOVE_STRIDE: i32 = 0x400;
/// Arrival radius, Q13 [0x1400, verified at 0x40be1d].
pub const ARRIVE_RADIUS: i32 = 0x1400;
/// Click-order effect radius, Q5 [0xC0 = 6 tiles, verified at 0x40c080].
pub const ORDER_RADIUS: i32 = 0xC0;
/// Order validity window, frames [0x197, verified at 0x4247d4].
pub const ORDER_WINDOW: u16 = 0x197;
/// Stop distance stored on a click order ["go all the way", verified].
pub const ORDER_STOP_DIST: i32 = 1_000_000;
/// Unit-manager invocations per frame (phases 0..5) [verified].
pub const PHASES_PER_FRAME: u32 = 6;
/// Footprint probes: X offsets, Q5 [dword table @0x4543e4, verified].
pub const PROBE_X: [i32; 8] = [-11, -11, 12, 12, 0, 0, -11, 12];
/// Footprint probes: Y offsets, Q5 [dword table @0x454404, verified].
pub const PROBE_Y: [i32; 8] = [-11, 12, -11, 12, -11, 12, 0, 0];
/// Maximum climb between probe floor and robot z, z-units [verified].
pub const CLIMB_LIMIT: i32 = 4;
/// Facing codes [verified: N=0x00 E=0x40 S=0x80 W=0xC0 none=0xFFFF].
pub const FACING_NONE: u16 = 0xFFFF;
pub const FACING_N: u16 = 0x0000;
pub const FACING_E: u16 = 0x0040;
pub const FACING_S: u16 = 0x0080;
pub const FACING_W: u16 = 0x00C0;
/// Robot states on the modeled path [verified]; state 1's producer is
/// undecoded (guard/patrol moves — RE-EXW-SIM.md sec 9).
pub const STATE_IDLE: u16 = 0;
pub const STATE_ORDERED: u16 = 3;
pub const STATE_MOVING: u16 = 4;
/// Spread-claim destination offsets, tile units [jumptable @0x424898,
/// verified]: 12 slots, assigned first-free per consumer.
pub const SPREAD_OFFSETS: [(i32, i32); 12] = [
    (0, 0),
    (1, 0),
    (-1, 0),
    (0, -1),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
    (-2, 0),
    (0, -2),
    (2, 0),
];

/// Octagonal distance used for every range/velocity decision
/// [FUN_0041ebf8, verified]: `max(|dx|,|dy|) + min(|dx|,|dy|)/2`
/// (the min term shifts right = floor, as the original `sar`).
pub fn dist_octagonal(dx: i32, dy: i32) -> i32 {
    let ax = dx.unsigned_abs();
    let ay = dy.unsigned_abs();
    let (mx, mn) = if ax > ay { (ax, ay) } else { (ay, ax) };
    (mx + mn / 2) as i32
}

/// The 64-entry angle threshold table [FUN_0041eb7d reads the 64 words
/// behind runtime pointer 0x46cbd0+4]. Data provenance [verified
/// §7j.37/2]: the words are SINTABLE.BIN words[2..66] — the 64-entry
/// ascending quarter-sine thresholds (peak 0x7FFF) of the file's
/// full 256-word sine ramp. Injected by the host so the hermetic
/// crate never embeds asset bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
/// The 64-entry octile→sector threshold table (SINTABLE.BIN words
/// 2..66) + the optional full 256-word sine array (the §7j.37/2
/// dual-use file table).
pub struct AngleTable {
    thresholds: [u16; 64],
    sine: Option<Box<[u16; 256]>>,
}

impl AngleTable {
    /// Build from the 256-word (512-byte) SINTABLE.BIN word array,
    /// taking words[2..66] as thresholds. Returns `None` on a short
    /// table. Also retains the FULL 256-word array — it is the
    /// byte-angle sine table (word[a] = round(sin(a·π/128)·32767),
    /// corpus-verified §7j.37/2): the "cos"/"sin" lookups of
    /// FUN_0041eb65/77 are pure word reads at a / (a−0x40).
    pub fn from_sintable_words(words: &[i16]) -> Option<Self> {
        if words.len() < 66 {
            return None;
        }
        let mut t = [0u16; 64];
        for (i, w) in words.iter().skip(2).take(64).enumerate() {
            t[i] = *w as u16;
        }
        let mut sine = [0u16; 256];
        for (i, w) in words.iter().take(256).enumerate() {
            sine[i] = *w as u16;
        }
        Some(AngleTable {
            thresholds: t,
            sine: Some(Box::new(sine)),
        })
    }

    /// Build from 64 raw thresholds (test constructor — no sine
    /// table: the homing velocity lookups are unavailable and the
    /// missile keeps its staged velocity).
    pub fn from_thresholds(words: &[u16]) -> Option<Self> {
        if words.len() < 64 {
            return None;
        }
        let mut t = [0u16; 64];
        t.copy_from_slice(&words[..64]);
        Some(AngleTable {
            thresholds: t,
            sine: None,
        })
    }

    /// The byte-angle "cos" lookup FUN_0041eb65 [verified §7j.37/2]:
    /// sine-word[a & 0xFF]. `None` when built from thresholds only.
    pub fn sine_word(&self, index: u16) -> Option<u16> {
        self.sine.as_ref().map(|s| s[(index & 0xFF) as usize])
    }

    /// 256-direction angle byte for a Q13 delta [FUN_0041eb7d +
    /// FUN_0041ebc1, verified]. Callers pass the target deltas in Q13
    /// (the EXW caller feeds `Δ(Q5) << 8`). Cardinal results: N=0x00,
    /// E=0x40, S=0x80, W=0xC0.
    pub fn angle_byte(&self, dx: i32, dy: i32) -> u16 {
        let dist = dist_octagonal(dx, dy);
        let dist_hi = (dist >> 8).max(1);
        // ratio = |dx| * 0x80 / dist_hi, computed wide to mirror the
        // original (|Δx|<<7 before the idiv); clamp 0x7FFF [verified].
        let ratio = ((dx.unsigned_abs() as u64) << 7) / u64::from(dist_hi as u32);
        let ratio = ratio.min(0x7FFF) as u32;
        // First sector whose threshold EXCEEDS the ratio; 0x3F default
        // [verified scan: stop at the first table[i] > ratio].
        let mut sector: u32 = 0x3F;
        for (i, t) in self.thresholds.iter().enumerate() {
            if ratio < u32::from(*t) {
                sector = i as u32;
                break;
            }
        }
        // Quadrant fold [verified].
        if dx >= 0 && dy > 0 {
            0x7F - sector as u16
        } else if dx < 0 && dy >= 0 {
            sector as u16 + 0x80
        } else if dx < 0 && dy <= 0 {
            0x100 - sector as u16
        } else {
            sector as u16
        }
    }
}

/// Mission walkability data: the DAT type planes + the CGR height
/// sprites (the get_z_pos inputs). Plane-major u8 z-planes, `w*h`
/// each — exactly the on-disk .DAT payload layout (FORMATS-MISSION
/// sec 4); indexing mirrors get_from_dat_file@0041eb28
/// (`dat[z*w*h + y*w + x]`, with 0xFF read back as type 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terrain {
    width: i32,
    height: i32,
    dat: Vec<u8>,
    /// Per tile-type 32×32 sub-tile height maps, slot = type-1 (the
    /// CGR directory entry index) [verified: sprite bytes are the
    /// height field].
    heights: Vec<[u8; 1024]>,
    /// Type-3 trigger latch (get_z_pos side effect at 004dc688/8c/90):
    /// `(z_level, tile_x, tile_y)` of the last type-3 probe — hashed
    /// sim state, consumed by the tile-effect logic in a later slice.
    pub last_trigger: Option<(i32, i32, i32)>,
}

/// The mission DAT's raw plane bytes after the EXW header skip + the
/// (bytes at least 0x80) sweep [load_mission 0x41dde4, verified]:
/// `Some(8*w*h)` swept plane bytes, or `None` on a malformed header.
/// This is the PRE-PAD
/// form: the walkability [`Terrain`] layers the PAD 0xFF marks on top
/// of a private copy, while the viewport TOT mirror (the seen marks
/// compare DAT bytes against zero) reads exactly this.
pub fn dat_plane_bytes(dat: &[u8]) -> Option<Vec<u8>> {
    if dat.len() < 4 {
        return None;
    }
    let width = u16::from_le_bytes([dat[0], dat[1]]) as i32;
    let height = u16::from_le_bytes([dat[2], dat[3]]) as i32;
    let n = (width * height) as usize;
    if width <= 0 || height <= 0 || dat.len() != 4 + 8 * n {
        return None;
    }
    let mut planes = dat[4..].to_vec();
    // Sweep: planes 0..6, bytes >= 0x80 -> 0 [0x41dde4, verified].
    for z in 0..7 {
        for b in planes[z * n..z * n + n].iter_mut() {
            if *b >= 0x80 {
                *b = 0;
            }
        }
    }
    Some(planes)
}

impl Terrain {
    /// Build from the raw pieces. `dat` must be `8 * w * h` bytes
    /// (8 plane-major u8 planes); `heights` holds the CGR sprite
    /// bodies (1024 bytes each). Wrong sizes return `None` instead of
    /// panicking (charter).
    pub fn from_parts(
        width: i32,
        height: i32,
        dat: Vec<u8>,
        heights: Vec<[u8; 1024]>,
    ) -> Option<Self> {
        if width <= 0 || height <= 0 || dat.len() != (8 * width * height) as usize {
            return None;
        }
        Some(Terrain {
            width,
            height,
            dat,
            heights,
            last_trigger: None,
        })
    }

    /// Build from the raw on-disk mission bytes, mirroring EXW
    /// load_mission@0041dc5a [verified, docs/RE-EXW-SIM.md sec 7c]:
    /// `dat` is the whole `.DAT` file (u16 w + u16 h + 8 plane-major
    /// u8 planes), `pad` the whole `.PAD` file (6-byte `(x, y, level)`
    /// records, 0xFFFF-x fill ends the list), `cgr` the zone-level
    /// `.CGR` bank (u16 count + count u32 offsets + sprite bodies).
    ///
    /// Loader rules applied verbatim: bytes >= 0x80 in planes 0..6 are
    /// swept to 0 (plane 7 untouched); every PAD record writes 0xFF at
    /// `DAT[level][y][x]` (0xFF later reads back as type 1 — the pad
    /// materialises a deck block); height slot `t-1` is the RAW 1024
    /// bytes at `CGR[2 + 4*(t-1) + dir[t-1] + 6]` (no codec). The EXW
    /// write is fully UNCHECKED [0x41ded0, verified absence of bounds];
    /// out-of-range level/x/y are skipped here instead of writing out of
    /// bounds (shipped records are in-bounds, so corpus behavior is
    /// identical — charter: no panics, no UB). Returns `None` on
    /// malformed inputs.
    pub fn from_mission_bytes(dat: &[u8], pad: &[u8], cgr: &[u8]) -> Option<Self> {
        // DAT: dims + 8 planes, swept by the shared loader helper.
        let width = u16::from_le_bytes([*dat.first()?, *dat.get(1)?]) as i32;
        let height = u16::from_le_bytes([*dat.get(2)?, *dat.get(3)?]) as i32;
        let n = (width * height) as usize;
        let mut planes = dat_plane_bytes(dat)?;
        // PAD marks: DAT[level][y][x] = 0xFF [0x41ded0, verified].
        for rec in pad.chunks_exact(6) {
            let x = u16::from_le_bytes([rec[0], rec[1]]) as i32;
            let y = u16::from_le_bytes([rec[2], rec[3]]) as i32;
            let level = u16::from_le_bytes([rec[4], rec[5]]) as i32;
            if x == -1 {
                break; // 0xFFFF fill ends the record run [0x41defa]
            }
            if !(0..8).contains(&level) || x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            planes[level as usize * n + y as usize * width as usize + x as usize] = 0xFF;
        }
        // CGR: u16 count + count u32 offsets; slot s body at
        // 2 + 4*s + dir[s] + 6 [0x41e328..0x41e353, verified].
        if cgr.len() < 2 {
            return None;
        }
        let count = u16::from_le_bytes([cgr[0], cgr[1]]) as usize;
        if cgr.len() < 2 + 4 * count {
            return None;
        }
        let mut heights = Vec::with_capacity(count);
        for s in 0..count {
            let o = 2 + 4 * s;
            let dir = u32::from_le_bytes([cgr[o], cgr[o + 1], cgr[o + 2], cgr[o + 3]]) as usize;
            let body = dir.checked_add(4 * s + 8)?;
            let end = body.checked_add(1024)?;
            if end > cgr.len() {
                return None;
            }
            let mut map = [0u8; 1024];
            map.copy_from_slice(&cgr[body..end]);
            heights.push(map);
        }
        Terrain::from_parts(width, height, planes, heights)
    }

    /// Map size in tiles (DAT_004eddec / DAT_004eddf0).
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// DAT type at a tile [FUN_0041eb28, verified]: plane-major byte,
    /// 0xFF remapped to 1. Out-of-range coordinates read as 0 (the EXW
    /// callers pre-clamp via the move_is_possible bounds test; this is
    /// the panic-free equivalent).
    pub fn dat_type(&self, x: i32, y: i32, z: i32) -> u8 {
        if !(0..8).contains(&z) || x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 0;
        }
        let n = (self.width * self.height) as usize;
        let idx = z as usize * n + y as usize * self.width as usize + x as usize;
        let t = self.dat[idx];
        if t == 0xFF {
            1
        } else {
            t
        }
    }

    fn height_of(&self, ty: u8, sx: i32, sy: i32) -> u8 {
        if ty == 0 {
            return 0;
        }
        self.heights
            .get(ty as usize - 1)
            .map_or(0, |s| s[sy as usize * 32 + sx as usize])
    }

    /// Floor z at a Q5 position [FUN_0041e231 get_z_pos, verified]:
    /// clamp z 0..0xFF, level = z>>5; search the DAT type at level,
    /// then level+1 (if <7), then level-2, while the type is empty
    /// (0 or 0x2A); empty everywhere -> 0. Height = the CGR sprite
    /// byte at the sub-tile position; return `level*0x20 + byte`,
    /// with the 0x1F tile-top rule bridging into level+1.
    pub fn floor_z(&mut self, q5x: i32, q5y: i32, z: i32) -> i32 {
        let zb = z.clamp(0, 0xFF);
        let tx = q5x >> 5;
        let ty = q5y >> 5;
        let sx = q5x & 0x1F;
        let sy = q5y & 0x1F;
        let mut lvl = zb >> 5;
        let mut tyb = self.dat_type(tx, ty, lvl);
        if tyb == 3 {
            self.last_trigger = Some((lvl, tx, ty));
        }
        if (tyb == 0 || tyb == 0x2A) && lvl < 7 {
            lvl += 1;
            tyb = self.dat_type(tx, ty, lvl);
            if tyb == 3 {
                self.last_trigger = Some((lvl, tx, ty));
            }
        }
        if (tyb == 0 || tyb == 0x2A) && lvl > 1 {
            lvl -= 2;
            tyb = self.dat_type(tx, ty, lvl);
            if tyb == 3 {
                self.last_trigger = Some((lvl, tx, ty));
            }
        }
        if tyb == 0 || tyb == 0x2A {
            return 0;
        }
        let h = i32::from(self.height_of(tyb, sx, sy));
        let base = lvl * 0x20;
        if h != 0x1F {
            return base + h;
        }
        if lvl > 6 {
            return base + h;
        }
        let above = self.dat_type(tx, ty, lvl + 1);
        if above == 3 {
            self.last_trigger = Some((lvl + 1, tx, ty));
        }
        if above != 0 && above != 0x2A {
            let h2 = i32::from(self.height_of(above, sx, sy));
            if h2 != 0 {
                return (lvl + 1) * 0x20 + h2;
            }
        }
        base + h
    }
}

/// One robot record — the modeled subset of the 0xA8-byte EXW record
/// (base 0x4c69e4; field map in docs/RE-EXW-SIM.md sec 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Robot {
    /// World X, Q13 (+0x00).
    pub pos_x: i32,
    /// World Y, Q13 (+0x04).
    pub pos_y: i32,
    /// Floor z, Q5, clamped 0..0xFF (+0x08).
    pub z: i32,
    /// State word (+0x0C).
    pub state: u16,
    /// Last angle byte used (+0x0E).
    pub dir_byte: u16,
    /// Facing (+0x10): cardinal code or FACING_NONE.
    pub facing: u16,
    /// Walk animation phase (+0x12): ((angle+4)&0xFF)>>3.
    pub anim: u16,
    /// Spawn variant (+0x18): RandA()&3.
    pub variant: u16,
    /// 8-word probe floor-z cache (+0x1A..+0x29); slot i is probe i's
    /// own climb-compare reference AND get_z_pos z input (the sar of
    /// dword@+0x18+2i — sign-extended, so 0xFFFF reads back as -1)
    /// [verified asm 0x41e8ce, amendment 7b.2].
    pub probe_z: [u16; 8],
    /// Stop distance for the active order (+0x74).
    pub stop_dist: i32,
    /// Move target, Q5 (the DAT_0046cc30/60 pair; None = -1).
    pub target: Option<(i32, i32)>,
    /// Alive flag (+0x7C @ 0x4c6a60 — offset label corrected from
    /// +0x78 2026-08-21, RE-EXW-SIM sec 6c.7).
    pub alive: bool,
    /// Countdown buying phases 4/5 (+0x80 @ 0x4c6a64): the phase gate
    /// is `phase < 4 || phase*32 < drop_countdown` [verified
    /// expression; offset label corrected 2026-08-21, sec 6c.7].
    pub drop_countdown: i32,
    /// HP (+0x78): spawn `5000 + 100*battery` (the dropship-landing
    /// init, RE-EXW-SIM 7f.8); damage subtracts here behind the
    /// shield gate (FUN_0040e230, 7g.1); `< 1` is the death gate.
    pub hp: i32,
    /// Armor word (+0x30): the pad-charged shield that bleeds 10 per
    /// phase-1 pass off-pad (7g.3), charges +20 on pads behind the
    /// +0x98 pool (7g.4), clamps [0, 3000]; the bar denominates 2500.
    pub armor: i16,
    /// Hit-flash word (+0x2E): damage bumps it (before the hp write,
    /// 7g.1); the portrait pass clamps to 5 then decrements per
    /// frame while the robot is alive with hp ≥ 1 (7g.8).
    pub hit_flash: u16,
    /// Under-attack alarm word (+0x34): set 100 when the +0xA4
    /// accumulator trips (7g.1), decays 1/frame in the phase-0
    /// pre-walk (7g.2).
    pub alarm: u16,
    /// Alarm accumulator (+0xA4): +3 per damaging hit while the alarm
    /// word is 0; `> 100` on a player-type robot trips the alarm and
    /// resets this to 0 (7g.1).
    pub alarm_ctr: i32,
    /// Shield pool (+0x88): absorbs damage before hp (`max(0, s-d)`),
    /// decays 2 per frame, set 0x20 by the ordered/auto-shield
    /// conversions and 10000/150 by the +0xA0 booster (7g.1/7g.2).
    pub shield: i32,
    /// Auto-shield charges (+0x8C): equipment stat 0x2A; a damaging
    /// hit with charges > 0 and shield == 0 spends one charge for a
    /// 0x20 shield instead of taking damage (7g.1).
    pub shield_charges: i32,
    /// Shield booster countdown (+0xA0): pickup case 7 arms 200;
    /// while nonzero the pool is forced 10000 and on expiry set 150
    /// (7g.2).
    pub shield_boost: i32,
    /// Battery stat (+0x94): equipment stat 0x2B; the landing HP
    /// formula is `5000 + 100*battery` (7f.8).
    pub battery: i32,
    /// Armor-charge pool (+0x98): equipment stat 0x2C × 200; a pad
    /// pass drains it by the charge amount BEFORE armor charges
    /// (FUN_004100b7, 7g.4 — mechanics verified, design intent
    /// unclear).
    pub armor_pool: i32,
    /// Robot TYPE word (+0x2A): 0 = the player type in SP
    /// (`[0x4edb90]` = 0, GameMain 0x41c34c); gates the alarm trip
    /// and the case-4 pickup.
    pub kind: u16,
    /// Death flag (+0x9C): set 1 by the SP death subset (7g.6);
    /// readers not yet census'd.
    pub death_flag: u16,
    /// The 7 weapon slots (+0x36.., 8-byte groups {id@+0, ammo@+2,
    /// cooldown@+6}) — the COMMAND consumer's fire surface
    /// [§7j.37/1]. Zeroed at spawn (the fresh-campaign empty
    /// loadout); the host stages them through
    /// [`MissionSim::stage_robot_weapons`].
    pub weapons: [WeaponSlot; 7],
    /// Weapon enable mask (+0x6E, the order-bits word): bit k = slot
    /// k armed. The fire gate's first term.
    pub weapon_mask: u16,
}

impl Robot {
    /// Tile coordinates of the robot center (Q13 >> 13).
    pub fn tile(&self) -> (i32, i32) {
        (self.pos_x >> 13, self.pos_y >> 13)
    }

    /// Q5 position (what the walkability code consumes).
    pub fn q5(&self) -> (i32, i32) {
        (self.pos_x >> 8, self.pos_y >> 8)
    }
}

/// The pending click order (the 0x4eabb0 family, RE-EXW-SIM.md sec 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    /// Order tile (0x4eabb4/6) and z (0x4eabb8).
    pub tile: (i32, i32, i32),
    /// Validity window in frames (0x4eabb2; 0 when exactly one robot
    /// is alive — the armer's special case).
    pub window: u16,
    /// 12-slot spread-claim array (0x4eabba).
    pub claims: [bool; 12],
}

/// What [`MissionSim::apply_damage`] did — the presentation half of
/// FUN_0040e230 the host stages (7g.6): `died` drives the sidebar
/// redraw countdown (`DAT_0046ccec = 3`) + the death SFX family, and
/// the five debris rows are the FUN_00420608 staging inputs
/// (x, y in Q5, z, the phase param) exactly as computed in EXW.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageOutcome {
    /// The hit passed the gates (not dead/state-2).
    pub applied: bool,
    /// The robot died this hit (the SP subset ran).
    pub died: bool,
    /// The five debris staging rows `(x_q5, y_q5, z, phase_param)`.
    pub debris: [(i32, i32, i32, i32); 5],
}

/// What [`MissionSim::apply_pickup`] did — the presentation half of
/// the FUN_0040eba0 case bodies the host stages (7h.2): `effect` is
/// the 0x4dc5d0 sprite-effect row id (1 = the reinforcement
/// drop-in, 6 = the shield bubble, 7 = the health cross-up, 0xE =
/// the booster flare) plus the per-case 0x43a48e SFX queue entry —
/// both unwired until the mission SFX/effects slices land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickupOutcome {
    /// The case body ran (robot index valid, case in 1/2/3/7).
    pub applied: bool,
    /// The 0x4dc5d0 row effect id for the case (7h.2).
    pub effect: i32,
}

/// Shield tick granted by the ordered/auto-shield conversions
/// (+0x88 = 0x20, 7g.1).
pub const SHIELD_TICK: i32 = 0x20;
/// Shield decay per frame in the phase-0 pre-walk (7g.2).
pub const SHIELD_DECAY: i32 = 2;
/// The pool override while the +0xA0 booster counts down (7g.2).
pub const SHIELD_BOOST_POOL: i32 = 10_000;
/// The pool left when the booster expires (7g.2).
pub const SHIELD_BOOST_LEFTOVER: i32 = 150;
/// Armor clamp on pad charges (FUN_004100b7, 7g.4).
pub const ARMOR_MAX: i16 = 0xBB8;
/// Armor bleed per phase-1 pass off-pad (7g.3).
pub const ARMOR_BLEED: i16 = 10;
/// Armor charge per phase-1 pad pass (robots() 0x40bc72, 7g.3).
pub const ARMOR_CHARGE: i32 = 0x14;
/// The kind-5 scorch ring [RE-EXW-SIM 7j.9, verified asm
/// 0x421465..0x4215d8 + the shared tail 0x421285..0x421291]: the
/// NINE FUN_00422287 calls the debris stager makes per death
/// debris, in the EXW write order. Offsets are world Q5 (±0x20 =
/// one tile after the writer's `>>5`); values are corners 1,
/// edges 2, center 4. Overlapping rings (the five debris jitter
/// within ±1 tile of the corpse) are last-write-wins in staging
/// order — the ring order matters and is pinned as decoded.
pub const DEBRIS_SCORCH_RING: [(i32, i32, u8); 9] = [
    (-0x20, -0x20, 1), // TL  0x421476
    (-0x20, 0, 2),     // L   0x4214a3
    (-0x20, 0x20, 1),  // BL  0x4214d3
    (0, -0x20, 2),     // T   0x421500
    (0, 0, 4),         // C   0x42152a
    (0, 0x20, 2),      // B   0x421557
    (0x20, -0x20, 1),  // TR  0x421587
    (0x20, 0, 2),      // R   0x4215b4
    (0x20, 0x20, 1),   // BR  0x421291
];
/// The reinforcement-staging value FUN_0040eba0 case 1 writes to
/// `drop_countdown` (+0x80, 7h.2).
pub const PICKUP_DROP: i32 = 0x3E8;
/// The pool value the shield pickup writes (+0x88, case 2, 7h.2).
pub const PICKUP_SHIELD: i32 = 0x3E8;
/// The health pickup increment (+0x78, case 3, 7h.2); the sum
/// clamps at the landing HP ceiling 5000 (0x1388).
pub const PICKUP_HEALTH: i32 = 0x9C4;
/// The HP ceiling the health pickup clamps to (case 3, 7h.2 —
/// also the hp-bar full denominator, 7f.1).
pub const HP_MAX: i32 = 0x1388;
/// The booster countdown the shield-booster pickup arms (+0xA0,
/// case 7, 7h.2).
pub const PICKUP_BOOST: i32 = 0xC8;

/// The pickup range-table A bases per terrain set (7 dwords at
/// DGROUP 0x454a58, PE bytes, 7h.1): tile words in the closed
/// groups `[base, base+0x10)` map to cases 1/3/2/4.
pub const PICKUP_RANGE_A: [i32; 7] = [0x4E, 0x75, 0x75, 0x358, 0x75, 0xA3, 0xA3];
/// The pickup range-table B bases per terrain set (7 dwords at
/// DGROUP 0x454a74, PE bytes, 7h.1): tile words in the closed
/// groups `[base, base+0xC)` map to cases 9/7/8.
pub const PICKUP_RANGE_B: [i32; 7] = [0x75, 0x535, 0x70B, 0x656, 0x535, 0x4FE, 0x31E];

/// The FUN_0040eba0 dispatch decode [RE-EXW-SIM 7h.1, verified asm
/// 0x40ebaa..0x40ecef]: which pickup case a per-tile type word
/// selects for terrain set `set` (the `_DAT_004edd8c` index, 7h.4).
/// Each table base splits into closed 4-word groups — table A
/// `[A, A+0x10)` → cases 1/3/2/4, table B `[B, B+0xC)` → cases
/// 9/7/8 — so `Some(1..=9)` exactly when the word is one of the
/// 28 pickup words of that set. The tile-word PRODUCER (the
/// type-DB mirror + probe-latch walk, 7h.3) is still host-seamed.
pub fn pickup_case(tile_word: i32, set: usize) -> Option<u8> {
    const CASES_A: [u8; 4] = [1, 3, 2, 4];
    const CASES_B: [u8; 3] = [9, 7, 8];
    let tables: [(&[i32], &[u8], i32); 2] = [
        (&PICKUP_RANGE_A, &CASES_A, 16),
        (&PICKUP_RANGE_B, &CASES_B, 12),
    ];
    for (table, cases, span) in tables {
        if let Some(base) = table.get(set).copied() {
            let d = tile_word - base;
            if d >= 0 && d < span {
                return Some(cases[(d / 4) as usize]);
            }
        }
    }
    None
}

/// The mission simulation slice: terrain + robots + one pending order.
///
/// Driven by [`MissionSim::advance_frame`], which mirrors one
/// MissionShell frame: six unit-manager phases, then the order-window
/// tick. Presentation-side effects (sidebar redraw flags, SFX queues,
/// the dropship staging FUN_0041faf0 performs) are deliberately absent.
#[derive(Debug, PartialEq, Eq)]
pub struct MissionSim {
    pub terrain: Terrain,
    pub(crate) robots: Vec<Robot>,
    order: Option<Order>,
    pub(crate) angles: AngleTable,
    rng: Pcg32,
    frame: u64,
    /// Per-tile armor-pad bytes — the +0x18 byte of the per-tile
    /// 0x1E record at 0x4796bc (7g.3): nonzero = pad. The runtime
    /// producer is the scorch-ring writer FUN_00422287 (7j.9) —
    /// the death tail stages nine writes per debris; the static
    /// map-load fill leaves them 0 on ZONEA, so the default
    /// (empty = all zero) bleeds armor exactly like the shipped
    /// corpus until a death; the host stages real bytes through
    /// [`MissionSim::set_armor_pads`].
    /// Derived mission data — hashed only through its armor effect.
    armor_pads: Vec<u8>,
    /// The player TYPE word ([0x4edb90]): 0 in all SP games
    /// (GameMain 0x41c34c) — gates the alarm trip + case-4 pickup.
    player_type: u16,
    /// The COMMAND ring (the W5 injection seam's E-side home):
    /// staged payloads, drained once per frame by the consumer
    /// [§7j.37/1]. NOT hashed (empty on every corpus path).
    pub(crate) commands: Vec<CommandRecord>,
    /// The 400×0x36 weapon-anim bank at 0x4c71f4 [§7j.37]. Slot
    /// order = the original record order; kind 0 = free. NOT part of
    /// `state_hash` (the S3 T2 watch surface is its own dump row).
    pub(crate) weapon_bank: Vec<WeaponRecord>,
    /// The 50×0x22 projectile bank at 0x4cc654 [7j.13/5].
    pub(crate) enemy_bank: Vec<EnemyProjectile>,
    /// The ORDER-target triple 0x4dd484/88/8c (bit1 records write
    /// it; the weapon dispatch aims at it).
    pub(crate) order_target: (i32, i32, i32),
    /// The order-active flag 0x4dc6bc.
    pub(crate) order_flag: bool,
    /// The DIFFICULTY dword 0x46cbf8 (0..2) — the only selector of
    /// the difficulty-scaled damage rows [§7j.15/2]. Staged by the
    /// host; 0 = the modeled default.
    pub(crate) difficulty: u32,
}

impl MissionSim {
    /// Create with terrain, the angle table, and a PRNG seed.
    pub fn new(terrain: Terrain, angles: AngleTable, seed: u64) -> Self {
        MissionSim {
            robots: Vec::new(),
            order: None,
            angles,
            rng: Pcg32::new(seed, STREAM_MISSION),
            frame: 0,
            armor_pads: Vec::new(),
            player_type: 0,
            commands: Vec::new(),
            weapon_bank: vec![WeaponRecord::default(); WEAPON_BANK_SLOTS],
            enemy_bank: vec![EnemyProjectile::default(); ENEMY_BANK_SLOTS],
            order_target: (0, 0, 0),
            order_flag: false,
            difficulty: 0,
            terrain,
        }
    }

    /// Stage the DIFFICULTY dword (0x46cbf8, 0..2) — the
    /// difficulty-scaled damage selector. The host seeds it from the
    /// scenario's boot difficulty.
    pub fn set_difficulty(&mut self, difficulty: u32) {
        self.difficulty = difficulty;
    }

    /// Robots in record order.
    pub fn robots(&self) -> &[Robot] {
        &self.robots
    }

    /// Robots in record order, mutably (test/host seam for direct
    /// state setup; normal gameplay goes through the order API).
    pub fn robots_mut(&mut self) -> &mut [Robot] {
        &mut self.robots
    }

    /// The pending order, if any.
    pub fn order(&self) -> Option<Order> {
        self.order
    }

    /// Frames elapsed.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// One `RandA()` draw from the SHARED mission stream [the EXW
    /// global RandA; spawn variants, walk phases, and the pickup
    /// awards all draw from it]. Read/write seam for host-staged
    /// producers (RE-EXW-SIM 7f.6): every draw advances the stream
    /// exactly like the original, so the sim hash moves with it —
    /// nothing on the default corpus path calls this.
    pub fn rand_a_state(&self) -> u64 {
        self.rng.state()
    }

    /// The staged armor-pad/scorch bank as stored (the +0x18 byte
    /// family, 7g.3/7j.9): lazily materialized, so its length is 0
    /// until the first `scorch_write` grows it (the all-zero ZONEA
    /// corpus). Read-only view for the canonical dump emitter (W6).
    pub fn armor_pads(&self) -> &[u8] {
        &self.armor_pads
    }

    /// One `RandA()` draw from the SHARED mission stream (advances it
    /// exactly like the original — see the canonical-emitter note above).
    pub fn rand_a(&mut self) -> u32 {
        self.rng.next_u32()
    }

    /// Stage the battery stat (+0x94) — the equipment stats-copy
    /// switch case 0x2B seam [7g.4]: writes the stat and re-runs the
    /// dropship-landing HP init `5000 + 100*battery` (7f.8). Staging
    /// models the pre-mission shop point, exactly like the D52
    /// host-side landing did.
    pub fn set_battery(&mut self, idx: usize, battery: i32) {
        let Some(r) = self.robots.get_mut(idx) else {
            return;
        };
        r.battery = battery;
        r.hp = 5000 + 100 * battery;
    }

    /// Stage the per-tile armor-pad bytes (the per-tile 0x1E record
    /// +0x18 mirror, 7g.3): linear tile order (`y*width + x`),
    /// shorter arrays read as zero-padded (the all-zero default).
    /// NOTE [7j.10]: staged bytes are TRANSIENT — the epilogue fade
    /// in [`MissionSim::advance_frame`] decrements every nonzero
    /// byte once per frame, exactly like the original (the original
    /// has no permanent pad producer; host staging is a test seam).
    pub fn set_armor_pads(&mut self, pads: &[u8]) {
        self.armor_pads = pads.to_vec();
    }

    /// The armor pad byte for a linear tile (record +0x18, 7g.3):
    /// zero when unstaged/out of range — the shipped ZONEA corpus
    /// leaves every pad byte 0 until a death scorches (7j.9), and
    /// whatever lands fades within `value` frames (7j.10).
    pub fn armor_pad_byte(&self, tile: usize) -> u8 {
        self.armor_pads.get(tile).copied().unwrap_or(0)
    }

    /// FUN_00422287 [RE-EXW-SIM 7j.9, verified 0x422287..0x4222cd]:
    /// the per-tile type-DB +0x18 byte writer. `(world_x>>5,
    /// world_y>>5)` (arithmetic shifts) → tile, dropped when either
    /// coordinate is negative or ≥ the map w/h, then
    /// `byte[0x4796d4 + tile*0x1E] = value` with the zero-extended
    /// value clamped `≥ 8 → 7`. The engine mirror is zero-padded:
    /// the default all-zero corpus stays zero until the first
    /// write grows the array. Public as the host seam for the
    /// census'd-but-unwired producers (the water-splash event
    /// tick FUN_00424051, 7j.10 — its five same-tile re-rolls and
    /// the death ring both fade via the advance_frame tick).
    pub fn scorch_write(&mut self, world_x: i32, world_y: i32, value: u8) {
        let tx = world_x >> 5;
        let ty = world_y >> 5;
        if tx < 0 || ty < 0 {
            return;
        }
        let (w, h) = self.terrain.size();
        if tx >= w || ty >= h {
            return;
        }
        let tile = (ty * w + tx) as usize;
        let value = if u32::from(value) >= 8 { 7 } else { value };
        if self.armor_pads.len() <= tile {
            self.armor_pads.resize(tile + 1, 0);
        }
        self.armor_pads[tile] = value;
    }

    /// Apply damage to robot `idx` — the SP core of
    /// FUN_0040e230@0x40e230 [RE-EXW-SIM 7f.5 + 7g.1/7g.6, verified
    /// decompile + asm]. Gates: `state == 2` and `!alive` return
    /// untouched. `state == 3` (ordered) converts the hit into a
    /// shield tick `shield = 0x20` and returns. A live auto-shield
    /// (`shield_charges > 0 && shield == 0`) spends a charge for the
    /// 0x20 shield instead. Otherwise the damage path: the alarm
    /// accumulator (+3 while the alarm word is 0, trip at > 100 on a
    /// player-type robot → alarm 100, accumulator 0), then `shield >
    /// 0` absorbs (`max(0, s-d)`) else `hit_flash += 1` BEFORE
    /// `hp -= damage`. `hp < 1` runs the SP death subset: alive/hp/
    /// drop cleared, death flag set, armor zeroed, and five debris
    /// staged from the SHARED stream (2 RandA draws each — y jitter
    /// first, then x — moving the sim hash exactly like the
    /// original), each debris also writing the NINE-tile scorch
    /// ring into the armor-pad mirror [7j.9 — the phase-1 reader
    /// treats those bytes as pads, so the ring is sim state, not
    /// presentation]. The SFX/FUN_0042382c/`DAT_0046ccec = 3`
    /// sidebar redraw signal and the debris sprite staging are
    /// presentation — the host reads them off [`DamageOutcome`].
    /// The MP kill bookkeeping + respawn branch is out of model
    /// (SP sim).
    pub fn apply_damage(&mut self, idx: usize, damage: i32, killer: i32) -> DamageOutcome {
        let mut out = DamageOutcome {
            applied: false,
            died: false,
            debris: [(0, 0, 0, 0); 5],
        };
        let (state, alive) = {
            let Some(r) = self.robots.get(idx) else {
                return out;
            };
            (r.state, r.alive)
        };
        if !alive || state == 2 {
            return out;
        }
        out.applied = true;
        if state == STATE_ORDERED {
            // The ordered conversion: damage becomes a shield tick.
            self.robots[idx].shield = 0x20;
            return out;
        }
        if self.robots[idx].shield_charges != 0 && self.robots[idx].shield == 0 {
            // The auto-shield idle: spend a charge, raise 0x20.
            self.robots[idx].shield_charges -= 1;
            self.robots[idx].shield = 0x20;
            return out;
        }
        // The damage path: alarm first, then the absorb/subtract.
        if self.robots[idx].alarm == 0 {
            self.robots[idx].alarm_ctr += 3;
        }
        if self.robots[idx].alarm_ctr > 100 && self.robots[idx].kind == self.player_type {
            self.robots[idx].alarm = 100;
            self.robots[idx].alarm_ctr = 0;
        }
        if self.robots[idx].shield == 0 {
            let r = &mut self.robots[idx];
            r.hit_flash = r.hit_flash.wrapping_add(1);
            r.hp -= damage;
        } else {
            let s = self.robots[idx].shield - damage;
            self.robots[idx].shield = if s < 0 { 0 } else { s };
        }
        if self.robots[idx].hp < 1 {
            // The SP death subset [7g.6]: the seven order words
            // (+0x38..+0x68) are not modeled — noted in 7g.6; every
            // modeled write below is the verified sequence.
            let (pos_x, pos_y, z) = {
                let r = &self.robots[idx];
                (r.pos_x, r.pos_y, r.z)
            };
            {
                let r = &mut self.robots[idx];
                r.alive = false;
                r.drop_countdown = 0;
                r.hp = 0;
                r.death_flag = 1;
                r.armor = 0;
            }
            // Five debris, two shared-stream draws each: the y
            // jitter draws FIRST, then the x jitter (asm
            // 0x40e72d/0x40e74f); z walks +8k, the phase param 2k.
            // Each staged debris also runs the kind-5 SCORCH RING
            // [7j.9]: the nine FUN_00422287 type-DB +0x18 writes
            // (3×3 tiles around the debris tile, corners 1 / edges
            // 2 / center 4) — sim state the phase-1 armor pass
            // reads as pads, landing in staging order so overlaps
            // are last-write-wins exactly like the EXW.
            for (k, d) in out.debris.iter_mut().enumerate() {
                let ry = self.rng.next_u32();
                let rx = self.rng.next_u32();
                d.0 = (rx & 0x1F) as i32 + (pos_x >> 8) - 0x10;
                d.1 = (ry & 0x1F) as i32 + (pos_y >> 8) - 0x10;
                d.2 = z + 8 * k as i32;
                d.3 = 2 * k as i32;
                for &(dx, dy, value) in DEBRIS_SCORCH_RING.iter() {
                    self.scorch_write(d.0 + dx, d.1 + dy, value);
                }
            }
            out.died = true;
        }
        // `killer` reaches only the MP bookkeeping (7g.6) — kept in
        // the signature for the callers' fidelity, unused in SP.
        let _ = killer;
        out
    }

    /// Apply a pickup to robot `idx` — the FUN_0040eba0 case bodies
    /// 1/2/3/7 [RE-EXW-SIM 7h.2, verified asm]: case 1 stages the
    /// reinforcement (`drop_countdown = 1000`), case 2 refills the
    /// shield pool (`shield = 1000`), case 3 heals (`hp += 2500`
    /// clamped `> 5000 → 5000`), case 7 arms the shield booster
    /// (`shield_boost = 200` — the phase-0 pre-walk consumes it,
    /// 7g.2). No alive/state gates (the caller fires the dispatch
    /// on the tile match alone); cases 4/8/9 (score-money/ammo/
    /// episode) are NOT this seam — case 4 is the D52
    /// score/money host seam, 8/9 remain unlanded. The SFX +
    /// 0x4dc5d0 effect-row staging are presentation — the host
    /// reads the effect id off [`PickupOutcome`]. Nothing on the
    /// default corpus path invokes this (the tile-word producer
    /// 7h.3 is host-seamed).
    pub fn apply_pickup(&mut self, idx: usize, case: u8) -> PickupOutcome {
        let mut out = PickupOutcome {
            applied: false,
            effect: 0,
        };
        // The field value per case (7h.2), paired with the
        // 0x4dc5d0 effect-row id the tail stages for it.
        let (value, effect) = match case {
            1 => (PICKUP_DROP, 1),
            2 => (PICKUP_SHIELD, 6),
            3 => (PICKUP_HEALTH, 7),
            7 => (PICKUP_BOOST, 0xE),
            _ => return out,
        };
        let Some(r) = self.robots.get_mut(idx) else {
            return out;
        };
        out.applied = true;
        out.effect = effect;
        match case {
            1 => r.drop_countdown = value,
            2 => r.shield = value,
            3 => {
                r.hp += value;
                if r.hp > HP_MAX {
                    r.hp = HP_MAX;
                }
            }
            _ => r.shield_boost = value,
        }
        out
    }

    /// Spawn a robot from an MRK marker record [FUN_0040cca0,
    /// verified]: `pos = tile*0x2000 + 0xF00`, `z = level*0x20 - 1`,
    /// probe cache seeded with z, variant = rng&3, then one
    /// move_is_possible pass settles the floor.
    pub fn spawn_robot(&mut self, marker: (i32, i32, i32)) -> usize {
        let z = marker.2 * 0x20 - 1;
        // HP = the dropship-landing init over the CURRENT battery
        // (7f.8): the record spawns with battery 0 → 5000; a battery
        // staged before spawn lands in one write, after spawn
        // re-runs the landing formula through set_battery.
        let robot = Robot {
            pos_x: marker.0 * Q13_PER_TILE + SPAWN_CENTER,
            pos_y: marker.1 * Q13_PER_TILE + SPAWN_CENTER,
            z,
            state: STATE_IDLE,
            dir_byte: 0,
            facing: FACING_NONE,
            anim: 0,
            variant: (self.rng.next_u32() & 3) as u16,
            probe_z: [z as u16; 8],
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
            weapons: [WeaponSlot::default(); 7],
            weapon_mask: 0,
        };
        let idx = self.robots.len();
        self.robots.push(robot);
        // Settle the floor at the spawn spot (the loader's one probe;
        // best-effort — a floor taller than CLIMB_LIMIT above the
        // seeded z leaves the seeds untouched, amendment 7b.4).
        let (wx, wy) = (self.robots[idx].pos_x, self.robots[idx].pos_y);
        let _ = self.move_possible(idx, wx, wy);
        idx
    }

    /// Arm the click order at a robot [FUN_004247b5, verified]: one
    /// pending order at a time; window 0x197 (0 if exactly one alive
    /// robot); the clicked robot snaps to its spread-assigned tile
    /// center and takes state 3. Returns false when an order is
    /// already pending (the EXW early-out).
    pub fn arm_order_at_robot(&mut self, idx: usize) -> bool {
        if self.order.is_some() {
            return false;
        }
        let (tx, ty) = self.robots[idx].tile();
        let tz = self.robots[idx].z;
        let window = if self.robots.iter().filter(|r| r.alive).count() == 1 {
            0
        } else {
            ORDER_WINDOW
        };
        self.order = Some(Order {
            tile: (tx, ty, tz),
            window,
            claims: [false; 12],
        });
        // The armer also spread-assigns + writes the clicked robot's
        // position (slot 0 = the order tile itself, so the clicked
        // robot snaps to its own tile ORIGIN — no +0xF00 here, unlike
        // the MRK spawn [verified 0x42486a..0x424882]) and sets
        // state 3.
        if let Some(dest) = self.claim_spread_tile() {
            self.robots[idx].pos_x = dest.0 * Q13_PER_TILE;
            self.robots[idx].pos_y = dest.1 * Q13_PER_TILE;
        }
        self.robots[idx].state = STATE_ORDERED;
        true
    }

    /// FUN_004248c8 [verified]: first free claim slot -> the order tile
    /// offset by SPREAD_OFFSETS[slot]. `None` when all 12 are taken
    /// (the EXW out-of-range jump skips the assignment).
    fn claim_spread_tile(&mut self) -> Option<(i32, i32)> {
        let order = self.order.as_mut()?;
        let slot = order.claims.iter().position(|c| !*c)?;
        order.claims[slot] = true;
        let (ox, oy) = (order.tile.0, order.tile.1);
        let (dx, dy) = SPREAD_OFFSETS[slot];
        Some((ox + dx, oy + dy))
    }

    /// One frame: six unit-manager phases, then the order-window tick
    /// (MissionShell order, verified): decrement the window when
    /// nonzero, then clear the order if the window hit 0 or every
    /// robot is dead or state-3 (the window=0 single-robot armer case
    /// therefore clears on the next frame's tick). After the phases,
    /// the portrait-pass hit_flash decay (7g.8) runs for every alive
    /// robot with hp ≥ 1 — the EXW does it inside the FUN_004072bf
    /// sidebar draw for the SLOT robots; every corpus robot is a
    /// slot robot (single squad, base 0).
    pub fn advance_frame(&mut self) {
        // The COMMAND-record consumer (FUN_00409138) runs after the
        // input/click chain and BEFORE the six robot phases
        // [MissionShell §1, verified]. With no staged records it
        // reduces to the recharge pass over zeroed slots — the
        // pre-S3 behavior is unchanged (the S0/S1/S2 chains pin it).
        self.consume_commands();
        for phase in 0..PHASES_PER_FRAME {
            self.robots_phase(phase as i32);
        }
        // The enemy pass [MissionShell §1]: 4× per frame — the
        // weapon-anim tick (FUN_00410823, phase arg i) + the 50×0x22
        // projectile tick (FUN_00412010). FUN_004197d4 (the odd-pass
        // robot-hit expiry walker) is an E-gap until the enemy-fire
        // producers land — nothing stages the 0x22 bank today.
        for i in 0..4 {
            self.weapon_tick(i);
            self.enemy_tick();
        }
        // The mission-epilogue +0x18 fade [7j.10, verified
        // 0x42405a..0x42409e]: FUN_00424051 runs in the epilogue
        // chain right after the debris tick and decrements EVERY
        // nonzero armor-pad/scorch byte on the map by 1 — no gate,
        // every frame. The ring bytes a death writes during the
        // phases therefore start fading the same frame (a value-4
        // center arms its pad for exactly four phase-1 passes).
        for b in &mut self.armor_pads {
            if *b != 0 {
                *b -= 1;
            }
        }
        // The portrait-pass decay (7g.8): clamp 5, decrement, only
        // while alive && hp ≥ 1 && nonzero.
        for r in &mut self.robots {
            if r.alive && r.hp >= 1 && r.hit_flash != 0 {
                r.hit_flash = r.hit_flash.min(5) - 1;
            }
        }
        if let Some(order) = &mut self.order {
            if order.window != 0 {
                order.window -= 1;
            }
            if order.window == 0
                || self
                    .robots
                    .iter()
                    .all(|r| !r.alive || r.state == STATE_ORDERED)
            {
                self.clear_order();
            }
        }
        self.frame += 1;
    }

    /// FUN_0041faf0's hashed half: drop the pending order.
    fn clear_order(&mut self) {
        self.order = None;
    }

    /// The phase gate [verified expression]: phases 0..3 always run;
    /// 4/5 only while `drop_countdown > phase*32`.
    fn phase_gate(phase: i32, robot: &Robot) -> bool {
        phase < 4 || phase * 32 < robot.drop_countdown
    }

    /// One unit-manager pass over all robots (the modeled subset of
    /// FUN_0040b9f6): order consumption + move-toward-target.
    fn robots_phase(&mut self, phase: i32) {
        // The phase-0 pre-walk (7g.2): over ALL records with NO
        // alive gate — the alarm word/counter decay, the shield
        // pool's 2/frame decay, and the +0xA0 booster family. (The
        // EXW also decays word@+0x32 here; its producer is unknown
        // and it is always 0 — deliberately unmodeled, 7g.2.)
        if phase == 0 {
            for r in &mut self.robots {
                if r.alarm != 0 {
                    r.alarm -= 1;
                }
                if r.alarm_ctr != 0 {
                    r.alarm_ctr -= 1;
                }
                if r.shield != 0 {
                    r.shield -= SHIELD_DECAY;
                    if r.shield < 0 {
                        r.shield = 0;
                    }
                }
                if r.shield_boost != 0 {
                    r.shield = SHIELD_BOOST_POOL;
                    r.shield_boost -= 1;
                    if r.shield_boost < 1 {
                        r.shield_boost = 0;
                        r.shield = SHIELD_BOOST_LEFTOVER;
                    }
                }
            }
        }
        for idx in 0..self.robots.len() {
            let robot = &self.robots[idx];
            if !robot.alive || !Self::phase_gate(phase, robot) {
                continue;
            }
            // The armor pass (7g.3): PHASE 1, alive robots only —
            // the pad byte of the tile under the robot center decides
            // charge (+20 behind the +0x98 pool, FUN_004100b7) vs
            // bleed (-10, clamp ≥ 0). The 0x7d2/0x7d3 tile-word gates
            // around it in EXW are unmodeled (7g.5 open producer).
            if phase == 1 {
                let (tx, ty) = self.robots[idx].tile();
                let (w, _) = self.terrain.size();
                let pad = self.armor_pad_byte((ty * w + tx).max(0) as usize);
                if pad != 0 {
                    self.armor_charge(idx, ARMOR_CHARGE);
                } else {
                    let a = self.robots[idx].armor.wrapping_sub(ARMOR_BLEED);
                    self.robots[idx].armor = if a < 0 { 0 } else { a };
                }
            }
            // Order consumption: pending order, state outside {3,4,5},
            // robot within ORDER_RADIUS of the order tile center.
            if let Some(order) = self.order {
                let state = self.robots[idx].state;
                if state != STATE_ORDERED && state != STATE_MOVING && state != 5 {
                    let (qx, qy) = self.robots[idx].q5();
                    let cx = order.tile.0 * Q5_PER_TILE + 0x10;
                    let cy = order.tile.1 * Q5_PER_TILE + 0x10;
                    if dist_octagonal(qx - cx, qy - cy) < ORDER_RADIUS {
                        if let Some(dest) = self.claim_spread_tile() {
                            self.robots[idx].state = STATE_MOVING;
                            self.robots[idx].stop_dist = ORDER_STOP_DIST;
                            self.robots[idx].target =
                                Some((dest.0 * Q5_PER_TILE, dest.1 * Q5_PER_TILE));
                        }
                    }
                }
            }
            // Move toward target (states 1 and 4 with a target).
            let robot = &self.robots[idx];
            if robot.state != STATE_MOVING && robot.state != 1 {
                continue;
            }
            let Some((tqx, tqy)) = robot.target else {
                continue;
            };
            let (qx, qy) = robot.q5();
            let dx_q5 = tqx - qx;
            let dy_q5 = tqy - qy;
            // Distance over Q13 deltas (the EXW caller feeds Δ<<8);
            // min 1. The ARRIVE comparisons use this unclamped value,
            // the velocity divisor is clamped to [1, 0xFFFF] [verified
            // asm 0x40bd90..0x40bf0b].
            let dist_raw = dist_octagonal(dx_q5 << 8, dy_q5 << 8).max(1);
            let angle = self.angles.angle_byte(dx_q5 << 8, dy_q5 << 8);
            if dist_raw > robot.stop_dist || dist_raw < ARRIVE_RADIUS {
                // Arrival [verified]: state 4 -> 3 + snap to the tile
                // grid; state 1 -> idle, clear target + stop distance.
                if self.robots[idx].state == STATE_MOVING {
                    self.robots[idx].state = STATE_ORDERED;
                    self.robots[idx].pos_x &= !0x1FFF;
                    self.robots[idx].pos_y &= !0x1FFF;
                } else {
                    self.robots[idx].state = STATE_IDLE;
                    self.robots[idx].stop_dist = 0;
                    self.robots[idx].target = None;
                }
            } else {
                let dist = dist_raw.min(0xFFFF);
                let vx = ((dx_q5 << 16) / dist) << 2;
                let vy = ((dy_q5 << 16) / dist) << 2;
                self.robot_move(idx, vx, vy, angle);
            }
        }
    }

    /// FUN_004100b7 mechanics [7g.4, verified 0x4100b7..0x4102b6]:
    /// `amount == 0` returns; a nonzero +0x98 pool absorbs the call
    /// (pool -= amount, return while > 0, clamp 0 + the "pool empty"
    /// slot SFX on the transition — presentation); only a zero pool
    /// charges the armor word `+= amount` (i16 wrapping) clamped
    /// ≤ 3000. The old/new SFX families are presentation.
    fn armor_charge(&mut self, idx: usize, amount: i32) {
        if amount == 0 {
            return;
        }
        if self.robots[idx].armor_pool != 0 {
            self.robots[idx].armor_pool -= amount;
            if self.robots[idx].armor_pool > 0 {
                return;
            }
            self.robots[idx].armor_pool = 0;
            return;
        }
        let new = self.robots[idx].armor.wrapping_add(amount as i16);
        self.robots[idx].armor = new;
        if i32::from(new) > i32::from(ARMOR_MAX) {
            self.robots[idx].armor = ARMOR_MAX;
        }
    }

    /// The mover tick [FUN_0040c536, verified]: try the diagonal; on
    /// block pick a cardinal facing from the angle (±0x20 compass
    /// bands) by probing single-axis strides; slide on that axis;
    /// blocked cardinal -> perpendicular axis mover keyed by the
    /// REQUESTED delta sign.
    fn robot_move(&mut self, idx: usize, dx: i32, dy: i32, angle: u16) {
        if self.robots[idx].state == STATE_ORDERED || self.robots[idx].state == 5 {
            return;
        }
        self.robots[idx].dir_byte = angle;
        let saved_z = self.robots[idx].z;
        // Direct diagonal: candidate = (pos + delta) >> 8 [verified].
        let cx = (self.robots[idx].pos_x + dx) >> 8;
        let cy = (self.robots[idx].pos_y + dy) >> 8;
        if self.move_possible(idx, cx, cy) {
            self.robots[idx].pos_x += dx;
            self.robots[idx].pos_y += dy;
            self.robots[idx].facing = FACING_NONE;
            self.robots[idx].anim = (angle.wrapping_add(4) & 0xFF) >> 3;
            return;
        }
        if self.robots[idx].facing == FACING_NONE {
            self.robots[idx].z = saved_z;
            // Compass band of the angle: (angle + 0x20) & 0xC0.
            let facing = match angle.wrapping_add(0x20) & 0xC0 {
                0x0000 => {
                    if self.probe(idx, 0, -MOVE_STRIDE) {
                        FACING_N
                    } else {
                        self.pick_slide_x(idx, dx)
                    }
                }
                0x0040 => {
                    if self.probe(idx, MOVE_STRIDE, 0) {
                        FACING_E
                    } else {
                        self.pick_slide_y(idx, dy)
                    }
                }
                0x0080 => {
                    if self.probe(idx, 0, MOVE_STRIDE) {
                        FACING_S
                    } else {
                        self.pick_slide_x(idx, dx)
                    }
                }
                _ => {
                    if self.probe(idx, -MOVE_STRIDE, 0) {
                        FACING_W
                    } else {
                        self.pick_slide_y(idx, dy)
                    }
                }
            };
            if facing == FACING_NONE {
                return;
            }
            self.robots[idx].facing = facing;
        }
        // Slide on the chosen facing; blocked N/S -> move_x_who,
        // blocked E/W -> move_y_who [verified switch].
        let facing = self.robots[idx].facing;
        let px = self.robots[idx].pos_x;
        let py = self.robots[idx].pos_y;
        match facing {
            FACING_N => {
                if self.move_possible(idx, px, py - MOVE_STRIDE) {
                    self.robots[idx].pos_y -= MOVE_STRIDE;
                } else {
                    self.move_x_who(idx, dx);
                }
            }
            FACING_E => {
                if self.move_possible(idx, px + MOVE_STRIDE, py) {
                    self.robots[idx].pos_x += MOVE_STRIDE;
                } else {
                    self.move_y_who(idx, dy);
                }
            }
            FACING_S => {
                if self.move_possible(idx, px, py + MOVE_STRIDE) {
                    self.robots[idx].pos_y += MOVE_STRIDE;
                } else {
                    self.move_x_who(idx, dx);
                }
            }
            FACING_W => {
                if self.move_possible(idx, px - MOVE_STRIDE, py) {
                    self.robots[idx].pos_x -= MOVE_STRIDE;
                } else {
                    self.move_y_who(idx, dy);
                }
            }
            _ => {}
        }
        // Success tail [verified]: anim from the angle, dir byte =
        // facing, facing consumed.
        self.robots[idx].anim = (angle.wrapping_add(4) & 0xFF) >> 3;
        self.robots[idx].dir_byte = self.robots[idx].facing;
        self.robots[idx].facing = FACING_NONE;
    }

    /// Facing-picker helper for y-dominant bands: the y stride is
    /// blocked, try the x axis ordered by the REQUESTED dx sign
    /// [verified picker order].
    fn pick_slide_x(&mut self, idx: usize, dx: i32) -> u16 {
        if dx < 0 {
            if self.probe(idx, -MOVE_STRIDE, 0) {
                FACING_W
            } else if self.probe(idx, MOVE_STRIDE, 0) {
                FACING_E
            } else {
                FACING_NONE
            }
        } else if self.probe(idx, MOVE_STRIDE, 0) {
            FACING_E
        } else if self.probe(idx, -MOVE_STRIDE, 0) {
            FACING_W
        } else {
            FACING_NONE
        }
    }

    /// Facing-picker helper for x-dominant bands: the x stride is
    /// blocked, try the y axis ordered by the REQUESTED dy sign
    /// [verified picker order].
    fn pick_slide_y(&mut self, idx: usize, dy: i32) -> u16 {
        if dy < 0 {
            if self.probe(idx, 0, -MOVE_STRIDE) {
                FACING_N
            } else if self.probe(idx, 0, MOVE_STRIDE) {
                FACING_S
            } else {
                FACING_NONE
            }
        } else if self.probe(idx, 0, MOVE_STRIDE) {
            FACING_S
        } else if self.probe(idx, 0, -MOVE_STRIDE) {
            FACING_N
        } else {
            FACING_NONE
        }
    }

    /// move_x_who@0040cac2 [verified]: one ±0x400 x stride by the sign
    /// of the requested delta; sets the axis facing on success.
    fn move_x_who(&mut self, idx: usize, dx: i32) {
        let px = self.robots[idx].pos_x;
        let py = self.robots[idx].pos_y;
        if dx < 0 {
            if self.move_possible(idx, px - MOVE_STRIDE, py) {
                self.robots[idx].pos_x -= MOVE_STRIDE;
                self.robots[idx].facing = FACING_W;
            }
        } else if self.move_possible(idx, px + MOVE_STRIDE, py) {
            self.robots[idx].pos_x += MOVE_STRIDE;
            self.robots[idx].facing = FACING_E;
        }
    }

    /// move_y_who@0040cb4f [verified]: one ±0x400 y stride by the sign
    /// of the requested delta; sets the axis facing on success.
    fn move_y_who(&mut self, idx: usize, dy: i32) {
        let px = self.robots[idx].pos_x;
        let py = self.robots[idx].pos_y;
        if dy < 0 {
            if self.move_possible(idx, px, py - MOVE_STRIDE) {
                self.robots[idx].pos_y -= MOVE_STRIDE;
                self.robots[idx].facing = FACING_N;
            }
        } else if self.move_possible(idx, px, py + MOVE_STRIDE) {
            self.robots[idx].pos_y += MOVE_STRIDE;
            self.robots[idx].facing = FACING_S;
        }
    }

    /// Non-mutating probe [FUN_0040cbda move_is_possible2, verified]:
    /// full footprint test but the robot's z is restored afterward
    /// (the probe-z cache mutation is kept, as in EXW).
    fn probe(&mut self, idx: usize, dx: i32, dy: i32) -> bool {
        let saved = self.robots[idx].z;
        let px = self.robots[idx].pos_x;
        let py = self.robots[idx].pos_y;
        let ok = self.move_possible(idx, px + dx, py + dy);
        self.robots[idx].z = saved;
        ok
    }

    /// move_is_possible@0041e897 [verified; amendment 7b.2/3]. NOTE on
    /// scale: the EXW function receives the candidate ALREADY in Q5
    /// (`(pos+delta)>>8` at every call site); this method takes Q13-world
    /// coordinates and performs that shift itself, so callers pass
    /// `pos + delta` verbatim (see robot_move/probe/move_x_who/
    /// move_y_who). Each probe i queries and climb-compares against ITS
    /// OWN cached word `probe_z[i]` (sar → signed; 0xFFFF = −1); on any
    /// failure nothing is written, on pass the probe floors are cached
    /// and the center re-read sets the robot z.
    fn move_possible(&mut self, idx: usize, wx: i32, wy: i32) -> bool {
        let (q5x, q5y) = (wx >> 8, wy >> 8);
        let (w, h) = self.terrain.size();
        let Self {
            robots, terrain, ..
        } = self;
        let robot = &mut robots[idx];
        let mut floors = [0u16; 8];
        for (((&ox, &oy), &pz), f) in PROBE_X
            .iter()
            .zip(PROBE_Y.iter())
            .zip(robot.probe_z.iter())
            .zip(floors.iter_mut())
        {
            let px = q5x + ox;
            let py = q5y + oy;
            if px < 0 || px >> 5 >= w || py < 0 || py >> 5 >= h {
                return false;
            }
            // zref = sign-extended probe cache word i: both the
            // get_z_pos z input and the climb reference.
            let zref = pz as i16 as i32;
            let fl = terrain.floor_z(px, py, zref);
            if (fl - zref).abs() > CLIMB_LIMIT {
                return false;
            }
            *f = fl as u16;
        }
        // Center re-read: z clamped only from above at the call site
        // (min(z, 0xFF)); floor_z clamps 0..0xFF internally, so passing
        // the raw z is equivalent.
        robot.z = terrain.floor_z(q5x, q5y, robot.z);
        robot.probe_z = floors;
        true
    }

    /// State hash over the per-tick mover coverage list
    /// (docs/RE-EXW-SIM.md sec 7): frame, order + claims, robot
    /// fields, the terrain trigger latch, and the RNG state. FNV-1a in
    /// the pinned order below; any consumed-input divergence shifts it.
    pub fn state_hash(&self) -> StateHash {
        let mut h = Fnv1a64::new();
        h.write_u64(self.frame);
        match self.order {
            None => h.write_u8(0),
            Some(o) => {
                h.write_u8(1);
                h.write_i32(o.tile.0);
                h.write_i32(o.tile.1);
                h.write_i32(o.tile.2);
                h.write_u16(o.window);
                for c in o.claims {
                    h.write_u8(u8::from(c));
                }
            }
        }
        for r in &self.robots {
            h.write_u8(u8::from(r.alive));
            h.write_i32(r.pos_x);
            h.write_i32(r.pos_y);
            h.write_i32(r.z);
            h.write_u16(r.state);
            h.write_u16(r.dir_byte);
            h.write_u16(r.facing);
            h.write_u16(r.anim);
            h.write_u16(r.variant);
            for z in r.probe_z {
                h.write_u16(z);
            }
            h.write_i32(r.stop_dist);
            match r.target {
                None => {
                    h.write_u8(0);
                    h.write_i32(0);
                    h.write_i32(0);
                }
                Some((tx, ty)) => {
                    h.write_u8(1);
                    h.write_i32(tx);
                    h.write_i32(ty);
                }
            }
            h.write_i32(r.drop_countdown);
            // The damage-unit fields (7g, D52 follow-up): appended in
            // record-offset order after the modeled set — hp, armor,
            // hit_flash, alarm, kind, shield, shield_charges,
            // shield_boost, battery, armor_pool, alarm_ctr,
            // death_flag.
            h.write_i32(r.hp);
            h.write_i16(r.armor);
            h.write_u16(r.hit_flash);
            h.write_u16(r.alarm);
            h.write_u16(r.kind);
            h.write_i32(r.shield);
            h.write_i32(r.shield_charges);
            h.write_i32(r.shield_boost);
            h.write_i32(r.battery);
            h.write_i32(r.armor_pool);
            h.write_i32(r.alarm_ctr);
            h.write_u16(r.death_flag);
        }
        match self.terrain.last_trigger {
            None => h.write_u8(0),
            Some((z, x, y)) => {
                h.write_u8(1);
                h.write_i32(z);
                h.write_i32(x);
                h.write_i32(y);
            }
        }
        h.write_u64(self.rng.state());
        h.write_u64(self.rng.stream());
        StateHash(h.finish())
    }
}

/// PCG stream for mission-sim draws (kept distinct from the skeleton
/// `Sim` stream so the buckets can merge without renumbering).
const STREAM_MISSION: u64 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_terrain(w: i32, h: i32, floor_type: u8, heights: Vec<[u8; 1024]>) -> Terrain {
        let mut dat = vec![floor_type; (8 * w * h) as usize];
        // DAT plane 1 is the walkable deck in these tests: fill plane 0
        // with the type, plane 1 empty like a one-level room.
        for b in dat.iter_mut().take((w * h) as usize) {
            *b = floor_type;
        }
        Terrain::from_parts(w, h, dat, heights).unwrap()
    }

    fn zero_heights(n: usize) -> Vec<[u8; 1024]> {
        vec![[0u8; 1024]; n]
    }

    /// Threshold table shaped like the corpus quarter-sine (0x647..).
    fn sintable_like() -> AngleTable {
        let mut words = [0i16; 256];
        for i in 0..64 {
            let v = 0x0647 + (0x7FFF - 0x0647) * i as i32 / 63;
            words[2 + i] = v as i16;
        }
        AngleTable::from_sintable_words(&words).unwrap()
    }

    #[test]
    fn dist_octagonal_matches_original() {
        // max + min/2 with BOTH args abs'd first — the result is always
        // non-negative [FUN_0041ebf8, verified cdq/xor/sub ×2 at
        // 0x41ebfc..0x41ec08; amendment 7b.1].
        assert_eq!(dist_octagonal(100, 0), 100);
        assert_eq!(dist_octagonal(0, -7), 7);
        assert_eq!(dist_octagonal(10, 4), 12);
        assert_eq!(dist_octagonal(-10, -4), 12);
        assert_eq!(dist_octagonal(5, 5), 7);
    }

    #[test]
    fn angle_fold_cardinals() {
        let t = sintable_like();
        // Pure +y (south in map space): dx=0, dy>0 -> 0x7F - sector(0)
        // with ratio 0 -> sector 0 => 0x7F... the cardinal for "mostly
        // south" comes out 0x80 only when |dx| is small but nonzero;
        // pin the exact quadrant-fold outputs instead of the sector
        // granularity:
        let a = t.angle_byte(0, 256);
        assert_eq!(a & 0xC0, 0x40, "dy>0 quadrant folds into E/S band");
        let b = t.angle_byte(-256, 256);
        assert_eq!(b & 0xC0, 0x80, "dx<0 dy>0 -> S/W band");
        let c = t.angle_byte(-256, -256);
        assert!((0x100 - 0x3F..=0x100).contains(&c), "dx<0 dy<0 -> top fold");
        // Ratio peak: pure x hits sector 0x3F.
        assert_eq!(t.angle_byte(0x7FFF, 0), 0x3F);
    }

    #[test]
    fn floor_z_empty_and_0xff_remap() {
        let mut t = flat_terrain(4, 4, 0, zero_heights(1));
        assert_eq!(t.floor_z(16, 16, 0), 0, "no type anywhere -> z 0");
        let mut t2 = flat_terrain(4, 4, 0xFF, zero_heights(1));
        // 0xFF reads back as type 1 -> height slot 0.
        assert_eq!(t2.dat_type(0, 0, 0), 1);
        assert_eq!(t2.floor_z(16, 16, 0), 0, "type 1 height 0 -> z 0");
    }

    /// A minimal valid CGR bank for `slot_bodies`: u16 count + u32
    /// directory + per-slot (8-byte header + 1024-byte raw map). The
    /// directory values are chosen so the EXW body address
    /// `dir[s] + 4*s + 8` [0x41e328, verified] lands exactly on each
    /// slot's 1024-byte map (real files store the same relationship:
    /// dir[s] is NOT a plain file offset — dir[1]=1538 in MISSIONA.CGR
    /// puts slot 1's body at 1550, 6 bytes past its 1544 header).
    fn cgr_bank(slot_bodies: &[[u8; 1024]]) -> Vec<u8> {
        let count = slot_bodies.len();
        let mut v = vec![count as u8, 0];
        // body_s = 2 + 4*count + s*(8 + 1024) + 8 (after its header)
        let body_at = |s: usize| 2 + 4 * count + s * (8 + 1024) + 8;
        for s in 0..count {
            let dir = (body_at(s) - 4 * s - 8) as u32;
            v.extend_from_slice(&dir.to_le_bytes());
        }
        for body in slot_bodies {
            v.extend_from_slice(&1u32.to_le_bytes());
            v.extend_from_slice(&32u16.to_le_bytes());
            v.extend_from_slice(&32u16.to_le_bytes());
            v.extend_from_slice(body);
        }
        v
    }

    fn dat_file(w: u16, h: u16, planes: &[u8]) -> Vec<u8> {
        let mut v = w.to_le_bytes().to_vec();
        v.extend_from_slice(&h.to_le_bytes());
        v.extend_from_slice(planes);
        v
    }

    fn pad_file(recs: &[(u16, u16, u16)]) -> Vec<u8> {
        let mut v = Vec::new();
        for (x, y, k) in recs {
            v.extend_from_slice(&x.to_le_bytes());
            v.extend_from_slice(&y.to_le_bytes());
            v.extend_from_slice(&k.to_le_bytes());
        }
        v.extend_from_slice(&[0xFF; 6]); // terminator fill
        v
    }

    #[test]
    fn from_mission_bytes_loader_rules() {
        // 4x2 map, plane 0 all type 1 (deck), plane 4 one 0x90 byte to
        // sweep, plus a PAD mark at level 5 tile (1,1).
        let w = 4u16;
        let h = 2u16;
        let n = (w * h) as usize;
        let mut planes = vec![0u8; 8 * n];
        for b in planes[..n].iter_mut() {
            *b = 1;
        }
        planes[4 * n + 3] = 0x90; // swept in plane 4
        planes[7 * n + 1] = 0x90; // plane 7 NOT swept
        let mut bodies = vec![[7u8; 1024]; 2];
        bodies[0] = [0x1Fu8; 1024];
        let mut t = Terrain::from_mission_bytes(
            &dat_file(w, h, &planes),
            &pad_file(&[(1, 1, 5), (0, 0, 9)]), // 9 = out of range, skipped
            &cgr_bank(&bodies),
        )
        .unwrap();
        assert_eq!(t.size(), (4, 2));
        // Deck floor: type 1 -> slot 0 height 0x1F -> z 0x1F.
        assert_eq!(t.floor_z(16, 16, 0), 0x1F);
        // Sweep cleared the plane-4 byte; plane 7 untouched.
        assert_eq!(t.dat_type(3, 0, 4), 0);
        assert_eq!(t.dat_type(1, 0, 7), 0x90);
        // PAD mark: level 5, tile (1,1) reads back as type 1.
        assert_eq!(t.dat_type(1, 1, 5), 1);
        assert_eq!(t.dat_type(1, 1, 4), 0, "mark is plane-local");
        // Second slot kept its own body (type 2 -> slot 1 height 7).
        let mut t2planes = planes.clone();
        t2planes[..n].copy_from_slice(&[2, 2, 2, 2, 2, 2, 2, 2]);
        let mut t2 = Terrain::from_mission_bytes(
            &dat_file(w, h, &t2planes),
            &pad_file(&[]),
            &cgr_bank(&bodies),
        )
        .unwrap();
        assert_eq!(t2.floor_z(16, 16, 0), 7);
        // Malformed inputs -> None (no panics).
        assert!(Terrain::from_mission_bytes(&[0, 4], &[], &[]).is_none());
        let truncated_cgr = [1u8, 0]; // claims 1 slot, no directory
        assert!(
            Terrain::from_mission_bytes(&dat_file(2, 2, &[0; 32]), &[0xFF; 6], &truncated_cgr)
                .is_none()
        );
    }

    #[test]
    fn floor_z_search_and_slope() {
        // Type 5 at level 0 with height 8; type 6 at level 1.
        let mut heights = zero_heights(8);
        heights[4] = [8u8; 1024]; // type 5 slot
        heights[5] = [4u8; 1024]; // type 6 slot
        let w: i32 = 4;
        let h: i32 = 4;
        let mut dat = vec![0u8; (8 * w * h) as usize];
        dat[0] = 5; // plane 0, tile (0,0)
        dat[(w * h) as usize] = 6; // plane 1, tile (0,0)
        let mut t = Terrain::from_parts(w, h, dat, heights.clone()).unwrap();
        // z query at level 0 finds type 5 -> 0*0x20 + 8.
        assert_eq!(t.floor_z(16, 16, 0), 8);
        // Level-1 type 6 with height 4 -> 0x24.
        assert_eq!(t.floor_z(16, 16, 0x20), 0x24);
        // 0x1F top rule: type 7 (slot 6) height 0x1F, level+1 type 6
        // height 4 -> bridges to 0x24.
        heights[6] = [0x1Fu8; 1024];
        let mut dat2 = vec![0u8; (8 * w * h) as usize];
        dat2[0] = 7;
        dat2[(w * h) as usize] = 6;
        let mut t2 = Terrain::from_parts(w, h, dat2, heights).unwrap();
        assert_eq!(t2.floor_z(16, 16, 0), 0x24);
        // Type 3 latch.
        let mut dat3 = vec![0u8; (8 * w * h) as usize];
        dat3[0] = 3;
        let mut t3 = Terrain::from_parts(w, h, dat3, zero_heights(8)).unwrap();
        let _ = t3.floor_z(16, 16, 0);
        assert_eq!(t3.last_trigger, Some((0, 0, 0)));
    }

    #[test]
    fn spawn_settles_and_tiles() {
        // Height byte 3 at level 0: the seeded probe zref (level-0
        // marker → 0xFFFF → −1 signed) climbs: |3−(−1)| = 4 ≤ 4, so the
        // loader's one settle probe passes and the floor is written.
        let mut sim = MissionSim::new(
            flat_terrain(8, 8, 5, {
                let mut hs = zero_heights(8);
                hs[4] = [3u8; 1024];
                hs
            }),
            sintable_like(),
            12345,
        );
        let idx = sim.spawn_robot((2, 3, 0));
        let r = &sim.robots()[idx];
        assert_eq!(r.pos_x, 2 * Q13_PER_TILE + SPAWN_CENTER);
        assert_eq!(r.pos_y, 3 * Q13_PER_TILE + SPAWN_CENTER);
        assert_eq!(r.tile(), (2, 3));
        assert_eq!(r.z, 3, "floor settled through the spawn probe");
        assert_eq!(r.probe_z[0], 3);
        assert_eq!(r.probe_z[7], 3);
        // Tall floor (height 8): |8−(−1)| = 9 > 4 → the settle is
        // best-effort and FAILS, leaving the seeds untouched (z −1,
        // probe words 0xFFFF) — faithful EXW behavior [7b.4].
        let mut sim2 = MissionSim::new(
            flat_terrain(8, 8, 5, {
                let mut hs = zero_heights(8);
                hs[4] = [8u8; 1024];
                hs
            }),
            sintable_like(),
            12345,
        );
        let idx2 = sim2.spawn_robot((2, 3, 0));
        let r2 = &sim2.robots()[idx2];
        assert_eq!(r2.z, -1, "tall floor: no settle");
        assert_eq!(r2.probe_z[0], 0xFFFF);
    }

    #[test]
    fn order_walk_and_arrival_snap() {
        // Flat 16x16, floor type 5 height 3 (walkable from the seeded
        // spawn probes: |3−(−1)| = 4).
        let mut hs = zero_heights(8);
        hs[4] = [3u8; 1024];
        let mut sim = MissionSim::new(flat_terrain(16, 16, 5, hs), sintable_like(), 7);
        let a = sim.spawn_robot((2, 8, 0));
        let b = sim.spawn_robot((6, 8, 0));
        // Click on robot a: order tile = a's tile; b is 4 tiles away
        // (inside the 6-tile radius) and gets spread slot 1 = order
        // tile +1 x, so it walks ~3 tiles west and snaps on arrival.
        assert!(sim.arm_order_at_robot(a));
        assert!(!sim.arm_order_at_robot(b), "one pending order at a time");
        assert_eq!(sim.robots()[a].state, STATE_ORDERED);
        // The armer snap writes the tile ORIGIN (no +0xF00).
        assert_eq!(sim.robots()[a].pos_x, 2 * Q13_PER_TILE);
        assert_eq!(sim.robots()[a].pos_y, 8 * Q13_PER_TILE);
        // Run frames until b arrives (it starts IDLE, is picked up on
        // the first frame's phases, then walks to the spread tile).
        let mut frames = 0;
        while frames < 200 && sim.robots()[b].state != STATE_ORDERED {
            sim.advance_frame();
            frames += 1;
        }
        assert!(frames < 200, "walk terminates");
        let rb = &sim.robots()[b];
        assert_eq!(rb.state, STATE_ORDERED, "state 4 -> 3 on arrival");
        assert_eq!(rb.pos_x & 0x1FFF, 0, "arrival snaps to the tile grid");
        assert_eq!(rb.pos_y & 0x1FFF, 0, "arrival snaps to the tile grid");
        // Slot 1 spread offset = (+1, 0): target tile = a's tile + 1.
        assert_eq!(rb.pos_x >> 13, 3);
        assert_eq!(rb.pos_y >> 13, 8);
        assert!(sim.order().is_none(), "order consumed once all state-3");
    }

    #[test]
    fn climb_blocks_across_a_wall() {
        // Single wall column x == 6 (type 6, height 20); floor type 5
        // (height 3) everywhere else. Both robots stand a clear tile
        // from the wall so their spawn settles pass; b (east) is 5.97
        // tiles from the order tile (inside the 0xC0 radius) and its
        // slot-1 spread target (4, 8) lies across the wall: the 8-probe
        // footprint always has a probe on the wall column when
        // crossing, |20-3| = 17 > CLIMB_LIMIT, so b can never reach
        // tile <= 6 and never arrives.
        let mut hs = zero_heights(8);
        hs[4] = [3u8; 1024];
        hs[5] = [20u8; 1024];
        let w = 16;
        let h = 16;
        let mut dat = vec![0u8; (8 * w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                dat[(y * w + x) as usize] = if x == 6 { 6u8 } else { 5u8 };
            }
        }
        let mut sim = MissionSim::new(
            Terrain::from_parts(w, h, dat, hs).unwrap(),
            sintable_like(),
            3,
        );
        let a = sim.spawn_robot((3, 8, 0)); // west of the wall
        let b = sim.spawn_robot((9, 8, 0)); // east, 5.97 tiles from a
        assert!(sim.arm_order_at_robot(a));
        assert_eq!(sim.robots()[b].z, 3, "b settled on the east floor");
        for _ in 0..60 {
            sim.advance_frame();
            assert!(sim.robots()[b].pos_x >> 13 >= 7, "wall never crossed");
        }
        assert_eq!(
            sim.robots()[b].state,
            STATE_MOVING,
            "unreachable target never arrives"
        );
    }

    #[test]
    fn determinism_two_runs_identical_hashes() {
        let mut hs = zero_heights(8);
        hs[4] = [8u8; 1024];
        let run = |seed: u64| -> Vec<StateHash> {
            let mut sim =
                MissionSim::new(flat_terrain(16, 16, 5, hs.clone()), sintable_like(), seed);
            let a = sim.spawn_robot((2, 8, 0));
            let _ = sim.spawn_robot((3, 8, 0));
            sim.arm_order_at_robot(a);
            let mut out = Vec::new();
            for _ in 0..40 {
                sim.advance_frame();
                out.push(sim.state_hash());
            }
            out
        };
        let x = run(99);
        let y = run(99);
        assert_eq!(x, y, "same seed -> identical hash stream");
        let z = run(100);
        assert_ne!(x[0], z[0], "seed enters the hash");
    }

    #[test]
    fn hash_covers_mover_fields() {
        let mut hs = zero_heights(8);
        hs[4] = [8u8; 1024];
        let mut sim = MissionSim::new(flat_terrain(8, 8, 5, hs), sintable_like(), 5);
        let a = sim.spawn_robot((2, 3, 0));
        let base = sim.state_hash();
        sim.robots_mut()[a].pos_x += 1;
        assert_ne!(sim.state_hash().0, base.0, "pos_x covered");
        sim.robots_mut()[a].anim ^= 1;
        assert_ne!(sim.state_hash().0, base.0, "anim covered");
    }

    /// A sim with one robot staged for damage tests (state IDLE, all
    /// vitals at the spawn defaults unless the test overrides them).
    fn damage_sim() -> MissionSim {
        let hs = zero_heights(8);
        let mut sim = MissionSim::new(flat_terrain(8, 8, 5, hs), sintable_like(), 11);
        sim.spawn_robot((2, 3, 0));
        sim
    }

    #[test]
    fn damage_core_follows_fun_0040e230() {
        // 7g.1: state-3 robots convert damage into a shield tick.
        let mut sim = damage_sim();
        sim.robots_mut()[0].state = STATE_ORDERED;
        let out = sim.apply_damage(0, 100, -1);
        assert!(out.applied && !out.died);
        assert_eq!(sim.robots()[0].shield, 0x20);
        assert_eq!(
            sim.robots()[0].hp,
            5000,
            "the ordered conversion takes nothing"
        );

        // The auto-shield idle: charges > 0 with shield == 0 spends
        // one charge for the 0x20 tick.
        let mut sim = damage_sim();
        sim.robots_mut()[0].shield_charges = 2;
        sim.apply_damage(0, 100, -1);
        assert_eq!(
            (sim.robots()[0].shield_charges, sim.robots()[0].shield),
            (1, 0x20)
        );
        assert_eq!(sim.robots()[0].hp, 5000);

        // The shield absorbs before hp and clamps at 0 on overflow.
        let mut sim = damage_sim();
        sim.robots_mut()[0].shield = 100;
        sim.apply_damage(0, 30, -1);
        assert_eq!((sim.robots()[0].shield, sim.robots()[0].hp), (70, 5000));
        sim.apply_damage(0, 80, -1);
        assert_eq!((sim.robots()[0].shield, sim.robots()[0].hp), (0, 5000));

        // The hp path: hit_flash bumps BEFORE the hp subtract.
        let mut sim = damage_sim();
        sim.apply_damage(0, 100, -1);
        assert_eq!((sim.robots()[0].hit_flash, sim.robots()[0].hp), (1, 4900));

        // Gates: dead and state-2 robots are untouched.
        let mut sim = damage_sim();
        sim.robots_mut()[0].alive = false;
        assert!(!sim.apply_damage(0, 100, -1).applied);
        sim.robots_mut()[0].alive = true;
        sim.robots_mut()[0].state = 2;
        assert!(!sim.apply_damage(0, 100, -1).applied);
        assert_eq!(sim.robots()[0].hp, 5000);
    }

    #[test]
    fn damage_alarm_accumulator_trips_at_100() {
        // 7g.1: +3 per un-alarm'd hit; > 100 on a player-type robot
        // trips the alarm word to 100 and resets the accumulator.
        // 34 hits x 3 = 102 > 100 with hp raised first.
        let mut sim = damage_sim();
        sim.set_battery(0, 90); // hp 14000 — survives 34 x 100
        for _ in 0..33 {
            sim.apply_damage(0, 100, -1);
        }
        assert_eq!((sim.robots()[0].alarm, sim.robots()[0].alarm_ctr), (0, 99));
        sim.apply_damage(0, 100, -1);
        assert_eq!((sim.robots()[0].alarm, sim.robots()[0].alarm_ctr), (100, 0));
        // While the alarm word is nonzero the accumulator idles.
        sim.apply_damage(0, 100, -1);
        assert_eq!(sim.robots()[0].alarm_ctr, 0);
    }

    #[test]
    fn death_subset_clears_vitals_and_draws_ten_stream_values() {
        // 7g.6: hp < 1 clears alive/hp/drop/armor, sets the death
        // flag, and stages five debris from the SHARED stream (two
        // draws each — the y jitter first).
        let mut sim = damage_sim();
        sim.robots_mut()[0].armor = 1234;
        sim.robots_mut()[0].drop_countdown = 77;
        let hash_before = sim.state_hash().0;
        let out = sim.apply_damage(0, 9999, -1);
        assert!(out.applied && out.died);
        let r = sim.robots()[0];
        assert!(!r.alive);
        assert_eq!(
            (r.hp, r.armor, r.drop_countdown, r.death_flag),
            (0, 0, 0, 1)
        );
        // Ten draws left the stream (the hash covers the rng state).
        assert_ne!(sim.state_hash().0, hash_before);
        // The debris jitter: (rand & 0x1f) + pos>>8 - 0x10 — the y
        // (row 1) uses the FIRST draw of each pair, x the second.
        let (px, py, z) = (r.pos_x, r.pos_y, r.z);
        let mut replay = damage_sim();
        replay.robots_mut()[0].armor = 1234;
        replay.robots_mut()[0].drop_countdown = 77;
        replay.apply_damage(0, 9999, -1);
        for (k, d) in out.debris.iter().enumerate() {
            assert_eq!(d.2, z + 8 * k as i32, "debris z walks +8k");
            assert_eq!(d.3, 2 * k as i32, "the phase param is 2k");
            assert!(
                ((px >> 8) - 0x10..=(px >> 8) + 0x1F).contains(&d.0),
                "x jitter window k={k}"
            );
            assert!(
                ((py >> 8) - 0x10..=(py >> 8) + 0x1F).contains(&d.1),
                "y jitter window k={k}"
            );
        }
        // A second lethal hit on the corpse does nothing.
        assert!(!sim.apply_damage(0, 9999, -1).applied);
    }

    #[test]
    fn armor_bleeds_off_pad_and_charges_on_pads() {
        // 7g.3: phase 1 bleeds -10/frame (clamp 0) with the default
        // all-zero pad bytes — the shipped ZONEA behavior.
        let mut sim = damage_sim();
        sim.robots_mut()[0].armor = 100;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 90);
        sim.robots_mut()[0].armor = 5;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 0, "the bleed clamps at 0");

        // A staged pad byte under the robot (tile (2,3) on the 8x8
        // map = linear 26) charges +20/frame instead. The byte is
        // TRANSIENT (7j.10): the epilogue fade decays it 1/frame,
        // so stage the max value 7 to keep it armed across these
        // two frames (7j.9 clamps writes at 7).
        let mut pads = vec![0u8; 64];
        pads[3 * 8 + 2] = 7;
        sim.set_armor_pads(&pads);
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 20);
        assert_eq!(sim.armor_pad_byte(3 * 8 + 2), 6, "the fade ran");
        // Clamp at 3000 (0xBB8).
        sim.robots_mut()[0].armor = 2995;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 3000);

        // The +0x98 pool drains BEFORE armor charges (7g.4): pool 50
        // needs three passes (50->30->10->0) before the first +20 —
        // the value-7 pad outlives the four frames (7j.10 fade).
        let mut sim = damage_sim();
        let mut pads = vec![0u8; 64];
        pads[3 * 8 + 2] = 7;
        sim.set_armor_pads(&pads);
        sim.robots_mut()[0].armor_pool = 50;
        sim.advance_frame();
        assert_eq!((sim.robots()[0].armor_pool, sim.robots()[0].armor), (30, 0));
        sim.advance_frame();
        assert_eq!((sim.robots()[0].armor_pool, sim.robots()[0].armor), (10, 0));
        sim.advance_frame();
        assert_eq!((sim.robots()[0].armor_pool, sim.robots()[0].armor), (0, 0));
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 20, "the drained pool charges");
    }

    #[test]
    fn death_stages_the_3x3_scorch_ring_into_the_pad_mirror() {
        // 7j.9: the death tail writes the NINE-tile scorch ring per
        // debris (corners 1, edges 2, center 4) into the type-DB
        // +0x18 mirror, in the EXW order — overlapping rings are
        // last-write-wins in staging order, and the bytes arm the
        // phase-1 armor-pad charge exactly like the original.
        let mut sim = damage_sim();
        let out = sim.apply_damage(0, 9999, -1);
        assert!(out.died);
        let (w, h) = sim.terrain.size();

        // The ring table itself: the 3×3 pattern at tile ±1.
        let mut pattern = std::collections::HashMap::new();
        for &(dx, dy, v) in DEBRIS_SCORCH_RING.iter() {
            pattern.insert((dx / 0x20, dy / 0x20), v);
        }
        for (ty, row) in [[1, 2, 1], [2, 4, 2], [1, 2, 1]].iter().enumerate() {
            for (tx, &v) in row.iter().enumerate() {
                assert_eq!(
                    pattern[&(tx as i32 - 1, ty as i32 - 1)],
                    v,
                    "ring pattern at ({},{})",
                    tx as i32 - 1,
                    ty as i32 - 1
                );
            }
        }

        // Fold the five rings in staging order (the expected final
        // byte state) and compare every tile in the covered span.
        let mut expected = std::collections::HashMap::new();
        for d in out.debris.iter() {
            for &(dx, dy, v) in DEBRIS_SCORCH_RING.iter() {
                let tx = (d.0 + dx) >> 5;
                let ty = (d.1 + dy) >> 5;
                if tx >= 0 && ty >= 0 && tx < w && ty < h {
                    expected.insert((tx, ty), v);
                }
            }
        }
        assert!(
            expected.len() < 45,
            "the five jittered rings overlap ({} distinct tiles)",
            expected.len()
        );
        for ty in -1..=6i32 {
            for tx in -1..=6i32 {
                let byte = if tx >= 0 && ty >= 0 && tx < w && ty < h {
                    sim.armor_pad_byte((ty * w + tx) as usize)
                } else {
                    continue;
                };
                assert_eq!(
                    i32::from(byte),
                    i32::from(expected.get(&(tx, ty)).copied().unwrap_or(0)),
                    "pad byte at tile ({tx},{ty})"
                );
            }
        }

        // A scorched tile charges armor on the next phase-1 pass
        // (the raw-reader semantics, 7j.9 item 1): kill first, pick
        // a scorched tile FROM the fold, spawn a live survivor
        // there, and run one frame — deterministic for the fixed
        // seed, no jitter assumptions.
        let mut sim2 = MissionSim::new(flat_terrain(8, 8, 5, zero_heights(8)), sintable_like(), 11);
        let first = sim2.spawn_robot((2, 3, 0));
        let _ = sim2.apply_damage(first, 9999, -1);
        let (sx, sy) = *expected.keys().next().unwrap();
        let survivor = sim2.spawn_robot((sx, sy, 0));
        sim2.robots_mut()[survivor].armor = 0;
        assert_ne!(
            sim2.armor_pad_byte((sy * w + sx) as usize),
            0,
            "the folded tile is scorched in the twin sim too"
        );
        sim2.advance_frame();
        assert_eq!(
            sim2.robots()[survivor].armor,
            20,
            "a survivor on a scorched tile charges +20 (the raw reader)"
        );
    }

    #[test]
    fn the_epilogue_fade_decays_every_nonzero_pad_byte_per_frame() {
        // 7j.10: FUN_00424051's head (0x42405a..0x42409e) runs in
        // the mission epilogue every frame, unconditionally: every
        // nonzero +0x18 byte −1. The death ring is therefore
        // TRANSIENT — a value-4 center arms its pad for exactly
        // four phase-1 passes, then bleeds.
        let mut sim = damage_sim();
        let mut pads = vec![0u8; 64];
        pads[0] = 1;
        pads[1] = 7;
        pads[63] = 3;
        sim.set_armor_pads(&pads);
        sim.advance_frame();
        assert_eq!(
            (
                sim.armor_pad_byte(0),
                sim.armor_pad_byte(1),
                sim.armor_pad_byte(63)
            ),
            (0, 6, 2),
            "every nonzero byte decays 1/frame; zeros stay 0"
        );
        // A value-1 pad arms exactly ONE phase-1 charge: frame 1
        // charges +20, the fade clears the byte, frame 2 bleeds.
        let mut sim = damage_sim();
        let mut pads = vec![0u8; 64];
        pads[3 * 8 + 2] = 1;
        sim.set_armor_pads(&pads);
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 20, "the single frame charges");
        sim.advance_frame();
        assert_eq!(sim.robots()[0].armor, 10, "the faded pad bleeds -10");
        // The death ring fades too: kill, then run value frames.
        let mut sim = damage_sim();
        let _ = sim.apply_damage(0, 9999, -1);
        let peak = (0..64).map(|t| sim.armor_pad_byte(t)).max().unwrap();
        assert!((1..=7).contains(&peak));
        for _ in 0..i32::from(peak) {
            sim.advance_frame();
        }
        assert_eq!(
            (0..64).map(|t| sim.armor_pad_byte(t)).max().unwrap(),
            0,
            "the whole ring faded after `peak` frames"
        );
    }

    #[test]
    fn scorch_write_bounds_and_clamp_follow_fun_00422287() {
        // 7j.9: negative/out-of-map world coords are dropped; the
        // zero-extended byte value clamps >= 8 -> 7; the mirror
        // grows zero-padded on first write.
        let mut sim = damage_sim();
        assert_eq!(sim.armor_pad_byte(0), 0, "no writes yet");
        sim.scorch_write(2 * 0x20 + 5, 3 * 0x20 + 7, 4);
        assert_eq!(sim.armor_pad_byte(3 * 8 + 2), 4, "in-tile offsets >>5");
        sim.scorch_write(2 * 0x20, 3 * 0x20, 9);
        assert_eq!(sim.armor_pad_byte(3 * 8 + 2), 7, "9 clamps to 7");
        sim.scorch_write(2 * 0x20, 3 * 0x20, 0xFF);
        assert_eq!(sim.armor_pad_byte(3 * 8 + 2), 7, "0xFF clamps to 7");
        // Bounds: x/y < 0 or >= map w/h are dropped.
        sim.scorch_write(-1, 0, 4);
        sim.scorch_write(0, -1, 4);
        sim.scorch_write(8 * 0x20, 0, 4);
        sim.scorch_write(0, 8 * 0x20, 4);
        for tile in [0usize, 8, 7 * 8, 63] {
            assert_eq!(sim.armor_pad_byte(tile), 0, "out-of-map writes dropped");
        }
        // The growth is zero-padded: untouched tiles between the
        // writes read 0.
        assert_eq!(sim.armor_pad_byte(3 * 8 + 3), 0);
    }

    #[test]
    fn shield_decay_and_booster_follow_the_phase0_pre_walk() {
        // 7g.2: the pool decays 2/frame, clamped at 0.
        let mut sim = damage_sim();
        sim.robots_mut()[0].shield = 5;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].shield, 3);
        sim.advance_frame();
        assert_eq!(sim.robots()[0].shield, 1);
        sim.advance_frame();
        assert_eq!(sim.robots()[0].shield, 0, "-1 clamps to 0");

        // The +0xA0 booster: 10000 while counting down, 150 left on
        // expiry.
        let mut sim = damage_sim();
        sim.robots_mut()[0].shield_boost = 2;
        sim.advance_frame();
        assert_eq!(
            (sim.robots()[0].shield, sim.robots()[0].shield_boost),
            (10000, 1)
        );
        sim.advance_frame();
        assert_eq!(
            (sim.robots()[0].shield, sim.robots()[0].shield_boost),
            (150, 0)
        );
        sim.advance_frame();
        assert_eq!(sim.robots()[0].shield, 148, "normal decay resumes");
    }

    #[test]
    fn alarm_and_hit_flash_decay_per_frame() {
        // 7g.2: the alarm word/counter decay in the phase-0 pre-walk.
        let mut sim = damage_sim();
        sim.robots_mut()[0].alarm = 3;
        sim.robots_mut()[0].alarm_ctr = 7;
        sim.advance_frame();
        assert_eq!((sim.robots()[0].alarm, sim.robots()[0].alarm_ctr), (2, 6));

        // 7g.8: hit_flash clamps to 5 then decrements, only while
        // alive with hp >= 1; dead robots freeze.
        let mut sim = damage_sim();
        sim.robots_mut()[0].hit_flash = 9;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].hit_flash, 4, "clamp 5 then -1");
        sim.advance_frame();
        assert_eq!(sim.robots()[0].hit_flash, 3);
        sim.robots_mut()[0].hit_flash = 4;
        sim.robots_mut()[0].alive = false;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].hit_flash, 4, "dead robots freeze");
        sim.robots_mut()[0].alive = true;
        sim.robots_mut()[0].hp = 0;
        sim.advance_frame();
        assert_eq!(sim.robots()[0].hit_flash, 4, "hp < 1 freezes");
    }

    #[test]
    fn battery_seam_runs_the_landing_formula() {
        // 7f.8: the landing HP init is 5000 + 100*battery.
        let mut sim = damage_sim();
        assert_eq!(sim.robots()[0].hp, 5000);
        sim.set_battery(0, 7);
        assert_eq!((sim.robots()[0].battery, sim.robots()[0].hp), (7, 5700));
        sim.set_battery(0, 0);
        assert_eq!(sim.robots()[0].hp, 5000);
        // Out-of-range idx is a no-op (charter).
        sim.set_battery(9, 7);
    }

    #[test]
    fn hash_covers_the_damage_fields() {
        let mut sim = damage_sim();
        let base = sim.state_hash();
        sim.robots_mut()[0].hp -= 1;
        assert_ne!(sim.state_hash().0, base.0, "hp covered");
        sim.robots_mut()[0].armor = 40;
        assert_ne!(sim.state_hash().0, base.0, "armor covered");
        sim.robots_mut()[0].shield = 30;
        assert_ne!(sim.state_hash().0, base.0, "shield covered");
        sim.robots_mut()[0].hit_flash = 2;
        assert_ne!(sim.state_hash().0, base.0, "hit_flash covered");
        sim.robots_mut()[0].death_flag = 1;
        assert_ne!(sim.state_hash().0, base.0, "death_flag covered");
    }

    #[test]
    fn pickup_case_decodes_the_range_tables() {
        // 7h.1: table A groups → 1/3/2/4, table B → 9/7/8, four
        // CLOSED words per case. Set 0 (A=0x4e, B=0x75):
        for g in 0..4i32 {
            let b_cases = [9, 7, 8];
            for o in 0..4i32 {
                assert_eq!(
                    pickup_case(0x4E + 4 * g + o, 0),
                    Some([1, 3, 2, 4][g as usize])
                );
                if (g as usize) < b_cases.len() {
                    assert_eq!(pickup_case(0x75 + 4 * g + o, 0), Some(b_cases[g as usize]));
                }
            }
        }
        // The blocks are closed: the words just past each block and
        // every non-pickup word decode to None.
        assert_eq!(pickup_case(0x4E + 16, 0), None);
        assert_eq!(pickup_case(0x75 + 12, 0), None);
        assert_eq!(pickup_case(0x4D, 0), None);
        assert_eq!(pickup_case(0x10, 0), None);
        // Set 3 (A=0x358), set 2 (B=0x70b), set 5 (A=0xa3, B=0x4fe)
        // + a bogus set index.
        assert_eq!(pickup_case(0x358, 3), Some(1));
        assert_eq!(pickup_case(0x358 + 12, 3), Some(4));
        assert_eq!(pickup_case(0x70B + 4, 2), Some(7));
        assert_eq!(pickup_case(0xA3 + 8, 5), Some(2));
        assert_eq!(pickup_case(0x4FE + 8, 5), Some(8));
        assert_eq!(pickup_case(0x4FE + 4, 7), None, "set 7 has no table");
    }

    #[test]
    fn apply_pickup_follows_fun_0040eba0_cases() {
        // 7h.2: case 1 stages the reinforcement drop.
        let mut sim = damage_sim();
        let out = sim.apply_pickup(0, 1);
        assert_eq!((out.applied, out.effect), (true, 1));
        assert_eq!(sim.robots()[0].drop_countdown, 1000);

        // Case 2 refills the shield pool.
        let mut sim = damage_sim();
        sim.robots_mut()[0].shield = 40;
        let out = sim.apply_pickup(0, 2);
        assert_eq!((out.applied, out.effect), (true, 6));
        assert_eq!(sim.robots()[0].shield, 1000);

        // Case 3 heals +2500, clamped at 5000.
        let mut sim = damage_sim();
        sim.robots_mut()[0].hp = 1000;
        sim.apply_pickup(0, 3);
        assert_eq!(sim.robots()[0].hp, 3500);
        let out = sim.apply_pickup(0, 3);
        assert_eq!((out.applied, out.effect), (true, 7));
        assert_eq!(sim.robots()[0].hp, 5000, "0x1388 clamp");

        // Case 7 arms the booster countdown.
        let mut sim = damage_sim();
        let out = sim.apply_pickup(0, 7);
        assert_eq!((out.applied, out.effect), (true, 0xE));
        assert_eq!(sim.robots()[0].shield_boost, 200);

        // Cases 4/8/9 and a bad robot index are not this seam.
        let mut sim = damage_sim();
        assert!(!sim.apply_pickup(0, 4).applied);
        assert!(!sim.apply_pickup(0, 8).applied);
        assert!(!sim.apply_pickup(0, 9).applied);
        assert!(!sim.apply_pickup(9, 1).applied);
        assert_eq!(sim.robots()[0].hp, 5000);
    }

    #[test]
    fn apply_pickup_moves_the_hash_and_boost_feeds_the_pre_walk() {
        // The pickup writes hash-covered fields (7g/D53), and the
        // case-7 arming feeds the phase-0 pre-walk booster decay.
        let mut sim = damage_sim();
        let base = sim.state_hash();
        sim.apply_pickup(0, 7);
        assert_ne!(sim.state_hash().0, base.0, "shield_boost covered");
        // One frame of the pre-walk: shield forced 10000 while the
        // booster counts down (7g.2).
        sim.advance_frame();
        assert_eq!(sim.robots()[0].shield, SHIELD_BOOST_POOL);
        assert_eq!(sim.robots()[0].shield_boost, 199);
    }
}
