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
//! PIN REGENERATION 2026-08-21 (MAP OVERLAY unit, RE-EXW-SIM 7e):
//! the FRAME pins (spawn/walk/armed) moved ONCE when the map button
//! chrome 0x5E @ (0x213,0x1b5) joined the normal-frame tail draw
//! (FUN_00403938 0x40724e - the last thing every non-overlay frame
//! paints). The two NEW overlay pins (frame + sim hash at the
//! toggle moment) cover the strategic-map compose. The SIM pins
//! (spawn/click) and every observation pin are UNCHANGED - the
//! overlay is presentation-only (D17).
//!
//! PIN REGENERATION 2026-08-21 (DITHER unit, RE-EXW-SIM 7i + D55):
//! the four FRAME pins (spawn/walk/overlay/armed) moved ONCE when
//! the dead/hit dither (FUN_00401ae6 + the 0x4e6ed8 noise ring)
//! joined the portrait pass - ZONEA spawns a 1-robot squad, so the
//! two beyond-squad boxes carry full static every frame and the
//! overlay frame's frozen sidebar includes it. The overlay test's
//! "stale sidebar" reference changed from an EARLIER normal frame
//! to the last-presented frame (the static seeds redraw per
//! present, so normal sidebars now differ frame to frame - exactly
//! like the EXW's per-blit FUN_0041ec59 draws). The SIM pins
//! (spawn/click/overlay/armed) and every observation pin are
//! UNCHANGED - the dither is presentation-only (D17); the sim hash
//! has covered hit_flash since D53.
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
/// CGR, BIN, LNK, SINTABLE, DANTE, GAMEPAL, GENERAL, SMLFONT, MRK,
/// TABLE, MAPTRAN0..7, MIN, NUMBERS (the map-overlay family tail,
/// 7e; the score-strip bank, 7f.9), then FLAGS + BLOWUP (the
/// effect banks, 7j.4/7j.9).
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
        read(&["GAMEGFX", "TABLE.BIN"])?,
        read(&["GAMEGFX", "MAPTRAN0.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN1.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN2.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN3.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN4.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN5.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN6.TRN"])?,
        read(&["GAMEGFX", "MAPTRAN7.TRN"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.MIN"])?,
        read(&["GAMEGFX", "NUMBERS.BIN"])?,
        read(&["GAMEGFX", "FLAGS.BIN"])?,
        read(&["GAMEGFX", "BLOWUP.BIN"])?,
    ])
}

/// The MAPTRAN ramp slices in slot order (chain parameter shape).
fn maptran_of(files: &[Vec<u8>]) -> Vec<&[u8]> {
    files[13..21].iter().map(|v| v.as_slice()).collect()
}

/// Aim the staged mission cursor at the absolute screen (x, y) with
/// one move pump. Target-driven delta: the cursor boots at the
/// GameInit center (320,240) and clamps into [9,631]x[9,463]
/// [D160/RE-EXD-MAP §5h], so raw absolute deltas from (0,0) would
/// land wrong (and every scripted target here is inside the box).
fn aim(host: &mut GameHost, x: i32, y: i32) {
    let (cx, cy) = host.mission().expect("mission staged").cursor();
    host.pump_frame(
        4,
        &InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            ..InputFrame::default()
        },
    );
}

