//! S0-12 strict-coverage oracle for the eight fresh-session T0
//! campaign/config rows (RE-EXW-SIM §7j.64, D153): `score`,
//! `money`, `difficulty`, `zone`, `mission`, `mode`,
//! `linear-mission-m`, `sfx-master-gate`.
//!
//! Two halves, the static-oracle convention (the S0-07..S0-11
//! pattern, e.g. `static_loader_differential.rs`):
//!
//! 1. ORIGINAL-SIDE TRANSCRIPTION (corpus-free): every fact below is
//!    hand-transcribed from the §7j.64 instruction-level decode of
//!    `ghidra-project/exw-text-objdump.txt` — the GameMain boot-init
//!    head, the episode-loop slot boot, the name-entry fresh-campaign
//!    arm, the linear-mission-m derivation clamp, and the SOUND
//!    loader default. The transcription is the coverage: it pins the
//!    ORIGINAL's fresh-session semantics in code, independently of
//!    the engine.
//! 2. E-SIDE COMPARISON (corpus-gated): the S0 canonical run's
//!    anchor-frame T0 rows against that table. Five rows are CLOSED
//!    both sides; the remaining three are pinned as LOUD, NAMED gaps
//!    (the S0-12b seam unit) — each assertion below states the
//!    original value it diverges from, so the seam landing flips
//!    exactly these assertions.
//!
//! This test lives in bedlam-game (unlike the S0-07..S0-11 core
//! oracles) because the rows' E half IS the canonical harness
//! (`parity_harness/canonical.rs`, re-exported here the
//! canonical_dump_gate way) — bedlam-core cannot see it.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::{Path, PathBuf};

use canonical::run_canonical;
use diffharness::dump::{decode_dump, Channel};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR").is_dir()
}

// ---------------------------------------------------------------------
// 1. Original-side transcription (§7j.64) — the coverage half
// ---------------------------------------------------------------------

/// §7j.64/A — the GameMain boot-init head writes the difficulty cell
/// `ebx` = 1 (0x41c14a; ebx re-set to 1 at 0x41c12e) and the mode
/// cell `eax` = 0 (0x41c145).
const ORIGINAL_FRESH_DIFFICULTY: u32 = 1;
/// §7j.64/B — the episode-loop slot boot stores `edx` = 1 (set
/// 0x41c41c) into BOTH zone (0x41c42a) and mission (0x41c430); score
/// takes the FUN_0043a5fc fresh-path return ecx = 0 (0x41c44e).
const ORIGINAL_FRESH_ZONE: u32 = 1;
const ORIGINAL_FRESH_MISSION: u32 = 1;
const ORIGINAL_FRESH_SCORE: u32 = 0;
/// §7j.64/E — the boot config loader FUN_004252c0 loads HKCU "SOUND"
/// through the D128 bounded loader with bounds [0,1] and DEFAULT 1:
/// the absent-registry fresh value (sound ON).
const ORIGINAL_FRESH_SFX_GATE: u32 = 1;

/// The linear-mission-m derivation (§7j.64/D, EXW 0x41c520..0x41c556):
/// `m = clamp(5*(zone-2) + mission - 1, floor 1, cap 26)` — recomputed
/// from the CURRENT slot every episode; never a persisted counter.
fn original_linear_mission_m(zone: u32, mission: u32) -> i32 {
    (5 * (zone as i32 - 2) + mission as i32 - 1).clamp(1, 26)
}

/// The name-entry fresh-campaign money formula (§7j.64/C, EXW
/// 0x43aaa3: `imul [0x46cbf8],0x1f4` off the `0xfa0` base) — the same
/// identity `menu::start_score` carries.
fn original_fresh_money(difficulty: i32) -> i32 {
    4000 - 500 * difficulty
}

/// The shipped per-zone mission counts (the corpus census shape the
/// S0-11 oracle pinned: A=1, B..F=7, G=1 = 37 missions).
const SHIPPED_MISSIONS_PER_ZONE: [u32; 7] = [1, 7, 7, 7, 7, 7, 1];

