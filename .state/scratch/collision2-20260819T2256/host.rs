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
use bedlam_render::{blit_indexed, center_in_canonical, render, Frame, RenderInput, Vga6};

use crate::config::GameConfig;
use crate::fsm::{Scene, SceneAction, SceneFsm};
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
        for _ in 0..executed {
            self.fsm.tick(input);
        }
        self.sync_movie();
        self.sync_music();
        self.pump_movie(dt_subticks);
        self.frame = self.render_now();
        self.composite_movie();
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
                if slot.player.advance(dt_subticks).is_ok() {
                    let packets = slot.player.take_audio();
                    self.mixer.queue_pcm_u8(&packets);
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
                    self.mixer.queue_pcm_u8(&first);
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

    /// Composite the started movie over the canonical frame (D31):
    /// centered clipped indexed blit (bedlam-render), palette replaced
    /// by the movie palette (canonical 6-bit), palette_dirty forced so
    /// presentation re-uploads. Runs AFTER render_now, so the movie
    /// draws above every render pass - the fullscreen-movie topology
    /// of the original title screen.
    fn composite_movie(&mut self) {
        let Some(slot) = self.movie.as_ref() else {
            return;
        };
        if !slot.started {
            return;
        }
        let player = &slot.player;
        let info = player.info();
        let (dx, dy) = center_in_canonical(info.width, info.height);
        blit_indexed(&mut self.frame, player.pixels(), info.width, info.height, dx, dy);
        self.frame.palette = player.palette();
        self.frame.palette_dirty = true;
    }

    /// Parity render pass: canonical frame from the current sim.
    fn render_now(&mut self) -> Frame {
        let input = RenderInput {
            sim: self.driver.sim(),
            prev_sim: None, // parity config: interpolation off (D17)
            alpha: 0.0,
            palette: self.palette,
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
    use crate::fsm::BOOT_TICKS;

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
}
