//! TITLE.SMK playback integration gate (P5, D31). Skips when the
//! corpus is absent (CI); when present it drives GameHost end-to-end
//! through a full TITLE.SMK playback on the Title scene and pins:
//!
//! 1. PACING: movie frame k (0-based index, frame 0 held from Title
//!    entry) decodes exactly at host pump ceil(k * 15_998_400 /
//!    4_000_000) after Title entry - the x240-us accumulator on the
//!    60 Hz / 4-sub-tick host grid - sampled at k = 1, 2, 3, 5, 600
//!    and the final frame 1226;
//! 2. COMPOSITING: the canonical frame equals a DIRECT SmkStream walk
//!    composited independently (full 640x320 raster at anchor y=80
//!    plus the folded 6-bit palette) at sampled frame indices;
//! 3. DETERMINISM: two independent full playbacks produce identical
//!    SHA-256 chains over every per-pump frame parity hash and every
//!    rendered audio sample;
//! 4. HASH ISOLATION: the per-pump scene-hash chain is IDENTICAL with
//!    and without the movie loaded (D17 bucket b).
//!
//! game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level. No decoded media enters
//! git - only hashes are asserted.

use std::path::PathBuf;

use bedlam_assets::smk::{SmkFrameStatus, SmkStream};
use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;
use bedlam_game::{GameConfig, GameHost, Scene};
use sha2::{Digest, Sha256};

/// Host pace: 60 Hz frames = 4 sub-ticks = 4_000_000 x240-us units.
const SUBTICKS_PER_PUMP: u32 = 4;
const UNITS_PER_PUMP: u64 = SUBTICKS_PER_PUMP as u64 * 1_000_000;
/// TITLE.SMK frame period in x240-us units (66_660 us * 240).
const PERIOD: u64 = 15_998_400;
/// TITLE.SMK frame count (D30 gate).
const FRAMES: u64 = 1227;

/// Loop pump (1-based, counted from the pump AFTER the Boot -> Title
/// transition) at which movie frame k (0-based) has decoded. Playback
/// starts mid-pump on the transition itself (sync_movie starts the
/// slot, then pump_movie advances it with that pump dt), so the player
/// accumulator after loop pump p holds (p + 1) * 4_000_000 units:
/// frame k lands on ceil(k * period / 4_000_000) - 1.
fn pump_of_frame(k: u64) -> u64 {
    (k * PERIOD).div_ceil(UNITS_PER_PUMP) - 1
}

fn title() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX/TITLE.SMK")
}

fn host() -> GameHost {
    GameHost::new(
        &GameConfig::default(),
        &SimConfig::default(),
        [[0u8, 0, 0]; 256],
    )
}

/// Walk Boot to the Title scene (BOOT_TICKS pumps at 60 Hz).
fn reach_title(h: &mut GameHost) {
    let idle = InputFrame::default();
    while h.scene() == Scene::Boot {
        h.pump_frame(SUBTICKS_PER_PUMP, &idle);
    }
}

/// One full playback: the SHA-256 chain over per-pump frame parity
/// hashes + rendered audio samples, the scene-hash chain, and the
/// (pump, frame_index) samples at the milestones.
fn playback(data: &[u8]) -> (Vec<u8>, Vec<u64>, Vec<(u64, u32)>) {
    let mut h = host();
    h.load_movie(Scene::Title, data).unwrap();
    reach_title(&mut h);
    let idle = InputFrame::default();
    let mut chain = Sha256::new();
    let mut scene_chain: Vec<u64> = Vec::new();
    let mut samples: Vec<(u64, u32)> = Vec::new();
    const MILESTONES: [u64; 6] = [1, 2, 3, 5, 600, 1226];
    let mut next_milestone = 0usize;
    let mut audio = [0i16; 184];
    let mut pumps = 0u64;
    loop {
        h.pump_frame(SUBTICKS_PER_PUMP, &idle);
        pumps += 1;
        chain.update(h.frame().parity_hash().to_le_bytes());
        h.render_audio(&mut audio).unwrap();
        for s in audio {
            chain.update(s.to_le_bytes());
        }
        scene_chain.push(h.scene_hash().0);
        let idx = h.movie().unwrap().frame_index() as u64;
        if next_milestone < MILESTONES.len() && idx == MILESTONES[next_milestone] {
            samples.push((pumps, idx as u32));
            next_milestone += 1;
        }
        let done = h.movie().is_none() || h.movie().unwrap().finished();
        assert!(pumps < 20_000, "playback runaway");
        if done && next_milestone == MILESTONES.len() {
            return (chain.finalize().to_vec(), scene_chain, samples);
        }
    }
}

