//! Scripted moving stacks, EXW 0x4223b8/0x423081. See RE-EXW-ELEVATORS.md.
use crate::mission::MissionSim;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Elevator {
    state: u16,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    levels: u8,
}

impl MissionSim {
    pub fn stage_elevators(&mut self) {
        self.elevators.clear();
        let (w, h) = self.terrain.size();
        self.elevator_targets = vec![0; (w * h) as usize];
        self.elevator_bias = vec![0; (w * h) as usize];
        if self.zone != 1 || self.mission_no != 1 || self.network_mode == 2 {
            return;
        }
        for &(zone, mission, index, state, x, y, w, h) in crate::claim_rects::RECTS {
            if zone == 1 && mission == 1 {
                self.elevators.push(Elevator {
                    state,
                    x: x.into(),
                    y: y.into(),
                    w: w.into(),
                    h: h.into(),
                    levels: [1, 1, 1, 1, 5, 1, 2][index as usize],
                });
            }
        }
    }

    pub fn elevator_bias(&self) -> &[u8] {
        &self.elevator_bias
    }

    pub(crate) fn pad_elevator_trigger(&mut self, robot: usize) {
        if self.zone != 1 || self.mission_no != 1 || self.network_mode == 2 {
            return;
        }
        let r = &self.robots[robot];
        let Some(slot) = self
            .terrain
            .pad_slot_at(r.pos_x >> 13, r.pos_y >> 13, r.z >> 5)
        else {
            return;
        };
        match slot {
            1 => self.set_elevator(0, 2),
            2 => self.set_elevator(1, 1),
            3 => {
                self.set_elevator(2, 1);
                self.set_elevator(3, 2);
            }
            4..=7 => self.set_elevator(4, 2),
            8 => self.set_elevator(5, 1),
            9 => self.set_elevator(6, 2),
            _ => {}
        }
    }

