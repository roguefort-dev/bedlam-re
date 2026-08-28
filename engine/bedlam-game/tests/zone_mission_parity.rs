//! The per-zone/per-mission parity EVIDENCE suite (P5, docs/
//! P5-ZONE-GATES §1 acceptance shape) — the D178 ZONEA closure
//! (tests/zonea_mission1_parity.rs, §7) LIFTED to a parameterized
//! shape so the §1 criterion table is executable for ANY ledger
//! mission. The suite carries ONE `ZoneSpec` per CLOSED zone and
//! runs every closed zone's evidence on every run (a zone's
//! disposition unit APPENDS its spec — the closed set never loses
//! its executable evidence, and every `p5-zone-{b..f}` gate runs
//! this same suite): zone B (§8, D192) and zone C (§9, D193).
//! Missions 1–5 stage through the CAMPAIGN episode-slot seam (the
//! completion mask whose first-uncompleted sub is the mission),
//! missions 6–7 — the MP-only files — through the SELECT
//! write-pair seam (§7j.73/D183). Per criterion, this file's
//! machine-checkable half:
//!
//! | §1 | Criterion | This file's leg |
//! |----|-----------|-----------------|
//! | 1 | scripted flows complete crash-free | `zone_scripted_flows_complete_crash_free` — every closed zone's committed flows (zone B: S5/S5B/S5C at MISSION1; zone C: NONE — no committed .scen stages it, the generated battery IS the whole leg) PLUS a generated per-mission battery (boot→mission, passive steady-state, full-staging destroy+pickup+platforms+critters) for EVERY mission of EVERY closed zone: full declared frame budget, dump verifies, two-run byte identity |
//! | 2 | T1 rules vs RE | `zone_t1_rules_spot` (the shared selection/economy arithmetic + the per-zone/per-mission fetch chain, the zone-level CGR/BIN/LNK pin of D184) + `zone_anchor_ts_statics_rederived_from_tot` (the anchor TS/T0 statics re-derived INDEPENDENTLY from the TOT header bytes + the §7j.64 formula, never the engine's own output) |
//! | 3 | perceptual frame checks (T2, diagnostic band) | the machine band is the two-run anchor/frame byte identity of criterion 1 (identical transcripts at the key moments); thresholds + owner feel sign-off stay the operator diagnostic process, never a pixel-exact gate |
//! | 4 | differ structural spot-check | the structural contract inside criterion 1's decode (anchor frame 0, monotone frame_no, record count = declared budget + 1, the scenario id riding the header); the cross-channel differ itself is the differ_gate gate command (ZONEA dumps); not tick-complete by design (§0b) |
//! | 5 | cross-OS replay hash equality (our engine) | the two-run byte + chain-digest identity of criterion 1 is the local half; the cross-OS half is the ubuntu+windows CI matrix running the determinism/hash_fixture suites (the D181 channel, §7 criterion 5) |
//! | 6 | original SAVED/OPTIONS.BDL import read-only, bounds-checked, fuzzed | `zone_saved_import_seam_stages_the_shipped_campaign` — the FILE-LEVEL seam evidence (every zone's §-table cites it): the shipped SAVED.BDL slot 0 campaign IS ZONEB/MISSION1 (zone cell 2 — zone B's own staging leg, §8), empty slots reject loud, bounded deterministic fuzz Ok/Err only with every Ok staging an in-model slot (any zone) |
//! | 7 | DM carve-out | for zone B the carve-out is LOAD-BEARING: missions 6–7 ARE the MP-only files (§7j.73). The checked legs: the maps load (criterion 1's SELECT-seam flows stage them) and local SP semantics are correct (criteria 1–2). Full DM/netplay = future work, out of the parity exit |
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

// ---------------------------------------------------------------------
// The zone parameter (P5-ZONE-GATES §2 — the shipped-mission census)
// ---------------------------------------------------------------------

