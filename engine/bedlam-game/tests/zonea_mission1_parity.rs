//! ZONEA-MISSION1 zone-parity EVIDENCE gate (P5, docs/P5-ZONE-GATES
//! §1 acceptance shape; the D178 closure unit). One place that
//! executes, per acceptance criterion, the machine-checkable half of
//! the zone A closure for its only mission:
//!
//! | §1 | Criterion | This file's leg |
//! |----|-----------|-----------------|
//! | 1 | scripted flows complete crash-free | `zonea_scripted_flows_complete_crash_free` — every ZONEA-shaped S-scenario (S0-S4, S6-S8) runs to its declared frame budget through the canonical runner; the dump verifies; two runs byte-identical |
//! | 2 | T1 rules vs RE | `zonea_t1_rules_spot` (the campaign/economy/selection rules) + `zonea_structural_spot_check` (the anchor TS statics re-derived independently from the corpus bytes); the deep oracle suites run as gate commands (see docs/required-gates.toml p5-zone-a) |
//! | 3 | perceptual frame checks at key moments (T2, diagnostic band) | key-moment frame pins live in mission_scene_gate (spawn/mid-walk frame hashes, GAMEPAL fold, 254/256 non-black) — a gate command; T2 stays the 0b diagnostic band (thresholds + owner sign-off), never a pixel-exact gate |
//! | 4 | differ structural spot-check | `zonea_structural_spot_check` — the S0 dump's structural contract (anchor, monotone frame_no, record count) + the anchor statics; the differ itself runs as the differ_gate gate command |
//! | 5 | cross-OS replay hash equality (our engine) | the replay-hash fixtures run as gate commands (bedlam-core hash_fixture + this crate's determinism suite); the ubuntu+windows CI matrix enforces both on every push — the corpus-gated chains additionally run wherever a corpus is present |
//! | 6 | original SAVED/OPTIONS.BDL import read-only, bounds-checked, fuzzed | `original_saved_and_options_import_seam` + `import_seam_fuzz_bounded` over the REAL shipped files (RE-EXW-SIM §7j.70) |
//! | 7 | DM carve-out | a note, not a check: ZONEA-MISSION1 hosts no deathmatch-only content (DM is mode-level: the same maps under netplay, out of the parity exit); its map load + local SP semantics are criteria 1-2 above |
//!
//! Corpus-gated (skips cleanly without game-data, the CI contract).
//! Read-only over game-data; every corpus read is fs::read only.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::PathBuf;

use bedlam_core::sim::SimConfig;
use bedlam_game::{
    mission_asset_names, mission_number_for_mask, GameConfig, GameError, GameHost, FULL_MASK,
    MAX_STAGE,
};
use canonical::run_canonical;
use diffharness::dump::{decode_dump, Channel};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR/ZONEA/MISSION1.TOT").is_file()
}

fn scen_path(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/diffharness/scenarios")
        .join(format!("{id}.scen"))
}

/// The S-scenarios whose staging is ZONEA/MISSION1 (S5/S5B/S5C run
/// the ZONEB set-2 pickup corridors — zone B's evidence, not A's).
const ZONEA_SCENARIOS: [&str; 8] = ["S0", "S1", "S2", "S3", "S4", "S6", "S7", "S8"];

/// Expected canonical records for one scenario: anchor + its declared
/// `frames = N` header (parsed from the file, not hardcoded per id).
fn declared_frames(id: &str) -> u64 {
    let text = fs::read_to_string(scen_path(id)).expect("scenario committed");
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with("frames"))
        .unwrap_or_else(|| panic!("{id}: no frames header"));
    let n: u64 = line
        .split('=')
        .nth(1)
        .expect("frames = <n>")
        .trim()
        .split(['#', ';'])
        .next()
        .expect("numeric")
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("{id}: bad frames header {line:?}"));
    n
}

// ---------------------------------------------------------------------
// §1 criterion 1 — all scripted flows complete without crashes
// ---------------------------------------------------------------------

