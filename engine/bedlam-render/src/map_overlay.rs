//! The strategic-map overlay family [RE-EXW-SIM sec 7e, decoded
//! 2026-08-21]: FUN_004089b1's draw + FUN_00402ab8's 4×4 territory
//! stamp + FUN_00408dcc's ring stamper.
//!
//! EXW anchors (all [verified] decompile + asm + shipped bytes):
//! - FUN_004089b1@0x4089b1: clear the presented 480×480, blit
//!   TABLE.BIN image 0 (480×480 RLE) at (0,0), stamp every
//!   (row, col, z) whose LNK-resolved word is nonzero — mask
//!   `MIN[cw*16]`, color `MAPTRAN[variant[tile]][mask byte]`,
//!   dest `(0x80+row+col−2z)*640 + (0xf0−2row+2col)` — then the
//!   robot markers GENERAL.BIN 0x55 (selected) / 0x56 at
//!   `x = 2(tx−ty)+0xf0−0xc`, `y = tx+ty+0x80−0x1e−(z>>4)`. The PAD
//!   /order markers 0x57..0x59 need the order staging (unmodeled)
//!   and are deliberately NOT drawn (never-invent).
//! - FUN_00408dcc@0x408dcc: per robot, max-stamp an 11×11-tile
//!   square (col outer, row inner, ring index advancing per
//!   IN-BOUNDS tile) with the Chebyshev-diamond rings at 0x454cf8
//!   (7 center → 1 corners).
//! - The mirror words advance one LNK step per overlay draw (the
//!   same `word = LNK[word]` walk the terrain pass does —
//!   idempotent on ZONEA's identity LNK).

use crate::mission_view::MissionView;
use crate::ui_bank::draw_sprite;

/// The 121-entry territory ring table at 0x454cf8 [PE bytes
/// verified: 11×11 Chebyshev-diamond distance rings, 7 at the
/// center falling to 1 at the corners; consumed in scan order with
/// the index advancing per IN-BOUNDS tile, asm 0x408e5c..0x408e8f].
pub const TERRITORY_RINGS: [u8; 121] = [
    1, 2, 2, 3, 4, 4, 4, 3, 2, 2, 1, // row 0
    2, 3, 4, 4, 5, 5, 5, 4, 4, 3, 2, //
    2, 4, 4, 5, 5, 6, 5, 5, 4, 4, 2, //
    2, 3, 4, 5, 5, 6, 6, 6, 5, 4, 3, //
    4, 5, 5, 6, 6, 7, 7, 7, 6, 5, 4, //
    4, 5, 6, 6, 7, 7, 7, 6, 6, 5, 4, //
    4, 5, 5, 6, 6, 7, 7, 7, 6, 5, 4, //
    2, 3, 4, 5, 5, 6, 6, 6, 5, 4, 3, //
    2, 4, 4, 5, 5, 6, 5, 5, 4, 4, 2, //
    2, 3, 4, 4, 5, 5, 5, 4, 4, 3, 2, //
    1, 2, 2, 3, 4, 4, 4, 3, 2, 2, 1, //
];

/// Overlay stamp lattice origin: tile (row, col, z) lands at
/// `(row', col') = (0x80+row+col−2z, 0xf0−2row+2col)` [asm
/// 0x4089e8..0x408a5e + FUN_00402ab8's `EDX*0x280+ECX` dest].
pub const MAP_LATTICE: (i32, i32, i32, i32) = (0x80, 0xF0, 2, 2);
/// Robot-marker anchor offsets [asm 0x408b60..0x408b8a]: the marker
/// lifts `0x1e + z>>4` rows above and `0xc` cols left of the tile
/// lattice cell (the sprite's own footprint centering).
pub const MAP_MARKER_OFFSETS: (i32, i32, i32) = (0x1E, 0xC, 4);
/// Robot marker sprite ids from GENERAL.BIN [7e.1d]: 0x55 the
/// selected robot (SP mode 0: slot == DAT_0046cbdc), 0x56 others.
pub const MAP_MARKER_SPRITES: (u16, u16) = (0x55, 0x56);

/// One robot marker's inputs [7e.1d]: the Q13 world position
/// (record +0x00/+0x04), the Q5 floor z (+0x08), and whether this
/// robot is the selected squad slot.
#[derive(Debug, Clone, Copy)]
pub struct OverlayRobot {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub selected: bool,
}

/// The strategic-map overlay state: the mission's `.MIN` 4×4 mask
/// bank (the ArenaAlloc(0x7530) buffer at 0x4edd9c), the 8 loaded
/// `GAMEGFX\MAPTRAN{0..7}.TRN` 256-byte color ramps (pointer slots
/// 0x4dd464+4i), and the per-tile territory variant bytes (the
/// 0x4c420c array — presentation-half state, never hashed).
#[derive(Debug, Clone)]
pub struct MapOverlay {
    min: Vec<u8>,
    ramps: Vec<[u8; 256]>,
    variant: Vec<u8>,
    width: i32,
    height: i32,
}

