//! MissionScene — the Mission-scene composition of the two
//! corpus-verified halves (DESIGN-GAME sec 11, added 2026-08-21):
//! bedlam-core `MissionSim` (the P2d/P4 sim slice) + bedlam-render
//! `MissionView` (the isometric viewport + robot entity overlay).
//!
//! NO decoding lives here — every behavior is anchored to an
//! already-RE-pinned EXW fact:
//! - staging: load_mission@0041dc5a + load_markers@0040cca0
//!   [RE-EXW-SIM sec 7c] — file bytes in, terrain + angle table +
//!   spawned robots out, `MissionShell` RNG reseed 0x1E240 [sec 1];
//! - per-frame: the MissionShell loop order — input (mouse_l_click)
//!   BEFORE the six unit-manager phases [sec 1], so the click seam
//!   runs before `advance_frame`;
//! - the click seam: robot-sprite click family ~0x433cbc arms the
//!   order AT the clicked robot [sec 6.4]; viewport clicks are
//!   x < 0x1E0, x >= 0x1E0 runs the sidebar producer [sec 6c —
//!   select strips + order rows + the redraw countdown];
//! - present: the viewport pass order enqueue -> terrain -> window
//!   [RE-EXW-MISSIONVIEW secs 5d/7].
//!
//! [design] tags below are reimplementation choices documented in
//! DESIGN-GAME sec 11, not RE claims.

use bedlam_core::hash::StateHash;
use bedlam_core::input::InputFrame;
use bedlam_core::mission::dist_octagonal;
use bedlam_core::mission::{AngleTable, MissionSim, Robot, Terrain};
use bedlam_core::rng::Pcg32;
use bedlam_render::mission_view::{
    present_window, DrawParams, MissionView, RobotView, VIEW_BUF_LEN,
};
use bedlam_render::ui_bank::draw_sprite;
use bedlam_render::Vga6;

use crate::loading::Plane;
use crate::GameError;

/// Sidebar robot-select strip x-ranges `[lo, hi]` per squad slot
/// (inclusive; slot 2's `[0x24B,0x27B]` is the asm's [0x24A< x <0x27C]
/// encoding) [RE-EXW-SIM sec 6c.2, asm 0x40d220..0x40d3b0].
pub const SIDEBAR_SELECT_STRIPS: [(i32, i32); 3] = [(0x1E7, 0x217), (0x219, 0x249), (0x24B, 0x27B)];
/// Sidebar robot-select strip y-range, inclusive [sec 6c.2].
pub const SIDEBAR_SELECT_STRIP_Y: (i32, i32) = (5, 0x35);
/// Sidebar order-row button rect, inclusive [sec 6c.4, asm 0x40d659].
pub const SIDEBAR_ORDER_RECT: (i32, i32, i32, i32) = (0x1E9, 0x275, 0x57, 0xB8);
/// Order-row pitch/first-row y: `row = (y - 0x57) / 14`, clamped to
/// 6 (7 rows exactly covering the rect height) [sec 6c.4].
pub const SIDEBAR_ORDER_ROW: (i32, i32) = (0x57, 14);
/// Order-row sprite x positions — the row body + the count well
/// [sec 6c.8a, asm 0x4084c1/0x4084dd: FUN_00401ca2 @ (0x1EB, y) and
/// (0x25A, y)]. GENERAL.BIN geometry: body 108x11 (x 0x1EB..0x257),
/// well 27x11 (x 0x25A..0x275).
pub const SIDEBAR_ROW_SPRITE_X: (i32, i32) = (0x1EB, 0x25A);
/// First order-row body y + pitch [sec 6c.8a: y = 0x59 + 14*i].
pub const SIDEBAR_ROW_SPRITE_Y: (i32, i32) = (0x59, 14);
/// Order-row sprite ids from GENERAL.BIN [sec 6c.8a]: armed rows
/// draw 0x47 + 0x4A, unarmed rows 0x49 + 0x4C.
pub const SIDEBAR_ROW_SPRITES: [(u16, u16); 2] = [(0x47, 0x4A), (0x49, 0x4C)];
/// Select-portrait sprite ids [sec 6c.8d, FUN_004072bf]: slot k
/// draws `base_sel + k` (selected) or `base_unsel + k`, at
/// (0x1E7 + 0x32*k, 5) — 48x48 sprites filling strip y 5..0x35.
pub const SIDEBAR_PORTRAIT_IDS: (u16, u16) = (0x12, 0x15);
/// Select-portrait x base + pitch (the strip x positions) + y.
pub const SIDEBAR_PORTRAIT_XY: (i32, i32, i32) = (0x1E7, 0x32, 5);

/// The sidebar presentation half [RE-EXW-SIM sec 6c; D17 split —
/// none of this enters the sim state hash]: the selected squad slot
/// (`DAT_0046cbdc`), the redraw countdown (`DAT_0046ccec`: producers
/// set 2, the draw tail decrements while nonzero and runs the sidebar
/// redraw pass FUN_00408403 — modeled here as the countdown alone),
/// and the per-robot order-bits word (+0x6E) with its 7-bit
/// availability mask (the +0x38+8k gate words; the type-table file
/// source is open, so availability defaults to all-7 [design] and a
/// host seam installs the real mask when the table lands).
#[derive(Debug, Default)]
struct Sidebar {
    selected: usize,
    redraw: i32,
    order_bits: Vec<u16>,
    order_avail: Vec<u8>,
}

impl Sidebar {
    /// Per-robot state at spawn [sec 6c.6]: availability default
    /// 0x7F [design], order bits `1 << first available` (= bit 0
    /// under the default mask), selected slot 0 (load_markers
    /// 0x40ce0e), redraw 0 (the MissionShell entry reset 0x4478bf).
    fn new(robots: usize) -> Sidebar {
        Sidebar {
            selected: 0,
            redraw: 0,
            order_bits: (0..robots)
                .map(|_| default_order_bits(ALL_ORDERS))
                .collect(),
            order_avail: vec![ALL_ORDERS; robots],
        }
    }
}

/// Default availability until the type table lands [design].
const ALL_ORDERS: u8 = 0x7F;

/// `1 << first available group` [sec 6c.6]; 0 when nothing is
/// available (matches the EXW: no group word0 nonzero -> no bit).
const fn default_order_bits(avail: u8) -> u16 {
    if avail == 0 {
        0
    } else {
        1u16 << avail.trailing_zeros()
    }
}

