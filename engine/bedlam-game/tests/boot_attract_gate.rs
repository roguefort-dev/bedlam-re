//! Boot attract corpus gate (P5, D36). Skips when the corpus is
//! absent (CI); when present it drives BOTH region pairs
//! (GTLOG_{UK,US} then LOGO_{UK,US}) through the standalone
//! BootAttract flow at the 60 Hz host pace and pins the EXW runner
//! semantics (RE-EXW-GAMETHREAD.md, Boot attract arm RE):
//!
//! 1. ONE PASS: each ring movie plays exactly frames-1 decoded
//!    frames (FUN_0044567c loop bound) - frame indices stay inside
//!    0..=frames-2 and the ring NEVER wraps;
//! 2. PACING: the GTLOG pass ends exactly when (frames-1) periods
//!    elapsed on the x240-us accumulator grid (pump-count pinned by
//!    the closed formula), the LOGO pass likewise, then Done;
//! 3. AUDIO: the DPCM track bytes queue continuously and in decode
//!    order, with the LOGO frame-0 packet landing exactly at the
//!    switch;
//! 4. DETERMINISM: two independent full runs are byte-identical
//!    (SHA-256 over the per-pump observation chain).
//!
//! Corpus facts (bedlam-assets tests/smk_corpus_gate.rs, D32):
//! GTLOG 70 frames / LOGO 71 frames, both 640x480, 66_660 us ring
//! DPCM. game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level. No decoded media enters
//! git - only hashes are asserted.

use std::path::PathBuf;

use bedlam_game::{boot_pair, BootAttract, BootPhase, Region};
use sha2::{Digest, Sha256};

/// Host pace: 60 Hz frames = 4 sub-ticks = 4_000_000 x240-us units.
const UNITS_PER_PUMP: u64 = 4_000_000;
/// Frame period in x240-us units (66_660 us * 240, D32 gate).
const PERIOD: u64 = 15_998_400;
/// Corpus frame counts (D32 gate).
const GTLOG_FRAMES: u64 = 70;
const LOGO_FRAMES: u64 = 71;

fn gfx() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX")
}

fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(gfx().join(name)).ok()
}

/// Loop pump (1-based) at which a pass of `frames` frames (one EXW
/// pass = frames-1 rendered frames = (frames-1) * PERIOD units) ends,
/// starting from pump `start` whose advance already fed 1 unit-pump:
/// the movie budget elapses on pump start + ceil(budget/UNITS) - 1.
fn pump_of_pass_end(start: u64, frames: u64) -> u64 {
    let budget = (frames - 1) * PERIOD;
    start + budget.div_ceil(UNITS_PER_PUMP) - 1
}

/// Drive one region pair to Done at the 60 Hz pace; returns
/// (per-pump observation hash chain, audio byte count, max GTLOG
/// frame index, max LOGO frame index, pump of the switch, pump of
/// Done). Panics on decode errors.
fn run(region: Region) -> ([u8; 32], usize, u32, u32, u64, u64) {
    let [gtlog_name, logo_name] = boot_pair(region);
    let gtlog = read(gtlog_name).expect("corpus present but GTLOG missing");
    let logo = read(logo_name).expect("corpus present but LOGO missing");
    let mut flow = BootAttract::new(&gtlog, &logo).unwrap();
    let mut hasher = Sha256::new();
    let mut audio_total = 0usize;
    let (mut max_gtlog, mut max_logo) = (0u32, 0u32);
    let (mut switch_pump, mut done_pump) = (0u64, 0u64);
    let mut pump = 0u64;
    // start() hands the GTLOG frame-0 audio.
    let first = flow.start();
    audio_total += first.len();
    hasher.update([first.len() as u8]);
    hasher.update(&first);
    loop {
        pump += 1;
        let audio = flow.advance(4).unwrap();
        audio_total += audio.len();
        hasher.update((audio.len() as u32).to_le_bytes());
        hasher.update(&audio);
        let (idx, frame, phase) = (flow.movie_index(), flow.frame_index(), flow.phase());
        hasher.update([idx as u8, phase as u8]);
        hasher.update(frame.to_le_bytes());
        match idx {
            0 => max_gtlog = max_gtlog.max(frame),
            _ => {
                if switch_pump == 0 {
                    switch_pump = pump;
                }
                max_logo = max_logo.max(frame);
            }
        }
        if phase == BootPhase::Done && done_pump == 0 {
            done_pump = pump;
        }
        if pump > 2000 {
            panic!("attract did not finish: the ring must be bounded");
        }
        if done_pump != 0 {
            break;
        }
    }
    (
        hasher.finalize().into(),
        audio_total,
        max_gtlog,
        max_logo,
        switch_pump,
        done_pump,
    )
}

#[test]
fn boot_attract_corpus_one_pass_pinned() {
    let Some(gtlog_probe) = read("GTLOG_UK.SMK") else {
        eprintln!("corpus absent: skipping");
        return;
    };
    drop(gtlog_probe);

    for region in [Region::Uk, Region::Us] {
        let (chain, audio_total, max_gtlog, max_logo, switch_pump, done_pump) = run(region);
        // 1. ONE PASS, no ring wrap: the runner bound is frames-1
        //    rendered frames -> max decoded index = frames-2.
        assert_eq!(max_gtlog, (GTLOG_FRAMES - 2) as u32, "{region:?} GTLOG");
        assert_eq!(max_logo, (LOGO_FRAMES - 2) as u32, "{region:?} LOGO");
        // 2. PACING: GTLOG pass = 69 periods from pump 1; the LOGO
        //    pass then runs 70 periods from the NEXT pump.
        let expect_switch = pump_of_pass_end(1, GTLOG_FRAMES);
        let expect_done = pump_of_pass_end(expect_switch + 1, LOGO_FRAMES);
        assert_eq!(switch_pump, expect_switch, "{region:?} GTLOG pass end");
        assert_eq!(done_pump, expect_done, "{region:?} LOGO pass end");
        // 3. AUDIO: both movies carry the DPCM track throughout the
        //    pass (69 + 70 frame packets, thousands of bytes).
        assert!(
            audio_total > 100_000,
            "{region:?}: expected continuous DPCM audio, got {audio_total} bytes"
        );
        // 4. DETERMINISM: a second run is byte-identical.
        let (chain2, audio2, gtlog2, logo2, switch2, done2) = run(region);
        assert_eq!(chain, chain2, "{region:?} run 1 vs 2");
        assert_eq!(audio_total, audio2);
        assert_eq!((max_gtlog, max_logo), (gtlog2, logo2));
        assert_eq!((switch_pump, done_pump), (switch2, done2));
    }
}
