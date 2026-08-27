//! P5 all-37-mission READ-ONLY load census (docs/P5-ZONE-GATES.md §6,
//! unit `p5-mission-load-census`) — the per-mission/per-zone GAP TABLE's
//! executable half.
//!
//! For every one of the 37 ledger missions
//! (docs/P5-MISSION-LEDGER.toml — A:1, B..F:1-7 each, G:1) the census
//! drives the REAL engine load seams against the mission's runtime file
//! family (FORMATS-MISSION §0.2: .TOT .DAT .CGR .BIN .MIN .LNK/.LNG
//! .PAD .NME .TRT .POS .BDG .MRK) READ-ONLY from game-data/BEDLAM:
//!
//! 1. LOAD — `GameHost::load_mission` through the episode-slot seam
//!    (`stage_episode_slot` + the canonical 25-name fetch order) for
//!    the SP campaign missions 1-5, and through the SELECT
//!    mission-choice seam (`stage_select_mission`, RE-EXW-SIM
//!    §7j.73) for the MP-only missions 6-7 of zones B-F (the +5
//!    file-offset pair); `MissionScene::stage` + the claim bank
//!    directly only if no seam reaches the mission (defensive).
//!    Both host paths are the bedlam-render mission-view load seam
//!    (`MissionView::from_mission_bytes`) plus the bedlam-core loader
//!    family (Terrain/AngleTable/MapOverlay/MRK spawns) in one.
//! 2. DESTROY family — the mission's own .BDG/.POS/.TRT staged through
//!    `stage_destroy_family` (the canonical `destroy = 1` arm verbatim).
//! 3. PICKUP surface — the .TOT volume through `stage_pickup_surface` +
//!    the hazard stamper (the `pickup = 1` arm).
//! 4. CRITTERS — the .NME through `stage_critters` (the `critters = 1`
//!    arm); a file hosting an unmodeled section is a NAMED GAP, not a
//!    failure.
//! 5. PARSERS — every runtime file through its bedlam-assets parser
//!    (grid16/grid8/pad/mrk/pos/trt/nme/bdg + the zone min/lnk/lng/
//!    cgr/bin family).
//! 6. FRAMES — a short scripted run (FSM Boot→Mission + frames on the
//!    host path; activate + tick + present on the direct path). Panics
//!    here are CAUGHT and recorded as gaps — the census reports them
//!    instead of dying.
//!
//! PIN DISCIPLINE (the fingerprint rule, D28): the pinned table below
//! is the §6 doc table's machine form. It moves only when a loader
//! deliberately changes, and then it is re-baselined with a commit
//! message saying why. Skips (not fails) when the corpus is absent.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use bedlam_assets as assets;
use bedlam_core::destroy::{ObjectTypeTable, OBJECT_INSTANCE_SLOTS};
use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;
use bedlam_game::{
    mission_asset_names, mission_number_for_mask, ByteSource, GameConfig, GameHost, MissionScene,
    Scene, SceneAction, FULL_MASK,
};
// The canonical emitter module, vendored for its `MissionSource`
// (the EDITOR/GAMEGFX/SOUND/MIDI prefix resolution) and
// `linear_mission_m` (the §7j.64/D derived cell) — the single source
// of truth. `dead_code` is allowed HERE only because this census uses
// the two items above; tests/canonical_dump_gate.rs exercises the
// rest of the module and keeps it warning-clean.
#[path = "../examples/parity_harness/canonical.rs"]
#[allow(dead_code)]
mod canonical;

use canonical::{linear_mission_m, MissionSource};

/// The canonical runner's subtick budget (60 Hz × 4).
const DT_SUBTICKS: u32 = 4;
/// Scripted frames the census runs past mission entry per mission.
const CENSUS_FRAMES: u32 = 8;
/// The ledger's zone shape (docs/P5-MISSION-LEDGER.toml; the scaffold
/// checker re-derives exactly this from the corpus).
const ZONE_SHAPE: [(char, u32); 7] = [
    ('A', 1),
    ('B', 7),
    ('C', 7),
    ('D', 7),
    ('E', 7),
    ('F', 7),
    ('G', 1),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR/ZONEA/MISSION1.TOT").is_file()
}

