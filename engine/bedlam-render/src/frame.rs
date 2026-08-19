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

    /// Blit a smaller indexed raster (row-major, `src_w` x `src_h`) with
    /// its top-left at (`dst_x`, `dst_y`), clipped to the canonical
    /// rect: off-canvas coordinates are skipped, never panics. Palette
    /// indices are copied VERBATIM: the palette is applied by
    /// presentation, so no scaling, filtering or color math happens
    /// here (D31: the SMK/movie seam; bounds-checked like every other
    /// Frame draw).
    pub fn blit_indexed(&mut self, src: &[u8], src_w: u32, src_h: u32, dst_x: i32, dst_y: i32) {
        if src_w == 0 || src_h == 0 {
            return;
        }
        let (src_w, src_h) = (src_w as usize, src_h as usize);
        // Defensive: a mismatched buffer blits only the rows/cols that
        // exist, mirroring the render-crate rule that draw calls never
        // panic on out-of-range geometry.
        let usable_h = src_h.min(src.len() / src_w.max(1));
        for row in 0..usable_h {
            let y = dst_y + row as i32;
            if y < 0 || y >= CANON_H as i32 {
                continue;
            }
            let src_row = &src[row * src_w..(row + 1) * src_w];
            for (col, &index) in src_row.iter().enumerate() {
                let x = dst_x + col as i32;
                if x >= 0 && x < CANON_W as i32 {
                    self.indices[y as usize * CANON_W as usize + x as usize] = index;
                }
            }
        }
    }
}

#[cfg(test)]
mod blit_tests {
    use super::*;

    #[test]
    fn blit_full_replaces_region_row_major() {
        let mut f = Frame::new([[9, 9, 9]; 256]);
        let src: Vec<u8> = (0..12u8).collect(); // 4x3
        f.blit_indexed(&src, 4, 3, 10, 20);
        assert_eq!(f.get(10, 20), Some(0));
        assert_eq!(f.get(13, 20), Some(3));
        assert_eq!(f.get(10, 22), Some(8));
        assert_eq!(f.get(13, 22), Some(11));
        assert_eq!(f.get(14, 20), Some(0), "outside stays canvas");
    }

    #[test]
    fn blit_clips_on_every_edge() {
        let mut f = Frame::new([[0, 0, 0]; 256]);
        // 8x8 src placed at (-4, -4): only the bottom-right 4x4 quadrant lands
        let src: Vec<u8> = (0..64u8).collect();
        f.blit_indexed(&src, 8, 8, -4, -4);
        assert_eq!(f.get(0, 0), Some(4 * 8 + 4));
        assert_eq!(f.get(3, 3), Some(7 * 8 + 7));
        // fully off-canvas is a no-op, never a panic: the hash must be
        // unchanged across all three off-canvas placements
        let before = f.parity_hash();
        f.blit_indexed(&src, 8, 8, -100, -100);
        f.blit_indexed(&src, 8, 8, CANON_W as i32, 0);
        f.blit_indexed(&src, 8, 8, 0, CANON_H as i32);
        assert_eq!(f.parity_hash(), before);
    }

    #[test]
    fn blit_partial_stride_is_respected() {
        let mut f = Frame::new([[0, 0, 0]; 256]);
        // width 4, height 1, but a buffer with a stride-6 row
        let src = [7u8, 7, 7, 7, 99, 99];
        f.blit_indexed(&src, 4, 1, 0, 0);
        assert_eq!(f.get(0, 0), Some(7));
        assert_eq!(f.get(3, 0), Some(7));
        assert_eq!(f.get(4, 0), Some(0), "the stride tail never leaks");
    }
}