impl MapOverlay {
    /// Build from the staged bytes: `min` — the mission `.MIN`
    /// (load_mission's `.MIN` concat load, FUN_0041dbed/41cc7f);
    /// `maptran` — the eight 256-byte ramps in slot order. `min` may
    /// be any length (short banks stamp nothing past their end — the
    /// charter's no-panic guard; the EXW arena is 30000 B). Returns
    /// `None` unless exactly eight 256-byte ramps stage.
    pub fn new(min: &[u8], maptran: &[&[u8]], width: i32, height: i32) -> Option<Self> {
        if maptran.len() != 8 || maptran.iter().any(|r| r.len() != 256) {
            return None;
        }
        if width <= 0 || height <= 0 {
            return None;
        }
        let mut ramps = Vec::with_capacity(8);
        for r in maptran {
            let mut ramp = [0u8; 256];
            ramp.copy_from_slice(r);
            ramps.push(ramp);
        }
        Some(MapOverlay {
            min: min.to_vec(),
            ramps,
            variant: vec![0; (width * height) as usize],
            width,
            height,
        })
    }

    /// The map size in tiles.
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// The territory variant byte at a row-major tile (0 = no robot
    /// proximity — the MAPTRAN0 ramp).
    pub fn variant(&self, tile: usize) -> u8 {
        self.variant.get(tile).copied().unwrap_or(0)
    }

    /// FUN_00408dcc — stamp the 11×11 territory rings around a
    /// robot's tile [asm 0x408dcc..0x408e98]: scan col-outer /
    /// row-inner over `[tx−5, tx+5] × [ty−5, ty+5]`, and for every
    /// IN-BOUNDS tile apply `variant = max(variant, ring[k])` with
    /// the ring index `k` advancing per accepted tile (the clipped
    /// scan consumes the ring in scan order, exactly the asm's
    /// `edi`/`ebx` walk). Values of 0 in the table would skip the
    /// write but still consume the index [the `je 0x408e8b` path].
    pub fn stamp_territory(&mut self, tx: i32, ty: i32) {
        let mut k = 0usize;
        for cx in (tx - 5)..=(tx + 5) {
            for ry in (ty - 5)..=(ty + 5) {
                if cx < 0 || cx >= self.width || ry < 0 || ry >= self.height {
                    continue;
                }
                let ring = TERRITORY_RINGS[k.min(TERRITORY_RINGS.len() - 1)];
                k += 1;
                if ring == 0 {
                    continue;
                }
                let tile = (ry * self.width + cx) as usize;
                if ring > self.variant[tile] {
                    self.variant[tile] = ring;
                }
            }
        }
    }

