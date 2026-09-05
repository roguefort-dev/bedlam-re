//! The winit window + wgpu surface host (P4 step 1).
//!
//! RUNTIME-GATED: [`run_window`] is only ever called from the binary
//! behind `--window` / `BEDLAM_SHELL=1` (see src/main.rs); tests and
//! unattended runs use the headless path and never open a display.
//!
//! This module is the ONLY place in the workspace where a wall clock
//! is allowed to exist (the bedlam-platform boundary note). The loop
//! shape honors the Determinism Charter (D17): the measured frame
//! delta feeds [`FixedStepClock::advance`] and decides only HOW MANY
//! identical 60 Hz host pumps run - each pump hands the host the
//! same fixed dt and a snapshot of the accumulated input events, so
//! the hashed state stays a pure function of the pump/input
//! sequence, never of display timing. Presentation is the PARITY
//! path (D20): upload the canonical 640x480 indexed frame + palette
//! through [`ParityPipeline`], palette-expand + integer-scale onto
//! the surface, present on vsync (Fifo).
//!
//! Input events accumulate into [`ShellInput`] between pumps (the
//! provisional D38 seam); a focus loss clears held state so a key
//! held across an alt-tab cannot stick. Audio output (step 2,
//! D40): a cpal stream drains the ring [`crate::audio`] keeps, and
//! each iteration refills that ring from the host audio bus toward
//! a fixed watermark - the ONLY producer. No audio device means
//! the shell runs silent (stderr note), never fatal.
//!
//! P6 MODE PLUMBING (D205, the p6-present-loop-wiring unit): the
//! platform selects ONE immutable [`ModeConfig`]
//! ([`WindowOptions::mode`], default = modern, `--classic` on the
//! binary) and it is routed into BOTH consumers at construction:
//! the host ([`host_sim_config`] -> `GameHost::new`, so
//! `GameHost::should_present` answers under the plumbed mode) and
//! the mapper ([`shell_input_for`] -> `ShellInput::with_scheme`,
//! the D204 consumer's platform selection). The PRESENT GATE is
//! the D203 timing-lock consumer wired into the real loop:
//! [`present_due`] — modern presents every vsync, classic holds
//! the previous image on zero-tick host frames (the original
//! frame-locked present-coupled pacing). The gate is a
//! presentation-bucket decision ONLY: the fixed-step clock/pump
//! contract above is untouched, and no hashed value can see it.
//!
//! P6 COMPOSITION POLICY (the p6-high-refresh-interpolation unit):
//! WHEN the gate opens, the modern decoupled arm RECOMPOSES the
//! presented frame from latest state + the interpolated camera —
//! [`present_camera_alpha`] pairs the gate's host with the shell
//! clock's [`FixedStepClock::fraction`] (the accumulator fraction
//! of the pending tick) and [`ShellApp::present`] feeds it to
//! [`GameHost::recompose`] before the upload. CAMERA/SCROLL ONLY
//! (the 1996 sprites had no sub-pixel positions; RE-EXW-CAMERA §4
//! found no sub-tick camera in the original — the blend is the
//! deliberate modernization manufacture, §5), classic arm
//! unchanged (the frame-locked pacing presents only after a tick —
//! nothing to interpolate). Still presentation-bucket ONLY (D17 b):
//! the fixed-step clock/pump contract and the hashed trajectory
//! stay untouched.
//!
//! P6 UNCAPPED PRESENT MODE (the p6-uncapped-present-mode unit): the
//! optional uncapped present of PLAN §6 — "vsync-locked present at
//! any refresh (60/120/144/240/360Hz+) or uncapped" — is a
//! PRESENTATION OPTION at the platform level ([`WindowOptions::
//! vsync`]; default = the vsync-locked Fifo present exactly as
//! shipped). D200 layering: vsync is a platform knob and stays OUT
//! of [`ModeConfig`]. The request is ARBITRATED by the pacing policy
//! ([`effective_vsync`]): only the modern Decoupled arm honors it —
//! with the Fifo block gone the unconditional redraw cycle
//! free-runs, the loop presenting as fast as it runs, every present
//! recomposing from latest state at the clock's accumulator fraction
//! ([`present_camera_alpha`] — coherent frames by construction) —
//! while the classic FrameLocked arm declines it and pins
//! vsync-locked (the original's visible refresh follows the fixed
//! logic tick, never the display rate; RE-EXW-PACER §3). The wgpu
//! mapping ([`surface_present_mode`]) is a pure function: Locked ->
//! Fifo (at any refresh), Uncapped -> Immediate when the surface
//! offers it, else the honest Fifo fallback (best-effort platform
//! knob, noted at configure time, never fatal).
//!
//! P6 WINDOW MODES (the p6-window-modes unit, PLAN §6 QoL "window
//! modes"): the window-mode selection ([`WindowOptions::
//! window_mode`], [`WindowMode`]) — WINDOWED (default, exactly as
//! shipped) / BORDERLESS borderless-fullscreen / exclusive-style
//! FULLSCREEN best-effort. D200 layering with NO purist
//! arbitration this time (unlike [`Vsync`]): the visible window
//! chrome never touches the sim — the original was a fullscreen
//! DOS exclusive with no windowed mode to preserve — so BOTH
//! pacing arms accept the selection identically and the selection
//! selects NOTHING in the host (it never reaches `ModeConfig`,
//! `SimConfig` or any hash). The winit fullscreen target is a PURE
//! function under test ([`fullscreen_target`] over plain
//! [`VideoModeChoice`] data — hermetic, no window needed); the
//! impure half is ONE binder ([`apply_fullscreen`]) shared by the
//! window build and the F11 runtime toggle. F11 is a PLATFORM/
//! window-manager key OUTSIDE both control schemes: it is
//! intercepted at the event handler BEFORE the mapper and NEVER
//! reaches [`ShellInput`]. Bounds: the swapchain follows the
//! existing `Resized` reconfigure path only; the fixed-step
//! clock/pump contract and the hashed trajectory stay untouched.
//!
//! P6 SCALING SELECTION (the p6-scaling-options unit, PLAN §6
//! "Resolution independence + GPU rendering ... GPU-scales it
//! (nearest/integer default; fit/fill/smooth options)"): the
//! already-landed bedlam-platform scale surface ([`ScaleMode`] +
//! [`FilterMode`], consumed by the parity pipeline's GPU scale
//! path and [`cursor_to_game`]) exposed as a platform
//! presentation knob riding [`WindowOptions::present`], selected
//! by the binary's `--scale`/`--filter` words through the PURE
//! mapping [`scaling_present_config`]. D200 layering with NO
//! purist arbitration (the window-modes posture): the original
//! was a FIXED 640x480 DOS framebuffer with no scaling mode to
//! preserve, so BOTH pacing arms accept the selection identically
//! and it selects NOTHING in the host beyond the [`PresentConfig`]
//! the GPU scale path consumes — never `ModeConfig`, never
//! `SimConfig`, never a hash. Default = Integer + Nearest EXACTLY
//! as shipped (the canonical 640x480 indexed frame + palette ride
//! unchanged; goldens stay resolution-agnostic; the palette
//! expansion policy is not a knob and stays `VgaExpand::Original`
//! under every selection).
//!
//! P6 ENHANCED NATIVE RENDER (the p6-enhanced-native-render unit,
//! PLAN §6 "ENHANCED mode is explicitly non-parity and renders
//! supported world/UI passes natively; bespoke responsive layouts
//! target 16:9 and 16:10 (16:10 authoring master with 16:9 safe
//! region), while other aspect ratios fit/letterbox/pillarbox"):
//! the PRESENTATION-MODE SELECTION ([`WindowOptions::
//! presentation`], [`PresentationMode`]) — PARITY (default, the
//! shipped posture exactly: the whole target is the canonical
//! frame GPU-scaled per the scale selection) / ENHANCED (the
//! responsive composition: the frame FITS into the centered 16:9
//! safe region of the 16:10 authoring master through
//! [`responsive_frame`], the first native pass renders at
//! presentation resolution in the left margin). D200 layering with
//! NO purist arbitration (the D215 posture): the knob is OUT of
//! [`ModeConfig`], both pacing arms accept it identically, and it
//! selects NOTHING in the sim — the canonical 640x480 indexed
//! frame + palette and every hash are byte-identical under either
//! selection. The FIRST native pass is the MISSION-IDENTITY STRIP
//! ([`crate::native`]): the game's own identity bytes, glyphs,
//! text color and palette, never invented pixels, never over game
//! pixels. The extended viewport (showing more map) stays OUT —
//! a separately FLAGGED gameplay change per PLAN, never a silent
//! default.
//!
//! P7 STEAMDECK PLATFORM PROFILE (the p7-steamdeck-default unit,
//! PLAN §6 "SteamDeck defaults stretch"; docs/P7-PORTS.md §5,
//! D224): the platform class identified at startup
//! ([`crate::platform`] — the DMI identity, Valve board +
//! Jupiter/Galileo product, fail-closed to Generic) selects ONLY
//! the DEFAULT of the [`WindowOptions::present`] scale knob: on a
//! SteamDeck the default becomes the fill-the-panel
//! [`ScaleMode::Stretch`] (the whole frame onto the whole
//! 1280x800 panel — no bars, no crop); every other machine keeps
//! Integer + Nearest bit-for-bit (the D215 default is untouched).
//! The user's `--scale`/`--filter` words keep their exact landed
//! semantics and an explicit word ALWAYS wins. D200 layering with
//! NO purist arbitration: the profile is a platform knob OUT of
//! [`ModeConfig`], both pacing arms accept it identically, and it
//! selects NOTHING in the sim — the sim config and every hash are
//! identical under every class/CLI combination.
//!
//! EXIT CONTRACT (D48): after the loop ends (window close,
//! fatal, or the `auto_exit_after` hook) the teardown is ORDERED -
//! audio stream parked first, then every wgpu/EGL object while the
//! winit window is still alive (the lazy wgpu Global teardown
//! marshals Wayland requests through the window's proxies and
//! SEGVs if they are gone) - so the process exits 0 instead of
//! dumping core.
//!
//! winit 0.30 shape (D39): the window is created inside resumed()
//! through ActiveEventLoop::create_window (the pre-run EventLoop
//! form is deprecated) and held behind an Arc, because wgpu needs an
//! owned window handle to hand the surface the static lifetime that
//! outlives run_app.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bedlam_core::mode::ModeConfig;
use bedlam_core::sim::SimConfig;
#[cfg(test)]
use bedlam_game::GameConfig;
use bedlam_game::{GameError, GameHost, Scene};
use bedlam_platform::layout::{layout_cursor_to_game, responsive_frame, PresentationMode};
use bedlam_platform::scale::{scale_rect, FilterMode, PresentConfig, Rect, ScaleMode};
use bedlam_platform::{ParityGpu, ParityPipeline};
use bedlam_render::{VgaExpand, CANON_H, CANON_W};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::monitor::VideoModeHandle;
use winit::window::{Fullscreen, Window, WindowId};

use crate::audio::{AudioDevice, VolumeMixers, TARGET_FRAMES};
use crate::cdda::{self, CddaOptions};
#[cfg(test)]
use crate::chain::stage_scene;
use crate::chain::ChainConfig;
use crate::clock::FixedStepClock;
#[cfg(test)]
use crate::clock::SUBTICKS_PER_PUMP;
use crate::controller::ShellController;
use crate::headless::GameGfxSource;
use crate::input::{map_mouse_button, ControlScheme, ShellInput};
use crate::native::{
    build_identity_strip, strip_slot_for, NativeStripPlane, SMLFONT_NAME, STRIP_SCALE,
};
use crate::save::{AutosavePolicy, SaveSlotId};

/// Shell-level failures (window/surface/GPU init + propagated game
/// staging errors). The window loop cannot return through winit
/// callbacks, so fatal conditions are stashed and surfaced by
/// [`run_window`] after the loop exits.
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("event loop init failed: {0}")]
    EventLoop(String),
    #[error("window creation failed: {0}")]
    Window(String),
    #[error("surface error: {0}")]
    Surface(String),
    #[error("no present-capable GPU adapter")]
    NoAdapter,
    #[error("{0}")]
    Game(#[from] GameError),
}

/// The platform-level vsync option (P6 optional uncapped present
/// mode, PLAN §6 "vsync-locked present at any refresh
/// (60/120/144/240/360Hz+) or uncapped"): a PRESENTATION OPTION
/// under the D200 layering — vsync is a platform knob and stays OUT
/// of [`ModeConfig`] (a mode change is a new host; a vsync change is
/// a new window run).
///
/// The value is a REQUEST the pacing policy arbitrates (see
/// [`effective_vsync`]): only the modern Decoupled pacing arm honors
/// [`Vsync::Uncapped`]; the classic FrameLocked arm pins
/// [`Vsync::Locked`] — the original's visible refresh follows the
/// fixed logic tick, never the display rate (RE-EXW-PACER §3), so an
/// uncapped loop is nonsensical there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vsync {
    /// Vsync-locked present (DEFAULT — exactly the shipped Fifo
    /// present): the loop paces to the display at ANY refresh
    /// (60/120/144/240/360 Hz+; the D205 wiring + the D207
    /// composition policy make every vsync a coherent recomposed
    /// frame), logic fixed at the original tick rate.
    #[default]
    Locked,
    /// Uncapped present: no vsync wait — the loop presents as fast
    /// as it runs, every present recomposing from latest state at
    /// the accumulator fraction (coherent frames by construction).
    /// Requires the modern Decoupled pacing arm; declined otherwise.
    Uncapped,
}

/// The platform-level window-mode option (P6 QoL unit
/// `p6-window-modes`, PLAN §6 "QoL: window modes, ..."): a
/// PRESENTATION OPTION under the D200 layering — window chrome is a
/// platform knob and stays OUT of [`ModeConfig`]. NO purist
/// arbitration this time (unlike [`Vsync`]): the original was a
/// fullscreen DOS exclusive with no windowed mode to preserve, so
/// the visible window chrome never touches the sim — BOTH pacing
/// arms accept the selection identically and the selection selects
/// NOTHING in the host (never `SimConfig`, never hashed; a window
/// mode change is a window-run concern, like vsync a new window
/// run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowMode {
    /// A decorated window at the configured inner size (DEFAULT —
    /// exactly the shipped shape: this is what the host has always
    /// opened).
    #[default]
    Windowed,
    /// Borderless fullscreen: the window covers the current monitor
    /// (compositor-managed; no mode switch).
    Borderless,
    /// Exclusive-style fullscreen, BEST-EFFORT: an exclusive video
    /// mode when the current monitor offers one, else the honest
    /// borderless degradation (noted, never fatal — the same
    /// best-effort posture as the vsync surface mapping).
    Fullscreen,
}

/// A video mode as PLAIN DATA — the hermetic view of a winit
/// `VideoModeHandle` (the live handle exists only with a window,
/// so every selection function below carries these instead and the
/// binder resolves one back to a handle at the window site).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoModeChoice {
    pub width: u32,
    pub height: u32,
    /// winit's millihertz refresh rate (60000 = 60 Hz).
    pub refresh_millihertz: u32,
    pub bit_depth: u16,
}

impl VideoModeChoice {
    /// The plain-data view of a live winit video mode (the impure
    /// half — only meaningful at the window site).
    fn of(mode: &VideoModeHandle) -> VideoModeChoice {
        let size = mode.size();
        VideoModeChoice {
            width: size.width,
            height: size.height,
            refresh_millihertz: mode.refresh_rate_millihertz(),
            bit_depth: mode.bit_depth(),
        }
    }

    fn area(&self) -> u32 {
        self.width.saturating_mul(self.height)
    }
}

/// The winit fullscreen target as PURE selection data (which
/// fullscreen shape to request — the binder turns it into a live
/// `winit::window::Fullscreen` at the window site).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenTarget {
    /// `Fullscreen::Borderless` on the window's current monitor.
    Borderless,
    /// `Fullscreen::Exclusive` at the chosen video mode.
    Exclusive(VideoModeChoice),
}

/// The BEST-EFFORT exclusive video-mode pick (pure): largest area,
/// then highest refresh, then highest bit depth — a total order, so
/// the pick is INDEPENDENT of the candidate list order. The largest
/// area is the monitor's native resolution in practice, and the
/// pacing arms already present correctly at any refresh (D205), so
/// no refresh preference is invented here.
fn pick_exclusive_mode(candidates: &[VideoModeChoice]) -> Option<VideoModeChoice> {
    candidates.iter().copied().max_by(|a, b| {
        (
            a.area(),
            a.refresh_millihertz,
            a.bit_depth,
            a.width,
            a.height,
        )
            .cmp(&(
                b.area(),
                b.refresh_millihertz,
                b.bit_depth,
                b.width,
                b.height,
            ))
    })
}

