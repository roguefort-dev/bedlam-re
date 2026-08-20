//! Title-menu corpus gate (P4, D41/D42). Skips when the corpus is
//! absent (CI); when present it stages the real menu assets
//! (LANGUAGE.ENG + FULLFONT.BIN + FULLPAL.PAL + the SOUND/SFX
//! MENU1/MENU2 pair) on a GameHost and pins the EXW
//! NameEntryScreen semantics end-to-end (docs/RE-EXW-TITLEMENU.md):
//!
//! 1. TABLE: menu 1 builds the exact corpus [MENU_ITEMS] strings in
//!    EXW order (3/30/difficulty 0/31+" "+name/5/68/94, count 7);
//!    the difficulty cycle rebuilds slots for STANDARD / BEDLAM !!!;
//! 2. GEOMETRY: the strip draws bottom-anchored (row 0x1d6 -
//!    count*0x18, 24-px rows) on a black canvas;
//! 3. COLOR SETS (the D41 sec 2a pin, end-to-end): the selected
//!    row's glyph pixels index the GREEN FULLPAL ramp slice
//!    (233..=244, base 0x82) and the unselected rows the BLUE slice
//!    (244..=255, base 0) - same shapes, disjoint ramps;
//! 4. ACTIONS: the start click hands Title -> Brief off with the
//!    score seed 4000 - difficulty*500; difficulty/name/quit and
//!    the multiplayer count cycle all act (see the unit tests for
//!    the full dispatch; this pins the corpus-visible surface);
//! 5. SFX: the real MENU1/MENU2 RAWs stage as mixer instruments and
//!    render audible samples after a hover + click;
//! 6. ATTRACT REPLAY: MoviePlayer::restart rewinds the real
//!    TITLE.SMK to frame 0 and requeues its frame-0 audio (the EXW
//!    attract re-opens the file through FUN_004459f7);
//! 7. DETERMINISM: two independent scripted interaction runs are
//!    byte-identical (SHA-256 over the per-pump scene-hash +
//!    frame-parity-hash chain).
//!
//! game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level. No decoded media
//! enters git - only hashes are asserted.

use std::path::PathBuf;

use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;
use bedlam_game::menu::{
    MenuId, MenuPhase, ATTRACT_IDLE, ROW_H, STRIP_X_MAX, STRIP_X_MIN, STRIP_Y_MAX,
};
use bedlam_game::{GameConfig, GameHost, Scene};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn read(rel: &[&str]) -> Option<Vec<u8>> {
    std::fs::read(root().join(rel.iter().collect::<PathBuf>())).ok()
}

/// Host pace: 60 Hz frames = 4 sub-ticks.
const SUBTICKS_PER_PUMP: u32 = 4;

/// Stage the corpus menu on a fresh host and walk it to Title.
/// No movie staged: the menu owns the plane immediately.
fn corpus_host() -> GameHost {
    let language = read(&["LANGUAGE.ENG"]).expect("corpus present but LANGUAGE.ENG missing");
    let font = read(&["GAMEGFX", "FULLFONT.BIN"]).expect("FULLFONT.BIN missing");
    let pal = read(&["GAMEGFX", "FULLPAL.PAL"]).expect("FULLPAL.PAL missing");
    let hover = read(&["SOUND", "SFX", "MENU1.RAW"]).expect("MENU1.RAW missing");
    let click = read(&["SOUND", "SFX", "MENU2.RAW"]).expect("MENU2.RAW missing");
    let mut host = GameHost::new(
        &GameConfig::default(),
        &SimConfig::default(),
        [[0u8, 0, 0]; 256],
    );
    host.load_title_menu(&language, &font, &pal, &hover, &click)
        .unwrap();
    while host.scene() != Scene::Title {
        host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
    }
    host
}

/// One-tick exact hover to item i, then press + release over two
/// pumps. Returns nothing; asserts the landing.
fn menu_click(host: &mut GameHost, i: i8) {
    let menu = host.menu().expect("menu staged");
    let count = menu.count() as i32;
    let top = STRIP_Y_MAX - count * ROW_H;
    let y = top + i as i32 * ROW_H + ROW_H / 2;
    let x = (STRIP_X_MIN + STRIP_X_MAX) / 2;
    let (cx, cy) = menu.cursor();
    host.pump_frame(
        SUBTICKS_PER_PUMP,
        &InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            ..InputFrame::default()
        },
    );
    assert_eq!(host.menu().unwrap().sel(), i, "cursor landed on item {i}");
    host.pump_frame(
        SUBTICKS_PER_PUMP,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
}