/// One census row per mission (the §6 gap-table line, in full detail).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    id: String,
    /// TOT header dims.
    dims: String,
    /// "host" | "direct" | "FAIL <why>"
    load: String,
    /// "ok" | the named destroy-family refusal
    destroy: String,
    /// "ok" | the named pickup refusal
    pickup: String,
    /// "ok" | the named critter gap
    critters: String,
    /// "ok" | "ext:err, ..." for every parser refusal
    parsers: String,
    /// "ok" | "panic@<where>"
    frames: String,
    /// Named gaps (empty = clean).
    gaps: Vec<String>,
}

impl Row {
    /// The compact §6 summary form.
    fn summary(&self) -> String {
        if self.load.starts_with("FAIL") {
            format!("CANNOT-LOAD {}", self.load)
        } else if self.gaps.is_empty() {
            format!("{}:clean", self.load)
        } else {
            format!("{}:gaps {}", self.load, self.gaps.join("; "))
        }
    }
}

/// Stage the mission directly (missions the episode slot cannot reach):
/// `MissionScene::stage` + the claim bank — the `GameHost::load_mission`
/// body verbatim, minus the host state.
fn stage_direct(zone: i32, mission_no: i32, b: &[Vec<u8>]) -> Result<MissionScene, String> {
    let maptran: Vec<&[u8]> = b[13..21].iter().map(Vec::as_slice).collect();
    let mut scene = MissionScene::stage(
        &b[0],
        &b[1],
        &b[2],
        &b[3],
        &b[11],
        &b[4],
        &b[5],
        &b[6],
        &b[7],
        &b[8],
        &b[9],
        &b[10],
        &b[22],
        &b[23],
        &b[24],
        &b[12],
        &b[21],
        &maptran,
        zone,
        None,
        &[],
    )
    .map_err(|e| e.to_string())?;
    scene
        .sim_mut()
        .stage_claim_bank((zone + 1) as u32, mission_no as u32);
    Ok(scene)
}

/// The destroy-family staging — the canonical `destroy = 1` arm
/// verbatim (BDG table + POS length + TRT tier grammar + the staging
/// call + the mission-number cell).
fn stage_destroy(
    scene: &mut MissionScene,
    bdg: &[u8],
    pos: &[u8],
    trt: &[u8],
    zone: i32,
    mission: u32,
) -> Result<(), String> {
    let table = ObjectTypeTable::from_bdg_bytes(bdg).ok_or("BDG desync (FORMATS §16)")?;
    if pos.len() != 16 * OBJECT_INSTANCE_SLOTS {
        return Err(format!("POS is {} B (want 16*2000)", pos.len()));
    }
    let linear = linear_mission_m(zone as u32 + 1, mission);
    if bedlam_core::destroy::parse_trt(trt, linear).is_none() {
        return Err("TRT desync (FORMATS §14)".into());
    }
    if !scene
        .sim_mut()
        .stage_destroy_family(&table, pos, trt, (zone + 1) as u32, linear)
    {
        return Err("staging rejected (terrain not sized / POS length)".into());
    }
    scene.sim_mut().set_mission_no(mission);
    Ok(())
}

/// The pickup-surface arm — the canonical `pickup = 1` staging.
fn stage_pickup(scene: &mut MissionScene, tot: &[u8], zone: i32) -> Result<(), String> {
    if !scene.sim_mut().stage_pickup_surface(tot, (zone + 1) as u32) {
        return Err("TOT volume desynced from the staged terrain".into());
    }
    scene.sim_mut().stamp_hazard_words();
    Ok(())
}

/// Name the .NME sections `stage_critters` refuses (every non-empty
/// section other than Wanderers/MixedState5/SeekSteppers/
/// BallisticState6 — the kind-1 landing §7j.71 added Wanderers and
/// the kind-6 landing §7j.72 added BallisticState6 to the modeled
/// set).
fn unmodeled_nme_sections(nme: &[u8]) -> Vec<String> {
    let parsed = assets::misc::parse_nme(nme);
    parsed
        .sections
        .iter()
        .filter_map(|s| match s {
            assets::misc::NmeSection::Section { kind, count, .. } => {
                let modeled = matches!(
                    kind,
                    assets::misc::NmeSectionKind::Wanderers
                        | assets::misc::NmeSectionKind::MixedState5
                        | assets::misc::NmeSectionKind::SeekSteppers
                        | assets::misc::NmeSectionKind::BallisticState6
                );
                (*count != 0 && !modeled).then(|| format!("{kind:?}x{count}"))
            }
        })
        .collect()
}

