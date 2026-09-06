//! Actual Boot Camp toxic floor through production-style world staging.
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
    Some(sim)
}

#[test]
fn boot_camp_toxic_tiles_drain_health_and_kill_without_movement() {
    let Some(mut sim) = boot_camp() else { return };
    let width = sim.terrain.size().0 as usize;
    let tile = sim
        .object_grid()
        .iter()
        .position(|&word| word == 0x7d2)
        .expect("Boot Camp has toxic floor");
    let (x, y) = ((tile % width) as i32, (tile / width) as i32);
    let robot = sim.spawn_robot((x, y, 2));
    sim.advance_frame();
    assert_eq!(sim.robots()[robot].hp, 4985);
    for _ in 0..332 {
        sim.advance_frame();
    }
    assert_eq!(sim.robots()[robot].hp, 5);
    assert!(sim.robots()[robot].alive);
    sim.advance_frame();
    assert_eq!(sim.robots()[robot].hp, 0);
    assert!(!sim.robots()[robot].alive);
    assert!(sim.debris_bank().iter().any(|d| d.kind != 0));
}
