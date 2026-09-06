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
    /// The parsed .PAD records in slot order — the live run of the
    /// runtime 999×8-B slot bank @0x4e44f8 (the 0xFFFF-x terminator
    /// ends the list; the loader preserves file record order and
    /// marks every parsed slot active, 7j.16/§7j.40/1). Records are
    /// `(x, y, level)` tile triples.
    pad_slots: Vec<(i32, i32, i32)>,
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
            pad_slots: Vec::new(),
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
        // PAD marks: DAT[level][y][x] = 0xFF [0x41ded0, verified] +
        // the slot bank in file record order (the 7j.16 loader).
        let mut pad_slots = Vec::new();
        for rec in pad.chunks_exact(6) {
            let x = u16::from_le_bytes([rec[0], rec[1]]);
            let y = u16::from_le_bytes([rec[2], rec[3]]) as i32;
            let level = u16::from_le_bytes([rec[4], rec[5]]) as i32;
            if x == 0xFFFF {
                break; // 0xFFFF fill ends the record run [0x41defa]
            }
            let x = x as i32;
            pad_slots.push((x, y, level));
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
        let mut terrain = Terrain::from_parts(width, height, planes, heights)?;
        terrain.pad_slots = pad_slots;
        Some(terrain)
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

    /// The RAW DAT-volume plane byte at a tile — FUN_0041eb4c's read
    /// with NO 0xFF→1 remap (the pad-tile probe's `and eax,0xff;
    /// cmp eax,0xff` test, §7j.40/1). Out-of-range reads as `None`.
    pub fn raw_dat_byte(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        if !(0..8).contains(&z) || x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let n = (self.width * self.height) as usize;
        Some(self.dat[z as usize * n + y as usize * self.width as usize + x as usize])
    }

    /// The plane-0 (volume) byte at a LINEAR offset — the debris
    /// terrain-gate dword probe [§7j.44/3] reads four consecutive
    /// plane bytes with NO row/column upper bound (the original
    /// reads linear memory past the row end); past the plane it
    /// reads as 0. The negative-side clamps are the caller's.
    pub fn plane0_linear_byte(&self, off: usize) -> u8 {
        let n = (self.width * self.height) as usize;
        if off >= n {
            return 0;
        }
        self.dat[off]
    }

    /// FUN_00422e5e's slot scan [§7j.40/1]: the FIRST .PAD record
    /// (slot order = file record order) matching the tile and LEVEL
    /// (the robot's `z>>5`), or `None`. A hit implies the loader's
    /// 0xFF plane mark at that tile (same source records), so the
    /// probe's raw-byte 0xFF precondition holds by construction.
    pub fn pad_slot_at(&self, x: i32, y: i32, level: i32) -> Option<usize> {
        self.pad_slots
            .iter()
            .position(|&(px, py, pl)| px == x && py == y && pl == level)
    }

    /// The parsed .PAD records in slot order (the live run of the
    /// runtime slot bank; `None` past the terminator).
    pub fn pad_slot(&self, slot: usize) -> Option<(i32, i32, i32)> {
        self.pad_slots.get(slot).copied()
    }

    /// The parsed .PAD record count (the live run length).
    pub fn pad_slot_count(&self) -> usize {
        self.pad_slots.len()
    }

    /// Write one raw DAT volume byte [the FUN_0042394a /
    /// destroy-restore write side, §7j.25/2 + §7j.32/3]: the
    /// plane-major store at `dat[z·w·h + y·w + x]`. NOTE this is
    /// the RAW byte — a 0xFF write materializes a pad-style deck
    /// block for `dat_type` readers exactly like the PAD loader.
    /// Out-of-range writes are skipped (the panic-free charter;
    /// the EXW callers are in-bounds on shipped data).
    pub fn dat_write(&mut self, x: i32, y: i32, z: i32, byte: u8) {
        if !(0..8).contains(&z) || x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let n = (self.width * self.height) as usize;
        let idx = z as usize * n + y as usize * self.width as usize + x as usize;
        self.dat[idx] = byte;
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

/// One 0x1C-B escape-craft record — the shared frame
/// {active@+0, phase@+4, x@+8, y@+0xC, altitude@+0x10, img-group@+0x14,
/// dwell@+0x18} of the FUN_0041fbb1 animator machines (§7j.27/3,
/// §7j.40/6). E models the extraction DROPSHIP slot 0x4e6610 (machine
/// 2); the exit slots (machine 1) and the per-robot pod bank (machine
/// 3) stay the documented E-gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CraftRecord {
    pub active: bool,
    pub phase: i32,
    pub x: i32,
    pub y: i32,
    pub alt: i32,
    pub group: i32,
    pub dwell: i32,
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
/// The bare-floor word table C at DGROUP 0x454a90 (7 dwords, 7h.2
/// item 3): the word the pickup consume writes over the mirror
/// cell — the drawer stops drawing the pickup sprite. Indexed by
/// zone_index 0-based like the range tables (the 0x454a04..
/// 0x454ac8 family is one contiguous run of 7-dword tables at
/// exact 0x1C strides — no head slots, so `base+(cell-1)*4`;
/// §7h.5/1).
pub const PICKUP_FLOOR_WORD: [i32; 7] = [0x70B, 0x48F, 0x24C, 0x368, 0x48F, 0x39, 0x39];
/// The case-4 score/money award table [7f.6, FUN_0040eba0 case 4]:
/// `RandA()&1` picks the row (0 = score, 1 = money), `RandA()&3`
/// the amount.
pub const PICKUP_AWARDS: [[i32; 4]; 2] = [[1000, 2000, 5000, 10000], [10, 50, 100, 250]];

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
    pub(crate) hints: crate::tutorial::HintState,
    hint_scope: (u8, u8, u8),
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
    /// The tile-claim bank — the 0x2710 arena at [0x46af58] (EXD
    /// twin cell 0x119564), staged at EVERY mission load by
    /// FUN_004254e1 [§7j.63]: a whole-bank memset-0 then the stamp
    /// of the ACTIVE PREFIX of the 45-record door-rect list from
    /// the hardcoded per-(zone,mission) rect farm (crate::
    /// claim_rects). Deterministic and input-free — no RNG draws,
    /// no hashed fields; the readers are the §7j.63 gates
    /// (`stage_splash`, `platform_tile_build`, the death-blast
    /// smoke producer — that last host-seamed, §7j.24) and the
    /// canonical `static-claim-bank` TS row. Empty = unstaged
    /// (the reader gates then read 0, the pre-S0-11b behavior).
    pub(crate) claim_bank: Vec<u8>,
    /// The player TYPE word ([0x4edb90]): 0 in all SP games
    /// (GameMain 0x41c34c) — gates the alarm trip + case-4 pickup.
    pub(crate) player_type: u16,
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
    // --- The destroy family (W12-S4-prep, `destroy.rs`; NONE of
    //     these enter `state_hash` — the W6 split) ---
    /// The terrain-set/zone cell [0x4edd8c] (1..7) — indexes the
    /// rubble/water/hazard tables + gates the objective notify.
    pub(crate) zone: u32,
    /// The linear mission m [0x46ae8c] — the turret hp formula.
    pub(crate) linear: u32,
    /// The language latch [0x4eba1c] (1 = GER) — the destroy-tail
    /// GER gate. Host-staged; 0 = the modeled default.
    pub(crate) language: u32,
    /// The .BDG type table (0x4dedf2, ≤282 rows).
    pub(crate) object_types: Vec<crate::destroy::ObjectType>,
    /// The .POS instances (0x46cbf4).
    pub(crate) objects: Vec<crate::destroy::ObjectInstance>,
    /// The .TRT terrain-structure bank (0x4cccf8).
    pub(crate) structures: Vec<crate::destroy::TerrainStructure>,
    /// The object-presence word grid (0x460dfa): 0 empty,
    /// 0x7d2/0x7d3 hazard/clamp, 0x7d4 platform, n = instance
    /// n−1's footprint [§7j.12/1].
    pub(crate) object_grid: Vec<u16>,
    /// The platform STRENGTH bank (0x465daa; 0 = none) [§7j.12/2].
    pub(crate) platform_strength: Vec<u16>,
    /// The creep seed site 0x4dc5c8/cc — the last platform-damage
    /// tile (the S7 creep tick's reader).
    pub(crate) platform_site: (i32, i32),
    /// The within-zone mission number [0x4edd88] — the zone-3
    /// trigger-code sub-dispatch index (§7j.41/1). Host-staged;
    /// 1 = the modeled default.
    pub(crate) mission_no: u32,
    /// The platform epilogue family ARM (the `platforms = 1`
    /// grammar key): the original's creep tick runs EVERY frame;
    /// E arms it per scenario (D113) so the S0..S6 chains stay
    /// byte-identical.
    pub(crate) platform_family_armed: bool,
    /// The TOT-mirror plane words, 8 per tile (the 0x1E record
    /// +2·z words at 0x4796bc) [§7j.32/1].
    pub(crate) mirror_words: Vec<u16>,
    pub(crate) terrain_writes: Option<Vec<(usize, u16, u8)>>,
    /// The seen bytes, 8 per tile (the record +0x10+z bytes).
    pub(crate) mirror_seen: Vec<u8>,
    /// The +0x1B/+0x1C object-height pairs per tile [§7j.32/1].
    pub(crate) mirror_heights: Vec<(u8, u8)>,
    /// The zone-7 objective counter [0x46cce0] (§7j.32/2).
    pub(crate) objective_count: i32,
    /// The 128-slot debris ring (0x476fbc) — the T3 surface.
    pub(crate) debris: Vec<crate::destroy::DebrisRecord>,
    /// The +0x18 seq counter — the debris LRU eviction key.
    pub(crate) debris_seq: i32,
    /// Scripted moving-stack rectangles and per-tile animation bytes.
    pub(crate) elevators: Vec<crate::elevator::Elevator>,
    pub(crate) elevator_targets: Vec<u8>,
    pub(crate) elevator_bias: Vec<u8>,
    /// Active prefix of the guest boarding records at 0x4dcdb8.
    pub(crate) rides: Vec<crate::ride::Ride>,
    /// Packed guest payload/countdown records at 0x4ea828.
    pub(crate) fence_timers: [(u32, u16); 32],
    pub(crate) network_mode: u8,
    /// The 250-slot splash bank (0x4e9778) — the T3 surface.
    pub(crate) splashes: Vec<crate::destroy::SplashRecord>,
    /// The pending destroy score award ([0x4dd40c] delta).
    pub(crate) score_pending: i32,
    /// The score-strip redraw arm ([0x46ccf0] := 2).
    pub(crate) strip_arm: bool,
    /// The pending case-4 pickup SCORE award ([0x4dd40c] delta —
    /// the shell folds it beside the destroy award; §7h.5/2).
    pub(crate) pickup_score_pending: i32,
    /// The pending case-4 pickup MONEY award ([0x46ae70] delta).
    pub(crate) pickup_money_pending: i32,
    // --- The extraction family (W12-S6; NONE of it enters
    //     `state_hash` — the W6 split: the craft record is its own
    //     T3 dump row, and the sweep's robot-state effects ride the
    //     hashed robot fields) ---
    /// The extraction dropship record 0x4e6610 (FUN_0041fbb1
    /// machine 2) [§7j.40].
    pub(crate) dropship: CraftRecord,
    /// The extracted-robot counter [0x4dc680].
    pub(crate) extracted: i32,
    /// The extraction-complete flag [0x4dc67c].
    pub(crate) extraction_complete: bool,
    /// Producer tag: the pending order was armed by the PAD path
    /// (EXW's real producer — the armer's sole caller) vs the
    /// click-order seam approximation (S0..S5C). Gates the expiry
    /// deploy [§7j.40/5].
    pub(crate) order_pad_armed: bool,
    /// The surviving beacon tile words 0x4eabb4/6/8 — FUN_0041faf0
    /// clears only the flag/window pair [§7j.40/4]. Pad path only.
    pub(crate) beacon_tile_latch: Option<(i32, i32, i32)>,
    /// The surviving spread-claim words 0x4eabba (never released,
    /// §7j.20/3 — copied at deploy). Pad path only.
    pub(crate) beacon_claims_latch: [bool; 12],
    /// The objective-resolver phase cell 0x46cd00 {1 first, 2 done,
    /// 3 all-complete, 4 partial} + the light cells 0x46ccfc/0x46ccc4
    /// (§7j.32/5) — E models the zone-7 destroy-notify at-zero tail;
    /// the script-objective staging (tables 0x4557f8/0x456810) is
    /// the documented E-gap, so the cells read 0 elsewhere.
    pub(crate) objective_phase: u32,
    pub(crate) objective_blink: u32,
    pub(crate) objective_light: u32,
    // --- The critter family (W12-S8, `critter.rs`; NONE of it
    //     enters `state_hash` — the W6 split: the bank is its own
    //     T2 dump row, the critter-driven robot writes ride the
    //     hashed robot fields) ---
    /// The critter bank 0x4cff98 (0x7E stride, count 0x46cc2c).
    pub(crate) critters: Vec<crate::critter::CritterRecord>,
    /// The 0x4cec38 effect-row bank (80 × 0x20, §7j.24/5 — the
    /// critter-death staging surface; E-ONLY T3 row, never
    /// hashed).
    pub(crate) effect_rows: Vec<crate::critter::EffectRow>,
    /// The critter-family ARM (grammar `critters = 1`): the
    /// original's controller runs every mission; E arms it per
    /// scenario so the S0..S7 chains stay byte-identical (the
    /// per-frame draws on unarmed paths are the recorded stream
    /// gap, §7j.42/5).
    pub(crate) critter_family_armed: bool,
    // --- The S8 personnel/POI family (P5/G2, `poi.rs`; NONE of it
    //     enters `state_hash` — the W6 split, the critter-bank
    //     precedent: the poi-bank T2 row is the capture-plan side) ---
    /// The POI bank 0x4dabdc (0x1E stride, count 0x46cbf0, 128
    /// slots) — staged by stage_critters section 8; ticks beside the
    /// critter controller under the SAME family arm (§7j.77).
    pub(crate) pois: Vec<crate::poi::PoiRecord>,
    /// The five 0x1C exit slots 0x4e662c — the §7j.19 family's
    /// controller-read subset, host-staged (stage_poi_exit).
    pub(crate) poi_exits: [crate::poi::ExitSlot; crate::poi::EXIT_SLOTS],
    /// The escape counter [0x4eba0c].
    pub(crate) poi_escapes: i32,
    /// The panic timer [0x4eba10] (stamped 0x32 per escape; the
    /// MissionShell banner decrements it host-side).
    pub(crate) poi_panic: i32,
}

impl MissionSim {
    /// Configure original one-based zone/mission and network-mode hint gates.
    pub fn configure_hints(&mut self, zone: u8, mission: u8, network_mode: u8) {
        self.hint_scope = (zone, mission, network_mode);
        self.network_mode = network_mode;
        self.hints = Default::default();
    }

    pub fn hints(&self) -> &crate::tutorial::HintState {
        &self.hints
    }

    /// MissionShell calls the hint ticker once at its display tail.
    pub fn tick_hints(&mut self) {
        self.hints.tick();
    }

    /// Create with terrain, the angle table, and a PRNG seed.
    pub fn new(terrain: Terrain, angles: AngleTable, seed: u64) -> Self {
        MissionSim {
            hints: Default::default(),
            hint_scope: (0, 0, 0),
            robots: Vec::new(),
            order: None,
            angles,
            rng: Pcg32::new(seed, STREAM_MISSION),
            frame: 0,
            armor_pads: Vec::new(),
            claim_bank: Vec::new(),
            player_type: 0,
            commands: Vec::new(),
            weapon_bank: vec![WeaponRecord::default(); WEAPON_BANK_SLOTS],
            enemy_bank: vec![EnemyProjectile::default(); ENEMY_BANK_SLOTS],
            order_target: (0, 0, 0),
            order_flag: false,
            difficulty: 0,
            zone: 0,
            linear: 0,
            language: 0,
            object_types: Vec::new(),
            objects: Vec::new(),
            structures: Vec::new(),
            object_grid: Vec::new(),
            platform_strength: Vec::new(),
            platform_site: (0, 0),
            mission_no: 1,
            platform_family_armed: false,
            mirror_words: Vec::new(),
            terrain_writes: None,
            mirror_seen: Vec::new(),
            mirror_heights: Vec::new(),
            objective_count: 0,
            debris: vec![crate::destroy::DebrisRecord::default(); crate::destroy::DEBRIS_SLOTS],
            debris_seq: 0,
            elevators: Vec::new(),
            elevator_targets: Vec::new(),
            elevator_bias: Vec::new(),
            rides: Vec::new(),
            fence_timers: [(0, 0); 32],
            network_mode: 0,
            splashes: vec![crate::destroy::SplashRecord::default(); crate::destroy::SPLASH_SLOTS],
            score_pending: 0,
            strip_arm: false,
            pickup_score_pending: 0,
            pickup_money_pending: 0,
            dropship: CraftRecord::default(),
            extracted: 0,
            extraction_complete: false,
            order_pad_armed: false,
            beacon_tile_latch: None,
            beacon_claims_latch: [false; 12],
            objective_phase: 0,
            objective_blink: 0,
            objective_light: 0,
            critters: Vec::new(),
            effect_rows: vec![
                crate::critter::EffectRow {
                    age: 0,
                    x: 0,
                    y: 0,
                    z: 0,
                    cos: 0,
                    sin: 0,
                    ttl: 0,
                    id: 0
                };
                crate::critter::EFFECT_ROWS
            ],
            critter_family_armed: false,
            pois: Vec::new(),
            poi_exits: [crate::poi::ExitSlot::default(); crate::poi::EXIT_SLOTS],
            poi_escapes: 0,
            poi_panic: 0,
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

    /// The extraction dropship record (the S6 canonical T3 row).
    pub fn dropship(&self) -> CraftRecord {
        self.dropship
    }

    /// The extraction counters ([0x4dc680] extracted, [0x4dc67c]
    /// complete) — no watch row carries them; the canonical test
    /// asserts through this accessor.
    pub fn extraction_state(&self) -> (i32, bool) {
        (self.extracted, self.extraction_complete)
    }

    /// The surviving beacon tile words 0x4eabb4/6/8 (pad path only;
    /// `None` = never pad-armed — the pre-S6 row form).
    pub fn beacon_tile_latch(&self) -> Option<(i32, i32, i32)> {
        self.beacon_tile_latch
    }

    /// The surviving spread-claim words 0x4eabba (pad path only).
    pub fn beacon_claims_latch(&self) -> [bool; 12] {
        self.beacon_claims_latch
    }

    /// The objective-resolver cells (0x46cd00 phase, 0x46ccfc,
    /// 0x46ccc4) — §7j.32/5; no watch row (the objective-slots row
    /// needs the unmodeled script-objective staging).
    pub fn objective_cells(&self) -> (u32, u32, u32) {
        (
            self.objective_phase,
            self.objective_blink,
            self.objective_light,
        )
    }

    /// Stage the terrain-set/zone cell [0x4edd8c] (1..7) WITHOUT the
    /// destroy banks — the pad-script dispatcher keys on it
    /// (FUN_00433980's zone switch, §7j.40); the destroy staging
    /// sets the same cell with its banks.
    pub fn stage_zone_set(&mut self, zone: u32) {
        self.zone = zone;
    }

    /// The zone's extraction-pad slot set — the §7j.20/2 census (the
    /// .PAD slots whose zone scripts call the beacon armer;
    /// mechanical-parse provenance; zones 6/7 are not in the census).
    fn zone_extraction_slots(zone: u32) -> &'static [usize] {
        match zone {
            1 => &[0x10],
            2 => &[4, 5, 7, 0xE, 0x11],
            3 => &[0, 1, 6, 0xF, 0x15],
            4 => &[0, 2, 0x10, 0x15, 0x16],
            5 => &[8, 9, 0x3D],
            _ => &[],
        }
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

    /// FUN_004254e1 — the tile-claim bank INITIALIZER [§7j.63/C,
    /// verified 0x4254e1..0x425567; EXD twin 0x3657e]: memset-0 the
    /// whole 0x2710 arena, then walk the 45-record door-rect bank
    /// (staged from the hardcoded per-(zone,mission) farm —
    /// [`crate::claim_rects::RECTS`], the FUN_0042c4a0 store blocks)
    /// stopping at the first `state == 0` record, stamping
    /// `claim[line[y0+row] + x0 + col] = 1` with `line[y] = y*map_w`
    /// (the 0x4ea900 row-start table; map_w from the mission terrain,
    /// DAT_004eddec — the same dims the TOT header carries). Runs at
    /// EVERY MissionShell mission load (0x447b85, unconditional in
    /// SP — the mode==2 Head2Head filler legs are out of model).
    /// `zone_set` is the 1-based terrain set ([0x4edd8c]), `mission`
    /// the within-zone mission number ([0x4edd88]).
    ///
    /// The original has NO bounds checks (§7j.63/C); the shipped
    /// corpus is proven in-bounds by the S0-11 oracle, so the
    /// in-arena guard below is unreachable on real data and only
    /// keeps synthetic input from panicking (charter: no UB).
    pub fn stage_claim_bank(&mut self, zone_set: u32, mission: u32) {
        /// The arena allocation: 0x2710 B, the 7th per-mission bump
        /// block (0x41d9cd..0x41d9d7).
        const BANK_LEN: usize = 0x2710;
        /// The door-rect list: 45 records of stride 0x10
        /// (0x4dcae8..0x4dcdb8).
        const RECT_COUNT: usize = 45;

        let (w, _h) = self.terrain.size();
        let map_w = w.max(0) as usize;
        // The rect bank after the 0x447b7b whole-bank memset-0 +
        // FUN_0042c4a0: only the (zone, mission) case's records are
        // written; records past the last written one stay inactive.
        let mut rects = vec![[0u16; 5]; RECT_COUNT];
        let mut written = [false; RECT_COUNT];
        for &(z, m, rec, state, x0, y0, rw, rh) in crate::claim_rects::RECTS {
            if u32::from(z) != zone_set || u32::from(m) != mission {
                continue;
            }
            let r = &mut rects[rec as usize];
            r[0] = state;
            r[1] = x0;
            r[2] = y0;
            r[3] = rw;
            r[4] = rh;
            written[rec as usize] = true;
        }
        let high = written.iter().position(|x| !x).unwrap_or(RECT_COUNT);

        self.claim_bank = vec![0u8; BANK_LEN];
        for rect in &rects[..high] {
            if rect[0] == 0 {
                break; // the ACTIVE-PREFIX rule (§7j.63/C)
            }
            for row in 0..rect[4] as usize {
                for col in 0..rect[3] as usize {
                    let y = rect[2] as usize + row;
                    let x = rect[1] as usize + col;
                    let tile = y * map_w + x;
                    if tile < BANK_LEN {
                        self.claim_bank[tile] = 1;
                    }
                }
            }
        }
    }

    /// The staged claim bank (empty = unstaged — the pre-S0-11b
    /// reader-gate behavior; the canonical TS row emits whatever is
    /// staged, `run_canonical` missions always carry the full
    /// 0x2710 image).
    pub fn claim_bank(&self) -> &[u8] {
        &self.claim_bank
    }

    /// The player TYPE word [0x4edb90] (§7j.68/D159): 0 for the
    /// whole SP campaign — the census's only SP-path writer is the
    /// GameMain boot store (0x41c34c / EXD 0x2cc84), the save
    /// family never restores the cell, and the MP lobby/sync
    /// writers are gated off in SP. No setter exists by design:
    /// the constant IS the faithful SP model. Gates the alarm
    /// trip, the critter bounty, and the case-4 pickup seam.
    pub fn player_type(&self) -> u16 {
        self.player_type
    }

    /// The claim byte for a linear tile (the §7j.63 reader gates):
    /// 0 when unstaged/out-of-bank.
    pub(crate) fn claim_byte(&self, tile: usize) -> u8 {
        self.claim_bank.get(tile).copied().unwrap_or(0)
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

    /// FUN_0042223c [§7j.12/5, verified 0x42223c..0x422287]: the
    /// type-DB +0x18 byte INCREMENT writer —
    /// `byte[0x4796d4+0x1E·tile] += value; if ≥ 8 → 7`. The
    /// platform damage/build path's scorch (both add 4); the byte
    /// decays via the same advance_frame fade as the absolute
    /// writes.
    pub fn scorch_increment(&mut self, world_x: i32, world_y: i32, value: u8) {
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
        if self.armor_pads.len() <= tile {
            self.armor_pads.resize(tile + 1, 0);
        }
        let v = self.armor_pads[tile].saturating_add(value).min(7);
        self.armor_pads[tile] = v;
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
    /// 1/2/3/4/7 (+ the 8/9 presentation ids) [RE-EXW-SIM 7h.2 +
    /// 7f.6, verified asm]: case 1 stages the reinforcement
    /// (`drop_countdown = 1000`), case 2 refills the shield pool
    /// (`shield = 1000`), case 3 heals (`hp += 2500` clamped
    /// `> 5000 → 5000`), case 7 arms the shield booster
    /// (`shield_boost = 200` — the phase-0 pre-walk consumes it,
    /// 7g.2). Case 4 draws the score/money award on the SHARED
    /// mission stream (`row = RandA()&1` then `amount =
    /// PICKUP_AWARDS[row][RandA()&3]`, 7f.6) and stages it in the
    /// pending pair the shell folds (`take_pickup_awards`) — the
    /// [0x4dd40c]/[0x46ae70] cells are shell session state (the
    /// destroy-score fold precedent). Cases 8 (ammo, effect 0xC)
    /// and 9 (episode, effect 0xD) return their effect ids with NO
    /// field writes — host-seamed: the robot weapons[7] bank is the
    /// D51 host seam (W12-S3) and no shipped mission stages
    /// case-8/9 cells (§7h.5/2). No alive/state gates (the caller
    /// fires the dispatch on the tile match alone). The SFX +
    /// 0x4dc5d0 effect-row staging are presentation — the host
    /// reads the effect id off [`PickupOutcome`].
    pub fn apply_pickup(&mut self, idx: usize, case: u8) -> PickupOutcome {
        let mut out = PickupOutcome {
            applied: false,
            effect: 0,
        };
        if !matches!(case, 1..=4 | 7..=9) || self.robots.get(idx).is_none() {
            return out;
        }
        out.applied = true;
        // The 0x4dc5d0 effect-row id per case [7h.2 + 7j/1]:
        // 1→1, 2→6, 3→7, 4→1 (reuses the drop-in id), 7→0xE,
        // 8→0xC, 9→0xD.
        let effect: i32 = match case {
            1 => 1,
            2 => 6,
            3 => 7,
            4 => 1,
            7 => 0xE,
            8 => 0xC,
            _ => 0xD,
        };
        out.effect = effect;
        let r = &mut self.robots[idx];
        match case {
            1 => r.drop_countdown = PICKUP_DROP,
            2 => r.shield = PICKUP_SHIELD,
            3 => {
                r.hp += PICKUP_HEALTH;
                if r.hp > HP_MAX {
                    r.hp = HP_MAX;
                }
            }
            4 => {
                // The two shared-stream draws [7f.6, verified]:
                // row first, amount second.
                let row = (self.rng.next_u32() & 1) as usize;
                let amount = PICKUP_AWARDS[row][(self.rng.next_u32() & 3) as usize];
                if row == 0 {
                    self.pickup_score_pending += amount;
                } else {
                    self.pickup_money_pending += amount;
                }
            }
            7 => r.shield_boost = PICKUP_BOOST,
            // 8/9: presentation-only host seams (documented above).
            _ => {}
        }
        out
    }

    /// Stage the PICKUP SURFACE — the init_tiles@00407e11 seam
    /// [§7h.4/1, verified 0x407fb0..0x407ff8; §7h.5/2]: parses the
    /// mission `.TOT` volume (`u16 w + u16 h + 8 × w·h u16`
    /// plane-major, FORMATS §2) and stages EVERY plane word into
    /// the TOT mirror (the pre-cleared mirror makes the EXW
    /// nonzero-filter equivalent to a plain copy), the SEEN bytes
    /// (`seen := 1` exactly where the swept+PAD DAT volume byte is
    /// 0 — the DAT byte gates ONLY the seen flag, §7h.4/1), and
    /// the terrain-set cell `zone` ([0x4edd8c] = zone_index+1,
    /// D99). The +0x1B/+0x1C heights pair is NOT staged (its
    /// producer is the zone-7 objective family, §7j.32).
    ///
    /// CALL ORDER: after [`MissionSim::stage_destroy_family`] when
    /// both stage — the destroy staging resets the mirror banks,
    /// and the EXW original carries the pre-stamped building words
    /// inside the shipped TOT volume itself (FORMATS §2), so the
    /// TOT staging is the later, complete write. Returns false on
    /// a malformed TOT or a size/zone mismatch with the terrain
    /// (never guess). Arming note: the consume protocol
    /// (clear→move→test→fire) is ALWAYS live — with no staged
    /// words the fire reads word 0 and never triggers, so the
    /// S0..S4 no-inject invariant holds by construction (§7h.5/3).
    pub fn stage_pickup_surface(&mut self, tot: &[u8], zone: u32) -> bool {
        let (w, h) = self.terrain.size();
        if w <= 0 || h <= 0 {
            return false;
        }
        let n = (w * h) as usize;
        if tot.len() != 4 + 2 * 8 * n {
            return false;
        }
        let tw = u16::from_le_bytes([tot[0], tot[1]]) as i32;
        let th = u16::from_le_bytes([tot[2], tot[3]]) as i32;
        if tw != w || th != h {
            return false;
        }
        self.mirror_words = vec![0u16; 8 * n];
        self.mirror_seen = vec![0u8; 8 * n];
        if self.mirror_heights.len() != n {
            self.mirror_heights = vec![(0u8, 0u8); n];
        }
        for tile in 0..n {
            for z in 0..8usize {
                let o = 4 + 2 * (z * n + tile);
                let word = u16::from_le_bytes([tot[o], tot[o + 1]]);
                if word != 0 {
                    self.mirror_words[tile * 8 + z] = word;
                }
                // The seen gate: the RAW swept+PAD volume byte (the
                // engine `dat` holds exactly those bytes).
                if self.terrain.dat[z * n + tile] == 0 {
                    self.mirror_seen[tile * 8 + z] = 1;
                }
            }
        }
        self.zone = zone;
        true
    }

    /// The pending case-4 pickup awards `(score, money)` — the
    /// shell folds them into the session cells beside the destroy
    /// award ([0x4dd40c]/[0x46ae70]). Taking clears both. Zero on
    /// every no-inject path.
    pub fn take_pickup_awards(&mut self) -> (i32, i32) {
        let out = (self.pickup_score_pending, self.pickup_money_pending);
        self.pickup_score_pending = 0;
        self.pickup_money_pending = 0;
        out
    }

    /// The move-toward-target consume fire [0x40bf18..0x40bff8,
    /// §7h.4/3, verified; §7h.5/2]: the latched probe cell's mirror
    /// word selects a pickup case for the staged terrain set; on a
    /// match the tile is CONSUMED — (a) the DAT volume byte := 0
    /// (the cell leaves the collision plane and becomes EMPTY:
    /// walkable-through afterward), (b) the mirror word := the
    /// bare-floor word (table C, [`PICKUP_FLOOR_WORD`]), (c) seen
    /// := 1 — then the §7h.2 dispatch runs on the ORIGINAL word's
    /// case. The MP-only {x,y,z} staging (0x4dc6ac/b0/b4, gated
    /// [0x4edb88]==2) is SP-unreachable — unwired by design.
    fn fire_pickup(&mut self, idx: usize, z: i32, tx: i32, ty: i32) {
        let (w, _) = self.terrain.size();
        if w <= 0 || !(0..8).contains(&z) || tx < 0 || ty < 0 {
            return;
        }
        let tile = ty * w + tx;
        if tile < 0 {
            return;
        }
        let tile = tile as usize;
        let word = i32::from(self.mirror_word(tile, z as usize));
        // The staged set [0x4edd8c] = zone_index+1 → the 0-based
        // table index (§7h.5/1); an unstaged cell 0 never fires.
        let Some(set) = usize::try_from(self.zone)
            .ok()
            .and_then(|z| z.checked_sub(1))
        else {
            return;
        };
        let Some(case) = pickup_case(word, set) else {
            return;
        };
        let floor = PICKUP_FLOOR_WORD.get(set).copied().unwrap_or(word);
        // (a) the collision-plane consume.
        self.terrain.dat_write(tx, ty, z, 0);
        // (b)+(c) the mirror writes.
        if tile * 8 + (z as usize) < self.mirror_words.len() {
            self.write_mirror_cell(tile * 8 + z as usize, floor as u16, 1);
        }
        // The dispatch (the case-4 draws advance the shared stream;
        // cases 1/2/3/7 write the robot fields).
        self.apply_pickup(idx, case);
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

    /// The zone pad-trigger dispatcher's extraction subset
    /// [FUN_00433980 @0x433cfb → FUN_004247b5, §7j.19/4 + §7j.40/2]:
    /// a WALKING robot (state ∈ {1,4} with a move target — the
    /// 0x46cc30 order-word gate, both tested before the dispatch
    /// call 0x40bd16..0x40bd58) standing on a .PAD-marked tile whose
    /// slot is the zone's extraction pad arms the beacon AT THE
    /// ROBOT'S TILE. The revisit latch (0x4eb9fc/0x4eb9f4) is
    /// unmodeled: after a trigger the beacon is armed (the armer's
    /// one-at-a-time head gate) and the robot is halted state 3, so
    /// a repeat probe is inert [derived, §7j.40/1].
    fn pad_extraction_trigger(&mut self, idx: usize) {
        if self.order.is_some() {
            return; // the armer's one-beacon-at-a-time head gate
        }
        let (tx, ty, level) = {
            let r = &self.robots[idx];
            (r.pos_x >> 13, r.pos_y >> 13, r.z >> 5)
        };
        let zone = self.zone;
        if zone == 1 && (self.mission_no != 1 || self.network_mode == 2) {
            return;
        }
        let Some(slot) = self.terrain.pad_slot_at(tx, ty, level) else {
            return;
        };
        if !Self::zone_extraction_slots(zone).contains(&slot) {
            return;
        }
        // FUN_004247b5 (the shared armer body = arm_order_at_robot):
        // arm at the robot's tile, halt it state 3, spread-claim —
        // plus the producer tag + the surviving-words latch.
        if self.arm_order_at_robot(idx) {
            self.order_pad_armed = true;
            self.beacon_tile_latch = self.order.map(|o| o.tile);
        }
    }

    /// FUN_0041faf0 (the DROPSHIP DEPLOYER) [§7j.40/4, full body]:
    /// stamp the craft from the beacon words + clear the flag/window
    /// pair ONLY — the tile words survive in
    /// [`MissionSim::beacon_tile_latch`] and the claims (never
    /// released anywhere, §7j.20/3) in
    /// [`MissionSim::beacon_claims_latch`].
    fn deploy_dropship(&mut self) {
        if let Some(o) = self.order.take() {
            self.dropship = CraftRecord {
                active: true,
                phase: 1,
                x: o.tile.0 << 5,
                y: o.tile.1 << 5,
                alt: 0x200,
                group: 0,
                dwell: 0,
            };
            self.beacon_claims_latch = o.claims;
            self.order_pad_armed = false;
        }
    }

    /// FUN_0041fbb1 machine 2 (the extraction dropship) per frame
    /// [§7j.27/3 + §7j.40/6]. Runs BEFORE the MissionShell beacon
    /// block (animator 0x448012 < beacon block 0x448306): a craft
    /// deployed at this frame's beacon block first animates the next
    /// frame.
    fn dropship_tick(&mut self) {
        if !self.dropship.active {
            return;
        }
        match self.dropship.phase {
            1 => {
                // DESCEND: 2-frame flicker (groups 0/1), −0x20 while
                // ≥ 0x101, then the (alt>>2)·3 shrink; < 1 → land.
                self.dropship.group = (self.dropship.group + 1) & 1;
                if self.dropship.alt >= 0x101 {
                    self.dropship.alt -= 0x20;
                } else {
                    self.dropship.alt = (self.dropship.alt >> 2) * 3;
                }
                if self.dropship.alt < 1 {
                    self.dropship.alt = 0;
                    self.dropship.phase = 2;
                    self.dropship.dwell = 10;
                    self.extraction_sweep();
                }
            }
            2 => {
                // LANDED: RandA-jittered 0/1 altitude (a SHARED-STREAM
                // draw), flicker, dwell 10 → depart.
                self.dropship.alt = if self.rand_a() & 7 == 0 { 1 } else { 0 };
                self.dropship.group ^= 1;
                self.dropship.dwell -= 1;
                if self.dropship.dwell == 0 {
                    self.dropship.phase = 3;
                }
            }
            3 => {
                // DEPART: accelerating rise + the group-scaled left
                // drift; > 0x200 → done + the complete flag.
                self.dropship.alt += (self.dropship.alt >> 2) + 1;
                self.dropship.x -= self.dropship.group * 4;
                self.dropship.group = if self.dropship.group < 5 {
                    self.dropship.group + 1
                } else {
                    4
                };
                if self.dropship.alt > 0x200 {
                    self.dropship.active = false;
                    self.extraction_complete = true;
                }
            }
            _ => {}
        }
    }

    /// The phase-1 landing EXTRACTION SWEEP [§7j.19/1 machine 2]:
    /// every alive robot state ∈ {3,4} → state 5 + stop 1e6 (the
    /// +0x74 order target) + [0x4dc680]++. The +0x90 timer := 0x28
    /// write is outside the 31-leaf canonical pin (E-gap,
    /// §7j.40/6); the SFX is presentation.
    fn extraction_sweep(&mut self) {
        let MissionSim {
            robots, extracted, ..
        } = self;
        for r in robots.iter_mut() {
            if r.alive && (r.state == STATE_ORDERED || r.state == STATE_MOVING) {
                r.state = 5;
                r.stop_dist = 1_000_000;
                *extracted += 1;
            }
        }
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
        // The CRITTER CONTROLLER (FUN_00412f34, MissionShell
        // 0x447fe1) runs BEFORE the click dispatcher / command
        // consumer / robot phases (0x448021+). Armed per scenario
        // (`critters = 1`, D114): unarmed = the S0..S7 pinned
        // chains, byte-identical (the original's per-frame draws
        // on unarmed paths are the recorded stream gap, §7j.42/5).
        if self.critter_family_armed {
            self.critter_tick();
            // The POI controller (FUN_00412a98, MissionShell
            // 0x447fe6 — the call immediately after the critter
            // controller's 0x447fe1; §7j.77/3) rides the SAME
            // family arm: both banks stage from the one .NME load.
            self.poi_tick();
        }
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
        // projectile tick (FUN_00412010) + on ODD passes the
        // robot-hit expiry walker FUN_004197d4 (0x44805c `test
        // dl,1`): for every alive robot × every projectile of type
        // {0x65, 0x67, 0x68}, the 0x10/0x10/0x20 px box at the
        // record's position → the disburser + FUN_0040e230(robot,
        // FUN_00419aff(type), owner −1) [§7j.42; the walker draws
        // nothing on its own]. With an empty bank it is a structural
        // no-op (the pre-S8 behavior), so it runs ungated.
        for i in 0..4 {
            self.weapon_tick(i);
            self.enemy_tick();
            if i & 1 == 1 {
                self.critter_projectile_walker();
            }
        }
        // The DEBRIS TICK (FUN_00420549, the MissionShell epilogue
        // call 0x448076 — §7j.7/7, §7j.44): runs AFTER the robot
        // phases + the enemy passes and BEFORE the armor-pad fade
        // (FUN_00424051). The delays/anim/free lifecycle never
        // hashes (the debris ring is the T3 surface); the physics
        // pass mutates HASHED state only through the robot/critter
        // damage lanes, which need a staged physics-class debris —
        // none exists on the unarmed paths (the staging-key
        // discipline: S0..S6/S8 stage none, S4/S7 stage the
        // destroy-tail kinds).
        self.ride_tick();
        self.debris_tick();
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
        // The escape-craft animator (FUN_0041fbb1 machine 2, the
        // extraction dropship) runs BEFORE the MissionShell beacon
        // block (0x448012 < 0x448306) — a craft deployed below first
        // animates the NEXT frame [§7j.40/3].
        self.dropship_tick();
        self.fence_tick();
        // The platform CREEP tick (FUN_00422a9c, the epilogue call
        // 0x44808a — §7j.41/4): the ORIGINAL runs it every frame,
        // unconditionally drawing the 1/32 gate RandA; E arms it
        // with the platform family (grammar `platforms = 1`) so
        // the S0..S6 chains stay byte-identical — the per-frame
        // gate draw on unarmed paths is the recorded E-gap (D113).
        // Placement: after the dropship animator draw and before
        // the beacon block, matching the original's relative draw
        // order (0x448012 < 0x44808a < 0x448306).
        if self.platform_family_armed {
            self.platform_creep_tick();
        }
        self.elevator_tick();
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
                // The MissionShell beacon block 0x448306..0x448381:
                // the sole expiry consumer is FUN_0041faf0 — it
                // deploys the dropship AND clears the beacon
                // flag/window. EXW's real beacon producer is the pad
                // step-on (the armer's sole caller) — the click-order
                // seam (the S0..S5C approximation) never reaches the
                // deploy in the original, so E gates the deploy on
                // the pad tag [§7j.40/5]; a beacon expiring while a
                // craft is in flight stays ARMED at window 0 (the
                // block's dropship-active gate skips everything).
                if self.order_pad_armed {
                    if !self.dropship.active {
                        self.deploy_dropship();
                    }
                } else {
                    self.clear_order();
                }
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
                // The tile-0x62 TRAP lane (FUN_0040fe93, §7j.25/7 —
                // the destroy-family unit): armor-first with the
                // exact intra-walk interleaving unpinned
                // [§7j.38/6 hypothesis; corpus-never on ZONEA].
                // Pure no-op until destructibles are staged (the
                // grid word stays 0 — the no-inject invariant).
                self.robot_trap_lane(idx);
            }
            // The zone pad-trigger extraction subset (§7j.40/2): the
            // dispatcher call sits between the armor pass and the
            // move math, dual-gated on the walking states + a move
            // target (0x40bd16..0x40bd58). The armer's halt takes
            // effect before this robot's move the same phase.
            {
                let r = &self.robots[idx];
                if matches!(r.state, 1 | STATE_MOVING) && r.target.is_some() {
                    let r = &self.robots[idx];
                    if let Some(slot) =
                        self.terrain
                            .pad_slot_at(r.pos_x >> 13, r.pos_y >> 13, r.z >> 5)
                    {
                        let (zone, mission, mode) = self.hint_scope;
                        self.hints.probe(zone, mission, mode, slot);
                    }
                    self.pad_elevator_trigger(idx);
                    self.pad_ride_trigger(idx);
                    if self.robots[idx].state != 2 {
                        self.pad_extraction_trigger(idx);
                    }
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
                // The probe-latch consume protocol [0x40bef2..
                // 0x40bf0b, §7h.4/3, verified]: clear the latch
                // (the −1 sentinel) immediately BEFORE the move,
                // then the ≠ −1 test immediately AFTER — the move's
                // nine probes may latch a type-3 cell on the way
                // past (no standing-on required, ±11/12 Q5 reach).
                // UNCONDITIONAL like EXW: with no staged mirror
                // words the fire reads word 0 and never triggers
                // (the S0..S4 no-inject invariant, §7h.5/3).
                self.terrain.last_trigger = None;
                self.robot_move(idx, vx, vy, angle);
                if let Some((lz, ltx, lty)) = self.terrain.last_trigger {
                    self.fire_pickup(idx, lz, ltx, lty);
                }
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
    pub(crate) fn robot_move(&mut self, idx: usize, dx: i32, dy: i32, angle: u16) {
        if self.robots[idx].state == STATE_ORDERED || self.robots[idx].state == 5 {
            return;
        }
        // EXW 0x40c570..0x40c58f: an active hint writes state zero
        // and skips movement, retaining the target for a later command.
        if self.hints.active().is_some() {
            self.robots[idx].state = STATE_IDLE;
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
    pub(crate) fn move_possible(&mut self, idx: usize, wx: i32, wy: i32) -> bool {
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
    #[test]
    fn boot_camp_ride_boards_then_arrives_on_tenth_frame() {
        let mut sim = MissionSim::new(
            flat_terrain(100, 100, 1, zero_heights(8)),
            sintable_like(),
            1,
        );
        sim.zone = 1;
        sim.spawn_robot((5, 61, 1));
        sim.robots[0].z = 31;
        sim.terrain.pad_slots = vec![(99, 99, 0); 16];
        sim.terrain.pad_slots[0] = (5, 61, 0);
        sim.stage_rides();
        assert_eq!(sim.rides().len(), 7);
        sim.platform_strength = vec![0; 10000];
        sim.object_grid = vec![0; 10000];
        sim.mirror_words = vec![0; 80000];
        sim.mirror_seen = vec![0; 80000];
        let destination = 57 * 100 + 8;
        sim.platform_strength[destination] = 199;
        sim.object_grid[destination] = 0x7d4;
        sim.mirror_words[destination * 8 + 1] = 0x25d;
        sim.mirror_words[destination * 8 + 2] = 0x25d;
        sim.observe_terrain_writes();
        sim.stage_command_record(CommandRecord {
            marker: 0,
            id: 0,
            spot: 0,
            flags: 1,
            x: 6 * 32,
            y: 61 * 32,
            z: 0,
        });
        sim.advance_frame();
        assert_eq!(sim.rides()[0].countdown, 9);
        assert_eq!(sim.robots[0].state, 2);
        assert_eq!(sim.robots[0].pos_x, (5 << 13) + 0x1000);
        assert!(sim.robots[0].target.is_none());
        sim.pad_ride_trigger(0);
        assert_eq!(sim.rides()[0].countdown, 9, "busy ride cannot restart");
        for _ in 0..8 {
            sim.advance_frame();
        }
        assert_eq!(sim.robots[0].state, 2);
        sim.advance_frame();
        assert_eq!(
            (sim.robots[0].pos_x, sim.robots[0].pos_y),
            (8 << 13, 57 << 13)
        );
        assert_eq!(sim.robots[0].state, 0);
        assert!(sim.rides()[0].rider.is_none());
        assert_eq!(sim.robots[0].probe_z, [sim.robots[0].z as u16; 8]);
        assert_eq!(sim.platform_strength[destination], 0);
        assert_eq!(sim.object_grid[destination], 0);
        assert_eq!(sim.take_terrain_writes(), vec![(destination * 8 + 1, 0, 0)]);
        assert_eq!(sim.mirror_words[destination * 8 + 2], 0x25d);
        sim.stage_command_record(CommandRecord {
            marker: 0,
            id: 0,
            spot: 0,
            flags: 1,
            x: 9 * 32,
            y: 57 * 32,
            z: 0,
        });
        sim.advance_frame();
        assert_ne!(sim.robots[0].pos_x, 8 << 13, "arrival accepts movement");
    }

    #[test]
    fn boot_camp_exit_pad_keeps_mission_and_network_gates() {
        for (mission, mode, expected) in [(1, 0, true), (2, 0, false), (1, 2, false)] {
            let mut sim = damage_sim();
            sim.zone = 1;
            sim.mission_no = mission;
            sim.network_mode = mode;
            let r = &sim.robots[0];
            let marker = (r.pos_x >> 13, r.pos_y >> 13, r.z >> 5);
            sim.terrain.pad_slots = vec![(7, 7, 0); 16];
            sim.terrain.pad_slots.push(marker);
            sim.pad_extraction_trigger(0);
            assert_eq!(
                sim.order_pad_armed, expected,
                "mission={mission}, mode={mode}"
            );
        }
    }

    #[test]
    fn hint_pad_halts_same_phase_and_movement_command_dismisses_after_grace() {
        let mut sim = damage_sim();
        sim.configure_hints(1, 1, 0);
        sim.zone = 1;
        let r = &sim.robots[0];
        let pos = (r.pos_x, r.pos_y);
        sim.terrain.pad_slots = vec![(7, 7, 0); 18];
        sim.terrain
            .pad_slots
            .push((r.pos_x >> 13, r.pos_y >> 13, r.z >> 5));
        let command = CommandRecord {
            marker: 0,
            id: 0,
            spot: 0,
            flags: 1,
            x: 6 * 32,
            y: 3 * 32,
            z: 0,
        };
        sim.stage_command_record(command);
        sim.advance_frame();
        assert_eq!(sim.hints().active(), Some(0));
        assert_eq!((sim.robots[0].pos_x, sim.robots[0].pos_y), pos);
        assert_eq!(sim.robots[0].state, STATE_IDLE);
        for _ in 0..8 {
            sim.tick_hints();
        }
        sim.stage_command_record(command);
        sim.advance_frame();
        assert_eq!(sim.hints().active(), Some(0));
        assert_eq!((sim.robots[0].pos_x, sim.robots[0].pos_y), pos);
        sim.tick_hints();
        sim.stage_command_record(command);
        sim.advance_frame();
        assert_eq!(sim.hints().active(), None);
        assert_ne!((sim.robots[0].pos_x, sim.robots[0].pos_y), pos);
    }

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

    /// A synthetic `.TOT` volume (`u16 w + u16 h + 8 × w·h u16
    /// plane-major, FORMATS §2) with `(tile, z, word)` overrides.
    fn tot_volume(w: i32, h: i32, words: &[(usize, usize, u16)]) -> Vec<u8> {
        let n = (w * h) as usize;
        let mut tot = vec![0u8; 4 + 2 * 8 * n];
        tot[0..2].copy_from_slice(&(w as u16).to_le_bytes());
        tot[2..4].copy_from_slice(&(h as u16).to_le_bytes());
        for &(tile, z, word) in words {
            let o = 4 + 2 * (z * n + tile);
            tot[o..o + 2].copy_from_slice(&word.to_le_bytes());
        }
        tot
    }

    /// The pickup walk fixture [§7h.4/§7h.5]: flat 16×16 floor type 5
    /// (height 3, slot 4), the type-3 pickup cells at (5,8) and (4,8)
    /// with heights[2] = 3 so they floor exactly like the deck (the
    /// walk passes THROUGH the probe reach, no standing-on). `a`
    /// spawns at (2,8) (the clicked armer), `b` at (6,8) (the walker
    /// west through both cells toward spread slot (3,8)).
    fn pickup_walk_sim() -> MissionSim {
        let mut hs = zero_heights(8);
        hs[4] = [3u8; 1024];
        hs[2] = [3u8; 1024]; // type-3 height slot
        let w = 16;
        let h = 16;
        let mut dat = vec![0u8; (8 * w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                dat[(y * w + x) as usize] = if (x == 4 || x == 5) && y == 8 {
                    3u8
                } else {
                    5u8
                };
            }
        }
        MissionSim::new(
            Terrain::from_parts(w, h, dat, hs).unwrap(),
            sintable_like(),
            21,
        )
    }

    #[test]
    fn stage_pickup_surface_follows_init_tiles() {
        // §7h.4/1: EVERY nonzero TOT word stages (the DAT byte gates
        // ONLY the seen flag); the zone cell writes [0x4edd8c].
        let mut sim = pickup_walk_sim();
        let a = sim.spawn_robot((2, 8, 0));
        let _ = a;
        // (0,0) plane 0 byte 0 (empty cell) + a word: the word MUST
        // stage and seen MUST be 1 — the §7h.4/1 correction pin.
        sim.terrain.dat_write(0, 0, 0, 0);
        let tot = tot_volume(16, 16, &[(8 * 16 + 4, 0, 0x52), (0, 0, 0x131)]);
        assert!(sim.stage_pickup_surface(&tot, 1));
        assert_eq!(sim.mirror_word(8 * 16 + 4, 0), 0x52);
        assert_eq!(sim.mirror_word(0, 0), 0x131, "word stages at a DAT-0 cell");
        assert_eq!(sim.mirror_seen(0, 0), 1, "seen at the DAT-0 cell");
        assert_eq!(sim.mirror_seen(8 * 16 + 4, 0), 0, "no seen under type 3");
        // The rest of the deck (type 5, DAT≠0): words stay 0 (not in
        // the TOT), seen 0.
        assert_eq!(sim.mirror_word(8 * 16 + 6, 0), 0);
        assert_eq!(sim.mirror_seen(8 * 16 + 6, 0), 0);
        // Malformed inputs are refused, never guessed.
        assert!(!sim.stage_pickup_surface(&tot[..tot.len() - 2], 1));
        let bad_dims = tot_volume(8, 8, &[]);
        assert!(!sim.stage_pickup_surface(&bad_dims, 1));
    }

    #[test]
    fn walk_fires_pickups_through_the_probe_reach() {
        // The full consume protocol [§7h.4/3, 0x40bef2..0x40bff8]:
        // the walker's probes latch the type-3 cells walking PAST
        // them; each fire consumes the cell (DAT 0 / floor word /
        // seen 1) and dispatches on the original word — case 4 at
        // (5,8) (word 0x5A = A+12, zone-1 tables) stages the award,
        // case 3 at (4,8) (word 0x52 = A+4) heals +2500.
        let mut sim = pickup_walk_sim();
        let tot = tot_volume(16, 16, &[(8 * 16 + 5, 0, 0x5A), (8 * 16 + 4, 0, 0x52)]);
        assert!(sim.stage_pickup_surface(&tot, 1));
        let a = sim.spawn_robot((2, 8, 0));
        let b = sim.spawn_robot((6, 8, 0));
        sim.robots_mut()[b].hp = 1000;
        assert!(sim.arm_order_at_robot(a));
        let mut frames = 0;
        while frames < 200 && sim.robots()[b].state != STATE_ORDERED {
            sim.advance_frame();
            frames += 1;
        }
        assert!(frames < 200, "the walk terminates through the pickups");
        // Both cells consumed.
        assert_eq!(sim.terrain.dat_type(5, 8, 0), 0, "case-4 cell consumed");
        assert_eq!(sim.terrain.dat_type(4, 8, 0), 0, "case-3 cell consumed");
        assert_eq!(sim.mirror_word(8 * 16 + 5, 0), PICKUP_FLOOR_WORD[0] as u16);
        assert_eq!(sim.mirror_word(8 * 16 + 4, 0), PICKUP_FLOOR_WORD[0] as u16);
        assert_eq!(sim.mirror_seen(8 * 16 + 5, 0), 1);
        assert_eq!(sim.mirror_seen(8 * 16 + 4, 0), 1);
        // The case-3 body ran (hp 1000 + 2500) and the case-4 award
        // staged exactly one table value on one side of the pair.
        assert_eq!(sim.robots()[b].hp, 3500);
        let (s, m) = sim.take_pickup_awards();
        assert!(
            (PICKUP_AWARDS[0].contains(&s) && m == 0) || (PICKUP_AWARDS[1].contains(&m) && s == 0),
            "one row, one table amount: ({s}, {m})"
        );
        assert_eq!(sim.take_pickup_awards(), (0, 0));
        // The armer (state 3, never in the move-toward block) fired
        // nothing — its record is untouched.
        assert_eq!(sim.robots()[a].hp, 5000);
    }

    #[test]
    fn inert_words_latch_but_never_fire() {
        // The ZONEA corpus shape [§7h.4/5]: a type-3 cell whose word
        // (0x81 = a set-2 case-4 shape) is OUT of the staged set's
        // ranges latches on the walk past but never fires — the
        // corpus-dead invariant, synthetically.
        let mut sim = pickup_walk_sim();
        let tot = tot_volume(16, 16, &[(8 * 16 + 5, 0, 0x81), (8 * 16 + 4, 0, 0x81)]);
        assert!(sim.stage_pickup_surface(&tot, 1));
        let a = sim.spawn_robot((2, 8, 0));
        let b = sim.spawn_robot((6, 8, 0));
        assert!(sim.arm_order_at_robot(a));
        let mut frames = 0;
        while frames < 200 && sim.robots()[b].state != STATE_ORDERED {
            sim.advance_frame();
            frames += 1;
        }
        assert!(frames < 200, "the walk still terminates");
        assert_eq!(sim.terrain.dat_type(5, 8, 0), 3, "cell untouched");
        assert_eq!(sim.terrain.dat_type(4, 8, 0), 3, "cell untouched");
        assert_eq!(sim.mirror_word(8 * 16 + 5, 0), 0x81);
        assert_eq!(sim.mirror_seen(8 * 16 + 5, 0), 0);
        assert_eq!(sim.take_pickup_awards(), (0, 0));
        assert_eq!(sim.robots()[b].hp, 5000);
    }

    #[test]
    fn move_sub_tick_clears_the_latch() {
        // 0x40bef2: the clear runs at EVERY move-toward-target
        // sub-tick — a stale latch (set by an earlier probe family,
        // e.g. the wander/drift robot_move site that has no clear)
        // does not survive the next sub-tick.
        let mut hs = zero_heights(8);
        hs[4] = [3u8; 1024];
        let mut sim = MissionSim::new(flat_terrain(16, 16, 5, hs), sintable_like(), 5);
        let a = sim.spawn_robot((2, 8, 0));
        let b = sim.spawn_robot((6, 8, 0));
        assert!(sim.arm_order_at_robot(a));
        // Mid-walk: b is state 4 with a target.
        sim.advance_frame();
        assert_eq!(sim.robots()[b].state, STATE_MOVING);
        sim.terrain.last_trigger = Some((0, 1, 1)); // stale latch
        sim.advance_frame();
        assert_eq!(
            sim.terrain.last_trigger, None,
            "the move sub-tick cleared the stale latch"
        );
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

        // Case 4 (7f.6): applied with the drop-in effect id 1; the
        // two shared-stream draws stage a pending award of exactly
        // one table value, one side of the pair (§7h.5/2).
        let mut sim = damage_sim();
        let out = sim.apply_pickup(0, 4);
        assert_eq!((out.applied, out.effect), (true, 1));
        let (s, m) = sim.take_pickup_awards();
        let sums: (bool, bool) = (
            PICKUP_AWARDS[0].contains(&s) && m == 0,
            PICKUP_AWARDS[1].contains(&m) && s == 0,
        );
        assert!(sums.0 || sums.1, "one row, one table amount: ({s}, {m})");
        assert_eq!(sim.take_pickup_awards(), (0, 0), "taking clears");
        assert_eq!(sim.robots()[0].hp, 5000, "case 4 writes no robot field");

        // Cases 8/9: applied (the dispatch ran) with their
        // presentation ids, but NO field writes — host-seamed
        // bodies (§7h.5/2).
        let mut sim = damage_sim();
        assert_eq!(
            (sim.apply_pickup(0, 8).applied, sim.robots()[0].shield_boost),
            (true, 0)
        );
        assert_eq!(
            (
                sim.apply_pickup(0, 9).applied,
                sim.robots()[0].drop_countdown
            ),
            (true, 0)
        );
        assert_eq!(sim.apply_pickup(0, 8).effect, 0xC);
        assert_eq!(sim.apply_pickup(0, 9).effect, 0xD);
        assert_eq!(sim.take_pickup_awards(), (0, 0));
        // A bad robot index is never applied.
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
