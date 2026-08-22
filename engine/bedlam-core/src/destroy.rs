//! The destructible-object / terrain-structure destroy family
//! (P4.2/W12-S4-prep, DESIGN-DIFFHARNESS §7 S4 row; RE-EXW-SIM
//! §7j.10/§7j.11/§7j.12/§7j.13/§7j.14/§7j.25/§7j.32 + the §7j.38
//! RNG census + the §7j.39 landing addendum — every [verified]
//! tag below cites those sections).
//!
//! Scope: the E-side model of the mission-load STAGING (the .BDG
//! type table + the .POS instance list + the .TRT terrain-structure
//! bank, host-seamed per the D51 pattern), the two impact RESOLVERS
//! (FUN_0041a894 objects, FUN_0041bc1c terrain structures), the
//! destroy TAIL (objective notify → terrain RESTORE → the
//! five-effect debris loop → score award → the four perimeter CHAIN
//! walks), the widened 128-slot debris ring (the 7j.11 20-kind
//! table), the splash STAGER (gates + the 250×0xA bank; the 7j.10
//! tick body is the S4 pairing E-gap), the platform ENTRY
//! (FUN_00422693 destroy/weaken over host-staged strength words),
//! and the tile-0x62 trap lane (FUN_0040fe93).
//!
//! W6 SPLIT: NONE of this enters `MissionSim::state_hash`. The
//! watched surfaces here (mirror words/seen, the DAT volume bytes
//! via the restore, the debris ring, the splash bank, the score
//! award) are T1/T2/T3 dump rows, each with its own blob — the hash
//! stays the 31-leaf robot model.
//!
//! NO-INJECT INVARIANT: with nothing staged (every default-empty
//! bank) the resolvers pass through, the stagers no-op on their
//! bounds/gates, the trap lane's grid word is 0, and
//! `advance_frame` is byte-identical to the pre-S4 engine — pinned
//! by the S0/S1/S2 canonical chains [asserted by tests]. S3 re-pins
//! ONCE at this landing: its artillery volleys reach the burst
//! window, and the per-pair FUN_004244a1 k6 gate + the k11 50% gate
//! + the stager's k11 SFX-gate each draw the SHARED stream whether
//!   or not destructibles are staged [§7j.39/9 — the pinned E-side
//!   chain moves before any O1 S3 capture exists].
//!
//! RNG DISCIPLINE: the destroy family consumes the shared mission
//! RandA stream in the ORIGINAL's exact order/count (§7j.38): the
//! five-effect loop draws per its case table (8/8/8/8/8/0/0/72/9
//! RandA for sel 1..9), the chain walks draw once per QUALIFYING
//! candidate, the platform k7 destroy draws 2×5, the trap k12
//! debris draws 3×5, the kind-11 stager body draws 1 (the SFX gate
//! — the sound is T4, the draw is real), the script blast draws 1
//! for the k6 1-in-8 gate (+1 for its delay when it passes), and
//! the artillery pair's k11 gate draws 1. RandB draws (the DEADMAN
//! SFX pick, the FUN_0041a225 effects-bank jitter) are T3/T4 —
//! unmodeled, never drawn.
//!
//! E-gaps (documented — S4/S7 findings name them): the splash TICK
//! body (the 7j.10 odd-frame fall/absorb + the per-tick 5-draw
//! scorch re-roll + the water stamps — §7j.38/6), the platform
//! SPREAD ring FUN_00422832 + the CREEP tick (S7 — the creep SEED
//! site is staged), the trigger producers FUN_00422e0a/FUN_00422600
//! (bridge builds — S7-routed no-ops), the FUN_0041a225 effects
//! bank (0x4cf638, RandB-fed), the critter area-damage leg of the
//! script blast (no critter bank in E), the debris PHYSICS pass
//! FUN_0040de9c, the at-zero extraction-arm tail of the objective
//! notify (the S6 seam), and every SFX family (T4).

use crate::mission::{MissionSim, DEBRIS_SCORCH_RING};

/// The object type-table row cap (0x11A = 282 records, loader cap).
pub const OBJECT_TYPE_SLOTS: usize = 282;
/// The .POS instance-slot count (2000 × 16 B).
pub const OBJECT_INSTANCE_SLOTS: usize = 2000;
/// The debris-ring slot count (0x1800 B / 0x30 at 0x476fbc).
pub const DEBRIS_SLOTS: usize = 128;
/// The splash-bank slot count (250 × 0xA at 0x4e9778).
pub const SPLASH_SLOTS: usize = 250;
/// The terrain-structure bank cap (the 0x20-stride 0x4cccf8 array).
pub const STRUCTURE_SLOTS: usize = 250;

/// The per-zone RUBBLE word table at DGROUP 0x454a04 [§7j.38/3,
/// DGROUP bytes]: the FUN_0041bc1c death-stamp source, indexed by
/// the zone cell [0x4edd8c] (1..7; slot 0 is the 0xFFFFFFFF unused
/// head).
pub const RUBBLE_WORD: [i32; 8] = [
    -1, // unused head
    0x20, 0x20, 0x348, // ZONEC restores word 0x348
    0x20, 0x20, 0x20, 0x20,
];

/// The per-zone water-range BASE table at DGROUP 0x454ae4
/// [§7j.38/4 — recorded for the platform entry + the later splash
/// tick pairing; a z-word is "water" iff it lies in
/// `[WATER_RANGE[zone], WATER_RANGE[zone]+0xE)`].
pub const WATER_RANGE: [i32; 8] = [
    0x25D, // unused head (the table's own slot 0)
    0x25D, 0xBD, 0x3BD, 0x5E8, 0xBD, 0xEC, 0xC3,
];

/// The 0x7d2 hazard-word zone bases [§7j.12/6, DGROUP 0x454a20
/// bytes]: a tile whose z-word lies in `[base, base+4)` stamps
/// object-grid word 0x7d2 at mission load (slot 0 unused).
pub const HAZARD_7D2: [i32; 8] = [0, 0x20, 0x49, 0x49, 0x34E, 0x49, 0x77, 0x77];
/// The 0x7d3 phase-clamp zone bases [§7j.12/6, DGROUP 0x454a3c]:
/// z-word in `[base, base+4)` → grid word 0x7d3.
pub const HAZARD_7D3: [i32; 8] = [0, 0x49, 0x77, 0x77, 0x49, 0x4E, 0x4E, 0x349];

/// One 8-B effect entry of the .BDG type row [§7j.25]: selector
/// u16 + x/y/z u16 TILE offsets staged relative to the destroyed
/// instance's origin. ON DISK at record +0x12+8m (head 0x3A); in
/// the 78-B in-memory arena row at +0x16+8m (the load-computed
/// count word@+0x12 shifts everything past it). Selector 1..9
/// dispatch the destroy-tail debris/effect cases; 0 or >9 skips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjectEffectEntry {
    pub selector: u16,
    pub dx: u16,
    pub dy: u16,
    pub dz: u16,
}

/// One .BDG type-table row (78 B in the original arena at
/// 0x4dedf2) [§7j.25 grammar, FORMATS §16]. Empty disk rows
/// (control ≠ 1) become all-zero rows; the four on-disk template
/// banks are 2·W·H·D u16 words each in the DISK order
/// current-TOT(+0x3E), under-TOT(+0x46), current-DAT(+0x42),
/// under-DAT(+0x4A) [§7j.32/1]. The +0x3E/+0x42 CURRENT pair has
/// zero runtime readers (editor payload, §7j.32/2) — kept loaded
/// as the faithful option.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectType {
    pub w: u16,
    pub h: u16,
    pub d: u16,
    pub hp: i32,
    pub chain: u16,
    /// Objective/score code @+0xE (0xb = the score-10 type).
    pub kind: i32,
    pub count: u16,
    pub effects: [ObjectEffectEntry; 5],
    pub bank_current_tot: Vec<u16>,
    pub bank_under_tot: Vec<u16>,
    pub bank_current_dat: Vec<u16>,
    pub bank_under_dat: Vec<u16>,
}

