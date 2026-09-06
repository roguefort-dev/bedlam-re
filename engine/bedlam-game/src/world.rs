//! Production destructible-world staging, EXW MissionShell 0x447b5d..0x447b8f.
use crate::{ByteSource, GameError};
use bedlam_core::{
    destroy::{parse_trt, ObjectTypeTable, OBJECT_INSTANCE_SLOTS},
    mission::MissionSim,
};

#[derive(Debug)]
pub struct WorldAssets {
    names: [String; 3],
    table: ObjectTypeTable,
    pos: Vec<u8>,
    trt: Vec<u8>,
    tot: Vec<u8>,
    zone: u32,
    mission: u32,
    tier: u32,
    size: (i32, i32),
}

fn bad(reason: &'static str) -> GameError {
    GameError::BadMissionAsset {
        what: "mission world",
        reason,
    }
}

impl WorldAssets {
    /// Fetch and validate before replacing the active host mission.
    pub fn load(
        source: &mut dyn ByteSource,
        zone: i32,
        mission: i32,
        tot: &[u8],
    ) -> Result<Self, GameError> {
        if !(0..7).contains(&zone) || mission < 1 || tot.len() < 4 {
            return Err(bad("invalid mission slot or TOT header"));
        }
        let size = (
            i32::from(u16::from_le_bytes([tot[0], tot[1]])),
            i32::from(u16::from_le_bytes([tot[2], tot[3]])),
        );
        if size.0 <= 0 || size.1 <= 0 || tot.len() != 4 + 16 * size.0 as usize * size.1 as usize {
            return Err(bad("TOT dimensions or volume length"));
        }
        let base = format!("ZONE{}/MISSION{mission}", (b'A' + zone as u8) as char);
        let names = [
            format!("{base}.BDG"),
            format!("{base}.POS"),
            format!("{base}.TRT"),
        ];
        let table = ObjectTypeTable::from_bdg_bytes(&source.load(&names[0])?)
            .ok_or_else(|| bad("malformed BDG table"))?;
        let pos = source.load(&names[1])?;
        if pos.len() != 16 * OBJECT_INSTANCE_SLOTS {
            return Err(bad("POS must contain 2000 records"));
        }
        // Derived current-slot tier, EXW 0x41c534/0x41c53e/0x41c550.
        let tier = (5 * (zone - 1) + mission - 1).clamp(1, 26) as u32;
        let trt = source.load(&names[2])?;
        if parse_trt(&trt, tier).is_none() {
            return Err(bad("malformed TRT bank"));
        }
        Ok(Self {
            names,
            table,
            pos,
            trt,
            tot: tot.to_vec(),
            zone: (zone + 1) as u32,
            mission: mission as u32,
            tier,
            size,
        })
    }

    pub fn names(&self) -> &[String; 3] {
        &self.names
    }

    pub(crate) fn install(self, sim: &mut MissionSim) -> Result<(), GameError> {
        if sim.terrain.size() != self.size {
            return Err(bad("world dimensions differ from mission"));
        }
        // These calls cannot reject the dimensions and file lengths checked above.
        assert!(sim.stage_destroy_family(&self.table, &self.pos, &self.trt, self.zone, self.tier));
        sim.set_mission_no(self.mission);
        sim.stage_rides();
        sim.stage_elevators();
        assert!(sim.stage_pickup_surface(&self.tot, self.zone));
        sim.stamp_hazard_words();
        sim.observe_terrain_writes();
        Ok(())
    }
}
