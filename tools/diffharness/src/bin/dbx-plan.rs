//! dbx-plan — compile a scenario + the watch registry into a D81 capgen
//! plan v2 (DESIGN-DIFFHARNESS.md §3/§10; RUNTIME.md "S0 live channel
//! mechanics" / D81).
//!
//! Everything here is DERIVED from `watches.toml` (the committed
//! registry): every watch address is parsed out of the row's `exd_addr`,
//! every fixed extent out of the row's `extent` string, and the two
//! code-breakpoint addresses out of the `frame-counter` / `s0-trigger`
//! rows. The per-row resolution table below only adds the FORM (fixed /
//! span / resolve-expression / deferred) and asserts the registry fields
//! it relies on — a registry edit that invalidates a row fails this
//! build loudly instead of silently emitting a stale address.
//!
//! Supported scenario tiers: T0 + TS (the S0 shape). T1+ extents need
//! the count-cell resolver (W5); passing such a scenario is an error.
//!
//! Usage:
//! ```text
//! dbx-plan <scenario.scen> [--out <capture-plan.json>]
//! ```
//! Default output: stdout. The committed artifact for review is
//! `capture-plans/<id>.json` (tests/plan_regen.rs pins byte-equality).

use diffharness::registry;
use diffharness::runner::{Scenario, Step};
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

/// The tiers this compiler can resolve today (the S0/S1 shape).
const SUPPORTED_TIERS: [&str; 3] = ["T0", "T1", "TS"];

// ------------------------------------------------------------- resolution

#[derive(Debug, Clone, PartialEq, Eq)]
enum Form {
    /// Fixed address + fixed length (both derived from the registry row).
    Fixed { addr: u64, len: u64 },
    /// Multi-cell row dumped as ONE contiguous span (the registry id
    /// stays unique in the transcript; the span layout is noted in the
    /// plan comment). `cells` are the row's parsed addresses.
    Span {
        base: u64,
        len: u64,
        cells: Vec<u64>,
    },
    /// Address read through a pointer cell at capture time.
    PtrCell { cell: u64, len_expr: String },
    /// Fixed base address + a length EXPRESSION over resolve symbols
    /// (the count-driven T1 bank form: base + count-cell resolve row).
    CountExpr { addr: u64, len_expr: String },
}

#[derive(Debug, Clone)]
struct RowPlan {
    id: String,
    form: Form,
}

#[derive(Debug)]
struct PlanError(String);

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dbx-plan: {}", self.0)
    }
}

fn die(msg: String) -> PlanError {
    PlanError(msg)
}

/// Parse the leading hex/decimal integer of a token like "0x1195f0",
/// "282" or "0x1f38 (999*8)" (stops at whitespace or '(' — deliberately
/// NOT at '-', so "0x62-stride rows" stays unparseable).
fn parse_int_prefix(token: &str) -> Option<u64> {
    let t = token.trim();
    let end = t
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(t.len());
    let s = &t[..end];
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Every hex/decimal cell address in an `exd_addr` expression
/// ("0x1074b8 / 0x10748c" -> both).
fn exd_cells(exd_addr: &str) -> Vec<u64> {
    exd_addr
        .split(|c: char| c.is_whitespace() || c == '/' || c == '(' || c == ')')
        .filter(|t| !t.is_empty())
        .filter_map(parse_int_prefix)
        .collect()
}

/// Fixed numeric extent: "4", "0x800 ring", "0x1f38 (999*8)" (leading
/// token), "282*0x4E" (product). None = symbolic (caller decides
/// deferral).
fn parse_extent(extent: &str) -> Option<u64> {
    if let Some(v) = parse_int_prefix(extent) {
        return Some(v);
    }
    let e = extent.trim();
    if let Some((a, b)) = e.split_once('*') {
        if !b.contains('*') {
            let (b, _) = b.split_once(' ').unwrap_or((b, ""));
            return match (parse_int_prefix(a), parse_int_prefix(b)) {
                (Some(x), Some(y)) => Some(x.checked_mul(y)?),
                _ => None,
            };
        }
    }
    None
}

/// The count-cell resolve symbol a count-driven bank row feeds.
fn count_symbol(id: &str) -> &'static str {
    match id {
        "robot-bank" => "robot_count",
        "trt-array" => "trt_count",
        "object-instances" => "obj_count",
        other => unreachable!("no count symbol for {other:?} (guard in resolve_row)"),
    }
}

/// Parse a "count*<stride>" / "<n>*<stride>" extent into its stride
/// token (hex or decimal), e.g. "count*0xA8" -> "0xA8",
/// "2000*0x14" -> "0x14".
fn extent_stride(extent: &str, id: &str) -> Result<String, PlanError> {
    let Some((count, stride)) = extent.trim().split_once('*') else {
        return Err(die(format!(
            "row {id} extent {:?} is not count*stride: update dbx-plan",
            extent
        )));
    };
    let stride = stride.trim();
    if parse_int_prefix(stride).is_none() {
        return Err(die(format!(
            "row {id} extent stride {stride:?} does not parse as an integer"
        )));
    }
    let _ = count; // "count" (symbolic) or a numeric cap — the live cell decides
    Ok(stride.to_string())
}

/// The map-w/h grid form: extent must mention w*h (symbolic); the
/// per-tile size is asserted against `expect` (the registry layout).
fn grid(id: &str, addr: u64, extent: &str, per_tile: &str) -> Result<Option<RowPlan>, PlanError> {
    let e = extent.trim();
    // per-tile size 1 is written "w*h" in the registry (no *1 tail)
    let want = if per_tile == "1" {
        "w*h".to_string()
    } else {
        format!("w*h*{per_tile}")
    };
    let head = e.split_whitespace().next().unwrap_or("");
    if head != want {
        return Err(die(format!(
            "row {id} extent {:?} no longer starts with {want:?}: update dbx-plan",
            extent
        )));
    }
    Ok(Some(RowPlan {
        id: id.to_string(),
        form: Form::CountExpr {
            addr,
            len_expr: if per_tile == "1" {
                "$map_w*$map_h".into()
            } else {
                format!("$map_w*$map_h*{per_tile}")
            },
        },
    }))
}

