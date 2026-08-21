//! Mission scene corpus gate (P4 scene step, DESIGN-GAME sec 11).
//! Skips when the corpus is absent (CI); when present it drives the
//! composed MissionScene - bedlam-core MissionSim + bedlam-render
//! MissionView through GameHost - over the REAL shipped ZONEA/
//! MISSION1 bytes staged via host.load_mission, and EXTENDS the
//! render corpus pin family (the terrain pin 90a9e929eea24ced and
//! the entity pins 8d2c559df035b75b / 8804f9deec6b1fee live in
//! bedlam-render tests/mission_view_gate.rs and are NOT re-derived
//! here - this gate pins the SCENE-composed frames):
//!
//! 1. STAGING: the 10-file chain (TOT/DAT/PAD/CGR/BIN/LNK plus the
//!    GAMEGFX tail SINTABLE, DANTE, GAMEPAL, then MRK) stages the
//!    ZONEA/MISSION1 mission; MRK record 0 is (21, 73, z-level 1)
//!    plus the staged second marker (18, 73, 1) - the host/test seam
//!    the network override 0x46cbe0 fills in the original
//!    (RE-EXW-SIM sec 7c.8). Entering Mission activates the camera
//!    at robot 0's Q5 spawn.
//! 2. SPAWN FRAME: the entry pump presents the 480x480 viewport at
//!    canonical (0,0) - real terrain + both DANTE robots, under the
//!    folded GAMEPAL palette (the GAMEPAL present tail; the frame
//!    palette IS GAMEPAL); frame parity hash pinned.
//! 3. CLICK SEAM: a scripted left-click at robot 0's projected
//!    screen position arms the order AT robot 0 (tile (21,73), snap
//!    to tile origin, state 3) - the sec 6.4 semantics.
//! 4. WALK: three advance frames later the second robot is mid-walk
//!    (state 4, live anim); frame parity hash pinned.
//! 5. DETERMINISM: two independent full runs produce identical hash
//!    traces.
//!
//! PIN REGENERATION 2026-08-21 (GAMEPAL unit): the two FRAME pins
//! (spawn + mid-walk) were regenerated ONCE when the mission plane
//! palette changed from the host stand-in (all black in this gate)
//! to the folded GAMEGFX\GAMEPAL.PAL - Frame::parity_hash covers the
//! palette, so the pins moved; the SIM pins (spawn/click) and every
//! observation pin are UNCHANGED (the palette touches no sim state).
//!
//! PIN REGENERATION 2026-08-21 (SIDEBAR ART unit): the two FRAME
//! pins moved AGAIN, once, when the [480,640) strip stopped being
//! black - the sidebar then carried the select portraits + the
//! order-row chrome from the real GAMEGFX\GENERAL.BIN
//! (FUN_004072bf/FUN_00408403, RE-EXW-SIM 6c.8; GENERAL.BIN +
//! SMLFONT.BIN joined the staged chain). The SIM pins
//! (spawn/click) and every observation pin were UNCHANGED - the
//! sidebar art is presentation-only (D17).
//!
//! PIN REGENERATION 2026-08-21 (WEAPON TABLE unit, D51): the two
//! FRAME pins moved ONCE more, for two verified reasons: (a) the
//! ui_bank RLE codec was corrected to the FUN_00401ca2 asm - a
//! literal control word with bit14 set ends the line (every shipped
//! sidebar sprite row is one 0x4000|w word; the old decode painted
//! each sprite as a single row), and RLE transparency copies
//! literal bytes verbatim rather than filtering zeros; (b) the
//! weapon table is now the REAL data (RE-EXW-SIM 7d: host-staged
//! session state, fresh-campaign default EMPTY), so the default
//! path draws portraits but NO rows - rows + NAME/COUNT text draw
//! only under a staged loadout, pinned separately by the new
//! ARMED spawn-frame pin. The SIM pins (spawn/click) and every
//! observation pin are UNCHANGED.
//!
//! game-data access is read-only. No game bytes enter git - only
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
/// CGR, BIN, LNK, SINTABLE, DANTE, GAMEPAL, GENERAL, SMLFONT, MRK.
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
        read(&["GAMEGFX", "GAMEPAL.PAL"])?,
        read(&["GAMEGFX", "GENERAL.BIN"])?,
        read(&["GAMEGFX", "SMLFONT.BIN"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.MRK"])?,
    ])
}

