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
//! winit 0.30 shape (D39): the window is created inside resumed()
//! through ActiveEventLoop::create_window (the pre-run EventLoop
//! form is deprecated) and held behind an Arc, because wgpu needs an
//! owned window handle to hand the surface the static lifetime that
//! outlives run_app.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bedlam_game::{GameConfig, GameError, GameHost, Scene};
use bedlam_platform::scale::PresentConfig;
use bedlam_platform::{ParityGpu, ParityPipeline};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::audio::{AudioDevice, TARGET_FRAMES};
use crate::chain::{stage_boot, stage_scene, ChainConfig};
use crate::clock::{FixedStepClock, SUBTICKS_PER_PUMP};
use crate::headless::GameGfxSource;
use crate::input::{map_mouse_button, map_winit_key, ShellInput, ShellKey};

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
        }
    }
}

/// Open the window host and run until the window closes (or Escape).
/// Blocks on the winit event loop; `pollster` blocks again inside
/// adapter/device setup. The caller owns the runtime gate.
pub fn run_window(mut opts: WindowOptions) -> Result<(), ShellError> {
    opts.size = (opts.size.0.max(64), opts.size.1.max(64));
    let event_loop = EventLoop::new().map_err(|e| ShellError::EventLoop(e.to_string()))?;

    let mut source = GameGfxSource::new(&opts.gfx_dir);
    let mut host = GameHost::new(
        &GameConfig::default(),
        &bedlam_core::sim::SimConfig::default(),
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

    let mut app = ShellApp {
        opts,
        source,
        host,
        scene: Scene::Boot,
        input: ShellInput::new(),
        clock: FixedStepClock::host(),
        gfx: None,
        audio,
        fatal: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| ShellError::EventLoop(e.to_string()))?;
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
struct WindowHost {
    window: Arc<Window>,
    surface: bedlam_platform::wgpu::Surface<'static>,
    gpu: ParityGpu,
    pipeline: ParityPipeline,
    surface_cfg: bedlam_platform::wgpu::SurfaceConfiguration,
    cursor: Option<PhysicalPosition<f64>>,
    last_frame: Instant,
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
            window,
            surface,
            gpu,
            pipeline,
            surface_cfg,
            cursor: None,
            last_frame: Instant::now(),
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
struct ShellApp {
    opts: WindowOptions,
    source: GameGfxSource,
    host: GameHost,
    scene: Scene,
    input: ShellInput,
    clock: FixedStepClock,
    /// The window half, absent until resumed() builds it.
    gfx: Option<WindowHost>,
    /// The audio output (step 2, D40), absent when no device
    /// exists - the shell runs silent then, never fatal.
    audio: Option<AudioDevice>,
    fatal: Option<ShellError>,
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
            let frame = self.input.tick();
            self.host.pump_frame(SUBTICKS_PER_PUMP, &frame);
        }
    }

    /// Upload + present the canonical frame (PARITY path, D20).
    /// Split field borrows: gfx mutable, host frame read-only.
    fn present(&mut self) {
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
            Ok(gfx) => self.gfx = Some(gfx),
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
                if let Some((key, pressed)) = map_winit_key(&event) {
                    if key == ShellKey::Escape && pressed {
                        // Provisional D38 binding: Escape backs out of
                        // the shell (the EXW Escape target is the
                        // options screen - P2e input RE will pin it).
                        event_loop.exit();
                        return;
                    }
                    self.input.set_key(key, pressed);
                }
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
        // 5. Present on the next vsync.
        if let Some(g) = self.gfx.as_ref() {
            g.window.request_redraw();
        }
    }
}
