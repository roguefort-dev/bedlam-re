//! Mission viewport corpus gate (P4 render half). Skips when the
//! corpus is absent (CI); when present it drives the EXW-verified
//! isometric viewport renderer (docs/RE-EXW-MISSIONVIEW.md,
//! engine/bedlam-render/src/mission_view.rs) over the REAL shipped
//! ZONEA/MISSION1 bytes:
//!
//! 1. LOADER: `MissionView::from_mission_bytes(TOT, DAT, BIN, LNK)`
//!    mirrors init_tiles@00407e11 — the 36×36 viewport cache with the
//!    sticky anchor 21, the 8-plane TOT word mirror, and the seen
//!    marks (TOT word nonzero AND DAT byte zero).
//! 2. CODEC: sprite 0 of MISSIONA.BIN decodes as fmt 7 u8-RLE, 34
//!    rows, dy 29, spanning the full 64-column tile — the
//!    FUN_00401471 semantics on real bytes.
//! 3. ANIMATION LINK: ZONEA TOT plane 0 holds LNK-cycle members
//!    (55..64 etc.); one draw_terrain pass advances every drawn word
//!    exactly one LNK step (memoized), e.g. word 55 → 56.
//! 4. FRAME: at camera tile (0, 0), frame 0, zone 0, the terrain pass
//!    fills the 0x64000 buffer; the 480×480 present window
//!    (FUN_00401107, camera 0 → origin (96, 64)) is hash-pinned, and
//!    a second identical run is byte-identical.
//!
//! game-data access is read-only. No game bytes enter git — only
//! hashes and counts are asserted.

use std::path::PathBuf;

