//! Mission isometric viewport terrain renderer (P4 render half).
//!
//! Pure reimplementation of the EXW viewport draw chain decoded in
//! docs/RE-EXW-MISSIONVIEW.md:
//!
//! - `init_tiles@00407e11` — the camera-independent 36×36 viewport
//!   tile cache (2:1 iso grid, sticky anchor 21) and the TOT→type-DB
//!   mirror (per tile 8 plane words + 8 seen bytes);
//! - the terrain loop of `FUN_00403938` — 8-layer bottom-up walk with
//!   the 0x5000-per-level dest step, the seen-chase columns, the
//!   off-map per-zone edge sprites (`FUN_00408030`) and the per-frame
//!   LNK animation walk (word → LNK[word], memoized back);
//! - `FUN_00401471` — the BIN sprite codec (directory entry at
//!   `2 + 4*id`, sprite at `entry + u32[entry]`; fmt 0 raw 64×64
//!   skip-0, fmt 1..3 u16 RLE, fmt ≥ 4 u8 RLE; row stride 640);
//! - `FUN_00401107` — the present window (480×480 crop of the
//!   0x64000 buffer).
//!
//! Hermetic: no I/O, no clock, no ambient randomness (the off-map edge
//! variants take the caller's `&mut Pcg32` — the EXW uses the mission
//! RandB there; our bit-stream is the charter T3 stand-in, only the
//! CONSUMPTION SHAPE is mirrored). Everything draws into a caller-owned
//! 0x64000 stride-640 palette-index buffer; nothing here knows about
//! windows or scaling.

use bedlam_core::rng::Pcg32;

/// Tile render buffer stride: 640 px (EXW 0x280 words everywhere).
pub const VIEW_STRIDE: usize = 0x280;

/// Tile render buffer size: 0x64000 bytes [FUN_0041d954 alloc].
pub const VIEW_BUF_LEN: usize = 0x64000;

/// Dest byte step per z level: 32 rows × stride [FUN_00403938].
pub const Z_LEVEL_STEP: i32 = 0x5000;

/// Upper dest cap for tile blits: `0x4ede18 + 0x59b00` [FUN_00403938].
/// Sprite rows may legally extend past it inside the 0x64000 buffer;
/// this crate bounds every write to the buffer instead (charter: no
/// panics; in-bounds pixels identical — the EXW relies on arena slack).
pub const DRAW_CAP: i32 = 0x59B00;

/// Viewport cache window: x in `0..0x260`, y in `0..0x320`
/// [init_tiles, verified].
const CACHE_MAX_X: i32 = 0x260;
const CACHE_MAX_Y: i32 = 0x320;

/// Grid origin and steps of the 36×36 iso cache [init_tiles, verified].
const CACHE_X0: i32 = 0x130;
const CACHE_Y0: i32 = -0x100;
const CACHE_STEP_GX_X: i32 = 0x20;
const CACHE_STEP_GY_X: i32 = -0x20;
const CACHE_STEP_X_Y: i32 = 0x10;

/// The sticky gx anchor of the first in-bounds cache cell:
/// (gx, gy) = (12, 4) ⇒ 12 + 5 = 17 [init_tiles, verified].
const CACHE_ANCHOR: i32 = 17;

/// One cached viewport tile: the screen-space cell that always draws
/// the map tile `(dtile_x + cam_tile_x, dtile_y + cam_tile_y)`
/// [init_tiles, verified]. `buf_off` indexes the 0x64000 buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewEntry {
    /// Byte offset into the tile buffer (before the shake offset).
    pub buf_off: i32,
    /// Tile delta from the camera tile X (sticky anchor 21).
    pub dtile_x: i32,
    /// Tile delta from the camera tile Y (sticky anchor 21).
    pub dtile_y: i32,
}

/// Per-zone off-map edge sprite base [FUN_00408030, verified]:
/// zones 1/2/4/7 → base 0x37, zone 3 → 0x23e, zone 5 → 0x65,
/// zone 6 → fixed 0x2ec, zone 0 → fixed 1; `+ rand(0..9)` where
/// random (the EXW `FUN_0041ec59(9, ..)` shape).
fn edge_sprite_base(zone: i32) -> (i32, u32) {
    match zone {
        1 | 2 | 4 | 7 => (0x37, 9),
        3 => (0x23e, 9),
        5 => (0x65, 9),
        6 => (0x2ec, 0),
        _ => (1, 0),
    }
}

/// A decoded BIN sprite reference (checked, borrowed).
struct Sprite<'a> {
    data: &'a [u8],
}

