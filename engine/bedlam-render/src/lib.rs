//! bedlam-render: pure game-state-to-canonical-frame renderer (P3).
//!
//! Contract (docs/DESIGN-RENDER.md sec 1; D9/D12/D16/D20):
//! - render() is a PURE function of its RenderInput to a Frame:
//!   a 640x480 8-bit indexed framebuffer plus a 256-entry 6-bit VGA
//!   palette. This is the parity / golden representation. Everything
//!   above it - scaling, letterboxing, refresh rate, interpolation
//!   policy, GPU backend - is presentation (bedlam-platform) and NEVER
//!   feeds back into render or sim state.
//! - Hermetic like bedlam-core (docs/PLAN.md sec 7): no I/O, no clock,
//!   no threads, no ambient randomness. The ONLY float in this crate is
//!   the interpolation alpha, a presentation hint (D17): it may shape
//!   the frame via the interpolated camera position (quantized to the
//!   integer pixel grid) but never the simulation, and parity goldens
//!   run with interpolation OFF (prev_sim = None means alpha is ignored
//!   entirely).
//! - The composition pass ORDER is parity-relevant and fixed
//!   (DESIGN-RENDER sec 7 / RE-EXW-PACER sec 1): world, sprites, row
//!   blits, overlays, entities.

#![forbid(unsafe_code)]

pub mod blit;
pub mod compose;
pub mod frame;
pub mod map_overlay;
pub mod mission_view;
pub mod ui_bank;

pub use blit::{blit_indexed, center_in_canonical};
pub use compose::{render, MovieFrame, RenderInput};
pub use frame::{sanitize_palette, Frame, Vga6, VgaExpand};

/// Canonical framebuffer width in pixels (EXW 640x480 words
/// 00456ec6/00456ec8, consumed by CursorToGame@0044b428 and the F-key
/// BMP writer [verified, docs/RE-EXW-TICK.md]).
pub const CANON_W: u32 = 640;

/// Canonical framebuffer height in pixels.
pub const CANON_H: u32 = 480;

/// Canonical framebuffer size in bytes: 640 * 480 palette indices
/// (the 0x4b000 = 307200 MemCopy argument of the mission loop
/// [verified, docs/RE-EXW-PACER sec 1]).
pub const INDICES_LEN: usize = CANON_W as usize * CANON_H as usize;

/// Camera (scroll) clamp in 640x480 game coords: x 9..=631
/// (100 Hz mouse poll clamp [verified, docs/RE-EXW-TICK.md]).
pub const CAMERA_MIN_X: i32 = 9;
pub const CAMERA_MAX_X: i32 = 631;

/// Camera (scroll) clamp: y 9..=463.
pub const CAMERA_MIN_Y: i32 = 9;
pub const CAMERA_MAX_Y: i32 = 463;

/// Clamp a camera position to the original scroll bounds.
pub fn clamp_camera(x: i32, y: i32) -> (i32, i32) {
    (
        x.clamp(CAMERA_MIN_X, CAMERA_MAX_X),
        y.clamp(CAMERA_MIN_Y, CAMERA_MAX_Y),
    )
}
