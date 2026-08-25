//! S0-14 strict-coverage oracle for the two DYNAMIC-ONLY registry
//! rows: `s0-trigger` (tier S0 — the dump point itself) and
//! `frame-counter` (T0 — the timing cell) — RE-EXW-SIM §7j.66, D156.
//!
//! These rows close under the DYNAMIC-ONLY PLACEMENT disposition,
//! separately from static closure (D156): the trigger row has extent 0
//! (a breakpoint, no comparable bytes — its coverage IS the ordering
//! pin plus the capture machinery arming it), and the counter row is
//! the T2 timing cell deliberately never bit-compared. Two halves, the
//! static-oracle convention (S0-07..S0-13 pattern):
//!
//! 1. ORIGINAL-SIDE TRANSCRIPTION (corpus-free): the §7j.66 decode of
//!    `ghidra-project/exw-text-objdump.txt` — the MissionShell tail
//!    ordering (normal-path PresentEnd call 0x4486c9, pause path
//!    0x44861f, register-form counter increment 0x4486ce-da ALWAYS
//!    after the flip), the PresentEnd call-site census (62 direct
//!    sites — a BP at the function entry 0x425a03 fires on every
//!    menu/loading/cinematic present, so it is NOT the trigger), and
//!    the writer census WITH the eight counter RESETS (the D81
//!    correction: the bounded cinematic screens reuse the counter as
//!    their 100/200/300-frame duration timer). The tail is
//!    transcribed as a state machine and the dump-point value model
//!    `O1(k) = C0 + (k-1)` derived from it.
//! 2. E-SIDE + DIFFER CLASSIFICATION (corpus-gated / synthetic): E's
//!    canonical `frame-counter` is the mission-relative PRE-increment
//!    value (== the record `frame_no`, strictly increasing — E never
//!    carries the menu offset), and the transcribed O1 model
//!    (counter = C0 + frame_no) vs E compares through the differ as
//!    `T2Reported` — report-only, never a failing finding, no
//!    alignment shift — exactly the machinery these rows close under.
//!
//! This test lives in bedlam-game because the E half is the canonical
//! harness (`parity_harness/canonical.rs`, re-exported the
//! canonical_dump_gate way).

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::{Path, PathBuf};

use canonical::run_canonical;
use diffharness::differ::{report_text, run_diff, Class, DiffConfig, Mode, Verdict};
use diffharness::dump::{decode_dump, Channel, DumpHeader, FrameRecord, WatchRecord};
use diffharness::hash::sha256;
use diffharness::registry;
use diffharness::runner::{stitch, Scenario, Transcript};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn scen_path(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/diffharness/scenarios")
        .join(format!("{id}.scen"))
}

fn corpus_present() -> bool {
    root().join("EDITOR/ZONEA/MISSION1.TOT").is_file()
}

// ---------------------------------------------------------------------
// 1. Original-side transcription (§7j.66) — the coverage half
// ---------------------------------------------------------------------

/// §7j.66/A — the MissionShell tail sites. The dump point is the
/// NORMAL-PATH PresentEnd CALL, not the function entry.
const PRESENT_ENTRY: u32 = 0x425a03;
/// Direct `e8` call sites of PresentEnd in .text (§7j.66/B) — the
/// census that makes a function-entry BP unusable as the trigger.
const PRESENT_CALL_SITES: usize = 62;
/// THE EXW S0 dump point: the normal-path call (BP before the call
/// executes = after the last state writer, before the flip). The EXD
/// twin is 0x5a6eb (RE-EXD-MAP §2).
const TAIL_NORMAL_PRESENT: u32 = 0x4486c9;
/// The pause-path flip (the SP PAUSED redraw; scenarios never take it
/// — the P key 0x19 is banned). The pause path jumps PAST the normal
/// call straight to the counter increment.
const TAIL_PAUSE_PRESENT: u32 = 0x44861f;
/// The counter increment, register form (read/inc/write-back; the
/// unrelated [0x4dc67c] read rides the middle at 0x4486d5). ALWAYS
/// after the flip, both paths.
const TAIL_INC_READ: u32 = 0x4486ce;
const TAIL_INC_WRITE: u32 = 0x4486da;
/// The EXD mission-tail twin (CALL FUN_00010670 then the register-form
/// increment 0x5a6f0-0x5a6fd) — order identical (RE-EXD-MAP §2).
const EXD_TRIGGER: u32 = 0x5a6eb;
const EXD_INC: u32 = 0x5a6f0;

