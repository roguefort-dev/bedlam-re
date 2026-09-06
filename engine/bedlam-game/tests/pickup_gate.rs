//! Original Boot Camp gold pickup collection through movement probes.
use bedlam_core::{
    destroy::ObjectTypeTable,
    mission::{AngleTable, MissionSim, Terrain},
};
fn boot_camp() -> Option<MissionSim> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../game-data/BEDLAM/EDITOR/ZONEA");
    if !root.exists() {
        return None;
    }
    let read = |name: &str| std::fs::read(root.join(name)).unwrap();
    let terrain = Terrain::from_mission_bytes(
        &read("MISSION1.DAT"),
        &read("MISSION1.PAD"),
        &read("MISSIONA.CGR"),
    )
    .unwrap();
    let sine = std::fs::read(root.join("../../GAMEGFX/SINTABLE.BIN")).unwrap();
    let words: Vec<_> = sine
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    let mut sim = MissionSim::new(terrain, AngleTable::from_sintable_words(&words).unwrap(), 1);
    let table = ObjectTypeTable::from_bdg_bytes(&read("MISSION1.BDG")).unwrap();
    assert!(sim.stage_destroy_family(&table, &read("MISSION1.POS"), &read("MISSION1.TRT"), 1, 1));
    assert!(sim.stage_pickup_surface(&read("MISSION1.TOT"), 1));
    sim.stamp_hazard_words();
    sim.stamp_sentries();
    Some(sim)
}

#[test]
fn boot_camp_gold_pickup_is_collected_and_ceases_to_block_movement() {
    let Some(mut sim) = boot_camp() else { return };
    let tile = (35 * sim.terrain.size().0 + 3) as usize;
    assert_eq!(sim.terrain.dat_type(3, 35, 1), 3);
    assert!((0x81..=0x84).contains(&sim.mirror_words()[tile * 8 + 1]));
    sim.spawn_robot((2, 34, 1));
    sim.stage_command_record(bedlam_core::weapon::CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 3 * 32 + 16,
        y: 35 * 32 + 16,
        z: 0,
    });
    for _ in 0..120 {
        sim.advance_frame();
    }
    assert_eq!(sim.terrain.dat_type(3, 35, 1), 0);
    assert_eq!(sim.mirror_words()[tile * 8 + 1], 0x48f);
    assert_eq!(sim.mirror_seen_bank()[tile * 8 + 1], 1);
    let awards = sim.take_pickup_awards();
    assert!(
        awards.0 > 0 || awards.1 > 0,
        "gold must award score or money"
    );
    assert_eq!(sim.take_pickup_awards(), (0, 0));
    assert!(
        sim.robots()[0].pos_y >> 8 >= 35 * 32,
        "pickup still blocks the lane"
    );
    assert!(sim.robots()[0].alive);
}

#[test]
fn boot_camp_gold_corridor_passes_between_solid_scaffold_pillars() {
    let Some(mut sim) = boot_camp() else { return };
    for y in [47, 50] {
        assert_eq!(sim.terrain.dat_type(2, y, 1), 1);
        assert_eq!(sim.terrain.dat_type(4, y, 1), 1);
    }
    sim.spawn_robot((3, 46, 1));
    sim.stage_command_record(bedlam_core::weapon::CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 3 * 32 + 15,
        y: 50 * 32 + 24,
        z: 0,
    });
    for _ in 0..180 {
        sim.advance_frame();
    }
    let robot = &sim.robots()[0];
    assert!(robot.alive);
    assert_eq!(robot.z, 31);
    assert!(
        robot.pos_y >> 8 >= 50 * 32,
        "stopped before the second pillar gap: {:?}",
        robot
    );
    for y in [47, 48] {
        assert_eq!(
            sim.terrain.dat_type(3, y, 1),
            0,
            "gold still blocks the gap"
        );
    }
    for y in [47, 50] {
        assert_eq!(sim.terrain.dat_type(2, y, 1), 1);
        assert_eq!(sim.terrain.dat_type(4, y, 1), 1);
    }
}
