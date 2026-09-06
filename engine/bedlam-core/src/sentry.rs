//! TRT pop-up guns, EXW 0x417264/0x417698 and 0x412307.
//! Branch and coordinate provenance: docs/RE-EXW-SENTRIES.md.
use crate::{
    mission::MissionSim,
    weapon::{weapon_damage, EnemyProjectile},
};

impl MissionSim {
    /// Loader stamps, after TOT initialization so the initial frame survives it.
    pub fn stamp_sentries(&mut self) {
        for i in 0..self.structures.len() {
            let s = self.structures[i];
            if s.active {
                self.terrain.dat_write(s.x, s.y, s.z, 0x66);
                self.sentry_word(i, s.frame);
            }
        }
    }

    fn sentry_word(&mut self, i: usize, frame: i32) {
        let s = self.structures[i];
        let (w, h) = self.terrain.size();
        if !(0..w).contains(&s.x) || !(0..h).contains(&s.y) || !(0..8).contains(&s.z) {
            return;
        }
        let cell = ((s.y * w + s.x) * 8 + s.z) as usize;
        if cell < self.mirror_words.len() {
            self.write_mirror_cell(cell, (frame + 1) as u16, self.mirror_seen[cell]);
        }
    }

    pub(crate) fn sentry_tick(&mut self) {
        for i in 0..self.structures.len() {
            let mut s = self.structures[i];
            if !s.active {
                continue;
            }
            let (x, y) = (s.x * 32 + 16, s.y * 32 + 16);
            let nearest = self
                .robots
                .iter()
                .filter(|r| r.alive)
                .map(|r| {
                    let dx = x - (r.pos_x >> 8);
                    let dy = y - (r.pos_y >> 8);
                    (
                        dx.abs().max(dy.abs()) + (dx.abs().min(dy.abs()) >> 1),
                        dx,
                        dy,
                    )
                })
                .min_by_key(|r| r.0);
            if let Some((_, dx, dy)) = nearest.filter(|r| r.0 < 129) {
                if s.state == 1 {
                    s.state = 2;
                } else if (5..9).contains(&s.state) {
                    s.state = if dx.abs() >= dy.abs() {
                        if dx >= 0 {
                            8
                        } else {
                            7
                        }
                    } else if dy >= 0 {
                        5
                    } else {
                        6
                    };
                }
            } else if s.state != 1 && s.state != 4 {
                s.state = if s.frame == 7 { 4 } else { 3 };
            }
            self.structures[i] = s;
            let mut next = None;
            let mut wrap = false;
            match s.state {
                2 => {
                    if s.frame < 7 {
                        next = Some(s.frame + 1);
                    } else {
                        self.structures[i].state = 6;
                    }
                }
                3 => {
                    if s.frame < 8 {
                        self.structures[i].state = 4;
                    } else if s.frame < 12 {
                        next = Some(s.frame - 1);
                    } else {
                        next = Some(s.frame + 1);
                        wrap = true;
                    }
                }
                4 => {
                    if s.frame > 0 {
                        next = Some(s.frame - 1);
                        if s.frame == 1 {
                            self.structures[i].state = 1;
                        }
                    }
                }
                5..=8 => {
                    let desired = [11, 7, 9, 13][(s.state - 5) as usize];
                    if s.frame == desired {
                        self.sentry_fire(i);
                    }
                    let counter = self.structures[i].fire_counter;
                    if s.frame == desired && counter != 0 {
                        let base = [0x16, 0xe, 0x12, 0x1a][(s.state - 5) as usize];
                        self.sentry_word(i, counter + base);
                        self.structures[i].fire_counter =
                            if counter == 4 { 1 } else { counter + 1 };
                    } else {
                        let delta = match s.state {
                            5 if s.frame < 11 => 1,
                            5 if s.frame >= 12 => -1,
                            6 if s.frame > 7 && s.frame < 12 => -1,
                            6 if s.frame > 11 => 1,
                            7 if s.frame > 9 && s.frame < 14 => -1,
                            7 if s.frame > 13 || s.frame < 9 => 1,
                            8 if s.frame > 8 && s.frame < 13 => 1,
                            8 if s.frame > 13 || s.frame < 9 => -1,
                            _ => 0,
                        };
                        if delta != 0 {
                            next = Some(s.frame + delta);
                            wrap = true;
                        }
                    }
                }
                _ => {}
            }
            if let Some(mut frame) = next {
                if wrap {
                    frame = match frame {
                        15 => 7,
                        6 => 14,
                        _ => frame,
                    };
                }
                self.structures[i].frame = frame;
                self.sentry_word(i, frame);
            }
        }
    }