/// PURE: the winit fullscreen target for the window-mode selection
/// — the function under test, hermetic (plain data in, plain data
/// out; no window needed). [`WindowMode::Windowed`] maps to `None`
/// (exactly as shipped — the selection changes nothing until
/// asked); [`WindowMode::Borderless`] maps to
/// [`FullscreenTarget::Borderless`] regardless of candidates (no
/// mode switch is involved); [`WindowMode::Fullscreen`] maps to the
/// best-effort exclusive pick, else the HONEST borderless
/// degradation (an empty candidate list degrades rather than
/// failing — best-effort, never fatal). NO purist arbitration: this
/// reads the window-mode selection alone, never [`ModeConfig`].
fn fullscreen_target(mode: WindowMode, candidates: &[VideoModeChoice]) -> Option<FullscreenTarget> {
    match mode {
        WindowMode::Windowed => None,
        WindowMode::Borderless => Some(FullscreenTarget::Borderless),
        WindowMode::Fullscreen => Some(
            pick_exclusive_mode(candidates).map_or(FullscreenTarget::Borderless, |choice| {
                FullscreenTarget::Exclusive(choice)
            }),
        ),
    }
}

/// PURE: the fullscreen shape the F11 toggle ENTERS when the window
/// is currently windowed — the selection's preferred fullscreen
/// target. A WINDOWED selection still enters BORDERLESS on F11 (the
/// desktop F11 convention: the toggle must do something sensible
/// from every selection); BORDERLESS re-enters borderless;
/// FULLSCREEN enters its best-effort exclusive shape. Toggling OFF
/// always returns to windowed (`None`).
fn preferred_fullscreen(mode: WindowMode, candidates: &[VideoModeChoice]) -> FullscreenTarget {
    match mode {
        WindowMode::Windowed | WindowMode::Borderless => FullscreenTarget::Borderless,
        WindowMode::Fullscreen => {
            fullscreen_target(mode, candidates).unwrap_or(FullscreenTarget::Borderless)
        }
    }
}

/// PURE: the F11 toggle transition — the fullscreen target to apply
/// given whether the window is fullscreen RIGHT NOW (leaving →
/// `None` = windowed; entering → the selection's preferred shape).
fn toggle_fullscreen_target(
    mode: WindowMode,
    candidates: &[VideoModeChoice],
    fullscreen_now: bool,
) -> Option<FullscreenTarget> {
    if fullscreen_now {
        None
    } else {
        Some(preferred_fullscreen(mode, candidates))
    }
}

/// Whether a physical key is the PLATFORM window-mode toggle
/// (P6 QoL): F11 and nothing else. The toggle is a WINDOW-MANAGER
/// key OUTSIDE both control schemes — the handler intercepts it
/// before the mapper, so it NEVER reaches [`ShellInput`]; the
/// pin below additionally shows it maps to nothing in either
/// scheme, so even a forwarding bug could not make it sim input.
fn is_window_toggle_key(key: PhysicalKey) -> bool {
    matches!(key, PhysicalKey::Code(winit::keyboard::KeyCode::F11))
}

/// PURE: the binary's `--scale` word over the full domain —
/// `integer` / `fit` / `fill` / `stretch` -> [`ScaleMode`]. Fail-closed:
/// any other word is `None` and the binary exits 2 (a presentation
/// knob never guesses; the same posture as `--save-slot`'s domain
/// rejection).
pub fn scale_mode_from_cli(word: &str) -> Option<ScaleMode> {
    match word {
        "integer" => Some(ScaleMode::Integer),
        "fit" => Some(ScaleMode::Fit),
        "fill" => Some(ScaleMode::Fill),
        "stretch" => Some(ScaleMode::Stretch),
        _ => None,
    }
}

/// PURE: the binary's `--filter` word — `nearest` (the parity
/// default) / `linear` (smooth) -> [`FilterMode`]. Fail-closed
/// like the scale word.
pub fn filter_mode_from_cli(word: &str) -> Option<FilterMode> {
    match word {
        "nearest" => Some(FilterMode::Nearest),
        "linear" => Some(FilterMode::Linear),
        _ => None,
    }
}

/// PURE: the composed presentation config for a scaling selection
/// (P6 resolution independence, PLAN §6 "nearest/integer default;
/// fit/fill/smooth options") — the ONE mapping the binary applies
/// to [`WindowOptions::present`]. NO purist arbitration (the
/// window-modes posture: the original was a fixed 640x480 DOS
/// framebuffer with no scaling mode to preserve): the selection
/// touches EXACTLY the two knob fields — the 6-to-8 bit palette
/// expansion policy stays the parity [`VgaExpand::Original`] under
/// every selection, so the canonical 640x480 indexed frame +
/// palette ride unchanged and only the destination geometry /
/// sampling of the parity blit is selected. Defaults in, defaults
/// out: `ScaleMode::default()` = Integer and
/// `FilterMode::default()` = Nearest give `PresentConfig::default()`
/// bit-for-bit (the shipped posture).
pub fn scaling_present_config(scale: ScaleMode, filter: FilterMode) -> PresentConfig {
    PresentConfig {
        scale,
        filter,
        expand: VgaExpand::Original,
    }
}

/// PURE: the binary's `--presentation` word — `parity` (the shipped
/// posture default: the whole target is the canonical frame
/// GPU-scaled per the scale selection) / `enhanced` (the responsive
/// layout composition + the native passes) -> [`PresentationMode`].
/// Fail-closed like the scale/filter words (the `--save-slot`
/// domain posture: a presentation knob never guesses).
pub fn presentation_mode_from_cli(word: &str) -> Option<PresentationMode> {
    match word {
        "parity" => Some(PresentationMode::Parity),
        "enhanced" => Some(PresentationMode::Enhanced),
        _ => None,
    }
}

/// PURE: where the canonical 640x480 frame lands on a `w` x `h`
/// target — the composition decision the present site consults
/// (P6 ENHANCED opener). PARITY keeps the LANDED
/// [`scale_rect`] path over the whole target exactly as shipped
/// (the `draw` path — including the Fill uv crop); ENHANCED uses
/// the responsive layout's WORLD rect (the frame fits whole inside
/// the centered 16:9 safe region — the Fill crop never applies
/// there, the frame is never cropped in the responsive layout).
fn frame_draw_rect(presentation: PresentationMode, cfg: &PresentConfig, w: u32, h: u32) -> Rect {
    match presentation {
        PresentationMode::Parity => scale_rect(cfg.scale, CANON_W, CANON_H, w, h),
        PresentationMode::Enhanced => responsive_frame(w, h).world,
    }
}

/// One bounded runtime volume adjustment (P6 QoL volume mixers,
/// D212): which bus moves and which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VolumeAdjust {
    MusicUp,
    MusicDown,
    SfxUp,
    SfxDown,
}

impl VolumeAdjust {
    /// The ORIGINAL'S OWN stepper step (RE-EXW-INPUT sec 5: the EXW
    /// mission-shell Up/Down arrows move g_music_volume by ±5 on a
    /// 0..100 clamp) — the modern platform keys keep the same step
    /// and clamp.
    pub(crate) fn step(self) -> i8 {
        match self {
            VolumeAdjust::MusicUp | VolumeAdjust::SfxUp => 5,
            VolumeAdjust::MusicDown | VolumeAdjust::SfxDown => -5,
        }
    }
}

/// PURE: the bounded P6 QoL volume-key set (D212) — PageUp/PageDown
/// adjust the MUSIC bus, BracketRight/BracketLeft the SFX bus, and
/// NOTHING else is a volume key. PLATFORM-ONLY keys OUTSIDE both
/// control schemes: the handler intercepts them BEFORE the mapper,
/// so they NEVER reach [`ShellInput`]; the pin below additionally
/// shows all four map to nothing in either scheme, so even a
/// forwarding bug could not make them sim input (the F11 posture).
fn volume_adjust_key(key: PhysicalKey) -> Option<VolumeAdjust> {
    use winit::keyboard::KeyCode as K;
    match key {
        PhysicalKey::Code(K::PageUp) => Some(VolumeAdjust::MusicUp),
        PhysicalKey::Code(K::PageDown) => Some(VolumeAdjust::MusicDown),
        PhysicalKey::Code(K::BracketRight) => Some(VolumeAdjust::SfxUp),
        PhysicalKey::Code(K::BracketLeft) => Some(VolumeAdjust::SfxDown),
        _ => None,
    }
}

/// Apply one bounded adjustment to a selection (PURE — a change is a
/// new value, never a host mutation): the music and sfx buses step
/// independently, each clamped 0..=100 the original's own way.
fn volume_mixers_stepped(mixers: VolumeMixers, adj: VolumeAdjust) -> VolumeMixers {
    match adj {
        VolumeAdjust::MusicUp | VolumeAdjust::MusicDown => {
            mixers.with_music(mixers.music().stepped(adj.step()))
        }
        VolumeAdjust::SfxUp | VolumeAdjust::SfxDown => {
            mixers.with_sfx(mixers.sfx().stepped(adj.step()))
        }
    }
}

/// Window host options.
#[derive(Debug, Clone)]
pub struct WindowOptions {
    /// Install root (e.g. game-data/BEDLAM; the source also
    /// accepts GAMEGFX directly - see [`GameGfxSource`]).
    pub gfx_dir: PathBuf,
    /// Region + language wiring.
    pub config: ChainConfig,
    /// Window title.
    pub title: String,
    /// Initial inner size in logical pixels.
    pub size: (u32, u32),
    /// Presentation config (PARITY defaults if unchanged). The P6
    /// SCALING SELECTION rides this knob (D215, PLAN §6
    /// "nearest/integer default; fit/fill/smooth options"): default
    /// = Integer + Nearest EXACTLY as shipped; the binary's
    /// `--scale`/`--filter` words select [`ScaleMode`]/
    /// [`FilterMode`] through the PURE [`scaling_present_config`]
    /// mapping. The selection selects NOTHING in the host: never
    /// `ModeConfig`, never `SimConfig`, never a hash — the
    /// already-landed GPU scale path (the parity pipeline draw +
    /// [`cursor_to_game`]) is the only consumer.
    pub present: PresentConfig,
    /// The P6 mode this host runs under (D205, the platform-level
    /// classic/modern selection): ONE immutable [`ModeConfig`]
    /// routed into BOTH platform consumers at construction — the
    /// host (via [`host_sim_config`], so the present gate
    /// [`GameHost::should_present`] answers under this mode) and
    /// the input mapper (via [`shell_input_for`], the D204
    /// control-scheme consumer's selection). Default = modern
    /// (PLAN §6; the binary's `--classic` selects the classic
    /// preset). A mode change is a new host, never a mid-run
    /// mutation. Presentation options (`present`, `vsync` below)
    /// stay OUT of the mode per the D200 layering: window mode,
    /// vsync and scaling are platform knobs, not purist toggles.
    pub mode: ModeConfig,
    /// The platform-level vsync option (P6 uncapped present mode,
    /// PLAN §6 "vsync-locked ... or uncapped"; D200 layering — a
    /// platform knob, OUT of `ModeConfig`): default
    /// [`Vsync::Locked`], the vsync-locked Fifo present exactly as
    /// shipped. [`Vsync::Uncapped`] is a request the pacing policy
    /// arbitrates ([`effective_vsync`]): honored only under the
    /// modern Decoupled arm; the classic arm pins locked.
    pub vsync: Vsync,
    /// The platform-level window-mode option (P6 QoL, PLAN §6
    /// "QoL: window modes"; D200 layering — a platform knob, OUT
    /// of `ModeConfig`, with NO purist arbitration): default
    /// [`WindowMode::Windowed`], the decorated window exactly as
    /// shipped. [`WindowMode::Borderless`] /
    /// [`WindowMode::Fullscreen`] select the fullscreen shapes
    /// ([`fullscreen_target`]); F11 toggles at runtime. The
    /// selection selects nothing in the host — it never reaches
    /// the sim config or any hash.
    pub window_mode: WindowMode,
    /// The platform-level per-bus volume selection (P6 QoL volume
    /// mixers, PLAN §6 "QoL: ... volume mixers"; D200 layering — a
    /// PLATFORM knob, OUT of `ModeConfig`, with NO purist
    /// arbitration: audio is presentation bucket, D17 b). Default
    /// [`VolumeMixers::SHIPPED`] = both buses FULL = the shipped mix
    /// exactly — the device-bound stream is bit-identical at the
    /// default, and the engine's mixed parity stream is NEVER
    /// touched by any setting (the gain applies at the
    /// [`crate::audio::AudioFeed::fill_from`] watermark site only).
    /// The binary's `--music`/`--sfx` select the starting levels;
    /// PageUp/PageDown and BracketRight/BracketLeft adjust at
    /// runtime.
    pub volume: VolumeMixers,
    /// The platform-level save-slot selection (P6 QoL save slots,
    /// D213; PLAN §6 "save slots + metadata + opt-in autosave";
    /// D200 layering — a PLATFORM knob, OUT of `ModeConfig`, with NO
    /// purist arbitration): the slot of the original's own FIVE-slot
    /// domain (RE-EXW-SAVE sec 1) that the platform's save surface
    /// targets. Default = [`SaveSlotId::FIRST`] — a MODERN platform
    /// default (the original has no persistent selection: its
    /// save/load screens pick the slot by click, per screen). The
    /// selection selects nothing in the host — it never reaches the
    /// sim config or any hash; the binary's `--save-slot` selects
    /// the starting slot.
    pub save_slot: SaveSlotId,
    /// The platform-level autosave policy (P6 QoL; D213; RE-EXW-SAVE
    /// sec 4 — the exhaustive EXW writer census: the ONLY savegame
    /// writers are the save screen's slot commit and the first-run
    /// five-EMPTY init, BOTH user-initiated — THE SHIPPED GAME NEVER
    /// AUTOSAVES). Default = [`AutosavePolicy::Off`], the shipped
    /// posture exactly; [`AutosavePolicy::On`] is the modern OPT-IN
    /// whose save opportunities mirror the original's own gating
    /// (single-player, campaign boundary — never mid-mission). The
    /// policy is presentation/platform policy ONLY: the surface
    /// lands INERT (the D201 seam posture — no engine write seam
    /// ships in this unit; the new versioned save format writer is
    /// future engine work, config-not-state), so nothing here can
    /// reach the sim, a hash, or any file. The binary's `--autosave`
    /// opts in.
    pub autosave: AutosavePolicy,
    /// The platform-level CDDA user-supply + local-cache selection
    /// (P7, PLAN §6 "CDDA: user-supplied original tracks (WAV/CD),
    /// optional local lossy cache generated on first run — never
    /// redistributed"; D223, the docs/P7-PORTS.md §4 contract): the
    /// explicit music search-dir override (the binary's
    /// `--music-dir`; `BEDLAM_MUSIC_DIR` is consulted when the flag
    /// is absent) and the cache policy (default ON = the plan's
    /// generated-on-first-run posture; `--no-music-cache` opts
    /// out). D200 layering with NO purist arbitration: a PLATFORM
    /// knob, OUT of `ModeConfig`, that selects NOTHING in the host —
    /// the window startup resolves the user-supplied tracks through
    /// the documented lookup (SILENT MISS: music silent + a stderr
    /// note, never fatal) and refreshes the optional local lossy
    /// cache into the USER-OWNED cache home (never game-data/,
    /// never the repo, keyed by source identity, regenerated on
    /// mismatch, never redistributed). Music is presentation bucket
    /// (D17 b): the sim config and every hash are untouched by any
    /// setting, and the headless path owns no surface (the binary
    /// notes + ignores the flags there).
    pub music: CddaOptions,
    /// The platform-level frame-presentation selection (P6 ENHANCED
    /// native render opener, PLAN §6 "PARITY mode keeps the
    /// canonical 640x480 indexed frame + palette and GPU-scales it
    /// ... ENHANCED mode is explicitly non-parity and renders
    /// supported world/UI passes natively"): PARITY (default — the
    /// shipped posture exactly, the whole target is the canonical
    /// frame GPU-scaled per `present`) or ENHANCED (the responsive
    /// 16:10-master / 16:9-safe-region layout + the native passes,
    /// starting with the mission-identity strip). D200 layering
    /// with NO purist arbitration (the D215 posture): the knob is
    /// OUT of `ModeConfig`, both pacing arms accept it identically,
    /// and it selects NOTHING in the host — the canonical frame +
    /// palette and every hash are byte-identical under either
    /// selection (the headless path owns no surface, so the binary
    /// notes + ignores the flag there).
    pub presentation: PresentationMode,
    /// TEST/REPRO HOOK (D48): auto-exit the loop this long after the
    /// first resume, through the SAME exit path as window close
    /// (`ActiveEventLoop::exit`). `None` (the default) never fires;
    /// the shell binary wires it from `BEDLAM_WINDOW_EXIT_MS` so an
    /// unattended run can exercise the window teardown end to end.
    /// The deadline check is the one extra wall-clock read in this
    /// module - it decides only WHEN the loop stops, never any
    /// hashed host state (the headless path stays the gate).
    pub auto_exit_after: Option<Duration>,
}

impl WindowOptions {
    /// PARITY defaults over `gfx_dir`: 960x720 logical (a 1.5x
    /// integer scale of 640x480 at 1.0 DPI), Integer + Nearest.
    pub fn new(gfx_dir: impl Into<PathBuf>) -> WindowOptions {
        WindowOptions {
            gfx_dir: gfx_dir.into(),
            config: ChainConfig::default(),
            title: String::from("Bedlam (1996) - re shell"),
            size: (960, 720),
            present: PresentConfig::default(),
            mode: ModeConfig::default(),
            vsync: Vsync::default(),
            window_mode: WindowMode::default(),
            volume: VolumeMixers::SHIPPED,
            save_slot: SaveSlotId::FIRST,
            autosave: AutosavePolicy::Off,
            music: CddaOptions::default(),
            presentation: PresentationMode::default(),
            auto_exit_after: None,
        }
    }
}