    fn set_elevator(&mut self, index: usize, wanted: u16) {
        let Some(rect) = self.elevators.get(index).copied() else {
            return;
        };
        if rect.state == wanted || rect.state >= 3 {
            return;
        }
        let (w, h) = self.terrain.size();
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                if !(0..w).contains(&x) || !(0..h).contains(&y) {
                    continue;
                }
                let tile = (y * w + x) as usize;
                if self.elevator_bias[tile] & 0x7f == self.elevator_targets[tile] {
                    self.elevator_targets[tile] = rect.levels << 4;
                    self.elevator_bias[tile] = if wanted == 1 { 0x80 } else { 0 };
                }
            }
        }
        self.elevators[index].state = wanted;
    }

    pub(crate) fn elevator_tick(&mut self) {
        let (w, h) = self.terrain.size();
        for index in 0..self.elevators.len() {
            let rect = self.elevators[index];
            for y in rect.y..rect.y + rect.h {
                for x in rect.x..rect.x + rect.w {
                    if !(0..w).contains(&x) || !(0..h).contains(&y) {
                        continue;
                    }
                    let tile = (y * w + x) as usize;
                    let old = self.elevator_bias[tile];
                    if old & 0x7f == self.elevator_targets[tile]
                        || tile * 8 + 7 >= self.mirror_words.len()
                    {
                        continue;
                    }
                    let raising = old & 0x80 != 0;
                    let top = if raising { 6 } else { 7 };
                    let level = (0..=top)
                        .rev()
                        .find(|&z| self.mirror_words[tile * 8 + z] != 0)
                        .unwrap_or(0);
                    let frame = old & 15;
                    self.terrain.dat_write(
                        x,
                        y,
                        (level + usize::from(raising)) as i32,
                        if raising {
                            0x40 + 2 * frame
                        } else {
                            0x5f - 2 * frame
                        },
                    );
                    let next = (old & 0x80) | ((old & 0x7f) + 1);
                    self.elevator_bias[tile] = next;
                    if next & 15 != 0 {
                        continue;
                    }
                    let highest = (0..8)
                        .rev()
                        .find(|&z| self.mirror_words[tile * 8 + z] != 0)
                        .map(|z| z as i32)
                        .unwrap_or(-1);
                    if raising {
                        self.terrain.dat_write(x, y, highest + 1, 1);
                        if highest < 6 {
                            self.terrain.dat_write(x, y, highest + 2, 0);
                        }
                        for z in (1..8).rev() {
                            self.write_mirror_cell(
                                tile * 8 + z,
                                self.mirror_words[tile * 8 + z - 1],
                                self.mirror_seen[tile * 8 + z - 1],
                            );
                        }
                        if x + 1 < w
                            && y + 1 < h
                            && self.elevator_targets[tile + 1] != 0
                            && self.elevator_targets[tile + w as usize] != 0
                        {
                            // EXW clears the bottom word only; keeps its seen byte.
                            self.write_mirror_cell(tile * 8, 0, self.mirror_seen[tile * 8]);
                        }
                    } else {
                        if highest > 0 {
                            self.terrain.dat_write(x, y, highest, 0);
                        }
                        self.terrain.dat_write(x, y, 7, 0);
                        for z in 0..7 {
                            self.write_mirror_cell(
                                tile * 8 + z,
                                self.mirror_words[tile * 8 + z + 1],
                                self.mirror_seen[tile * 8 + z + 1],
                            );
                        }
                        self.write_mirror_cell(tile * 8 + 7, 0, 0);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::{AngleTable, Terrain};

    fn staged(levels: u8) -> MissionSim {
        let terrain = Terrain::from_parts(8, 8, vec![1; 8 * 8 * 8], vec![]).unwrap();
        let angles = AngleTable::from_thresholds(&[0; 64]).unwrap();
        let mut sim = MissionSim::new(terrain, angles, 1);
        sim.stage_elevators();
        sim.elevators = vec![Elevator {
            state: 1,
            x: 2,
            y: 2,
            w: 2,
            h: 2,
            levels,
        }];
        sim.mirror_words = (0..64)
            .flat_map(|_| [10, 20, 30, 40, 50, 60, 70, 80])
            .collect();
        sim.mirror_seen = (0..64).flat_map(|_| [1, 0, 1, 0, 1, 0, 1, 0]).collect();
        sim.observe_terrain_writes();
        sim
    }

    #[test]
    fn lowering_moves_only_at_sixteen_and_raising_restores_upper_stack() {
        let mut sim = staged(1);
        let t = 2 * 8 + 2;
        let unrelated = sim.mirror_words[..8].to_vec();
        sim.set_elevator(0, 2);
        sim.elevator_tick();
        assert_eq!(sim.elevator_bias[t], 1);
        assert_eq!(sim.terrain.dat_type(2, 2, 7), 0x5f);
        assert!(sim.take_terrain_writes().is_empty());
        for _ in 0..14 {
            sim.elevator_tick();
        }
        assert_eq!(sim.mirror_words[t * 8], 10);
        sim.elevator_tick();
        assert_eq!(
            &sim.mirror_words[t * 8..t * 8 + 8],
            &[20, 30, 40, 50, 60, 70, 80, 0]
        );
        assert_eq!(
            &sim.mirror_seen[t * 8..t * 8 + 8],
            &[0, 1, 0, 1, 0, 1, 0, 0]
        );
        assert_eq!(sim.terrain.dat_type(2, 2, 7), 0);
        sim.take_terrain_writes();
        sim.elevator_tick();
        assert!(sim.take_terrain_writes().is_empty());
        sim.set_elevator(0, 1);
        sim.elevator_tick();
        assert_eq!(sim.terrain.dat_type(2, 2, 7), 0x40);
        for _ in 0..15 {
            sim.elevator_tick();
        }
        assert_eq!(
            &sim.mirror_words[t * 8..t * 8 + 8],
            &[0, 20, 30, 40, 50, 60, 70, 80]
        );
        assert_eq!(
            sim.mirror_words[(3 * 8 + 3) * 8],
            20,
            "edge keeps bottom when both neighbors are not tagged"
        );
        assert_eq!(&sim.mirror_words[..8], unrelated);
    }

    #[test]
    fn raising_an_empty_stack_stamps_ground_without_a_ninth_plane() {
        let mut sim = staged(1);
        sim.mirror_words.fill(0);
        sim.elevators[0].state = 2;
        sim.set_elevator(0, 1);
        for _ in 0..16 {
            sim.elevator_tick();
        }
        assert_eq!(sim.terrain.dat_type(2, 2, 0), 1);
        assert_eq!(sim.terrain.dat_type(2, 2, 1), 0);
        assert_eq!(sim.elevator_bias[18], 0x90);
        assert!(sim.mirror_words.iter().all(|&word| word == 0));
    }

    #[test]
    fn five_level_motion_and_in_flight_requests_keep_original_semantics() {
        let mut sim = staged(5);
        let t = 2 * 8 + 2;
        sim.set_elevator(0, 2);
        sim.elevator_tick();
        sim.set_elevator(0, 1);
        assert_eq!(
            sim.elevator_bias[t], 1,
            "busy tiles are not restarted or reversed"
        );
        assert_eq!(
            sim.elevators[0].state, 1,
            "rectangle state still accepts request"
        );
        for _ in 0..79 {
            sim.elevator_tick();
        }
        assert_eq!(sim.elevator_bias[t], 80);
        assert_eq!(
            &sim.mirror_words[t * 8..t * 8 + 8],
            &[60, 70, 80, 0, 0, 0, 0, 0]
        );
        sim.take_terrain_writes();
        sim.set_elevator(0, 1);
        sim.elevator_tick();
        assert!(
            sim.take_terrain_writes().is_empty(),
            "same-state requests are no-ops"
        );
    }
}
