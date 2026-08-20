//! Mission-sim corpus gate (P4 slice tail). Skips when the corpus is
//! absent (CI); when present it drives the EXW-verified mission-load +
//! spawn + order->walk chain on the REAL shipped bytes
//! (docs/RE-EXW-SIM.md sec 7c, engine/bedlam-core/src/mission.rs):
//!
//! 1. LOADER: `Terrain::from_mission_bytes(DAT, PAD, zone CGR)` builds
//!    the 25x75 ZONEA/MISSION1 map with the load_mission@0041dc5a
//!    rules — the deck floor reads z 31 (type 1 -> CGR slot 0, 0x1F
//!    raw height bytes, no codec), the type-37 wall column reads as
//!    the low ground (climb 30 = blocked, the real-map wall), and the
//!    PAD 0xFF mark materialises as a deck tile.
//! 2. SPAWN: ZONEA MRK record 0 (21, 73, z-level 1) spawns per
//!    load_markers@0040cca0 — pos = tile*0x2000 + 0xF00, z = 31, the
//!    one settle probe passes on the real floor. ZONEB/MISSION1 MRK
//!    record 0 (27, 71, z-level 3) settles at z 95 on top of the real
//!    three-level deck stack (multi-level map pin).
//! 3. ORDER->WALK: a second staged robot (the host-side marker the
//!    0x46cbe0 network override would spawn in the original) at
//!    (18, 73); the click-order arms at robot 0's tile and robot 1
//!    walks the REAL 4-tile stretch east — spread slot 1 target
//!    (22, 73) — through the six-phase unit manager until the arrival
//!    snap, exercising move_is_possible/get_z_pos against the real
//!    DAT type grid + CGR height maps at every sub-tick.
//! 4. HASH-PINNED: the state hash at spawn / arm / arrival is pinned
//!    exactly; two independent runs are hash-identical end to end.
//!
//! game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level. No game bytes enter git
//! — only hashes are asserted.

use std::path::PathBuf;

use bedlam_core::mission::{
    AngleTable, MissionSim, Terrain, Q13_PER_TILE, SPAWN_CENTER, STATE_ORDERED,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn read(rel: &[&str]) -> Option<Vec<u8>> {
    std::fs::read(root().join(rel.iter().collect::<PathBuf>())).ok()
}

/// (ZONEA DAT, PAD, MRK, zone CGR, SINTABLE)
/// The staged corpus inputs: (ZONEA DAT, PAD, MRK, zone CGR, SINTABLE).
type Zonea = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);