impl<'a> Sprite<'a> {
    /// Resolve sprite `id` in `bank` [FUN_00401471, verified]:
    /// entry = `2 + 4*id`, sprite = `entry + u32[entry]`.
    fn resolve(bank: &'a [u8], id: u16) -> Option<Sprite<'a>> {
        let entry = 2usize + 4 * id as usize;
        let off = u32::from_le_bytes(bank.get(entry..entry + 4)?.try_into().ok()?) as usize;
        let start = entry.checked_add(off)?;
        let data = bank.get(start..)?;
        Some(Sprite { data })
    }

    /// Header `{fmt, dy, dx, gate, rows}`; `None` when the gate word is
    /// zero (the EXW returns without drawing).
    fn header(&self) -> Option<(u16, i32, i32, i32)> {
        if self.data.len() < 10 {
            return None;
        }
        let word = |i: usize| u16::from_le_bytes([self.data[2 * i], self.data[2 * i + 1]]) as i32;
        let fmt = word(0);
        let dy = word(1);
        let dx = word(2);
        let gate = word(3);
        let rows = word(4);
        if gate == 0 || rows == 0 {
            return None;
        }
        Some((fmt as u16, dy, dx, rows))
    }

    /// Decode RLE rows, calling `pixel(row, col, byte)` for every
    /// non-zero palette index [FUN_00401471, verified]. Only the two
    /// RLE families the shipped BINs use are decoded; fmt 0 (raw
    /// 64×64) is included for completeness. A decode that runs off
    /// the sprite simply stops (the EXW trusts the data; we don't).
    fn for_each_pixel(&self, mut pixel: impl FnMut(i32, i32, u8)) -> Option<(u16, i32, i32, i32)> {
        let (fmt, dy, dx, rows) = self.header()?;
        let mut p = 10usize;
        let get = |p: usize| -> Option<u8> { self.data.get(p).copied() };
        match fmt {
            0 => {
                for r in 0..64i32 {
                    for c in 0..64i32 {
                        if let Some(b) = get(p) {
                            if b != 0 {
                                pixel(dy + r, dx + c, b);
                            }
                        }
                        p += 1;
                    }
                }
            }
            1..=3 => {
                let mut row = dy;
                for _ in 0..rows {
                    let mut col = dx;
                    'row: loop {
                        let lo = get(p)?;
                        let hi = get(p + 1)?;
                        p += 2;
                        let w = u16::from_le_bytes([lo, hi]);
                        if w & 0x8000 != 0 {
                            if w & 0x4000 != 0 {
                                row += 1;
                                break 'row;
                            }
                            col += (w & 0x0FFF) as i32;
                        } else {
                            let run = (w & 0x0FFF) as usize;
                            for _ in 0..run {
                                let b = get(p)?;
                                p += 1;
                                if b != 0 {
                                    pixel(row, col, b);
                                }
                                col += 1;
                            }
                            if w & 0x4000 != 0 {
                                row += 1;
                                break 'row;
                            }
                        }
                    }
                }
            }
            _ => {
                let mut row = dy;
                for _ in 0..rows {
                    let mut col = dx;
                    'row: loop {
                        let b0 = get(p)?;
                        p += 1;
                        if b0 & 0x80 != 0 {
                            if b0 & 0x40 != 0 {
                                row += 1;
                                break 'row;
                            }
                            col += (b0 & 0x3F) as i32 + 1;
                        } else {
                            let run = (b0 & 0x3F) as usize + 1;
                            for _ in 0..run {
                                let b = get(p)?;
                                p += 1;
                                if b != 0 {
                                    pixel(row, col, b);
                                }
                                col += 1;
                            }
                            if b0 & 0x40 != 0 {
                                row += 1;
                                break 'row;
                            }
                        }
                    }
                }
            }
        }
        Some((fmt, dy, dx, rows))
    }
}

/// Inputs of one terrain pass [`MissionView::draw_terrain`].
#[derive(Debug)]
pub struct DrawParams<'a> {
    /// Camera TILE X (`cam Q5 >> 5`).
    pub cam_tx: i32,
    /// Camera TILE Y (`cam Q5 >> 5`).
    pub cam_ty: i32,
    /// Zone index 0..=7: selects the off-map edge sprite family
    /// [FUN_00408030].
    pub zone: i32,
    /// Drives the random edge variants. The EXW uses the shared
    /// mission RandB here; ours is the charter T3 statistical
    /// stand-in (deterministic, not bit-identical to the original).
    pub rng: &'a mut Pcg32,
    /// Optional 256-byte XLAT table for the water/darkness remap.
    /// The EXW per-frame remap comes from the `u32[0x4dd444]` pointer
    /// table and only engages with the `_DAT_004edbd4` water flag set
    /// [MISSIONVIEW §8.2 open item]; `None` = plain copy.
    pub remap: Option<&'a [u8; 256]>,
}

impl<'a> DrawParams<'a> {
    /// The plain-copy configuration (no remap).
    pub fn new(cam_tx: i32, cam_ty: i32, zone: i32, rng: &'a mut Pcg32) -> Self {
        DrawParams {
            cam_tx,
            cam_ty,
            zone,
            rng,
            remap: None,
        }
    }
}

/// The mission viewport: parsed TOT mirror + BIN bank + LNK table plus
/// the rebuilt viewport cache. `draw_terrain` mutates the mirrored
/// words (the memoized LNK walk is REAL STATE, exactly as in the EXW
/// type DB at 0x4796bc).
#[derive(Debug, Clone)]
pub struct MissionView {
    width: i32,
    height: i32,
    /// Filtered 36×36 cache, gy-major scan order [init_tiles].
    cache: Vec<ViewEntry>,
    /// Per `(tile, z)`: the TOT plane word, later the memoized
    /// LNK-resolved sprite id (type DB +0x00..+0x0f).
    words: Vec<u16>,
    /// Per `(tile, z)`: TOT word nonzero AND DAT byte zero (the
    /// purely-visual stack marker, type DB +0x10..+0x17).
    seen: Vec<u8>,
    /// Height-bias bytes (type DB +0x1a) — zero-filled by init_tiles in
    /// the EXW; no shipped producer found yet [MISSIONVIEW §8.1]. The
    /// static-frame/anim-window tail bytes (+0x18/+0x1b/+0x1c) share
    /// that zero-fill provenance and no semantics until the producer
    /// is found, so they are not carried as state.
    bias: Vec<u8>,
    bank: Vec<u8>,
    lnk: Vec<u16>,
    /// Entity/robot sprite bank (`GAMEGFX\DANTE.BIN`) staged by the
    /// host [FUN_0041df10 seam, MISSIONVIEW sec 6].
    entity_bank: Vec<u8>,
    /// The per-frame entity sprite list (empty until
    /// [`MissionView::enqueue_robots`]).
    sprite_list: SpriteList,
}