/// The host's sim config derived from the platform options (P6
/// D205): the immutable mode rides `SimConfig` into
/// `GameHost::new`; seed and time base stay the defaults — the
/// mode is the ONLY platform selection that enters the sim, and it
/// enters as config, never state (D201: not hashed, not
/// serialized; the hashed trajectory is arm-invariant).
fn host_sim_config(opts: &WindowOptions) -> SimConfig {
    SimConfig {
        mode: opts.mode,
        ..SimConfig::default()
    }
}

/// The input accumulator derived from the platform options (P6
/// D205): the control-scheme arm of the SAME plumbed mode selects
/// the mapper's [`ControlScheme`] (the D204 consumer's platform
/// selection — until this unit the window path ran default-modern).
fn shell_input_for(opts: &WindowOptions) -> ShellInput {
    ShellInput::new().with_scheme(ControlScheme::for_mode(opts.mode))
}

/// Whether the present loop may put a new image on the surface for
/// THIS host frame — the timing-lock pacing policy wired into the
/// real loop (P6 D203 consumer / D205 wiring). Pure delegation to
/// [`GameHost::should_present`]: MODERN presents every vsync
/// (zero-tick high-refresh frames recompose and present too);
/// CLASSIC holds the previously presented image on zero-tick host
/// frames (the original frame-locked present-coupled pacing,
/// RE-EXW-PACER §3 — the visible refresh follows the fixed logic
/// tick, never the display rate). The boot frame is presentable in
/// both arms. Presentation-bucket ONLY (D17 b): the answer never
/// reaches the sim, the state hash or the scene hash. Shared
/// crate-visible with the pacing benchmark harness (`crate::pacing`),
/// which records the SAME answers hermetically.
pub(crate) fn present_due(host: &GameHost) -> bool {
    host.should_present()
}

/// The camera-interpolation alpha for THIS host frame, if the mode's
/// composition policy recomposes — the p6-high-refresh-interpolation
/// companion of [`present_due`] (P6, PLAN §6 "the frame is composed
/// from latest state + camera/scroll interpolation").
///
/// MODERN (decoupled) arm: `Some(clock.fraction())` — the accumulator
/// fraction of the pending logic tick, i.e. where the present lands
/// between the last executed tick and the next one; the presented
/// frame recomposes from LATEST state with the camera lerped from
/// the last executed tick toward the present (camera/scroll ONLY —
/// sprites stay grid-quantized; the sub-pixel blitter stays
/// default-off and out of scope). On a 60 Hz display the steady
/// state reads 1.0, so the interpolated camera IS the parity camera
/// — the policy only becomes visible when the display outpaces the
/// fixed tick rate (the high-refresh present it exists for).
/// CLASSIC (frame-locked) arm: `None` — the arm presents only after
/// a tick executes, so the presented image is exactly the tick-state
/// camera (the original's shape, RE-EXW-CAMERA §4); nothing to
/// interpolate.
///
/// Presentation-bucket ONLY (D17 b): the alpha derives from measured
/// display timing and never reaches the sim, the state hash or the
/// scene hash (the clock/pump contract is untouched). Shared
/// crate-visible with the pacing benchmark harness (`crate::pacing`),
/// which records the SAME answers hermetically.
pub(crate) fn present_camera_alpha(host: &GameHost, clock: &FixedStepClock) -> Option<f32> {
    host.camera_interpolation().then(|| clock.fraction())
}

/// Arbitrate the vsync request against the pacing policy selected by
/// the timing-lock arm of the plumbed mode (P6 uncapped present
/// mode, PLAN §6 "vsync-locked present at any refresh ... or
/// uncapped"): the MODERN Decoupled arm honors the request — with
/// the Fifo block gone, the unconditional redraw cycle free-runs and
/// the loop presents as fast as it runs, every present recomposing
/// from latest state at the clock's accumulator fraction (coherent
/// frames by construction) — while the CLASSIC FrameLocked arm
/// DECLINES uncapped and pins [`Vsync::Locked`]: the original's
/// visible refresh follows the fixed logic tick, never the display
/// rate (RE-EXW-PACER §3 — one loop pass per flip), so an uncapped
/// loop is nonsensical there. This reads exactly the arm
/// `GameHost::present_pacing` reads (D203; agreement unit-pinned).
/// Presentation-bucket ONLY (D17 b): the answer configures the
/// swapchain and never reaches the sim or any hash.
fn effective_vsync(mode: ModeConfig, requested: Vsync) -> Vsync {
    use bedlam_core::mode::{PuristToggle, ToggleArm};
    if mode.arm(PuristToggle::TimingLock) == ToggleArm::Classic {
        Vsync::Locked
    } else {
        requested
    }
}

/// The wgpu swapchain PresentMode for the effective vsync selection
/// — the PURE MAPPING the surface configuration consumes
/// (unit-pinned hermetically; no window needed).
/// [`Vsync::Locked`] maps to `Fifo` unconditionally (the only
/// universally supported mode — the shipped present, at any refresh:
/// the blocking present paces the loop to the display).
/// [`Vsync::Uncapped`] maps to `Immediate` when the surface offers
/// it, else falls back to `Fifo`: Mailbox is NOT uncapped (it still
/// paces to the display), so an uncapped request without `Immediate`
/// support degrades honestly to vsync-locked — a best-effort
/// platform knob, never fatal (the same posture as a missing audio
/// device; the configure site notes the fallback).
fn surface_present_mode(
    effective: Vsync,
    offered: &[bedlam_platform::wgpu::PresentMode],
) -> bedlam_platform::wgpu::PresentMode {
    use bedlam_platform::wgpu::PresentMode;
    match effective {
        Vsync::Locked => PresentMode::Fifo,
        Vsync::Uncapped => {
            if offered.contains(&PresentMode::Immediate) {
                PresentMode::Immediate
            } else {
                PresentMode::Fifo
            }
        }
    }
}

/// Open the window host and run until the window closes.
/// Blocks on the winit event loop; `pollster` blocks again inside
/// adapter/device setup. The caller owns the runtime gate.
pub fn run_window(mut opts: WindowOptions) -> Result<(), ShellError> {
    opts.size = (opts.size.0.max(64), opts.size.1.max(64));
    let event_loop = EventLoop::new().map_err(|e| ShellError::EventLoop(e.to_string()))?;

    let mut controller = ShellController::new(
        GameGfxSource::new(&opts.gfx_dir),
        opts.config,
        &host_sim_config(&opts),
    )?;

    // Audio (step 2, D40): open the default output device, best
    // effort - no device (or no workable config) runs silent, the
    // game itself never depends on it. Prefill the ring so playback
    // does not start in an underrun.
    let audio = AudioDevice::open_default();
    match audio.as_ref() {
        Some(dev) => {
            let facts = dev.facts();
            eprintln!(
                "bedlam-shell: audio output {} Hz, {} ch, {}",
                facts.rate, facts.channels, facts.format
            );
            // P6 QoL volume mixers (D212): the platform selection
            // enters HERE and only here — the feed's gain site. The
            // engine stream and every hash are untouched.
            if dev.mixers() != opts.volume {
                dev.set_mixers(opts.volume);
                eprintln!(
                    "bedlam-shell: volume music {}%, sfx {}%",
                    opts.volume.music().percent(),
                    opts.volume.sfx().percent()
                );
            }
            let feed = dev.feed().clone();
            if let Err(err) = feed.fill_from_controller(&mut controller, TARGET_FRAMES) {
                eprintln!("bedlam-shell: audio prefill failed ({err}); continuing");
            }
        }
        None => eprintln!("bedlam-shell: no audio output device; running silent"),
    }

    // P7 CDDA user-supply + local-cache surface (D223, the
    // docs/P7-PORTS.md §4 contract): the ONE-SHOT startup probe of
    // the user-supplied music locations (documented lookup, SILENT
    // MISS — a miss is music silent + a note, never fatal) and the
    // optional local lossy cache refresh under the guarded
    // USER-OWNED cache home. Presentation bucket ONLY (D17 b): this
    // reads user-owned files, writes the user-owned cache, prints
    // notes — and touches nothing else. The explicit override is
    // the CLI flag, else the env var; the install dir closes the
    // search (the packaged game's user drops rips there; in the
    // repo layout that is the operator's read-only corpus copy).
    let music_dir = opts
        .music
        .search_dir
        .clone()
        .or_else(|| std::env::var_os(cdda::MUSIC_DIR_ENV).map(PathBuf::from));
    cdda::startup(music_dir.as_deref(), &opts.gfx_dir, opts.music.cache);

    // The mapper's scheme comes from the SAME plumbed mode
    // (computed before `opts` moves into the app struct).
    let input = shell_input_for(&opts);
    let mut app = ShellApp {
        opts,
        controller,
        input,
        clock: FixedStepClock::host(),
        smlfont: None,
        gfx: None,
        audio,
        exit_deadline: None,
        fatal: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| ShellError::EventLoop(e.to_string()))?;

    // --- Ordered teardown (D48; fixes the Escape SIGSEGV, coredumps
    // 422346 + 1150695) ---
    // 1. Stop the audio stream FIRST: AudioDevice::drop quiets the
    //    feed (the callback's dead-feed guard - exact silence, no
    //    ring touch), pauses, and drops the stream, so the cpal
    //    callback thread is parked before anything else dies.
    drop(app.audio.take());
    // 2. All wgpu/EGL objects die HERE, while the winit window is
    //    still alive: the LAST wgpu-object drop runs the lazy wgpu
    //    Global teardown (eglTerminate through Mesa), which
    //    marshals Wayland requests through the window's proxies -
    //    those proxies must not be freed yet. WindowHost declares
    //    `window` last, so its field-order drop already honors this;
    //    taking gfx out makes the contract explicit at the exit
    //    site. The window Arc is released only after the teardown.
    drop(app.gfx.take());
    match app.fatal.take() {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// sRGB-format preference for the swapchain (fall back to the first
/// supported format otherwise).
fn wgpu_like_srgb(format: &bedlam_platform::wgpu::TextureFormat) -> bool {
    format.is_srgb()
}

/// The video modes of the window's CURRENT monitor as plain data
/// (the hermetic selection input, collected at the window site —
/// absent monitor yields the empty list, which the selection
/// degrades from, never fails on).
fn monitor_video_choices(window: &Window) -> Vec<VideoModeChoice> {
    window
        .current_monitor()
        .map(|m| m.video_modes().map(|v| VideoModeChoice::of(&v)).collect())
        .unwrap_or_default()
}

/// Bind the PURE fullscreen target to LIVE winit handles — the ONE
/// impure half of the window-mode selection, shared by the window
/// build and the F11 toggle (both go through here, so the shapes
/// can never disagree). `None` leaves/keeps the window windowed.
/// The exclusive binding degrades honestly to borderless when the
/// chosen mode is gone by resolve time (noted, never fatal).
fn apply_fullscreen(window: &Window, target: Option<FullscreenTarget>) {
    let monitor = window.current_monitor();
    let fullscreen = target.map(|t| match t {
        FullscreenTarget::Borderless => Fullscreen::Borderless(monitor),
        FullscreenTarget::Exclusive(choice) => match monitor
            .as_ref()
            .and_then(|m| m.video_modes().find(|v| VideoModeChoice::of(v) == choice))
        {
            Some(mode) => Fullscreen::Exclusive(mode),
            None => {
                eprintln!(
                    "bedlam-shell: exclusive video mode unavailable; using borderless fullscreen (best-effort)"
                );
                Fullscreen::Borderless(monitor)
            }
        },
    });
    window.set_fullscreen(fullscreen);
}

/// The live window half (everything that only exists once a window
/// does). Built once, inside resumed().
///
/// FIELD ORDER IS LOAD-BEARING (D48): Rust drops fields in
/// declaration order, and every wgpu/EGL object (`surface`, `gpu`,
/// `pipeline`) wraps this window's Wayland/X proxies. The wgpu
/// Global tears EGL down LAZILY, at the drop of the LAST wgpu
/// object (the pipeline's bind group in the crash stacks 422346 /
/// 1150695), and that teardown (`eglTerminate` through Mesa)
/// marshals Wayland requests THROUGH THE WINDOW'S PROXIES - which
/// SEGVs if the winit window died first. Declaring `window` last
/// keeps the window (and its proxies) alive through the entire wgpu
/// teardown in a plain field-order drop.
struct WindowHost {
    surface: bedlam_platform::wgpu::Surface<'static>,
    gpu: ParityGpu,
    pipeline: ParityPipeline,
    /// The ENHANCED native-pass plane pipeline (P6 opener): a
    /// palette-indexed UI plane of the strip's own dimensions,
    /// built through the SAME landed parity-pipeline path. None
    /// until the first strip; rebuilt only when the strip's
    /// dimensions change (the plane size is fixed at texture
    /// creation — identity text changes are the only resize
    /// trigger, rare by construction).
    ui: Option<(u32, u32, ParityPipeline)>,
    surface_cfg: bedlam_platform::wgpu::SurfaceConfiguration,
    cursor: Option<PhysicalPosition<f64>>,
    last_frame: Instant,
    /// The winit window - ALWAYS dropped last (see the struct doc).
    window: Arc<Window>,
}

impl WindowHost {
    /// Build the window + surface + GPU + pipeline on `event_loop`
    /// (the winit 0.30 canonical site). The clock origin starts
    /// HERE so the first measured delta is a real frame.
    fn open(event_loop: &ActiveEventLoop, opts: &WindowOptions) -> Result<WindowHost, ShellError> {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(opts.title.clone())
                    .with_inner_size(LogicalSize::new(opts.size.0, opts.size.1))
                    .with_resizable(true),
            )
            .map_err(|e| ShellError::Window(e.to_string()))?;
        let window = Arc::new(window);

        // P6 window modes (p6-window-modes): apply the platform
        // selection's fullscreen target BEFORE the surface is
        // sized, so the swapchain starts at the fullscreen extent
        // (any later extent change arrives as a Resized event —
        // the existing reconfigure path, nothing new). The
        // exclusive request degrades honestly (noted, never
        // fatal).
        let video_choices = monitor_video_choices(&window);
        if opts.window_mode == WindowMode::Fullscreen
            && fullscreen_target(opts.window_mode, &video_choices)
                == Some(FullscreenTarget::Borderless)
        {
            eprintln!(
                "bedlam-shell: no exclusive video mode on this monitor; using borderless fullscreen (best-effort)"
            );
        }
        apply_fullscreen(&window, fullscreen_target(opts.window_mode, &video_choices));

        let instance = bedlam_platform::wgpu::Instance::default();
        // The Arc is the owned window handle that gives the surface
        // a static lifetime (it must outlive run_app unborrowed).
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| ShellError::Surface(e.to_string()))?;
        let (adapter, gpu) =
            ParityGpu::new_for_surface(&instance, &surface).ok_or(ShellError::NoAdapter)?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu_like_srgb)
            .unwrap_or_else(|| caps.formats.first().copied().expect("at least one format"));
        let size = window.inner_size();
        let mut surface_cfg = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| ShellError::Surface(String::from("no default surface configuration")))?;
        // P6 uncapped present mode: the platform vsync option,
        // arbitrated by the pacing policy — the classic frame-locked
        // arm pins vsync (RE-EXW-PACER §3: one loop pass per flip),
        // the modern arm honors the request. Locked = the shipped
        // Fifo present (any refresh); Uncapped = Immediate when the
        // surface offers it, else the honest Fifo fallback
        // (best-effort, noted, never fatal).
        let vsync = effective_vsync(opts.mode, opts.vsync);
        surface_cfg.present_mode = surface_present_mode(vsync, &caps.present_modes);
        if vsync == Vsync::Uncapped
            && surface_cfg.present_mode != bedlam_platform::wgpu::PresentMode::Immediate
        {
            eprintln!(
                "bedlam-shell: uncapped present unsupported on this surface; staying vsync-locked (Fifo)"
            );
        }
        surface.configure(gpu.device(), &surface_cfg);
        let pipeline = ParityPipeline::new(&gpu, format);

        window.request_redraw();
        Ok(WindowHost {
            surface,
            gpu,
            pipeline,
            ui: None,
            surface_cfg,
            cursor: None,
            last_frame: Instant::now(),
            window,
        })
    }

    /// Reconfigure the swapchain (init + resize). Zero dimensions
    /// are clamped to 1: wgpu rejects empty surfaces.
    fn reconfigure(&mut self, width: u32, height: u32) {
        self.surface_cfg.width = width.max(1);
        self.surface_cfg.height = height.max(1);
        self.surface.configure(self.gpu.device(), &self.surface_cfg);
    }
}

