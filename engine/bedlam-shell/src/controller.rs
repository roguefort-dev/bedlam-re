//! Scene lifecycle shared by window input, production replays, and the smoke harness.

use bedlam_core::{input::InputFrame, sim::SimConfig};
use bedlam_game::{ByteSource, GameConfig, GameError, GameHost, Scene};

use crate::chain::{stage_boot, stage_scene, ChainConfig};
use crate::clock::SUBTICKS_PER_PUMP;

/// A snapshot of player input, with no scene-transition authority.
#[derive(Debug, Clone, Default)]
pub struct ProductionInput(InputFrame);

impl From<InputFrame> for ProductionInput {
    fn from(frame: InputFrame) -> Self {
        Self(frame)
    }
}

/// One scene visit, measured in fixed host pumps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneVisit {
    pub scene: Scene,
    pub pumps: u64,
}

/// Owns the game and stages every scene before another input pump can run.
/// Read-only host access supports rendering and diagnostics without exposing
/// the legacy `GameHost::apply` transition shortcut.
///
/// ```compile_fail
/// use bedlam_shell::ShellController;
/// use bedlam_game::{ByteSource, SceneAction};
/// fn cannot_complete<S: ByteSource>(game: &mut ShellController<S>) {
///     game.pump(SceneAction::MissionComplete);
/// }
/// ```
/// ```compile_fail
/// use bedlam_shell::ShellController;
/// use bedlam_game::{ByteSource, SceneAction};
/// fn cannot_mutate_host<S: ByteSource>(game: &mut ShellController<S>) {
///     game.host().apply(SceneAction::MissionComplete);
/// }
/// ```
pub struct ShellController<S: ByteSource> {
    host: GameHost,
    source: S,
    config: ChainConfig,
    visits: Vec<SceneVisit>,
}

impl<S: ByteSource> ShellController<S> {
    pub fn new(mut source: S, config: ChainConfig, sim: &SimConfig) -> Result<Self, GameError> {
        let mut host = GameHost::new(&GameConfig::default(), sim, [[0; 3]; 256]);
        stage_boot(&mut host, &mut source, config)?;
        let scene = host.scene();
        Ok(Self {
            host,
            source,
            config,
            visits: vec![SceneVisit { scene, pumps: 0 }],
        })
    }

    pub fn host(&self) -> &GameHost {
        &self.host
    }
    pub fn source(&self) -> &S {
        &self.source
    }
    pub fn visits(&self) -> &[SceneVisit] {
        &self.visits
    }

    /// Presentation-only asset fetch (for example the native UI font).
    pub fn load_asset(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
        self.source.load(name)
    }

    pub fn recompose(&mut self, alpha: f32) -> bool {
        self.host.recompose(alpha)
    }
    pub fn render_audio(&mut self, out: &mut [i16]) -> Result<usize, GameError> {
        self.host.render_audio(out)
    }

    pub fn pump(&mut self, input: ProductionInput) -> Result<(), GameError> {
        self.step(input, None).map(|_| ())
    }

    /// Deterministic replay drain, at the historical pre-staging audio boundary.
    pub fn pump_with_audio(
        &mut self,
        input: ProductionInput,
        out: &mut [i16],
    ) -> Result<usize, GameError> {
        self.step(input, Some(out))
    }

    fn step(
        &mut self,
        input: ProductionInput,
        audio: Option<&mut [i16]>,
    ) -> Result<usize, GameError> {
        // Retry a failed entry before allowing any further input to reach it.
        self.stage_entered()?;
        self.host.pump_frame(SUBTICKS_PER_PUMP, &input.0);
        let mixed = audio.map(|out| self.host.render_audio(out)).transpose();
        self.stage_entered()?;
        self.visits.last_mut().expect("seeded").pumps += 1;
        Ok(mixed?.unwrap_or(0))
    }