impl MissionView {
    /// Build from the raw on-disk bytes, mirroring load_mission +
    /// init_tiles [MISSIONVIEW §1–§2]:
    ///
    /// - `tot` — the mission `.TOT` (u16 w + u16 h + 8 plane-major
    ///   `w*h` u16 planes) with the 4-byte header skipped;
    /// - `dat` — the mission `.DAT` AFTER its own header skip + sweep
    ///   (use [`bedlam_core::mission::Terrain`] for those rules);
    ///   here only the raw `8*w*h` plane bytes are read;
    /// - `bin` — the zone `MISSION{A..G}.BIN` terrain sprite bank;
    /// - `lnk` — the zone `.LNK` u16[8192] animation link table.
    ///
    /// Returns `None` on malformed inputs (charter: no panics).
    pub fn from_mission_bytes(tot: &[u8], dat: &[u8], bin: &[u8], lnk: &[u8]) -> Option<Self> {
        if tot.len() < 4 || lnk.len() < 0x4000 {
            return None;
        }
        let width = u16::from_le_bytes([tot[0], tot[1]]) as i32;
        let height = u16::from_le_bytes([tot[2], tot[3]]) as i32;
        let n = (width * height) as usize;
        if width <= 0 || height <= 0 || tot.len() != 4 + 16 * n || dat.len() != 8 * n {
            return None;
        }
        // TOT mirror + seen marks (plane stride w*h u16 / w*h u8).
        let mut words = vec![0u16; 8 * n];
        let mut seen = vec![0u8; 8 * n];
        for z in 0..8usize {
            let tot_plane = &tot[4 + 2 * z * n..4 + 2 * (z + 1) * n];
            let dat_plane = &dat[z * n..(z + 1) * n];
            for i in 0..n {
                let w = u16::from_le_bytes([tot_plane[2 * i], tot_plane[2 * i + 1]]);
                if w != 0 {
                    words[8 * i + z] = w;
                    if dat_plane[i] == 0 {
                        seen[8 * i + z] = 1;
                    }
                }
            }
        }
        let mut lnk_words = [0u16; 8192];
        for (i, w) in lnk_words.iter_mut().enumerate() {
            *w = u16::from_le_bytes([lnk[2 * i], lnk[2 * i + 1]]);
        }
        Some(MissionView {
            width,
            height,
            cache: build_cache(),
            words,
            seen,
            bias: vec![0; n],
            bank: bin.to_vec(),
            lnk: lnk_words.to_vec(),
            entity_bank: Vec::new(),
            sprite_list: SpriteList::new(),
        })
    }

    /// Map size in tiles (TOT header).
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// The rebuilt viewport cache (gy-major scan order).
    pub fn cache(&self) -> &[ViewEntry] {
        &self.cache
    }

    /// Memoized plane word / sprite id at `(tile, z)` (the type-DB
    /// word, advanced one LNK step per drawn frame).
    pub fn word(&self, tile: usize, z: usize) -> u16 {
        self.words[8 * tile + z]
    }

    /// The seen marker at `(tile, z)`.
    pub fn seen(&self, tile: usize, z: usize) -> u8 {
        self.seen[8 * tile + z]
    }

    /// One LNK animation step [MISSIONVIEW §1]: `LNK[w]` for word `w`.
    pub fn lnk_step(&self, w: u16) -> u16 {
        self.lnk[w as usize]
    }

    /// The map-overlay word consume [RE-EXW-SIM 7e.1c, asm
    /// 0x408aa1..0x408ab5]: read the mirror word at `(tile, z)`,
    /// advance it one LNK step, memoize it back (the same
    /// destructive walk `draw_terrain` does), and return the new
    /// word — 0 means "no stamp".
    pub fn overlay_word_step(&mut self, tile: usize, z: usize) -> u16 {
        let i = 8 * tile + z;
        let cw = self.lnk[self.words[i] as usize];
        self.words[i] = cw;
        cw
    }

    /// Stage the entity bank (`GAMEGFX\DANTE.BIN` bytes) [the
    /// FUN_0041df10 staging seam; unstaged ⇒ entity flushes draw
    /// nothing].
    pub fn set_entity_bank(&mut self, bin: &[u8]) {
        self.entity_bank = bin.to_vec();
    }

    /// Queued sprite-list node count (test observability).
    pub fn sprite_nodes(&self) -> usize {
        self.sprite_list.len()
    }

    /// Test seam: the live sprite list.
    #[cfg(test)]
    pub(crate) fn sprite_list_for_test(&self) -> &SpriteList {
        &self.sprite_list
    }

    /// Project one robot to viewport screen space [the enqueue head,
    /// MISSIONVIEW sec 5d, verified]: `wx/wy = pos>>8` (Q5),
    /// `sx = colAdj + (dx-dy) + 0x110`, `sy = shake + ((dx+dy)>>1) +
    /// 0x10C + rowAdj - z` with `dx/dy = wx/wy - cam Q5` and the
    /// same-camera colAdj/rowAdj fine terms the present window uses.
    /// The scene host reuses this for its click hit-test (the EXW
    /// sprite-click family ~0x433cbc tests the drawn sprite position).
    pub fn project_robot(
        &self,
        r: &RobotView,
        cam_q5_x: i32,
        cam_q5_y: i32,
        shake_y: i32,
    ) -> (i32, i32) {
        let wx = r.pos_x >> 8;
        let wy = r.pos_y >> 8;
        let dx = wx - cam_q5_x;
        let dy = wy - cam_q5_y;
        let col_adj = ((cam_q5_x & 0x1F) - (cam_q5_y & 0x1F) + 0x20) & 0x3F;
        let row_adj = ((cam_q5_x & 0x1F) + (cam_q5_y & 0x1F)) >> 1;
        let sx = col_adj + (dx - dy) + 0x110;
        let sy = shake_y + ((dx + dy) >> 1) + 0x10C + row_adj - r.z;
        (sx, sy)
    }

