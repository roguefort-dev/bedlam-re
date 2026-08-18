//! Pure presentation-geometry tests (no GPU needed).

use bedlam_platform::scale::{scale_rect, uv_rect, FilterMode, PresentConfig, ScaleMode};
use bedlam_render::VgaExpand;

/// Integer default: exact-multiple target maps 1:1, no bars.
#[test]
fn integer_exact_fit() {
    let r = scale_rect(ScaleMode::Integer, 640, 480, 1280, 960);
    assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 1280, 960));
}

/// Integer on 16:9: largest integer factor 2 -> 1280x960 centered,
/// pillarbox bars 320 px left/right and 60 px top/bottom.
#[test]
fn integer_16_9_pillarbox() {
    let r = scale_rect(ScaleMode::Integer, 640, 480, 1920, 1080);
    assert_eq!((r.x, r.y, r.w, r.h), (320, 60, 1280, 960));
}

/// Fit on 16:9: fractional scale 2.25 -> 1440x1080, letterbox only
/// horizontally (240 px side bars).
#[test]
fn fit_16_9_letterbox() {
    let r = scale_rect(ScaleMode::Fit, 640, 480, 1920, 1080);
    assert_eq!((r.x, r.y, r.w, r.h), (240, 0, 1440, 1080));
}

/// Fit never exceeds the target even after rounding.
#[test]
fn fit_clamps_rounding() {
    let r = scale_rect(ScaleMode::Fit, 640, 480, 641, 481);
    assert!(r.w <= 641 && r.h <= 481);
    let r = scale_rect(ScaleMode::Fit, 640, 480, 1, 1);
    assert_eq!((r.w, r.h), (1, 1));
}

/// Fill covers the whole target and crops the source uv rect to the
/// target aspect (centered): 4:3 source into 16:9 keeps full width,
/// crops top/bottom to 75 percent.
#[test]
fn fill_crops_uv() {
    let r = scale_rect(ScaleMode::Fill, 640, 480, 1920, 1080);
    assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 1920, 1080));
    let uv = uv_rect(ScaleMode::Fill, 640, 480, 1920, 1080);
    assert_eq!(uv[0], 0.0);
    assert_eq!(uv[2], 1.0);
    assert!((uv[1] - 0.125).abs() < 1e-6);
    assert!((uv[3] - 0.875).abs() < 1e-6);
}

/// Non-fill modes sample the full source.
#[test]
fn non_fill_uv_full() {
    for m in [ScaleMode::Integer, ScaleMode::Fit] {
        assert_eq!(uv_rect(m, 640, 480, 1920, 1080), [0.0, 0.0, 1.0, 1.0]);
    }
    // Degenerate dims: full uv, zero rect.
    assert_eq!(
        uv_rect(ScaleMode::Fill, 640, 480, 0, 0),
        [0.0, 0.0, 1.0, 1.0]
    );
    let r = scale_rect(ScaleMode::Integer, 640, 480, 0, 480);
    assert_eq!((r.w, r.h), (0, 0));
}

/// Targets smaller than the source yield a zero rect (no integer
/// factor fits; the caller skips the draw rather than cropping).
#[test]
fn integer_too_small_target_is_zero_rect() {
    let r = scale_rect(ScaleMode::Integer, 640, 480, 300, 200);
    assert_eq!((r.x, r.y, r.w, r.h), (0, 0, 0, 0));
}

/// Parity defaults: Integer + Nearest + Original (DESIGN-RENDER 4/8).
#[test]
fn parity_defaults() {
    let cfg = PresentConfig::default();
    assert_eq!(cfg.scale, ScaleMode::Integer);
    assert_eq!(cfg.filter, FilterMode::Nearest);
    assert_eq!(cfg.expand, VgaExpand::Original);
}
