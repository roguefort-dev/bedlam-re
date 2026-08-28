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
use bedlam_game::{GameConfig, GameError, GameHost, Scene};
use bedlam_platform::scale::{scale_rect, PresentConfig};
use bedlam_platform::{ParityGpu, ParityPipeline};
use bedlam_render::{CANON_H, CANON_W};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::audio::{AudioDevice, TARGET_FRAMES};
use crate::chain::{stage_boot, stage_scene, ChainConfig};
use crate::clock::{FixedStepClock, SUBTICKS_PER_PUMP};
use crate::headless::GameGfxSource;
use crate::input::{map_mouse_button, ControlScheme, ShellInput};

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
    /// Presentation config (PARITY defaults if unchanged).
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
    /// mutation. Presentation options (`present` above) stay OUT
    /// of the mode per the D200 layering: window mode, vsync and
    /// scaling are platform knobs, not purist toggles.
    pub mode: ModeConfig,
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
/// reaches the sim, the state hash or the scene hash.
fn present_due(host: &GameHost) -> bool {
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
/// scene hash (the clock/pump contract is untouched).
fn present_camera_alpha(host: &GameHost, clock: &FixedStepClock) -> Option<f32> {
    host.camera_interpolation().then(|| clock.fraction())
}

/// Open the window host and run until the window closes.
/// Blocks on the winit event loop; `pollster` blocks again inside
/// adapter/device setup. The caller owns the runtime gate.
pub fn run_window(mut opts: WindowOptions) -> Result<(), ShellError> {
    opts.size = (opts.size.0.max(64), opts.size.1.max(64));
    let event_loop = EventLoop::new().map_err(|e| ShellError::EventLoop(e.to_string()))?;

    let mut source = GameGfxSource::new(&opts.gfx_dir);
    // P6 mode plumbing (D205): ONE immutable ModeConfig from the
    // platform options feeds BOTH construction sites — the host
    // (so the present gate answers under the plumbed mode) and the
    // input mapper (the D204 consumer's platform selection).
    let mut host = GameHost::new(
        &GameConfig::default(),
        &host_sim_config(&opts),
        [[0u8, 0, 0]; 256],
    );
    stage_boot(&mut host, &mut source, opts.config)?;

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
            let feed = dev.feed().clone();
            if let Err(err) = feed.fill_from(&mut host, TARGET_FRAMES) {
                eprintln!("bedlam-shell: audio prefill failed ({err}); continuing");
            }
        }
        None => eprintln!("bedlam-shell: no audio output device; running silent"),
    }

    // The mapper's scheme comes from the SAME plumbed mode
    // (computed before `opts` moves into the app struct).
    let input = shell_input_for(&opts);
    let mut app = ShellApp {
        opts,
        source,
        host,
        scene: Scene::Boot,
        input,
        clock: FixedStepClock::host(),
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
        // Fifo is the only universally supported mode: vsync present.
        surface_cfg.present_mode = bedlam_platform::wgpu::PresentMode::Fifo;
        surface.configure(gpu.device(), &surface_cfg);
        let pipeline = ParityPipeline::new(&gpu, format);

        window.request_redraw();
        Ok(WindowHost {
            surface,
            gpu,
            pipeline,
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
    source: GameGfxSource,
    host: GameHost,
    scene: Scene,
    input: ShellInput,
    clock: FixedStepClock,
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
fn cursor_to_game(
    px: f64,
    py: f64,
    win_w: u32,
    win_h: u32,
    cfg: &PresentConfig,
) -> Option<(i32, i32)> {
    use bedlam_platform::scale::ScaleMode;
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

impl ShellApp {
    /// Stage the scene the host just entered. On staging failure the
    /// loop records the error and exits (a missing corpus asset is
    /// fatal for the window host too - the EXW movie opens fail
    /// hard).
    fn stage_entered(&mut self, event_loop: &ActiveEventLoop) {
        if self.host.scene() != self.scene {
            let config = self.opts.config;
            if let Err(err) = stage_scene(&mut self.host, &mut self.source, config) {
                self.fatal = Some(err.into());
                event_loop.exit();
                return;
            }
            self.scene = self.host.scene();
        }
    }

    /// Execute `pumps` fixed 60 Hz host pumps (timing decided HOW
    /// MANY; each pump is the same fixed dt + an input snapshot).
    fn run_pumps(&mut self, pumps: u32) {
        for _ in 0..pumps {
            let mut frame = self.input.tick();
            // Absolute-pointer steering (operator 2026-08-23): the DOS
            // menu integrates RELATIVE deltas, so the internal pointer
            // drifts from the real cursor and clicks land on the wrong
            // strip. While a menu is staged, replace the raw deltas with
            // the steering delta to the REAL cursor mapped into game
            // space; the menu still owns its position (parity path
            // unchanged), it just tracks the window cursor exactly.
            if let Some(target) = self.game_cursor_target() {
                if let Some((mx, my)) = self.host.menu_cursor() {
                    frame.mouse_dx = (target.0 - mx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    frame.mouse_dy = (target.1 - my).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
            }
            self.host.pump_frame(SUBTICKS_PER_PUMP, &frame);
        }
    }

    /// The real cursor in game space, when the window knows both the
    /// cursor position and the surface size.
    fn game_cursor_target(&self) -> Option<(i32, i32)> {
        let g = self.gfx.as_ref()?;
        let pos = g.cursor?;
        cursor_to_game(
            pos.x,
            pos.y,
            g.surface_cfg.width,
            g.surface_cfg.height,
            &self.opts.present,
        )
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
        if !present_due(&self.host) {
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
        if let Some(alpha) = present_camera_alpha(&self.host, &self.clock) {
            self.host.recompose(alpha);
        }
        let Some(g) = self.gfx.as_mut() else {
            return;
        };
        g.pipeline.upload_frame(self.host.frame());
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
        let buffer = g.pipeline.draw(
            &view,
            g.surface_cfg.width,
            g.surface_cfg.height,
            &self.opts.present,
        );
        g.gpu.queue().submit([buffer]);
        surface_texture.present();
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
        self.run_pumps(pumps);
        // 3. Refill the audio ring toward the watermark (the device
        //    callback drains it at its own pace; production self-
        //    balances against the device clock - D40). The Arc comes
        //    out first so host and audio borrow disjointly.
        if let Some(feed) = self.audio.as_ref().map(|dev| dev.feed().clone()) {
            if let Err(err) = feed.fill_from(&mut self.host, TARGET_FRAMES) {
                eprintln!("bedlam-shell: audio fill failed ({err}); continuing");
            }
        }
        // 4. Stage any scene the pumps entered.
        self.stage_entered(event_loop);
        if self.fatal.is_some() {
            return;
        }
        // 5. Request the next frame. UNCONDITIONAL in both arms
        //    (loop liveness: the vsync-paced redraw cycle keeps
        //    about_to_wait pumping even when the classic present
        //    gate holds the previous image — the gate lives at the
        //    present site, `ShellApp::present`). The Fifo present
        //    then blocks to vsync, pacing the loop.
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
