//! Mission scene corpus gate (P4 scene step, DESIGN-GAME sec 11).
//! Skips when the corpus is absent (CI); when present it drives the
//! composed MissionScene — bedlam-core MissionSim + bedlam-render
//! MissionView through GameHost — over the REAL shipped ZONEA/
//! MISSION1 bytes staged via host.load_mission, and EXTENDS the
//! render corpus pin family (the terrain pin 90a9e929eea24ced and
//! the entity pins 8d2c559df035b75b / 8804f9deec6b1fee live in
//! bedlam-render tests/mission_view_gate.rs and are NOT re-derived
//! here — this gate pins the SCENE-composed frames):
//!
//! 1. STAGING: the 9-file chain (TOT/DAT/PAD/CGR/BIN/LNK + SINTABLE
//!    + DANTE + MRK) stages the ZONEA/MISSION1 mission; MRK record 0
//!    (21, 73, z-level 1) + the staged second marker (18, 73, 1) —
//!    the host/test seam the network override 0x46cbe0 fills in the
//!    original (RE-EXW-SIM sec 7c.8). Entering Mission activates the
//!    camera at robot 0's Q5 spawn.
//! 2. SPAWN FRAME: the entry pump presents the 480x480 viewport at
//!    canonical (0,0) — real terrain + both DANTE robots; frame
//!    parity hash pinned.
//! 3. CLICK SEAM: a scripted left-click at robot 0's projected
//!    screen position arms the order AT robot 0 (tile (21,73), snap
//!    to tile origin, state 3) — the sec 6.4 semantics.
//! 4. WALK: three advance frames later the second robot is mid-walk
//!    (state 4, live anim); frame parity hash pinned.
//! 5. DETERMINISM: two independent full runs produce identical hash
//!    traces.
//!
//! game-data access is read-only. No game bytes enter git — only
//! hashes and counts are asserted.

use std::path::PathBuf;

use bedlam_core::input::InputFrame;
use bedlam_game::{GameConfig, GameHost, Scene, SceneAction};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn read(rel: &[&str]) -> Option<Vec<u8>> {
    std::fs::read(root().join(rel.iter().collect::<PathBuf>())).ok()
}

