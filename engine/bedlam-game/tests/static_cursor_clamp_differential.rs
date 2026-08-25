//! S0-17 static-parity oracle — the `static-cursor-clamp` row decoded,
//! classified, and the DOS/classic-input adapter pinned to the
//! twin-verified constants (D160, RE-EXD-MAP §5h).
//!
//! **The row's old gloss is disproven on all three counts.** The cells
//! 0x1074ac/0x1074b0 are NOT "clamp maxima" and the space is NOT
//! 320×240:
//! - `[0x1074b0]` = g_cursor_x, `[0x1074ac]` = g_cursor_y — the LIVE
//!   hardware-cursor POSITION pair, the EXW `g_cursor_x/y`
//!   @0x4eddc4/0x4eddc8 twins (RE-EXW-INPUT §4). Identity locked two
//!   ways: the INT 33h mickey axes (horizontal cx → 0x1074b0,
//!   vertical dx → 0x1074ac) and the in-mission hotspot twins
//!   (EXD 0x2f6d9 ⟷ EXW 0x41ec9d carrying IDENTICAL literals
//!   0x1ee/0x271/0xc3/0x146).
//! - 0xf0/0x140 are the GameInit boot-CENTER literals (X=320, Y=240 of
//!   640×480) — instruction-exact twins EXD 0x2c79a..0x2c7b2 ⟷ EXW
//!   0x41c083..0x41c09b, in the RNG-seed boot sandwich.
//! - The REAL clamp box **[9,631]×[9,463]** is enforced by the EXD
//!   mouse poll handler @0x12615..0x12659 (INT 33h AX=0003 buttons +
//!   AX=000B mickeys, integrate-then-clamp) and is the EXW
//!   ScrollUpdate box @0x425b2e..0x425b84 VERBATIM. The 9 is the
//!   24×24 hardware-cursor sprite hotspot offset (the sprite draws at
//!   (X−9, Y−9), EXD FUN_00012962 @0x12970..0x12992, ×640 stride).
//!
//! **Classification (the task charter):** hardware/input-profile-only.
//! The pair is host hardware-cursor state — written by the boot plant +
//! a hardware poll, redrawn from the 100 Hz interrupt family, driven by
//! raw mickeys — the D17 non-hashed bucket on BOTH channels, never read
//! by the deterministic sim. The registry row is NEVER counted as
//! static parity (S0 ledger: 24/27 static + 2/27 dynamic-only +
//! 1/27 hardware/input-profile-only) and stays EXD-address-only by
//! documented choice (the D139/D143 anti-ghost vehicle).
//!
//! **The adapter half:** `bedlam_core::input::InputFrame`
//! (mouse_dx/dy deltas — exactly the EXD INT-33h mickey model) +
//! `bedlam_core::frame::FrameState` integrate-then-clamp. This unit
//! re-pins its constants to the originals: clamp 9..=631 / 9..=463,
//! boot at the center (320,240). The canonical emission carries no
//! cursor field, so no chain moves.

use bedlam_core::frame::{
    FrameState, SimDriver, CURSOR_BOOT_X, CURSOR_BOOT_Y, CURSOR_MAX_X, CURSOR_MAX_Y, CURSOR_MIN_X,
    CURSOR_MIN_Y,
};
use bedlam_core::input::InputFrame;
use bedlam_core::sim::SimConfig;

// ---------------------------------------------------------------
// Original-side transcription (EXD poll handler 0x12615..0x12659).
// ---------------------------------------------------------------

/// The EXD GameInit boot plant, transcribed: X := 0x140 (esi),
/// Y := 0xf0 — the center of the 640×480 logical space.
const EXD_BOOT_X: i32 = 0x140;
const EXD_BOOT_Y: i32 = 0xf0;

/// The EXD clamp literals (EXW ScrollUpdate 0x425b2e..0x425b84 twins).
const EXD_X_MIN: i32 = 9;
const EXD_X_MAX: i32 = 0x277; // 631
const EXD_Y_MIN: i32 = 9;
const EXD_Y_MAX: i32 = 0x1cf; // 463

/// One EXD poll pass, transcribed instruction-faithfully from
/// 0x12615..0x12659 (X) and 0x1263b..0x12659 (Y):
/// `t = pos + cwde(mickey)`; `if t < min { min } else if t >= max
/// { max } else { t }`.
///
/// Note on the ≥ vs > edge: the EXD max branch is `cmp t,max; jb keep`
/// (i.e. `t >= max → max`) while EXW is `cmp t,max; jle keep`
/// (`t > max → max`) — at t == max both produce max, so the forms are
/// semantically identical; the transcription uses the EXD form.
fn exd_poll(x: i32, y: i32, mickey_h: i16, mickey_v: i16) -> (i32, i32) {
    let step = |pos: i32, delta: i16, min: i32, max: i32| {
        let t = pos + i32::from(delta);
        if t < min {
            min
        } else if t >= max {
            max
        } else {
            t
        }
    };
    (
        step(x, mickey_h, EXD_X_MIN, EXD_X_MAX),
        step(y, mickey_v, EXD_Y_MIN, EXD_Y_MAX),
    )
}

