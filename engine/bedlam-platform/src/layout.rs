//! The ENHANCED-mode responsive layout contract (P6 opener, PLAN §6
//! "Resolution independence + GPU rendering ... ENHANCED mode ...
//! bespoke responsive layouts target 16:9 and 16:10 (16:10
//! authoring master with 16:9 safe region), while other aspect
//! ratios fit/letterbox/pillarbox"; design inputs
//! docs/RESEARCH-HD-ASSET-PIPELINE.md §5.A + §8).
//!
//! Pure presentation geometry ONLY (no wgpu types, no clock, no
//! corpus): the authoring master, the centered 16:9 safe region on
//! ANY target, the world rect (the canonical 640x480 frame reusing
//! the LANDED [`crate::scale::scale_rect`] Fit shape), the pillarbox
//! margins the first native UI pass lives in, and the ABSOLUTE
//! cursor inverse through the world rect. Everything here is plain
//! arithmetic — unit-testable without a GPU or a window.
//!
//! D200 layering: the layout is presentation data consumed by the
//! present site only; it never enters ModeConfig, SimConfig, any
//! hash, or the save format (the ENHANCED composition is explicitly
//! NON-PARITY — the canonical frame + palette ride unchanged
//! underneath it).

use bedlam_render::{CANON_H, CANON_W};

use crate::scale::{scale_rect, Rect, ScaleMode};

/// The 16:10 authoring master width (px). The master is the
/// layout's authored coordinate space — the widest canvas the
/// bespoke ENHANCED layouts target (PLAN §6; §5.A "canvas master is
/// 16:10").
pub const MASTER_W: u32 = 1920;

/// The 16:10 authoring master height (px).
pub const MASTER_H: u32 = 1200;

/// The frame-presentation path selection (P6 ENHANCED opener, PLAN
/// §6 "PARITY mode keeps the canonical 640x480 indexed frame +
/// palette and GPU-scales it ... ENHANCED mode is explicitly
/// non-parity and renders supported world/UI passes natively").
///
/// A PLATFORM presentation knob per D200 (OUT of
/// [`bedlam_core`-owned ModeConfig]; both pacing arms accept it
/// identically — it selects NOTHING in the sim): PARITY is the
/// shipped posture exactly — the whole target is the canonical
/// frame GPU-scaled per the PresentConfig scale selection; ENHANCED
/// composes the responsive layout instead (the canonical frame fits
/// into the safe region; the supported native passes — currently
/// the shell's mission-identity strip — render at presentation
/// resolution in the margins). The canonical frame + palette the
/// engine renders are byte-identical under either selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationMode {
    /// The shipped posture: the canonical frame GPU-scaled over the
    /// whole target per the PresentConfig scale/filter selection
    /// (D215). The default.
    #[default]
    Parity,
    /// The responsive composition: safe region + fitted world +
    /// native passes in the margins (explicitly NON-PARITY).
    Enhanced,
}

/// The authoring master rect (16:10, origin at 0,0).
pub fn authoring_master() -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: MASTER_W,
        h: MASTER_H,
    }
}

/// The SAFE REGION on a `w` x `h` target: the largest centered rect
/// with aspect 16:9 (or narrower, never wider) that fits — exactly
/// the §5.A contract, "the centered 16:9 rectangle is the safe
/// region". Wider-than-16:9 targets pillarbox it (side bars),
/// taller targets (16:10, 4:3, square) letterbox it (top/bottom
/// bands): the OTHER aspect ratios fit/letterbox/pillarbox against
/// the same master rule (PLAN §6). Zero-size inputs yield a zero
/// rect (callers skip the draw then, the scale_rect convention).
pub fn safe_region(w: u32, h: u32) -> Rect {
    if w == 0 || h == 0 {
        return Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
    }
    // Integer math, floor: the widest 16:9 rect (h = w*9/16) that
    // fits both dimensions.
    let sw = w.min(h.saturating_mul(16) / 9);
    let sh = sw * 9 / 16;
    Rect {
        x: (w - sw) / 2,
        y: (h - sh) / 2,
        w: sw,
        h: sh,
    }
}

/// The safe region inside the 16:10 authoring master (the §5.A
/// authoring example: centered 1920x1080 in 1920x1200).
pub fn master_safe_region() -> Rect {
    safe_region(MASTER_W, MASTER_H)
}

/// The responsive composition of one target: the safe region plus
/// the WORLD rect — where the canonical 640x480 frame lands. The
/// world reuses the LANDED [`scale_rect`] Fit shape (the existing
/// PresentConfig geometry, PLAN §6 "fit/letterbox/pillarbox via the
/// existing shapes"): the whole frame fits inside the safe region,
/// centered, pillarboxed by the safe region's own margins when the
/// target is wide. The Enhanced composition NEVER crops the world
/// (the Fill crop semantics do not exist here — the whole canonical
/// frame is always visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsiveFrame {
    /// The centered 16:9 safe region on the target.
    pub safe: Rect,
    /// The canonical frame's destination rect, inside `safe`.
    pub world: Rect,
}

/// Compose the responsive frame for a `w` x `h` target (PURE).
pub fn responsive_frame(w: u32, h: u32) -> ResponsiveFrame {
    let safe = safe_region(w, h);
    let inner = scale_rect(ScaleMode::Fit, CANON_W, CANON_H, safe.w, safe.h);
    ResponsiveFrame {
        safe,
        world: Rect {
            x: safe.x + inner.x,
            y: safe.y + inner.y,
            w: inner.w,
            h: inner.h,
        },
    }
}

/// The pillarbox margins INSIDE the safe region that flank the
/// world rect: `(left, right)`. Zero-width rects (never negative)
/// when the world fills the safe region's width — the native UI
/// pass renders in the left margin only when the layout gives it
/// room. The bars OUTSIDE the safe region are NOT returned: they
/// are the future HD-pack/outpaint territory (§5.A/§8) and stay
/// matte black until that separately-scoped seam lands.
pub fn world_margins(frame: &ResponsiveFrame) -> (Rect, Rect) {
    let left = Rect {
        x: frame.safe.x,
        y: frame.safe.y,
        w: frame.world.x.saturating_sub(frame.safe.x),
        h: frame.safe.h,
    };
    let right_edge = frame.world.x + frame.world.w;
    let right = Rect {
        x: right_edge,
        y: frame.safe.y,
        w: frame
            .safe
            .x
            .saturating_add(frame.safe.w)
            .saturating_sub(right_edge),
        h: frame.safe.h,
    };
    (left, right)
}

/// Map a window-space physical cursor position to canonical game
/// space (640x480) through the responsive frame — the ABSOLUTE
/// inverse of the world rect, mirroring the window host's
/// Integer/Fit cursor mapping (the Enhanced composition never crops
/// the world, so unlike parity Fill there is no relative-only case).
/// Bars clamp to the frame edge; a degenerate world yields None.
pub fn layout_cursor_to_game(px: f64, py: f64, frame: &ResponsiveFrame) -> Option<(i32, i32)> {
    let r = frame.world;
    if r.w == 0 || r.h == 0 {
        return None;
    }
    let gx = ((px - f64::from(r.x)) * f64::from(CANON_W) / f64::from(r.w)).round() as i32;
    let gy = ((py - f64::from(r.y)) * f64::from(CANON_H) / f64::from(r.h)).round() as i32;
    Some((
        gx.clamp(0, CANON_W as i32 - 1),
        gy.clamp(0, CANON_H as i32 - 1),
    ))
}
