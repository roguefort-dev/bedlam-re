//! The bedlam-shell binary (P4 step 1): boots the wired chain
//! (boot attract -> title -> brief -> mission -> cutscene loading
//! flow, the D31-D37 sites) from a BEDLAM install directory.
//!
//! Modes:
//! - default: HEADLESS smoke - a fixed 60 Hz pump count, neutral
//!   input, no window, no GPU, no clock (safe for unattended runs
//!   and CI; prints the journey report).
//! - `--window` or `BEDLAM_SHELL=1`: the winit + wgpu window host
//!   (see [`bedlam_shell::window`]) - interactive display only.
//!   `--classic` selects the P6 classic purist mode for the window
//!   host (default = modern; the platform-level ModeConfig
//!   selection, D205). `--uncapped` requests the P6 optional
//!   uncapped present (a platform presentation option, PLAN §6
//!   "vsync-locked ... or uncapped"; honored only under the modern
//!   pacing arm - `--classic` pins vsync). `--fullscreen` /
//!   `--borderless` select the P6 QoL window mode (PLAN §6 "QoL:
//!   window modes"; default = a decorated window exactly as
//!   shipped; the exclusive-style fullscreen is best-effort; F11
//!   toggles at runtime). `--music PCT` / `--sfx PCT` select the
//!   P6 QoL volume mixers' starting levels (PLAN §6 "QoL: ...
//!   volume mixers"; a platform per-bus selection, default 100 =
//!   the shipped mix exactly; PageUp/PageDown and
//!   BracketRight/BracketLeft adjust at runtime). `--save-slot N`
//!   selects the P6 QoL save slot of the original's five (PLAN §6
//!   "QoL: ... save slots + metadata + opt-in autosave"; D213;
//!   default 1) and `--autosave` OPTS IN to the autosave policy —
//!   never the default: the shipped game never autosaves
//!   (RE-EXW-SAVE). `--scale MODE` (`integer`/`fit`/`fill`) and
//!   `--filter MODE` (`nearest`/`linear`) select the P6 resolution-
//!   independence scaling (PLAN §6 "GPU-scales it (nearest/integer
//!   default; fit/fill/smooth options)"; D215; a PURE platform
//!   presentation mapping over the already-landed bedlam-platform
//!   scale surface; default = integer + nearest exactly as shipped).
//!
//! Usage: bedlam-shell [INSTALL_DIR] [--window] [--classic] [--uncapped] [--fullscreen] [--borderless] [--music PCT] [--sfx PCT] [--save-slot N] [--autosave] [--scale MODE] [--filter MODE] [--pumps N]
//! INSTALL_DIR defaults to `game-data/BEDLAM` (repo layout; GAMEGFX
//! is resolved inside it).

use std::path::PathBuf;
use std::process::ExitCode;

use bedlam_platform::scale::{FilterMode, ScaleMode};

use bedlam_core::mode::ModeConfig;
use bedlam_shell::headless::{run_headless, HeadlessOptions, HeadlessReport};
use bedlam_shell::save::{AutosavePolicy, SaveSlotId};
use bedlam_shell::window::{
    filter_mode_from_cli, run_window, scale_mode_from_cli, scaling_present_config, Vsync,
    WindowMode, WindowOptions,
};

const DEFAULT_GFX: &str = "game-data/BEDLAM";

