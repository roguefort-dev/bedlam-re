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
//! Supported scenario tiers: T0/T1/TS (the S0/S1 shape) + T2/T3 (the
//! W12-S3/S4 widening, D109). T2/T3 rows WITHOUT an EXD alias stay
//! explicit coverage gaps (deferred, never emitted — the differ's
//! coverage discipline); the aliased rows compile to their full fixed
//! bank spans (the D162 §5i census filled all 17 — count-driven rows
//! ride CountExpr, tile-claims rides the second PtrCell).
//!
//! Channels (`--channel o1|o2`, D138): `o1` (default) = the EXD/DOSBox
//! form every committed plan pins — addresses are the registry rows'
//! `exd_addr` cells in the `CS:` selector form capgen consumes.
//! `o2` = the EXW/Wine spot-check form (DESIGN §2 O2 / §10 W11): every
//! address swaps to the row's `exw_addr` canon cell in flat
//! `0x`-prefixed linear form (the W11 host ptrace driver reads EXW
//! addresses directly — zero translation), the DOSBox boot/arm
//! command machinery is replaced by a `trigger` object (the s0-trigger
//! row's OPERATIONAL frame-tail site `O2_TRIGGER_SITE` — the
//! normal-path PresentEnd CALL, NOT the row's callee canon — + the
//! EXW frame-counter cell), and the ONE
//! D137-pinned span split applies (`static-map-wh` = the 8-byte span
//! @0x4eddec, w LOW/adjacent cells — vs the EXD 0x30 span, h LOW;
//! D137's 0x24-apart arithmetic was corrected by D138). Walk-phase
//! keystore scenarios are O1-only (the BPLM stop-indexed menu walk is
//! DOSBox machinery, D84).
//!
//! Usage:
//! ```text
//! dbx-plan <scenario.scen> [--out <capture-plan.json>] [--channel o1|o2]
//! ```
//! Default output: stdout. The committed artifacts for review are
//! `capture-plans/<id>.json` (o1) and `capture-plans/<id>-o2.json`
//! (o2) — tests pin byte-equality for both.

use diffharness::registry;
use diffharness::runner::{Scenario, Step};
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

/// The tiers this compiler can resolve today (T0/T1/TS + the D109
/// T2/T3 widening: unaliased rows are deferred, never fabricated).
const SUPPORTED_TIERS: [&str; 5] = ["T0", "T1", "T2", "T3", "TS"];

/// The capture channel a plan targets (DESIGN §2, D138): O1 =
/// BEDLAM.EXD under pinned DOSBox-X (the rows' `exd_addr` cells, `CS:`
/// selector form, capgen/D81 boot+arm command machinery); O2 =
/// BEDLAM.EXW under pinned Wine (the rows' `exw_addr` CANON cells in
/// flat linear form, read directly by the W11 host ptrace driver —
/// zero address translation).
/// The O2 operational trigger site (gate p4-trigger-address; D156,
/// RE-EXW-SIM §7j.66/W11): the MissionShell NORMAL-path PresentEnd
/// CALL 0x4486C9 (`4486c9: e8 35 d3 fd ff call 0x425a03`, verified in
/// ghidra-project/exw-text-objdump.txt) — NOT the callee entry
/// 0x425A03, which has 62 direct call sites (menus, loading screens,
/// cinematics, the pause redraw), so a breakpoint at the function
/// entry fires on every present on the way to the mission and is not
/// a frame-tail trigger; the pause-path present 0x44861F is never
/// taken by the scenarios. The registry s0-trigger row KEEPS
/// exw_addr = 0x425a03 as the function canon-of-record and
/// exd_addr = 0x5a6eb as the EXD dump-point twin (5a6eb CALL
/// FUN_00010670, increment 0x5a6f0-fd right after — order identical);
/// an instruction call site has no registry cell home, so the
/// operational site is pinned here at the emitter. Emitted fragment
/// (exact, gate-checked):
///   "trigger": { "site": "0x004486C9", "frame_counter": "0x0046AE68" }
const O2_TRIGGER_SITE: u64 = 0x4486c9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    O1,
    O2,
}

impl Channel {
    /// The registry address field this channel compiles from. A row
    /// whose channel-side address is EMPTY is a gap on that channel
    /// (on O2 today: the EXD-only rows, e.g. static-cursor-clamp — its
    /// EXW cursor clamps live in host 640x480 space with no alias
    /// row) and never emits there — the mirror of the EXD-gap rule.
    fn src(self, row: &diffharness::Watch) -> &str {
        match self {
            Channel::O1 => &row.exd_addr,
            Channel::O2 => &row.exw_addr,
        }
    }

    /// One cell address as a plan-JSON `addr` string: O1 = the `CS:`
    /// flat-selector form (capgen resolves CS to the DOSBox flat
    /// selector); O2 = the flat `0x`-prefixed Win32 linear form.
    fn addr(self, a: u64) -> String {
        match self {
            Channel::O1 => format!("CS:{a:08X}"),
            Channel::O2 => format!("0x{a:08X}"),
        }
    }

