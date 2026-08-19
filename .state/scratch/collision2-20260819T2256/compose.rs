//! Frame composition: the fixed pass pipeline (DESIGN-RENDER sec 7).
//!
//! Pass ORDER is the parity-relevant contract, mirroring the original
//! mission-loop composition [verified, RE-EXW-PACER sec 1]:
//! world -> sprites -> row blits -> overlays -> entities -> present.
//! The skeleton passes are deliberately dumb placeholders whose CONTENT
//! will be replaced by the P4 map/sprite/text passes; what is pinned
//! here is the order, the camera handling, and the palette-dirty
//! derivation. Dirty-row optimization is NOT implemented: the skeleton
//! always fully recomposes, which sec 7 designates the correctness
//! reference (output-identity would be the gate for any later row-dirty
//! fast path).
//!
//! Movie plane (P5, D31): when RenderInput carries a decoded
//! full-frame movie raster, the pipeline is REPLACED, not extended -
//! render() emits the movie blit plus its own palette, exactly like the
//! original title screen where TITLE.SMK owns the whole DAC. Scene
//! composition resumes the moment the plane is absent.

use bedlam_core::sim::Sim;

use crate::clamp_camera;
use crate::frame::{sanitize_palette, Frame, Vga6};
use crate::{CANON_H, CANON_W};

/// Palette index of the world checkerboard dark square (stub).
const IDX_WORLD_A: u8 = 0;
/// Palette index of the world checkerboard light square (stub).
const IDX_WORLD_B: u8 = 1;
/// Palette index of the sprite-pass placeholder block.
const IDX_SPRITE: u8 = 3;
/// Palette index of the entity-pass placeholder block.
const IDX_ENTITY: u8 = 5;

/// World grid quantization of the stub checkerboard (pixels).
const WORLD_GRID: i32 = 16;
/// Sprite-pass placeholder block size (px).
const SPRITE_SIZE: u32 = 8;
/// Entity-pass placeholder block size (px).
const ENTITY_SIZE: u32 = 4;

/// A decoded movie frame presented as the whole canonical output
/// (P5/D31): an indexed raster of width x height plus the 256-entry
/// 6-bit palette the movie owns while it plays.
///
/// Built by the host movie player from its decoder state; render()
/// never decodes anything. The raster may be smaller than the canon in
/// either dimension (TITLE.SMK is 640x320 on the 640x480 canon) or
/// larger; either way it is CENTERED and clipped, never scaled, and the
/// exposed canon area outside the raster keeps index 0 (the first
/// palette entry of the same movie palette) [design, D31: exact EXW
/// title placement pending title-screen RE; centered letterbox is the
/// documented choice until then].
#[derive(Debug, Clone, Copy)]
pub struct MovieFrame<'a> {
    pub width: u32,
    pub height: u32,
    /// width * height palette indices, row-major, row 0 = top.
    pub pixels: &'a [u8],
    /// The palette the movie uploads to the DAC for this frame
    /// (6-bit canonical; hosts dequantize decoder palettes with
    /// [`crate::palmap_dequantize_palette`]).
    pub palette: [Vga6; 256],
}

/// Input to render(): the current sim, optionally the previous-tick sim
/// for camera interpolation, the interpolation alpha, the canonical
/// 6-bit palette for this frame, and an optional movie plane that
/// replaces the scene pipeline while present.
///
/// D17 boundary: prev_sim + alpha are PRESENTATION inputs. They shape
/// the interpolated camera (quantized to the integer grid) and nothing
/// else; with prev_sim = None (the parity/golden configuration) alpha
/// is ignored entirely and the output depends only on sim + palette.
/// The movie plane is presentation-side state by construction (a
/// decode driven by the host clock, D17 bucket b) and never touches
/// hashed sim or scene state.
pub struct RenderInput<'a> {
    /// Current simulation state (only hashed-bucket accessors are read).
    pub sim: &'a Sim,
    /// Previous-tick simulation state for camera interpolation; None =
    /// interpolation OFF = the parity/golden configuration.
    pub prev_sim: Option<&'a Sim>,
    /// Interpolation alpha, nominally 0..=1: PRESENTATION hint only.
    /// Saturated on use; out-of-range values never panic and never
    /// extrapolate. Ignored when prev_sim is None.
    pub alpha: f32,
    /// Canonical 6-bit palette for this frame (components masked to
    /// 0..=63 by render()). The skeleton takes it as an argument; the
    /// P4 palette-bank pass will derive bank rotation and fades from
    /// hashed sim state instead (DESIGN-RENDER sec 11 open items 2/6).
    /// IGNORED when a movie plane is present: the movie owns the DAC.
    pub palette: [Vga6; 256],
    /// Decoded movie raster replacing the scene pipeline (D31). None =
    /// normal scene composition.
    pub movie: Option<MovieFrame<'a>>,
}

