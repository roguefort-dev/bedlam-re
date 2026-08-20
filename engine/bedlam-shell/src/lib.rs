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
//! - [`input`]: the input adapter skeleton. Winit key/mouse events
//!   accumulate into a [`bedlam_core::input::InputFrame`] per tick
//!   through a shell-owned button-bit layout (provisional, D38 - the
//!   EXW scan-code keystore map is RE-EXW-INPUT.md, engine-side
//!   binding lands with P2e input RE).
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

pub mod chain;
pub mod clock;
pub mod headless;
pub mod input;
pub mod window;

pub use chain::{scene_assets, stage_boot, stage_scene, ChainConfig};
pub use clock::FixedStepClock;
pub use headless::{
    default_walk, run_headless, GameGfxSource, HeadlessOptions, HeadlessReport, SceneVisit,
    WalkStep,
};
pub use input::{map_mouse_button, map_winit_key, ShellInput, ShellKey};
pub use window::{run_window, ShellError, WindowOptions};
