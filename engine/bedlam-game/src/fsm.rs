//! Scene FSM - the hashed scene-state machine (DESIGN-GAME secs 3/7).
//!
//! The machine is pure integer state advanced once per EXECUTED 60 Hz sim
//! tick with the same pending-input semantics as bedlam-core Sim
//! (D17 a + D26): mouse-button edges are level-sampled AT THE TICK GRID
//! (the EXW 100 Hz KeySink latch analog, RE-EXW-INPUT), which is why edge
//! detection lives HERE, hashed, instead of in the per-frame bucket. The
//! same input script therefore yields the same scene hash at any host
//! rate (tests/determinism.rs).
//!
//! NO per-mission code (PLAN P3): the Mission scene is a state, not a
//! simulation; mission quirks are data.

use bedlam_core::hash::{Fnv1a64, StateHash};
use bedlam_core::input::InputFrame;

use crate::SCENE_HASH_TAG;

/// Boot hold before the title screen: 12 ticks = 200 ms at 60 Hz
/// [design; the EXW boot spin is GoFlag-paced, not tick-counted].
pub const BOOT_TICKS: u16 = 12;

/// Full-mask table verbatim from B2 @0x81d9a: completed-sub bits per
/// stage slot. Slot 0 is empty (the boot quirk below), slot 1 has one
/// mission, slots 2..=8 have four subs each (bits 0..=3).
pub const FULL_MASK: [u8; 9] = [0, 1, 15, 15, 15, 15, 15, 15, 15];

/// Linear mission counter ceiling: 27 linear missions 0..=26
/// (census sec 7).
pub const MAX_LINEAR: u16 = 26;

/// Highest stage slot (zones map onto slots 2..8; 1 is the intro slot).
pub const MAX_STAGE: u8 = 8;

/// One screen of the game (DESIGN-GAME sec 3; the PLAN P3 scene list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Scene {
    Boot = 0,
    Title = 1,
    Options = 2,
    Brief = 3,
    Select = 4,
    Mission = 5,
    Debrief = 6,
    Shop = 7,
    Cutscene = 8,
    Quit = 9,
}

/// Scene-transition intent. Advance/Back are input-derived (per-tick
/// mouse edges); the rest are host/sim intents applied explicitly via
/// SceneFsm::apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneAction {
    None,
    Advance,
    Back,
    Options,
    MissionComplete,
    MissionFail,
    Quit,
}

/// Episode progression - deliberately the B2 save shape {mask, slot,
/// linear} minus money/stats (those stay sim-side, DESIGN-GAME sec 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Episode {
    stage: u8,
    mask: u8,
    linear: u16,
}

impl Episode {
    /// Boot state: stage 1 / mask 0 / linear 0. The B2 boot plants
    /// stage=1 / sub=1 as constants (census sec 7), so the machine
    /// starts at slot 1 rather than the empty slot 0.
    pub const fn boot() -> Episode {
        Episode {
            stage: 1,
            mask: 0,
            linear: 0,
        }
    }

    /// Current stage slot, 1..=MAX_STAGE.
    pub fn stage(&self) -> u8 {
        self.stage
    }

    /// Completed-sub bitmask within the current stage.
    pub fn mask(&self) -> u8 {
        self.mask
    }

    /// Linear mission counter, 0..=MAX_LINEAR.
    pub fn linear(&self) -> u16 {
        self.linear
    }

    /// Stage the campaign slot to `(stage, mask)` — the D51 host seam
    /// (W12-S5/D108): the canonical runner stands in for the
    /// campaign-advance (0x41c9e5) / save-load-restore (0x43c2b8)
    /// shells the engine does not model, planting the slot whose
    /// mission the next Mission entry stages. `linear` is left
    /// untouched (the staged fresh-slot contract; a played campaign
    /// carries its own counter — the recorded live-capture seam).
    /// Returns false on an out-of-range stage or a mask that is not
    /// a sub-mask of the stage's completion mask (never guess).
    pub fn stage_slot(&mut self, stage: u8, mask: u8) -> bool {
        if !(1..=MAX_STAGE).contains(&stage) {
            return false;
        }
        let subs = FULL_MASK[stage as usize];
        if subs == 0 || mask & !subs != 0 {
            return false;
        }
        self.stage = stage;
        self.mask = mask;
        true
    }

    /// Register one completed mission: linear +1 (capped at MAX_LINEAR),
    /// the current sub marked done, and when the stage mask fills, the
    /// slot advances and the mask resets (census sec 7). Returns true
    /// when THIS completion completed the zone (the cutscene trigger).
    pub fn complete(&mut self) -> bool {
        self.linear = (self.linear + 1).min(MAX_LINEAR);
        let subs = FULL_MASK[self.stage as usize];
        if subs == 0 || self.mask == subs {
            return false; // defensive: stage already full
        }
        // Lowest unset bit = placeholder sub selection
        // [design, DESIGN-GAME open Q5].
        let mut sub = 0u8;
        while self.mask >> sub & 1 != 0 {
            sub += 1;
        }
        self.mask |= 1 << sub;
        if self.mask == subs {
            self.stage = (self.stage + 1).min(MAX_STAGE);
            self.mask = 0;
            return true;
        }
        false
    }
}