/// The bedlam-assets parser family over the runtime file set. Returns
/// "ok" or one "ext:err" per refusal.
#[allow(clippy::too_many_arguments)]
fn parser_family(
    tot: &[u8],
    dat: &[u8],
    pad: &[u8],
    mrk: &[u8],
    pos: &[u8],
    trt: &[u8],
    bdg: &[u8],
    nme: &[u8],
    min: &[u8],
    lnk: &[u8],
    lng: &[u8],
    cgr: &[u8],
    bin: &[u8],
) -> String {
    let mut errs: Vec<String> = Vec::new();
    let mut check = |ext: &str, r: Result<(), String>| {
        if let Err(e) = r {
            errs.push(format!("{ext}:{e}"));
        }
    };
    // .NME/.BDG never fail (they walk with zero-fill semantics).
    let _ = (assets::misc::parse_nme(nme), assets::misc::parse_bdg(bdg));
    check("tot", parse_err(assets::mission::parse_grid16(tot)));
    check("dat", parse_err(assets::mission::parse_grid8(dat)));
    check("pad", parse_err(assets::mission::parse_pad(pad)));
    check("mrk", parse_err(assets::mission::parse_mrk(mrk)));
    check("pos", parse_err(assets::mission::parse_pos(pos)));
    check("trt", parse_err(assets::mission::parse_trt(trt)));
    check("min", parse_err(assets::misc::parse_min(min)));
    check("lnk", parse_err(assets::misc::parse_lnk_lng(lnk)));
    check("lng", parse_err(assets::misc::parse_lnk_lng(lng)));
    check("cgr", parse_err(assets::tiles::parse_cgr_tiles(cgr)));
    check("bin", parse_err(assets::sprites::parse_bin_images(bin)));
    if errs.is_empty() {
        "ok".into()
    } else {
        errs.join(", ")
    }
}

