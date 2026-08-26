//! W7 the differ (DESIGN-DIFFHARNESS.md §6 + §6a, RE-EXD-MAP §8) —
//! normalizer + comparison modes + report writer + fingerprint
//! manifest.
//!
//! The differ NEVER compares raw bytes across implementations. Each
//! side's dump is first **normalized** into typed canonical fields per
//! the §6a grammar:
//!
//! - **Channel E**: the dump blobs already ARE canonical grammar
//!   (`parity_harness/canonical.rs::emit_frame`); the normalizer parses
//!   them into named fields (which also validates them).
//! - **Channel O1 (EXD/DOSBox-X)**: raw guest bytes convert per the
//!   RE-EXD-MAP §8 field map. Only offsets individually pinned in the
//!   ledger are mapped; every other canonical field is simply NOT
//!   covered by the O1 side (a STRUCTURAL coverage finding in the
//!   report — never zero-filled-and-compared, never guessed). The one
//!   cross-row form is the D90 move-target splice: the raw 0x60-B span
//!   (x[12]+y[12] u32 at 0xf75ec) is consumed into the robot-bank row
//!   as the per-robot `target_*` trio (bounded by the same frame's
//!   robot count), so the raw side carries no standalone
//!   move-target-words row.
//! - **Channel O2 (EXW/Wine)**: same row forms as O1 except the robot
//!   record uses the RE-EXW-SIM §3/§7f/§7g EXW field table (the
//!   seed-#1 EXW-front discrepancy is recorded OPEN in RE-EXD-MAP §8 —
//!   the first live EXW capture arbitrates it).
//!
//! **Comparison modes** (§6):
//! - [`Mode::DoubleRun`] — O1 vs O1 (the DH-G1 verdict instrument):
//!   every row byte-exact EXCEPT the budgeted classes (frame-counter
//!   T2 report-only; rng-state-a/b T3 accepted — "identical chains
//!   MODULO the frame-counter/RNG blob bytes").
//! - [`Mode::CrossChannel`] — O1 vs E with an optional O2 tiebreak:
//!   per-row/per-field class table (STRUCTURAL exact, T1-exact, T2
//!   tolerant, T3 statistical), coverage findings for rows/fields one
//!   side cannot source (the §6a E-gap list falls out of the data,
//!   never silently skipped), and O2 arbitration: a T1 diff where the
//!   O2 canon agrees with E is `original-divergence` (EXD≠EXW; engine
//!   keeps EXW); anything else is `engine-bug` (provisional when no
//!   tiebreak dump is supplied).
//!
//! **Alignment**: frame-indexed by the record `frame_no` (NOT the
//! frame-counter watch — the O1 counter carries the mission-entry
//! constant C₀ = the scripted menu walk's leftover, deterministic per
//! script but not mission-relative; it never resets inside the mission
//! loop, RE-EXW-SIM §7j.66 — the menu-path resets that produce C₀ are
//! recorded there).
//! A constant shift ≤8 is detected, applied, and reported as a finding
//! (§6 anchor-event alignment); anything worse is a STRUCTURAL
//! misalignment and only the common frames compare.
//!
//! Everything is deterministic: identical inputs produce identical
//! report text and manifest JSON.

use std::collections::BTreeMap;
use std::fmt;

use crate::dump::{Channel, Dump, FrameRecord};
use crate::Watch;

// ---------------------------------------------------------------------
// Config + value model
// ---------------------------------------------------------------------

/// Which comparison mode (DESIGN §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// O1 vs O1 double-run: byte-exact except the budgeted classes.
    DoubleRun,
    /// O1 vs E (optional O2 tiebreak): the per-field class table.
    CrossChannel,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::DoubleRun => "double-run",
            Mode::CrossChannel => "cross-channel",
        }
    }
}

/// Differ tuning. Defaults are the DESIGN §6 budget; the CLI exposes
/// the quantum for live calibration.
#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub mode: Mode,
    /// Tolerated |Δ| for T2-tolerant numeric fields (Q13 sub-tile
    /// granularity by default). Diffs within the quantum are counted
    /// but not reported; beyond it they are report-only findings.
    pub t2_quantum: i64,
}

impl DiffConfig {
    pub fn new(mode: Mode) -> Self {
        DiffConfig {
            mode,
            t2_quantum: 0x20,
        }
    }
}

/// One normalized field value. Ints compare numerically (widths are a
/// grammar concern, handled at parse); Bytes compare byte-wise (the
/// statics + raw passthrough rows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldVal {
    Int(i128),
    Bytes(Vec<u8>),
}

impl fmt::Display for FieldVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldVal::Int(v) => write!(f, "{v}"),
            // Statics/passthrough blobs print as length + sha-less hex
            // head — enough to identify, bounded in size.
            FieldVal::Bytes(b) => {
                if b.len() <= 16 {
                    write!(f, "{}B[{}]", b.len(), hex_head(b, 16))
                } else {
                    write!(f, "{}B[{}..]", b.len(), hex_head(b, 8))
                }
            }
        }
    }
}

fn hex_head(b: &[u8], n: usize) -> String {
    b.iter()
        .take(n)
        .map(|x| format!("{x:02x}"))
        .collect::<String>()
}

// ---------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------

/// One watch row normalized into named canonical fields (§6a paths:
/// `count`, `robot[i].hp`, `tile.x`, `claim[3]`, …). Rows/fields a
/// channel cannot source are simply ABSENT — absence is coverage, and
/// coverage asymmetry is a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormRow {
    pub id: String,
    pub fields: Vec<(String, FieldVal)>,
}

impl NormRow {
    pub fn field(&self, name: &str) -> Option<&FieldVal> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
}

/// One normalized frame (alignment key = `frame_no`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormFrame {
    pub frame_no: u64,
    pub injection_applied: bool,
    pub rows: Vec<NormRow>,
}

impl NormFrame {
    pub fn row(&self, id: &str) -> Option<&NormRow> {
        self.rows.iter().find(|r| r.id == id)
    }
}

/// Normalization failures — always loud, always naming the row.
#[derive(Debug)]
pub enum NormalizeError {
    /// Row blob length does not fit the pinned form.
    BadLength {
        id: String,
        frame_no: u64,
        len: usize,
        want: String,
    },
    /// Channel with no normalization table yet.
    UnsupportedChannel(Channel),
    /// Scenario mismatch between the two dumps.
    ScenarioMismatch { a: String, b: String },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizeError::BadLength { id, frame_no, len, want } => write!(
                f,
                "watch {id:?} at frame {frame_no}: {len} bytes does not fit the pinned form ({want})"
            ),
            NormalizeError::UnsupportedChannel(c) => {
                write!(f, "no normalization table for channel {}", c.name())
            }
            NormalizeError::ScenarioMismatch { a, b } => {
                write!(f, "scenario mismatch: {a:?} vs {b:?}")
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

fn need(
    id: &str,
    frame_no: u64,
    bytes: &[u8],
    want: &str,
    len: usize,
) -> Result<(), NormalizeError> {
    if bytes.len() != len {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: bytes.len(),
            want: want.to_string(),
        });
    }
    Ok(())
}

fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}
fn i16le(b: &[u8]) -> i16 {
    i16::from_le_bytes([b[0], b[1]])
}
fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn i32le(b: &[u8]) -> i32 {
    i32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn u64le(b: &[u8]) -> u64 {
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Robot-record field read kinds for the raw-channel maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    /// dword i32.
    I32,
    /// word u16, zero-extended to i32.
    U16,
    /// word i16, sign-extended to i32 (canonical `armor`).
    I16,
}

/// The EXD robot-record field map (RE-EXD-MAP §8, all [verified];
/// back half pinned by the W7-followup probe, D88 — every EXW §3/§7f/§7g
/// offset coincides in EXD with the semantic twin EXACT).
/// Offsets in the 0xA8-stride record; (offset, kind) pairs.
const EXD_ROBOT_MAP: &[(&str, usize, FieldKind)] = &[
    ("pos_x", 0x00, FieldKind::I32),
    ("pos_y", 0x04, FieldKind::I32),
    ("z", 0x08, FieldKind::I32),
    ("state", 0x0C, FieldKind::U16),
    ("dir_byte", 0x0E, FieldKind::U16),
    ("facing", 0x10, FieldKind::U16),
    ("anim", 0x12, FieldKind::U16),
    ("variant", 0x18, FieldKind::U16),
    ("probe_z[0]", 0x1A, FieldKind::U16),
    ("probe_z[1]", 0x1C, FieldKind::U16),
    ("probe_z[2]", 0x1E, FieldKind::U16),
    ("probe_z[3]", 0x20, FieldKind::U16),
    ("probe_z[4]", 0x22, FieldKind::U16),
    ("probe_z[5]", 0x24, FieldKind::U16),
    ("probe_z[6]", 0x26, FieldKind::U16),
    ("probe_z[7]", 0x28, FieldKind::U16),
    ("kind", 0x2A, FieldKind::U16),
    ("hit_flash", 0x2E, FieldKind::U16),
    ("armor", 0x30, FieldKind::I16),
    ("alarm", 0x34, FieldKind::U16),
    ("stop_dist", 0x74, FieldKind::I32),
    ("hp", 0x78, FieldKind::I32),
    ("alive", 0x7C, FieldKind::I32), // presence word -> !=0 (applied below)
    ("drop_countdown", 0x80, FieldKind::I32), // D88: the phase-4/5 gate
    // word, NOT the +0x2C pod timer
    ("shield", 0x88, FieldKind::I32),
    ("shield_charges", 0x8C, FieldKind::I32),
    ("battery", 0x94, FieldKind::I32),
    ("armor_pool", 0x98, FieldKind::I32),
    ("death_flag", 0x9C, FieldKind::U16),
    ("shield_boost", 0xA0, FieldKind::I32),
    ("alarm_ctr", 0xA4, FieldKind::I32),
];

/// The EXW robot-record field map (RE-EXW-SIM §3 + §7f/§7g; the §8
/// seed-#1 EXW-front conflict is OPEN — this is the per-field-evidence
/// table). Same tuple shape as [`EXD_ROBOT_MAP`]; the offsets coincide
/// with EXD on every pinned row (§8 back-half probe, D88) except the
/// front x/y pair under arbitration.
const EXW_ROBOT_MAP: &[(&str, usize, FieldKind)] = EXD_ROBOT_MAP;

const ROBOT_STRIDE: usize = 0xA8;

/// The canonical robot record's per-field names in §6a order (the
/// `state_hash` field list; 94 bytes/record).
const CANON_ROBOT_FIELDS: &[(&str, usize)] = &[
    ("alive", 1),
    ("pos_x", 4),
    ("pos_y", 4),
    ("z", 4),
    ("state", 2),
    ("dir_byte", 2),
    ("facing", 2),
    ("anim", 2),
    ("variant", 2),
    ("probe_z", 16),
    ("stop_dist", 4),
    ("target", 9), // present u8 + tx i32 + ty i32
    ("drop_countdown", 4),
    ("hp", 4),
    ("armor", 2),
    ("hit_flash", 2),
    ("alarm", 2),
    ("kind", 2),
    ("shield", 4),
    ("shield_charges", 4),
    ("shield_boost", 4),
    ("battery", 4),
    ("armor_pool", 4),
    ("alarm_ctr", 4),
    ("death_flag", 2),
];

/// Canonical robot record byte length (94 — pinned by the W6 gate
/// fixture; a grammar change moves it loudly there first).
const CANON_ROBOT_REC: usize = 94;

// ---------------------------------------------------------------------
// The T2 bank rows (W12-S3 — DESIGN §7 S3 row; RE-EXD-MAP §5c twins)
// ---------------------------------------------------------------------

/// Weapon-anim bank slot count: the free-slot finder bound 400·0x36
/// = 0x5460 (EXD FUN_00023295; EXW 400×0x36 at 0x4c71f4).
const WEAPON_SLOTS: usize = 400;
/// Projectile bank slot count: the tick loop bound 50 (EXD
/// FUN_00022a52 `iVar8 < 0x32`; EXW 50×0x22 at 0x4cc654).
const PROJ_SLOTS: usize = 50;

/// The weapon record fields (guest offsets — identical EXW/EXD, the
/// record IS the canonical layout, no gaps): type w@+0, owner
/// d@+2, target d@+6, tick d@+0xA, draw_ctr d@+0xE, x/y/z
/// d@+0x12/+0x16/+0x1A, vx/vy/vz d@+0x1E/+0x22/+0x26, class d@+0x2A,
/// arc d@+0x2E, trail d@+0x32.
const WEAPON_REC_FIELDS: &[(u16, &str)] = &[
    (0x00, "kind"),
    (0x02, "owner"),
    (0x06, "target"),
    (0x0A, "tick"),
    (0x0E, "draw_ctr"),
    (0x12, "x"),
    (0x16, "y"),
    (0x1A, "z"),
    (0x1E, "vx"),
    (0x22, "vy"),
    (0x26, "vz"),
    (0x2A, "class"),
    (0x2E, "arc"),
    (0x32, "trail"),
];

/// The projectile record fields (guest offsets): type w@+0, xyz
/// d@+2/+6/+0xA, v d@+0xE/+0x12/+0x16 — plus the +0x1A/+0x1E TAIL
/// words (the clamp-0..7 counter and the free countdown; E models
/// no producer — its blob carries modeled ZEROS, so a live O1
/// nonzero tail surfaces as a T2 finding, never silence).
const PROJ_REC_FIELDS: &[(u16, &str)] = &[
    (0x00, "kind"),
    (0x02, "x"),
    (0x06, "y"),
    (0x0A, "z"),
    (0x0E, "vx"),
    (0x12, "vy"),
    (0x16, "vz"),
    (0x1A, "tail_ctr"),
    (0x1E, "tail_cdn"),
];

/// Parse one full weapon-anim bank into named fields. `recs` is the
/// 400·0x36 record span (E: after the u32 count; O1: the raw span).
fn weapon_bank_row(frame_no: u64, recs: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "weapon-anim-bank";
    need(
        id,
        frame_no,
        recs,
        "400*0x36 records (the full bank)",
        WEAPON_SLOTS * 0x36,
    )?;
    let mut fields = Vec::with_capacity(1 + WEAPON_SLOTS * WEAPON_REC_FIELDS.len());
    fields.push(("count".to_string(), FieldVal::Int(WEAPON_SLOTS as i128)));
    for i in 0..WEAPON_SLOTS {
        let r = &recs[i * 0x36..];
        for &(off, name) in WEAPON_REC_FIELDS {
            let v = if off == 0 {
                u16le(r) as i128
            } else {
                i32le(&r[off as usize..]) as i128
            };
            fields.push((format!("w[{i}].{name}"), FieldVal::Int(v)));
        }
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// Parse one full projectile bank into named fields (the same walk
/// for both channels — the layouts are field-exact twins).
fn projectile_bank_row(frame_no: u64, recs: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "projectile-bank";
    need(
        id,
        frame_no,
        recs,
        "50*0x22 records (the full bank)",
        PROJ_SLOTS * 0x22,
    )?;
    let mut fields = Vec::with_capacity(1 + PROJ_SLOTS * PROJ_REC_FIELDS.len());
    fields.push(("count".to_string(), FieldVal::Int(PROJ_SLOTS as i128)));
    for i in 0..PROJ_SLOTS {
        let r = &recs[i * 0x22..];
        for &(off, name) in PROJ_REC_FIELDS {
            let v = if off == 0 {
                u16le(r) as i128
            } else {
                i32le(&r[off as usize..]) as i128
            };
            fields.push((format!("p[{i}].{name}"), FieldVal::Int(v)));
        }
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The channel-E bank rows: u32 count + the records (the count is
/// the bank size, pinned to the slot total — a shorter row is a
/// truncated dump, fail loud).
fn bank_row_canonical<'a>(
    id: &str,
    frame_no: u64,
    b: &'a [u8],
    slots: usize,
    rec: usize,
) -> Result<&'a [u8], NormalizeError> {
    if b.len() < 4 {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: b.len(),
            want: format!("u32 count + {slots}*{rec:#x} records"),
        });
    }
    let n = u32le(b) as usize;
    if n != slots || b.len() != 4 + n * rec {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: b.len(),
            want: format!("count {slots} + {slots}*{rec:#x} records (the full bank)"),
        });
    }
    Ok(&b[4..])
}