#[test]
fn menu_corpus_table_geometry_and_color_sets() {
    let mut host = corpus_host();
    let menu = host.menu().unwrap();
    // 1. TABLE: the exact corpus strings, EXW order.
    assert_eq!(menu.count(), 7);
    let slots = menu.slots();
    assert_eq!(slots[0], b"New Single Player Game".as_slice());
    assert_eq!(slots[1], b"Start Saved Game".as_slice());
    assert_eq!(slots[2], b"Difficulty: SIMPLE".as_slice());
    assert_eq!(slots[3], b"Name: ".as_slice());
    assert_eq!(slots[4], b"View Hall of Fame".as_slice());
    assert_eq!(slots[5], b"Credits".as_slice());
    assert_eq!(slots[6], b"Quit to Windows".as_slice());

    // 2/3. GEOMETRY + COLOR: hover item 0, inspect the frame.
    menu_click(&mut host, 2); // difficulty -> STANDARD (also rebuilds)
    assert_eq!(
        host.menu().unwrap().slots()[2],
        b"Difficulty: STANDARD".as_slice()
    );
    menu_click(&mut host, 2); // -> BEDLAM !!!
    assert_eq!(
        host.menu().unwrap().slots()[2],
        b"Difficulty: BEDLAM !!!".as_slice()
    );
    menu_click(&mut host, 2); // back to SIMPLE
                              // Hover (no click) onto item 0 for the color pin.
    let menu = host.menu().unwrap();
    let top = STRIP_Y_MAX - 7 * ROW_H;
    let y = top + ROW_H / 2;
    let x = (STRIP_X_MIN + STRIP_X_MAX) / 2;
    let (cx, cy) = menu.cursor();
    host.pump_frame(
        SUBTICKS_PER_PUMP,
        &InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            ..InputFrame::default()
        },
    );
    assert_eq!(host.menu().unwrap().sel(), 0);
    host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
    let frame = host.frame();
    // Canvas black well above the strip.
    for r in (0..200u32).step_by(7) {
        for c in (0..640u32).step_by(11) {
            assert_eq!(frame.get(c, r), Some(0), "row {r} col {c} above strip");
        }
    }
    // Strip rows: every item row carries glyph pixels.
    for i in 0..7u32 {
        let r = (STRIP_Y_MAX - 7 * ROW_H + i as i32 * ROW_H) as u32;
        let any = (0..640u32).any(|c| frame.get(c, r) != Some(0));
        assert!(any, "item row {i} (row {r}) draws");
    }
    // Color sets: the selected row indexes the green slice, the
    // others the blue slice (D41 sec 2a).
    let row_values = |r0: u32, r1: u32| -> Vec<u8> {
        (r0..r1)
            .flat_map(|r| (0..640u32).map(move |c| (r, c)))
            .filter_map(|(r, c)| frame.get(c, r).filter(|&v| v != 0))
            .collect()
    };
    let selected = row_values(303, 326);
    let unselected = row_values(327, 350);
    assert!(!selected.is_empty() && !unselected.is_empty());
    assert!(
        selected.iter().all(|&v| (233..=244).contains(&v)),
        "selected row uses the green ramp slice, got {:?}",
        selected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(
        unselected.iter().all(|&v| (244..=255).contains(&v)),
        "unselected rows use the blue ramp slice, got {:?}",
        unselected
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
    );
    // The blue rows are NOT all shadow (both sets actually draw).
    assert!(unselected.iter().any(|&v| v > 244));
    assert!(selected.iter().any(|&v| v < 244));
}

#[test]
fn menu_corpus_start_hands_off_with_the_seed() {
    let mut host = corpus_host();
    // Difficulty 1 -> seed 3500.
    menu_click(&mut host, 2);
    menu_click(&mut host, 0);
    assert_eq!(host.scene(), Scene::Brief);
    assert_eq!(host.menu_start_score_seen(), Some(3500));
}

#[test]
fn menu_corpus_sfx_audible() {
    let mut host = corpus_host();
    // Hover + click land MENU1/MENU2 as sounding instrument voices
    // with the real RAW waves.
    menu_click(&mut host, 4); // HOF stub: click sfx, inert action
    assert_eq!(host.scene(), Scene::Title);
    let mut buf = [0i16; 2048];
    let n = host.render_audio(&mut buf).unwrap();
    assert!(n > 0);
    assert!(
        buf[..n * 2].iter().any(|&s| s != 0),
        "corpus MENU1/MENU2 render audible samples"
    );
}

