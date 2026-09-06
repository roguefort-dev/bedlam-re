//! EXW 0x44889a staging, 0x448b80 notification and 0x41f527 radar.
//! Provenance: docs/RE-EXW-MISSIONVIEW.md objective audits.
use crate::mission::MissionSim;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectiveGroup {
    /// Original POS slots; -1 is a completed target, 5000 a rescue quota.
    pub targets: Vec<i16>,
    pub remaining: u16,
    pub quota: i16,
    pub pad: i16,
    pub complete: bool,
}

impl MissionSim {
    /// Initialize original single-player objective groups after world staging.
    pub fn stage_objectives(&mut self) {
        self.objectives = Default::default();
        self.objective_count = 0;
        if self.network_mode == 2 || self.zone == 1 {
            return;
        }
        if self.zone == 7 {
            for i in 0..self.objects.len() {
                if (0x44..=0x47).contains(&self.objects[i].id) {
                    self.objective_count += 1;
                    self.objective_height(i, false);
                }
            }
            return;
        }
        if !(2..=6).contains(&self.zone) || !(1..=5).contains(&self.mission_no) {
            return;
        }
        let index = ((self.zone - 2) * 5 + self.mission_no - 1) as usize;
        let mut stream = crate::objective_data::CAMPAIGN[index].iter().copied();
        let mut group = 0;
        while let Some(value) = stream.next() {
            match value {
                -1 => break,
                -2 => group += 1,
                slot => {
                    let g = &mut self.objectives[group];
                    g.targets.push(slot);
                    g.remaining += 1;
                    if slot == 5000 {
                        g.quota = stream.next().expect("verified quota stream");
                        g.pad = stream.next().expect("verified pad stream");
                    }
                }
            }
        }
        for group in 0..6 {
            if self.objectives[group].quota != 0 {
                continue;
            }
            for target in 0..self.objectives[group].targets.len() {
                let slot = self.objectives[group].targets[target];
                if let Some(index) = self
                    .objects
                    .iter()
                    .position(|o| i32::from(o.slot) == i32::from(slot))
                {
                    self.objective_height(index, false);
                }
            }
        }
    }

    pub fn objectives(&self) -> &[ObjectiveGroup; 6] {
        &self.objectives
    }

    /// The caller's group-zero gate (0x4486ec), not the all-groups status.
    pub fn primary_objective_complete(&self) -> bool {
        self.objectives[0].complete
    }

    fn objective_height(&mut self, index: usize, clear: bool) {
        let object = self.objects[index];
        let Some(kind) = self.object_types.get((object.id & 0x3fff) as usize) else {
            return;
        };
        let height = if clear {
            (0, 0)
        } else {
            (object.z as u8, (object.z as u8).wrapping_add(kind.d as u8))
        };
        let (w, h) = self.terrain.size();
        for y in object.y..object.y + i32::from(kind.h) {
            for x in object.x..object.x + i32::from(kind.w) {
                if (0..w).contains(&x) && (0..h).contains(&y) {
                    if let Some(cell) = self.mirror_heights.get_mut((y * w + x) as usize) {
                        *cell = height;
                    }
                }
            }
        }
    }

    pub(crate) fn objective_notify(&mut self, index: usize) {
        if self.network_mode == 2 {
            return;
        }
        if self.zone == 7 {
            if !(0x44..=0x47).contains(&(self.objects[index].id & 0x3fff)) {
                return;
            }
            self.objective_count -= 1;
            self.objective_height(index, true);
            if self.objective_count == 0 {
                self.objective_phase = 3;
                self.objective_blink = 32;
                self.objective_light = 50;
            }
            return;
        }
        self.objective_notification(i32::from(self.objects[index].slot));
    }

    pub(crate) fn objective_notification(&mut self, slot: i32) {
        if self.network_mode == 2 || self.zone == 1 || self.zone == 7 {
            return;
        }
        let mut changed = false;
        for group in 0..6 {
            for target in 0..self.objectives[group].targets.len() {
                if i32::from(self.objectives[group].targets[target]) != slot {
                    continue;
                }
                if slot == 5000 && self.poi_escapes < i32::from(self.objectives[group].quota) {
                    continue;
                }
                if slot != 5000 {
                    if let Some(index) = self.objects.iter().position(|o| i32::from(o.slot) == slot)
                    {
                        self.objective_height(index, true);
                    }
                }
                let g = &mut self.objectives[group];
                g.targets[target] = -1;
                g.remaining -= 1;
                changed = true;
                if g.remaining == 0 {
                    g.complete = true;
                    self.objective_phase = if group == 0 { 1 } else { 2 };
                    self.objective_blink = 32;
                } else if slot != 5000 {
                    self.objective_phase = 4;
                    self.objective_blink = 32;
                }
            }
        }
        if changed && self.objectives.iter().all(|g| g.remaining == 0) {
            self.objective_phase = 3;
            self.objective_blink = 32;
        }
        // Original message/audio calls 0x4239ef remain presentation work.
    }