// ---------------------------------------------------------------------
// The destroy-family rows (W12-S4 — DESIGN §7 S4 row; the registry
// T1 destroy rows + the T3 debris/splash rows — every row EXD-
// aliased, the D162 §5i census; the subset-form O1 arms landed
// with the debris/splash pair)
// ---------------------------------------------------------------------

/// Object-instance slot cap: the .POS 2000×0x10 array (loader cap
/// CMP 0x7d0; the guest count cell is bounded by the same walk).
const OBJECT_SLOTS: usize = 2000;
/// TRT structure cap: 250×0x20 (loader clear ECX=0x1f40).
const TRT_SLOTS: usize = 250;
/// Debris ring: 128×0x30 at 0x476fbc.
const DEBRIS_SLOTS_W12: usize = 128;
/// Splash bank: 250×0xA at 0x4e9778.
const SPLASH_SLOTS: usize = 250;

/// The object-instance record fields. CANONICAL form: `{slot u16,
/// x, y, z, id, flags u8, hp}` (23 B; the E blob, §6a). The guest
/// 0x14-stride record is the O1 raw form — its id DWORD carries
/// the same {type-low-byte, 0x40-flag-bit-8} shape, and dead
/// (id==-1) records never ride (the count cell bounds the live
/// run on the guest; E emits only live records keyed by slot).
fn object_instances_fields(out: &mut Vec<(String, FieldVal)>, rec: &[u8]) {
    let rd32 = |p: usize| i32le(&rec[p..p + 4]) as i128;
    let slot = u16le(rec) as i128;
    out.push((format!("obj[{slot}].slot"), FieldVal::Int(slot)));
    out.push((format!("obj[{slot}].x"), FieldVal::Int(rd32(2))));
    out.push((format!("obj[{slot}].y"), FieldVal::Int(rd32(6))));
    out.push((format!("obj[{slot}].z"), FieldVal::Int(rd32(10))));
    out.push((format!("obj[{slot}].id"), FieldVal::Int(rec[14] as i128)));
    out.push((
        format!("obj[{slot}].destroyed"),
        FieldVal::Int(i128::from(rec[18] & 0x40 != 0)),
    ));
    out.push((format!("obj[{slot}].hp"), FieldVal::Int(rd32(19))));
}

/// E canonical object-instances: u32 count + 23-B records.
fn object_instances_canonical(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "object-instances";
    if b.len() < 4 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "u32 count + 23*B records".into(),
        });
    }
    let n = u32le(b) as usize;
    if n > OBJECT_SLOTS || b.len() != 4 + n * 23 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("count <= {OBJECT_SLOTS} + n*23 records"),
        });
    }
    let mut fields = Vec::with_capacity(1 + n * 7);
    fields.push(("count".to_string(), FieldVal::Int(n as i128)));
    for i in 0..n {
        object_instances_fields(&mut fields, &b[4 + i * 23..]);
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// O1 raw object-instances: the guest 2000×0x14 bank with the
/// count cell FIRST (EXD *(0x119584) pointer + count 0x119554).
/// The count cell is capture plumbing — it bounds the walk, never
/// compared (the live records are the state; the trailing dead
/// id==-1 records stay out).
fn object_instances_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "object-instances";
    if b.len() < 4 || !(b.len() - 4).is_multiple_of(0x14) {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "u32 count + n*0x14 records (the guest bank)".into(),
        });
    }
    let count = u32le(b) as usize;
    let total = (b.len() - 4) / 0x14;
    if count > OBJECT_SLOTS || total > OBJECT_SLOTS || count > total {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("count <= {OBJECT_SLOTS} records"),
        });
    }
    // The walk covers the WHOLE dumped span, skipping dead id==-1
    // slots — the count cell is capture plumbing (never compared:
    // D105), and the ZONEB .POS bank carries LIVE slots past dead
    // holes (max slot 1128 over 1096 live — the S5 finding that
    // pinned this; a count-bounded walk would silently drop 32
    // live objects). Slot identity stays the watched state.
    let mut fields = Vec::with_capacity(1 + count * 7);
    let mut live = 0usize;
    for i in 0..total {
        let rec = &b[4 + i * 0x14..4 + (i + 1) * 0x14];
        if i32le(&rec[0xC..0x10]) == -1 {
            continue; // dead slot — never live state
        }
        live += 1;
        // The guest record: {x, y, z, id dword, hp} — rebuild the
        // canonical 23-B shape keyed by the SLOT INDEX.
        let mut canon = [0u8; 23];
        canon[0..2].copy_from_slice(&(i as u16).to_le_bytes());
        canon[2..14].copy_from_slice(&rec[0..12]);
        canon[14..18].copy_from_slice(&rec[0xC..0x10]);
        canon[18] = rec[0xD] & 0x40;
        canon[19..23].copy_from_slice(&rec[0x10..0x14]);
        object_instances_fields(&mut fields, &canon);
    }
    fields.insert(0, ("count".to_string(), FieldVal::Int(live as i128)));
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The TRT fields {active, hp, x, y, z} i32 ×5 — both channels the
/// same record grammar (the resolver read-set, §7j.14; the scratch
/// +0x04/+0x08 loader words are the turret-AI E-gap, out of the
/// row). E: u32 count + 20-B records; O1: the 0x20-stride guest
/// records at stride offsets {+0, +0x10, +0x14, +0x18, +0x1C}.
fn trt_fields(out: &mut Vec<(String, FieldVal)>, i: usize, v: [i128; 5]) {
    for (k, name) in ["active", "hp", "x", "y", "z"].iter().enumerate() {
        out.push((format!("trt[{i}].{name}"), FieldVal::Int(v[k])));
    }
}

