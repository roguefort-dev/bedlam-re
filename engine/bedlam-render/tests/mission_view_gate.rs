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
//! 5. ENTITIES: the FUN_00403938 robot loop + FUN_0040798e/0179b
//!    enqueue/flush overlay (MISSIONVIEW sec 5b–5d) draws the
//!    ZONEA/MISSION1 spawned robot and the order-walking second robot
//!    from a bedlam-core MissionSim onto the frame with real
//!    GAMEGFX\DANTE.BIN sprites — hash-pinned at spawn and mid-walk,
//!    with the terrain-only frame as the no-entities regression pin.
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
        "a326c73fe710e501",
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
    // EXW 0x408035 subtracts one from the original zone. Engine zone
    // zero (A) uses random water edges, not the invalid-zone fallback.
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
    // Invalid zone retains the fixed fallback, without RNG dependence.
    assert_eq!(draw(-1, 7), draw(-1, 8));
    let a = draw(0, 7);
    let a2 = draw(0, 7);
    let b = draw(0, 8);
    assert_eq!(a, a2, "same stream reproduces");
    assert_ne!(a, b, "a different edge stream changes off-map tiles");
}

/// The ZONEA/MISSION1 entity overlay: the FUN_00403938 robot loop
/// over a bedlam-core MissionSim, enqueued (FUN_0040798e) and flushed
/// (FUN_0040179b) onto the terrain frame with real DANTE.BIN sprites
/// [MISSIONVIEW sec 5b–5d]. Two pinned moments: both robots at their
/// spawn spots, and the second robot mid-walk east (order armed at
/// robot 0 — the same scripted run the sim corpus gate pins).
#[test]
fn zonea_mission1_entity_overlay_frame_hash_pinned() {
    let Some([tot, dat, bin, lnk]) = zonea() else {
        eprintln!("corpus absent - skipping (CI)");
        return;
    };
    let Some(dante) = read(&["GAMEGFX", "DANTE.BIN"]) else {
        eprintln!("DANTE.BIN absent - skipping");
        return;
    };
    let (Some(pad), Some(cgr), Some(sintable)) = (
        read(&["EDITOR", "ZONEA", "MISSION1.PAD"]),
        read(&["EDITOR", "ZONEA", "MISSIONA.CGR"]),
        read(&["GAMEGFX", "SINTABLE.BIN"]),
    ) else {
        eprintln!("sim corpus files absent - skipping");
        return;
    };

    // --- the sim side: the scripted spawn + walk ---------------------
    let terrain =
        bedlam_core::mission::Terrain::from_mission_bytes(&dat, &pad, &cgr).expect("DAT parse");
    let mut words = [0i16; 256];
    for (i, w) in words.iter_mut().enumerate() {
        *w = i16::from_le_bytes([sintable[2 * i], sintable[2 * i + 1]]);
    }
    let angles = bedlam_core::mission::AngleTable::from_sintable_words(&words).expect("angles");
    let mut sim = bedlam_core::mission::MissionSim::new(terrain, angles, 0x1E240);
    let a = sim.spawn_robot((21, 73, 1)); // ZONEA MRK record 0
    let b = sim.spawn_robot((18, 73, 1)); // staged second robot
    assert!(sim.arm_order_at_robot(a));

    // --- the view side -----------------------------------------------
    let planes = dat_planes(&dat);
    let mut view = MissionView::from_mission_bytes(&tot, &planes, &bin, &lnk).unwrap();
    view.set_entity_bank(&dante);
    // DANTE.BIN sanity: 160 sprites [corpus]; the two spawn-default
    // frames (anim word + the +0x20 base) resolve with nonzero gates.
    assert_eq!(u16::from_le_bytes([dante[0], dante[1]]), 160);
    for id in [0u16, 0x20] {
        let e = 2 + 4 * id as usize;
        let off = u32::from_le_bytes(dante[e..e + 4].try_into().unwrap()) as usize;
        let s = e + off;
        let hdr = |i: usize| u16::from_le_bytes([dante[s + 2 * i], dante[s + 2 * i + 1]]);
        assert!(hdr(3) != 0, "DANTE sprite {id:#x} gate word");
    }

    // Camera on the robots: Q5 (19*32, 73*32) — tile (19, 73), fine 0.
    let cam_q5 = (19 * 32, 73 * 32);
    let render =
        |sim: &bedlam_core::mission::MissionSim, view: &mut MissionView| -> (Vec<u8>, usize) {
            let robots: Vec<_> = sim
                .robots()
                .iter()
                .map(bedlam_render::mission_view::RobotView::from_sim)
                .collect();
            view.enqueue_robots(&robots, cam_q5.0, cam_q5.1, 0, sim.frame());
            let queued = view.sprite_nodes();
            let mut buf = vec![0u8; VIEW_BUF_LEN];
            let mut rng = Pcg32::new(0x1E240, 0);
            view.draw_terrain(&mut buf, &mut DrawParams::new(19, 73, 0, &mut rng));
            (buf, queued)
        };

    // --- moment A: spawn (both robots at their MRK spots) ------------
    assert_eq!(view.sprite_nodes(), 0, "nothing queued before enqueue");
    let (buf_a, queued_a) = render(&sim, &mut view);
    // Spawn defaults enqueue exactly two DANTE sprites per robot
    // (body = anim 0, base + 0x20) [MISSIONVIEW sec 5d].
    assert_eq!(queued_a, 4, "2 robots x 2 sprites");
    assert_eq!(view.sprite_nodes(), 0, "draw_terrain consumed the list");
    let win_a = present_window(&buf_a, 0, 0).expect("crop");
    let non_zero = win_a.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero > 50_000,
        "the crop carries real content ({non_zero})"
    );

    // --- moment B: mid-walk ------------------------------------------
    for _ in 0..3 {
        sim.advance_frame();
    }
    let walker = &sim.robots()[b];
    assert_eq!(walker.state, bedlam_core::mission::STATE_MOVING);
    assert_ne!(walker.anim, 0, "the walk anim phase is live");
    let (buf_b, queued_b) = render(&sim, &mut view);
    assert_eq!(queued_b, 4);

    // --- the entity effect is real and localized ---------------------
    // Without entities the SAME terrain state draws differently only
    // where the robots stand (the list is empty without enqueue).
    let terrain_only = {
        let mut v2 = MissionView::from_mission_bytes(&tot, &planes, &bin, &lnk).unwrap();
        // Two LNK steps to match the two drawn frames of the reused view.
        let mut rng = Pcg32::new(0x1E240, 0);
        let mut buf = vec![0u8; VIEW_BUF_LEN];
        v2.draw_terrain(&mut buf, &mut DrawParams::new(19, 73, 0, &mut rng));
        v2.draw_terrain(&mut buf, &mut DrawParams::new(19, 73, 0, &mut rng));
        buf
    };
    let diff_a: usize = buf_a
        .iter()
        .zip(terrain_only.iter())
        .filter(|(x, y)| x != y)
        .count();
    assert!(
        diff_a > 500,
        "the robots paint real pixels ({diff_a} bytes differ)"
    );

    // --- hash pins ----------------------------------------------------
    let mut h = Fnv1a64::new();
    h.write_bytes(&win_a);
    let hash_a = format!("{:016x}", h.finish());
    let win_b = present_window(&buf_b, 0, 0).expect("crop");
    let mut h2 = Fnv1a64::new();
    h2.write_bytes(&win_b);
    let hash_b = format!("{:016x}", h2.finish());
    eprintln!("entity frame A hash {hash_a}, frame B hash {hash_b}");
    assert_ne!(hash_a, hash_b, "the walking robot changes the frame");
    assert_eq!(
        hash_a, "761094cbe33c6b84",
        "ZONEA/MISSION1 spawn-moment entity frame at camera (19,73)"
    );
    assert_eq!(
        hash_b, "00dc2ec0c12c052a",
        "ZONEA/MISSION1 mid-walk entity frame (3 frames, robot b walking east)"
    );
}