/// A scripted mickey walk with fixed (deterministic) deltas: probes
/// boot-center motion, sub-min pushes, in-range steps, exact-max landings,
/// beyond-max pushes, negative saturation, and back to center.
const WALK: [(i16, i16); 24] = [
    (10, -10),
    (-500, 500),
    (-4000, 4000),
    (321, -321),
    (-1, 1),
    (0, 0),
    (630, -630),
    (i16::MAX, i16::MIN),
    (i16::MIN, i16::MAX),
    (100, 100),
    (-1000, -1000),
    (2000, 2000),
    (-9, 9),
    (9, -9),
    (631, 463),
    (-631, -463),
    (311, 151),
    (-311, -151),
    (i16::MAX, i16::MAX),
    (i16::MIN, i16::MIN),
    (160, 120),
    (-160, -120),
    (5, 5),
    (-5, -5),
];

// ---------------------------------------------------------------
// The decode pins.
// ---------------------------------------------------------------

/// The GameInit plants are the CENTER of 640×480 on BOTH channels —
/// instruction-exact twins (EXD `mov esi,0x140; [0x1074b0]:=esi;
/// [0x1074ac]:=0xf0` @0x2c79a..0x2c7b2 ⟷ EXW `mov ebx,0x140;
/// [0x4eddc4]:=ebx; [0x4eddc8]:=0xf0` @0x41c083..0x41c09b).
#[test]
fn boot_center_plants_the_screen_middle_both_channels() {
    assert_eq!(EXD_BOOT_X, 320, "0x140 = 640/2");
    assert_eq!(EXD_BOOT_Y, 240, "0xf0 = 480/2");
    // The adapter boot equals the original boot plant.
    assert_eq!((CURSOR_BOOT_X, CURSOR_BOOT_Y), (EXD_BOOT_X, EXD_BOOT_Y));
    let f = FrameState::new();
    assert_eq!((f.cursor_x, f.cursor_y), (EXD_BOOT_X, EXD_BOOT_Y));
    let d = SimDriver::new(&SimConfig::default());
    assert_eq!((d.frame().cursor_x, d.frame().cursor_y), (320, 240));
}

/// The REAL clamp literals — the EXD poll box IS the EXW ScrollUpdate
/// box verbatim: [9,631]×[9,463]. (631 = 640−9; the bottom margin is
/// ASYMMETRIC — 480−463 = 17 — literal-pinned on both channels, not a
/// typo to "fix" to 471.)
#[test]
fn clamp_box_literals_are_the_exw_scrollupdate_box() {
    assert_eq!((EXD_X_MIN, EXD_X_MAX), (9, 0x277));
    assert_eq!((EXD_Y_MIN, EXD_Y_MAX), (9, 0x1cf));
    assert_eq!(EXD_X_MAX, 631);
    assert_eq!(EXD_Y_MAX, 463);
    // The adapter consts ARE the original literals.
    assert_eq!((CURSOR_MIN_X, CURSOR_MAX_X), (EXD_X_MIN, EXD_X_MAX));
    assert_eq!((CURSOR_MIN_Y, CURSOR_MAX_Y), (EXD_Y_MIN, EXD_Y_MAX));
    // Box sanity: a legal cursor position on both channels.
    const {
        assert!(EXD_X_MIN < EXD_BOOT_X && EXD_BOOT_X < EXD_X_MAX);
        assert!(EXD_Y_MIN < EXD_BOOT_Y && EXD_BOOT_Y < EXD_Y_MAX);
    }
}