/// The per-row resolution table: form + the registry facts to ASSERT
/// (anti-ghost — a changed row fails the build, it never silently
/// re-emits). Deferred rows list the missing pin explicitly.
fn resolve_row(row: &diffharness::Watch) -> Result<Option<RowPlan>, PlanError> {
    let id = row.id.as_str();
    let plan = |form: Form| {
        Ok(Some(RowPlan {
            id: row.id.clone(),
            form,
        }))
    };

    // --- T0: every verified row is a fixed 4-byte cell read.
    if row.tier == "T0" {
        if row.exd_addr.is_empty() {
            return Ok(None); // explicit gap (difficulty / sfx gate): never dumped
        }
        if row.extent != "4" || row.indirect {
            return Err(die(format!(
                "T0 row {id} changed shape (extent {:?}, indirect {}): \
                 update dbx-plan's T0 form",
                row.extent, row.indirect
            )));
        }
        let addr = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
            die(format!(
                "T0 row {id} has no parsable exd_addr: {:?}",
                row.exd_addr
            ))
        })?;
        return plan(Form::Fixed { addr, len: 4 });
    }

    // --- T1: the P4 slice (robot/order/terrain banks). Gap rows are
    // skipped like T0 gaps; bank rows are count-driven (resolve rows
    // feed $symbols); grid rows derive their extent from map w/h.
    if row.tier == "T1" {
        if row.exd_addr.is_empty() {
            return Ok(None); // explicit gap (blink-cursor/no-extract-latch)
        }
        let cells = exd_cells(&row.exd_addr);
        let first = cells.first().copied().ok_or_else(|| {
            die(format!(
                "T1 row {id} has no parsable exd_addr: {:?}",
                row.exd_addr
            ))
        })?;
        return match id {
            // count-driven banks: extent "count*<stride>" + a count cell
            // named in exd_addr. [derived-pinned] the count cell is the
            // SECOND exd cell of the row (RE-EXD-MAP sec 5 bank rows).
            "robot-bank" | "trt-array" => {
                if cells.len() != 2 {
                    return Err(die(format!(
                        "row {id} exd_addr {:?} no longer carries base + count cell",
                        row.exd_addr
                    )));
                }
                let stride = extent_stride(&row.extent, id)?;
                let sym = count_symbol(id);
                plan(Form::CountExpr {
                    addr: cells[0],
                    len_expr: format!("${sym}*{stride}"),
                })
            }
            // partial alias: EXD covers the selected-idx cell only
            // (registry note; RE-EXD-MAP sec 5) — dump the 4 verified
            // bytes, never a fabricated 12-byte triple.
            "selection-triple" => plan(Form::Fixed {
                addr: first,
                len: 4,
            }),
            // click-order target {x,y,z}: 3 contiguous u32 cells =
            // one 12-byte span (W5-followup pinned 0x10e0a4/a8/ac).
            "order-target" => {
                if cells.len() != 3 {
                    return Err(die(format!(
                        "order-target exd_addr {:?} no longer has exactly 3 cells",
                        row.exd_addr
                    )));
                }
                let (lo, hi) = (
                    cells.iter().copied().min().expect("3 cells"),
                    cells.iter().copied().max().expect("3 cells"),
                );
                if hi - lo != 2 * 4 {
                    return Err(die(format!(
                        "order-target cells are no longer u32-spaced: {lo:#x}..{hi:#x}"
                    )));
                }
                plan(Form::Fixed {
                    addr: lo,
                    len: 2 * 4 + 4,
                })
            }
            "per-player-selected" => {
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row {id} extent {:?} stopped parsing as fixed: update dbx-plan",
                        row.extent
                    )));
                };
                plan(Form::Fixed { addr: first, len })
            }
            "beacon-family" => {
                if cells.len() != 5 {
                    return Err(die(format!(
                        "beacon-family exd_addr {:?} no longer has exactly 5 cells",
                        row.exd_addr
                    )));
                }
                let (lo, hi) = (
                    cells.iter().copied().min().expect("5 cells"),
                    cells.iter().copied().max().expect("5 cells"),
                );
                // [derived-pinned] five u16-spaced cells = a 10-byte span
                // (the registry layout gloss "u32 flag, u32 timer, 3 x
                // tile" notwithstanding, the EXW/EXD cell lists are
                // 2-byte spaced — RE-EXD-MAP sec 5 row).
                if hi - lo != 4 * 2 {
                    return Err(die(format!(
                        "beacon-family cells are no longer u16-spaced: {lo:#x}..{hi:#x}"
                    )));
                }
                plan(Form::Span {
                    base: lo,
                    len: 4 * 2 + 2,
                    cells,
                })
            }
            "spread-claims" => {
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row {id} extent {:?} stopped parsing as fixed: update dbx-plan",
                        row.extent
                    )));
                };
                plan(Form::Fixed { addr: first, len })
            }
            // map-w/h-driven grids: extent "w*h*K" -> $map_w*$map_h*K
            "tile-word-grid" | "platform-strength" => grid(id, first, &row.extent, "2"),
            "typedb-mirror-rows" => grid(id, first, &row.extent, "0x1E"),
            "typedb-fade-byte" | "armor-pad-reads" => grid(id, first, &row.extent, "1"),
            "variant-flag-bytes" => {
                if cells.len() != 2 || cells[1] - cells[0] != 1 {
                    return Err(die(format!(
                        "variant-flag-bytes exd_addr {:?} no longer has 2 adjacent cells",
                        row.exd_addr
                    )));
                }
                let lo = cells[0];
                // two adjacent per-tile byte planes -> one span
                plan(Form::CountExpr {
                    addr: lo,
                    len_expr: "(2*$map_w*$map_h)+1".into(),
                })
            }
            "object-instances" => {
                if !row.indirect || cells.len() != 2 {
                    return Err(die(format!(
                        "object-instances lost its pointer+count cell pair \
                         (indirect {}, exd_addr {:?})",
                        row.indirect, row.exd_addr
                    )));
                }
                let stride = extent_stride(&row.extent, id)?;
                plan(Form::PtrCell {
                    cell: cells[0],
                    len_expr: format!("$obj_count*{stride}"),
                })
            }
            "move-target-words" => {
                // [derived-pinned] (D90, RE-EXD-MAP sec 5 row): the
                // x/y u32 array pair 0x30 apart — the fixed 0x60-B
                // span at the x base covers x[12]+y[12]; the per-robot
                // bound is the cap cell 0x11950c (≤ 12) and the differ
                // bounds by the same frame's robot-bank count.
                if cells.len() != 2 || cells[1] - cells[0] != 0x30 {
                    return Err(die(format!(
                        "move-target-words exd_addr {:?} no longer carries \
                         the x/y array pair 0x30 apart",
                        row.exd_addr
                    )));
                }
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row {id} extent {:?} stopped parsing as fixed: update dbx-plan",
                        row.extent
                    )));
                };
                if len != 0x60 {
                    return Err(die(format!(
                        "move-target-words extent is {len:#x}, not the pinned \
                         0x60 x[12]+y[12] span: update dbx-plan"
                    )));
                }
                plan(Form::Span {
                    base: cells[0],
                    len: 0x60,
                    cells,
                })
            }
            other => Err(die(format!(
                "T1 registry row {other:?} has no dbx-plan resolution form"
            ))),
        };
    }

    // --- TS fixed-extent rows.
    match id {
        "static-type-table" | "static-pad-slots" | "static-player-type" | "static-dither-noise" => {
            let Some(len) = parse_extent(&row.extent) else {
                return Err(die(format!(
                    "row {id} extent {:?} stopped parsing as fixed: update dbx-plan",
                    row.extent
                )));
            };
            let addr = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                die(format!(
                    "row {id} has no parsable exd_addr: {:?}",
                    row.exd_addr
                ))
            })?;
            plan(Form::Fixed { addr, len })
        }
        // Two-cell rows dumped as one contiguous span.
        "static-map-wh" => {
            let cells = exd_cells(&row.exd_addr);
            if cells.len() != 2 {
                return Err(die(format!(
                    "static-map-wh exd_addr {:?} no longer has exactly 2 cells",
                    row.exd_addr
                )));
            }
            let (lo, hi) = (cells[0].min(cells[1]), cells[0].max(cells[1]));
            // [derived-pinned] w 0x1074b8 / h 0x10748c are 0x2c apart
            // (RE-EXD-MAP sec 5b); the span covers both u32s.
            if hi - lo != 0x2c {
                return Err(die(format!(
                    "static-map-wh cells are no longer 0x2c apart: {lo:#x}..{hi:#x}"
                )));
            }
            plan(Form::Span {
                base: lo,
                len: 0x2c + 4,
                cells,
            })
        }
        "static-cursor-clamp" => {
            let cells = exd_cells(&row.exd_addr);
            if cells.len() != 2 {
                return Err(die(format!(
                    "static-cursor-clamp exd_addr {:?} no longer has exactly 2 cells",
                    row.exd_addr
                )));
            }
            let (lo, hi) = (cells[0].min(cells[1]), cells[0].max(cells[1]));
            if hi - lo != 4 {
                return Err(die(format!(
                    "static-cursor-clamp cells are no longer contiguous: {lo:#x}..{hi:#x}"
                )));
            }
            plan(Form::Span {
                base: lo,
                len: 8,
                cells,
            })
        }
        // Pointer-cell rows: volume extents from the loader statics.
        "static-tot-volume" => {
            if !row.indirect {
                return Err(die("static-tot-volume lost its indirect flag".into()));
            }
            let cell = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                die(format!(
                    "static-tot-volume has no parsable exd_addr: {:?}",
                    row.exd_addr
                ))
            })?;
            // TOT volume = u16 w + u16 h + 8 planes * w*h u16
            // (FORMATS-MISSION sec 2: "u16 w + u16 h + 8 x w*h u16 planes").
            plan(Form::PtrCell {
                cell,
                len_expr: "4+16*$map_w*$map_h".into(),
            })
        }
        "static-dat-volume" => {
            if !row.indirect {
                return Err(die("static-dat-volume lost its indirect flag".into()));
            }
            let cell = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                die(format!(
                    "static-dat-volume has no parsable exd_addr: {:?}",
                    row.exd_addr
                ))
            })?;
            // DAT volume = u16 w + u16 h + 8 planes * w*h u8
            // (FORMATS-MISSION sec 4: "u16 w + u16 h + w*h*8").
            plan(Form::PtrCell {
                cell,
                len_expr: "4+8*$map_w*$map_h".into(),
            })
        }
        "static-claim-bank" => {
            if !row.indirect {
                return Err(die("static-claim-bank lost its indirect flag".into()));
            }
            let cell = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                die(format!(
                    "static-claim-bank has no parsable exd_addr: {:?}",
                    row.exd_addr
                ))
            })?;
            let Some(len) = parse_extent(&row.extent) else {
                return Err(die(format!(
                    "static-claim-bank extent {:?} stopped parsing: update dbx-plan",
                    row.extent
                )));
            };
            plan(Form::PtrCell {
                cell,
                len_expr: format!("{len}"),
            })
        }
        // Deferred: extent formulas not yet pinned to RE facts.
        "static-cgr-volume" | "static-bin-terrain" | "static-min-bank" => {
            if row.extent != "bank-sized" {
                return Err(die(format!(
                    "row {id} extent {:?} changed from bank-sized: if the bank \
                     size is now pinned, resolve it in dbx-plan instead of deferring",
                    row.extent
                )));
            }
            Ok(None)
        }
        "static-lnk-map" => {
            if row.extent != "map-sized" {
                return Err(die(format!(
                    "static-lnk-map extent {:?} changed from map-sized: resolve \
                     it in dbx-plan if the in-memory size is now pinned",
                    row.extent
                )));
            }
            Ok(None)
        }
        "static-order-table" => {
            if row.extent != "0x62-stride rows" {
                return Err(die(format!(
                    "static-order-table extent {:?} changed: resolve the row \
                     count in dbx-plan if now pinned",
                    row.extent
                )));
            }
            Ok(None)
        }
        "static-yline-zbase" => {
            if row.extent != "table-sized" {
                return Err(die(format!(
                    "static-yline-zbase extent {:?} changed: resolve the table \
                     sizes in dbx-plan if now pinned",
                    row.extent
                )));
            }
            Ok(None)
        }
        other => Err(die(format!(
            "registry row {other:?} (tier {}) has no dbx-plan resolution form",
            row.tier
        ))),
    }
}

