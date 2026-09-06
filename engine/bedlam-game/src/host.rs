//! GameHost - the per-frame pump wiring core + render + audio
//! (DESIGN-GAME sec 4; the FUN_0043d00b order poll -> sim -> render ->
//! present). ALL file I/O crossing this crate hides behind the injected
//! ByteSource / ByteSink traits below, so the pump itself stays
//! hermetic, replayable and testable.

use bedlam_audio::{Mixer, MusicScript};
use bedlam_core::frame::SimDriver;
use bedlam_core::hash::StateHash;
use bedlam_core::input::InputFrame;

/// Skip chord bits mirrored from the shell button module (ESCAPE |
/// ADVANCE); pending the P2e input-RE bit pinning.
const SKIP_BUTTONS: u32 = (1 << 9) | (1 << 10);

/// Level-based skip test for one input frame (also true when the left
/// mouse button is held).
fn cinema_skip_requested(input: &InputFrame) -> bool {
    input.buttons & SKIP_BUTTONS != 0 || input.mouse_buttons & 1 != 0
}
use bedlam_core::sim::{Sim, SimConfig};
use bedlam_render::{render, Frame, MovieFrame, RenderInput, Vga6};

use crate::boot::{BootAttract, BootPhase};
use crate::brief::{BriefIntro, BriefPhase};
use crate::config::GameConfig;
use crate::fsm::{Scene, SceneAction, SceneFsm};
use crate::loading::{LoadingFlow, LoadingPhase, TextRow};
use crate::menu::{MenuAction, TitleMenu};
use crate::mission::MissionScene;
use crate::movie::MoviePlayer;
use crate::music::{self, MusicPump};
use crate::save::SaveSlotImport;
use crate::GameError;

/// Injected read side of every asset the game consumes. The hermetic
/// rule (DESIGN-GAME sec 8): the crate itself never touches fs.
pub trait ByteSource {
    /// Load one named asset as raw bytes.
    fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError>;
}

/// Injected write side (saves, options stores).
pub trait ByteSink {
    /// Store one named asset.
    fn store(&mut self, name: &str, bytes: &[u8]) -> Result<(), GameError>;
}

/// The present pacing policy of one host — the FIRST consumer of the
/// timing-lock purist axis (P6, PLAN §6 "time-based simulation" +
/// D201/D203; selected from the immutable mode, NEVER a Hz).
///
/// Both arms share the fixed 60 Hz logic tick and the D17 accumulator
/// (the sim never sees display timing); the axis selects only how the
/// PLATFORM's presents couple to that tick:
///
/// - [`PresentPacing::Decoupled`] (MODERN arm): present every host
///   frame regardless of ticks executed — accumulator-driven, the
///   PLAN §6 high-refresh present (a 240 Hz host presents 240 frames
///   of which most carry zero logic ticks; the shell clock
///   `bedlam-shell/src/clock.rs` is built for exactly this shape).
/// - [`PresentPacing::FrameLocked`] (CLASSIC arm): the original
///   frame-locked present-coupled pacing [verified, RE-EXW-PACER §3 /
///   D16: "one sim/render frame per display flip = vsync-locked, no
///   software frame clock" — the FUN_0043d00b loop pass and its
///   PresentEnd are ONE event, `g_frame_count++` exactly once per
///   flip]. A host frame is presentable only when it executed >= 1
///   logic tick; zero-tick frames leave the previously presented
///   image up. On the original 60 Hz display class this is
///   indistinguishable from the original (one tick per flip); on
///   faster hosts the VISIBLE refresh follows the fixed tick rate,
///   never the display rate — the purist cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentPacing {
    /// MODERN arm: the accumulator-driven present, decoupled from the
    /// tick rate (every host frame is presentable).
    Decoupled,
    /// CLASSIC arm: the original frame-locked, present-coupled pacing
    /// (a present is due only when a logic tick executed).
    FrameLocked,
}

/// The composition root: hashed sim + hashed scene FSM + un-hashed
/// frame state (inside the SimDriver), the audio mixer and the latest
/// canonical frame.
pub struct GameHost {
    driver: SimDriver,
    mixer: Mixer,
    fsm: SceneFsm,
    music: Option<MusicPump>,
    /// Scene the mixer script is attached to (None = needs sync).
    music_scene: Option<Scene>,
    /// The loaded movie and the scene it plays on (D31).
    movie: Option<MovieSlot>,
    /// The post-cutscene loading flow (D34): BETWEEN interlude +
    /// region-variant loading screen, presentation-only like the movie.
    loading: Option<LoadingFlow>,
    /// The boot attract pair (D36): GTLOG then LOGO, presentation-only
    /// like the movie - one EXW pass each on the Boot scene.
    boot: Option<BootAttract>,
    /// The briefing intro pair (D37): BRF_DROP one pass then the
    /// zone backdrop ring, presentation-only like the movie - on
    /// the Brief scene.
    brief: Option<BriefIntro>,
    /// The title menu (D41/D42): the NameEntryScreen model, strip
    /// and draw plane, presentation-only. On the Title scene it also
    /// OWNS the input path (explicit intents replace the generic
    /// click-advance, D42.1).
    menu: Option<TitleMenu>,
    /// Score seed of the menu's last Start action (survives the menu
    /// drop on the Title exit; D42.8).
    menu_start_score: Option<i32>,
    /// The staged mission (DESIGN-GAME sec 11): sim + viewport
    /// composition, INERT until the FSM enters Mission and dropped on
    /// exit (the flow never ends on its own).
    mission: Option<MissionScene>,
    preparation: Option<crate::armoury::journey::Preparation>,
    preparation_slot: Option<(u8, u8)>,
    preparation_transition: bool,
    shop_random: crate::armoury::random::ShopRandom,
    frame: Frame,
    palette: [Vga6; 256],
    /// Sim ticks executed by the most recent [`GameHost::pump_frame`]
    /// (None before the first pump). Pacing-policy bookkeeping ONLY
    /// — the un-hashed presentation bucket (D17 b): it feeds
    /// [`GameHost::should_present`] and can never reach the sim or
    /// the state/scene hashes.
    last_pump_ticks: Option<u32>,
    /// Camera-interpolation ENDPOINT: a clone of the sim as of the
    /// pump BEFORE the last executed tick batch (None until a pump
    /// executes a tick). Presentation-bucket ONLY (D17 b,
    /// docs/RE-EXW-CAMERA.md §5): it is read exclusively by
    /// [`GameHost::recompose`] as the `prev_sim` half of the
    /// camera-only lerp — never advanced, never hashed, never
    /// serialized; the hashed trajectory is identical with or
    /// without it (pinned by test
    /// `camera_interpolation_never_touches_the_hashed_buckets`).
    prev_sim: Option<Sim>,
}

/// One loaded movie: the player plus its lifecycle state. Loads are
/// INERT until the FSM enters the target scene (frame 0 decodes at load,
/// its audio queued only on entry), and leaving the scene stops
/// playback and drops the mixer stream.
#[derive(Debug)]
struct MovieSlot {
    player: MoviePlayer,
    scene: Scene,
    started: bool,
    elapsed_subticks: u64,
    decoded: u32,
}

impl GameHost {
    /// Build a host: sim from sim_config, mixer master from the config
    /// (the volume >> 1 UI mapping), first frame pre-rendered.
    pub fn new(config: &GameConfig, sim_config: &SimConfig, palette: [Vga6; 256]) -> GameHost {
        let mut mixer = Mixer::new();
        mixer.set_master_volume(config.music_master());
        let mut host = GameHost {
            driver: SimDriver::new(sim_config),
            mixer,
            fsm: SceneFsm::new(),
            music: None,
            music_scene: None,
            movie: None,
            loading: None,
            boot: None,
            brief: None,
            menu: None,
            menu_start_score: None,
            mission: None,
            preparation: None,
            preparation_slot: None,
            preparation_transition: false,
            shop_random: crate::armoury::random::ShopRandom::from_state(0),
            frame: Frame::new(palette),
            palette,
            last_pump_ticks: None,
            prev_sim: None,
        };
        host.sync_music();
        host.frame = host.render_now();
        host
    }

    /// One host frame (FUN_0043d00b order: poll -> sim -> render ->
    /// present). Returns the number of sim ticks executed.
    ///
    /// 1. the SimDriver quantizes dt to whole 60 Hz ticks (banking the
    ///    remainder) and runs the frame-rate systems;
    /// 2. the scene FSM advances once per EXECUTED tick with the same
    ///    pending input (D17 a + D26);
    /// 3. the music script follows any scene change;
    /// 4. the canonical frame is re-rendered and stored for present.
    pub fn pump_frame(&mut self, dt_subticks: u32, input: &InputFrame) -> u32 {
        // Camera-interpolation endpoint (P6 high-refresh present,
        // D17 b / docs/RE-EXW-CAMERA.md §5): snapshot the sim BEFORE
        // the advance; if this pump executes >= 1 tick the snapshot
        // is the state at (current - executed) ticks — for the
        // canonical one-tick pump exactly one tick back, the
        // `prev_sim` half of the camera lerp. A zero-tick pump keeps
        // the previous endpoint (the sim did not move, so the old
        // endpoint is still the tick-before-latest state). Pure
        // presentation bucket: the clone is read by recompose only.
        let pre_advance = self.driver.sim().clone();
        let executed = self.driver.advance(dt_subticks, input);
        if executed > 0 {
            self.prev_sim = Some(pre_advance);
        }
        // Pacing bookkeeping (P6 timing-lock consumer): recorded BEFORE
        // the rest of the pump so a mid-pump panic can never leave a
        // stale gate answer behind; presentation-bucket only.
        self.last_pump_ticks = Some(executed);
        self.apply_cinema_skip(input);
        // Menu input ownership (D42.1): while a menu is staged on
        // Title, the menu IS the Title input path - the FSM is fed
        // NEUTRAL frames (ticks count, no generic click-advance) and
        // menu outcomes become explicit intents. Re-checked per
        // executed tick: a Start/Quit intent can leave the scene
        // mid-loop.
        for _ in 0..executed {
            if self.menu.is_some() && self.fsm.scene() == Scene::Title {
                let movies_playing = self.title_movie_playing();
                let tick = self
                    .menu
                    .as_mut()
                    .expect("checked staged")
                    .tick(input, movies_playing);
                self.apply_menu_tick(tick);
                self.fsm.tick(&InputFrame::default());
            } else if self.preparation.is_some()
                && matches!(self.fsm.scene(), Scene::Select | Scene::Shop)
            {
                use crate::armoury::journey::{Action, Phase};
                let preparation = self.preparation.as_mut().expect("staged preparation");
                let action = if self.preparation_transition {
                    Action::None
                } else {
                    preparation.tick(input, &mut self.shop_random)
                };
                let phase = preparation.phase();
                match action {
                    Action::Launch { zone, mission } => {
                        self.preparation_slot = Some((zone, mission));
                        self.fsm.enter(Scene::Mission);
                    }
                    Action::Back => self.fsm.enter(Scene::Title),
                    Action::Briefing { .. } => self.fsm.enter(Scene::Brief),
                    Action::None => {
                        let scene = match phase {
                            Phase::Room => Scene::Select,
                            Phase::Armoury => Scene::Shop,
                        };
                        if self.fsm.scene() != scene {
                            self.fsm.enter(scene);
                        }
                    }
                }
                self.fsm.tick(&InputFrame::default());
            } else {
                // Mission input (DESIGN-GAME sec 11): while a mission
                // is active it consumes the SAME frame — the Mission
                // scene FSM ignores mouse actions, so no neutral-frame
                // split is needed (unlike Title). Order inside the
                // mission tick: pointer -> click seam -> phases, the
                // MissionShell order [RE-EXW-SIM sec 1].
                let mission_was_active = self.fsm.scene() == Scene::Mission;
                if mission_was_active {
                    if let Some(mission) = self.mission.as_mut() {
                        let outcome = mission.tick(input);
                        if outcome == crate::mission::MissionOutcome::Failed {
                            self.fsm.apply(SceneAction::MissionFail);
                        } else if outcome == crate::mission::MissionOutcome::ExtractionComplete
                            && self.preparation_slot == Some((1, 1))
                        {
                            if let Some(preparation) = self.preparation.as_mut() {
                                preparation.finish_training(mission);
                                self.preparation_slot = None;
                                self.preparation_transition = true;
                                self.fsm.apply(SceneAction::MissionComplete);
                                // Boot Camp debrief returns immediately (EXW 0x44427e).
                                self.fsm.apply(SceneAction::Advance);
                            }
                        }
                    }
                }
                if mission_was_active && self.fsm.scene() != Scene::Mission {
                    self.fsm.tick(&InputFrame::default());
                } else {
                    self.fsm.tick(input);
                }
            }
        }
        self.sync_movie();
        self.sync_boot();
        self.sync_brief();
        self.sync_loading();
        self.sync_menu();
        self.sync_mission();
        self.sync_music();
        self.pump_movie(dt_subticks);
        self.pump_boot(dt_subticks);
        self.pump_brief(dt_subticks);
        self.pump_loading(dt_subticks);
        self.frame = self.render_now();
        executed
    }

    /// Apply a host/sim intent the input path cannot derive (Options,
    /// Quit, MissionComplete, MissionFail).
    pub fn apply(&mut self, action: SceneAction) {
        self.fsm.apply(action);
        self.sync_music();
    }

    /// Current scene.
    pub fn scene(&self) -> Scene {
        self.fsm.scene()
    }

    /// The immutable mode the host's sim runs under (P6 seam, D201):
    /// the ONE [`bedlam_core::mode::ModeConfig`] injected at sim
    /// construction via [`SimConfig`], carried by the driver. The
    /// purist-toggle axes are read on the host — the timing-lock arm
    /// through [`GameHost::present_pacing`]/[`GameHost::should_present`]
    /// (the axis's first consumer, D203; control mapping later) —
    /// never mutated mid-run; a mode change is a new host.
    pub fn mode(&self) -> bedlam_core::mode::ModeConfig {
        self.driver.mode()
    }

    /// The present pacing policy this host runs under — the timing-lock
    /// axis's consumer (P6, D201/D203). Selected from the immutable
    /// [`bedlam_core::mode::ModeConfig`]: the MODERN arm decouples the
    /// present from the tick rate, the CLASSIC arm restores the
    /// original frame-locked present-coupled pacing. This is a POLICY
    /// selector, never a rate: the logic tick stays fixed at the
    /// original rate in BOTH arms and no display rate ever enters the
    /// sim or the state hash (Determinism Charter, PLAN §3).
    pub fn present_pacing(&self) -> PresentPacing {
        use bedlam_core::mode::{PuristToggle, ToggleArm};
        if self.mode().arm(PuristToggle::TimingLock) == ToggleArm::Classic {
            PresentPacing::FrameLocked
        } else {
            PresentPacing::Decoupled
        }
    }

    /// Whether the platform should present [`GameHost::frame`] after
    /// the most recent [`GameHost::pump_frame`] — the pacing-policy
    /// consumer the present loop asks each host frame.
    ///
    /// - `Decoupled` (modern): always `true` — every host frame
    ///   presents, zero-tick frames included (they recompose from
    ///   latest state; PLAN §6 high-refresh present).
    /// - `FrameLocked` (classic): `true` iff the last pump executed
    ///   at least one logic tick (the original one-flip-one-loop-pass
    ///   lock, RE-EXW-PACER §3). A zero-tick host frame returns
    ///   `false`: the previously presented image stays up. Before
    ///   the FIRST pump the pre-rendered boot frame is presentable
    ///   in both arms (the platform must blit once to have anything
    ///   on the surface at all).
    ///
    /// Presentation-bucket only: the answer depends on the immutable
    /// mode plus the last pump's tick count — neither is hashed, so
    /// the same pump script yields identical sim/scene hashes in both
    /// arms (pinned by test `timing_lock_pacing_never_touches_hashed_
    /// buckets`).
    pub fn should_present(&self) -> bool {
        match self.present_pacing() {
            PresentPacing::Decoupled => true,
            PresentPacing::FrameLocked => !matches!(self.last_pump_ticks, Some(0)),
        }
    }