#[test]
fn original_fresh_session_scalar_table() {
    // The fresh-session values the §7j.64 decode pins, stated once as
    // the transcription the E comparison below is judged against.
    // difficulty 1 / mode 0 (boot head), zone 1 / mission 1 / score 0
    // (slot boot), sfx 1 (registry default).
    assert_eq!(ORIGINAL_FRESH_DIFFICULTY, 1);
    assert_eq!(ORIGINAL_FRESH_ZONE, 1);
    assert_eq!(ORIGINAL_FRESH_MISSION, 1);
    assert_eq!(ORIGINAL_FRESH_SCORE, 0);
    assert_eq!(ORIGINAL_FRESH_SFX_GATE, 1);
    // §7j.64/C — money := 4000 - 500*d at the fresh-campaign arm
    // (0x43aaca); the untouched-toggle fresh boot carries the A
    // difficulty default 1.
    assert_eq!(original_fresh_money(1), 3500, "fresh-boot money (d=1)");
    // The cycled-difficulty variants (the name-entry toggle domain).
    assert_eq!(original_fresh_money(0), 4000);
    assert_eq!(original_fresh_money(2), 3000);
    // §7j.64/D — linear-mission-m fresh (zone 1, mission 1):
    // 5*(-1) + 1 - 1 = -5 -> the 0x41c550 floor -> 1.
    // GAP COMPANION (S0-12b): E emits the 0-based progress counter 0.
    assert_eq!(original_linear_mission_m(1, 1), 1);
}

#[test]
fn original_linear_mission_derivation_table() {
    // Hand-computed spot table (the §7j.64/D decode, instruction
    // arithmetic `lea eax,[eax+eax*4]; dec`).
    let table: &[(u32, u32, i32)] = &[
        (1, 1, 1), // fresh slot: -5 -> floor 1
        (2, 1, 1), // 5*0 + 0 = 0 -> floor 1
        (2, 2, 1), // 1
        (2, 7, 6), // zone B last: 6
        (3, 1, 5), // 5*1 + 0
        (3, 7, 11),
        (4, 1, 10),
        (5, 1, 15),
        (6, 1, 20),
        (6, 7, 26), // the corpus max: 5*4 + 6 = 26, exactly at the cap
        (7, 1, 25), // zone G's only mission
        (7, 2, 26), // were zone 7 to carry a second mission
        (7, 3, 26), // 27 -> the 0x41c53e cap 26 (unreachable on corpus
                    // data: zone 7 ships 1 mission)
    ];
    for &(zone, mission, expect) in table {
        assert_eq!(
            original_linear_mission_m(zone, mission),
            expect,
            "linear-mission-m({zone},{mission})"
        );
    }
    // The cap arm's boundary: the raw expression first reaches 27 at
    // zone 7 mission 3 (5*5 + 3 - 1 = 27).
    assert_eq!(5 * (7 - 2) + 3 - 1, 27);

    // The full shipped-corpus census: the transcription walked over
    // every (zone, mission) the game can stage.
    let mut count = 0usize;
    let mut sum = 0i64;
    let mut max = 0i32;
    for (zone_idx, &missions) in SHIPPED_MISSIONS_PER_ZONE.iter().enumerate() {
        let zone = zone_idx as u32 + 1;
        for mission in 1..=missions {
            let m = original_linear_mission_m(zone, mission);
            assert!((1..=26).contains(&m), "m in 1..=26 on corpus data");
            count += 1;
            sum += i64::from(m);
            max = max.max(m);
        }
    }
    // 37 missions; the census identities the S0-11 corpus shape
    // pinned (A=1/B..F=7/G=1). m=1 exactly for zone-1 m1, zone-2 m1
    // and m2 (the three floor cases); max 26 at zone-6 m7.
    assert_eq!(count, 37);
    assert_eq!(max, 26);
    let floors = (1..=7u32)
        .flat_map(|zone| {
            let missions = SHIPPED_MISSIONS_PER_ZONE[(zone - 1) as usize];
            (1..=missions).map(move |mission| original_linear_mission_m(zone, mission))
        })
        .filter(|&m| m == 1)
        .count();
    assert_eq!(floors, 3, "exactly three floor cases on the corpus");
    assert_eq!(
        sum, 482,
        "corpus linear-m sum (hand-summed: 1 + (1+1+2+3+4+5+6) + (5..=11) + \
         (10..=16) + (15..=21) + (20..=26) + 25)"
    );
}

// ---------------------------------------------------------------------
// 2. E-side comparison (corpus-gated S0 run)
// ---------------------------------------------------------------------

