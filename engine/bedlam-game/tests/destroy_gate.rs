//! The W12-S4-prep destroy-family gate (DESIGN §7 S4 row; D104):
//! the staging seams, the two resolvers, the destroy tail's
//! restore/effects/score/chain, the stagers, the platform entry,
//! the trap lane, and the disbursers — with the RNG-draw counts
//! asserted against a same-seed reference sim (the shared-stream
//! discipline of §7j.38). The synthetic core is corpus-free
//! (CI-safe); the S0/S1/S2/S3 chain pins live in
//! canonical_dump_gate.rs; the grammar test pins the parser
//! against the real .BDG files when the corpus is present
//! (the D104 adoption fix — a synthetic-only roundtrip missed a
//! 4-byte head desync that rejected every shipped file).

use bedlam_core::destroy::{
    ObjectEffectEntry, ObjectType, ObjectTypeTable, ARTILLERY_PAIRS, DEBRIS_SEQ_TABLES,
    OBJECT_INSTANCE_SLOTS,
};
use bedlam_core::mission::{AngleTable, MissionSim, Terrain};
use bedlam_core::weapon::{weapon_damage, WeaponRecord};

const W: i32 = 32;
const H: i32 = 32;

fn synth_sim() -> MissionSim {
    let terrain = Terrain::from_parts(W, H, vec![0u8; 8 * (W * H) as usize], Vec::new())
        .expect("synthetic terrain");
    let angles = AngleTable::from_thresholds(&[0u16; 64]).expect("threshold table");
    MissionSim::new(terrain, angles, 0xDEAD_BEEF)
}

/// A one-active-row BDG table in the TRUE disk grammar [FORMATS
/// §16]: W=1,H=1,D=2, hp 100, the given chain/kind; effects at
/// +0x12 (NO count word on disk — it is load-computed), the four
/// template banks at +0x3A carrying distinct marker words.
fn bdg_row(effects: [ObjectEffectEntry; 5], chain: u16, kind: i32, hp: i32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&1u16.to_le_bytes()); // control
    b.extend_from_slice(&1u16.to_le_bytes()); // W
    b.extend_from_slice(&1u16.to_le_bytes()); // H
    b.extend_from_slice(&2u16.to_le_bytes()); // D
    b.extend_from_slice(&hp.to_le_bytes());
    b.extend_from_slice(&chain.to_le_bytes());
    b.extend_from_slice(&kind.to_le_bytes());
    for e in &effects {
        b.extend_from_slice(&e.selector.to_le_bytes());
        b.extend_from_slice(&e.dx.to_le_bytes());
        b.extend_from_slice(&e.dy.to_le_bytes());
        b.extend_from_slice(&e.dz.to_le_bytes());
    }
    // Four banks, W·H·D = 2 words each, disk order
    // current-TOT, under-TOT, current-DAT, under-DAT [§7j.32/1].
    for bank in [0x1111u16, 0x2222, 0x3333, 0x4444] {
        for v in [bank, bank + 1] {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b
}

fn one_type(
    effects: [ObjectEffectEntry; 5],
    chain: u16,
    kind: i32,
    hp: i32,
    under_tot: u16,
    under_dat: u16,
) -> ObjectTypeTable {
    ObjectTypeTable {
        rows: vec![ObjectType {
            w: 1,
            h: 1,
            d: 1,
            hp,
            chain,
            kind,
            count: 0,
            effects,
            bank_current_tot: vec![7],
            bank_under_tot: vec![under_tot],
            bank_current_dat: vec![0],
            bank_under_dat: vec![under_dat],
        }],
    }
}

fn plain_type(hp: i32) -> ObjectTypeTable {
    one_type([ObjectEffectEntry::default(); 5], 0, 30, hp, 0, 0)
}

fn empty_pos() -> Vec<u8> {
    let mut pos = vec![0u8; 16 * OBJECT_INSTANCE_SLOTS];
    for slot in 0..OBJECT_INSTANCE_SLOTS {
        pos[slot * 16 + 12..slot * 16 + 16].copy_from_slice(&(-1i32).to_le_bytes());
    }
    pos
}

fn pos_with(x: i32, y: i32, z: i32, id: i32) -> Vec<u8> {
    let mut pos = empty_pos();
    pos[0..4].copy_from_slice(&x.to_le_bytes());
    pos[4..8].copy_from_slice(&y.to_le_bytes());
    pos[8..12].copy_from_slice(&z.to_le_bytes());
    pos[12..16].copy_from_slice(&id.to_le_bytes());
    pos
}

fn trt_empty() -> Vec<u8> {
    vec![0u8, 0]
}

fn trt_with(x: i32, y: i32, z: i32) -> Vec<u8> {
    let mut b = vec![1u8, 0];
    b.extend_from_slice(&x.to_le_bytes());
    b.extend_from_slice(&y.to_le_bytes());
    b.extend_from_slice(&z.to_le_bytes());
    b
}

/// The number of `rand_a` draws a same-seed reference sim needs to
/// reach the test sim's stream state (Pcg32 advances once per draw,
/// so the state pins the count exactly). Use as a DELTA around the
/// action under test (spawn_robot etc. draw too).
fn draw_count(sim: &MissionSim) -> u32 {
    let mut probe = synth_sim();
    let target = sim.rand_a_state();
    for n in 0..200_000u32 {
        if probe.rand_a_state() == target {
            return n;
        }
        probe.rand_a();
    }
    panic!("stream state not reachable: the sim drew >200k or used another seed");
}

#[test]
fn bdg_grammar_roundtrip() {
    let effects = [ObjectEffectEntry {
        selector: 1,
        dx: 0,
        dy: 0,
        dz: 0,
    }; 5];
    let mut bytes = bdg_row(effects, 0, 30, 100);
    // An empty control row + a second active row.
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&bdg_row(effects, 7, 40, 100));
    let table = ObjectTypeTable::from_bdg_bytes(&bytes).expect("parse");
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[0].hp, 100);
    assert_eq!(table.rows[0].w, 1);
    assert_eq!(table.rows[0].d, 2);
    assert_eq!(table.rows[0].effects[0].selector, 1);
    // The disk order current-TOT, under-TOT, current-DAT,
    // under-DAT [§7j.32/1]: bank 0 = 0x1111, bank 1 (under-TOT) =
    // 0x2222.
    assert_eq!(table.rows[0].bank_current_tot, vec![0x1111, 0x1112]);
    assert_eq!(table.rows[0].bank_under_tot, vec![0x2222, 0x2223]);
    assert_eq!(table.rows[0].bank_current_dat, vec![0x3333, 0x3334]);
    assert_eq!(table.rows[0].bank_under_dat, vec![0x4444, 0x4445]);
    // Trailing bytes = a desync — fail loud.
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(ObjectTypeTable::from_bdg_bytes(&bad).is_none());
}

