//! Whole-corpus static oracle for the tile-claim bank — diffharness
//! registry row `static-claim-bank` (EXW 0x46af58 / EXD 0x119564 pointer
//! cells, arena extent 0x2710 = 10000 B).
//!
//! Expected side: an independent transcription of the original's
//! mission-load initializer chain (re-verified instruction-by-instruction
//! 2026-08-25, docs/RE-EXW-SIM.md §7j.63):
//!
//! 1. `FUN_004254e1` @0x4254e1 (called from MissionShell @0x447b85,
//!    right after the rect filler) FIRST memsets the whole 10000-B bank
//!    to 0 (`mov ecx,0x2710; mov edi,[0x46af58]; call 0x402965`), THEN
//!    walks the 45-record door-rect bank 0x4dcae8..0x4dcdb8 (stride
//!    0x10, grammar §7j.34 `{+0 state, +2 x0, +4 y0, +6 w, +8 h, +0xA
//!    variant}`) stopping at the first `state == 0` record, and stamps
//!    `claim[line[y0+row] + x0 + col] = 1` for row in 0..h, col in 0..w —
//!    with NO bounds checks anywhere (the original trusts its data).
//!    EXD twin 0x3657e instruction-equivalent (rect bank 0x92c64, line
//!    table 0x8b78c).
//! 2. The rect bank content at that moment = the `0x447b7b` whole-bank
//!    memset-0 followed by `FUN_0042c4a0`, a per-zone/mission HARDCODED
//!    store farm (zone dispatch table 0x42c484 ×7, mode gate
//!    [0x4edb88]==2 → skip, mission tables 0x42c420/34/48/5c/70 ×5 for
//!    zones 2..6, `==1`-only cases for zones 1 and 7).
//! 3. `line[y] = y * map_w` — the row-start table 0x4ea900, with the
//!    map w/h from each mission's TOT header (read independently here).
//!
//! Actual side: LANDED (S0-11b, the staging seam) —
//! [`bedlam_core::mission::MissionSim::stage_claim_bank`] stages the
//! bank at every mission load (host `load_mission`), the two modeled
//! reader gates (`stage_splash`, `platform_tile_build`) read it
//! (§7j.63), and the canonical `static-claim-bank` TS row emits the
//! image (DESIGN §6a). The third §7j.63 reader — the FUN_0042382c
//! death-blast smoke producer — is HOST-SEAMED presentation
//! (§7j.24), so no sim gate exists for it. This oracle now pins BOTH
//! sides: the expected image (the independent transcription below,
//! unchanged) vs the staged `claim_bank()` per corpus mission
//! (`claim_staging_matches_the_independent_image`), plus the
//! promoted data module pinned byte-identical to this test's own
//! transcription copy (`promoted_rect_farm_is_byte_identical`).
//! The gates' refusal behavior on claimed tiles is proven in
//! bedlam-core's `claim_seam_tests` (destroy.rs).
//!
//! Scope: valid shipped corpus only (SP fresh sessions). The H2H
//! (mode==2) filler legs are out of scope for the S0 row. No production
//! parser, loader, or terrain helper is reused on the expected side
//! (the actual side builds a synthetic all-zero terrain of the TOT
//! dims — the claim initializer reads only `terrain.size()`, and the
//! DAT/TOT dim agreement is the static_loader_differential's own pin).

use std::fs;
use std::path::{Path, PathBuf};

/// The arena allocation: `mov eax,0x2710; call 0x41db89; mov
/// ds:0x46af58,eax` @0x41d9cd..0x41d9d7 (EXD twin @0x2e300). The bank is
/// the 7th per-mission bump block — same absolute span every mission.
const BANK_LEN: usize = 0x2710;
/// The door-rect list the initializer stamps from: 45 records of stride
/// 0x10 (0x4dcae8..0x4dcdb8 — the §7j.21 boundary).
const RECT_COUNT: usize = 45;