/// Render one canonical frame. Pure: same input, same output bytes.
pub fn render(input: &RenderInput) -> Frame {
    match input.movie.as_ref() {
        Some(mv) => render_movie(mv),
        None => render_scene(input),
    }
}

/// Scene composition (the fixed pass pipeline).
fn render_scene(input: &RenderInput) -> Frame {
    let alpha = input.alpha.clamp(0.0, 1.0);
    let camera = camera_for(input, alpha);
    let mut frame = Frame::new(sanitize_palette(input.palette));
    pass_world(&mut frame, camera);
    pass_sprites(&mut frame, camera, input.sim.actor());
    pass_rows(&mut frame);
    pass_overlays(&mut frame);
    pass_entities(&mut frame, camera, input.sim);
    frame.palette_dirty = palette_dirty(input);
    frame
}

/// Movie composition (D31): centered clipped blit, movie palette,
/// palette_dirty EVERY frame (the decoder swaps palettes at frame
/// granularity; presentation re-uploads per frame - a dirty-tracking
/// optimization would have to prove output identity, same rule as the
/// row-dirty fast path).
fn render_movie(mv: &MovieFrame) -> Frame {
    let mut frame = Frame::new(sanitize_palette(mv.palette));
    blit_movie(&mut frame, mv);
    frame.palette_dirty = true;
    frame
}

/// Centered, clipped, unscaled raster blit. A plane whose pixels slice
/// is shorter than width * height draws nothing (bands stay index 0):
/// the MoviePlayer never builds such a plane, and render never panics
/// on out-of-range draw data by contract.
fn blit_movie(frame: &mut Frame, mv: &MovieFrame) {
    let w = mv.width as usize;
    let h = mv.height as usize;
    if w == 0 || h == 0 || mv.pixels.len() < w * h {
        return;
    }
    let x0 = ((CANON_W as i64 - mv.width as i64) / 2) as i32;
    let y0 = ((CANON_H as i64 - mv.height as i64) / 2) as i32;
    for dy in 0..h {
        let y = y0 + dy as i32;
        if !(0..CANON_H as i32).contains(&y) {
            continue;
        }
        // Source columns visible on the 640-wide canon: sx0..sx1.
        let sx0 = (-x0).max(0) as usize;
        let sx1 = ((CANON_W as i32 - x0).max(0) as usize).min(w);
        if sx0 >= sx1 {
            continue;
        }
        let dst = y as usize * CANON_W as usize + (x0 + sx0 as i32) as usize;
        let src = dy * w;
        frame.indices[dst..dst + (sx1 - sx0)]
            .copy_from_slice(&mv.pixels[src + sx0..src + sx1]);
    }
}

/// Interpolated camera: linear blend of the previous and current actor
/// position when interpolating, else the current position; ALWAYS
/// clamped to the original scroll bounds (9..=631 / 9..=463,
/// docs/RE-EXW-TICK). Quantized to the integer pixel grid so frame
/// bytes stay bit-deterministic for a given alpha: only IEEE-754
/// multiply-add-round touches the float, no libm. Sprites themselves
/// stay grid-quantized - interpolation touches the camera ONLY (D12);
/// sub-pixel blitting stays an off-by-default presentation option
/// (DESIGN-RENDER sec 9).
fn camera_for(input: &RenderInput, alpha: f32) -> (i32, i32) {
    let cur = input.sim.actor();
    let cam = match input.prev_sim {
        None => cur,
        Some(prev) => {
            let p = prev.actor();
            (
                p.0 + ((cur.0 - p.0) as f32 * alpha).round() as i32,
                p.1 + ((cur.1 - p.1) as f32 * alpha).round() as i32,
            )
        }
    };
    clamp_camera(cam.0, cam.1)
}