/// Robots spawned per player from the MRK records
/// [load_markers, RE-EXW-SIM sec 7c.7, verified]: zones {0,1,2,7} -> 1,
/// zone 3 -> 2, else 3.
pub fn robots_per_player(zone: i32) -> usize {
    if zone < 3 || zone == 7 {
        1
    } else if zone == 3 {
        2
    } else {
        3
    }
}

/// The zone index a campaign stage slot plays [design; the B2
/// order[8] zone table @0x81dba is the DOS-side anchor, the EXW path
/// arithmetic is `EDITOR\ZONE{chr(0x41+zone)}`, sec 7c.1]: stage 1
/// (boot camp) -> zone 0 (A), stages 2..=7 -> zones 1..=6 (B..=G),
/// the endgame stage cap stays at zone 6.
pub fn zone_for_stage(stage: u8) -> i32 {
    (i32::from(stage) - 1).clamp(0, 6)
}

/// The mission number for a stage's completion mask [design; the
/// Episode::complete lowest-unset-bit arithmetic, the same selection
/// briefing_name_for_slot uses for the BRF letter index]: first
/// uncompleted sub + 1.
pub fn mission_number_for_mask(mask: u8) -> i32 {
    let mut sub = 0u8;
    while mask >> sub & 1 != 0 {
        sub += 1;
    }
    i32::from(sub) + 1
}

/// The mission asset names in fetch order [design chain convention:
/// the load_mission path-1 trio (TOT/DAT/PAD, sec 7c.1), then the
/// zone-level path-2 pair (CGR/BIN) + LNK, then the GAMEGFX staging
/// family tail (SINTABLE, DANTE, GAMEPAL, GENERAL, SMLFONT — staged
/// after the mission files in MissionShell, sec 7c header; GAMEPAL
/// is the mission plane palette, MISSIONVIEW sec 6; GENERAL +
/// SMLFONT are the sidebar art banks, sec 6c.8c) and the markers.
/// Names carry the `EDITOR` tree sub-path with '/' separators for
/// the mission files; the byte source resolves them under
/// `EDITOR/` or `GAMEGFX/` [see bedlam-shell GameGfxSource].
pub fn mission_asset_names(zone: i32, mission: i32) -> Vec<String> {
    let zone_dir = format!("ZONE{}", (b'A' + zone as u8) as char);
    let zone_file = format!("MISSION{}", (b'A' + zone as u8) as char);
    let per_mission = format!("{zone_dir}/MISSION{mission}");
    [
        format!("{per_mission}.TOT"),
        format!("{per_mission}.DAT"),
        format!("{per_mission}.PAD"),
        format!("{zone_dir}/{zone_file}.CGR"),
        format!("{zone_dir}/{zone_file}.BIN"),
        format!("{zone_dir}/{zone_file}.LNK"),
        "SINTABLE.BIN".to_string(),
        "DANTE.BIN".to_string(),
        "GAMEPAL.PAL".to_string(),
        "GENERAL.BIN".to_string(),
        "SMLFONT.BIN".to_string(),
        format!("{per_mission}.MRK"),
    ]
    .to_vec()
}

/// The staged mission: sim + viewport + the fixed camera, plus the
/// presentation buffers. INERT until [`MissionScene::activate`] (the
/// host calls it on the Mission-scene entry; DESIGN-GAME sec 11
/// LIFECYCLE, the D31/D37 movie pattern).
#[derive(Debug)]
pub struct MissionScene {
    sim: MissionSim,
    view: MissionView,
    zone: i32,
    /// Q5 camera pair (the EXW `_DAT_004edde4/8` scroll anchor) —
    /// [design] FIXED at the first spawned robot's Q5 position for
    /// this slice (scroll input is out of scope).
    cam_q5: (i32, i32),
    /// Pointer in 640x480 screen space (0,0 top-left), clamped on
    /// every integrate [the menu D42 pattern; EXW clamps in the ISR].
    cursor: (i32, i32),
    /// Left-button level at the last consumed tick (the D26 hashed
    /// edge-latch analog; only the left bit matters to this seam).
    prev_buttons: u8,
    /// The off-map edge-variant stream (DrawParams.rng) [design:
    /// Pcg32::new(0x1E240, 0) — the MissionShell seed; zone 0 = ZONEA
    /// draws fixed edges and consumes none, MISSIONVIEW sec 7].
    edge_rng: Pcg32,
    /// The 0x64000 viewport buffer (DAT_004ede18).
    buf: Vec<u8>,
    /// The 640x480 presentation plane: the 480x480 present window at
    /// (0,0) — the EXW mission screen is viewport [0,480)x[0,480) +
    /// sidebar [480,640) [sec 6.2], NOT letterbox-centered.
    plane: Vec<u8>,
    /// The folded GAMEPAL palette the mission plane presents under
    /// [MISSIONVIEW sec 6: GAMEPAL loads into the 0x4edbf8 0x302-B
    /// blob the mission-load pass copies to 0x4ddb34, SIM sec 7c.3].
    palette: [Vga6; 256],
    /// GAMEGFX\GENERAL.BIN staged bytes (`_DAT_004edd7c`): the
    /// sidebar art bank — select portraits 0x12..0x17, order-row
    /// chrome 0x47/0x49 + 0x4A/0x4C, HP/armor bars [sec 6c.8c].
    general: Vec<u8>,
    /// `LoadFile("GAMEGFX\SMLFONT.BIN", _DAT_004ede7c)`.
    /// SMLFONT.BIN (63 glyphs) is the sidebar text bank [6c.8c];
    /// no text draws until the type table lands (never invented).
    smlfont: Vec<u8>,
    /// The sidebar presentation half [sec 6c; D17 split — outside
    /// the sim hash].
    sidebar: Sidebar,
    /// Presents executed (the one-render-per-host-frame rhythm).
    render_count: u64,
    active: bool,
}