#[test]
fn zonea_scripted_flows_complete_crash_free() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    for id in ZONEA_SCENARIOS {
        let src = fs::read_to_string(scen_path(id)).unwrap_or_else(|e| panic!("{id}: {e}"));
        let expected = declared_frames(id) + 1; // + the anchor record
        let run = run_canonical(&src, &root)
            .unwrap_or_else(|e| panic!("{id} did not complete crash-free: {e}"));
        assert_eq!(
            run.manifest.frame_count, expected,
            "{id}: flow must run its full declared budget"
        );
        assert!(!run.bytes.is_empty(), "{id}: dump emitted");
        let rerun =
            run_canonical(&src, &root).unwrap_or_else(|e| panic!("{id} re-run failed: {e}"));
        assert_eq!(run.bytes, rerun.bytes, "{id}: two-run byte identity");
        assert_eq!(
            run.manifest.chain_digest, rerun.manifest.chain_digest,
            "{id}: chain digest identity"
        );
        // The dump itself verifies (CRC/shape/grammar — the structural
        // decode pass), and the scenario id rides the header.
        let dump = decode_dump(&run.bytes).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(dump.header.channel, Channel::Engine);
        assert_eq!(dump.header.scenario, id);
        assert_eq!(dump.frames.len() as u64, expected);
        let nos: Vec<u64> = dump.frames.iter().map(|f| f.frame_no).collect();
        let mut sorted = nos.clone();
        sorted.sort_unstable();
        assert_eq!(nos, sorted, "{id}: frame_no monotone");
        assert_eq!(nos.first(), Some(&0), "{id}: anchor frame 0");
        eprintln!(
            "zonea parity: {id} ok ({} records)",
            run.manifest.frame_count
        );
    }
}

// ---------------------------------------------------------------------
// §1 criteria 2+4 — T1 spot rules + the differ structural spot check
// ---------------------------------------------------------------------

/// The S0 anchor's TS statics against values INDEPENDENTLY
/// re-derived from the corpus bytes (not the engine's own loader
/// output): the TOT header is read straight off the file, the fresh
/// campaign scalars off the RE-pinned §7j.64 boot defaults.
#[test]
fn zonea_structural_spot_check() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // Independent re-derivation #1: the map dims straight from the
    // TOT header bytes (u16 w + u16 h, FORMATS-MISSION §2).
    let tot = fs::read(root.join("EDITOR/ZONEA/MISSION1.TOT")).expect("TOT readable");
    assert_eq!(tot.len(), 30_004, "4 + 16*w*h with w*h = 25*75");
    let w = u16::from_le_bytes([tot[0], tot[1]]);
    let h = u16::from_le_bytes([tot[2], tot[3]]);
    assert_eq!((w, h), (25, 75));

    let src = fs::read_to_string(scen_path("S0")).expect("S0 committed");
    let run = run_canonical(&src, &root).expect("S0 canonical run");
    let dump = decode_dump(&run.bytes).expect("S0 dump verifies");

    // TS statics ride the anchor frame only.
    let anchor = &dump.frames[0];
    let map_wh = [25u32.to_le_bytes(), 75u32.to_le_bytes()].concat();
    assert_eq!(anchor.watch("static-map-wh"), Some(&map_wh[..]));
    for later in &dump.frames[1..] {
        assert!(later.watch("static-map-wh").is_none());
    }

    // Fresh-campaign T1 scalars (the §7j.64/D154 boot-default pins):
    // money 3500 = 4000 - 500*difficulty(1) via the name-entry arm
    // 0x43aaca; difficulty 1 (the 0x41c14a boot write); zone 0 (the
    // 0-based E convention, O1 normalizer maps cell-1); mode 0 (SP).
    assert_eq!(anchor.watch("money"), Some(&3500u32.to_le_bytes()[..]));
    assert_eq!(anchor.watch("difficulty"), Some(&1u32.to_le_bytes()[..]));
    assert_eq!(anchor.watch("zone"), Some(&0u32.to_le_bytes()[..]));
    assert_eq!(anchor.watch("mode"), Some(&0u32.to_le_bytes()[..]));
    // The derived linear-mission-m cell: fresh (zone 1, mission 1)
    // -> 5*(-1)+1-1 = -5 -> the floor -> 1 (§7j.64/D).
    assert_eq!(
        anchor.watch("linear-mission-m"),
        Some(&1u32.to_le_bytes()[..])
    );
}