// ------------------------------------------------------- W5 step compile

/// The seam row id a boot key writes (§5.5 keys are registry rows).
fn boot_row_id(key: &str) -> Option<&'static str> {
    match key {
        "difficulty" => Some("difficulty"),
        _ => None,
    }
}

/// The registry rows an injection step WRITES; every one must carry an
/// EXD alias or the step cannot compile for O1 (anti-ghost: gaps are
/// named, never fabricated).
fn step_rows(step: &Step) -> &'static [&'static str] {
    match step {
        Step::Keystore { .. } => &["inj-key-state"],
        Step::Order { .. } | Step::Pad { .. } => &["order-target"],
        Step::Command { .. } => &["inj-command-ring", "inj-command-count"],
        Step::Boot { .. } | Step::Advance { .. } | Step::Capture | Step::UntilAnchor { .. } => &[],
    }
}

/// One injected write as a plan JSON body (no trailing comma).
#[derive(Debug, Clone)]
enum InjectWrite {
    Plain {
        frame: Option<u64>,
        addr: String,
        bytes: String,
    },
    Command {
        frame: u64,
        base: String,
        count_cell: String,
        bytes: String,
    },
    /// The DESIGN §5.4 pad op (D86): read the .PAD slot record from the
    /// bank at the capture-frame stop, validate the loader marks, write
    /// {x,y,z} i32-LE x3 to the order-target triple (capgen applies it;
    /// the addresses are registry-derived like every inject row).
    Pad {
        frame: u64,
        bank: String,
        slot: u32,
        target: [String; 3],
    },
}

/// One WALK-phase write (D84): a plain seam write applied at walk stop
/// `stop` (BPLM-on-frame-counter hit; one stop per counter-writing
/// screen frame — the write becomes the NEXT screen frame's input).
/// Literal addresses only: resolve ($symbols) runs at the anchor, after
/// the walk.
#[derive(Debug, Clone)]
struct WalkWrite {
    stop: u64,
    addr: String,
    bytes: String,
}

/// Calibration rows dumped at EVERY walk stop (D84): registry-anchored
/// T0 screen-state cells; values ride the transcript as comments so a
/// calibration run maps menu transitions to stop indices mechanically.
#[derive(Debug, Clone)]
struct WalkWatch {
    id: String,
    addr: String,
    len: u32,
}

/// The compiled plan sections (DESIGN §5 vocabulary): frame-0 boot
/// writes, anchor-relative mission inject rows, stop-indexed walk rows.
type CompiledSteps = (Vec<InjectWrite>, Vec<InjectWrite>, Vec<WalkWrite>);

