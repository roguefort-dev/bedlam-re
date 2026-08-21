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
//!   x < 0x1E0, x >= 0x1E0 is the sidebar [sec 6.2];
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
use bedlam_render::Vga6;

use crate::loading::Plane;
use crate::GameError;

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
/// family tail (SINTABLE, DANTE — staged after the mission files in
/// MissionShell, sec 7c header) and the markers]. Names carry the
/// `EDITOR` tree sub-path with '/' separators; the byte source
/// resolves them under `EDITOR/` [see bedlam-shell GameGfxSource].
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
    /// Malformed bytes -> [`GameError::BadMissionAsset`], never a
    /// panic (charter); nothing is mutated on error.
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
            render_count: 0,
            active: false,
        })
    }

    /// Fix the camera at the first robot's Q5 position [DESIGN-GAME
    /// sec 11 LIFECYCLE; the EXW cam pair points at the spawn].
    /// Idempotent.
    pub fn activate(&mut self) {
        if self.active {
            return;
        }
        self.cam_q5 = self.sim.robots().first().map(Robot::q5).unwrap_or((0, 0));
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
    /// seam on a left-button EDGE, then `advance_frame` (the six
    /// unit-manager phases + the order-window tick). Inert until
    /// [`MissionScene::activate`].
    pub fn tick(&mut self, input: &InputFrame) {
        if !self.active {
            return;
        }
        self.cursor.0 = (self.cursor.0 + i32::from(input.mouse_dx)).clamp(0, 639);
        self.cursor.1 = (self.cursor.1 + i32::from(input.mouse_dy)).clamp(0, 479);
        let left = input.mouse_buttons & 0x01;
        if self.prev_buttons == 0 && left != 0 {
            self.click_robot();
        }
        self.prev_buttons = left;
        self.sim.advance_frame();
    }

    /// The robot click seam [DESIGN-GAME sec 11; RE-EXW-SIM sec 6.4
    /// click-on-robot arm]: clicks land in the viewport
    /// (`x < 0x1E0`; the sidebar is out of scope -> no-op), hit-test
    /// every alive robot by the enqueue projection (MISSIONVIEW
    /// sec 5d) inside a 0x20-px box [design: half the 64-px sprite
    /// cell; the EXW walks the sprite outlines ~0x433cbc], nearest
    /// octagonal screen distance wins (ties -> lowest index), and the
    /// order is armed AT that robot (the EXW arms at the clicked
    /// robot's tile — one pending order, spread-assign, state 3).
    fn click_robot(&mut self) {
        if self.cursor.0 >= 0x1E0 {
            return; // sidebar: no-op for this slice
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
    /// canonical (0, 0) of the 640x480 plane (sidebar stays black
    /// this slice). Advances the LNK walk + edge stream once — one
    /// render per host frame (D17 bucket b). Inert until active.
    pub fn present(&mut self) -> Option<&[u8]> {
        if !self.active {
            return None;
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

    /// The presentation plane under the host palette (GAMEPAL is the
    /// next unit — the host palette stands in).
    pub(crate) fn plane(&mut self, host_palette: &[Vga6; 256]) -> Option<Plane<'_>> {
        self.present().map(|pixels| Plane {
            w: 640,
            h: 480,
            pixels,
            palette: *host_palette,
        })
    }
}

/// A minimal hermetic mission for host tests: a 4x4 type-1 deck
/// (CGR slot 0 raw 0x1F heights), one MRK record at (1, 1, z-level
/// 1), an empty BIN (no terrain sprites draw), a zero LNK (words
/// stay put), SINTABLE-shaped angle words, an empty DANTE bank.
/// Files in [`mission_asset_names`] order.
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
    vec![tot, dat, pad, cgr, mrk, bin, lnk, sintable, dante]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staged(markers: &[(i32, i32, i32)]) -> MissionScene {
        let f = synth_mission_files();
        MissionScene::stage(
            &f[0], &f[1], &f[2], &f[3], &f[4], &f[5], &f[6], &f[7], &f[8], 0, None, markers,
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
        let short = &f[1][..10];
        assert!(matches!(
            MissionScene::stage(
                &f[0],
                short,
                &f[2],
                &f[3],
                &f[4],
                &f[5],
                &f[6],
                &f[7],
                &f[8],
                0,
                None,
                &[]
            ),
            Err(GameError::BadMissionAsset {
                what: "DAT/PAD/CGR",
                ..
            })
        ));
        let short_mrk = &f[4][..8];
        assert!(matches!(
            MissionScene::stage(
                &f[0],
                &f[1],
                &f[2],
                &f[3],
                short_mrk,
                &f[5],
                &f[6],
                &f[7],
                &f[8],
                0,
                None,
                &[]
            ),
            Err(GameError::BadMissionAsset { what: "MRK", .. })
        ));
        let short_sin = &f[7][..100];
        assert!(matches!(
            MissionScene::stage(
                &f[0],
                &f[1],
                &f[2],
                &f[3],
                &f[4],
                &f[5],
                &f[6],
                short_sin,
                &f[8],
                0,
                None,
                &[]
            ),
            Err(GameError::BadMissionAsset {
                what: "SINTABLE",
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
        // The synth TOT has no words and the BIN is empty: the plane
        // stays all zero but the LNK walk counted one render.
        assert!(plane.iter().all(|&b| b == 0));
        assert_eq!(m.render_count(), 1);
        m.present();
        assert_eq!(m.render_count(), 2);
    }
}