impl MissionScene {
    /// `Terrain` from DAT+PAD+CGR, the angle table from SINTABLE
    /// words 2..66, `MissionSim` seeded 0x1E240 (the MissionShell
    /// reseed), the first `robots_override.unwrap_or(
    /// robots_per_player(zone))` MRK records spawned verbatim, then
    /// any staged markers (the host/test seam the network override
    /// 0x46cbe0 fills in the original, sec 7c.8), and the viewport
    /// over TOT + swept DAT planes + BIN + LNK with DANTE staged.
    /// GAMEPAL (770 B, the parse_vga770 family) folds to the
    /// canonical 6-bit palette and owns the plane [MISSIONVIEW
    /// sec 6]. GENERAL.BIN + SMLFONT.BIN stage as the sidebar art
    /// banks [sec 6c.8c]. Malformed bytes -> [`GameError::BadMissionAsset`],
    /// never a panic (charter); nothing is mutated on error.
    #[allow(clippy::too_many_arguments)]
    pub fn stage(
        tot: &[u8],
        dat: &[u8],
        pad: &[u8],
        cgr: &[u8],
        mrk: &[u8],
        bin: &[u8],
        lnk: &[u8],
        sintable: &[u8],
        dante: &[u8],
        gamepal: &[u8],
        general: &[u8],
        smlfont: &[u8],
        zone: i32,
        robots_override: Option<usize>,
        staged_markers: &[(i32, i32, i32)],
    ) -> Result<MissionScene, GameError> {
        let bad =
            |what: &'static str, reason: &'static str| GameError::BadMissionAsset { what, reason };
        let terrain = Terrain::from_mission_bytes(dat, pad, cgr)
            .ok_or_else(|| bad("DAT/PAD/CGR", "malformed mission terrain bytes"))?;
        let mut words = [0i16; 256];
        if sintable.len() < 512 {
            return Err(bad("SINTABLE", "shorter than 256 words"));
        }
        for (i, w) in words.iter_mut().enumerate() {
            *w = i16::from_le_bytes([sintable[2 * i], sintable[2 * i + 1]]);
        }
        let angles = AngleTable::from_sintable_words(&words)
            .ok_or_else(|| bad("SINTABLE", "short words array"))?;
        // GAMEPAL folds exactly like the loading palettes (6-bit file
        // values; the expand/fold round trip is lossless).
        let palette = crate::loading::loading_palette(gamepal)
            .map_err(|_| bad("GAMEPAL", "not a 770-byte VGA palette"))?;
        let mut sim = MissionSim::new(terrain, angles, 0x1E240);
        // MRK: 12 staged 16-B records `(flag, x, y, z-level)`; robot i
        // takes record i verbatim (flag dropped) [sec 7c.7, verified].
        let count = robots_override.unwrap_or_else(|| robots_per_player(zone));
        if mrk.len() < 16 * count {
            return Err(bad("MRK", "fewer marker records than robots"));
        }
        for i in 0..count {
            let rec = &mrk[16 * i..16 * i + 16];
            let word = |o: usize| {
                i32::try_from(u32::from_le_bytes([
                    rec[o],
                    rec[o + 1],
                    rec[o + 2],
                    rec[o + 3],
                ]))
                .unwrap_or(0)
            };
            sim.spawn_robot((word(4), word(8), word(12)));
        }
        for &marker in staged_markers {
            sim.spawn_robot(marker);
        }
        // The viewport reads the swept PRE-PAD plane bytes (the seen
        // marks compare DAT bytes against zero).
        let planes = bedlam_core::mission::dat_plane_bytes(dat)
            .ok_or_else(|| bad("DAT", "malformed plane bytes"))?;
        let mut view = MissionView::from_mission_bytes(tot, &planes, bin, lnk)
            .ok_or_else(|| bad("TOT/BIN/LNK", "malformed viewport bytes"))?;
        view.set_entity_bank(dante);
        let sidebar = Sidebar::new(sim.robots().len());
        Ok(MissionScene {
            sim,
            view,
            zone,
            cam_q5: (0, 0),
            cursor: (0, 0),
            prev_buttons: 0,
            edge_rng: Pcg32::new(0x1E240, 0),
            buf: vec![0u8; VIEW_BUF_LEN],
            plane: vec![0u8; 640 * 480],
            palette,
            general: general.to_vec(),
            smlfont: smlfont.to_vec(),
            sidebar,
            render_count: 0,
            active: false,
        })
    }

    /// Fix the camera at the first robot's Q5 position [DESIGN-GAME
    /// sec 11 LIFECYCLE; the EXW cam pair points at the spawn] and
    /// arm the initial sidebar draw (MissionShell 0x447C74 sets the
    /// redraw countdown 2 after the mission-load calls [6c.8e], so
    /// the rows draw on the entry frames). Idempotent.
    pub fn activate(&mut self) {
        if self.active {
            return;
        }
        self.cam_q5 = self.sim.robots().first().map(Robot::q5).unwrap_or((0, 0));
        self.sidebar.redraw = 2;
        self.active = true;
    }

    /// Whether the scene owns the Mission screen yet.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The fixed Q5 camera.
    pub fn camera(&self) -> (i32, i32) {
        self.cam_q5
    }

    /// The pointer position in screen space.
    pub fn cursor(&self) -> (i32, i32) {
        self.cursor
    }

    /// Presents executed since staging.
    pub fn render_count(&self) -> u64 {
        self.render_count
    }

    /// The hashed sim half (gate seam).
    pub fn sim(&self) -> &MissionSim {
        &self.sim
    }

    /// The sim state hash (the D17 hashed half of the composition).
    pub fn state_hash(&self) -> StateHash {
        self.sim.state_hash()
    }

    /// One executed 60 Hz tick [DESIGN-GAME sec 11 PER FRAME; the
    /// MissionShell order, RE-EXW-SIM sec 1]: integrate the pointer
    /// from mouse deltas (clamp 0..=639 / 0..=479), run the click
    /// seam on a left-button EDGE — the sidebar producer at
    /// `x >= 0x1E0` [sec 6c], the robot arm below it [sec 6.4] —
    /// then `advance_frame` (the six unit-manager phases + the
    /// order-window tick). Inert until [`MissionScene::activate`].
    pub fn tick(&mut self, input: &InputFrame) {
        if !self.active {
            return;
        }
        self.cursor.0 = (self.cursor.0 + i32::from(input.mouse_dx)).clamp(0, 639);
        self.cursor.1 = (self.cursor.1 + i32::from(input.mouse_dy)).clamp(0, 479);
        let left = input.mouse_buttons & 0x01;
        if self.prev_buttons == 0 && left != 0 {
            if self.cursor.0 >= 0x1E0 {
                self.sidebar_control();
            } else {
                self.click_robot();
            }
        }
        self.prev_buttons = left;
        self.sim.advance_frame();
    }