/// Erase the parser's payload type into a plain string error.
fn parse_err<T>(r: Result<T, assets::AssetsError>) -> Result<(), String> {
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// The FSM-driven frame probe: Boot hold → Title → Brief → Select →
/// Mission, then the activation frame + CENSUS_FRAMES frames (the
/// canonical runner's shape, null input).
fn frames_probe_host(host: &mut GameHost) -> Result<(), String> {
    let null = InputFrame::default();
    let mut guard = 0u32;
    while host.scene() == Scene::Boot {
        host.pump_frame(DT_SUBTICKS, &null);
        guard += 1;
        if guard > 600 {
            return Err("boot hold never ended".into());
        }
    }
    host.apply(SceneAction::Advance); // Title -> Brief
    host.apply(SceneAction::Advance); // Brief -> Select
    host.apply(SceneAction::Advance); // Select -> Mission
    if host.scene() != Scene::Mission {
        return Err(format!("FSM did not reach Mission ({:?})", host.scene()));
    }
    for _ in 0..=CENSUS_FRAMES {
        host.pump_frame(DT_SUBTICKS, &null);
    }
    Ok(())
}

/// The direct frame probe: activate + tick + present.
fn frames_probe_scene(scene: &mut MissionScene) -> Result<(), String> {
    scene.activate();
    for _ in 0..CENSUS_FRAMES {
        scene.tick(&InputFrame::default());
        scene.present();
    }
    Ok(())
}

fn census_mission(letter: char, mission: u32) -> Row {
    let zone = i32::from(letter as u8 - b'A');
    let id = format!("ZONE{letter}-MISSION{mission}");
    let mut gaps: Vec<String> = Vec::new();
    let mut src = MissionSource::new(root());
    let zone_dir = format!("ZONE{letter}");
    let per_mission = format!("{zone_dir}/MISSION{mission}");

    // The canonical 25-name family (fetch-order pinned by the runner).
    let names = mission_asset_names(zone, mission as i32);
    let mut b: Vec<Vec<u8>> = Vec::with_capacity(names.len());
    for n in &names {
        b.push(src.load(n).unwrap_or_default());
    }
    // The runtime family members the 25-name fetch does not carry,
    // plus the language-gate alternate.
    let nme = src.load(&format!("{per_mission}.NME")).unwrap_or_default();
    let trt = src.load(&format!("{per_mission}.TRT")).unwrap_or_default();
    let pos = src.load(&format!("{per_mission}.POS")).unwrap_or_default();
    let bdg = src.load(&format!("{per_mission}.BDG")).unwrap_or_default();
    let lng = src
        .load(&format!("{zone_dir}/MISSION{letter}.LNG"))
        .unwrap_or_default();

    let dims = if b[0].len() >= 4 {
        format!(
            "{}x{}",
            u16::from_le_bytes([b[0][0], b[0][1]]),
            u16::from_le_bytes([b[0][2], b[0][3]])
        )
    } else {
        "?".into()
    };

    // The parser family over everything fetched (missing = empty bytes
    // surface as parser refusals — that IS the census finding).
    let parsers = parser_family(
        &b[0], &b[1], &b[2], &b[11], &pos, &trt, &bdg, &nme, &b[21], &b[5], &lng, &b[3], &b[4],
    );
    if parsers != "ok" {
        gaps.push(format!("parser {parsers}"));
    }

    // The load-seam reachability — THREE paths:
    // - `host`: the episode-slot seam (stage + the completion mask
    //   the mission_number_for_mask derivation covers = the SP
    //   campaign missions 1..5);
    // - `select`: the SELECT mission-choice seam (RE-EXW-SIM §7j.73,
    //   the census G1 class CLOSED): the MP write pair
    //   {zone 2..=6, mission 1..=2} + the load-time +5 (0x4467df)
    //   stages the MP-only MISSION6/MISSION7 files — the missions
    //   no stage mask can express (they are not campaign subs);
    // - `direct`: `MissionScene::stage` + the claim bank (the
    //   `load_mission` body verbatim) — the defensive fallback for
    //   a mission no seam reaches (no ledger row uses it today).
    let stage_no = zone as u8 + 1;
    let mask = ((1u32 << (mission - 1)) - 1) as u8;
    let slot_reaches = mission_number_for_mask(mask) as u32 == mission
        && mask & !FULL_MASK[stage_no as usize] == 0;
    let select_pair = ((2..=6).contains(&stage_no) && (6..=7).contains(&mission))
        .then_some((stage_no, (mission - 5) as u8));

    let mut row = Row {
        id,
        dims,
        load: String::new(),
        destroy: String::new(),
        pickup: String::new(),
        critters: String::new(),
        parsers,
        frames: String::new(),
        gaps,
    };

    let config = match GameConfig::load(&mut src) {
        Ok(c) => c,
        Err(e) => {
            row.load = format!("FAIL options: {e}");
            return row;
        }
    };

    // The per-mission staging closure shared by both load paths:
    // campaign seed → destroy → pickup → critters (the canonical
    // ordering), all through the MissionScene seam.
    #[allow(clippy::too_many_arguments)]
    fn stage_families(
        scene: &mut MissionScene,
        bdg: &[u8],
        pos: &[u8],
        trt: &[u8],
        tot: &[u8],
        nme: &[u8],
        zone: i32,
        mission: u32,
        gaps: &mut Vec<String>,
        destroy: &mut String,
        pickup: &mut String,
        critters: &mut String,
    ) {
        // The campaign seed (§7j.64/C: money := 4000−500·d, d=1).
        let money = bedlam_game::menu::start_score(1);
        scene.set_campaign(0, money);
        scene.sim_mut().set_difficulty(1);
        *destroy = match stage_destroy(scene, bdg, pos, trt, zone, mission) {
            Ok(()) => "ok".into(),
            Err(e) => {
                gaps.push(format!("destroy {e}"));
                e
            }
        };
        *pickup = match stage_pickup(scene, tot, zone) {
            Ok(()) => "ok".into(),
            Err(e) => {
                gaps.push(format!("pickup {e}"));
                e
            }
        };
        *critters = match scene.sim_mut().stage_critters(nme, 1) {
            Some(_) => {
                scene.sim_mut().arm_critter_family();
                "ok".into()
            }
            None => {
                let sections = unmodeled_nme_sections(nme);
                let what = if sections.is_empty() {
                    "unmodeled content".to_string()
                } else {
                    sections.join(",")
                };
                gaps.push(format!("critters refused ({what})"));
                format!("refused({what})")
            }
        };
    }

    let seam_staged: Option<&'static str> = if slot_reaches {
        Some("host")
    } else if select_pair.is_some() {
        Some("select")
    } else {
        None
    };

    if let Some(label) = seam_staged {
        let mut host = GameHost::new(&config, &SimConfig::default(), [[0u8, 0, 0]; 256]);
        let staged_ok = if slot_reaches {
            host.stage_episode_slot(stage_no, mask)
        } else {
            // §7j.73: the MP write pair; the +5 turns mission 1..2
            // into the MISSION6/MISSION7 file numbers.
            let (zone_set, mp_mission) = select_pair.expect("select path has the pair");
            host.stage_select_mission(zone_set, mp_mission)
        };
        if !staged_ok {
            row.load = format!("FAIL seam-staging rejected (stage {stage_no}, mask {mask})");
            return row;
        }
        let maptran: Vec<&[u8]> = b[13..21].iter().map(Vec::as_slice).collect();
        let loaded = host.load_mission(
            &b[0],
            &b[1],
            &b[2],
            &b[3],
            &b[4],
            &b[5],
            &b[6],
            &b[7],
            &b[8],
            &b[9],
            &b[10],
            &b[11],
            &b[23],
            &b[24],
            &b[12],
            &maptran,
            &b[21],
            &b[22],
            None,
            &[],
        );
        match loaded {
            Err(e) => {
                row.load = format!("FAIL {e}");
            }
            Ok(()) => {
                row.load = label.into();
                {
                    let scene = host.mission_mut().expect("mission staged");
                    stage_families(
                        scene,
                        &bdg,
                        &pos,
                        &trt,
                        &b[0],
                        &nme,
                        zone,
                        mission,
                        &mut row.gaps,
                        &mut row.destroy,
                        &mut row.pickup,
                        &mut row.critters,
                    );
                }
                let probe = catch_unwind(AssertUnwindSafe(|| frames_probe_host(&mut host)));
                row.frames = match probe {
                    Ok(Ok(())) => "ok".into(),
                    Ok(Err(e)) => {
                        row.gaps.push(format!("frames {e}"));
                        e
                    }
                    Err(_) => {
                        row.gaps.push("frames panic (FSM frame run)".into());
                        "panic".into()
                    }
                };
            }
        }
    } else {
        row.gaps
            .push("no engine seam stages it — staged directly".into());
        match stage_direct(zone, mission as i32, &b) {
            Err(e) => row.load = format!("FAIL direct {e}"),
            Ok(mut scene) => {
                row.load = "direct".into();
                stage_families(
                    &mut scene,
                    &bdg,
                    &pos,
                    &trt,
                    &b[0],
                    &nme,
                    zone,
                    mission,
                    &mut row.gaps,
                    &mut row.destroy,
                    &mut row.pickup,
                    &mut row.critters,
                );
                let probe = catch_unwind(AssertUnwindSafe(|| frames_probe_scene(&mut scene)));
                row.frames = match probe {
                    Ok(Ok(())) => "ok".into(),
                    Ok(Err(e)) => {
                        row.gaps.push(format!("frames {e}"));
                        e
                    }
                    Err(_) => {
                        row.gaps.push("frames panic (direct scene run)".into());
                        "panic".into()
                    }
                };
            }
        }
    }
    row
}

