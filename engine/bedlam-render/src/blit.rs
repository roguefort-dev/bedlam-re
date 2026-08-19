//! Indexed-raster blit through the canonical Frame seam (D31).
//!
//! Movie rasters (decoded .SMK frames) are palette-indexed images of
//! arbitrary size; the canonical Frame is the fixed 640x480 indexed
//! surface presentation consumes. This module is the one documented
//! crossing: pure, clipped, no scaling (integer placement only), so a
//! composited frame stays bit-deterministic and hashable through the
//! existing Frame::parity_hash.

use crate::frame::Frame;
use crate::{CANON_H, CANON_W};

/// Blit a row-major indexed raster (src_w * src_h, row 0 = top) with
/// its top-left at (dst_x, dst_y), clipped to the canonical rect on
/// all four sides. A src shorter than src_w * src_h copies only the
/// whole rows present (defensive: render never panics on draw input).
/// Zero-sized sources are no-ops.
pub fn blit_indexed(frame: &mut Frame, src: &[u8], src_w: u32, src_h: u32, dst_x: i32, dst_y: i32) {
    frame.blit_indexed(src, src_w, src_h, dst_x, dst_y);
}

/// Centered placement policy for movie rasters [design, D31]: the
/// original EXW title-movie placement is not yet RE-d (the FUN_0044567c
/// movie-runner body is unmapped), so the documented choice is exact
/// centering in the 640x480 canon until that RE lands. Euclidean
/// division: odd/oversized rasters floor and stay non-panicking.
pub fn center_in_canonical(src_w: u32, src_h: u32) -> (i32, i32) {
    (
        (CANON_W as i64 - src_w as i64).div_euclid(2) as i32,
        (CANON_H as i64 - src_h as i64).div_euclid(2) as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame::new([[0, 0, 0]; 256])
    }

    #[test]
    fn interior_blit_copies_rows_top_down() {
        let mut f = frame();
        let src: Vec<u8> = (0..12u8).collect(); // 4x3, row 0 = 0,1,2,3
        blit_indexed(&mut f, &src, 4, 3, 10, 20);
        for y in 0..3u32 {
            for x in 0..4u32 {
                assert_eq!(f.get(10 + x, 20 + y), Some((y * 4 + x) as u8));
            }
        }
        assert_eq!(f.get(9, 20), Some(0), "left neighbor untouched");
        assert_eq!(f.get(14, 20), Some(0), "right neighbor untouched");
    }

    #[test]
    fn clips_on_all_four_edges() {
        let mut f = frame();
        let src: Vec<u8> = vec![7u8; 4 * 2]; // 4x2 raster
                                             // Top-left overhang by (1,1).
        blit_indexed(&mut f, &src, 4, 2, -1, -1);
        assert_eq!(f.get(0, 0), Some(7), "clipped corner still written");
        assert_eq!(f.get(2, 0), Some(7), "col 3 of the only surviving row");
        assert_eq!(f.get(2, 1), Some(0), "row 1 fell off the top");
        assert_eq!(f.get(639, 479), Some(0), "far corner untouched");
        // Off-screen entirely: no-op, no panic.
        blit_indexed(&mut f, &src, 4, 2, -100, -100);
        blit_indexed(&mut f, &src, 4, 2, 700, 500);
        // Right/bottom overhang.
        blit_indexed(&mut f, &src, 4, 2, 637, 478);
        assert_eq!(f.get(639, 479), Some(7));
    }

    #[test]
    fn short_source_copies_only_whole_rows() {
        let mut f = frame();
        let src: Vec<u8> = vec![9u8; 4 * 2 - 1]; // one byte short of 2 rows
        blit_indexed(&mut f, &src, 4, 3, 0, 0);
        for x in 0..4u32 {
            assert_eq!(f.get(x, 0), Some(9));
            assert_eq!(f.get(x, 1), Some(0), "partial row skipped");
        }
    }

    #[test]
    fn zero_sized_source_is_a_no_op() {
        let mut f = frame();
        blit_indexed(&mut f, &[1, 2, 3], 0, 5, 0, 0);
        blit_indexed(&mut f, &[1, 2, 3], 5, 0, 0, 0);
        assert_eq!(f.get(0, 0), Some(0));
    }

    #[test]
    fn full_canon_raster_at_origin_is_exact() {
        let mut f = frame();
        let src: Vec<u8> = (0..CANON_W as usize * CANON_H as usize)
            .map(|i| (i % 256) as u8)
            .collect();
        blit_indexed(&mut f, &src, CANON_W, CANON_H, 0, 0);
        assert_eq!(&f.indices[..], &src[..]);
    }

    #[test]
    fn center_policy_is_centered_and_safe() {
        // 640x320 (TITLE.SMK): centered vertically at 80.
        assert_eq!(center_in_canonical(640, 320), (0, 80));
        // Oversized rasters: negative origin, never a panic.
        assert_eq!(center_in_canonical(1024, 1024), (-192, -272));
        // Odd sizes floor.
        assert_eq!(center_in_canonical(641, 321), (-1, 79));
    }
}