/// §7j.66/C — the eight BOUNDED CINEMATIC SCREEN loops: each RESETS
/// the counter to 0 (`xor reg; mov [0x46ae68],reg`) at entry, then
/// counts present→inc to its duration bound, exiting with counter ==
/// bound. `(reset write, cmp site, bound frames, inc site)`. This is
/// the D81 correction: an INC-only census misses these mov stores
/// (the same trap the W1 EXD census documented).
const CINEMATIC_RESETS: [(u32, u32, u32, u32); 8] = [
    (0x44466f, 0x444675, 200, 0x44469b),
    (0x4446e4, 0x4446ea, 100, 0x44470d),
    (0x4449f9, 0x4449ff, 300, 0x444a3a),
    (0x444c4b, 0x444c51, 200, 0x444c77),
    (0x444f87, 0x444f8d, 100, 0x444fb0),
    (0x445167, 0x44516d, 100, 0x445190),
    (0x44526c, 0x445278, 300, 0x4452a2),
    (0x4453b7, 0x4453bd, 200, 0x4453e7),
];

/// §7j.66/C — the five INTERACTIVE menu screens: NO reset, cumulative
/// counting, and the in-loop order there is inc-THEN-present (the
/// opposite of the mission tail and the cinematic loops).
const MENU_CUMULATIVE_INC: [u32; 5] = [0x43afa0, 0x43d4f7, 0x43d53f, 0x43da5a, 0x43f31f];

/// §7j.66/D — the MissionShell HEAD (loading-screen presents) contains
/// NO counter writer; nothing between the last menu screen and the
/// loop's first increment touches the cell.
const MISSION_HEAD_HAS_COUNTER_WRITER: bool = false;

/// The transcribed tail state machine (§7j.66/A): one MissionShell
/// loop pass. Returns `(dumped_value, bp_fires, next_counter)`.
///
/// - The dump BP sits at the call instruction and fires BEFORE it
///   executes, so the dumped value is the PRE-increment count.
/// - Exactly ONE present per pass (normal 0x4486c9, or the pause
///   redraw 0x44861f + the P spin, which then jumps past the normal
///   call).
/// - Exactly ONE counter increment per pass, after the flip.
/// - The trigger BP (at the normal-path call) does NOT fire on pause
///   passes — scenarios never pause (P banned), so every scripted
///   pass fires.
fn tail_pass(counter: u32, normal_path: bool) -> (u32, bool, u32) {
    let dumped = counter; // BP pre-call: the PRE-increment value
    let next = counter.wrapping_add(1); // 0x4486ce → 0x4486da
    (dumped, normal_path, next)
}

/// The dump-point value model (§7j.66/D): dump k (1-based) carries
/// `C0 + (k-1)` where C0 = the counter at mission-loop entry — a
/// deterministic function of the scripted menu walk, never reset by
/// the mission itself.
fn o1_dump_value(c0: u32, k: u32) -> u32 {
    // The tail model applied k times from c0; the k-th dump reads the
    // counter BEFORE the k-th increment.
    let mut counter = c0;
    for _ in 1..k {
        counter = tail_pass(counter, true).2;
    }
    tail_pass(counter, true).0
}

