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
