//! Road sentry on the actual Boot Camp map.
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
fn boot_camp_road_sentry_opens_and_fires_at_the_neighboring_lane() {
    let Some(mut sim) = boot_camp() else { return };
    assert_eq!(
        sim.structures()
            .iter()
            .map(|s| (s.x, s.y, s.z))
            .collect::<Vec<_>>(),
        vec![(14, 15, 1), (11, 15, 1), (10, 33, 1)]
    );
    let cell = ((33 * sim.terrain.size().0 + 10) * 8 + 1) as usize;
    assert_eq!(sim.mirror_words()[cell], 1);
    sim.spawn_robot((12, 34, 1));
    let mut fired = false;
    for _ in 0..32 {
        sim.advance_frame();
        fired |= sim.enemy_bank().iter().any(|p| p.kind == 0x66);
    }
    assert!(fired, "road gun must produce shots through advance_frame");
    assert_eq!(sim.structures()[2].state, 7);
    assert_eq!(sim.structures()[2].frame, 9);
    assert!(sim.mirror_words()[cell] >= 20);
    assert!(sim.robots()[0].alive);
    assert_eq!(sim.robots()[0].hp, 5000);
}