fn trt_canonical(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "trt-array";
    if b.len() < 4 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "u32 count + 20*B records".into(),
        });
    }
    let n = u32le(b) as usize;
    if n > TRT_SLOTS || b.len() != 4 + n * 20 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("count <= {TRT_SLOTS} + n*20 records"),
        });
    }
    let mut fields = Vec::with_capacity(1 + n * 5);
    fields.push(("count".to_string(), FieldVal::Int(n as i128)));
    for i in 0..n {
        let rec = &b[4 + i * 20..];
        trt_fields(
            &mut fields,
            i,
            [
                i32le(rec) as i128,
                i32le(&rec[4..]) as i128,
                i32le(&rec[8..]) as i128,
                i32le(&rec[12..]) as i128,
                i32le(&rec[16..]) as i128,
            ],
        );
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

fn trt_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "trt-array";
    if b.len() < 4 || !(b.len() - 4).is_multiple_of(0x20) {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "u32 count + n*0x20 records (the guest bank)".into(),
        });
    }
    let n = u32le(b) as usize;
    let total = (b.len() - 4) / 0x20;
    if n > TRT_SLOTS || total > TRT_SLOTS || n > total {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("count <= {TRT_SLOTS} records"),
        });
    }
    let mut fields = Vec::with_capacity(1 + n * 5);
    fields.push(("count".to_string(), FieldVal::Int(n as i128)));
    for i in 0..n {
        let rec = &b[4 + i * 0x20..];
        trt_fields(
            &mut fields,
            i,
            [
                i32le(rec) as i128,
                i32le(&rec[0x10..]) as i128,
                i32le(&rec[0x14..]) as i128,
                i32le(&rec[0x18..]) as i128,
                i32le(&rec[0x1C..]) as i128,
            ],
        );
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The bare u16 grid field walk (tile-word-grid /
/// platform-strength): both channels dump the same w·h·2 span.
fn grid_fields(id: &str, out: &mut Vec<(String, FieldVal)>, b: &[u8]) {
    for (t, w) in b.chunks_exact(2).enumerate() {
        out.push((format!("tile[{t}]"), FieldVal::Int(u16le(w) as i128)));
    }
    let _ = id;
}

/// The typedb-mirror-rows walk. CANONICAL: u32 changed-count +
/// {tile u16, 8×(word u16, seen u8)} 26-B records (compact-active).
/// RAW (O1): the full 0x1E-stride w·h rows — the SAME nonzero-tile
/// filter canonicalizes it (identical content ⇒ identical rows;
/// the filter is the D104 last-nonzero-prefix contract applied at
/// tile granularity: a tile is active iff any of its 8 z-words or
/// seen bytes is nonzero). The two layouts differ (compact words
/// at 2+3z/seen at 4+3z; raw words at 2z/seen at 0x10+z), so the
/// field walk takes the extracted per-z pairs, never a slice.
fn mirror_fields(out: &mut Vec<(String, FieldVal)>, tile: usize, zw: [u16; 8], zs: [u8; 8]) {
    out.push((format!("tile[{tile}].tile"), FieldVal::Int(tile as i128)));
    for z in 0..8 {
        out.push((
            format!("tile[{tile}].z{z}.word"),
            FieldVal::Int(i128::from(zw[z])),
        ));
        out.push((
            format!("tile[{tile}].z{z}.seen"),
            FieldVal::Int(i128::from(zs[z])),
        ));
    }
}

/// Extract the per-z (word, seen) pairs from one RAW 0x1E-stride
/// row (words +0x00..+0x0F, seen +0x10..+0x17).
fn mirror_row_pairs(row: &[u8]) -> ([u16; 8], [u8; 8]) {
    let mut zw = [0u16; 8];
    let mut zs = [0u8; 8];
    for z in 0..8 {
        zw[z] = u16le(&row[2 * z..]);
        zs[z] = row[0x10 + z];
    }
    (zw, zs)
}

fn mirror_row_active(row: &[u8]) -> bool {
    let (zw, zs) = mirror_row_pairs(row);
    (0..8).any(|z| zw[z] != 0 || zs[z] != 0)
}

fn mirror_canonical(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "typedb-mirror-rows";
    if b.len() < 4 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "u32 count + n*26 records".into(),
        });
    }
    let n = u32le(b) as usize;
    if b.len() != 4 + n * 26 {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("4 + {n}*26 (the compact-active form)"),
        });
    }
    let mut fields = Vec::with_capacity(1 + n * 17);
    fields.push(("count".to_string(), FieldVal::Int(n as i128)));
    for i in 0..n {
        let rec = &b[4 + i * 26..];
        let tile = u16le(rec) as usize;
        // The compact tail: 8×(word u16 @ 2+3z, seen u8 @ 4+3z).
        let mut zw = [0u16; 8];
        let mut zs = [0u8; 8];
        for z in 0..8 {
            zw[z] = u16le(&rec[2 + 3 * z..]);
            zs[z] = rec[4 + 3 * z];
        }
        mirror_fields(&mut fields, tile, zw, zs);
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

fn mirror_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "typedb-mirror-rows";
    if !b.len().is_multiple_of(0x1E) {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "w*h*0x1E rows (the full guest grid)".into(),
        });
    }
    let tiles = b.len() / 0x1E;
    let mut fields = Vec::new();
    let mut active = 0usize;
    for t in 0..tiles {
        let row = &b[t * 0x1E..(t + 1) * 0x1E];
        if mirror_row_active(row) {
            active += 1;
            let (zw, zs) = mirror_row_pairs(row);
            mirror_fields(&mut fields, t, zw, zs);
        }
    }
    fields.insert(0, ("count".to_string(), FieldVal::Int(active as i128)));
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The debris-ring row (D162: EXD alias 0x93064 verified; the T2
/// full-bank row). Canonical: u32 128 + 42-B records; the O1 side
/// is the bare 128×0x30 guest span (`debris_o1`).
fn debris_canonical(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "debris-stager";
    let recs = bank_row_canonical(id, frame_no, b, DEBRIS_SLOTS_W12, 42)?;
    let mut fields = Vec::with_capacity(1 + DEBRIS_SLOTS_W12 * 4);
    fields.push(("count".to_string(), FieldVal::Int(DEBRIS_SLOTS_W12 as i128)));
    for i in 0..DEBRIS_SLOTS_W12 {
        let rec = &recs[i * 42..];
        let rd32 = |p: usize| i32le(&rec[p..p + 4]) as i128;
        fields.push((format!("d[{i}].active"), FieldVal::Int(rec[0] as i128)));
        fields.push((format!("d[{i}].kind"), FieldVal::Int(rd32(25))));
        fields.push((format!("d[{i}].delay"), FieldVal::Int(rd32(33))));
        fields.push((format!("d[{i}].seq"), FieldVal::Int(rd32(21))));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The splash-bank row (D162: EXD alias 0x107774 verified).
/// Canonical: u32 250 + 10-B records {x, y, z, delay, age} — the
/// guest 0xA stride exactly (the O1 side is the bare span,
/// `splash_o1`).
fn splash_canonical(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "splash-records";
    let recs = bank_row_canonical(id, frame_no, b, SPLASH_SLOTS, 10)?;
    let mut fields = Vec::with_capacity(1 + SPLASH_SLOTS * 5);
    fields.push(("count".to_string(), FieldVal::Int(SPLASH_SLOTS as i128)));
    for i in 0..SPLASH_SLOTS {
        let rec = &recs[i * 10..];
        let rd16 = |p: usize| u16le(&rec[p..]) as i128;
        fields.push((format!("s[{i}].x"), FieldVal::Int(rd16(0))));
        fields.push((format!("s[{i}].y"), FieldVal::Int(rd16(2))));
        fields.push((format!("s[{i}].z"), FieldVal::Int(rd16(4))));
        fields.push((format!("s[{i}].delay"), FieldVal::Int(rd16(6))));
        fields.push((format!("s[{i}].age"), FieldVal::Int(rd16(8))));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

// ---------------------------------------------------------------------
// The D162 subset-form extraction arms — the four rows whose E
// canonical record is a SUBSET of the guest record: the normalizer
// walks the GUEST full span and projects E's modeled fields at the
// guest offsets (the D87 field-map class; every canonical leaf
// sources from the guest — zero field gaps by construction).
// ---------------------------------------------------------------------

/// The debris ring's guest-span walk: the bare 128×0x30 bank at
/// 0x476fbc (EXW) / 0x93064 (EXD) — the fixed full bank, slot
/// identity watched. The projection reads the four canonical
/// leaves from the §7j.11 record: active u8@+0x00, kind d@+0x1C,
/// delay d@+0x24, seq d@+0x18. The +0x18 is the DUAL field the
/// ENGINE splits (§7j.44: E keeps the LRU-eviction role as its
/// global staging counter `debris_seq`, the walk-cursor role as
/// `anim`); the projection carries the guest's raw +0x18 — its
/// value diverges from E's counter by construction and stays
/// silent only because the row is T3 (never bit-compared). If the
/// row is ever re-tiered, this offset pair is the known encoding
/// difference (DESIGN §6a, the subset-arm table).
fn debris_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "debris-stager";
    need(
        id,
        frame_no,
        b,
        "128*0x30 records (the full guest ring)",
        DEBRIS_SLOTS_W12 * 0x30,
    )?;
    let mut fields = Vec::with_capacity(1 + DEBRIS_SLOTS_W12 * 4);
    fields.push(("count".to_string(), FieldVal::Int(DEBRIS_SLOTS_W12 as i128)));
    for i in 0..DEBRIS_SLOTS_W12 {
        let rec = &b[i * 0x30..];
        fields.push((format!("d[{i}].active"), FieldVal::Int(rec[0] as i128)));
        fields.push((
            format!("d[{i}].kind"),
            FieldVal::Int(i32le(&rec[0x1C..]) as i128),
        ));
        fields.push((
            format!("d[{i}].delay"),
            FieldVal::Int(i32le(&rec[0x24..]) as i128),
        ));
        fields.push((
            format!("d[{i}].seq"),
            FieldVal::Int(i32le(&rec[0x18..]) as i128),
        ));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The splash bank's guest-span walk: the bare 250×0xA bank at
/// 0x4e9778 (EXW) / 0x107774 (EXD) — the guest stride IS the
/// canonical record, so the projection is identity and only the
/// count is synthesized from the fixed bank.
fn splash_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "splash-records";
    need(
        id,
        frame_no,
        b,
        "250*0xA records (the full guest bank)",
        SPLASH_SLOTS * 10,
    )?;
    let mut fields = Vec::with_capacity(1 + SPLASH_SLOTS * 5);
    fields.push(("count".to_string(), FieldVal::Int(SPLASH_SLOTS as i128)));
    for i in 0..SPLASH_SLOTS {
        let rec = &b[i * 10..];
        let rd16 = |p: usize| u16le(&rec[p..]) as i128;
        fields.push((format!("s[{i}].x"), FieldVal::Int(rd16(0))));
        fields.push((format!("s[{i}].y"), FieldVal::Int(rd16(2))));
        fields.push((format!("s[{i}].z"), FieldVal::Int(rd16(4))));
        fields.push((format!("s[{i}].delay"), FieldVal::Int(rd16(6))));
        fields.push((format!("s[{i}].age"), FieldVal::Int(rd16(8))));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The critter bank's guest-span walk: the bare count×0x7E bank at
/// 0x4cff98 (EXW, count 0x46cc2c) / 0x10e81c (EXD, count
/// 0x1194dc) — the dbx-plan `$critter_count*0x7E` span, no prefix.
/// The projection maps E's 23 modeled leaves (§7j.17/§7j.42 + the
/// critter.rs field docs): kind w@+0x00, species w@+0x02, attacker
/// i16@+0x04, hp i16@+0x06, mode w@+0x0C, anim w@+0x0E, heading
/// d@+0x10, impact_x d@+0x1C, impact_y d@+0x20, presence
/// w@+0x24, target d@+0x2A/+0x2E/+0x32, xyz d@+0x36/+0x3A/+0x3E,
/// home d@+0x42/+0x46, death_ctr d@+0x52, countdown w@+0x56
/// (zero-extended into the canonical i32), facing w@+0x72,
/// target_robot i16@+0x7A, fuse w@+0x7C.
fn critter_bank_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "critter-bank";
    if b.is_empty() || !b.len().is_multiple_of(0x7E) {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: "count*0x7E records (the guest critter bank)".into(),
        });
    }
    // Bank capacity 0xAC44 / 0x7E = 350 slots (§7j.17: the EXW
    // arena at 0x4cff98; the count cell bounds the plan span).
    const CRITTER_CAP: usize = 0xAC44 / 0x7E;
    let count = b.len() / 0x7E;
    if count > CRITTER_CAP {
        return Err(NormalizeError::BadLength {
            id: id.into(),
            frame_no,
            len: b.len(),
            want: format!("<= {CRITTER_CAP}*0x7E records (the bank capacity)"),
        });
    }
    let mut fields = Vec::with_capacity(1 + count * 23);
    fields.push(("count".to_string(), FieldVal::Int(count as i128)));
    let w16 = |rec: &[u8], p: usize| u16le(&rec[p..]) as i128;
    let s16 = |rec: &[u8], p: usize| u16le(&rec[p..]) as i16 as i128;
    let d32 = |rec: &[u8], p: usize| i32le(&rec[p..]) as i128;
    for i in 0..count {
        let rec = &b[i * 0x7E..];
        fields.push((format!("critter[{i}].kind"), FieldVal::Int(w16(rec, 0x00))));
        fields.push((
            format!("critter[{i}].species"),
            FieldVal::Int(w16(rec, 0x02)),
        ));
        fields.push((
            format!("critter[{i}].attacker"),
            FieldVal::Int(s16(rec, 0x04)),
        ));
        fields.push((format!("critter[{i}].hp"), FieldVal::Int(s16(rec, 0x06))));
        fields.push((format!("critter[{i}].mode"), FieldVal::Int(w16(rec, 0x0C))));
        fields.push((format!("critter[{i}].anim"), FieldVal::Int(w16(rec, 0x0E))));
        fields.push((
            format!("critter[{i}].heading"),
            FieldVal::Int(d32(rec, 0x10)),
        ));
        fields.push((
            format!("critter[{i}].presence"),
            FieldVal::Int(w16(rec, 0x24)),
        ));
        fields.push((
            format!("critter[{i}].target_x"),
            FieldVal::Int(d32(rec, 0x2A)),
        ));
        fields.push((
            format!("critter[{i}].target_y"),
            FieldVal::Int(d32(rec, 0x2E)),
        ));
        fields.push((
            format!("critter[{i}].target_z"),
            FieldVal::Int(d32(rec, 0x32)),
        ));
        fields.push((
            format!("critter[{i}].impact_x"),
            FieldVal::Int(d32(rec, 0x1C)),
        ));
        fields.push((
            format!("critter[{i}].impact_y"),
            FieldVal::Int(d32(rec, 0x20)),
        ));
        fields.push((format!("critter[{i}].x"), FieldVal::Int(d32(rec, 0x36))));
        fields.push((format!("critter[{i}].y"), FieldVal::Int(d32(rec, 0x3A))));
        fields.push((format!("critter[{i}].z"), FieldVal::Int(d32(rec, 0x3E))));
        fields.push((
            format!("critter[{i}].home_x"),
            FieldVal::Int(d32(rec, 0x42)),
        ));
        fields.push((
            format!("critter[{i}].home_y"),
            FieldVal::Int(d32(rec, 0x46)),
        ));
        fields.push((
            format!("critter[{i}].countdown"),
            FieldVal::Int(w16(rec, 0x56)),
        ));
        fields.push((
            format!("critter[{i}].death_ctr"),
            FieldVal::Int(d32(rec, 0x52)),
        ));
        fields.push((
            format!("critter[{i}].target_robot"),
            FieldVal::Int(s16(rec, 0x7A)),
        ));
        fields.push((format!("critter[{i}].fuse"), FieldVal::Int(w16(rec, 0x7C))));
        fields.push((
            format!("critter[{i}].facing"),
            FieldVal::Int(w16(rec, 0x72)),
        ));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// The effect-row bank's guest-span walk: the bare 80×0x20 LRU
/// bank at 0x4cec38 (EXW) / 0x9d534 (EXD) — the fixed full bank
/// (§7j.24/5: always-evict, every row carries state). The
/// projection maps E's 8 modeled leaves: age w@+0x00, x d@+0x02,
/// y d@+0x06, z d@+0x0A, cos d@+0x0E, sin d@+0x12, ttl d@+0x16,
/// id w@+0x1A.
fn effect_rows_o1(frame_no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "effect-rows";
    const ROWS: usize = 80;
    const STRIDE: usize = 0x20;
    need(
        id,
        frame_no,
        b,
        "80*0x20 records (the full guest LRU bank)",
        ROWS * STRIDE,
    )?;
    let mut fields = Vec::with_capacity(1 + ROWS * 8);
    fields.push(("count".to_string(), FieldVal::Int(ROWS as i128)));
    let w16 = |p: usize| u16le(&b[p..]) as i128;
    let d32 = |p: usize| i32le(&b[p..]) as i128;
    for i in 0..ROWS {
        let o = i * STRIDE;
        fields.push((format!("row[{i}].age"), FieldVal::Int(w16(o))));
        fields.push((format!("row[{i}].id"), FieldVal::Int(w16(o + 0x1A))));
        fields.push((format!("row[{i}].x"), FieldVal::Int(d32(o + 0x02))));
        fields.push((format!("row[{i}].y"), FieldVal::Int(d32(o + 0x06))));
        fields.push((format!("row[{i}].z"), FieldVal::Int(d32(o + 0x0A))));
        fields.push((format!("row[{i}].cos"), FieldVal::Int(d32(o + 0x0E))));
        fields.push((format!("row[{i}].sin"), FieldVal::Int(d32(o + 0x12))));
        fields.push((format!("row[{i}].ttl"), FieldVal::Int(d32(o + 0x16))));
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

fn robot_row_from_map(
    id: &str,
    frame_no: u64,
    bytes: &[u8],
    map: &[(&'static str, usize, FieldKind)],
) -> Result<NormRow, NormalizeError> {
    if !bytes.len().is_multiple_of(ROBOT_STRIDE) {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: bytes.len(),
            want: format!("n*{ROBOT_STRIDE:#x} (count*stride records)"),
        });
    }
    let n = bytes.len() / ROBOT_STRIDE;
    let mut fields = vec![("count".to_string(), FieldVal::Int(n as i128))];
    for i in 0..n {
        let rec = &bytes[i * ROBOT_STRIDE..(i + 1) * ROBOT_STRIDE];
        for (name, off, kind) in map {
            let path = format!("robot[{i}].{name}");
            let v = if *name == "alive" {
                FieldVal::Int(i32::from(i32le(&rec[*off..*off + 4]) != 0) as i128)
            } else {
                match kind {
                    FieldKind::U16 => FieldVal::Int(u16le(&rec[*off..*off + 2]) as i128),
                    FieldKind::I16 => FieldVal::Int(i16le(&rec[*off..*off + 2]) as i128),
                    FieldKind::I32 => FieldVal::Int(i32le(&rec[*off..*off + 4]) as i128),
                }
            };
            fields.push((path, v));
        }
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// Channel-E robot-bank parse: the §6a canonical record grammar
/// (mirror of `robot_bank_blob` in the emitter — the W6 gate pins the
/// bytes, this pins the read side).
fn robot_row_canonical(frame_no: u64, bytes: &[u8]) -> Result<NormRow, NormalizeError> {
    let id = "robot-bank";
    if bytes.len() < 4 {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: bytes.len(),
            want: "u32 count + n*94 canonical records".into(),
        });
    }
    let n = u32le(bytes) as usize;
    if bytes.len() != 4 + n * CANON_ROBOT_REC {
        return Err(NormalizeError::BadLength {
            id: id.to_string(),
            frame_no,
            len: bytes.len(),
            want: format!("4 + {n}*{CANON_ROBOT_REC}"),
        });
    }
    let mut fields = vec![("count".to_string(), FieldVal::Int(n as i128))];
    for i in 0..n {
        let rec = &bytes[4 + i * CANON_ROBOT_REC..4 + (i + 1) * CANON_ROBOT_REC];
        let mut p = 0usize;
        for (name, len) in CANON_ROBOT_FIELDS {
            match *name {
                "alive" => {
                    fields.push((format!("robot[{i}].alive"), FieldVal::Int(rec[p] as i128)));
                }
                "probe_z" => {
                    for k in 0..8 {
                        fields.push((
                            format!("robot[{i}].probe_z[{k}]"),
                            FieldVal::Int(u16le(&rec[p + 2 * k..]) as i128),
                        ));
                    }
                }
                "target" => {
                    fields.push((
                        format!("robot[{i}].target_present"),
                        FieldVal::Int(rec[p] as i128),
                    ));
                    fields.push((
                        format!("robot[{i}].target_x"),
                        FieldVal::Int(i32le(&rec[p + 1..]) as i128),
                    ));
                    fields.push((
                        format!("robot[{i}].target_y"),
                        FieldVal::Int(i32le(&rec[p + 5..]) as i128),
                    ));
                }
                "armor" => {
                    fields.push((
                        format!("robot[{i}].armor"),
                        FieldVal::Int(i16le(&rec[p..]) as i128),
                    ));
                }
                other => {
                    let v = if *len == 2 {
                        FieldVal::Int(u16le(&rec[p..]) as i128)
                    } else {
                        FieldVal::Int(i32le(&rec[p..]) as i128)
                    };
                    fields.push((format!("robot[{i}].{other}"), v));
                }
            }
            p += len;
        }
    }
    Ok(NormRow {
        id: id.to_string(),
        fields,
    })
}

/// Normalize one frame's rows for one channel.
pub fn normalize_frame(
    frame: &FrameRecord,
    channel: Channel,
    reg: &[Watch],
) -> Result<Vec<NormRow>, NormalizeError> {
    let no = frame.frame_no;
    let _ = reg; // membership is enforced by encode/stitch; the tier
                 // lookup happens at compare time.
    let mut rows = Vec::new();
    match channel {
        Channel::Engine => {
            for w in &frame.watches {
                rows.push(normalize_engine_row(&w.id, no, &w.bytes)?);
            }
        }
        Channel::O1ExdDosboxX | Channel::O2ExwWine | Channel::O3Street => {
            // Cross-row splice (D90): the raw move-target span is
            // indexed by ABSOLUTE robot id and bounded by the SAME
            // frame's robot-bank count (RE-EXD-MAP §5) — parse it
            // first, then fold it into the robot-bank row. The span
            // carries NO standalone canonical row on the raw side
            // (the E §6a move-target-words row stays an E-only
            // coverage note in cross-channel reports).
            let span = match frame.watches.iter().find(|w| w.id == "move-target-words") {
                Some(w) => Some(parse_move_target_span(no, &w.bytes)?),
                None => None,
            };
            let mut saw_robot_bank = false;
            for w in &frame.watches {
                if w.id == "move-target-words" {
                    continue; // consumed by the splice below
                }
                let mut row = match channel {
                    Channel::O1ExdDosboxX => normalize_o1_row(&w.id, no, &w.bytes)?,
                    Channel::O2ExwWine => normalize_o2_row(&w.id, no, &w.bytes)?,
                    // The O3 field map (D142 §5, W10-impl-b): the
                    // reconstruction rebuilds EXW state — same cells,
                    // same layouts — so the O3 raw rows are O2-form
                    // and normalize through the O2 table verbatim.
                    // The §6 seam set is a COMPARE-time classification
                    // (Class::O3Seam), never a normalization
                    // difference: seam rows normalize identically so a
                    // clean capture still compares clean.
                    Channel::O3Street => normalize_o3_row(&w.id, no, &w.bytes)?,
                    // normalize_frame never runs for the Engine
                    // channel's rows through this arm.
                    Channel::Engine => unreachable!("guest-channel arm"),
                };
                if w.id == "robot-bank" {
                    saw_robot_bank = true;
                    if let Some(span) = &span {
                        splice_move_targets(no, &mut row, span)?;
                    }
                }
                rows.push(row);
            }
            if span.is_some() && !saw_robot_bank {
                // The plan always pairs the span with the bank row; a
                // lone span has no bound — fail loud, never guess.
                return Err(NormalizeError::BadLength {
                    id: "move-target-words".into(),
                    frame_no: no,
                    len: 0x60,
                    want: "the robot-bank row in the same frame (the span's robot bound)".into(),
                });
            }
        }
    }
    Ok(rows)
}

// ---------------------------------------------------------------------
// The move-target splice (D90 — RE-EXD-MAP §5/§8)
// ---------------------------------------------------------------------

/// Move-target span slot count: the CAP cell bound (≤ 12 robots; the
/// fixed 0x60-B EXD span at 0xf75ec covers x[12] u32 + y[12] u32).
const MOVE_TARGET_SLOTS: usize = 12;

/// One parsed span slot in §6a canonical form: (present, tx, ty). Both
/// sides are Q5 (`tile<<5`) — raw i32 comparison, no shift.
type MoveTarget = (i128, i128, i128);

/// Parse the raw O1/O2 move-target span: x[i] u32 @+4i, y[i] u32
/// @+0x30+4i; x == −1 = no target (spawn −1-fill / arrive-clear), and
/// an absent target canonicalizes to (0, 0, 0) — exactly the E §6a
/// row's encoding of `Robot::target: None`.
fn parse_move_target_span(frame_no: u64, b: &[u8]) -> Result<Vec<MoveTarget>, NormalizeError> {
    need(
        "move-target-words",
        frame_no,
        b,
        "0x60 (x[12] u32 + y[12] u32 — the D90 span)",
        0x60,
    )?;
    let mut out = Vec::with_capacity(MOVE_TARGET_SLOTS);
    for i in 0..MOVE_TARGET_SLOTS {
        let x = i32le(&b[4 * i..]);
        let y = i32le(&b[0x30 + 4 * i..]);
        if x == -1 {
            out.push((0, 0, 0));
        } else {
            out.push((1, x as i128, y as i128));
        }
    }
    Ok(out)
}

/// Fold the parsed span into a robot-bank row: `robot[i].target_*` for
/// i < count (absolute-id indexing), inserted after each robot's
/// `stop_dist` to mirror the CANON_ROBOT_FIELDS order.
fn splice_move_targets(
    frame_no: u64,
    row: &mut NormRow,
    span: &[MoveTarget],
) -> Result<(), NormalizeError> {
    let count = match row.field("count") {
        Some(FieldVal::Int(n)) => *n as usize,
        _ => {
            return Err(NormalizeError::BadLength {
                id: row.id.clone(),
                frame_no,
                len: row.fields.len(),
                want: "a count field (robot-bank row)".into(),
            });
        }
    };
    if count > MOVE_TARGET_SLOTS {
        return Err(NormalizeError::BadLength {
            id: "robot-bank".into(),
            frame_no,
            len: count,
            want: format!(
                "≤ {MOVE_TARGET_SLOTS} robots (the move-target span bound, RE-EXD-MAP sec 5)"
            ),
        });
    }
    for (i, &(present, tx, ty)) in span.iter().enumerate().take(count) {
        let trio = [
            (format!("robot[{i}].target_present"), FieldVal::Int(present)),
            (format!("robot[{i}].target_x"), FieldVal::Int(tx)),
            (format!("robot[{i}].target_y"), FieldVal::Int(ty)),
        ];
        let at = row
            .fields
            .iter()
            .position(|(n, _)| n == &format!("robot[{i}].stop_dist"))
            .ok_or_else(|| NormalizeError::BadLength {
                id: row.id.clone(),
                frame_no,
                len: row.fields.len(),
                want: format!("robot[{i}].stop_dist (robot-bank map order)"),
            })?;
        for (k, f) in trio.into_iter().enumerate() {
            row.fields.insert(at + 1 + k, f);
        }
    }
    Ok(())
}

fn normalize_engine_row(id: &str, no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let int = |name: &str, v: i128| (name.to_string(), FieldVal::Int(v));
    let row = |fields: Vec<(String, FieldVal)>| NormRow {
        id: id.to_string(),
        fields,
    };
    match id {
        // u32 scalar rows.
        "frame-counter" | "score" | "money" | "difficulty" | "zone" | "mission" | "mode"
        | "linear-mission-m" | "selection-triple" | "blink-cursor" | "sfx-master-gate" => {
            need(id, no, b, "u32", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "rng-state-a" | "rng-state-b" => {
            need(id, no, b, "u64", 8)?;
            Ok(row(vec![int("value", u64le(b) as i128)]))
        }
        "robot-bank" => robot_row_canonical(no, b),
        "weapon-anim-bank" => {
            weapon_bank_row(no, bank_row_canonical(id, no, b, WEAPON_SLOTS, 0x36)?)
        }
        "projectile-bank" => {
            projectile_bank_row(no, bank_row_canonical(id, no, b, PROJ_SLOTS, 0x22)?)
        }
        // The destroy-family rows (W12-S4).
        "object-instances" => object_instances_canonical(no, b),
        "trt-array" => trt_canonical(no, b),
        "typedb-mirror-rows" => mirror_canonical(no, b),
        "debris-stager" => debris_canonical(no, b),
        "splash-records" => splash_canonical(no, b),
        // The extraction dropship row (W12-S6): the 0x1C craft
        // record {active, phase, x, y, alt, group, dwell} exactly
        // as the emitter lays it out (§7j.40/6). D162 pinned the
        // EXD twin 0x1081c4 (§5i) and the O1 plan emits the row,
        // but the O1 normalizer arm is NOT landed (the full-record
        // identity form is its own follow-up): the raw side still
        // falls to the passthrough, so the row reports E-only
        // coverage findings in cross-channel reports today.
        "dropship-frame" => {
            need(
                id,
                no,
                b,
                "craft {active,phase,x,y,alt,group,dwell} i32*7",
                28,
            )?;
            Ok(row(vec![
                int("active", u32le(b) as i128),
                int("phase", i32le(&b[4..]) as i128),
                int("x", i32le(&b[8..]) as i128),
                int("y", i32le(&b[12..]) as i128),
                int("alt", i32le(&b[16..]) as i128),
                int("group", i32le(&b[20..]) as i128),
                int("dwell", i32le(&b[24..]) as i128),
            ]))
        }
        // The critter-bank row (W12-S8): D162 pinned the EXD alias
        // 0x10e81c (count cell 0x1194dc, §5i) and the subset-form
        // O1 arm landed with it. Canonical = u32 count + count ×
        // the modeled 0x7E-record subset exactly as the emitter
        // lays it out (74 B/record; §7j.42/1).
        "critter-bank" => {
            if b.len() < 4 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "u32 count + count*74 (the critter record)".into(),
                });
            }
            let count = u32le(b) as usize;
            if b.len() != 4 + count * 74 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: format!("4 + {count}*74 (the critter record)"),
                });
            }
            let mut fields = Vec::with_capacity(2 + count * 18);
            fields.push(int("count", count as i128));
            let u16at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as i128;
            let i32at = |o: usize| i32le(&b[o..]) as i128;
            for i in 0..count {
                let o = 4 + i * 74;
                fields.push(int(&format!("critter[{i}].kind"), u16at(o)));
                fields.push(int(&format!("critter[{i}].species"), u16at(o + 2)));
                fields.push(int(
                    &format!("critter[{i}].attacker"),
                    u16at(o + 4) as i16 as i128,
                ));
                fields.push(int(
                    &format!("critter[{i}].hp"),
                    u16at(o + 6) as i16 as i128,
                ));
                fields.push(int(&format!("critter[{i}].mode"), u16at(o + 8)));
                fields.push(int(&format!("critter[{i}].anim"), u16at(o + 10)));
                fields.push(int(&format!("critter[{i}].heading"), i32at(o + 12)));
                fields.push(int(&format!("critter[{i}].presence"), i32at(o + 16)));
                fields.push(int(&format!("critter[{i}].target_x"), i32at(o + 20)));
                fields.push(int(&format!("critter[{i}].target_y"), i32at(o + 24)));
                fields.push(int(&format!("critter[{i}].target_z"), i32at(o + 28)));
                fields.push(int(&format!("critter[{i}].impact_x"), i32at(o + 32)));
                fields.push(int(&format!("critter[{i}].impact_y"), i32at(o + 36)));
                fields.push(int(&format!("critter[{i}].x"), i32at(o + 40)));
                fields.push(int(&format!("critter[{i}].y"), i32at(o + 44)));
                fields.push(int(&format!("critter[{i}].z"), i32at(o + 48)));
                fields.push(int(&format!("critter[{i}].home_x"), i32at(o + 52)));
                fields.push(int(&format!("critter[{i}].home_y"), i32at(o + 56)));
                fields.push(int(&format!("critter[{i}].countdown"), i32at(o + 60)));
                fields.push(int(&format!("critter[{i}].death_ctr"), i32at(o + 64)));
                fields.push(int(
                    &format!("critter[{i}].target_robot"),
                    u16at(o + 68) as i16 as i128,
                ));
                fields.push(int(&format!("critter[{i}].fuse"), u16at(o + 70)));
                fields.push(int(&format!("critter[{i}].facing"), u16at(o + 72)));
            }
            Ok(row(fields))
        }
        // The effect-rows row (W12-S8): D162 pinned the EXD alias
        // 0x9d534 (§5i) and the subset-form O1 arm landed with it.
        // Canonical = u32 count + count × 28 B {age u16, id u16, x,
        // y, z, cos, sin, ttl} (the E-modeled subset of the
        // 0x20-stride guest row).
        "effect-rows" => {
            const STRIDE: usize = 28;
            if b.len() < 4 || !(b.len() - 4).is_multiple_of(STRIDE) {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "u32 count + count*28 (the effect row)".into(),
                });
            }
            let count = u32le(b) as usize;
            if b.len() != 4 + count * STRIDE {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: format!("4 + {count}*28 (the effect row)"),
                });
            }
            let mut fields = Vec::with_capacity(2 + count * 8);
            fields.push(int("count", count as i128));
            let u16at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as i128;
            let i32at = |o: usize| i32le(&b[o..]) as i128;
            for i in 0..count {
                let o = 4 + i * STRIDE;
                fields.push(int(&format!("row[{i}].age"), u16at(o)));
                fields.push(int(&format!("row[{i}].id"), u16at(o + 2)));
                fields.push(int(&format!("row[{i}].x"), i32at(o + 4)));
                fields.push(int(&format!("row[{i}].y"), i32at(o + 8)));
                fields.push(int(&format!("row[{i}].z"), i32at(o + 12)));
                fields.push(int(&format!("row[{i}].cos"), i32at(o + 16)));
                fields.push(int(&format!("row[{i}].sin"), i32at(o + 20)));
                fields.push(int(&format!("row[{i}].ttl"), i32at(o + 24)));
            }
            Ok(row(fields))
        }
        "tile-word-grid" | "platform-strength" => {
            // Both channels dump the same w·h·2 span — one shared
            // field walk (a length mismatch is a STRUCTURAL
            // finding, fail loud here so the report names it).
            if !b.len().is_multiple_of(2) {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "w*h*2 (the tile-major u16 span)".into(),
                });
            }
            let mut fields = Vec::with_capacity(b.len() / 2);
            grid_fields(id, &mut fields, b);
            Ok(row(fields))
        }
        "move-target-words" => {
            if b.len() < 4 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "u32 count + n*9".into(),
                });
            }
            let n = u32le(b) as usize;
            if b.len() != 4 + n * 9 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: format!("4 + {n}*9"),
                });
            }
            let mut fields = vec![int("count", n as i128)];
            for i in 0..n {
                let rec = &b[4 + i * 9..];
                fields.push(int(&format!("robot[{i}].present"), rec[0] as i128));
                fields.push(int(&format!("robot[{i}].tx"), i32le(&rec[1..]) as i128));
                fields.push(int(&format!("robot[{i}].ty"), i32le(&rec[4..]) as i128));
            }
            Ok(row(fields))
        }
        "beacon-family" => {
            need(id, no, b, "flag u32 + timer u32 + tile i32*3", 20)?;
            Ok(row(vec![
                int("flag", u32le(b) as i128),
                int("timer", u32le(&b[4..]) as i128),
                int("tile.x", i32le(&b[8..]) as i128),
                int("tile.y", i32le(&b[12..]) as i128),
                int("tile.z", i32le(&b[16..]) as i128),
            ]))
        }
        "order-target" => {
            need(id, no, b, "i32*3", 12)?;
            Ok(row(vec![
                int("x", i32le(b) as i128),
                int("y", i32le(&b[4..]) as i128),
                int("z", i32le(&b[8..]) as i128),
            ]))
        }
        "spread-claims" => {
            need(id, no, b, "u16*12", 24)?;
            let mut fields = Vec::new();
            for k in 0..12 {
                fields.push(int(&format!("claim[{k}]"), u16le(&b[2 * k..]) as i128));
            }
            Ok(row(fields))
        }
        // The no-extract latch (D133/D136): E's canonical form =
        // u32 count + count u32 slot words (all zero on SP — the
        // claim path is MP-lobby-only). The O1/O2 raw side is the
        // BARE count-driven span (parsed in `normalize_o1_row`).
        "no-extract-latch" => {
            if b.len() < 4 || !(b.len() - 4).is_multiple_of(4) {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "u32 count + count*4 (the no-extract latch)".into(),
                });
            }
            let n = u32le(b) as usize;
            if b.len() != 4 + n * 4 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: format!("u32 count + count*4 (count says {n})"),
                });
            }
            let mut fields = Vec::with_capacity(1 + n);
            fields.push(int("count", n as i128));
            for i in 0..n {
                fields.push(int(&format!("slots[{i}]"), u32le(&b[4 + i * 4..]) as i128));
            }
            Ok(row(fields))
        }
        "per-player-selected" => {
            need(id, no, b, "4 * {x,y,z} i32", 48)?;
            let mut fields = Vec::new();
            for p in 0..4 {
                for (k, n) in ["x", "y", "z"].iter().enumerate() {
                    fields.push(int(
                        &format!("player[{p}].{n}"),
                        i32le(&b[p * 12 + k * 4..]) as i128,
                    ));
                }
            }
            Ok(row(fields))
        }
        "typedb-fade-byte" | "armor-pad-reads" => {
            if b.len() < 4 {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "u32 len + len bytes".into(),
                });
            }
            let n = u32le(b) as usize;
            if b.len() != 4 + n {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: format!("4 + {n}"),
                });
            }
            // The W12-S4 canonicalization: the length is a lazy
            // materialization artifact (the bank keeps its grown
            // length after the ≤7-frame fade zeroes the bytes), so
            // BOTH channels canonicalize to the last-nonzero
            // prefix — the D104 differ contract for this row.
            let blob = truncate_trailing_zeros(&b[4..]);
            Ok(row(vec![
                int("len", blob.len() as i128),
                ("bytes".to_string(), FieldVal::Bytes(blob)),
            ]))
        }
        "static-map-wh" => {
            need(id, no, b, "w u32 + h u32", 8)?;
            Ok(row(vec![
                int("w", u32le(b) as i128),
                int("h", u32le(&b[4..]) as i128),
            ]))
        }
        // Anything else (future E rows, TS statics the E side may one
        // day emit): byte passthrough — comparable, never guessed.
        _ => Ok(NormRow {
            id: id.to_string(),
            fields: vec![("raw".to_string(), FieldVal::Bytes(b.to_vec()))],
        }),
    }
}

