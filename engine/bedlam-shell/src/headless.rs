//! The headless boot smoke (P4 step 1) + the corpus byte source.
//!
//! This is the default shell path: it boots the SAME wired chain as
//! the window host (boot attract -> title -> brief -> mission ->
//! cutscene loading flow, the D31-D37 sites) but drives it with a
//! FIXED pump count and neutral input - no window, no GPU, no wall
//! clock anywhere. That is the Determinism Charter shape of a smoke:
//! the display decides nothing here either; a run is a pure function
//! of (options, corpus bytes), which the two-run byte-identity gate
//! pins.
//!
//! [`GameGfxSource`] is the ONLY filesystem reader in the engine
//! crates (tools/inspect excepted): everything else stays hermetic
//! behind [`bedlam_game::ByteSource`]. It also records the fetch
//! order + sizes so the report can pin exactly which corpus files
//! the wired chain touched.
//!
//! Audio is smoke-driven the same way (D40): each pump mixes
//! [`PUMP_FRAMES`] frames from the host audio bus (the SAME bus the
//! window device path consumes) into a discard sink, counting frames
//! and non-silent samples. The mix is un-hashed (D17 bucket b) so
//! this changes no parity value - it only proves the D31 stream bus
//! and the entry-audio sites actually produce PCM on the walk.

use std::collections::VecDeque;
const SEP1: char = '/';
const SEP2: char = '\\';
const PARENT: &str = "..";
use std::path::{Path, PathBuf};

use bedlam_core::input::InputFrame;
use bedlam_game::{ByteSource, GameConfig, GameError, GameHost, Scene, SceneAction};

use crate::audio::PUMP_FRAMES;
use crate::chain::{stage_boot, stage_scene, ChainConfig};
use crate::clock::SUBTICKS_PER_PUMP;

/// fs-backed [`ByteSource`] over one install tree (the shipped
/// BEDLAM directory): a name resolves `GAMEGFX/<name>` first, then
/// `SOUND/SFX/<name>`, then `<root>/<name>` - the graphics corpus
/// lives in GAMEGFX, the menu SFX pair in SOUND/SFX (the EXW
/// "SOUND\SFX\MENU1.RAW" path, D42.7), and the LANGUAGE.* files sit
/// at the install root (EXW reads them from its working directory;
/// the tiered lookup keeps both the install root and the bare
/// GAMEGFX dir usable as roots). Bare file names only - separators
/// / parent hops are rejected, the host only ever emits corpus
/// names and the source refuses to become a generic file reader.
#[derive(Debug)]
pub struct GameGfxSource {
    root: PathBuf,
    fetched: Vec<(String, usize)>,
}

impl GameGfxSource {
    /// A source rooted at `root` (no existence check - the first
    /// fetch reports missing assets as errors).
    pub fn new(root: impl Into<PathBuf>) -> GameGfxSource {
        GameGfxSource {
            root: root.into(),
            fetched: Vec::new(),
        }
    }

    /// The corpus root this source reads.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Everything fetched so far, in fetch order, with byte sizes.
    pub fn fetched(&self) -> &[(String, usize)] {
        &self.fetched
    }
}

impl ByteSource for GameGfxSource {
    fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
        if name.is_empty() || name.contains(SEP1) || name.contains(SEP2) || name.contains(PARENT) {
            return Err(GameError::AssetMissing {
                name: name.to_string(),
            });
        }
        let gfx_path = self.root.join("GAMEGFX").join(name);
        let sfx_path = self.root.join("SOUND").join("SFX").join(name);
        let path = if gfx_path.is_file() {
            gfx_path
        } else if sfx_path.is_file() {
            sfx_path
        } else {
            self.root.join(name)
        };
        let bytes = std::fs::read(path).map_err(|_| GameError::AssetMissing {
            name: name.to_string(),
        })?;
        self.fetched.push((name.to_string(), bytes.len()));
        Ok(bytes)
    }
}

/// One host-applied walk step: hold the CURRENT scene for `hold`
/// host pumps, then apply `action` (the input path cannot derive UI
/// intents yet - ADVANCE edges come from mouse clicks in EXW; the
/// shell script stands in for them).
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

/// Headless smoke options.
#[derive(Debug, Clone)]
pub struct HeadlessOptions {
    /// Install root (e.g. game-data/BEDLAM; GAMEGFX and the root
    /// are both searched - see [`GameGfxSource`]).
    pub gfx_dir: PathBuf,
    /// Region + language wiring (see [`ChainConfig`]).
    pub config: ChainConfig,
    /// Total host pumps to execute (fixed - the headless path owns
    /// no clock by construction).
    pub pumps: u64,
    /// The scripted scene walk (see [`default_walk`]).
    pub walk: Vec<WalkStep>,
}

impl HeadlessOptions {
    /// Defaults over `gfx_dir`: the [`default_walk`] campaign,
    /// enough pumps to finish it with a Select tail.
    pub fn new(gfx_dir: impl Into<PathBuf>) -> HeadlessOptions {
        HeadlessOptions {
            gfx_dir: gfx_dir.into(),
            config: ChainConfig::default(),
            pumps: 600,
            walk: default_walk(),
        }
    }
}