fn compile_steps(scen: &Scenario, reg: &[diffharness::Watch]) -> Result<CompiledSteps, PlanError> {
    // Walk phase (before until-anchor): BOOT writes at frame 0 + the
    // SCRIPTED MENU WALK (D84) — stop-indexed keystore writes, one stop
    // per counter-writing screen frame, re-armed per input because the
    // AnyKeyWait twin consumes bytes on read. ORDER/PAD/COMMAND are
    // mission-phase seams — the menu walk is keyboard-driven.
    let (walk, mission) = scen.phases();
    let mut boot: Vec<InjectWrite> = Vec::new();
    let mut walk_rows: Vec<WalkWrite> = Vec::new();
    let mut walk_boundary: u64 = 0;
    for step in walk {
        match step {
            Step::Boot { key, value } => {
                let Some(row_id) = boot_row_id(key) else {
                    return Err(die(format!("boot key {key:?} has no seam row")));
                };
                let row = reg.iter().find(|r| r.id == row_id).ok_or_else(|| {
                    die(format!("boot key {key:?}: registry row {row_id:?} missing"))
                })?;
                let cells = exd_cells(&row.exd_addr);
                if cells.is_empty() {
                    return Err(die(format!(
                        "boot write {key} ({row_id}): EXD alias is a registry gap \
                         (status {:?}) — the O1 plan never fabricates its address",
                        row.exd_status
                    )));
                }
                boot.push(InjectWrite::Plain {
                    frame: None,
                    addr: format!("CS:{:08X}", cells[0]),
                    bytes: (*value as u32)
                        .to_le_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                });
            }
            Step::Keystore { entries } => {
                walk_boundary += 1;
                let row = reg
                    .iter()
                    .find(|r| r.id == "inj-key-state")
                    .ok_or_else(|| die("registry row inj-key-state missing".into()))?;
                let base = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                    die(
                        "walk keystore step: inj-key-state EXD alias is a registry gap \
                         (status {:?}) — anchor it before this scenario can compile for O1"
                            .to_string(),
                    )
                })?;
                for (scan, val) in entries {
                    walk_rows.push(WalkWrite {
                        stop: walk_boundary,
                        addr: format!("CS:{:08X}", base + *scan as u64),
                        bytes: format!("{val:02x}"),
                    });
                }
            }
            Step::Order { .. } | Step::Pad { .. } | Step::Command { .. } => {
                return Err(die(
                    "walk-phase order/pad/command steps are not menu-walk steps: the \
                     walk phase supports boot + keystore only (the title menu is \
                     keyboard-driven; order/command are mission-phase seams — DESIGN §5)"
                        .into(),
                ))
            }
            Step::Advance { frames } => {
                walk_boundary += frames;
            }
            Step::Capture | Step::UntilAnchor { .. } => {}
        }
    }
    if walk_boundary > 1_000_000 {
        return Err(die(format!(
            "walk schedule exceeds 1,000,000 stops ({walk_boundary}) — a runaway \
             `step` count would stall the capture run on the plan time limit"
        )));
    }

    // Mission phase: anchor-relative boundary numbering = capture frame
    // numbers (anchor frame = 1).
    let mut out: Vec<InjectWrite> = Vec::new();
    let mut boundary: u64 = 1;
    let frames_total = scen.frames + 1;
    for step in mission {
        let writes = match step {
            Step::Advance { frames } => {
                boundary += frames;
                continue;
            }
            Step::Capture | Step::UntilAnchor { .. } => continue,
            Step::Boot { key, .. } => {
                return Err(die(format!(
                    "boot step {key} is walk-phase only (before until-anchor; with \
                     no until-anchor the whole schedule is mission phase)"
                )))
            }
            Step::Keystore { entries } => {
                let row = reg
                    .iter()
                    .find(|r| r.id == "inj-key-state")
                    .ok_or_else(|| die("registry row inj-key-state missing".into()))?;
                let base = exd_cells(&row.exd_addr).first().copied().ok_or_else(|| {
                    die("keystore step: inj-key-state EXD alias is a registry gap".into())
                })?;
                entries
                    .iter()
                    .map(|(scan, val)| InjectWrite::Plain {
                        frame: Some(boundary),
                        addr: format!("CS:{:08X}", base + *scan as u64),
                        bytes: format!("{val:02x}"),
                    })
                    .collect::<Vec<_>>()
            }
            Step::Order { x, y, z } => {
                let row = reg
                    .iter()
                    .find(|r| r.id == "order-target")
                    .ok_or_else(|| die("registry row order-target missing".into()))?;
                let cells = exd_cells(&row.exd_addr);
                if cells.len() != 3 {
                    return Err(die(
                        "order step: order-target EXD alias is a registry gap (needs \
                         all three xyz cells)"
                            .into(),
                    ));
                }
                [*x, *y, *z]
                    .into_iter()
                    .zip(cells)
                    .map(|(v, cell)| InjectWrite::Plain {
                        frame: Some(boundary),
                        addr: format!("CS:{cell:08X}"),
                        bytes: (v as u32)
                            .to_le_bytes()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect(),
                    })
                    .collect::<Vec<_>>()
            }
            Step::Pad { slot } => {
                // The DESIGN §5.4 pad op (D86): the write target is the
                // order-target triple with the tile READ from the pad
                // bank at runtime. The bank is the step's READ anchor —
                // its own explicit gap error, distinct from the
                // step_rows WRITE-seam rule; the triple cells follow
                // the Order step's 3-cell rule.
                let bank_row = reg
                    .iter()
                    .find(|r| r.id == "static-pad-slots")
                    .ok_or_else(|| die("registry row static-pad-slots missing".into()))?;
                let bank = exd_cells(&bank_row.exd_addr)
                    .first()
                    .copied()
                    .ok_or_else(|| {
                        die(format!(
                            "pad step: static-pad-slots EXD alias is a registry gap \
                         (status {:?}) — the pad bank is the step's READ anchor and \
                         is never fabricated",
                            bank_row.exd_status
                        ))
                    })?;
                let row = reg
                    .iter()
                    .find(|r| r.id == "order-target")
                    .ok_or_else(|| die("registry row order-target missing".into()))?;
                let cells = exd_cells(&row.exd_addr);
                if cells.len() != 3 {
                    return Err(die(
                        "pad step: order-target EXD alias is a registry gap (needs \
                         all three xyz cells)"
                            .into(),
                    ));
                }
                if *slot > 998 {
                    return Err(die(format!(
                        "pad step slot {slot} out of range 0..998 (999 .PAD slots)"
                    )));
                }
                vec![InjectWrite::Pad {
                    frame: boundary,
                    bank: format!("CS:{bank:08X}"),
                    slot: *slot,
                    target: core::array::from_fn(|i| format!("CS:{:08X}", cells[i])),
                }]
            }
            Step::Command { bytes } => {
                let ring = reg
                    .iter()
                    .find(|r| r.id == "inj-command-ring")
                    .ok_or_else(|| die("registry row inj-command-ring missing".into()))?;
                let count = reg
                    .iter()
                    .find(|r| r.id == "inj-command-count")
                    .ok_or_else(|| die("registry row inj-command-count missing".into()))?;
                let base = exd_cells(&ring.exd_addr).first().copied().ok_or_else(|| {
                    die("command step: inj-command-ring EXD alias is a registry gap".into())
                })?;
                let cell = exd_cells(&count.exd_addr).first().copied().ok_or_else(|| {
                    die("command step: inj-command-count EXD alias is a registry gap".into())
                })?;
                vec![InjectWrite::Command {
                    frame: boundary,
                    base: format!("CS:{base:08X}"),
                    count_cell: format!("CS:{cell:08X}"),
                    bytes: bytes.iter().map(|b| format!("{b:02x}")).collect(),
                }]
            }
        };
        // every step row must be aliasable (anti-ghost gate)
        for row_id in step_rows(step) {
            let row = reg
                .iter()
                .find(|r| r.id == *row_id)
                .ok_or_else(|| die(format!("registry row {row_id:?} missing")))?;
            if exd_cells(&row.exd_addr).is_empty() {
                return Err(die(format!(
                    "injection step on seam {row_id} ({}): EXD alias is a registry \
                     gap (status {:?}) — anchor it before this scenario can compile \
                     for O1; the engine side (W6) consumes the step directly",
                    step_kind(step),
                    row.exd_status
                )));
            }
        }
        if boundary > frames_total {
            return Err(die(format!(
                "injection step at boundary {boundary} is past the capture window \
                 (frames={} -> {} records)",
                scen.frames, frames_total
            )));
        }
        out.extend(writes);
        boundary += 1;
    }
    Ok((boot, out, walk_rows))
}

fn step_kind(step: &Step) -> &'static str {
    match step {
        Step::Keystore { .. } => "keystore",
        Step::Order { .. } => "order",
        Step::Pad { .. } => "pad",
        Step::Command { .. } => "command",
        Step::Boot { .. } => "boot",
        Step::Advance { .. } => "advance",
        Step::Capture => "capture",
        Step::UntilAnchor { .. } => "until-anchor",
    }
}

fn inject_json(w: &InjectWrite) -> String {
    match w {
        InjectWrite::Plain { frame, addr, bytes } => match frame {
            Some(f) => format!(
                "    {{ \"frame\": {f}, \"addr\": \"{}\", \"bytes\": \"{bytes}\" }}",
                jstr(addr).trim_matches('"')
            ),
            None => format!(
                "    {{ \"addr\": \"{}\", \"bytes\": \"{bytes}\" }}",
                jstr(addr).trim_matches('"')
            ),
        },
        InjectWrite::Command {
            frame,
            base,
            count_cell,
            bytes,
        } => format!(
            "    {{ \"frame\": {frame}, \"op\": \"command\", \"base\": \"{}\", \
             \"stride\": 128, \"count_cell\": \"{}\", \"bytes\": \"{bytes}\" }}",
            jstr(base).trim_matches('"'),
            jstr(count_cell).trim_matches('"')
        ),
        InjectWrite::Pad {
            frame,
            bank,
            slot,
            target,
        } => format!(
            "    {{ \"frame\": {frame}, \"op\": \"pad\", \"bank\": \"{}\", \"slot\": {slot}, \
             \"target\": [\"{}\", \"{}\", \"{}\"] }}",
            jstr(bank).trim_matches('"'),
            jstr(&target[0]).trim_matches('"'),
            jstr(&target[1]).trim_matches('"'),
            jstr(&target[2]).trim_matches('"')
        ),
    }
}

fn walk_json(w: &WalkWrite) -> String {
    format!(
        "    {{ \"stop\": {}, \"addr\": \"{}\", \"bytes\": \"{}\" }}",
        w.stop,
        jstr(&w.addr).trim_matches('"'),
        w.bytes
    )
}

/// The fixed calibration trio for walk scenarios: the T0 screen-state
/// cells (mode/zone/mission), every address registry-derived (the
/// anti-ghost rule holds for calibration rows too).
fn walk_watches(reg: &[diffharness::Watch]) -> Result<Vec<WalkWatch>, PlanError> {
    let mut out = Vec::new();
    for (row_id, cal_id) in [
        ("mode", "walk-mode"),
        ("zone", "walk-zone"),
        ("mission", "walk-mission"),
    ] {
        let row = reg
            .iter()
            .find(|r| r.id == row_id)
            .ok_or_else(|| die(format!("walk calibration row {row_id:?} missing")))?;
        let Some(cell) = exd_cells(&row.exd_addr).first().copied() else {
            return Err(die(format!(
                "walk calibration row {row_id}: EXD alias is a registry gap \
                 (status {:?}) — calibration rows never fabricate addresses",
                row.exd_status
            )));
        };
        out.push(WalkWatch {
            id: cal_id.to_string(),
            addr: format!("CS:{cell:08X}"),
            len: 4,
        });
    }
    Ok(out)
}