fn normalize_o1_row(id: &str, no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    let int = |name: &str, v: i128| (name.to_string(), FieldVal::Int(v));
    let row = |fields: Vec<(String, FieldVal)>| NormRow {
        id: id.to_string(),
        fields,
    };
    match id {
        "frame-counter" | "score" | "money" | "difficulty" | "mission" | "mode"
        | "linear-mission-m" | "selection-triple" | "sfx-master-gate" => {
            need(id, no, b, "u32 cell", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        // blink-cursor (D132): the EXD twin 0x10e108 is a plain 4-B
        // u32 cell (dbx-plan Form::Fixed) — it MUST ride the named
        // u32 arm, not the raw passthrough: E normalizes it to the
        // "value" field, and the field-name join would otherwise
        // turn it into two field-level coverage findings instead of
        // a clean compare (the D136 sfx-master-gate precedent).
        "blink-cursor" => {
            need(id, no, b, "u32 cell", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "zone" => {
            // D108 (§6a zone convention): the guest cell (EXW
            // 0x4edd8c / EXD 0x107500) is the 1-based terrain SET
            // (zone_index+1, D99) while E's canonical row carries the
            // 0-based mission-slot INDEX — canonicalize the cell down
            // (a 0 cell passes through: an unstaged cell is a
            // finding, never wrapped).
            need(id, no, b, "u32 cell", 4)?;
            let cell = u32le(b) as i128;
            Ok(row(vec![int(
                "value",
                if cell == 0 { cell } else { cell - 1 },
            )]))
        }
        "rng-state-a" | "rng-state-b" => {
            // Channel-native state word (§6a): u32 LCG state zero
            // extends into the canonical u64.
            need(id, no, b, "u32 cell", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "robot-bank" => robot_row_from_map(id, no, b, EXD_ROBOT_MAP),
        // The T2 banks (W12-S3): the O1 raw form is the FULL span
        // (no count cell on the guest — the free-slot walk is the
        // bound; EXD 0x980d4 / 0x10e174 twins, RE-EXD-MAP §5c). The
        // record layouts are field-exact, so both channels share the
        // same field walk.
        "weapon-anim-bank" => weapon_bank_row(no, b),
        "projectile-bank" => projectile_bank_row(no, b),
        // The destroy-family rows (W12-S4): the O1 raw forms are the
        // guest banks (EXD 0xfe37c / 0xf93cc grids, *(0x119584)
        // object bank + count 0x119554, the 0x95264 TRT bank +
        // count 0x11949c, the 0xac1e4 0x1E-stride mirror rows).
        "object-instances" => object_instances_o1(no, b),
        "trt-array" => trt_o1(no, b),
        "typedb-mirror-rows" => mirror_o1(no, b),
        // The D162 subset-form rows: the bare guest spans, each
        // projected onto E's modeled subset (see the per-row
        // parsers; the EXD/EXW record layouts are field-exact
        // twins — the §5i accessor-twin census).
        "debris-stager" => debris_o1(no, b),
        "splash-records" => splash_o1(no, b),
        "critter-bank" => critter_bank_o1(no, b),
        "effect-rows" => effect_rows_o1(no, b),
        "tile-word-grid" | "platform-strength" => normalize_engine_row(id, no, b),
        "order-target" => {
            need(id, no, b, "3 contiguous i32 cells", 12)?;
            Ok(row(vec![
                int("x", i32le(b) as i128),
                int("y", i32le(&b[4..]) as i128),
                int("z", i32le(&b[8..]) as i128),
            ]))
        }
        "beacon-family" => {
            // Five u16-spaced cells (dbx-plan span form) zero-extend
            // into the canonical u32 words.
            need(id, no, b, "5 u16 cells", 10)?;
            Ok(row(vec![
                int("flag", u16le(b) as i128),
                int("timer", u16le(&b[2..]) as i128),
                int("tile.x", u16le(&b[4..]) as i128),
                int("tile.y", u16le(&b[6..]) as i128),
                int("tile.z", u16le(&b[8..]) as i128),
            ]))
        }
        "spread-claims" => {
            need(id, no, b, "u16*12", 24)?;
            let mut fields = Vec::new();
            for k in 0..12 {
                fields.push(int(&format!("claim[{k}]"), u16le(&b[2 * k..]) as i128));
            }
            Ok(row(fields))
        }
        // The no-extract latch (D133/D136): the O1 raw form is the
        // BARE count-driven span (dbx-plan's $robot_count*4 — the
        // count cell bounds the walk, no prefix rides) — canonical
        // form = u32 count + count u32 slot words.
        "no-extract-latch" => {
            if !b.len().is_multiple_of(4) {
                return Err(NormalizeError::BadLength {
                    id: id.to_string(),
                    frame_no: no,
                    len: b.len(),
                    want: "count*4 u32 slots (the bare no-extract latch span)".into(),
                });
            }
            let n = b.len() / 4;
            let mut fields = Vec::with_capacity(1 + n);
            fields.push(int("count", n as i128));
            for i in 0..n {
                fields.push(int(&format!("slots[{i}]"), u32le(&b[i * 4..]) as i128));
            }
            Ok(row(fields))
        }
        "per-player-selected" => {
            need(id, no, b, "4*0xC anchor cells", 48)?;
            let mut fields = Vec::new();
            for p in 0..4 {
                for (k, n) in ["x", "y", "z"].iter().enumerate() {
                    fields.push(int(
                        &format!("player[{p}].{n}"),
                        i32le(&b[p * 12 + k * 4..]) as i128,
                    ));
                }
            }
            Ok(row(fields))
        }
        "typedb-fade-byte" | "armor-pad-reads" => {
            // Raw w*h grid -> the last-nonzero prefix (the D104
            // canonicalization: E's lazy bank and the full guest
            // grid carry the same content; trailing zeros — faded
            // scorch bytes or untouched tiles — are not state).
            // Subsumes the §6a "len 0 == all-zero grid" rule.
            let blob = truncate_trailing_zeros(b);
            Ok(row(vec![
                int("len", blob.len() as i128),
                ("bytes".to_string(), FieldVal::Bytes(blob)),
            ]))
        }
        "static-map-wh" => {
            // 48-B span: h @+0x00 (cell 0x10748c), w @+0x2C (0x1074b8).
            need(id, no, b, "0x2c+4 span", 0x2c + 4)?;
            Ok(row(vec![
                int("w", u32le(&b[0x2c..]) as i128),
                int("h", u32le(b) as i128),
            ]))
        }
        // move-target-words never reaches here on the raw side — the
        // normalize_frame pre-pass consumes it into the robot-bank
        // splice (D90); it has no standalone raw canonical form.
        // Statics (TS) rows and anything unrecognized: byte passthrough
        // — exact-compare when both sides carry them, a coverage
        // finding when only one does.
        _ => Ok(NormRow {
            id: id.to_string(),
            fields: vec![("raw".to_string(), FieldVal::Bytes(b.to_vec()))],
        }),
    }
}

fn normalize_o2_row(id: &str, no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    match id {
        // EXW cell forms identical to EXD for these rows.
        "frame-counter"
        | "score"
        | "money"
        | "difficulty"
        | "zone"
        | "mission"
        | "mode"
        | "linear-mission-m"
        | "selection-triple"
        | "rng-state-a"
        | "rng-state-b"
        | "order-target"
        | "beacon-family"
        | "spread-claims"
        | "no-extract-latch"
        | "sfx-master-gate"
        // blink-cursor: EXW cell 0x4dc5d0 (§6a sidebar family,
        // [verified]) is the same plain 4-B u32 form as the EXD
        // twin 0x10e108 — the named-arm requirement is identical to
        // O1 (D132; the D136 sfx three-channel precedent).
        | "blink-cursor"
        | "per-player-selected"
        | "typedb-fade-byte"
        | "armor-pad-reads"
        // The D162 subset-form rows: the EXW record layouts are
        // the field-exact twins of the EXD banks (the maps were
        // pinned EXW-side — §7j.11 debris, §7j.24/5 effect rows,
        // §7j.17 critter, §7j.10 splash — and §5i closed the EXD
        // aliases), so the guest-span projections are shared.
        | "debris-stager"
        | "splash-records"
        | "critter-bank"
        | "effect-rows" => normalize_o1_row(id, no, b),
        "robot-bank" => robot_row_from_map(id, no, b, EXW_ROBOT_MAP),
        // The W11 pin (D137, §7j.60; arithmetic corrected by D138):
        // the EXW w/h cells are ADJACENT u32s with w LOW (w 0x4eddec,
        // h 0x4eddf0 — 4 apart; 0x4eddf0−0x4eddec = 4, the stride cell
        // 0x4eddf4 right after) — the port reversed the field order vs
        // the EXD pair (0x2c apart, h LOW), so the O2 capture form is
        // NOT the O1 0x30 span. The capgen dumps ONE contiguous 8-byte
        // span @0x4eddec covering exactly the two cells: w @+0x00,
        // h @+0x04 (the product cell 0x4eddf4 is excluded, exactly
        // like the EXD span excludes 0x1074e4).
        "static-map-wh" => {
            need(id, no, b, "4+4 span", 8)?;
            Ok(NormRow {
                id: id.to_string(),
                fields: vec![
                    ("w".to_string(), FieldVal::Int(u32le(b) as i128)),
                    ("h".to_string(), FieldVal::Int(u32le(&b[4..]) as i128)),
                ],
            })
        }
        // move-target-words never reaches here — the normalize_frame
        // pre-pass consumes it into the robot-bank splice (D90).
        _ => Ok(NormRow {
            id: id.to_string(),
            fields: vec![("raw".to_string(), FieldVal::Bytes(b.to_vec()))],
        }),
    }
}

/// The O3 field map (D142 §5, W10-impl-b; spec O3-8STREET §5a): the
/// 8street reconstruction rebuilds EXW state — same cells, same layouts
/// — so the O3 raw rows are O2-form and normalize through the O2 table
/// VERBATIM. The D142 §6 seam set is NOT a normalization difference
/// (seam rows normalize identically so a clean capture compares clean);
/// it is a compare-time classification — `o3_seam_reason` +
/// `Class::O3Seam` in `run_diff`/`compare_field`.
fn normalize_o3_row(id: &str, no: u64, b: &[u8]) -> Result<NormRow, NormalizeError> {
    normalize_o2_row(id, no, b)
}

// ---------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------

/// Divergence classes (DESIGN §6 + the STRUCTURAL catcher).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Structural asymmetry in VALUES: frame alignment, record
    /// counts, statics bytes, injection schedule, T3 draw counts.
    /// Always fails the verdict.
    Structural,
    /// Row/field coverage asymmetry — one side cannot source the row
    /// or field (the §6a E-gap list and the RE-EXD-MAP §8 normalizer
    /// gaps fall out of the data here). Reported, metered, never
    /// silent; changes only when coverage deliberately changes, so it
    /// notes rather than fails the verdict.
    Coverage,
    /// E's canonical semantics differ from EXW canon (O2 arbitrates;
    /// provisional without a tiebreak dump).
    EngineBug,
    /// EXD diverges from EXW canon; the engine keeps EXW (log to
    /// docs/DIVERGENCES.md). Requires O2 arbitration.
    OriginalDivergence,
    /// Dump/injection artifact (assigned by callers post-triage; the
    /// runner double-run digest check is the detector).
    WatchArtifact,
    /// Budgeted statistical divergence (T3 rows with equal draw
    /// counts).
    AcceptedT3,
    /// Report-only tolerant diff (T2 fields beyond the quantum).
    T2Reported,
    /// O3 8street expected-divergence row (D142 §6, W10-impl-b): the
    /// reconstruction feeds the cell from a DIFFERENT source than
    /// EXW/EXD canon (OPTIONS.BDL vs the registry, always-on speech,
    /// ...), so the row diverges BY CONSTRUCTION whenever an O3 side
    /// participates. Report-only — never a channel finding, never
    /// tiebreak evidence (an OPTIONS.BDL-fed vote is not canon).
    O3Seam,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Structural => "structural",
            Class::Coverage => "coverage",
            Class::EngineBug => "engine-bug",
            Class::OriginalDivergence => "original-divergence",
            Class::WatchArtifact => "watch-artifact",
            Class::AcceptedT3 => "accepted-T3",
            Class::T2Reported => "T2-reported",
            Class::O3Seam => "o3-seam",
        }
    }
}

impl Class {
    /// Does this class fail the verdict?
    pub fn failing(self) -> bool {
        matches!(
            self,
            Class::Structural | Class::EngineBug | Class::WatchArtifact
        )
    }
}

// ---------------------------------------------------------------------
// The O3 seam ledger (D142 §6, W10-impl-b — spec: O3-8STREET §5a)
// ---------------------------------------------------------------------

/// The O3 seam ledger, row-id matcher: live registry rows whose 8street
/// cell is fed from a different source than EXW/EXD canon. They diverge
/// BY CONSTRUCTION on O3 and classify `o3-seam` (report-only), never as
/// channel findings.
const O3_SEAM_ROWS: &[(&str, &str)] = &[(
    "sfx-master-gate",
    "8street feeds sound_enable from SAVES/OPTIONS.BDL (options.cpp:125-246) \
     where EXW reads the HKCU registry (D128) and EXD parses CONFIG.BDL \
     (D134); E dumps constant 1 (D136)",
)];

/// The O3 seam ledger, EXW base-cell matcher: the whole registry-config
/// family (cells pinned by RE-EXW-TITLEMENU §7j.56/D128) — 8street reads
/// SAVES/OPTIONS.BDL + auto-detects language/misc from file existence
/// where EXW reads HKCU\Software\Mirage\Bedlam\1.00. Matching a row's
/// `exw_addr` BASE CELL catches rows added later, before any dedicated
/// id joins O3_SEAM_ROWS. Deliberately ABSENT (O3-8STREET §5a): the
/// volume cell 0x4ddb2c (a trigger — scancode — deviation, not a feed
/// deviation: arrow-key drift on a live O3 capture is a genuine
/// finding, never a seam) and CDDA (behavior, no canon watch cell).
const O3_SEAM_CELLS: &[(&str, &str)] = &[
    (
        "0x4ede58",
        "SOUND gate cell: OPTIONS.BDL `sound` field vs the registry SOUND \
         value (the sfx-master-gate row's cell)",
    ),
    (
        "0x4ede5c",
        "SOUND sister gate cell: one OPTIONS.BDL `sound` value feeds both \
         cells where EXW loads the registry value (D134)",
    ),
    (
        "0x4eb93c",
        "SPEECH cell: forced ALWAYS-ON on 8street (options.cpp:211-215, \
         RESEARCH-8STREET §5) vs the registry/CONFIG.BDL value",
    ),
    (
        "0x4edbd8",
        "ACTIONPAN cell: OPTIONS.BDL `actionpan` field (auto-created \
         default file) vs the registry value (D128)",
    ),
    (
        "0x46cca4",
        "CINEMATICS cell: OPTIONS.BDL-derived / file-existence \
         auto-detect vs the registry value (D128)",
    ),
    (
        "0x4eba1c",
        "LANGUAGE cell: SDL-locale auto-detect (options.cpp:125-202) vs \
         the registry value (D128)",
    ),
    (
        "0x4e444c",
        "DEFAULTNAME cell: OPTIONS.BDL playername[8] in SAVES/ vs the \
         registry name (D128)",
    ),
];

/// Does `exw_addr` anchor a row on the seam cell `cell`? The registry
/// expressions are `"<base> + ..."`, `"<base>"`, or `"<a> / <b>"` (two
/// cells) — match the BASE form: the expression's first cell, exactly.
fn exw_addr_on_cell(exw_addr: &str, cell: &str) -> bool {
    let first = exw_addr
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .split('+')
        .next()
        .unwrap_or("")
        .trim();
    first.eq_ignore_ascii_case(cell)
}

/// The O3 seam lookup (D142 §6): `Some(reason)` = the row is
/// never-comparable on O3 (classify `o3-seam`, exclude from tiebreak
/// arbitration). Matched by row id first, then the registry row's EXW
/// base cell.
pub fn o3_seam_reason(id: &str, exw_addr: &str) -> Option<&'static str> {
    if let Some((_, reason)) = O3_SEAM_ROWS.iter().find(|(rid, _)| *rid == id) {
        return Some(reason);
    }
    O3_SEAM_CELLS
        .iter()
        .find(|(cell, _)| exw_addr_on_cell(exw_addr, cell))
        .map(|(_, reason)| *reason)
}

/// The per-field comparison class for a (row, field) pair.
fn field_class(row: &str, field: &str, tier: &str) -> Class {
    match (row, field) {
        // Budgeted T2: the never-resetting counter + in-tick positions.
        ("frame-counter", _) => Class::T2Reported,
        // Statistical rows: never bit-compared (draw counts checked
        // separately).
        ("rng-state-a", _) | ("rng-state-b", _) => Class::AcceptedT3,
        ("robot-bank", "count") | ("move-target-words", "count") => Class::Structural,
        ("weapon-anim-bank", "count") | ("projectile-bank", "count") => Class::Structural,
        // The no-extract-latch count (D136): structural like every
        // other count word — the robot-count scenario seams (D91/
        // D103/D108 `_e_staging`) surface here exactly as they
        // already do on robot-bank.count.
        ("no-extract-latch", "count") => Class::Structural,
        // The destroy-family counts (W12-S4): structural like the
        // other count words.
        ("object-instances", "count")
        | ("trt-array", "count")
        | ("typedb-mirror-rows", "count")
        | ("debris-stager", "count")
        | ("splash-records", "count") => Class::Structural,
        // The D162 subset-form counts: structural like every other
        // bank count word — a count mismatch is a staging
        // divergence, never a T2/T3 budget item.
        ("critter-bank", "count") | ("effect-rows", "count") => Class::Structural,
        ("robot-bank", f) if is_t2_position(f) => Class::T2Reported,
        ("move-target-words", f) if f.ends_with(".tx") || f.ends_with(".ty") => Class::T2Reported,
        // TS statics: byte-exact structural comparison.
        (r, _) if r.starts_with("static-") => Class::Structural,
        _ => match tier {
            "T2" => Class::T2Reported,
            "T3" => Class::AcceptedT3,
            // T0/T1/T4/S0/TI rows and robot/move-target fields default
            // exact (T4 event payloads are exact-grammar compares).
            _ => Class::EngineBug,
        },
    }
}

/// T2-tolerant position fields (§6: "positions in-tick").
fn is_t2_position(f: &str) -> bool {
    f.ends_with(".pos_x")
        || f.ends_with(".pos_y")
        || f.ends_with(".z")
        || f.contains(".probe_z[")
        || f.ends_with(".target_x")
        || f.ends_with(".target_y")
}

/// One aggregated finding: first occurrence + total frames affected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub class: Class,
    pub row: String,
    pub field: String,
    pub first_frame: u64,
    pub frames: u64,
    pub a: Option<FieldVal>,
    pub b: Option<FieldVal>,
    pub detail: String,
}

/// Per-side run fingerprint for the report + manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideFingerprint {
    pub channel: &'static str,
    pub scenario: String,
    pub build_sha256: String,
    pub pins: Vec<String>,
    pub frame_count: u64,
    pub chain_digest: String,
    pub dump_sha256: String,
}

