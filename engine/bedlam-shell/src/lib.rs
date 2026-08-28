//! bedlam-shell: the native executable shell (P4 step 1) - the
//! ONLY place in the workspace where a window, a GPU surface and a
//! wall clock are allowed to exist (the bedlam-platform boundary note
//! hands exactly this role to the "future window host").
//!
//! Layout:
//! - [`clock`]: the fixed-step present clock. Pure integer-rational
//!   arithmetic; the struct never reads a clock itself - the window
//!   host feeds measured frame deltas in and gets back how many 60 Hz
//!   host pumps are due (DESIGN-RENDER sec 8 / Determinism Charter:
//!   timing decides HOW MANY identical pumps ran, never their content
//!   - each pump hands the host the same fixed dt).
//! - [`input`]: the input adapter + the P6 control-scheme consumer
//!   (D204). Winit key/mouse events accumulate into a
//!   [`bedlam_core::input::InputFrame`] per tick through a
//!   shell-owned button-bit layout (provisional, D38 - the EXW
//!   scan-code keystore map is RE-EXW-INPUT.md, engine-side binding
//!   lands with P2e input RE); the control-scheme arm of the
//!   immutable mode selects the mapping policy (modern = remappable
//!   WASD/1-4 + wheel zoom + gamepad; classic = the original
//!   scheme), entirely upstream of the frame contract.
//! - [`audio`]: the platform audio output (step 2, D40): a cpal
//!   output stream at the mixer-native 11025 Hz, drained through a
//!   bounded ring by the device callback while the main loop mixes
//!   into it from the GameHost audio bus. Device-gated exactly like
//!   the window: never built on the headless path; the mixer stays
//!   hermetic and the mixed stream stays un-hashed (D17 bucket b).
//! - [`save`]: the P6 QoL save-slot platform surface (D213): the
//!   five-slot selection, the EXW-faithful slot metadata presentation
//!   and the OPT-IN autosave policy — platform knobs OUT of
//!   ModeConfig (D200), grounded in docs/RE-EXW-SAVE.md over the
//!   engine's import-only save seam. Inert by design until the new
//!   versioned save format writer lands (config-not-state, D201).
//! - [`cdda`]: the P7 CDDA user-supply + local-cache surface
//!   (D223, docs/P7-PORTS.md §4): the documented lookup over
//!   user-supplied original track locations (the mixed-mode CD's
//!   seven CDDA tracks, WAV/CD) with the SILENT MISS posture, plus
//!   the OPTIONAL local lossy cache (IMA ADPCM, 4:1) generated on
//!   first run into a USER-OWNED cache dir — keyed by source
//!   identity, regenerated on mismatch, never redistributed, and
//!   never in game-data/ or the repo. Presentation bucket only
//!   (D17 b): nothing here reaches the host, the sim or any hash,
//!   and the headless smoke never runs it.
//! - [`native`]: the FIRST native ENHANCED pass (P6 opener, D217):
//!   the mission-identity strip — a pure palette-indexed UI-plane
//!   builder over the game's own SMLFONT glyphs, drawn at
//!   presentation resolution into the responsive layout's left
//!   margin by the window present site (ENHANCED mode only; the
//!   canonical frame rides byte-identical underneath).
//! - [`pacing`]: the P6 frame-pacing benchmark harness (PLAN §6
//!   closing QoL instrument, D219) — the pure cadence replay over
//!   measured frame deltas (pumps due, present-gate answers, the
//!   recompose alpha cadence at 240 Hz, the p95 frame-time
//!   budget) plus the pinned hardware profile and its
//!   unavailable-profile skip-clean posture; the bounded
//!   wall-clock measurement lives in `examples/frame-pacing.rs`
//!   (profile-gated) and the scheduled CI workflow
//!   `.github/workflows/frame-pacing.yml`.
//! - [`chain`]: the D31-D37 asset wiring - which corpus files each
//!   scene needs and the staging calls that hand them to
//!   [`bedlam_game::GameHost`] (the host never loads by itself).
//! - [`window`]: the winit window + wgpu surface + vsync present
//!   loop. RUNTIME-GATED: it only runs behind `--window` /
//!   `BEDLAM_SHELL=1` so tests and unattended runs never open a
//!   display; the headless smoke path ([`headless`]) is the default.
//!
//! The binary (`src/main.rs`) boots the wired chain from an install
//! tree: boot attract (D36) -> title (D31) -> brief/cutscene/shop/
//! loading (D32-D37) as the scene FSM walks them.

#![forbid(unsafe_code)]

pub mod audio;
pub mod cdda;
pub mod chain;
pub mod clock;
pub mod headless;
pub mod input;
pub mod native;
pub mod pacing;
pub mod save;
pub mod window;

pub use audio::{
    AudioDevice, AudioFeed, StreamFacts, VolumeLevel, VolumeMixers, PUMP_FRAMES, RING_CAP_FRAMES,
    TARGET_FRAMES,
};
pub use cdda::{CddaOptions, MusicCachePolicy, CDDA_TRACK_COUNT, MUSIC_DIR_ENV};
pub use chain::{scene_assets, stage_boot, stage_scene, ChainConfig};
pub use clock::FixedStepClock;
pub use headless::{
    default_walk, run_headless, GameGfxSource, HeadlessOptions, HeadlessReport, SceneVisit,
    WalkStep,
};
pub use input::{
    map_mouse_button, map_winit_key, Bindings, ControlScheme, GamepadButton, ShellInput, ShellKey,
};
pub use pacing::{
    benchmark_report, percentile_ns, profile_for, replay_cadence, summarize, CadenceDriver,
    FramePacing, PacingProfile, PacingSummary, PacingTrace, PacingVerdict, ProfileSelection,
    PINNED_240HZ, PROFILE_ENV,
};
pub use save::{
    save_level_text, summarize_saved_bdl, AutosavePolicy, SaveSlotId, SaveSlotMetadata,
    SaveSlotRow, EMPTY_SLOT_LINE,
};
pub use window::{run_window, ShellError, WindowOptions};