    /// Run the robot entity loop of FUN_00403938 [MISSIONVIEW sec 5d,
    /// verified]: rebuild the per-frame sprite list (the EXW clears
    /// the bucket grid + arena before the loop), project every
    /// alive+clipped robot to screen, and enqueue its sprites. The
    /// list is consumed by the next [`MissionView::draw_terrain`]
    /// (entities enqueue BEFORE the terrain pass in the EXW too).
    ///
    /// `cam_q5_x/cam_q5_y` are the Q5 pixel cameras (`_DAT_004edde4/8`),
    /// `shake_y` the vertical shake term, `frame_count` the global
    /// frame counter (overlay anim).
    pub fn enqueue_robots(
        &mut self,
        robots: &[RobotView],
        cam_q5_x: i32,
        cam_q5_y: i32,
        shake_y: i32,
        frame_count: u64,
    ) {
        let mut list = SpriteList::new();
        let cam_tx = cam_q5_x >> 5;
        let cam_ty = cam_q5_y >> 5;
        for r in robots {
            if !r.alive {
                continue;
            }
            let (sx, sy) = self.project_robot(r, cam_q5_x, cam_q5_y, shake_y);
            if !(0..ENTITY_CLIP).contains(&sx) || !(0..ENTITY_CLIP).contains(&sy) {
                continue;
            }
            let wx = r.pos_x >> 8;
            let wy = r.pos_y >> 8;
            let layer = r.z >> 5;
            let ex = wx + 0xB;
            let ey = wy + 0xB;
            // 1. Shield (states 5/6): sy - 0x48, mode 0x12e, frame
            //    clamp(10 - wobble%4, 0..9) [sec 5d].
            if r.state == 5 || r.state == 6 {
                let f = (10 - r.wobble % 4).clamp(0, 9) as u16;
                list.enqueue(
                    sx,
                    sy - 0x48,
                    NodeBank::Shield,
                    ex,
                    ey,
                    cam_tx,
                    cam_ty,
                    f,
                    layer,
                    0x12E,
                );
            }
            // 2. Body (DANTE[anim]) unless hidden: state 2 with a
            //    >0xf stats word, state 5 past 0xf wobble, state 6.
            let hidden = (r.state == 2 && r.type_stats > 0xF)
                || (r.state == 5 && r.wobble > 0xF)
                || r.state == 6;
            if !hidden {
                list.enqueue(
                    sx,
                    sy,
                    NodeBank::Dante,
                    ex,
                    ey,
                    cam_tx,
                    cam_ty,
                    r.anim,
                    layer,
                    300,
                );
                // 3. Variant sprite (+0x88).
                if r.variant_sprite {
                    list.enqueue(
                        sx,
                        sy,
                        NodeBank::Variant,
                        ex,
                        ey,
                        cam_tx,
                        cam_ty,
                        r.variant,
                        layer,
                        300,
                    );
                }
                // 4. Animated overlay (+0x16 active).
                if r.overlay_active {
                    let f = r.frame_base as i32 * 3 + (frame_count % 3) as i32 + 0x40;
                    list.enqueue(
                        sx,
                        sy,
                        NodeBank::Dante,
                        ex,
                        ey,
                        cam_tx,
                        cam_ty,
                        f as u16,
                        layer,
                        300,
                    );
                }
                // 5. Unconditional base+0x20 sprite.
                list.enqueue(
                    sx,
                    sy,
                    NodeBank::Dante,
                    ex,
                    ey,
                    cam_tx,
                    cam_ty,
                    r.frame_base + 0x20,
                    layer,
                    300,
                );
            }
        }
        self.sprite_list = list;
    }

    /// One terrain pass over the viewport: the FUN_00403938 terrain
    /// loop [MISSIONVIEW §3]. Wrong buffer length returns without
    /// drawing. See [`DrawParams`] for the inputs.
    pub fn draw_terrain(&mut self, buf: &mut [u8], p: &mut DrawParams<'_>) {
        let DrawParams {
            cam_tx,
            cam_ty,
            zone,
            ref mut rng,
            remap,
        } = *p;
        if buf.len() != VIEW_BUF_LEN {
            return;
        }
        // The entity sprite list is consumed by this pass (the EXW
        // clears the bucket grid per frame; enqueue_robots rebuilds
        // it) — buckets flush interleaved with the terrain layers
        // [MISSIONVIEW sec 5b].
        let list = std::mem::take(&mut self.sprite_list);
        let entity_bank = &self.entity_bank;
        let (w, h) = (self.width, self.height);
        let cap = DRAW_CAP;
        for ci in 0..self.cache.len() {
            let entry = self.cache[ci];
            let mut dest = entry.buf_off;
            let tx = entry.dtile_x + cam_tx;
            let ty = entry.dtile_y + cam_ty;
            let tile_idx = |x: i32, y: i32| (y * w + x) as usize;
            if !(0..w).contains(&tx) || !(0..h).contains(&ty) {
                // Off-map edge [FUN_00408030]: per-zone base + rand(9),
                // drawn WITHOUT remap (EBX = 0 at 0x406b1f).
                let (base, limit) = edge_sprite_base(zone);
                let id = if limit == 0 {
                    base
                } else {
                    base + (rng.next_u32() % limit) as i32
                };
                if 0 <= dest && dest < cap {
                    if let Some(sprite) = Sprite::resolve(&self.bank, id as u16) {
                        blit_sprite(buf, &sprite, dest, None);
                    }
                }
                continue;
            }
            let rec = tile_idx(tx, ty);
            let bias_word = self.bias[rec];
            let bias = if bias_word & 0x7F == 0 {
                0i32
            } else if bias_word & 0x80 == 0 {
                (bias_word & 0x0F) as i32 * 0x500
            } else {
                (bias_word & 0x0F) as i32 * -0x500
            };
            let mut cursor = 0usize;
            for layer in 0..8usize {
                if cursor == layer {
                    let word = self.words[8 * rec + layer];
                    if word != 0 && 0 <= dest + bias && dest + bias < cap {
                        // Per-frame LNK animation walk, memoized back
                        // into the type-DB word [MISSIONVIEW §1].
                        let sprite_id = self.lnk[word as usize];
                        self.words[8 * rec + layer] = sprite_id;
                        // The EXW frame index (static byte vs the
                        // u32[0x456ca8] sequence) only selects the
                        // per-frame remap table; shipped records carry
                        // zero tails so the branch is static. Remap
                        // selection stays caller-controlled here.
                        if let Some(sprite) = Sprite::resolve(&self.bank, sprite_id) {
                            blit_sprite(buf, &sprite, dest, remap);
                        }
                    }
                    cursor += 1;
                    // Seen-chase: consecutive seen levels above draw
                    // one level up + bias while seen && word != 0.
                    while cursor < 8
                        && self.seen[8 * rec + cursor] != 0
                        && self.words[8 * rec + cursor] != 0
                    {
                        let d2 = dest - Z_LEVEL_STEP + bias;
                        if 0 <= d2 && d2 < cap {
                            let word2 = self.words[8 * rec + cursor];
                            let sprite_id = self.lnk[word2 as usize];
                            self.words[8 * rec + cursor] = sprite_id;
                            if let Some(sprite) = Sprite::resolve(&self.bank, sprite_id) {
                                blit_sprite(buf, &sprite, d2, remap);
                            }
                        }
                        cursor += 1;
                    }
                }
                // Sprite-list flush [MISSIONVIEW sec 5b]: this cell's
                // bucket at this layer, in ascending painter order.
                // Only the DANTE bank is staged; Shield/Variant nodes
                // stay queued but draw nothing (sec 5d seam).
                for node in list.bucket(tx - cam_tx + 9, ty - cam_ty + 9, layer) {
                    debug_assert_eq!(
                        node.layer as usize, layer,
                        "node layer matches its bucket layer"
                    );
                    match node.bank {
                        NodeBank::Dante => flush_node(buf, node, entity_bank),
                        NodeBank::Shield | NodeBank::Variant => {}
                    }
                }
                dest -= Z_LEVEL_STEP;
            }
        }
    }
}

