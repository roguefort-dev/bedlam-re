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

#[test]
fn boot_camp_pool_can_be_crossed_between_its_ramps() {
    let Some(mut sim) = boot_camp() else { return };
    sim.stage_elevators();
    sim.spawn_robot((10, 46, 2));
    // PAD 2, near platform, entrance ramp, exit ramp, far platform.
    // These are map waypoints, not a substitute for a live input journey.
    for (x, y) in [(10, 44), (9, 42), (9, 40), (12, 37), (14, 37)] {
        sim.stage_command_record(bedlam_core::weapon::CommandRecord {
            marker: 0,
            id: 0,
            spot: 0,
            flags: 1,
            x: x * 32 + 16,
            y: y * 32 + 16,
            z: 0,
        });
        for frame in 0..180 {
            sim.advance_frame();
            let r = &sim.robots()[0];
            assert!(r.alive, "died approaching {x},{y}");
            if frame > 0 && r.state == 0 {
                break;
            }
        }
        let r = &sim.robots()[0];
        let dx = (r.pos_x >> 8) - i32::from(x * 32 + 16);
        let dy = (r.pos_y >> 8) - i32::from(y * 32 + 16);
        assert!(dx * dx + dy * dy <= 32 * 32, "missed {x},{y}: {r:?}");
    }
    let health = sim.robots()[0].hp;
    assert!(
        health > 0 && health < 5000,
        "crossing must incur toxic damage"
    );
    assert_eq!(sim.robots()[0].z, 63, "reached the far platform");
    // Longer than the idle pool death budget: the exit must actually be safe.
    for _ in 0..334 {
        sim.advance_frame();
    }
    assert_eq!(sim.robots()[0].hp, health);
    assert!(sim.robots()[0].alive);
}
