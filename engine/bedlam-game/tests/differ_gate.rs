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
//!    move-target-words rows are E-only; the 26 unmapped robot
//!    fields per robot) + the one T2 frame-counter note (the O1
//!    counter carries menu frames — never matches E by construction).
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

/// Robot canonical record (94 B) -> EXD 0xA8 record with the 8
/// RE-EXD-MAP §8-mapped fields placed (everything else zero).
fn inv_robot_bank(canon: &[u8]) -> Vec<u8> {
    assert!(canon.len() >= 4);
    let n = u32::from_le_bytes(canon[0..4].try_into().unwrap()) as usize;
    assert_eq!(canon.len(), 4 + n * 94);
    let mut out = Vec::with_capacity(n * 0xA8);
    for i in 0..n {
        let rec = &canon[4 + i * 94..4 + (i + 1) * 94];
        // §6a order: alive@0, pos_x@1, pos_y@5, z@9, state@13,
        // drop_countdown@52, hp@56 (stop_dist@39).
        let mut r = vec![0u8; 0xA8];
        let alive = rec[0];
        let pos_x = i32::from_le_bytes(rec[1..5].try_into().unwrap());
        let pos_y = i32::from_le_bytes(rec[5..9].try_into().unwrap());
        let z = i32::from_le_bytes(rec[9..13].try_into().unwrap());
        let state = u16::from_le_bytes(rec[13..15].try_into().unwrap());
        let stop = i32::from_le_bytes(rec[39..43].try_into().unwrap());
        let drop = i32::from_le_bytes(rec[52..56].try_into().unwrap());
        let hp = i32::from_le_bytes(rec[56..60].try_into().unwrap());
        r[0x00..0x04].copy_from_slice(&pos_x.to_le_bytes());
        r[0x04..0x08].copy_from_slice(&pos_y.to_le_bytes());
        r[0x08..0x0C].copy_from_slice(&z.to_le_bytes());
        r[0x0C..0x0E].copy_from_slice(&state.to_le_bytes());
        r[0x2C..0x2E].copy_from_slice(&(drop as u16).to_le_bytes());
        r[0x74..0x78].copy_from_slice(&stop.to_le_bytes());
        r[0x78..0x7C].copy_from_slice(&hp.to_le_bytes());
        r[0x7C..0x80].copy_from_slice(&(alive as i32).to_le_bytes());
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
            // E-only rows: the O1 plan cannot carry them.
            "blink-cursor" | "move-target-words" => continue,
            "rng-state-a" | "rng-state-b" => {
                let v = u64::from_le_bytes(w.bytes[..8].try_into().unwrap()) as u32;
                v.wrapping_add(rng_wander).to_le_bytes().to_vec()
            }
            "frame-counter" => {
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                (v + menu_frames).to_le_bytes().to_vec()
            }
            "robot-bank" => inv_robot_bank(&w.bytes),
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
    // and the robot field gaps are 26 per robot (1 robot on ZONEA).
    for (id, frames_total, pinned_chain, expect_coverage) in [
        ("S0", 3u64, "8901789a88cf61fe", 0u64),
        ("S1", 401u64, "1c4e7b4c9d9b0947", 2 + 26),
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
        // blink-cursor + move-target-words rows are E-only (S1); the
        // robot field gaps are 26 per robot per frame (S1: 1 robot).
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