/// One scripted run: boot out of Boot, Advance to Mission, stage the
/// mission, one entry pump (spawn frame), aim the cursor at robot
/// 0's projection, click (arm), then three walk pumps. Then the MAP
/// OVERLAY moment (RE-EXW-SIM 7e): a click in the map-toggle strip
/// rect opens the strategic map; the next pump presents the overlay
/// frame. Returns the observation chain (spawn frame hash, sim hash
/// at spawn, click sim hash, walker state/anim after the walk,
/// mid-walk frame hash, overlay frame hash, sim hash at the overlay
/// moment). `arm_loadout` stages robot 0's weapon loadout (the D51
/// seam) before the scene activates - the fresh-campaign default is
/// EMPTY [RE-EXW-SIM 7d.4].
fn scripted_run(
    files: &[Vec<u8>],
    loadout: Option<&[(u16, u16); 7]>,
) -> (u64, u64, u64, (u16, u16, i32), u64, u64, u64) {
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
        &files[23],
        &files[24],
        &files[12],
        &maptran_of(files),
        &files[21],
        &files[22],
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
    // 252). Aim the cursor (target-driven; it boots at the GameInit
    // center, D160), then the click edge.
    aim(&mut host, 304, 252);
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

    // --- the map overlay moment (7e) ------------------------------------
    // Move to the map-toggle strip [0x213,0x24D]x[0x1B5,0x1CF] and
    // click: the overlay bit flips, the lockout arms 5, and the next
    // pump presents the strategic map instead of the viewport.
    // Target-driven aim again (the cursor sits at (304,252) after
    // the robot click above).
    aim(&mut host, 0x230, 0x1C0);
    host.pump_frame(
        4,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    {
        let mission = host.mission().expect("still on Mission");
        assert!(mission.map_overlay_on(), "the strip opened the map");
        assert!(mission.map_lockout() > 0, "the 5-frame lockout armed");
    }
    host.pump_frame(4, &InputFrame::default());
    let overlay_frame = host.frame().parity_hash();
    let overlay_sim = host.mission().expect("still on Mission").state_hash().0;

    (
        spawn_frame,
        spawn_sim,
        click_sim,
        walker_obs,
        walk_frame,
        overlay_frame,
        overlay_sim,
    )
}

#[test]
fn zonea_mission1_scene_frames_hash_pinned() {
    let Some(files) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };

    let (spawn_frame, spawn_sim, click_sim, walker_obs, walk_frame, overlay_frame, overlay_sim) =
        scripted_run(&files, None);
    eprintln!(
        "scene pins: spawn_frame {spawn_frame:016x} spawn_sim {spawn_sim:016x} \
         click_sim {click_sim:016x} walker {walker_obs:?} walk_frame {walk_frame:016x} \
         overlay_frame {overlay_frame:016x} overlay_sim {overlay_sim:016x}"
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
        &files[23],
        &files[24],
        &files[12],
        &maptran_of(&files),
        &files[21],
        &files[22],
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
    // The HP/armor bars band [RE-EXW-SIM 7f.1, FUN_0040807f]: both
    // spawned robots draw the FULL HP bar (0x18 — staged hp 5000,
    // the dropship-landing default) and the EMPTY armor bar (0x8E —
    // armor 0, the gate sprite; the fresh campaign still draws it)
    // from the real GENERAL.BIN every present.
    let bars = band(0x3C, 0x55);
    assert!(
        bars > 100,
        "the bars band carries the GENERAL.BIN art ({bars})"
    );
    // The score-strip band [7f.2, FUN_004085ce]: the entry frame
    // draws icon 0xA + nine "0" score digits (score 0) and icon 0xB
    // + "004000" (the fresh-campaign money 4000) from the real
    // NUMBERS.BIN — the strip countdown armed 2 at activate.
    let strip = band(0x18E, 0x1B0);
    assert!(
        strip > 200,
        "the strip band carries the NUMBERS.BIN art ({strip})"
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
    // pump's present (1 left). Target-driven aim (D160 boot center).
    aim(&mut host, 0x219, 5);
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
    // (GAMEPAL, sidebar art, the codec fix, the faithful empty
    // loadout, and the bars + score strip unit - see the header).
    // Sim pins RE-PINNED ONCE 2026-08-21 (the damage unit, D52
    // follow-up): the state hash now covers the Robot damage fields
    // (hp/armor/hit_flash/alarm/kind/shield family — spawn hp 5000
    // is the only nonzero new value, so the sim pins move while the
    // FRAME pins stay put: the bars draw the same 5000/0 values).
    // FRAME pins RE-PINNED ONCE 2026-08-21 (the dither unit, D55):
    // ZONEA spawns a 1-robot squad, so slots 1/2 now carry the
    // FUN_00401ae6 static every frame (RE-EXW-SIM 7i) — the frame
    // hashes move, the SIM hashes do NOT (the dither is
    // presentation-only; the sim hash covers hit_flash since D53).
    assert_eq!(
        format!("{spawn_frame:016x}"),
        "7fdada56b10f1cad",
        "ZONEA/MISSION1 spawn-moment scene frame (GAMEPAL + portraits + bars + score strip + dither, empty loadout)"
    );
    assert_eq!(
        format!("{spawn_sim:016x}"),
        "1cc7b8e125165988",
        "sim state hash at the spawn moment"
    );
    assert_eq!(
        format!("{click_sim:016x}"),
        "0bf4fb534d6b3bd5",
        "sim state hash after the click arm"
    );
    assert_eq!(
        format!("{walk_frame:016x}"),
        "58ea10373e8d4284",
        "ZONEA/MISSION1 mid-walk scene frame (GAMEPAL + portraits + bars + dither, empty loadout)"
    );
    // The overlay pins [7e]: the strategic-map frame after the strip
    // click, and the sim hash at that moment (the overlay never
    // touches the sim — the hash differs from click_sim only by the
    // frames that elapsed). The overlay frame carries the STALE
    // sidebar half (bars + the strip pixels from the entry frames —
    // the non-returning tail skips the sidebar passes but never
    // clears them either).
    assert_eq!(
        format!("{overlay_frame:016x}"),
        "1d70e0bd059f5ae0",
        "ZONEA/MISSION1 strategic-map overlay frame (backdrop + stamps + markers + frozen dithered sidebar)"
    );
    assert_eq!(
        format!("{overlay_sim:016x}"),
        "78a16ba63607d197",
        "sim state hash at the overlay moment"
    );

    // The ARMED path: robot 0 carries a staged loadout, so the entry
    // frames draw the order rows AND the NAME/COUNT text through the
    // real SMLFONT glyphs (FUN_00408403 + FUN_00420260, RE 7d.5).
    // Structural pins: the rows band now carries chrome, the name
    // column carries color-0x24 text pixels, and the sim hash at the
    // spawn moment is IDENTICAL to the default run (this loadout has
    // NO battery group: set_battery(0, 0) rewrites the spawn
    // defaults, so the hash does not move — a BATTERY PACK group
    // would land hp through the sim, D52 follow-up).
    let (armed_frame, armed_sim, _, _, _, _, _) = scripted_run(&files, Some(&loadout));
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
            &files[23],
            &files[24],
            &files[12],
            &maptran_of(&files),
            &files[21],
            &files[22],
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
            "this battery-less loadout leaves the sim hash at the spawn defaults"
        );
    }
    assert_eq!(
        format!("{armed_frame:016x}"),
        "6050d20755b2d852",
        "ZONEA/MISSION1 spawn frame under a staged loadout (rows + text + bars + strip + dither)"
    );

    // Determinism: two independent runs are identical.
    let again = scripted_run(&files, None);
    assert_eq!(spawn_frame, again.0, "spawn frame reproducible");
    assert_eq!(spawn_sim, again.1, "spawn sim hash reproducible");
    assert_eq!(click_sim, again.2, "click sim hash reproducible");
    assert_eq!(walker_obs, again.3, "walker observation reproducible");
    assert_eq!(walk_frame, again.4, "walk frame reproducible");
    assert_eq!(overlay_frame, again.5, "overlay frame reproducible");
    assert_eq!(overlay_sim, again.6, "overlay sim hash reproducible");
    let armed_again = scripted_run(&files, Some(&loadout));
    assert_eq!(armed_frame, armed_again.0, "armed frame reproducible");
}