/// The logical space is 640×480 on BOTH channels — EXD sets VESA mode
/// 0x101 (`mov ebx,0x101; mov eax,0x4f02; int 0x10` @0x1259a); the
/// cursor sprite walks a ×640 stride (`lea ecx,[ecx+ecx*4]; shl ecx,7`
/// @0x1297e) and draws 24×24 at (X−9, Y−9) — the hotspot offset −9,
/// the same 9 as the clamp margin. The in-mission panel hit-test
/// twins carry identical literals inside the box.
#[test]
fn space_is_640x480_and_the_hotspot_offset_is_the_margin() {
    // VESA 0x101 = 640x480x8.
    assert_eq!(0x101, 257);
    // The stride decode: *5 << 7 = *640.
    let stride = |y: i32| (y + y * 4) << 7;
    assert_eq!(stride(1), 640);
    assert_eq!(stride(2), 1280);
    // The sprite is 24x24 (0x18 loop bounds @0x129b1/0x129b6) and
    // draws at (X-9, Y-9): the hotspot offset equals the margin.
    assert_eq!(0x18, 24);
    assert_eq!(EXD_X_MIN, 9);
    assert_eq!(EXD_Y_MIN, 9);
    // The in-mission panel hotspot twins (EXD 0x2f6d9..0x2f79a ⟷ EXW
    // 0x41ec9d): 494..=625 x 195..=326 — inside the clamp box.
    let (hx_lo, hx_hi, hy_lo, hy_hi) = (0x1ee, 0x271, 0xc3, 0x146);
    assert_eq!((hx_lo, hx_hi, hy_lo, hy_hi), (494, 625, 195, 326));
    assert!(hx_lo > EXD_X_MIN && hx_hi <= EXD_X_MAX);
    assert!(hy_lo > EXD_Y_MIN && hy_hi <= EXD_Y_MAX);
    // The sidebar gate twin (EXD 0x1268f): X >= 0x1e0 (480) flips the
    // cursor-sprite selector — inside the box.
    assert_eq!(0x1e0, 480);
    const {
        assert!(480 > EXD_X_MIN && 480 < EXD_X_MAX);
    }
}

// ---------------------------------------------------------------
// The transcription and the adapter equality.
// ---------------------------------------------------------------

/// The EXD poll transcription hits every clamp edge exactly: sub-min
/// pushes saturate at 9, in-range motion is exact, the max edges pin
/// at 631/463, and beyond-max pushes saturate.
#[test]
fn poll_handler_transcription_pins_every_edge() {
    // From the boot center.
    assert_eq!(exd_poll(320, 240, 0, 0), (320, 240));
    // Sub-min: 320 + (-4000) -> 9 both axes.
    assert_eq!(exd_poll(320, 240, -4000, -4000), (9, 9));
    // Min edge is inclusive-in-value: 9 + 0 stays 9.
    assert_eq!(exd_poll(9, 9, 0, 0), (9, 9));
    // Just above min.
    assert_eq!(exd_poll(9, 9, 1, 1), (10, 10));
    // Land exactly on the max.
    assert_eq!(exd_poll(320, 240, 311, 223), (631, 463));
    // The EXD >= form at t == max produces max (identical to the EXW
    // > form — both saturate to the same value).
    assert_eq!(exd_poll(631, 463, 0, 0), (631, 463));
    assert_eq!(exd_poll(631, 463, 1, 1), (631, 463));
    // Below max is kept: 630 stays 630.
    assert_eq!(exd_poll(631, 463, -1, -1), (630, 462));
    // Beyond max saturates.
    assert_eq!(exd_poll(320, 240, i16::MAX, i16::MAX), (631, 463));
    assert_eq!(exd_poll(320, 240, i16::MIN, i16::MIN), (9, 9));
    // Negative deltas from min clamp at min.
    assert_eq!(exd_poll(9, 463, -1, 1), (9, 463));
}

/// THE ADAPTER PARITY PROOF: `FrameState::advance_frame` equals the
/// EXD poll-handler transcription over the scripted mickey walk from
/// the boot center (the InputFrame delta model IS the INT-33h mickey
/// model; cwde sign-extension = i16→i32).
#[test]
fn adapter_matches_the_original_transcription() {
    let mut f = FrameState::new();
    let (mut x, mut y) = (EXD_BOOT_X, EXD_BOOT_Y);
    for (dx, dy) in WALK {
        f.advance_frame(
            4,
            &InputFrame {
                mouse_dx: dx,
                mouse_dy: dy,
                ..InputFrame::default()
            },
        );
        let expect = exd_poll(x, y, dx, dy);
        assert_eq!(
            (f.cursor_x, f.cursor_y),
            expect,
            "divergence after mickey ({dx}, {dy})"
        );
        (x, y) = expect;
    }
}

// ---------------------------------------------------------------
// Sensitivity — both directions.
// ---------------------------------------------------------------