    /// A `$symbol` reference (PtrCell rows): the driver substitutes
    /// the resolve-row value; the CS: prefix only exists on O1.
    fn sym_addr(self, sym: &str) -> String {
        match self {
            Channel::O1 => format!("CS:${sym}"),
            Channel::O2 => format!("${sym}"),
        }
    }
}

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
    /// A count cell rides the blob HEAD (D109): dump the 4-byte cell
    /// at `cell` first, then the inner form's span — capgen
    /// concatenates the two dumps into the one watch blob. This is
    /// the O1 bank-row grammar the differ's normalizers pin (u32
    /// count + records for trt-array/object-instances; robot-bank
    /// stays a bare span — its normalizer has no count prefix).
    Prefixed { cell: u64, inner: Box<Form> },
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
        // the latch is per-robot over the SAME robot count cell as
        // the bank (D133: 0xf929c + i*4, count 0x11958c)
        "no-extract-latch" => "robot_count",
        // the D162 T2 census rows (§5i): the .NME loader count cells
        "critter-bank" => "critter_count",
        "poi-bank" => "poi_count",
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
/// re-emits). Deferred rows list the missing pin explicitly. `ch`
/// selects the address source (O1 = exd_addr, O2 = exw_addr).
fn resolve_row(row: &diffharness::Watch, ch: Channel) -> Result<Option<RowPlan>, PlanError> {
    let id = row.id.as_str();
    // The channel gate: a row whose CHANNEL-side address is a gap
    // never emits on that channel (on O2: the EXD-only rows). Since
    // D162 (§5i) the T2/T3 alias set is COMPLETE — the census rows
    // emit on BOTH channels registry-driven (the differ normalizers
    // passthrough-compare the raw spans; the subset-form extraction
    // arms are the named follow-up), so no W11-era allowlist exists.
    if ch.src(row).is_empty() {
        return Ok(None);
    }
    let plan = |form: Form| {
        Ok(Some(RowPlan {
            id: row.id.clone(),
            form,
        }))
    };

    // --- T0: every verified row is a fixed 4-byte cell read.
    if row.tier == "T0" {
        if ch.src(row).is_empty() {
            return Ok(None); // defensive: a gap-status row (none remain —
                             // difficulty closed by the W5-followup, the
                             // sfx gate by the D134 twin census)
        }
        if row.extent != "4" || row.indirect {
            return Err(die(format!(
                "T0 row {id} changed shape (extent {:?}, indirect {}): \
                 update dbx-plan's T0 form",
                row.extent, row.indirect
            )));
        }
        let addr = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
            die(format!(
                "T0 row {id} has no parsable {} address: {:?}",
                if ch == Channel::O1 {
                    "exd_addr"
                } else {
                    "exw_addr"
                },
                ch.src(row)
            ))
        })?;
        return plan(Form::Fixed { addr, len: 4 });
    }

    // --- T1: the P4 slice (robot/order/terrain banks). Gap rows are
    // skipped like T0 gaps; bank rows are count-driven (resolve rows
    // feed $symbols); grid rows derive their extent from map w/h.
    if row.tier == "T1" {
        if ch.src(row).is_empty() {
            return Ok(None); // defensive: a gap-status row (none remain —
                             // the last W1 gap, sfx-master-gate, was closed
                             // by the D134 twin census) would never dump
        }
        let cells = exd_cells(ch.src(row));
        let first = cells.first().copied().ok_or_else(|| {
            die(format!(
                "T1 row {id} has no parsable {} address: {:?}",
                if ch == Channel::O1 {
                    "exd_addr"
                } else {
                    "exw_addr"
                },
                ch.src(row)
            ))
        })?;
        return match id {
            // count-driven banks: extent "count*<stride>" + a count cell
            // named in exd_addr. [derived-pinned] the count cell is the
            // SECOND exd cell of the row (RE-EXD-MAP sec 5 bank rows).
            // robot-bank stays a BARE span (the differ's robot O1
            // walk has no count prefix); trt-array pins its count
            // cell onto the blob head (trt_o1 walks 0..count, D109).
            "robot-bank" => {
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
            "trt-array" => {
                if cells.len() != 2 {
                    return Err(die(format!(
                        "row {id} exd_addr {:?} no longer carries base + count cell",
                        row.exd_addr
                    )));
                }
                let stride = extent_stride(&row.extent, id)?;
                let sym = count_symbol(id);
                plan(Form::Prefixed {
                    cell: cells[1],
                    inner: Box::new(Form::CountExpr {
                        addr: cells[0],
                        len_expr: format!("${sym}*{stride}"),
                    }),
                })
            }
            // no-extract-latch twin 0xf929c (D133): per-robot u32
            // CLAIMED flag over the robot count — a bare count-driven
            // span like the bank (stride 4; the array itself is the
            // fixed 12-slot 0x30 boot memset both sides, so the tail
            // past count is always 0).
            "no-extract-latch" => {
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
            // selection triple fully mapped since D132 (slot/base/size
            // cells 0x11954c/0x11955c/0x11958c) — but the EXD cells are
            // NOT contiguous, so the plan still dumps the canonical 4-B
            // D83 form = the SELECTED-SLOT cell (the differ's u32 row);
            // base/size ride the bank rows' count cells.
            // [derived-pinned] (D138): the two channels list the cells
            // in DIFFERENT orders — the EXD list is field-ordered
            // (selected FIRST: 0x11954c, +0x10 base, +0x30 size), the
            // EXW list is field-ordered TOO but NOT ascending (base
            // 0x46cbd4 / selected 0x46cbdc / size 0x46cbd8 — selected
            // is the HIGHEST address, size sits between; the D132
            // label-swap pairing, RE-EXD-MAP sec 5 row) — so the
            // selected-slot pick is per-channel: cells[0] on O1,
            // cells[1] on O2. The gap asserts pin the geometry; a
            // registry reorder fails loud.
            "selection-triple" => {
                if cells.len() != 3 {
                    return Err(die(format!(
                        "selection-triple {} no longer has exactly 3 cells",
                        if ch == Channel::O1 {
                            "exd_addr"
                        } else {
                            "exw_addr"
                        }
                    )));
                }
                let sel = match ch {
                    Channel::O1 => {
                        if cells[1] - cells[0] != 0x10 || cells[2] - cells[1] != 0x30 {
                            return Err(die(format!(
                                "selection-triple exd_addr {:?} no longer the EXD field \
                                 order (selected/base/size 0x10+0x30 apart)",
                                row.exd_addr
                            )));
                        }
                        cells[0]
                    }
                    Channel::O2 => {
                        // [derived-pinned] the EXW list is FIELD-ordered
                        // (base/selected/size) but NOT ascending: the
                        // selected cell is the HIGHEST (base 0x46cbd4
                        // +8 size 0x46cbd8 +4 selected 0x46cbdc).
                        if cells[1] - cells[0] != 8 || cells[1] - cells[2] != 4 {
                            return Err(die(format!(
                                "selection-triple exw_addr {:?} no longer the EXW field \
                                 order (base +8 size +4 selected)",
                                row.exw_addr
                            )));
                        }
                        cells[1]
                    }
                };
                plan(Form::Fixed { addr: sel, len: 4 })
            }
            // blink-cursor twin 0x10e108 (D132): plain 4-B u32 scalar —
            // the S1 blink-cursor-from-spawn hypothesis watch (expected
            // constant 0 on corpus paths, §7j.59.E).
            "blink-cursor" => plan(Form::Fixed {
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
                // [derived-pinned] (D108/D109): the O1 walk covers the
                // WHOLE bank — the ZONEB .POS surface carries LIVE slots
                // past dead holes (max slot 1128 over 1096 live; a
                // count-bounded span silently drops 32 objects and
                // breaks the count field). The differ's O1 normalizer
                // pins the blob grammar: u32 count cell FIRST + the
                // full 2000*0x14 records (dead id==-1 slots skipped in
                // the walk) — the count cell rides the blob head via
                // the prefix form, so $obj_count needs no resolve row.
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row {id} extent {:?} stopped parsing as the fixed \
                         full-bank span: update dbx-plan",
                        row.extent
                    )));
                };
                if len != 2000 * 0x14 {
                    return Err(die(format!(
                        "object-instances extent is {len:#x}, not the pinned \
                         full-bank 2000*0x14 = {:#x}: update dbx-plan (the \
                         count-bounded span drops the ZONEB live-past-dead \
                         slots — D108)",
                        2000 * 0x14
                    )));
                }
                plan(Form::Prefixed {
                    cell: cells[1],
                    inner: Box::new(Form::PtrCell {
                        cell: cells[0],
                        len_expr: "2000*0x14".into(),
                    }),
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

    // --- T2/T3 (the D109 tier widening; D162 §5i closed the alias
    // set): unaliased rows (none today) are explicit coverage gaps —
    // never dumped on O1 (the differ's coverage discipline: they
    // surface as E-only rows in cross-channel reports, never
    // silence). The aliased fixed-span banks are FULL spans (the
    // differ's O1 normalizers require the whole bank with NO count
    // cell — the guest free-slot walk is the bound, RE-EXD-MAP
    // sec 5c); the count-driven/indirect rows carry deliberate
    // forms below (CountExpr / PtrCell / per-channel Span).
    if row.tier == "T2" || row.tier == "T3" {
        if row.exd_addr.is_empty() {
            return Ok(None); // E-only coverage row (debris/splash/mortar/...)
                             // — deferred on BOTH channels: the differ's
                             // O2 arms cover exactly the aliased set (the
                             // O2 emission set is channel-symmetric, D138)
        }
        let cells = exd_cells(ch.src(row));
        let first = cells.first().copied().ok_or_else(|| {
            die(format!(
                "{} row {id} has no parsable {} address: {:?}",
                row.tier,
                if ch == Channel::O1 {
                    "exd_addr"
                } else {
                    "exw_addr"
                },
                ch.src(row)
            ))
        })?;
        return match id {
            "weapon-anim-bank" | "projectile-bank" => {
                if row.indirect || cells.len() != 1 {
                    return Err(die(format!(
                        "row {id} changed shape (indirect {}, exd_addr {:?}): \
                         the T2 banks are single fixed cells",
                        row.indirect, row.exd_addr
                    )));
                }
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row {id} extent {:?} stopped parsing as the fixed \
                         full-bank span: update dbx-plan",
                        row.extent
                    )));
                };
                // [derived-pinned] the full-bank pins (RE-EXD-MAP
                // sec 5c): 400*0x36 = 0x5460 — the EXD free-slot
                // finder FUN_00023295 bound; 50*0x22 = 0x6A4 — the
                // tick twin FUN_00022a52's 50-slot walk.
                let want = if id == "weapon-anim-bank" {
                    400 * 0x36
                } else {
                    50 * 0x22
                };
                if len != want {
                    return Err(die(format!(
                        "row {id} extent is {len:#x}, not the pinned full-bank \
                         {want:#x}: update dbx-plan (the differ's O1 walk \
                         requires the WHOLE bank)"
                    )));
                }
                plan(Form::Fixed { addr: first, len })
            }
            // The D162 count-driven T2 rows (§5i): the .NME loader
            // twins pin base + count cell on BOTH channels (critter
            // 0x10e81c/0x1194dc ⟷ 0x4cff98/0x46cc2c; poi
            // 0x971d4/0x119580 ⟷ 0x4dabdc/0x46cbf0). The counts are
            // mission-load statics (loader-written once), so the
            // anchor-resolved count bounds a stable span — the
            // robot-bank CountExpr grammar verbatim.
            "critter-bank" | "poi-bank" => {
                if row.indirect || cells.len() != 2 {
                    return Err(die(format!(
                        "row {id} changed shape (indirect {}, addr {:?}): \
                         the D162 census rows carry base + count cell",
                        row.indirect,
                        ch.src(row)
                    )));
                }
                let stride = extent_stride(&row.extent, id)?;
                let sym = count_symbol(id);
                plan(Form::CountExpr {
                    addr: cells[0],
                    len_expr: format!("${sym}*{stride}"),
                })
            }
            // The D162 objective row (§5i): fixed 6-slot span at the
            // base cell; the resolver PHASE cell (cells[1]) rides the
            // registry layout note, never the dump span.
            "objective-slots" => {
                if row.indirect || cells.len() != 2 {
                    return Err(die(format!(
                        "row objective-slots changed shape (indirect {}, addr {:?}): \
                         expected base + resolver-phase cell",
                        row.indirect,
                        ch.src(row)
                    )));
                }
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row objective-slots extent {:?} stopped parsing: update dbx-plan",
                        row.extent
                    )));
                };
                if len != 6 * 0x20 {
                    return Err(die(format!(
                        "row objective-slots extent is {len:#x}, not the pinned \
                         6-slot 0xC0 span: update dbx-plan"
                    )));
                }
                plan(Form::Fixed { addr: first, len })
            }
            // The D162 escape pair (§5i): per-channel spans — the EXW
            // cells are ADJACENT (8 B), the EXD pair sits 0xC apart so
            // the O1 span is 16 B (count@+0, timer@+0xC; the +8 dword
            // is foreign, never read — registry layout note).
            "escape-counters" => {
                if row.indirect || cells.len() != 2 {
                    return Err(die(format!(
                        "row escape-counters changed shape (indirect {}, addr {:?}): \
                         expected the counter pair",
                        row.indirect,
                        ch.src(row)
                    )));
                }
                let (base, len) = match ch {
                    Channel::O1 => {
                        if cells[1] != cells[0] + 0xC {
                            return Err(die(format!(
                                "escape-counters exd geometry changed: expected \
                                 the pair 0xC apart, got {:#x}/{:#x}",
                                cells[0], cells[1]
                            )));
                        }
                        (cells[0], 16u64)
                    }
                    Channel::O2 => {
                        if cells[1] != cells[0] + 4 {
                            return Err(die(format!(
                                "escape-counters exw geometry changed: expected \
                                 the adjacent pair, got {:#x}/{:#x}",
                                cells[0], cells[1]
                            )));
                        }
                        (cells[0], 8u64)
                    }
                };
                plan(Form::Span {
                    base,
                    len,
                    cells: cells.clone(),
                })
            }
            // The D162 tile-claims row (§5i): the second PtrCell row —
            // BOTH channels keep the arena behind a pointer cell
            // (EXW 0x46af58 / EXD 0x119564; the static-claim-bank TS
            // row's grammar verbatim, per-frame instead of one-shot).
            "tile-claims" => {
                if !row.indirect || cells.len() != 1 {
                    return Err(die(format!(
                        "row tile-claims changed shape (indirect {}, addr {:?}): \
                         expected the single pointer cell",
                        row.indirect,
                        ch.src(row)
                    )));
                }
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "row tile-claims extent {:?} stopped parsing: update dbx-plan",
                        row.extent
                    )));
                };
                if len != 10000 {
                    return Err(die(format!(
                        "row tile-claims extent is {len}, not the pinned \
                         0x2710 arena: update dbx-plan"
                    )));
                }
                plan(Form::PtrCell {
                    cell: first,
                    len_expr: format!("{len}"),
                })
            }
            other => {
                // The generic aliased T2/T3 path (the twelve D162
                // fixed-span rows): fixed span only, NEVER a guessed
                // address — an indirect row (pointer cell), a
                // count-driven extent, or a multi-cell row needs its
                // own deliberate form above, so all die loudly here.
                if row.indirect {
                    return Err(die(format!(
                        "aliased {} row {other:?} is indirect — add an explicit \
                         PtrCell form in dbx-plan, never the generic fixed span",
                        row.tier
                    )));
                }
                if cells.len() != 1 {
                    return Err(die(format!(
                        "aliased {} row {other:?} carries {} cells — a multi-cell \
                         row needs a deliberate form, never cells-first truncation",
                        row.tier,
                        cells.len()
                    )));
                }
                let Some(len) = parse_extent(&row.extent) else {
                    return Err(die(format!(
                        "aliased {} row {other:?} extent {:?} has no fixed form: \
                         add a deliberate dbx-plan form (count-driven banks \
                         need a count-cell prefix too)",
                        row.tier, row.extent
                    )));
                };
                plan(Form::Fixed { addr: first, len })
            }
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
            let addr = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "row {id} has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            plan(Form::Fixed { addr, len })
        }
        // Two-cell rows dumped as one contiguous span. THE ONE
        // D137-PINNED CHANNEL SPLIT (arithmetic corrected by D138):
        // the EXD pair (w 0x1074b8 / h 0x10748c) sits 0x2c apart with
        // h LOW — the O1 span = 0x30 @h; the EXW pair (w 0x4eddec /
        // h 0x4eddf0) is ADJACENT (4 apart) with w LOW — the O2 span
        // = 8 @w. The port reversed the field order relative to
        // address order (O1's low cell is h, O2's is w), so the O2
        // form is NOT the O1 form relabelled (RE-EXW-SIM sec 7j.60).
        "static-map-wh" => {
            let cells = exd_cells(ch.src(row));
            if cells.len() != 2 {
                return Err(die(format!(
                    "static-map-wh {} no longer has exactly 2 cells",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    }
                )));
            }
            let (lo, hi) = (cells[0].min(cells[1]), cells[0].max(cells[1]));
            match ch {
                // [derived-pinned] w 0x1074b8 / h 0x10748c are 0x2c
                // apart (RE-EXD-MAP sec 5b); the span covers both u32s.
                Channel::O1 => {
                    if hi - lo != 0x2c {
                        return Err(die(format!(
                            "static-map-wh exd_addr cells are no longer 0x2c apart: \
                             {lo:#x}..{hi:#x}"
                        )));
                    }
                    plan(Form::Span {
                        base: lo,
                        len: 0x2c + 4,
                        cells,
                    })
                }
                // [derived-pinned] (D137, arithmetic CORRECTED by
                // D138): the O2 capture form = ONE 8-byte span
                // @0x4eddec, w@+0x00 / h@+0x04 — the EXW cells are
                // ADJACENT u32s with w LOW (0x4eddf0−0x4eddec = 4;
                // the stride cell 0x4eddf4 right after, excluded like
                // the EXD span excludes 0x1074e4) — the differ's
                // normalize_o2_row arm parses exactly this. (D137's
                // "0x24 apart / len 0x28" was an arithmetic
                // impossibility for these cells; this assert is what
                // caught it.)
                Channel::O2 => {
                    if hi - lo != 4 {
                        return Err(die(format!(
                            "static-map-wh exw_addr cells are no longer adjacent: \
                             {lo:#x}..{hi:#x}"
                        )));
                    }
                    plan(Form::Span {
                        base: lo,
                        len: 4 + 4,
                        cells,
                    })
                }
            }
        }
        "static-cursor-clamp" => {
            let cells = exd_cells(ch.src(row));
            if cells.len() != 2 {
                return Err(die(format!(
                    "static-cursor-clamp {} no longer has exactly 2 cells",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    }
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
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-tot-volume has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
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
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-dat-volume has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
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
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-claim-bank has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
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
        // Pinned: the .MIN bank is the ArenaAlloc(0x7530 = 30000 B)
        // verbatim zone-file image (RE-EXW-SIM sec 7j.62 / D149) — the
        // stale tail beyond the file prefix is proven never read, so
        // the full arena allocation is the extent.
        "static-min-bank" => {
            if !row.indirect {
                return Err(die("static-min-bank lost its indirect flag".into()));
            }
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-min-bank has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            if row.extent != "0x7530 (30000 B)" {
                return Err(die(format!(
                    "static-min-bank extent {:?} changed from the pinned 0x7530 \
                     ArenaAlloc size (7j.62/D149): update dbx-plan if the pinned \
                     size moved",
                    row.extent
                )));
            }
            plan(Form::PtrCell {
                cell,
                len_expr: "0x7530".into(),
            })
        }
        // Pinned: the CGR height bank is the UNIFORM 132354-B file
        // image (7j.69/D161) — u16 count 128 + 512-B self-relative
        // directory + 128 x 1030-B records, every shipped .CGR exactly
        // that (FORMATS sec 18, VERIFIED 44/44). The 0x20788 arena is
        // documented but deliberately NOT the extent (unlike the MIN
        // bank, the CGR corpus is uniform, so the tightest pin is the
        // file image — and it keeps the passthrough compare free of
        // the 646-B stale arena tail).
        "static-cgr-volume" => {
            if !row.indirect {
                return Err(die("static-cgr-volume lost its indirect flag".into()));
            }
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-cgr-volume has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            if row.extent != "0x20562 (132354 B file image)" {
                return Err(die(format!(
                    "static-cgr-volume extent {:?} changed from the pinned uniform \
                     132354-B file image (7j.69/D161): update dbx-plan if the pinned \
                     image size moved",
                    row.extent
                )));
            }
            plan(Form::PtrCell {
                cell,
                len_expr: "0x20562".into(),
            })
        }
        // Pinned: the BIN sprite bank extent is the 0x258960 BOOT-PASS
        // arena (7j.69/D161) — the EXW alloc 0x41d666 sits in the early
        // boot family (successor instruction loads GENERAL.BIN into the
        // SIBLING bank 0x4edd7c), NOT in FUN_0041d954; EXD twin 0x2e098.
        // The MIN-bank situation exactly (D149/D152): the shipped sizes
        // vary (2041594..2443943) and no outside cell carries the byte
        // length (the count word lives inside the bank at +0), so the
        // ARENA is the pin; the stale tail is proven never read (all
        // content readers reach sprites through the self-relative
        // directory, 7j.36).
        "static-bin-terrain" => {
            if !row.indirect {
                return Err(die("static-bin-terrain lost its indirect flag".into()));
            }
            let cell = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-bin-terrain has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            if row.extent != "0x258960 (2460000 B arena)" {
                return Err(die(format!(
                    "static-bin-terrain extent {:?} changed from the pinned 0x258960 \
                     boot-pass arena (7j.69/D161): update dbx-plan if the pinned \
                     arena size moved",
                    row.extent
                )));
            }
            plan(Form::PtrCell {
                cell,
                len_expr: "0x258960".into(),
            })
        }
        // Pinned: the LNK/LNG link table is the u16[8192] image loaded
        // DIRECTLY at the fixed .bss address (7j.69/D161) — EXW
        // 0x45cdda / EXD 0x10336c, whole-file reads behind the language
        // gates (0x4eba1c / 0x10768c) with no bound anywhere in the
        // loader; all 44 .LNK + 7 .LNG are exactly 16384 B. The old
        // "(0x8000)" gloss had no loader immediate and retires. NOT
        // pointer-indirect: the load target IS the .bss span (the
        // order-table Form::Fixed precedent, D157/D158).
        "static-lnk-map" => {
            if row.indirect {
                return Err(die(
                    "static-lnk-map became indirect — it is a DIRECT .bss span \
                     (7j.69/D161): re-pin the form if the load target moved"
                        .into(),
                ));
            }
            let addr = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-lnk-map has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            if row.extent != "0x4000 (8192 u16)" {
                return Err(die(format!(
                    "static-lnk-map extent {:?} changed from the pinned u16[8192] \
                     = 0x4000 table (7j.69/D161): update dbx-plan if the pinned \
                     table size moved",
                    row.extent
                )));
            }
            plan(Form::Fixed { addr, len: 0x4000 })
        }
        // Pinned: the order/weapon table is a DIRECT .bss fixed span —
        // 12 rows x 0x62 = 0x498 (RE-EXW-SIM sec 7j.67 / D157), pinned
        // from BOTH ends (the GameMain boot memset immediate ecx=0x498
        // @0x41c3d6 / EXD 0x2cd0f, and the successor chassis base
        // adjacent-EXW; the EXD twin sits past a 0x90-B path buffer at
        // 0x9237c — a channel layout divergence, not an extent change).
        // NOT pointer-indirect: the table IS the .bss image, so the
        // form is Fixed at the registry address (the min-bank PtrCell
        // precedent does NOT apply here).
        "static-order-table" => {
            if row.indirect {
                return Err(die(
                    "static-order-table became indirect — it is a DIRECT .bss \
                     span (7j.67/D157): re-pin the form if the cell moved"
                        .into(),
                ));
            }
            let addr = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                die(format!(
                    "static-order-table has no parsable {} address: {:?}",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    },
                    ch.src(row)
                ))
            })?;
            if row.extent != "0x498 (12x0x62 rows)" {
                return Err(die(format!(
                    "static-order-table extent {:?} changed from the pinned \
                     12x0x62 = 0x498 geometry (7j.67/D157): update dbx-plan if \
                     the pinned geometry moved",
                    row.extent
                )));
            }
            plan(Form::Fixed { addr, len: 0x498 })
        }
        // Pinned: the y-line row table is h dwords = 4*$map_h (7j.69,
        // re-verifying the S0-08/D147 loops first-hand: the EXD bound
        // is h<<2 under `jl` @0x2e727 — h entries, NOT h+1). The
        // registry row's SECOND table (the z-base plane table, 8
        // dwords) is NON-CONTIGUOUS with this one and the inter-table
        // gap DIFFERS per channel (EXW 0x1cc apart; EXD ~0x7c000
        // apart) — no single span mirrors the layout, so the row
        // emits TWO spans: this one under the registry id, the z-base
        // under the derived id `static-yline-zbase#zbase` (see
        // companion_span).
        "static-yline-zbase" => {
            let cells = exd_cells(ch.src(row));
            if cells.len() != 2 {
                return Err(die(format!(
                    "static-yline-zbase {} no longer has exactly 2 cells",
                    if ch == Channel::O1 {
                        "exd_addr"
                    } else {
                        "exw_addr"
                    }
                )));
            }
            if row.extent != "4*$map_h (h dwords) + 0x20 (8 dwords)" {
                return Err(die(format!(
                    "static-yline-zbase extent {:?} changed from the pinned two-span \
                     form 4*h + 8 dwords (7j.69/D161): update dbx-plan if the pinned \
                     table geometry moved",
                    row.extent
                )));
            }
            // [derived-pinned] cell[0] is the y-line table on BOTH
            // channels (EXW 0x4ea900 / EXD 0x8b78c — the registry
            // slash order; the geometry assert catches a swap).
            let (yline, zbase) = (cells[0], cells[1]);
            match ch {
                Channel::O1 => {
                    if yline != 0x8b78c || zbase != 0x107718 {
                        return Err(die(format!(
                            "static-yline-zbase exd_addr cells moved: expected \
                             y-line 0x8b78c / z-base 0x107718 (7j.69), got \
                             {yline:#x}/{zbase:#x}"
                        )));
                    }
                }
                Channel::O2 => {
                    if yline != 0x4ea900 || zbase != 0x4eaacc {
                        return Err(die(format!(
                            "static-yline-zbase exw_addr cells moved: expected \
                             y-line 0x4ea900 / z-base 0x4eaacc (7j.69), got \
                             {yline:#x}/{zbase:#x}"
                        )));
                    }
                }
            }
            plan(Form::CountExpr {
                addr: yline,
                len_expr: "4*$map_h".into(),
            })
        }
        other => Err(die(format!(
            "registry row {other:?} (tier {}) has no dbx-plan resolution form",
            row.tier
        ))),
    }
}

