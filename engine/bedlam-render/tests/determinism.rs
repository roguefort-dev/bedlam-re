//! Determinism and boundary tests for the render crate (D12/D17,
//! DESIGN-RENDER sec 5/10).

use bedlam_core::input::InputFrame;
use bedlam_core::sim::{Sim, SimConfig};
use bedlam_render::compose::RenderInput;
use bedlam_render::{clamp_camera, render, sanitize_palette, Frame, Vga6, VgaExpand};

fn palette() -> [Vga6; 256] {
    let mut p = [[0u8; 3]; 256];
    for (i, c) in p.iter_mut().enumerate() {
        *c = [
            (i & 0x3f) as u8,
            ((i * 3) & 0x3f) as u8,
            ((i * 7) & 0x3f) as u8,
        ];
    }
    p
}

/// Deterministic lockstep: a fresh sim run of the same script always
/// yields the same state, so prev/cur pairs are built by re-running.
fn sim_after(seed: u64, ticks: u32, input: &InputFrame) -> Sim {
    let mut sim = Sim::new(&SimConfig {
        seed,
        time_base: bedlam_core::time::TimeBase::NOMINAL,
    });
    for _ in 0..ticks {
        sim.tick(input);
    }
    sim
}

fn idle() -> InputFrame {
    InputFrame::default()
}

fn frame_of(sim: &Sim) -> bedlam_render::Frame {
    render(&RenderInput {
        sim,
        prev_sim: None,
        alpha: 0.0,
        palette: palette(),
    })
}

/// Same sim state, same palette -> byte-identical frames (purity), and
/// alpha is IGNORED with interpolation off (prev_sim = None): this is
/// the golden configuration, so host frame rate cannot move goldens.
#[test]
fn alpha_ignored_when_interpolation_off() {
    let sim = sim_after(7, 10, &idle());
    let a = frame_of(&sim);
    let b = render(&RenderInput {
        sim: &sim,
        prev_sim: None,
        alpha: 0.99,
        palette: palette(),
    });
    assert_eq!(a.parity_hash(), b.parity_hash());
    assert_eq!(a, b);
}

/// Same script -> same rendered frames at every tick (replay parity of
/// the FRAME layer, not just the sim hash).
#[test]
fn same_script_same_frames() {
    let mut input = idle();
    input.buttons = 1;
    let mut a = Sim::new(&SimConfig::default());
    let mut b = Sim::new(&SimConfig::default());
    let mut ha = Vec::new();
    let mut hb = Vec::new();
    for _ in 0..25 {
        a.tick(&input);
        b.tick(&input);
        ha.push(frame_of(&a).parity_hash());
        hb.push(frame_of(&b).parity_hash());
    }
    assert_eq!(ha, hb);
}

/// Different seeds must not render identical frames (the entity stub
/// consumes the entropy slot, so frames diverge).
#[test]
fn different_seed_different_frames() {
    let a = sim_after(1, 10, &idle());
    let b = sim_after(2, 10, &idle());
    assert_ne!(frame_of(&a).parity_hash(), frame_of(&b).parity_hash());
}

/// Interpolation endpoints: alpha = 0 keeps the previous camera,
/// alpha = 1 reaches the current camera (camera is actor-derived in
/// the skeleton). The two endpoints differ because the actor moved.
#[test]
fn interpolation_endpoints() {
    let mut input = idle();
    input.buttons = 1;
    let prev = sim_after(9, 40, &input);
    let cur = sim_after(9, 41, &input);
    let at0 = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 0.0,
        palette: palette(),
    });
    let at1 = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 1.0,
        palette: palette(),
    });
    let half = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 0.5,
        palette: palette(),
    });
    assert_ne!(at0.parity_hash(), at1.parity_hash());
    let h = half.parity_hash();
    assert!(h != at0.parity_hash() || h != at1.parity_hash());
}

/// Out-of-range alpha saturates instead of extrapolating or panicking.
#[test]
fn alpha_saturates() {
    let prev = sim_after(3, 5, &idle());
    let cur = sim_after(3, 6, &idle());
    let lo = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: -2.0,
        palette: palette(),
    });
    let hi = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 9.0,
        palette: palette(),
    });
    let zero = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 0.0,
        palette: palette(),
    });
    let one = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 1.0,
        palette: palette(),
    });
    assert_eq!(lo.parity_hash(), zero.parity_hash());
    assert_eq!(hi.parity_hash(), one.parity_hash());
}

