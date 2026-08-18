//! Canonical frame and palette types (DESIGN-RENDER sec 3-4).

use bedlam_core::hash::Fnv1a64;

use crate::{CANON_H, CANON_W, INDICES_LEN};

/// One canonical 6-bit VGA palette entry: each component 0..=63. The
/// palette stays 6-bit everywhere inside render/core; expansion to
/// 8-bit happens once, in presentation, under a named policy.
pub type Vga6 = [u8; 3];

/// 6-to-8 bit component expansion policy (DESIGN-RENDER sec 4).
///
/// - Original (default): v << 2 - byte-identical to what
///   SetPaletteRGB@0044aed4 uploads to DirectDraw [verified]; the
///   brightest entry maps to 252, never 255.
/// - Full: (v << 2) | (v >> 4) - full range, for hosts that prefer
///   true whites. Non-original by definition, but never affects
///   goldens: parity hashes the 6-bit canon (Frame::parity_hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VgaExpand {
    /// v << 2 (the original upload behavior).
    #[default]
    Original,
    /// (v << 2) | (v >> 4).
    Full,
}

impl VgaExpand {
    /// Expand one 6-bit component. The input is masked to 6 bits first:
    /// the canonical form is enforced at every boundary crossing.
    pub fn expand_component(&self, v: u8) -> u8 {
        let v = v & 0x3f;
        match self {
            VgaExpand::Original => v << 2,
            VgaExpand::Full => (v << 2) | (v >> 4),
        }
    }

    /// Expand a full palette entry to 8-bit RGB.
    pub fn expand_rgb(&self, c: Vga6) -> [u8; 3] {
        [
            self.expand_component(c[0]),
            self.expand_component(c[1]),
            self.expand_component(c[2]),
        ]
    }
}

/// Mask a raw palette to the canonical 6-bit form (components AND 0x3f).
pub fn sanitize_palette(raw: [Vga6; 256]) -> [Vga6; 256] {
    let mut out = raw;
    for c in &mut out {
        c[0] &= 0x3f;
        c[1] &= 0x3f;
        c[2] &= 0x3f;
    }
    out
}

/// Canonical frame: 640x480 8-bit indexed pixels, a 256-entry 6-bit
/// palette, and the palette-dirty handshake flag (word 004ee9b6 analog,
/// DESIGN-RENDER sec 2 fact 7: SetPaletteRGB sets it, DDFlipOrBlt
/// re-applies and clears it each present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Palette index per pixel, row-major, row 0 = top.
    pub indices: Box<[u8; INDICES_LEN]>,
    /// 6-bit canonical palette (components masked to 0..=63).
    pub palette: [Vga6; 256],
    /// Whether presentation must re-upload the palette. Derived by
    /// render() from hashed sim counters; see compose module docs.
    pub palette_dirty: bool,
}

impl Frame {
    /// New all-index-0 frame with the given (already canonical)
    /// palette and palette_dirty = false.
    pub fn new(palette: [Vga6; 256]) -> Frame {
        Frame {
            indices: Box::new([0; INDICES_LEN]),
            palette,
            palette_dirty: false,
        }
    }

    /// Index at (x, y), or None outside the canonical rect.
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        if x < CANON_W && y < CANON_H {
            Some(self.indices[y as usize * CANON_W as usize + x as usize])
        } else {
            None
        }
    }

    /// Set one index. Outside the canonical rect this is a no-op:
    /// render never panics on out-of-range draw coordinates.
    pub fn set(&mut self, x: u32, y: u32, index: u8) {
        if x < CANON_W && y < CANON_H {
            self.indices[y as usize * CANON_W as usize + x as usize] = index;
        }
    }

    /// Fill an axis-aligned rect, clipped to the canonical rect. The
    /// origin is i32 so camera math can hand in off-screen positions
    /// without the caller pre-clamping.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, w: u32, h: u32, index: u8) {
        for dy in 0..h as i32 {
            let y = y0 + dy;
            if y < 0 || (y as u32) >= CANON_H {
                continue;
            }
            for dx in 0..w as i32 {
                let x = x0 + dx;
                if x >= 0 && (x as u32) < CANON_W {
                    self.indices[y as usize * CANON_W as usize + x as usize] = index;
                }
            }
        }
    }

    /// Parity hash (DESIGN-RENDER sec 10): FNV-1a 64 over the indices
    /// then the 6-bit palette - resolution- and expansion-agnostic by
    /// construction (bedlam_core::hash for cross-crate consistency with
    /// sim state hashes). Goldens anchor on this value at sim-tick
    /// boundaries with interpolation off.
    pub fn parity_hash(&self) -> u64 {
        let mut h = Fnv1a64::new();
        h.write_bytes(&self.indices[..]);
        for c in &self.palette {
            h.write_bytes(&[c[0] & 0x3f, c[1] & 0x3f, c[2] & 0x3f]);
        }
        h.finish()
    }
}
