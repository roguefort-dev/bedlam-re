//! W7 differ gate (DESIGN-DIFFHARNESS.md §6/§6a, D87) — the differ
//! verified against the REAL engine dumps.
//!
//! Corpus-gated (skips without game-data). Uses the W6 canonical
//! emitter (`run_canonical`) on S0/S1 — the dumps whose chains are
//! PINNED in canonical_dump_gate.rs — then fabricates the O1-side
//! transcripts through the INVERSE of the O1 normalizer (test-only;
//! any normalizer map change breaks this coupling loudly) and runs
//! the differ:
//!
//! 1. CROSS-CHANNEL (E vs fabricated O1): verdict PASS-WITH-NOTES
//!    with exactly the expected coverage findings (blink-cursor +
//!    move-target-words rows are E-only — the move-target span is
//!    spliced into the O1 robot-bank row, so its E row has no raw
//!    counterpart — and ZERO robot field gaps: the D90 splice sources
//!    the target trio) + the one T2 frame-counter note (the
//!    O1 counter carries menu frames — never matches E by construction).
//!    No engine-bug/structural findings: the mapped-field contract
//!    holds on the real corpus.
//! 2. DOUBLE-RUN (fabricated O1 vs perturbed copy): the DH-G1 verdict
//!    shape — PASS with counter/RNG divergence budgeted, FAIL on any
//!    other byte diff.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::PathBuf;

use canonical::run_canonical;
use diffharness::differ::{report_text, run_diff, Class, DiffConfig, Mode, Verdict};
use diffharness::dump::{decode_dump, Channel, DumpHeader, FrameRecord};
use diffharness::hash::sha256;
use diffharness::registry;
use diffharness::runner::stitch;

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
// The inverse normalizer (canonical -> EXD raw; test-only)
// ---------------------------------------------------------------------

/// Robot canonical record (94 B) -> EXD 0xA8 record with the 31
/// RE-EXD-MAP §8-mapped leaf fields placed (everything else zero;
/// the canonical target trio is record-external — §5 move-target
/// arrays — so it is never placed).
fn inv_robot_bank(canon: &[u8]) -> Vec<u8> {
    assert!(canon.len() >= 4);
    let n = u32::from_le_bytes(canon[0..4].try_into().unwrap()) as usize;
    assert_eq!(canon.len(), 4 + n * 94);
    let mut out = Vec::with_capacity(n * 0xA8);
    for i in 0..n {
        let rec = &canon[4 + i * 94..4 + (i + 1) * 94];
        // §6a order: alive@0, pos_x@1, pos_y@5, z@9, state@13,
        // dir_byte@15, facing@17, anim@19, variant@21, probe_z@23(16B),
        // stop_dist@39, target@43(9B), drop@52, hp@56, armor@60,
        // hit_flash@62, alarm@64, kind@66, shield@68, charges@72,
        // boost@76, battery@80, pool@84, ctr@88, death@92.
        let mut r = vec![0u8; 0xA8];
        let u16at = |p: usize| u16::from_le_bytes(rec[p..p + 2].try_into().unwrap());
        let i32at = |p: usize| i32::from_le_bytes(rec[p..p + 4].try_into().unwrap());
        r[0x00..0x04].copy_from_slice(&i32at(1).to_le_bytes());
        r[0x04..0x08].copy_from_slice(&i32at(5).to_le_bytes());
        r[0x08..0x0C].copy_from_slice(&i32at(9).to_le_bytes());
        r[0x0C..0x0E].copy_from_slice(&u16at(13).to_le_bytes());
        r[0x0E..0x10].copy_from_slice(&u16at(15).to_le_bytes());
        r[0x10..0x12].copy_from_slice(&u16at(17).to_le_bytes());
        r[0x12..0x14].copy_from_slice(&u16at(19).to_le_bytes());
        r[0x18..0x1A].copy_from_slice(&u16at(21).to_le_bytes());
        for k in 0..8 {
            r[0x1A + 2 * k..0x1C + 2 * k].copy_from_slice(&u16at(23 + 2 * k).to_le_bytes());
        }
        r[0x2A..0x2C].copy_from_slice(&u16at(66).to_le_bytes());
        r[0x2E..0x30].copy_from_slice(&u16at(62).to_le_bytes());
        r[0x30..0x32]
            .copy_from_slice(&i16::from_le_bytes(rec[60..62].try_into().unwrap()).to_le_bytes());
        r[0x34..0x36].copy_from_slice(&u16at(64).to_le_bytes());
        r[0x74..0x78].copy_from_slice(&i32at(39).to_le_bytes());
        r[0x78..0x7C].copy_from_slice(&i32at(56).to_le_bytes());
        let alive = rec[0];
        r[0x7C..0x80].copy_from_slice(&(alive as i32).to_le_bytes());
        r[0x80..0x84].copy_from_slice(&i32at(52).to_le_bytes()); // D88: +0x80
        r[0x88..0x8C].copy_from_slice(&i32at(68).to_le_bytes());
        r[0x8C..0x90].copy_from_slice(&i32at(72).to_le_bytes());
        r[0x94..0x98].copy_from_slice(&i32at(80).to_le_bytes());
        r[0x98..0x9C].copy_from_slice(&i32at(84).to_le_bytes());
        r[0x9C..0x9E].copy_from_slice(&u16at(92).to_le_bytes());
        r[0xA0..0xA4].copy_from_slice(&i32at(76).to_le_bytes());
        r[0xA4..0xA8].copy_from_slice(&i32at(88).to_le_bytes());
        out.extend(r);
    }
    out
}

