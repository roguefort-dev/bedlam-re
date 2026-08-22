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
use diffharness::runner::Scenario;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

/// The tiers this compiler can resolve today (the S0 shape).
const SUPPORTED_TIERS: [&str; 2] = ["T0", "TS"];

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
}

fn emit_plan(scen: &Scenario, reg: &[diffharness::Watch]) -> Result<Emitted, PlanError> {
    // Tier gate: only the S0 shape today.
    for t in &scen.tiers {
        if !SUPPORTED_TIERS.contains(&t.as_str()) {
            return Err(die(format!(
                "scenario {} tier {t:?} is not compilable yet: T1+ extents need \
                 the W5 count-cell resolver (dbx-plan supports {:?})",
                scen.id, SUPPORTED_TIERS
            )));
        }
    }

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
                other => {
                    return Err(die(format!(
                        "PtrCell row {other:?} has no resolve symbol in dbx-plan"
                    )))
                }
            };
            resolve.push((name.into(), cell));
        }
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
                    _ => unreachable!("checked above"),
                };
                format!(
                    "    {{ \"id\": {}, \"addr\": \"CS:${sym}\", \"len\": {} }}",
                    jstr(&p.id),
                    jstr(len_expr)
                )
            }
        }
    };

    let mut j = String::new();
    j.push_str("{\n");
    j.push_str(&format!(
        "  \"_comment\": \"{} live capture plan (D81; GENERATED by dbx-plan from watches.toml - do not hand-edit, regenerate). Boot trap: BPLM {fc_cell:X} (the frame-counter cell) armed at the parked pre-boot halt fires on the first post-boot write. Arm stop: SELINFO CS flat guard (base==0), then BP CS:{tail:08X} = the registry s0-trigger row (the BP ack echoes the numeric selector - the per-run pin). Anchor frame = the first BP hit = mission frame 2 tail (frame 1 passed before the trap fired; alignment is by the frame-counter watch). TS statics ride the anchor frame; T0 rows every frame. Deferred TS rows carry unpinned extents (see _deferred). INTERACTIVE: the operator walks the title menu on the desktop; the anchor frame-counter and RNG bytes are menu-timing dependent across runs (T2/T3 classes, DESIGN section 6) - the live double-run verdict is identical-chains-modulo-those-cells; byte-identical chains need the W5 scripted walk.\",\n",
        scen.id
    ));
    j.push_str("  \"logfile\": \"dosbox-harness.log\",\n");
    j.push_str("  \"time_limit\": 1800,\n");
    j.push_str("  \"boot_timeout\": 1800,\n");
    j.push_str("  \"boot_retries\": 24,\n");
    j.push_str(&format!("  \"frames\": {},\n", scen.frames + 1));
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
        "dbx-plan: scenario {} -> {} anchor rows + {} per-frame rows, {} deferred; \
         frames={} (anchor + {} post-anchor records for the stitcher)",
        scen.id,
        emitted.anchor_count,
        emitted.frame_count,
        emitted.deferred.len(),
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
        // T0: 11 rows, 2 gaps (difficulty, sfx-master-gate) -> 9 per-frame.
        // TS: 15 rows, 6 deferred -> 9 anchor-only + 0... but T0 rides the
        // anchor too: 9 + 9 = 18 anchor rows.
        let anchor_count = count_rows(&emitted.json, "anchor_watches");
        let frame_count = count_rows(&emitted.json, "watches");
        assert_eq!(frame_count, 9, "T0 rows minus the 2 gaps");
        assert_eq!(anchor_count, 18, "T0 + resolved TS rows");
        // 6 TS extent gaps + the 2 explicit T0 EXD gaps
        assert_eq!(emitted.deferred.len(), 8);
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
    fn t1_scenario_refused() {
        let src = include_str!("../../scenarios/S1.scen");
        let scen = Scenario::parse(src).unwrap();
        assert!(emit_plan(&scen, &registry()).is_err());
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