/// One visited scene and how many host pumps it absorbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneVisit {
    pub scene: Scene,
    pub pumps: u64,
}

/// What the smoke run did. `PartialEq` is the determinism gate: two
/// runs over the same corpus must produce the IDENTICAL report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessReport {
    /// Host pumps executed.
    pub pumps: u64,
    /// Scene visits in order (the walk actually taken).
    pub scenes: Vec<SceneVisit>,
    /// Walk actions applied, with the pump index of each.
    pub actions: Vec<(u64, SceneAction)>,
    /// Corpus fetches in order, with byte sizes.
    pub assets: Vec<(String, usize)>,
    /// Final hashed scene state (the D17 chain tail).
    pub scene_hash: u64,
    /// Canonical frame parity hash at the last pump.
    pub frame_hash: u64,
    /// Audio frames mixed off the host bus during the walk (D40
    /// smoke drain: PUMP_FRAMES per pump, discarded after counting).
    pub audio_frames: u64,
    /// Non-silent samples among them (L and R counted separately) -
    /// the entry-audio/movie signal the walk actually produced.
    pub audio_nonzero_samples: u64,
}

/// Run the headless smoke: construct the host, stage the boot pair,
/// then pump `pumps` fixed 60 Hz frames with neutral input, staging
/// every scene the walk enters and recording the whole journey.
///
/// Every mutation site checks for a scene change (an applied action
/// transitions immediately; the pump loop transitions on tick
/// boundaries - Boot auto-exits at BOOT_TICKS) so staging always
/// happens BETWEEN the transition and the next pump, exactly where
/// the D31 lifecycle expects the slot to exist.
pub fn run_headless(opts: &HeadlessOptions) -> Result<HeadlessReport, GameError> {
    let mut source = GameGfxSource::new(&opts.gfx_dir);
    let mut host = GameHost::new(
        &GameConfig::default(),
        &bedlam_core::sim::SimConfig::default(),
        [[0u8, 0, 0]; 256],
    );
    stage_boot(&mut host, &mut source, opts.config)?;

    let neutral = InputFrame::default();
    let mut walk: VecDeque<WalkStep> = opts.walk.iter().copied().collect();
    let mut scenes = vec![SceneVisit {
        scene: host.scene(),
        pumps: 0,
    }];
    let mut actions: Vec<(u64, SceneAction)> = Vec::new();
    let mut held = 0u64;
    // Audio smoke drain: one reusable scratch (fixed size, no
    // allocation in the loop), zeroed so stale data can never leak
    // into a count.
    let mut audio_scratch = vec![0i16; PUMP_FRAMES * 2];
    let mut audio_frames = 0u64;
    let mut audio_nonzero = 0u64;

    for pump in 0..opts.pumps {
        // 1. Walk step due? Apply the host intent.
        if held >= walk.front().map_or(u64::MAX, |step| step.0) {
            let (_, action) = walk.pop_front().expect("front checked");
            if action != SceneAction::None {
                host.apply(action);
                actions.push((pump, action));
            }
            held = 0;
        }
        // 2. Stage on a scene change from the apply (pre-pump: the
        //    slot must exist before sync_* runs inside the pump).
        stage_if_entered(&mut host, &mut source, opts.config, &mut scenes, &mut held)?;
        // 3. One fixed 60 Hz host pump.
        host.pump_frame(SUBTICKS_PER_PUMP, &neutral);
        // 4. Audio smoke drain: the same per-pump mix the device
        //    path would consume (D40; chunking-invariant, un-hashed).
        let mixed = host.render_audio(&mut audio_scratch)?;
        audio_frames += mixed as u64;
        audio_nonzero += audio_scratch[..mixed * 2]
            .iter()
            .filter(|&&s| s != 0)
            .count() as u64;
        // 5. Stage on a scene change from the pump (auto exits).
        stage_if_entered(&mut host, &mut source, opts.config, &mut scenes, &mut held)?;
        // 6. Bookkeeping.
        scenes.last_mut().expect("seeded").pumps += 1;
        held += 1;
    }

    Ok(HeadlessReport {
        pumps: opts.pumps,
        scenes,
        actions,
        assets: source.fetched().to_vec(),
        scene_hash: host.scene_hash().0,
        frame_hash: host.frame().parity_hash(),
        audio_frames,
        audio_nonzero_samples: audio_nonzero,
    })
}

/// Stage the just-entered scene (fetch + hand the D31-D37 assets to
/// the host) and open a new visit entry. No-op when the scene did
/// not change.
fn stage_if_entered(
    host: &mut GameHost,
    source: &mut GameGfxSource,
    config: ChainConfig,
    scenes: &mut Vec<SceneVisit>,
    held: &mut u64,
) -> Result<(), GameError> {
    let current = scenes.last().expect("seeded").scene;
    if host.scene() == current {
        return Ok(());
    }
    stage_scene(host, source, config)?;
    scenes.push(SceneVisit {
        scene: host.scene(),
        pumps: 0,
    });
    *held = 0;
    Ok(())
}