/// T1 rules spot table (EXW-anchored selection/economy arithmetic —
/// the deep oracle suites are the gate commands; this pins the rules
/// the zone-A flows themselves consume).
#[test]
fn zonea_t1_rules_spot() {
    // The B2 @0x81d9a full-mask table: slot 0 empty (the boot quirk),
    // slot 1 one sub, slots 2..=8 four subs (the census G1 pin: a
    // stage derives missions 1..=5 only — 6..7 wait on the SELECT
    // shell).
    assert_eq!(FULL_MASK[0], 0);
    assert_eq!(FULL_MASK[1], 1);
    for subs in FULL_MASK.iter().take(usize::from(MAX_STAGE) + 1).skip(2) {
        assert_eq!(*subs, 15);
    }
    // Mission selection = first-uncompleted sub + 1 (the same
    // arithmetic briefing_name_for_slot uses).
    assert_eq!(mission_number_for_mask(0), 1);
    assert_eq!(mission_number_for_mask(0b0001), 2);
    assert_eq!(mission_number_for_mask(0b1111), 5);

    // Campaign economy: the §7j.64/C name-entry seed 4000 - 500*d
    // (0x43aaca); boot difficulty 1 -> 3500 (D154).
    assert_eq!(bedlam_game::menu::start_score(0), 4000);
    assert_eq!(bedlam_game::menu::start_score(1), 3500);
    assert_eq!(bedlam_game::menu::start_score(2), 3000);

    // The ZONEA/MISSION1 fetch chain: the canonical 25-name set with
    // the per-mission trio heads (RE-EXW-SIM §7c: path-1 TOT/DAT/PAD,
    // path-2 CGR/BIN/LNK + the GAMEGFX tail).
    let names = mission_asset_names(0, 1);
    assert_eq!(names.len(), 25);
    assert_eq!(names[0], "ZONEA/MISSION1.TOT");
    assert_eq!(names[1], "ZONEA/MISSION1.DAT");
    assert_eq!(names[2], "ZONEA/MISSION1.PAD");
    assert_eq!(names[3], "ZONEA/MISSIONA.CGR");
    assert_eq!(names[4], "ZONEA/MISSIONA.BIN");
    assert_eq!(names[5], "ZONEA/MISSIONA.LNK");
    assert!(names.contains(&"GAMEPAL.PAL".to_string()));
    assert!(names.contains(&"SINTABLE.BIN".to_string()));
    assert!(names.iter().filter(|n| n.ends_with(".MRK")).count() == 1);
}

// ---------------------------------------------------------------------
// §1 criterion 6 — original SAVED/OPTIONS.BDL import seam
// (read-only, bounds-checked, fuzzed; RE-EXW-SIM §7j.70)
// ---------------------------------------------------------------------

fn palette() -> [bedlam_render::Vga6; 256] {
    [[0, 0, 0]; 256]
}

fn host() -> GameHost {
    GameHost::new(&GameConfig::default(), &SimConfig::default(), palette())
}

#[test]
fn original_saved_and_options_import_seam() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // SAVED.BDL: exactly 900 B (5 x 180 — the §7j.70 stride
    // arithmetic closing on the real file).
    let saved = fs::read(root.join("SAVED.BDL")).expect("shipped SAVED.BDL");
    assert_eq!(saved.len(), 900);

    // Slot 0 is the live campaign ("PLAYER"): mask 0, zone 2 (-> the
    // ZONEB stage), score 0xA40B, money 580, difficulty 1.
    let mut host = host();
    let import = host.import_saved_slot(&saved, 0).expect("slot 0 imports");
    assert_eq!(import.name, "PLAYER..");
    assert_eq!(import.stage, 2);
    assert_eq!(import.mask, 0);
    assert_eq!(import.score, 0xA40B);
    assert_eq!(import.money, 580);
    assert_eq!(import.difficulty, 1);
    // The staging is the restore's own effect: the zone cell write +
    // mask replay select ZONEB/MISSION1 next.
    assert_eq!(host.mission_slot(), (1, 1));
    assert_eq!(host.mission_asset_names()[0], "ZONEB/MISSION1.TOT");

    // Slots 1..4 are the shipped "EMPTY" rows: the EXW zero-dword
    // predicate rejects them loud, and NOTHING is staged (the
    // previous import's slot survives an empty-slot attempt).
    for slot in 1..5 {
        assert!(matches!(
            host.import_saved_slot(&saved, slot),
            Err(GameError::SaveSlotEmpty { slot: s }) if s == slot
        ));
    }
    assert_eq!(host.mission_slot(), (1, 1));

    // OPTIONS.BDL: the typed import over the real 41-B record.
    let options = fs::read(root.join("OPTIONS.BDL")).expect("shipped OPTIONS.BDL");
    assert_eq!(options.len(), 41);
    let parsed = bedlam_assets::bdl::parse_options_bdl(&options).expect("options parse");
    let config = GameConfig::from_options(&parsed).expect("options validate");
    assert_eq!(config.player_name, "Player..");
    assert_eq!(config.volume, 75);
    assert_eq!(config.language, 0);
    assert_eq!(config.installdrive, b'C');
    assert!(config.backbuffer && config.actionpan && config.cd_audio);
    assert!(config.code_no_title && config.midi && config.sound);
}