    fn stage_entered(&mut self) -> Result<(), GameError> {
        if self.host.scene() != self.visits.last().expect("seeded").scene {
            if self.host.preparation_return_pending() {
                self.host
                    .resume_preparation(&mut self.source, self.config.language)?;
            } else if self.host.scene() == Scene::Brief
                && self.host.menu_start_score_seen().is_some()
                && self.host.preparation().is_none()
            {
                let balance = self
                    .host
                    .menu_start_score_seen()
                    .expect("new game score")
                    .max(0) as u32;
                self.host.load_preparation(
                    &mut self.source,
                    1,
                    [false; 27],
                    [0; 15],
                    balance,
                    self.config.language,
                )?;
            } else if !(self.host.scene() == Scene::Shop && self.host.preparation().is_some()) {
                stage_scene(&mut self.host, &mut self.source, self.config)?;
            }
            self.visits.push(SceneVisit {
                scene: self.host.scene(),
                pumps: 0,
            });
        }
        Ok(())
    }
}

/// Explicitly synthetic compatibility harness. Never a player input source.
pub mod harness {
    use super::*;
    use bedlam_game::SceneAction;

    /// One host-applied walk step: hold the CURRENT scene for `hold`
    /// host pumps, then apply `action`. This compatibility smoke fabricates
    /// transitions and must never be counted as a production gameplay trace.
    pub type WalkStep = (u64, SceneAction);

    /// The default campaign walk: boot attract -> title -> boot-camp
    /// brief -> select -> mission -> debrief -> cutscene (zone 1
    /// complete; the D32-D35 chain: cutscene + BETWEEN + region loading
    /// screen + FULLFONT/FULLPAL + LANGUAGE) -> select. The cutscene
    /// hold is long enough to run a real slice of the ZONEDONE pass and
    /// hand the plane to the D34 loading flow.
    pub fn default_walk() -> Vec<WalkStep> {
        vec![
            (30, SceneAction::Advance),         // Title -> Brief
            (40, SceneAction::Advance),         // Brief -> Select
            (10, SceneAction::Advance),         // Select -> Mission
            (20, SceneAction::MissionComplete), // Mission -> Debrief (zone 1)
            (10, SceneAction::Advance),         // Debrief -> Cutscene (pending)
            (400, SceneAction::Advance),        // Cutscene -> Select
        ]
    }

    pub(crate) struct HarnessController<S: ByteSource>(pub(crate) ShellController<S>);

    impl<S: ByteSource> HarnessController<S> {
        pub(crate) fn apply(&mut self, action: SceneAction) -> Result<(), GameError> {
            self.0.host.apply(action);
            self.0.stage_entered()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::GameGfxSource;
    use crate::input::ShellKey;

    fn game() -> ShellController<GameGfxSource> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM");
        ShellController::new(
            GameGfxSource::new(root),
            ChainConfig::default(),
            &SimConfig::default(),
        )
        .unwrap()
    }