/// Every `$name` referenced by a len expression.
fn expr_symbols(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in expr.split('$').skip(1) {
        let end = name
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(name.len());
        let sym = name[..end].to_string();
        if !sym.is_empty() && !out.contains(&sym) {
            out.push(sym);
        }
    }
    out
}

// ----------------------------------------------------------------- emit

fn jstr(s: &str) -> String {
    // Zero-dep emitter: our strings are ASCII without quotes/backslashes
    // (asserted — anything else needs a real escaper, not silent pass-through).
    assert!(
        s.is_ascii() && !s.contains('"') && !s.contains('\\') && !s.contains('\n'),
        "plan string needs escaping: {s:?}"
    );
    format!("\"{s}\"")
}

struct Emitted {
    json: String,
    deferred: Vec<String>,
    anchor_count: usize,
    frame_count: usize,
    inject_count: usize,
    walk_count: usize,
}

fn emit_plan(scen: &Scenario, reg: &[diffharness::Watch]) -> Result<Emitted, PlanError> {
    // Tier gate: only the S0 shape today.
    for t in &scen.tiers {
        if !SUPPORTED_TIERS.contains(&t.as_str()) {
            return Err(die(format!(
                "scenario {} tier {t:?} is not compilable yet: T2+ watches have \
                 no EXD aliases yet (dbx-plan supports {:?})",
                scen.id, SUPPORTED_TIERS
            )));
        }
    }

    // W5 step compilation (DESIGN §5): boot writes + frame-boundary
    // inject rows + the D84 walk rows (the scripted menu walk), all
    // gated on registry aliases (gaps are named, never fabricated).
    // Emitted keys exist only when the scenario carries them (S0/S1
    // artifacts stay minimal).
    let (boot_writes, inject_rows, walk_rows) = compile_steps(scen, reg)?;
    let walk_cal = if walk_rows.is_empty() {
        Vec::new()
    } else {
        walk_watches(reg)?
    };
    let mut step_json = String::new();
    if !walk_rows.is_empty() {
        step_json.push_str("  \"walk\": [\n");
        for (i, w) in walk_rows.iter().enumerate() {
            step_json.push_str(&walk_json(w));
            step_json.push_str(if i + 1 < walk_rows.len() { ",\n" } else { "\n" });
        }
        step_json.push_str("  ],\n");
        step_json.push_str("  \"walk_watches\": [\n");
        for (i, w) in walk_cal.iter().enumerate() {
            step_json.push_str(&format!(
                "    {{ \"id\": {}, \"addr\": \"{}\", \"len\": {} }}",
                jstr(&w.id),
                w.addr,
                w.len
            ));
            step_json.push_str(if i + 1 < walk_cal.len() { ",\n" } else { "\n" });
        }
        step_json.push_str("  ],\n");
    }
    if !boot_writes.is_empty() {
        step_json.push_str("  \"boot_writes\": [\n");
        for (i, w) in boot_writes.iter().enumerate() {
            step_json.push_str(&inject_json(w));
            step_json.push_str(if i + 1 < boot_writes.len() {
                ",\n"
            } else {
                "\n"
            });
        }
        step_json.push_str("  ],\n");
    }
    if !inject_rows.is_empty() {
        step_json.push_str("  \"inject\": [\n");
        for (i, w) in inject_rows.iter().enumerate() {
            step_json.push_str(&inject_json(w));
            step_json.push_str(if i + 1 < inject_rows.len() {
                ",\n"
            } else {
                "\n"
            });
        }
        step_json.push_str("  ],\n");
    }
    let inject_count = inject_rows.len();

    // The two registry anchors of the live flow (anti-ghost: derived, not typed).
    let frame_counter = reg
        .iter()
        .find(|r| r.id == "frame-counter" && r.tier == "T0" && !r.exd_addr.is_empty())
        .ok_or_else(|| die("registry row frame-counter (T0) missing".into()))?;
    let trigger = reg
        .iter()
        .find(|r| r.id == "s0-trigger" && r.tier == "S0" && !r.exd_addr.is_empty())
        .ok_or_else(|| die("registry row s0-trigger (S0) missing".into()))?;
    let fc_cell = exd_cells(&frame_counter.exd_addr)
        .first()
        .copied()
        .ok_or_else(|| die("frame-counter exd_addr does not parse".into()))?;
    let tail = exd_cells(&trigger.exd_addr)
        .first()
        .copied()
        .ok_or_else(|| die("s0-trigger exd_addr does not parse".into()))?;

    // Resolve rows (registry order preserved).
    let mut anchor: Vec<RowPlan> = Vec::new();
    let mut per_frame: Vec<RowPlan> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    for row in reg {
        if !scen.tiers.contains(&row.tier) {
            continue; // e.g. the S0 trigger row: not a dump row
        }
        match resolve_row(row)? {
            Some(p) => {
                if row.tier == "TS" {
                    anchor.push(p);
                } else {
                    anchor.push(p.clone());
                    per_frame.push(p);
                }
            }
            None => deferred.push(format!("{} ({})", row.id, row.extent)),
        }
    }

    // Resolve cells: the map w/h loader statics + the pointer cells of
    // every PtrCell row above (each derived from ITS row's exd_addr).
    let map_wh = reg
        .iter()
        .find(|r| r.id == "static-map-wh")
        .ok_or_else(|| die("registry row static-map-wh missing".into()))?;
    let map_cells = exd_cells(&map_wh.exd_addr);
    if map_cells.len() != 2 {
        return Err(die(format!(
            "static-map-wh exd_addr {:?} no longer has exactly 2 cells",
            map_wh.exd_addr
        )));
    }
    // [derived-pinned] slash order = w / h (RE-EXD-MAP sec 5b
    // "w 0x1074b8 / h 0x10748c"); the first cell is the larger one
    // (the span asserts the 0x2c gap).
    let (w_cell, h_cell) = (map_cells[0], map_cells[1]);
    if w_cell <= h_cell {
        return Err(die(format!(
            "static-map-wh cell order changed: expected w > h, got {w_cell:#x}/{h_cell:#x}"
        )));
    }
    let mut resolve: Vec<(String, u64)> = vec![("map_w".into(), w_cell), ("map_h".into(), h_cell)];
    for p in &anchor {
        if let Form::PtrCell { cell, .. } = p.form {
            let name = match p.id.as_str() {
                "static-tot-volume" => "tot_ptr",
                "static-dat-volume" => "dat_ptr",
                "static-claim-bank" => "claim_ptr",
                "object-instances" => "obj_ptr",
                other => {
                    return Err(die(format!(
                        "PtrCell row {other:?} has no resolve symbol in dbx-plan"
                    )))
                }
            };
            resolve.push((name.into(), cell));
        }
    }
    // Every $symbol referenced by any len expression (CountExpr bank
    // rows AND PtrCell lens) must carry a resolve row: count cells come
    // from the bank row's own exd_addr (second cell), map w/h from
    // static-map-wh.
    let mut lens: Vec<&str> = Vec::new();
    for p in &anchor {
        match &p.form {
            Form::CountExpr { len_expr, .. } | Form::PtrCell { len_expr, .. } => {
                lens.push(len_expr)
            }
            Form::Fixed { .. } | Form::Span { .. } => {}
        }
    }
    for name in lens.iter().flat_map(|e| expr_symbols(e)) {
        if resolve.iter().any(|(n, _)| n == &name) {
            continue;
        }
        let cell = match name.as_str() {
            "robot_count" | "trt_count" | "obj_count" => {
                let row_id = match name.as_str() {
                    "robot_count" => "robot-bank",
                    "trt_count" => "trt-array",
                    _ => "object-instances",
                };
                let row = reg
                    .iter()
                    .find(|r| r.id == row_id)
                    .ok_or_else(|| die(format!("symbol ${name} has no source registry row")))?;
                exd_cells(&row.exd_addr).get(1).copied().ok_or_else(|| {
                    die(format!(
                        "row {} exd_addr {:?} lost its count cell",
                        row.id, row.exd_addr
                    ))
                })?
            }
            other => {
                return Err(die(format!(
                    "len expression references unknown symbol ${other}"
                )))
            }
        };
        resolve.push((name, cell));
    }

    let watch_json = |p: &RowPlan| -> String {
        match &p.form {
            Form::Fixed { addr, len } => format!(
                "    {{ \"id\": {}, \"addr\": \"CS:{addr:08X}\", \"len\": {len} }}",
                jstr(&p.id)
            ),
            Form::Span { base, len, .. } => format!(
                "    {{ \"id\": {}, \"addr\": \"CS:{base:08X}\", \"len\": {len} }}",
                jstr(&p.id)
            ),
            Form::PtrCell { len_expr, .. } => {
                let sym = match p.id.as_str() {
                    "static-tot-volume" => "tot_ptr",
                    "static-dat-volume" => "dat_ptr",
                    "static-claim-bank" => "claim_ptr",
                    "object-instances" => "obj_ptr",
                    _ => unreachable!("checked above"),
                };
                format!(
                    "    {{ \"id\": {}, \"addr\": \"CS:${sym}\", \"len\": {} }}",
                    jstr(&p.id),
                    jstr(len_expr)
                )
            }
            Form::CountExpr { addr, len_expr } => format!(
                "    {{ \"id\": {}, \"addr\": \"CS:{addr:08X}\", \"len\": {} }}",
                jstr(&p.id),
                jstr(len_expr)
            ),
        }
    };

    let mut j = String::new();
    j.push_str("{\n");
    j.push_str(&format!(
        "  \"_comment\": \"{} live capture plan (D81/D84; GENERATED by dbx-plan from watches.toml - do not hand-edit, regenerate). Boot trap: BPLM {fc_cell:X} (the frame-counter cell) armed at the parked pre-boot halt fires on the first post-boot write. Arm stop: SELINFO CS flat guard (base==0), then BP CS:{tail:08X} = the registry s0-trigger row (the BP ack echoes the numeric selector - the per-run pin). resolve_at=anchor: the loader statics (map w/h, TOT/DAT/claim pointers) are MISSION-load values - they are read at the anchor stop (mission start), never at the pre-mission arm stop (D84). WALK phase (D84, when the walk key is present): the BPLM stays armed after the accept stop; one stop per counter-writing screen frame; stop i applies its rows via SMV (they become screen frame i+1 input - keystore writes need re-arm per input, the AnyKeyWait twin consumes on read); arm_commands run at the LAST walk stop (BPDEL * drops the BPLM, BP arms the anchor); walk_watches are calibration dumps at every stop riding the transcript as comments. Anchor frame = the first BP hit after arm; alignment is by the frame-counter watch. TS statics ride the anchor frame; T0 rows every frame. Deferred TS rows carry unpinned extents (see _deferred). Without a walk key the operator walks the title menu on the desktop; the anchor frame-counter and RNG bytes are menu-timing dependent across runs (T2/T3 classes, DESIGN section 6) - the live double-run verdict is identical-chains-modulo-those-cells; byte-identical chains need the scripted walk (S0W).\",\n",
        scen.id
    ));
    j.push_str("  \"logfile\": \"dosbox-harness.log\",\n");
    j.push_str("  \"time_limit\": 1800,\n");
    j.push_str("  \"boot_timeout\": 1800,\n");
    j.push_str("  \"boot_retries\": 24,\n");
    j.push_str(&format!("  \"frames\": {},\n", scen.frames + 1));
    j.push_str("  \"resolve_at\": \"anchor\",\n");
    j.push_str("  \"env\": { \"SDL_VIDEODRIVER\": \"\", \"SDL_AUDIODRIVER\": \"dummy\" },\n");
    j.push_str("  \"boot_commands\": [\n");
    j.push_str(&format!(
        "    {{ \"cmd\": \"BPLM {:X}\", \"expect\": \"Set linear memory breakpoint at {fc_cell:08X}\" }}\n",
        fc_cell
    ));
    j.push_str("  ],\n");
    j.push_str("  \"arm_commands\": [\n");
    j.push_str("    { \"cmd\": \"BPDEL *\", \"expect\": \"Breakpoints deleted\" },\n");
    j.push_str(&format!(
        "    {{ \"cmd\": \"BP CS:{tail:08X}\", \"expect\": \"Set breakpoint at\" }}\n",
    ));
    j.push_str("  ],\n");
    j.push_str("  \"resolve\": [\n");
    for (i, (name, cell)) in resolve.iter().enumerate() {
        j.push_str(&format!(
            "    {{ \"name\": {}, \"addr\": \"CS:{cell:08X}\", \"len\": 4 }}{}",
            jstr(name),
            if i + 1 < resolve.len() { ",\n" } else { "\n" }
        ));
    }
    j.push_str("  ],\n");
    j.push_str("  \"anchor_watches\": [\n");
    for (i, p) in anchor.iter().enumerate() {
        j.push_str(&watch_json(p));
        j.push_str(if i + 1 < anchor.len() { ",\n" } else { "\n" });
    }
    j.push_str("  ],\n");
    j.push_str("  \"watches\": [\n");
    for (i, p) in per_frame.iter().enumerate() {
        j.push_str(&watch_json(p));
        j.push_str(if i + 1 < per_frame.len() { ",\n" } else { "\n" });
    }
    j.push_str("  ],\n");
    j.push_str(&step_json);
    j.push_str("  \"_deferred\": [\n");
    for (i, d) in deferred.iter().enumerate() {
        j.push_str(&format!(
            "    {}{}",
            jstr(d),
            if i + 1 < deferred.len() { ",\n" } else { "\n" }
        ));
    }
    j.push_str("  ]\n");
    j.push_str("}\n");
    Ok(Emitted {
        json: j,
        deferred,
        anchor_count: anchor.len(),
        frame_count: per_frame.len(),
        inject_count,
        walk_count: walk_rows.len(),
    })
}