/// A pre-mission screen walk (§7j.66/C): cinematic screens reset to 0
/// then count to their bound (exit value == bound); interactive menu
/// screens count frames cumulatively. C0 = what the LAST screen left.
#[derive(Clone, Copy)]
enum Leg {
    /// A bounded cinematic screen of the given duration bound
    /// (one of the eight 100/200/300 constants).
    Cinematic(u32),
    /// An interactive menu screen held `frames` frames.
    Menu(u32),
}

fn walk_c0(walk: &[Leg]) -> u32 {
    let mut counter = 0u32; // whatever the walk's first reset establishes
    for leg in walk {
        counter = match leg {
            Leg::Cinematic(bound) => *bound, // reset to 0, count to bound, exit == bound
            Leg::Menu(frames) => counter.wrapping_add(*frames), // cumulative, no reset
        };
    }
    counter
}

#[test]
fn tail_ordering_dumps_preincrement_one_inc_per_pass() {
    // The ordering contract both twins share: the dump reads
    // PRE-increment, strictly increasing by exactly 1 per pass.
    let c0 = walk_c0(&[Leg::Menu(12)]);
    for k in 1..=8u32 {
        assert_eq!(o1_dump_value(c0, k), c0 + (k - 1), "dump {k}");
    }
    // Determinism: the same script (same C0) replays identically —
    // the DH-G1 double-run premise.
    let a: Vec<u32> = (1..=8).map(|k| o1_dump_value(c0, k)).collect();
    let b: Vec<u32> = (1..=8).map(|k| o1_dump_value(c0, k)).collect();
    assert_eq!(a, b);
    // The pause pass still presents + increments exactly once, but
    // the trigger BP does not fire there (the pause path jumps past
    // the normal call 0x44861f → 0x4486ce).
    let (dumped, bp, next) = tail_pass(500, false);
    assert_eq!(dumped, 500);
    assert!(!bp, "the pause pass must not fire the dump BP");
    assert_eq!(next, 501, "exactly one increment on the pause pass too");
    // E's emission is the mission-relative pre-increment value: the
    // O1 value is E's (k-1) shifted by the walk constant C0.
    for k in 1..=8u32 {
        assert_eq!(o1_dump_value(c0, k), (k - 1) + c0);
    }
}

#[test]
fn d81_correction_the_counter_is_walk_determined_not_a_boot_total() {
    // The eight cinematic resets make C0 a function of the LAST
    // screens of the walk — NOT a boot-frame total. Same boot, same
    // total duration, different screen order → different C0 (a
    // no-reset model could not produce this).
    let a = walk_c0(&[Leg::Menu(112), Leg::Cinematic(300)]);
    let b = walk_c0(&[Leg::Cinematic(300), Leg::Menu(112)]);
    assert_eq!(a, 300, "the trailing cinematic RESETS away the menu count");
    assert_eq!(b, 300 + 112);
    // A no-reset (boot-total) model would give 412 for BOTH; the asm
    // resets make them differ.
    assert_ne!(a, b);
    // Every cinematic exit value equals its bound (the loop's jge
    // exits at counter == bound): the durations are exactly the
    // census constants.
    for &(reset, _cmp, bound, _inc) in &CINEMATIC_RESETS {
        assert_eq!(walk_c0(&[Leg::Cinematic(bound)]), bound, "reset {reset:#x}");
        assert!(matches!(bound, 100 | 200 | 300), "the pinned bounds");
    }
}