    /// Objective glyphs in absolute radar coordinates, in original group order.
    pub fn objective_radar_markers(&self) -> Vec<(usize, i32, i32)> {
        let mut result = Vec::new();
        if self.zone == 1 {
            return result;
        }
        for (index, group) in self.objectives.iter().enumerate() {
            let icon = if index == 0 { 5 } else { 6 };
            if group.quota != 0 && group.targets.first().is_some_and(|&t| t != -1) {
                if let Some((x, y, _)) = usize::try_from(group.pad)
                    .ok()
                    .and_then(|slot| self.terrain.pad_slot(slot))
                {
                    result.push((icon, 2 * x, 2 * y));
                }
                continue;
            }
            for &slot in &group.targets {
                let Some(object) = self
                    .objects
                    .iter()
                    .find(|o| i32::from(o.slot) == i32::from(slot))
                else {
                    continue;
                };
                let Some(kind) = self.object_types.get((object.id & 0x3fff) as usize) else {
                    continue;
                };
                result.push((
                    icon,
                    2 * object.x + i32::from(kind.w),
                    2 * object.y + i32::from(kind.h),
                ));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::{AngleTable, Terrain};

    fn sim(zone: u32, mission: u32) -> MissionSim {
        let terrain = Terrain::from_parts(4, 4, vec![0; 4 * 4 * 8], vec![]).unwrap();
        let mut sim = MissionSim::new(terrain, AngleTable::from_thresholds(&[0; 64]).unwrap(), 1);
        sim.stage_zone_set(zone);
        sim.set_mission_no(mission);
        sim.stage_objectives();
        sim
    }

    #[test]
    fn primary_secondary_partial_and_all_done_are_distinct() {
        let mut sim = sim(2, 2);
        assert_eq!(
            std::array::from_fn::<_, 6, _>(|i| sim.objectives[i].remaining),
            [1, 5, 5, 1, 3, 2]
        );
        sim.objective_notification(408);
        assert_eq!(sim.objective_cells(), (4, 32, 0));
        sim.objective_notification(408);
        assert_eq!(sim.objectives[1].remaining, 4);
        sim.objective_notification(778);
        assert!(sim.primary_objective_complete());
        assert_eq!(sim.objective_cells(), (1, 32, 0));
        assert!(!sim.objectives[1].complete);
        for slot in [407, 406, 409, 421] {
            sim.objective_notification(slot);
        }
        assert_eq!(sim.objective_cells(), (2, 32, 0));
        for slot in [963, 962, 503, 594, 779, 405, 609, 596, 598, 774, 775] {
            sim.objective_notification(slot);
        }
        assert!(sim
            .objectives
            .iter()
            .all(|g| g.complete && g.remaining == 0));
        assert_eq!(sim.objective_cells(), (3, 32, 0));
    }

    #[test]
    fn rescue_quota_requires_enough_escapes_and_notifies_once() {
        let mut sim = sim(5, 1);
        assert_eq!(sim.objectives[0].targets, [5000]);
        assert_eq!((sim.objectives[0].quota, sim.objectives[0].pad), (40, 0));
        sim.poi_escapes = 39;
        sim.objective_notification(5000);
        assert!(!sim.primary_objective_complete());
        assert_eq!(sim.objective_cells(), (0, 0, 0));
        sim.poi_escapes = 40;
        sim.objective_notification(5000);
        assert!(sim.primary_objective_complete());
        assert_eq!(sim.objective_cells(), (1, 32, 0));
        sim.objective_notification(5000);
        assert_eq!(sim.objectives[0].remaining, 0);
        assert_eq!(sim.objectives[0].targets, [-1]);
    }

    #[test]
    fn final_zone_counts_object_ids_and_clears_footprint_on_destruction() {
        use crate::destroy::{ObjectType, ObjectTypeTable};
        let mut sim = sim(7, 1);
        let mut types = vec![ObjectType::default(); 0x48];
        types[0] = ObjectType {
            w: 1,
            h: 1,
            d: 1,
            hp: 1,
            kind: 0x44,
            ..Default::default()
        };
        types[0x44] = ObjectType {
            w: 1,
            h: 1,
            d: 2,
            hp: 1,
            kind: 0,
            ..Default::default()
        };
        let mut pos = vec![0xff; 2000 * 16];
        for (slot, id) in [0, 0x44].into_iter().enumerate() {
            for (field, value) in [slot as i32 + 1, 1, 1, id].into_iter().enumerate() {
                pos[16 * slot + 4 * field..16 * slot + 4 * field + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        assert!(sim.stage_destroy_family(&ObjectTypeTable { rows: types }, &pos, &[0, 0], 7, 1));
        sim.stage_objectives();
        assert_eq!(sim.objective_count, 1);
        assert_eq!(sim.mirror_heights[5], (0, 0));
        assert_eq!(sim.mirror_heights[6], (1, 3));
        assert!(sim.resolve_object_impact(1 << 13, 1 << 13, 0, 1, false));
        assert_eq!(sim.objective_count, 1, "kind code is not an objective id");
        assert!(sim.resolve_object_impact(2 << 13, 1 << 13, 0, 1, false));
        assert_eq!(sim.objective_count, 0);
        assert_eq!(sim.mirror_heights[6], (0, 0));
        assert_eq!(sim.objective_cells(), (3, 32, 50));
    }

    #[test]
    fn every_campaign_stream_has_six_bounded_groups() {
        for zone in 2..=6 {
            for mission in 1..=5 {
                let sim = sim(zone, mission);
                for group in sim.objectives() {
                    assert!((1..=12).contains(&group.targets.len()));
                    assert_eq!(group.remaining as usize, group.targets.len());
                    assert!(group
                        .targets
                        .iter()
                        .all(|&s| (0..2000).contains(&s) || s == 5000));
                    assert!(!group.complete);
                }
            }
        }
        let mut sim = sim(2, 2);
        sim.network_mode = 2;
        sim.stage_objectives();
        assert!(sim.objectives.iter().all(|g| g.targets.is_empty()));
    }
}