/// The scene state machine - the HASHED scene bucket (DESIGN-GAME sec 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneFsm {
    scene: Scene,
    /// Ticks spent in the current scene (reset on enter).
    scene_ticks: u64,
    /// Boot countdown, BOOT_TICKS at construction.
    boot_left: u16,
    episode: Episode,
    /// Set when a mission completion filled the stage mask; consumed by
    /// the Debrief -> Cutscene transition.
    zone_complete_pending: bool,
    /// Last consumed tick mouse state (bits 0/1) - the hashed
    /// level-sampled latch (D26).
    prev_mouse: u8,
}

impl Default for SceneFsm {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneFsm {
    /// Fresh machine: Boot scene, full boot countdown, episode at the
    /// B2 boot constants, latches cleared.
    pub fn new() -> SceneFsm {
        SceneFsm {
            scene: Scene::Boot,
            scene_ticks: 0,
            boot_left: BOOT_TICKS,
            episode: Episode::boot(),
            zone_complete_pending: false,
            prev_mouse: 0,
        }
    }

    /// Current scene.
    pub fn scene(&self) -> Scene {
        self.scene
    }

    /// Ticks spent in the current scene.
    pub fn scene_ticks(&self) -> u64 {
        self.scene_ticks
    }

    /// Episode progression state.
    pub fn episode(&self) -> &Episode {
        &self.episode
    }

    /// Stage the campaign episode slot — the D51 host seam (W12-S5,
    /// D108; see [`Episode::stage_slot`]).
    pub fn stage_episode_slot(&mut self, stage: u8, mask: u8) -> bool {
        self.episode.stage_slot(stage, mask)
    }

    /// Whether the next Debrief advance plays the zone cutscene.
    pub fn zone_complete_pending(&self) -> bool {
        self.zone_complete_pending
    }

    /// One executed sim tick: derive the action from mouse-button edges
    /// (level-sampled at the tick grid, D26), apply it, return it.
    pub fn tick(&mut self, input: &InputFrame) -> SceneAction {
        let mouse = input.mouse_buttons & 0x03;
        let left = mouse & 1 != 0 && self.prev_mouse & 1 == 0;
        let right = mouse & 2 != 0 && self.prev_mouse & 2 == 0;
        self.prev_mouse = mouse;
        let action = match self.scene {
            // Boot ignores input: the GameGoRelease latch-clear analog.
            Scene::Boot => SceneAction::None,
            _ if left => SceneAction::Advance,
            _ if right => SceneAction::Back,
            _ => SceneAction::None,
        };
        self.apply(action);
        action
    }

    /// The full transition function: the input path plus the host/sim
    /// intents (Options, Quit, MissionComplete, MissionFail).
    /// Deterministic and total.
    pub fn apply(&mut self, action: SceneAction) {
        self.scene_ticks = self.scene_ticks.saturating_add(1);
        let next = match self.scene {
            Scene::Boot => {
                if self.boot_left > 0 {
                    self.boot_left -= 1;
                }
                if self.boot_left == 0 {
                    Some(Scene::Title)
                } else {
                    None
                }
            }
            Scene::Title => match action {
                SceneAction::Advance => Some(Scene::Brief),
                SceneAction::Options => Some(Scene::Options),
                SceneAction::Quit => Some(Scene::Quit),
                _ => None,
            },
            Scene::Options => match action {
                SceneAction::Back => Some(Scene::Title),
                _ => None,
            },
            Scene::Brief => match action {
                SceneAction::Advance => Some(Scene::Select),
                SceneAction::Back => Some(Scene::Title),
                _ => None,
            },
            Scene::Select => match action {
                SceneAction::Advance => Some(Scene::Mission),
                _ => None,
            },
            Scene::Mission => match action {
                SceneAction::MissionComplete => {
                    let zone = self.episode.complete();
                    self.zone_complete_pending |= zone;
                    Some(Scene::Debrief)
                }
                SceneAction::MissionFail => Some(Scene::Debrief),
                _ => None,
            },
            Scene::Debrief => match action {
                SceneAction::Advance => {
                    if self.zone_complete_pending {
                        self.zone_complete_pending = false;
                        Some(Scene::Cutscene)
                    } else {
                        Some(Scene::Shop)
                    }
                }
                _ => None,
            },
            Scene::Cutscene => match action {
                SceneAction::Advance => Some(Scene::Select),
                _ => None,
            },
            Scene::Shop => match action {
                SceneAction::Advance => Some(Scene::Select),
                _ => None,
            },
            Scene::Quit => None,
        };
        if let Some(scene) = next {
            self.enter(scene);
        }
    }

