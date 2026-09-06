//! Read-only campaign objective census and B-2 destruction lifecycle.
use bedlam_core::{
    destroy::ObjectTypeTable,
    mission::{AngleTable, MissionSim, Terrain},
};

fn mission(zone: u32, mission: u32) -> Option<MissionSim> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../game-data/BEDLAM/EDITOR")
        .join(format!("ZONE{}", (b'A' + zone as u8 - 1) as char));
    if !root.exists() {
        return None;
    }
    let read = |extension: &str| {
        std::fs::read(root.join(format!("MISSION{mission}.{extension}"))).unwrap()
    };
    let cgr = std::fs::read(root.join(format!("MISSION{}.CGR", (b'A' + zone as u8 - 1) as char)))
        .unwrap();
    let terrain = Terrain::from_mission_bytes(&read("DAT"), &read("PAD"), &cgr).unwrap();
    let mut sim = MissionSim::new(terrain, AngleTable::from_thresholds(&[0; 64]).unwrap(), 1);
    let table = ObjectTypeTable::from_bdg_bytes(&read("BDG")).unwrap();
    assert!(sim.stage_destroy_family(&table, &read("POS"), &read("TRT"), zone, 1));
    sim.set_mission_no(mission);
    sim.stage_objectives();
    Some(sim)
}

#[test]
fn campaign_targets_resolve_to_actual_pos_slots_and_pad_records() {
    for zone in 2..=6 {
        for number in 1..=5 {
            let Some(sim) = mission(zone, number) else {
                return;
            };
            for group in sim.objectives() {
                for &target in &group.targets {
                    if target == 5000 {
                        assert!(group.quota > 0);
                        assert!(sim.terrain.pad_slot(group.pad as usize).is_some());
                    } else {
                        assert!(
                            sim.objects().iter().any(|o| o.slot == target as u16),
                            "zone{zone} mission{number} missing POS{target}"
                        );
                    }
                }
            }
            let expected = sim
                .objectives()
                .iter()
                .map(|g| g.targets.len())
                .sum::<usize>();
            assert_eq!(sim.objective_radar_markers().len(), expected);
        }
    }
}

#[test]
fn b2_real_object_destruction_removes_marker_and_completes_primary_only() {
    let Some(mut sim) = mission(2, 2) else { return };
    assert_eq!(sim.objective_radar_markers().len(), 17);
    let object = *sim.objects().iter().find(|o| o.slot == 778).unwrap();
    assert_eq!((object.x, object.y, object.id), (4, 89, 73));
    assert!(!sim.primary_objective_complete());
    assert!(sim.resolve_object_impact(object.x << 13, object.y << 13, 0, object.hp, true));
    assert!(sim.primary_objective_complete());
    assert_eq!(sim.objective_cells(), (1, 32, 0));
    assert!(sim.objectives()[1..].iter().all(|g| !g.complete));
    assert_eq!(sim.objective_radar_markers().len(), 16);
    assert!(sim.objective_radar_markers().iter().all(|m| m.0 == 6));
    assert!(!sim.resolve_object_impact(object.x << 13, object.y << 13, 0, 100, true));
    assert_eq!(sim.objectives()[0].remaining, 0);
}
