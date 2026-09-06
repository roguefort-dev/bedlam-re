//! Selected-robot death wipe and squad failure, EXW 0x44809e/0x44764c.
use bedlam_core::mission::Robot;

#[derive(Debug, Default)]
pub(crate) struct DeathWipe {
    next: u16,
    pub display: u16,
    pub failed: bool,
}
impl DeathWipe {
    pub fn cancel(&mut self) {
        self.next = 0;
        self.display = 0;
    }

    pub fn tick(
        &mut self,
        robots: &[Robot],
        selected: &mut usize,
        player_type: u16,
        extracted: bool,
        single_player: bool,
    ) -> bool {
        if !single_player || self.failed {
            return false;
        }
        if self.next == 0 {
            if !robots
                .get(*selected)
                .is_some_and(|r| r.death_flag != 0 && !r.alive)
            {
                return false;
            }
            self.next = 1;
        }
        self.display = self.next;
        self.next = (self.next + 40).min(480);
        if self.next < 480 {
            return false;
        }
        let mut changed = false;
        for (slot, robot) in robots.iter().enumerate() {
            if robot.alive && robot.kind == player_type && slot != *selected {
                *selected = slot;
                changed = true;
            }
        }
        if changed {
            self.cancel();
        } else if !extracted && !robots.is_empty() && robots.iter().all(|r| r.death_flag != 0) {
            self.failed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedlam_core::mission::{AngleTable, MissionSim, Terrain};
    fn squad() -> Vec<Robot> {
        let mut sim = MissionSim::new(
            Terrain::from_parts(8, 8, vec![0; 512], vec![]).unwrap(),
            AngleTable::from_thresholds(&[0; 64]).unwrap(),
            1,
        );
        for _ in 0..3 {
            sim.spawn_robot((2, 2, 1));
        }
        sim.robots().to_vec()
    }
    #[test]
    fn terminal_at_twelve_and_last_survivor_wins() {
        let mut robots = squad();
        robots[0].alive = false;
        robots[0].death_flag = 1;
        let mut selected = 0;
        let mut wipe = DeathWipe::default();
        for n in 0..11 {
            assert!(!wipe.tick(&robots, &mut selected, 0, false, true));
            assert_eq!(wipe.display, 1 + 40 * n);
            assert_eq!(selected, 0);
            assert!(!wipe.failed);
        }
        assert!(wipe.tick(&robots, &mut selected, 0, false, true));
        assert_eq!(selected, 2);
        assert_eq!(wipe.display, 0);
        assert!(!wipe.failed);
        for r in &mut robots {
            r.alive = false;
            r.death_flag = 1;
        }
        for _ in 0..11 {
            wipe.tick(&robots, &mut selected, 0, false, true);
            assert!(!wipe.failed);
        }
        wipe.tick(&robots, &mut selected, 0, false, true);
        assert!(wipe.failed);
    }
    #[test]
    fn cancellation_extraction_network_and_death_flag_gates() {
        let mut robots = squad();
        robots[0].alive = false;
        robots[0].death_flag = 1;
        let mut selected = 0;
        let mut wipe = DeathWipe::default();
        wipe.tick(&robots, &mut selected, 0, false, true);
        selected = 1;
        wipe.cancel();
        wipe.tick(&robots, &mut selected, 0, false, true);
        assert_eq!(wipe.display, 0);
        for r in &mut robots {
            r.alive = false;
            r.death_flag = 1;
        }
        for _ in 0..20 {
            wipe.tick(&robots, &mut selected, 0, true, true);
        }
        assert!(!wipe.failed);
        let mut network = DeathWipe::default();
        for _ in 0..20 {
            network.tick(&robots, &mut selected, 0, false, false);
        }
        assert_eq!(network.display, 0);
        let mut flag = DeathWipe::default();
        robots[2].death_flag = 0;
        for _ in 0..20 {
            flag.tick(&robots, &mut selected, 0, false, true);
        }
        assert!(!flag.failed, "alive is not the failure oracle");
    }
}