    /// Enter a scene: reset the per-scene tick counter. The edge latch
    /// is deliberately NOT cleared (the P-latch clear analog,
    /// RE-EXW-INPUT: MissionShell clears its latches so a HELD key does
    /// not re-trigger - here edges are consumed in the tick they are
    /// derived, so no old-scene edge can leak, and leaving the held
    /// level in place means a button held across the boundary must be
    /// released and re-pressed before it acts in the new scene).
    fn enter(&mut self, scene: Scene) {
        self.scene = scene;
        self.scene_ticks = 0;
    }

    /// Hashed scene-state view: FNV-1a 64 (bedlam-core hash crate)
    /// over canonical LE fields with the BDLG tag (DESIGN-GAME sec 7).
    pub fn scene_hash(&self) -> StateHash {
        let mut h = Fnv1a64::new();
        h.write_bytes(SCENE_HASH_TAG);
        h.write_u8(self.scene as u8);
        h.write_u64(self.scene_ticks);
        h.write_u16(self.boot_left);
        h.write_u8(self.episode.stage);
        h.write_u8(self.episode.mask);
        h.write_u16(self.episode.linear);
        h.write_u8(u8::from(self.zone_complete_pending));
        h.write_u8(self.prev_mouse);
        StateHash(h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle() -> InputFrame {
        InputFrame::default()
    }

    fn left() -> InputFrame {
        InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        }
    }

    fn right() -> InputFrame {
        InputFrame {
            mouse_buttons: 2,
            ..InputFrame::default()
        }
    }

    #[test]
    fn boot_reaches_title_after_the_hold() {
        let mut fsm = SceneFsm::new();
        assert_eq!(fsm.scene(), Scene::Boot);
        // The countdown hits zero ON tick 12: the title appears after
        // exactly BOOT_TICKS ticks (200 ms at 60 Hz).
        for i in 0..BOOT_TICKS - 1 {
            // Held clicks during boot are ignored (latch-clear analog).
            fsm.tick(&left());
            assert_eq!(fsm.scene(), Scene::Boot, "tick {i}");
        }
        fsm.tick(&idle());
        assert_eq!(fsm.scene(), Scene::Title);
        assert_eq!(fsm.scene_ticks(), 0, "scene_ticks reset on enter");
    }

    #[test]
    fn happy_path_walks_every_scene() {
        let mut fsm = SceneFsm::new();
        while fsm.scene() == Scene::Boot {
            fsm.tick(&idle());
        }
        assert_eq!(fsm.scene(), Scene::Title);
        fsm.apply(SceneAction::Options);
        assert_eq!(fsm.scene(), Scene::Options);
        fsm.apply(SceneAction::Back);
        assert_eq!(fsm.scene(), Scene::Title);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Brief);
        fsm.apply(SceneAction::Back);
        assert_eq!(fsm.scene(), Scene::Title);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Brief);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Select);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Mission);
        fsm.apply(SceneAction::MissionFail);
        assert_eq!(fsm.scene(), Scene::Debrief);
        assert_eq!(fsm.episode().linear(), 0, "fail applies no progress");
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Shop);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Select);
    }

    #[test]
    fn mission_completion_fills_mask_and_gates_cutscene() {
        let mut fsm = SceneFsm::new();
        while fsm.scene() != Scene::Title {
            fsm.tick(&idle());
        }
        fsm.apply(SceneAction::Advance);
        fsm.apply(SceneAction::Advance);
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Mission);
        // Slot 1 full-mask is 1: a single completion zones out.
        fsm.apply(SceneAction::MissionComplete);
        assert_eq!(fsm.scene(), Scene::Debrief);
        assert_eq!(fsm.episode().linear(), 1);
        assert_eq!(fsm.episode().stage(), 2, "slot advanced");
        assert_eq!(fsm.episode().mask(), 0, "mask reset");
        assert!(fsm.zone_complete_pending());
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Cutscene);
        assert!(!fsm.zone_complete_pending(), "consumed");
        fsm.apply(SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Select);
        // Slot 2 full-mask is 15: four completions to zone out again.
        for expect_zone in [false, false, false, true] {
            fsm.apply(SceneAction::Advance); // Select -> Mission
            assert_eq!(fsm.scene(), Scene::Mission);
            fsm.apply(SceneAction::MissionComplete);
            assert_eq!(fsm.scene(), Scene::Debrief);
            assert_eq!(fsm.zone_complete_pending(), expect_zone);
            // Walk back to Select through the post-mission screens.
            fsm.apply(SceneAction::Advance);
            assert_eq!(
                fsm.scene(),
                if expect_zone {
                    Scene::Cutscene
                } else {
                    Scene::Shop
                }
            );
            fsm.apply(SceneAction::Advance);
            assert_eq!(fsm.scene(), Scene::Select);
        }
        assert_eq!(fsm.episode().stage(), 3);
        assert_eq!(fsm.episode().linear(), 5, "1 + 4 completions");
    }

    #[test]
    fn linear_counter_caps_at_max() {
        let mut ep = Episode::boot();
        ep.stage = MAX_STAGE;
        for _ in 0..(MAX_LINEAR + 10) {
            ep.complete();
        }
        assert_eq!(ep.linear(), MAX_LINEAR);
        assert_eq!(ep.stage(), MAX_STAGE, "stage caps too");
    }

    #[test]
    fn stage_slot_seam_validates_and_plants() {
        // W12-S5/D108: the D51 host seam plants the campaign slot
        // (the canonical runner's `zone` key). Mask must be a
        // sub-mask of the stage's completion mask; linear untouched.
        let mut ep = Episode::boot();
        ep.linear = 4;
        assert!(ep.stage_slot(2, 0), "stage 2 / mask 0 = ZONEB/MISSION1");
        assert_eq!((ep.stage(), ep.mask(), ep.linear()), (2, 0, 4));
        assert!(ep.stage_slot(7, 0b0111), "a valid partial mask");
        assert_eq!((ep.stage(), ep.mask()), (7, 0b0111));
        assert!(!ep.stage_slot(0, 0), "stage 0 is not a slot");
        assert!(!ep.stage_slot(MAX_STAGE + 1, 0), "past the cap");
        // Stage 1 has ONE sub (FULL_MASK[1] = 1): mask 2 is not a
        // sub-mask of it.
        assert!(!ep.stage_slot(1, 2));
        assert!(ep.stage_slot(1, 1), "the full stage-1 mask");
        // The planted slot drives the mission-slot selection (the
        // host.rs integration: zone letter B -> names ZONEB/...).
        let mut fsm = SceneFsm::new();
        assert!(fsm.stage_episode_slot(2, 0));
        assert_eq!(
            crate::mission::mission_asset_names(
                crate::mission::zone_for_stage(fsm.episode().stage()),
                crate::mission::mission_number_for_mask(fsm.episode().mask()),
            )[0],
            "ZONEB/MISSION1.TOT"
        );
    }

    #[test]
    fn quit_is_terminal() {
        let mut fsm = SceneFsm::new();
        while fsm.scene() != Scene::Title {
            fsm.tick(&idle());
        }
        fsm.apply(SceneAction::Quit);
        assert_eq!(fsm.scene(), Scene::Quit);
        for action in [
            SceneAction::Advance,
            SceneAction::Back,
            SceneAction::Options,
            SceneAction::Quit,
        ] {
            fsm.apply(action);
            assert_eq!(fsm.scene(), Scene::Quit);
        }
        // Input ticks after quit change nothing but the tick counter.
        let before = fsm.scene_hash();
        fsm.tick(&left());
        assert_eq!(fsm.scene(), Scene::Quit);
        assert_ne!(fsm.scene_hash(), before, "scene_ticks still hashes");
    }

    #[test]
    fn tick_derives_edges_per_tick() {
        let mut fsm = SceneFsm::new();
        while fsm.scene() != Scene::Title {
            fsm.tick(&idle());
        }
        // Press: exactly one Advance edge on the first held tick.
        assert_eq!(fsm.tick(&left()), SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Brief);
        // Held ACROSS the scene change: no re-fire (the P-latch clear
        // analog - the held level carries, a re-press is required).
        assert_eq!(fsm.tick(&left()), SceneAction::None, "held: no edge");
        assert_eq!(fsm.scene(), Scene::Brief);
        assert_eq!(fsm.tick(&idle()), SceneAction::None, "release: no edge");
        // Release then press: a fresh edge.
        assert_eq!(fsm.tick(&right()), SceneAction::Back);
        assert_eq!(fsm.scene(), Scene::Title);
        // Both buttons: left wins (probe order).
        let both = InputFrame {
            mouse_buttons: 3,
            ..InputFrame::default()
        };
        assert_eq!(fsm.tick(&both), SceneAction::Advance);
        assert_eq!(fsm.scene(), Scene::Brief);
    }

    #[test]
    fn mission_ignores_input_actions() {
        let mut fsm = SceneFsm::new();
        while fsm.scene() != Scene::Mission {
            fsm.apply(SceneAction::Advance);
        }
        let before = fsm.scene_ticks();
        fsm.tick(&left());
        fsm.tick(&right());
        assert_eq!(fsm.scene(), Scene::Mission, "input cannot leave Mission");
        assert_eq!(fsm.scene_ticks(), before + 2, "ticks still counted");
    }
}