fn run_census() -> Vec<Row> {
    let mut rows = Vec::new();
    for (letter, count) in ZONE_SHAPE {
        for mission in 1..=count {
            rows.push(census_mission(letter, mission));
        }
    }
    rows
}

/// The §6 gap table's machine form (the pinned census). Drift here
/// fails loud; a deliberate loader change re-baselines it WITH its
/// commit message saying why (the D28 fingerprint rule).
const PINNED: &[(&str, &str)] = &[
    ("ZONEA-MISSION1", "host:clean"),
    ("ZONEB-MISSION1", "host:gaps critters refused (Chasersx10)"),
    (
        "ZONEB-MISSION2",
        "host:gaps critters refused (Shootersx3,Chasersx6)",
    ),
    ("ZONEB-MISSION3", "host:gaps critters refused (Chasersx7)"),
    (
        "ZONEB-MISSION4",
        "host:gaps critters refused (Shootersx1,Chasersx12)",
    ),
    (
        "ZONEB-MISSION5",
        "host:gaps critters refused (Shootersx1,Chasersx16)",
    ),
    ("ZONEB-MISSION6", "select:clean"),
    ("ZONEB-MISSION7", "select:clean"),
    (
        "ZONEC-MISSION1",
        "host:gaps critters refused (Shootersx1,Chasersx10)",
    ),
    ("ZONEC-MISSION2", "host:gaps critters refused (Chasersx13)"),
    (
        "ZONEC-MISSION3",
        "host:gaps critters refused (Shootersx4,Chasersx9,CloseCombatx4)",
    ),
    ("ZONEC-MISSION4", "host:gaps critters refused (Chasersx15)"),
    (
        "ZONEC-MISSION5",
        "host:gaps critters refused (Shootersx1,Chasersx2)",
    ),
    ("ZONEC-MISSION6", "select:clean"),
    ("ZONEC-MISSION7", "select:clean"),
    (
        "ZONED-MISSION1",
        "host:gaps critters refused (Shootersx4,Chasersx9)",
    ),
    (
        "ZONED-MISSION2",
        "host:gaps critters refused (Shootersx8,Chasersx7)",
    ),
    (
        "ZONED-MISSION3",
        "host:gaps critters refused (Shootersx8,Chasersx4)",
    ),
    (
        "ZONED-MISSION4",
        "host:gaps critters refused (Shootersx8,Chasersx4)",
    ),
    ("ZONED-MISSION5", "host:gaps critters refused (Shootersx4)"),
    ("ZONED-MISSION6", "select:clean"),
    ("ZONED-MISSION7", "select:clean"),
    (
        "ZONEE-MISSION1",
        "host:gaps critters refused (Shootersx4,Chasersx6,CloseCombatx5,Personnelx12)",
    ),
    (
        "ZONEE-MISSION2",
        "host:gaps critters refused (Shootersx1,Chasersx5,CloseCombatx5,Personnelx12)",
    ),
    (
        "ZONEE-MISSION3",
        "host:gaps critters refused (Shootersx3,Chasersx5,CloseCombatx6,Personnelx12)",
    ),
    (
        "ZONEE-MISSION4",
        "host:gaps critters refused (Shootersx4,Chasersx8,CloseCombatx8,Personnelx12)",
    ),
    (
        "ZONEE-MISSION5",
        "host:gaps critters refused (Shootersx5,Chasersx13,CloseCombatx4,Personnelx13)",
    ),
    ("ZONEE-MISSION6", "select:clean"),
    ("ZONEE-MISSION7", "select:clean"),
    (
        "ZONEF-MISSION1",
        "host:gaps critters refused (Chasersx3,CloseCombatx4,Personnelx9)",
    ),
    ("ZONEF-MISSION2", "host:gaps critters refused (Personnelx9)"),
    ("ZONEF-MISSION3", "host:gaps critters refused (Personnelx9)"),
    ("ZONEF-MISSION4", "host:gaps critters refused (Personnelx9)"),
    (
        "ZONEF-MISSION5",
        "host:gaps critters refused (Personnelx19)",
    ),
    ("ZONEF-MISSION6", "select:clean"),
    ("ZONEF-MISSION7", "select:clean"),
    (
        "ZONEG-MISSION1",
        "host:gaps critters refused (Shootersx3,Chasersx23,CloseCombatx6,Personnelx9)",
    ),
];