/// Blit one sprite at byte offset `dest` with the FUN_00401471 codec
/// (stride 640, transparency 0, optional XLAT remap). Every write is
/// bounds-checked against `buf` [charter; the EXW relies on arena
/// slack past its `0x59b00` cap — see MISSIONVIEW §3 note].
fn blit_sprite(buf: &mut [u8], sprite: &Sprite<'_>, dest: i32, remap: Option<&[u8; 256]>) {
    let base = dest.max(0) as usize;
    sprite.for_each_pixel(|row, col, b| {
        let off = base + row as usize * VIEW_STRIDE + col as usize;
        if let Some(slot) = buf.get_mut(off) {
            *slot = match remap {
                Some(t) => t[b as usize],
                None => b,
            };
        }
    });
}

/// Build the 36×36 filtered viewport cache [init_tiles, verified]:
/// sticky anchor 17 (the gx of the first in-bounds cell, +5), entries
/// emitted gy-major — 467 cells on the fixed window.
fn build_cache() -> Vec<ViewEntry> {
    let mut out = Vec::new();
    for gy in 0..36i32 {
        for gx in 0..36i32 {
            let x = CACHE_X0 + gx * CACHE_STEP_GX_X + gy * CACHE_STEP_GY_X;
            let y = CACHE_Y0 + gy * CACHE_STEP_X_Y + gx * CACHE_STEP_X_Y;
            if !(0..CACHE_MAX_X).contains(&x) || !(0..CACHE_MAX_Y).contains(&y) {
                continue;
            }
            out.push(ViewEntry {
                buf_off: y * VIEW_STRIDE as i32 + x,
                dtile_x: gx - CACHE_ANCHOR,
                dtile_y: gy - CACHE_ANCHOR,
            });
        }
    }
    out
}

/// The 480×480 present window [FUN_00401107, verified]: base
/// `buf + 0xa040` (row 64, col 64) + the fine-camera offset
/// `col += ((camx & 0x1f) - (camy & 0x1f) + 0x20) & 0x3f`,
/// `row += ((camx & 0x1f) + (camy & 0x1f)) >> 1` (the latter in whole
/// rows only in the scaled path; camera 0 → window origin (96, 64)).
///
/// Returns `None` when `buf` has the wrong length. The result borrows
/// `buf` (a 480×480 packed crop, row stride 480).
pub fn present_window(buf: &[u8], cam_fine_x: i32, cam_fine_y: i32) -> Option<Vec<u8>> {
    if buf.len() != VIEW_BUF_LEN {
        return None;
    }
    let col_adj = (((cam_fine_x & 0x1F) - (cam_fine_y & 0x1F)) + 0x20) & 0x3F;
    let row_adj = ((cam_fine_x & 0x1F) + (cam_fine_y & 0x1F)) >> 1;
    let x0 = 64 + col_adj;
    let y0 = 64 + row_adj;
    let mut out = vec![0u8; 480 * 480];
    for row in 0..480usize {
        let src = (y0 as usize + row) * VIEW_STRIDE + x0 as usize;
        let dst = row * 480;
        out[dst..dst + 480].copy_from_slice(buf.get(src..src + 480)?);
    }
    Some(out)
}

// ===========================================================================
// Entity overlay: the sprite list (FUN_0040798e enqueue, FUN_0040179b
// flush, the robot loop of FUN_00403938) — MISSIONVIEW sec 5b–5d.
// ===========================================================================

/// Sprite-list bucket grid dimension: 36×36×8 head pointers at
/// 0x46cdbc, zero-cleared every frame [MISSIONVIEW sec 5, asm
/// ECX=0xa200 @0x403950].
const BUCKET_DIM: i32 = 36;

/// Entity clip bound: sx and sy must satisfy `0 <= v < 0x23f`
/// [robot loop, verified].
const ENTITY_CLIP: i32 = 0x23F;

/// Which staged bank a sprite-list node blits from. Only the DANTE
/// robot bank is staged by this crate; the other EXW banks (shield
/// DAT_0046af38, variant DAT_0046af44) are carried for node-stream
/// shape and skipped while unstaged [MISSIONVIEW sec 5d].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeBank {
    /// `GAMEGFX\DANTE.BIN` at 0x4ede2c [LoadFile @0x41e02e].
    Dante,
    /// Shield bank DAT_0046af38 (state 5/6 hit flash).
    Shield,
    /// Variant-equipment bank DAT_0046af44.
    Variant,
}

/// One queued sprite [FUN_0040798e node, verified]: the EXW node is
/// 48 B at the arena cursor; only the flushed fields are modeled.
#[derive(Debug, Clone, Copy)]
struct SpriteNode {
    /// Dest byte offset into the 0x64000 buffer (`sx + sy*0x280`).
    dest: i32,
    bank: NodeBank,
    frame: u16,
    /// Flush layer 0..7 (clamped on enqueue).
    layer: u8,
    /// Painter sort key `wx + wy` (the +0xb-adjusted pixel coords).
    sort: i32,
    /// Blit mode (`300` plain, `0x130` mask, `0x12d/0x12e/0x12f`
    /// remaps — see [`flush_node`]).
    mode: i32,
}

