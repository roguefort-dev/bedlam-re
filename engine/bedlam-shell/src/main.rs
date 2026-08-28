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
//!   pacing arm - `--classic` pins vsync).
//!
//! Usage: bedlam-shell [INSTALL_DIR] [--window] [--classic] [--uncapped] [--pumps N]
//! INSTALL_DIR defaults to `game-data/BEDLAM` (repo layout; GAMEGFX
//! is resolved inside it).

use std::path::PathBuf;
use std::process::ExitCode;

use bedlam_core::mode::ModeConfig;
use bedlam_shell::headless::{run_headless, HeadlessOptions, HeadlessReport};
use bedlam_shell::window::{run_window, Vsync, WindowOptions};

const DEFAULT_GFX: &str = "game-data/BEDLAM";

fn main() -> ExitCode {
    let mut gfx_dir: Option<PathBuf> = None;
    let mut window = false;
    let mut classic = false;
    let mut uncapped = false;
    let mut pumps: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--window" | "-w" => window = true,
            "--classic" | "-c" => classic = true,
            "--uncapped" | "-u" => uncapped = true,
            "--pumps" => match args.next().and_then(|v| v.parse().ok()) {
                Some(n) => pumps = Some(n),
                None => {
                    eprintln!("bedlam-shell: --pumps needs a number");
                    return ExitCode::from(2);
                }
            },
            "--help" | "-h" => {
                println!("usage: bedlam-shell [INSTALL_DIR] [--window] [--classic] [--uncapped] [--pumps N]");
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
        run_window(opts).map(|()| None).map_err(Into::into)
    } else {
        if uncapped {
            // The headless path owns no present surface; the option
            // is a no-op there (noted, never fatal).
            eprintln!("bedlam-shell: --uncapped is a window-present option; ignored headless");
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