/// The whole diff result: findings + meter + fingerprints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    pub mode: Mode,
    pub a: SideFingerprint,
    pub b: SideFingerprint,
    pub tiebreak: Option<SideFingerprint>,
    pub paired_frames: u64,
    /// Constant frame shift applied to side B (0 = none found).
    pub shift: i64,
    pub findings: Vec<Finding>,
    /// T2 diffs suppressed by the quantum (count, per row+field).
    pub suppressed: BTreeMap<String, u64>,
    /// Event-timing table: row id -> (first change frame A, first
    /// change frame B, change count A, change count B). Rows that
    /// never change on a side are None.
    pub timing: BTreeMap<String, (Option<u64>, Option<u64>, u64, u64)>,
    pub t2_quantum: i64,
    pub verdict: Verdict,
}

impl DiffResult {
    /// Counts by class (the divergence meter).
    pub fn meter(&self) -> BTreeMap<Class, u64> {
        let mut m = BTreeMap::new();
        for f in &self.findings {
            *m.entry(f.class).or_insert(0) += 1;
        }
        m
    }
    pub fn count(&self, class: Class) -> u64 {
        self.findings.iter().filter(|f| f.class == class).count() as u64
    }
    pub fn first_divergence(&self) -> Option<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.class.failing())
            .min_by_key(|f| (f.first_frame, f.row.clone(), f.field.clone()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Zero findings at all.
    Pass,
    /// Only budgeted findings (original-divergence / accepted-T3 /
    /// T2-reported).
    PassWithNotes,
    /// Any structural / engine-bug / watch-artifact finding.
    Fail,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::PassWithNotes => "PASS-WITH-NOTES",
            Verdict::Fail => "FAIL",
        }
    }
}