    /// FUN_004089b1's draw into the caller's plane (the presented
    /// 480×480 at stride 640 in the EXW backbuffer; the engine
    /// passes the mission plane's viewport half) [7e.1a–d]:
    ///
    /// 1. the caller clears the region first (the 0x4b000 rep-stos);
    /// 2. `table` (TABLE.BIN) image 0 at (0,0) — the backdrop;
    /// 3. per (row, col, z): `cw = view.overlay_word_step(tile, z)`
    ///    (the LNK advance, memoized back — the same destructive
    ///    walk the terrain pass does); `cw != 0` stamps the 4×4
    ///    mask `min[cw*16]` colored `ramp[variant[tile]][mask]` at
    ///    the lattice cell;
    /// 4. per marker robot: sprite 0x55/0x56 from `general`.
    ///
    /// The PAD/order markers 0x57..0x59 are NOT drawn — their
    /// staging (0x4eaaee/0x4e44f8) is unmodeled (never-invent).
    pub fn draw(
        &self,
        plane: &mut [u8],
        stride: usize,
        view: &mut MissionView,
        table: &[u8],
        general: &[u8],
        robots: &[OverlayRobot],
    ) {
        // Backdrop: FUN_00401e39(TABLE.BIN, image 0, transp=1, 0, 0).
        draw_sprite(plane, stride, table, 0, 0, 0, true);
        // Territory stamps [asm 0x4089e8..0x408ae3].
        let (row0, col0, row_step, col_step) = MAP_LATTICE;
        let rows = (plane.len() / stride) as i32;
        for row in 0..self.height {
            for col in 0..self.width {
                let tile = (row * self.width + col) as usize;
                let variant = self.variant(tile) as usize;
                let ramp = self.ramps.get(variant).unwrap_or(&self.ramps[0]);
                for z in 0..8i32 {
                    let cw = view.overlay_word_step(tile, z as usize);
                    if cw == 0 {
                        continue;
                    }
                    let base = usize::from(cw) * 16;
                    if base + 16 > self.min.len() {
                        continue; // short bank: nothing to stamp
                    }
                    let py = row0 + row + col - row_step * z;
                    let px = col0 - col_step * row + col_step * col;
                    for r in 0..4i32 {
                        for c in 0..4i32 {
                            let m = self.min[base + (r * 4 + c) as usize];
                            if m == 0 {
                                continue;
                            }
                            let (x, y) = (px + c, py + r);
                            if x >= 0 && y >= 0 && x < stride as i32 && y < rows {
                                plane[y as usize * stride + x as usize] = ramp[m as usize];
                            }
                        }
                    }
                }
            }
        }
        // Robot markers [asm 0x408ae5..0x408bb6].
        let (sel, other) = MAP_MARKER_SPRITES;
        let (lift, left, zshift) = MAP_MARKER_OFFSETS;
        for robot in robots {
            let tx = ((robot.x >> 8) + 0x10) >> 5;
            let ty = ((robot.y >> 8) + 0x10) >> 5;
            let id = if robot.selected { sel } else { other };
            let px = 2 * tx - 2 * ty + col0 - left;
            let py = tx + ty + row0 - lift - (robot.z >> zshift);
            draw_sprite(plane, stride, general, id, px, py, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1-tile TOT with plane word 5 + a 256-word LNK with
    /// LNK[5] = 3 (everything else 0).
    fn tiny_view() -> MissionView {
        let n = 1usize;
        let mut tot = vec![0u8; 4 + 16 * n];
        tot[0..2].copy_from_slice(&1u16.to_le_bytes());
        tot[2..4].copy_from_slice(&1u16.to_le_bytes());
        tot[4..6].copy_from_slice(&5u16.to_le_bytes());
        let dat = vec![0u8; 8 * n];
        let mut lnk = vec![0u8; 0x4000];
        lnk[10..12].copy_from_slice(&3u16.to_le_bytes());
        MissionView::from_mission_bytes(&tot, &dat, &[], &lnk).expect("tiny view")
    }

    #[test]
    fn rings_shape_matches_pe_bytes() {
        assert_eq!(TERRITORY_RINGS[5 * 11 + 5], 7, "center ring");
        assert_eq!(TERRITORY_RINGS[0], 1, "corner ring");
        assert_eq!(TERRITORY_RINGS[120], 1, "far corner ring");
        assert_eq!(TERRITORY_RINGS[55], 4, "edge mid ring");
    }

    #[test]
    fn territory_stamp_is_max_wins_rings() {
        let zero = [0u8; 256];
        let empty_ramps: Vec<&[u8]> = vec![&zero; 8];
        let mut o = MapOverlay::new(&[], &empty_ramps, 16, 16).expect("overlay");
        o.stamp_territory(8, 8);
        assert_eq!(o.variant(8 * 16 + 8), 7);
        // The table is consumed in SCAN ORDER (col-outer, row-inner):
        // tile (3,3) is the first accepted tile (k=0 -> ring 1) and
        // tile (4,4) is k = 1*11+1 = 12 -> flat[12] = 3.
        assert_eq!(o.variant(3 * 16 + 3), 1);
        assert_eq!(o.variant(4 * 16 + 4), 3);
        // outside the square
        assert_eq!(o.variant(2 * 16 + 2), 0);
        // a second stamp overlapping keeps the max
        o.stamp_territory(10, 8);
        assert_eq!(o.variant(9 * 16 + 8), 7, "overlap takes the max");
    }

    #[test]
    fn territory_stamp_clips_the_ring_scan() {
        let zero = [0u8; 256];
        let empty_ramps: Vec<&[u8]> = vec![&zero; 8];
        let mut o = MapOverlay::new(&[], &empty_ramps, 16, 16).expect("overlay");
        // A robot at the top-left corner: the first 11 accepted tiles
        // consume rings 1..7..1 of row 0, so tile (0, 0) gets ring 1
        // but the CENTER of the clipped scan is NOT the robot tile.
        o.stamp_territory(0, 0);
        assert_eq!(o.variant(0), 1, "corner tile gets the scan's first ring");
        // Scan is col-outer: tile (row 1, col 0) is the second
        // accepted tile (k=1) -> flat[1] = 2; tile (row 0, col 1)
        // lands after the first column's six accepts (k=6) -> 4.
        assert_eq!(o.variant(16), 2);
        assert_eq!(o.variant(1), 4);
        // Peak ring lands mid-scan (accepted #55..61 region)
        let peak = (0..11)
            .flat_map(|r| (0..11).map(move |c| (r, c)))
            .map(|(r, c)| o.variant(r * 16 + c))
            .max();
        // Only 36 tiles are in bounds, so the scan consumes k=0..35
        // — max(flat[0..36]) = 6 (the full square's 7 needs k≥48).
        assert_eq!(peak, Some(6), "the clipped scan stops at ring 6");
    }

    #[test]
    fn draw_stamps_mask_through_ramp_and_advances_lnk() {
        let mut view = tiny_view();
        // MIN: 4 masks; mask 3 = a 4x4 with byte values 1..16.
        let mut min = vec![0u8; 4 * 16];
        for (i, b) in min.iter_mut().enumerate().skip(3 * 16) {
            *b = (i - 3 * 16 + 1) as u8;
        }
        // MAPTRAN ramp 0 (variant 0 far from any robot): entry v ->
        // 0x40 + v (wrapped to u8).
        let ramp0: Vec<u8> = (0..256u32).map(|v| (0x40 + v) as u8).collect();
        let maptran: Vec<&[u8]> = vec![
            &ramp0[..],
            &ramp0,
            &ramp0,
            &ramp0,
            &ramp0,
            &ramp0,
            &ramp0,
            &ramp0,
        ];
        let o = MapOverlay::new(&min, &maptran, 1, 1).expect("overlay");
        // A minimal backdrop bank: one 1x1 raw sprite at id 0 (the
        // record offset is relative to the entry: record = 2+off).
        let mut table = Vec::new();
        table.extend_from_slice(&1u16.to_le_bytes()); // count
        table.extend_from_slice(&4u32.to_le_bytes()); // entry 0: record at 2+4
        table.extend_from_slice(&0u16.to_le_bytes()); // flags: raw
        table.extend_from_slice(&1u16.to_le_bytes()); // w
        table.extend_from_slice(&1u16.to_le_bytes()); // h
        table.push(0xAA); // pixel
        let mut plane = vec![0u8; 640 * 480];
        o.draw(&mut plane, 640, &mut view, &table, &[], &[]);
        // LNK[5] = 3: the stamp is mask 3 at z 0 — lattice
        // (row', col') = (0x80, 0xf0), pixel (r, c) = ramp[mask].
        for r in 0..4i32 {
            for c in 0..4i32 {
                let m = min[3 * 16 + (r * 4 + c) as usize];
                assert_eq!(
                    plane[(0x80 + r) as usize * 640 + (0xF0 + c) as usize],
                    0x40 + m,
                    "stamp pixel ({r},{c})"
                );
            }
        }
        // The mirror word advanced to 3 (and a further step would be
        // LNK[3] = 0 — nothing more stamps).
        assert_eq!(view.word(0, 0), 3);
        // The backdrop blit landed at (0, 0).
        assert_eq!(plane[0], 0xAA);
    }

    #[test]
    fn draw_marks_selected_robot_at_lattice_anchor() {
        let mut view = tiny_view();
        let min = vec![0u8; 4 * 16];
        let zero = [0u8; 256];
        let empty: Vec<&[u8]> = vec![&zero[..]; 8];
        let o = MapOverlay::new(&min, &empty, 1, 1).expect("overlay");
        // A GENERAL bank: sprites 0x55..0x56, each a 1x1 raw sprite
        // with pixel id+1 (the entry offset is relative to the
        // entry, FUN_00401ca2).
        let general_count = 0x57u16;
        let mut general = vec![0u8; 2 + 4 * general_count as usize];
        general[0..2].copy_from_slice(&general_count.to_le_bytes());
        for id in 0x55u16..0x57 {
            let entry = 2 + 4 * id as usize;
            let start = general.len();
            general.extend_from_slice(&0u16.to_le_bytes()); // flags: raw
            general.extend_from_slice(&1u16.to_le_bytes()); // w
            general.extend_from_slice(&1u16.to_le_bytes()); // h
            general.push(id as u8 + 1); // pixel value id+1
            let off = (start as u32) - (entry as u32);
            general[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
        }
        // Robot at Q13 (16 << 13, 16 << 13) -> tile (16, 16); z = 32
        // (Q5, one level) -> z>>4 = 2.
        let robots = [OverlayRobot {
            x: 16 << 13,
            y: 16 << 13,
            z: 32,
            selected: true,
        }];
        let mut plane = vec![0u8; 640 * 480];
        o.draw(&mut plane, 640, &mut view, &[], &general, &robots);
        // tx = ty = 16: the marker x collapses to 0xF0-0xC.
        let px = 0xF0 - 0xC;
        let py = 16 + 16 + 0x80 - 0x1E - 2;
        assert_eq!(
            plane[py as usize * 640 + px as usize],
            0x55 + 1,
            "selected marker sprite 0x55 at the anchored lattice cell"
        );
    }
}