    /// The sidebar producer (mouse subset of sidebar_control@0040d197
    /// [RE-EXW-SIM sec 6c]): robot-select strips + the 7 order rows,
    /// gated exactly like the asm — alive robots only, squad slot
    /// within the spawned group, order availability per robot. Sets
    /// the redraw countdown to 2 on every fire. The map-toggle strip
    /// [sec 6c.1] is out of scope (screen-mode globals + the overlay
    /// family) — its rect is disjoint from both wired regions, so
    /// clicks there stay a no-op, and keyboard latches wait for the
    /// P2e button map.
    fn sidebar_control(&mut self) {
        let (x, y) = self.cursor;
        // Robot-select strips [sec 6c.2]: squad slot = strip index,
        // gated by the spawned group size (the DAT_0046cbd8 analog)
        // and the target's ALIVE word.
        for (slot, &(lo, hi)) in SIDEBAR_SELECT_STRIPS.iter().enumerate() {
            if (lo..=hi).contains(&x)
                && (SIDEBAR_SELECT_STRIP_Y.0..=SIDEBAR_SELECT_STRIP_Y.1).contains(&y)
            {
                if slot < self.sim.robots().len() && self.sim.robots()[slot].alive {
                    self.sidebar.selected = slot;
                    self.sidebar.redraw = 2;
                }
                return;
            }
        }
        // Order rows [sec 6c.4]: row = (y - 0x57)/14 clamped to 6,
        // gate = the selected robot's availability bit, toggle the
        // bit in its order-bits word.
        let (x0, x1, y0, y1) = SIDEBAR_ORDER_RECT;
        if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
            let row = (((y - SIDEBAR_ORDER_ROW.0) / SIDEBAR_ORDER_ROW.1) as usize).min(6);
            let robot = self.sidebar.selected;
            let avail = self.sidebar.order_avail.get(robot).copied().unwrap_or(0);
            if avail >> row & 1 != 0 {
                self.sidebar.order_bits[robot] ^= 1 << row;
                self.sidebar.redraw = 2;
            }
        }
    }

    /// The robot click seam [DESIGN-GAME sec 11; RE-EXW-SIM sec 6.4
    /// click-on-robot arm]: clicks land in the viewport
    /// (`x < 0x1E0`; `tick` dispatches `x >= 0x1E0` to the sidebar
    /// producer — the guard below is belt-and-braces), hit-test
    /// every alive robot by the enqueue projection (MISSIONVIEW
    /// sec 5d) inside a 0x20-px box [design: half the 64-px sprite
    /// cell; the EXW walks the sprite outlines ~0x433cbc], nearest
    /// octagonal screen distance wins (ties -> lowest index), and the
    /// order is armed AT that robot (the EXW arms at the clicked
    /// robot's tile — one pending order, spread-assign, state 3).
    fn click_robot(&mut self) {
        if self.cursor.0 >= 0x1E0 {
            return; // sidebar: the tick dispatcher owns this half
        }
        let cam = self.cam_q5;
        let (cx, cy) = self.cursor;
        let mut best: Option<(i32, usize)> = None;
        for (idx, r) in self.sim.robots().iter().enumerate() {
            if !r.alive {
                continue;
            }
            let view = RobotView::from_sim(r);
            let (sx, sy) = self.view.project_robot(&view, cam.0, cam.1, 0);
            let (dx, dy) = (sx - cx, sy - cy);
            if dx.abs() > 0x20 || dy.abs() > 0x20 {
                continue;
            }
            let dist = dist_octagonal(dx, dy);
            if best.is_none_or(|(d, _)| dist < d) {
                best = Some((dist, idx));
            }
        }
        if let Some((_, idx)) = best {
            self.sim.arm_order_at_robot(idx);
        }
    }

    /// One present [DESIGN-GAME sec 11; MISSIONVIEW secs 5d/7]:
    /// enqueue the robots (camera Q5, shake 0, the sim frame), run
    /// the terrain pass into the 0x64000 buffer, crop the 480x480
    /// present window with the fine-camera offset, and blit it at
    /// canonical (0, 0) of the 640x480 plane. Then the SIDEBAR ART
    /// half [RE-EXW-SIM 6c.8, the FUN_00403938 tail order]: the
    /// select portraits every present (FUN_004072bf — squad-size +
    /// alive gates, 0x12+slot selected / 0x15+slot not), and the
    /// order-row chrome on the redraw countdown (FUN_00408403 —
    /// armed rows 0x47+0x4A, unarmed 0x49+0x4C, rows gated by the
    /// availability bit; the FUN_00408403 decrements-then-draws
    /// rhythm [asm 0x407205..0x407217]). Name/count text, HP/armor
    /// bars, the score strip, the deploy panel and the blink cursor
    /// stay unwired — each needs state the sim does not model (see
    /// 6c.8, never invented). Advances the LNK walk + edge stream
    /// once — one render per host frame (D17 bucket b). Inert until
    /// active.
    pub fn present(&mut self) -> Option<&[u8]> {
        if !self.active {
            return None;
        }
        self.draw_sidebar_portraits();
        if self.sidebar.redraw > 0 {
            self.sidebar.redraw -= 1;
            self.draw_sidebar_rows();
        }
        let robots: Vec<_> = self.sim.robots().iter().map(RobotView::from_sim).collect();
        self.view
            .enqueue_robots(&robots, self.cam_q5.0, self.cam_q5.1, 0, self.sim.frame());
        let (cam_x, cam_y) = self.cam_q5;
        let zone = self.zone;
        self.view.draw_terrain(
            &mut self.buf,
            &mut DrawParams::new(cam_x >> 5, cam_y >> 5, zone, &mut self.edge_rng),
        );
        let win = present_window(&self.buf, cam_x, cam_y)?;
        for row in 0..480usize {
            let dst = row * 640;
            self.plane[dst..dst + 480].copy_from_slice(&win[row * 480..(row + 1) * 480]);
        }
        self.render_count += 1;
        Some(&self.plane)
    }

    /// The FUN_00408403 order-row chrome pass [sec 6c.8a]: 7 rows
    /// over the SELECTED robot, row i drawn iff its availability
    /// bit is set (the name-index gate analog), armed rows (the
    /// order-bits word bit i) drawing sprites 0x47 + 0x4A, unarmed
    /// rows 0x49 + 0x4C, at (0x1EB, 0x59+14i) and (0x25A, 0x59+14i)
    /// from GENERAL.BIN. Presentation half only.
    fn draw_sidebar_rows(&mut self) {
        let robot = self.sidebar.selected;
        let Some(&avail) = self.sidebar.order_avail.get(robot) else {
            return;
        };
        let bits = self.sidebar.order_bits.get(robot).copied().unwrap_or(0);
        let (y0, pitch) = SIDEBAR_ROW_SPRITE_Y;
        for i in 0..7u8 {
            if avail >> i & 1 == 0 {
                continue; // no weapon in this group
            }
            let armed = usize::from(bits >> i & 1 == 0);
            let y = y0 + pitch * i32::from(i);
            let (body, well) = SIDEBAR_ROW_SPRITES[armed];
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                body,
                SIDEBAR_ROW_SPRITE_X.0,
                y,
                true,
            );
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                well,
                SIDEBAR_ROW_SPRITE_X.1,
                y,
                true,
            );
        }
    }

    /// The FUN_004072bf select-portrait subset [sec 6c.8d]: slot k
    /// within the spawned squad draws its 48x48 portrait (0x12+k
    /// when selected, 0x15+k otherwise) at (0x1E7+0x32*k, 5), gated
    /// by the target's alive word (the HP gate needs the unmodeled
    /// +0x78 field). Every present. Presentation half only.
    fn draw_sidebar_portraits(&mut self) {
        let selected = self.sidebar.selected;
        for (slot, alive) in self.sim.robots().iter().map(|r| r.alive).enumerate() {
            if slot >= 3 || !alive {
                continue;
            }
            let id = if slot == selected {
                SIDEBAR_PORTRAIT_IDS.0
            } else {
                SIDEBAR_PORTRAIT_IDS.1
            } + slot as u16;
            let (x0, pitch, y) = SIDEBAR_PORTRAIT_XY;
            draw_sprite(
                &mut self.plane,
                640,
                &self.general,
                id,
                x0 + pitch * slot as i32,
                y,
                true,
            );
        }
    }

    /// The presentation plane under the mission's OWN palette: the
    /// folded GAMEPAL staged with the mission (the host palette no
    /// longer stands in — DESIGN-GAME sec 11 PRESENT, GAMEPAL unit).
    pub(crate) fn plane(&mut self) -> Option<Plane<'_>> {
        let palette = self.palette;
        self.present().map(|pixels| Plane {
            w: 640,
            h: 480,
            pixels,
            palette,
        })
    }

    /// The folded GAMEPAL palette the plane presents under (gate
    /// seam).
    pub fn palette(&self) -> &[Vga6; 256] {
        &self.palette
    }

    /// The staged GAMEGFX\SMLFONT.BIN bytes (`_DAT_004ede7c`) — the
    /// sidebar text bank, staged for the row text slice (the
    /// name/count draws wait on the type table, RE-EXW-SIM 6c.8).
    pub fn sidebar_font_bank(&self) -> &[u8] {
        &self.smlfont
    }

    /// The selected sidebar squad slot (`DAT_0046cbdc`, sec 6c.2).
    pub fn sidebar_selected(&self) -> usize {
        self.sidebar.selected
    }

    /// The sidebar redraw countdown (`DAT_0046ccec`, sec 6c.5):
    /// producers set 2, each present decrements while nonzero.
    pub fn sidebar_redraw(&self) -> i32 {
        self.sidebar.redraw
    }

    /// The robot's order-bits word (+0x6E, sec 6c.3/6c.6): bit i =
    /// order i active.
    pub fn order_bits(&self, robot: usize) -> u16 {
        self.sidebar.order_bits.get(robot).copied().unwrap_or(0)
    }

    /// Install a robot's order-availability mask (the +0x38+8k gate
    /// words, sec 6c.6) — the host seam standing in for the
    /// runtime-loaded per-type order table at 0x4de664 until its
    /// file source is decoded; the default is all-7 [design]. The
    /// robot's order bits keep their current value (the EXW writes
    /// the mask once at spawn).
    pub fn set_order_availability(&mut self, robot: usize, mask: u8) {
        if let Some(slot) = self.sidebar.order_avail.get_mut(robot) {
            *slot = mask;
        }
    }
}