#[test]
fn corpus_s0_anchor_t0_rows_match_the_transcription() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let s0 = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/diffharness/scenarios/S0.scen"),
    )
    .expect("S0.scen committed");
    let run = run_canonical(&s0, &root()).expect("S0 canonical run");
    let dump = decode_dump(&run.bytes).expect("S0 dump verifies");
    assert_eq!(dump.header.channel, Channel::Engine);
    let anchor = &dump.frames[0];

    let u32row = |id: &str| -> u32 {
        let bytes = anchor.watch(id).unwrap_or_else(|| panic!("row {id}"));
        u32::from_le_bytes(bytes.try_into().expect("4-byte T0 row"))
    };

    // ---- CLOSED both sides (§7j.64/G) ----
    assert_eq!(
        u32row("score"),
        ORIGINAL_FRESH_SCORE,
        "score: 0x41c44e fresh 0 == E"
    );
    assert_eq!(
        u32row("mission"),
        ORIGINAL_FRESH_MISSION,
        "mission: 0x41c430 fresh 1 == E slot"
    );
    assert_eq!(u32row("mode"), 0, "mode: 0x41c145 fresh 0 == E SP");
    // zone: E emits the 0-based stage; the guest cell is the 1-based
    // set (D99) — the O1 normalizer maps cell-1 (D108). The
    // transcription value is E+1.
    assert_eq!(
        u32row("zone") + 1,
        ORIGINAL_FRESH_ZONE,
        "zone: 0x41c42a fresh 1 == E stage 0 + the D99/D108 +1 map"
    );
    // sfx: the loader default 1 (§7j.64/E) == E's D136 constant 1 —
    // closed under the D134/D136/D144 machine-config seam (a
    // sound-disabled capture machine dumps 0: the loud finding).
    assert_eq!(u32row("sfx-master-gate"), ORIGINAL_FRESH_SFX_GATE);

    // ---- LOUD NAMED GAPS (the S0-12b seam unit; each assertion
    // states the original value it diverges from and flips when the
    // seam lands — never silently re-baselined) ----
    assert_eq!(
        u32row("difficulty"),
        0,
        "GAP S0-12b: E fresh default 0 vs the original boot write {} \
         (0x41c14a, 7j.64/A)",
        ORIGINAL_FRESH_DIFFICULTY
    );
    assert_eq!(
        u32row("money"),
        4000,
        "GAP S0-12b: E fresh default 4000 (d=0) vs the original \
         fresh-boot {} (4000-500*1, 0x43aaca with d=1, 7j.64/C)",
        original_fresh_money(ORIGINAL_FRESH_DIFFICULTY as i32)
    );
    assert_eq!(
        u32row("linear-mission-m"),
        0,
        "GAP S0-12b: E emits the 0-based progress counter 0 vs the \
         original DERIVED cell 1 (the 0x41c550 floor of \
         5*(1-2)+1-1, 7j.64/D)"
    );
}

#[test]
fn corpus_boot_difficulty_seed_matches_the_name_entry_formula() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    // The grammar's boot difficulty key IS the name-entry toggle seam
    // (§7j.64/C): the seed must land through the engine's own
    // start_score formula, proving the ORIGINAL fresh-boot state
    // (d=1 -> money 3500) is expressible on E today — the S0-12b gap
    // is the fresh DEFAULT, not a missing mechanism.
    let scen = "scenario = \"SD1\"\ntiers = T0,TS\nframes = 2\nboot difficulty=1\nuntil-anchor mission-start\nstep 2\n";
    let run = run_canonical(scen, &root()).expect("boot difficulty=1 consumed");
    let dump = decode_dump(&run.bytes).expect("SD1 dump verifies");
    let u32row = |id: &str| -> u32 {
        let bytes = dump.frames[0]
            .watch(id)
            .unwrap_or_else(|| panic!("row {id}"));
        u32::from_le_bytes(bytes.try_into().expect("4-byte T0 row"))
    };
    assert_eq!(u32row("money"), original_fresh_money(1) as u32);
    assert_eq!(u32row("difficulty"), ORIGINAL_FRESH_DIFFICULTY);
    // The original fresh-boot linear value is expressible through the
    // zone-staging seam arithmetic the S0-12b seam unit will pin.
    assert_eq!(
        original_linear_mission_m(ORIGINAL_FRESH_ZONE, ORIGINAL_FRESH_MISSION),
        1,
        "the derived-cell value the S0-12b linear seam must emit"
    );
}