// ----------------------------------------------------------------- main

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let scen_path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("usage: dbx-plan <scenario.scen> [--out <capture-plan.json>]");
            return ExitCode::FAILURE;
        }
    };
    let mut out_path: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => match args.next() {
                Some(p) => out_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("dbx-plan: --out needs a path");
                    return ExitCode::FAILURE;
                }
            },
            other => {
                eprintln!("dbx-plan: unknown argument {other:?}");
                return ExitCode::FAILURE;
            }
        }
    }

    let scen_src = match std::fs::read_to_string(&scen_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("dbx-plan: cannot read {scen_path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let scen = match Scenario::parse(&scen_src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("dbx-plan: {e}");
            return ExitCode::FAILURE;
        }
    };
    let reg = registry();
    let emitted = match emit_plan(&scen, &reg) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "dbx-plan: scenario {} -> {} anchor rows + {} per-frame rows, {} deferred, {} inject rows, {} walk rows; \
         frames={} (anchor + {} post-anchor records for the stitcher)",
        scen.id,
        emitted.anchor_count,
        emitted.frame_count,
        emitted.deferred.len(),
        emitted.inject_count,
        emitted.walk_count,
        scen.frames + 1,
        scen.frames
    );
    match out_path {
        Some(p) => {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&p, &emitted.json) {
                eprintln!("dbx-plan: cannot write {p:?}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("dbx-plan: wrote {}", p.display());
        }
        None => print!("{}", emitted.json),
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s0() -> Scenario {
        Scenario::parse(include_str!("../../scenarios/S0.scen")).unwrap()
    }

    fn s1() -> Scenario {
        Scenario::parse(include_str!("../../scenarios/S1.scen")).unwrap()
    }

    #[test]
    fn extent_forms_parse() {
        assert_eq!(parse_extent("4"), Some(4));
        assert_eq!(parse_extent("10000"), Some(10000));
        assert_eq!(parse_extent("0x800 ring"), Some(0x800));
        assert_eq!(parse_extent("0x1f38 (999*8)"), Some(0x1f38));
        assert_eq!(parse_extent("282*0x4E"), Some(282 * 0x4E));
        assert_eq!(parse_extent("map-sized (8 u16 planes)"), None);
        assert_eq!(parse_extent("0x62-stride rows"), None);
    }

    #[test]
    fn exd_cells_split() {
        assert_eq!(exd_cells("0x1195f0"), vec![0x1195f0]);
        assert_eq!(exd_cells("0x1074b8 / 0x10748c"), vec![0x1074b8, 0x10748c]);
        assert_eq!(
            exd_cells("0x8ded4 (cursor 0x108424)"),
            vec![0x8ded4, 0x108424]
        );
    }

    #[test]
    fn s0_plan_resolves_registry_rows() {
        let scen = s0();
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        // T0: 11 rows, 1 gap (sfx-master-gate; difficulty closed by the
        // W5-followup) -> 10 per-frame. TS: 15 rows, 6 deferred -> 9
        // anchor-only + T0 rides the anchor too: 10 + 9 = 19 anchor rows.
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 10, "T0 rows minus the 1 gap");
        assert_eq!(anchor_count, 19, "T0 + resolved TS rows");
        // 6 TS extent gaps + the 1 explicit T0 EXD gap
        assert_eq!(emitted.deferred.len(), 7);
        // every emitted id is a real registry row of the scenario tiers
        for id in row_ids(&emitted.json) {
            let row = reg
                .iter()
                .find(|r| r.id == id)
                .unwrap_or_else(|| panic!("plan id {id:?} is not in the registry"));
            assert!(
                scen.tiers.contains(&row.tier),
                "plan id {id:?} tier {} not in scenario tiers",
                row.tier
            );
            assert!(
                !row.exd_addr.is_empty(),
                "plan id {id:?} is an EXD gap — must never be emitted"
            );
        }
        // stitcher contract: frames + 1 records
        let frames: u64 = extract_frames(&emitted.json);
        assert_eq!(frames, scen.frames + 1);
        // the w/h loader statics land under their documented cells
        // (RE-EXD-MAP sec 5b: w 0x1074b8 / h 0x10748c — never swapped)
        assert!(emitted
            .json
            .contains("\"name\": \"map_w\", \"addr\": \"CS:001074B8\""));
        assert!(emitted
            .json
            .contains("\"name\": \"map_h\", \"addr\": \"CS:0010748C\""));
    }

    #[test]
    fn s0_plan_matches_committed_artifact() {
        let emitted = emit_plan(&s0(), &registry()).unwrap();
        let committed = include_str!("../../capture-plans/S0.json");
        assert_eq!(emitted.json, committed, "capture-plans/S0.json is stale: regenerate with dbx-plan scenarios/S0.scen --out capture-plans/S0.json");
    }

    #[test]
    fn s1_plan_resolves_t1_rows() {
        let scen = Scenario::parse(include_str!("../../scenarios/S1.scen")).unwrap();
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        // T1: 17 rows - 2 gaps (blink-cursor/no-extract-latch;
        // order-target closed by the W5-followup) = 15 resolved (the
        // move-target-words 0x60 span filled by W7-followup2, D90).
        // T0: 10 per-frame + TS: 9 anchor-only, same as S0.
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 10 + 15, "T0 minus 1 gap + T1 resolved");
        assert_eq!(anchor_count, 19 + 15, "T0 + TS + T1 rows");
        assert_eq!(
            emitted.deferred.len(),
            9,
            "7 S0 deferrals + T1: 2 gaps (move-target no longer deferred)"
        );
        // count-cell resolve rows exist with the registry-derived cells
        assert!(emitted
            .json
            .contains("\"name\": \"robot_count\", \"addr\": \"CS:0011958C\""));
        assert!(emitted
            .json
            .contains("\"name\": \"trt_count\", \"addr\": \"CS:0011949C\""));
        assert!(emitted
            .json
            .contains("\"name\": \"obj_ptr\", \"addr\": \"CS:00119584\""));
        assert!(emitted
            .json
            .contains("\"name\": \"obj_count\", \"addr\": \"CS:00119554\""));
        // count-driven extents compiled to expressions
        assert!(emitted.json.contains("\"len\": \"$robot_count*0xA8\""));
        assert!(emitted.json.contains("\"len\": \"$trt_count*0x20\""));
        assert!(emitted.json.contains("\"len\": \"$obj_count*0x14\""));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h*2\""));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h*0x1E\""));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h\""));
        // gaps never emit (order-target closed by the W5-followup and
        // now emits its verified 12-byte triple)
        for id in row_ids(&emitted.json) {
            assert!(
                id != "blink-cursor" && id != "no-extract-latch",
                "gap row {id:?} must never be emitted"
            );
        }
        assert!(
            emitted
                .json
                .contains("{ \"id\": \"order-target\", \"addr\": \"CS:0010E0A4\", \"len\": 12 }"),
            "order-target must emit its verified triple"
        );
        assert!(
            emitted.json.contains(
                "{ \"id\": \"move-target-words\", \"addr\": \"CS:000F75EC\", \"len\": 96 }"
            ),
            "move-target-words must emit the pinned 0x60 x[12]+y[12] span (D90)"
        );
    }

    #[test]
    fn s1_plan_matches_committed_artifact() {
        let emitted = emit_plan(&s1(), &registry()).unwrap();
        let committed = include_str!("../../capture-plans/S1.json");
        assert_eq!(emitted.json, committed, "capture-plans/S1.json is stale: regenerate with dbx-plan scenarios/S1.scen --out capture-plans/S1.json");
    }

    #[test]
    fn t2_scenario_refused() {
        let src = "scenario = X\ntiers = T2\nframes = 1\n";
        let scen = Scenario::parse(src).unwrap();
        assert!(emit_plan(&scen, &registry()).is_err());
    }

    #[test]
    fn injection_steps_gate_on_registry_gaps() {
        // The committed registry with the §5 seam aliases CLEARED (the
        // pre-W5-followup state): each step kind must fail loudly,
        // naming the seam. Proves the gate still bites on any future
        // gap.
        let reg = registry_with_gaps();
        for src in [
            "scenario = X\ntiers = T0\nframes = 4\nboot difficulty=1\n",
            "scenario = X\ntiers = T0\nframes = 4\nuntil-anchor m\nkeystore 0x1f=1\n",
            "scenario = X\ntiers = T0\nframes = 4\nuntil-anchor m\norder 1 2 3\n",
            "scenario = X\ntiers = T0\nframes = 4\nuntil-anchor m\npad 3\n",
            "scenario = X\ntiers = T0\nframes = 4\nuntil-anchor m\ncommand 01 02\n",
            // walk-phase keystore with a cleared alias must name the gap
            "scenario = X\ntiers = T0\nframes = 4\nkeystore 0x1f=1\n",
        ] {
            let scen = Scenario::parse(src).unwrap();
            let err = emit_plan(&scen, &reg)
                .err()
                .map(|e| e.to_string())
                .expect("gap-gated step must not compile");
            assert!(
                err.contains("gap") || err.contains("walk"),
                "error must name the gate: {err}"
            );
        }
    }

    #[test]
    fn injection_steps_compile_with_aliases() {
        // The REAL registry (W5-followup §5c pins): keystore 0x894d4,
        // order target 0x10e0a4/a8/ac, ring 0x9255c + count 0x119588.
        // Proves the compiler emission against the anchored EXD
        // addresses — frame accounting, byte layout, command op shape.
        let src = "scenario = X\ntiers = T0\nframes = 4\n\
                   until-anchor mission-start\n\
                   step 2\n\
                   keystore 0x1f=1, 0x2a=0\n\
                   order 29 18 0\n\
                   command 01 02 3f\n";
        let scen = Scenario::parse(src).unwrap();
        let emitted = emit_plan(&scen, &registry()).unwrap();
        // boundaries: anchor=1, step 2 -> 3; keystore @3 (base 0x894d4
        // + scan 0x1f / 0x2a), order @4, command @5 (frames=4 -> the
        // 5th record is the window edge)
        assert!(emitted
            .json
            .contains("\"frame\": 3, \"addr\": \"CS:000894F3\", \"bytes\": \"01\""));
        assert!(emitted
            .json
            .contains("\"frame\": 3, \"addr\": \"CS:000894FE\", \"bytes\": \"00\""));
        assert!(emitted
            .json
            .contains("\"frame\": 4, \"addr\": \"CS:0010E0A4\", \"bytes\": \"1d000000\""));
        assert!(emitted
            .json
            .contains("\"frame\": 4, \"addr\": \"CS:0010E0A8\", \"bytes\": \"12000000\""));
        assert!(emitted.json.contains(
            "\"frame\": 5, \"op\": \"command\", \"base\": \"CS:0009255C\", \
             \"stride\": 128, \"count_cell\": \"CS:00119588\", \"bytes\": \"01023f\""
        ));
        assert_eq!(emitted.inject_count, 6);
    }

    #[test]
    fn pad_step_compiles_op_row() {
        // D86: the pad step un-gated — the op row's bank comes from
        // static-pad-slots (the READ anchor, EXD 0xf63c) and the three
        // targets from order-target; every address registry-derived.
        let src = "scenario = X\ntiers = T0\nframes = 4\n\
                   until-anchor mission-start\n\
                   step 2\n\
                   pad 8\n";
        let scen = Scenario::parse(src).unwrap();
        let emitted = emit_plan(&scen, &registry()).unwrap();
        // boundary: anchor=1, step 2 -> 3 (the same accounting as the
        // alias test); ZONEA zone-1 census slot 8 as the pick example
        assert!(emitted.json.contains(
            "\"frame\": 3, \"op\": \"pad\", \"bank\": \"CS:0000F63C\", \"slot\": 8, \
             \"target\": [\"CS:0010E0A4\", \"CS:0010E0A8\", \"CS:0010E0AC\"]"
        ));
        assert_eq!(emitted.inject_count, 1);
    }

    #[test]
    fn pad_step_bank_gap_refused() {
        // The READ anchor has its own gap error, distinct from the
        // step_rows write-seam rule: static-pad-slots cleared -> the
        // pad bank address is never fabricated.
        let mut reg = registry();
        for row in reg.iter_mut() {
            if row.id == "static-pad-slots" {
                row.exd_addr = String::new();
            }
        }
        let src = "scenario = X\ntiers = T0\nframes = 4\nuntil-anchor m\npad 3\n";
        let scen = Scenario::parse(src).unwrap();
        let err = emit_plan(&scen, &reg)
            .err()
            .map(|e| e.to_string())
            .expect("pad step must not compile without the pad-bank anchor");
        assert!(
            err.contains("static-pad-slots") && err.contains("gap"),
            "error must name the READ-anchor gap: {err}"
        );
    }

    #[test]
    fn injection_step_past_window_refused() {
        let src = "scenario = X\ntiers = T0\nframes = 1\n\
                   until-anchor mission-start\n\
                   step 3\n\
                   command 01\n";
        let scen = Scenario::parse(src).unwrap();
        assert!(emit_plan(&scen, &registry())
            .err()
            .map(|e| e.to_string())
            .is_some_and(|e| e.contains("past the capture window")));
    }

    #[test]
    fn walk_phase_compiles_stop_indexed_keystore() {
        // D84: walk-phase keystore steps -> stop-indexed rows; Advance
        // consumes stops; boot rides at frame 0; the calibration trio
        // is registry-derived; resolve_at is anchor.
        let src = "scenario = X\ntiers = T0\nframes = 2\n\
                   boot difficulty=1\n\
                   step 5\n\
                   keystore 0x01=1\n\
                   step 3\n\
                   keystore 0x01=0, 0x1c=1\n\
                   until-anchor mission-start\n";
        let scen = Scenario::parse(src).unwrap();
        let emitted = emit_plan(&scen, &registry()).unwrap();
        // boundaries: stop 6 = ESC press (0x894d4+1), stop 10 = ESC
        // release + ENTER press (0x894d4+0x1c)
        assert!(emitted
            .json
            .contains("\"stop\": 6, \"addr\": \"CS:000894D5\", \"bytes\": \"01\""));
        assert!(emitted
            .json
            .contains("\"stop\": 10, \"addr\": \"CS:000894D5\", \"bytes\": \"00\""));
        assert!(emitted
            .json
            .contains("\"stop\": 10, \"addr\": \"CS:000894F0\", \"bytes\": \"01\""));
        assert_eq!(emitted.walk_count, 3);
        // calibration trio: registry T0 cells, prefixed ids
        for (cal, cell) in [
            ("walk-mode", "CS:001075D8"),
            ("walk-zone", "CS:00107500"),
            ("walk-mission", "CS:00119610"),
        ] {
            assert!(
                emitted.json.contains(&format!(
                    "{{ \"id\": \"{cal}\", \"addr\": \"{cell}\", \"len\": 4 }}"
                )),
                "missing calibration row {cal}"
            );
        }
        // boot write + resolve position
        assert!(emitted.json.contains("\"boot_writes\""));
        assert!(emitted.json.contains("\"resolve_at\": \"anchor\""));
        // no mission-phase inject rows
        assert_eq!(emitted.inject_count, 0);
        assert!(!emitted.json.contains("\"inject\""));
    }

    #[test]
    fn walk_phase_rejects_mission_seam_steps() {
        // steps BEFORE until-anchor are walk phase (runner.rs phases())
        let base = "scenario = X\ntiers = T0\nframes = 1\n";
        for src in [
            format!("{base}order 1 2 3\nuntil-anchor mission-start\n"),
            format!("{base}pad 3\nuntil-anchor mission-start\n"),
            format!("{base}command 01\nuntil-anchor mission-start\n"),
        ] {
            let scen = Scenario::parse(&src).unwrap();
            let err = emit_plan(&scen, &registry())
                .err()
                .map(|e| e.to_string())
                .expect("walk-phase mission-seam step must not compile");
            assert!(
                err.contains("not menu-walk steps"),
                "error must name the walk gate: {err}"
            );
        }
    }

    #[test]
    fn walk_runaway_stop_count_refused() {
        let src = "scenario = X\ntiers = T0\nframes = 1\n\
                   step 2000000\nkeystore 0x01=1\nuntil-anchor mission-start\n";
        let scen = Scenario::parse(src).unwrap();
        assert!(emit_plan(&scen, &registry())
            .err()
            .map(|e| e.to_string())
            .is_some_and(|e| e.contains("runaway")));
    }

    #[test]
    fn s0w_plan_matches_committed_artifact() {
        let scen = Scenario::parse(include_str!("../../scenarios/S0W.scen")).unwrap();
        let emitted = emit_plan(&scen, &registry()).unwrap();
        let committed = include_str!("../../capture-plans/S0W.json");
        assert_eq!(emitted.json, committed, "capture-plans/S0W.json is stale: regenerate with dbx-plan scenarios/S0W.scen --out capture-plans/S0W.json");
    }

    /// The committed registry with the §5 seam aliases CLEARED — the
    /// pre-W5-followup gap state, so the alias gates can still be
    /// proven to bite. Compiler tests only.
    fn registry_with_gaps() -> Vec<diffharness::Watch> {
        let mut reg = registry();
        for row in reg.iter_mut() {
            match row.id.as_str() {
                "inj-key-state" | "order-target" | "inj-command-ring" | "inj-command-count"
                | "difficulty" => {
                    row.exd_addr = String::new();
                }
                _ => {}
            }
        }
        reg
    }

    fn count_rows(json: &str, key: &str) -> usize {
        let section = json
            .split(&format!("\"{key}\": ["))
            .nth(1)
            .unwrap()
            .split("]")
            .next()
            .unwrap();
        section.matches("\"id\"").count()
    }

    fn row_ids(json: &str) -> Vec<String> {
        let mut ids = Vec::new();
        for key in ["anchor_watches", "watches"] {
            let section = json
                .split(&format!("\"{key}\": ["))
                .nth(1)
                .unwrap()
                .split("]")
                .next()
                .unwrap();
            for line in section.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("{ \"id\": \"") {
                    let id = rest.split('"').next().unwrap();
                    ids.push(id.to_string());
                }
            }
        }
        ids
    }

    fn extract_frames(json: &str) -> u64 {
        for line in json.lines() {
            if let Some(rest) = line.trim().strip_prefix("\"frames\": ") {
                return rest.trim_end_matches(',').parse().unwrap();
            }
        }
        panic!("no frames key");
    }
}