/// One zone's disposition evidence shape. The census drives the
/// parameter: zones A and G carry ONE mission, zones B..F carry
/// SEVEN (missions 1–5 campaign, 6–7 the MP-only SELECT files —
/// the grammar's `mission` key domains match §7j.73).
struct ZoneSpec {
    /// The zone letter (the scenario grammar `zone` key's value).
    letter: char,
    /// The zone's shipped missions (§2 census: B..F → 1..=7).
    missions: std::ops::RangeInclusive<u8>,
    /// Committed .scen flows already staged inside this zone at
    /// mission 1 (their deeper per-flow chains stay pinned by
    /// canonical_dump_gate; this suite re-runs them as the zone's
    /// criterion-1 leg on top of the generated battery).
    committed: &'static [&'static str],
    /// The zone's TOT map dims (§2, the census size arithmetic).
    dims: (u16, u16),
}

/// The CLOSED zones this suite carries disposition evidence for
/// (P5-ZONE-GATES §8 = B, D192 — the first 7-mission disposition;
/// §9 = C, D193 — the first PURE instantiation: no committed
/// flows, the generated per-mission battery is the whole
/// criterion-1 leg). A later zone's unit APPENDS its spec; the
/// tests below iterate the list and read nothing else zone-specific.
const ZONES: &[ZoneSpec] = &[
    ZoneSpec {
        letter: 'B',
        missions: 1..=7,
        committed: &["S5", "S5B", "S5C"],
        dims: (100, 100),
    },
    ZoneSpec {
        letter: 'C',
        missions: 1..=7,
        committed: &[],
        dims: (100, 100),
    },
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    ZONES.iter().all(|zone| {
        root()
            .join(format!("EDITOR/ZONE{}/MISSION1.TOT", zone.letter))
            .is_file()
    })
}

fn scen_path(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/diffharness/scenarios")
        .join(format!("{id}.scen"))
}

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

/// The 0-based zone index (the emitted `zone` watch value —
/// `mission_slot().0`, the E convention).
fn zone_index(zone: &ZoneSpec) -> i32 {
    i32::from(zone.letter as u8) - i32::from(b'A')
}

/// The DERIVED linear-mission-m expectation, re-derived from the
/// §7j.64/D formula (clamp(5·(zone−2) + mission − 1, 1, 26), zone
/// 1-based) — never the engine's own emitted cell.
fn linear_for(zone: &ZoneSpec, mission: u8) -> u32 {
    let z = i32::from(zone.letter as u8) - i32::from(b'A') + 1;
    (5 * (z - 2) + i32::from(mission) - 1).clamp(1, 26) as u32
}

/// The campaign completion mask whose first-uncompleted sub selects
/// exactly `mission` (§7j.73: `mission_number_for_mask` inverts it —
/// mask (1<<(m−1))−1: m=1 → 0, m=2 → 0b0001, … m=5 → 0b1111).
fn campaign_mask(mission: u8) -> u8 {
    assert!((1..=5).contains(&mission), "the campaign domain is 1..=5");
    (1u8 << (mission - 1)) - 1
}

// ---------------------------------------------------------------------
// §1 criterion 1 — all scripted flows complete without crashes
// (per mission: the generated battery; plus the committed flows)
// ---------------------------------------------------------------------

/// One generated per-mission flow of the battery.
struct FlowSpec {
    /// Id tag (rides the dump header's scenario id).
    tag: &'static str,
    tiers: &'static str,
    frames: u32,
    destroy: bool,
    pickup: bool,
    platforms: bool,
    critters: bool,
}

/// The per-mission battery (§7 criterion 1 executable for ANY
/// ledger mission): A = boot→mission (the S0 shape), B = passive
/// steady-state (the S1 shape), C = the FULL-STAGING run (the
/// mission's own destroy + pickup + platform + critter families —
/// the S4/S5/S7/S8 staging keys together).
const FLOWS: [FlowSpec; 3] = [
    FlowSpec {
        tag: "A",
        tiers: "T0,TS",
        frames: 2,
        destroy: false,
        pickup: false,
        platforms: false,
        critters: false,
    },
    FlowSpec {
        tag: "B",
        tiers: "T0",
        frames: 120,
        destroy: false,
        pickup: false,
        platforms: false,
        critters: false,
    },
    FlowSpec {
        tag: "C",
        tiers: "T0,T1,T2,T3,TS",
        frames: 48,
        destroy: true,
        pickup: true,
        platforms: true,
        critters: true,
    },
];