/// The live window host state.
///
/// FIELD ORDER IS LOAD-BEARING (D48): `audio` precedes `gfx` so a
/// plain struct drop stops the audio stream BEFORE any wgpu/EGL
/// object begins dying (the cpal callback thread must never outlive
/// the start of teardown). `run_window` additionally performs the
/// teardown explicitly - see its ordered-teardown block.
struct ShellApp {
    opts: WindowOptions,
    controller: ShellController<GameGfxSource>,
    input: ShellInput,
    clock: FixedStepClock,
    /// The ENHANCED native pass's SMLFONT bank cache (P6 opener):
    /// `None` = not fetched yet, `Some(None)` = fetched and missing
    /// (the strip stays disabled, noted once, never fatal — the
    /// best-effort platform posture). The headless path never
    /// populates it (it owns no present surface).
    smlfont: Option<Option<Vec<u8>>>,
    /// The audio output (step 2, D40), absent when no device
    /// exists - the shell runs silent then, never fatal. Dropped
    /// BEFORE `gfx` (declaration order).
    audio: Option<AudioDevice>,
    /// The window half, absent until resumed() builds it. Its drop
    /// runs the whole wgpu/EGL teardown, which must see a live
    /// window (see WindowHost).
    gfx: Option<WindowHost>,
    /// When the auto-exit hook (D48) fires; absent when disabled.
    exit_deadline: Option<Instant>,
    fatal: Option<ShellError>,
}

/// Map a window-space physical cursor position to canonical game
/// space (640x480), inverting the presentation scale rect exactly.
/// Bars clamp to the frame edge. None for a degenerate rect or the
/// Fill mode (Fill crops the source - its inverse needs the uv rect
/// and is out of scope; relative aiming is used there instead).
/// Stretch maps the WHOLE frame onto the WHOLE target, so it
/// inverts absolutely like Integer/Fit (no crop, no bars).
fn cursor_to_game(
    px: f64,
    py: f64,
    win_w: u32,
    win_h: u32,
    cfg: &PresentConfig,
) -> Option<(i32, i32)> {
    if cfg.scale == ScaleMode::Fill {
        return None;
    }
    let r = scale_rect(cfg.scale, CANON_W, CANON_H, win_w, win_h);
    if r.w == 0 || r.h == 0 {
        return None;
    }
    let gx = ((px - r.x as f64) * f64::from(CANON_W) / r.w as f64).round() as i32;
    let gy = ((py - r.y as f64) * f64::from(CANON_H) / r.h as f64).round() as i32;
    Some((
        gx.clamp(0, CANON_W as i32 - 1),
        gy.clamp(0, CANON_H as i32 - 1),
    ))
}

