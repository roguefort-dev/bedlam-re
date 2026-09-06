//! Real Boot Camp pad dispatch and stack/collision transition.
use bedlam_core::{
    destroy::ObjectTypeTable,
    mission::{AngleTable, MissionSim, Terrain},
    weapon::CommandRecord,
};

#[test]
fn boot_camp_pressure_pad_lowers_all_eighteen_tiles() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../game-data/BEDLAM/EDITOR/ZONEA");
    if !root.exists() {
        return;
    }
    let read = |name: &str| std::fs::read(root.join(name)).unwrap();
    let terrain = Terrain::from_mission_bytes(
        &read("MISSION1.DAT"),
        &read("MISSION1.PAD"),
        &read("MISSIONA.CGR"),
    )
    .unwrap();
    let mut sim = MissionSim::new(terrain, AngleTable::from_thresholds(&[0; 64]).unwrap(), 1);
    let table = ObjectTypeTable::from_bdg_bytes(&read("MISSION1.BDG")).unwrap();
    assert!(sim.stage_destroy_family(&table, &read("MISSION1.POS"), &read("MISSION1.TRT"), 1, 1));
    assert!(sim.stage_pickup_surface(&read("MISSION1.TOT"), 1));
    sim.stage_elevators();
    sim.spawn_robot((5, 53, 2));
    let width = sim.terrain.size().0 as usize;
    let cells: Vec<_> = (51..53)
        .flat_map(|y| (2..11).map(move |x| y * width + x))
        .collect();
    let before: Vec<_> = cells
        .iter()
        .map(|&t| sim.mirror_words()[t * 8..t * 8 + 8].to_vec())
        .collect();
    let initial_height = sim.terrain.floor_z(5 * 32 + 16, 51 * 32 + 16, 63);
    sim.stage_command_record(CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 6 * 32,
        y: 53 * 32,
        z: 0,
    });
    sim.advance_frame();
    assert!(cells.iter().all(|&t| sim.elevator_bias()[t] == 1));
    for _ in 0..15 {
        sim.advance_frame();
    }
    for (&tile, old) in cells.iter().zip(before) {
        assert_eq!(&sim.mirror_words()[tile * 8..tile * 8 + 7], &old[1..]);
        assert_eq!(sim.mirror_words()[tile * 8 + 7], 0);
        assert_eq!(sim.elevator_bias()[tile], 16);
    }
    assert_eq!(
        sim.terrain.floor_z(5 * 32 + 16, 51 * 32 + 16, 63),
        initial_height - 32
    );
}