#[test]
fn census_matches_pinned_table() {
    if !corpus_present() {
        eprintln!("corpus absent — census skipped");
        return;
    }
    let rows = run_census();
    assert_eq!(rows.len(), 37, "the ledger census is 37 missions");
    for row in &rows {
        let pinned = PINNED
            .iter()
            .find(|(id, _)| *id == row.id)
            .unwrap_or_else(|| panic!("{} missing from the pinned table", row.id));
        assert_eq!(
            (row.id.as_str(), row.summary().as_str()),
            *pinned,
            "census drift for {} — re-baseline deliberately",
            row.id
        );
    }
    assert_eq!(PINNED.len(), 37, "no stale pinned rows");
}

/// Prints the full census table for humans (the §6 source):
/// `cargo test --release -p bedlam-game --test mission_load_census --
///  census_print_table --ignored --nocapture`.
#[test]
#[ignore = "probe/report helper — asserts nothing"]
fn census_print_table() {
    if !corpus_present() {
        eprintln!("corpus absent — census skipped");
        return;
    }
    let rows = run_census();
    println!("mission | dims | load | destroy | pickup | critters | parsers | frames | summary");
    for row in &rows {
        println!(
            "{} | {} | {} | {} | {} | {} | {} | {} | {}",
            row.id,
            row.dims,
            row.load,
            row.destroy,
            row.pickup,
            row.critters,
            row.parsers,
            row.frames,
            row.summary()
        );
    }
}
