//! Loading-screen asset gate (P5): the LAB_0041c69e zone-transition tail
//! (EXW) loads, in order, GAMEGFX\BETWEEN.BIN (interlude still, drawn
//! via draw_IMG_in_buffer FUN_00401e39(0,..) = entry 0), then the
//! region-variant pair LOAD_UK/LOAD_US.BIN + LOADPAL/LOADPALU.PAL
//! (FUN_0041cc7f call sites; region byte at DAT_0046ae64). This gate
//! pins what that flow selects on, through the existing validated
//! decoders (sprites::parse_bin_images, pal::parse_vga770):
//!
//! Facts (observed 2026-08-20, read-only, manifest-bracketed):
//! - All three BINs are SINGLE-IMAGE banks: count=1, entry 0 off=4,
//!   flags=0x0003 (bit0 RLE16 + bit1 hotspot), hot=(0,0), 640x480
//!   rle16, decode ok. Full-screen VGA-mode-0x101 stills: the render
//!   Frame is 640x480x8, so the wiring unit blits 1:1 - no letterbox,
//!   no scale (unlike the 640x320 TITLE movie, D31).
//! - LOAD_UK.BIN and LOAD_US.BIN are BYTE-IDENTICAL (file sha256
//!   a63963c9..); LOADPAL.PAL and LOADPALU.PAL likewise (70ba2ca5..).
//!   The region split is a path selection in EXW code only - this data
//!   set ships one loading screen. Decode parity is therefore
//!   region-independent; Region::loading_bin/loading_pal (movies.rs,
//!   D32) still select the names EXW asks for.
//! - Both palettes: exactly 770 B (2-byte lead-in + 768 B of 6-bit
//!   VGA triples), 244 distinct colors of 256, entry 0 black, entry 1
//!   white. Expanded 8-bit RGB plane sha256 7e74c681.. pins content.
//! - Decoded BETWEEN plane sha256 6c706182.., LOAD plane sha256
//!   2d100f8b.. (distinct art: interlude vs loading screen).
//!
//! Runtime facts reserved for the NEXT unit (host wiring, from
//! RE-EXW-GAMETHREAD.md LAB_0041c69e): palette tail 0x2a2..0x301
//! forced to 0x3f after load; text draws at x=150/180/210 (zone==6
//! adds x=260); FadeSetup(pal, 10) = 10-step 50 Hz fade-in.
//!
//! Regen path: the ignored test below is the ONLY documented way to
//! print the tables. game-data access is read-only; the run is
//! bracketed by MANIFEST.sha256 checks at the shell level.

use std::path::{Path, PathBuf};

use bedlam_assets::pal::parse_vga770;
use bedlam_assets::sprites::parse_bin_images;

fn gamegfx() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX")
}

/// Pinned per-bank facts: (file sha256 head, entry off, flags, hot,
/// w, h, decoded-plane sha256).
type BankFacts<'a> = (u64, u32, u16, (i16, i16), u16, u16, &'a str);

/// First 8 bytes of the file sha256, enough to spot a re-ripped disc;
/// the decoded-plane hash below carries the content pin.
fn sha_head(data: &[u8]) -> u64 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let d = h.finalize();
    u64::from_be_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]])
}

fn plane_sha(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

const BETWEEN: BankFacts<'static> = (
    0x2414_928d_ac57_330b,
    4,
    0x0003,
    (0, 0),
    640,
    480,
    "6c70618282898dd136ef1e9aecc20661c94ef82a44b98bc02d469f67bd2fe1b6",
);

const LOAD: BankFacts<'static> = (
    0xa639_63c9_8ccc_d519,
    4,
    0x0003,
    (0, 0),
    640,
    480,
    "2d100f8b50ff30534f789ac4da35d372c8ca8cb1aa9fb6e2fabc42f3a9c8ed0b",
);

#[test]
fn loading_banks_are_pinned() {
    let dir = gamegfx();
    if !dir.is_dir() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    for (name, (fsha, off, flags, hot, w, h, psha)) in [
        ("BETWEEN.BIN", BETWEEN),
        ("LOAD_UK.BIN", LOAD),
        ("LOAD_US.BIN", LOAD),
    ] {
        let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            sha_head(&data),
            fsha,
            "{name} file identity (full-disc pin)"
        );
        let bank =
            parse_bin_images(&data).unwrap_or_else(|e| panic!("{name}: parse rejected: {e}"));
        assert_eq!(bank.count, 1, "{name}: single-image bank");
        assert_eq!(bank.images.len(), 1, "{name}: images.len");
        let im = &bank.images[0];
        assert_eq!(im.off, off, "{name}: entry-0 directory offset");
        assert_eq!(im.flags, flags, "{name}: flags (RLE16 | hotspot)");
        assert_eq!(im.hot, Some(hot), "{name}: hotspot");
        assert_eq!((im.w, im.h), (w, h), "{name}: full-screen raster");
        assert_eq!(im.codec, "rle16", "{name}: codec");
        assert!(im.ok, "{name}: decode ok");
        let px = im
            .pixels
            .as_deref()
            .unwrap_or_else(|| panic!("{name}: pixels"));
        assert_eq!(px.len(), w as usize * h as usize, "{name}: plane size");
        assert_eq!(plane_sha(px), psha, "{name}: decoded plane content");
    }
}