/// The strategic-map overlay on real corpus bytes [RE-EXW-SIM 7e]:
/// the overlay frame is the TABLE.BIN backdrop + the MIN/MAPTRAN
/// territory stamps + the GENERAL.BIN robot markers — the viewport
/// half is REPLACED (nothing of the terrain frame survives below
/// the backdrop's footprint... in fact the backdrop is a full
/// 480x480 RLE image, so the half is fully owned by the map), the
/// sidebar half keeps its stale art (the non-returning tail skips
/// the sidebar passes + the button chrome), consecutive overlay
/// presents are byte-identical (ZONEA's identity LNK makes the word
/// consume idempotent), and toggling back redraws the viewport +
/// the chrome.
#[test]
fn zonea_map_overlay_frame_composes_and_toggles() {
    let Some(files) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };
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
        &files[23],
        &files[24],
        &files[12],
        &maptran_of(&files),
        &files[21],
        &files[22],
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
    let normal = host.frame().indices.to_vec();
    // The map button chrome 0x5E from the real GENERAL.BIN draws at
    // (0x213,0x1b5) on every normal frame [7e.5].
    assert_ne!(normal[0x1B5 * 640 + 0x213], 0, "chrome pixel nonzero");

    // Open the map: click the strip rect (target-driven aim, D160).
    aim(&mut host, 0x230, 0x1C0);
    host.pump_frame(
        4,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    assert!(
        host.mission().expect("staged").map_overlay_on(),
        "the strip opened the map"
    );
    // The FROZEN sidebar baseline: the click frame's sidebar half —
    // whether that frame presented normal (drew the sidebar one
    // last time) or already-overlay (kept the prior frame's pixels)
    // — is exactly what every later overlay frame must show. (The
    // dither unit D55 made normal sidebars differ per frame: the
    // static seeds redraw every present, so `normal` — an EARLIER
    // frame — is no longer the byte-identical reference.)
    let frozen: Vec<u8> = host
        .frame()
        .indices
        .chunks(640)
        .flat_map(|row| row[480..].to_vec())
        .collect();
    host.pump_frame(4, &InputFrame::default());
    let map1 = host.frame().indices.to_vec();
    // The viewport half is fully owned by the backdrop (480x480 RLE
    // image 0): it carries heavy content and differs from the
    // terrain frame's half.
    let map_viewport_nonzero = map1[..480 * 480].iter().filter(|&&b| b != 0).count();
    assert!(
        map_viewport_nonzero > 50_000,
        "the backdrop + stamps own the viewport half ({map_viewport_nonzero})"
    );
    let diff: usize = (0..480 * 480).filter(|&i| map1[i] != normal[i]).count();
    assert!(diff > 10_000, "the overlay replaced the terrain ({diff})");
    // The territory stamps paint through the MAPTRAN ramps: MAPTRAN0
    // is a flat 0x6B ramp, so the far-from-robot tiles that stamp
    // paint 0x6B pixels (the walker moved, so rings 1..7 exist too —
    // but the flat base dominates by area).
    let flat = map1[..480 * 480].iter().filter(|&&b| b == 0x6B).count();
    assert!(flat > 5_000, "MAPTRAN0 flat-color stamps paint ({flat})");
    // The sidebar half keeps its stale pixels: it is byte-identical
    // to the FROZEN baseline above (the non-returning tail skipped
    // the sidebar passes; only [0,480) columns were redrawn).
    let sidebar: Vec<u8> = map1
        .chunks(640)
        .flat_map(|row| row[480..].to_vec())
        .collect();
    assert_eq!(
        sidebar, frozen,
        "the sidebar half survives the overlay frame"
    );
    // The mission keeps ticking under the overlay (the next frame is
    // a fresh overlay compose — the walker's markers/rings moved, so
    // the frames legitimately differ in the viewport half while the
    // sidebar half stays frozen).
    host.pump_frame(4, &InputFrame::default());
    let map2 = host.frame().indices.to_vec();
    let sidebar2: Vec<u8> = map2
        .chunks(640)
        .flat_map(|row| row[480..].to_vec())
        .collect();
    assert_eq!(
        sidebar2, frozen,
        "the sidebar half stays frozen across overlay frames"
    );
    assert!(
        map2[..480 * 480].iter().any(|&b| b != 0),
        "the overlay keeps composing"
    );
    assert!(
        host.mission().expect("staged").sim().frame() >= 4,
        "the sim kept ticking under the overlay"
    );

    // Close the map: the strip is still clickable through the
    // overlay (sidebar dispatch runs at x >= 0x1E0 regardless,
    // 0x40b85e before the overlay check).
    host.pump_frame(
        4,
        &InputFrame {
            mouse_buttons: 0,
            ..InputFrame::default()
        },
    );
    for _ in 0..6 {
        host.pump_frame(4, &InputFrame::default()); // spend the lockout
    }
    host.pump_frame(
        4,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    assert!(
        !host.mission().expect("staged").map_overlay_on(),
        "the strip closed the map"
    );
    host.pump_frame(4, &InputFrame::default());
    let reopened = host.frame().indices.to_vec();
    let viewport_redrawn = (0..480usize)
        .filter(|&r| reopened[r * 640..r * 640 + 480] != map1[r * 640..r * 640 + 480])
        .count();
    assert!(
        viewport_redrawn > 100,
        "the terrain frame returns ({viewport_redrawn} rows)"
    );
    assert_ne!(reopened[0x1B5 * 640 + 0x213], 0, "the chrome draws again");
}

/// The effect-row + debris-stager draws on real corpus bytes
/// [RE-EXW-SIM 7j]: the pickup seam stages a FLAGS.BIN row that
/// visibly lands in the viewport (the camera sits AT robot 0, so
/// the icon projects mid-screen), the damage seam's five debris
/// records draw BLOWUP.BIN tumble sprites until the kind-5
/// sequence table's −1 terminator frees them (the viewport then
/// returns to a stable two-frame identity — the dead robot's
/// missing DANTE body is a constant), and the select-strip click
/// lights the blink cursor at the selected portrait (GENERAL.BIN
/// 0x51+ at (0x1F0 + 0x32*slot, 0xD)). The pinned frames of the
/// other tests are untouched — effects only draw once staged.
#[test]
fn zonea_effect_rows_and_debris_draw_and_expire() {
    let Some(files) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };
    fn viewport(f: &[u8; 640 * 480]) -> Vec<u8> {
        f[..480 * 480].to_vec()
    }
    // The scripted effects journey. The CONTROL host runs the
    // IDENTICAL pump sequence and damage call, so at every capture
    // index the two hosts carry the same LNK-walk animation frame
    // and the same (dead) robot set - the only divergence is the
    // staged effect row. Returns (row_frame, debris_mid_frame,
    // cursor_pixels).
    let journey = |with_row: bool, with_cursor: bool| -> (Vec<u8>, Vec<u8>, usize) {
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
            &files[23],
            &files[24],
            &files[12],
            &maptran_of(&files),
            &files[21],
            &files[22],
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

        // --- the debris [7j.5/7j.7]: kill robot 0 - five kind-5
        // records stage with the 2k delays, tumble for
        // 2*4 + 13 + margin ticks, then the -1 terminator frees
        // every record.
        let outcome = host
            .mission_mut()
            .expect("staged")
            .apply_damage(0, 6000, -1);
        assert!(outcome.died, "robot 0 dies (hp 5000)");
        assert_eq!(host.mission().expect("staged").debris_active(), 5);
        host.pump_frame(4, &InputFrame::default());
        host.pump_frame(4, &InputFrame::default());
        assert!(
            host.mission().expect("staged").debris_active() > 0,
            "the tumble still animates"
        );
        let debris_mid = viewport(&host.frame().indices);
        for _ in 0..30 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(
            host.mission().expect("staged").debris_active(),
            0,
            "the -1 terminator freed every record"
        );

        // --- the effect row [7j.1/7j.4]: a shield pickup (case 2,
        // id 6) at ROBOT 1 (the staged marker - robot 0 is dead)
        // stages one row; the next present draws FLAGS.BIN sprite 5.
        let row_frame = if with_row {
            let outcome = host.mission_mut().expect("staged").pickup(1, 2);
            assert!(outcome.applied);
            assert_eq!(outcome.effect, 6);
            assert_eq!(host.mission().expect("staged").effect_row_count(), 1);
            host.pump_frame(4, &InputFrame::default());
            viewport(&host.frame().indices)
        } else {
            host.pump_frame(4, &InputFrame::default());
            viewport(&host.frame().indices)
        };

        // --- the blink cursor [7j.6]: 0 until the select-ack; the
        // strip click lights GENERAL 0x51+ at (0x222, 0xD).
        let mut cursor_pixels = 0;
        if with_cursor {
            assert_eq!(host.mission().expect("staged").sidebar_cursor(), 0);
            // Target-driven aim (D160 boot center).
            aim(&mut host, 0x219, 5);
            host.pump_frame(
                4,
                &InputFrame {
                    mouse_buttons: 1,
                    ..InputFrame::default()
                },
            );
            assert_eq!(host.mission().expect("staged").sidebar_cursor(), 2);
            let frame = host.frame();
            cursor_pixels = (0xD..0xD + 10)
                .map(|r| {
                    (0x222..0x222 + 10)
                        .filter(|&c| frame.indices[r * 640 + c] != 0)
                        .count()
                })
                .sum();
        }
        (row_frame, debris_mid, cursor_pixels)
    };

    // Determinism first: two identical journeys agree byte-for-byte
    // (the effects are deterministic staged state, animation
    // included).
    let (row_a, debris_a, cursor_a) = journey(true, true);
    let (row_b, debris_b, cursor_b) = journey(true, true);
    assert_eq!(row_a, row_b, "two runs: the row frames are identical");
    assert_eq!(debris_a, debris_b, "two runs: the debris frames match");
    assert_eq!(cursor_a, cursor_b, "two runs: the cursor pixels match");

    // The CONTROL comparison: same pumps + same death, no row - the
    // divergence at the capture index is exactly the FLAGS icon
    // (robot 1's shield is not drawn anywhere; the LNK animation
    // state is shared).
    let (control_row, _, _) = journey(false, false);
    let row_diff: usize = (0..480 * 480)
        .filter(|&i| row_a[i] != control_row[i])
        .count();
    assert!(
        row_diff > 10 && row_diff < 2000,
        "the FLAGS icon lands, and ONLY it ({row_diff} px)"
    );
    assert!(cursor_a > 4, "the blink cursor draws ({cursor_a} px)");
}