/// The parsed .BDG destructible-object library (≤282 rows).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectTypeTable {
    pub rows: Vec<ObjectType>,
}

impl ObjectTypeTable {
    /// Parse the whole .BDG file [FORMATS §16, §7j.25/8 — verified
    /// 37/37 corpus files to the last byte, EXACTLY 282 records
    /// per file (10434 total, 7907 active)]. Records start at
    /// offset 0 (NO header): control u16; ≠1 → a 2-B empty row;
    /// ==1 → W/H/D u16×3, hp i32, chain u16, type i32, the 5×8-B
    /// effect entries at +0x12, then the four template banks at
    /// +0x3A (head 0x3A — the in-memory 78-B row's count@+0x12 is
    /// LOAD-COMPUTED, NOT on disk). `None` = truncated/desynced
    /// input. `ObjectType::count` stays 0 (host-computed staging
    /// if ever needed).
    pub fn from_bdg_bytes(bytes: &[u8]) -> Option<Self> {
        let mut rows = Vec::with_capacity(OBJECT_TYPE_SLOTS);
        let mut pos = 0usize;
        while rows.len() < OBJECT_TYPE_SLOTS {
            if pos == bytes.len() {
                break;
            }
            if pos + 2 > bytes.len() {
                return None;
            }
            let control = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
            if control != 1 {
                rows.push(ObjectType::default());
                pos += 2;
                continue;
            }
            if pos + 0x3A > bytes.len() {
                return None;
            }
            let rd16 = |o: usize| u16::from_le_bytes([bytes[pos + o], bytes[pos + o + 1]]);
            let rd32 =
                |o: usize| i32::from_le_bytes(bytes[pos + o..pos + o + 4].try_into().unwrap());
            let w = rd16(0x02);
            let h = rd16(0x04);
            let d = rd16(0x06);
            let hp = rd32(0x08);
            let chain = rd16(0x0C);
            let kind = rd32(0x0E);
            let mut effects = [ObjectEffectEntry::default(); 5];
            for (m, e) in effects.iter_mut().enumerate() {
                let o = 0x12 + 8 * m;
                *e = ObjectEffectEntry {
                    selector: rd16(o),
                    dx: rd16(o + 2),
                    dy: rd16(o + 4),
                    dz: rd16(o + 6),
                };
            }
            // Four template banks, 2·W·H·D bytes each at +0x3A:
            // disk order current-TOT, under-TOT, current-DAT,
            // under-DAT [§7j.32/1 — interleaved vs the in-memory
            // slot order].
            let cells = (w as usize) * (h as usize) * (d as usize);
            let bank_bytes = cells * 2;
            let total = 0x3A + 4 * bank_bytes;
            if pos + total > bytes.len() {
                return None;
            }
            let bank = |slot: usize| -> Vec<u16> {
                let start = pos + 0x3A + slot * bank_bytes;
                (0..cells)
                    .map(|i| u16::from_le_bytes([bytes[start + 2 * i], bytes[start + 2 * i + 1]]))
                    .collect()
            };
            rows.push(ObjectType {
                w,
                h,
                d,
                hp,
                chain,
                kind,
                count: 0,
                effects,
                bank_current_tot: bank(0),
                bank_under_tot: bank(1),
                bank_current_dat: bank(2),
                bank_under_dat: bank(3),
            });
            pos += total;
        }
        if pos != bytes.len() {
            // Corpus files consume exactly; trailing bytes mean the
            // grammar desynced — fail loud rather than guess.
            return None;
        }
        Some(ObjectTypeTable { rows })
    }
}

/// One destructible-object INSTANCE (the 0x14-stride 0x46cbf4
/// array, staged from .POS at mission load) [§7j.12/1, FORMATS
/// §12]. `hp` is initialized from the type row's hp by the load
/// re-stamp pass; −1 = immune (never dies); `destroyed` = the id
/// dword's 0x40 flag byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectInstance {
    /// Footprint origin x (tiles).
    pub x: i32,
    /// Footprint origin y (tiles).
    pub y: i32,
    /// Base z level (the .POS word 2, 0..5; the restore runs
    /// [z, z+D) [§7j.32/3]).
    pub z: i32,
    /// The type-table row index (the .POS word 3).
    pub id: i32,
    pub destroyed: bool,
    pub hp: i32,
}

/// One terrain-structure record (the 0x20-stride 0x4cccf8 array,
/// staged from .TRT) [§7j.14/1, FORMATS §14]: a shooting SENTRY
/// TURRET. Only the resolver-facing fields are modeled; the
/// animator state words are the turret-AI E-gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainStructure {
    pub active: bool,
    pub hp: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// The .TRT staging parse [FORMATS §14]: u16 count + count × 12 B
/// `(x, y, z)` u32 records. `linear` is the [0x46ae8c] linear
/// mission m — hp = 250 + (250·m)/27 [§14, verified].
pub fn parse_trt(bytes: &[u8], linear: u32) -> Option<Vec<TerrainStructure>> {
    if bytes.len() < 2 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    if bytes.len() != 2 + 12 * count {
        return None;
    }
    let hp = 250 + (250 * linear as i32) / 27;
    let mut out = Vec::with_capacity(count.min(STRUCTURE_SLOTS));
    for i in 0..count.min(STRUCTURE_SLOTS) {
        let o = 2 + 12 * i;
        let rd = |k: usize| i32::from_le_bytes(bytes[o + k..o + k + 4].try_into().unwrap());
        out.push(TerrainStructure {
            active: true,
            hp,
            x: rd(0),
            y: rd(4),
            z: rd(8),
        });
    }
    Some(out)
}

/// One widened debris-ring record (the 0x30-B 0x476fbc stride,
/// §7j.5/§7j.11): the draw/tick gates + the per-kind physics
/// config. The +0x10/+0x14 init words are staged per kind and
/// consumed by the physics pass (E-gap) — carried for the T3 row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DebrisRecord {
    pub active: bool,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// +0x10 init word (0x40/0x20 per kind).
    pub init_a: i32,
    /// +0x14 init word.
    pub init_b: i32,
    /// +0x18 the sequence counter (the LRU eviction key,
    /// §7j.39/6).
    pub seq: i32,
    /// +0x1C the kind argument verbatim (1..20; the draw layer
    /// choice reads it).
    pub kind: i32,
    /// +0x20 the per-kind PHYSICS class (0 = none; 1/2/3/6 run
    /// FUN_0040de9c — E-gap, the class is carried for the T3 row).
    pub phys: i32,
    /// +0x24 the start delay (frames).
    pub delay: i32,
    /// +0x28 the caller param.
    pub param: i32,
    /// The seq-table index (`+0x2C` pointer in the original).
    pub table: u8,
}

// Default is derived: the all-zero record IS the free-slot shape
// (§7j.11 +0 == free).

/// One splash-bank record (250 × 0xA at 0x4e9778) [§7j.10/2]:
/// `{x tile, y tile, z level, delay, age}`. age 0 = free. The
/// TICK body (the odd-frame fall + the per-tick scorch) is the S4
/// pairing E-gap — records stage + hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SplashRecord {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub delay: u16,
    pub age: u16,
}

// ---------------------------------------------------------------------
// The debris kind configuration (§7j.11 items 2/3/5)
// ---------------------------------------------------------------------

/// The eleven −1-terminated i16 sprite walks (DGROUP
/// 0x454424..0x454510, bytes verified §7j.11/5). Index order is
/// the DGROUP address order: 0x454424 (k1/5/6/9/11/12), 0x45443e
/// (k19), 0x454458 (k18), 0x454472 (k16), 0x45448c (k17),
/// 0x4544a6 (k20), 0x4544c2 (k2/k8), 0x4544ce (k4), 0x4544e0
/// (k3), 0x4544f0 (k7), 0x4544fe (k10).
pub const DEBRIS_SEQ_TABLES: [&[i16]; 11] = [
    &[5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, -1],
    &[44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, -1],
    &[56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, -1],
    &[68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, -1],
    &[80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, -1],
    &[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, -1],
    &[0, 1, 2, 3, 4, -1],
    &[29, 30, 31, 32, 33, 34, 35, 36, -1],
    &[37, 38, 39, 40, 41, 42, 43, -1],
    &[17, 18, 19, 20, 21, 22, 23, -1],
    &[24, 25, 26, 27, 28, -1],
];