#[test]
fn census_counts_pin_the_asm() {
    // 13 INC-form sites + the 1 register-form mission tail = the 14
    // increment sites of the D81 census (that count stands); the
    // eight mov-form RESETS are what it missed.
    let inc_form_sites = MENU_CUMULATIVE_INC.len() + CINEMATIC_RESETS.len();
    assert_eq!(inc_form_sites, 13, "13 INC-form sites");
    assert_eq!(
        inc_form_sites + 1,
        14,
        "+ the register-form mission tail = 14"
    );
    // Site sanity: every pinned site lives in .text (0x401000..0x460000)
    // and the ordering constants are the pinned ones.
    for &s in MENU_CUMULATIVE_INC.iter() {
        assert!((0x401000..0x460000).contains(&s));
    }
    for &(reset, cmp, _b, inc) in &CINEMATIC_RESETS {
        assert!((0x401000..0x460000).contains(&reset));
        assert!(cmp > reset && cmp - reset < 0x10, "cmp rides the reset");
        assert!(inc > cmp, "the inc loop body follows");
    }
    assert_eq!(PRESENT_CALL_SITES, 62);
    assert_eq!(PRESENT_ENTRY, 0x425a03);
    assert_eq!(TAIL_NORMAL_PRESENT, 0x4486c9);
    assert_eq!(TAIL_PAUSE_PRESENT, 0x44861f);
    assert_eq!(TAIL_INC_READ, 0x4486ce);
    assert_eq!(TAIL_INC_WRITE, 0x4486da);
    assert_eq!(EXD_TRIGGER, 0x5a6eb);
    assert_eq!(EXD_INC, 0x5a6f0);
    const _: () = {
        // The ordering invariant as a compile-time pin: the increment
        // site is strictly AFTER the present call site, and the
        // MissionShell head has no counter writer (§7j.66/D).
        assert!(TAIL_INC_READ > TAIL_NORMAL_PRESENT, "inc AFTER the flip");
        assert!(!MISSION_HEAD_HAS_COUNTER_WRITER);
    };
}

// ---------------------------------------------------------------------
// 2. E-side + differ classification — the dynamic-only placement half
// ---------------------------------------------------------------------

fn frame(no: u64, watches: Vec<WatchRecord>) -> FrameRecord {
    let mut f = FrameRecord::new(no, false);
    f.watches = watches;
    f
}

fn dump_bytes(scenario_src: &str, frames: Vec<FrameRecord>, channel: Channel) -> Vec<u8> {
    let scen = Scenario::parse(scenario_src).expect("scenario parses");
    let header = DumpHeader::new(channel, sha256(b"s0-14-test"), scen.id.clone());
    stitch(&scen, &Transcript { frames }, &header, &registry())
        .expect("transcript stitches")
        .bytes
}

const SCEN_T0: &str = "scenario = \"FC14\"\ntiers = T0\nframes = 2\n";

/// E-side T0 frame: the counter is the mission-relative pre-increment
/// value == the record index (canonical.rs emits `sim.frame()-1`).
fn e_t0(no: u64) -> FrameRecord {
    frame(
        no,
        vec![
            WatchRecord::new("frame-counter", (no as u32).to_le_bytes().to_vec()),
            WatchRecord::new("score", 500u32.to_le_bytes().to_vec()),
        ],
    )
}

/// O1-side T0 frame built by the TRANSCRIBED model: counter = C0 + k
/// where k-1 is the record index (the menu-walk constant rides the
/// cell; everything else mirrors E).
fn o1_t0(no: u64, c0: u32) -> FrameRecord {
    let k = no as u32 + 1; // 1-based dump index
    frame(
        no,
        vec![
            WatchRecord::new("frame-counter", o1_dump_value(c0, k).to_le_bytes().to_vec()),
            WatchRecord::new("score", 500u32.to_le_bytes().to_vec()),
        ],
    )
}