#[test]
fn bdg_corpus_files_parse_eof_exact() {
    // The D104 adoption guard: pin the parser against the REAL
    // shipped files (37/37, EXACTLY 282 records each, EOF-exact —
    // the §7j.25/8 census). A synthetic-only roundtrip once missed
    // a 4-byte head desync (a phantom disk count word) that
    // rejected every corpus file; this row makes that class
    // permanently loud. Corpus-gated per the W9 skip discipline.
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR");
    let mut files = Vec::new();
    for zone in [
        "ZONEA", "ZONEB", "ZONEC", "ZONED", "ZONEE", "ZONEF", "ZONEG",
    ] {
        for m in 1..=7u32 {
            let p = root.join(zone).join(format!("MISSION{m}.BDG"));
            if p.exists() {
                files.push(p);
            }
        }
    }
    if files.is_empty() {
        eprintln!("skipping: no corpus BDG files (game-data absent)");
        return;
    }
    assert_eq!(files.len(), 37, "the shipped BDG census");
    let (mut total, mut active) = (0usize, 0usize);
    for p in &files {
        let bytes = std::fs::read(p).expect("read BDG");
        let table = ObjectTypeTable::from_bdg_bytes(&bytes)
            .unwrap_or_else(|| panic!("{} desynced", p.display()));
        assert_eq!(table.rows.len(), 282, "{} row count", p.display());
        total += table.rows.len();
        active += table
            .rows
            .iter()
            .filter(|r| *r != &ObjectType::default())
            .count();
        // Zero out-of-range selectors (the §7j.25/8 census).
        for r in &table.rows {
            for e in &r.effects {
                assert!(e.selector <= 9, "{} selector {}", p.display(), e.selector);
            }
        }
    }
    assert_eq!(
        (total, active),
        (10434, 7907),
        "the committed census counts"
    );
}

