//! Boot Camp beacon destruction and subsequent PAD16 entry, observed in DOSBox.
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
fn boot_camp_beacon_must_be_destroyed_before_the_robot_can_extract() {
    let Some(mut sim) = boot_camp() else { return };
    sim.configure_hints(1, 1, 0);
    sim.spawn_robot((16, 25, 5));
    let approach = bedlam_core::weapon::CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 17 * 32 + 24,
        y: 25 * 32 + 16,
        z: 0,
    };
    sim.stage_command_record(approach);
    sim.advance_frame();
    assert_eq!(sim.hints().active(), Some(14));
    for _ in 0..9 {
        sim.tick_hints();
    }
    sim.stage_command_record(approach);
    for _ in 0..30 {
        sim.advance_frame();
    }
    assert!(sim.order().is_none(), "intact tower must block PAD16");
    assert!(sim.hints().active().is_none());
    assert!(sim.robots()[0].pos_x >> 8 < 17 * 32);
    let beacon = sim.objects().iter().find(|o| o.slot == 96).unwrap();
    assert_eq!((beacon.x, beacon.y, beacon.z, beacon.hp), (17, 25, 5, 400));
    assert!(sim.resolve_object_impact(17 << 13, 25 << 13, 0, 400, true));
    assert_eq!(sim.take_destroy_score().0, 40);
    let tile = (25 * sim.terrain.size().0 + 17) as usize;
    for (z, word) in [(5, 1331), (6, 0), (7, 0)] {
        assert_eq!(sim.terrain.dat_type(17, 25, z), 0);
        assert_eq!(sim.mirror_words()[tile * 8 + z as usize], word);
        assert_eq!(sim.mirror_seen_bank()[tile * 8 + z as usize], 1);
    }
    assert_eq!(sim.terrain.pad_slot_at(17, 25, 4), Some(16));
    for _ in 0..30 {
        sim.advance_frame();
        if sim.beacon_tile_latch().is_some() {
            break;
        }
    }
    assert!(sim.robots()[0].alive);
    assert_eq!(sim.beacon_tile_latch(), Some((17, 25, 159)));
    for _ in 0..120 {
        sim.advance_frame();
        if sim.extraction_state().1 {
            break;
        }
    }
    assert_eq!(sim.extraction_state(), (1, true));
}
