//! Briefing-intro corpus gate (P5, D37). Skips when the corpus is
//! absent (CI); when present it drives the corpus pair
//! BRF_DROP.SMK + BRF_B1.SMK through the standalone BriefIntro
//! flow at the 60 Hz host pace and pins the EXW briefing-screen
//! movie head semantics (RE-EXW-GAMETHREAD.md, "Briefing screen +
//! BRF_DROP play site" D37 section):
//!
//! 1. ONE PASS: the drop plays exactly frames-1 rendered frames -
//!    frame indices stay inside 0..=frames-2 (29 of its 30 corpus
//!    frames render; the handoff bound is the frame index reaching
//!    count-1, which is never decoded);
//! 2. PACING: the handoff fires exactly when (frames-1) periods
//!    elapsed on the x240-us accumulator grid (pump-count pinned
//!    by the closed formula), then the backdrop ring owns the
//!    plane for the rest of the scene;
//! 3. SILENT: the corpus pair carries no audio track - zero PCM
//!    bytes queue across the whole run;
//! 4. RING CONTINUES: the 512-frame backdrop ring wraps (frame
//!    index observes a decrease after reaching its maximum) and
//!    keeps playing - the flow never ends by itself;
//! 5. DETERMINISM: two independent full runs are byte-identical
//!    (SHA-256 over the per-pump observation chain).
//!
//! Corpus facts (bedlam-assets tests/smk_corpus_gate.rs, D32):
//! BRF_DROP 640x480 / 30 frames / 33_330 us / non-ring / silent;
//! BRF_B1 640x480 / 512 frames / 33_330 us / ring / silent.
//! game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level. No decoded media
//! enters git - only hashes are asserted.

use std::path::PathBuf;

use bedlam_game::{BriefIntro, BriefPhase, BRIEFING_DROP_NAME};
use sha2::{Digest, Sha256};

/// Host pace: 60 Hz frames = 4 sub-ticks = 4_000_000 x240-us units.
const UNITS_PER_PUMP: u64 = 4_000_000;
/// Frame period in x240-us units (33_330 us * 240, D32 gate).
const PERIOD: u64 = 7_999_200;
/// Corpus frame counts (D32 gate).
const DROP_FRAMES: u64 = 30;
const BACKDROP_FRAMES: u64 = 512;

fn gfx() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX")
}

fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(gfx().join(name)).ok()
}

/// Loop pump (1-based) at which the drop pass ends: one EXW pass
/// = frames-1 rendered frames = (frames-1) * PERIOD units, so the
/// handoff lands on pump ceil(budget/UNITS_PER_PUMP) counting from
/// the first advance after start().
fn pump_of_handoff(frames: u64) -> u64 {
    let budget = (frames - 1) * PERIOD;
    budget.div_ceil(UNITS_PER_PUMP)
}

/// One full observation: (hash chain, audio bytes, max drop frame,
/// max backdrop frame, first backdrop-frame seen after a decrease =
/// wrap proof, handoff pump, pumps driven). Panics on decode
/// errors or if the ring stalls.
fn run() -> ([u8; 32], usize, u32, u32, Option<u32>, u64, u64) {
    let drop = read(BRIEFING_DROP_NAME).expect("corpus present but BRF_DROP.SMK missing");
    let backdrop = read("BRF_B1.SMK").expect("corpus present but BRF_B1.SMK missing");
    let mut flow = BriefIntro::new(&drop, &backdrop).unwrap();
    let mut hasher = Sha256::new();
    let mut audio_total = 0usize;
    let (mut max_drop, mut max_backdrop) = (0u32, 0u32);
    let (mut handoff_pump, mut wrap_frame) = (0u64, None);
    let mut prev_backdrop: Option<u32> = None;
    let mut pump = 0u64;
    // start() hands the drop frame-0 audio (empty: the corpus drop
    // is silent).
    let first = flow.start();
    audio_total += first.len();
    hasher.update([first.len() as u8]);
    hasher.update(&first);
    hasher.update([flow.phase() as u8]);
    // The handoff pump + a full backdrop ring cycle: 512 periods
    // ~ 1024 pumps; drive past a second wrap margin.
    let target_pumps =
        pump_of_handoff(DROP_FRAMES) + 2 * BACKDROP_FRAMES * PERIOD.div_ceil(UNITS_PER_PUMP);
    while pump < target_pumps {
        pump += 1;
        let audio = flow.advance(4).unwrap();
        audio_total += audio.len();
        hasher.update((audio.len() as u32).to_le_bytes());
        hasher.update(&audio);
        let (phase, frame) = (flow.phase(), flow.frame_index());
        hasher.update([phase as u8]);
        hasher.update(frame.to_le_bytes());
        match phase {
            BriefPhase::Drop | BriefPhase::Staged => max_drop = max_drop.max(frame),
            BriefPhase::Backdrop => {
                if handoff_pump == 0 {
                    handoff_pump = pump;
                }
                if let Some(prev) = prev_backdrop {
                    if frame < prev && wrap_frame.is_none() {
                        wrap_frame = Some(frame);
                    }
                }
                prev_backdrop = Some(frame);
                max_backdrop = max_backdrop.max(frame);
            }
        }
    }
    (
        hasher.finalize().into(),
        audio_total,
        max_drop,
        max_backdrop,
        wrap_frame,
        handoff_pump,
        pump,
    )
}

#[test]
fn brief_intro_corpus_pair_pinned() {
    let Some(probe) = read(BRIEFING_DROP_NAME) else {
        eprintln!("corpus absent: skipping");
        return;
    };
    drop(probe);

    let (chain, audio_total, max_drop, max_backdrop, wrap_frame, handoff_pump, pumps) = run();
    // 1. ONE PASS: 29 of the 30 corpus frames render; frame 29
    //    (= count-1, the handoff bound) never decodes.
    assert_eq!(max_drop, (DROP_FRAMES - 2) as u32);
    // 2. PACING: the handoff fires exactly when 29 periods elapsed
    //    (closed formula), never earlier.
    assert_eq!(handoff_pump, pump_of_handoff(DROP_FRAMES));
    // 3. SILENT: the corpus pair carries no track - nothing queues
    //    across the whole run, start included.
    assert_eq!(audio_total, 0, "corpus BRF pair must be silent");
    // 4. RING CONTINUES: a ring stream totals frames + 1 slots
    //    (bedlam-smk smk.rs: frame 0 is the setup frame, never
    //    replayed; the duplicated closing slot is index = frames),
    //    and the wrap jumps to frame 1. Over 2+ full cycles the
    //    index reached the closing slot (512) and the observed
    //    wrap is exactly 512 -> 1, with the flow still playing.
    assert_eq!(max_backdrop, BACKDROP_FRAMES as u32);
    assert_eq!(wrap_frame, Some(1));
    // 5. DETERMINISM: a second run is byte-identical.
    let (chain2, audio2, drop2, backdrop2, wrap2, handoff2, pumps2) = run();
    assert_eq!(chain, chain2, "run 1 vs 2");
    assert_eq!(audio_total, audio2);
    assert_eq!((max_drop, max_backdrop), (drop2, backdrop2));
    assert_eq!(wrap_frame, wrap2);
    assert_eq!((handoff_pump, pumps), (handoff2, pumps2));
}