#[test]
fn trt_hp_formula() {
    // hp = 250 + (250·m)/27 [FORMATS §14]: m = 0 → 250, m = 27 → 500.
    let s0 = bedlam_core::destroy::parse_trt(&trt_with(5, 6, 1), 0).unwrap();
    assert_eq!(s0.len(), 1);
    assert_eq!(s0[0].hp, 250);
    assert_eq!(s0[0].x, 5);
    let s27 = bedlam_core::destroy::parse_trt(&trt_with(5, 6, 1), 27).unwrap();
    assert_eq!(s27[0].hp, 500);
    assert!(bedlam_core::destroy::parse_trt(&[0u8], 0).is_none());
}

#[test]
fn staging_footprints_and_hazards() {
    let mut ty = plain_type(55);
    ty.rows[0].w = 2;
    ty.rows[0].h = 1;
    let mut sim = synth_sim();
    assert!(sim.stage_destroy_family(&ty, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0));
    assert_eq!(sim.objects().len(), 1);
    assert_eq!(sim.objects()[0].hp, 55);
    // The footprint stamps idx+1 over W×H at the origin.
    assert_eq!(sim.object_grid_word(3, 4), 1);
    assert_eq!(sim.object_grid_word(4, 4), 1);
    assert_eq!(sim.object_grid_word(5, 4), 0);
    // The hazard stamper over staged mirror words [§7j.12/6]:
    // zone 1 base 0x20 → a z-word 0x20 stamps 0x7d2 OVER the
    // footprint (the load order: footprints first).
    let n = (W * H) as usize;
    let mut words = vec![0u16; 8 * n];
    words[(4 * W + 3) as usize * 8] = 0x20; // z0 of tile (3,4)
    assert!(sim.stage_terrain_mirror(&words));
    sim.stamp_hazard_words();
    assert_eq!(sim.object_grid_word(3, 4), 0x7d2);
    assert_eq!(sim.object_grid_word(4, 4), 1); // untouched neighbor
}

#[test]
fn resolver_survivor_immune_destroyed() {
    let table = one_type([ObjectEffectEntry::default(); 5], 0, 30, 100, 0x77, 9);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0);
    // Survivor: pure hp subtract, NO draws, no restore.
    let before = sim.rand_a_state();
    assert!(!sim.resolve_object_impact(3 << 13, 4 << 13, 0, 30, true));
    assert_eq!(
        sim.rand_a_state(),
        before,
        "the survivor path draws nothing"
    );
    assert_eq!(sim.objects()[0].hp, 70);
    assert_eq!(sim.mirror_word((4 * W + 3) as usize, 0), 0);
    // Destroy: hp to 0, the restore writes the UNDER pair, the
    // score awards kind, zero draws on this effect set.
    assert!(sim.resolve_object_impact(3 << 13, 4 << 13, 0, 70, true));
    assert!(sim.objects()[0].destroyed);
    assert_eq!(sim.mirror_word((4 * W + 3) as usize, 0), 0x77);
    assert_eq!(
        sim.mirror_seen((4 * W + 3) as usize, 0),
        0,
        "seen := under_dat≠0 → 0"
    );
    assert_eq!(
        sim.terrain.dat_type(3, 4, 0),
        9,
        "volume := under_dat low byte"
    );
    let (score, strip) = sim.take_destroy_score();
    assert_eq!(score, 30, "score := the type kind value");
    assert!(strip);
    assert_eq!(sim.take_destroy_score(), (0, false), "taking clears");
    // Already destroyed → pass-through.
    assert!(!sim.resolve_object_impact(3 << 13, 4 << 13, 0, 1000, true));

    // The immune hp −1 record: never dies, no draws.
    let mut t2 = table.clone();
    t2.rows[0].hp = -1;
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&t2, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0);
    assert!(!sim2.resolve_object_impact(3 << 13, 4 << 13, 0, 1000, false));
    assert_eq!(sim2.objects()[0].hp, -1);
}