/// Fabricate the O1-side frame for one E canonical frame.
/// `menu_frames` offsets the frame-counter (the never-resetting O1
/// counter carries the menu walk); `rng_wander` perturbs the RNG words.
fn inv_frame(
    e: &FrameRecord,
    map_wh: Option<(u32, u32)>,
    menu_frames: u32,
    rng_wander: u32,
) -> FrameRecord {
    let mut f = FrameRecord::new(e.frame_no, e.injection_applied);
    for w in &e.watches {
        let id = w.id.as_str();
        let bytes: Vec<u8> = match id {
            // E-only rows: the O1 plan cannot carry them (blink-cursor
            // is a registry gap; move-target-words is CONSUMED into the
            // robot-bank splice on the O1 side, so the E row stays
            // E-only at the row level).
            "blink-cursor" => continue,
            "rng-state-a" | "rng-state-b" => {
                let v = u64::from_le_bytes(w.bytes[..8].try_into().unwrap()) as u32;
                v.wrapping_add(rng_wander).to_le_bytes().to_vec()
            }
            "frame-counter" => {
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                (v + menu_frames).to_le_bytes().to_vec()
            }
            "robot-bank" => inv_robot_bank(&w.bytes),
            // The T2 banks (W12-S3): E canonical = u32 count + the
            // records; the O1 raw form = the bare span (no count cell
            // on the guest — the free-slot walk is the bound).
            "weapon-anim-bank" => {
                assert_eq!(w.bytes.len(), 4 + 400 * 0x36);
                w.bytes[4..].to_vec()
            }
            "projectile-bank" => {
                assert_eq!(w.bytes.len(), 4 + 50 * 0x22);
                w.bytes[4..].to_vec()
            }
            "move-target-words" => {
                // canonical u32 count + n*9 records -> the 0x60 EXD
                // span: x[i]/y[i] u32 by ABSOLUTE id, -1 = none, the
                // tail slots stay at the spawn -1 fill.
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 9);
                let mut span = vec![0xFFu8; 0x60];
                for i in 0..n.min(12) {
                    let rec = &w.bytes[4 + i * 9..];
                    if rec[0] != 0 {
                        let (px, py) = (4 * i, 0x30 + 4 * i);
                        span[px..px + 4].copy_from_slice(
                            &i32::from_le_bytes(rec[1..5].try_into().unwrap()).to_le_bytes(),
                        );
                        span[py..py + 4].copy_from_slice(
                            &i32::from_le_bytes(rec[5..9].try_into().unwrap()).to_le_bytes(),
                        );
                    }
                }
                span
            }
            "beacon-family" => {
                // canonical u32 x5 -> the five u16 cells.
                w.bytes
                    .chunks(4)
                    .flat_map(|c| (u32::from_le_bytes(c.try_into().unwrap()) as u16).to_le_bytes())
                    .collect()
            }
            "typedb-fade-byte" | "armor-pad-reads" => {
                let len = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + len);
                if len == 0 {
                    // len 0 == all-zero w*h grid (§6a equivalence).
                    let (mw, mh) = map_wh.expect("anchor statics precede grid rows");
                    vec![0u8; (mw * mh) as usize]
                } else {
                    w.bytes[4..].to_vec()
                }
            }
            "static-map-wh" => {
                let wv = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                let hv = u32::from_le_bytes(w.bytes[4..8].try_into().unwrap());
                let mut span = vec![0u8; 0x30];
                span[0x00..0x04].copy_from_slice(&hv.to_le_bytes());
                span[0x2c..0x30].copy_from_slice(&wv.to_le_bytes());
                span
            }
            // identity rows (scalars, selection-triple, order-target,
            // per-player, spread-claims).
            _ => w.bytes.clone(),
        };
        f.push_watch(id, bytes);
    }
    f
}

/// Stitch fabricated O1 frames into dump bytes.
fn stitch_o1(scen_src: &str, frames: Vec<FrameRecord>) -> Vec<u8> {
    let scen = diffharness::runner::Scenario::parse(scen_src).unwrap();
    let header = DumpHeader::new(Channel::O1ExdDosboxX, sha256(b"exd-test"), scen.id.clone());
    stitch(
        &scen,
        &diffharness::runner::Transcript { frames },
        &header,
        &registry(),
    )
    .expect("fabricated transcript stitches")
    .bytes
}