#[path = "data/claim_rects.rs"]
mod claim_rects;

use claim_rects::RECTS;

fn editor_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR")
}

fn is_numbered_mission_stem(stem: &str) -> bool {
    let Some(number) = stem.strip_prefix("MISSION") else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

struct MissionFiles {
    zone_letter: char,
    zone_index: u8,
    mission: u8,
    tot: PathBuf,
}

/// Enumerate the shipped corpus (zones A..G, numbered MISSION stems) and
/// read each TOT header (u16 w, u16 h at offset 0 — FORMATS §4/§5;
/// verified against file sizes 4+16·w·h below).
fn shipped_missions() -> Option<Vec<MissionFiles>> {
    let root = editor_root();
    if !root.is_dir() {
        eprintln!("game-data corpus not found - skipping");
        return None;
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read EDITOR corpus directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();

    let mut missions = Vec::new();
    for dir in dirs {
        let Some(name) = dir.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        let Some(suffix) = name.strip_prefix("ZONE") else {
            continue;
        };
        let Some(letter) = suffix.chars().next() else {
            continue;
        };
        if suffix.len() != 1 || !letter.is_ascii_uppercase() {
            continue;
        }
        let Some(zone_index) = "ABCDEFG".find(letter) else {
            continue;
        };
        let mut stems: Vec<String> = fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read {name}: {error}"))
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.eq_ignore_ascii_case("tot"))
                    && entry
                        .path()
                        .file_stem()
                        .and_then(|v| v.to_str())
                        .is_some_and(is_numbered_mission_stem)
            })
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .map(str::to_owned)
            })
            .collect();
        stems.sort();
        for stem in stems {
            let mission: u8 = stem["MISSION".len()..]
                .parse()
                .expect("numbered mission stem parses");
            missions.push(MissionFiles {
                zone_letter: letter,
                zone_index: zone_index as u8 + 1,
                mission,
                tot: dir.join(format!("{stem}.TOT")),
            });
        }
    }
    Some(missions)
}

/// Independent transcription of the initializer (§7j.63/C): build the
/// 45-record rect bank from the pinned farm rows for (zone, mission),
/// then memset-0 the claim bank and stamp the ACTIVE PREFIX. Bounds are
/// NOT checked (the original trusts its data); every shipped mission is
/// proven in-bounds by `claim_bank_corpus_images_match_the_pinned_census`.
fn original_claim_image(zone: u8, mission: u8, map_w: u16, map_h: u16) -> Vec<u8> {
    image_from_table(&RECTS, zone, mission, map_w, map_h)
}

/// The claimed-tile count of an image (sensitivity helper).
fn claimed(bank: &[u8]) -> usize {
    bank.iter().filter(|&&b| b != 0).count()
}