/// Run the whole differ over two decoded dumps (+ optional O2
/// tiebreak). `a_bytes`/`b_bytes`/`t_bytes` are the RAW dump streams
/// (fingerprints need their sha256; decode happens inside).
pub fn run_diff(
    a_bytes: &[u8],
    b_bytes: &[u8],
    tiebreak_bytes: Option<&[u8]>,
    cfg: &DiffConfig,
    reg: &[Watch],
) -> Result<DiffResult, NormalizeError> {
    let a = crate::dump::decode_dump(a_bytes).map_err(dump_err)?;
    let b = crate::dump::decode_dump(b_bytes).map_err(dump_err)?;
    let t = match tiebreak_bytes {
        Some(t) => Some(crate::dump::decode_dump(t).map_err(dump_err)?),
        None => None,
    };
    if a.header.scenario != b.header.scenario {
        return Err(NormalizeError::ScenarioMismatch {
            a: a.header.scenario.clone(),
            b: b.header.scenario.clone(),
        });
    }
    let na = normalize_dump(&a, reg)?;
    let nb = normalize_dump(&b, reg)?;
    let nt = match &t {
        Some(t) => Some(normalize_dump(t, reg)?),
        None => None,
    };

    // ---- alignment: constant shift detection (§6), then pair by
    // frame_no (b.frame_no == a.frame_no + shift) ----
    let seq_a: Vec<i64> = na.iter().map(|f| f.frame_no as i64).collect();
    let seq_b: Vec<i64> = nb.iter().map(|f| f.frame_no as i64).collect();
    let shift = detect_shift(&seq_a, &seq_b);

    let mut pairs: Vec<(&NormFrame, &NormFrame)> = Vec::new();
    {
        let mut bi = 0usize;
        for fa in &na {
            let want = fa.frame_no as i64 + shift;
            while bi < nb.len() && (nb[bi].frame_no as i64) < want {
                bi += 1;
            }
            if bi < nb.len() && nb[bi].frame_no as i64 == want {
                pairs.push((fa, &nb[bi]));
                bi += 1; // frame_no strictly increases in both dumps
            }
        }
    }

    let mut findings: Vec<Finding> = Vec::new();
    if shift != 0 {
        // Anchor-event alignment (§6 "T1-timing: reported"): a
        // constant shift is an event-timing observation, budgeted like
        // T2 (report-only); the pairing below compares on the aligned
        // frames so the VALUES still judge the parity.
        findings.push(Finding {
            class: Class::T2Reported,
            row: "(alignment)".into(),
            field: "frame_no".into(),
            first_frame: 0,
            frames: 1,
            a: None,
            b: None,
            detail: format!(
                "constant frame shift {shift:+} detected and applied (anchor-event alignment, DESIGN sec 6)"
            ),
        });
    } else if na.len() != nb.len() || pairs.len() != na.len().max(nb.len()) {
        findings.push(Finding {
            class: Class::Structural,
            row: "(alignment)".into(),
            field: "frame_count".into(),
            first_frame: 0,
            frames: 1,
            a: Some(FieldVal::Int(na.len() as i128)),
            b: Some(FieldVal::Int(nb.len() as i128)),
            detail: format!(
                "frame sequences differ with no constant shift ({} pairs of {}/{})",
                pairs.len(),
                na.len(),
                nb.len()
            ),
        });
    }

    // ---- per-frame comparison ----
    let mut agg: BTreeMap<(Class, String, String), Finding> = BTreeMap::new();
    let mut suppressed: BTreeMap<String, u64> = BTreeMap::new();
    let tier_of = |id: &str| -> String {
        reg.iter()
            .find(|r| r.id == id)
            .map(|r| r.tier.clone())
            .unwrap_or_default()
    };

    let mut push = |f: Finding| {
        let key = (f.class, f.row.clone(), f.field.clone());
        agg.entry(key).and_modify(|e| e.frames += 1).or_insert(f);
    };

    let mut coverage_rows: BTreeMap<String, (u64, u64, String)> = BTreeMap::new(); // row -> (frames a-only, frames b-only, note)

    // The O3 seam classification (D142 §6, W10-impl-b): when ANY side
    // of the compare is the 8street channel, ledger rows (fed from
    // different sources than EXW/EXD canon) diverge BY CONSTRUCTION —
    // they report `o3-seam` (notes, never channel findings) and are
    // excluded from tiebreak arbitration (an OPTIONS.BDL-fed vote is
    // not canon evidence).
    let o3_involved = matches!(a.header.channel, Channel::O3Street)
        || matches!(b.header.channel, Channel::O3Street)
        || t.as_ref()
            .is_some_and(|t| matches!(t.header.channel, Channel::O3Street));
    let seam_of = |id: &str| -> Option<&'static str> {
        if !o3_involved {
            return None;
        }
        let exw = reg
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.exw_addr.as_str())
            .unwrap_or("");
        o3_seam_reason(id, exw)
    };

    for (fa, fb) in &pairs {
        // Injection flags must match (same schedule).
        if fa.injection_applied != fb.injection_applied {
            push(Finding {
                class: Class::Structural,
                row: "(frame)".into(),
                field: "injection_applied".into(),
                first_frame: fa.frame_no,
                frames: 1,
                a: Some(FieldVal::Int(fa.injection_applied as i128)),
                b: Some(FieldVal::Int(fb.injection_applied as i128)),
                detail: "injection schedule mismatch".into(),
            });
        }
        let ids_a: Vec<&str> = fa.rows.iter().map(|r| r.id.as_str()).collect();
        let ids_b: Vec<&str> = fb.rows.iter().map(|r| r.id.as_str()).collect();
        let mut ids: Vec<&str> = Vec::new();
        for id in ids_a.iter().copied().chain(ids_b.iter().copied()) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        for id in ids {
            let (ra, rb) = (fa.row(id), fb.row(id));
            match (ra, rb) {
                (None, Some(_)) | (Some(_), None) => {
                    let e = coverage_rows
                        .entry(id.to_string())
                        .or_insert((0, 0, String::new()));
                    if ra.is_none() {
                        e.0 += 1;
                    } else {
                        e.1 += 1;
                    }
                    continue;
                }
                (Some(ra), Some(rb)) => {
                    // Row-level field compare. The name-union join is
                    // HASH-INDEXED (first occurrence per name — the
                    // `field()` lookup semantics; rows carry unique
                    // names by construction): the S5-class mirror
                    // rows carry ~170k fields per frame and a linear
                    // union scan + per-name lookup is quadratic.
                    let tier = tier_of(id);
                    let seam = seam_of(id);
                    let mut b_first: std::collections::HashMap<&str, &FieldVal> =
                        std::collections::HashMap::with_capacity(rb.fields.len());
                    for (n, v) in &rb.fields {
                        b_first.entry(n.as_str()).or_insert(v);
                    }
                    let mut a_counted: std::collections::HashSet<&str> =
                        std::collections::HashSet::with_capacity(ra.fields.len());
                    for (n, _) in &ra.fields {
                        a_counted.insert(n.as_str());
                    }
                    // b-only names (first occurrence): field-level
                    // coverage gaps, one count per frame like the
                    // old union's dedup.
                    let mut b_counted: std::collections::HashSet<&str> =
                        std::collections::HashSet::new();
                    for name in rb.fields.iter().map(|(n, _)| n.as_str()) {
                        if a_counted.contains(name) || !b_counted.insert(name) {
                            continue;
                        }
                        let key = format!("{id}.{name}");
                        let e = coverage_rows.entry(key).or_insert((0, 0, String::new()));
                        e.1 += 1;
                    }
                    let mut a_first: std::collections::HashSet<&str> =
                        std::collections::HashSet::with_capacity(ra.fields.len());
                    for (name, va) in ra.fields.iter().map(|(n, v)| (n.as_str(), v)) {
                        if !a_first.insert(name) {
                            continue;
                        }
                        match b_first.get(name) {
                            Some(vb) => {
                                compare_field(
                                    id,
                                    name,
                                    &tier,
                                    seam,
                                    fa.frame_no,
                                    va,
                                    vb,
                                    cfg,
                                    &mut push,
                                    &mut suppressed,
                                    nt.as_ref()
                                        .and_then(|ntf| {
                                            ntf.iter().find(|f| f.frame_no == fa.frame_no)
                                        })
                                        .and_then(|f| f.row(id))
                                        .and_then(|r| r.field(name).cloned()),
                                );
                            }
                            None => {
                                // Field-level coverage gap (e.g. robot
                                // fields the O1 map cannot source).
                                let key = format!("{id}.{name}");
                                let e = coverage_rows.entry(key).or_insert((0, 0, String::new()));
                                e.0 += 1;
                            }
                        }
                    }
                }
                (None, None) => unreachable!("id came from one of the rows"),
            }
        }
    }

    // Coverage findings (deduped per row/field, with frame counts).
    // Seam rows (D142 §6) classify `o3-seam` instead of `coverage`
    // when an O3 side participates: a registry-config row carried by
    // one side only is the expected registry-vs-OPTIONS.BDL seam, not
    // coverage noise.
    for (key, (a_only, b_only, _)) in &coverage_rows {
        let (row, field) = match key.split_once('.') {
            Some((r, f)) => (r.to_string(), f.to_string()),
            None => (key.clone(), "(row)".to_string()),
        };
        let (class, detail) = match seam_of(&row) {
            Some(reason) => (Class::O3Seam, format!("o3-seam (D142 sec 6): {reason}")),
            None => (
                Class::Coverage,
                "coverage: frames carried by one side only".into(),
            ),
        };
        findings.push(Finding {
            class,
            row,
            field,
            first_frame: 0,
            frames: a_only + b_only,
            a: if *a_only > 0 {
                Some(FieldVal::Int(*a_only as i128))
            } else {
                None
            },
            b: if *b_only > 0 {
                Some(FieldVal::Int(*b_only as i128))
            } else {
                None
            },
            detail,
        });
    }
    findings.extend(agg.into_values());

    // ---- T3 draw-count check (§6: compare draw COUNTS, never bits) ----
    for id in ["rng-state-a", "rng-state-b"] {
        let (ca, cb) = (change_count(&na, id), change_count(&nb, id));
        if ca != cb {
            findings.push(Finding {
                class: Class::Structural,
                row: id.into(),
                field: "value".into(),
                first_frame: 0,
                frames: 1,
                a: Some(FieldVal::Int(ca as i128)),
                b: Some(FieldVal::Int(cb as i128)),
                detail: "T3 draw-count mismatch (state-change counts differ)".into(),
            });
        }
    }

    // ---- event-timing table (mechanical: canonical change frames) ----
    let mut timing = BTreeMap::new();
    let mut all_rows: Vec<&str> = Vec::new();
    for f in na.iter().chain(nb.iter()) {
        for r in &f.rows {
            if !all_rows.contains(&r.id.as_str()) {
                all_rows.push(r.id.as_str());
            }
        }
    }
    for id in all_rows {
        let ta = first_change(&na, id);
        let tb = first_change(&nb, id);
        let (ca, cb) = (change_count(&na, id), change_count(&nb, id));
        timing.insert(id.to_string(), (ta, tb, ca, cb));
    }

    // ---- verdict ----
    let verdict = if findings.is_empty() {
        Verdict::Pass
    } else if findings.iter().all(|f| !f.class.failing()) {
        Verdict::PassWithNotes
    } else {
        Verdict::Fail
    };

    let fp = |d: &Dump, raw: &[u8]| SideFingerprint {
        channel: d.header.channel.name(),
        scenario: d.header.scenario.clone(),
        build_sha256: crate::hash::hex_lower(&d.header.build_sha256),
        pins: d.header.pins.clone(),
        frame_count: d.trailer.frame_count,
        chain_digest: format!("{}", d.trailer.chain),
        dump_sha256: crate::hash::hex_lower(&crate::hash::sha256(raw)),
    };

    Ok(DiffResult {
        mode: cfg.mode,
        a: fp(&a, a_bytes),
        b: fp(&b, b_bytes),
        tiebreak: t.as_ref().map(|t| fp(t, tiebreak_bytes.unwrap_or(&[]))),
        paired_frames: pairs.len() as u64,
        shift,
        findings,
        suppressed,
        timing,
        t2_quantum: cfg.t2_quantum,
        verdict,
    })
}