/// The generated scenario source for one (zone, mission, flow)
/// triple (grammar v1.8: the `mission` key selects the staging
/// seam — 1..=5 the campaign mask, 6..=7 the SELECT MP pair).
fn flow_source(zone: &ZoneSpec, mission: u8, flow: &FlowSpec) -> String {
    let mut s = format!(
        "# generated zone-parity flow: ZONE{L}/MISSION{m} (battery {tag} — the P5 §-table \
         criterion-1 leg; the staging seam is the grammar v1.8 mission key)\n\
         scenario = \"P5{L}M{m}{tag}\"\n\
         tiers = {tiers}\n\
         anchor = mission-start\n\
         frames = {frames}\n\
         zone = \"{L}\"\n\
         mission = {m}\n",
        L = zone.letter,
        m = mission,
        tag = flow.tag,
        tiers = flow.tiers,
        frames = flow.frames,
    );
    if flow.destroy {
        s.push_str("destroy = 1\n");
    }
    if flow.pickup {
        s.push_str("pickup = 1\n");
    }
    if flow.platforms {
        s.push_str("platforms = 1\n");
    }
    if flow.critters {
        s.push_str("critters = 1\n");
    }
    s
}

/// The criterion-1 contract for one completed run: full declared
/// budget, the dump verifies, and a re-run is byte-identical.
fn assert_flow_completes(id: &str, src: &str, expected_frames: u64) {
    let run = run_canonical(src, &root())
        .unwrap_or_else(|e| panic!("{id} did not complete crash-free: {e}"));
    assert_eq!(
        run.manifest.frame_count,
        expected_frames + 1, // + the anchor record
        "{id}: flow must run its full declared budget"
    );
    assert!(!run.bytes.is_empty(), "{id}: dump emitted");
    let rerun = run_canonical(src, &root()).unwrap_or_else(|e| panic!("{id} re-run failed: {e}"));
    assert_eq!(run.bytes, rerun.bytes, "{id}: two-run byte identity");
    assert_eq!(
        run.manifest.chain_digest, rerun.manifest.chain_digest,
        "{id}: chain digest identity"
    );
    let dump = decode_dump(&run.bytes).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(dump.header.channel, Channel::Engine);
    assert_eq!(dump.header.scenario, id);
    assert_eq!(dump.frames.len() as u64, expected_frames + 1);
    let nos: Vec<u64> = dump.frames.iter().map(|f| f.frame_no).collect();
    let mut sorted = nos.clone();
    sorted.sort_unstable();
    assert_eq!(nos, sorted, "{id}: frame_no monotone");
    assert_eq!(nos.first(), Some(&0), "{id}: anchor frame 0");
    eprintln!(
        "zone parity: {id} ok ({} records)",
        run.manifest.frame_count
    );
}