#[test]
fn region_variants_ship_identical_bytes() {
    let dir = gamegfx();
    if !dir.is_dir() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    let uk = std::fs::read(dir.join("LOAD_UK.BIN")).expect("LOAD_UK.BIN");
    let us = std::fs::read(dir.join("LOAD_US.BIN")).expect("LOAD_US.BIN");
    assert_eq!(uk, us, "LOAD_UK == LOAD_US byte-for-byte (corpus fact)");
    let pu = std::fs::read(dir.join("LOADPAL.PAL")).expect("LOADPAL.PAL");
    let ps = std::fs::read(dir.join("LOADPALU.PAL")).expect("LOADPALU.PAL");
    assert_eq!(pu, ps, "LOADPAL == LOADPALU byte-for-byte (corpus fact)");
}

#[test]
fn loading_palettes_are_pinned() {
    let dir = gamegfx();
    if !dir.is_dir() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    for name in ["LOADPAL.PAL", "LOADPALU.PAL"] {
        let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(data.len(), 770, "{name}: 2-byte lead-in + 768 RGB");
        let pal = parse_vga770(&data).unwrap_or_else(|e| panic!("{name}: parse rejected: {e}"));
        assert_eq!(pal.0[0], [0, 0, 0], "{name}: entry 0 black");
        assert_eq!(pal.0[1], [255, 255, 255], "{name}: entry 1 white");
        let mut distinct = std::collections::BTreeSet::new();
        distinct.extend(pal.0.iter().copied());
        assert_eq!(distinct.len(), 244, "{name}: distinct colors");
        let flat: Vec<u8> = pal.0.iter().flat_map(|c| c.iter().copied()).collect();
        assert_eq!(
            plane_sha(&flat),
            "7e74c681f875a9e71c03d74a3b9cffbfcaea572f005b2041cf80a7c440e0b1c6",
            "{name}: expanded 8-bit RGB content"
        );
    }
}

#[test]
#[ignore = "inventory regeneration only; paste output into the pinned tables"]
fn regen_inventory() {
    for name in ["BETWEEN.BIN", "LOAD_UK.BIN", "LOAD_US.BIN"] {
        let data = std::fs::read(gamegfx().join(name)).expect("read bin");
        match parse_bin_images(&data) {
            Ok(b) => {
                println!(
                    "== {name}: count={} file_len={} file_sha_head={:016x}",
                    b.count,
                    data.len(),
                    sha_head(&data)
                );
                for (i, im) in b.images.iter().enumerate() {
                    println!(
                        "  [{i:3}] off={:6} flags={:04x} hot={:?} w={:4} h={:4} codec={} ok={} pix_sha={}",
                        im.off,
                        im.flags,
                        im.hot,
                        im.w,
                        im.h,
                        im.codec,
                        im.ok,
                        im.pixels.as_deref().map(plane_sha).unwrap_or_default()
                    );
                }
            }
            Err(e) => println!("== {name}: PARSE-REJECT {e}"),
        }
    }
    for name in ["LOADPAL.PAL", "LOADPALU.PAL"] {
        let data = std::fs::read(gamegfx().join(name)).expect("read pal");
        println!(
            "== {name}: len={} file_sha_head={:016x}",
            data.len(),
            sha_head(&data)
        );
        match parse_vga770(&data) {
            Ok(p) => {
                let flat: Vec<u8> = pal_flat(&p);
                let distinct: std::collections::BTreeSet<_> = p.0.iter().copied().collect();
                println!(
                    "   distinct={} rgb_sha={}",
                    distinct.len(),
                    plane_sha(&flat)
                );
            }
            Err(e) => println!("   PAL-REJECT {e}"),
        }
    }
}

/// Flatten the 256x3 expanded palette for hashing.
fn pal_flat(p: &bedlam_assets::Palette) -> Vec<u8> {
    p.0.iter().flat_map(|c| c.iter().copied()).collect()
}