fn zonea() -> Option<Zonea> {
    Some((
        read(&["EDITOR", "ZONEA", "MISSION1.DAT"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.PAD"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.MRK"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.CGR"])?,
        read(&["GAMEGFX", "SINTABLE.BIN"])?,
    ))
}

/// SINTABLE.BIN words[2..66] — the 64-entry angle threshold table
/// [inferred provenance, docs/RE-EXW-SIM.md sec 3 note; the corpus
/// words are 0x0647..0x7FF5 ascending, exactly the assumed shape].
fn angle_table(sintable: &[u8]) -> AngleTable {
    assert_eq!(sintable.len(), 512, "SINTABLE.BIN is 256 words");
    let mut words = [0i16; 256];
    for (i, w) in words.iter_mut().enumerate() {
        *w = i16::from_le_bytes([sintable[2 * i], sintable[2 * i + 1]]);
    }
    AngleTable::from_sintable_words(&words).expect("256 words")
}

fn build_sim(dat: &[u8], pad: &[u8], cgr: &[u8], sintable: &[u8]) -> MissionSim {
    let terrain =
        Terrain::from_mission_bytes(dat, pad, cgr).expect("mission DAT+PAD + zone CGR parse");
    // The EXW MissionShell reseeds the mission RNG to 0x1e240
    // [docs/RE-EXW-SIM.md sec 1].
    MissionSim::new(terrain, angle_table(sintable), 0x1E240)
}

/// One scripted run: spawn MRK[0] + the staged second walker, arm the
/// order at robot 0, run to arrival. Returns the hash trace
/// (post-spawn, post-arm, then one hash per frame until arrival).
fn scripted_walk(dat: &[u8], pad: &[u8], cgr: &[u8], sintable: &[u8]) -> (Vec<u64>, usize) {
    let mut sim = build_sim(dat, pad, cgr, sintable);
    let a = sim.spawn_robot((21, 73, 1)); // ZONEA MRK record 0
    let b = sim.spawn_robot((18, 73, 1)); // staged marker (host seam)
    let mut trace = vec![sim.state_hash().0];
    assert!(sim.arm_order_at_robot(a));
    trace.push(sim.state_hash().0);
    let mut frames = 0usize;
    while sim.robots()[b].state != STATE_ORDERED && frames < 400 {
        sim.advance_frame();
        trace.push(sim.state_hash().0);
        frames += 1;
    }
    assert!(frames < 400, "the walk terminates on the real map");
    (trace, frames)
}

#[test]
fn zonea_mission1_loader_spawn_walk_hash_pinned() {
    let Some((dat, pad, mrk, cgr, sintable)) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };

    // --- 1. Loader over the real bytes -------------------------------
    let mut probe = Terrain::from_mission_bytes(&dat, &pad, &cgr).unwrap();
    assert_eq!(probe.size(), (25, 75), "ZONEA/MISSION1 map dims");
    // Deck: type 1 -> slot 0 raw height 0x1F -> floor z 31 at level 0.
    assert_eq!(probe.floor_z(21 * 32 + 16, 73 * 32 + 16, 0), 31);
    // Wall column x=23 row 73: type 37 -> slot 36 low ground; from the
    // z-31 deck the climb is 30 > 4 = the real-map wall.
    assert_eq!(probe.floor_z(23 * 32 + 16, 73 * 32 + 16, 31), 1);
    // PAD[0] = (5, 61, 0): the 0xFF mark reads back as a type-1 deck
    // tile at level 0 [loader rule 7c.5].
    assert_eq!(probe.dat_type(5, 61, 0), 1);

    // --- 2. Spawn from the real MRK record 0 -------------------------
    let rec: [u8; 16] = mrk[0..16].try_into().unwrap();
    let u32at = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    assert_eq!(
        (u32at(0), u32at(4), u32at(8), u32at(12)),
        (1, 21, 73, 1),
        "ZONEA/MISSION1 MRK record 0: (flag, x, y, z-level)"
    );
    let mut sim = build_sim(&dat, &pad, &cgr, &sintable);
    let a = sim.spawn_robot((21, 73, 1));
    let r = &sim.robots()[a];
    assert_eq!(r.pos_x, 21 * Q13_PER_TILE + SPAWN_CENTER);
    assert_eq!(r.pos_y, 73 * Q13_PER_TILE + SPAWN_CENTER);
    assert_eq!(r.z, 31, "the real deck settles the spawn probe");

    // --- 3. Order -> walk on the real map ----------------------------
    let (trace, frames) = scripted_walk(&dat, &pad, &cgr, &sintable);
    let mut sim = build_sim(&dat, &pad, &cgr, &sintable);
    let a = sim.spawn_robot((21, 73, 1));
    let b = sim.spawn_robot((18, 73, 1));
    assert!(sim.arm_order_at_robot(a));
    for _ in 0..frames {
        sim.advance_frame();
    }
    let walker = &sim.robots()[b];
    // Spread slot 1 = order tile + (1, 0) = (22, 73): the walker ran
    // east 4 tiles. EXW arrival fires INSIDE the 0x1400 radius BEFORE
    // the origin, so a walker closing from the west snaps one tile
    // short of the target origin (21, 73) — approaching from the east
    // lands on 22 exactly (see the synthetic unit test); both are the
    // verified `pos &= ~0x1FFF` semantics [RE-EXW-SIM sec 5].
    assert_eq!((walker.pos_x >> 13, walker.pos_y >> 13), (21, 73));
    assert_eq!(walker.pos_x & 0x1FFF, 0, "arrival snaps to tile origin");
    assert_eq!(walker.pos_y & 0x1FFF, 0);
    assert_eq!(walker.z, 31, "stays on the real deck through the walk");
    assert_eq!(walker.state, STATE_ORDERED);
    assert!(sim.order().is_none(), "order consumed once all state-3");

    // --- 4. Hash pins (deterministic across machines/builds) ---------
    // The walk closes ~3.5 tiles in 7 frames (6 sub-ticks each at
    // 1/8-tile strides — the EXW frame cadence on real terrain).
    let n = trace.len();
    assert_eq!(frames, 7, "EXW cadence: 6x 1/8-tile sub-ticks per frame");
    assert_eq!(
        format!("{:016x}", trace[0]),
        "36ddc86345c8351c",
        "post-spawn"
    );
    assert_eq!(format!("{:016x}", trace[1]), "b19ef94c372750c0", "post-arm");
    assert_eq!(
        format!("{:016x}", trace[n - 1]),
        "4073faae1c72d8d4",
        "arrival"
    );
    assert_eq!(format!("{:016x}", trace[1]), "b19ef94c372750c0", "post-arm");
    assert_eq!(
        format!("{:016x}", trace[n - 1]),
        "4073faae1c72d8d4",
        "arrival"
    );
    let (trace2, frames2) = scripted_walk(&dat, &pad, &cgr, &sintable);
    assert_eq!(frames, frames2);
    assert_eq!(trace, trace2, "two scripted runs are hash-identical");
}

#[test]
fn zoneb_mission1_multilevel_spawn_settles_on_the_roof() {
    // ZONEB/MISSION1 MRK record 0 = (27, 71, z-level 3) on a real
    // three-deep type-1 deck stack: the spawn z (3*0x20 - 1 = 95) must
    // settle through the real height search at level 2 [7c.7].
    let (Some(dat), Some(cgr), Some(sintable)) = (
        read(&["EDITOR", "ZONEB", "MISSION1.DAT"]),
        read(&["EDITOR", "ZONEB", "MISSIONB.CGR"]),
        read(&["GAMEGFX", "SINTABLE.BIN"]),
    ) else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };
    let pad = read(&["EDITOR", "ZONEB", "MISSION1.PAD"]).expect("ZONEB PAD");
    let mut probe = Terrain::from_mission_bytes(&dat, &pad, &cgr).unwrap();
    assert_eq!(probe.size(), (100, 100), "ZONEB/MISSION1 map dims");
    assert_eq!(probe.floor_z(27 * 32 + 16, 71 * 32 + 16, 95), 95);
    let mut sim = build_sim(&dat, &pad, &cgr, &sintable);
    let a = sim.spawn_robot((27, 71, 3));
    let r = &sim.robots()[a];
    assert_eq!(r.z, 95, "spawn settles on the level-2 deck roof");
    assert_eq!(r.probe_z[0], 95);
}