/// Per-mission corpus identity pins: (zone letter, mission) → claimed
/// tiles. Transcribed from the §7j.63 census (interpreter + three
/// hand-verified cases); the total is 3049 across all 37 missions.
fn pinned_counts() -> [(&'static str, u8, usize); 37] {
    [
        ("A", 1, 59),
        ("B", 1, 226),
        ("B", 2, 17),
        ("B", 3, 110),
        ("B", 4, 141),
        ("B", 5, 76),
        ("B", 6, 0),
        ("B", 7, 0),
        ("C", 1, 508),
        ("C", 2, 100),
        ("C", 3, 70),
        ("C", 4, 21),
        ("C", 5, 201),
        ("C", 6, 0),
        ("C", 7, 0),
        ("D", 1, 84),
        ("D", 2, 287),
        ("D", 3, 98),
        ("D", 4, 91),
        ("D", 5, 68),
        ("D", 6, 0),
        ("D", 7, 0),
        ("E", 1, 148),
        ("E", 2, 82),
        ("E", 3, 58),
        ("E", 4, 50),
        ("E", 5, 65),
        ("E", 6, 0),
        ("E", 7, 0),
        ("F", 1, 132),
        ("F", 2, 83),
        ("F", 3, 131),
        ("F", 4, 47),
        ("F", 5, 76),
        ("F", 6, 0),
        ("F", 7, 0),
        ("G", 1, 20),
    ]
}

/// The S0 corpus mission (ZONEA/MISSION1): the exact claimed tile index
/// set under `line[y] = y*25` — 59 tiles (§7j.63/E).
const ZONEA_M1_TILES: [usize; 59] = [
    252, 253, 277, 278, 291, 292, 293, 294, 316, 317, 318, 319, 804, 829, 854, 891, 892, 893, 894,
    916, 917, 918, 919, 941, 942, 943, 944, 966, 967, 968, 969, 991, 992, 993, 994, 1109, 1110,
    1111, 1134, 1135, 1136, 1277, 1278, 1279, 1280, 1281, 1282, 1283, 1284, 1285, 1302, 1303, 1304,
    1305, 1306, 1307, 1308, 1309, 1310,
];

#[test]
fn claim_bank_corpus_images_match_the_pinned_census() {
    let Some(missions) = shipped_missions() else {
        return;
    };
    assert_eq!(missions.len(), 37, "37 shipped missions");
    let pins = pinned_counts();

    let mut total = 0usize;
    for (i, m) in missions.iter().enumerate() {
        let (letter, mission_pin, count_pin) = pins[i];
        assert_eq!(
            &m.zone_letter.to_string(),
            letter,
            "corpus order matches the pin table"
        );
        assert_eq!(m.mission, mission_pin, "corpus mission numbering");

        let tot = fs::read(&m.tot).unwrap_or_else(|e| panic!("read {}: {e}", m.tot.display()));
        assert_eq!(tot.len() % 4, 0, "TOT size multiple of 4");
        let map_w = u16::from_le_bytes([tot[0], tot[1]]);
        let map_h = u16::from_le_bytes([tot[2], tot[3]]);
        // The TOT grammar (FORMATS §4): 4-B header + 16·w·h payload.
        assert_eq!(tot.len(), 4 + 16 * map_w as usize * map_h as usize);
        // Zone A is 25×75; zone G is 100×25; the rest 100×100.
        let expected_dims = match m.zone_letter {
            'A' => (25, 75),
            'G' => (100, 25),
            _ => (100, 100),
        };
        assert_eq!((map_w, map_h), expected_dims, "map dims for {}", letter);
        // 10000 B arena vs w·h tiles: exact for 100×100, slack for the rest.
        assert!(
            (map_w as usize * map_h as usize) <= BANK_LEN,
            "map fits the 0x2710 arena"
        );

        let image = original_claim_image(m.zone_index, m.mission, map_w, map_h);
        assert_eq!(image.len(), BANK_LEN);
        let count = claimed(&image);
        assert_eq!(
            count, count_pin,
            "claimed-tile census for ZONE{}/M{}",
            m.zone_letter, m.mission
        );
        // Every byte is 0 or 1 (the stamp writes literal 1; the memset 0).
        assert!(image.iter().all(|&b| b <= 1));
        total += count;
    }
    assert_eq!(total, 3049, "total claimed tiles across the corpus");
}

#[test]
fn zonea_m1_image_is_the_exact_pinned_tile_set() {
    let image = original_claim_image(1, 1, 25, 75);
    let live: Vec<usize> = image
        .iter()
        .enumerate()
        .filter(|(_, &b)| b != 0)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(live, ZONEA_M1_TILES.to_vec());
    // The set is tile-disjoint (no rect overlap on ZONEA/M1): every byte
    // is written at most once — 59 == 59 distinct indices by construction.
    assert_eq!(live.len(), ZONEA_M1_TILES.len());
}

#[test]
fn missions_without_a_filler_case_stay_all_zero() {
    // Zones B..F missions 6..7: the mission tables bound at 5, the whole
    // filler skips, and the bank is the bare memset-0 (§7j.63/E).
    for zone in 2..=6u8 {
        for mission in [6u8, 7u8] {
            let image = original_claim_image(zone, mission, 100, 100);
            assert!(
                image.iter().all(|&b| b == 0),
                "ZONE{zone}/M{mission} must be all-zero"
            );
        }
    }
    // Zone A exists only as mission 1; a hypothetical A/M2 in the SP
    // table writes nothing (the case gates mission == 1).
    let a2 = original_claim_image(1, 2, 25, 75);
    assert!(a2.iter().all(|&b| b == 0));
}

/// Sensitivity proof for the oracle (temporary in-memory mutations; the
/// shipped corpus is never touched — the pinned table is queried through
/// a mutated copy here).
#[test]
fn oracle_is_sensitive_to_rect_content_and_the_prefix_rule() {
    // Baseline: ZONEA/M1 with the pinned table.
    let base = original_claim_image(1, 1, 25, 75);
    assert_eq!(claimed(&base), 59);

    // (1) Widen record 0 (w 9 -> 10): exactly the h=2 tiles of the new
    // column (x0+9 = 11) are added — rows y0=51..52, tiles 51·25+11 and
    // 52·25+11.
    let widened = with_rects_modified(|row| {
        if row.0 == 1 && row.1 == 1 && row.2 == 0 {
            (row.0, row.1, row.2, row.3, row.4, row.5, row.6 + 1, row.7)
        } else {
            row
        }
    })(1, 1, 25, 75);
    assert_eq!(claimed(&widened), 61);
    assert!(widened[51 * 25 + 11] == 1 && widened[52 * 25 + 11] == 1);
    assert_eq!(widened[50 * 25 + 11], 0, "the new column stops at h rows");

    // (2) The prefix rule: deactivating a MIDDLE record truncates the
    // walk — every later record's tiles vanish even though their bytes
    // are still "written" to the rect bank. Deactivate rec3 (state->0):
    // rec3 (10 tiles) + rec4 (3) + rec5 (4) + rec6 (8) disappear.
    let truncated = with_rects_modified(|row| {
        if row.0 == 1 && row.1 == 1 && row.2 == 3 {
            (row.0, row.1, row.2, 0, row.4, row.5, row.6, row.7)
        } else {
            row
        }
    })(1, 1, 25, 75);
    assert_eq!(claimed(&truncated), 59 - 10 - 3 - 4 - 8);

    // (3) The stamp arithmetic is row-major with line[y] = y*w: moving
    // rec4 (w=1, h=3) one row down shifts its three tiles by exactly
    // map_w each.
    let moved = with_rects_modified(|row| {
        if row.0 == 1 && row.1 == 1 && row.2 == 4 {
            (row.0, row.1, row.2, row.3, row.4, row.5 + 1, row.6, row.7)
        } else {
            row
        }
    })(1, 1, 25, 75);
    assert_eq!(claimed(&moved), 59);
    for i in 0..3usize {
        assert_eq!(moved[(32 + 1 + i) * 25 + 4], 1);
        assert_eq!(base[(32 + i) * 25 + 4], 1);
    }

    // (4) The original has NO bounds checks (§7j.63/C): a rect pushed
    // past the map computes an index beyond w*h. Our transcription
    // mirrors the arithmetic; the shipped corpus never exercises it
    // (asserted by the census test), so this proof computes the index
    // without indexing the bank.
    let y0: usize = 75; // one row past the ZONEA map
    let tile = y0 * 25 + 2;
    assert!(tile >= 25 * 75 && tile < BANK_LEN, "in-arena but off-map");
    let y1: usize = 400; // far past even the 10000-B arena
    assert!(
        y1 * 25 + 2 >= BANK_LEN,
        "an unchecked original write would be out of the arena"
    );
}

/// Query the initializer with a temporarily mutated pinned table
/// (the mutation lives only in this test's copy).
fn with_rects_modified(
    mutate: impl Fn((u8, u8, u8, u16, u16, u16, u16, u16)) -> (u8, u8, u8, u16, u16, u16, u16, u16),
) -> impl Fn(u8, u8, u16, u16) -> Vec<u8> {
    move |zone, mission, w, h| {
        let mutated: Vec<_> = RECTS.iter().map(|&r| mutate(r)).collect();
        image_from_table(&mutated, zone, mission, w, h)
    }
}

/// The S0-11b seam's data module (crate::claim_rects) must stay
/// byte-identical to this test's own transcription copy — the
/// anti-drift pin between the production table and the oracle.
#[test]
fn promoted_rect_farm_is_byte_identical() {
    assert_eq!(RECTS, bedlam_core::claim_rects::RECTS);
}

/// The ACTUAL side (S0-11b): the engine's staged claim bank equals
/// the independent transcription for every shipped mission — the
/// row's parity closed both sides.
#[test]
fn claim_staging_matches_the_independent_image() {
    let Some(missions) = shipped_missions() else {
        return;
    };
    for m in &missions {
        let tot = fs::read(&m.tot).unwrap_or_else(|e| panic!("read {}: {e}", m.tot.display()));
        let map_w = u16::from_le_bytes([tot[0], tot[1]]);
        let map_h = u16::from_le_bytes([tot[2], tot[3]]);
        let expected = original_claim_image(m.zone_index, m.mission, map_w, map_h);

        let n = map_w as usize * map_h as usize;
        let terrain = bedlam_core::mission::Terrain::from_parts(
            map_w as i32,
            map_h as i32,
            vec![0u8; 8 * n],
            Vec::new(),
        )
        .expect("synthetic terrain of the TOT dims");
        let angles =
            bedlam_core::mission::AngleTable::from_thresholds(&[0u16; 64]).expect("thresholds");
        let mut sim = bedlam_core::mission::MissionSim::new(terrain, angles, 0);
        sim.stage_claim_bank(u32::from(m.zone_index), u32::from(m.mission));
        assert_eq!(
            sim.claim_bank(),
            &expected[..],
            "staged image for ZONE{}/M{}",
            m.zone_letter,
            m.mission
        );
    }
}

/// The table-parameterized core of `original_claim_image` (same
/// transcription, explicit table so mutations can inject copies).
fn image_from_table(
    table: &[(u8, u8, u8, u16, u16, u16, u16, u16)],
    zone: u8,
    mission: u8,
    map_w: u16,
    map_h: u16,
) -> Vec<u8> {
    let mut rects = vec![[0u16; 6]; RECT_COUNT];
    let mut written = [false; RECT_COUNT];
    for &(z, m, rec, state, x0, y0, w, h) in table {
        if z != zone || m != mission {
            continue;
        }
        let r = &mut rects[rec as usize];
        r[0] = state;
        r[1] = x0;
        r[2] = y0;
        r[3] = w;
        r[4] = h;
        written[rec as usize] = true;
    }
    let high = written.iter().position(|w| !w).unwrap_or(RECT_COUNT);
    assert!(!written[high..].iter().any(|w| *w));

    let mut bank = vec![0u8; BANK_LEN];
    let (map_w, map_h) = (map_w as usize, map_h as usize);
    for rect in &rects[..high] {
        if rect[0] == 0 {
            break;
        }
        for row in 0..rect[4] as usize {
            for col in 0..rect[3] as usize {
                let y = rect[2] as usize + row;
                let x = rect[1] as usize + col;
                let tile = y * map_w + x; // line[y] + x  (line = row starts)
                                          // Faithful to the original's UNCHECKED write for every
                                          // in-bounds cell (all shipped + mutated data); the guard
                                          // only keeps mutated queries from panicking — off-map
                                          // arithmetic is proven compute-only in sensitivity (4).
                if y < map_h && tile < BANK_LEN {
                    bank[tile] = 1;
                }
            }
        }
    }
    bank
}