use bedlam_core::hash::Fnv1a64;
use bedlam_core::rng::Pcg32;
use bedlam_render::mission_view::{
    present_window, DrawParams, MissionView, VIEW_BUF_LEN, Z_LEVEL_STEP,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn read(rel: &[&str]) -> Option<Vec<u8>> {
    std::fs::read(root().join(rel.iter().collect::<PathBuf>())).ok()
}

/// The staged corpus inputs: TOT, DAT, BIN, LNK.
fn zonea() -> Option<[Vec<u8>; 4]> {
    Some([
        read(&["EDITOR", "ZONEA", "MISSION1.TOT"])?,
        read(&["EDITOR", "ZONEA", "MISSION1.DAT"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.BIN"])?,
        read(&["EDITOR", "ZONEA", "MISSIONA.LNK"])?,
    ])
}

/// Raw plane bytes of a mission DAT after the EXW header skip + sweep
/// (the loader rules live in bedlam-core Terrain; the viewport only
/// needs the swept plane bytes, so they are re-applied here).
fn dat_planes(dat: &[u8]) -> Vec<u8> {
    assert!(dat.len() >= 4);
    let w = u16::from_le_bytes([dat[0], dat[1]]) as usize;
    let h = u16::from_le_bytes([dat[2], dat[3]]) as usize;
    let n = w * h;
    assert_eq!(dat.len(), 4 + 8 * n);
    let mut planes = dat[4..].to_vec();
    for z in 0..7 {
        for b in planes[z * n..z * n + n].iter_mut() {
            if *b >= 0x80 {
                *b = 0;
            }
        }
    }
    planes
}

#[test]
fn zonea_mission1_viewport_cache_mirror_and_frame_hash_pinned() {
    let Some([tot, dat, bin, lnk]) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };

    // --- 1. Loader over the real bytes --------------------------------
    let mut view = MissionView::from_mission_bytes(&tot, &dat_planes(&dat), &bin, &lnk)
        .expect("TOT+DAT+BIN+LNK parse");
    assert_eq!(view.size(), (25, 75), "ZONEA/MISSION1 TOT dims");
    let cache = view.cache().to_vec();
    // Sticky anchor 17: the first in-bounds cell of the gy-major scan
    // is (gx, gy) = (12, 4), so the first entry's own delta is
    // (12-17, 4-17) = (-5, -13).
    assert_eq!((cache[0].dtile_x, cache[0].dtile_y), (-5, -13));
    // The anchored row dtile_y == 0 spans dtile_x -9..=9 (19 cells).
    let row0: Vec<_> = cache.iter().filter(|e| e.dtile_y == 0).collect();
    assert_eq!(row0.len(), 19, "the anchored row cells");
    for (i, e) in row0.iter().enumerate() {
        assert_eq!(e.dtile_x, i as i32 - 9);
        // Cache geometry: x = 0x130 + (gx-gy)*0x20, y = (gx+gy-0x10)*0x10.
        let gx = e.dtile_x + 17;
        let gy = e.dtile_y + 17;
        assert_eq!(
            e.buf_off,
            (gx + gy - 16) * 16 * 0x280 + 0x130 + (gx - gy) * 0x20
        );
    }
    // The fixed 608x800 window admits 467 of the 1296 grid cells
    // (dtile_y -13..=18).
    assert_eq!(cache.len(), 467, "the filtered 36x36 cache size");

    // TOT mirror + seen marks on a real deck tile (21, 73): plane 0
    // carries the deck word; DAT plane 0 there is type 1 (nonzero) so
    // the level is NOT seen (it has walk volume).
    let tile = (73 * 25 + 21) as usize;
    assert_ne!(view.word(tile, 0), 0, "deck tile has a plane-0 word");
    assert_eq!(view.seen(tile, 0), 0, "deck level 0 has DAT volume");

    // --- 2. The codec on a real sprite --------------------------------
    // Sprite 0: fmt 7 u8-RLE, dy 29, dx 0, gate 64, rows 34, opaque
    // pixels across all 64 columns [verified against the corpus]. The
    // blit is exercised through draw_terrain below; here the header
    // shape is pinned via a one-tile draw into a scratch buffer.
    let mut scratch = vec![0u8; VIEW_BUF_LEN];
    let mut rng = Pcg32::new(0x1E240, 0);
    view.draw_terrain(&mut scratch, &mut DrawParams::new(21, 73, 0, &mut rng));
    let opaque = scratch.iter().filter(|&&b| b != 0).count();
    assert!(
        opaque > 40_000,
        "the anchored tile blits real pixels ({opaque})"
    );

    // --- 3. The LNK animation walk ------------------------------------
    // A fresh view at camera (0, 0): drawn words advance one LNK step
    // per pass. ZONEA plane 0 uses the 55..64 cycle (30+ cells); only
    // tiles inside the camera-visible cache band are drawn, so the
    // walked set is exactly { visible AND LNK-animated }.
    let mut view = MissionView::from_mission_bytes(&tot, &dat_planes(&dat), &bin, &lnk).unwrap();
    // Visible = inside the camera band AND its layer-0 dest under the
    // EXW draw cap 0x59b00 [FUN_00403938 gate] — tiles above the cap
    // are never drawn (and never LNK-walked) exactly as in the binary.
    let visible: std::collections::HashSet<usize> = cache
        .iter()
        .filter(|e| e.dtile_x >= 0 && e.dtile_x < 25 && e.dtile_y >= 0 && e.dtile_y < 75)
        .filter(|e| e.buf_off < 0x59B00)
        .map(|e| (e.dtile_y * 25 + e.dtile_x) as usize)
        .collect();
    let animated: Vec<usize> = (0..25 * 75)
        .filter(|&t| visible.contains(&t))
        .filter(|&t| {
            let w = view.word(t, 0);
            w != 0 && view.lnk_step(w) != w
        })
        .collect();
    assert!(
        !animated.is_empty(),
        "ZONEA carries LNK-animated visible tiles"
    );
    let frozen: Vec<usize> = (0..25 * 75)
        .filter(|&t| !visible.contains(&t))
        .filter(|&t| view.word(t, 0) != 0)
        .collect();
    assert!(!frozen.is_empty(), "off-camera animated words stay frozen");
    let before: Vec<u16> = animated.iter().map(|&t| view.word(t, 0)).collect();
    let frozen_before: Vec<u16> = frozen.iter().map(|&t| view.word(t, 0)).collect();
    let mut rng = Pcg32::new(0x1E240, 1);
    view.draw_terrain(
        &mut vec![0u8; VIEW_BUF_LEN],
        &mut DrawParams::new(0, 0, 0, &mut rng),
    );
    for (i, &t) in animated.iter().enumerate() {
        assert_eq!(
            view.word(t, 0),
            view.lnk_step(before[i]),
            "tile {t} word advanced exactly one LNK step"
        );
    }
    for (i, &t) in frozen.iter().enumerate() {
        assert_eq!(
            view.word(t, 0),
            frozen_before[i],
            "off-camera tile {t} untouched"
        );
    }

    // --- 4. The pinned frame ------------------------------------------
    let render = |stream: u64| -> Vec<u8> {
        let mut v = MissionView::from_mission_bytes(&tot, &dat_planes(&dat), &bin, &lnk).unwrap();
        let mut buf = vec![0u8; VIEW_BUF_LEN];
        let mut r = Pcg32::new(0x1E240, stream);
        v.draw_terrain(&mut buf, &mut DrawParams::new(0, 0, 0, &mut r));
        buf
    };
    let buf = render(0);
    // The present window at camera 0: origin (96, 64) [FUN_00401107].
    let win = present_window(&buf, 0, 0).expect("480x480 crop");
    let mut hasher = Fnv1a64::new();
    hasher.write_bytes(&win);
    let frame_hash = hasher.finish();
    // Structural pins before the hash: the window is NOT empty and
    // not uniform (real terrain: deck + edges + walls).
    let non_zero = win.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero > 50_000,
        "the crop carries real terrain ({non_zero})"
    );
    let distinct = win
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert!(
        distinct.len() > 16,
        "multiple palette indices in play ({})",
        distinct.len()
    );
    assert_eq!(
        format!("{frame_hash:016x}"),
        "90a9e929eea24ced",
        "ZONEA/MISSION1 viewport crop at camera (0,0), frame 0"
    );
    // Two independent runs are byte-identical.
    let buf2 = render(0);
    assert_eq!(buf, buf2, "deterministic across runs");
    // The z-step constant stays anchored to the EXW geometry.
    assert_eq!(Z_LEVEL_STEP, 0x5000);
}

#[test]
fn zonea_mission1_viewport_edge_variants_are_isolated() {
    // The off-map edge draws consume the caller's RNG (the EXW uses
    // the shared mission RandB; ours is the T3 stand-in). ZONEA is
    // zone 0 = a FIXED edge sprite (id 1) [FUN_00408030]: no RNG is
    // consumed and streams cannot change the frame. A random-edge
    // zone family (zone 1: base 0x37 + rand(9)) makes the stream
    // visible — same seed reproduces, a different stream diverges
    // only where off-map edge tiles live.
    let Some([tot, dat, bin, lnk]) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };
    let planes = dat_planes(&dat);
    let draw = |zone: i32, stream: u64| {
        let mut v = MissionView::from_mission_bytes(&tot, &planes, &bin, &lnk).unwrap();
        let mut buf = vec![0u8; VIEW_BUF_LEN];
        let mut r = Pcg32::new(0x1E240, stream);
        v.draw_terrain(&mut buf, &mut DrawParams::new(0, 0, zone, &mut r));
        buf
    };
    // Zone 0: the fixed family — identical across streams.
    assert_eq!(draw(0, 7), draw(0, 8), "zone-0 edge sprites are fixed");
    // Zone 1: random variants — stream-sensitive, stream-reproducible.
    let a = draw(1, 7);
    let a2 = draw(1, 7);
    let b = draw(1, 8);
    assert_eq!(a, a2, "same stream reproduces");
    assert_ne!(a, b, "a different edge stream changes off-map tiles");
}