    /// Whether the presented frame should be recomposed with the
    /// interpolated camera — the PLAN §6 composition policy selected
    /// from the SAME timing-lock arm as [`GameHost::present_pacing`]
    /// (P6, `p6-high-refresh-interpolation`).
    ///
    /// - `Decoupled` (modern): `true` — the accumulator-driven
    ///   high-refresh present composes most frames between logic
    ///   ticks, so the camera/scroll blends from the last executed
    ///   tick toward the present (the accumulator fraction,
    ///   docs/RE-EXW-CAMERA.md §5). The original had NO sub-tick
    ///   camera anywhere (§4): this is the deliberate, budgeted
    ///   modernization manufacture.
    /// - `FrameLocked` (classic): `false` — the frame-locked pacing
    ///   presents only after a tick executes, so the presented image
    ///   is exactly the tick-state camera: nothing to interpolate
    ///   (the original's shape, unchanged).
    ///
    /// Camera/scroll ONLY: sprites stay grid-quantized (the 1996
    /// sprites had no sub-pixel positions); sub-pixel blitting stays
    /// a default-off presentation option out of scope here.
    pub fn camera_interpolation(&self) -> bool {
        matches!(self.present_pacing(), PresentPacing::Decoupled)
    }

    /// Recompose the presented frame under the mode's camera
    /// interpolation policy — the PRESENT-SITE companion of the
    /// decoupled pacing (P6, PLAN §6 "the frame is composed from
    /// latest state + camera/scroll interpolation").
    ///
    /// `alpha` is the accumulator fraction of the pending logic tick
    /// (0..=1, nominally the fraction between the last executed
    /// logic tick and the present; saturated by the renderer, never
    /// extrapolates). Under the MODERN arm with an interpolation
    /// endpoint staged this re-renders `frame` from the LATEST state
    /// with the camera lerped `(prev -> cur) · alpha` — everything
    /// else in the frame still comes from the current sim. Under the
    /// CLASSIC arm (or before the first executed tick) this is a
    /// NO-OP: the pump's parity frame stands.
    ///
    /// Returns whether an interpolated recompose happened.
    ///
    /// Presentation-bucket ONLY (D17 b): this mutates `frame` and
    /// nothing else — never the sim, the state hash or the scene
    /// hash. The next [`GameHost::pump_frame`] re-renders the parity
    /// frame regardless, so the pump path is byte-identical with or
    /// without interleaved recompose calls (pinned by test
    /// `camera_interpolation_never_touches_the_hashed_buckets`).
    pub fn recompose(&mut self, alpha: f32) -> bool {
        if !self.camera_interpolation() {
            return false;
        }
        // Take the endpoint out for the borrow split (render_with
        // needs &mut self for the mission/menu plane passes), then
        // restore it: the endpoint survives every recompose.
        let Some(prev) = self.prev_sim.take() else {
            return false;
        };
        self.frame = self.render_with(Some(&prev), alpha);
        self.prev_sim = Some(prev);
        true
    }

    /// The scene machine (hashed bucket).
    pub fn fsm(&self) -> &SceneFsm {
        &self.fsm
    }

    /// Hashed scene-state view (D17 a + D26).
    pub fn scene_hash(&self) -> StateHash {
        self.fsm.scene_hash()
    }

    /// The latest canonical frame (the PresentCopy analog: hand this
    /// to the platform; bedlam-game never presents by itself). The
    /// platform presents it each host frame iff [`GameHost::
    /// should_present`] says so (the timing-lock pacing policy).
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// The sim driver (hashed sim + un-hashed frame state).
    pub fn driver(&self) -> &SimDriver {
        &self.driver
    }

    /// Mutable driver access for explicit engine operations
    /// (snapshots). Gameplay routes through pump_frame.
    pub fn driver_mut(&mut self) -> &mut SimDriver {
        &mut self.driver
    }

    /// The audio mixer (un-hashed, D17 bucket b; hosts also use this
    /// for SFX note_on calls, which bypass the music script).
    pub fn mixer_mut(&mut self) -> &mut Mixer {
        &mut self.mixer
    }

    /// Load a music track (.MRS bytes): the pump pre-builds the script
    /// and the next scene sync attaches it.
    pub fn load_music(&mut self, mrs_bytes: &[u8]) -> Result<(), GameError> {
        self.music = Some(MusicPump::new(mrs_bytes)?);
        self.music_scene = None; // force re-attach
        self.sync_music();
        Ok(())
    }

    /// Host-paced audio render (D17 bucket b; never hashed). Delegates
    /// to the mixer on the internal Q16 grid, so chunking is host
    /// business only.
    pub fn render_audio(&mut self, out: &mut [i16]) -> Result<usize, GameError> {
        Ok(self.mixer.render(out)?)
    }

    /// Load a movie (.SMK bytes) to play on the given scene (D31). The stream
    /// opens and frame 0 decodes immediately (typed error on invalid
    /// bytes); playback itself starts when the FSM ENTERS the scene,
    /// stops when it leaves. Loading touches no hashed state - the
    /// scene hash is provably unchanged (unit-pinned).
    pub fn load_movie(&mut self, scene: Scene, data: &[u8]) -> Result<(), GameError> {
        self.movie = Some(MovieSlot {
            player: MoviePlayer::new(data)?,
            scene,
            started: false,
            elapsed_subticks: 0,
            decoded: 1,
        });
        Ok(())
    }

    /// Stop the movie (if any) and silence its audio stream. Safe when
    /// nothing is loaded. Leaving the movie scene does this implicitly.
    pub fn stop_movie(&mut self) {
        self.movie = None;
        self.mixer.clear_pcm_stream();
    }