/// ADAPTER SIDE: the OLD placeholder (clamp [0,639]×[0,479] from
/// (0,0) — the pre-D160 adapter) diverges from the transcription on
/// the very first walk steps: the re-pin is load-bearing, not
/// cosmetic.
#[test]
fn old_placeholder_adapter_fails_the_comparison() {
    let mut old = (0i32, 0i32); // the old boot
    let (mut x, mut y) = (EXD_BOOT_X, EXD_BOOT_Y);
    let diverged_boot = old != (x, y);
    let mut diverged_box = false;
    for (dx, dy) in WALK {
        old = (
            (old.0 + i32::from(dx)).clamp(0, 639),
            (old.1 + i32::from(dy)).clamp(0, 479),
        );
        let expect = exd_poll(x, y, dx, dy);
        if old != expect {
            diverged_box = true;
        }
        (x, y) = expect;
    }
    assert!(
        diverged_boot,
        "the old boot (0,0) is not the original center"
    );
    assert!(
        diverged_box,
        "the old [0,639]x[0,479] box never matches the walk"
    );
    // The concrete first-step divergence, pinned: the old adapter
    // lands at (10, -10) where the original (booted at center) lands
    // at (330, 230).
    let old_first = (10i32, -10i32);
    assert_ne!(old_first, exd_poll(EXD_BOOT_X, EXD_BOOT_Y, 10, -10));
    assert_eq!(exd_poll(EXD_BOOT_X, EXD_BOOT_Y, 10, -10), (330, 230));
}

/// ORIGINAL SIDE: mutations of the transcription are caught — a
/// swapped box (X clamped by the Y literals and vice versa) and a
/// margin-less box ([0,631]×[0,463]) both diverge from the pinned
/// walk, so the transcription pins cannot rot silently.
#[test]
fn original_side_mutations_fail() {
    let swap_poll = |x: i32, y: i32, dx: i16, dy: i16| {
        let step = |pos: i32, d: i16, min: i32, max: i32| (pos + i32::from(d)).clamp(min, max);
        (
            step(x, dx, EXD_Y_MIN, EXD_Y_MAX), // swapped: X uses the Y box
            step(y, dy, EXD_X_MIN, EXD_X_MAX),
        )
    };
    // The sub-min push separates the boxes: the true box clamps to 9;
    // the swap is identical here (9 == 9) — so separate with an
    // asymmetric push: X beyond 463 but under 631.
    assert_eq!(exd_poll(320, 240, 400, 0).0, 631, "true X max 631");
    assert_eq!(exd_poll(320, 240, 400, 0).0, 631, "true X max 631");
    assert_eq!(swap_poll(320, 240, 400, 0).0, 463, "swapped X max 463");

    let marginless_poll = |x: i32, y: i32, dx: i16, dy: i16| {
        (
            (x + i32::from(dx)).clamp(0, EXD_X_MAX),
            (y + i32::from(dy)).clamp(0, EXD_Y_MAX),
        )
    };
    // The sub-min push separates the margins: 320-4000 -> 9 (true)
    // vs 0 (margin-less).
    assert_eq!(exd_poll(320, 240, -4000, 0).0, 9);
    assert_eq!(marginless_poll(320, 240, -4000, 0).0, 0);
    // And both mutations diverge from the true transcription over the
    // walk.
    let (mut x, mut y) = (EXD_BOOT_X, EXD_BOOT_Y);
    let (mut swap_diff, mut margin_diff) = (false, false);
    for (dx, dy) in WALK {
        let truth = exd_poll(x, y, dx, dy);
        if swap_poll(x, y, dx, dy) != truth {
            swap_diff = true;
        }
        if marginless_poll(x, y, dx, dy) != truth {
            margin_diff = true;
        }
        (x, y) = truth;
    }
    assert!(swap_diff, "a swapped box must be caught");
    assert!(margin_diff, "a margin-less box must be caught");
}

// ---------------------------------------------------------------
// The D17 guarantee rides along unchanged.
// ---------------------------------------------------------------

/// The cursor stays a non-hashed per-host-frame value (the D17 bucket
/// on both channels): divergent mickey trajectories never move the
/// sim hash, and huge mickeys saturate rather than panic — the
/// hardware/input-profile classification is exactly why this row is
/// never counted as static parity.
#[test]
fn cursor_bucket_stays_out_of_the_sim_hash() {
    let config = SimConfig::default();
    let mut a = SimDriver::new(&config);
    let mut b = SimDriver::new(&config);
    for (dx, dy) in WALK {
        a.advance(
            4,
            &InputFrame {
                mouse_dx: dx,
                mouse_dy: dy,
                ..InputFrame::default()
            },
        );
    }
    for _ in 0..WALK.len() {
        b.advance(4, &InputFrame::default());
    }
    assert_ne!(
        (a.frame().cursor_x, a.frame().cursor_y),
        (b.frame().cursor_x, b.frame().cursor_y)
    );
    assert_eq!(a.sim().state_hash(), b.sim().state_hash());
    // Saturated inside the original box, never outside.
    let (cx, cy) = (a.frame().cursor_x, a.frame().cursor_y);
    assert!((CURSOR_MIN_X..=CURSOR_MAX_X).contains(&cx));
    assert!((CURSOR_MIN_Y..=CURSOR_MAX_Y).contains(&cy));
}