/// palette_dirty derivation (word 004ee9b6 analog,
/// DESIGN-RENDER sec 2 fact 7): true when presentation must re-upload
/// the palette. Derived purely from hashed sim counters: first frame
/// after interpolation-off start (no previous sim), any 12.5 Hz bank
/// cycle advance, or any 50 Hz fade step (the original FadeStep calls
/// SetPaletteRGB on all 256 entries every step). Limitation by design:
/// a stateless render cannot see caller-side palette CONTENT swaps;
/// hosts that swap palettes between frames compare Frame.palette
/// themselves (one array compare). Movie planes bypass this: they are
/// dirty every frame (see render_movie).
fn palette_dirty(input: &RenderInput) -> bool {
    match input.prev_sim {
        None => true,
        Some(prev) => {
            input.sim.pal_cycles() != prev.pal_cycles()
                || input.sim.fade_steps() != prev.fade_steps()
        }
    }
}

/// Pass 0: world layer - map tiles + static scenery (POS objects in the
/// original). Stub: a 16 px checkerboard offset by the camera, so scroll
/// behavior is observable and testable.
fn pass_world(frame: &mut Frame, camera: (i32, i32)) {
    for y in 0..CANON_H {
        let wy = y as i32 + camera.1;
        for x in 0..CANON_W {
            let wx = x as i32 + camera.0;
            let cell = (wx / WORLD_GRID) ^ (wy / WORLD_GRID);
            frame.set(
                x,
                y,
                if cell & 1 == 0 {
                    IDX_WORLD_A
                } else {
                    IDX_WORLD_B
                },
            );
        }
    }
}

/// Pass 1: AnimSprites analog (24-slot animator @0043f5b1 in the
/// original). Stub: one 8x8 marker at the actor screen position
/// (world pos minus camera plus viewport center). Draws ABOVE the
/// world pass.
fn pass_sprites(frame: &mut Frame, camera: (i32, i32), actor: (i32, i32)) {
    let sx = actor.0 - camera.0 + CANON_W as i32 / 2 - SPRITE_SIZE as i32 / 2;
    let sy = actor.1 - camera.1 + CANON_H as i32 / 2 - SPRITE_SIZE as i32 / 2;
    frame.fill_rect(sx, sy, SPRITE_SIZE, SPRITE_SIZE, IDX_SPRITE);
}

/// Pass 2: queued dirty-row blits (FUN_00402a56 analog). Stub: no-op.
/// Full recomposition already happened in pass 0; a row-dirty fast path
/// is only ever allowed if output-identical (DESIGN-RENDER sec 7).
fn pass_rows(_frame: &mut Frame) {}

/// Pass 3: DrawOverlays analog (15+15 text @0043fb80). Reserved for
/// the P4 text layer. Stub: no-op.
fn pass_overlays(_frame: &mut Frame) {}

/// Pass 4: AnimEntities analog (300-slot animator @0043f68d). Stub: one
/// 4x4 marker whose world position derives from hashed counters (the
/// last_draw entropy slot and service_ticks), proving that hashed
/// satellite state visibly reaches the frame. Draws ABOVE sprites.
fn pass_entities(frame: &mut Frame, camera: (i32, i32), sim: &Sim) {
    let wx = (sim.last_draw() % CANON_W) as i32;
    let wy = (sim.service_ticks() % CANON_H as u64) as i32;
    let sx = wx - camera.0 + CANON_W as i32 / 2 - ENTITY_SIZE as i32 / 2;
    let sy = wy - camera.1 + CANON_H as i32 / 2 - ENTITY_SIZE as i32 / 2;
    frame.fill_rect(sx, sy, ENTITY_SIZE, ENTITY_SIZE, IDX_ENTITY);
}