    /// The loaded movie player, if any (introspection for hosts:
    /// frame index, container facts, finished state).
    pub fn movie(&self) -> Option<&MoviePlayer> {
        self.movie.as_ref().map(|slot| &slot.player)
    }
    /// The zone-complete cutscene movie the CURRENT episode state
    /// selects (movies::cutscene_name over the hashed stage slot):
    /// ZONEDONE.SMK mid-game, END.SMK at the endgame ceiling
    /// (EXW LAB_0041c69e: ZONEDONE.SMK on every zone completion,
    /// END.SMK when the zone counter reads its last value). Pure
    /// name arithmetic for the caller ByteSource fetch - the host
    /// never loads anything by itself (DESIGN-GAME sec 8).
    pub fn cutscene_name(&self) -> &'static str {
        crate::movies::cutscene_name(self.fsm.episode().stage())
    }

    /// Load the zone-complete cutscene movie (.SMK bytes the caller
    /// fetched under [`Self::cutscene_name`]) onto the Cutscene
    /// scene (D32). Same lifecycle as [`Self::load_movie`] (D31):
    /// inert until the FSM enters Cutscene, dropped on leaving.
    pub fn load_cutscene(&mut self, data: &[u8]) -> Result<(), GameError> {
        self.load_movie(Scene::Cutscene, data)
    }

    /// The briefing backdrop movie the CURRENT hashed episode slot
    /// selects (movies::briefing_name_for_slot over stage + mask):
    /// the lettered zone stages 2..=6 pick BRF_{B..F}{sub}.SMK for
    /// the mission the slot is about to play (the letter map is now
    /// EXW-verified, D37: letter = zone@004edd8c + 0x40, zones
    /// 2..=6 = B..=F); the boot-camp and endgame stages pick None
    /// (no lettered backdrop in the corpus, so the caller stages no
    /// briefing pair there). Pure name arithmetic for the caller
    /// ByteSource fetch - the host never loads anything by itself
    /// (DESIGN-GAME sec 8). The DROP half of the pair is the
    /// region-independent movies::BRIEFING_DROP_NAME.
    pub fn briefing_name(&self) -> Option<String> {
        let episode = self.fsm.episode();
        crate::movies::briefing_name_for_slot(episode.stage(), episode.mask())
    }

    /// Load the briefing intro pair (BRF_DROP.SMK bytes + the zone
    /// backdrop .SMK bytes the caller fetched under
    /// [`Self::briefing_name`]) onto the Brief scene (D37): the D31
    /// lifecycle applied to the EXW briefing screen movie head
    /// [verified asm 0043d447..0043d490] - inert until the FSM
    /// enters Brief (entry starts the drop pass and queues its
    /// frame-0 audio - the corpus pair is silent), the drop plays
    /// exactly one EXW pass (frames 0..=frames-2 render: 29 of its
    /// 30 corpus frames; the handoff bound is the frame index
    /// reaching count-1, never decoded), then the zone backdrop
    /// ring takes the plane for the rest of the scene (unbounded -
    /// the EXW ring closes only on the UI exit); dropped + stream
    /// cleared when Brief is left. The pair is UNSKIPPABLE (the GO
    /// button arms only after the handoff - no input path exists).
    /// Staging touches no hashed state (unit-pinned).
    pub fn load_briefing(&mut self, drop: &[u8], backdrop: &[u8]) -> Result<(), GameError> {
        self.brief = Some(BriefIntro::new(drop, backdrop)?);
        Ok(())
    }

    /// The staged briefing-intro phase, if any (D37 introspection:
    /// Staged / Drop / Backdrop / None when nothing is staged).
    pub fn brief_phase(&self) -> Option<BriefPhase> {
        self.brief.as_ref().map(|flow| flow.phase())
    }

    /// The staged briefing intro, if any (introspection for hosts
    /// and gates: phase, frame index of the on-screen movie).
    pub fn brief_intro(&self) -> Option<&BriefIntro> {
        self.brief.as_ref()
    }

    /// Load the shop backdrop movie (SHOP.SMK bytes, fetched under
    /// movies::shop_name) onto the Shop scene (D33): the 61-frame
    /// ring plays behind the shop UI - same D31 lifecycle, inert
    /// until the FSM enters Shop, dropped + stream cleared on
    /// leaving.
    pub fn load_shop(&mut self, data: &[u8]) -> Result<(), GameError> {
        self.load_movie(Scene::Shop, data)
    }

    /// Load the boot attract pair (GTLOG.SMK bytes then LOGO.SMK
    /// bytes, the caller fetching per [`crate::movies::boot_pair`])
    /// onto the Boot scene (D36): the D31 lifecycle applied to the
    /// two-movie EXW sequence - inert until the FSM stands on Boot
    /// (the host is constructed there, so the first pump starts it),
    /// each movie exactly one EXW pass (frames-1 renders, ring movies
    /// never wrap), GTLOG frame-0 audio queued at start and the LOGO
    /// frame-0 audio at the switch; dropped + stream cleared when
    /// Boot is left. The EXW pair is UNSKIPPABLE (skip gate 004edbc4
    /// reads 0 until NameEntryScreen arms it), so no input path
    /// exists. Staging touches no hashed state (unit-pinned).
    pub fn load_boot_attract(&mut self, gtlog: &[u8], logo: &[u8]) -> Result<(), GameError> {
        self.boot = Some(BootAttract::new(gtlog, logo)?);
        Ok(())
    }

    /// The attract phase, if one is staged (D36 introspection:
    /// Staged / Playing / Done / None when nothing is staged).
    pub fn boot_attract_phase(&self) -> Option<BootPhase> {
        self.boot.as_ref().map(|flow| flow.phase())
    }

    /// The staged attract flow, if any (introspection for hosts and
    /// gates: movie index 0 = GTLOG / 1 = LOGO, frame index, phase).
    pub fn boot_attract(&self) -> Option<&BootAttract> {
        self.boot.as_ref()
    }

    /// Stage the title menu (D41/D42) from the corpus bytes the
    /// caller fetched: LANGUAGE.* (the [MENU_ITEMS] table),
    /// FULLFONT.BIN (glyph sets at bases 0x82 and 0), FULLPAL.PAL
    /// (the ramp folded into the plane palette tail) and the SFX
    /// pair MENU1.RAW/MENU2.RAW (mixer instruments 0xE0/0xE1, the
    /// EXW SfxLoad pair at NameEntryScreen entry). The menu is
    /// presentation-only: staging touches no hashed state, and while
    /// it owns the Title input path the FSM is fed neutral frames
    /// (D42.1) - a staged menu drops when Title is left.
    pub fn load_title_menu(
        &mut self,
        language: &[u8],
        font_bin: &[u8],
        fullpal: &[u8],
        sfx_hover: &[u8],
        sfx_click: &[u8],
    ) -> Result<(), GameError> {
        let menu = TitleMenu::new(language, font_bin, fullpal)?;
        self.mixer.load_wave(crate::menu::SFX_HOVER, sfx_hover)?;
        self.mixer.load_wave(crate::menu::SFX_CLICK, sfx_click)?;
        self.menu = Some(menu);
        Ok(())
    }

    /// Stage the region-specific title backdrop after `load_title_menu`.
    /// The palette belongs to the menu and survives title movie playback.
    pub fn load_title_backdrop(&mut self, bin: &[u8], pal: &[u8]) -> Result<(), GameError> {
        self.menu
            .as_mut()
            .ok_or(GameError::BadMenuAsset {
                what: "title backdrop",
                reason: "title menu is not staged",
            })?
            .load_backdrop(bin, pal)
    }

    /// The staged title menu, if any (introspection: id, slots,
    /// selection, phase, idle, cursor, start score).
    pub fn menu(&self) -> Option<&TitleMenu> {
        self.menu.as_ref()
    }

    /// Score seed of the menu's last Start action (D42.8): 4000 -
    /// 500*difficulty at the moment Start was clicked. Cached on the
    /// host because the menu itself drops when the handoff leaves
    /// Title; the value waits for the P2d sim-tail wiring.
    pub fn menu_start_score_seen(&self) -> Option<i32> {
        self.menu_start_score
    }

    /// The mission slot the engine stages for the next Mission entry
    /// — the RUNTIME mission-number source [RE-EXW-SIM §7j.73]: a
    /// staged SELECT mission choice (the SELECT screen's MP write
    /// pair) overrides the campaign pair, with the load-time +5
    /// (`build_mission_paths` 0x4467df: mission cell + 5 in MP mode)
    /// applied — `{zone 2..=6, mission 1..=2}` names
    /// ZONE{B..F}/MISSION{6,7}, the MP-only missions. Otherwise the
    /// campaign episode selects (zone from the stage, mission from
    /// the mask — the same lowest-unset-bit arithmetic
    /// `briefing_name_for_slot` uses). The zone drives the file
    /// names, the edge family and the robots-per-player count.
    pub fn mission_slot(&self) -> (i32, i32) {
        if let Some((zone, mission)) = self.preparation_slot {
            return (i32::from(zone) - 1, i32::from(mission));
        }
        if let Some(select) = self.fsm.select_slot() {
            return (
                i32::from(select.zone()) - 1,
                i32::from(select.mission()) + crate::mission::SELECT_MP_FILE_OFFSET,
            );
        }
        let episode = self.fsm.episode();
        (
            crate::mission::zone_for_stage(episode.stage()),
            crate::mission::mission_number_for_mask(episode.mask()),
        )
    }

    /// The mission asset names for the current episode slot, in fetch
    /// order (see [`crate::mission::mission_asset_names`]).
    pub fn mission_asset_names(&self) -> Vec<String> {
        let (zone, mission) = self.mission_slot();
        crate::mission::mission_asset_names(zone, mission)
    }

    /// Stage the campaign episode slot — the D51 host seam (W12-S5,
    /// D108): the host stands in for the campaign-advance (0x41c9e5)
    /// / save-load-restore (0x43c2b8) shells the engine does not
    /// model, planting the slot whose mission the next Mission entry
    /// stages. Must run BEFORE `mission_asset_names`/`load_mission`
    /// (the zone drives the names, the edge family and the
    /// robots-per-player count). Returns false on an out-of-range
    /// slot (never guess).
    pub fn stage_episode_slot(&mut self, stage: u8, mask: u8) -> bool {
        self.preparation_slot = None;
        self.fsm.stage_episode_slot(stage, mask)
    }

    /// Stage the SELECT screen's mission choice — the §7j.73 sibling
    /// seam of [`GameHost::stage_episode_slot`]: the host stands in
    /// for the SELECT screen's MP write arm (0x43edc2..0x43ee43),
    /// planting the runtime `{zone, mission}` cell pair whose +5
    /// file offset (0x4467df) makes the next Mission entry load
    /// ZONE{B..F}/MISSION{6,7} — the MP-only missions the stage-mask
    /// campaign space cannot express (the census G1 class). Must run
    /// BEFORE `mission_asset_names`/`load_mission`; campaign staging
    /// (`stage_episode_slot`, the restore/advance stand-in) clears
    /// it. The campaign completion semantics stay campaign-shaped
    /// (an MP debrief is out of this seam's scope). Returns false
    /// outside the arm's write domain — zone 2..=6, mission 1..=2
    /// (never guess).
    pub fn stage_select_mission(&mut self, zone: u8, mission: u8) -> bool {
        self.fsm.stage_select_mission(zone, mission)
    }

    /// Import one ORIGINAL SAVED.BDL slot read-only and stage its
    /// campaign state (PLAN §6 P5 save seam; RE-EXW-SIM §7j.70).
    /// This is the engine's model of the original's save-load
    /// restore arm: the header walk (bounds-checked: exact 900-B
    /// file, slot < 5, the EXW zero-dword@+0x0C empty predicate,
    /// zone/mask inside the modeled episode space) plus the staging
    /// through the D51 seam — the zone-cell write and the mask replay
    /// are exactly what [`GameHost::stage_episode_slot`] models.
    /// `score`/`money`/`difficulty` are RETURNED, never staged
    /// (sim-side cells per DESIGN-GAME sec 3, the §7j.64 census); the
    /// import never writes anything back (new saves use the new
    /// versioned format).
    pub fn import_saved_slot(
        &mut self,
        data: &[u8],
        slot: usize,
    ) -> Result<SaveSlotImport, GameError> {
        let import = crate::save::import_saved_slot(data, slot)?;
        if !self.fsm.stage_episode_slot(import.stage, import.mask) {
            // Unreachable: the import domain is exactly the staging
            // domain (save.rs validates against the same tables).
            return Err(GameError::SaveSlotInvalid {
                slot,
                zone: i32::from(import.stage),
                mask: u32::from(import.mask),
            });
        }
        Ok(import)
    }

    /// Stage the mission (DESIGN-GAME sec 11) from the corpus bytes
    /// the caller fetched, in [`GameHost::mission_asset_names`]
    /// order: TOT, DAT, PAD, CGR, BIN, LNK, SINTABLE, DANTE, GAMEPAL,
    /// GENERAL, SMLFONT, MRK, TABLE, MAPTRAN0..7, MIN, NUMBERS.
    /// GAMEPAL (770 B) is the mission plane palette (folds to the
    /// canonical 6-bit form; MISSIONVIEW sec 6); GENERAL.BIN +
    /// SMLFONT.BIN are the sidebar art banks (RE-EXW-SIM sec 6c.8c);
    /// NUMBERS.BIN is the score-strip bank (7f.9); TABLE.BIN is the
    /// strategic-map backdrop bank, the eight MAPTRAN ramps + the
    /// mission `.MIN` are the map-overlay family (RE-EXW-SIM 7e).
    /// The zone comes from the episode slot (consistent with the
    /// names the chain fetched); `staged_markers` is the host/test
    /// seam the network override 0x46cbe0 fills in the original
    /// (RE-EXW-SIM sec 7c.8) — a staged marker spawns one extra
    /// robot at activation-free spawn time. The scene is INERT until
    /// the FSM enters Mission and drops on exit; staging touches no
    /// hashed state (the movie pattern, D31).
    #[allow(clippy::too_many_arguments)]
    pub fn load_mission(
        &mut self,
        tot: &[u8],
        dat: &[u8],
        pad: &[u8],
        cgr: &[u8],
        bin: &[u8],
        lnk: &[u8],
        sintable: &[u8],
        dante: &[u8],
        gamepal: &[u8],
        general: &[u8],
        smlfont: &[u8],
        mrk: &[u8],
        flags: &[u8],
        blowup: &[u8],
        table: &[u8],
        maptran: &[&[u8]],
        min: &[u8],
        numbers: &[u8],
        robots_override: Option<usize>,
        staged_markers: &[(i32, i32, i32)],
    ) -> Result<(), GameError> {
        let (zone, mission_no) = self.mission_slot();
        let mut mission = MissionScene::stage(
            tot,
            dat,
            pad,
            cgr,
            mrk,
            bin,
            lnk,
            sintable,
            dante,
            gamepal,
            general,
            smlfont,
            numbers,
            flags,
            blowup,
            table,
            min,
            maptran,
            zone,
            robots_override,
            staged_markers,
        )?;
        // The tile-claim bank (S0-11b, §7j.63): the original stages
        // it at EVERY MissionShell mission load (the 0x447b85
        // FUN_004254e1 call — deterministic hardcoded data, no RNG
        // draws, no hashed fields), so E stages it right here at
        // load — not a scenario key. [0x4edd8c] is the 1-based
        // terrain set (zone_index + 1, D99); [0x4edd88] the
        // within-zone mission number.
        mission
            .sim_mut()
            .stage_claim_bank((zone + 1) as u32, mission_no as u32);
        if let Some(preparation) = &mut self.preparation {
            if self.preparation_slot.is_some() {
                preparation.deploy(&mut mission);
            }
        }
        if self.fsm.scene() == Scene::Mission {
            mission.activate();
        }
        self.mission = Some(mission);
        Ok(())
    }

    /// Stage the real selection/armoury flow. Asset loading is atomic: failed
    /// staging preserves the current scene and its existing preparation.
    pub fn load_preparation(
        &mut self,
        source: &mut dyn ByteSource,
        zone: u8,
        completed: [bool; 27],
        flags: [u32; 15],
        balance: u32,
        language: &str,
    ) -> Result<(), GameError> {
        let preparation = crate::armoury::journey::Preparation::load(
            source, zone, completed, flags, balance, language,
        )?;
        self.preparation = Some(preparation);
        self.preparation_slot = None;
        self.preparation_transition = false;
        self.fsm.enter(Scene::Select);
        self.frame = self.render_now();
        Ok(())
    }

    pub fn preparation_return_pending(&self) -> bool {
        self.fsm.scene() == Scene::Select
            && self.preparation.as_ref().is_some_and(|p| p.room_pending())
    }

    pub fn resume_preparation(
        &mut self,
        source: &mut dyn ByteSource,
        language: &str,
    ) -> Result<(), GameError> {
        if self.preparation_return_pending() {
            self.preparation
                .as_mut()
                .expect("pending preparation")
                .reload_room(source, language)?;
            self.sync_loading();
            self.frame = self.render_now();
        }
        Ok(())
    }

    pub fn preparation(&self) -> Option<&crate::armoury::journey::Preparation> {
        self.preparation.as_ref()
    }

    /// The staged mission, if any (gate/host introspection: camera,
    /// cursor, render count, the sim slice).
    pub fn mission(&self) -> Option<&MissionScene> {
        self.mission.as_ref()
    }

    /// The mutable mission seam [D51]: in the original the weapon
    /// loadout (the .bss session table at 0x4de664) is written by
    /// the shop FUN_00440e45 / a save-load / the MP lobby BEFORE the
    /// mission [RE-EXW-SIM 7d]; those shells are not modeled, so the
    /// host stands in for them via
    /// [`MissionScene::set_weapon_loadout`]. Nothing else should
    /// reach through here.
    pub fn mission_mut(&mut self) -> Option<&mut MissionScene> {
        self.mission.as_mut()
    }

    /// Type one character into the active name entry (the explicit
    /// shell text path, D42.6). False when no name entry is active.
    pub fn menu_type_char(&mut self, c: u8) -> bool {
        self.menu.as_mut().is_some_and(|m| m.type_char(c))
    }

    /// Backspace in the active name entry (D42.6).
    pub fn menu_backspace(&mut self) -> bool {
        self.menu.as_mut().is_some_and(|m| m.backspace())
    }

    /// Apply one menu tick report (inside the executed-tick loop):
    /// SFX straight to the mixer, host intents via the FSM, attract
    /// events against the staged Title movie slot. Nothing here
    /// touches hashed state beyond the FSM's own apply.
    fn apply_menu_tick(&mut self, tick: crate::menu::MenuTick) {
        if tick.hover_sfx {
            self.mixer.note_on(
                crate::menu::SFX_HOVER,
                crate::menu::SFX_RATIO_UNITY,
                crate::menu::SFX_VOLUME,
            );
        }
        if tick.click_sfx {
            self.mixer.note_on(
                crate::menu::SFX_CLICK,
                crate::menu::SFX_RATIO_UNITY,
                crate::menu::SFX_VOLUME,
            );
        }
        match tick.action {
            // Start: the Title -> Brief handoff (the P4 slice's
            // "start action hands off"); the score seed is cached on
            // the host for the future sim-tail wiring (D42.8).
            Some(MenuAction::Start { score }) => {
                self.menu_start_score = Some(score);
                self.apply(SceneAction::Advance);
            }
            Some(MenuAction::Quit) => self.apply(SceneAction::Quit),
            // Attract: restart the staged Title movie in place (the
            // EXW replay re-opens TITLE.SMK) and queue its frame-0
            // audio - the D31 entry rule applied to the replay. No
            // staged title movie = cancel the attract (D42.3).
            Some(MenuAction::Attract) => {
                let mut audio = Vec::new();
                let mut started = false;
                let mut failed = false;
                if let Some(slot) = self.movie.as_mut() {
                    if slot.scene == Scene::Title {
                        match slot.player.restart() {
                            Ok(()) => {
                                slot.started = true;
                                started = true;
                                audio = slot.player.take_audio();
                            }
                            Err(_) => failed = true,
                        }
                    }
                }
                if !started && !failed {
                    if let Some(menu) = self.menu.as_mut() {
                        menu.cancel_attract();
                    }
                }
                if !audio.is_empty() && self.mixer.queue_pcm_u8(&audio).is_err() {
                    failed = true;
                }
                if failed {
                    // The D31 self-terminate pattern: drop the slot,
                    // silence the stream (the menu falls back to
                    // Interactive on the next tick).
                    self.movie = None;
                    self.mixer.clear_pcm_stream();
                    if let Some(menu) = self.menu.as_mut() {
                        menu.cancel_attract();
                    }
                }
            }
            // Skip: finish the replay in place; the menu plane takes
            // over once the player stops playing (D42.3).
            Some(MenuAction::SkipAttract) => {
                if let Some(slot) = self.movie.as_mut() {
                    if slot.scene == Scene::Title {
                        slot.player.finish();
                    }
                }
            }
            None => {}
        }
    }

    /// Whether a Title-scene movie owns the screen: one that is
    /// The staged menu cursor (game space), for window-host absolute-
    /// pointer steering; None when no menu is staged (mission aiming
    /// stays relative and is never steered).
    pub fn menu_cursor(&self) -> Option<(i32, i32)> {
        self.menu.as_ref().map(|m| m.cursor())
    }

    /// Cinematic skip router (operator 2026-08-23). One frame-level
    /// intent: Space/Enter (ADVANCE), Escape, or left-click while a
    /// cinematic holds the screen finishes it NOW, so the ordinary
    /// end-of-movie sync paths advance exactly as on natural
    /// completion (parity baselines are inputless and unaffected).
    /// Skippable: the Title entry movie (EXW skip gate 004edbc4 is
    /// armed there - MoviePlayer::finish is the D42 skip) and the
    /// Cutscene movie (the D34 loading tail runs unconditionally
    /// after the movie call). The Boot attract pair is UNSKIPPABLE
    /// in EXW (the gate reads 0 there); skipping it is an operator
    /// modernization - the flow is dropped like an early Boot leave
    /// and an explicit Advance intent zeroes the countdown.
    /// Brief (unskippable, D37) and the Shop ambient ring are not
    /// cinematics and are never touched.
    fn apply_cinema_skip(&mut self, input: &InputFrame) {
        if !cinema_skip_requested(input) {
            return;
        }
        if self.title_movie_playing() {
            if let Some(slot) = self.movie.as_mut() {
                slot.player.finish();
            }
            return;
        }
        let cutscene_holds = self
            .movie
            .as_ref()
            .is_some_and(|slot| slot.scene == Scene::Cutscene && !slot.player.finished());
        if cutscene_holds {
            if let Some(slot) = self.movie.as_mut() {
                slot.player.finish();
            }
            return;
        }
        if self.fsm.scene() == Scene::Boot && self.boot.is_some() {
            self.boot = None;
            self.mixer.clear_pcm_stream();
            self.fsm.apply(SceneAction::Advance);
        }
    }

    /// playing (started, not finished) OR staged but not yet started
    /// by the scene sync (the load-to-sync gap of one pump - the
    /// EXW title movie takes the screen before the menu loop runs).
    /// The menu is inert behind it either way, and its plane yields.
    fn title_movie_playing(&self) -> bool {
        self.movie.as_ref().is_some_and(|slot| {
            slot.scene == Scene::Title && (!slot.started || !slot.player.finished())
        })
    }

    /// Title-menu scene sync (D42.1): a menu standing on Title goes
    /// live; one that was live is DROPPED when Title is left
    /// (NameEntryScreen owns everything and frees it on exit). A
    /// menu staged while the host stands elsewhere stays inert,
    /// waiting for its scene (the Staged semantics of the other
    /// flows). The SFX waves stay in the mixer either way.
    fn sync_menu(&mut self) {
        if let Some(menu) = self.menu.as_mut() {
            if self.fsm.scene() == Scene::Title {
                menu.mark_entered();
            } else if menu.entered() {
                self.menu = None;
            }
        }
    }

    /// Mission lifecycle (DESIGN-GAME sec 11): entering Mission
    /// activates the staged scene (fixing the camera at the spawn);
    /// leaving DROPS it (the flow never ends on its own, like the
    /// briefing backdrop). A mission staged but never entered stays
    /// staged (the menu entered/drop pattern). Called once per pump
    /// AFTER the tick loop, so the entry pump renders but does not
    /// tick the sim.
    fn sync_mission(&mut self) {
        if let Some(mission) = self.mission.as_mut() {
            if self.fsm.scene() == Scene::Mission {
                mission.activate();
            } else if mission.is_active() {
                self.mission = None;
            }
        }
    }

    /// Stage the zone-transition interlude still (BETWEEN.BIN entry-0
    /// bytes the caller fetched under [`crate::movies::interlude_name`])
    /// for the post-cutscene flow (D34). Inert until the FSM stands on
    /// Cutscene with the cutscene movie finished or absent; the still
    /// then owns the plane under the standing host palette until the
    /// Cutscene -> Select advance. Staging touches no hashed state
    /// (unit-pinned like the D31 movie isolation).
    pub fn load_interlude(&mut self, bin: &[u8]) -> Result<(), GameError> {
        let still = crate::loading::decode_entry0(bin)?;
        self.stage_loading_slot().between = Some(still);
        Ok(())
    }

    /// The staging target: a flow still in its Staged phase absorbs
    /// the new part; anything ACTIVE (a leftover Between/Loading from
    /// a transition whose drop has not pumped yet) is replaced by a
    /// fresh staged flow - the load_movie replace-the-slot semantics,
    /// applied per staged part.
    fn stage_loading_slot(&mut self) -> &mut crate::loading::LoadingFlow {
        let staged = matches!(
            self.loading.as_ref().map(|flow| flow.phase),
            None | Some(crate::loading::LoadingPhase::Staged)
        );
        if staged {
            self.loading
                .get_or_insert_with(crate::loading::LoadingFlow::staged);
        } else {
            self.loading = Some(crate::loading::LoadingFlow::staged());
        }
        self.loading.as_mut().unwrap()
    }

    /// Stage the region-variant loading screen (LOAD_UK/US.BIN
    /// entry-0 bytes + LOADPAL/LOADPALU.PAL bytes the caller fetched
    /// per [`crate::movies::Region`]) for the post-cutscene flow
    /// (D34). It owns the Select plane on the Cutscene -> Select
    /// transition, fading in from black over FadeSetup 10 steps at
    /// 50 Hz, and holds until Select is left. The palette folds to
    /// 6-bit; the 224..=255 tail carries the staged FULLPAL ramp once
    /// [`Self::load_loading_font`] ran (D35), else the folded file
    /// values.
    pub fn load_loading_screen(&mut self, bin: &[u8], pal: &[u8]) -> Result<(), GameError> {
        let still = crate::loading::decode_entry0(bin)?;
        let target = crate::loading::loading_palette(pal)?;
        let flow = self.stage_loading_slot();
        flow.screen = Some(still);
        flow.target = Some(target);
        Ok(())
    }

    /// Stage the loading-text pass (D35): FULLFONT.BIN bytes +
    /// FULLPAL.PAL bytes + the LANGUAGE.* bytes (the caller fetches
    /// whichever language file its EXW language index 004eba1c
    /// selected - the host never reads game-data). Inert until the
    /// flow enters Loading: the four FUN_0043c87c draws then land on
    /// the loading-screen raster (rows 150/180/210, +260 for zone 6,
    /// strings = table entries 0x45/0x46/zone+0x51/0x58) and the ramp
    /// replaces the fade-target tail (EXW order: draws, then the ramp
    /// copy, then FadeSetup). A bad font bank / ramp / language file
    /// is a staging error; staging touches no hashed state.
    pub fn load_loading_font(
        &mut self,
        font_bin: &[u8],
        fullpal: &[u8],
        language: &[u8],
    ) -> Result<(), GameError> {
        let font = crate::font::LoadingFont::from_bank(font_bin)?;
        let ramp = bedlam_assets::pal::parse_font_ramp(fullpal)?;
        let table = bedlam_assets::language::parse_menu_items(language)?;
        let flow = self.stage_loading_slot();
        flow.font = Some(font);
        flow.ramp = Some(ramp);
        flow.table = Some(table);
        Ok(())
    }

    /// The flow presentation phase, if one is staged (D34
    /// introspection: Staged / Between / Loading).
    pub fn loading_phase(&self) -> Option<LoadingPhase> {
        self.loading.as_ref().map(|flow| flow.phase)
    }

    /// Fade steps completed on the active loading screen (0..=10);
    /// None when no screen is fading.
    pub fn loading_fade_step(&self) -> Option<u16> {
        self.loading
            .as_ref()
            .filter(|flow| flow.phase == LoadingPhase::Loading)
            .map(|flow| flow.fade_step)
    }

    /// The pinned text rows of the active loading screen (D35): the
    /// zone-dependent draw rows of the four FUN_0043c87c text draws
    /// (the draws themselves already ran onto the plane at Loading
    /// entry through the staged font).
    pub fn loading_text_row(&self) -> Option<TextRow> {
        self.loading
            .as_ref()
            .filter(|flow| flow.phase == LoadingPhase::Loading)
            .and_then(|flow| flow.text_row)
    }

    /// Loading-flow scene sync (D34): the EXW zone-transition tail as
    /// phase transitions. The flow only runs on the zone-transition
    /// arm (episode stages 2..=7; the endgame arm loads no BETWEEN /
    /// LOAD assets, so a staged flow is dropped there and never
    /// activates). Between arms on Cutscene once the cutscene movie
    /// is over; Loading arms on the Select entry that follows a
    /// Cutscene the flow saw (a skip-advance still runs the loading
    /// screen - the EXW tail is unconditional after the movie call);
    /// leaving the flow scenes drops an ACTIVE flow, while a staged
    /// one keeps waiting for its cutscene.
    fn sync_loading(&mut self) {
        let scene = self.fsm.scene();
        let stage = self.fsm.episode().stage();
        let cutscene_movie_holds = self
            .movie
            .as_ref()
            .is_some_and(|slot| slot.scene == Scene::Cutscene && !slot.player.finished());
        enum Decision {
            Keep,
            CutsceneHold,
            Drop,
            Between,
            Loading(u8),
        }
        let decision = match self.loading.as_ref() {
            None => Decision::Keep,
            Some(flow) => match scene {
                Scene::Cutscene | Scene::Select if !crate::loading::flow_armed_at_stage(stage) => {
                    Decision::Drop
                }
                Scene::Cutscene => {
                    if flow.phase == LoadingPhase::Staged && !cutscene_movie_holds {
                        Decision::Between
                    } else {
                        Decision::CutsceneHold
                    }
                }
                Scene::Select => {
                    if flow.saw_cutscene && flow.phase != LoadingPhase::Loading {
                        Decision::Loading(stage.saturating_sub(1))
                    } else {
                        Decision::Keep
                    }
                }
                _ => {
                    if flow.phase != LoadingPhase::Staged {
                        Decision::Drop
                    } else {
                        Decision::Keep
                    }
                }
            },
        };
        match decision {
            Decision::Keep => {}
            Decision::Drop => self.loading = None,
            Decision::CutsceneHold | Decision::Between => {
                if let Some(flow) = self.loading.as_mut() {
                    flow.saw_cutscene = true;
                    if matches!(decision, Decision::Between) {
                        flow.enter_between();
                    }
                }
            }
            Decision::Loading(zone) => {
                if let Some(flow) = self.loading.as_mut() {
                    flow.enter_loading(zone);
                }
            }
        }
    }

    /// Loading-flow time pump (D34): feed the fade engine the same dt
    /// the host pumped - the 50 Hz steps bank on the 240 Hz grid
    /// exactly like movie frame periods.
    fn pump_loading(&mut self, dt_subticks: u32) {
        if let Some(flow) = self.loading.as_mut() {
            flow.advance(dt_subticks);
        }
        if self.preparation_transition
            && self.fsm.scene() == Scene::Select
            && !self.preparation_return_pending()
            && self.loading_fade_step().is_none_or(|step| step == 10)
        {
            self.preparation_transition = false;
            self.loading = None;
        }
    }

    /// Movie step (D31): advance the started player on the same dt the
    /// sim consumed, then queue whatever decoded. A decode failure
    /// mid-playback stops the movie and silences the stream -
    /// presentation self-terminates rather than propagating into the
    /// hash-bearing pump (the stream was structurally validated at
    /// load; only corrupt frame data reaches this arm).
    fn pump_movie(&mut self, dt_subticks: u32) {
        let mut failed = false;
        let mut game_over_finished = false;
        let mut campaign_movie_finished = false;
        if let Some(slot) = self.movie.as_mut() {
            if slot.started {
                let advanced = if slot.scene == Scene::Cutscene {
                    // FUN_0044567c displays frames-1 even for ring movies.
                    // Keep the last shown frame for its full period, as Boot does.
                    let target = slot.player.info().frames.saturating_sub(1).max(1);
                    slot.elapsed_subticks += u64::from(dt_subticks);
                    let result = slot
                        .player
                        .advance_limited(dt_subticks, target.saturating_sub(slot.decoded));
                    if let Ok(decoded) = result {
                        slot.decoded += decoded;
                        if slot.elapsed_subticks * 1_000_000
                            >= u64::from(target) * slot.player.info().us_per_frame * 240
                        {
                            slot.player.finish();
                        }
                    }
                    result.map(|_| ())
                } else {
                    slot.player.advance(dt_subticks)
                };
                if advanced.is_ok() {
                    game_over_finished = slot.scene == Scene::GameOver && slot.player.finished();
                    campaign_movie_finished = self.preparation_transition
                        && slot.scene == Scene::Cutscene
                        && slot.player.finished();
                    let packets = slot.player.take_audio();
                    // A stream-bus overflow means the host stopped
                    // draining audio (STREAM_CAP_BYTES has 16x headroom
                    // over the whole TITLE track) - self-terminate the
                    // movie rather than propagate into the pump.
                    if self.mixer.queue_pcm_u8(&packets).is_err() {
                        failed = true;
                    }
                } else {
                    failed = true;
                }
            }
        }
        if failed {
            self.movie = None;
            self.mixer.clear_pcm_stream();
        }
        if game_over_finished || campaign_movie_finished {
            self.fsm.apply(SceneAction::Advance);
        }
    }

    /// Scene-boundary movie lifecycle: start on entering the target
    /// scene (frame-0 audio queues HERE, one pump before any decode),
    /// stop on leaving it.
    fn sync_movie(&mut self) {
        let scene = self.fsm.scene();
        let mut stop = false;
        if let Some(slot) = self.movie.as_mut() {
            if !slot.started {
                if scene == slot.scene {
                    slot.started = true;
                    let first = slot.player.take_audio();
                    // Stream-bus overflow at start = the host stopped
                    // draining audio (16x-headroom cap): drop the slot
                    // instead of ignoring the failure.
                    if self.mixer.queue_pcm_u8(&first).is_err() {
                        stop = true;
                    }
                }
            } else if scene != slot.scene {
                stop = true;
            }
        }
        if stop {
            self.movie = None;
            self.mixer.clear_pcm_stream();
        }
    }

    /// Boot-attract scene sync (D36): start on the Boot scene (the
    /// GTLOG frame-0 audio queues HERE, one pump before any decode -
    /// the D31 entry semantics), stop + drop on leaving it. The EXW
    /// boot attract is unskippable, so there is no input abort path.
    fn sync_boot(&mut self) {
        let mut stop = false;
        if let Some(flow) = self.boot.as_mut() {
            match self.fsm.scene() {
                Scene::Boot => {
                    if flow.phase() == BootPhase::Staged {
                        let first = flow.start();
                        // Stream-bus overflow at start = the host
                        // stopped draining audio (16x-headroom cap):
                        // drop the flow, the D31 start pattern.
                        if self.mixer.queue_pcm_u8(&first).is_err() {
                            stop = true;
                        }
                    }
                }
                _ => stop = true,
            }
        }
        if stop {
            self.boot = None;
            self.mixer.clear_pcm_stream();
        }
    }

    /// Boot-attract time pump (D36): the same dt the sim consumed; the
    /// decoded PCM rides the D31 stream bus. A decode failure or
    /// stream overflow self-terminates the flow (the D31
    /// movie-lifecycle pattern - presentation self-terminates, never a
    /// pump error).
    fn pump_boot(&mut self, dt_subticks: u32) {
        let mut failed = false;
        if let Some(flow) = self.boot.as_mut() {
            match flow.advance(dt_subticks) {
                Ok(packets) => {
                    if self.mixer.queue_pcm_u8(&packets).is_err() {
                        failed = true;
                    }
                }
                Err(_) => failed = true,
            }
        }
        if failed {
            self.boot = None;
            self.mixer.clear_pcm_stream();
        }
    }

    /// Briefing-intro scene sync (D37, the sync_movie semantics):
    /// start when the FSM ENTERS Brief (the drop frame-0 audio
    /// queues HERE, one pump before any decode - the D31 entry
    /// rule); a STAGED flow stays inert on every other scene, an
    /// ACTIVE one is dropped on leaving Brief (the backdrop ring
    /// plays until the UI exits the scene - the flow never ends by
    /// itself). The EXW pair is unskippable, so there is no input
    /// abort path.
    fn sync_brief(&mut self) {
        let mut stop = false;
        if let Some(flow) = self.brief.as_mut() {
            if flow.phase() == BriefPhase::Staged {
                if self.fsm.scene() == Scene::Brief {
                    let first = flow.start();
                    // Stream-bus overflow at start = the host
                    // stopped draining audio (16x-headroom cap):
                    // drop the flow, the D31 start pattern.
                    if self.mixer.queue_pcm_u8(&first).is_err() {
                        stop = true;
                    }
                }
            } else if self.fsm.scene() != Scene::Brief {
                stop = true;
            }
        }
        if stop {
            self.brief = None;
            self.mixer.clear_pcm_stream();
        }
    }

    /// Briefing-intro time pump (D37): the same dt the sim
    /// consumed; the decoded PCM rides the D31 stream bus. A decode
    /// failure or stream overflow self-terminates the flow (the
    /// D31 movie-lifecycle pattern - presentation self-terminates,
    /// never a pump error).
    fn pump_brief(&mut self, dt_subticks: u32) {
        let mut failed = false;
        if let Some(flow) = self.brief.as_mut() {
            match flow.advance(dt_subticks) {
                Ok(packets) => {
                    if self.mixer.queue_pcm_u8(&packets).is_err() {
                        failed = true;
                    }
                }
                Err(_) => failed = true,
            }
        }
        if failed {
            self.brief = None;
            self.mixer.clear_pcm_stream();
        }
    }

    /// Parity render pass: canonical frame from the current sim —
    /// `prev_sim = None, alpha = 0` (the parity/golden
    /// configuration, D17: interpolation OFF; identical bytes with or
    /// without the P6 interpolation fields staged). See
    /// [`GameHost::render_with`] for the shared body.
    fn render_now(&mut self) -> Frame {
        self.render_with(None, 0.0)
    }

    /// The shared composition body. `prev` + `alpha` are the D12
    /// camera-interpolation inputs (P6 `p6-high-refresh-interpolation`):
    /// PRESENTATION inputs only — they shape the interpolated camera
    /// (integer-grid quantized, scroll-bounds clamped) and nothing
    /// else; with `prev = None` the output depends only on the
    /// current sim + palette (the parity contract). A started movie
    /// REPLACES the scene pipeline (D31); an active loading-flow
    /// plane (D34) or boot/brief attract plane takes priority the
    /// same way, and the mission/menu planes own the screen on their
    /// scenes — so a presented non-scene plane is
    /// interpolation-invariant by construction (the interpolated
    /// camera only exists in the scene path).
    fn render_with(&mut self, prev: Option<&Sim>, alpha: f32) -> Frame {
        // Menu gate inputs (D41/D42), hoisted before the disjoint
        // mission/menu plane borrows below.
        let title_movies_playing = self.title_movie_playing();
        // Mission plane (DESIGN-GAME sec 11): the active mission owns
        // the screen — the 480x480 viewport window at (0,0) plus the
        // black sidebar, under the mission's OWN folded GAMEPAL
        // (staged with the mission; MISSIONVIEW sec 6). Highest
        // precedence: scenes are exclusive and the mission drops when
        // Mission is left.
        let mission_frame = if self.fsm.scene() == Scene::Mission {
            self.mission
                .as_mut()
                .and_then(|mission| mission.plane())
                .map(|p| MovieFrame {
                    width: p.w,
                    height: p.h,
                    pixels: p.pixels,
                    palette: p.palette,
                })
        } else {
            None
        };
        // Menu plane (D41/D42): the staged menu owns the Title plane
        // whenever no Title movie is playing (the first pass and the
        // attract replay own it instead - the menu is inert behind
        // them and redraws when they end).
        let menu_frame = if title_movies_playing || self.fsm.scene() != Scene::Title {
            None
        } else {
            self.menu
                .as_mut()
                .and_then(|menu| menu.plane(&self.palette))
                .map(|p| MovieFrame {
                    width: p.w,
                    height: p.h,
                    pixels: p.pixels,
                    palette: p.palette,
                })
        };
        let preparation_frame = if !self.preparation_transition
            && matches!(self.fsm.scene(), Scene::Select | Scene::Shop)
        {
            self.preparation.as_ref().map(|p| MovieFrame {
                width: 640,
                height: 480,
                pixels: p.pixels(),
                palette: *p.palette(),
            })
        } else {
            None
        };
        let movie = self
            .loading
            .as_ref()
            .and_then(|flow| flow.plane(&self.palette))
            .map(|p| MovieFrame {
                width: p.w,
                height: p.h,
                pixels: p.pixels,
                palette: p.palette,
            })
            .or_else(|| {
                self.boot
                    .as_ref()
                    .and_then(|flow| flow.plane())
                    .map(|p| MovieFrame {
                        width: p.w,
                        height: p.h,
                        pixels: p.pixels,
                        palette: p.palette,
                    })
            })
            .or_else(|| {
                self.brief
                    .as_ref()
                    .and_then(|flow| flow.plane())
                    .map(|p| MovieFrame {
                        width: p.w,
                        height: p.h,
                        pixels: p.pixels,
                        palette: p.palette,
                    })
            })
            .or(mission_frame)
            .or(menu_frame)
            .or_else(|| {
                self.movie
                    .as_ref()
                    .filter(|slot| slot.started)
                    .map(|slot| MovieFrame {
                        width: slot.player.info().width,
                        height: slot.player.info().height,
                        pixels: slot.player.pixels(),
                        palette: slot.player.palette(),
                    })
            });
        let input = RenderInput {
            sim: self.driver.sim(),
            // P6 camera interpolation (p6-high-refresh-interpolation):
            // the parity path (pump) passes None/0.0 — interpolation
            // enters ONLY through recompose() at the present site.
            prev_sim: prev,
            alpha,
            palette: self.palette,
            movie: preparation_frame.or(movie),
        };
        render(&input)
    }

    /// Attach/detach the music script on scene changes (DESIGN-GAME
    /// sec 5): a scene with a track gets the pre-built script; every
    /// other scene gets silence (an empty script).
    fn sync_music(&mut self) {
        let scene = self.fsm.scene();
        if self.music_scene == Some(scene) {
            return;
        }
        self.music_scene = Some(scene);
        let script = match (music::track_name(scene), self.music.as_ref()) {
            (Some(_), Some(pump)) => pump.script().clone(),
            _ => MusicScript::new(),
        };
        self.mixer.load_script(script);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsm::{BOOT_TICKS, MAX_STAGE};

    fn palette() -> [Vga6; 256] {
        [[0, 0, 0]; 256]
    }

    fn synth_mrs_bytes() -> Vec<u8> {
        use bedlam_assets::music::Mrs;
        let mut stream: Vec<u8> = Vec::new();
        stream.extend_from_slice(&3u16.to_le_bytes());
        stream.push(0x60);
        stream.push(10);
        stream.extend_from_slice(&2u16.to_le_bytes());
        stream.push(0x60);
        stream.push(0xFF);
        stream.extend_from_slice(&0xFFFFu16.to_le_bytes());
        Mrs {
            chunk_count: 2,
            chan_count: 1,
            sizes: vec![0, stream.len() as u16],
            variants: vec![0, 1],
            start_offsets: vec![0xFFFF, 0],
            tick_delays: vec![0, 5],
            table_c: vec![0, 0],
            data_off: 28,
            streams: vec![Vec::new(), stream],
        }
        .to_bytes()
    }

    #[test]
    fn cinema_skip_chord_levels() {
        let neutral = InputFrame::default();
        assert!(!cinema_skip_requested(&neutral));
        let space = InputFrame {
            buttons: 1 << 10,
            ..InputFrame::default()
        };
        let escape = InputFrame {
            buttons: 1 << 9,
            ..InputFrame::default()
        };
        let click = InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        };
        assert!(cinema_skip_requested(&space));
        assert!(cinema_skip_requested(&escape));
        assert!(cinema_skip_requested(&click));
    }

    #[test]
    fn pump_executes_sim_and_fsm_on_the_same_grid() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        assert_eq!(host.scene(), Scene::Boot);
        for _ in 0..BOOT_TICKS {
            assert_eq!(host.pump_frame(4, &InputFrame::default()), 1);
        }
        assert_eq!(host.scene(), Scene::Title);
        assert_eq!(host.driver().sim().tick_index(), BOOT_TICKS as u64);
        // 3 sub-ticks bank without ticking anything.
        assert_eq!(host.pump_frame(3, &InputFrame::default()), 0);
        assert_eq!(host.driver().sim().tick_index(), BOOT_TICKS as u64);
    }

    #[test]
    fn mode_seam_is_injected_and_carried_not_mutated() {
        // P6 seam (D201): the ONE immutable ModeConfig rides SimConfig
        // into GameHost::new (sim construction) and is readable on the
        // host; there is no host-side mode setter (a mode change is a
        // new host). Default = modern; both arms of the plan-named
        // axes construct and pump identically — the seam lands inert,
        // which is why the canonical chains cannot move.
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        assert_eq!(
            GameHost::new(&GameConfig::default(), &SimConfig::default(), palette()).mode(),
            ModeConfig::MODERN,
            "default mode = modern (PLAN §6)"
        );
        for config in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let mut host = GameHost::new(
                &GameConfig::default(),
                &SimConfig {
                    mode: config,
                    ..SimConfig::default()
                },
                palette(),
            );
            assert_eq!(host.mode(), config);
            assert_eq!(host.mode(), host.driver().sim().mode());
            // Pumps normally in both arms (Boot -> ... grid unchanged).
            assert_eq!(host.pump_frame(4, &InputFrame::default()), 1);
            assert_eq!(host.scene(), Scene::Boot);
        }
        // The preset builders compose the same axes the host reads.
        let mixed = ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic);
        assert!(mixed.is_purist(PuristToggle::TimingLock));
        assert!(!mixed.is_purist(PuristToggle::ControlScheme));
    }

    #[test]
    fn timing_lock_pacing_selects_from_the_immutable_mode() {
        // P6 first consumer (D203): the timing-lock arm SELECTS the
        // present pacing policy on the host — a policy, never a Hz.
        // The surface is the ONE purist toggle (both arms); the
        // control-scheme axis is a targeted CONTROL: flipping IT
        // alone must not move this consumer (axis independence).
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        assert_eq!(modern.present_pacing(), PresentPacing::Decoupled);
        assert!(modern.should_present(), "boot frame is presentable");

        let purist_timing = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        assert_eq!(purist_timing.present_pacing(), PresentPacing::FrameLocked);
        // The CLASSIC preset carries the purist timing arm too.
        let classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::CLASSIC,
                ..SimConfig::default()
            },
            palette(),
        );
        assert_eq!(classic.present_pacing(), PresentPacing::FrameLocked);

        // Control (the OTHER axis, modern): pacing stays decoupled.
        let purist_controls = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        assert_eq!(purist_controls.present_pacing(), PresentPacing::Decoupled);
    }

    #[test]
    fn modern_pacing_presents_every_host_frame_at_high_refresh() {
        // MODERN arm (PLAN §6 high-refresh present): a 240 Hz-style
        // script (dt = 1 sub-tick per pump) mostly executes ZERO
        // ticks, and every frame still presents — the accumulator-
        // driven decoupled present. 240 pumps = 60 whole ticks.
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let mut ticks = 0u32;
        for _ in 0..240 {
            let executed = host.pump_frame(1, &InputFrame::default());
            ticks += executed;
            assert!(host.should_present(), "decoupled: zero-tick frames present");
        }
        assert_eq!(ticks, 60, "240 x 1 sub-tick = 60 whole ticks");
    }

    #[test]
    fn classic_pacing_locks_present_to_the_logic_frame() {
        // CLASSIC arm: the original frame-locked present-coupled
        // pacing (RE-EXW-PACER §3 — one sim/render frame per display
        // flip). The SAME 240 Hz-style script as the modern test:
        // presentable exactly on the pumps that executed a tick, the
        // zero-tick frames hold the previous image; and before the
        // first pump the boot frame is presentable (the platform
        // must blit once to show anything at all).
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};
        let mut host = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        assert!(host.should_present(), "boot frame presentable");
        let mut ticks = 0u32;
        let mut presents = 0u32;
        for _ in 0..240 {
            let executed = host.pump_frame(1, &InputFrame::default());
            ticks += executed;
            let due = host.should_present();
            assert_eq!(
                due,
                executed > 0,
                "frame-locked: present iff the pump executed a tick"
            );
            presents += u32::from(due);
        }
        // 60 ticks, 60 presents — the visible refresh follows the
        // fixed logic tick, never the host's faster display rate.
        assert_eq!(ticks, 60);
        assert_eq!(presents, 60);
    }

    #[test]
    fn classic_pacing_on_a_60hz_host_presents_every_flip() {
        // The original display class (60 Hz): dt = 4 sub-ticks = one
        // tick per pump, so the classic lock presents EVERY host
        // frame — indistinguishable from the original's one-loop-
        // pass-per-flip cadence on the hardware it shipped for.
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};
        let mut host = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        for _ in 0..32 {
            assert_eq!(host.pump_frame(4, &InputFrame::default()), 1);
            assert!(host.should_present());
        }
        // A banked short frame (3 sub-ticks, no tick) is the ONLY
        // shape a well-formed 60 Hz script never hits — the lock
        // holds exactly as in the original: no tick, no present.
        assert_eq!(host.pump_frame(3, &InputFrame::default()), 0);
        assert!(!host.should_present());
    }

    #[test]
    fn timing_lock_pacing_never_touches_the_hashed_buckets() {
        // The Determinism Charter pin at the new consumer: the SAME
        // pump script (inputs + dt sequence) yields the IDENTICAL
        // executed-tick sequence, sim tick count, sim state hash and
        // scene hash in BOTH arms — the pacing policy differs only
        // in should_present(), which lives in the un-hashed
        // presentation bucket (D17 b). Display rate and pacing never
        // enter the sim or the state hash.
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let mut modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let mut classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        // A script with tick-carrying AND zero-tick frames (the
        // accumulator banks): dt sequence 4,1,1,1,1,3,2,2 cycled,
        // with a moving-pointer input every 5th frame.
        let script = [4u32, 1, 1, 1, 1, 3, 2, 2];
        let mut executed_modern = Vec::new();
        let mut executed_classic = Vec::new();
        let mut arms_disagree_on_present = false;
        for i in 0..16 {
            let dt = script[i % script.len()];
            let input = if i % 5 == 0 {
                InputFrame {
                    mouse_dx: 3,
                    mouse_dy: -2,
                    ..InputFrame::default()
                }
            } else {
                InputFrame::default()
            };
            let m = modern.pump_frame(dt, &input);
            let c = classic.pump_frame(dt, &input);
            executed_modern.push(m);
            executed_classic.push(c);
            if modern.should_present() != classic.should_present() {
                arms_disagree_on_present = true;
            }
        }
        assert_eq!(executed_modern, executed_classic);
        assert_eq!(
            modern.driver().sim().tick_index(),
            classic.driver().sim().tick_index()
        );
        assert_eq!(
            modern.driver().sim().state_hash(),
            classic.driver().sim().state_hash()
        );
        assert_eq!(modern.scene_hash(), classic.scene_hash());
        // And the arms DO differ on the policy surface itself (the
        // consumer is real, not inert): over this script some frame
        // exists where the arms disagree on presenting.
        assert!(
            arms_disagree_on_present,
            "the pacing arms must disagree on should_present somewhere"
        );
    }

    #[test]
    fn camera_interpolation_selects_from_the_timing_lock_arm() {
        // P6 composition policy (p6-high-refresh-interpolation): the
        // SAME timing-lock arm that selects present pacing selects
        // whether the presented frame recomposes with the
        // interpolated camera — modern interpolates, classic (the
        // original frame-locked shape, RE-EXW-CAMERA §4) never does.
        // The control-scheme axis is the axis-independence CONTROL:
        // flipping IT alone must not move this consumer.
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        assert!(modern.camera_interpolation(), "modern interpolates");

        let purist_timing = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        assert!(!purist_timing.camera_interpolation(), "classic never does");

        let classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::CLASSIC,
                ..SimConfig::default()
            },
            palette(),
        );
        assert!(
            !classic.camera_interpolation(),
            "the CLASSIC preset carries the timing arm"
        );

        // Control (the OTHER axis, modern): interpolation stays on.
        let purist_controls = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        assert!(purist_controls.camera_interpolation());
    }

    #[test]
    fn recompose_interpolates_only_on_the_decoupled_arm() {
        // The present-site consumer: recompose(alpha) re-renders the
        // presented frame from LATEST state with the camera lerped
        // toward the previous executed tick — ONLY under the modern
        // arm. Classic is a no-op (the pump's parity frame stands:
        // the frame-locked pacing presents only after a tick, the
        // exact tick-state camera of the original,
        // RE-EXW-CAMERA §4/§5).
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let moving = InputFrame {
            buttons: 1, // the placeholder payload's hash-visible move bit
            ..InputFrame::default()
        };
        // Walk the actor well past the scroll-clamp floor (9) so
        // camera endpoints differ visibly in the stub world pass.
        let ticks = 40u32;

        let mut modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        for _ in 0..ticks {
            modern.pump_frame(4, &moving);
        }
        assert!(modern.driver().sim().actor().0 >= 10);
        let parity = modern.frame().parity_hash();
        // One more executed tick stages prev (one tick back) and
        // moves the current camera one pixel.
        modern.pump_frame(4, &moving);
        let parity_after = modern.frame().parity_hash();
        assert!(modern.recompose(0.0), "modern recomposes");
        let interpolated = modern.frame().parity_hash();
        assert_ne!(
            interpolated, parity_after,
            "alpha 0 keeps the PREVIOUS tick's camera: not the parity frame"
        );
        // alpha 1 reaches the CURRENT camera: byte-identical to the
        // pump's parity frame (the lerp endpoint, D12).
        assert!(modern.recompose(1.0));
        assert_eq!(modern.frame().parity_hash(), parity_after);
        // Purity: the same alpha recomposes to the same bytes.
        assert!(modern.recompose(0.0));
        assert_eq!(modern.frame().parity_hash(), interpolated);
        // The parity frame the PUMP renders is unchanged by any of
        // the recomposes above (the next pump re-renders parity).
        assert_ne!(parity, parity_after, "the actor moved the camera");

        let mut classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        for _ in 0..ticks {
            classic.pump_frame(4, &moving);
        }
        classic.pump_frame(4, &moving);
        let classic_parity = classic.frame().parity_hash();
        for alpha in [0.0f32, 0.25, 0.5, 1.0] {
            assert!(!classic.recompose(alpha), "classic never recomposes");
        }
        assert_eq!(
            classic.frame().parity_hash(),
            classic_parity,
            "classic arm unchanged: the parity frame stands"
        );
    }

    #[test]
    fn recompose_is_inert_before_the_first_executed_tick() {
        // No endpoint staged (prev_sim None until a pump executes a
        // tick): recompose is a no-op even on the modern arm — the
        // boot frame and zero-tick pre-history present exactly the
        // parity composition.
        let mut modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let boot = modern.frame().parity_hash();
        assert!(!modern.recompose(0.5));
        assert_eq!(modern.frame().parity_hash(), boot);
        // A zero-tick pump still stages nothing.
        modern.pump_frame(0, &InputFrame::default());
        assert!(!modern.recompose(0.5));
        assert_eq!(modern.frame().parity_hash(), boot);
        // The first executed tick stages the endpoint: recompose goes
        // live (returns true; the frame may legitimately equal parity
        // when the camera did not move — the return value is the pin).
        modern.pump_frame(4, &InputFrame::default());
        assert!(modern.recompose(0.5));
    }

    #[test]
    fn camera_interpolation_never_touches_the_hashed_buckets() {
        // The Determinism Charter pin at the composition consumer:
        // the SAME pump script yields the IDENTICAL executed-tick
        // sequence, sim tick count, state hash and scene hash in both
        // arms — with the modern arm running interleaved recompose
        // calls (the accumulator fractions a high-refresh present
        // would feed) and the classic arm running none. The
        // interpolated camera lives entirely in the un-hashed
        // presentation bucket (D17 b); alpha derives from display
        // timing and can NEVER reach the sim.
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let mut modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let mut classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        // A 240 Hz-shaped script (1 sub-tick pumps, tick every 4th)
        // with movement input and present-site recomposes between
        // the pumps — the shape the window loop drives.
        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let alphas = [0.0f32, 0.25, 0.5, 0.75, 1.0];
        let mut executed_modern = Vec::new();
        let mut executed_classic = Vec::new();
        for (i, dt) in script.iter().copied().cycle().take(40).enumerate() {
            let input = if i % 3 == 0 {
                InputFrame {
                    buttons: 1,
                    mouse_dx: 2,
                    mouse_dy: 1,
                    ..InputFrame::default()
                }
            } else {
                InputFrame::default()
            };
            executed_modern.push(modern.pump_frame(dt, &input));
            executed_classic.push(classic.pump_frame(dt, &input));
            // Present site: modern recomposes at the accumulator
            // fraction, classic declines.
            let recomposed = modern.recompose(alphas[i % alphas.len()]);
            assert!(!classic.recompose(alphas[i % alphas.len()]));
            if i == 0 {
                // First pump executes a tick immediately (dt 4), so
                // the endpoint is staged from frame 0 on.
                assert!(recomposed);
            }
        }
        assert_eq!(executed_modern, executed_classic);
        assert_eq!(
            modern.driver().sim().tick_index(),
            classic.driver().sim().tick_index()
        );
        assert_eq!(
            modern.driver().sim().state_hash(),
            classic.driver().sim().state_hash()
        );
        assert_eq!(modern.scene_hash(), classic.scene_hash());
    }

    #[test]
    fn control_scheme_mapping_never_touches_the_hashed_buckets() {
        // The D201 seam-inertness property GENERALIZED to the second
        // axis (D204): the control-scheme arm selects the INPUT
        // MAPPING POLICY at the platform/input seam (which physical
        // key maps to which game-semantic button - shell-side), so
        // the mapping lives ENTIRELY upstream of the InputFrame. The
        // host-level consequence: the SAME InputFrame script yields
        // the identical executed-tick sequence, sim tick count, sim
        // state hash and scene hash in BOTH arms - the sim never sees
        // the scheme. The script deliberately holds `buttons` bit 0
        // (the placeholder payload's movement bit, hash-visible
        // today): if the scheme ever leaked past the frame contract
        // this pin fails loud. (Where the arms DO differ - the mapped
        // frames a physical stream produces - is pinned at the shell
        // seam, the consumer's own test surface.)
        use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};

        let mut modern = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let mut classic = GameHost::new(
            &GameConfig::default(),
            &SimConfig {
                mode: ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic),
                ..SimConfig::default()
            },
            palette(),
        );
        // A movement-and-pointing script: bit 0 held every 2nd frame,
        // pointer deltas every 3rd, a click every 5th - the frames a
        // mixed modern-mapped WASD+mouse stream produces.
        let script = [4u32, 4, 4, 1, 3];
        let mut executed_modern = Vec::new();
        let mut executed_classic = Vec::new();
        for i in 0..15 {
            let dt = script[i % script.len()];
            let input = InputFrame {
                buttons: if i % 2 == 0 { 1 } else { 0 },
                mouse_dx: if i % 3 == 0 { 3 } else { 0 },
                mouse_dy: if i % 3 == 0 { -2 } else { 0 },
                mouse_buttons: if i % 5 == 0 { 1 } else { 0 },
            };
            executed_modern.push(modern.pump_frame(dt, &input));
            executed_classic.push(classic.pump_frame(dt, &input));
        }
        assert_eq!(executed_modern, executed_classic);
        assert_eq!(
            modern.driver().sim().tick_index(),
            classic.driver().sim().tick_index()
        );
        assert_eq!(
            modern.driver().sim().state_hash(),
            classic.driver().sim().state_hash()
        );
        assert_eq!(modern.scene_hash(), classic.scene_hash());
    }

    #[test]
    fn click_input_walks_the_scenes() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        let click = InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        };
        let idle = InputFrame::default();
        // Press (edge) walks Title -> Brief; hold does not re-fire
        // across the boundary (P-latch clear analog).
        host.pump_frame(4, &click);
        assert_eq!(host.scene(), Scene::Brief);
        host.pump_frame(4, &click);
        assert_eq!(host.scene(), Scene::Brief);
        host.pump_frame(4, &idle);
        host.pump_frame(4, &click);
        assert_eq!(host.scene(), Scene::Select);
        // The frame is present after every pump.
        assert_eq!(host.frame().indices.len(), bedlam_render::INDICES_LEN);
    }

    #[test]
    fn music_attaches_per_scene_and_audibly_renders() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        // Give instrument 8 (variant 1 + 7) a wave to play.
        host.mixer_mut()
            .load_wave(8, &[128u8, 200, 100, 128])
            .unwrap();
        host.load_music(&synth_mrs_bytes()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        // Title has no track: the mix must stay silent.
        let mut buf = [0i16; 8820];
        host.render_audio(&mut buf).unwrap();
        assert!(buf.iter().all(|&s| s == 0), "no track on Title");
        // Options attaches the script; the note at tick 3 (30 ms) fires.
        host.apply(SceneAction::Options);
        assert_eq!(host.scene(), Scene::Options);
        host.render_audio(&mut buf).unwrap();
        assert!(buf.iter().any(|&s| s != 0), "Options track must be audible");
        // And detaches again on a trackless scene.
        host.apply(SceneAction::Back);
        assert_eq!(host.scene(), Scene::Title);
        let mut quiet = [0i16; 8820];
        host.render_audio(&mut quiet).unwrap();
        assert!(quiet.iter().all(|&s| s == 0), "silence after detach");
    }

    /// Synthetic 2-frame 4x4 SMK, 40 ms/frame, raw-PCM mono 8-bit
    /// 11025 Hz track (byte-compatible with the movie.rs and
    /// bedlam-assets smk.rs fixtures).
    fn synth_smk() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"SMK2");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&2u32.to_le_bytes());
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        for i in 0..7u32 {
            d.extend_from_slice(&(if i == 0 { 16u32 } else { 0u32 }).to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes());
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes());
        }
        for i in 0..7u32 {
            d.extend_from_slice(
                &(if i == 0 {
                    0x4000_0000u32 | 11_025
                } else {
                    0u32
                })
                .to_le_bytes(),
            );
        }
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&17u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.push(0x03);
        d.push(0x02);
        d.push(0x00);
        d.extend_from_slice(&[0x02, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0x00, 0x00]);
        d.extend_from_slice(&[0x06, 0x00, 0x00, 0x00, 0xAA, 0x55]);
        d.extend_from_slice(&[0x00, 0x00]);
        d.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x00]);
        d
    }

    #[test]
    fn movie_is_inert_until_its_scene_and_never_touches_the_hash() {
        // Same pump sequence with and without a loaded movie: the scene
        // hash chain must be IDENTICAL (D17 bucket b - the movie is
        // presentation).
        let walk = |with_movie: bool| -> Vec<u64> {
            let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
            if with_movie {
                host.load_movie(Scene::Title, &synth_smk()).unwrap();
            }
            let mut chain = Vec::new();
            let click = InputFrame {
                mouse_buttons: 1,
                ..InputFrame::default()
            };
            for _ in 0..(BOOT_TICKS + 6) {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            host.pump_frame(4, &click); // Title -> Brief
            for _ in 0..4 {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            chain
        };
        assert_eq!(walk(false), walk(true));
    }

    #[test]
    fn title_movie_composites_palette_and_queues_audio() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_movie(Scene::Title, &synth_smk()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        // Title entered: movie started, frame 0 held.
        assert!(host.movie().is_some());
        assert_eq!(host.movie().unwrap().frame_index(), 0);
        // The composite frame carries the folded movie palette and the
        // dirty flag (frame 0 palette entry = 6-bit [1,2,3]).
        assert_eq!(host.frame().palette[0], [1, 2, 3]);
        assert!(host.frame().palette_dirty);
        // Frame-0 audio [AA 55] queued on the stream bus at the config
        // master gain (default volume 100 -> master 50 -> Q8 gain 100:
        // ((b - 128) << 8) * 100 >> 8).
        let expect = |b: u8| (((i32::from(b) - 128) << 8) * 100) >> 8;
        let mut buf = [0i16; 4];
        host.render_audio(&mut buf).unwrap();
        assert_eq!(
            buf,
            [
                expect(0xAA) as i16,
                expect(0xAA) as i16,
                expect(0x55) as i16,
                expect(0x55) as i16
            ]
        );
        // Advance past the 40 ms period (10 sub-ticks = 3 pumps of 4
        // sub-ticks): frame 1 decodes, its audio [11 22 33] queues,
        // the stream finishes and HOLDS the last frame.
        for _ in 0..3 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert!(host.movie().unwrap().finished());
        assert_eq!(host.movie().unwrap().frame_index(), 1);
        let mut buf = [0i16; 6];
        host.render_audio(&mut buf).unwrap();
        assert_eq!(buf[0], expect(0x11) as i16);
        assert_eq!(buf[2], expect(0x22) as i16);
        assert_eq!(buf[4], expect(0x33) as i16);
    }

    #[test]
    fn leaving_the_movie_scene_stops_playback_and_silences() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_movie(Scene::Title, &synth_smk()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        let click = InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        };
        host.pump_frame(4, &click); // Title -> Brief
        assert!(host.movie().is_none(), "slot dropped on scene exit");
        let mut buf = [1i16; 8];
        host.render_audio(&mut buf).unwrap();
        assert!(buf.iter().all(|&s| s == 0), "stream bus cleared");
    }

    #[test]
    fn sim_intents_route_through_apply() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        assert_eq!(host.scene(), Scene::Mission);
        host.apply(SceneAction::MissionComplete);
        assert_eq!(host.scene(), Scene::Debrief);
        assert_eq!(host.fsm().episode().linear(), 1);
    }

    #[test]
    fn cutscene_selection_follows_the_episode_stage() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        // Boot: stage 1, no completions - the zone-complete name.
        assert_eq!(host.cutscene_name(), "ZONEDONE.SMK");
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        // Title -> Brief -> Select -> Mission.
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        host.apply(SceneAction::Advance);
        assert_eq!(host.scene(), Scene::Mission);
        // Zone cadence per FULL_MASK: slot 1 has one sub, slots 2..=7
        // have four (1 + 4*6 = 25 completions routed through Debrief).
        for (zone, &subs) in [1u32, 4, 4, 4, 4, 4, 4].iter().enumerate() {
            for sub in 0..subs {
                assert_eq!(host.scene(), Scene::Mission);
                host.apply(SceneAction::MissionComplete); // -> Debrief
                host.apply(SceneAction::Advance); // -> Cutscene | Shop
                let zone_done = sub + 1 == subs;
                assert_eq!(
                    host.scene(),
                    if zone_done {
                        Scene::Cutscene
                    } else {
                        Scene::Shop
                    }
                );
                if zone_done {
                    // Read WHILE on Cutscene: the name the host would
                    // play for this zone-complete scene. Only the 7th
                    // zone (the endgame) selects END.
                    assert_eq!(
                        host.cutscene_name(),
                        if zone + 1 == 7 {
                            "END.SMK"
                        } else {
                            "ZONEDONE.SMK"
                        }
                    );
                }
                host.apply(SceneAction::Advance); // -> Select
                host.apply(SceneAction::Advance); // Select -> Mission
            }
        }
        // The 7th zone completion left the stage at MAX_STAGE: the
        // endgame ceiling holds the END selection from then on.
        assert_eq!(host.fsm().episode().stage(), MAX_STAGE);
        assert_eq!(host.cutscene_name(), "END.SMK");
    }

    #[test]
    fn load_cutscene_binds_the_cutscene_scene() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_cutscene(&synth_smk()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance); // Title -> Brief
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
        assert_eq!(host.scene(), Scene::Mission);
        // Slot 1 has one sub: the FIRST completion zones out, so the
        // Debrief advance lands on Cutscene.
        host.apply(SceneAction::MissionComplete);
        host.apply(SceneAction::Advance);
        assert_eq!(host.scene(), Scene::Cutscene);
        // Inert until the next pump starts it (D31 lifecycle).
        assert_eq!(host.movie().unwrap().frame_index(), 0);
        host.pump_frame(4, &InputFrame::default());
        assert!(host.movie().is_some(), "started on Cutscene entry");
        // EXW's frames-1 bound displays only frame0 of this two-frame
        // fixture, holds it for its period, and never consumes frame1.
        for _ in 0..3 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(host.movie().unwrap().frame_index(), 0);
        assert!(host.movie().unwrap().finished());
        // Leaving Cutscene drops the slot and clears the stream.
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Select);
        assert!(host.movie().is_none(), "slot dropped on Cutscene exit");
    }

    #[test]
    fn briefing_selection_walks_the_campaign() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        // Boot camp (stage 1): the brief carries no lettered backdrop
        // (no BRF_A in the corpus).
        assert_eq!(host.briefing_name(), None);
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance); // Title -> Brief
        assert_eq!(host.scene(), Scene::Brief);
        assert_eq!(host.briefing_name(), None, "boot-camp brief");
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
                                          // Boot-camp mission (stage 1, its only sub): no backdrop
                                          // either; completing it zones out to stage 2.
        assert_eq!(host.scene(), Scene::Mission);
        assert_eq!(host.briefing_name(), None, "boot-camp mission");
        host.apply(SceneAction::MissionComplete); // -> Debrief (stage -> 2)
        host.apply(SceneAction::Advance); // zone complete -> Cutscene
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
                                          // Lettered zone stages 2..=6 = B..=F, four subs each per
                                          // FULL_MASK; the slot names the mission it is ABOUT to play:
                                          // sub = lowest-unset mask bit + 1 (the complete() arithmetic).
        for (zone, letter) in ['B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            for sub in 1..=4u8 {
                assert_eq!(host.scene(), Scene::Mission);
                let stage = zone as u8 + 2;
                assert_eq!(
                    host.briefing_name().as_deref(),
                    Some(format!("BRF_{letter}{sub}.SMK").as_str()),
                    "stage {stage} sub {sub}"
                );
                host.apply(SceneAction::MissionComplete); // -> Debrief
                host.apply(SceneAction::Advance); // -> Shop (mid-zone)
                host.apply(SceneAction::Advance); // -> Select
                host.apply(SceneAction::Advance); // -> Mission
            }
        }
        // The F-zone completion advanced to stage 7 = the endgame
        // zone (EXW zone 7): no lettered backdrop there, nor at the
        // MAX_STAGE ceiling its completion reaches.
        assert_eq!(host.fsm().episode().stage(), 7);
        for _ in 0..4 {
            assert_eq!(host.briefing_name(), None, "endgame zone");
            host.apply(SceneAction::MissionComplete); // -> Debrief
            host.apply(SceneAction::Advance); // -> Shop | Cutscene
            host.apply(SceneAction::Advance); // -> Select
            host.apply(SceneAction::Advance); // -> Mission
        }
        assert_eq!(host.fsm().episode().stage(), MAX_STAGE);
        assert_eq!(host.briefing_name(), None, "post-endgame ceiling");
    }

    #[test]
    fn brief_intro_lifecycle_on_the_brief_scene() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_briefing(&synth_smk(), &synth_smk()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        // Still on Title: inert (D31 lifecycle) - the flow stays
        // Staged and is NOT the plain movie slot.
        assert_eq!(host.brief_phase(), Some(crate::BriefPhase::Staged));
        assert!(host.movie().is_none());
        host.apply(SceneAction::Advance); // Title -> Brief
        host.pump_frame(4, &InputFrame::default());
        // Entry starts the drop pass and queues its frame-0 audio
        // (the synth drop carries a track; the corpus pair is
        // silent).
        assert_eq!(host.brief_phase(), Some(crate::BriefPhase::Drop));
        let expect = |b: u8| (((i32::from(b) - 128) << 8) * 100) >> 8;
        let mut buf = [0i16; 4];
        host.render_audio(&mut buf).unwrap();
        assert_eq!(
            buf,
            [
                expect(0xAA) as i16,
                expect(0xAA) as i16,
                expect(0x55) as i16,
                expect(0x55) as i16
            ]
        );
        // 2-frame synth drop: one EXW pass = 1 period = 9.6
        // sub-ticks. The entry pump banked 4; the second reaches 8
        // (still Drop); the third crosses 9.6 - the handoff fires.
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.brief_phase(), Some(crate::BriefPhase::Drop));
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.brief_phase(), Some(crate::BriefPhase::Backdrop));
        assert_eq!(host.brief_intro().unwrap().frame_index(), 0);
        // Leaving Brief (-> Select) drops the flow and clears the
        // stream bus.
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Select);
        assert_eq!(host.brief_phase(), None);
        let mut quiet = [1i16; 8];
        host.render_audio(&mut quiet).unwrap();
        assert!(quiet.iter().all(|&s| s == 0), "stream bus cleared");
    }

    #[test]
    fn brief_intro_never_touches_the_scene_hash() {
        // Same pump sequence with and without the staged pair: the
        // scene-hash chain must be IDENTICAL (D17 bucket b - the
        // briefing pair is presentation, like the movie).
        let walk = |with_pair: bool| -> Vec<u64> {
            let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
            if with_pair {
                host.load_briefing(&synth_smk(), &synth_smk()).unwrap();
            }
            let mut chain = Vec::new();
            while host.scene() == Scene::Boot {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            host.apply(SceneAction::Advance); // Title -> Brief
            for _ in 0..8 {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            host.apply(SceneAction::Advance); // Brief -> Select
            for _ in 0..4 {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            chain
        };
        assert_eq!(walk(false), walk(true));
    }

    #[test]
    fn load_shop_binds_the_shop_scene() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_shop(&synth_smk()).unwrap();
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance); // Title -> Brief
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
                                          // This tests the legacy shop movie slot, independently of game over.
        host.fsm.enter(Scene::Shop);
        assert_eq!(host.scene(), Scene::Shop);
        // Inert until the next pump starts it (D31), then it plays
        // like the Title movie (the corpus SHOP ring wraps).
        assert_eq!(host.movie().unwrap().frame_index(), 0);
        host.pump_frame(4, &InputFrame::default());
        assert!(host.movie().is_some(), "started on Shop entry");
        for _ in 0..3 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert!(host.movie().unwrap().finished());
        // Shop -> Select drops the slot and clears the stream.
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Select);
        assert!(host.movie().is_none(), "slot dropped on Shop exit");
    }

    #[test]
    fn boot_attract_lifecycle_on_the_boot_scene() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_boot_attract(&synth_smk(), &synth_smk()).unwrap();
        // Staged until the first pump: the host is CONSTRUCTED on the
        // Boot scene, so the very first pump starts it (the D31 entry
        // semantics applied at scene standstill).
        assert_eq!(host.boot_attract_phase(), Some(crate::BootPhase::Staged));
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.boot_attract_phase(), Some(crate::BootPhase::Playing));
        assert_eq!(host.boot_attract().unwrap().movie_index(), 0);
        // The composite frame carries the folded movie palette and the
        // dirty flag (frame-0 palette entry = 6-bit [1,2,3]).
        assert_eq!(host.frame().palette[0], [1, 2, 3]);
        assert!(host.frame().palette_dirty);
        // GTLOG frame-0 audio [AA 55] queued on the stream bus at the
        // config master gain (the D31 title-movie shape).
        let expect = |b: u8| (((i32::from(b) - 128) << 8) * 100) >> 8;
        let mut buf = [0i16; 4];
        host.render_audio(&mut buf).unwrap();
        assert_eq!(
            buf,
            [
                expect(0xAA) as i16,
                expect(0xAA) as i16,
                expect(0x55) as i16,
                expect(0x55) as i16
            ]
        );
        // The 2-frame synth pair (one EXW pass = 1 period = 40 ms per
        // movie) completes inside the 200 ms Boot hold: GTLOG pass
        // ends at pump 5 (20 sub-ticks >= 19.2), LOGO at pump 10.
        for _ in 0..10 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(host.boot_attract_phase(), Some(crate::BootPhase::Done));
        assert_eq!(host.boot_attract().unwrap().movie_index(), 1);
        // Boot -> Title drops the flow and clears the stream.
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(host.scene(), Scene::Title);
        assert_eq!(host.boot_attract_phase(), None);
        let mut quiet = [1i16; 8];
        host.render_audio(&mut quiet).unwrap();
        assert!(quiet.iter().all(|&s| s == 0), "stream bus cleared");
    }

    #[test]
    fn boot_attract_never_touches_the_scene_hash() {
        // Same pump sequence with and without the staged attract: the
        // scene-hash chain must be IDENTICAL (D17 bucket b - the boot
        // attract is presentation, like the movie and the loading
        // flow).
        let walk = |with_attract: bool| -> Vec<u64> {
            let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
            if with_attract {
                host.load_boot_attract(&synth_smk(), &synth_smk()).unwrap();
            }
            let mut chain = Vec::new();
            for _ in 0..(BOOT_TICKS + 4) {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            chain
        };
        assert_eq!(walk(false), walk(true));
    }
    /// Marker host palette: every entry distinct, 6-bit clean.
    fn marker_palette() -> [Vga6; 256] {
        let mut p = [[0u8; 3]; 256];
        for (i, c) in p.iter_mut().enumerate() {
            *c = [(i as u8) & 0x3f, 7, 9];
        }
        p
    }

    /// Full-canon 640x480 single-image raw BIN (fill byte = identity).
    fn full_still_bin(fill: u8) -> Vec<u8> {
        let mut img = Vec::new();
        img.extend_from_slice(&0u16.to_le_bytes());
        img.extend_from_slice(&640i16.to_le_bytes());
        img.extend_from_slice(&480i16.to_le_bytes());
        img.extend(std::iter::repeat_n(fill, 640 * 480));
        let mut v = 1u16.to_le_bytes().to_vec();
        v.extend_from_slice(&4u32.to_le_bytes());
        v.extend_from_slice(&img);
        v
    }

    /// 770-byte palette file: entry 0 = [12,34,56], entry 223 =
    /// [21,22,23], tail entries 224..=255 = [1,2,3] (to be overridden).
    fn load_pal() -> Vec<u8> {
        let mut d = vec![0u8; 770];
        for (i, c) in [(0usize, [12u8, 34, 56]), (223, [21, 22, 23])] {
            d[2 + i * 3..2 + i * 3 + 3].copy_from_slice(&c);
        }
        for i in 224..256usize {
            d[2 + i * 3..2 + i * 3 + 3].copy_from_slice(&[1, 2, 3]);
        }
        d
    }

    /// Walk the FSM to the FIRST zone-complete Cutscene (stage 2):
    /// boot out, three advances to Mission, the one-sub boot-camp
    /// completion zones out to the cutscene.
    fn walk_to_first_cutscene(host: &mut GameHost) {
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance); // Title -> Brief
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
        host.apply(SceneAction::MissionComplete); // -> Debrief (stage 2)
        host.apply(SceneAction::Advance); // -> Cutscene
    }

    #[test]
    fn loading_flow_lifecycle_through_the_zone_transition() {
        let mut host = GameHost::new(
            &GameConfig::default(),
            &SimConfig::default(),
            marker_palette(),
        );
        host.load_cutscene(&synth_smk()).unwrap();
        host.load_interlude(&full_still_bin(0xB7)).unwrap();
        host.load_loading_screen(&full_still_bin(0x10), &load_pal())
            .unwrap();
        // Inert from construction through the first completion.
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Staged));
        walk_to_first_cutscene(&mut host);
        assert_eq!(host.scene(), Scene::Cutscene);
        // Cutscene entered: the movie starts first, the flow holds.
        host.pump_frame(4, &InputFrame::default());
        assert!(host.movie().is_some());
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Staged));
        // The 2-frame synth movie finishes within 3 more pumps; the
        // interlude then owns the Cutscene plane: full-canon BETWEEN
        // raster (1:1, no letterbox) under the standing host palette.
        for _ in 0..3 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert!(host.movie().unwrap().finished());
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Between));
        assert!(host.frame().indices.iter().all(|&i| i == 0xB7));
        assert_eq!(host.frame().palette, marker_palette());
        // Cutscene -> Select: the loading screen takes over at fade
        // step 0 - LOAD raster present, palette still all black.
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Select);
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Loading));
        assert_eq!(host.loading_fade_step(), Some(0));
        assert!(host.frame().indices.iter().all(|&i| i == 0x10));
        assert_eq!(host.frame().palette, [[0u8; 3]; 256]);
        // Zone 1 completed (stage 2): the 3-row text pass (D35: the
        // draws ran at Loading entry; no font staged here - pristine).
        assert_eq!(
            host.loading_text_row().map(|row| row.rows),
            Some(&[150, 180, 210][..])
        );
        // 11 more pumps = 48 sub-ticks total = 200 ms = the 10-step
        // 50 Hz fade complete: folded palette. No staged ramp: the
        // tail keeps the folded file values ([1,2,3] round-trips).
        for _ in 0..11 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(host.loading_fade_step(), Some(10));
        let pal = host.frame().palette;
        assert_eq!(pal[0], [12, 34, 56], "file entry 0 folded back");
        assert_eq!(pal[223], [21, 22, 23], "pre-tail entry preserved");
        assert_eq!(pal[224], [1, 2, 3], "no ramp: folded file tail stands");
        assert_eq!(pal[255], [1, 2, 3], "no ramp: folded file tail stands");
        // Leaving Select drops the flow entirely.
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Mission);
        assert_eq!(host.loading_phase(), None);
        assert_eq!(host.loading_fade_step(), None);
    }

    #[test]
    fn loading_font_pass_draws_the_rows_and_tints_the_tail() {
        let mut host = GameHost::new(
            &GameConfig::default(),
            &SimConfig::default(),
            marker_palette(),
        );
        host.load_cutscene(&synth_smk()).unwrap();
        host.load_interlude(&full_still_bin(0xB7)).unwrap();
        host.load_loading_screen(&full_still_bin(0x10), &load_pal())
            .unwrap();
        // The D35 staging seam: FULLFONT bank + FULLPAL ramp + the
        // LANGUAGE table, inert until the flow enters Loading.
        host.load_loading_font(
            &crate::font::synth::font_bin(),
            &crate::font::synth::fullpal_bin(),
            &crate::font::synth::language_bin(b"Congrats!"),
        )
        .unwrap();
        walk_to_first_cutscene(&mut host);
        for _ in 0..4 {
            host.pump_frame(4, &InputFrame::default());
        } // movie done -> Between
        host.apply(SceneAction::Advance); // -> Select
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Loading));
        assert_eq!(host.loading_fade_step(), Some(0));
        // Zone 1 (stage 2): three rows drew through the staged font -
        // entry 0x45 "Congrats!" carries the bang glyph (fill 0xF0) at
        // row 150; 0x46 "Now move out to" and 0x52 "The Airport" carry
        // lowercase glyphs (fill 0xF2) at rows 180/210.
        let frame = host.frame();
        for (row, fill) in [(150usize, 0xF0u8), (180, 0xF2), (210, 0xF2)] {
            let band = &frame.indices[row * 640..(row + 2) * 640];
            assert!(band.contains(&fill), "row {row}: glyph fill {fill:#x} drew");
        }
        // Well above the first row: the untouched still fill.
        assert!(
            frame.indices[100 * 640..110 * 640]
                .iter()
                .all(|&v| v == 0x10),
            "above the text rows: pristine still"
        );
        // A bad font bank is a staging error (typed, no state change).
        assert!(host
            .load_loading_font(
                &[1u8, 0],
                &crate::font::synth::fullpal_bin(),
                &crate::font::synth::language_bin(b"x")
            )
            .is_err());
        // 11 more pumps = fade complete: the tail carries the staged
        // ramp (entry 224 black, 233 white, 255 = (31*7)&0x3f), the
        // pre-tail file values stand.
        for _ in 0..11 {
            host.pump_frame(4, &InputFrame::default());
        }
        let pal = host.frame().palette;
        assert_eq!(pal[224], [0, 0, 0], "ramp entry 0 (black)");
        assert_eq!(pal[233], [63, 63, 63], "ramp entry 9 (white)");
        assert_eq!(pal[255], [25, 25, 25], "ramp end (31*7)&0x3f");
        assert_eq!(pal[0], [12, 34, 56], "pre-tail file value stands");
    }

    #[test]
    fn loading_flow_never_touches_the_scene_hash() {
        // Same pump sequence with and without the staged flow: the
        // scene hash chain must be IDENTICAL (D17 bucket b).
        let walk = |with_flow: bool| -> Vec<u64> {
            let mut host = GameHost::new(
                &GameConfig::default(),
                &SimConfig::default(),
                marker_palette(),
            );
            if with_flow {
                host.load_interlude(&full_still_bin(0xB7)).unwrap();
                host.load_loading_screen(&full_still_bin(0x10), &load_pal())
                    .unwrap();
            }
            let mut chain = Vec::new();
            walk_to_first_cutscene(&mut host);
            for _ in 0..4 {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            host.apply(SceneAction::Advance); // Cutscene -> Select
            for _ in 0..12 {
                host.pump_frame(4, &InputFrame::default());
                chain.push(host.scene_hash().0);
            }
            host.apply(SceneAction::Advance); // Select -> Mission
            host.pump_frame(4, &InputFrame::default());
            chain.push(host.scene_hash().0);
            chain
        };
        assert_eq!(walk(false), walk(true));
    }

    #[test]
    fn endgame_arm_drops_the_staged_flow() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        while host.scene() == Scene::Boot {
            host.pump_frame(4, &InputFrame::default());
        }
        host.apply(SceneAction::Advance); // Title -> Brief
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
                                          // Slot cadence per FULL_MASK: zone 1 has one sub, zones 2..=7
                                          // four each. The flow is staged with each zone completion
                                          // (the caller pattern: stage it when the cutscene is staged).
        for (zone, &subs) in [1u32, 4, 4, 4, 4, 4, 4].iter().enumerate() {
            for sub in 0..subs {
                assert_eq!(host.scene(), Scene::Mission);
                host.load_interlude(&full_still_bin(0xB7)).unwrap();
                host.load_loading_screen(&full_still_bin(0x10), &load_pal())
                    .unwrap();
                host.load_loading_font(
                    &crate::font::synth::font_bin(),
                    &crate::font::synth::fullpal_bin(),
                    &crate::font::synth::language_bin(b"Congrats!"),
                )
                .unwrap();
                host.apply(SceneAction::MissionComplete); // -> Debrief
                host.apply(SceneAction::Advance); // -> Cutscene | Shop
                if sub + 1 < subs {
                    assert_eq!(host.scene(), Scene::Shop);
                    // Mid-zone Select never saw the cutscene: the
                    // staged flow stays inert (EXW has no tail there).
                    host.apply(SceneAction::Advance); // -> Select
                    host.pump_frame(4, &InputFrame::default());
                    assert_eq!(
                        host.loading_phase(),
                        Some(crate::LoadingPhase::Staged),
                        "mid-zone Select: inert"
                    );
                    host.apply(SceneAction::Advance); // -> Mission
                }
            }
            assert_eq!(host.scene(), Scene::Cutscene);
            let stage = host.fsm().episode().stage();
            host.pump_frame(4, &InputFrame::default());
            if zone + 1 == 7 {
                // The endgame completion reached MAX_STAGE: the EXW
                // endgame arm (END.SMK + credits) loads no BETWEEN /
                // LOAD assets - the staged flow is dropped, never
                // activated.
                assert_eq!(stage, MAX_STAGE);
                assert_eq!(host.loading_phase(), None, "endgame arm drops it");
            } else {
                assert_eq!(
                    host.loading_phase(),
                    Some(crate::LoadingPhase::Between),
                    "zone-transition arm at stage {stage}"
                );
            }
            host.apply(SceneAction::Advance); // -> Select
            host.pump_frame(4, &InputFrame::default());
            if zone + 1 == 6 {
                // Completing zone 6 (into the endgame zone): the
                // zone-6 arm adds the fourth text row, and the
                // staged font drew it (0xF2 = lowercase glyph fill).
                let row = host.loading_text_row().unwrap();
                assert_eq!(row.rows, &[150, 180, 210, 260][..]);
                let band = &host.frame().indices[260 * 640..262 * 640];
                assert!(band.contains(&0xF2), "zone-6 fourth row drew");
            }
            if zone + 1 == 7 {
                assert_eq!(host.loading_phase(), None, "stays dropped");
            } else {
                assert_eq!(
                    host.loading_phase(),
                    Some(crate::LoadingPhase::Loading),
                    "Select after the cutscene"
                );
            }
            host.apply(SceneAction::Advance); // Select -> Mission
        }
    }

    #[test]
    fn skip_advance_bypasses_the_interlude_but_runs_the_loading_screen() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_cutscene(&synth_smk()).unwrap();
        host.load_interlude(&full_still_bin(0xB7)).unwrap();
        host.load_loading_screen(&full_still_bin(0x10), &load_pal())
            .unwrap();
        walk_to_first_cutscene(&mut host);
        // One pump in: the movie still plays frame 0. Advancing now
        // skips ahead - the EXW tail is unconditional after the movie
        // call returns, so the loading screen still runs; only the
        // interlude visual is bypassed.
        host.pump_frame(4, &InputFrame::default());
        assert!(!host.movie().unwrap().finished());
        host.apply(SceneAction::Advance);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Select);
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Loading));
        assert_eq!(host.loading_fade_step(), Some(0));
        assert!(host.frame().indices.iter().all(|&i| i == 0x10));
    }

    #[test]
    fn interlude_without_a_movie_shows_immediately_on_cutscene() {
        let mut host = GameHost::new(
            &GameConfig::default(),
            &SimConfig::default(),
            marker_palette(),
        );
        host.load_interlude(&full_still_bin(0xB7)).unwrap();
        walk_to_first_cutscene(&mut host);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.loading_phase(), Some(crate::LoadingPhase::Between));
        assert!(host.frame().indices.iter().all(|&i| i == 0xB7));
        assert_eq!(host.frame().palette, marker_palette());
    }

    #[test]
    fn bad_staging_assets_error_without_state_change() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        // Garbage bank / short palette: typed errors, no staging.
        assert!(host.load_interlude(&[1u8, 2, 3]).is_err());
        assert!(host
            .load_loading_screen(&full_still_bin(0x10), &[0u8; 9])
            .is_err());
        assert_eq!(host.loading_phase(), None);
        // The error display is pinned.
        assert_eq!(
            GameError::BadLoadingAsset {
                what: "image bank entry 0",
                reason: "undecoded raster"
            }
            .to_string(),
            "loading-flow asset image bank entry 0: undecoded raster"
        );
    }

    /// Walk a fresh host to Title (the boot hold), pump-neutral.
    fn walk_to_title(host: &mut GameHost) {
        while host.scene() != Scene::Title {
            host.pump_frame(4, &InputFrame::default());
        }
    }

    /// One-tick hover to item i (exact cursor delta), then a click
    /// press+release over two pumps.
    fn menu_click(host: &mut GameHost, i: i8) {
        let menu = host.menu().expect("menu staged");
        let count = menu.count() as i32;
        let top = crate::menu::STRIP_Y_MAX - count * crate::menu::ROW_H;
        let y = top + i as i32 * crate::menu::ROW_H + crate::menu::ROW_H / 2;
        let x = (crate::menu::STRIP_X_MIN + crate::menu::STRIP_X_MAX) / 2;
        let (cx, cy) = menu.cursor();
        let hover = InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            ..InputFrame::default()
        };
        host.pump_frame(4, &hover);
        let press = InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        };
        host.pump_frame(4, &press);
        host.pump_frame(4, &InputFrame::default());
    }

    #[test]
    fn squad_death_enters_game_over_then_movie_returns_to_title() {
        let f = crate::mission::synth_mission_files();
        let maptran: Vec<&[u8]> = f[14..22].iter().map(|v| v.as_slice()).collect();
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_mission(
            &f[0],
            &f[1],
            &f[2],
            &f[3],
            &f[5],
            &f[6],
            &f[7],
            &f[8],
            &f[9],
            &f[10],
            &f[11],
            &f[4],
            &f[23],
            &f[24],
            &f[12],
            &maptran,
            &f[13],
            &f[22],
            None,
            &[(3, 1, 1)],
        )
        .unwrap();
        host.fsm.enter(Scene::Mission);
        host.sync_mission();
        let count = host.mission().unwrap().sim().robots().len();
        for slot in 0..count {
            host.mission_mut().unwrap().apply_damage(slot, 6000, -1);
        }
        let episode = *host.fsm.episode();
        for _ in 0..11 {
            host.pump_frame(4, &InputFrame::default());
            assert_eq!(host.scene(), Scene::Mission);
        }
        let held = InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        };
        host.pump_frame(4, &held);
        assert_eq!(host.scene(), Scene::GameOver);
        assert!(host.mission().is_none());
        assert_eq!(*host.fsm.episode(), episode);
        host.load_movie(Scene::GameOver, &synth_smk()).unwrap();
        for _ in 0..4 {
            host.pump_frame(4, &held);
        }
        assert_eq!(host.scene(), Scene::Title);
        assert_eq!(*host.fsm.episode(), episode);
    }

    #[test]
    fn mission_lifecycle_stages_inert_activates_and_drops() {
        // DESIGN-GAME sec 11: staging touches no hashed state; the
        // mission is INERT until the FSM enters Mission (no tick, no
        // plane), activates on entry (camera at the spawn), and drops
        // when the scene is left.
        let f = crate::mission::synth_mission_files();
        let maptran: Vec<&[u8]> = f[14..22].iter().map(|v| v.as_slice()).collect();
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        host.load_mission(
            &f[0],
            &f[1],
            &f[2],
            &f[3],
            &f[5],
            &f[6],
            &f[7],
            &f[8],
            &f[9],
            &f[10],
            &f[11],
            &f[4],
            &f[23],
            &f[24],
            &f[12],
            &maptran,
            &f[13],
            &f[22],
            None,
            &[(3, 1, 1)],
        )
        .expect("synth mission stages");
        assert!(host.mission().is_some());
        // Pumps on Title: staged inert - the frame equals a NO-mission
        // host at the same tick (the mission owns no plane outside
        // Mission) and the sim never ticks.
        let mut plain = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        walk_to_title(&mut host);
        walk_to_title(&mut plain);
        for _ in 0..3 {
            let input = InputFrame::default();
            host.pump_frame(4, &input);
            plain.pump_frame(4, &input);
        }
        assert!(!host.mission().expect("staged").is_active());
        assert_eq!(
            host.frame().parity_hash(),
            plain.frame().parity_hash(),
            "a staged-but-inert mission changes no pixels"
        );
        assert_eq!(host.mission().expect("staged").sim().frame(), 0);
        // Enter Mission: activation fixes the camera at robot 0 Q5.
        host.apply(SceneAction::Advance); // Title -> Brief
        host.apply(SceneAction::Advance); // -> Select
        host.apply(SceneAction::Advance); // -> Mission
        host.pump_frame(4, &InputFrame::default());
        let mission = host.mission().expect("staged");
        assert!(mission.is_active());
        assert_eq!(mission.camera(), (16, 16));
        // The entry pump renders but does not tick; the next pump
        // does (one advance_frame per executed tick).
        assert_eq!(mission.sim().frame(), 0);
        assert_eq!(mission.render_count(), 1);
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.mission().expect("staged").sim().frame(), 1);
        // Leaving the scene drops the mission entirely.
        host.apply(SceneAction::MissionComplete); // -> Debrief
        host.pump_frame(4, &InputFrame::default());
        assert!(host.mission().is_none());
    }

    #[test]
    fn mission_slot_and_names_follow_the_episode() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        // Fresh game: stage 1 boot camp = zone A, no completions =
        // MISSION1.
        assert_eq!(host.mission_slot(), (0, 1));
        assert_eq!(host.mission_asset_names()[0], "ZONEA/MISSION1.TOT");
        assert_eq!(host.mission_asset_names()[8], "GAMEPAL.PAL");
        assert_eq!(host.mission_asset_names()[9], "GENERAL.BIN");
        assert_eq!(host.mission_asset_names()[10], "SMLFONT.BIN");
        assert_eq!(host.mission_asset_names()[12], "TABLE.BIN");
        assert_eq!(host.mission_asset_names()[13], "MAPTRAN0.TRN");
        assert_eq!(host.mission_asset_names()[20], "MAPTRAN7.TRN");
        assert_eq!(host.mission_asset_names()[21], "ZONEA/MISSIONA.MIN");
        assert_eq!(host.mission_asset_names().len(), 25);
        assert_eq!(host.mission_asset_names()[22], "NUMBERS.BIN");
        assert_eq!(host.mission_asset_names()[23], "FLAGS.BIN");
        assert_eq!(host.mission_asset_names()[24], "BLOWUP.BIN");
        walk_to_first_cutscene(&mut host); // completes zone 1
        host.apply(SceneAction::Advance); // Cutscene -> Select
                                          // Stage 2 now: zone B, still MISSION1 (mask reset).
        assert_eq!(host.mission_slot(), (1, 1));
        assert_eq!(host.mission_asset_names()[3], "ZONEB/MISSIONB.CGR");
    }

    #[test]
    fn select_mission_seam_stages_the_mp_files() {
        // §7j.73: the SELECT screen's MP write arm {zone 2..=6,
        // mission 1..=2} + the load-time +5 (0x4467df) — the ten
        // MP-only missions (the census G1 class) now stage through
        // the host seam.
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        assert!(host.stage_episode_slot(2, 0), "campaign slot first");
        assert!(host.stage_select_mission(2, 1));
        assert_eq!(host.mission_slot(), (1, 6), "zone cell 2 = ZONEB, 1+5 = 6");
        assert_eq!(host.mission_asset_names()[0], "ZONEB/MISSION6.TOT");
        assert_eq!(host.mission_asset_names()[3], "ZONEB/MISSIONB.CGR");
        assert!(host.stage_select_mission(6, 2));
        assert_eq!(host.mission_slot(), (5, 7));
        assert_eq!(host.mission_asset_names()[0], "ZONEF/MISSION7.TOT");
        // The write domain (the arm's exact rows): zones B..F,
        // missions 1..2 — nothing else, and a rejected staging
        // plants nothing.
        for (zone, mission) in [(1u8, 1u8), (7, 1), (2, 0), (2, 3), (0, 1)] {
            assert!(
                !host.stage_select_mission(zone, mission),
                "{zone}/{mission}"
            );
        }
        assert_eq!(host.mission_slot(), (5, 7), "rejections plant nothing");
        // Campaign staging clears the pair (the restore/advance
        // shells rewrite the runtime cells): the episode slot
        // selects again.
        assert!(host.stage_episode_slot(2, 0));
        assert_eq!(host.mission_slot(), (1, 1));
        assert_eq!(host.mission_asset_names()[0], "ZONEB/MISSION1.TOT");
    }

    #[test]
    fn select_mission_staging_never_touches_the_scene_hash() {
        // The staged SELECT pair is staging-only state (§7j.73):
        // which bytes the next Mission entry loads, never a sim
        // field — the D31 movie pattern. The pair itself leaves the
        // hashed scene bucket untouched, and clearing it (campaign
        // staging) hashes exactly what campaign staging alone would
        // hash.
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let mut plain = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        let before = host.scene_hash();
        assert!(host.stage_select_mission(3, 2));
        assert_eq!(host.scene_hash(), before, "the SELECT pair is unhashed");
        assert!(host.stage_episode_slot(3, 0));
        assert!(plain.stage_episode_slot(3, 0));
        assert_eq!(
            host.scene_hash(),
            plain.scene_hash(),
            "clearing the pair hashes like plain campaign staging"
        );
    }

    #[test]
    fn menu_interaction_never_touches_the_scene_hash() {
        // D42.1/D42.8: while the menu owns Title input the FSM is fed
        // neutral frames, so the scene-hash chain of a menu run under
        // CLICKING input equals a no-menu run under NEUTRAL input
        // (the no-menu run under the same clicks would advance - the
        // generic D26 path the menu replaces).
        let run = |menu: bool, neutral: bool| -> Vec<u64> {
            let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
            if menu {
                crate::menu::tests::stage_synth_menu(&mut host);
            }
            walk_to_title(&mut host);
            let mut chain = Vec::new();
            for buttons in [0u8, 1, 1, 0, 1, 0, 2, 0] {
                let input = if neutral {
                    InputFrame::default()
                } else {
                    InputFrame {
                        mouse_dx: 3,
                        mouse_dy: 2,
                        mouse_buttons: buttons,
                        ..InputFrame::default()
                    }
                };
                host.pump_frame(4, &input);
                chain.push(host.scene_hash().0);
            }
            chain
        };
        assert_eq!(run(true, false), run(false, true));
    }

    #[test]
    fn menu_start_click_hands_off_to_brief() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        crate::menu::tests::stage_synth_menu(&mut host);
        walk_to_title(&mut host);
        assert_eq!(host.scene(), Scene::Title);
        assert!(host.menu().is_some());
        // Generic clicks no longer advance Title (the menu owns the
        // path): clicking outside the strip opens the multiplayer
        // menu instead of moving the scene.
        host.pump_frame(
            4,
            &InputFrame {
                mouse_dx: 10,
                mouse_dy: 10,
                mouse_buttons: 1,
                ..InputFrame::default()
            },
        );
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.scene(), Scene::Title);
        assert_eq!(host.menu().unwrap().id(), crate::menu::MenuId::Multi);
        // Back to Main, then Start on item 0 -> Brief with the seed.
        menu_click(&mut host, 3); // Main Menu item
        assert_eq!(host.menu().unwrap().id(), crate::menu::MenuId::Main);
        menu_click(&mut host, 0);
        assert_eq!(host.scene(), Scene::Brief, "start hands off");
        assert!(host.menu().is_none(), "menu dropped on leaving Title");
        assert_eq!(
            host.menu_start_score_seen(),
            Some(4000),
            "score seed exposed for the sim tail"
        );
    }

    #[test]
    fn menu_quit_confirm_reaches_the_quit_scene() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        crate::menu::tests::stage_synth_menu(&mut host);
        walk_to_title(&mut host);
        menu_click(&mut host, 6); // Quit to Windows -> confirm menu
        assert_eq!(host.menu().unwrap().id(), crate::menu::MenuId::QuitConfirm);
        assert_eq!(host.scene(), Scene::Title);
        menu_click(&mut host, 0); // confirmed
        assert_eq!(host.scene(), Scene::Quit);
        assert!(host.menu().is_none());
    }

    #[test]
    fn menu_attract_replays_the_title_movie_and_skips() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        crate::menu::tests::stage_synth_menu(&mut host);
        walk_to_title(&mut host);
        host.load_movie(Scene::Title, &synth_smk()).unwrap();
        // First pass: 2 frames at 40 ms = 10 pumps at 60 Hz; the
        // menu is inert while it plays (no idle counting), then
        // starts counting once the plane is its.
        while !host.movie().unwrap().finished() {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(
            host.menu().unwrap().phase(),
            crate::menu::MenuPhase::Interactive
        );
        assert_eq!(
            host.menu().unwrap().idle(),
            0,
            "no idle while the pass played"
        );
        // Idle to 0x300 (768 ticks = 768 pumps at one tick each).
        let mut attract_seen = false;
        for _ in 0..crate::menu::ATTRACT_IDLE {
            host.pump_frame(4, &InputFrame::default());
            if host.menu().unwrap().phase() == crate::menu::MenuPhase::Attract {
                attract_seen = true;
                break;
            }
        }
        assert!(attract_seen, "attract fired at the threshold");
        // The attract restarted the movie from frame 0.
        let player = host.movie().unwrap();
        assert!(!player.finished(), "replay running");
        assert!(player.frame_index() <= 1, "restarted near frame 0");
        assert_eq!(
            host.menu().unwrap().phase(),
            crate::menu::MenuPhase::Attract
        );
        // Skip: any click finishes the replay and returns the menu.
        host.pump_frame(
            4,
            &InputFrame {
                mouse_buttons: 1,
                ..InputFrame::default()
            },
        );
        assert!(host.movie().unwrap().finished(), "skip finished the replay");
        assert_eq!(
            host.menu().unwrap().phase(),
            crate::menu::MenuPhase::Interactive
        );
        // The menu plane owns the screen now (strip rows non-black,
        // the movie raster is not 4x4).
        let frame = host.frame();
        let strip_rows_have_pixels =
            (300..470u32).any(|r| (0..640u32).any(|c| frame.get(c, r) != Some(0)));
        assert!(strip_rows_have_pixels, "menu visible after the skip");
    }

    #[test]
    fn menu_plane_takes_over_when_the_title_movie_ends() {
        let movie_then_menu = || {
            let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
            crate::menu::tests::stage_synth_menu(&mut host);
            walk_to_title(&mut host);
            host.load_movie(Scene::Title, &synth_smk()).unwrap();
            host
        };
        let mut host = movie_then_menu();
        // During the first pass the movie owns the plane: the frame
        // carries the folded movie palette (synth frame-0 entry 0 =
        // 6-bit [1,2,3]) instead of the host palette.
        host.pump_frame(4, &InputFrame::default());
        assert_eq!(host.frame().palette[0], [1, 2, 3], "movie palette mid-pass");
        // After the pass the menu owns it: the palette returns to
        // the host's (black canvas) with the staged FULLPAL ramp in
        // the 224..=255 tail, and the bottom-anchored strip draws.
        for _ in 0..12 {
            host.pump_frame(4, &InputFrame::default());
        }
        let frame = host.frame();
        assert_eq!(frame.palette[0], [0, 0, 0], "host palette under the menu");
        assert!(
            frame.palette[224..].iter().any(|c| c != &[0, 0, 0]),
            "ramp tail"
        );
        let above = (0..200u32).all(|r| (0..640u32).all(|c| frame.get(c, r) == Some(0)));
        assert!(above, "canvas black above the strip");
        let strip = (302..470u32).any(|r| (0..640u32).any(|c| frame.get(c, r) != Some(0)));
        assert!(strip, "text strip drawn bottom-anchored");
    }

    #[test]
    fn menu_sfx_play_through_the_mixer() {
        let mut host = GameHost::new(&GameConfig::default(), &SimConfig::default(), palette());
        crate::menu::tests::stage_synth_menu(&mut host);
        walk_to_title(&mut host);
        // Hover to item 0: MENU1 fires; click: MENU2 fires. Both land
        // as sounding voices on instruments 0xE0/0xE1.
        menu_click(&mut host, 0);
        assert!(
            host.mixer_mut().voice_playing(crate::menu::SFX_HOVER, 0)
                || host.mixer_mut().voice_playing(crate::menu::SFX_CLICK, 0)
        );
        // The staged waves render audible (non-silent) samples.
        let mut buf = [0i16; 256];
        let n = host.render_audio(&mut buf).unwrap();
        assert!(n > 0);
        assert!(buf[..n * 2].iter().any(|&s| s != 0), "sfx audible");
    }
}