#[test]
fn menu_corpus_attract_restart_rewinds_title_smk() {
    let Some(smk) = read(&["GAMEGFX", "TITLE.SMK"]) else {
        eprintln!("corpus absent: skipping");
        return;
    };
    let mut player = bedlam_game::MoviePlayer::new(&smk).unwrap();
    assert_eq!(player.frame_index(), 0);
    let first_audio = player.take_audio();
    assert!(!first_audio.is_empty(), "frame-0 audio queues at open");
    // Advance past a few frame periods (66_660 us * 240 / 1_000_000
    // sub-ticks per frame, ~16 per frame).
    for _ in 0..50 {
        player.advance(SUBTICKS_PER_PUMP).unwrap();
    }
    assert!(player.frame_index() > 0);
    // Restart: the EXW attract replay shape - frame 0 back on the
    // raster, frame-0 audio requeued, not finished.
    player.restart().unwrap();
    assert_eq!(player.frame_index(), 0);
    assert!(!player.finished());
    assert_eq!(player.take_audio(), first_audio);
    // And it plays on from the top.
    for _ in 0..50 {
        player.advance(SUBTICKS_PER_PUMP).unwrap();
    }
    assert!(player.frame_index() > 0);
}

/// A scripted corpus interaction (hover walk, difficulty cycle,
/// name entry with typing, quit-confirm cancel, multiplayer count,
/// idle attract with no staged movie = host-cancelled) hashed per
/// pump over scene hash + frame parity hash.
fn scripted_run() -> [u8; 32] {
    let mut host = corpus_host();
    let mut hasher = Sha256::new();
    let mut hash_pump = |host: &mut GameHost| {
        hasher.update(host.scene_hash().0.to_le_bytes());
        hasher.update(host.frame().parity_hash().to_le_bytes());
        host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
    };
    // Walk the hover across every item of menu 1.
    for i in 0..7 {
        let menu = host.menu().unwrap();
        let top = STRIP_Y_MAX - 7 * ROW_H;
        let y = top + i * ROW_H + ROW_H / 2;
        let x = (STRIP_X_MIN + STRIP_X_MAX) / 2;
        let (cx, cy) = menu.cursor();
        host.pump_frame(
            SUBTICKS_PER_PUMP,
            &InputFrame {
                mouse_dx: (x - cx) as i16,
                mouse_dy: (y - cy) as i16,
                ..InputFrame::default()
            },
        );
        for _ in 0..3 {
            hash_pump(&mut host);
        }
    }
    // Difficulty cycle x3 (full rotation), then name entry: type
    // BEDLAM, backspace once, exit by click (GOD rule not hit - the
    // name is non-empty).
    for _ in 0..3 {
        menu_click(&mut host, 2);
    }
    menu_click(&mut host, 3);
    assert_eq!(host.menu().unwrap().phase(), MenuPhase::NameEntry);
    for c in b"BEDLAM" {
        assert!(host.menu_type_char(*c));
    }
    assert!(host.menu_backspace());
    // Name-entry exit: a raw click (the strip hover is inactive in
    // the sub-loop, so menu_click's landing assert would not hold).
    host.pump_frame(
        SUBTICKS_PER_PUMP,
        &InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        },
    );
    host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
    assert_eq!(host.menu().unwrap().name(), b"BEDLA".as_slice());
    // Quit-confirm then cancel back to Main.
    menu_click(&mut host, 6);
    assert_eq!(host.menu().unwrap().id(), MenuId::QuitConfirm);
    menu_click(&mut host, 1);
    // Multiplayer detour: players count up and down, back to Main.
    menu_click(&mut host, -1); // outside-strip click opens Multi
    assert_eq!(host.menu().unwrap().id(), MenuId::Multi);
    menu_click(&mut host, 2); // left click: players 3
    assert_eq!(host.menu().unwrap().players(), 3);
    menu_click(&mut host, 2); // players 4
    menu_click(&mut host, 3); // Main Menu
                              // Idle past the attract threshold with no staged movie: the
                              // host cancels, the run stays on Title.
    for _ in 0..(ATTRACT_IDLE + 64) {
        hash_pump(&mut host);
    }
    assert_eq!(host.scene(), Scene::Title);
    hasher.finalize().into()
}

#[test]
fn menu_corpus_scripted_interaction_is_deterministic() {
    let Some(probe) = read(&["LANGUAGE.ENG"]) else {
        eprintln!("corpus absent: skipping");
        return;
    };
    drop(probe);
    assert_eq!(scripted_run(), scripted_run());
}