/// The per-frame entity sprite list: 36×36×8 insertion-sorted buckets
/// [FUN_0040798e, verified]. A fresh list per frame is the faithful
/// model of the EXW's per-frame head-array clear + arena reset.
/// `Default` is the CONSUMED state (no buckets) — a node enqueued
/// into it is dropped, exactly like the EXW's post-flush frame tail.
#[derive(Debug, Clone, Default)]
pub struct SpriteList {
    buckets: Vec<Vec<SpriteNode>>,
}

impl SpriteList {
    fn new() -> Self {
        SpriteList {
            buckets: vec![Vec::new(); (BUCKET_DIM * BUCKET_DIM * 8) as usize],
        }
    }

    /// Total queued nodes (observability seam for tests/hosts).
    pub fn len(&self) -> usize {
        self.buckets.iter().map(Vec::len).sum()
    }

    /// Is the list empty?
    pub fn is_empty(&self) -> bool {
        self.buckets.iter().all(Vec::is_empty)
    }

    /// Enqueue one sprite [FUN_0040798e, verified]: bucket
    /// `[(wx>>5) - cam_tx + 9][(wy>>5) - cam_ty + 9][layer]`, node
    /// sorted ascending by `wx + wy`, stable after equal sorts.
    /// `wx`/`wy` are the entity-loop pixel coords (already +0xb).
    /// Negative bucket coords drop the node exactly as the EXW
    /// early-out; coords ≥ 36 are unreachable through the sx/sy clip
    /// (both clips bound dx ≤ 0x1c8 ⇒ bx ≤ 23) and dropped here as a
    /// defensive bound.
    // The argument list mirrors the EXW's register/stack convention
    // (EAX/EDX/EBX/ECX + frame/layer/mode) plus the two camera words
    // the original reads from globals.
    #[allow(clippy::too_many_arguments)]
    fn enqueue(
        &mut self,
        sx: i32,
        sy: i32,
        bank: NodeBank,
        wx: i32,
        wy: i32,
        cam_tx: i32,
        cam_ty: i32,
        frame: u16,
        layer: i32,
        mode: i32,
    ) {
        let bx = (wx >> 5) - cam_tx + 9;
        let by = (wy >> 5) - cam_ty + 9;
        if bx < 0 || by < 0 || bx >= BUCKET_DIM || by >= BUCKET_DIM {
            return;
        }
        if self.buckets.is_empty() {
            return; // consumed/default list: drop, as the EXW frame tail
        }
        let layer = layer.clamp(0, 7) as u8;
        let node = SpriteNode {
            dest: sx + sy * VIEW_STRIDE as i32,
            bank,
            frame,
            layer,
            sort: wx + wy,
            mode,
        };
        let idx = (by * BUCKET_DIM + bx) as usize * 8 + layer as usize;
        let bucket = &mut self.buckets[idx];
        // Stable ascending insert: after every node with sort <= ours.
        let pos = bucket
            .iter()
            .position(|n| n.sort > node.sort)
            .unwrap_or(bucket.len());
        bucket.insert(pos, node);
    }

    /// The bucket at `(bx, by, layer)` (camera-relative +9 indices).
    fn bucket(&self, bx: i32, by: i32, layer: usize) -> &[SpriteNode] {
        if !(0..BUCKET_DIM).contains(&bx) || !(0..BUCKET_DIM).contains(&by) {
            return &[];
        }
        self.buckets
            .get((by * BUCKET_DIM + bx) as usize * 8 + layer)
            .map(|b| b.as_slice())
            .unwrap_or(&[])
    }
}

/// The render-facing robot record: the fields of the 0xA8-B EXW robot
/// record the viewport consumes [MISSIONVIEW sec 5d]. [`RobotView::from_sim`]
/// fills the spawn defaults; hosts can override the tail fields.
#[derive(Debug, Clone, Copy)]
pub struct RobotView {
    /// World X, Q13 (+0x00).
    pub pos_x: i32,
    /// World Y, Q13 (+0x04).
    pub pos_y: i32,
    /// Floor z, Q5 px (+0x08): subtracted raw from sy, `>> 5` for the
    /// layer.
    pub z: i32,
    /// State word (+0x0C): 5/6 draw the shield sprite and (6, or a
    /// long-5) hide the body.
    pub state: u16,
    /// Walk anim phase (+0x12) = the DANTE body frame.
    pub anim: u16,
    /// Frame-base word (+0x14): the unconditional `base + 0x20`
    /// sprite and the overlay base. Zero at spawn.
    pub frame_base: u16,
    /// Overlay countdown active (+0x16 != 0xFFFF): gates the
    /// `base*3 + frame%3 + 0x40` sprite. False at spawn.
    pub overlay_active: bool,
    /// Overlay anim divisor input: the global frame count mod 3.
    pub frame_tick: u32,
    /// Spawn variant (+0x18) — the Variant-bank frame.
    pub variant: u16,
    /// Variant-equipment flag (+0x88 != 0). False at spawn.
    pub variant_sprite: bool,
    /// Hit/dying wobble (+0x90): the shield frame and the state-5
    /// hide gate.
    pub wobble: i32,
    /// The 0x4dcdd4 stats word for the state-2 body-hide gate
    /// (+0x84 index). Unmodeled producer ⇒ 0 (gate never fires).
    pub type_stats: i32,
    /// Alive (+0x7C).
    pub alive: bool,
}

impl RobotView {
    /// Spawn-default view of a [`bedlam_core::mission::Robot`] — the
    /// subset the sim models, with the EXW spawn-zero tail
    /// [FUN_0040cca0: +0x14 = 0, +0x16 = 0xFFFF, +0x88 = 0].
    pub fn from_sim(r: &bedlam_core::mission::Robot) -> Self {
        RobotView {
            pos_x: r.pos_x,
            pos_y: r.pos_y,
            z: r.z,
            state: r.state,
            anim: r.anim,
            frame_base: 0,
            overlay_active: false,
            frame_tick: 0,
            variant: r.variant,
            variant_sprite: false,
            wobble: 0,
            type_stats: 0,
            alive: r.alive,
        }
    }
}