#[test]
fn ger_gate_skips_the_tail() {
    let effects = [ObjectEffectEntry {
        selector: 8,
        dx: 0,
        dy: 0,
        dz: 0,
    }; 5];
    let table = one_type(effects, 0, 0xb, 1, 0x77, 9);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0);
    // Stage the GER language latch [§7j.25/1]: the whole
    // restore/effect/score/chain tail skips (the record still dies).
    sim.set_language(1);
    let before = sim.rand_a_state();
    assert!(sim.resolve_object_impact(3 << 13, 4 << 13, 0, 10, true));
    assert_eq!(sim.rand_a_state(), before, "the GER gate draws nothing");
    assert_eq!(sim.mirror_word((4 * W + 3) as usize, 0), 0, "no restore");
    assert_eq!(sim.take_destroy_score(), (0, false), "no score");
    // The ENG edition runs the full tail.
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&table, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0);
    assert!(sim2.resolve_object_impact(3 << 13, 4 << 13, 0, 10, true));
    assert_eq!(sim2.mirror_word((4 * W + 3) as usize, 0), 0x77);
    assert_eq!(sim2.take_destroy_score().0, 10, "kind 0xb scores 10");
}

#[test]
fn effect_loop_draw_counts() {
    // The §7j.38/1 case table: sel 1..9 → 8/8/8/8/8/0/0/72/9 RandA
    // (sel 6/7 stage k10 without draws; the k11 SFX-gate case rides
    // the artillery test instead).
    for (sel, draws) in [
        (1u16, 8u32),
        (2, 8),
        (3, 8),
        (4, 8),
        (5, 8),
        (6, 0),
        (7, 0),
        (8, 72),
        (9, 9),
    ] {
        let effects = [
            ObjectEffectEntry {
                selector: sel,
                dx: 0,
                dy: 0,
                dz: 0,
            },
            ObjectEffectEntry::default(),
            ObjectEffectEntry::default(),
            ObjectEffectEntry::default(),
            ObjectEffectEntry::default(),
        ];
        let table = one_type(effects, 0, 30, 1, 0, 0);
        let mut sim = synth_sim();
        sim.stage_destroy_family(&table, &pos_with(3, 4, 0, 0), &trt_empty(), 1, 0);
        let base = draw_count(&sim);
        sim.resolve_object_impact(3 << 13, 4 << 13, 0, 1, false);
        assert_eq!(draw_count(&sim) - base, draws, "selector {sel}");
    }
}

#[test]
fn chain_walks_detonate_neighbors() {
    // Two adjacent 1×1 objects: destroying A walks the perimeter
    // and detonates the CHAINABLE neighbor at damage 1000 with ONE
    // qualifying draw [§7j.38/2, §7j.39/5].
    let mk = |chain: u16| {
        let mut t = plain_type(500);
        t.rows[0].chain = chain;
        t
    };
    let mut pos = empty_pos();
    for (slot, (x, id)) in [(0usize, (3i32, 0i32)), (1, (4, 1))] {
        pos[slot * 16..slot * 16 + 16].copy_from_slice(&{
            let mut r = [0u8; 16];
            r[0..4].copy_from_slice(&x.to_le_bytes());
            r[4..8].copy_from_slice(&4i32.to_le_bytes());
            r[8..12].copy_from_slice(&0i32.to_le_bytes());
            r[12..16].copy_from_slice(&id.to_le_bytes());
            r
        });
    }
    let table = ObjectTypeTable {
        rows: vec![mk(0).rows[0].clone(), mk(1).rows[0].clone()],
    };
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &pos, &trt_empty(), 1, 0);
    let base = draw_count(&sim);
    assert!(sim.resolve_object_impact(3 << 13, 4 << 13, 0, 500, true));
    // B died through the chain (500 hp < 1000 chain damage).
    assert!(sim.objects()[1].destroyed, "the neighbor chain-detonated");
    // Draws: ONE qualifying candidate in the walks + B's own tail
    // (no effects, no further chainable neighbors → 0) = 1.
    assert_eq!(draw_count(&sim) - base, 1);
    // The non-chainable variant: B survives the walk and the walk
    // draws NOTHING — the chainability gate precedes the roll
    // [§7j.38/2 protocol: word > 0 → alive → chain ≠ 0 → draw].
    let table2 = ObjectTypeTable {
        rows: vec![mk(0).rows[0].clone(), mk(0).rows[0].clone()],
    };
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&table2, &pos, &trt_empty(), 1, 0);
    let base2 = draw_count(&sim2);
    assert!(sim2.resolve_object_impact(3 << 13, 4 << 13, 0, 500, true));
    assert!(!sim2.objects()[1].destroyed);
    assert_eq!(draw_count(&sim2) - base2, 0);
}

