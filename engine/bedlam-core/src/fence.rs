//! EXW delayed fence shutdown; see docs/RE-EXW-FENCE.md.
use crate::mission::{MissionSim, PICKUP_FLOOR_WORD};

impl MissionSim {
    fn fence_payload(&self, id: i32) -> u32 {
        if self.network_mode == 2 {
            return 0;
        }
        match (self.zone, self.mission_no, id) {
            (1, 1, 0x85) => 0x847f,
            (1, 1, 0x86) => 0x83,
            (2, 2, 0x70..=0x75) => [0x67, 0x68, 0x69, 0x6d, 0x64, 0x65][id as usize - 0x70],
            _ => 0,
        }
    }

    /// EXW 0x41f215 object pass: generator and linked fence origin markers.
    /// Retained immutable ids include destroyed generators, preserving the
    /// initial target-type list built by 0x41f867. Destroyed records themselves
    /// cannot match the original full flag-bearing id comparisons.
    pub fn fence_radar_objects(&self) -> Vec<(usize, i32, i32)> {
        let mut targets = Vec::new();
        for object in &self.objects {
            let payload = self.fence_payload(object.id);
            for id in [payload & 0xff, (payload >> 8) & 0xff] {
                if id != 0 && !targets.contains(&(id as i32)) {
                    targets.push(id as i32);
                }
            }
        }
        self.objects
            .iter()
            .filter_map(|object| {
                if object.destroyed {
                    return None;
                }
                let icon = if self.fence_payload(object.id) != 0 {
                    9
                } else if targets.contains(&object.id) {
                    10
                } else {
                    return None;
                };
                Some((icon, object.x, object.y))
            })
            .collect()
    }

    pub(crate) fn schedule_fence_shutdown(&mut self, id: i32) {
        let payload = self.fence_payload(id);
        if payload == 0 {
            return;
        }
        let slot = self.fence_timers.iter().position(|t| t.0 == 0).unwrap_or(0);
        self.fence_timers[slot] = (payload, 8);
        // The producer's chase-camera call remains presentation work.
    }

