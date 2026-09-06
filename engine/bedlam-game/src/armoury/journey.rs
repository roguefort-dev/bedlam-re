//! Shared mission-room/armoury journey, ready for host scene staging.
use super::{
    catalog::{Catalog, Mode},
    input::{ArmouryInput, Outcome},
    random::ShopRandom,
    render::ArmouryRenderer,
    transactions::Transactions,
};
use crate::{
    mission_room::{MissionRoom, RoomAction},
    ByteSource, GameError,
};
use bedlam_core::input::InputFrame;
use bedlam_render::Vga6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Room,
    Armoury,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    Back,
    Briefing {
        zone: u8,
        mission: u8,
    },
    Launch {
        zone: u8,
        mission: u8,
    },
}

/// Keeps selection and purchases together until the host stages the mission.
/// The secondary random stream remains caller-owned across visits.
pub struct Preparation {
    room: MissionRoom,
    shop: ArmouryInput,
    renderer: ArmouryRenderer,
    phase: Phase,
    selected: Option<(u8, u8)>,
}
impl Preparation {
    pub fn load(
        source: &mut dyn ByteSource,
        zone: u8,
        completed: [bool; 27],
        flags: [u32; 15],
        balance: u32,
        language: &str,
    ) -> Result<Self, GameError> {
        let room = MissionRoom::load(source, zone, completed, language)?;
        let catalog = Catalog::new(Mode::Campaign, zone, flags).expect("room validates zone");
        let shop = ArmouryInput::new(Transactions::new(catalog, balance));
        let renderer = ArmouryRenderer::load(source)?;
        Ok(Self {
            room,
            shop,
            renderer,
            phase: Phase::Room,
            selected: None,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn transactions(&self) -> &Transactions {
        self.shop.state()
    }
    pub(crate) fn deploy(&mut self, mission: &mut crate::mission::MissionScene) {
        let weapons = self
            .transactions()
            .weapons()
            .map(|row| row.map_or((0, 0), |r| (r.name, r.amount)));
        let mut equipment = self
            .transactions()
            .equipment()
            .map(|row| row.map_or((0, 0), |r| (r.name, r.amount)));
        let before = equipment;
        mission.deploy_loadout(0, &weapons, &mut equipment);
        self.shop.consume_equipment(std::array::from_fn(|i| {
            before[i].0 != 0 && equipment[i].0 == 0
        }));
    }
    pub fn cursor(&self) -> (i32, i32) {
        match self.phase {
            Phase::Room => self.room.cursor(),
            Phase::Armoury => self.shop.cursor(),
        }
    }
    pub fn pixels(&self) -> &[u8] {
        match self.phase {
            Phase::Room => self.room.pixels(),
            Phase::Armoury => self.renderer.pixels(),
        }
    }
    pub fn palette(&self) -> &[Vga6; 256] {
        match self.phase {
            Phase::Room => self.room.palette(),
            Phase::Armoury => self.renderer.palette(),
        }
    }
    pub fn tick(&mut self, input: &InputFrame, random: &mut ShopRandom) -> Action {
        match self.phase {
            Phase::Room => match self.room.tick(input) {
                RoomAction::None => Action::None,
                RoomAction::Back => Action::Back,
                RoomAction::Briefing { zone, mission } => Action::Briefing { zone, mission },
                RoomAction::Armoury { zone, mission } => {
                    self.selected = Some((zone, mission));
                    self.phase = Phase::Armoury;
                    self.shop.draw(&mut self.renderer, false);
                    Action::None
                }
            },
            Phase::Armoury => {
                let outcome = self.shop.tick_with_random(input, random);
                self.shop
                    .draw(&mut self.renderer, input.mouse_buttons & 1 != 0);
                if outcome == Outcome::Done {
                    let (zone, mission) = self.selected.expect("armoury requires selection");
                    Action::Launch { zone, mission }
                } else {
                    Action::None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source;
    impl ByteSource for Source {
        fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
            let root =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM");
            let path = if name.starts_with("LANGUAGE.") {
                root.join(name)
            } else {
                root.join("GAMEGFX").join(name)
            };
            std::fs::read(path).map_err(|_| GameError::AssetMissing { name: name.into() })
        }
    }
    fn click(p: &mut Preparation, r: &mut ShopRandom, x: i32, y: i32) -> Action {
        let (cx, cy) = p.cursor();
        p.tick(
            &InputFrame {
                mouse_dx: (x - cx) as i16,
                mouse_dy: (y - cy) as i16,
                mouse_buttons: 1,
                ..Default::default()
            },
            r,
        )
    }
    #[test]
    fn host_consumes_preparation_input_and_enters_selected_mission() {
        use crate::{GameConfig, GameHost, Scene};
        let mut host = GameHost::new(
            &GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0; 3]; 256],
        );
        host.load_preparation(&mut Source, 1, [false; 27], [0; 15], 3500, "LANGUAGE.ENG")
            .unwrap();
        fn pointer(host: &mut GameHost, x: i32, y: i32) {
            let (cx, cy) = host.preparation().unwrap().cursor();
            host.pump_frame(
                4,
                &InputFrame {
                    mouse_dx: (x - cx) as i16,
                    mouse_dy: (y - cy) as i16,
                    mouse_buttons: 1,
                    ..Default::default()
                },
            );
        }
        pointer(&mut host, 255, 315);
        for _ in 0..5 {
            host.pump_frame(4, &InputFrame::default());
        }
        pointer(&mut host, 255, 80);
        assert_eq!(host.scene(), Scene::Shop);
        pointer(&mut host, 500, 400);
        for _ in 0..12 {
            host.pump_frame(4, &InputFrame::default());
        }
        pointer(&mut host, 590, 455);
        assert_eq!(host.scene(), Scene::Mission);
        assert_eq!(host.mission_slot(), (0, 1));
        assert!(host.preparation().unwrap().transactions().has_weapon());
    }

    #[test]
    fn boot_camp_selection_auto_and_launch_keep_slot_and_loadout_together() {
        let mut p =
            Preparation::load(&mut Source, 1, [false; 27], [0; 15], 3500, "LANGUAGE.ENG").unwrap();
        let mut rng = ShopRandom::from_state(0);
        click(&mut p, &mut rng, 255, 315);
        for _ in 0..5 {
            p.tick(&InputFrame::default(), &mut rng);
        }
        click(&mut p, &mut rng, 255, 80);
        assert_eq!(p.phase(), Phase::Armoury);
        let before = p.pixels().to_vec();
        click(&mut p, &mut rng, 500, 400);
        assert!(p.transactions().has_weapon());
        assert_ne!(p.pixels(), before);
        for _ in 0..12 {
            p.tick(&InputFrame::default(), &mut rng);
        }
        assert_eq!(
            click(&mut p, &mut rng, 590, 455),
            Action::Launch {
                zone: 1,
                mission: 1
            }
        );
        assert!(p
            .transactions()
            .weapons()
            .iter()
            .flatten()
            .any(|row| row.amount > 0));
    }
}