    fn sentry_fire(&mut self, i: usize) {
        let s = self.structures[i];
        let (x, y) = (s.x * 32 + 16, s.y * 32 + 16);
        let lane = self.robots.iter().any(|r| {
            let (rx, ry) = (r.pos_x >> 8, r.pos_y >> 8);
            ((s.z - ((r.z >> 8) + 31)) >> 5).abs() < 2
                && match s.state {
                    5 => (x - rx).abs() < 40 && ry < y,
                    6 => (x - rx).abs() < 40 && ry > y,
                    7 => (y - ry).abs() < 40 && rx > x,
                    8 => (y - ry).abs() < 40 && rx < x,
                    _ => false,
                }
        });
        if !lane {
            self.structures[i].fire_counter = 0;
            self.sentry_word(i, s.frame);
        } else if s.fire_counter & 1 != 0 {
            let (vx, vy) = match s.state {
                5 => (0, -255),
                6 => (0, 255),
                7 => (255, 0),
                _ => (-255, 0),
            };
            self.stage_enemy_projectile(EnemyProjectile {
                kind: 0x66,
                x: s.x * 8192 + 0xf00,
                y: s.y * 8192 + 0xf00,
                z: s.z << 13,
                vx,
                vy,
                vz: 20,
            });
        }
        if lane && s.fire_counter == 0 {
            self.structures[i].fire_counter = 1;
        }
    }