fn dump_err(e: crate::dump::DumpError) -> NormalizeError {
    // Dump integrity failures surface as bad-length style errors; the
    // text carries everything.
    NormalizeError::BadLength {
        id: format!("(dump: {e})"),
        frame_no: 0,
        len: 0,
        want: "an integrity-verified dump".into(),
    }
}

/// Detect a constant shift between the frame_no sequences: returns k
/// such that b[i] == a[i] + k for all i (|k| <= 8), else 0.
fn detect_shift(a: &[i64], b: &[i64]) -> i64 {
    if a == b || a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0;
    }
    for k in -8i64..=8 {
        if k == 0 {
            continue;
        }
        if b.iter().zip(a.iter()).all(|(bv, av)| *bv == *av + k) {
            return k;
        }
    }
    0
}

/// The D104 armor/fade canonicalization: the last-nonzero prefix —
/// E lazily materializes the +0x18 byte bank (it keeps its grown
/// length after the ≤7-frame fade zeroes the tail) while the guest
/// grid is full-size; identical content canonicalizes identically.
fn truncate_trailing_zeros(b: &[u8]) -> Vec<u8> {
    let end = b.iter().rposition(|&x| x != 0).map(|i| i + 1).unwrap_or(0);
    b[..end].to_vec()
}

/// Normalize every frame of a dump.
pub fn normalize_dump(dump: &Dump, reg: &[Watch]) -> Result<Vec<NormFrame>, NormalizeError> {
    let mut out = Vec::with_capacity(dump.frames.len());
    for f in &dump.frames {
        out.push(NormFrame {
            frame_no: f.frame_no,
            injection_applied: f.injection_applied,
            rows: normalize_frame(f, dump.header.channel, reg)?,
        });
    }
    Ok(out)
}