fn main() -> ExitCode {
    let mut gfx_dir: Option<PathBuf> = None;
    let mut window = false;
    let mut classic = false;
    let mut uncapped = false;
    let mut fullscreen = false;
    let mut borderless = false;
    let mut music: Option<u8> = None;
    let mut sfx: Option<u8> = None;
    let mut save_slot: Option<SaveSlotId> = None;
    let mut autosave = false;
    let mut scale: Option<ScaleMode> = None;
    let mut filter: Option<FilterMode> = None;
    let mut pumps: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--window" | "-w" => window = true,
            "--classic" | "-c" => classic = true,
            "--uncapped" | "-u" => uncapped = true,
            "--fullscreen" | "-f" => fullscreen = true,
            "--borderless" | "-b" => borderless = true,
            "--music" => match args.next().and_then(|v| v.parse().ok()) {
                Some(p) => music = Some(p),
                None => {
                    eprintln!("bedlam-shell: --music needs a percent 0..=100");
                    return ExitCode::from(2);
                }
            },
            "--sfx" => match args.next().and_then(|v| v.parse().ok()) {
                Some(p) => sfx = Some(p),
                None => {
                    eprintln!("bedlam-shell: --sfx needs a percent 0..=100");
                    return ExitCode::from(2);
                }
            },
            "--save-slot" => match args.next().and_then(|v| v.parse::<u8>().ok()) {
                Some(n) => match SaveSlotId::new(n.wrapping_sub(1)) {
                    Some(slot) => save_slot = Some(slot),
                    None => {
                        eprintln!("bedlam-shell: --save-slot needs a slot 1..=5");
                        return ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("bedlam-shell: --save-slot needs a slot 1..=5");
                    return ExitCode::from(2);
                }
            },
            "--autosave" => autosave = true,
            "--scale" => match args.next().and_then(|w| scale_mode_from_cli(&w)) {
                Some(mode) => scale = Some(mode),
                None => {
                    eprintln!("bedlam-shell: --scale needs integer|fit|fill");
                    return ExitCode::from(2);
                }
            },
            "--filter" => match args.next().and_then(|w| filter_mode_from_cli(&w)) {
                Some(mode) => filter = Some(mode),
                None => {
                    eprintln!("bedlam-shell: --filter needs nearest|linear");
                    return ExitCode::from(2);
                }
            },
            "--pumps" => match args.next().and_then(|v| v.parse().ok()) {
                Some(n) => pumps = Some(n),
                None => {
                    eprintln!("bedlam-shell: --pumps needs a number");
                    return ExitCode::from(2);
                }
            },
            "--help" | "-h" => {
                println!("usage: bedlam-shell [INSTALL_DIR] [--window] [--classic] [--uncapped] [--fullscreen] [--borderless] [--music PCT] [--sfx PCT] [--save-slot N] [--autosave] [--scale MODE] [--filter MODE] [--pumps N]");
                println!("  --window: interactive host (env BEDLAM_WINDOW_EXIT_MS=N auto-exits after N ms)");
                println!(
                    "  --classic: window host runs the classic purist mode (P6 ModeConfig preset;"
                );
                println!("             default = modern)");
                println!("  --uncapped: window host requests the uncapped present (no vsync wait;");
                println!(
                    "              P6 platform option; honored only in the modern pacing arm --"
                );
                println!("              --classic pins vsync; ignored headless)");
                println!(
                    "  --fullscreen: window host opens exclusive-style fullscreen (P6 QoL window"
                );
                println!(
                    "                mode, best-effort: borderless when no exclusive video mode;"
                );
                println!("                default = windowed; F11 toggles; ignored headless)");
                println!(
                    "  --borderless: window host opens borderless fullscreen (P6 QoL window mode;"
                );
                println!("                default = windowed; F11 toggles; ignored headless)");
                println!("  --music PCT: window host starts the music bus at PCT percent of the");
                println!("               shipped mix (P6 QoL volume mixers, 0..=100, clamped;");
                println!(
                    "               default 100 = shipped; PageUp/PageDown adjust; ignored headless)"
                );
                println!(
                    "  --sfx PCT: window host starts the SFX bus at PCT percent of the shipped"
                );
                println!("             mix (P6 QoL volume mixers, 0..=100, clamped; default 100 =");
                println!(
                    "             shipped; BracketRight/BracketLeft adjust; ignored headless)"
                );
                println!(
                    "  --save-slot N: window host targets save slot N of the original five (P6"
                );
                println!("                 QoL save slots, 1..=5; default 1; ignored headless)");
                println!(
                    "  --autosave: OPT IN to autosaving the campaign to the selected slot at the"
                );
                println!("              original's own save opportunities (single-player campaign");
                println!("              boundaries); NEVER the default - the shipped game never");
                println!("              autosaves; ignored headless)");
                println!("  --scale MODE: window host scales the canonical 640x480 frame MODE =");
                println!(
                    "                integer (largest integer scale, bars - default) | fit (whole"
                );
                println!("                frame inside, bars) | fill (whole target, cropped) (P6");
                println!("                resolution independence; ignored headless)");
                println!(
                    "  --filter MODE: window host samples the scaled frame MODE = nearest (the"
                );
                println!(
                    "                 parity pixels - default) | linear (smooth) (P6 resolution"
                );
                println!("                 independence; ignored headless)");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("bedlam-shell: unknown flag {other}");
                return ExitCode::from(2);
            }
            other => gfx_dir = Some(PathBuf::from(other)),
        }
    }
    if !window && std::env::var_os("BEDLAM_SHELL").is_some_and(|v| v != "0") {
        window = true;
    }
    let gfx_dir = gfx_dir.unwrap_or_else(|| PathBuf::from(DEFAULT_GFX));

    let result: Result<Option<HeadlessReport>, Box<dyn std::error::Error>> = if window {
        let mut opts = WindowOptions::new(&gfx_dir);
        // D48 test/repro hook: exit the window loop after N ms
        // through the same path as Escape (teardown verification
        // without a human at the keyboard).
        if let Some(ms) = std::env::var("BEDLAM_WINDOW_EXIT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
        {
            opts.auto_exit_after = Some(std::time::Duration::from_millis(ms));
        }
        // P6 platform selection (D205): `--classic` runs the window
        // host under the classic ModeConfig preset (purist timing
        // lock + original control scheme); default = modern. The
        // headless smoke path stays neutral/modern by construction —
        // it is the hashed-trajectory surface and owns no present
        // loop or mapper.
        if classic {
            opts.mode = ModeConfig::CLASSIC;
        }
        // P6 optional uncapped present (PLAN §6 "vsync-locked ...
        // or uncapped"): a PLATFORM presentation option (D200
        // layering — outside ModeConfig; the binary's `--uncapped`
        // selects it). The pacing policy arbitrates the request:
        // honored only under the modern arm — the loop then presents
        // as fast as it runs, every present recomposing from latest
        // state at the accumulator fraction; `--classic` pins
        // vsync-locked (RE-EXW-PACER §3).
        if uncapped {
            opts.vsync = Vsync::Uncapped;
        }
        // P6 QoL window modes (PLAN §6 "QoL: window modes"): a
        // PLATFORM presentation option (D200 layering — outside
        // ModeConfig, NO purist arbitration: the original was a
        // fullscreen DOS exclusive with no windowed mode to
        // preserve, so both pacing arms accept the selection
        // identically and it selects nothing in the host). Default
        // = windowed exactly as shipped; `--borderless` /
        // `--fullscreen` select the fullscreen shapes (exclusive
        // best-effort); F11 toggles at runtime.
        if fullscreen {
            opts.window_mode = WindowMode::Fullscreen;
        }
        if borderless {
            opts.window_mode = WindowMode::Borderless;
        }
        // P6 QoL volume mixers (PLAN §6 "QoL: ... volume mixers"): a
        // PLATFORM per-bus selection (D200 layering — outside
        // ModeConfig, NO purist arbitration: audio is presentation
        // bucket, D17 b). Default = the shipped mix exactly; the
        // gain applies at the audio feed's device-bound copy only —
        // the engine's mixed parity stream and every hash are
        // untouched by any setting (RE-EXW-MUSIC sec 7 re-anchor).
        if let Some(pct) = music {
            opts.volume = opts
                .volume
                .with_music(bedlam_shell::audio::VolumeLevel::new(pct));
        }
        if let Some(pct) = sfx {
            opts.volume = opts
                .volume
                .with_sfx(bedlam_shell::audio::VolumeLevel::new(pct));
        }
        // P6 QoL save slots + opt-in autosave (PLAN §6 "save slots +
        // metadata + opt-in autosave"; D213): PLATFORM knobs (D200
        // layering — outside ModeConfig) over the original's own
        // five-slot domain (RE-EXW-SAVE). The selection targets a
        // slot; `--autosave` OPTS IN to the policy — NEVER the
        // default (the shipped game never autosaves). Both are
        // platform surface only: nothing here reaches the sim config
        // or any hash, and no write ships in this unit (the new
        // versioned save format writer is future engine work,
        // config-not-state per the D201 posture).
        if let Some(slot) = save_slot {
            opts.save_slot = slot;
        }
        if autosave {
            opts.autosave = AutosavePolicy::On(opts.save_slot);
        }
        // P6 resolution independence (PLAN §6 "GPU-scales it
        // (nearest/integer default; fit/fill/smooth options)"): the
        // SCALING SELECTION is a PURE platform presentation mapping
        // over the already-landed bedlam-platform scale surface —
        // OUT of ModeConfig (D200 layering), NO purist arbitration
        // (the original was a fixed 640x480 DOS framebuffer with no
        // scaling mode to preserve, so both pacing arms accept it
        // identically) and it selects NOTHING in the host beyond the
        // PresentConfig the GPU scale path consumes. Defaults in,
        // defaults out: no flags = Integer + Nearest = the shipped
        // posture bit-for-bit.
        opts.present =
            scaling_present_config(scale.unwrap_or_default(), filter.unwrap_or_default());
        run_window(opts).map(|()| None).map_err(Into::into)
    } else {
        if uncapped {
            // The headless path owns no present surface; the option
            // is a no-op there (noted, never fatal).
            eprintln!("bedlam-shell: --uncapped is a window-present option; ignored headless");
        }
        if fullscreen || borderless {
            // Same posture for the window-mode options: no window,
            // no chrome to select.
            eprintln!(
                "bedlam-shell: --fullscreen/--borderless are window-host options; ignored headless"
            );
        }
        if music.is_some() || sfx.is_some() {
            // Same posture for the volume mixers: the headless path
            // owns no audio device, so there is no device-bound copy
            // to scale — and CRITICALLY the engine's mixed stream
            // (the determinism gate) must never see the knobs.
            eprintln!("bedlam-shell: --music/--sfx are window-host options; ignored headless");
        }
        if save_slot.is_some() || autosave {
            // Same posture for the save surface: the headless path is
            // the hashed-trajectory surface and owns no save screen —
            // and the policy NEVER writes anything by itself (D213:
            // the surface is inert until the versioned save format
            // writer lands).
            eprintln!(
                "bedlam-shell: --save-slot/--autosave are window-host options; ignored headless"
            );
        }
        if scale.is_some() || filter.is_some() {
            // Same posture for the scaling selection: the headless
            // path owns no surface, so there is no destination rect
            // to select — and the canonical frame it hashes is the
            // SOURCE, untouched by any selection (goldens stay
            // resolution-agnostic).
            eprintln!("bedlam-shell: --scale/--filter are window-host options; ignored headless");
        }
        let mut opts = HeadlessOptions::new(&gfx_dir);
        if let Some(pumps) = pumps {
            opts.pumps = pumps;
        }
        run_headless(&opts).map(Some).map_err(Into::into)
    };
    match result {
        Ok(Some(report)) => {
            print_report(&report);
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("bedlam-shell: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Human summary of the headless journey (scene walk, fetches,
/// hashes).
fn print_report(report: &bedlam_shell::headless::HeadlessReport) {
    println!("bedlam-shell headless smoke: {} host pumps", report.pumps);
    for visit in &report.scenes {
        println!("  {:?}: {} pumps", visit.scene, visit.pumps);
    }
    for action in &report.actions {
        println!("  pump {}: {:?}", action.0, action.1);
    }
    println!("assets fetched ({}):", report.assets.len());
    for (name, size) in &report.assets {
        println!("  {name}: {size} bytes");
    }
    println!("final scene hash: {:016x}", report.scene_hash);
    println!("final frame parity hash: {:016x}", report.frame_hash);
    println!(
        "audio mixed: {} frames, {} non-silent samples",
        report.audio_frames, report.audio_nonzero_samples
    );
}