#[test]
fn s0_s1_cross_and_double_run() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    let reg = registry();

    // S0 = T0+TS only (no T1 rows -> no coverage asymmetry); S1 adds
    // the T1 slice: blink-cursor + move-target-words rows are E-only
    // (the latter because the O1 side consumes its span into the
    // robot-bank splice) and the robot field gaps are ZERO — the D90
    // splice sources the record-external target trio (34 canonical
    // leaves, all 34 shared, RE-EXD-MAP §8). S2 (D91) is the first
    // scenario whose splice carries a live present=1 span both ways:
    // the staged walker's target (22,73) fabricates into the EXD
    // x[1]/y[1] u32 pair and splices back — same 2 row-level
    // findings, zero field gaps. S3 (W12-S3, D103) adds the T2 slice:
    // the two full bank rows fabricate as the bare spans and parse
    // back through the shared field walk — still exactly the 2
    // row-level findings, zero field gaps, zero T2 diffs.
    for (id, frames_total, pinned_chain, expect_coverage) in [
        ("S0", 3u64, "8901789a88cf61fe", 0u64),
        ("S1", 401u64, "1c4e7b4c9d9b0947", 2u64),
        ("S2", 17u64, "809f4961b7757da4", 2u64),
        // Re-pinned at the W12-S4-prep landing (D104, §7j.39/9) —
        // the artillery burst-pair application draws the shared
        // stream (was 49193732e6dbc546).
        ("S3", 133u64, "e29f76f5585401e1", 2u64),
    ] {
        let src = fs::read_to_string(scen_path(id)).unwrap();
        let e_run = run_canonical(&src, &root).unwrap();
        assert_eq!(e_run.manifest.frame_count, frames_total);
        assert_eq!(
            e_run.manifest.chain_digest, pinned_chain,
            "the pinned E content"
        );
        let e_dump = decode_dump(&e_run.bytes).unwrap();

        // Fabricate the O1 side (+2000 menu frames on the counter).
        let mut o1_frames = Vec::new();
        let mut map_wh: Option<(u32, u32)> = None;
        for ef in &e_dump.frames {
            if let Some(b) = ef.watch("static-map-wh") {
                map_wh = Some((
                    u32::from_le_bytes(b[..4].try_into().unwrap()),
                    u32::from_le_bytes(b[4..8].try_into().unwrap()),
                ));
            }
            o1_frames.push(inv_frame(ef, map_wh, 2000, 0));
        }
        let o1_bytes = stitch_o1(&src, o1_frames);

        // ---- cross-channel: the mapped-field contract holds ----
        let res = run_diff(
            &e_run.bytes,
            &o1_bytes,
            None,
            &DiffConfig::new(Mode::CrossChannel),
            &reg,
        )
        .unwrap();
        assert_eq!(
            res.verdict,
            Verdict::PassWithNotes,
            "{id} cross verdict\n{}",
            report_text(&res)
        );
        assert_eq!(res.count(Class::EngineBug), 0, "{id}");
        assert_eq!(res.count(Class::Structural), 0, "{id}");
        // blink-cursor + move-target-words rows are E-only (S1): the
        // splice sources every robot leaf, so exactly the 2 row-level
        // findings remain — no field-level gaps.
        assert_eq!(res.count(Class::Coverage), expect_coverage, "{id}");
        if expect_coverage > 0 {
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "blink-cursor" && f.class == Class::Coverage));
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "move-target-words" && f.class == Class::Coverage));
        }
        // The never-resetting O1 counter is the single T2 note.
        assert_eq!(res.count(Class::T2Reported), 1, "{id}");
        // The anchor statics normalized identically (w/h from the span).
        assert!(res.findings.iter().all(|f| f.row != "static-map-wh"));

        // ---- double-run: the DH-G1 verdict shape ----
        let o1_frames_b: Vec<FrameRecord> = e_dump
            .frames
            .iter()
            .map(|ef| inv_frame(ef, map_wh, 2003, 0x1357_1357))
            .collect();
        let o1_bytes_b = stitch_o1(&src, o1_frames_b);
        let res = run_diff(
            &o1_bytes,
            &o1_bytes_b,
            None,
            &DiffConfig::new(Mode::DoubleRun),
            &reg,
        )
        .unwrap();
        assert_eq!(
            res.verdict,
            Verdict::Pass,
            "{id} double-run modulo counter/RNG\n{}",
            report_text(&res)
        );
        assert!(res.findings.is_empty(), "{id}");

        // ...and FAILS on any other byte diff (money on S1 stays 4000;
        // perturb the fabricated score row's neighbor through money).
        let mut bad: Vec<FrameRecord> = e_dump
            .frames
            .iter()
            .map(|ef| inv_frame(ef, map_wh, 2000, 0))
            .collect();
        for f in bad.iter_mut() {
            if let Some(w) = f.watches.iter_mut().find(|w| w.id == "money") {
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) - 7;
                w.bytes = v.to_le_bytes().to_vec();
            }
        }
        let bad_bytes = stitch_o1(&src, bad);
        let res = run_diff(
            &o1_bytes,
            &bad_bytes,
            None,
            &DiffConfig::new(Mode::DoubleRun),
            &reg,
        )
        .unwrap();
        assert_eq!(res.verdict, Verdict::Fail, "{id}");
        assert_eq!(res.first_divergence().unwrap().row, "money");
    }
}