    fn title(game: &mut ShellController<GameGfxSource>) {
        for _ in 0..100 {
            if game.host().scene() == Scene::Title {
                break;
            }
            game.pump(
                InputFrame {
                    buttons: ShellKey::Escape.bit(),
                    ..Default::default()
                }
                .into(),
            )
            .unwrap();
        }
        assert_eq!(game.host().scene(), Scene::Title);
        assert!(
            game.host().menu().is_some(),
            "entry assets must be staged before the next input"
        );
        game.pump(InputFrame::default().into()).unwrap();
        game.pump(
            InputFrame {
                buttons: ShellKey::Escape.bit(),
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
        game.pump(InputFrame::default().into()).unwrap();
    }

    #[test]
    fn training_extraction_returns_through_movie_to_next_zone_with_live_inventory() {
        let mut game = game();
        // Focused return-boundary fixture: real room/shop inputs and assets,
        // with the robot subsequently positioned beside the actual exit.
        // This is not the end-to-end Boot Camp product trace.
        game.host
            .load_preparation(
                &mut game.source,
                1,
                [false; 27],
                [0; 15],
                3500,
                "LANGUAGE.ENG",
            )
            .unwrap();
        game.stage_entered().unwrap();
        fn click(game: &mut ShellController<GameGfxSource>, x: i32, y: i32) {
            for _ in 0..12 {
                game.pump(ProductionInput::default()).unwrap();
            }
            let (cx, cy) = game.host().preparation().unwrap().cursor();
            game.pump(
                InputFrame {
                    mouse_dx: (x - cx) as i16,
                    mouse_dy: (y - cy) as i16,
                    mouse_buttons: 1,
                    ..Default::default()
                }
                .into(),
            )
            .unwrap();
        }
        click(&mut game, 255, 315);
        click(&mut game, 255, 80);
        let category = bedlam_game::armoury::catalog::CATEGORIES[6];
        click(&mut game, category.anchor.0, category.anchor.1);
        let (x, y) = category.panel_origin();
        click(&mut game, x + 10, y + 4 + 9 + 4);
        click(&mut game, 500, 350);
        assert_eq!(
            game.host.preparation().unwrap().transactions().weapons()[0]
                .unwrap()
                .amount,
            600
        );
        click(&mut game, 590, 455);
        assert_eq!(game.host.scene(), Scene::Mission);
        {
            let mission = game.host.mission_mut().unwrap();
            mission.set_campaign(70, 3250);
            let sim = mission.sim_mut();
            let robot = &mut sim.robots_mut()[0];
            robot.pos_x = 16 << 13;
            robot.pos_y = 25 << 13;
            robot.z = 159;
            robot.probe_z.fill(159);
            robot.weapons[0].ammo = 420;
            assert!(sim.resolve_object_impact(17 << 13, 25 << 13, 0, 400, true));
            sim.configure_hints(1, 1, 0);
            sim.stage_command_record(bedlam_core::weapon::CommandRecord {
                marker: 0,
                id: 0,
                spot: 0,
                flags: 1,
                x: 568,
                y: 816,
                z: 0,
            });
        }
        for frame in 0..180 {
            game.pump(ProductionInput::default()).unwrap();
            if game.host.scene() != Scene::Mission {
                break;
            }
            if frame == 12 {
                game.host
                    .mission_mut()
                    .unwrap()
                    .sim_mut()
                    .stage_command_record(bedlam_core::weapon::CommandRecord {
                        marker: 0,
                        id: 0,
                        spot: 0,
                        flags: 1,
                        x: 568,
                        y: 816,
                        z: 0,
                    });
            }
        }
        assert_eq!(
            game.host.scene(),
            Scene::Cutscene,
            "no blank debrief advance"
        );
        assert_eq!(game.host.fsm().episode().stage(), 2);
        assert!(game.host.movie().is_some(), "zone movie is staged");
        let mut audio = [0i16; 1600];
        for _ in 0..2000 {
            game.pump_with_audio(ProductionInput::default(), &mut audio)
                .unwrap();
            if game.host.scene() == Scene::Select && game.host.loading_phase().is_none() {
                break;
            }
        }
        assert_eq!(
            game.host.scene(),
            Scene::Select,
            "movie {:?}, loading {:?}",
            game.host.movie().map(|m| (m.frame_index(), m.finished())),
            game.host.loading_phase()
        );
        assert!(
            game.host.loading_phase().is_none(),
            "loading must finish automatically"
        );
        let p = game.host.preparation().unwrap();
        assert_eq!(p.phase(), bedlam_game::armoury::journey::Phase::Room);
        assert!(!p.room_pending());
        assert_eq!(p.transactions().balance(), 3250);
        let plasma = p.transactions().weapons()[0].unwrap();
        assert_eq!((plasma.amount, plasma.paid), (420, 350));
        assert!(
            p.transactions().catalog().available(8, 4),
            "zone2 scanner is unlocked"
        );
        // Select a Zone B region and open its retained armoury.
        click(&mut game, 338, 312);
        click(&mut game, 255, 80);
        assert_eq!(game.host.scene(), Scene::Shop);
        click(&mut game, 590, 455);
        assert_eq!(game.host.scene(), Scene::Mission);
        assert_eq!(game.host.mission_slot(), (1, 2));
        let mission = game.host.mission().unwrap();
        assert_eq!(mission.campaign(), (110, 3250));
        assert_eq!(mission.sim().robots()[0].weapons[0].ammo, 420);
        assert_eq!(mission.sim().objectives()[0].targets, [778]);
        assert_eq!(mission.sim().objective_radar_markers().len(), 17);
        assert!(!mission.sim().primary_objective_complete());
    }

    #[test]
    fn production_input_journey_stages_each_entry_before_the_next_pump() {
        let mut game = game();
        title(&mut game);
        let (x, y) = game.host().menu_cursor().unwrap();
        game.pump(
            InputFrame {
                mouse_dx: (320 - x) as i16,
                mouse_dy: (314 - y) as i16,
                mouse_buttons: 1,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
        assert_eq!(game.host().scene(), Scene::Select);
        fn click(game: &mut ShellController<GameGfxSource>, x: i32, y: i32) {
            let (cx, cy) = game.host().preparation().unwrap().cursor();
            game.pump(
                InputFrame {
                    mouse_dx: (x - cx) as i16,
                    mouse_dy: (y - cy) as i16,
                    mouse_buttons: 1,
                    ..Default::default()
                }
                .into(),
            )
            .unwrap();
        }
        click(&mut game, 255, 315);
        for _ in 0..5 {
            game.pump(ProductionInput::default()).unwrap();
        }
        click(&mut game, 255, 80);
        assert_eq!(game.host().scene(), Scene::Shop);
        click(&mut game, 500, 400);
        for _ in 0..12 {
            game.pump(ProductionInput::default()).unwrap();
        }
        let expected = game
            .host()
            .preparation()
            .unwrap()
            .transactions()
            .weapons()
            .map(|row| row.map_or((0, 0), |r| (r.name, r.amount)));
        let expected_cash = game.host().preparation().unwrap().transactions().balance();
        assert_ne!(expected_cash, 4000, "shop must spend before deployment");
        let equipment = *game
            .host()
            .preparation()
            .unwrap()
            .transactions()
            .equipment();
        assert!(
            equipment
                .iter()
                .flatten()
                .any(|r| (0x2a..=0x2c).contains(&r.name)),
            "this production journey must exercise equipment transfer"
        );
        click(&mut game, 590, 455);
        assert_eq!(game.host().scene(), Scene::Mission);
        assert_eq!(
            game.host().mission().unwrap().weapon_loadout(0),
            Some(&expected)
        );
        let mission = game.host().mission().unwrap();
        for robot in mission.sim().robots().iter().filter(|r| r.kind == 0) {
            for (slot, (name, ammo)) in robot.weapons.iter().zip(expected) {
                assert_eq!((slot.id, slot.ammo), (name, ammo as i16));
            }
            assert_ne!(
                robot.weapon_mask, 0,
                "purchased weapons must be armed in simulation"
            );
        }
        let first = mission
            .sim()
            .robots()
            .iter()
            .position(|r| r.kind == 0)
            .unwrap();
        for row in equipment.iter().flatten() {
            let robot = &mission.sim().robots()[first];
            match row.name {
                0x2a => assert_eq!(robot.shield_charges, i32::from(row.amount as i16)),
                0x2b => {
                    assert_eq!(robot.battery, i32::from(row.amount as i16));
                    assert_eq!(robot.hp, 5000 + 100 * robot.battery);
                }
                0x2c => assert_eq!(robot.armor_pool, i32::from(row.amount as i16) * 200),
                0x2d | 0x2e => assert_eq!(mission.scanner_level(0), Some((row.name - 0x2c) as u8)),
                _ => panic!("unexpected equipment"),
            }
        }
        for (before, after) in equipment.iter().zip(
            game.host()
                .preparation()
                .unwrap()
                .transactions()
                .equipment(),
        ) {
            if before.is_some_and(|r| (0x2a..=0x2c).contains(&r.name)) {
                assert!(after.is_none(), "deployment consumes without a sell/refund");
            } else {
                assert_eq!(before, after);
            }
        }
        assert!(
            game.host().mission().is_some(),
            "mission is ready when the transition pump returns"
        );
        let world = game.host().mission().unwrap().sim();
        assert!(
            !world.objects().is_empty(),
            "production stages POS instances"
        );
        assert_eq!(world.rides().len(), 7);
        assert_eq!(world.rides()[0].marker, (5, 61, 0));
        assert_eq!(world.rides()[0].destination, (8, 57, 2));
        assert!(
            world.objects().iter().any(|o| o.hp > 0),
            "BDG initializes hit points"
        );
        assert!(
            world.object_grid().iter().any(|&w| w != 0),
            "footprints and hazards are live"
        );
        assert!(
            world.mirror_words().iter().any(|&w| w != 0),
            "TOT mirror is live"
        );
        assert!(game
            .source()
            .fetched()
            .iter()
            .any(|(name, _)| name.contains("ZONEA/MISSION1")));
        assert_eq!(
            game.visits().iter().map(|v| v.scene).collect::<Vec<_>>(),
            vec![
                Scene::Boot,
                Scene::Title,
                Scene::Select,
                Scene::Shop,
                Scene::Mission
            ]
        );
        assert_eq!(
            game.host().mission().unwrap().campaign().1,
            expected_cash as i32,
            "mission displays the remaining shop balance"
        );
        let before = game.host().mission().unwrap().sim().robots()[0].q5();
        let (cx, cy) = game.host().mission().unwrap().cursor();
        game.pump(
            InputFrame {
                mouse_dx: (336 - cx) as i16,
                mouse_dy: (185 - cy) as i16,
                mouse_buttons: 1,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
        for _ in 0..40 {
            game.pump(ProductionInput::default()).unwrap();
        }
        assert_ne!(
            game.host().mission().unwrap().sim().robots()[0].q5(),
            before,
            "ordinary ground input moves the deployed player: {:?}",
            game.host().mission().unwrap().sim().robots()[0]
        );
        for _ in 0..200 {
            if game
                .host()
                .mission()
                .unwrap()
                .sim()
                .hints()
                .active()
                .is_some()
            {
                break;
            }
            game.pump(ProductionInput::default()).unwrap();
        }
        assert_eq!(
            game.host().mission().unwrap().sim().hints().active(),
            Some(0),
            "production movement reaches the welcome strip: {:?}",
            game.host().mission().unwrap().sim().robots()[0]
        );
        for _ in 0..10 {
            game.pump(ProductionInput::default()).unwrap();
        }
        game.pump(
            InputFrame {
                mouse_buttons: 1,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
        assert_eq!(game.host().mission().unwrap().sim().hints().active(), None);
        let resumed_from = game.host().mission().unwrap().sim().robots()[0].q5();
        for _ in 0..20 {
            game.pump(ProductionInput::default()).unwrap();
        }
        assert_ne!(
            game.host().mission().unwrap().sim().robots()[0].q5(),
            resumed_from,
            "welcome dismissal resumes walking instead of arming extraction: {:?}",
            game.host().mission().unwrap().sim().robots()[0]
        );
        let plasma_before = game.host().mission().unwrap().sim().robots()[0].weapons[0].ammo;
        game.pump(
            InputFrame {
                mouse_buttons: 2,
                ..Default::default()
            }
            .into(),
        )
        .unwrap();
        let fired = game.host().mission().unwrap();
        assert!(fired.sim().robots()[0].weapons[0].ammo < plasma_before);
        assert!(
            fired.sim().weapon_bank().iter().any(|shot| shot.kind == 5),
            "ordinary right mouse input creates actual Plasma Cannon shots"
        );
        // Arbitrary mouse edges never stand in for MissionComplete.
        for _ in 0..3 {
            game.pump(InputFrame::default().into()).unwrap();
            game.pump(
                InputFrame {
                    mouse_buttons: 1,
                    ..Default::default()
                }
                .into(),
            )
            .unwrap();
        }
        assert_eq!(game.host().scene(), Scene::Mission);
    }

    #[test]
    fn failed_entry_blocks_further_input_until_assets_can_be_staged() {
        struct FailingTitle {
            source: GameGfxSource,
            failures: u8,
        }
        impl ByteSource for FailingTitle {
            fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
                if name == "TITLE.SMK" && self.failures > 0 {
                    self.failures -= 1;
                    return Err(GameError::AssetMissing { name: name.into() });
                }
                self.source.load(name)
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM");
        let mut game = ShellController::new(
            FailingTitle {
                source: GameGfxSource::new(root),
                failures: 2,
            },
            ChainConfig::default(),
            &SimConfig::default(),
        )
        .unwrap();
        for _ in 0..100 {
            if game
                .pump(
                    InputFrame {
                        buttons: ShellKey::Escape.bit(),
                        ..Default::default()
                    }
                    .into(),
                )
                .is_err()
            {
                break;
            }
        }
        assert_eq!(game.host().scene(), Scene::Title);
        assert_eq!(game.visits().last().unwrap().scene, Scene::Boot);
        let tick = game.host().driver().sim().tick_index();
        assert!(game
            .pump(
                InputFrame {
                    mouse_buttons: 1,
                    ..Default::default()
                }
                .into()
            )
            .is_err());
        assert_eq!(
            game.host().driver().sim().tick_index(),
            tick,
            "failed staging must not consume another input step"
        );
        game.pump(ProductionInput::default()).unwrap();
        assert_eq!(game.visits().last().unwrap().scene, Scene::Title);
        assert!(game.host().menu().is_some());
    }
}