/// palette_dirty: first frame dirty; static counters stay clean; a
/// 12.5 Hz bank cycle advance (first fire = microstep 24, inside tick
/// 5) and any fade step both set it again (word 004ee9b6 analog).
#[test]
fn palette_dirty_follows_satellites() {
    let f0 = frame_of(&sim_after(0, 0, &idle()));
    assert!(f0.palette_dirty, "first frame must be dirty");

    // Tick 1: no pal fire (24 lands in tick 5), fade disarmed.
    let f1 = render(&RenderInput {
        sim: &sim_after(0, 1, &idle()),
        prev_sim: Some(&sim_after(0, 0, &idle())),
        alpha: 0.0,
        palette: palette(),
    });
    assert!(!f1.palette_dirty, "tick 1 has no satellite advance");

    // Tick 3 (microsteps 11..15): still nothing.
    let f3 = render(&RenderInput {
        sim: &sim_after(0, 3, &idle()),
        prev_sim: Some(&sim_after(0, 2, &idle())),
        alpha: 0.0,
        palette: palette(),
    });
    assert!(!f3.palette_dirty, "tick 3 has no satellite advance");

    // Tick 5 contains microstep 24: the 12.5 Hz bank cycle fires.
    let f5 = render(&RenderInput {
        sim: &sim_after(0, 5, &idle()),
        prev_sim: Some(&sim_after(0, 4, &idle())),
        alpha: 0.0,
        palette: palette(),
    });
    assert!(f5.palette_dirty, "pal cycle fired in tick 5");

    // Fade: armed after tick 1, the first fire is microstep 6 (tick 2).
    let prev = sim_after(0, 1, &idle());
    let mut cur = sim_after(0, 1, &idle());
    cur.set_fading(true);
    cur.tick(&idle());
    let ff = render(&RenderInput {
        sim: &cur,
        prev_sim: Some(&prev),
        alpha: 0.0,
        palette: palette(),
    });
    assert!(ff.palette_dirty, "fade step advanced -> dirty");
}

/// Camera clamps to the original scroll bounds 9..=631 / 9..=463.
#[test]
fn camera_clamps() {
    assert_eq!(clamp_camera(-100, -100), (9, 9));
    assert_eq!(clamp_camera(10000, 10000), (631, 463));
    assert_eq!(clamp_camera(320, 240), (320, 240));
}

/// Expansion policies: Original = v << 2 (brightest 252, matching the
/// SetPaletteRGB upload); Full = (v << 2) | (v >> 4) (true 255).
#[test]
fn vga_expand_policies() {
    assert_eq!(VgaExpand::Original.expand_component(0), 0);
    assert_eq!(VgaExpand::Original.expand_component(63), 252);
    assert_eq!(VgaExpand::Original.expand_component(31), 124);
    assert_eq!(VgaExpand::Full.expand_component(63), 255);
    assert_eq!(VgaExpand::Full.expand_component(15), 60);
    assert_eq!(VgaExpand::Full.expand_component(16), 65);
    // Input masked to 6 bits at the boundary.
    assert_eq!(VgaExpand::Original.expand_component(0xff), 252);
    assert_eq!(VgaExpand::Full.expand_rgb([63, 31, 0]), [255, 125, 0]);
}

/// sanitize_palette masks every component to 6 bits.
#[test]
fn palette_sanitized() {
    let mut p = [[200u8; 3]; 256];
    p[0] = [255, 64, 63];
    let s = sanitize_palette(p);
    assert_eq!(s[0], [63, 0, 63]);
    assert_eq!(s[255], [8, 8, 8]); // 200 & 63 = 8
}

/// Frame geometry + no-panic drawing outside bounds.
#[test]
fn frame_geometry_and_clipping() {
    assert_eq!(bedlam_render::INDICES_LEN, 307200);
    let mut f = Frame::new(palette());
    assert_eq!(f.get(0, 0), Some(0));
    assert_eq!(f.get(639, 479), Some(0));
    assert_eq!(f.get(640, 479), None);
    assert_eq!(f.get(0, 480), None);
    f.set(640, 480, 9); // out of range: no-op, no panic
    f.fill_rect(-10, -10, 20, 20, 7); // clipped
    assert_eq!(f.get(0, 0), Some(7));
    assert_eq!(f.get(9, 9), Some(7));
    assert_eq!(f.get(10, 10), Some(0));
    f.fill_rect(635, 475, 100, 100, 3);
    assert_eq!(f.get(639, 479), Some(3));
}

/// World pass scrolls with the camera: moving the actor moves the
/// checkerboard phase (observable camera application).
#[test]
fn world_scrolls_with_camera() {
    let mut input = idle();
    input.buttons = 1;
    let a = sim_after(5, 0, &idle());
    let b = sim_after(5, 16, &input);
    let fa = frame_of(&a);
    let fb = frame_of(&b);
    // Column 0 world cell flips once the camera advanced 16 px.
    assert_ne!(fa.get(0, 0), fb.get(0, 0));
}

/// parity_hash covers indices and the 6-bit palette, and is agnostic
/// to 8-bit aliases of 6-bit values.
#[test]
fn parity_hash_covers_both() {
    let mut f = frame_of(&sim_after(1, 1, &idle()));
    let h0 = f.parity_hash();
    f.set(100, 100, 200);
    assert_ne!(f.parity_hash(), h0);
    let mut p2 = palette();
    p2[7] = [1, 2, 3];
    f.palette = p2;
    assert_ne!(f.parity_hash(), h0);
    // 6-bit masking inside the hash: 63 and 255 hash the same.
    let mut p3 = palette();
    p3[7] = [63, 63, 63];
    f.palette = p3;
    let h1 = f.parity_hash();
    let mut p4 = palette();
    p4[7] = [255, 255, 255];
    f.palette = p4;
    assert_eq!(f.parity_hash(), h1);
}