#[test]
fn title_playback_gate() {
    let path = title();
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }
    let data = std::fs::read(&path).expect("read TITLE.SMK");

    // 3 + 1 + pacing: two identical full playbacks.
    let (chain1, scene_chain, samples) = playback(&data);
    let (chain2, scene_chain2, samples2) = playback(&data);
    assert_eq!(chain1, chain2, "two full playbacks must be byte-identical");
    assert_eq!(scene_chain, scene_chain2);
    assert_eq!(samples, samples2);
    for (i, (pump, idx)) in samples.iter().enumerate() {
        let k = [1u64, 2, 3, 5, 600, 1226][i];
        assert_eq!(*idx as u64, k, "sampled movie frame index");
        assert_eq!(*pump, pump_of_frame(k), "frame {k} decode pump");
    }
    assert_eq!(samples.last().unwrap().0, pump_of_frame(1226));

    // 4: hash isolation - the same pumps without the movie.
    let mut h = host();
    reach_title(&mut h);
    let idle = InputFrame::default();
    let bare: Vec<u64> = (0..scene_chain.len())
        .map(|_| {
            h.pump_frame(SUBTICKS_PER_PUMP, &idle);
            h.scene_hash().0
        })
        .collect();
    assert_eq!(scene_chain, bare, "movie never touches the scene hash");

    // 2: compositing cross-check against a direct SmkStream walk,
    // lockstepped on the same accumulator arithmetic.
    let mut h = host();
    h.load_movie(Scene::Title, &data).unwrap();
    reach_title(&mut h);
    let mut stream = SmkStream::open(&data).unwrap();
    stream.first_frame().unwrap();
    let idle = InputFrame::default();
    let targets = [1u64, 600, 1226];
    let mut next_target = 0usize;
    let mut checked = 0usize;
    let mut pumps = 0u64;
    loop {
        h.pump_frame(SUBTICKS_PER_PUMP, &idle);
        pumps += 1;
        // Direct walk catches up to everything due at this pump. The
        // host accumulator includes the transition pump (+1).
        while (u64::from(stream.frame_index()) + 1) < FRAMES
            && (u64::from(stream.frame_index()) + 1) * PERIOD <= (pumps + 1) * UNITS_PER_PUMP
        {
            match stream.next_frame().unwrap() {
                SmkFrameStatus::More | SmkFrameStatus::Last => {}
                SmkFrameStatus::Done => unreachable!("1227-frame walk overrun"),
            }
        }
        let k = u64::from(stream.frame_index());
        if next_target < targets.len() && k == targets[next_target] && pumps == pump_of_frame(k) {
            let frame = h.frame();
            let mut want_pal = [[0u8; 3]; 256];
            for (d, s) in want_pal.iter_mut().zip(stream.palette().iter()) {
                *d = [s[0] >> 2 & 0x3f, s[1] >> 2 & 0x3f, s[2] >> 2 & 0x3f];
            }
            assert_eq!(frame.palette, want_pal, "palette at movie frame {k}");
            let px = stream.pixels();
            assert_eq!(px.len(), 640 * 320, "raster size");
            for row in 0..320usize {
                let y = 80 + row;
                assert_eq!(
                    &frame.indices[y * 640..y * 640 + 640],
                    &px[row * 640..row * 640 + 640],
                    "raster row {row} at movie frame {k}"
                );
            }
            checked += 1;
            next_target += 1;
        }
        assert!(pumps < 20_000, "cross-check runaway");
        if next_target == targets.len() {
            break;
        }
    }
    assert_eq!(checked, 3, "three composites cross-checked");
}
