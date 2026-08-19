//! Full-screen movie snapshot (P5 playback integration, DECISIONS D31).
//!
//! [`Movie`] is the render-side half of Smacker playback: the immutable
//! snapshot of the LAST decoded frame (palette-index raster + palette).
//! The decode side (bedlam-assets `SmkStream`) owns the stream; whoever
//! drives it copies pixels/palette into this struct and bumps
//! [`Movie::revision`], and [`crate::compose::render`] composes it as the
//! topmost pass - while a movie plays, the video IS the screen.
//!
//! Palette note: the container stores 6-bit components; the vendored
//! decoder expands them through PALMAP = (v << 2) | (v >> 4) (exactly
//! [`crate::frame::VgaExpand::Full`]). Since
//! `((v << 2) | (v >> 4)) >> 2 == v | (v >> 6) == v` for v < 64, folding
//! the decoded 8-bit entries back with `>> 2` recovers the canonical
//! 6-bit [`crate::frame::Vga6`] form EXACTLY - no quantization loss in
//! either direction.

use crate::frame::Vga6;

/// Snapshot of one decoded movie frame for the render pass.
///
/// Plain mutable data by design: the pump owns it and rewrites the
/// fields in place per decoded frame (no per-frame allocation after
/// construction). `frame_index`/`revision` are diagnostics + dirty
/// tracking for hosts; render itself is a pure function of the current
/// contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// Raster width in pixels (need not match the canonical frame; the
    /// blit centers and clips).
    pub width: u32,
    /// Raster height in pixels.
    pub height: u32,
    /// `width * height` palette indices, row-major, row 0 = top.
    pub pixels: Box<[u8]>,
    /// Canonical 6-bit palette (the movie palette REPLACES the game
    /// palette while the movie composes - the title video carries its
    /// own colors).
    pub palette: [Vga6; 256],
    /// Zero-based index of the decoded frame this snapshot holds.
    pub frame_index: u32,
    /// Monotonic bump per decoded frame. Hosts compare consecutive
    /// revisions to detect content changes without hashing.
    pub revision: u64,
}

impl Movie {
    /// New all-zero snapshot of the given raster size. `pixels` is
    /// allocated once (`width * height` bytes); the pump overwrites it
    /// per frame.
    pub fn new(width: u32, height: u32) -> Movie {
        Movie {
            width,
            height,
            pixels: vec![0u8; width as usize * height as usize].into_boxed_slice(),
            palette: [[0u8; 3]; 256],
            frame_index: 0,
            revision: 0,
        }
    }
}