#[test]
fn structure_resolver_rubble_stamp() {
    let table = plain_type(1);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &empty_pos(), &trt_with(6, 7, 2), 3, 0);
    // Zone 3 rubble = 0x348 [§7j.38/3]; linear 0 → hp 250, so 300
    // kills it.
    sim.resolve_structure_impact(6 << 13, 7 << 13, 300);
    assert!(!sim.structures()[0].active);
    let tile = (7 * W + 6) as usize;
    assert_eq!(sim.mirror_word(tile, 2), 0x348);
    assert_eq!(sim.mirror_seen(tile, 2), 1);
    assert_eq!(sim.terrain.dat_type(6, 7, 2), 0);
    // A k15 debris staged (the ring slot) — no draws.
    assert_eq!(draw_count(&sim), 0);
    assert!(sim.debris_bank().iter().any(|d| d.active && d.kind == 15));
    // A survivor structure: pure subtract.
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&table, &empty_pos(), &trt_with(6, 7, 2), 3, 27);
    assert_eq!(sim2.structures()[0].hp, 500);
    sim2.resolve_structure_impact(6 << 13, 7 << 13, 10);
    assert_eq!(sim2.structures()[0].hp, 490);
    assert!(sim2.structures()[0].active);
}

#[test]
fn stagers_gates_and_writes() {
    let table = plain_type(1);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &empty_pos(), &trt_empty(), 1, 0);
    // Debris bounds: OOB stages nothing, draws nothing.
    let before = sim.rand_a_state();
    assert!(!sim.stage_debris(-1, 0, 0x20, 1, 0, 0));
    assert!(!sim.stage_debris(0, H << 5, 0x20, 1, 0, 0));
    assert_eq!(sim.rand_a_state(), before);
    // Kind 11: the SFX-gate draw is real [§7j.11/4].
    assert!(sim.stage_debris(5 * 0x20, 5 * 0x20, 0x40, 11, 0, 0));
    assert_eq!(draw_count(&sim), 1);
    // The scorch ring lands in the +0x18 bank (the armor-pad row).
    assert!(sim.stage_debris(8 * 0x20, 8 * 0x20, 0x40, 5, 0, 0));
    assert!(sim.armor_pads().iter().any(|&b| b != 0));
    // The splash gates: nonzero DAT volume / nonzero z-word refuse.
    sim.terrain.dat_write(9, 9, 0, 3);
    assert!(!sim.stage_splash(9, 9, 0, 0));
    assert!(sim.stage_splash(10, 10, 0, 0));
    let n = (W * H) as usize;
    let mut words = vec![0u16; 8 * n];
    words[(11 * W + 11) as usize * 8] = 5;
    sim.stage_terrain_mirror(&words);
    assert!(!sim.stage_splash(11, 11, 0, 0), "nonzero z-word refuses");
    // z_structure_write: the FUN_0042394a semantics.
    sim.z_structure_write(12, 12, 1, 0x25D, 0);
    assert_eq!(sim.mirror_word((12 * W + 12) as usize, 1), 0x25D);
    assert_eq!(sim.mirror_seen((12 * W + 12) as usize, 1), 1);
    sim.z_structure_write(12, 12, 1, 0, 0);
    assert_eq!(sim.mirror_word((12 * W + 12) as usize, 1), 0);
    assert_eq!(sim.mirror_seen((12 * W + 12) as usize, 1), 0);
}