/// The staged corpus inputs in load_mission order: TOT, DAT, PAD,
/// CGR, BIN, LNK, SINTABLE, DANTE, MRK.
fn zonea() -> Option<Vec<Vec<u8>>> {
    Some(vec![
        read(&["EDITOR", "ZONEA", "MISSION1.TOT"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.DAT"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.PAD"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.CGR"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.BIN"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.LNK"])?,
        read(&["GAMEGFX", "SINTABLE.BIN"])?,
        read(&["GAMEGFX", "DANTE.BIN"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.MRK"])?,
    ])
}

/// One scripted run: boot out of Boot, Advance to Mission, stage the
/// mission, one entry pump (spawn frame), aim the cursor at robot
/// 0's projection, click (arm), then three walk pumps. Returns the
/// observation chain (spawn frame hash, sim hash at spawn, click
/// sim hash, walker state/anim after the walk, mid-walk frame hash).
fn scripted_run(files: &[Vec<u8>]) -> (u64, u64, u64, (u16, u16, i32), u64) {
    let mut host = GameHost::new(
        &GameConfig::default(),
        &bedlam_core::sim::SimConfig::default(),
        [[0u8, 0, 0]; 256],
    );
    // Stage BEFORE entering the scene (the chain fetches on the
    // transition; staging is inert until Mission either way).
    host.load_mission(
        &files[0],
        &files[1],
        &files[2],
        &files[3],
        &files[4],
        &files[5],
        &files[6],
        &files[7],
        &files[8],
        None,
        &[(18, 73, 1)],
    )
    .expect("ZONEA/MISSION1 stages");
    while host.scene() == Scene::Boot {
        host.pump_frame(4, &InputFrame::default());
    }
    host.apply(SceneAction::Advance); // Title -> Brief
    host.apply(SceneAction::Advance); // -> Select
    host.apply(SceneAction::Advance); // -> Mission
    assert_eq!(host.scene(), Scene::Mission);

    // --- the spawn moment ---------------------------------------------
    // Entry pump: activation (camera fixed at robot 0 Q5) + first
    // present; no sim tick yet.
    host.pump_frame(4, &InputFrame::default());
    let mission = host.mission().expect("staged on Mission");
    assert!(mission.is_active());
    // Robot 0 Q5 = tile (21,73) + 0xF00 center >> 8 = (687, 2351).
    assert_eq!(mission.camera(), (687, 2351));
    assert_eq!(mission.sim().robots().len(), 2, "MRK[0] + staged marker");
    assert_eq!(mission.sim().frame(), 0, "the entry pump renders only");
    assert_eq!(mission.render_count(), 1);
    let spawn_frame = host.frame().parity_hash();
    let spawn_sim = mission.state_hash().0;

    // --- the click -----------------------------------------------------
    // Robot 0 projected at camera == its own position: dx=dy=0,
    // colAdj 0x20, rowAdj 15, z 31 -> (0x130, 0x10C+15-31) = (304,
    // 252). Aim the cursor (deltas integrate from (0,0)), then the
    // click edge.
    host.pump_frame(
        4,
        &InputFrame {
            mouse_dx: 304,
            mouse_dy: 252,
            ..InputFrame::default()
        },
    );
    host.pump_frame(
        4,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    let mission = host.mission().expect("still on Mission");
    let order = mission.sim().order().expect("click armed the order");
    assert_eq!(order.tile.0, 21, "armed AT the clicked robot's tile");
    assert_eq!(order.tile.1, 73);
    assert_eq!(mission.sim().robots()[0].state, 3, "state 3 [FUN_004247b5]");
    assert_eq!(
        mission.sim().robots()[0].pos_x,
        21 << 13,
        "snap to the tile origin"
    );
    let click_sim = mission.state_hash().0;

    // --- mid-walk ------------------------------------------------------
    for _ in 0..3 {
        host.pump_frame(4, &InputFrame::default());
    }
    let mission = host.mission().expect("still on Mission");
    let walker = &mission.sim().robots()[1];
    assert_eq!(walker.state, 4, "the staged walker is mid-walk");
    assert_ne!(walker.anim, 0, "the walk anim phase is live");
    let walker_obs = (walker.state, walker.anim, walker.pos_x);
    let walk_frame = host.frame().parity_hash();

    (spawn_frame, spawn_sim, click_sim, walker_obs, walk_frame)
}

#[test]
fn zonea_mission1_scene_frames_hash_pinned() {
    let Some(files) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };

    let (spawn_frame, spawn_sim, click_sim, walker_obs, walk_frame) = scripted_run(&files);
    eprintln!(
        "scene pins: spawn_frame {spawn_frame:016x} spawn_sim {spawn_sim:016x} \
         click_sim {click_sim:016x} walker {walker_obs:?} walk_frame {walk_frame:016x}"
    );

    // Structural pins first: the frame carries real terrain + robots
    // in the viewport window and stays black in the sidebar columns
    // (the EXW mission screen split, sec 6.2).
    let mut host = GameHost::new(
        &GameConfig::default(),
        &bedlam_core::sim::SimConfig::default(),
        [[0u8, 0, 0]; 256],
    );
    host.load_mission(
        &files[0],
        &files[1],
        &files[2],
        &files[3],
        &files[4],
        &files[5],
        &files[6],
        &files[7],
        &files[8],
        None,
        &[(18, 73, 1)],
    )
    .unwrap();
    while host.scene() == Scene::Boot {
        host.pump_frame(4, &InputFrame::default());
    }
    host.apply(SceneAction::Advance);
    host.apply(SceneAction::Advance);
    host.apply(SceneAction::Advance);
    host.pump_frame(4, &InputFrame::default());
    let frame = host.frame();
    let viewport_nonzero = frame.indices[..480 * 480]
        .iter()
        .filter(|&&b| b != 0)
        .count();
    assert!(
        viewport_nonzero > 50_000,
        "the viewport window carries real content ({viewport_nonzero})"
    );
    // The sidebar columns [480,640) stay black across every row (the
    // EXW mission screen split, sec 6.2 — sidebar art is a later
    // unit).
    let sidebar: usize = (0..480)
        .map(|r| {
            frame.indices[r * 640 + 480..(r + 1) * 640]
                .iter()
                .filter(|&&b| b != 0)
                .count()
        })
        .sum();
    assert_eq!(sidebar, 0, "the sidebar stays black this slice");

    // The hash pins (extend the render-gate family: these are the
    // SCENE-composed frames — host pipeline + fixed spawn camera +
    // one render per pump).
    assert_eq!(
        format!("{spawn_frame:016x}"),
        "51ef4fe93eaaed77",
        "ZONEA/MISSION1 spawn-moment scene frame"
    );
    assert_eq!(
        format!("{spawn_sim:016x}"),
        "36ddc86345c8351c",
        "sim state hash at the spawn moment"
    );
    assert_eq!(
        format!("{click_sim:016x}"),
        "f35db41f0efb858d",
        "sim state hash after the click arm"
    );
    assert_eq!(
        format!("{walk_frame:016x}"),
        "7bae11a5c7f34ab6",
        "ZONEA/MISSION1 mid-walk scene frame"
    );

    // Determinism: two independent runs are identical.
    let again = scripted_run(&files);
    assert_eq!(spawn_frame, again.0, "spawn frame reproducible");
    assert_eq!(spawn_sim, again.1, "spawn sim hash reproducible");
    assert_eq!(click_sim, again.2, "click sim hash reproducible");
    assert_eq!(walker_obs, again.3, "walker observation reproducible");
    assert_eq!(walk_frame, again.4, "walk frame reproducible");
}