/// The seq table for a kind (the `+0x2C` pointer target). Kinds
/// 13/14/15 share the kind-1 body (its tail jumps into the k20
/// ring path — §7j.11/3).
pub fn debris_seq_table(kind: i32) -> &'static [i16] {
    match kind {
        1 | 5 | 6 | 9 | 11 | 12 | 13 | 14 | 15 => DEBRIS_SEQ_TABLES[0],
        2 | 8 => DEBRIS_SEQ_TABLES[6],
        3 => DEBRIS_SEQ_TABLES[8],
        4 => DEBRIS_SEQ_TABLES[7],
        7 => DEBRIS_SEQ_TABLES[9],
        10 => DEBRIS_SEQ_TABLES[10],
        16 => DEBRIS_SEQ_TABLES[3],
        17 => DEBRIS_SEQ_TABLES[4],
        18 => DEBRIS_SEQ_TABLES[2],
        19 => DEBRIS_SEQ_TABLES[1],
        20 => DEBRIS_SEQ_TABLES[5],
        _ => &[-1],
    }
}

/// The per-kind stage-time scorch write [§7j.11/3, CORRECTED]:
/// Ring kinds stage the NINE-write 3×3 ring (corners 1 / edges 2 /
/// center 4); k2/k8 stage ONE center write (values 3/4); the rest
/// stage none.
pub enum DebrisScorch {
    Ring,
    Center(u8),
    None,
}

/// The kind configuration table [§7j.11/2]: (table, phys class,
/// init words, scorch).
pub fn debris_kind_config(kind: i32) -> (&'static [i16], i32, i32, DebrisScorch) {
    match kind {
        1 => (debris_seq_table(1), 6, 0x40, DebrisScorch::Ring),
        2 => (debris_seq_table(2), 0, 0x20, DebrisScorch::Center(3)),
        3 => (debris_seq_table(3), 1, 0x20, DebrisScorch::Ring),
        4 => (debris_seq_table(4), 2, 0x20, DebrisScorch::Ring),
        5 => (debris_seq_table(5), 0, 0x40, DebrisScorch::Ring),
        6 | 12 => (debris_seq_table(6), 6, 0x40, DebrisScorch::Ring),
        7 => (debris_seq_table(7), 0, 0x40, DebrisScorch::None),
        8 => (debris_seq_table(8), 2, 0x20, DebrisScorch::Center(4)),
        9 => (debris_seq_table(9), 3, 0x40, DebrisScorch::Ring),
        10 => (debris_seq_table(10), 0, 0x40, DebrisScorch::None),
        11 => (debris_seq_table(11), 0, 0x40, DebrisScorch::Ring),
        13..=15 => (debris_seq_table(13), 6, 0x40, DebrisScorch::Ring),
        16 => (debris_seq_table(16), 6, 0x40, DebrisScorch::None),
        17 => (debris_seq_table(17), 6, 0x40, DebrisScorch::None),
        18 => (debris_seq_table(18), 6, 0x40, DebrisScorch::None),
        19 => (debris_seq_table(19), 6, 0x40, DebrisScorch::None),
        20 => (debris_seq_table(20), 6, 0x40, DebrisScorch::Ring),
        _ => (&[-1], 0, 0x40, DebrisScorch::None),
    }
}

/// The artillery burst pair lists [§7j.38/5 — the 7 walks behind
/// the 0x456bf0 pointers, (Δy,Δx) until the 500 sentinel]:
/// expanding square rings; list 0 is the full 3×3 block INCLUDING
/// the center, list 6 carries a 2-pair tail duplicate (the
/// original fires those tiles TWICE — faithful).
pub const ARTILLERY_PAIRS: [&[(i16, i16)]; 7] = [
    &[
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ],
    &[
        (-1, -2),
        (0, -2),
        (1, -2),
        (2, -1),
        (2, 0),
        (2, 1),
        (-1, 2),
        (0, 2),
        (1, 2),
        (-2, -1),
        (-2, 0),
        (-2, 1),
    ],
    &[
        (-2, -3),
        (-1, -3),
        (0, -3),
        (1, -3),
        (2, -3),
        (-3, -2),
        (-2, -2),
        (2, -2),
        (3, -2),
        (-3, -1),
        (3, -1),
        (-3, 0),
        (3, 0),
        (-3, 1),
        (3, 1),
        (-3, 2),
        (-2, 2),
        (2, 2),
        (3, 2),
        (-2, 3),
        (-1, 3),
        (0, 3),
        (1, 3),
        (2, 3),
    ],
    &[
        (-2, -4),
        (-1, -4),
        (0, -4),
        (1, -4),
        (2, -4),
        (-3, -3),
        (3, -3),
        (-4, -2),
        (-4, -1),
        (-4, 0),
        (-4, 1),
        (-4, 2),
        (4, -2),
        (4, -1),
        (4, 0),
        (4, 1),
        (4, 2),
        (-3, 3),
        (3, 3),
        (-2, 4),
        (-1, 4),
        (0, 4),
        (1, 4),
        (2, 4),
    ],
    &[
        (-2, -5),
        (-1, -5),
        (0, -5),
        (1, -5),
        (2, -5),
        (-4, -4),
        (-3, -4),
        (3, -4),
        (4, -4),
        (-4, -3),
        (4, -3),
        (-5, -2),
        (-5, -1),
        (-5, 0),
        (-5, 1),
        (-5, 2),
        (5, -2),
        (5, -1),
        (5, 0),
        (5, 1),
        (5, 2),
        (-4, 3),
        (4, 3),
        (-4, 4),
        (-3, 4),
        (3, 4),
        (4, 4),
        (-2, 5),
        (-1, 5),
        (0, 5),
        (1, 5),
        (2, 5),
    ],
    &[
        (-3, -6),
        (-2, -6),
        (-1, -6),
        (0, -6),
        (1, -6),
        (2, -6),
        (3, -6),
        (-3, 6),
        (-2, 6),
        (-1, 6),
        (0, 6),
        (1, 6),
        (2, 6),
        (3, 6),
        (-6, -3),
        (-6, -2),
        (-6, -1),
        (-6, 0),
        (-6, 1),
        (-6, 2),
        (-6, 3),
        (6, -3),
        (6, -2),
        (6, -1),
        (6, 0),
        (6, 1),
        (6, 2),
        (6, 3),
        (-5, -5),
        (-4, -5),
        (-3, -5),
        (3, -5),
        (4, -5),
        (5, -5),
        (-5, 5),
        (-4, 5),
        (-3, 5),
        (3, 5),
        (4, 5),
        (5, 5),
        (-5, -4),
        (-5, -3),
        (5, -4),
        (5, -3),
        (-5, 4),
        (-5, 3),
        (5, 4),
        (5, 3),
    ],
    &[
        (-1, -8),
        (0, -8),
        (1, -8),
        (-1, 8),
        (0, 8),
        (1, 8),
        (-8, -1),
        (-8, 0),
        (-8, 1),
        (8, -1),
        (8, 0),
        (8, 1),
        (-4, -7),
        (-3, -7),
        (-2, -7),
        (-1, -7),
        (0, -7),
        (1, -7),
        (2, -7),
        (3, -7),
        (4, -7),
        (-4, 7),
        (-3, 7),
        (-2, 7),
        (-1, 7),
        (0, 7),
        (1, 7),
        (2, 7),
        (3, 7),
        (4, 7),
        (-7, -4),
        (-7, -3),
        (-7, -2),
        (-7, -1),
        (-7, 0),
        (-7, 1),
        (-7, 2),
        (-7, 3),
        (-7, 4),
        (7, -4),
        (7, -3),
        (7, -2),
        (7, -1),
        (7, 0),
        (7, 1),
        (7, 2),
        (7, 3),
        (7, 4),
        (-6, -6),
        (-5, -6),
        (-4, -6),
        (-6, -5),
        (-6, -4),
        (6, -6),
        (5, -6),
        (4, -6),
        (6, -5),
        (6, -4),
        (6, 6),
        (5, 6),
        (4, 6),
        (6, 5),
        (6, 4),
        (-6, 6),
        (-5, 6),
        (-4, 6),
        (-6, -5),
        (-6, -4), // the faithful 2-pair tail duplicate
    ],
];

