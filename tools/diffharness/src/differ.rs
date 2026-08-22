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
//!   report — never zero-filled-and-compared, never guessed).
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
//! frame-counter watch — the O1 counter never resets, RE-EXD-MAP §8).
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
    /// O1 row whose raw form is not pinned (the deferred
    /// move-target-words extent) — the capture plan should not emit it.
    UnpinnedForm { id: String },
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
            NormalizeError::UnpinnedForm { id } => write!(
                f,
                "watch {id:?}: raw O1 form is not pinned (deferred plan row) — cannot normalize"
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
    let mut rows = Vec::new();
    for w in &frame.watches {
        let id = w.id.as_str();
        // Registry membership is already enforced by encode/stitch; the
        // tier lookup here only feeds the passthrough decision.
        let _tier = reg.iter().find(|r| r.id == id).map(|r| r.tier.as_str());
        let row = match channel {
            Channel::Engine => normalize_engine_row(id, no, &w.bytes)?,
            Channel::O1ExdDosboxX => normalize_o1_row(id, no, &w.bytes)?,
            Channel::O2ExwWine => normalize_o2_row(id, no, &w.bytes)?,
            Channel::O3Street => {
                return Err(NormalizeError::UnsupportedChannel(channel));
            }
        };
        rows.push(row);
    }
    Ok(rows)
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
        | "linear-mission-m" | "selection-triple" | "blink-cursor" => {
            need(id, no, b, "u32", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "rng-state-a" | "rng-state-b" => {
            need(id, no, b, "u64", 8)?;
            Ok(row(vec![int("value", u64le(b) as i128)]))
        }
        "robot-bank" => robot_row_canonical(no, b),
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
            Ok(row(vec![
                int("len", n as i128),
                ("bytes".to_string(), FieldVal::Bytes(b[4..].to_vec())),
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
        "frame-counter" | "score" | "money" | "difficulty" | "zone" | "mission" | "mode"
        | "linear-mission-m" | "selection-triple" => {
            need(id, no, b, "u32 cell", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "rng-state-a" | "rng-state-b" => {
            // Channel-native state word (§6a): u32 LCG state zero
            // extends into the canonical u64.
            need(id, no, b, "u32 cell", 4)?;
            Ok(row(vec![int("value", u32le(b) as i128)]))
        }
        "robot-bank" => robot_row_from_map(id, no, b, EXD_ROBOT_MAP),
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
            // Raw w*h grid -> the §6a equivalence "len 0 == all-zero
            // grid": an all-zero blob canonicalizes to len 0 (the
            // ZONEA corpus shape until a death materializes the bank).
            if b.iter().all(|&x| x == 0) {
                Ok(row(vec![
                    int("len", 0),
                    ("bytes".to_string(), FieldVal::Bytes(vec![])),
                ]))
            } else {
                Ok(row(vec![
                    int("len", b.len() as i128),
                    ("bytes".to_string(), FieldVal::Bytes(b.to_vec())),
                ]))
            }
        }
        "static-map-wh" => {
            // 48-B span: h @+0x00 (cell 0x10748c), w @+0x2C (0x1074b8).
            need(id, no, b, "0x2c+4 span", 0x2c + 4)?;
            Ok(row(vec![
                int("w", u32le(&b[0x2c..]) as i128),
                int("h", u32le(b) as i128),
            ]))
        }
        "move-target-words" => Err(NormalizeError::UnpinnedForm { id: id.to_string() }),
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
        | "per-player-selected"
        | "typedb-fade-byte"
        | "armor-pad-reads" => normalize_o1_row(id, no, b),
        "robot-bank" => robot_row_from_map(id, no, b, EXW_ROBOT_MAP),
        // The EXW w/h cells are not the EXD 0x2c span — the O2 capture
        // form is W11's pin; until then the row normalizes to ZERO
        // fields (a per-field coverage finding, not a hard error and
        // never a guessed parse).
        "static-map-wh" => Ok(NormRow {
            id: id.to_string(),
            fields: Vec::new(),
        }),
        "move-target-words" => Err(NormalizeError::UnpinnedForm { id: id.to_string() }),
        _ => Ok(NormRow {
            id: id.to_string(),
            fields: vec![("raw".to_string(), FieldVal::Bytes(b.to_vec()))],
        }),
    }
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

/// The per-field comparison class for a (row, field) pair.
fn field_class(row: &str, field: &str, tier: &str) -> Class {
    match (row, field) {
        // Budgeted T2: the never-resetting counter + in-tick positions.
        ("frame-counter", _) => Class::T2Reported,
        // Statistical rows: never bit-compared (draw counts checked
        // separately).
        ("rng-state-a", _) | ("rng-state-b", _) => Class::AcceptedT3,
        ("robot-bank", "count") | ("move-target-words", "count") => Class::Structural,
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
                    // Row-level field compare.
                    let tier = tier_of(id);
                    let mut names: Vec<&str> = ra.fields.iter().map(|(n, _)| n.as_str()).collect();
                    for n in rb.fields.iter().map(|(n, _)| n.as_str()) {
                        if !names.contains(&n) {
                            names.push(n);
                        }
                    }
                    for name in names {
                        let (va, vb) = (ra.field(name), rb.field(name));
                        match (va, vb) {
                            (Some(va), Some(vb)) => {
                                compare_field(
                                    id,
                                    name,
                                    &tier,
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
                            (None, Some(_)) | (Some(_), None) => {
                                // Field-level coverage gap (e.g. robot
                                // fields the O1 map cannot source).
                                let key = format!("{id}.{name}");
                                let e = coverage_rows.entry(key).or_insert((0, 0, String::new()));
                                if va.is_none() {
                                    e.0 += 1;
                                } else {
                                    e.1 += 1;
                                }
                            }
                            (None, None) => {}
                        }
                    }
                }
                (None, None) => unreachable!("id came from one of the rows"),
            }
        }
    }

    // Coverage findings (deduped per row/field, with frame counts).
    for (key, (a_only, b_only, _)) in &coverage_rows {
        let (row, field) = match key.split_once('.') {
            Some((r, f)) => (r.to_string(), f.to_string()),
            None => (key.clone(), "(row)".to_string()),
        };
        findings.push(Finding {
            class: Class::Coverage,
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
            detail: "coverage: frames carried by one side only".into(),
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
#[allow(clippy::too_many_arguments)]
fn compare_field(
    id: &str,
    name: &str,
    tier: &str,
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
        Class::OriginalDivergence | Class::WatchArtifact | Class::Coverage => {
            // Never assigned by field comparison (coverage asymmetry
            // never reaches a value compare; the other two are
            // caller-triage labels).
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