/// Convert the visible pointer into the relative input consumed by the scene.
fn steer_pointer(
    host: &GameHost,
    frame: &mut bedlam_core::input::InputFrame,
    target: Option<(i32, i32)>,
) {
    if let Some(target) = target {
        let cursor = match host.scene() {
            Scene::Title => host.menu_cursor(),
            Scene::Mission => host.mission().map(|mission| mission.cursor()),
            _ => None,
        };
        if let Some((mx, my)) = cursor {
            frame.mouse_dx = (target.0 - mx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            frame.mouse_dy = (target.1 - my).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }
}

#[cfg(test)]
mod mission_pointer_tests {
    use super::*;
    use bedlam_core::input::InputFrame;

    #[test]
    fn visible_map_button_click_reaches_mission_at_scaled_resolutions() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM");
        for (width, height) in [(1200, 900), (1920, 1080)] {
            let mut host =
                GameHost::new(&GameConfig::default(), &SimConfig::default(), [[0; 3]; 256]);
            for _ in 0..bedlam_game::BOOT_TICKS {
                host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
            }
            // Real mouse edges traverse Title -> Brief -> Select -> Mission.
            for _ in 0..3 {
                host.pump_frame(
                    SUBTICKS_PER_PUMP,
                    &InputFrame {
                        mouse_buttons: 1,
                        ..Default::default()
                    },
                );
                host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
            }
            assert_eq!(host.scene(), Scene::Mission);
            stage_scene(
                &mut host,
                &mut GameGfxSource::new(&root),
                ChainConfig::default(),
            )
            .unwrap();
            host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
            assert!(!host.mission().unwrap().map_overlay_on());
            let config = PresentConfig::default();
            let rect = scale_rect(config.scale, CANON_W, CANON_H, width, height);
            let px = rect.x as f64 + 560.0 * rect.w as f64 / CANON_W as f64;
            let py = rect.y as f64 + 450.0 * rect.h as f64 / CANON_H as f64;
            let mut frame = InputFrame {
                mouse_buttons: 1,
                ..Default::default()
            };
            // First event after entry/focus need not contain a relative delta.
            steer_pointer(
                &host,
                &mut frame,
                cursor_to_game(px, py, width, height, &config),
            );
            host.pump_frame(SUBTICKS_PER_PUMP, &frame);
            assert_eq!(host.mission().unwrap().cursor(), (560, 450));
            assert!(host.mission().unwrap().map_overlay_on());
        }
    }
}

impl ShellApp {
    /// Execute `pumps` fixed 60 Hz host pumps (timing decided HOW
    /// MANY; each pump is the same fixed dt + an input snapshot).
    fn run_pumps(&mut self, pumps: u32) -> Result<(), GameError> {
        for _ in 0..pumps {
            let mut frame = self.input.tick();
            // Both menus and missions consume canonical relative deltas.
            // Align the active scene's cursor with the visible window pointer
            // before the click is consumed, including after entry or refocus.
            steer_pointer(
                self.controller.host(),
                &mut frame,
                self.game_cursor_target(),
            );
            self.controller.pump(frame.into())?;
        }
        Ok(())
    }

    /// The real cursor in game space, when the window knows both the
    /// cursor position and the surface size. PARITY inverts the
    /// scale-rect mapping ([`cursor_to_game`]); ENHANCED inverts the
    /// responsive layout's WORLD rect instead (P6 opener — the
    /// click targets live in the responsive layout,
    /// RESEARCH-HD-ASSET-PIPELINE §8; the layout never crops the
    /// frame, so the mapping is always absolute, never Fill's
    /// relative-only case).
    fn game_cursor_target(&self) -> Option<(i32, i32)> {
        let g = self.gfx.as_ref()?;
        let pos = g.cursor?;
        match self.opts.presentation {
            PresentationMode::Parity => cursor_to_game(
                pos.x,
                pos.y,
                g.surface_cfg.width,
                g.surface_cfg.height,
                &self.opts.present,
            ),
            PresentationMode::Enhanced => layout_cursor_to_game(
                pos.x,
                pos.y,
                &responsive_frame(g.surface_cfg.width, g.surface_cfg.height),
            ),
        }
    }

    /// Upload + present the canonical frame (PARITY path, D20).
    /// Split field borrows: gfx mutable, host frame read-only.
    fn present(&mut self) {
        // P6 PRESENT GATE (D203 consumer, wired D205): the
        // timing-lock pacing policy decides whether THIS host frame
        // may put a new image on the surface. MODERN (default)
        // presents every vsync; CLASSIC holds the previously
        // presented image on zero-tick host frames (the surface
        // keeps the last presented buffer — wgpu needs no re-
        // present for that). The redraw request itself stays
        // UNCONDITIONAL (about_to_wait step 5) so the loop keeps
        // its vsync-driven liveness in BOTH arms — only the
        // surface write is gated. Presentation-bucket only: the
        // gate reads the plumbed mode + the last pump's tick count,
        // neither hashed.
        if !present_due(self.controller.host()) {
            return;
        }
        // P6 COMPOSITION POLICY (p6-high-refresh-interpolation): the
        // modern arm recomposes the frame it is about to upload from
        // latest state + the interpolated camera at the accumulator
        // fraction of the pending tick (zero-tick high-refresh
        // frames included — that is the entire point: the camera
        // sweeps between logic ticks). Classic arm: no alpha, the
        // pump's parity frame uploads as-is. Presentation-bucket
        // only (D17 b): recompose mutates the frame and nothing
        // else; the next pump re-renders parity regardless.
        if let Some(alpha) = present_camera_alpha(self.controller.host(), &self.clock) {
            self.controller.recompose(alpha);
        }
        // P6 ENHANCED NATIVE RENDER (the opener): the composition
        // decision is made BEFORE the surface borrow so the strip
        // staging (which may fetch the SMLFONT bank through the
        // source) never fights it. PARITY runs the landed path
        // EXACTLY as before (the whole target is the canonical
        // frame GPU-scaled per the scale selection — bit-for-bit
        // the same calls). ENHANCED composes the responsive layout:
        // the canonical frame FITS whole into the centered 16:9
        // safe region (the Fill crop never applies) and the first
        // native pass — the mission-identity strip — renders at
        // presentation resolution in the left margin. Both
        // compositions read the SAME engine frame: the canonical
        // 640x480 indexed frame + palette are byte-identical under
        // either selection (presentation-bucket only, D17 b).
        let enhanced = self.opts.presentation == PresentationMode::Enhanced;
        let layout = enhanced
            .then(|| {
                self.gfx
                    .as_ref()
                    .map(|g| responsive_frame(g.surface_cfg.width, g.surface_cfg.height))
            })
            .flatten();
        let strip = enhanced.then(|| self.stage_native_strip()).flatten();
        let strip_target = match (&strip, &layout) {
            (Some(plane), Some(frame)) => crate::native::strip_rect(frame, plane.w, plane.h),
            _ => None,
        };
        let Some(g) = self.gfx.as_mut() else {
            return;
        };
        g.pipeline.upload_frame(self.controller.host().frame());
        let surface_texture = match g.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(err) => {
                // Outdated/Lost after a resize: reconfigure and skip;
                // the next vsync presents.
                eprintln!("bedlam-shell: surface acquire failed ({err}); reconfiguring");
                let (w, h) = (g.surface_cfg.width, g.surface_cfg.height);
                g.reconfigure(w, h);
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&bedlam_platform::wgpu::TextureViewDescriptor::default());
        match enhanced {
            false => {
                // PARITY: the landed present path, unchanged.
                let buffer = g.pipeline.draw(
                    &view,
                    g.surface_cfg.width,
                    g.surface_cfg.height,
                    &self.opts.present,
                );
                g.gpu.queue().submit([buffer]);
            }
            true => {
                // ENHANCED: the frame into the layout's world rect
                // (the composition decision fn — the Fill crop never
                // applies, the frame fits whole inside the safe
                // region), then the native strip pass over the matte
                // margin. The frame's filter + palette-expansion
                // still ride the presentation config (the strip
                // keeps Nearest + the SAME expansion so the colors
                // agree exactly).
                let world = frame_draw_rect(
                    self.opts.presentation,
                    &self.opts.present,
                    g.surface_cfg.width,
                    g.surface_cfg.height,
                );
                let mut buffers = vec![g.pipeline.draw_rect(&view, world, &self.opts.present)];
                if let (Some(plane), Some(rect)) = (&strip, strip_target) {
                    let stale =
                        g.ui.as_ref()
                            .is_none_or(|(pw, ph, _)| *pw != plane.w || *ph != plane.h);
                    if stale {
                        g.ui = Some((
                            plane.w,
                            plane.h,
                            ParityPipeline::with_plane(
                                &g.gpu,
                                g.surface_cfg.format,
                                plane.w,
                                plane.h,
                            ),
                        ));
                    }
                    if let Some((_, _, ui)) = g.ui.as_mut() {
                        ui.upload_indexed(&plane.indices, &self.controller.host().frame().palette);
                        let cfg = PresentConfig {
                            scale: ScaleMode::Integer,
                            filter: FilterMode::Nearest,
                            expand: self.opts.present.expand,
                        };
                        buffers.push(ui.draw_rect(&view, rect, &cfg));
                    }
                }
                g.gpu.queue().submit(buffers);
            }
        }
        surface_texture.present();
    }

    /// Stage the ENHANCED mission-identity strip (P6 opener): the
    /// pure plane build gated on the mission scene + the slot the
    /// engine answers, over the cached SMLFONT bank. Presentation
    /// bucket ONLY — reads the host, never mutates it.
    fn stage_native_strip(&mut self) -> Option<NativeStripPlane> {
        let slot = strip_slot_for(
            self.controller.host().scene(),
            self.controller.host().mission_slot(),
        )?;
        let bank = self.smlfont_bytes()?;
        build_identity_strip(bank, slot.0, slot.1, STRIP_SCALE)
    }

    /// The SMLFONT.BIN bytes through the existing corpus source
    /// (fetched once, cached; a miss disables the strip with one
    /// note — best-effort platform surface, never fatal).
    fn smlfont_bytes(&mut self) -> Option<&[u8]> {
        if self.smlfont.is_none() {
            let fetched = match self.controller.load_asset(SMLFONT_NAME) {
                Ok(bytes) => Some(bytes),
                Err(err) => {
                    eprintln!(
                        "bedlam-shell: {SMLFONT_NAME} unavailable; the ENHANCED identity strip is disabled ({err})"
                    );
                    None
                }
            };
            self.smlfont = Some(fetched);
        }
        self.smlfont.as_ref().and_then(|fetched| fetched.as_deref())
    }
}

impl ApplicationHandler for ShellApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // First resume builds the window half; later resumes (some
        // platforms suspend/resume) reuse the existing window.
        if self.gfx.is_some() {
            return;
        }
        match WindowHost::open(event_loop, &self.opts) {
            Ok(gfx) => {
                self.gfx = Some(gfx);
                // Arm the auto-exit hook (D48) from the first live
                // frame; a stale deadline can never predate resume.
                self.exit_deadline = self.opts.auto_exit_after.map(|d| Instant::now() + d);
            }
            Err(err) => {
                self.fatal = Some(err);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Destroyed => {}
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.input.clear_held();
                    if let Some(g) = self.gfx.as_mut() {
                        g.cursor = None;
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.reconfigure(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.present(),
            WindowEvent::KeyboardInput { event, .. } => {
                // P6 QoL window-mode toggle (p6-window-modes): F11
                // is a PLATFORM/window-manager key OUTSIDE both
                // control schemes — intercepted HERE, before the
                // mapper, so it NEVER reaches ShellInput. The
                // toggle moves only the window chrome through the
                // SAME binder the window build used; the extent
                // change arrives as a Resized event (the existing
                // reconfigure path), and nothing about the
                // clock/pump contract or the host moves.
                if is_window_toggle_key(event.physical_key) {
                    if event.state == ElementState::Pressed {
                        if let Some(g) = self.gfx.as_ref() {
                            let choices = monitor_video_choices(&g.window);
                            let target = toggle_fullscreen_target(
                                self.opts.window_mode,
                                &choices,
                                g.window.fullscreen().is_some(),
                            );
                            apply_fullscreen(&g.window, target);
                        }
                    }
                    return;
                }
                // P6 QoL volume mixers (D212): PageUp/PageDown and
                // BracketRight/BracketLeft are PLATFORM keys OUTSIDE
                // both control schemes — intercepted HERE, before the
                // mapper, so they NEVER reach ShellInput (the F11
                // posture; pinned dead to both schemes by test). The
                // adjustment touches ONLY the audio feed's device-
                // bound gain: not the host, not the input queue, not
                // any hash. No device means nothing to adjust.
                if let Some(adj) = volume_adjust_key(event.physical_key) {
                    if event.state == ElementState::Pressed {
                        if let Some(dev) = self.audio.as_ref() {
                            let mixers = volume_mixers_stepped(dev.mixers(), adj);
                            if mixers != dev.mixers() {
                                dev.set_mixers(mixers);
                                eprintln!(
                                    "bedlam-shell: volume music {}%, sfx {}%",
                                    mixers.music().percent(),
                                    mixers.sfx().percent()
                                );
                            }
                        }
                    }
                    return;
                }
                // The scheme-aware physical->semantic path (P6 D204:
                // the control-scheme arm selects the mapping policy).
                // Escape is a GAME key (operator 2026-08-23): it must
                // never close the window. It rides the input queue
                // like every other key (bound in BOTH scheme arms);
                // the EXW options-screen target will consume it once
                // P2e input RE pins the binding. Until then it is an
                // in-game no-op. Exit = window close only.
                self.input
                    .set_physical_key(event.physical_key, event.state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(mask) = map_mouse_button(button) {
                    self.input.set_mouse(mask, state == ElementState::Pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(g) = self.gfx.as_mut() {
                    if let Some(last) = g.cursor.replace(position) {
                        self.input
                            .mouse_move((position.x - last.x) as i32, (position.y - last.y) as i32);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.wheel(delta),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 0. Auto-exit hook (D48, test repro): fire the SAME exit
        //    path as Escape once the deadline passes. Checked before
        //    any pump so the teardown sees a settled loop.
        if let Some(deadline) = self.exit_deadline {
            if Instant::now() >= deadline {
                event_loop.exit();
                return;
            }
        }
        // 1. Measure the inter-frame delta (the ONLY clock read) and
        //    decide how many identical pumps are due (short-lived
        //    borrow: the pump/stage calls need the whole self).
        let pumps = {
            let Some(g) = self.gfx.as_mut() else {
                return;
            };
            let now = Instant::now();
            let delta_ns =
                u64::try_from(now.duration_since(g.last_frame).as_nanos()).unwrap_or(u64::MAX);
            g.last_frame = now;
            self.clock.advance(delta_ns)
        };
        // 2. Run them (fixed dt + input snapshots each).
        if let Err(err) = self.run_pumps(pumps) {
            self.fatal = Some(err.into());
            event_loop.exit();
            return;
        }
        // 3. Refill the audio ring toward the watermark (the device
        //    callback drains it at its own pace; production self-
        //    balances against the device clock - D40). The Arc comes
        //    out first so host and audio borrow disjointly.
        if let Some(feed) = self.audio.as_ref().map(|dev| dev.feed().clone()) {
            if let Err(err) = feed.fill_from_controller(&mut self.controller, TARGET_FRAMES) {
                eprintln!("bedlam-shell: audio fill failed ({err}); continuing");
            }
        }
        // 5. Request the next frame. UNCONDITIONAL in both arms
        //    (loop liveness: the vsync-paced redraw cycle keeps
        //    about_to_wait pumping even when the classic present
        //    gate holds the previous image — the gate lives at the
        //    present site, `ShellApp::present`). Under the effective
        //    Fifo present the surface write blocks to vsync, pacing
        //    the loop; under an HONORED uncapped selection
        //    (Immediate) nothing blocks and this cycle free-runs —
        //    the uncapped loop shape: the loop presents as fast as
        //    it runs, each iteration still executing at most what
        //    the clock banks (fixed dt per pump, the contract
        //    untouched).
        if let Some(g) = self.gfx.as_ref() {
            g.window.request_redraw();
        }
    }
}

#[cfg(test)]
mod present_loop_tests {
    //! The P6 platform wiring pins (D205, `p6-present-loop-wiring`):
    //! the ONE plumbed [`WindowOptions::mode`] reaches BOTH platform
    //! consumers (host + mapper), the present gate holds the previous
    //! image only on the classic arm's zero-tick frames, and the
    //! plumbing never touches the hashed trajectory. Test surface =
    //! the ONE purist toggle, both arms (per-axis mixes appear only
    //! as the axis-independence control), never the feature
    //! cross-product (D200).
    use super::*;
    use bedlam_core::input::InputFrame;
    use bedlam_core::mode::{PuristToggle, ToggleArm};

    fn opts_with(mode: ModeConfig) -> WindowOptions {
        let mut opts = WindowOptions::new("test-install");
        opts.mode = mode;
        opts
    }

    fn host_for(opts: &WindowOptions) -> GameHost {
        GameHost::new(
            &GameConfig::default(),
            &host_sim_config(opts),
            [[0u8, 0, 0]; 256],
        )
    }

    /// Default = modern (PLAN §6): the platform selection starts on
    /// the modern arm, exactly like the sim default it feeds.
    #[test]
    fn window_options_default_mode_is_modern() {
        assert_eq!(WindowOptions::new("test-install").mode, ModeConfig::MODERN);
    }

    /// ONE plumbed selection drives BOTH consumers: the mode rides
    /// the derived SimConfig into the host unchanged (so the
    /// present gate answers under it), and the SAME mode selects
    /// the mapper's ControlScheme via `ControlScheme::for_mode`
    /// (the D204 consumer's platform selection).
    #[test]
    fn one_plumbed_selection_reaches_host_and_mapper() {
        for (mode, scheme) in [
            (ModeConfig::MODERN, ControlScheme::Modern),
            (ModeConfig::CLASSIC, ControlScheme::Classic),
        ] {
            let opts = opts_with(mode);
            assert_eq!(host_sim_config(&opts).mode, mode);
            let host = host_for(&opts);
            assert_eq!(host.mode(), mode);
            assert_eq!(shell_input_for(&opts).scheme(), scheme);
            assert_eq!(ControlScheme::for_mode(opts.mode), scheme);
        }
    }

    /// Axis independence at the PLATFORM level: each consumer reads
    /// its own arm of the same plumbed mode — the timing-lock arm
    /// moves present pacing only, the control-scheme arm moves the
    /// mapper only. Per-axis mixes are the controls here, never the
    /// feature cross-product.
    #[test]
    fn each_consumer_reads_only_its_own_arm() {
        use bedlam_game::host::PresentPacing;
        let timing_only =
            opts_with(ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic));
        assert_eq!(
            host_for(&timing_only).present_pacing(),
            PresentPacing::FrameLocked
        );
        assert_eq!(
            shell_input_for(&timing_only).scheme(),
            ControlScheme::Modern
        );
        let controls_only =
            opts_with(ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic));
        assert_eq!(
            host_for(&controls_only).present_pacing(),
            PresentPacing::Decoupled
        );
        assert_eq!(
            shell_input_for(&controls_only).scheme(),
            ControlScheme::Classic
        );
    }

    /// THE PRESENT GATE (the D203 consumer wired into the loop):
    /// `present_due` — what [`ShellApp::present`] consults before
    /// touching the surface — presents every host frame on the
    /// MODERN arm (zero-tick frames included) and on the CLASSIC
    /// arm holds the previous image exactly on zero-tick frames
    /// (present iff the pump executed a tick; the boot frame is
    /// presentable in both arms so the platform has something to
    /// blit).
    #[test]
    fn present_gate_holds_the_previous_image_only_on_classic_zero_tick_frames() {
        for (mode, expect) in [
            (ModeConfig::MODERN, [true, true, true, true]),
            (ModeConfig::CLASSIC, [true, false, true, false]),
        ] {
            let mut host = host_for(&opts_with(mode));
            // Boot frame (no pump yet): presentable in both arms.
            assert!(present_due(&host), "boot frame presentable");
            // Script: a 60 Hz tick frame, then a zero-tick frame,
            // then a tick frame, then a short banked frame (3
            // sub-ticks — no whole tick after a 4, so zero ticks).
            let dts = [SUBTICKS_PER_PUMP, 0, SUBTICKS_PER_PUMP, 3];
            let mut observed = [false; 4];
            for (i, dt) in dts.iter().copied().enumerate() {
                let executed = host.pump_frame(dt, &InputFrame::default());
                if mode.is_purist(PuristToggle::TimingLock) {
                    assert_eq!(executed > 0, expect[i], "classic: gate iff a tick executed");
                }
                observed[i] = present_due(&host);
            }
            assert_eq!(observed, expect, "gate cadence for {mode:?}");
        }
    }

    /// The plumbing is CONFIG-NOT-STATE (the D201/D203/D204 property
    /// at the platform boundary): the SAME pump script through
    /// hosts built from the modern and classic platform options
    /// yields the identical executed-tick sequence, sim tick count,
    /// sim state hash, scene hash AND frame parity hash — the
    /// platform selection can never touch the hashed trajectory,
    /// only the presentation bucket.
    #[test]
    fn platform_mode_plumbing_never_touches_the_hashed_trajectory() {
        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut modern = host_for(&opts_with(ModeConfig::MODERN));
        let mut classic = host_for(&opts_with(ModeConfig::CLASSIC));
        let mut executed_modern = Vec::new();
        let mut executed_classic = Vec::new();
        for (i, dt) in script.iter().copied().enumerate() {
            let input = if i % 3 == 0 {
                InputFrame {
                    mouse_dx: 2,
                    mouse_dy: 1,
                    ..InputFrame::default()
                }
            } else {
                InputFrame::default()
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
        assert_eq!(modern.frame().parity_hash(), classic.frame().parity_hash());
    }

    /// The composition-policy selection at the platform boundary
    /// (the p6-high-refresh-interpolation companion of
    /// `present_due`): the SAME timing-lock arm that gates the
    /// present selects the recompose alpha — modern
    /// `Some(clock.fraction())` (the accumulator fraction of the
    /// pending tick), classic `None` (the frame-locked arm presents
    /// only after a tick — nothing to interpolate). Axis
    /// independence: the control-scheme arm alone moves nothing
    /// here.
    #[test]
    fn present_camera_alpha_is_the_modern_arm_only() {
        let mut clock = FixedStepClock::host();
        // A 240 Hz-style cadence: bank lands mid-pending-tick so the
        // alpha is a real interior value, not the 60 Hz steady 1.0.
        for _ in 0..6 {
            clock.advance(4_166_666);
        }
        assert!((clock.fraction() - 0.5).abs() < 1e-6);

        let modern = host_for(&opts_with(ModeConfig::MODERN));
        assert_eq!(
            present_camera_alpha(&modern, &clock),
            Some(clock.fraction())
        );

        let classic = host_for(&opts_with(ModeConfig::CLASSIC));
        assert_eq!(present_camera_alpha(&classic, &clock), None);

        let timing_only =
            opts_with(ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic));
        assert_eq!(present_camera_alpha(&host_for(&timing_only), &clock), None);

        let controls_only =
            opts_with(ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic));
        assert_eq!(
            present_camera_alpha(&host_for(&controls_only), &clock),
            Some(clock.fraction())
        );
    }

    /// The PRESENT-SITE recompose the loop wires
    /// (`present_camera_alpha` -> `GameHost::recompose`, both arms
    /// consulted every present) never touches the hashed trajectory:
    /// the SAME pump script with the modern arm recomposing at the
    /// clock's accumulator fractions and the classic arm declining
    /// yields the identical executed-tick sequence, sim tick count,
    /// state hash and scene hash — the camera interpolation is
    /// presentation-bucket only (D17 b). The frame parity hash
    /// deliberately MAY diverge on the modern arm (the interpolated
    /// camera is the feature — pinned host-side by
    /// `recompose_interpolates_only_on_the_decoupled_arm`); the
    /// platform pin is that nothing hashed moves.
    #[test]
    fn present_site_recompose_never_touches_the_hashed_trajectory() {
        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut modern = host_for(&opts_with(ModeConfig::MODERN));
        let mut classic = host_for(&opts_with(ModeConfig::CLASSIC));
        let mut clock = FixedStepClock::host();
        let moving = InputFrame {
            buttons: 1,
            mouse_dx: 2,
            mouse_dy: 1,
            ..InputFrame::default()
        };
        let mut executed_modern = Vec::new();
        let mut executed_classic = Vec::new();
        let mut recomposed = 0usize;
        for (i, &dt) in script.iter().cycle().take(40).enumerate() {
            let input = if i % 3 == 0 {
                moving
            } else {
                InputFrame::default()
            };
            // The clock tracks the vsync cadence the present site
            // reads it at; the pump script stays the contract under
            // test (hashes depend on the dt sequence only).
            clock.advance(4_166_666);
            executed_modern.push(modern.pump_frame(dt, &input));
            executed_classic.push(classic.pump_frame(dt, &input));
            if let Some(alpha) = present_camera_alpha(&modern, &clock) {
                assert!(modern.recompose(alpha), "endpoint staged from frame 0");
                recomposed += 1;
            }
            assert!(!classic.recompose(clock.fraction()));
        }
        assert!(
            recomposed > 0,
            "the modern present site recomposes on this script"
        );
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
}

#[cfg(test)]
mod uncapped_present_tests {
    //! The P6 uncapped-present-mode pins (PLAN §6 "vsync-locked
    //! present at any refresh (60/120/144/240/360Hz+) or uncapped"):
    //! a PLATFORM presentation option (D200 layering — OUT of
    //! ModeConfig, default = the shipped vsync-locked Fifo present),
    //! arbitrated by the pacing policy (the modern Decoupled arm
    //! honors it; the classic FrameLocked arm pins locked), mapped
    //! to a wgpu PresentMode by a PURE function — all hermetic, no
    //! window needed. The shell fixed-step clock/pump contract and
    //! the hashed trajectory stay untouched. Test surface = the ONE
    //! purist toggle, both arms (per-axis mixes only as the
    //! axis-independence control), never the feature cross-product;
    //! the catalog stays EMPTY (a plan-named present unit is not a
    //! catalog entry).
    use super::*;
    use bedlam_core::input::InputFrame;
    use bedlam_core::mode::{PuristToggle, ToggleArm};
    use bedlam_game::host::PresentPacing;
    use bedlam_platform::wgpu::PresentMode;

    fn opts_with(mode: ModeConfig) -> WindowOptions {
        let mut opts = WindowOptions::new("test-install");
        opts.mode = mode;
        opts
    }

    fn host_for(opts: &WindowOptions) -> GameHost {
        GameHost::new(
            &GameConfig::default(),
            &host_sim_config(opts),
            [[0u8, 0, 0]; 256],
        )
    }

    /// The shipped default: the platform vsync option starts LOCKED
    /// — the Fifo present exactly as before this unit (the option
    /// changes nothing until asked), and Locked maps to Fifo
    /// regardless of what the surface offers (Fifo is universally
    /// supported).
    #[test]
    fn vsync_option_defaults_to_the_shipped_locked_present() {
        assert_eq!(Vsync::default(), Vsync::Locked);
        assert_eq!(WindowOptions::new("test-install").vsync, Vsync::Locked);
        assert_eq!(
            surface_present_mode(
                Vsync::Locked,
                &[
                    PresentMode::Fifo,
                    PresentMode::Mailbox,
                    PresentMode::Immediate
                ]
            ),
            PresentMode::Fifo
        );
    }

    /// POLICY SELECTION: the pacing policy (the timing-lock arm —
    /// exactly the selector `GameHost::present_pacing` reads)
    /// arbitrates the request. The modern Decoupled arm honors
    /// uncapped; the classic FrameLocked arm declines it and pins
    /// locked (the original's visible refresh follows the fixed
    /// logic tick, never the display rate — RE-EXW-PACER §3).
    /// Axis independence: the control-scheme arm alone never
    /// declines. The decline is end-to-end: a classic arm asking
    /// uncapped still configures Fifo.
    #[test]
    fn uncapped_is_honored_only_by_the_decoupled_pacing_arm() {
        let timing_classic =
            ModeConfig::default().with(PuristToggle::TimingLock, ToggleArm::Classic);
        let controls_classic =
            ModeConfig::default().with(PuristToggle::ControlScheme, ToggleArm::Classic);
        for (mode, requested, expect) in [
            (ModeConfig::MODERN, Vsync::Locked, Vsync::Locked),
            (ModeConfig::MODERN, Vsync::Uncapped, Vsync::Uncapped),
            (ModeConfig::CLASSIC, Vsync::Locked, Vsync::Locked),
            (ModeConfig::CLASSIC, Vsync::Uncapped, Vsync::Locked),
            (timing_classic, Vsync::Uncapped, Vsync::Locked),
            (controls_classic, Vsync::Uncapped, Vsync::Uncapped),
        ] {
            assert_eq!(
                effective_vsync(mode, requested),
                expect,
                "{mode:?} x {requested:?}"
            );
            // The arbitration agrees with the host's own pacing
            // policy on the same plumbed mode: Uncapped is
            // effective iff the pacing is Decoupled AND uncapped
            // was requested.
            let pacing = host_for(&opts_with(mode)).present_pacing();
            assert_eq!(
                effective_vsync(mode, requested) == Vsync::Uncapped,
                pacing == PresentPacing::Decoupled && requested == Vsync::Uncapped,
                "{mode:?}: arbitration tracks the host policy"
            );
        }
        // The DECLINE reaches the swapchain: a classic arm asking
        // uncapped on an Immediate-capable surface still gets Fifo.
        assert_eq!(
            surface_present_mode(
                effective_vsync(ModeConfig::CLASSIC, Vsync::Uncapped),
                &[PresentMode::Fifo, PresentMode::Immediate]
            ),
            PresentMode::Fifo
        );
    }

    /// THE WGPU MAPPING (the pure function the surface configuration
    /// consumes): Locked -> Fifo always; Uncapped -> Immediate when
    /// offered; else the honest Fifo fallback — Mailbox is NOT
    /// uncapped (it still paces to the display), so an
    /// Immediate-less surface degrades all the way to vsync-locked
    /// rather than half-honoring the request.
    #[test]
    fn surface_present_mode_is_the_pure_locked_or_immediate_mapping() {
        assert_eq!(
            surface_present_mode(Vsync::Locked, &[PresentMode::Fifo]),
            PresentMode::Fifo
        );
        assert_eq!(
            surface_present_mode(Vsync::Locked, &[PresentMode::Mailbox]),
            PresentMode::Fifo
        );
        assert_eq!(
            surface_present_mode(
                Vsync::Uncapped,
                &[PresentMode::Fifo, PresentMode::Immediate]
            ),
            PresentMode::Immediate
        );
        assert_eq!(
            surface_present_mode(Vsync::Uncapped, &[PresentMode::Immediate]),
            PresentMode::Immediate
        );
        assert_eq!(
            surface_present_mode(Vsync::Uncapped, &[PresentMode::Fifo]),
            PresentMode::Fifo
        );
        assert_eq!(
            surface_present_mode(
                Vsync::Uncapped,
                &[
                    PresentMode::Fifo,
                    PresentMode::Mailbox,
                    PresentMode::FifoRelaxed
                ]
            ),
            PresentMode::Fifo,
            "Mailbox/FifoRelaxed still pace to the display: not uncapped"
        );
    }

    /// The selection is PRESENTATION-BUCKET ONLY (D17 b): it lives
    /// in `WindowOptions` and never enters `ModeConfig`/`SimConfig`,
    /// so the derived host config is bit-identical and the SAME pump
    /// script yields the identical executed-tick sequence, sim tick
    /// count, state hash, scene hash AND frame parity hash through
    /// hosts built with either option — the uncapped loop may run
    /// its presents faster, but the pumps it runs are the same fixed
    /// contract they always were.
    #[test]
    fn uncapped_selection_never_touches_the_hashed_trajectory() {
        let locked_opts = WindowOptions::new("test-install");
        let mut uncapped_opts = WindowOptions::new("test-install");
        assert_eq!(locked_opts.mode, uncapped_opts.mode, "same modern mode");
        uncapped_opts.vsync = Vsync::Uncapped;
        assert_eq!(
            host_sim_config(&locked_opts),
            host_sim_config(&uncapped_opts),
            "the option never reaches the sim config"
        );

        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut locked = host_for(&locked_opts);
        let mut uncapped = host_for(&uncapped_opts);
        let mut executed_locked = Vec::new();
        let mut executed_uncapped = Vec::new();
        for (i, &dt) in script.iter().cycle().take(30).enumerate() {
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
            executed_locked.push(locked.pump_frame(dt, &input));
            executed_uncapped.push(uncapped.pump_frame(dt, &input));
        }
        assert_eq!(executed_locked, executed_uncapped);
        assert_eq!(
            locked.driver().sim().tick_index(),
            uncapped.driver().sim().tick_index()
        );
        assert_eq!(
            locked.driver().sim().state_hash(),
            uncapped.driver().sim().state_hash()
        );
        assert_eq!(locked.scene_hash(), uncapped.scene_hash());
        assert_eq!(locked.frame().parity_hash(), uncapped.frame().parity_hash());
    }

    /// GATE ANSWERS are option-invariant: the vsync selection lives
    /// upstream of the host (never in `ModeConfig`), so both arms'
    /// present-gate and alpha answers are IDENTICAL under either
    /// option — the uncapped loop runs the same presents more often;
    /// it never changes WHAT the gate answers.
    #[test]
    fn vsync_option_never_changes_the_gate_answers() {
        for mode in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let mut locked = WindowOptions::new("t");
            let mut uncapped = WindowOptions::new("t");
            locked.mode = mode;
            uncapped.mode = mode;
            uncapped.vsync = Vsync::Uncapped;
            let mut a = host_for(&locked);
            let mut b = host_for(&uncapped);
            let mut clock = FixedStepClock::host();
            for dt in [4u32, 0, 4, 3, 0, 4] {
                clock.advance(4_166_666);
                a.pump_frame(dt, &InputFrame::default());
                b.pump_frame(dt, &InputFrame::default());
                assert_eq!(present_due(&a), present_due(&b), "{mode:?} dt {dt}");
                assert_eq!(
                    present_camera_alpha(&a, &clock),
                    present_camera_alpha(&b, &clock),
                    "{mode:?} dt {dt}"
                );
            }
        }
    }

    /// THE UNCAPPED LOOP SHAPE (hermetic simulation of the free-run
    /// cycle): the gate stays open every iteration (the Decoupled
    /// arm presents every host frame), the alpha is the clock
    /// fraction each time, and the rapid-fire presents are COHERENT
    /// BY CONSTRUCTION — `recompose` always re-renders from LATEST
    /// state, so repeated presents at a fixed fraction are
    /// idempotent and a burst of them never accumulates drift —
    /// while the hashed buckets stay byte-frozen through the whole
    /// burst.
    #[test]
    fn uncapped_presents_are_coherent_and_drift_free() {
        let mut opts = WindowOptions::new("test-install");
        opts.vsync = Vsync::Uncapped;
        let mut host = host_for(&opts);
        // Stage the interpolation endpoint: one full pump.
        assert_eq!(
            host.pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default()),
            1
        );
        let state0 = host.driver().sim().state_hash();
        let scene0 = host.scene_hash();
        let ticks0 = host.driver().sim().tick_index();

        let mut clock = FixedStepClock::host();
        // An uncapped iteration cadence far above the tick rate (a
        // 1000 Hz-class loop: 1 ms deltas).
        for _ in 0..12 {
            assert_eq!(
                clock.advance(1_000_000),
                0,
                "no whole pump per uncapped iteration here"
            );
            assert!(
                present_due(&host),
                "uncapped gate: every iteration presents"
            );
            let alpha = present_camera_alpha(&host, &clock).expect("modern arm interpolates");
            assert!((0.0..=1.0).contains(&alpha));
            assert!(host.recompose(alpha), "endpoint stays staged");
        }
        // The burst never touched anything hashed.
        assert_eq!(host.driver().sim().state_hash(), state0);
        assert_eq!(host.scene_hash(), scene0);
        assert_eq!(host.driver().sim().tick_index(), ticks0);
        // Idempotent at a fixed fraction and drift-free across
        // fractions: recompose re-renders from latest state, so the
        // frame depends on the CURRENT fraction alone, never on how
        // many presents preceded it.
        host.recompose(0.5);
        let once = host.frame().parity_hash();
        host.recompose(0.5);
        host.recompose(0.5);
        assert_eq!(
            host.frame().parity_hash(),
            once,
            "idempotent at a fixed alpha"
        );
        host.recompose(0.75);
        let direct = host.frame().parity_hash();
        host.recompose(0.25);
        host.recompose(0.75);
        assert_eq!(
            host.frame().parity_hash(),
            direct,
            "drift-free: present history never enters the frame"
        );
    }
}

#[cfg(test)]
mod window_mode_tests {
    //! The P6 window-modes pins (`p6-window-modes`, PLAN §6 QoL
    //! "window modes"): a PLATFORM presentation option (D200
    //! layering — OUT of ModeConfig) with NO purist arbitration
    //! (the original was a fullscreen DOS exclusive with no
    //! windowed mode to preserve, so both pacing arms accept the
    //! selection identically and it selects NOTHING in the host).
    //! The winit fullscreen target is a PURE function over plain
    //! data — hermetic, no window needed — and the F11 toggle is a
    //! platform key OUTSIDE both control schemes, never reaching
    //! ShellInput. The shell fixed-step clock/pump contract and the
    //! hashed trajectory stay untouched. Test surface = the ONE
    //! selection, all three shapes (both mode arms appear only in
    //! the option-invariant controls), never the feature
    //! cross-product; the catalog stays EMPTY (a plan-named QoL
    //! unit is not a catalog entry).
    use super::*;
    use crate::input::{Bindings, ControlScheme};
    use bedlam_core::input::InputFrame;
    use bedlam_core::mode::ModeConfig;
    use winit::keyboard::KeyCode;

    fn opts_with(window_mode: WindowMode) -> WindowOptions {
        let mut opts = WindowOptions::new("test-install");
        opts.window_mode = window_mode;
        opts
    }

    fn host_for(opts: &WindowOptions) -> GameHost {
        GameHost::new(
            &GameConfig::default(),
            &host_sim_config(opts),
            [[0u8, 0, 0]; 256],
        )
    }

    fn mode_1920x1080_60_32() -> VideoModeChoice {
        VideoModeChoice {
            width: 1920,
            height: 1080,
            refresh_millihertz: 60_000,
            bit_depth: 32,
        }
    }

    /// A plausible monitor list: several sizes/refreshes/depths.
    fn monitor_choices() -> Vec<VideoModeChoice> {
        vec![
            VideoModeChoice {
                width: 1280,
                height: 1024,
                refresh_millihertz: 75_000,
                bit_depth: 32,
            },
            VideoModeChoice {
                width: 1920,
                height: 1080,
                refresh_millihertz: 60_000,
                bit_depth: 24,
            },
            mode_1920x1080_60_32(),
            VideoModeChoice {
                width: 1920,
                height: 1080,
                refresh_millihertz: 144_000,
                bit_depth: 32,
            },
        ]
    }

    /// The shipped default: the window-mode option starts WINDOWED —
    /// the decorated window at the configured inner size exactly as
    /// before this unit (the option changes nothing until asked),
    /// and WINDOWED never requests a fullscreen target even on a
    /// monitor with modes to offer.
    #[test]
    fn window_mode_defaults_to_the_shipped_windowed() {
        assert_eq!(WindowMode::default(), WindowMode::Windowed);
        assert_eq!(
            WindowOptions::new("test-install").window_mode,
            WindowMode::Windowed
        );
        assert_eq!(
            fullscreen_target(WindowMode::Windowed, &monitor_choices()),
            None
        );
    }

    /// THE PURE TARGET MAPPING (hermetic, no window): Windowed ->
    /// None; Borderless -> Borderless regardless of candidates (no
    /// mode switch is involved); Fullscreen -> the best-effort
    /// exclusive pick, degrading HONESTLY to borderless when the
    /// monitor offers no modes (or is absent — the empty list).
    /// No purist arbitration: the answer is independent of
    /// ModeConfig by construction (the function never sees one).
    #[test]
    fn fullscreen_target_maps_the_selection_purely() {
        let choices = monitor_choices();
        assert_eq!(fullscreen_target(WindowMode::Windowed, &choices), None);
        for candidates in [&choices[..], &[][..]] {
            assert_eq!(
                fullscreen_target(WindowMode::Borderless, candidates),
                Some(FullscreenTarget::Borderless),
                "borderless needs no video mode"
            );
        }
        assert_eq!(
            fullscreen_target(WindowMode::Fullscreen, &choices),
            Some(FullscreenTarget::Exclusive(VideoModeChoice {
                width: 1920,
                height: 1080,
                refresh_millihertz: 144_000,
                bit_depth: 32
            }))
        );
        assert_eq!(
            fullscreen_target(WindowMode::Fullscreen, &[]),
            Some(FullscreenTarget::Borderless),
            "honest degradation, never fatal"
        );
    }

    /// The BEST-EFFORT exclusive pick: largest area, then highest
    /// refresh, then highest bit depth — and a TOTAL order, so the
    /// pick is independent of the candidate list order (every
    /// permutation picks the same mode).
    #[test]
    fn exclusive_pick_is_largest_area_then_refresh_then_depth() {
        let best = VideoModeChoice {
            width: 2560,
            height: 1440,
            refresh_millihertz: 60_000,
            bit_depth: 24,
        };
        let mut choices = monitor_choices();
        choices.push(best);
        assert_eq!(pick_exclusive_mode(&choices), Some(best));
        // Refresh breaks an area tie.
        assert_eq!(
            pick_exclusive_mode(&[
                VideoModeChoice {
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: 60_000,
                    bit_depth: 32
                },
                VideoModeChoice {
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: 144_000,
                    bit_depth: 24
                },
            ]),
            Some(VideoModeChoice {
                width: 1920,
                height: 1080,
                refresh_millihertz: 144_000,
                bit_depth: 24
            })
        );
        // Depth breaks an area+refresh tie.
        assert_eq!(
            pick_exclusive_mode(&[
                mode_1920x1080_60_32(),
                VideoModeChoice {
                    width: 1920,
                    height: 1080,
                    refresh_millihertz: 60_000,
                    bit_depth: 24
                }
            ]),
            Some(mode_1920x1080_60_32())
        );
        // Order independence: every permutation of the list picks
        // the same mode (a pure selection must not see list order).
        let mut shuffled = choices.clone();
        shuffled.rotate_left(2);
        assert_eq!(
            pick_exclusive_mode(&shuffled),
            pick_exclusive_mode(&choices)
        );
        assert_eq!(pick_exclusive_mode(&[]), None);
    }

    /// The selection is PRESENTATION-BUCKET ONLY (D17 b, the
    /// no-arbitration pin): it lives in `WindowOptions` and never
    /// enters `ModeConfig`/`SimConfig`, so the derived host config
    /// is bit-identical and the SAME pump script through hosts
    /// built under every window mode yields the identical
    /// executed-tick sequence, sim tick count, state hash, scene
    /// hash AND frame parity hash — window chrome cannot touch the
    /// hashed trajectory.
    #[test]
    fn window_mode_selection_never_touches_the_sim_or_the_hashed_trajectory() {
        let windowed = opts_with(WindowMode::Windowed);
        let borderless = opts_with(WindowMode::Borderless);
        let fullscreen = opts_with(WindowMode::Fullscreen);
        assert_eq!(host_sim_config(&windowed), host_sim_config(&borderless));
        assert_eq!(
            host_sim_config(&windowed),
            host_sim_config(&fullscreen),
            "the selection never reaches the sim config"
        );

        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut hosts = [
            host_for(&windowed),
            host_for(&borderless),
            host_for(&fullscreen),
        ];
        let mut executed = [Vec::new(), Vec::new(), Vec::new()];
        for (i, &dt) in script.iter().cycle().take(30).enumerate() {
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
            for (host, log) in hosts.iter_mut().zip(executed.iter_mut()) {
                log.push(host.pump_frame(dt, &input));
            }
        }
        for host in &hosts[1..] {
            assert_eq!(executed[1], executed[0], "same executed ticks");
            assert_eq!(
                host.driver().sim().tick_index(),
                hosts[0].driver().sim().tick_index()
            );
            assert_eq!(
                host.driver().sim().state_hash(),
                hosts[0].driver().sim().state_hash()
            );
            assert_eq!(host.scene_hash(), hosts[0].scene_hash());
            assert_eq!(host.frame().parity_hash(), hosts[0].frame().parity_hash());
        }
    }

    /// GATE ANSWERS are option-invariant: the window-mode selection
    /// lives entirely upstream of the host (never in
    /// `ModeConfig`), so both pacing arms' present-gate and alpha
    /// answers are IDENTICAL under every window mode — window
    /// chrome decides where the image appears, never WHAT the gate
    /// answers.
    #[test]
    fn window_mode_option_never_changes_the_gate_answers() {
        for mode in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let opts = [
                WindowMode::Windowed,
                WindowMode::Borderless,
                WindowMode::Fullscreen,
            ]
            .map(|wm| {
                let mut o = opts_with(wm);
                o.mode = mode;
                o
            });
            let mut hosts = [host_for(&opts[0]), host_for(&opts[1]), host_for(&opts[2])];
            let mut clock = FixedStepClock::host();
            for dt in [4u32, 0, 4, 3, 0, 4] {
                clock.advance(4_166_666);
                for host in &mut hosts {
                    host.pump_frame(dt, &InputFrame::default());
                }
                assert_eq!(
                    present_due(&hosts[1]),
                    present_due(&hosts[0]),
                    "{mode:?} dt {dt}"
                );
                assert_eq!(
                    present_due(&hosts[2]),
                    present_due(&hosts[0]),
                    "{mode:?} dt {dt}"
                );
                assert_eq!(
                    present_camera_alpha(&hosts[1], &clock),
                    present_camera_alpha(&hosts[0], &clock),
                    "{mode:?} dt {dt}"
                );
            }
        }
    }

    /// THE F11 TOGGLE TRANSITION (pure): leaving fullscreen always
    /// returns to windowed (`None`, from every selection); entering
    /// uses the selection's PREFERRED shape — a WINDOWED selection
    /// enters BORDERLESS (the desktop F11 convention: the toggle
    /// must do something sensible from every selection), BORDERLESS
    /// re-enters borderless, FULLSCREEN enters its best-effort
    /// exclusive shape (degrading honestly on an empty list).
    #[test]
    fn toggle_target_enters_the_preferred_shape_and_leaves_to_windowed() {
        let choices = monitor_choices();
        for mode in [
            WindowMode::Windowed,
            WindowMode::Borderless,
            WindowMode::Fullscreen,
        ] {
            assert_eq!(
                toggle_fullscreen_target(mode, &choices, true),
                None,
                "{mode:?}: leaving always returns to windowed"
            );
        }
        assert_eq!(
            toggle_fullscreen_target(WindowMode::Windowed, &choices, false),
            Some(FullscreenTarget::Borderless)
        );
        assert_eq!(
            toggle_fullscreen_target(WindowMode::Borderless, &choices, false),
            Some(FullscreenTarget::Borderless)
        );
        assert_eq!(
            toggle_fullscreen_target(WindowMode::Fullscreen, &choices, false),
            fullscreen_target(WindowMode::Fullscreen, &choices)
        );
        // The degradation composes into the toggle too.
        assert_eq!(
            toggle_fullscreen_target(WindowMode::Fullscreen, &[], false),
            Some(FullscreenTarget::Borderless)
        );
    }

    /// F11 is the ONLY platform toggle key, and it is DEAD to both
    /// control schemes: the handler intercepts it before the
    /// mapper (never reaching ShellInput), and this pin closes the
    /// other direction too — even if it were forwarded, the modern
    /// default Bindings table binds it to nothing and the classic
    /// fixed table (the original EXW scheme, RE-EXW-INPUT §6:
    /// keyboard = hotkeys/volume/pause/any-key only) maps nothing
    /// at F11 — the toggle can never become sim input.
    #[test]
    fn f11_is_the_only_platform_toggle_key_and_is_dead_to_both_schemes() {
        assert!(is_window_toggle_key(PhysicalKey::Code(KeyCode::F11)));
        for key in [
            KeyCode::F10,
            KeyCode::F12,
            KeyCode::Escape,
            KeyCode::KeyW,
            KeyCode::AltLeft,
        ] {
            assert!(!is_window_toggle_key(PhysicalKey::Code(key)), "{key:?}");
        }
        let f11 = PhysicalKey::Code(KeyCode::F11);
        assert_eq!(Bindings::modern_default().get(f11), None);
        assert_eq!(
            ControlScheme::Modern.map_key(f11, &Bindings::modern_default()),
            None
        );
        assert_eq!(
            ControlScheme::Classic.map_key(f11, &Bindings::modern_default()),
            None,
            "classic ignores the table and the original binds no F11"
        );
    }

    /// THE P6 QoL VOLUME-KEY SET (D212): four keys and nothing else,
    /// all PLATFORM-ONLY — the handler intercepts them before the
    /// mapper, so they never reach ShellInput, and this pin shows
    /// all four map to nothing in either scheme (the F11 posture),
    /// so even a forwarding bug could not make them sim input. The
    /// ORIGINAL volume keys (Up/Down arrows, RE-EXW-INPUT sec 5)
    /// are deliberately NOT platform keys — they are scheme keys in
    /// the original and stay there.
    #[test]
    fn volume_keys_are_platform_only_and_dead_to_both_schemes() {
        use winit::keyboard::KeyCode as K;
        let cases = [
            (K::PageUp, VolumeAdjust::MusicUp),
            (K::PageDown, VolumeAdjust::MusicDown),
            (K::BracketRight, VolumeAdjust::SfxUp),
            (K::BracketLeft, VolumeAdjust::SfxDown),
        ];
        for (code, adj) in cases {
            let key = PhysicalKey::Code(code);
            assert_eq!(volume_adjust_key(key), Some(adj), "{code:?}");
            assert_eq!(Bindings::modern_default().get(key), None, "{code:?}");
            assert_eq!(
                ControlScheme::Modern.map_key(key, &Bindings::modern_default()),
                None,
                "{code:?} unbound in the modern default table"
            );
            assert_eq!(
                ControlScheme::Classic.map_key(key, &Bindings::modern_default()),
                None,
                "classic ignores the table and the original binds no {code:?}"
            );
        }
        for key in [
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::F11,
            KeyCode::Escape,
            KeyCode::KeyW,
        ] {
            assert_eq!(
                volume_adjust_key(PhysicalKey::Code(key)),
                None,
                "{key:?} is not a volume key"
            );
        }
    }

    /// The bounded runtime stepping: the ORIGINAL'S OWN ±5 step on a
    /// 0..=100 clamp (RE-EXW-INPUT sec 5: the EXW mission-shell
    /// stepper moves g_music_volume by ±5, clamped), buses
    /// independent, value semantics (a step returns a new
    /// selection, never mutates).
    #[test]
    fn volume_stepping_is_the_original_step_and_clamp() {
        assert_eq!(VolumeAdjust::MusicUp.step(), 5);
        assert_eq!(VolumeAdjust::MusicDown.step(), -5);
        let shipped = VolumeMixers::SHIPPED;
        let down = volume_mixers_stepped(shipped, VolumeAdjust::MusicDown);
        assert_eq!(down.music().percent(), 95);
        assert_eq!(down.sfx().percent(), 100, "buses step independently");
        assert_eq!(shipped.music().percent(), 100, "value semantics");
        // Clamp at both ends: 100 -> (21 downs) -> 0 -> stays 0.
        let muted = (0..21).fold(shipped, |m, _| {
            volume_mixers_stepped(m, VolumeAdjust::MusicDown)
        });
        assert_eq!(muted.music().percent(), 0);
        let still = volume_mixers_stepped(muted, VolumeAdjust::MusicDown);
        assert_eq!(still.music().percent(), 0, "clamped at 0");
        let top = volume_mixers_stepped(muted, VolumeAdjust::MusicUp);
        assert_eq!(top.music().percent(), 5);
        // SFX arms symmetric, music untouched.
        let sfx = volume_mixers_stepped(shipped, VolumeAdjust::SfxDown);
        assert_eq!(sfx.sfx().percent(), 95);
        assert_eq!(sfx.music().percent(), 100);
    }

    /// The platform default is the SHIPPED MIX exactly, and the
    /// selection selects NOTHING in the sim (D212 bounds, the
    /// D210 no-arbitration shape): the derived SimConfig is
    /// bit-identical under any volume selection — the knob lives
    /// only in the shell audio path (pinned feed-side in
    /// `volume_mixers_never_touch_the_engine_stream`).
    #[test]
    fn volume_selection_never_touches_the_sim_config() {
        let dir = std::path::PathBuf::from("gfx");
        let mut opts = WindowOptions::new(&dir);
        assert_eq!(opts.volume, VolumeMixers::SHIPPED);
        let baseline = host_sim_config(&opts);
        opts.volume = VolumeMixers::new(0, 0);
        assert_eq!(
            host_sim_config(&opts),
            baseline,
            "the volume selection never reaches the sim config"
        );
        assert_eq!(opts.volume.stream_gain_q8(), 0);
    }

    #[test]
    fn save_surface_never_touches_the_sim_config() {
        // D213: the save-slot selection + the opt-in autosave policy
        // are PLATFORM knobs OUT of ModeConfig (D200) — the knobs
        // never enter the derived SimConfig under ANY setting, and
        // the shipped defaults are the FIRST slot + the OFF policy
        // (RE-EXW-SAVE sec 4: the shipped game never autosaves).
        let dir = std::path::PathBuf::from("gfx");
        let mut opts = WindowOptions::new(&dir);
        assert!(crate::save::window_save_surface_is_shipped(&opts));
        let baseline = host_sim_config(&opts);
        opts.save_slot = SaveSlotId::LAST;
        opts.autosave = AutosavePolicy::On(SaveSlotId::LAST);
        assert_eq!(
            host_sim_config(&opts),
            baseline,
            "the save surface never reaches the sim config"
        );
        assert_eq!(
            baseline,
            SimConfig {
                mode: opts.mode,
                ..SimConfig::default()
            }
        );
    }

    #[test]
    fn cdda_surface_never_touches_the_sim_config() {
        // D223: the CDDA user-supply + local-cache selection is a
        // PLATFORM knob OUT of ModeConfig (D200) — the music path is
        // presentation bucket (D17 b: audio never enters a hash), so
        // the knob never enters the derived SimConfig under ANY
        // setting, and the default is the enabled cache with no
        // search-dir override (the plan's generated-on-first-run
        // posture over the documented lookup).
        let dir = std::path::PathBuf::from("gfx");
        let mut opts = WindowOptions::new(&dir);
        assert_eq!(opts.music, CddaOptions::default());
        let baseline = host_sim_config(&opts);
        opts.music.search_dir = Some(std::path::PathBuf::from("/user/music"));
        opts.music.cache = crate::cdda::MusicCachePolicy::Disabled;
        assert_eq!(
            host_sim_config(&opts),
            baseline,
            "the CDDA surface never reaches the sim config"
        );
        assert_eq!(
            baseline,
            SimConfig {
                mode: opts.mode,
                ..SimConfig::default()
            }
        );
    }
}

#[cfg(test)]
mod scaling_option_tests {
    //! The p6-scaling-options unit (D215): the SCALING SELECTION —
    //! the already-landed bedlam-platform ScaleMode/FilterMode
    //! exposed as a platform presentation knob riding
    //! WindowOptions::present. Every pin below is the
    //! no-arbitration posture: a PURE mapping over plain data that
    //! selects nothing in the host beyond the PresentConfig the GPU
    //! scale path consumes (a plan-named resolution unit is not a
    //! catalog entry).
    use super::*;
    use bedlam_core::input::InputFrame;
    use bedlam_core::mode::ModeConfig;

    fn opts_with(present: PresentConfig) -> WindowOptions {
        let mut opts = WindowOptions::new("test-install");
        opts.present = present;
        opts
    }

    fn host_for(opts: &WindowOptions) -> GameHost {
        GameHost::new(
            &GameConfig::default(),
            &host_sim_config(opts),
            [[0u8, 0, 0]; 256],
        )
    }

    /// The full 4x2 selection cross product through the composed
    /// PURE mapping (the binary's only route into `present`).
    fn selections() -> Vec<PresentConfig> {
        [
            ScaleMode::Integer,
            ScaleMode::Fit,
            ScaleMode::Fill,
            ScaleMode::Stretch,
        ]
        .into_iter()
        .flat_map(|scale| {
            [FilterMode::Nearest, FilterMode::Linear]
                .into_iter()
                .map(move |filter| scaling_present_config(scale, filter))
        })
        .collect()
    }

    /// The shipped default: Integer + Nearest — the option changes
    /// NOTHING until asked (the parity blit runs the exact config it
    /// always has), and defaults through the composed mapping give
    /// `PresentConfig::default()` bit-for-bit.
    #[test]
    fn scaling_defaults_to_the_shipped_integer_nearest() {
        assert_eq!(PresentConfig::default().scale, ScaleMode::Integer);
        assert_eq!(PresentConfig::default().filter, FilterMode::Nearest);
        assert_eq!(
            WindowOptions::new("test-install").present,
            PresentConfig::default()
        );
        assert_eq!(
            scaling_present_config(ScaleMode::default(), FilterMode::default()),
            PresentConfig::default()
        );
    }

    /// The CLI words cover the full domain and FAIL CLOSED: every
    /// selection word maps to exactly one mode, anything else is
    /// `None` (the binary exits 2 — a presentation knob never
    /// guesses).
    #[test]
    fn scaling_cli_words_map_the_full_domain_and_fail_closed() {
        assert_eq!(scale_mode_from_cli("integer"), Some(ScaleMode::Integer));
        assert_eq!(scale_mode_from_cli("fit"), Some(ScaleMode::Fit));
        assert_eq!(scale_mode_from_cli("fill"), Some(ScaleMode::Fill));
        assert_eq!(scale_mode_from_cli("stretch"), Some(ScaleMode::Stretch));
        assert_eq!(filter_mode_from_cli("nearest"), Some(FilterMode::Nearest));
        assert_eq!(filter_mode_from_cli("linear"), Some(FilterMode::Linear));
        for word in ["smooth", "INTEGER", "int", "", "linear ", "0", "Stretch"] {
            assert_eq!(scale_mode_from_cli(word), None, "scale word {word:?}");
        }
        for word in ["smooth", "bilinear", "NEAREST", "", "nearest;"] {
            assert_eq!(filter_mode_from_cli(word), None, "filter word {word:?}");
        }
    }

    /// THE PURE MAPPING: the composed config covers exactly the 4x2
    /// selection cross product and touches ONLY the two knob fields
    /// — the 6-to-8 bit palette expansion stays the parity
    /// `VgaExpand::Original` under every selection, so the canonical
    /// 640x480 indexed frame + palette ride unchanged.
    #[test]
    fn scaling_selection_is_a_pure_present_config_mapping() {
        for scale in [
            ScaleMode::Integer,
            ScaleMode::Fit,
            ScaleMode::Fill,
            ScaleMode::Stretch,
        ] {
            for filter in [FilterMode::Nearest, FilterMode::Linear] {
                let cfg = scaling_present_config(scale, filter);
                assert_eq!(cfg.scale, scale);
                assert_eq!(cfg.filter, filter);
                assert_eq!(
                    cfg.expand,
                    VgaExpand::Original,
                    "the expansion policy is not a knob"
                );
            }
        }
        for cfg in selections() {
            assert_eq!(cfg.expand, VgaExpand::Original);
        }
    }

    /// The selection is PRESENTATION-BUCKET ONLY (D17 b, the
    /// no-arbitration pin): it lives in `WindowOptions` and never
    /// enters `ModeConfig`/`SimConfig`, so the derived host config
    /// is bit-identical and the SAME pump script through hosts
    /// built under every selection yields the identical
    /// executed-tick sequence, sim tick count, state hash, scene
    /// hash AND frame parity hash — the scaling cannot touch the
    /// hashed trajectory (goldens stay resolution-agnostic).
    #[test]
    fn scaling_selection_never_touches_the_sim_or_the_hashed_trajectory() {
        let opts_all = selections().into_iter().map(opts_with).collect::<Vec<_>>();
        for opts in &opts_all[1..] {
            assert_eq!(
                host_sim_config(&opts_all[0]),
                host_sim_config(opts),
                "the selection never reaches the sim config"
            );
        }

        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
        let mut executed = vec![Vec::new(); hosts.len()];
        for (i, &dt) in script.iter().cycle().take(30).enumerate() {
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
            for (host, log) in hosts.iter_mut().zip(executed.iter_mut()) {
                log.push(host.pump_frame(dt, &input));
            }
        }
        for (i, host) in hosts.iter().enumerate().skip(1) {
            assert_eq!(executed[i], executed[0], "same executed ticks");
            assert_eq!(
                host.driver().sim().tick_index(),
                hosts[0].driver().sim().tick_index()
            );
            assert_eq!(
                host.driver().sim().state_hash(),
                hosts[0].driver().sim().state_hash()
            );
            assert_eq!(host.scene_hash(), hosts[0].scene_hash());
            assert_eq!(host.frame().parity_hash(), hosts[0].frame().parity_hash());
        }
    }

    /// GATE ANSWERS are option-invariant: the scaling selection
    /// lives entirely upstream of the host (never in
    /// `ModeConfig`), so BOTH pacing arms accept it identically —
    /// the present-gate and alpha answers are IDENTICAL under every
    /// selection in the modern AND the classic arm.
    #[test]
    fn scaling_option_never_changes_the_gate_answers() {
        for mode in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let mut opts_all = selections().into_iter().map(opts_with).collect::<Vec<_>>();
            for opts in &mut opts_all {
                opts.mode = mode;
            }
            let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
            let mut clock = FixedStepClock::host();
            for dt in [4u32, 0, 4, 3, 0, 4] {
                clock.advance(4_166_666);
                for host in &mut hosts {
                    host.pump_frame(dt, &InputFrame::default());
                }
                for (i, host) in hosts.iter().enumerate().skip(1) {
                    assert_eq!(
                        present_due(host),
                        present_due(&hosts[0]),
                        "{mode:?} dt {dt} selection {i}"
                    );
                    assert_eq!(
                        present_camera_alpha(host, &clock),
                        present_camera_alpha(&hosts[0], &clock),
                        "{mode:?} dt {dt} selection {i}"
                    );
                }
            }
        }
    }

    /// The FILL cursor-uv handling already exists window-side (the
    /// parity blit crops the centered source sub-rect — its inverse
    /// needs the uv rect, so absolute cursor mapping is unavailable
    /// under Fill and relative aiming is used instead): pin it over
    /// the whole selection — None under Fill regardless of filter,
    /// an exact absolute mapping under Integer/Fit/Stretch (Stretch
    /// maps the whole frame onto the whole target, so it inverts
    /// absolutely like the bar modes).
    #[test]
    fn fill_scaling_cursor_is_relative_only_and_filter_invariant() {
        for cfg in selections() {
            match cfg.scale {
                ScaleMode::Fill => {
                    assert_eq!(
                        cursor_to_game(960.0, 540.0, 1920, 1080, &cfg),
                        None,
                        "Fill crops: no absolute cursor mapping"
                    );
                }
                _ => {
                    assert_eq!(
                        cursor_to_game(960.0, 540.0, 1920, 1080, &cfg),
                        Some((320, 240)),
                        "integer/fit/stretch map absolutely (the filter never matters)"
                    );
                }
            }
        }
    }

    /// The P7 SteamDeck PLATFORM PROFILE is PRESENTATION-BUCKET
    /// ONLY (D224, the queue's invariance pin over the profile
    /// selection): the startup mapping (profile class + CLI word ->
    /// default scale -> [`scaling_present_config`]) lives entirely
    /// upstream of the host, so the derived host config is
    /// bit-identical and the SAME pump script through hosts built
    /// under the Generic default, the SteamDeck default AND the
    /// CLI-override combinations yields the identical
    /// executed-tick sequence, sim tick count, state hash, scene
    /// hash AND frame parity hash — the profile cannot touch the
    /// hashed trajectory.
    #[test]
    fn profile_selection_never_touches_the_sim_or_the_hashed_trajectory() {
        use crate::platform::{profile_default_scale, startup_scale_selection, PlatformClass};
        // Every class x CLI-word combination the startup can compose.
        let cases: Vec<ScaleMode> = [
            (PlatformClass::Generic, None),
            (PlatformClass::SteamDeck, None),
            (PlatformClass::SteamDeck, Some(ScaleMode::Integer)),
            (PlatformClass::SteamDeck, Some(ScaleMode::Fit)),
            (PlatformClass::SteamDeck, Some(ScaleMode::Fill)),
            (PlatformClass::Generic, Some(ScaleMode::Stretch)),
        ]
        .into_iter()
        .map(|(class, cli)| startup_scale_selection(class, cli))
        .collect();
        // The profile defaults are exactly the two pinned arms.
        assert_eq!(
            profile_default_scale(PlatformClass::Generic),
            cases[0],
            "generic default stays Integer"
        );
        assert_eq!(
            profile_default_scale(PlatformClass::SteamDeck),
            cases[1],
            "steamdeck default is the fill-the-panel stretch"
        );

        let opts_all = cases
            .into_iter()
            .map(|scale| {
                let mut opts = WindowOptions::new("test-install");
                opts.present = scaling_present_config(scale, FilterMode::default());
                opts
            })
            .collect::<Vec<_>>();
        for opts in &opts_all[1..] {
            assert_eq!(
                host_sim_config(&opts_all[0]),
                host_sim_config(opts),
                "the profile never reaches the sim config"
            );
        }

        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
        let mut executed = vec![Vec::new(); hosts.len()];
        for (i, &dt) in script.iter().cycle().take(30).enumerate() {
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
            for (host, log) in hosts.iter_mut().zip(executed.iter_mut()) {
                log.push(host.pump_frame(dt, &input));
            }
        }
        for (i, host) in hosts.iter().enumerate().skip(1) {
            assert_eq!(executed[i], executed[0], "same executed ticks");
            assert_eq!(
                host.driver().sim().tick_index(),
                hosts[0].driver().sim().tick_index()
            );
            assert_eq!(
                host.driver().sim().state_hash(),
                hosts[0].driver().sim().state_hash()
            );
            assert_eq!(host.scene_hash(), hosts[0].scene_hash());
            assert_eq!(host.frame().parity_hash(), hosts[0].frame().parity_hash());
        }
    }

    /// PROFILE GATE ANSWERS are option-invariant (the D200 pin, the
    /// queue's "both pacing arms accept it" requirement): the
    /// profile-derived present defaults live entirely upstream of
    /// the host, so the present-gate and alpha answers are
    /// IDENTICAL under the Generic and SteamDeck defaults in the
    /// modern AND the classic arm.
    #[test]
    fn profile_selection_never_changes_the_gate_answers() {
        use crate::platform::{startup_scale_selection, PlatformClass};
        for mode in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let opts_all = [
                startup_scale_selection(PlatformClass::Generic, None),
                startup_scale_selection(PlatformClass::SteamDeck, None),
                startup_scale_selection(PlatformClass::SteamDeck, Some(ScaleMode::Integer)),
            ]
            .into_iter()
            .map(|scale| {
                let mut opts = WindowOptions::new("test-install");
                opts.present = scaling_present_config(scale, FilterMode::default());
                opts.mode = mode;
                opts
            })
            .collect::<Vec<_>>();
            let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
            let mut clock = FixedStepClock::host();
            for dt in [4u32, 0, 4, 3, 0, 4] {
                clock.advance(4_166_666);
                for host in &mut hosts {
                    host.pump_frame(dt, &InputFrame::default());
                }
                for (i, host) in hosts.iter().enumerate().skip(1) {
                    assert_eq!(
                        present_due(host),
                        present_due(&hosts[0]),
                        "{mode:?} dt {dt} profile case {i}"
                    );
                    assert_eq!(
                        present_camera_alpha(host, &clock),
                        present_camera_alpha(&hosts[0], &clock),
                        "{mode:?} dt {dt} profile case {i}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod cursor_tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn cursor_to_game_exact_rects() {
        let cfg = PresentConfig::default(); // Integer + Nearest
                                            // Exact 2x window: rect = 1280x960 at (0,0).
        assert_eq!(
            cursor_to_game(640.0, 480.0, 1280, 960, &cfg),
            Some((320, 240))
        );
        assert_eq!(cursor_to_game(0.0, 0.0, 1280, 960, &cfg), Some((0, 0)));
        // 1920x1080, Integer: sx=3, sy=2 -> s=2 -> rect 1280x960 at (320,60).
        assert_eq!(cursor_to_game(320.0, 60.0, 1920, 1080, &cfg), Some((0, 0)));
        assert_eq!(
            cursor_to_game(960.0, 540.0, 1920, 1080, &cfg),
            Some((320, 240))
        );
        // Bars clamp to the frame edge, never negative.
        assert_eq!(cursor_to_game(0.0, 0.0, 1920, 1080, &cfg), Some((0, 0)));
        assert_eq!(
            cursor_to_game(1919.0, 1079.0, 1920, 1080, &cfg),
            Some((639, 479))
        );
        // Degenerate: window smaller than the frame -> None.
        assert_eq!(cursor_to_game(100.0, 100.0, 320, 200, &cfg), None);
    }

    #[test]
    fn steering_delta_snaps_menu_cursor_to_target() {
        let (mx, my) = (11, 22);
        let target = (400, 300);
        let dx = (target.0 - mx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let dy = (target.1 - my).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let moved = (
            (mx + i32::from(dx)).clamp(0, 639),
            (my + i32::from(dy)).clamp(0, 479),
        );
        assert_eq!(moved, target);
    }
}

#[cfg(test)]
mod enhanced_native_tests {
    //! The p6-enhanced-native-render unit (D217): the ENHANCED
    //! presentation-mode selection, the responsive layout contract,
    //! and the first native pass's bounds. Every pin below is the
    //! no-arbitration posture (the D215 shape): a platform
    //! presentation knob OUT of ModeConfig that selects nothing in
    //! the host — the canonical 640x480 indexed frame + palette and
    //! every hash are byte-identical under either selection (a
    //! plan-named presentation unit is not a catalog entry).
    use super::*;
    use bedlam_core::input::InputFrame;
    use bedlam_core::mode::ModeConfig;
    use bedlam_platform::layout::{
        authoring_master, master_safe_region, safe_region, world_margins, ResponsiveFrame,
    };

    fn opts_with(presentation: PresentationMode) -> WindowOptions {
        let mut opts = WindowOptions::new("test-install");
        opts.presentation = presentation;
        opts
    }

    fn host_for(opts: &WindowOptions) -> GameHost {
        GameHost::new(
            &GameConfig::default(),
            &host_sim_config(opts),
            [[0u8, 0, 0]; 256],
        )
    }

    /// The shipped default: PARITY — the option changes NOTHING
    /// until asked (the parity present path runs the exact calls it
    /// always has), and every other WindowOptions field is
    /// default-identical under either selection.
    #[test]
    fn presentation_defaults_to_the_shipped_parity() {
        assert_eq!(
            WindowOptions::new("test-install").presentation,
            PresentationMode::Parity
        );
        let parity = WindowOptions::new("test-install");
        let mut enhanced = WindowOptions::new("test-install");
        enhanced.presentation = PresentationMode::Enhanced;
        // The knob selects nothing else: the derived host config is
        // the same object either way.
        assert_eq!(host_sim_config(&parity), host_sim_config(&enhanced));
        assert_eq!(parity.present, enhanced.present);
        assert_eq!(parity.mode, enhanced.mode);
    }

    /// The CLI words cover the full domain and FAIL CLOSED: every
    /// word maps to exactly one mode, anything else is None (the
    /// binary exits 2 — a presentation knob never guesses).
    #[test]
    fn presentation_cli_words_map_the_domain_and_fail_closed() {
        assert_eq!(
            presentation_mode_from_cli("parity"),
            Some(PresentationMode::Parity)
        );
        assert_eq!(
            presentation_mode_from_cli("enhanced"),
            Some(PresentationMode::Enhanced)
        );
        for word in ["native", "PARITY", "enh", "", "enhanced ", "1"] {
            assert_eq!(presentation_mode_from_cli(word), None, "word {word:?}");
        }
    }

    /// THE RESPONSIVE LAYOUT CONTRACT (PLAN §6 "16:10 authoring
    /// master with 16:9 safe region, other aspect ratios
    /// fit/letterbox/pillarbox"): the master is 16:10, its safe
    /// region the centered 16:9; EVERY target's safe region is the
    /// largest centered 16:9 (or narrower) rect that fits — 16:9
    /// full-bleed, wider pillarboxed, taller letterboxed; the world
    /// rect reuses the LANDED Fit shape inside the safe region.
    #[test]
    fn responsive_layout_pins_the_master_and_safe_regions() {
        assert_eq!(
            authoring_master(),
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1200
            }
        );
        // The 16:9 safe region inside the 16:10 master: centered
        // 1920x1080 (60px bands top and bottom).
        assert_eq!(
            master_safe_region(),
            Rect {
                x: 0,
                y: 60,
                w: 1920,
                h: 1080
            }
        );
        // 16:9 target: full bleed.
        assert_eq!(
            safe_region(1920, 1080),
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
        // 16:10 target: 16:9 letterboxed.
        assert_eq!(
            safe_region(1920, 1200),
            Rect {
                x: 0,
                y: 60,
                w: 1920,
                h: 1080
            }
        );
        // 4:3 target: 16:9 letterboxed at full width.
        assert_eq!(
            safe_region(640, 480),
            Rect {
                x: 0,
                y: 60,
                w: 640,
                h: 360
            }
        );
        // Wider than 16:9: pillarboxed.
        assert_eq!(
            safe_region(2560, 1080),
            Rect {
                x: 320,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
        // Square: 16:9 letterboxed.
        assert_eq!(
            safe_region(1000, 1000),
            Rect {
                x: 0,
                y: 219,
                w: 1000,
                h: 562
            }
        );
        // Zero-size inputs degrade to a zero rect (no draw).
        assert_eq!(safe_region(0, 480).w, 0);
        assert_eq!(safe_region(640, 0).h, 0);
        // The safe region never exceeds its target and always sits
        // inside it (letterbox/pillarbox geometry is centered).
        for (w, h) in [
            (1920u32, 1080),
            (1920, 1200),
            (640, 480),
            (2560, 1080),
            (1001, 999),
        ] {
            let r = safe_region(w, h);
            assert!(r.w <= w && r.h <= h);
            assert!(r.x + r.w <= w && r.y + r.h <= h);
            // 16:9 with floor semantics (w*9/16), never wider.
            assert_eq!(r.h, r.w * 9 / 16, "16:9 aspect ({w}x{h})");
            assert!(r.w * 9 <= r.h * 16 + 15);
        }
    }

    /// The world rect is the LANDED Fit composition inside the safe
    /// region — the existing PresentConfig shape reused verbatim
    /// (PLAN §6 "fit/letterbox/pillarbox via the existing shapes"):
    /// the whole 640x480 frame always visible, pillarboxed by the
    /// safe region on wide targets, and the frame_draw_rect
    /// decision fn answers it under Enhanced while Parity keeps the
    /// landed scale_rect answer (including the Fill crop — which
    /// the Enhanced layout never applies).
    #[test]
    fn enhanced_world_reuses_the_landed_fit_shape() {
        let frame = responsive_frame(1920, 1200);
        assert_eq!(frame.safe, master_safe_region());
        // 4:3 fits 640x480 into 1920x1080 as 1440x1080, centered.
        assert_eq!(
            frame.world,
            Rect {
                x: 240,
                y: 60,
                w: 1440,
                h: 1080
            }
        );
        let (left, right) = world_margins(&frame);
        assert_eq!(
            left,
            Rect {
                x: 0,
                y: 60,
                w: 240,
                h: 1080
            }
        );
        assert_eq!(
            right,
            Rect {
                x: 1680,
                y: 60,
                w: 240,
                h: 1080
            }
        );
        // A 4:3 target letterboxes the safe region and fits the
        // frame into it (480x360 centered, 80px bars INSIDE the
        // safe region — the safe region is 16:9, the frame 4:3, so
        // bars exist on every aspect; they are only wide enough to
        // host the strip on wide targets).
        let tall = responsive_frame(640, 480);
        assert_eq!(
            tall.world,
            Rect {
                x: 80,
                y: 60,
                w: 480,
                h: 360
            }
        );
        assert_eq!(world_margins(&tall).0.w, 80);
        // The decision fn: Enhanced answers the layout's world rect
        // whatever the scale selection (never the Fill crop);
        // Parity answers the landed scale_rect for every mode.
        let fill = scaling_present_config(ScaleMode::Fill, FilterMode::Nearest);
        assert_eq!(
            frame_draw_rect(PresentationMode::Enhanced, &fill, 1920, 1200),
            responsive_frame(1920, 1200).world,
            "the Enhanced layout never crops the frame"
        );
        for scale in [ScaleMode::Integer, ScaleMode::Fit, ScaleMode::Fill] {
            let cfg = scaling_present_config(scale, FilterMode::Nearest);
            assert_eq!(
                frame_draw_rect(PresentationMode::Parity, &cfg, 1920, 1200),
                scale_rect(scale, CANON_W, CANON_H, 1920, 1200),
                "Parity keeps the landed path for {scale:?}"
            );
        }
    }

    /// The ENHANCED cursor mapping is ABSOLUTE through the world
    /// rect (the click targets live in the responsive layout,
    /// RESEARCH-HD-ASSET-PIPELINE §8) — exact corners/center, bars
    /// clamp to the frame edge, and the mapping never degrades to
    /// Fill's relative-only case because the layout never crops.
    #[test]
    fn enhanced_cursor_maps_absolutely_through_the_layout() {
        let frame = responsive_frame(1920, 1200);
        // World rect (240,60)-(1680,1140) over 640x480.
        assert_eq!(layout_cursor_to_game(240.0, 60.0, &frame), Some((0, 0)));
        assert_eq!(
            layout_cursor_to_game(1680.0, 1140.0, &frame),
            Some((639, 479))
        );
        assert_eq!(
            layout_cursor_to_game(960.0, 600.0, &frame),
            Some((320, 240))
        );
        // The bars clamp to the frame edge (never outside).
        assert_eq!(layout_cursor_to_game(0.0, 0.0, &frame), Some((0, 0)));
        assert_eq!(
            layout_cursor_to_game(1920.0, 1200.0, &frame),
            Some((639, 479))
        );
        // Degenerate target: no world, no mapping.
        let empty = ResponsiveFrame {
            safe: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            world: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        };
        assert_eq!(layout_cursor_to_game(5.0, 5.0, &empty), None);
    }

    /// THE PARITY BOUNDS (the trajectory pin, the D215 shape): the
    /// selection is PRESENTATION-BUCKET ONLY (D17 b) — it lives in
    /// `WindowOptions` and never enters `ModeConfig`/`SimConfig`,
    /// so the SAME pump script through hosts built under either
    /// presentation mode yields the identical executed-tick
    /// sequence, sim tick count, state hash, scene hash AND frame
    /// parity hash, and the canonical 640x480 indexed frame +
    /// palette are byte-identical (goldens stay canonical-frame
    /// based and resolution-agnostic under either mode).
    #[test]
    fn presentation_selection_never_touches_the_sim_or_the_hashed_trajectory() {
        let opts_all = [PresentationMode::Parity, PresentationMode::Enhanced].map(opts_with);
        assert_eq!(
            host_sim_config(&opts_all[0]),
            host_sim_config(&opts_all[1]),
            "the selection never reaches the sim config"
        );
        let script = [4u32, 1, 1, 1, 1, 3, 2, 2, 0, 4];
        let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
        let mut executed = vec![Vec::new(); hosts.len()];
        for (i, &dt) in script.iter().cycle().take(30).enumerate() {
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
            for (host, log) in hosts.iter_mut().zip(executed.iter_mut()) {
                log.push(host.pump_frame(dt, &input));
            }
        }
        for (i, host) in hosts.iter().enumerate().skip(1) {
            assert_eq!(executed[i], executed[0], "same executed ticks");
            assert_eq!(
                host.driver().sim().tick_index(),
                hosts[0].driver().sim().tick_index()
            );
            assert_eq!(
                host.driver().sim().state_hash(),
                hosts[0].driver().sim().state_hash()
            );
            assert_eq!(host.scene_hash(), hosts[0].scene_hash());
            assert_eq!(host.frame().parity_hash(), hosts[0].frame().parity_hash());
            // The canonical frame + palette ride BYTE-IDENTICAL —
            // the ENHANCED composition reads them, it never rewrites
            // them (the native pass is a separate plane).
            assert_eq!(host.frame().indices, hosts[0].frame().indices);
            assert_eq!(host.frame().palette, hosts[0].frame().palette);
        }
    }

    /// GATE ANSWERS are option-invariant: the presentation
    /// selection lives entirely upstream of the host (never in
    /// `ModeConfig`), so BOTH pacing arms accept it identically —
    /// the present-gate and alpha answers are IDENTICAL under
    /// either selection in the modern AND the classic arm.
    #[test]
    fn presentation_option_never_changes_the_gate_answers() {
        for mode in [ModeConfig::MODERN, ModeConfig::CLASSIC] {
            let mut opts_all =
                [PresentationMode::Parity, PresentationMode::Enhanced].map(opts_with);
            for opts in &mut opts_all {
                opts.mode = mode;
            }
            let mut hosts: Vec<GameHost> = opts_all.iter().map(host_for).collect();
            let mut clock = FixedStepClock::host();
            for dt in [4u32, 0, 4, 3, 0, 4] {
                clock.advance(4_166_666);
                for host in &mut hosts {
                    host.pump_frame(dt, &InputFrame::default());
                }
                for (i, host) in hosts.iter().enumerate().skip(1) {
                    assert_eq!(
                        present_due(host),
                        present_due(&hosts[0]),
                        "{mode:?} dt {dt} selection {i}"
                    );
                    assert_eq!(
                        present_camera_alpha(host, &clock),
                        present_camera_alpha(&hosts[0], &clock),
                        "{mode:?} dt {dt} selection {i}"
                    );
                }
            }
        }
    }
}