/// Field comparison + O2 arbitration for one (frame, row, field).
/// `seam` = the D142 §6 O3 seam reason when an O3 side participates
/// AND the row is on the ledger: the row never compares as a channel
/// finding (equality stays silent; divergence is the by-construction
/// OPTIONS.BDL/registry/always-on difference, reported `o3-seam`) and
/// never arbitrates (the seam vote is not canon evidence).
#[allow(clippy::too_many_arguments)]
fn compare_field(
    id: &str,
    name: &str,
    tier: &str,
    seam: Option<&'static str>,
    frame_no: u64,
    va: &FieldVal,
    vb: &FieldVal,
    cfg: &DiffConfig,
    push: &mut dyn FnMut(Finding),
    suppressed: &mut BTreeMap<String, u64>,
    tiebreak_val: Option<FieldVal>,
) {
    let class = field_class(id, name, tier);
    let equal = va == vb;
    if let Some(reason) = seam {
        // The O3 seam (D142 §6, W10-impl-b): report-only. This branch
        // precedes every ordinary class — a seam row can never yield
        // Structural/EngineBug/T2 notes while O3 participates, and the
        // tiebreak value is deliberately never consulted.
        if !equal {
            push(Finding {
                class: Class::O3Seam,
                row: id.into(),
                field: name.into(),
                first_frame: frame_no,
                frames: 1,
                a: Some(va.clone()),
                b: Some(vb.clone()),
                detail: format!("o3-seam (D142 sec 6): {reason}"),
            });
        }
        return;
    }
    match class {
        Class::AcceptedT3 => {
            // Never bit-compared. Presence asymmetry is coverage;
            // equality is silence. (Draw counts checked per-run.)
            let _ = equal;
        }
        Class::T2Reported => {
            if equal {
                return;
            }
            let delta = numeric_delta(va, vb);
            match delta {
                Some(d) if d.abs() <= cfg.t2_quantum as i128 => {
                    *suppressed.entry(format!("{id}.{name}")).or_insert(0) += 1;
                }
                _ => push(Finding {
                    class: Class::T2Reported,
                    row: id.into(),
                    field: name.into(),
                    first_frame: frame_no,
                    frames: 1,
                    a: Some(va.clone()),
                    b: Some(vb.clone()),
                    detail: format!("T2-tolerant diff beyond quantum {}", cfg.t2_quantum),
                }),
            }
        }
        Class::Structural => {
            if !equal {
                push(Finding {
                    class: Class::Structural,
                    row: id.into(),
                    field: name.into(),
                    first_frame: frame_no,
                    frames: 1,
                    a: Some(va.clone()),
                    b: Some(vb.clone()),
                    detail: "structural (exact) mismatch".into(),
                });
            }
        }
        Class::EngineBug => {
            if equal {
                return;
            }
            // T1-exact: arbitrate with O2 when available. va = E (side
            // A), vb = O1 (side B oracle), tv = O2 (EXW canon).
            let (class, detail) = match &tiebreak_val {
                Some(tv) if tv == vb => (
                    Class::EngineBug,
                    "O2/EXW canon agrees with O1: the engine (E) is the outlier",
                ),
                Some(tv) if tv == va => (
                    Class::OriginalDivergence,
                    "O2/EXW canon agrees with E: EXD diverges from EXW (engine keeps EXW; \
                     log to docs/DIVERGENCES.md)",
                ),
                Some(_) => (
                    Class::EngineBug,
                    "all three channels differ (E wrong against both oracles)",
                ),
                None => (
                    Class::EngineBug,
                    "provisional engine-bug: no O2 tiebreak dump supplied",
                ),
            };
            // findings are aggregated via `push`; the arbitration
            // detail carries the O2 reading.
            push(Finding {
                class,
                row: id.into(),
                field: name.into(),
                first_frame: frame_no,
                frames: 1,
                a: Some(va.clone()),
                b: Some(vb.clone()),
                detail: detail.into(),
            });
        }
        Class::OriginalDivergence | Class::WatchArtifact | Class::Coverage | Class::O3Seam => {
            // Never assigned by field comparison (coverage asymmetry
            // never reaches a value compare; the other two are
            // caller-triage labels; O3Seam is assigned only by the
            // seam branch above).
        }
    }
}

fn numeric_delta(a: &FieldVal, b: &FieldVal) -> Option<i128> {
    match (a, b) {
        (FieldVal::Int(x), FieldVal::Int(y)) => Some(x - y),
        _ => None,
    }
}

fn first_change(frames: &[NormFrame], id: &str) -> Option<u64> {
    // Seed with the row's first appearance: the event timing records
    // when the row's canonical bytes first CHANGE (absent->present at
    // frame 0 is appearance, not an event).
    let mut prev: Option<&NormRow> = frames.first().and_then(|f| f.row(id));
    for f in frames.iter().skip(1) {
        let cur = f.row(id);
        match (prev, cur) {
            (Some(p), Some(c)) if p.fields != c.fields => return Some(f.frame_no),
            (None, Some(_)) => return Some(f.frame_no), // appeared mid-run
            _ => {}
        }
        prev = cur;
    }
    None
}

fn change_count(frames: &[NormFrame], id: &str) -> u64 {
    let mut prev: Option<&NormRow> = frames.first().and_then(|f| f.row(id));
    let mut n = 0;
    for f in frames.iter().skip(1) {
        let cur = f.row(id);
        let changed = match (prev, cur) {
            (Some(p), Some(c)) => p.fields != c.fields,
            (None, Some(_)) | (Some(_), None) => true,
            _ => false,
        };
        if changed {
            n += 1;
        }
        prev = cur;
    }
    n
}

// ---------------------------------------------------------------------
// Report writer + fingerprint manifest
// ---------------------------------------------------------------------

/// The human report (DESIGN §6: divergence meter, first divergence,
/// event-timing table, both chains). Deterministic.
pub fn report_text(res: &DiffResult) -> String {
    let mut s = String::new();
    let side = |tag: &str, f: &SideFingerprint| {
        format!(
            "{tag}: {}  scenario {}  chain {}  frames {}  dump-sha256 {}\n     pins [{}]\n",
            f.channel,
            f.scenario,
            f.chain_digest,
            f.frame_count,
            f.dump_sha256,
            f.pins.join(", ")
        )
    };
    s.push_str(&format!(
        "BEDLAM DIFF REPORT  (mode: {})\n{}{}{}",
        res.mode.name(),
        side("  A", &res.a),
        side("  B", &res.b),
        match &res.tiebreak {
            Some(t) => side("  T", t),
            None => String::new(),
        }
    ));
    s.push_str(&format!(
        "ALIGNMENT: {} frame pairs by frame_no{}  (T2 quantum {})\n",
        res.paired_frames,
        if res.shift != 0 {
            format!(", constant shift {:+} applied", res.shift)
        } else {
            String::new()
        },
        res.t2_quantum
    ));

    s.push_str("METER:");
    let meter = res.meter();
    let total_classes: [Class; 7] = [
        Class::Structural,
        Class::Coverage,
        Class::EngineBug,
        Class::OriginalDivergence,
        Class::WatchArtifact,
        Class::AcceptedT3,
        Class::T2Reported,
    ];
    for c in total_classes {
        s.push_str(&format!(
            " {}={}",
            c.name(),
            meter.get(&c).copied().unwrap_or(0)
        ));
    }
    s.push('\n');

    match res.first_divergence() {
        Some(f) => s.push_str(&format!(
            "FIRST DIVERGENCE: frame {} row {} field {}: A={} B={} ({}) - {}\n",
            f.first_frame,
            f.row,
            f.field,
            f.a.as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".into()),
            f.b.as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".into()),
            f.class.name(),
            f.detail
        )),
        None => s.push_str("FIRST DIVERGENCE: none\n"),
    }

    if !res.suppressed.is_empty() {
        s.push_str("T2 SUPPRESSED (within quantum):");
        for (k, v) in &res.suppressed {
            s.push_str(&format!(" {k}x{v}"));
        }
        s.push('\n');
    }

    s.push_str("FINDINGS:\n");
    if res.findings.is_empty() {
        s.push_str("  (none)\n");
    } else {
        let mut sorted: Vec<&Finding> = res.findings.iter().collect();
        sorted.sort_by(|x, y| {
            (x.class, x.row.clone(), x.field.clone()).cmp(&(
                y.class,
                y.row.clone(),
                y.field.clone(),
            ))
        });
        let cap = 48usize;
        for (i, f) in sorted.iter().enumerate() {
            if i >= cap {
                s.push_str(&format!(
                    "  ... {} more aggregates (counts in the manifest)\n",
                    sorted.len() - cap
                ));
                break;
            }
            s.push_str(&format!(
                "  [{}] {}.{}: frames {} (first {}): A={} B={} - {}\n",
                f.class.name(),
                f.row,
                f.field,
                f.frames,
                f.first_frame,
                f.a.as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "-".into()),
                f.b.as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "-".into()),
                f.detail
            ));
        }
    }

    s.push_str("EVENT TIMING (first-change frame A/B, changes A/B):\n");
    for (id, (ta, tb, ca, cb)) in &res.timing {
        s.push_str(&format!(
            "  {id}: A {:?}/{ca}  B {:?}/{cb}\n",
            ta.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            tb.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
        ));
    }

    s.push_str(&format!("VERDICT: {}\n", res.verdict.name()));
    s
}

/// The fingerprint manifest (JSON, git-carried; dumps stay under
/// runtime/ — DESIGN §3 hygiene). Hand-rolled like `Manifest::to_json`
/// (zero-dep charter; all strings are ids/hex/class names — no
/// escaping needed).
pub fn manifest_json(res: &DiffResult) -> String {
    let fp = |f: &SideFingerprint| {
        format!(
            "{{\"channel\": \"{}\", \"scenario\": \"{}\", \"build_sha256\": \"{}\", \
             \"pins\": [{}], \"frame_count\": {}, \"chain_digest\": \"{}\", \
             \"dump_sha256\": \"{}\"}}",
            f.channel,
            f.scenario,
            f.build_sha256,
            f.pins
                .iter()
                .map(|p| format!("\"{p}\""))
                .collect::<Vec<_>>()
                .join(", "),
            f.frame_count,
            f.chain_digest,
            f.dump_sha256
        )
    };
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"mode\": \"{}\",\n", res.mode.name()));
    s.push_str(&format!("  \"paired_frames\": {},\n", res.paired_frames));
    s.push_str(&format!("  \"shift\": {},\n", res.shift));
    s.push_str(&format!("  \"t2_quantum\": {},\n", res.t2_quantum));
    s.push_str(&format!("  \"a\": {},\n", fp(&res.a)));
    s.push_str(&format!("  \"b\": {},\n", fp(&res.b)));
    s.push_str(&format!(
        "  \"tiebreak\": {},\n",
        res.tiebreak
            .as_ref()
            .map(fp)
            .unwrap_or_else(|| "null".into())
    ));
    s.push_str("  \"meter\": {");
    let meter = res.meter();
    let entries = [
        Class::Structural,
        Class::Coverage,
        Class::EngineBug,
        Class::OriginalDivergence,
        Class::WatchArtifact,
        Class::AcceptedT3,
        Class::T2Reported,
    ]
    .iter()
    .map(|c| format!("\"{}\": {}", c.name(), meter.get(c).copied().unwrap_or(0)))
    .collect::<Vec<_>>()
    .join(", ");
    s.push_str(&entries);
    s.push_str("},\n");
    match res.first_divergence() {
        Some(f) => s.push_str(&format!(
            "  \"first_divergence\": {{\"frame\": {}, \"row\": \"{}\", \"field\": \"{}\", \
             \"class\": \"{}\"}},\n",
            f.first_frame,
            f.row,
            f.field,
            f.class.name()
        )),
        None => s.push_str("  \"first_divergence\": null,\n"),
    }
    s.push_str(&format!("  \"verdict\": \"{}\"\n", res.verdict.name()));
    s.push_str("}\n");
    s
}
