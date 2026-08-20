//! GameHost - the per-frame pump wiring core + render + audio
//! (DESIGN-GAME sec 4; the FUN_0043d00b order poll -> sim -> render ->
//! present). ALL file I/O crossing this crate hides behind the injected
//! ByteSource / ByteSink traits below, so the pump itself stays
//! hermetic, replayable and testable.

use bedlam_audio::{Mixer, MusicScript};
use bedlam_core::frame::SimDriver;
use bedlam_core::hash::StateHash;
use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;
use bedlam_render::{render, Frame, MovieFrame, RenderInput, Vga6};

use crate::boot::{BootAttract, BootPhase};
use crate::brief::{BriefIntro, BriefPhase};
use crate::config::GameConfig;
use crate::fsm::{Scene, SceneAction, SceneFsm};
use crate::loading::{LoadingFlow, LoadingPhase, TextRow};
use crate::menu::{MenuAction, TitleMenu};
use crate::movie::MoviePlayer;
use crate::music::{self, MusicPump};
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
    frame: Frame,
    palette: [Vga6; 256],
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
            frame: Frame::new(palette),
            palette,
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
        let executed = self.driver.advance(dt_subticks, input);
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
            } else {
                self.fsm.tick(input);
            }
        }
        self.sync_movie();
        self.sync_boot();
        self.sync_brief();
        self.sync_loading();
        self.sync_menu();
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

    /// The scene machine (hashed bucket).
    pub fn fsm(&self) -> &SceneFsm {
        &self.fsm
    }

    /// Hashed scene-state view (D17 a + D26).
    pub fn scene_hash(&self) -> StateHash {
        self.fsm.scene_hash()
    }

    /// The latest canonical frame (the PresentCopy analog: hand this to
    /// the platform; bedlam-game never presents by itself).
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
    }

    /// Movie step (D31): advance the started player on the same dt the
    /// sim consumed, then queue whatever decoded. A decode failure
    /// mid-playback stops the movie and silences the stream -
    /// presentation self-terminates rather than propagating into the
    /// hash-bearing pump (the stream was structurally validated at
    /// load; only corrupt frame data reaches this arm).
    fn pump_movie(&mut self, dt_subticks: u32) {
        let mut failed = false;
        if let Some(slot) = self.movie.as_mut() {
            if slot.started {
                let advanced = slot.player.advance(dt_subticks);
                if advanced.is_ok() {
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

    /// Parity render pass: canonical frame from the current sim. A
    /// started movie REPLACES the scene pipeline (D31): the plane is
    /// the decoded raster + the folded 6-bit movie palette, and render
    /// emits the centered letterboxed blit - one compositing path,
    /// inside bedlam-render. An active loading-flow plane (D34) takes
    /// priority: BETWEEN / the fading loading screen own the screen the
    /// same way, and a full-screen 640x480 still centers at the origin,
    /// i.e. the 1:1 no-letterbox blit the loading gate pins.
    fn render_now(&mut self) -> Frame {
        // Menu plane (D41/D42): the staged menu owns the Title plane
        // whenever no Title movie is playing (the first pass and the
        // attract replay own it instead - the menu is inert behind
        // them and redraws when they end).
        let menu_frame = if self.title_movie_playing() || self.fsm.scene() != Scene::Title {
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
            prev_sim: None, // parity config: interpolation off (D17)
            alpha: 0.0,
            palette: self.palette,
            movie,
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
        // Inert until the next pump starts it (D31 lifecycle), then it
        // plays like the Title movie.
        assert_eq!(host.movie().unwrap().frame_index(), 0);
        host.pump_frame(4, &InputFrame::default());
        assert!(host.movie().is_some(), "started on Cutscene entry");
        // 40 ms period: 3 more pumps decode frame 1 and finish the
        // 2-frame synth stream (non-ring hold).
        for _ in 0..3 {
            host.pump_frame(4, &InputFrame::default());
        }
        assert_eq!(host.movie().unwrap().frame_index(), 1);
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
                                          // A mission FAIL routes through Debrief to Shop (the fail
                                          // path never pends a zone completion).
        host.apply(SceneAction::MissionFail);
        assert_eq!(host.scene(), Scene::Debrief);
        host.apply(SceneAction::Advance);
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