/// Blit one sprite-list node with the FUN_0040179b codec
/// [MISSIONVIEW sec 5c, asm-authoritative]: directory `2 + 4*(id &
/// 0xFFF)`, sprite at `entry + u32[entry]`, header read SKIPS the fmt
/// word (dy, dx, gate-unchecked, rows), ALWAYS u16-RLE decode, and —
/// unlike the terrain blit — literal runs copy RAW bytes with no
/// zero-skip. Mode dispatch: `0x130` paints 0xFF; the remap modes
/// `0x12d/0x12e/0x12f` are the EXW's water-ON paths (TXPAL1 64-KiB
/// composition / DARKPAL XLAT) and degrade to plain copy while the
/// water flag is off [asm: `CMP [0x4edbd4],0; JZ plain`], which is
/// the behavior modeled here (water flag producer unfound, §8.2).
/// Every write is bounds-checked [charter].
fn flush_node(buf: &mut [u8], node: &SpriteNode, bank: &[u8]) {
    // Directory + header (+2 skips the fmt word) [sec 5c].
    let id = (node.frame & 0xFFF) as usize;
    let entry = 2 + 4 * id;
    let off = match bank.get(entry..entry + 4) {
        Some(w) => u32::from_le_bytes(w.try_into().unwrap()) as usize,
        None => return,
    };
    let data = match bank.get(entry + off + 2..) {
        Some(d) => d,
        None => return,
    };
    let word = |i: usize| u16::from_le_bytes([data[2 * i], data[2 * i + 1]]) as i32;
    if data.len() < 8 {
        return;
    }
    let dy = word(0);
    let dx = word(1);
    let rows = word(3);
    if rows <= 0 {
        return;
    }
    let base = node.dest + dy * VIEW_STRIDE as i32 + dx;
    let mut p = 8usize; // stream starts after dy/dx/gate/rows
    let mut dest = base;
    let mut row_start = base;
    let mask = node.mode == 0x130;
    let mut left = rows;
    let get = |p: usize| -> Option<u16> {
        data.get(p..p + 2)
            .and_then(|w| w.try_into().ok())
            .map(u16::from_le_bytes)
    };
    while left > 0 {
        let Some(w) = get(p) else { return };
        p += 2;
        if w & 0x8000 != 0 {
            if w & 0x4000 != 0 {
                // End of row: advance to the next row start.
                dest = row_start + VIEW_STRIDE as i32;
                row_start = dest;
                left -= 1;
                continue;
            }
            dest += (w & 0x0FFF) as i32;
        } else {
            let run = (w & 0x0FFF) as usize;
            for _ in 0..run {
                let Some(b) = data.get(p).copied() else {
                    return;
                };
                p += 1;
                // Mode 0x130 paints the mask; every other modeled
                // mode is the raw copy (zeros included) [sec 5c].
                let out = if mask { 0xFF } else { b };
                if dest >= 0 {
                    if let Some(slot) = buf.get_mut(dest as usize) {
                        *slot = out;
                    }
                }
                dest += 1;
            }
            if w & 0x4000 != 0 {
                dest = row_start + VIEW_STRIDE as i32;
                row_start = dest;
                left -= 1;
            }
        }
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;

    /// A minimal 1x1 mission payload for view construction.
    fn tiny_view() -> MissionView {
        let mut tot = [0u8; 20]; // 1x1, all planes zero
        tot[0] = 1;
        tot[2] = 1;
        let dat = [0u8; 8];
        let bin = [0u8; 0];
        let lnk = [0u8; 0x4000];
        MissionView::from_mission_bytes(&tot, &dat, &bin, &lnk).unwrap()
    }

    /// One synthetic sprite in the FUN_0040179b bank layout: directory
    /// slot `2 + 4*id`, u32 rel offset, header {fmt, dy, dx, gate,
    /// rows}, then a u16-RLE stream. 2 rows: skip 2 + literal `lits`
    /// (INCLUDING zeros) + EOR, then literal 1 (0xAB) + EOR.
    fn synth_bank(id: usize, dy: u16, lits: &[u8]) -> Vec<u8> {
        assert!(id < 2);
        let mut bank = vec![0u8; 2 + 4 * 2]; // count + 2 slots
        bank[0] = 2; // two sprites
        let mut sprite = Vec::new();
        sprite.extend_from_slice(&3u16.to_le_bytes()); // fmt (skipped)
        sprite.extend_from_slice(&dy.to_le_bytes());
        sprite.extend_from_slice(&0u16.to_le_bytes()); // dx
        sprite.extend_from_slice(&64u16.to_le_bytes()); // gate (unchecked)
        sprite.extend_from_slice(&2u16.to_le_bytes()); // rows
        sprite.extend_from_slice(&(0x8000u16 | 2).to_le_bytes()); // skip 2
        sprite.extend_from_slice(&(lits.len() as u16).to_le_bytes());
        sprite.extend_from_slice(lits);
        sprite.extend_from_slice(&0xC000u16.to_le_bytes()); // EOR
        sprite.extend_from_slice(&1u16.to_le_bytes());
        sprite.push(0xAB);
        sprite.extend_from_slice(&0xC000u16.to_le_bytes());
        // u32 rel offset is relative to the DIRECTORY SLOT (entry).
        let entry = 2 + 4 * id;
        let off = (bank.len() - entry) as u32;
        bank[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
        bank.extend_from_slice(&sprite);
        bank
    }

    fn node(dest: i32, frame: u16, mode: i32) -> SpriteNode {
        SpriteNode {
            dest,
            bank: NodeBank::Dante,
            frame,
            layer: 0,
            sort: 0,
            mode,
        }
    }

    #[test]
    fn flush_copies_literal_zeros_but_mode_130_paints_ff() {
        // FUN_0040179b [sec 5c]: literal runs copy RAW bytes (no
        // zero-skip — unlike the terrain blit); mode 0x130 paints
        // 0xFF; skip words leave the buffer untouched.
        let bank = synth_bank(0, 0, &[0x11, 0x00, 0x22]);
        let mut buf = vec![0u8; VIEW_BUF_LEN];
        flush_node(&mut buf, &node(100, 0, 300), &bank);
        assert_eq!(buf[102], 0x11, "skip 2 then literal");
        assert_eq!(buf[103], 0x00, "literal ZERO is copied");
        assert_eq!(buf[104], 0x22);
        assert_eq!(buf[105], 0x00, "the skip word after the run");
        assert_eq!(buf[100 + VIEW_STRIDE], 0xAB, "row 1 via EOR + stride");
        let mut buf2 = vec![0xEE; VIEW_BUF_LEN];
        flush_node(&mut buf2, &node(100, 0, 0x130), &bank);
        assert_eq!(buf2[102], 0xFF);
        assert_eq!(buf2[103], 0xFF, "mode 0x130 paints over the zero");
        assert_eq!(buf2[104], 0xFF);
        assert_eq!(buf2[105], 0xEE, "skipped bytes untouched by the mask");
        assert_eq!(buf2[100 + VIEW_STRIDE], 0xFF);
    }

    #[test]
    fn flush_header_dy_offsets_the_dest() {
        let bank = synth_bank(0, 3, &[0x77]);
        let mut buf = vec![0u8; VIEW_BUF_LEN];
        flush_node(&mut buf, &node(10, 0, 300), &bank);
        assert_eq!(buf[10 + 3 * VIEW_STRIDE + 2], 0x77, "dy rows + skip 2");
    }

    #[test]
    fn sprite_list_inserts_ascending_and_stable_after_equals() {
        let mut list = SpriteList::new();
        let mut e = |sort: i32, frame: u16| {
            list.enqueue(0, 0, NodeBank::Dante, sort, 0, 0, 0, frame, 0, 300);
        };
        e(20, 1);
        e(5, 2);
        e(20, 3); // equal sort: after node 1
        e(10, 4);
        let order: Vec<u16> = list.bucket(9, 9, 0).iter().map(|n| n.frame).collect();
        assert_eq!(order, vec![2, 4, 1, 3], "ascending, enqueue order on ties");
        assert_eq!(list.len(), 4);
        // Negative bucket coords drop the node (the EXW early-out).
        list.enqueue(0, 0, NodeBank::Dante, -320, 0, 0, 0, 9, 0, 300);
        assert_eq!(list.len(), 4);
    }

    const Q13: i32 = 0x2000;

    fn robot_at(tile_x: i32, tile_y: i32, z: i32, anim: u16) -> RobotView {
        RobotView {
            pos_x: tile_x * Q13 + 0xF00,
            pos_y: tile_y * Q13 + 0xF00,
            z,
            state: 0,
            anim,
            frame_base: 0,
            overlay_active: false,
            frame_tick: 0,
            variant: 0,
            variant_sprite: false,
            wobble: 0,
            type_stats: 0,
            alive: true,
        }
    }

    #[test]
    fn robot_projection_and_spawn_defaults() {
        // [sec 5d] camera at Q5 (0, 0): a spawned robot at tile (0, 0)
        // has wx = wy = 15 px, so sx = 0x20 + 0 + 0x110 = 0x130 and
        // sy = (30>>1) + 0x10c - 31 = 0xfc; it enqueues exactly TWO
        // sprites (DANTE[anim], DANTE[0x20]).
        let mut v = tiny_view();
        let robots = [robot_at(0, 0, 31, 5)];
        v.enqueue_robots(&robots, 0, 0, 0, 0);
        assert_eq!(v.sprite_nodes(), 2, "spawn defaults: body + base");
        let b = v.sprite_list_for_test().bucket(9, 9, 0);
        let frames: Vec<u16> = b.iter().map(|n| n.frame).collect();
        assert_eq!(frames, vec![5, 0x20]);
        // Bucket coords: (wx+0xb)>>5 = 0, +9 => 9; dest = sx + sy*0x280.
        assert_eq!(b[0].dest, 0x130 + 0xFC * 0x280);
        // Clipped out: a robot far off-screen enqueues nothing.
        let mut v2 = tiny_view();
        v2.enqueue_robots(&[robot_at(40, 0, 31, 0)], 0, 0, 0, 0);
        assert_eq!(v2.sprite_nodes(), 0, "sx clip drops it");
        // Dead robots draw nothing.
        let mut v3 = tiny_view();
        let mut dead = robot_at(0, 0, 31, 0);
        dead.alive = false;
        v3.enqueue_robots(&[dead], 0, 0, 0, 0);
        assert_eq!(v3.sprite_nodes(), 0);
    }

    #[test]
    fn robot_states_gate_the_body_and_add_the_shield() {
        let mut v = tiny_view();
        let mut r = robot_at(0, 0, 31, 7);
        r.state = 6; // dying: shield yes, body no
        v.enqueue_robots(&[r], 0, 0, 0, 0);
        let b = v.sprite_list_for_test().bucket(9, 9, 0);
        assert_eq!(b.len(), 1, "state 6: only the shield node");
        assert_eq!(b[0].bank, NodeBank::Shield);
        assert_eq!(b[0].mode, 0x12E);
        assert_eq!(b[0].dest, 0x130 + (0xFC - 0x48) * 0x280, "sy - 0x48");
        let mut v2 = tiny_view();
        let mut r2 = robot_at(0, 0, 31, 7);
        r2.state = 5;
        r2.wobble = 0x10; // > 0xf: body hidden, shield remains
        v2.enqueue_robots(&[r2], 0, 0, 0, 0);
        assert_eq!(v2.sprite_nodes(), 1);
        let mut v3 = tiny_view();
        let mut r3 = robot_at(0, 0, 31, 7);
        r3.state = 5;
        r3.wobble = 4; // <= 0xf: body AND shield
        r3.overlay_active = true; // +0x40 overlay joins the body
        r3.frame_base = 2;
        v3.enqueue_robots(&[r3], 0, 0, 0, 7); // frame_count%3 = 1
        let frames: Vec<u16> = v3
            .sprite_list_for_test()
            .bucket(9, 9, 0)
            .iter()
            .map(|n| n.frame)
            .collect();
        assert_eq!(
            frames,
            vec![9, 7, 2 * 3 + 1 + 0x40, 2 + 0x20],
            "shield(clamp 10-wobble%4), body, overlay, base+0x20"
        );
    }
}