/// The DERIVED second span of a multi-table row (7j.69/D161): a row
/// whose tables are non-contiguous AND differently-laid-out per
/// channel emits one span per table — the registry id keeps the
/// row's first table, the companion rides `<id>#<name>`. capgen's
/// keep-first dedupe never drops it (distinct ids) and the differ's
/// static-* passthrough compares each span byte-exact. None for
/// every single-span row.
fn companion_span(row: &diffharness::Watch, ch: Channel) -> Result<Option<RowPlan>, PlanError> {
    match row.id.as_str() {
        // The z-base plane table: exactly 8 dwords (the loop eax =
        // 4,8,..,0x20 under `jne` @0x2e748/D147 — the store base
        // +0x107714/+0x4eaac8 is the adjacent pre-incremented
        // screen-scale cell, never a table entry). resolve_row
        // already asserted the cell pair per channel.
        "static-yline-zbase" => {
            let zbase = exd_cells(ch.src(row))[1];
            Ok(Some(RowPlan {
                id: "static-yline-zbase#zbase".into(),
                form: Form::Fixed {
                    addr: zbase,
                    len: 32,
                },
            }))
        }
        _ => Ok(None),
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

fn compile_steps(
    scen: &Scenario,
    reg: &[diffharness::Watch],
    ch: Channel,
) -> Result<CompiledSteps, PlanError> {
    // Walk phase (before until-anchor): BOOT writes at frame 0 + the
    // SCRIPTED MENU WALK (D84) — stop-indexed keystore writes, one stop
    // per counter-writing screen frame, re-armed per input because the
    // AnyKeyWait twin consumes bytes on read. ORDER/PAD/COMMAND are
    // mission-phase seams — the menu walk is keyboard-driven.
    // O2: the walk is REFUSED — the BPLM stop machinery is DOSBox/O1
    // only (D84); a scripted menu walk under Wine needs W11 driver
    // support that does not exist yet (the channel flag never invents
    // capture semantics, it only swaps addresses).
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
                let cells = exd_cells(ch.src(row));
                if cells.is_empty() {
                    return Err(die(format!(
                        "boot write {key} ({row_id}): the {} alias is a registry gap \
                         (status {:?}) — the plan never fabricates its address",
                        if ch == Channel::O1 { "EXD" } else { "EXW" },
                        row.exd_status
                    )));
                }
                boot.push(InjectWrite::Plain {
                    frame: None,
                    addr: ch.addr(cells[0]),
                    bytes: (*value as u32)
                        .to_le_bytes()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                });
            }
            Step::Keystore { entries } => {
                if ch == Channel::O2 {
                    return Err(die(
                        "o2 channel: walk-phase keystore steps have no O2 form — the \
                         BPLM stop-indexed menu walk is DOSBox/O1 machinery (D84); a \
                         scripted menu walk under Wine needs W11 driver support"
                            .into(),
                    ));
                }
                walk_boundary += 1;
                let row = reg
                    .iter()
                    .find(|r| r.id == "inj-key-state")
                    .ok_or_else(|| die("registry row inj-key-state missing".into()))?;
                let base = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                    die(
                        "walk keystore step: inj-key-state EXD alias is a registry gap \
                         (status {:?}) — anchor it before this scenario can compile for O1"
                            .to_string(),
                    )
                })?;
                for (scan, val) in entries {
                    walk_rows.push(WalkWrite {
                        stop: walk_boundary,
                        addr: ch.addr(base + *scan as u64),
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
                let base = exd_cells(ch.src(row)).first().copied().ok_or_else(|| {
                    die("keystore step: inj-key-state channel alias is a registry gap".into())
                })?;
                entries
                    .iter()
                    .map(|(scan, val)| InjectWrite::Plain {
                        frame: Some(boundary),
                        addr: ch.addr(base + *scan as u64),
                        bytes: format!("{val:02x}"),
                    })
                    .collect::<Vec<_>>()
            }
            Step::Order { x, y, z } => {
                let row = reg
                    .iter()
                    .find(|r| r.id == "order-target")
                    .ok_or_else(|| die("registry row order-target missing".into()))?;
                let cells = exd_cells(ch.src(row));
                if cells.len() != 3 {
                    return Err(die(
                        "order step: order-target channel alias is a registry gap (needs \
                         all three xyz cells)"
                            .into(),
                    ));
                }
                [*x, *y, *z]
                    .into_iter()
                    .zip(cells)
                    .map(|(v, cell)| InjectWrite::Plain {
                        frame: Some(boundary),
                        addr: ch.addr(cell),
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
                let bank = exd_cells(ch.src(bank_row))
                    .first()
                    .copied()
                    .ok_or_else(|| {
                        die(format!(
                            "pad step: static-pad-slots {} alias is a registry gap \
                         (status {:?}) — the pad bank is the step's READ anchor and \
                         is never fabricated",
                            if ch == Channel::O1 { "EXD" } else { "EXW" },
                            bank_row.exd_status
                        ))
                    })?;
                let row = reg
                    .iter()
                    .find(|r| r.id == "order-target")
                    .ok_or_else(|| die("registry row order-target missing".into()))?;
                let cells = exd_cells(ch.src(row));
                if cells.len() != 3 {
                    return Err(die(
                        "pad step: order-target channel alias is a registry gap (needs \
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
                    bank: ch.addr(bank),
                    slot: *slot,
                    target: core::array::from_fn(|i| ch.addr(cells[i])),
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
                let base = exd_cells(ch.src(ring)).first().copied().ok_or_else(|| {
                    die("command step: inj-command-ring channel alias is a registry gap".into())
                })?;
                let cell = exd_cells(ch.src(count)).first().copied().ok_or_else(|| {
                    die("command step: inj-command-count channel alias is a registry gap".into())
                })?;
                vec![InjectWrite::Command {
                    frame: boundary,
                    base: ch.addr(base),
                    count_cell: ch.addr(cell),
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
            if exd_cells(ch.src(row)).is_empty() {
                return Err(die(format!(
                    "injection step on seam {row_id} ({}): the {} alias is a registry \
                     gap (status {:?}) — anchor it before this scenario can compile \
                     on this channel; the engine side (W6) consumes the step directly",
                    step_kind(step),
                    if ch == Channel::O1 { "EXD" } else { "EXW" },
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
/// anti-ghost rule holds for calibration rows too). O1-ONLY by
/// construction: the O2 channel refuses walk-bearing scenarios in
/// compile_steps, so walk_rows is empty there and this never runs.
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

/// The span-bearing form under an optional count-cell prefix (one
/// level — resolve_row never nests prefixes).
fn span_form(form: &Form) -> &Form {
    match form {
        Form::Prefixed { inner, .. } => inner.as_ref(),
        f => f,
    }
}

/// One RowPlan as a plan-JSON watch row (addr/len per form; a
/// Prefixed row adds the capgen `prefix` sub-row — dump the 4-byte
/// count cell first, then the span, one concatenated blob). The addr
/// FORM is channel-selected (CS: selector vs flat EXW linear).
fn watch_row_json(p: &RowPlan, ch: Channel) -> String {
    match &p.form {
        Form::Fixed { addr, len } => format!(
            "    {{ \"id\": {}, \"addr\": \"{}\", \"len\": {len} }}",
            jstr(&p.id),
            ch.addr(*addr)
        ),
        Form::Span { base, len, .. } => format!(
            "    {{ \"id\": {}, \"addr\": \"{}\", \"len\": {len} }}",
            jstr(&p.id),
            ch.addr(*base)
        ),
        Form::PtrCell { len_expr, .. } => {
            let sym = match p.id.as_str() {
                "static-tot-volume" => "tot_ptr",
                "static-dat-volume" => "dat_ptr",
                "static-claim-bank" => "claim_ptr",
                "static-min-bank" => "min_ptr",
                "static-cgr-volume" => "cgr_ptr",
                "static-bin-terrain" => "bin_ptr",
                "object-instances" => "obj_ptr",
                // the D162 per-frame tile-claims row (§5i): same cell
                // as the TS row, its own symbol (per-row maps)
                "tile-claims" => "claim3_ptr",
                _ => unreachable!("emit_plan gates the PtrCell ids"),
            };
            format!(
                "    {{ \"id\": {}, \"addr\": \"{}\", \"len\": {} }}",
                jstr(&p.id),
                ch.sym_addr(sym),
                jstr(len_expr)
            )
        }
        Form::CountExpr { addr, len_expr } => format!(
            "    {{ \"id\": {}, \"addr\": \"{}\", \"len\": {} }}",
            jstr(&p.id),
            ch.addr(*addr),
            jstr(len_expr)
        ),
        Form::Prefixed { cell, inner } => {
            let mut s = watch_row_json(
                &RowPlan {
                    id: p.id.clone(),
                    form: inner.as_ref().clone(),
                },
                ch,
            );
            let tail = format!(
                ", \"prefix\": {{ \"addr\": \"{}\", \"len\": 4 }} }}",
                ch.addr(*cell)
            );
            let cut = s.trim_end_matches(" }").len();
            s.replace_range(cut.., &tail);
            s
        }
    }
}

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

/// The O1 default (byte-identical to every committed capture plan —
/// the `s*_plan_matches_committed_artifact` tests pin it). Test-module
/// convenience: `main` calls `emit_plan_channel` directly.
#[cfg(test)]
fn emit_plan(scen: &Scenario, reg: &[diffharness::Watch]) -> Result<Emitted, PlanError> {
    emit_plan_channel(scen, reg, Channel::O1)
}

/// The channel-aware emitter (D138): `--channel o2` swaps every
/// address to the registry `exw_addr` canon cell (flat linear form),
/// applies the ONE D137 span split (static-map-wh), reads the map w/h
/// resolve pair from the EXW cells, and replaces the DOSBox boot/arm
/// command machinery with the ptrace `trigger` object. Everything
/// else (row set, extents, resolve symbols, staging seams, frames
/// contract) is channel-symmetric.
fn emit_plan_channel(
    scen: &Scenario,
    reg: &[diffharness::Watch],
    ch: Channel,
) -> Result<Emitted, PlanError> {
    // Tier gate: only the S0 shape today.
    for t in &scen.tiers {
        if !SUPPORTED_TIERS.contains(&t.as_str()) {
            return Err(die(format!(
                "scenario {} tier {t:?} is not compilable (dbx-plan supports {:?}; \
                 the T2/T3 alias set is complete since D162 — a new tier needs \
                 its own forms)",
                scen.id, SUPPORTED_TIERS
            )));
        }
    }

    // W5 step compilation (DESIGN §5): boot writes + frame-boundary
    // inject rows + the D84 walk rows (the scripted menu walk), all
    // gated on registry aliases (gaps are named, never fabricated).
    // Emitted keys exist only when the scenario carries them (S0/S1
    // artifacts stay minimal).
    let (boot_writes, inject_rows, walk_rows) = compile_steps(scen, reg, ch)?;
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
        .find(|r| r.id == "frame-counter" && r.tier == "T0" && !ch.src(r).is_empty())
        .ok_or_else(|| die("registry row frame-counter (T0) missing".into()))?;
    let trigger = reg
        .iter()
        .find(|r| r.id == "s0-trigger" && r.tier == "S0" && !ch.src(r).is_empty())
        .ok_or_else(|| die("registry row s0-trigger (S0) missing".into()))?;
    let fc_cell = exd_cells(ch.src(frame_counter))
        .first()
        .copied()
        .ok_or_else(|| die("frame-counter channel address does not parse".into()))?;
    let tail = exd_cells(ch.src(trigger))
        .first()
        .copied()
        .ok_or_else(|| die("s0-trigger channel address does not parse".into()))?;

    // Resolve rows (registry order preserved).
    let mut anchor: Vec<RowPlan> = Vec::new();
    let mut per_frame: Vec<RowPlan> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    for row in reg {
        if !scen.tiers.contains(&row.tier) {
            continue; // e.g. the S0 trigger row: not a dump row
        }
        match resolve_row(row, ch)? {
            Some(p) => {
                if row.tier == "TS" {
                    anchor.push(p);
                } else {
                    anchor.push(p.clone());
                    per_frame.push(p);
                }
                // The multi-table rows contribute their derived second
                // span (anchor-riding like every TS static; a
                // non-TS multi-table row would need the same treatment
                // in per_frame — none exists today, so gate loudly).
                if let Some(c) = companion_span(row, ch)? {
                    if row.tier != "TS" {
                        return Err(die(format!(
                            "row {} needs a per-frame companion span — extend \
                             the emit loop for tier {}",
                            row.id, row.tier
                        )));
                    }
                    anchor.push(c);
                }
            }
            None => deferred.push(format!("{} ({})", row.id, row.extent)),
        }
    }

    // Resolve cells: the map w/h loader statics + the pointer cells of
    // every PtrCell row above (each derived from ITS row's channel
    // address field).
    let map_wh = reg
        .iter()
        .find(|r| r.id == "static-map-wh")
        .ok_or_else(|| die("registry row static-map-wh missing".into()))?;
    let map_cells = exd_cells(ch.src(map_wh));
    if map_cells.len() != 2 {
        return Err(die(format!(
            "static-map-wh {} no longer has exactly 2 cells",
            if ch == Channel::O1 {
                "exd_addr"
            } else {
                "exw_addr"
            }
        )));
    }
    // [derived-pinned] the slash order is w / h on BOTH channels, but
    // the GEOMETRY differs (the D137 split, arithmetic corrected by
    // D138): the EXD pair (w 0x1074b8 / h 0x10748c, RE-EXD-MAP sec 5b)
    // sits 0x2c apart with h LOW, while the EXW pair (w 0x4eddec / h
    // 0x4eddf0, sec 7j.60) is ADJACENT (4 apart) with w LOW — the
    // resolve pair reads the channel's own cells and the gap assert
    // pins the geometry per channel.
    let (w_cell, h_cell) = (map_cells[0], map_cells[1]);
    match ch {
        Channel::O1 => {
            if w_cell != h_cell + 0x2c {
                return Err(die(format!(
                    "static-map-wh exd_addr cell geometry changed: expected w above h \
                     by 0x2c, got {w_cell:#x}/{h_cell:#x}"
                )));
            }
        }
        Channel::O2 => {
            if h_cell != w_cell + 4 {
                return Err(die(format!(
                    "static-map-wh exw_addr cell geometry changed: expected w below h \
                     by 4 (adjacent u32s), got {w_cell:#x}/{h_cell:#x}"
                )));
            }
        }
    }
    let mut resolve: Vec<(String, u64)> = vec![("map_w".into(), w_cell), ("map_h".into(), h_cell)];
    for p in &anchor {
        if let Form::PtrCell { cell, .. } = span_form(&p.form) {
            let name = match p.id.as_str() {
                "static-tot-volume" => "tot_ptr",
                "static-dat-volume" => "dat_ptr",
                "static-claim-bank" => "claim_ptr",
                "static-min-bank" => "min_ptr",
                "static-cgr-volume" => "cgr_ptr",
                "static-bin-terrain" => "bin_ptr",
                "object-instances" => "obj_ptr",
                "tile-claims" => "claim3_ptr",
                other => {
                    return Err(die(format!(
                        "PtrCell row {other:?} has no resolve symbol in dbx-plan"
                    )))
                }
            };
            resolve.push((name.into(), *cell));
        }
    }
    // Every $symbol referenced by any len expression (CountExpr bank
    // rows AND PtrCell lens) must carry a resolve row: count cells come
    // from the bank row's own exd_addr (second cell), map w/h from
    // static-map-wh.
    let mut lens: Vec<&str> = Vec::new();
    for p in &anchor {
        match span_form(&p.form) {
            Form::CountExpr { len_expr, .. } | Form::PtrCell { len_expr, .. } => {
                lens.push(len_expr)
            }
            Form::Fixed { .. } | Form::Span { .. } => {}
            Form::Prefixed { .. } => unreachable!("span_form unwraps prefixes"),
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
                exd_cells(ch.src(row)).get(1).copied().ok_or_else(|| {
                    die(format!(
                        "row {} {} lost its count cell",
                        row.id,
                        if ch == Channel::O1 {
                            "exd_addr"
                        } else {
                            "exw_addr"
                        }
                    ))
                })?
            }
            // the D162 census count cells (§5i): the .NME loader twins
            "critter_count" | "poi_count" => {
                let row_id = if name == "critter_count" {
                    "critter-bank"
                } else {
                    "poi-bank"
                };
                let row = reg
                    .iter()
                    .find(|r| r.id == row_id)
                    .ok_or_else(|| die(format!("symbol ${name} has no source registry row")))?;
                exd_cells(ch.src(row)).get(1).copied().ok_or_else(|| {
                    die(format!(
                        "row {} {} lost its count cell",
                        row.id,
                        if ch == Channel::O1 {
                            "exd_addr"
                        } else {
                            "exw_addr"
                        }
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

    let watch_json = |p: &RowPlan| -> String { watch_row_json(p, ch) };

    let mut j = String::new();
    j.push_str("{\n");
    match ch {
        Channel::O1 => {
            let boot_note = if walk_rows.is_empty() {
                "Responsive boot: BPINT 21 AH=4B stops at EXEC, then a real-mode BP 5FBB:0000 resolves to the verified EXD linear entry 0x0005FBB0. Fresh EV CS EIP CR0 + SELINFO CS output proves the protected-mode flat entry before the mission anchor is armed; BPLIST proves that anchor is the only remaining breakpoint. All mission-anchor waits use plain RUN."
            } else {
                "WALK boot: BPLM on the frame-counter cell is armed at the parked pre-boot halt and stays active for the stop-indexed menu walk; this legacy heavy-watch flow deliberately retains RUNWATCH."
            };
            j.push_str(&format!(
                "  \"_comment\": \"{} live capture plan (D81/D84; GENERATED by dbx-plan from watches.toml - do not hand-edit, regenerate). {boot_note} Mission anchor: BP CS:{tail:08X} = the registry s0-trigger row (the BP ack echoes the numeric selector - the per-run pin). resolve_at=anchor: the loader statics (map w/h, TOT/DAT/claim pointers) are MISSION-load values - they are read at the anchor stop (mission start), never at the pre-mission arm stop (D84). WALK phase (D84, when the walk key is present): the BPLM stays armed after the accept stop; one stop per counter-writing screen frame; stop i applies its rows via SMV (they become screen frame i+1 input - keystore writes need re-arm per input, the AnyKeyWait twin consumes on read); arm_commands run at the LAST walk stop (BPDEL * drops the BPLM, BP arms the anchor); walk_watches are calibration dumps at every stop riding the transcript as comments. Anchor frame = the first BP hit after arm; alignment is by the frame-counter watch. TS statics ride the anchor frame; T0 rows every frame. Deferred TS rows carry unpinned extents (see _deferred). Without a walk key the operator walks the title menu on the desktop; the anchor frame-counter and RNG bytes are menu-timing dependent across runs (T2/T3 classes, DESIGN section 6) - the live double-run verdict is identical-chains-modulo-those-cells; byte-identical chains need the scripted walk (S0W).\",\n",
                scen.id
            ));
            j.push_str("  \"logfile\": \"dosbox-harness.log\",\n");
            j.push_str("  \"time_limit\": 1800,\n");
            j.push_str("  \"boot_timeout\": 1800,\n");
            j.push_str("  \"boot_retries\": 24,\n");
        }
        Channel::O2 => {
            j.push_str(&format!(
                "  \"_comment\": \"{} O2 spot-check capture plan (W11-prep, D138; GENERATED by dbx-plan --channel o2 from watches.toml - do not hand-edit, regenerate). Channel form (DESIGN section 2 O2 + section 10 W11): every addr is the registry row EXW canon address (0x-prefixed flat linear), read DIRECTLY by the host ptrace driver - zero address translation. trigger.site = the s0-trigger row's OPERATIONAL frame-tail PresentEnd CALL site 0x004486C9 (D156: the row's exw_addr 0x00425A03 keeps the CALLEE canon - PresentEnd has 62 direct call sites, so the entry is not a usable frame-tail trigger; the first hit after mission load is the anchor frame), trigger.frame_counter = the EXW g_frame_count cell (frame alignment). resolve rows are read at the anchor hit (resolve_at=anchor - the loader statics are mission-load values). static-map-wh rides the D137/D138 O2 form: ONE 8-byte span at 0x004EDDEC with w at +0x00 / h at +0x04 (the EXW cells are ADJACENT u32s with w LOW - NOT the EXD 0x30 span, h LOW 0x2c apart; D137 0x24-apart arithmetic corrected by D138). selection-triple dumps the SELECTED-SLOT cell 0x0046CBDC (the EXW list is FIELD-ordered base/selected/size but selected is the highest address - the D132 pairing). Deferred rows: any row whose EXW address is a registry gap (the EXD-only rows - static-cursor-clamp; never fabricated) — the T2/T3 set is fully aliased since D162 (RE-EXD-MAP sec 5i) and emits registry-driven. inject/boot_writes rows carry EXW seam cells with frame = the Nth trigger hit after the anchor; injection on O2 is driver policy (DESIGN section 10 W11 names process_vm_readv observation - the rows are data for a writing driver). Walk-phase keystore scenarios are REFUSED on o2 (the BPLM stop-indexed menu walk is DOSBox/O1 machinery, D84). TS statics ride the anchor frame; T0 rows every frame. The tiebreak verdict this plan feeds: dbx-diff cross-channel with the O2 dump as the arbiter.\",\n",
                scen.id
            ));
            j.push_str("  \"channel\": \"o2\",\n");
            j.push_str("  \"logfile\": \"o2-harness.log\",\n");
            j.push_str("  \"time_limit\": 1800,\n");
        }
    }
    j.push_str(&format!("  \"frames\": {},\n", scen.frames + 1));
    j.push_str("  \"resolve_at\": \"anchor\",\n");
    // D91: the markers staging key is an E-side seam — the O1 capture
    // has NO equivalent write (fabricating an 0xA8 robot record +
    // count bump would be ghost staging). Record it explicitly so the
    // live comparison knows the robot-count diff is the scenario
    // seam, never a finding. D103 (grammar v1.3): the loadout key is
    // the same discipline — the O1 side arms robots by playing the
    // session (the original fills the slots from the session table
    // at spawn), so the weapon-slot/ammo diff is the recorded seam.
    // D105 (grammar v1.4): the destroy key is an EQUIVALENCE seam —
    // the original loads the mission's .BDG/.POS/.TRT natively at
    // mission load, so the staged content is identical on both
    // channels (no O1 write); the mirror banks stage EMPTY on E
    // until the S5 init_tiles pairing. D108 (grammar v1.5): the
    // zone + pickup keys are the same equivalence discipline (the
    // campaign-slot shells + the init_tiles TOT fill). Byte-identity:
    // loadout-less/zone-less/pickup-less scenarios emit the same
    // bytes as before (the pinned capture-plans).
    let mut staging: Vec<String> = Vec::new();
    if !scen.markers.is_empty() {
        let mut m = String::from("    \"markers\": [\n");
        for (i, (x, y, z)) in scen.markers.iter().enumerate() {
            m.push_str(&format!(
                "      {{ \"x\": {x}, \"y\": {y}, \"z\": {z} }}{}",
                if i + 1 < scen.markers.len() {
                    ",\n"
                } else {
                    "\n"
                }
            ));
        }
        m.push_str(
            "    ],\n    \"note\": \"E-side staging seam (D91): extra squad robots the \
             ENGINE canonical run banks after the MRK squad. The O1 capture stages \
             NO equivalent (never fabricated): its robot-count diff vs E is this \
             scenario seam, not a finding\"",
        );
        staging.push(m);
    }
    if !scen.loadout.is_empty() {
        let mut l = String::from("    \"loadout\": [\n");
        for (i, lr) in scen.loadout.iter().enumerate() {
            let slots = lr
                .slots
                .iter()
                .map(|(id, ammo)| format!("{id:#x}:{ammo}"))
                .collect::<Vec<_>>()
                .join(", ");
            // NOTE: mask is DECIMAL here — JSON has no hex literals
            // (the D103 emitter's 0x-form made loadout-bearing plans
            // unparseable; S3 is the first compilable one, D109). The
            // slot ids stay hex INSIDE the slots string.
            l.push_str(&format!(
                "      {{ \"robot\": {}, \"mask\": {}, \"slots\": \"{}\" }}{}",
                lr.robot,
                lr.mask,
                slots,
                if i + 1 < scen.loadout.len() {
                    ",\n"
                } else {
                    "\n"
                }
            ));
        }
        l.push_str(
            "    ],\n    \"loadout_note\": \"E-side staging seam (D103, grammar v1.3): \
             weapon slots staged through the stage_robot_weapons host seam on E. The \
             O1 capture arms its robots by playing the session (the original fills the \
             slots from the session table at spawn); the weapon-slot/ammo diff vs E is \
             this scenario seam, not a finding\"",
        );
        staging.push(l);
    }
    if scen.destroy {
        staging.push(
            "    \"destroy\": true,\n    \"destroy_note\": \"E-side EQUIVALENCE seam \
             (D105, grammar v1.4): the mission's own .BDG type table + .POS instances \
             + .TRT structures staged through the stage_destroy_family host seam on E. \
             The ORIGINAL loads all three files natively at mission load \
             (FUN_0041a4f8 + FUN_004170a6), so the staged CONTENT is identical on both \
             channels — no O1 write, no seam diff; the destroy-row bytes compare \
             directly. The TOT-mirror/seen banks stage EMPTY on E (the init_tiles TOT \
             fill is the S5 pairing — the recorded mirror-rows divergence until then)\""
                .to_string(),
        );
    }
    if let Some(z) = scen.zone {
        staging.push(format!(
            "    \"zone\": \"{z}\",\n    \"zone_note\": \"E-side EQUIVALENCE seam \
             (D108, grammar v1.5): the campaign episode slot staged to zone {z} \
             through the stage_episode_slot host seam (the host stands in for the \
             campaign-advance / save-load-restore shells). The LIVE O1 capture \
             reaches this zone by playing the campaign or a save — its own \
             linear/mission counters are the live-capture seam, never fabricated; \
             record the session's zone cell (1-based set) for the cross-check\""
        ));
    }
    if scen.pickup {
        staging.push(
            "    \"pickup\": true,\n    \"pickup_note\": \"E-side EQUIVALENCE seam \
             (D108, grammar v1.5): the mission's own .TOT staged through the \
             stage_pickup_surface host seam (the init_tiles fill + the zone/set \
             cell + the load-order hazard stamper). The ORIGINAL stages the same \
             volume natively at mission load (FUN_00407e11), so the mirror-row \
             bytes compare directly — the S4-era empty-mirror divergence is closed \
             for pickup scenarios\""
                .to_string(),
        );
    }
    if scen.platforms {
        staging.push(
            "    \"platforms\": true,\n    \"platforms_note\": \"E-side arm key \
             (D113, grammar v1.6): the epilogue creep tick FUN_00422a9c runs \
             ARMED on E (the MissionShell epilogue call, §7j.41/4). The ORIGINAL \
             calls it EVERY frame unconditionally — one RandA gate-draw per frame \
             consumed even with no platform staged — so on O1 the tick needs NO \
             staging (an equivalence), but the per-frame draw shifts the O1 RNG \
             stream vs an unarmed E run on EVERY scenario: the rng-state rows are \
             the channel finding class (budgeted, never a fabricated write). The \
             platform banks/tick writers need no inject rows — the run's own fire \
             produces every state change\""
                .to_string(),
        );
    }
    if scen.critters {
        staging.push(
            "    \"critters\": true,\n    \"critters_note\": \"E-side staging+arm key \
             (D114, grammar v1.7): the mission's .NME staged through the \
             FUN_00416458 spawn schedule (stage_critters — the §7j.18 grammar, \
             difficulty-scaled) and the controller FUN_00412f34 ARMED (the \
             MissionShell 0x447fe1 call). The ORIGINAL loads .NME natively at \
             every mission load and runs the controller UNGATED — the loader's \
             kind-4 heading draws + the controller's per-frame draws are \
             CONSUMED on O1 on every scenario — so an unarmed E run's rng-state \
             rows drift vs O1 (the budgeted channel class, like platforms). No \
             inject rows: the run's own engage/fire/death traffic produces every \
             state change; the critter bank + effect rows are EXD-ALIASED since \
             D162 (RE-EXD-MAP sec 5i: 0x10e81c + count 0x1194dc / 0x9d534) — \
             their cross-channel FIELD compare awaits the differ extraction \
             arms (the named follow-up), the 0x68 fire rides the ALIASED projectile \
             bank\""
                .to_string(),
        );
    }
    if !staging.is_empty() {
        j.push_str("  \"_e_staging\": {\n");
        j.push_str(&staging.join(",\n"));
        j.push_str("\n  },\n");
    }
    match ch {
        Channel::O1 => {
            // The DOSBox debugger machinery. Non-walk O1 plans opt into
            // the responsive code-BP entry path; stop-indexed menu walks
            // retain the BPLM that drives their walk stops.
            j.push_str(
                "  \"env\": { \"SDL_VIDEODRIVER\": \"\", \"SDL_AUDIODRIVER\": \"dummy\" },\n",
            );
            if walk_rows.is_empty() {
                j.push_str("  \"boot_trap\": \"entry\",\n");
            } else {
                j.push_str("  \"boot_commands\": [\n");
                j.push_str(&format!(
                    "    {{ \"cmd\": \"BPLM {:X}\", \"expect\": \"Set linear memory breakpoint at {fc_cell:08X}\" }}\n",
                    fc_cell
                ));
                j.push_str("  ],\n");
            }
            j.push_str("  \"arm_commands\": [\n");
            j.push_str("    { \"cmd\": \"BPDEL *\", \"expect\": \"Breakpoints deleted\" },\n");
            j.push_str(&format!(
                "    {{ \"cmd\": \"BP CS:{tail:08X}\", \"expect\": \"Set breakpoint at\" }}\n",
            ));
            j.push_str("  ],\n");
        }
        Channel::O2 => {
            // The ptrace trigger (DESIGN section 10 W11): the EXW
            // OPERATIONAL frame-tail call site (O2_TRIGGER_SITE, D156
            // — the s0-trigger row's exw_addr 0x425a03 is the CALLEE
            // canon, not a usable trigger: 62 direct call sites) + the
            // EXW frame-counter cell (registry-derived, anti-ghost),
            // replacing the DOSBox boot/arm commands.
            j.push_str(&format!(
                "  \"trigger\": {{ \"site\": \"{}\", \"frame_counter\": \"{}\" }},\n",
                ch.addr(O2_TRIGGER_SITE),
                ch.addr(fc_cell)
            ));
        }
    }
    j.push_str("  \"resolve\": [\n");
    for (i, (name, cell)) in resolve.iter().enumerate() {
        j.push_str(&format!(
            "    {{ \"name\": {}, \"addr\": \"{}\", \"len\": 4 }}{}",
            jstr(name),
            ch.addr(*cell),
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
            eprintln!(
                "usage: dbx-plan <scenario.scen> [--out <capture-plan.json>] [--channel o1|o2]"
            );
            return ExitCode::FAILURE;
        }
    };
    let mut out_path: Option<PathBuf> = None;
    let mut channel = Channel::O1;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => match args.next() {
                Some(p) => out_path = Some(PathBuf::from(p)),
                None => {
                    eprintln!("dbx-plan: --out needs a path");
                    return ExitCode::FAILURE;
                }
            },
            "--channel" => match args.next().as_deref() {
                Some("o1") => channel = Channel::O1,
                Some("o2") => channel = Channel::O2,
                other => {
                    eprintln!("dbx-plan: --channel expects o1 or o2, got {other:?}");
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
    let emitted = match emit_plan_channel(&scen, &reg, channel) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "dbx-plan: scenario {} -> channel {} -> {} anchor rows + {} per-frame rows, {} deferred, {} inject rows, {} walk rows; \
         frames={} (anchor + {} post-anchor records for the stitcher)",
        scen.id,
        match channel {
            Channel::O1 => "o1",
            Channel::O2 => "o2",
        },
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
        // the RETIRED pre-D158 order-table string stays a None pin
        // (the '-' is not a delimiter); its pinned replacement parses
        assert_eq!(parse_extent("0x62-stride rows"), None);
        assert_eq!(parse_extent("0x498 (12x0x62 rows)"), Some(0x498));
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
        // T0: 11 rows, 0 gaps (difficulty closed by the W5-followup,
        // sfx-master-gate by the D134 twin census) -> 11 per-frame.
        // TS: 15 rows, 0 deferred (min-bank 0x7530 by 7j.62/D149;
        // order-table 0x498 by 7j.67/D157/S0-15a; cgr/bin/lnk/yline
        // by 7j.69/D161 — the yline row emits TWO spans, its registry
        // id + the #zbase companion) -> 11 + 11 + 5 = 27 anchor rows.
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 11, "all T0 rows (gap set empty since D134)");
        assert_eq!(
            anchor_count, 27,
            "T0 + resolved TS rows (two spans on yline)"
        );
        // ZERO deferred rows: the last four TS extent gaps closed by
        // 7j.69/D161 (the T0 EXD gap set is empty since D134)
        assert_eq!(emitted.deferred.len(), 0);
        // every emitted id is a real registry row of the scenario
        // tiers — a `#name` suffix strips to its parent row (the
        // 7j.69/D161 companion-span convention: one registry row,
        // two emitted spans)
        for id in row_ids(&emitted.json) {
            let base = id.split('#').next().unwrap_or(&id);
            let row = reg
                .iter()
                .find(|r| r.id == base)
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
        assert!(
            emitted.json.contains("\"boot_trap\": \"entry\""),
            "non-walk O1 plans must opt into the responsive entry path"
        );
        assert!(
            !emitted.json.contains("\"cmd\": \"BPLM"),
            "responsive non-walk O1 plans must not arm BPLM"
        );
        // the w/h loader statics land under their documented cells
        // (RE-EXD-MAP sec 5b: w 0x1074b8 / h 0x10748c — never swapped)
        assert!(emitted
            .json
            .contains("\"name\": \"map_w\", \"addr\": \"CS:001074B8\""));
        assert!(emitted
            .json
            .contains("\"name\": \"map_h\", \"addr\": \"CS:0010748C\""));
        // the four 7j.69/D161 pins, EXD cells: cgr_ptr/bin_ptr resolve
        // rows + the four spans + the yline #zbase companion
        assert!(emitted
            .json
            .contains("\"name\": \"cgr_ptr\", \"addr\": \"CS:00107540\""));
        assert!(emitted
            .json
            .contains("\"name\": \"bin_ptr\", \"addr\": \"CS:00107434\""));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-cgr-volume\", \"addr\": \"CS:$cgr_ptr\", \"len\": \"0x20562\" }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-bin-terrain\", \"addr\": \"CS:$bin_ptr\", \"len\": \"0x258960\" }"
        ));
        assert!(emitted
            .json
            .contains("{ \"id\": \"static-lnk-map\", \"addr\": \"CS:0010336C\", \"len\": 16384 }"));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-yline-zbase\", \"addr\": \"CS:0008B78C\", \"len\": \"4*$map_h\" }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-yline-zbase#zbase\", \"addr\": \"CS:00107718\", \"len\": 32 }"
        ));
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
        // T1: 17 rows - 1 gap (sfx-master-gate is T0; order-target
        // closed by the W5-followup, blink-cursor by D132,
        // no-extract-latch by D133) = 17 resolved (the
        // move-target-words 0x60 span filled by W7-followup2, D90).
        // T0: 11 per-frame + TS: 15 anchor rows with 0 deferred (gap
        // set empty since D134: sfx-master-gate now emits like every
        // other T0 row; min-bank resolved by 7j.62/D149; order-table
        // by 7j.67/D157 + S0-15a/D158; cgr/bin/lnk/yline by 7j.69/
        // D161 — yline emits its registry id + the #zbase companion).
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 11 + 17, "all T0 rows + T1 resolved");
        assert_eq!(anchor_count, 27 + 17, "T0 + TS + T1 rows");
        assert_eq!(
            emitted.deferred.len(),
            0,
            "zero S0-shape TS deferrals (move-target, blink-cursor, no-extract-latch AND \
             sfx-master-gate resolved by D90/D132/D133/D134; min-bank pinned by \
             7j.62/D149; order-table pinned by 7j.67/D157; cgr/bin/lnk/yline \
             pinned by 7j.69/D161)"
        );
        // count-cell resolve rows exist with the registry-derived cells
        // (obj_count is GONE — D109: the object row dumps the FULL
        // bank, so its count cell rides the blob head as the prefix,
        // not a resolve symbol)
        assert!(emitted
            .json
            .contains("\"name\": \"robot_count\", \"addr\": \"CS:0011958C\""));
        assert!(emitted
            .json
            .contains("\"name\": \"trt_count\", \"addr\": \"CS:0011949C\""));
        assert!(emitted
            .json
            .contains("\"name\": \"obj_ptr\", \"addr\": \"CS:00119584\""));
        assert!(!emitted.json.contains("\"name\": \"obj_count\""));
        // count-driven extents compiled to expressions
        assert!(emitted.json.contains("\"len\": \"$robot_count*0xA8\""));
        // D109: trt-array pins its count cell onto the blob head (the
        // differ's trt_o1 walks 0..count) and object-instances dumps
        // the FULL 2000-slot bank + count prefix (the ZONEB .POS
        // live-past-dead holes — the count-bounded span dropped 32
        // live objects, D108)
        assert!(emitted.json.contains(
            "{ \"id\": \"trt-array\", \"addr\": \"CS:00095264\", \"len\": \"$trt_count*0x20\", \
             \"prefix\": { \"addr\": \"CS:0011949C\", \"len\": 4 } }"
        ));
        // the pinned order-table row on O1 (7j.67/D157 + S0-15a/D158:
        // the 12x0x62 = 0x498 DIRECT .bss span at the EXD 0x91ee4 —
        // Fixed, never pointer-indirect)
        assert!(emitted.json.contains(
            "{ \"id\": \"static-order-table\", \"addr\": \"CS:00091EE4\", \"len\": 1176 }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"object-instances\", \"addr\": \"CS:$obj_ptr\", \"len\": \"2000*0x14\", \
             \"prefix\": { \"addr\": \"CS:00119554\", \"len\": 4 } }"
        ));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h*2\""));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h*0x1E\""));
        assert!(emitted.json.contains("\"len\": \"$map_w*$map_h\""));
        // the historical gaps all emit now (order-target closed by the
        // W5-followup; blink-cursor by D132; no-extract-latch by D133;
        // sfx-master-gate by D134 — the gap set is empty)
        assert!(
            emitted
                .json
                .contains("{ \"id\": \"sfx-master-gate\", \"addr\": \"CS:0010743C\", \"len\": 4 }"),
            "sfx-master-gate must emit its verified twin cell (D134)"
        );
        assert!(
            emitted
                .json
                .contains("{ \"id\": \"blink-cursor\", \"addr\": \"CS:0010E108\", \"len\": 4 }"),
            "blink-cursor must emit its verified twin cell (D132)"
        );
        assert!(
            emitted.json
                .contains("{ \"id\": \"no-extract-latch\", \"addr\": \"CS:000F929C\", \"len\": \"$robot_count*4\" }"),
            "no-extract-latch must emit its verified count-driven span (D133)"
        );
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
    fn s1_plan_compiles_o2() {
        // D138: the O2 channel form — every address swaps to the
        // registry exw_addr canon cell (flat 0x form), the DOSBox
        // boot/arm machinery is replaced by the ptrace trigger, and
        // the ONE span split + the D132 cell-order pick apply.
        let scen = s1();
        let reg = registry();
        let emitted = emit_plan_channel(&scen, &reg, Channel::O2).unwrap();
        // channel marker + no DOSBox machinery + no CS: form anywhere
        assert!(emitted.json.contains("  \"channel\": \"o2\",\n"));
        for banned in ["CS:", "boot_commands", "arm_commands", "\"env\""] {
            assert!(
                !emitted.json.contains(banned),
                "o2 plan must not carry the DOSBox machinery: found {banned:?}"
            );
        }
        // the ptrace trigger: the OPERATIONAL call site (D156/D171 —
        // the s0-trigger row's exw_addr keeps the callee canon
        // 0x00425A03; PresentEnd's entry has 62 call sites) + the
        // EXW g_frame_count cell
        assert!(emitted.json.contains(
            "\"trigger\": { \"site\": \"0x004486C9\", \"frame_counter\": \"0x0046AE68\" }"
        ));
        // THE span split (D137 arithmetic CORRECTED by D138): the EXW
        // w/h cells are ADJACENT u32s with w LOW — the 8-byte span,
        // NOT the EXD 0x30 span
        assert!(emitted
            .json
            .contains("{ \"id\": \"static-map-wh\", \"addr\": \"0x004EDDEC\", \"len\": 8 }"));
        assert!(
            !emitted
                .json
                .contains("{ \"id\": \"static-map-wh\", \"addr\": \"0x004EDDEC\", \"len\": 48 }"),
            "the O2 map-wh row must NOT be the EXD 0x30 span form"
        );
        // the resolve pair reads the EXW cells
        assert!(emitted
            .json
            .contains("\"name\": \"map_w\", \"addr\": \"0x004EDDEC\""));
        assert!(emitted
            .json
            .contains("\"name\": \"map_h\", \"addr\": \"0x004EDDF0\""));
        // count cells + pointer cells read the EXW twins — the
        // robot_count is 0x46cbd8 (the PER-PLAYER 0x11958c twin, the
        // W8-prep correction; the registry parenthetical was fixed in
        // this unit), NOT the TOTAL 0x46ccbc
        assert!(emitted
            .json
            .contains("\"name\": \"robot_count\", \"addr\": \"0x0046CBD8\""));
        assert!(!emitted.json.contains("0x0046CCBC"));
        assert!(emitted
            .json
            .contains("\"name\": \"trt_count\", \"addr\": \"0x0046CCD4\""));
        assert!(emitted
            .json
            .contains("\"name\": \"obj_ptr\", \"addr\": \"0x0046CBF4\""));
        assert!(emitted
            .json
            .contains("\"name\": \"claim_ptr\", \"addr\": \"0x0046AF58\""));
        // the pinned MIN-bank resolve row + span (7j.62/D149: the
        // 0x7530 ArenaAlloc image, EXW cell 0x4edd9c)
        assert!(emitted
            .json
            .contains("\"name\": \"min_ptr\", \"addr\": \"0x004EDD9C\""));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-min-bank\", \"addr\": \"$min_ptr\", \"len\": \"0x7530\" }"
        ));
        // the pinned order-table row (7j.67/D157 + S0-15a/D158: the
        // 12x0x62 = 0x498 DIRECT .bss span, EXW 0x4de664 — the Fixed
        // form, NOT pointer-indirect like min-bank)
        assert!(emitted.json.contains(
            "{ \"id\": \"static-order-table\", \"addr\": \"0x004DE664\", \"len\": 1176 }"
        ));
        // the four 7j.69/D161 pins, EXW cells: the CGR file-image
        // PtrCell (0x20562 — the uniform 132354-B image, NOT the
        // 0x20788 arena), the BIN boot-pass-arena PtrCell, the LNK
        // direct .bss u16[8192] span, and the yline/zbase pair (the
        // two-span row: the registry id + the #zbase companion)
        assert!(emitted
            .json
            .contains("\"name\": \"cgr_ptr\", \"addr\": \"0x004EDD60\""));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-cgr-volume\", \"addr\": \"$cgr_ptr\", \"len\": \"0x20562\" }"
        ));
        assert!(emitted
            .json
            .contains("\"name\": \"bin_ptr\", \"addr\": \"0x004EDE1C\""));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-bin-terrain\", \"addr\": \"$bin_ptr\", \"len\": \"0x258960\" }"
        ));
        assert!(emitted
            .json
            .contains("{ \"id\": \"static-lnk-map\", \"addr\": \"0x0045CDDA\", \"len\": 16384 }"));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-yline-zbase\", \"addr\": \"0x004EA900\", \"len\": \"4*$map_h\" }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"static-yline-zbase#zbase\", \"addr\": \"0x004EAACC\", \"len\": 32 }"
        ));
        // bank rows keep their forms on the EXW cells
        assert!(emitted.json.contains(
            "{ \"id\": \"robot-bank\", \"addr\": \"0x004C69E4\", \"len\": \"$robot_count*0xA8\" }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"trt-array\", \"addr\": \"0x004CCCF8\", \"len\": \"$trt_count*0x20\", \
             \"prefix\": { \"addr\": \"0x0046CCD4\", \"len\": 4 } }"
        ));
        assert!(emitted.json.contains(
            "{ \"id\": \"object-instances\", \"addr\": \"$obj_ptr\", \"len\": \"2000*0x14\", \
             \"prefix\": { \"addr\": \"0x0046CBE8\", \"len\": 4 } }"
        ));
        assert!(emitted
            .json
            .contains("{ \"id\": \"no-extract-latch\", \"addr\": \"0x0046AED4\", \"len\": \"$robot_count*4\" }"));
        // selection-triple dumps the EXW SELECTED-SLOT cell 0x46cbdc
        // (the EXW list is FIELD-ordered base/selected/size but NOT
        // ascending — the D132 pairing; cells[1], never cells[0])
        assert!(emitted
            .json
            .contains("{ \"id\": \"selection-triple\", \"addr\": \"0x0046CBDC\", \"len\": 4 }"));
        assert!(!emitted.json.contains("0x0046CBD4"));
        // every emitted id is a real registry row of the scenario
        // tiers with a non-empty EXW address (never a fabricated gap);
        // a `#name` suffix strips to its parent row (7j.69/D161)
        for id in row_ids(&emitted.json) {
            let base = id.split('#').next().unwrap_or(&id);
            let row = reg
                .iter()
                .find(|r| r.id == base)
                .unwrap_or_else(|| panic!("plan id {id:?} is not in the registry"));
            assert!(
                scen.tiers.contains(&row.tier),
                "plan id {id:?} tier {} not in scenario tiers",
                row.tier
            );
            assert!(
                !row.exw_addr.is_empty(),
                "plan id {id:?} is an EXW gap — must never be emitted on o2"
            );
        }
        // row-set symmetry with O1 minus the EXD-only row: the same
        // 28 per-frame rows, 44-1 anchor rows (static-cursor-clamp is
        // EXD-only), 1 deferred (the same EXD-only row)
        assert_eq!(count_rows(&emitted.json, "watches"), 28);
        assert_eq!(count_rows(&emitted.json, "anchor_watches"), 43);
        assert_eq!(emitted.deferred.len(), 1);
        assert!(emitted
            .deferred
            .iter()
            .any(|d| d.starts_with("static-cursor-clamp")));
        // stitcher contract unchanged
        assert_eq!(extract_frames(&emitted.json), scen.frames + 1);
    }

    #[test]
    fn s1_o2_plan_matches_committed_artifact() {
        let emitted = emit_plan_channel(&s1(), &registry(), Channel::O2).unwrap();
        let committed = include_str!("../../capture-plans/S1-o2.json");
        assert_eq!(emitted.json, committed, "capture-plans/S1-o2.json is stale: regenerate with dbx-plan scenarios/S1.scen --channel o2 --out capture-plans/S1-o2.json");
    }

    #[test]
    fn o2_refuses_walk_scenarios() {
        // The BPLM stop-indexed menu walk is DOSBox/O1 machinery
        // (D84); the o2 channel never invents capture semantics.
        let src = "scenario = X\ntiers = T0\nframes = 2\nkeystore 0x01=1\nuntil-anchor m\n";
        let scen = Scenario::parse(src).unwrap();
        let err = emit_plan_channel(&scen, &registry(), Channel::O2)
            .err()
            .map(|e| e.to_string())
            .expect("walk scenario must not compile on o2");
        assert!(
            err.contains("menu walk") && err.contains("O1"),
            "error must name the walk gate: {err}"
        );
    }

    #[test]
    fn o2_compiles_inject_steps_on_exw_cells() {
        // Mission-phase steps swap to the EXW seam cells (the W11
        // driver's injection policy reads them as data); the frame
        // accounting is channel-neutral.
        let src = "scenario = X\ntiers = T0\nframes = 4\n\
                    until-anchor mission-start\n\
                    step 2\n\
                    keystore 0x1f=1\n\
                    order 29 18 0\n\
                    command 01\n";
        let scen = Scenario::parse(src).unwrap();
        let emitted = emit_plan_channel(&scen, &registry(), Channel::O2).unwrap();
        // boundaries: anchor=1, step 2 -> 3; keystore @3 (EXW keystore
        // base 0x4edc44 + scan 0x1f), order @4 (EXW triple
        // 0x4dd484/88/8c), command @5 (EXW ring 0x4dd4a0 + count
        // 0x46cbe0)
        assert!(emitted
            .json
            .contains("\"frame\": 3, \"addr\": \"0x004EDC63\", \"bytes\": \"01\""));
        assert!(emitted
            .json
            .contains("\"frame\": 4, \"addr\": \"0x004DD484\", \"bytes\": \"1d000000\""));
        assert!(emitted.json.contains(
            "\"frame\": 5, \"op\": \"command\", \"base\": \"0x004DD4A0\", \
             \"stride\": 128, \"count_cell\": \"0x0046CBE0\", \"bytes\": \"01\""
        ));
    }

    fn s2() -> Scenario {
        Scenario::parse(include_str!("../../scenarios/S2.scen")).unwrap()
    }

    #[test]
    fn s2_plan_compiles_the_order_seam_plus_e_staging() {
        let scen = s2();
        assert_eq!(scen.markers, vec![(18, 73, 1)], "the D91 staging key");
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        // Same tier set as S1 -> the same row shape (27 anchor TS/T0 +
        // 17 T1; 11 T0 + 17 T1 per-frame — gap set empty since D134;
        // TS fully resolved incl. the 7j.69/D161 four + the yline
        // #zbase companion).
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 11 + 17);
        assert_eq!(anchor_count, 27 + 17);
        // The order step's inject rows: frame 1 (the first mission
        // boundary), the three i32-LE cells of the order-target triple
        // (21, 73, 1) at the registry-derived cells.
        let injects = extract_injects(&emitted.json);
        assert_eq!(injects.len(), 3, "one row per order-target cell");
        let expect: [(&str, &str); 3] = [
            ("CS:0010E0A4", "15000000"),
            ("CS:0010E0A8", "49000000"),
            ("CS:0010E0AC", "01000000"),
        ];
        for (addr, bytes) in expect {
            assert!(
                injects
                    .iter()
                    .any(|(f, a, b)| *f == Some(1) && a == addr && b == bytes),
                "missing inject {addr}={bytes} at frame 1 in {injects:?}"
            );
        }
        // The E-side staging seam is RECORDED, never fabricated: no
        // inject row may target the robot bank/count cells, and the
        // _e_staging field names the marker + the seam.
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("{ \"x\": 18, \"y\": 73, \"z\": 1 }"));
        assert!(emitted.json.contains("scenario seam, not a finding"));
        for (_, addr, _) in &injects {
            assert!(
                !addr.ends_with("F6D34") && !addr.ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
        // stitcher contract: frames + 1 records
        let frames: u64 = extract_frames(&emitted.json);
        assert_eq!(frames, scen.frames + 1);
    }

    #[test]
    fn s2_plan_matches_committed_artifact() {
        let emitted = emit_plan(&s2(), &registry()).unwrap();
        let committed = include_str!("../../capture-plans/S2.json");
        assert_eq!(emitted.json, committed, "capture-plans/S2.json is stale: regenerate with dbx-plan scenarios/S2.scen --out capture-plans/S2.json");
    }

    #[test]
    fn loadout_seam_is_recorded_never_fabricated() {
        // D103 (grammar v1.3): a loadout-bearing scenario records the
        // seam in _e_staging — E arms through stage_robot_weapons,
        // the O1 capture arms its robots by PLAYING the session —
        // and never writes the robot weapon cells. Tiers stay
        // T0/T1/TS so the plan compiles (S3 itself carries T2, which
        // dbx-plan refuses until the remaining T2 aliases land).
        let src = "scenario = LX\ntiers = T0,T1,TS\nframes = 4\nmarkers = 18,73,1\n\
                   loadout = 0,0x3,9:2,0x10:4; 1,0x1,0x20:2\n";
        let scen = Scenario::parse(src).unwrap();
        assert_eq!(scen.loadout.len(), 2, "the v1.3 staging key parses");
        let emitted = emit_plan(&scen, &registry()).unwrap();
        assert!(emitted.json.contains("\"loadout\": ["));
        assert!(emitted
            .json
            .contains("{ \"robot\": 0, \"mask\": 3, \"slots\": \"0x9:2, 0x10:4\" }"));
        assert!(emitted
            .json
            .contains("{ \"robot\": 1, \"mask\": 1, \"slots\": \"0x20:2\" }"));
        assert!(emitted
            .json
            .contains("E-side staging seam (D103, grammar v1.3)"));
        // The markers block + its D91 note ride the same object.
        assert!(emitted.json.contains("{ \"x\": 18, \"y\": 73, \"z\": 1 }"));
        assert!(emitted.json.contains("E-side staging seam (D91)"));
        // Never fabricated: no inject row touches the robot bank /
        // count cells (the weapon slots live in the robot records).
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert!(
                !addr.ends_with("F6D34") && !addr.ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
    }

    #[test]
    fn destroy_seam_is_recorded_never_fabricated() {
        // D105 (grammar v1.4): a destroy-bearing scenario records the
        // EQUIVALENCE seam in _e_staging — E stages the mission's own
        // .BDG/.POS/.TRT through stage_destroy_family, the ORIGINAL
        // loads the same files natively, so no O1 write exists to
        // fabricate. The destroy rows' EXD cells must carry no inject
        // row. (Tiers T0/T1/TS keep the plan compilable; a real S4
        // plan needs the T3 tier unit first, like S3's T2.)
        let src = "scenario = DX\ntiers = T0,T1,TS\nframes = 4\nmarkers = 18,73,1\n\
                   destroy = 1\n";
        let scen = Scenario::parse(src).unwrap();
        assert!(scen.destroy, "the v1.4 staging key parses");
        let emitted = emit_plan(&scen, &registry()).unwrap();
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("\"destroy\": true"));
        assert!(emitted.json.contains("E-side EQUIVALENCE seam"));
        assert!(emitted
            .json
            .contains("(D105, grammar v1.4): the mission's own .BDG type table"));
        // Never fabricated: no inject row touches the destroy-family
        // EXD cells — the object bank/count (0x119584/0x119554), the
        // TRT bank/count (0x95264/0x11949c), the mirror rows
        // (0xac1e4), the grids (0xfe37c, 0xf93cc).
        for (_, addr, _) in &extract_injects(&emitted.json) {
            for cell in [
                "119584", "119554", "95264", "11949C", "AC1E4", "FE37C", "F93CC",
            ] {
                assert!(
                    !addr.to_uppercase().ends_with(cell),
                    "ghost staging write to the destroy bank cell {cell}: {addr}"
                );
            }
        }
    }

    #[test]
    fn t2_tiers_compile_the_full_alias_set() {
        // D109: the T2 tier compiles — the two W12-S3 banks
        // (weapon-anim 0x980d4 / projectile 0x10e174, RE-EXD-MAP
        // sec 5c) emit as the FULL fixed spans the differ's O1
        // normalizers require (no count cell on the guest); D162
        // (§5i) added the three census rows — mortar-trail the full
        // 0x830 span, critter/poi the count-driven CountExpr forms
        // over the .NME loader count cells.
        let src = "scenario = X\ntiers = T2\nframes = 1\n";
        let scen = Scenario::parse(src).unwrap();
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        assert!(emitted.json.contains(
            "{ \"id\": \"weapon-anim-bank\", \"addr\": \"CS:000980D4\", \"len\": 21600 }"
        ));
        assert!(emitted
            .json
            .contains("{ \"id\": \"projectile-bank\", \"addr\": \"CS:0010E174\", \"len\": 1700 }"));
        assert!(emitted.json.contains(
            "{ \"id\": \"mortar-trail-bank\", \"addr\": \"CS:00091574\", \"len\": 2080 }"
        ));
        assert!(emitted.json.contains(
            "\"id\": \"critter-bank\", \"addr\": \"CS:0010E81C\", \"len\": \"$critter_count*0x7E\""
        ));
        assert!(emitted.json.contains(
            "\"id\": \"poi-bank\", \"addr\": \"CS:000971D4\", \"len\": \"$poi_count*0x1E\""
        ));
        // the count cells ride the resolve rows (loader statics —
        // read at the anchor like every resolve cell)
        assert!(emitted.json.contains("critter_count"));
        assert!(emitted.json.contains("poi_count"));
        assert_eq!(emitted.deferred.len(), 0);
    }

    #[test]
    fn s3_plan_compiles_the_t2_tier() {
        // D109: S3 (the W12-S3 command-fire scenario) compiles at
        // T0,T1,T2,TS — the first T2-tier plan. T2 adds the two full
        // bank spans per frame; the loadout seam records (D103) with
        // a VALID-JSON decimal mask (the old 0x-form made
        // loadout-bearing plans unparseable).
        let scen = Scenario::parse(include_str!("../../scenarios/S3.scen")).unwrap();
        assert!(scen.tiers.iter().any(|t| t == "T2"));
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        // T0 11 + T1 17 + T2 5 per-frame (all five T2 rows aliased —
        // D162/§5i: weapon, projectile, mortar-trail + the two
        // count-driven .NME rows); anchor adds the 16 TS spans
        // (15 resolved rows + the yline #zbase companion, 7j.69/D161).
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 11 + 17 + 5);
        assert_eq!(anchor_count, frame_count + 16);
        // deferred: EMPTY since D162 (the TS gap set closed at
        // 7j.69/D161, the T2 set at §5i)
        assert_eq!(emitted.deferred.len(), 0);
        // the count-driven census rows carry their resolve symbols
        assert!(emitted.json.contains("\"len\": \"$critter_count*0x7E\""));
        assert!(emitted.json.contains("\"len\": \"$poi_count*0x1E\""));
        // the 8 command volleys compile to inject rows (the S3 frame
        // schedule), and the loadout seam records never-fabricated
        assert_eq!(emitted.inject_count, 8);
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("\"loadout\": ["));
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert!(
                !addr.ends_with("F6D34") && !addr.ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
    }

    #[test]
    fn s3_plan_matches_committed_artifact() {
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S3.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S3.json");
        assert_eq!(emitted.json, committed, "capture-plans/S3.json is stale: regenerate with dbx-plan scenarios/S3.scen --out capture-plans/S3.json");
    }

    #[test]
    fn s4_t3_rows_now_emitted() {
        // D109 refused the T3 tier (no alias yet); D162 (§5i) closed
        // the set — S4 (the W12-S4 destroy scenario, T0,T1,T3,TS)
        // now EMITS all 14 T3 rows with their census-pinned EXD
        // addresses (anti-ghost: a stale address fails the build).
        // The debris-stager/splash-records rows the scenario's E-side
        // records ride are O1-dumpable now; their cross-channel
        // FIELD compare still awaits the differ extraction arms (the
        // named follow-up — the differ treats the raw O1 rows as
        // coverage until then).
        let scen = Scenario::parse(include_str!("../../scenarios/S4.scen")).unwrap();
        assert!(scen.tiers.iter().any(|t| t == "T3"));
        let reg = registry();
        let emitted = emit_plan(&scen, &reg).unwrap();
        // T0 11 + T1 17 + T3 14 per-frame; anchor adds the 16 TS
        // spans (15 resolved rows + the yline #zbase companion,
        // 7j.69/D161).
        assert_eq!(count_rows(&emitted.json, "watches"), 11 + 17 + 14);
        assert_eq!(
            count_rows(&emitted.json, "anchor_watches"),
            11 + 17 + 14 + 16
        );
        // deferred: EMPTY since D162
        assert_eq!(emitted.deferred.len(), 0);
        // the census pins ride the plan verbatim (a representative
        // fixed span, the count-free full banks)
        for (id, addr, len) in [
            ("debris-stager", "CS:00093064", 128 * 0x30),
            ("effect-rows", "CS:0009D534", 80 * 0x20),
            ("splash-records", "CS:00107774", 250 * 0xA),
            ("door-rects", "CS:00092C64", 45 * 0x10),
            ("arrival-rides", "CS:0010DA48", 45 * 0x24),
        ] {
            let want = format!("{{ \"id\": \"{id}\", \"addr\": \"{addr}\", \"len\": {len} }}");
            assert!(
                emitted.json.contains(&want),
                "T3 row {id} must carry its D162 pin {addr}/{len:#x}"
            );
        }
        // the per-frame tile-claims row reads through the pointer
        // cell (the second PtrCell row — its own resolve symbol)
        assert!(emitted.json.contains("CS:$claim3_ptr"));
        // every emitted T3 row exists in the registry (no ghosts)
        for id in row_ids(&emitted.json) {
            let base = id.split('#').next().unwrap_or(&id);
            assert!(
                reg.iter().any(|r| r.id == base),
                "emitted row {id:?} has no registry row"
            );
        }
    }

    #[test]
    fn s4_plan_matches_committed_artifact() {
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S4.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S4.json");
        assert_eq!(emitted.json, committed, "capture-plans/S4.json is stale: regenerate with dbx-plan scenarios/S4.scen --out capture-plans/S4.json");
    }

    #[test]
    fn s5_plan_compiles_the_zone_and_pickup_seams() {
        // D108 (grammar v1.5): S5 compiles (tiers T0/T1/TS — no T2/T3
        // rows ride a pickup walk) with the zone + pickup
        // EQUIVALENCE seams recorded in _e_staging and NO fabricated
        // O1 write: the zone cell (EXD 0x107500) and the mirror rows
        // (0xac1e4) carry no inject row — the original stages both
        // natively at mission load.
        let s5 = Scenario::parse(include_str!("../../scenarios/S5.scen")).unwrap();
        assert_eq!(s5.zone, Some('B'));
        assert!(s5.pickup);
        assert!(s5.destroy);
        assert_eq!(s5.markers, vec![(28, 21, 3), (25, 21, 3)]);
        let emitted = emit_plan(&s5, &registry()).unwrap();
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("\"zone\": \"B\""));
        assert!(emitted.json.contains("\"pickup\": true"));
        assert!(emitted
            .json
            .contains("The LIVE O1 capture reaches this zone by playing the campaign"));
        assert!(emitted
            .json
            .contains("the S4-era empty-mirror divergence is closed"));
        // Never fabricated: no inject row touches the zone cell or
        // the mirror rows.
        for (_, addr, _) in &extract_injects(&emitted.json) {
            for cell in ["107500", "AC1E4"] {
                assert!(
                    !addr.to_uppercase().ends_with(cell),
                    "ghost staging write to the {cell} cell: {addr}"
                );
            }
        }
        // S5B compiles the same shape.
        let s5b = Scenario::parse(include_str!("../../scenarios/S5B.scen")).unwrap();
        assert_eq!(s5b.zone, Some('B'));
        assert!(s5b.pickup);
        assert!(emit_plan(&s5b, &registry()).is_ok());
        // S5C (W12-S5C): the pre-damaged-walker variant — the third
        // marker (the gunner ON the walker's tile) + its loadout seam
        // + the frame-1 command and the frame-37 order both compile;
        // the gunner's loadout records in _e_staging (never an O1
        // write).
        let s5c = Scenario::parse(include_str!("../../scenarios/S5C.scen")).unwrap();
        assert_eq!(s5c.zone, Some('B'));
        assert!(s5c.pickup);
        assert_eq!(
            s5c.markers,
            vec![(78, 10, 3), (73, 10, 3), (73, 10, 3)],
            "clicker + walker + gunner (on the walker's tile)"
        );
        let emitted = emit_plan(&s5c, &registry()).unwrap();
        assert!(emitted.json.contains("\"loadout\": ["));
        // 4 inject rows: the command append at frame 1 + the
        // order-target triple at frame 37 (the S2 shape — no inject
        // row ever touches the robot bank/count).
        assert_eq!(emitted.inject_count, 4);
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert!(
                !addr.to_uppercase().ends_with("F6D34") && !addr.to_uppercase().ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
    }

    #[test]
    fn s5c_plan_matches_committed_artifact() {
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S5C.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S5C.json");
        assert_eq!(emitted.json, committed, "capture-plans/S5C.json is stale: regenerate with dbx-plan scenarios/S5C.scen --out capture-plans/S5C.json");
    }

    #[test]
    fn s6_plan_matches_committed_artifact() {
        // W12-S6 (§7j.40, D112): the pad op (D86) at frame 1 — the
        // bank read from the static-pad-slots registry row, the
        // triple written to the order-target cells, slot 18 = the
        // census ground pad (19,70,0) — + the two COMMAND bit0
        // SELECT records at frames 2/9 (the walk legs; raw Q5 target
        // words 0x0260/0x0920 and 0x0260/0x0860). No staging seam
        // rows: the run banks the MRK squad only (no markers) and
        // the zone cell is the live game's own staging on O1 (an
        // equivalence, never fabricated).
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S6.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S6.json");
        assert_eq!(emitted.json, committed, "capture-plans/S6.json is stale: regenerate with dbx-plan scenarios/S6.scen --out capture-plans/S6.json");
        // The three injects, in frame order: the pad op (frame 1 —
        // the bank read anchor + slot 18 + the order-target triple)
        // then the two command records (frames 2/9, the ring append
        // at CS:0009255C, the leg targets 0x0260/0x0920 and
        // 0x0260/0x0860 in the record payload). No row ever touches
        // the robot bank/count.
        assert_eq!(emitted.inject_count, 3);
        assert!(
            committed.contains("\"op\": \"pad\"")
                && committed.contains("\"bank\": \"CS:0000F63C\"")
                && committed.contains("\"slot\": 18"),
            "the pad op row"
        );
        assert!(committed.contains("60022009000000"), "leg-1 record bytes");
        assert!(committed.contains("60026008000000"), "leg-2 record bytes");
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert_eq!(addr, "CS:0009255C", "the command ring append");
            assert!(
                !addr.to_uppercase().ends_with("F6D34") && !addr.to_uppercase().ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
    }

    #[test]
    fn s7_plan_compiles_the_platform_seams() {
        // W12-S7 (§7j.41, D113): grammar v1.6 `platforms = 1` arms the
        // epilogue creep tick on E — an ARM key, recorded in
        // _e_staging as the RNG-stream equivalence (the ORIGINAL
        // draws one gate RandA per frame unconditionally). The
        // platform banks and the tick writers need NO inject rows:
        // the run's own fire (5 COMMAND records: the frame-1
        // artillery + the four grenade volleys) produces every state
        // change; the destroy/pickup/loadout seams record as on
        // S4/S5/S3.
        let s7 = Scenario::parse(include_str!("../../scenarios/S7.scen")).unwrap();
        assert!(s7.platforms);
        assert!(s7.destroy);
        assert!(s7.pickup);
        assert_eq!(
            s7.markers,
            vec![(3, 57, 2)],
            "the gunner ON the trigger tile"
        );
        let emitted = emit_plan(&s7, &registry()).unwrap();
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("\"platforms\": true"));
        assert!(emitted
            .json
            .contains("one RandA gate-draw per frame consumed even with no platform staged"));
        assert!(emitted.json.contains("\"destroy\": true"));
        assert!(emitted.json.contains("\"pickup\": true"));
        assert!(emitted.json.contains("\"loadout\": ["));
        // 5 inject rows: the frame-1 artillery command + the four
        // grenade volleys (f18/f22/f26/f30) — all ring appends, never
        // a staging write to the robot bank/count or a platform bank.
        assert_eq!(emitted.inject_count, 5);
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert_eq!(addr, "CS:0009255C", "the command ring append");
            assert!(
                !addr.to_uppercase().ends_with("F6D34") && !addr.to_uppercase().ends_with("11958C"),
                "ghost staging write to the robot bank/count: {addr}"
            );
        }
    }

    #[test]
    fn s7_plan_matches_committed_artifact() {
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S7.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S7.json");
        assert_eq!(emitted.json, committed, "capture-plans/S7.json is stale: regenerate with dbx-plan scenarios/S7.scen --out capture-plans/S7.json");
    }

    #[test]
    fn s8_plan_compiles_the_critter_seam() {
        // W12-S8 (§7j.42, D114): grammar v1.7 `critters = 1` stages
        // the .NME + arms the controller on E — the staging+arm key,
        // recorded in _e_staging as the RNG-stream equivalence (the
        // ORIGINAL's loader heading draws + per-frame controller
        // draws are consumed on O1 on every scenario). No inject
        // rows beyond the run's own fire: ONE command record (the
        // frame-1 artillery burst that produces the deaths); since
        // D162 (§5i) the critter bank + effect rows carry EXD aliases
        // and emit (critter-bank count-driven, effect-rows the full
        // span), and the 0x68 fire rides the ALIASED projectile bank.
        let s8 = Scenario::parse(include_str!("../../scenarios/S8.scen")).unwrap();
        assert!(s8.critters);
        assert!(!s8.destroy && !s8.pickup && !s8.platforms);
        assert_eq!(s8.markers, vec![(18, 13, 1)], "the gunner on the flat row");
        let emitted = emit_plan(&s8, &registry()).unwrap();
        assert!(emitted.json.contains("\"_e_staging\": {"));
        assert!(emitted.json.contains("\"critters\": true"));
        assert!(emitted
            .json
            .contains("the loader's kind-4 heading draws + the controller's per-frame draws"));
        // ONE inject row: the frame-1 artillery command — the ring
        // append, never a staging write.
        assert_eq!(emitted.inject_count, 1);
        for (_, addr, _) in &extract_injects(&emitted.json) {
            assert_eq!(addr, "CS:0009255C", "the command ring append");
        }
        // D162 (§5i): the critter bank + the effect rows are ALIASED
        // and emitted now (critter count-driven over the loader count
        // cell; effect rows the full 0xA00 span) — the cross-channel
        // FIELD compare still awaits the differ extraction arms (the
        // named follow-up).
        assert!(emitted.json.contains("\"id\": \"critter-bank\""));
        assert!(emitted.json.contains("\"len\": \"$critter_count*0x7E\""));
        assert!(emitted.json.contains("\"id\": \"effect-rows\""));
    }

    #[test]
    fn s8_plan_matches_committed_artifact() {
        let emitted = emit_plan(
            &Scenario::parse(include_str!("../../scenarios/S8.scen")).unwrap(),
            &registry(),
        )
        .unwrap();
        let committed = include_str!("../../capture-plans/S8.json");
        assert_eq!(emitted.json, committed, "capture-plans/S8.json is stale: regenerate with dbx-plan scenarios/S8.scen --out capture-plans/S8.json");
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
        assert!(!emitted.json.contains("\"boot_trap\": \"entry\""));
        assert!(
            emitted.json.contains("\"cmd\": \"BPLM 1195F0\""),
            "walk plans retain their BPLM-driven stop schedule"
        );
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

    /// (frame, addr, bytes) rows of the "inject" section. Step-less
    /// plans carry no inject section at all — empty, not a panic.
    fn extract_injects(json: &str) -> Vec<(Option<u64>, String, String)> {
        let Some(section) = json.split("\"inject\": [").nth(1) else {
            return Vec::new();
        };
        let section = section.split("]").next().unwrap();
        let mut out = Vec::new();
        for line in section.lines() {
            let line = line
                .trim()
                .trim_start_matches('{')
                .trim_end_matches(',')
                .trim_end_matches('}');
            let mut frame = None;
            let mut addr = String::new();
            let mut bytes = String::new();
            for part in line.split(", ") {
                let part = part.trim();
                if let Some(rest) = part.strip_prefix("\"frame\": ") {
                    frame = Some(rest.parse().unwrap());
                } else if let Some(rest) = part.strip_prefix("\"addr\": ") {
                    addr = rest.trim_matches('"').to_string();
                } else if let Some(rest) = part.strip_prefix("\"bytes\": ") {
                    bytes = rest.trim_matches('"').to_string();
                }
            }
            if !addr.is_empty() {
                out.push((frame, addr, bytes));
            }
        }
        out
    }
}