#[test]
fn zone_scripted_flows_complete_crash_free() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    for zone in ZONES {
        // The zone's committed flows (staged at mission 1; zone C
        // carries NONE — the battery below is its whole leg).
        for id in zone.committed {
            let src = fs::read_to_string(scen_path(id)).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_flow_completes(id, &src, declared_frames(id));
        }
        // The generated battery for EVERY shipped mission of the
        // zone: 1..=5 through the campaign episode-slot seam, 6..=7
        // through the SELECT MP write-pair seam (the grammar's
        // mission key).
        for mission in zone.missions.clone() {
            for flow in &FLOWS {
                let id = format!("P5{}M{}{}", zone.letter, mission, flow.tag);
                assert_flow_completes(
                    &id,
                    &flow_source(zone, mission, flow),
                    u64::from(flow.frames),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// §1 criteria 2+4 — the anchor TS/T0 statics re-derived from the
// zone's TOT headers + the structural spot check
// ---------------------------------------------------------------------

#[test]
fn zone_anchor_ts_statics_rederived_from_tot() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    for zone in ZONES {
        for mission in zone.missions.clone() {
            // Independent re-derivation: the map dims straight from the
            // TOT header bytes (u16 w + u16 h, FORMATS-MISSION §2) and
            // the §2 size arithmetic 4 + 16·w·h.
            let tot =
                fs::read(root().join(format!("EDITOR/ZONE{}/MISSION{mission}.TOT", zone.letter)))
                    .unwrap_or_else(|e| panic!("MISSION{mission}.TOT readable: {e}"));
            let w = u16::from_le_bytes([tot[0], tot[1]]);
            let h = u16::from_le_bytes([tot[2], tot[3]]);
            assert_eq!(
                (w, h),
                zone.dims,
                "ZONE{}/MISSION{mission}: TOT header dims (the §2 zone table)",
                zone.letter
            );
            assert_eq!(
                tot.len(),
                (4 + 16 * u32::from(w) * u32::from(h)) as usize,
                "ZONE{}/MISSION{mission}: 4 + 16·w·h",
                zone.letter
            );

            // The anchor frame's TS/T0 statics against values derived
            // from those bytes + the §7j.64 formula — never the engine's
            // own loader output.
            let src = flow_source(zone, mission, &FLOWS[0]);
            let run = run_canonical(&src, &root())
                .unwrap_or_else(|e| panic!("ZONE{}/MISSION{mission} boot flow: {e}", zone.letter));
            let dump = decode_dump(&run.bytes).expect("dump verifies");
            let anchor = &dump.frames[0];
            let map_wh = [u32::from(w).to_le_bytes(), u32::from(h).to_le_bytes()].concat();
            assert_eq!(
                anchor.watch("static-map-wh"),
                Some(&map_wh[..]),
                "ZONE{}/MISSION{mission}: anchor map-wh == the TOT header dims",
                zone.letter
            );
            for later in &dump.frames[1..] {
                assert!(later.watch("static-map-wh").is_none());
            }
            // Fresh-campaign T1 scalars (§7j.64/D154): money
            // 4000−500·difficulty(1) = 3500, difficulty 1 (the 0x41c14a
            // boot write), zone the 0-based index, mode 0 (SP), mission
            // the within-zone number, linear the derived clamp.
            assert_eq!(anchor.watch("money"), Some(&3500u32.to_le_bytes()[..]));
            assert_eq!(anchor.watch("difficulty"), Some(&1u32.to_le_bytes()[..]));
            assert_eq!(
                anchor.watch("zone"),
                Some(&(zone_index(zone) as u32).to_le_bytes()[..])
            );
            assert_eq!(anchor.watch("mode"), Some(&0u32.to_le_bytes()[..]));
            assert_eq!(
                anchor.watch("mission"),
                Some(&u32::from(mission).to_le_bytes()[..])
            );
            assert_eq!(
                anchor.watch("linear-mission-m"),
                Some(&linear_for(zone, mission).to_le_bytes()[..]),
                "ZONE{}/MISSION{mission}: linear == clamp(5·(zone−2)+m−1, 1, 26)",
                zone.letter
            );
        }
    }
}

// ---------------------------------------------------------------------
// §1 criterion 2 — the T1 rules spot table (EXW-anchored selection/
// economy arithmetic + the per-mission fetch chain + the seams)
// ---------------------------------------------------------------------

#[test]
fn zone_t1_rules_spot() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    // The shared selection arithmetic the zone's staging consumes
    // (the B2 @0x81d9a full-mask table + the first-unset-bit rule).
    assert_eq!(FULL_MASK[0], 0);
    assert_eq!(FULL_MASK[1], 1);
    for subs in FULL_MASK.iter().take(usize::from(MAX_STAGE) + 1).skip(2) {
        assert_eq!(*subs, 15);
    }
    assert_eq!(mission_number_for_mask(0), 1);
    assert_eq!(mission_number_for_mask(0b0001), 2);
    assert_eq!(mission_number_for_mask(0b0011), 3);
    assert_eq!(mission_number_for_mask(0b0111), 4);
    assert_eq!(mission_number_for_mask(0b1111), 5);
    // Saturated at 5 — the SP SELECT domain (§7j.73): the campaign
    // path can never name an MP file.
    assert_eq!(mission_number_for_mask(0b11111), 5);

    // Campaign economy: the §7j.64/C name-entry seed 4000 − 500·d.
    assert_eq!(bedlam_game::menu::start_score(0), 4000);
    assert_eq!(bedlam_game::menu::start_score(1), 3500);
    assert_eq!(bedlam_game::menu::start_score(2), 3000);

    // The per-zone/per-mission fetch chain: 25 names, the
    // mission-level path-1 trio + MRK, the ZONE-LEVEL path-2
    // CGR/BIN/LNK (the D184 no-swap pin — even where a
    // mission-number variant bank ships, MISSION6.BIN is
    // runtime-dead editor residue), and the GAMEGFX staging tail.
    for zone in ZONES {
        for mission in zone.missions.clone() {
            let names = mission_asset_names(zone_index(zone), i32::from(mission));
            assert_eq!(names.len(), 25, "MISSION{mission}: the 25-name set");
            let per = format!("ZONE{}/MISSION{mission}", zone.letter);
            let zone_file = format!("ZONE{}/MISSION{}", zone.letter, zone.letter);
            assert_eq!(names[0], format!("{per}.TOT"));
            assert_eq!(names[1], format!("{per}.DAT"));
            assert_eq!(names[2], format!("{per}.PAD"));
            assert_eq!(names[3], format!("{zone_file}.CGR"));
            assert_eq!(names[4], format!("{zone_file}.BIN"));
            assert_eq!(names[5], format!("{zone_file}.LNK"));
            assert!(names.contains(&"GAMEPAL.PAL".to_string()));
            assert!(names.contains(&"SINTABLE.BIN".to_string()));
            let mrks: Vec<_> = names.iter().filter(|n| n.ends_with(".MRK")).collect();
            assert_eq!(mrks, vec![&format!("{per}.MRK")], "one MRK, mission-level");
        }

        // The staging seams themselves (the host arms the flows ride):
        // campaigns 1..=5 stage through the completion mask; 6..=7
        // through the SELECT write pair; the pair's domain rejects
        // loud; campaign staging CLEARS the pair (§7j.73).
        let stage = u8::try_from(zone_index(zone) + 1).expect("stage");
        for mission in 1u8..=5 {
            let mut host = host();
            assert!(
                host.stage_episode_slot(stage, campaign_mask(mission)),
                "stage {stage} / mask {}",
                campaign_mask(mission)
            );
            assert_eq!(
                host.mission_slot(),
                (zone_index(zone), i32::from(mission)),
                "the campaign mask selects ZONE{}/MISSION{mission}",
                zone.letter
            );
            assert_eq!(
                host.mission_asset_names()[0],
                format!("ZONE{}/MISSION{mission}.TOT", zone.letter)
            );
        }
        let zone_cell = stage; // the SELECT write arm's 1-based zone cell
        for (cell, file) in [(1u8, 6u8), (2, 7)] {
            let mut host = host();
            assert!(host.stage_select_mission(zone_cell, cell));
            assert_eq!(
                host.mission_slot(),
                (zone_index(zone), i32::from(file)),
                "the SELECT pair +5 selects ZONE{}/MISSION{file}",
                zone.letter
            );
            assert_eq!(
                host.mission_asset_names()[0],
                format!("ZONE{}/MISSION{file}.TOT", zone.letter)
            );
        }
        {
            // The write-arm domain (0x43edc2..0x43ee43): zone 2..=6,
            // mission 1..=2 — everything else rejects loud.
            let mut host = host();
            for bad in [(1u8, 1u8), (7, 1), (0, 1), (2, 0), (2, 3)] {
                assert!(!host.stage_select_mission(bad.0, bad.1), "{bad:?} rejects");
            }
            // Campaign staging clears the staged pair: the slot returns
            // to the campaign arithmetic.
            assert!(host.stage_select_mission(zone_cell, 2));
            assert!(host.stage_episode_slot(stage, 0));
            assert_eq!(host.mission_slot(), (zone_index(zone), 1));
        }
    }
}

// ---------------------------------------------------------------------
// §1 criterion 6 — the original SAVED/OPTIONS.BDL import seam over
// the REAL shipped files (RE-EXW-SIM §7j.70). FILE-LEVEL evidence:
// every zone's §-table cites these two tests. The shipped slot-0
// campaign IS ZONEB/MISSION1 (zone cell 2 → ZONEB/MISSION1 — zone
// B's own staging leg, §8; no other zone's campaign can ride the
// shipped file, so the zone-C/… legs ride the fuzz's in-model
// staging assert below + the campaign-mask staging of criterion 2).
// ---------------------------------------------------------------------

fn palette() -> [bedlam_render::Vga6; 256] {
    [[0, 0, 0]; 256]
}

fn host() -> GameHost {
    GameHost::new(&GameConfig::default(), &SimConfig::default(), palette())
}

#[test]
fn zone_saved_import_seam_stages_the_shipped_campaign() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let saved = fs::read(root().join("SAVED.BDL")).expect("shipped SAVED.BDL");
    assert_eq!(saved.len(), 900, "5 × 180 (the §7j.70 stride arithmetic)");

    // Slot 0 is the live campaign ("PLAYER"): zone cell 2 →
    // ZONEB/MISSION1 — the import's staging is zone B's disposition
    // seam evidence (§8); hardcoded to zone B, never derived from
    // the zone list (the shipped save is what it is).
    let zoneb = i32::from(b'B') - i32::from(b'A');
    let mut host = host();
    let import = host.import_saved_slot(&saved, 0).expect("slot 0 imports");
    assert_eq!(import.stage, 2);
    assert_eq!(import.mask, 0);
    assert_eq!(host.mission_slot(), (zoneb, 1));
    assert_eq!(host.mission_asset_names()[0], "ZONEB/MISSION1.TOT");

    // The shipped EMPTY rows reject loud and stage NOTHING (the
    // previous import's slot survives an empty-slot attempt).
    for slot in 1..5 {
        assert!(matches!(
            host.import_saved_slot(&saved, slot),
            Err(GameError::SaveSlotEmpty { slot: s }) if s == slot
        ));
    }
    assert_eq!(host.mission_slot(), (zoneb, 1));

    // OPTIONS.BDL: the typed import over the real 41-B record.
    let options = fs::read(root().join("OPTIONS.BDL")).expect("shipped OPTIONS.BDL");
    assert_eq!(options.len(), 41);
    let parsed = bedlam_assets::bdl::parse_options_bdl(&options).expect("options parse");
    let config = GameConfig::from_options(&parsed).expect("options validate");
    assert_eq!(config.volume, 75);
    assert_eq!(config.player_name, "Player..");
}

/// Bounded deterministic fuzz of the import seam over the REAL file
/// bytes: the full header-window bit-flip sweep per slot — Ok/Err
/// only, never a panic; every Ok stages a slot inside the model.
#[test]
fn zone_saved_import_seam_fuzz_bounded() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let saved = fs::read(root().join("SAVED.BDL")).expect("shipped SAVED.BDL");
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
                // In-model imports stage to their own slot (the
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
    // Truncation + size attacks: Ok/Err only, never a panic.
    let mut seed = 0xC0FFEE_u64;
    let mut byte = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(0x42);
        (seed >> 33) as u8
    };
    for len in [0usize, 1, 0x0C, 0x10, 0x18, 179, 180, 181, 899, 901, 2048] {
        let d: Vec<u8> = (0..len).map(|_| byte()).collect();
        let mut host = host();
        let _ = host.import_saved_slot(&d, 2);
    }
    for slot in 0..5usize {
        let d: Vec<u8> = (0..900).map(|_| byte()).collect();
        let mut host = host();
        let _ = host.import_saved_slot(&d, slot);
    }
}
