//! Real Boot Camp pad dispatch and stack/collision transition.
use bedlam_core::{
    destroy::ObjectTypeTable,
    mission::{AngleTable, MissionSim, Terrain},
    weapon::CommandRecord,
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
    sim.stage_elevators();
    Some(sim)
}

#[test]
fn boot_camp_pressure_pad_lowers_all_eighteen_tiles() {
    let Some(mut sim) = boot_camp() else { return };
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

#[test]
fn boot_camp_blue_pad_raises_the_six_tile_lift() {
    let Some(mut sim) = boot_camp() else { return };
    assert_eq!(sim.terrain.pad_slot(2), Some((10, 46, 1)));
    sim.spawn_robot((10, 46, 2));
    let width = sim.terrain.size().0 as usize;
    let cells: Vec<_> = (44..46)
        .flat_map(|y| (9..12).map(move |x| y * width + x))
        .collect();
    let before: Vec<_> = cells
        .iter()
        .map(|&t| sim.mirror_words()[t * 8..t * 8 + 8].to_vec())
        .collect();
    sim.stage_command_record(CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 11 * 32,
        y: 46 * 32,
        z: 0,
    });
    sim.advance_frame();
    assert!(cells.iter().all(|&t| sim.elevator_bias()[t] == 0x81));
    for _ in 0..15 {
        sim.advance_frame();
    }
    for (&tile, old) in cells.iter().zip(before) {
        assert_eq!(&sim.mirror_words()[tile * 8 + 1..tile * 8 + 8], &old[..7]);
        assert_eq!(sim.elevator_bias()[tile], 0x90);
    }
}

#[test]
fn boot_camp_blue_pad_can_be_approached_from_the_platform() {
    let Some(mut sim) = boot_camp() else { return };
    sim.spawn_robot((10, 48, 2));
    sim.stage_command_record(CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 10 * 32 + 16,
        y: 44 * 32 + 16,
        z: 0,
    });
    for _ in 0..160 {
        sim.advance_frame();
    }
    let width = sim.terrain.size().0 as usize;
    assert_eq!(
        sim.elevator_bias()[44 * width + 9],
        0x90,
        "robot={:?}",
        sim.robots()[0]
    );
}

#[test]
fn boot_camp_green_pad_joins_both_lifts_into_a_crossable_platform() {
    let Some(mut sim) = boot_camp() else { return };
    assert_eq!(sim.terrain.pad_slot(3), Some((15, 37, 1)));
    sim.spawn_robot((14, 37, 2));
    let width = sim.terrain.size().0 as usize;
    let cells: Vec<_> = (35..40)
        .flat_map(|y| (16..20).map(move |x| (x, y * width + x)))
        .collect();
    let before: Vec<_> = cells
        .iter()
        .map(|&(_, t)| sim.mirror_words()[t * 8..t * 8 + 8].to_vec())
        .collect();
    sim.stage_command_record(CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 20 * 32 + 16,
        y: 37 * 32 + 16,
        z: 0,
    });
    for _ in 0..160 {
        sim.advance_frame();
    }
    for ((x, tile), old) in cells.into_iter().zip(before) {
        if x < 18 {
            assert_eq!(sim.elevator_bias()[tile], 0x90);
            assert_eq!(&sim.mirror_words()[tile * 8 + 1..tile * 8 + 8], &old[..7]);
        } else {
            assert_eq!(sim.elevator_bias()[tile], 16);
            assert_eq!(&sim.mirror_words()[tile * 8..tile * 8 + 7], &old[1..]);
            assert_eq!(sim.mirror_words()[tile * 8 + 7], 0);
        }
    }
    let r = &sim.robots()[0];
    assert!(r.alive);
    assert!(r.pos_x >> 8 >= 19 * 32, "did not cross both lifts: {r:?}");
    assert_eq!(r.z, 63);
}

#[test]
fn boot_camp_hidden_pad_lowers_five_levels_and_opens_the_scaffold_passage() {
    let Some(mut sim) = boot_camp() else { return };
    assert_eq!(sim.terrain.pad_slot(6), Some((5, 32, 0)));
    sim.spawn_robot((6, 32, 1));
    let width = sim.terrain.size().0 as usize;
    let cells: Vec<_> = (32..35).map(|y| y * width + 4).collect();
    let before: Vec<_> = cells
        .iter()
        .map(|&t| sim.mirror_words()[t * 8..t * 8 + 8].to_vec())
        .collect();
    sim.stage_command_record(CommandRecord {
        marker: 0,
        id: 0,
        spot: 0,
        flags: 1,
        x: 2 * 32 + 16,
        y: 32 * 32 + 16,
        z: 0,
    });
    for _ in 0..180 {
        sim.advance_frame();
    }
    for (tile, old) in cells.into_iter().zip(before) {
        assert_eq!(sim.elevator_bias()[tile], 80);
        assert_eq!(&sim.mirror_words()[tile * 8..tile * 8 + 3], &old[5..]);
        assert_eq!(&sim.mirror_words()[tile * 8 + 3..tile * 8 + 8], &[0; 5]);
    }
    let r = &sim.robots()[0];
    assert!(r.alive);
    assert!(
        r.pos_x >> 8 < 4 * 32,
        "did not cross lowered section: {r:?}"
    );
    assert_eq!(r.z, 31);
}