    pub(crate) fn sentry_projectile_tick(&mut self, i: usize) {
        let mut p = self.enemy_bank[i];
        let (w, h) = self.terrain.size();
        let mut contact = 0;
        for _ in 0..10 {
            p.x += p.vx;
            p.y += p.vy;
            if p.x < 0
                || p.y < 0
                || p.x >= w * 8192
                || p.y >= h * 8192
                || !(0..65536).contains(&p.z)
            {
                contact = 1;
            } else if self.robots.iter().any(|r| {
                r.alive
                    && ((r.pos_x >> 8) - (p.x >> 8)).abs() < 16
                    && ((r.pos_y >> 8) - (p.y >> 8)).abs() < 16
                    && (r.z - (p.z >> 8)).abs() < 32
            }) {
                contact = 3;
            } else if p.vz != 0 {
                p.vz -= 1;
            } else if self.terrain.floor_z(p.x >> 8, p.y >> 8, p.z >> 8) > p.z >> 8 {
                contact = 2;
            }
            if contact != 0 {
                break;
            }
        }
        let (x, y) = (p.x, p.y);
        p.x -= p.vx;
        p.y -= p.vy;
        self.enemy_bank[i] = p;
        if contact == 2 || contact == 3 {
            self.projectile_disburser(i);
        }
        if contact == 2 {
            let damage = weapon_damage(0x66, self.difficulty);
            self.resolve_object_impact(x, y, 0, damage, false);
            self.resolve_structure_impact(x, y, damage);
        }
        if contact != 0 {
            self.enemy_bank[i].kind = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        destroy::parse_trt,
        mission::{AngleTable, Terrain},
    };

    fn scene() -> MissionSim {
        let terrain = Terrain::from_parts(16, 16, vec![0; 16 * 16 * 8], vec![[0; 1024]]).unwrap();
        let mut sim = MissionSim::new(terrain, AngleTable::from_thresholds(&[0; 64]).unwrap(), 123);
        let mut trt = vec![1, 0];
        for n in [4i32, 4, 1] {
            trt.extend(n.to_le_bytes());
        }
        sim.structures = parse_trt(&trt, 0).unwrap();
        sim.mirror_words = vec![0; 16 * 16 * 8];
        sim.mirror_seen = vec![1; 16 * 16 * 8];
        sim.stamp_sentries();
        sim
    }

    #[test]
    fn opens_aims_fires_then_retracts_without_changing_seen() {
        let mut sim = scene();
        sim.spawn_robot((4, 6, 1));
        let cell = (4 * 16 + 4) * 8 + 1;
        for frame in 1..=7 {
            sim.sentry_tick();
            assert_eq!(sim.mirror_words[cell], frame + 1);
        }
        sim.sentry_tick(); // open -> south-facing state
        sim.sentry_tick(); // counter starts, no shot
        assert_eq!(sim.mirror_words[cell], 16);
        assert!(sim.enemy_bank.iter().all(|p| p.kind == 0));
        sim.sentry_tick(); // counter 2 -> 3
        sim.sentry_tick(); // counter 3 fires
        let p = sim.enemy_bank[0];
        assert_eq!(
            (p.kind, p.x, p.y, p.z, p.vx, p.vy, p.vz),
            (0x66, 36608, 36608, 8192, 0, 255, 20)
        );
        sim.robots[0].alive = false;
        for _ in 0..7 {
            sim.sentry_tick();
        }
        assert_eq!((sim.structures[0].state, sim.structures[0].frame), (1, 0));
        assert_eq!(sim.mirror_words[cell], 1);
        assert_eq!(sim.mirror_seen[cell], 1);
    }

    #[test]
    fn inactive_gun_does_not_animate_or_overwrite_rubble() {
        let mut sim = scene();
        sim.spawn_robot((4, 6, 1));
        sim.structures[0].active = false;
        let before = sim.mirror_words.clone();
        sim.sentry_tick();
        assert_eq!(sim.mirror_words, before);
        assert_eq!(sim.structures[0].frame, 0);
    }

    #[test]
    fn bolt_moves_nine_steps_per_dispatch_and_arms_without_changing_height() {
        let mut sim = scene();
        let p = EnemyProjectile {
            kind: 0x66,
            x: 8192,
            y: 8192,
            z: 8192,
            vx: 255,
            vy: 0,
            vz: 20,
        };
        sim.stage_enemy_projectile(p);
        sim.enemy_tick();
        assert_eq!(
            (
                sim.enemy_bank[0].x,
                sim.enemy_bank[0].z,
                sim.enemy_bank[0].vz
            ),
            (8192 + 9 * 255, 8192, 10)
        );
        sim.enemy_tick();
        assert_eq!(
            (
                sim.enemy_bank[0].x,
                sim.enemy_bank[0].z,
                sim.enemy_bank[0].vz
            ),
            (8192 + 18 * 255, 8192, 0)
        );
    }

    #[test]
    fn armed_delay_does_not_bypass_robot_contact_or_apply_robot_damage() {
        let mut sim = scene();
        sim.spawn_robot((1, 1, 1));
        let r = &sim.robots[0];
        let hp = r.hp;
        let p = EnemyProjectile {
            kind: 0x66,
            x: r.pos_x,
            y: r.pos_y,
            z: r.z << 8,
            vx: 255,
            vy: 0,
            vz: 20,
        };
        sim.stage_enemy_projectile(p);
        sim.enemy_tick();
        assert_eq!(sim.enemy_bank[0].kind, 0);
        assert_eq!(sim.enemy_bank[0].x, p.x);
        assert_eq!(sim.enemy_bank[0].vz, 20);
        assert_eq!(sim.robots[0].hp, hp);
    }

    #[test]
    fn terrain_damage_uses_contact_tile_after_projectile_rolls_back() {
        let mut sim = scene();
        let mut dat = vec![0; 16 * 16 * 8];
        dat[16 * 16 + 4 * 16 + 5] = 1;
        sim.terrain = Terrain::from_parts(16, 16, dat, vec![[1; 1024]]).unwrap();
        sim.structures[0].x = 5;
        let x = 5 * 8192 - 128;
        sim.stage_enemy_projectile(EnemyProjectile {
            kind: 0x66,
            x,
            y: 4 * 8192 + 0xf00,
            z: 8192,
            vx: 255,
            vy: 0,
            vz: 0,
        });
        sim.enemy_tick();
        assert_eq!(sim.enemy_bank[0].x, x);
        assert_eq!(sim.enemy_bank[0].kind, 0);
        assert!(
            !sim.structures[0].active,
            "damage lands in tile 5, not reverted tile 4"
        );
    }
}