    pub(crate) fn fence_tick(&mut self) {
        for slot in 0..self.fence_timers.len() {
            let (payload, countdown) = self.fence_timers[slot];
            if payload == 0 {
                continue;
            }
            if countdown != 0 {
                self.fence_timers[slot].1 -= 1;
                continue;
            }
            for id in [payload as i32 & 0xff, (payload as i32) >> 8] {
                if id == 0 {
                    continue;
                }
                for i in 0..self.objects.len() {
                    let object = self.objects[i];
                    // Original compares the complete id/flags dword.
                    if object.id != id || object.destroyed {
                        continue;
                    }
                    self.objects[i].destroyed = true;
                    self.terrain.dat_write(object.x, object.y, object.z, 0);
                    let (w, h) = self.terrain.size();
                    if object.x < 0
                        || object.y < 0
                        || object.x >= w
                        || object.y >= h
                        || !(0..8).contains(&object.z)
                    {
                        continue;
                    }
                    // Direct raw-set indexing at 0x422d47.
                    if let Some(&floor) = PICKUP_FLOOR_WORD.get(self.zone as usize) {
                        let cell = ((object.y * w + object.x) * 8 + object.z) as usize;
                        if cell < self.mirror_words.len() {
                            self.write_mirror_cell(cell, floor as u16, 1);
                        }
                    }
                }
            }
            self.fence_timers[slot].0 = 0;
            // Original SFX 0x22/channel 3 remains unwired.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destroy::{ObjectInstance, ObjectTypeTable};
    use crate::mission::{AngleTable, Terrain};

    fn staged() -> MissionSim {
        let terrain = Terrain::from_parts(8, 8, vec![1; 8 * 8 * 8], vec![]).unwrap();
        let angles = AngleTable::from_thresholds(&[0; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, 1);
        let pos = vec![0xff; 2000 * 16];
        assert!(sim.stage_destroy_family(&ObjectTypeTable::default(), &pos, &[0, 0], 1, 1));
        sim.mirror_words.fill(99);
        sim.observe_terrain_writes();
        for (i, id) in [0x7f, 0x84, 0x83, 0x7f].into_iter().enumerate() {
            sim.objects.push(ObjectInstance {
                x: i as i32 + 1,
                y: 2,
                z: 1,
                id,
                destroyed: i == 3,
                hp: 123,
                slot: i as u16,
            });
        }
        sim
    }

    #[test]
    fn shutdown_waits_then_clears_only_linked_origins_without_damage_tail() {
        let mut sim = staged();
        sim.schedule_fence_shutdown(0x85);
        for _ in 0..8 {
            sim.fence_tick();
        }
        assert!(!sim.objects[0].destroyed);
        assert!(sim.take_terrain_writes().is_empty());
        sim.fence_tick();
        assert!(sim.objects[0].destroyed && sim.objects[1].destroyed);
        assert!(!sim.objects[2].destroyed);
        assert_eq!(sim.objects[0].hp, 123);
        assert_eq!(sim.score_pending, 0);
        assert!(sim.debris.iter().all(|d| !d.active));
        assert_eq!(
            sim.take_terrain_writes(),
            vec![(137, 0x48f, 1), (145, 0x48f, 1)]
        );
        assert_eq!(sim.mirror_words[138], 99);
        assert_eq!(sim.terrain.dat_type(1, 2, 1), 0);
        assert_eq!(sim.terrain.dat_type(1, 2, 2), 1);
        sim.fence_tick();
        assert!(sim.take_terrain_writes().is_empty());
    }

    #[test]
    fn b2_radar_links_survive_generator_death_until_delayed_shutdown() {
        let mut sim = staged();
        sim.zone = 2;
        sim.mission_no = 2;
        for (object, id) in sim.objects.iter_mut().zip([0x70, 0x67, 0x68, 0x71]) {
            object.id = id;
            object.destroyed = false;
        }
        assert_eq!(
            sim.fence_radar_objects(),
            vec![(9, 1, 2), (10, 2, 2), (10, 3, 2), (9, 4, 2)]
        );
        for (id, payload) in (0x70..=0x75).zip([0x67, 0x68, 0x69, 0x6d, 0x64, 0x65]) {
            assert_eq!(sim.fence_payload(id), payload);
        }
        sim.objects[0].destroyed = true;
        sim.schedule_fence_shutdown(0x70);
        assert_eq!(
            sim.fence_radar_objects(),
            vec![(10, 2, 2), (10, 3, 2), (9, 4, 2)]
        );
        for _ in 0..9 {
            sim.fence_tick();
        }
        assert_eq!(sim.fence_radar_objects(), vec![(10, 3, 2), (9, 4, 2)]);
        assert!(!sim.objects[2].destroyed);
        sim.network_mode = 2;
        assert!(sim.fence_radar_objects().is_empty());
        assert_eq!(sim.fence_payload(0x70), 0);
    }

    #[test]
    fn timer_allocation_gates_and_secondary_generator() {
        let mut sim = staged();
        sim.network_mode = 2;
        sim.schedule_fence_shutdown(0x85);
        assert!(sim.fence_timers.iter().all(|t| t.0 == 0));
        sim.network_mode = 0;
        sim.mission_no = 2;
        sim.schedule_fence_shutdown(0x85);
        assert!(sim.fence_timers.iter().all(|t| t.0 == 0));
        sim.mission_no = 1;
        for _ in 0..32 {
            sim.schedule_fence_shutdown(0x85);
        }
        sim.schedule_fence_shutdown(0x86);
        assert_eq!(sim.fence_timers[0], (0x83, 8));
        assert_eq!(sim.fence_timers[31], (0x847f, 8));
        for _ in 0..9 {
            sim.fence_tick();
        }
        assert!(sim.objects[2].destroyed);
        assert!(sim.fence_timers.iter().all(|t| t.0 == 0));
    }
}