/// A minimal hermetic mission for host tests: a 4x4 type-1 deck
/// (CGR slot 0 raw 0x1F heights), one MRK record at (1, 1, z-level
/// 1), an empty BIN (no terrain sprites draw), a zero LNK (words
/// stay put), SINTABLE-shaped angle words, an empty DANTE bank, a
/// 770-B synth GAMEPAL, and synth sidebar banks (GENERAL: tiny
/// solid sprites for the portraits 0x12..0x17 + row chrome
/// 0x47/0x49/0x4A/0x4C; SMLFONT: 63 empty glyphs — no text draws).
/// Files in [`MissionScene::stage`] parameter order (MRK 5th,
/// GENERAL + SMLFONT after GAMEPAL).
#[cfg(test)]
pub(crate) fn synth_mission_files() -> Vec<Vec<u8>> {
    let w = 4usize;
    let h = 4usize;
    let n = w * h;
    // DAT: header + 8 planes, plane 0 all type 1 (deck).
    let mut dat = vec![0u8; 4 + 8 * n];
    dat[0..2].copy_from_slice(&(w as u16).to_le_bytes());
    dat[2..4].copy_from_slice(&(h as u16).to_le_bytes());
    for b in dat[4..4 + n].iter_mut() {
        *b = 1;
    }
    // PAD: empty.
    let pad = Vec::new();
    // CGR: 1 sprite, dir offset 0 -> body at 2+4*0+0+6... the
    // loader rule is body = dir + 4*s + 8 = 8; 2 pad bytes then
    // 1024 x 0x1F.
    let mut cgr = Vec::new();
    cgr.extend_from_slice(&1u16.to_le_bytes());
    cgr.extend_from_slice(&0u32.to_le_bytes());
    cgr.extend_from_slice(&[0u8; 2]);
    cgr.extend_from_slice(&[0x1Fu8; 1024]);
    // TOT: header + 8 zero u16 planes.
    let mut tot = vec![0u8; 4 + 16 * n];
    tot[0..2].copy_from_slice(&(w as u16).to_le_bytes());
    tot[2..4].copy_from_slice(&(h as u16).to_le_bytes());
    // BIN/LNK/MRK: empty bank, zeroed link table, one record.
    let bin = Vec::new();
    let mut lnk = vec![0u8; 0x4000];
    lnk.fill(0);
    let mut mrk = Vec::new();
    for word in [1u32, 1, 1, 1] {
        mrk.extend_from_slice(&word.to_le_bytes());
    }
    // SINTABLE: 256 words, thresholds ascending over 2..66.
    let mut sintable = Vec::new();
    for i in 0..256u16 {
        let t = if (2..66).contains(&i) {
            0x0647 + (i as u32 - 2) * (0x7FF5 - 0x0647) / 63
        } else {
            0
        } as u16;
        sintable.extend_from_slice(&t.to_le_bytes());
    }
    // DANTE: empty bank (entity flushes draw nothing).
    let dante = Vec::new();
    // GAMEPAL: 770-B synth palette — entry i carries the 6-bit
    // components (i*3, i*3+1, i*3+2) & 0x3F so the fold is visible.
    let mut gamepal = vec![0u8; 2];
    for i in 0..256usize {
        for c in 0..3usize {
            gamepal.push(((i * 3 + c) & 0x3F) as u8);
        }
    }
    assert_eq!(gamepal.len(), 770);
    // GENERAL: a synth UI bank — tiny 2x2 solid sprites for the
    // portraits (0x12..0x17) and the row chrome (0x47/0x49 body +
    // 0x4A/0x4C well), distinct pixel values per id so tests can
    // tell armed from unarmed rows; everything else empty.
    let general_count = 0x4Du16;
    let mut general = vec![0u8; 2 + 4 * general_count as usize];
    general[0..2].copy_from_slice(&general_count.to_le_bytes());
    let put = |bank: &mut Vec<u8>, id: u16, color: u8| {
        let entry = 2 + 4 * id as usize;
        let start = bank.len();
        bank.extend_from_slice(&3u16.to_le_bytes()); // flags: hotspot + RLE
        bank.extend_from_slice(&[0, 0, 0, 0]); // yhot, xhot
        bank.extend_from_slice(&2u16.to_le_bytes()); // w
        bank.extend_from_slice(&2u16.to_le_bytes()); // h
                                                     // RLE: two rows of literal-2 solid color.
        bank.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        bank.extend_from_slice(&[0x02, 0x00, color, color, 0x00, 0xC0]);
        let off = (start as u32) - entry as u32;
        bank[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
    };
    for id in 0x12u16..0x18 {
        put(&mut general, id, 0x20 + id as u8);
    }
    for (id, color) in [(0x47u16, 0xA7u8), (0x49, 0xB9), (0x4A, 0xCA), (0x4C, 0xDC)] {
        put(&mut general, id, color);
    }
    // SMLFONT: 63 entries, all empty (no text draws this slice).
    let mut smlfont = vec![0u8; 2 + 4 * 63];
    smlfont[0..2].copy_from_slice(&63u16.to_le_bytes());
    vec![
        tot, dat, pad, cgr, mrk, bin, lnk, sintable, dante, gamepal, general, smlfont,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged(markers: &[(i32, i32, i32)]) -> MissionScene {
        let f = synth_mission_files();
        MissionScene::stage(
            &f[0], &f[1], &f[2], &f[3], &f[4], &f[5], &f[6], &f[7], &f[8], &f[9], &f[10], &f[11],
            0, None, markers,
        )
        .expect("synth mission stages")
    }

    #[test]
    fn robots_per_player_table() {
        assert_eq!(robots_per_player(0), 1);
        assert_eq!(robots_per_player(1), 1);
        assert_eq!(robots_per_player(2), 1);
        assert_eq!(robots_per_player(3), 2);
        assert_eq!(robots_per_player(4), 3);
        assert_eq!(robots_per_player(7), 1);
    }

    #[test]
    fn mission_names_follow_the_zone_arithmetic() {
        assert_eq!(
            mission_asset_names(0, 1),
            vec![
                "ZONEA/MISSION1.TOT",
                "ZONEA/MISSION1.DAT",
                "ZONEA/MISSION1.PAD",
                "ZONEA/MISSIONA.CGR",
                "ZONEA/MISSIONA.BIN",
                "ZONEA/MISSIONA.LNK",
                "SINTABLE.BIN",
                "DANTE.BIN",
                "GAMEPAL.PAL",
                "GENERAL.BIN",
                "SMLFONT.BIN",
                "ZONEA/MISSION1.MRK",
            ]
        );
        assert_eq!(
            mission_asset_names(2, 5)[3..6],
            [
                "ZONEC/MISSIONC.CGR",
                "ZONEC/MISSIONC.BIN",
                "ZONEC/MISSIONC.LNK"
            ]
        );
        assert_eq!(zone_for_stage(1), 0);
        assert_eq!(zone_for_stage(7), 6);
        assert_eq!(zone_for_stage(8), 6, "endgame stays at zone G");
        assert_eq!(mission_number_for_mask(0), 1);
        assert_eq!(mission_number_for_mask(0b0111), 4);
    }

    #[test]
    fn stage_spawns_mrk_robots_and_fixes_camera_on_activate() {
        let mut m = staged(&[(3, 1, 1)]);
        assert!(!m.is_active(), "staged inert");
        assert_eq!(m.sim().robots().len(), 2, "MRK[0] + staged marker");
        // Inert tick + present: nothing happens.
        m.tick(&InputFrame::default());
        assert!(m.present().is_none());
        m.activate();
        assert!(m.is_active());
        // Camera at robot 0 Q5: tile (1,1) + 0xF00 center -> Q5
        // (1*32+15, 1*32+15) = (47, 47).
        assert_eq!(m.camera(), (47, 47));
    }

    #[test]
    fn bad_bytes_error_without_panic() {
        let f = synth_mission_files();
        // (dat, mrk, sintable, gamepal) override slices, else the
        // synth files.
        let try_stage = |dat: &[u8], mrk: &[u8], sintable: &[u8], gamepal: &[u8]| {
            MissionScene::stage(
                &f[0],
                dat,
                &f[2],
                &f[3],
                mrk,
                &f[5],
                &f[6],
                sintable,
                &f[8],
                gamepal,
                &f[10],
                &f[11],
                0,
                None,
                &[],
            )
        };
        assert!(matches!(
            try_stage(&f[1][..10], &f[4], &f[7], &f[9]),
            Err(GameError::BadMissionAsset {
                what: "DAT/PAD/CGR",
                ..
            })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4][..8], &f[7], &f[9]),
            Err(GameError::BadMissionAsset { what: "MRK", .. })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4], &f[7][..100], &f[9]),
            Err(GameError::BadMissionAsset {
                what: "SINTABLE",
                ..
            })
        ));
        assert!(matches!(
            try_stage(&f[1], &f[4], &f[7], &f[9][..100]),
            Err(GameError::BadMissionAsset {
                what: "GAMEPAL",
                ..
            })
        ));
    }

    #[test]
    fn click_seam_arms_at_the_projected_robot() {
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        // Robot 0 projected at camera == its own position:
        // dx=dy=0, colAdj = ((15)-(15)+0x20)&0x3F = 0x20, rowAdj =
        // (15+15)>>1 = 15, z settles 31 -> sx = 0x130 = 304,
        // sy = 0x10C + 15 - 31 = 252.
        assert_eq!(m.sim().robots()[0].q5(), m.camera());
        assert_eq!(m.sim().robots()[0].z, 31, "the deck settles the spawn");
        fn click_at(m: &mut MissionScene, x: i32, y: i32) {
            // The cursor integrates DELTAS (clamped 0..=639/0..=479);
            // aim it at the absolute (x, y) with one move tick.
            let (cx, cy) = m.cursor();
            m.tick(&InputFrame {
                mouse_dx: (x - cx) as i16,
                mouse_dy: (y - cy) as i16,
                mouse_buttons: 0,
                ..InputFrame::default()
            });
            m.tick(&InputFrame {
                mouse_buttons: 1,
                ..InputFrame::default()
            });
        }
        // Sidebar click: no arm.
        click_at(&mut m, 600, 252);
        assert!(m.sim().order().is_none(), "sidebar click is a no-op");
        // Far click: no arm.
        click_at(&mut m, 10, 10);
        assert!(m.sim().order().is_none(), "no robot under the pointer");
        // On-robot click: armed at robot 0, snapped to its tile
        // origin, state 3 [FUN_004247b5].
        click_at(&mut m, 304, 252);
        let order = m.sim().order().expect("armed");
        assert_eq!(order.tile.0, 1, "the order tile is robot 0's tile");
        assert_eq!(m.sim().robots()[0].state, 3);
        assert_eq!(m.sim().robots()[0].pos_x, 1 << 13, "snap to tile origin");
    }

    #[test]
    fn present_blits_the_window_at_origin_once_per_call() {
        let mut m = staged(&[]);
        assert!(m.present().is_none(), "inert before activate");
        m.activate();
        let plane = m.present().expect("active presents");
        assert_eq!(plane.len(), 640 * 480);
        // The synth TOT has no words and the BIN is empty: the
        // VIEWPORT stays all zero but the LNK walk counted one
        // render...
        assert!(plane[..640 * 480]
            .chunks_exact(640)
            .all(|r| r[..480].iter().all(|&b| b == 0)));
        assert_eq!(m.render_count(), 1);
        m.present();
        assert_eq!(m.render_count(), 2);
    }

    #[test]
    fn plane_carries_the_folded_gamepal() {
        // The mission plane presents under its OWN palette: the
        // folded GAMEPAL entry i = ((i*3+c) & 0x3F) for the synth
        // file (the fold keeps the 6-bit file values exactly).
        let mut m = staged(&[]);
        m.activate();
        let mut want = [[0u8; 3]; 256];
        for (i, entry) in want.iter_mut().enumerate() {
            *entry = [
                ((i * 3) & 0x3F) as u8,
                ((i * 3 + 1) & 0x3F) as u8,
                ((i * 3 + 2) & 0x3F) as u8,
            ];
        }
        assert_eq!(m.palette(), &want);
        let plane = m.plane().expect("active plane");
        assert_eq!(plane.palette, want, "the plane palette IS GAMEPAL");
    }

    /// Sidebar click helper: aim + click, mirroring the EXW
    /// mouse_l_click -> sidebar_control dispatch (sec 6c).
    fn sidebar_click(m: &mut MissionScene, x: i32, y: i32) {
        let (cx, cy) = m.cursor();
        m.tick(&InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            mouse_buttons: 0,
            ..InputFrame::default()
        });
        m.tick(&InputFrame {
            mouse_buttons: 1,
            ..InputFrame::default()
        });
    }

    #[test]
    fn sidebar_select_strips_follow_the_asm_gates() {
        // Two-robot squad (MRK[0] + one staged marker): strips 0/1
        // select, strip 2 is gated off (DAT_0046cbd8 analog < 3),
        // out-of-strip clicks keep state [sec 6c.2, asm
        // 0x40d220..0x40d3b0].
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        assert_eq!(m.sidebar_selected(), 0, "slot 0 selected at spawn");
        assert_eq!(m.order_bits(0), 1, "spawn default = 1<<first available");
        assert_eq!(
            m.sidebar_redraw(),
            2,
            "activate arms the initial draw (MissionShell 0x447c74)"
        );
        // Strip 1 (x 0x219..0x249): selects slot 1, redraw = 2.
        sidebar_click(&mut m, 0x219, 5);
        assert_eq!(m.sidebar_selected(), 1);
        assert_eq!(m.sidebar_redraw(), 2);
        // Strip 2 with a 2-robot squad: gated off, nothing changes.
        m.sidebar.redraw = 0;
        sidebar_click(&mut m, 0x24B, 0x35);
        assert_eq!(m.sidebar_selected(), 1, "slot 2 gated (squad < 3)");
        assert_eq!(m.sidebar_redraw(), 0, "no fire -> no redraw");
        // Strip bounds: x = 0x218 is between strips 0/1 -> no-op;
        // y = 0x36 is one past the strip bottom -> no-op.
        sidebar_click(&mut m, 0x218, 5);
        sidebar_click(&mut m, 0x1E7, 0x36);
        assert_eq!(m.sidebar_selected(), 1);
        assert_eq!(m.sidebar_redraw(), 0);
        // Strip 0 bottom-left corner (0x1E7, 5) is INSIDE [asm
        // inclusive]: fires, back to slot 0.
        sidebar_click(&mut m, 0x1E7, 5);
        assert_eq!(m.sidebar_selected(), 0);
        assert_eq!(m.sidebar_redraw(), 2);
    }

    #[test]
    fn sidebar_order_rows_toggle_the_selected_robot() {
        // Row click on the SELECTED robot's bits word: row = (y -
        // 0x57)/14 clamp 6, gate = availability, toggle + redraw = 2
        // [sec 6c.4, asm 0x40d659..0x40d712].
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        // Select slot 1, then click row 0 of the order rect.
        sidebar_click(&mut m, 0x219, 5);
        assert_eq!(m.order_bits(1), 1, "robot 1 spawn default");
        sidebar_click(&mut m, 0x200, 0x57);
        assert_eq!(m.order_bits(1), 0, "bit 0 toggled off");
        assert_eq!(m.order_bits(0), 1, "robot 0 untouched");
        assert_eq!(m.sidebar_redraw(), 2);
        // Row boundaries: y 0x57..0x64 = row 0, 0x65 = row 1;
        // y 0xB8 = row 6 (in), 0xB9 = out; x 0x1E9 in / 0x1E8 out /
        // 0x275 in / 0x276 out.
        sidebar_click(&mut m, 0x200, 0x64);
        assert_eq!(m.order_bits(1), 1, "y=0x64 still row 0");
        sidebar_click(&mut m, 0x200, 0x65);
        assert_eq!(m.order_bits(1) >> 1 & 1, 1, "y=0x65 is row 1");
        sidebar_click(&mut m, 0x275, 0xB8);
        assert_eq!(m.order_bits(1) >> 6 & 1, 1, "y=0xB8 is row 6");
        m.sidebar.redraw = 0;
        sidebar_click(&mut m, 0x275, 0xB9);
        sidebar_click(&mut m, 0x1E8, 0x57);
        sidebar_click(&mut m, 0x276, 0x57);
        assert_eq!(m.order_bits(1), 0b1000011, "out-of-rect clicks no-op");
        assert_eq!(m.sidebar_redraw(), 0);
        // Availability gate: clear row 3 -> its click neither toggles
        // nor redraws (the +0x38+8k gate word == 0 path).
        m.set_order_availability(1, 0x7F & !(1 << 3));
        sidebar_click(&mut m, 0x200, 0x57 + 3 * 14);
        assert_eq!(m.order_bits(1), 0b1000011, "gated row untouched");
        assert_eq!(m.sidebar_redraw(), 0, "gate fail -> no redraw");
    }

    #[test]
    fn sidebar_redraw_counts_down_per_present() {
        // DAT_0046ccec: producers set 2, the draw tail decrements
        // once per frame while nonzero [sec 6c.5, asm 0x407205];
        // activate arms the INITIAL draw with 2 (0x447c74, 6c.8e).
        let mut m = staged(&[]);
        m.activate();
        assert_eq!(m.sidebar_redraw(), 2, "the entry trigger");
        m.present().expect("active presents");
        assert_eq!(m.sidebar_redraw(), 1);
        m.present();
        assert_eq!(m.sidebar_redraw(), 0);
        m.present();
        assert_eq!(m.sidebar_redraw(), 0, "sticks at zero");
    }

    #[test]
    fn sidebar_art_draws_rows_and_portraits() {
        // The FUN_00408403 row chrome + the FUN_004072bf portraits
        // [sec 6c.8]: synth GENERAL sprites carry distinct colors
        // (0x12+k -> 0x32+k portraits, 0x47/0x4A armed, 0x49/0x4C
        // unarmed), so the plane pins which sprite landed where.
        let mut m = staged(&[(3, 1, 1)]);
        m.activate();
        let plane = m.present().expect("the entry frame draws (countdown 2)");
        let px = |p: &[u8], x: usize, y: usize| p[y * 640 + x];
        // Portraits: 2-robot squad -> slots 0 (selected, 0x12 ->
        // color 0x32) and 1 (not selected, 0x16 -> 0x36) at
        // (0x1E7,5) and (0x219,5); slot 2 gated (squad < 3).
        assert_eq!(px(plane, 0x1E7, 5), 0x32);
        assert_eq!(px(plane, 0x219, 5), 0x36);
        assert_eq!(px(plane, 0x24B, 5), 0, "slot 2 gated (squad < 3)");
        // Rows (robot 0 selected, avail 0x7F, bits = 1): row 0
        // ARMED -> body 0x47 (0xA7) at (0x1EB,0x59) + well 0x4A
        // (0xCA) at (0x25A,0x59); rows 1..6 unarmed -> 0xB9/0xDC.
        assert_eq!(px(plane, 0x1EB, 0x59), 0xA7, "row 0 armed body");
        assert_eq!(px(plane, 0x25A, 0x59), 0xCA, "row 0 armed well");
        for i in 1..7 {
            let y = 0x59 + 14 * i;
            assert_eq!(px(plane, 0x1EB, y as usize), 0xB9, "row {i} unarmed body");
            assert_eq!(px(plane, 0x25A, y as usize), 0xDC, "row {i} unarmed well");
        }
        // Gated row: clear availability bit 6 -> after a redraw
        // trigger, row 6 draws nothing (the plane keeps its old
        // pixels, so wipe the row first).
        m.set_order_availability(0, 0x7F & !(1 << 6));
        for y in 0x59 + 14 * 6..0x59 + 14 * 6 + 2 {
            for x in 0x1EB..0x1EB + 2 {
                m.plane[y as usize * 640 + x] = 0;
            }
        }
        m.sidebar.redraw = 2;
        m.present();
        assert_eq!(
            px(&m.plane, 0x1EB, (0x59 + 14 * 6) as usize),
            0,
            "row 6 gated"
        );
        // Selecting slot 1 moves the rows to robot 1 and the armed
        // portrait to slot 1 (bit 0 default armed).
        sidebar_click(&mut m, 0x219, 5);
        m.present();
        assert_eq!(px(&m.plane, 0x1E7, 5), 0x35, "slot 0 now unselected (0x15)");
        assert_eq!(px(&m.plane, 0x219, 5), 0x33, "slot 1 selected (0x13)");
        assert_eq!(px(&m.plane, 0x1EB, 0x59), 0xA7, "robot 1 row 0 armed body");
        // The countdown drains: present again (1 -> 0, still draws),
        // wipe row 0, then one more present draws NO rows.
        m.present();
        for y in 0x59..0x59 + 2 {
            for x in 0x1EB..0x1EB + 2 {
                m.plane[y as usize * 640 + x] = 0;
            }
        }
        m.present();
        assert_eq!(px(&m.plane, 0x1EB, 0x59), 0, "no countdown -> no row draw");
        // The staged font bank rides along (63 synth glyphs) — the
        // text slice's input.
        assert_eq!(
            u16::from_le_bytes([m.sidebar_font_bank()[0], m.sidebar_font_bank()[1]]),
            63
        );
    }

    #[test]
    fn sidebar_state_never_reaches_the_sim_hash() {
        // D17 split pin: identical tick counts + a sidebar click vs a
        // dead sidebar click -> identical sim state hashes (the
        // sidebar half is presentation-only).
        let mut a = staged(&[(3, 1, 1)]);
        let mut b = staged(&[(3, 1, 1)]);
        a.activate();
        b.activate();
        // a: strip-1 select + row-0 toggle; b: clicks in the sidebar
        // dead zone (map-toggle rect, sec 6c.1 — wired regions are
        // disjoint from it).
        sidebar_click(&mut a, 0x219, 5);
        sidebar_click(&mut a, 0x200, 0x57);
        sidebar_click(&mut b, 0x230, 0x1C0);
        sidebar_click(&mut b, 0x230, 0x1C0);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_ne!(a.order_bits(1), b.order_bits(1), "sidebar did change");
    }
}