#[test]
fn transcribed_o1_model_vs_e_is_t2_reported_never_a_finding() {
    // The dynamic-only placement contract: the counter row compares
    // report-only. C0 = 412 (a plausible scripted-walk leftover: the
    // trailing 300-bound cinematic then 112 interactive frames), far
    // beyond the 0x20 T2 quantum.
    let c0 = walk_c0(&[Leg::Cinematic(300), Leg::Menu(112)]);
    assert_eq!(c0, 412);
    let e = dump_bytes(SCEN_T0, vec![e_t0(0), e_t0(1), e_t0(2)], Channel::Engine);
    let o1 = dump_bytes(
        SCEN_T0,
        vec![o1_t0(0, c0), o1_t0(1, c0), o1_t0(2, c0)],
        Channel::O1ExdDosboxX,
    );
    let res = run_diff(
        &e,
        &o1,
        None,
        &DiffConfig::new(Mode::CrossChannel),
        &registry(),
    )
    .unwrap();
    assert_eq!(res.verdict, Verdict::PassWithNotes, "{}", report_text(&res));
    assert_eq!(res.count(Class::EngineBug), 0);
    assert_eq!(res.count(Class::Structural), 0);
    assert_eq!(res.count(Class::Coverage), 0);
    // The +C0 delta surfaces exactly as the budgeted T2 report — the
    // row is carried by both channels, so no coverage finding either.
    assert!(
        res.findings
            .iter()
            .any(|f| f.row == "frame-counter" && f.class == Class::T2Reported),
        "the C0 delta must be the T2 report:\n{}",
        report_text(&res)
    );
    // Nothing else reports: score compares clean, no alignment shift
    // (frame_no aligns 1:1 — E and the model share the record index).
    assert!(
        res.findings.iter().all(|f| f.row == "frame-counter"),
        "{}",
        report_text(&res)
    );
}

#[test]
fn double_run_same_script_is_counter_identical() {
    // DH-G1 premise: the same scripted walk leaves the same C0, so
    // the O1 double-run compares the counter byte-exact (the T2 class
    // exists for the E comparison, not for determinism doubts).
    let c0 = 412u32;
    let a = dump_bytes(
        SCEN_T0,
        vec![o1_t0(0, c0), o1_t0(1, c0), o1_t0(2, c0)],
        Channel::O1ExdDosboxX,
    );
    let b = dump_bytes(
        SCEN_T0,
        vec![o1_t0(0, c0), o1_t0(1, c0), o1_t0(2, c0)],
        Channel::O1ExdDosboxX,
    );
    let res = run_diff(&a, &b, None, &DiffConfig::new(Mode::DoubleRun), &registry()).unwrap();
    assert_eq!(res.verdict, Verdict::Pass, "{}", report_text(&res));
    assert!(res.findings.is_empty(), "{}", report_text(&res));
}

#[test]
fn e_canonical_counter_is_mission_relative_preincrement() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    // The E half on the REAL canonical S0 run: the frame-counter
    // watch equals the record frame_no on every frame (mission-
    // relative pre-increment — E never carries the menu constant),
    // 4 bytes, strictly increasing from 0. This is the row's E-side
    // classification: presence + form + the seam fact, pointedly NOT
    // a bit comparison against any original counter value.
    let src = fs::read_to_string(scen_path("S0")).unwrap();
    let run = run_canonical(&src, &root()).unwrap();
    let dump = decode_dump(&run.bytes).unwrap();
    assert!(!dump.frames.is_empty());
    let mut prev: Option<u64> = None;
    for f in &dump.frames {
        let w = f
            .watches
            .iter()
            .find(|w| w.id == "frame-counter")
            .expect("S0 is a T0 scenario — the counter rides every record");
        assert_eq!(w.bytes.len(), 4, "u32 g_frame_count");
        let v = u32::from_le_bytes(w.bytes[..].try_into().unwrap());
        assert_eq!(
            v as u64, f.frame_no,
            "E emits sim.frame()-1 == the record index (pre-increment)"
        );
        if let Some(p) = prev {
            assert!(f.frame_no > p, "strictly increasing frame_no");
        }
        prev = Some(f.frame_no);
    }
    assert_eq!(prev, Some(dump.trailer.frame_count - 1));
    // The E-side records never carry a menu offset: the first record
    // is exactly 0 (the anchor is the FIRST mission tick's tail).
    assert_eq!(
        dump.frames[0]
            .watches
            .iter()
            .find(|w| w.id == "frame-counter")
            .map(|w| u32::from_le_bytes(w.bytes[..].try_into().unwrap())),
        Some(0)
    );
    // Path check for the helper (never panics on the skip path).
    assert!(Path::new(&scen_path("S0")).exists());
}