#[test]
fn platform_entry_destroy_and_weaken() {
    let table = plain_type(1);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &empty_pos(), &trt_empty(), 1, 0);
    // Stage a platform at (5,5) z2 via the FUN_004228ce write half.
    assert!(sim.stage_platform(5, 5, 2, 0));
    let tile = (5 * W + 5) as usize;
    assert_eq!(sim.object_grid_word(5, 5), 0x7d4);
    assert_eq!(sim.mirror_word(tile, 2), 0x25D, "zone 1 water base");
    // No strength → the destroy arm (0 − damage ≤ 0): clear + five
    // k7 (10 draws) [§7j.38, §7j.12/2]. (The creep-seed site store
    // is pub(crate) — S7 seam state, asserted via the unit level.)
    let base = draw_count(&sim);
    sim.platform_damage(5, 5, 50);
    assert_eq!(draw_count(&sim) - base, 10);
    assert_eq!(sim.mirror_word(tile, 2), 0, "the water z-structure cleared");
    let k7 = sim
        .debris_bank()
        .iter()
        .filter(|d| d.active && d.kind == 7)
        .count();
    assert_eq!(k7, 5);
    // Weaken: strength −= damage, a +4 scorch, NO draws.
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&table, &empty_pos(), &trt_empty(), 1, 0);
    assert!(sim2.stage_platform(5, 5, 2, 200));
    let before = sim2.rand_a_state();
    sim2.platform_damage(5, 5, 50);
    assert_eq!(sim2.rand_a_state(), before, "the weaken arm draws nothing");
    assert_eq!(sim2.platform_strength_word(5, 5), 150);
}

#[test]
fn trap_lane_destroy() {
    let table = plain_type(1);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &pos_with(6, 6, 0, 0), &trt_empty(), 1, 0);
    // A robot on the trap tile: DAT byte 0x62 + the grid word.
    let idx = sim.spawn_robot((6, 6, 1));
    let zl = sim.robots()[idx].z >> 5;
    sim.terrain.dat_write(6, 6, zl, 0x62);
    let base = draw_count(&sim);
    assert!(sim.robot_trap_lane(idx));
    assert!(sim.objects()[0].destroyed);
    // Draws: 5 k12 × 3 jitter = 15 [§7j.38/6] (+ the tail's zero —
    // the sel-0 effects draw nothing).
    assert_eq!(draw_count(&sim) - base, 15);
    let k12 = sim
        .debris_bank()
        .iter()
        .filter(|d| d.active && d.kind == 12)
        .count();
    assert_eq!(k12, 5);
    // No 0x62 byte → no trap.
    let mut sim2 = synth_sim();
    sim2.stage_destroy_family(&table, &pos_with(6, 6, 0, 0), &trt_empty(), 1, 0);
    let idx2 = sim2.spawn_robot((6, 6, 1));
    assert!(!sim2.robot_trap_lane(idx2));
}

#[test]
fn weapon_disburser_arms() {
    let mut sim = synth_sim();
    sim.weapon_bank_mut()[0] = WeaponRecord {
        kind: 3,
        x: 5 << 13,
        y: 5 << 13,
        z: 1 << 13,
        owner: 2,
        ..WeaponRecord::default()
    };
    let base = draw_count(&sim);
    sim.weapon_disburser(0);
    assert_eq!(draw_count(&sim) - base, 2, "K2 stages with 2 jitter draws");
    assert_eq!(sim.weapon_bank_mut()[0].kind, 0, "the K2 arm frees");
    assert!(sim.debris_bank().iter().any(|d| d.active && d.kind == 2));
    // 0xF: the raw-asm no-op [§7j.39 — the §7j.14 map corrected].
    sim.weapon_bank_mut()[1] = WeaponRecord {
        kind: 0xF,
        x: 5 << 13,
        y: 5 << 13,
        z: 1 << 13,
        ..WeaponRecord::default()
    };
    let before = sim.rand_a_state();
    sim.weapon_disburser(1);
    assert_eq!(sim.rand_a_state(), before);
    assert_eq!(sim.weapon_bank_mut()[1].kind, 0xF, "0xF keeps its word");
    // 9..0xB: clear-only, no draws.
    sim.weapon_bank_mut()[2] = WeaponRecord {
        kind: 0xA,
        x: 5 << 13,
        y: 5 << 13,
        z: 1 << 13,
        ..WeaponRecord::default()
    };
    let before = sim.rand_a_state();
    sim.weapon_disburser(2);
    assert_eq!(sim.rand_a_state(), before);
    assert_eq!(sim.weapon_bank_mut()[2].kind, 0);
}

