//! Teleporter/elevator runtime, EXW 0x42034c; docs/RE-EXW-RIDES.md.
use crate::{destroy::WATER_RANGE, mission::MissionSim};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ride {
    pub pad_slot: usize,
    pub marker: (i32, i32, i32),
    pub destination: (i32, i32, i32),
    pub countdown: u8,
    pub rider: Option<usize>,
}

impl MissionSim {
    /// The verified Boot Camp branch of FUN_00425da4.
    pub fn stage_rides(&mut self) {
        self.rides.clear();
        if self.zone != 1 || self.mission_no != 1 || self.network_mode == 2 {
            return;
        }
        for slot in [0, 10, 11, 12, 13, 14, 15] {
            let Some(marker) = self.terrain.pad_slot(slot) else {
                break;
            };
            let destination = match slot {
                0 => (8, 57, 2),
                15 => (14, 32, 1),
                _ => (8, 26, 5),
            };
            self.rides.push(Ride {
                pad_slot: slot,
                marker,
                destination,
                countdown: 0,
                rider: None,
            });
        }
    }

    pub fn rides(&self) -> &[Ride] {
        &self.rides
    }

    pub(crate) fn pad_ride_trigger(&mut self, robot: usize) {
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
        let Some(ride) = self.rides.iter_mut().find(|ride| ride.pad_slot == slot) else {
            return;
        };
        if ride.rider.is_some() {
            return;
        }
        ride.rider = Some(robot);
        ride.countdown = 10;
        let r = &mut self.robots[robot];
        r.state = 2;
        r.stop_dist = 0;
        r.target = None;
        r.pos_x = (ride.marker.0 << 13) + 0x1000;
        r.pos_y = (ride.marker.1 << 13) + 0x1000;
    }

    pub(crate) fn ride_tick(&mut self) {
        for i in 0..self.rides.len() {
            if self.rides[i].countdown == 0 {
                continue;
            }
            self.rides[i].countdown -= 1;
            if self.rides[i].countdown != 0 {
                continue;
            }
            let Some(robot) = self.rides[i].rider.take() else {
                continue;
            };
            let (x, y, z) = self.rides[i].destination;
            let (w, h) = self.terrain.size();
            if (0..w).contains(&x) && (0..h).contains(&y) {
                let tile = (y * w + x) as usize;
                if self.platform_strength.get(tile).copied().unwrap_or(0) != 0 {
                    self.platform_strength[tile] = 0;
                    self.object_grid[tile] = 0;
                    if let Some(&base) = WATER_RANGE.get(self.zone as usize) {
                        for level in 0..8 {
                            let word = self
                                .mirror_words
                                .get(tile * 8 + level)
                                .copied()
                                .unwrap_or(0) as i32;
                            if (base..base + 14).contains(&word) {
                                self.z_structure_write(x, y, level as i32, 0, 0);
                                break;
                            }
                        }
                    }
                }
            }
            let height = self.terrain.floor_z(x << 5, y << 5, z * 32 - 1);
            let r = &mut self.robots[robot];
            r.pos_x = x << 13;
            r.pos_y = y << 13;
            r.z = height;
            r.probe_z.fill(height as u16);
            r.state = 0;
        }
    }
}
