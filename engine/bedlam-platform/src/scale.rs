//! Pure presentation geometry (no wgpu types): scaling modes and the
//! destination / uv rectangles they imply (DESIGN-RENDER sec 8 item 2).
//! All math here is plain arithmetic - unit-testable without a GPU.

use bedlam_render::VgaExpand;

/// A pixel rectangle on the presentation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// How the canonical 640x480 source maps onto the output target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleMode {
    /// Integer nearest-neighbor scale with pillarbox/letterbox bars:
    /// the largest integer scale factor that fits, centered (the
    /// DEFAULT, DESIGN-RENDER sec 8 item 2).
    #[default]
    Integer,
    /// Fit the whole frame inside the target (fractional scale
    /// allowed, letterbox bars).
    Fit,
    /// Fill the whole target, cropping the centered source sub-rect
    /// whose aspect matches the target.
    Fill,
    /// Stretch the WHOLE frame onto the WHOLE target: a non-uniform
    /// scale (aspect not preserved, no crop) — every source pixel is
    /// shown and every target pixel is covered. The P7 SteamDeck
    /// profile default's fill-the-panel arm (docs/P7-PORTS.md §5).
    Stretch,
}

/// Pixel filtering for the parity blit.
///
/// Nearest is the parity default. Linear bilinear-filters the EXPANDED
/// RGB values only - palette indices are never interpolated (four
/// neighbor lookups expanded then mixed in the shader).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    /// Nearest-neighbor (parity default).
    #[default]
    Nearest,
    /// Bilinear over expanded RGB (smooth presentation option).
    Linear,
}

/// Everything the parity blit needs besides the frame itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentConfig {
    pub scale: ScaleMode,
    pub filter: FilterMode,
    /// 6-to-8 bit palette expansion policy (DESIGN-RENDER sec 4).
    pub expand: VgaExpand,
}

impl Default for PresentConfig {
    /// Parity defaults: Integer + Nearest + Original expansion.
    fn default() -> Self {
        PresentConfig {
            scale: ScaleMode::Integer,
            filter: FilterMode::Nearest,
            expand: VgaExpand::Original,
        }
    }
}

/// Destination rectangle for the given mode, centered on the target.
/// Zero-size inputs yield a zero rect; callers skip the draw then.
pub fn scale_rect(mode: ScaleMode, sw: u32, sh: u32, dw: u32, dh: u32) -> Rect {
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
    }
    let (w, h) = match mode {
        ScaleMode::Integer => {
            // No integer factor fits (target smaller than the source):
            // zero rect, the caller skips the draw.
            let sx = dw / sw;
            let sy = dh / sh;
            if sx == 0 || sy == 0 {
                return Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                };
            }
            let s = sx.min(sy);
            (sw * s, sh * s)
        }
        ScaleMode::Fit => {
            let s = (dw as f64 / sw as f64).min(dh as f64 / sh as f64);
            (
                ((sw as f64) * s).round() as u32,
                ((sh as f64) * s).round() as u32,
            )
        }
        ScaleMode::Fill | ScaleMode::Stretch => (dw, dh),
    };
    let w = w.min(dw);
    let h = h.min(dh);
    Rect {
        x: (dw - w) / 2,
        y: (dh - h) / 2,
        w,
        h,
    }
}

/// Source uv sub-rect [u0, v0, u1, v1] sampled for the given mode: the
/// full frame for Integer/Fit/Stretch; the centered aspect-cropped
/// sub-rect for Fill. v = 0 is frame row 0 (top).
pub fn uv_rect(mode: ScaleMode, sw: u32, sh: u32, dw: u32, dh: u32) -> [f32; 4] {
    match mode {
        ScaleMode::Fill if sw > 0 && sh > 0 && dw > 0 && dh > 0 => {
            let s = (dw as f64 / sw as f64).max(dh as f64 / sh as f64);
            let fw = (dw as f64 / s) / sw as f64;
            let fh = (dh as f64 / s) / sh as f64;
            let u0 = (1.0 - fw) / 2.0;
            let v0 = (1.0 - fh) / 2.0;
            [u0 as f32, v0 as f32, (1.0 - u0) as f32, (1.0 - v0) as f32]
        }
        _ => [0.0, 0.0, 1.0, 1.0],
    }
}