/// One scripted run: boot out of Boot, Advance to Mission, stage the
/// mission, one entry pump (spawn frame), aim the cursor at robot
/// 0's projection, click (arm), then three walk pumps. Returns the
/// observation chain (spawn frame hash, sim hash at spawn, click
/// sim hash, walker state/anim after the walk, mid-walk frame hash).
/// `arm_loadout` stages robot 0's weapon loadout (the D51 seam)
/// before the scene activates - the fresh-campaign default is EMPTY
/// [RE-EXW-SIM 7d.4].
fn scripted_run(
    files: &[Vec<u8>],
    loadout: Option<&[(u16, u16); 7]>,
) -> (u64, u64, u64, (u16, u16, i32), u64) {
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
        &files[9],
        &files[10],
        &files[11],
        None,
        &[(18, 73, 1)],
    )
    .expect("ZONEA/MISSION1 stages");
    if let Some(groups) = loadout {
        host.mission_mut()
            .expect("staged")
            .set_weapon_loadout(0, groups);
    }
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

    let (spawn_frame, spawn_sim, click_sim, walker_obs, walk_frame) = scripted_run(&files, None);
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
        &files[9],
        &files[10],
        &files[11],
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
    // The sidebar columns [480,640) carry the select portraits from
    // the real GENERAL.BIN at the spawn frame (FUN_004072bf — 48x48
    // sprites for both alive robots), and — the faithful
    // fresh-campaign default (RE-EXW-SIM 7d.4, D51) — NO order
    // rows: the weapon table starts EMPTY until the pre-mission
    // shop fills it, so the rows band (y 0x57..0xB8, the 7 order
    // rects) stays black.
    let band = |y0: usize, y1: usize| -> usize {
        (y0..y1)
            .map(|r| {
                frame.indices[r * 640 + 480..(r + 1) * 640]
                    .iter()
                    .filter(|&&b| b != 0)
                    .count()
            })
            .sum()
    };
    let portraits = band(5, 0x36);
    assert!(
        portraits > 2_000,
        "the portrait band carries the GENERAL.BIN art ({portraits})"
    );
    assert_eq!(
        band(0x57, 0xB9),
        0,
        "the order-rows band stays black (empty loadout)"
    );

    // The GAMEPAL present tail: the frame palette IS the folded
    // GAMEGFX\GAMEPAL.PAL (6-bit file values verbatim; the frame
    // carries the mission plane palette through the MovieFrame seam)
    // and presentation must re-upload it (palette_dirty).
    let expect_gamepal = bedlam_assets::pal::parse_vga770(&files[8]).expect("GAMEPAL parses");
    let mut folded = [[0u8; 3]; 256];
    for (dst, src) in folded.iter_mut().zip(expect_gamepal.0) {
        *dst = [src[0] >> 2, src[1] >> 2, src[2] >> 2];
    }
    assert_eq!(
        frame.palette, folded,
        "the mission frame palette is GAMEPAL"
    );
    assert!(frame.palette_dirty, "the mission palette is a fresh upload");
    // 254 of 256 GAMEPAL entries are non-black on the corpus (entry 0
    // and 255 are) - the pin would pass vacuously under the old
    // all-black stand-in otherwise. Entry 1 = 6-bit (0x3E,0x3A,0x39).
    assert_eq!(
        folded.iter().filter(|&&c| c != [0u8, 0, 0]).count(),
        254,
        "GAMEPAL carries color"
    );
    assert_eq!(folded[1], [0x3E, 0x3A, 0x39]);

    // --- the sidebar producer on real corpus bytes (sec 6c) ----------
    // Stage both robots' loadouts (the D51 seam) so the producer has
    // rows to gate on: robot 0 NEEDLER CANNON #1 (30) + HADES BOMB
    // #1 (3), robot 1 the same. State after the entry pump: slot 0
    // selected, countdown 1 (the MissionShell entry trigger 0x447c74
    // set 2, the entry pump's one present decremented it), both
    // robots carry the spawn default bits 1<<0 [sec 6c.6].
    let mut loadout = [(0u16, 0u16); 7];
    loadout[0] = (2, 30); // NEEDLER CANNON #1
    loadout[1] = (9, 3); // HADES BOMB #1
    host.mission_mut()
        .expect("staged on Mission")
        .set_weapon_loadout(0, &loadout);
    host.mission_mut()
        .expect("staged on Mission")
        .set_weapon_loadout(1, &loadout);
    {
        let mission = host.mission().expect("staged on Mission");
        assert_eq!(mission.sidebar_selected(), 0);
        assert_eq!(
            mission.sidebar_redraw(),
            1,
            "2 armed - 1 present (seam adds none)"
        );
        assert_eq!(mission.order_bits(0), 1, "spawn default bit 0");
        assert_eq!(mission.order_bits(1), 1);
    }
    // Select strip 1 (x 0x219, squad of 2: MRK[0] + the staged
    // marker) -> slot 1, countdown set 2 then decremented by this
    // pump's present (1 left).
    host.pump_frame(
        4,
        &InputFrame {
            mouse_dx: 0x219,
            mouse_dy: 5,
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
    {
        let mission = host.mission().expect("still on Mission");
        assert_eq!(mission.sidebar_selected(), 1, "strip 1 selects slot 1");
        assert_eq!(
            mission.sidebar_redraw(),
            1,
            "2 set, 1 decremented by the present"
        );
    }
    // Strip 2 is gated off (squad 2 < 3, the DAT_0046cbd8 analog):
    // no select, no redraw. (Cursor is at (0x219,5); moving to
    // (0x24B,0x35) crosses no wired region on the way — motion never
    // fires, only edges.)
    host.pump_frame(
        4,
        &InputFrame {
            mouse_dx: 0x24B - 0x219,
            mouse_dy: 0x35 - 5,
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
    {
        let mission = host.mission().expect("still on Mission");
        assert_eq!(mission.sidebar_selected(), 1, "strip 2 gated (squad < 3)");
        assert_eq!(
            mission.sidebar_redraw(),
            0,
            "countdown ran out, no new fire"
        );
    }
    // Order row 0 on the SELECTED robot: bit 0 toggles off, countdown
    // 2 -> 1 after the click pump, and NO order is armed (sidebar
    // clicks are presentation-only, D17).
    host.pump_frame(
        4,
        &InputFrame {
            mouse_dx: 0x200 - 0x24B,
            mouse_dy: 0x57 - 0x35,
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
    {
        let mission = host.mission().expect("still on Mission");
        assert_eq!(mission.order_bits(1), 0, "row 0 toggled robot 1's bit 0");
        assert_eq!(mission.order_bits(0), 1, "robot 0 untouched");
        assert_eq!(mission.sidebar_redraw(), 1);
        assert!(mission.sim().order().is_none(), "sidebar click never arms");
        assert_eq!(mission.sim().robots()[0].state, 0, "robot 0 stays idle");
    }
    host.pump_frame(4, &InputFrame::default());
    host.pump_frame(4, &InputFrame::default());
    assert_eq!(
        host.mission().expect("still on Mission").sidebar_redraw(),
        0,
        "countdown drains to zero and sticks"
    );

    // The hash pins (extend the render-gate family: these are the
    // SCENE-composed frames — host pipeline + fixed spawn camera +
    // one render per pump). Regenerated ONCE per presentation unit
    // (GAMEPAL, sidebar art, and this unit: the codec fix + the
    // faithful empty loadout - see the header); the sim pins did
    // not move across any of them.
    assert_eq!(
        format!("{spawn_frame:016x}"),
        "9f20732f29a5baf2",
        "ZONEA/MISSION1 spawn-moment scene frame (GAMEPAL + portraits, empty loadout)"
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
        "27494d6ab505bcf3",
        "ZONEA/MISSION1 mid-walk scene frame (GAMEPAL + portraits, empty loadout)"
    );

    // The ARMED path: robot 0 carries a staged loadout, so the entry
    // frames draw the order rows AND the NAME/COUNT text through the
    // real SMLFONT glyphs (FUN_00408403 + FUN_00420260, RE 7d.5).
    // Structural pins: the rows band now carries chrome, the name
    // column carries color-0x24 text pixels, and the sim hash at the
    // spawn moment is IDENTICAL to the default run (the loadout is
    // presentation-only state, D17/D51).
    let (armed_frame, armed_sim, _, _, _) = scripted_run(&files, Some(&loadout));
    eprintln!("armed pins: spawn_frame {armed_frame:016x} spawn_sim {armed_sim:016x}");
    {
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
            &files[9],
            &files[10],
            &files[11],
            None,
            &[(18, 73, 1)],
        )
        .unwrap();
        host.mission_mut()
            .expect("staged")
            .set_weapon_loadout(0, &loadout);
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        let frame = host.frame();
        let rows_band: usize = (0x57..0xB9)
            .map(|r| {
                frame.indices[r * 640 + 480..(r + 1) * 640]
                    .iter()
                    .filter(|&&b| b != 0)
                    .count()
            })
            .sum();
        assert!(
            rows_band > 1_000,
            "the rows band carries the 2-row chrome ({rows_band})"
        );
        // Rows 0/1 exist (armed + unarmed); row 2+ empty. Row 0
        // body y 0x59..0x63, name text x 0x1ED.., count x 0x25C...
        let text_px = (0x5B..0x5B + 7)
            .flat_map(|y| (0x1ED..0x258).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.indices[y * 640 + x] == 0x24)
            .count();
        assert!(
            text_px > 20,
            "the NAME text paints color-0x24 glyphs ({text_px})"
        );
        let count_px = (0x5B..0x5B + 7)
            .flat_map(|y| (0x25C..0x276).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.indices[y * 640 + x] == 0x24)
            .count();
        assert!(count_px > 8, "the COUNT text '0030' paints ({count_px})");
        let mission = host.mission().expect("armed run");
        assert_eq!(mission.order_bits(0), 1, "row 0 armed by the spawn armer");
        assert_eq!(
            mission.state_hash().0,
            spawn_sim,
            "the loadout never reaches the sim hash"
        );
    }
    assert_eq!(
        format!("{armed_frame:016x}"),
        "51ebd515bc638e81",
        "ZONEA/MISSION1 spawn frame under a staged loadout (rows + text)"
    );

    // Determinism: two independent runs are identical.
    let again = scripted_run(&files, None);
    assert_eq!(spawn_frame, again.0, "spawn frame reproducible");
    assert_eq!(spawn_sim, again.1, "spawn sim hash reproducible");
    assert_eq!(click_sim, again.2, "click sim hash reproducible");
    assert_eq!(walker_obs, again.3, "walker observation reproducible");
    assert_eq!(walk_frame, again.4, "walk frame reproducible");
    let armed_again = scripted_run(&files, Some(&loadout));
    assert_eq!(armed_frame, armed_again.0, "armed frame reproducible");
}