/// Bounded deterministic fuzz of the import seam over the REAL file
/// bytes: bit flips across every slot header, truncations, and size
/// attacks — Ok/Err only, never a panic; every Ok stages cleanly.
#[test]
fn import_seam_fuzz_bounded() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let saved = fs::read(root().join("SAVED.BDL")).expect("shipped SAVED.BDL");

    // Full header-window bit flip sweep (offsets 0x00..0x18 of each
    // of the five slots).
    for slot in 0..5usize {
        for bit in 0..(0x18 * 8) {
            let mut d = saved.clone();
            let at = slot * 180 + bit / 8;
            if at >= d.len() {
                break;
            }
            d[at] ^= 1 << (bit % 8);
            let mut host = host();
            if let Ok(import) = host.import_saved_slot(&d, slot) {
                // In-model imports must stage to their own slot (the
                // endgame stage clamps to zone G, zone_for_stage).
                assert_eq!(
                    host.mission_slot(),
                    (
                        bedlam_game::zone_for_stage(import.stage),
                        mission_number_for_mask(import.mask)
                    )
                );
            }
        }
    }

    // Truncations at the header-sensitive offsets + size attacks.
    let mut seed = 0xC0FFEE_u64;
    let mut byte = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(0x42);
        (seed >> 33) as u8
    };
    for len in [
        0usize, 1, 0x0C, 0x0D, 0x0E, 0x10, 0x11, 0x12, 0x16, 0x18, 179, 180, 181, 359, 899, 901,
        2048,
    ] {
        let d: Vec<u8> = (0..len).map(|_| byte()).collect();
        let mut host = host();
        let _ = host.import_saved_slot(&d, 2);
    }
    // A random 900-B image per slot (full-payload fuzz).
    for slot in 0..5usize {
        let d: Vec<u8> = (0..900).map(|_| byte()).collect();
        let mut host = host();
        let _ = host.import_saved_slot(&d, slot);
    }

    // OPTIONS fuzz through the typed view: only the volume domain is
    // validated (0..=100); everything else passes through typed —
    // and nothing panics.
    let options = fs::read(root().join("OPTIONS.BDL")).expect("shipped OPTIONS.BDL");
    for bit in 0..(41 * 8) {
        let mut d = options.clone();
        d[bit / 8] ^= 1 << (bit % 8);
        if let Ok(parsed) = bedlam_assets::bdl::parse_options_bdl(&d) {
            let _ = GameConfig::from_options(&parsed);
        }
    }
}

// ---------------------------------------------------------------------
// §1 criterion 5 (local half) — the stitch seam the cross-OS replay
// evidence rides: a transcript re-stitches byte-identically. The
// cross-OS half is the CI matrix (ubuntu+windows) running the
// hash_fixture + determinism suites — the gate commands.
// ---------------------------------------------------------------------

#[test]
fn zonea_replay_stitch_is_stable() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    let src = fs::read_to_string(scen_path("S0")).expect("S0 committed");
    let a = run_canonical(&src, &root).expect("S0 run");
    let b = run_canonical(&src, &root).expect("S0 re-run");
    // The stitch seam is deterministic end-to-end: bytes, manifest
    // counters and the chain digest are all reproducible.
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(a.manifest.frame_count, b.manifest.frame_count);
    assert_eq!(a.manifest.chain_digest, b.manifest.chain_digest);
    // (The stitch seam itself is exercised inside run_canonical; the
    // differ's substrate is this byte-identical transcript.)
}