#[test]
fn script_blast_score_and_robot_lane() {
    let table = one_type([ObjectEffectEntry::default(); 5], 0, 120, 4000, 0, 0);
    let mut sim = synth_sim();
    sim.stage_destroy_family(&table, &pos_with(5, 5, 0, 0), &trt_empty(), 1, 0);
    // A robot 4 tiles off the blast center: OUTSIDE the ±0x20 Q5
    // box, untouched; one at the center takes the 0xD hit.
    let far = sim.spawn_robot((9, 9, 1));
    let near = sim.spawn_robot((5, 5, 1));
    let (far_hp, near_hp) = (sim.robots()[far].hp, sim.robots()[near].hp);
    let base = draw_count(&sim);
    sim.script_blast(5, 5, 1);
    assert_eq!(sim.robots()[far].hp, far_hp, "outside the box: untouched");
    // The damage lane used the FUN_00419aff(0xD) table value.
    assert_eq!(near_hp - sim.robots()[near].hp, weapon_damage(0xD, 0));
    // The blast's object resolver: 5000 ≥ 4000 hp → destroyed + score.
    assert!(sim.objects()[0].destroyed);
    assert_eq!(sim.take_destroy_score().0, 120);
    // Draws: the k6 gate (1, +1 when it passes) + the destroy tail's
    // zero (sel-0 effects) — ≥ 1 always [§7j.39/1].
    assert!(draw_count(&sim) - base >= 1);
}

#[test]
fn artillery_pair_tables_shape() {
    // The §7j.38/5 DGROUP dump, byte-exact: 217 pairs total across
    // the 7 lists; list 0 = 9 (the full 3×3 block incl. center);
    // list 6 = 68 (the radius-7 ring with the faithful 2-pair tail
    // duplicate — the §7j.38 "68 pairs" gloss was list 6's own
    // count, not the total; corrected in §7j.39).
    let total: usize = ARTILLERY_PAIRS.iter().map(|l| l.len()).sum();
    assert_eq!(total, 217);
    let lens: Vec<usize> = ARTILLERY_PAIRS.iter().map(|l| l.len()).collect();
    assert_eq!(lens, vec![9, 12, 24, 24, 32, 48, 68]);
    assert!(
        ARTILLERY_PAIRS[0].contains(&(0, 0)),
        "list 0 includes the center"
    );
    // The tail duplicate: the last two pairs equal (−6,−5),(−6,−4).
    let l6 = ARTILLERY_PAIRS[6];
    assert_eq!(l6[l6.len() - 2], (-6, -5));
    assert_eq!(l6[l6.len() - 1], (-6, -4));
    assert_eq!(l6.iter().filter(|&&p| p == (-6, -5)).count(), 2);
    // The seq tables: 11 walks, all −1-terminated.
    for t in DEBRIS_SEQ_TABLES.iter() {
        assert_eq!(*t.last().unwrap(), -1);
    }
}

#[test]
fn no_inject_pass_through() {
    // The no-inject invariant: with nothing staged, the RESOLVERS
    // pass through with zero stream motion. The script blast
    // always draws its k6 1-in-8 gate [§7j.39/1] — faithful even
    // on an empty map (the S3 re-pin's very cause).
    let mut sim = synth_sim();
    let before = sim.rand_a_state();
    assert!(!sim.resolve_object_impact(5 << 13, 5 << 13, 0, 1000, true));
    sim.resolve_structure_impact(5 << 13, 5 << 13, 1000);
    assert_eq!(sim.rand_a_state(), before, "the unstaged resolvers");
    sim.script_blast(5, 5, 1);
    assert!(sim.rand_a_state() != before, "the blast gate draws");
    assert_eq!(sim.take_destroy_score(), (0, false));
    // The blast's own splash stages on the empty tile (its gates
    // are occupancy checks, not staging checks — faithful); the k6
    // debris rides the 1-in-8 gate (seed-deterministic, ≤ 1).
    assert_eq!(sim.splash_bank().iter().filter(|s| s.age != 0).count(), 1);
    assert!(sim.debris_bank().iter().filter(|d| d.active).count() <= 1);
    // The bank rows stay at their default lengths (the W6 split).
    assert_eq!(sim.debris_bank().len(), 128);
    assert_eq!(sim.splash_bank().len(), 250);
}