impl MissionSim {
    // -----------------------------------------------------------------
    // Staging seams (the host side of the D51 pattern)
    // -----------------------------------------------------------------

    /// Stage the destroy family from the mission files: the .BDG
    /// type table, the .POS instance list (footprints stamped into
    /// the object-presence grid, hp := the type row's hp — the
    /// FUN_0041a4f8 load tail), and the .TRT terrain-structure
    /// bank. `linear` feeds the turret hp formula. Idempotent per
    /// mission (replaces any prior staging). `zone` is the
    /// [0x4edd8c] terrain set (1..7) — gates the objective notify
    /// and indexes the rubble/water tables. The mirror banks are
    /// (re)allocated EMPTY here; stage the TOT words with
    /// [`MissionSim::stage_terrain_mirror`] and the 0x7d2/0x7d3
    /// words with [`MissionSim::stamp_hazard_words`] after this
    /// (the load order 0x447b76 → 0x447b8f: footprints first, the
    /// hazard stamp second).
    pub fn stage_destroy_family(
        &mut self,
        bdg: &ObjectTypeTable,
        pos: &[u8],
        trt: &[u8],
        zone: u32,
        linear: u32,
    ) -> bool {
        let (w, h) = self.terrain.size();
        if w <= 0 || h <= 0 {
            return false;
        }
        let n = (w * h) as usize;
        self.zone = zone;
        self.linear = linear;
        self.object_types = bdg.rows.clone();
        self.structures = parse_trt(trt, linear).unwrap_or_default();
        // .POS: 2000 × 16 B `(x, y, z, id)`; the id 0xFFFFFFFF
        // sentinel ends the record run [FORMATS §12].
        self.objects.clear();
        if pos.len() == 16 * OBJECT_INSTANCE_SLOTS {
            for slot in 0..OBJECT_INSTANCE_SLOTS {
                let o = 16 * slot;
                let rd = |k: usize| i32::from_le_bytes(pos[o + k..o + k + 4].try_into().unwrap());
                let (x, y, z, id) = (rd(0), rd(4), rd(8), rd(12));
                if id == -1 {
                    continue;
                }
                self.objects.push(ObjectInstance {
                    x,
                    y,
                    z,
                    id,
                    destroyed: false,
                    hp: 0,
                });
            }
        } else {
            return false;
        }
        // The footprint re-stamp pass [§7j.13/4]: hp := the type
        // row's hp; word := record_index+1 over W×H at the origin.
        self.object_grid = vec![0u16; n];
        self.platform_strength = vec![0u16; n];
        for (idx, inst) in self.objects.iter_mut().enumerate() {
            let Some(ty) = bdg.rows.get(inst.id as usize) else {
                continue; // out-of-range id: no type row, no stamp
            };
            inst.hp = ty.hp;
            for i in 0..ty.h as i32 {
                for j in 0..ty.w as i32 {
                    let (x, y) = (inst.x + j, inst.y + i);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue; // EXW writes unchecked; E skips OOB
                    }
                    self.object_grid[(y * w + x) as usize] = (idx + 1) as u16;
                }
            }
        }
        // The mirror/seen banks: staged empty (the init_tiles TOT
        // staging is the S5 pairing; the restore writes land here).
        self.mirror_words = vec![0u16; 8 * n];
        self.mirror_seen = vec![0u8; 8 * n];
        self.mirror_heights = vec![(0u8, 0u8); n];
        self.platform_site = (0, 0);
        true
    }

    /// Stage the TOT-mirror plane words (8 per tile) — the
    /// init_tiles seam [§7h.4]: the host passes the per-tile
    /// NONZERO-filtered TOT words; the seen fill stays the S5
    /// pairing. Words arrive tile-major (8 consecutive per tile).
    pub fn stage_terrain_mirror(&mut self, words: &[u16]) -> bool {
        let (w, h) = self.terrain.size();
        let n = (w * h) as usize;
        if words.len() != 8 * n {
            return false;
        }
        if self.mirror_words.len() != 8 * n {
            self.mirror_words = vec![0u16; 8 * n];
            self.mirror_seen = vec![0u8; 8 * n];
            self.mirror_heights = vec![(0u8, 0u8); n];
        }
        self.mirror_words.copy_from_slice(words);
        true
    }

    /// FUN_00422f18 — the 0x7d2/0x7d3 mission-load stamper
    /// [§7j.12/6, verified]: for EVERY tile, for z 0..7, a mirror
    /// z-word in `[HAZARD_7D2[zone], +4)` stamps the object-grid
    /// word 0x7d2; in `[HAZARD_7D3[zone], +4)` stamps 0x7d3 (later
    /// z can overwrite earlier). Runs AFTER the footprint stamp in
    /// the original load order — call after
    /// [`MissionSim::stage_destroy_family`] +
    /// [`MissionSim::stage_terrain_mirror`].
    pub fn stamp_hazard_words(&mut self) {
        let (w, h) = self.terrain.size();
        if w <= 0 || h <= 0 || self.mirror_words.len() != 8 * (w * h) as usize {
            return;
        }
        let zone = self.zone as usize;
        let (Some(&b72), Some(&b73)) = (HAZARD_7D2.get(zone), HAZARD_7D3.get(zone)) else {
            return;
        };
        for tile in 0..(w * h) as usize {
            let mut word = 0u16;
            for z in 0..8 {
                let zw = self.mirror_words[tile * 8 + z] as i32;
                if zw != 0 && (b72..b72 + 4).contains(&zw) {
                    word = 0x7d2;
                } else if zw != 0 && (b73..b73 + 4).contains(&zw) {
                    word = 0x7d3;
                }
            }
            if word != 0 {
                self.object_grid[tile] = word;
            }
        }
    }

    /// The staged object instances in record order.
    pub fn objects(&self) -> &[ObjectInstance] {
        &self.objects
    }

    /// The object-presence grid word at a tile (0x460dfa).
    pub fn object_grid_word(&self, x: i32, y: i32) -> u16 {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return 0;
        }
        self.object_grid
            .get((y * w + x) as usize)
            .copied()
            .unwrap_or(0)
    }

    /// The terrain-structure bank (the .TRT staging).
    pub fn structures(&self) -> &[TerrainStructure] {
        &self.structures
    }

    /// The debris ring (T3 surface; never hashed).
    pub fn debris_bank(&self) -> &[DebrisRecord] {
        &self.debris
    }

    /// The splash bank (T3 surface; never hashed).
    pub fn splash_bank(&self) -> &[SplashRecord] {
        &self.splashes
    }

    /// The TOT-mirror plane word at (tile, z) — the 0x1E-record
    /// `+2·z` word.
    pub fn mirror_word(&self, tile: usize, z: usize) -> u16 {
        self.mirror_words.get(tile * 8 + z).copied().unwrap_or(0)
    }

    /// The seen byte at (tile, z) — the record's `+0x10+z` byte.
    pub fn mirror_seen(&self, tile: usize, z: usize) -> u8 {
        self.mirror_seen.get(tile * 8 + z).copied().unwrap_or(0)
    }

    /// The pending destroy score award (the [0x4dd40c] delta the
    /// scene folds into the campaign score) + whether the score
    /// strip redraw armed ([0x46ccf0] := 2). Taking clears both.
    pub fn take_destroy_score(&mut self) -> (i32, bool) {
        let out = (self.score_pending, self.strip_arm);
        self.score_pending = 0;
        self.strip_arm = false;
        out
    }

    // -----------------------------------------------------------------
    // The two resolvers + the destroy tail (FUN_0041a894 /
    // FUN_0041bc1c) [§7j.13, §7j.25, §7j.32, §7j.39]
    // -----------------------------------------------------------------

    /// FUN_0041a894 — the per-tile WEAPON-IMPACT OBJECT RESOLVER
    /// [§7j.13/1-2, verified]. `x`/`y` are Q13 world coords,
    /// `counter` the shared chain counter, `damage` the weapon
    /// damage, `score_flag` the fire site's push-1 score arm.
    /// Returns the destroy bit (1 = an object died; the callers
    /// that check it stop their walk). Draw-exact per §7j.38/1-2.
    pub fn resolve_object_impact(
        &mut self,
        x_q13: i32,
        y_q13: i32,
        counter: i32,
        damage: i32,
        score_flag: bool,
    ) -> bool {
        let (w, h) = self.terrain.size();
        if x_q13 < 0 || y_q13 < 0 || x_q13 >> 13 >= w || y_q13 >> 13 >= h {
            return false;
        }
        let tile = ((y_q13 >> 13) * w + (x_q13 >> 13)) as usize;
        let Some(&word) = self.object_grid.get(tile) else {
            return false; // nothing staged (the no-inject default)
        };
        if word == 0 || word == 0x7d2 || word == 0x7d3 {
            return false; // pass-through: empty / hazard / clamp
        }
        if word == 0x7d4 {
            // The platform entry [§7j.12/2]: damage the platform
            // word; the projectile keeps flying.
            self.platform_damage(x_q13 >> 13, y_q13 >> 13, damage);
            return false;
        }
        let Some(idx) = word.checked_sub(1) else {
            return false;
        };
        let Some(inst) = self.objects.get_mut(idx as usize) else {
            return false;
        };
        if inst.destroyed || inst.hp == -1 {
            return false; // already fired / immune
        }
        inst.hp -= damage;
        if inst.hp > 0 {
            return false; // survivor: pure subtract, nothing else
        }
        inst.hp = 0;
        inst.destroyed = true;
        self.destroy_tail(idx as usize, counter, score_flag);
        true
    }

    /// The destroy tail [§7j.25/1-3, §7j.38/1-2, §7j.39/5]:
    /// objective notify → the GER gate → the footprint terrain
    /// RESTORE → the five-effect loop → the score award → the four
    /// perimeter CHAIN walks. The trigger producers
    /// (FUN_00422e0a/FUN_00422600) are S7-routed no-ops; the
    /// [0x46cce4] quake notify is presentation.
    fn destroy_tail(&mut self, idx: usize, counter: i32, score_flag: bool) {
        // notify [0x46cce4] := 2 is presentation (the quake
        // countdown — the renderer's shake tables); zone ≠ 1 →
        // the objective notify FUN_00448b80 subset.
        if self.zone != 1 {
            self.objective_notify(idx);
        }
        let Some(ty) = self
            .object_types
            .get(self.objects[idx].id as usize)
            .cloned()
        else {
            return;
        };
        // The GER gate [§7j.25/1, refined]: type 0xb ∧ language 1
        // (GER) skips the restore/effect/score/chain tail WHOLE —
        // the record is already marked destroyed + notified.
        if ty.kind == 0xb && self.language == 1 {
            return;
        }
        // The terrain RESTORE [§7j.25/2, §7j.32/3]: nested i<H,
        // j<W, z ∈ [z0, min(z0+D, 8)); per cell the linear template
        // index (z·H+i)·W+j writes the UNDER pair.
        let (w, h) = self.terrain.size();
        let (ox, oy, oz) = {
            let inst = &self.objects[idx];
            (inst.x, inst.y, inst.z)
        };
        let cells = (ty.w as usize) * (ty.h as usize) * (ty.d as usize);
        let z_end = (oz + ty.d as i32).min(8);
        for zz in oz.max(0)..z_end {
            for i in 0..ty.h as i32 {
                for j in 0..ty.w as i32 {
                    let (x, y) = (ox + j, oy + i);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let lin =
                        (zz as usize * ty.h as usize + i as usize) * ty.w as usize + j as usize;
                    if lin >= cells {
                        continue;
                    }
                    let tile = (y * w + x) as usize;
                    let under_tot = ty.bank_under_tot.get(lin).copied().unwrap_or(0);
                    let under_dat = ty.bank_under_dat.get(lin).copied().unwrap_or(0);
                    self.mirror_words[tile * 8 + zz as usize] = under_tot;
                    let seen = if under_dat == 0 { 1 } else { 0 };
                    self.mirror_seen[tile * 8 + zz as usize] = seen;
                    self.terrain.dat_write(x, y, zz, (under_dat & 0xFF) as u8);
                }
            }
        }
        // The FIVE-EFFECT loop [§7j.25/3-4, §7j.38/1 — draw-exact].
        for (m, entry) in ty.effects.iter().enumerate() {
            let m = m as i32;
            let sel = entry.selector;
            if !(1..=9).contains(&sel) {
                continue; // 0 or >9: skip with NO draws
            }
            let ex = ox + entry.dx as i32;
            let ey = oy + entry.dy as i32;
            let ez = oz + entry.dz as i32;
            match sel {
                1 => {
                    // k14 at the entry center (+0xF,+0xF) then the
                    // effects stager (RandB-only — unmodeled) then
                    // 1 plain splash + a 4× (2R + probe + splash)
                    // loop. 8 RandA.
                    self.stage_debris(
                        ex * 0x20 + 0xF,
                        ey * 0x20 + 0xF,
                        ez * 0x20,
                        14,
                        counter + m,
                        score_flag as i32,
                    );
                    let pz = self.splash_probe_z(ex, ey, ez);
                    self.stage_splash(ex, ey, pz, 0);
                    let (mut sx, mut sy) = (ex, ey);
                    for _ in 0..4 {
                        sx -= (self.rand_a() & 1) as i32;
                        sy -= (self.rand_a() & 1) as i32;
                        let pz = self.splash_probe_z(sx, sy, ez);
                        self.stage_splash(sx, sy, pz, 0);
                    }
                }
                2..=5 => {
                    // k18/k17/k16/k19 single gibs + the 4× splash
                    // loop. 8 RandA each.
                    let (kind, dx, dy) = match sel {
                        2 => (18, 0x10, 0x30),
                        3 => (17, 0x30, 0x10),
                        4 => (16, 0x20, -0x10),
                        _ => (19, -0x20, 0),
                    };
                    self.stage_debris(
                        ex * 0x20 + dx,
                        ey * 0x20 + dy,
                        ez * 0x20,
                        kind,
                        0,
                        score_flag as i32,
                    );
                    let (mut sx, mut sy) = (ex, ey);
                    for _ in 0..4 {
                        sx -= (self.rand_a() & 1) as i32;
                        sy -= (self.rand_a() & 1) as i32;
                        let pz = self.splash_probe_z(sx, sy, ez);
                        self.stage_splash(sx, sy, pz, 0);
                    }
                }
                6 | 7 => {
                    // k10 quiet collapses + the DEADMAN SFX pick
                    // (RandB — T4, never drawn). 0 RandA.
                    let (dx, dy) = if sel == 6 { (0x10, 0x20) } else { (0x20, 0x10) };
                    self.stage_debris(ex * 0x20 + dx, ey * 0x20 + dy, ez * 0x20, 10, 0, -1);
                }
                8 => {
                    // The 24-iteration demolition shower: per i 3R
                    // (x/y ±3-tile RandA&7−3 jitter, z +R&3) + the
                    // water-z probe + k14 + splash (+ the RandB
                    // effects stager). 72 RandA.
                    self.stage_debris(
                        ex * 0x20 + 0xF,
                        ey * 0x20 + 0xF,
                        ez * 0x20,
                        14,
                        counter + m,
                        score_flag as i32,
                    );
                    for i in 0..24 {
                        let jx = ex + (self.rand_a() & 7) as i32 - 3;
                        let jy = ey + (self.rand_a() & 7) as i32 - 3;
                        let jz = ez + (self.rand_a() & 3) as i32;
                        let pz = self.splash_probe_z(jx, jy, jz);
                        self.stage_debris(
                            jx * 0x20,
                            jy * 0x20,
                            pz * 0x20,
                            14,
                            counter + m + (i >> 3),
                            score_flag as i32,
                        );
                        self.stage_splash(jx, jy, pz, 0);
                    }
                }
                _ => {
                    // sel 9: k20 at (+0xF,+0xF,+0xF) + the plain
                    // probe at (x−1, y−1) + the 3×3 splash ring
                    // (1R per cell → the delay). 9 RandA.
                    self.stage_debris(
                        ex * 0x20 + 0xF,
                        ey * 0x20 + 0xF,
                        ez * 0x20 + 0xF,
                        20,
                        counter + m,
                        score_flag as i32,
                    );
                    let pz = self.splash_probe_z(ex - 1, ey - 1, ez);
                    let pz = if pz >= 7 { 7 } else { pz };
                    let base = counter + 2;
                    for c in (ex - 2)..(ex + 1) {
                        for r in (ey - 2)..(ey + 1) {
                            let extra = (self.rand_a() & 3) as i32;
                            self.stage_splash(c, r, pz, (base + extra) as u16);
                        }
                    }
                }
            }
        }
        // The score award [§7j.13/3]: gated by the stack flag;
        // type 0xb → +10 else += the type value; the strip arms.
        if score_flag {
            self.score_pending += if ty.kind == 0xb { 10 } else { ty.kind };
            self.strip_arm = true;
        }
        // The four perimeter CHAIN walks [§7j.39/5 — corrected
        // geometry]: N row (y−1), S row (y+H), W edge (x−1), E edge
        // (x+W); one RandA per QUALIFYING candidate (word > 0
        // signed, instance alive, type chain ≠ 0); the recursive
        // detonation at damage 1000 with the forwarded flag.
        fn walk(sim: &mut MissionSim, x: i32, y: i32, counter: &mut i32, score_flag: bool) {
            let word = sim.object_grid_word(x, y);
            if (word as i16) - 1 <= 0 {
                return; // 0 / hazard / clamp words never qualify
            }
            let n = (word - 1) as usize;
            let Some(inst) = sim.objects.get(n) else {
                return;
            };
            if inst.destroyed {
                return;
            }
            let chainable = sim
                .object_types
                .get(inst.id as usize)
                .is_some_and(|t| t.chain != 0);
            if !chainable {
                return;
            }
            let roll = sim.rand_a();
            if roll & 3 == 0 {
                *counter += 1;
            }
            sim.resolve_object_impact(x * 0x2000, y * 0x2000, *counter, 1000, score_flag);
        }
        let mut ctr = counter;
        // Walk 1 — the N row: y' = y−1, x' = x+j, j ∈ [−1, W].
        let ny = oy - 1;
        for j in -1..=ty.w as i32 {
            let x = ox + j;
            if x > 0 && x < w && ny > 0 && ny < h {
                walk(self, x, ny, &mut ctr, score_flag);
            }
        }
        // Walk 2 — the S row: candidate gate at y+H; the RECURSION
        // passes (y+W)<<13 — the faithful original quirk
        // [§7j.39/5; corpus W == H everywhere].
        let sy = oy + ty.h as i32;
        for j in -1..=ty.w as i32 {
            let x = ox + j;
            if !(x > 0 && x < w && sy > 0 && sy < h) {
                continue;
            }
            let word = self.object_grid_word(x, sy);
            if (word as i16) - 1 <= 0 {
                continue;
            }
            let n = (word - 1) as usize;
            let chainable = match self.objects.get(n) {
                Some(inst) => {
                    !inst.destroyed
                        && self
                            .object_types
                            .get(inst.id as usize)
                            .is_some_and(|t| t.chain != 0)
                }
                None => false,
            };
            if !chainable {
                continue;
            }
            let roll = self.rand_a();
            if roll & 3 == 0 {
                ctr += 1;
            }
            let qy = oy + ty.w as i32; // the (y+W) recursion quirk
            self.resolve_object_impact(x * 0x2000, qy * 0x2000, ctr, 1000, score_flag);
        }
        // Walk 3 — the W edge: x' = x−1, y' = y+j, j ∈ [0, H).
        let wx = ox - 1;
        for j in 0..ty.h as i32 {
            let y = oy + j;
            if wx > 0 && wx < w && y > 0 && y < h {
                walk(self, wx, y, &mut ctr, score_flag);
            }
        }
        // Walk 4 — the E edge: x' = x+W, y' = y+j, j ∈ [0, H).
        let exx = ox + ty.w as i32;
        for j in 0..ty.h as i32 {
            let y = oy + j;
            if exx > 0 && exx < w && y > 0 && y < h {
                walk(self, exx, y, &mut ctr, score_flag);
            }
        }
    }

    /// FUN_00448b80's modeled subset [§7j.32/2]: zone-7 gate, the
    /// [0x46cce0] counter decrement, the +0x1B/+0x1C height clears
    /// over the footprint. The at-zero extraction-arm tail (SFX +
    /// the 0x46cd00/0x46ccfc/0x46ccc4 arm cells) is the S6-seam
    /// E-gap.
    fn objective_notify(&mut self, idx: usize) {
        if self.zone != 7 {
            return;
        }
        let Some(inst) = self.objects.get(idx) else {
            return;
        };
        let Some(ty) = self.object_types.get(inst.id as usize).cloned() else {
            return;
        };
        if !(0x44..=0x47).contains(&ty.kind) {
            return;
        }
        if self.objective_count > 0 {
            self.objective_count -= 1;
        }
        let (w, h) = self.terrain.size();
        for i in 0..ty.h as i32 {
            for j in 0..ty.w as i32 {
                let (x, y) = (inst.x + j, inst.y + i);
                if x < 0 || y < 0 || x >= w || y >= h {
                    continue;
                }
                self.mirror_heights[(y * w + x) as usize] = (0, 0);
            }
        }
    }

    /// Stage the language latch [0x4eba1c] (1 = GER) — the
    /// destroy-tail GER gate's selector. Host-seamed like
    /// [`MissionSim::set_difficulty`]; 0 = the modeled default.
    pub fn set_language(&mut self, language: u32) {
        self.language = language;
    }

    /// Stage one platform tile — the FUN_004228ce write half
    /// [§7j.12/3]: the water z-structure at the empty level (word =
    /// the zone water base, seen 1), the 0x7d4 grid word, the
    /// strength word. The spread-ring BUILD conditions (the
    /// emptiness/claim/robot-presence gates) are the S7 seam — the
    /// host stages the result.
    pub fn stage_platform(&mut self, x: i32, y: i32, z: i32, strength: u16) -> bool {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h || self.object_grid.len() != (w * h) as usize {
            return false;
        }
        let zone = self.zone as usize;
        let Some(&base) = WATER_RANGE.get(zone) else {
            return false;
        };
        let tile = (y * w + x) as usize;
        self.z_structure_write(x, y, z, base as u16, 0);
        self.object_grid[tile] = 0x7d4;
        self.platform_strength[tile] = strength;
        true
    }

    /// The platform STRENGTH word at a tile (0 = none) — the read
    /// seam for tests/hosts.
    pub fn platform_strength_word(&self, x: i32, y: i32) -> u16 {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return 0;
        }
        self.platform_strength
            .get((y * w + x) as usize)
            .copied()
            .unwrap_or(0)
    }

    /// FUN_0041bc1c — the TERRAIN-STRUCTURE damage resolver
    /// [§7j.14/1, verified]: scans the .TRT bank for an active
    /// record at the impact tile; a survivor takes the pure hp
    /// subtract; a death stamps the rubble word + seen + a zero
    /// DAT volume byte, stages k15 debris + the water-z splash.
    /// No RandA draws on this path (the k15 body has none).
    pub fn resolve_structure_impact(&mut self, x_q13: i32, y_q13: i32, damage: i32) {
        let (w, h) = self.terrain.size();
        if x_q13 < 0 || y_q13 < 0 || x_q13 >> 13 >= w || y_q13 >> 13 >= h {
            return;
        }
        let (tx, ty) = (x_q13 >> 13, y_q13 >> 13);
        let hit = self
            .structures
            .iter()
            .position(|s| s.active && s.x == tx && s.y == ty);
        let Some(i) = hit else {
            return;
        };
        self.structures[i].hp -= damage;
        if self.structures[i].hp > 0 {
            return;
        }
        self.structures[i].active = false;
        let (z, zone) = (self.structures[i].z, self.zone as usize);
        let rubble = RUBBLE_WORD.get(zone).copied().unwrap_or(0x20);
        let tile = (ty * w + tx) as usize;
        let zi = z as usize;
        if (0..8).contains(&z) && tile * 8 + zi < self.mirror_words.len() {
            self.mirror_words[tile * 8 + zi] = rubble as u16;
            self.mirror_seen[tile * 8 + zi] = 1;
        }
        self.terrain.dat_write(tx, ty, z, 0);
        // k15 debris (×0x20 coords, delay 0, param −1) + the splash
        // at the FUN_0041bd78 water z [§7j.32/8].
        self.stage_debris(tx * 0x20, ty * 0x20, z * 0x20, 15, 0, -1);
        let pz = self.splash_probe_z(tx, ty, z);
        self.stage_splash(tx, ty, pz, 0);
    }

    // -----------------------------------------------------------------
    // The stagers (FUN_00420608 debris, FUN_00424355 splash)
    // -----------------------------------------------------------------

    /// FUN_00420608 — the debris stager, all 20 kinds [§7j.11/2-3,
    /// §7j.39/6 head verified]: bounds `x/y ≥ 0 ∧ (x>>5) < w ∧
    /// (y>>5) < h` (else NO staging), z clamped [0x20, 0xFF],
    /// first-free allocation else min-`+0x18`-seq eviction, the
    /// per-kind scorch write (the NINE ring via the +0x18 byte
    /// bank / the k2/k8 single center / none), and the kind-11
    /// SFX-gate RandA draw (the sound is T4 — the DRAW is real).
    pub fn stage_debris(
        &mut self,
        x_q5: i32,
        y_q5: i32,
        z: i32,
        kind: i32,
        delay: i32,
        param: i32,
    ) -> bool {
        let (w, h) = self.terrain.size();
        if x_q5 < 0 || y_q5 < 0 || x_q5 >> 5 >= w || y_q5 >> 5 >= h {
            return false;
        }
        let z = z.clamp(0x20, 0xFF);
        if !(1..=20).contains(&kind) {
            return false;
        }
        // Allocation: first free slot, else the min-seq (LRU).
        let mut slot = None;
        let mut min_seq = i32::MAX;
        let mut min_slot = 0usize;
        for (i, r) in self.debris.iter().enumerate() {
            if !r.active {
                slot = Some(i);
                break;
            }
            if r.seq < min_seq {
                min_seq = r.seq;
                min_slot = i;
            }
        }
        let slot = slot.unwrap_or(min_slot);
        self.debris_seq += 1;
        let (table, phys, init, scorch) = debris_kind_config(kind);
        let table_idx = DEBRIS_SEQ_TABLES
            .iter()
            .position(|t| std::ptr::eq(*t, table))
            .unwrap_or(0) as u8;
        self.debris[slot] = DebrisRecord {
            active: true,
            x: x_q5,
            y: y_q5,
            z,
            init_a: init,
            init_b: 0,
            seq: self.debris_seq,
            kind,
            phys,
            delay,
            param,
            table: table_idx,
        };
        // The per-kind stage-time scorch [§7j.11/3]: the NINE-write
        // ring (corners 1 / edges 2 / center 4) / the k2-k8 single
        // center write / none.
        match scorch {
            DebrisScorch::Ring => {
                for &(dx, dy, value) in DEBRIS_SCORCH_RING.iter() {
                    self.scorch_write(x_q5 + dx, y_q5 + dy, value);
                }
            }
            DebrisScorch::Center(v) => self.scorch_write(x_q5, y_q5, v),
            DebrisScorch::None => {}
        }
        // The kind-11 arrival-SFX gate [§7j.11/4]: ONE RandA draw
        // (~50%) — the SFX family is T4-unmodeled, the draw is not.
        if kind == 11 {
            let _gate = self.rand_a() & 1;
        }
        true
    }

    /// FUN_0041bd78 — the water-z probe [§7j.10, verified]: clamp
    /// z ≤ 7, in-bounds x/y, then scan z upward while
    /// `volume(z) != 0 ∨ seen(z) != 0`; returns the first free
    /// level (7 if exhausted).
    pub fn splash_probe_z(&self, x: i32, y: i32, z: i32) -> i32 {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return 7;
        }
        let tile = (y * w + x) as usize;
        let mut z = z.clamp(0, 7);
        while z < 7 {
            let vol = self.terrain.dat_type(x, y, z);
            let seen = self
                .mirror_seen
                .get(tile * 8 + z as usize)
                .copied()
                .unwrap_or(0);
            if vol == 0 && seen == 0 {
                break;
            }
            z += 1;
        }
        z
    }

    /// FUN_00424355 — the splash STAGER [§7j.10, §7j.14/5]:
    /// in-bounds + z clamp ≤ 7 + DAT volume(z) == 0 + z-word(z) ==
    /// 0 + the tile-claim byte == 0 (the claim bank is host-staged
    /// zeros — the order-marker writers are the D82 seam, the
    /// platform stager is S7); allocation first age==0 slot else
    /// max-age eviction (the evicted record is flushed through the
    /// z-structure clear). NO RNG draws.
    pub fn stage_splash(&mut self, x: i32, y: i32, z: i32, delay: u16) -> bool {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        let z = z.min(7);
        let tile = (y * w + x) as usize;
        if self.terrain.dat_type(x, y, z) != 0 {
            return false;
        }
        if self.mirror_word(tile, z as usize) != 0 {
            return false;
        }
        // Allocation: first free, else max-age evict + flush.
        let mut slot = None;
        let mut max_age = -1i32;
        let mut max_slot = 0usize;
        for (i, r) in self.splashes.iter().enumerate() {
            if r.age == 0 {
                slot = Some(i);
                break;
            }
            if r.age as i32 > max_age {
                max_age = r.age as i32;
                max_slot = i;
            }
        }
        let slot = slot.unwrap_or_else(|| {
            // Flush the evicted record: FUN_0042394a(old, 0, 0).
            let old = self.splashes[max_slot];
            self.z_structure_write(old.x as i32, old.y as i32, old.z as i32, 0, 0);
            max_slot
        });
        self.splashes[slot] = SplashRecord {
            x: x as i16,
            y: y as i16,
            z: z as i16,
            delay,
            age: 1,
        };
        true
    }

    /// FUN_0042394a — the per-tile z-STRUCTURE writer [§7j.10,
    /// verified]: the TOT-mirror z-word at (tile, z) := `word`
    /// (0 = clear), the seen byte := (volume == 0) when word ≠ 0
    /// (cleared when word == 0), and the DAT volume byte :=
    /// `volume`.
    pub fn z_structure_write(&mut self, x: i32, y: i32, z: i32, word: u16, volume: u8) {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h || !(0..8).contains(&z) {
            return;
        }
        let tile = ((y * w + x) as usize) * 8 + z as usize;
        if self.mirror_words.len() <= tile {
            return; // nothing staged (the no-inject default)
        }
        self.mirror_words[tile] = word;
        self.mirror_seen[tile] = if word != 0 && volume == 0 { 1 } else { 0 };
        self.terrain.dat_write(x, y, z, volume);
    }

    // -----------------------------------------------------------------
    // The platform entry + the script blast + the trap lane
    // -----------------------------------------------------------------

    /// FUN_00422693 — the platform DAMAGE entry [§7j.12/2,
    /// verified]: bounds; scan z 0..7 for the FIRST water-range
    /// mirror z-word (none → exit — only real platforms take
    /// damage); `diff = (i16)strength − damage`; a non-positive
    /// diff → DESTROY (clear the water z-structure, zero both
    /// bank words, FIVE k7 debris with 2 RandA draws each — 10
    /// total); a positive diff → WEAKEN (strength := diff, the +4
    /// scorch increment, the spread-ring gate is the S7 E-gap).
    /// Both paths store the creep seed site.
    pub fn platform_damage(&mut self, x: i32, y: i32, damage: i32) {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let zone = self.zone as usize;
        let Some(&base) = WATER_RANGE.get(zone) else {
            return;
        };
        let mut water_z: Option<i32> = None;
        for z in 0..8 {
            let word = self.mirror_word((y * w + x) as usize, z) as i32;
            if word >= base && word < base + 0xE {
                water_z = Some(z as i32);
                break;
            }
        }
        let Some(z) = water_z else {
            return;
        };
        let tile = (y * w + x) as usize;
        let Some(&strength) = self.platform_strength.get(tile) else {
            return; // nothing staged (the no-inject default)
        };
        let strength = strength as i16;
        let diff = strength as i32 - damage;
        self.platform_site = (x, y);
        if diff <= 0 {
            // DESTROY: clear the water z-structure + both banks,
            // then five k7 debris (delay k·2, param −1).
            self.z_structure_write(x, y, z, 0, 0);
            self.platform_strength[tile] = 0;
            if self.object_grid.get(tile) == Some(&0x7d4) {
                self.object_grid[tile] = 0;
            }
            for k in 0..5i32 {
                let dx = (self.rand_a() & 0xF) as i32 + 8;
                let dy = (self.rand_a() & 0xF) as i32 + 8;
                self.stage_debris(x * 0x20 + dx, y * 0x20 + dy, z * 0x20, 7, k * 2, -1);
            }
        } else {
            // WEAKEN: strength := diff; the +4 scorch increment
            // (clamp 7); the spread-ring FUN_00422832 gate is the
            // S7 seam (needs the zone water range + robot
            // presence) — recorded, not modeled.
            if let Some(s) = self.platform_strength.get_mut(tile) {
                *s = diff as u16;
            }
            self.scorch_increment(x * 0x20, y * 0x20, 4);
        }
    }

    /// FUN_004244a1 — the SCRIPT BLAST [§7j.39/1, verified
    /// 0x4244a1..0x4245c4]: the splash stage → the STRUCTURE
    /// resolver at 5000 → the OBJECT resolver at 5000 (flag 1) →
    /// the k6 1-in-8 gate (1 RandA; +1 for the delay when it
    /// passes) → the all-actor area damage at z' = clamp(z−1, ≥1):
    /// every critter takes a kind-0xC hit (the critter bank is an
    /// E-gap — no-op, documented) and every robot the §7j.23 box
    /// test (|dx|,|dy| < 0x20 Q5 + |dz| < 0x30 mode 2) at kind 0xD
    /// through [`MissionSim::apply_damage`].
    pub fn script_blast(&mut self, x: i32, y: i32, z: i32) {
        let (w, h) = self.terrain.size();
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        self.stage_splash(x, y, z.min(7), 0);
        self.resolve_structure_impact(x << 13, y << 13, 5000);
        self.resolve_object_impact(x << 13, y << 13, 0, 5000, true);
        let gate = self.rand_a();
        if gate & 7 == 0 {
            let delay = (self.rand_a() & 7) as i32;
            self.stage_debris(x * 0x20, y * 0x20, z * 0x20, 6, delay, -1);
        }
        let zq = (z - 1).max(1);
        // The critter lane: the 0x46cc2c bank has no E model —
        // an S4+ coverage finding class, never silent state.
        // The robot lane [§7j.23 box test, §7j.39/1]:
        let (bx, by, bz) = (x * 0x20, y * 0x20, zq * 0x20);
        let n = self.robots.len();
        for i in 0..n {
            let (rx, ry, rz, alive) = {
                let r = &self.robots[i];
                let (rx, ry) = r.q5();
                (rx, ry, r.z, r.alive)
            };
            if !alive {
                continue;
            }
            if (rx - bx).abs() < 0x20 && (ry - by).abs() < 0x20 && (rz - bz).abs() < 0x30 {
                let damage = crate::weapon::weapon_damage(0xD, self.difficulty);
                self.apply_damage(i, damage, -1);
            }
        }
    }

    /// FUN_0040fe93 — the tile-0x62 TRAP lane [§7j.25/7, verified]:
    /// the robot's CURRENT tile with a DAT volume byte 0x62 ∧ a
    /// nonzero object-grid word fires the object resolver at
    /// damage 100 (no score); a destroy stages FIVE k12 debris
    /// (3 RandA draws each — x/y/z jitter). Modeled armor-first
    /// per §7j.38/6 (the exact intra-walk interleaving is
    /// unpinned, corpus-never).
    pub fn robot_trap_lane(&mut self, idx: usize) -> bool {
        let Some(r) = self.robots.get(idx) else {
            return false;
        };
        if !r.alive {
            return false;
        }
        let (tx, ty) = r.tile();
        let zl = r.z >> 5;
        if self.terrain.dat_type(tx, ty, zl) != 0x62 {
            return false;
        }
        if self.object_grid_word(tx, ty) == 0 {
            return false;
        }
        let destroyed = self.resolve_object_impact(tx * 0x2000, ty * 0x2000, 0, 100, false);
        if destroyed {
            for k in 0..5i32 {
                let jx = tx * 0x20 + (self.rand_a() & 0x1F) as i32;
                let jy = ty * 0x20 + (self.rand_a() & 0xF) as i32;
                let jz = zl * 0x20 + 0x10 + (self.rand_a() & 0x1F) as i32;
                self.stage_debris(jx, jy, jz, 12, k * 2, -1);
            }
        }
        destroyed
    }

    /// FUN_004124a4 — the WEAPON-ANIM debris disburser
    /// [§7j.14/3 + the §7j.39 raw-asm re-read]: kind-switched K2
    /// (2 jitter draws) / K3 / K6 / K9 / K0xC staging at
    /// (x>>8, y>>8, z>>8 − 0xA), delay 0, param = the owner
    /// dword; 9..0xB clear the kind word only; 0xC/0xD and the
    /// unlisted ids are no-ops (0xF included — the §7j.14 map
    /// corrected: 0xF keeps its word). Every staging/clear branch
    /// frees the slot (kind := 0).
    pub fn weapon_disburser(&mut self, i: usize) {
        let Some(rec) = self.weapon_bank.get(i) else {
            return;
        };
        let (kind, x, y, z, owner) = (rec.kind, rec.x, rec.y, rec.z, rec.owner);
        let (k, jitter) = match kind {
            2..=4 => (2, true),
            5 => (3, false),
            9..=0xB => {
                self.weapon_bank[i].kind = 0;
                return;
            }
            0xE | 0x13 | 0x17 | 0x1A | 0x1F => (0xC, false),
            0x24 => (6, false),
            0x29 => (9, false),
            _ => return, // 0xC/0xD/0xF/0x10..0x12/0x14..0x16/... no-op
        };
        let (mut dx, mut dy) = (x >> 8, y >> 8);
        if jitter {
            dy = (self.rand_a() & 7) as i32 + dy - 3;
            dx = (self.rand_a() & 7) as i32 + dx - 3;
        }
        self.stage_debris(dx, dy, (z >> 8) - 0xA, k, 0, owner);
        self.weapon_bank[i].kind = 0;
    }

    /// FUN_004126dc — the PROJECTILE debris disburser (the 0x22
    /// bank) [§7j.14/4]: types 0x65 → k20, 0x66 → k8, 0x67/0x68 →
    /// k4 at (x>>8, y>>8, z>>8) — NO z−0xA here; every branch
    /// clears the type word.
    pub fn projectile_disburser(&mut self, i: usize) {
        let Some(rec) = self.enemy_bank.get(i) else {
            return;
        };
        let kind = match rec.kind {
            0x65 => 20,
            0x66 => 8,
            0x67 | 0x68 => 4,
            _ => return,
        };
        let (x, y, z) = (rec.x, rec.y, rec.z);
        self.stage_debris(x >> 8, y >> 8, z >> 8, kind, 0, -1);
        self.enemy_bank[i].kind = 0;
    }
}
