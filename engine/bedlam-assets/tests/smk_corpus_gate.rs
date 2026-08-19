//! SMK corpus inventory gate (P5, D32): opens EVERY .SMK under
//! game-data/BEDLAM/GAMEGFX through the validated SmkStream seam and
//! pins the container facts the D31 playback integration selects on:
//! formats (w x h), frame counts, rates (us/frame), ring-ness, y-scale
//! flags, and audio track shapes. The inventory is the reject-or-map
//! verdict for the D31 policy: every corpus file MAPS onto the existing
//! playback path (letterbox blit, never scaled; PCM stream bus without
//! resampling) - nothing is rejected, no y-scale handling is owed.
//!
//! Facts (verified 2026-08-20, worker 1787177170):
//! - 25x BRF_{B..F}{1..5}: 640x480, 512 frames, 33330 us (~30 fps),
//!   RING, silent.
//! - BRF_DROP: same shape but 30 frames, non-ring.
//! - END/GAMEOVER/GTLOG_{UK,US}/LOGO_{UK,US}/ZONEDONE: 640x480,
//!   66660 us (15 fps), DPCM mono 8-bit 11025 Hz track 0; all ring
//!   except GAMEOVER (84 frames, non-ring).
//! - SHOP: 640x480, 61 frames, 25000 us (40 fps), ring, DPCM track 0.
//! - TITLE (already integrated, D30/D31): 640x320, 1227 frames,
//!   66660 us, non-ring, DPCM track 0.
//! All frame periods are exact integers on the x240-us accumulator
//! grid (33330*240 = 7_999_200, 25000*240 = 6_000_000,
//! 66660*240 = 15_998_400 units) - fractional periods bank, never
//! round (movie.rs, D31).
//!
//! Regen path: the ignored test below is the ONLY documented way to
//! print the table. game-data access is read-only; the run is
//! bracketed by MANIFEST.sha256 checks at the shell level.

use std::path::{Path, PathBuf};

use bedlam_assets::smk::{SmkAudioCodec, SmkStream, SmkYScale};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX")
}

/// (width, height, frames, us_per_frame, ring, audio track 0 codec).
/// None = silent; Some(Dpcm) = the one eligible corpus shape
/// (mono/8-bit/11025 Hz asserted separately so the shape cannot drift
/// silently).
type Facts = (u32, u32, u32, u64, bool, Option<SmkAudioCodec>);

const fn brf() -> Facts {
    (640, 480, 512, 33_330, true, None)
}

const TABLE: &[(&str, Facts)] = &[
    ("BRF_B1.SMK", brf()),
    ("BRF_B2.SMK", brf()),
    ("BRF_B3.SMK", brf()),
    ("BRF_B4.SMK", brf()),
    ("BRF_B5.SMK", brf()),
    ("BRF_C1.SMK", brf()),
    ("BRF_C2.SMK", brf()),
    ("BRF_C3.SMK", brf()),
    ("BRF_C4.SMK", brf()),
    ("BRF_C5.SMK", brf()),
    ("BRF_D1.SMK", brf()),
    ("BRF_D2.SMK", brf()),
    ("BRF_D3.SMK", brf()),
    ("BRF_D4.SMK", brf()),
    ("BRF_D5.SMK", brf()),
    ("BRF_DROP.SMK", (640, 480, 30, 33_330, false, None)),
    ("BRF_E1.SMK", brf()),
    ("BRF_E2.SMK", brf()),
    ("BRF_E3.SMK", brf()),
    ("BRF_E4.SMK", brf()),
    ("BRF_E5.SMK", brf()),
    ("BRF_F1.SMK", brf()),
    ("BRF_F2.SMK", brf()),
    ("BRF_F3.SMK", brf()),
    ("BRF_F4.SMK", brf()),
    ("BRF_F5.SMK", brf()),
    ("END.SMK", (640, 480, 348, 66_660, true, Some(SmkAudioCodec::Dpcm))),
    ("GAMEOVER.SMK", (640, 480, 84, 66_660, false, Some(SmkAudioCodec::Dpcm))),
    ("GTLOG_UK.SMK", (640, 480, 70, 66_660, true, Some(SmkAudioCodec::Dpcm))),
    ("GTLOG_US.SMK", (640, 480, 70, 66_660, true, Some(SmkAudioCodec::Dpcm))),
    ("LOGO_UK.SMK", (640, 480, 71, 66_660, true, Some(SmkAudioCodec::Dpcm))),
    ("LOGO_US.SMK", (640, 480, 71, 66_660, true, Some(SmkAudioCodec::Dpcm))),
    ("SHOP.SMK", (640, 480, 61, 25_000, true, Some(SmkAudioCodec::Dpcm))),
    ("TITLE.SMK", (640, 320, 1227, 66_660, false, Some(SmkAudioCodec::Dpcm))),
    ("ZONEDONE.SMK", (640, 480, 75, 66_660, true, Some(SmkAudioCodec::Dpcm))),
];

#[test]
fn smk_corpus_inventory_is_pinned() {
    let dir = corpus();
    if !dir.is_dir() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    let mut files: Vec<&str> = std::fs::read_dir(&dir)
        .expect("read GAMEGFX")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("smk")))
        .map(|e| e.file_name().to_string_lossy().to_uppercase())
        .map(|n| match TABLE.iter().find(|(k, _)| *k == n) {
            Some((k, _)) => *k,
            None => panic!("corpus file {n} missing from the pinned table"),
        })
        .collect();
    files.sort_unstable();
    let mut keys: Vec<&str> = TABLE.iter().map(|(k, _)| *k).collect();
    keys.sort_unstable();
    assert_eq!(files, keys, "corpus file set must match the table exactly");

    for (name, facts) in TABLE {
        let data = std::fs::read(dir.join(name)).expect("read corpus smk");
        let info = SmkStream::open(&data)
            .unwrap_or_else(|e| panic!("{name}: open rejected: {e}"))
            .info()
            .clone();
        let (w, h, frames, us, ring, codec) = *facts;
        assert_eq!((info.width, info.height), (w, h), "{name} raster");
        assert_eq!(info.frames, frames, "{name} frame count");
        assert_eq!(info.us_per_frame, us, "{name} frame interval");
        assert_eq!(info.ring_frame, ring, "{name} ring flag");
        assert_eq!(info.y_scale, SmkYScale::None, "{name} y-scale (D31: never scaled)");
        match (codec, info.audio[0]) {
            (None, None) => {}
            (Some(expected), Some(m)) => {
                assert_eq!(m.codec, expected, "{name} track 0 codec");
                // The D31 map verdict: the ONE audio shape the playback
                // path consumes, corpus-wide.
                assert_eq!(
                    (m.channels, m.bitdepth, m.rate_hz),
                    (1, 8, 11_025),
                    "{name} track 0 must stay mixer-native"
                );
            }
            (expected, got) => panic!(
                "{name}: track 0 mismatch: table {expected:?} vs corpus {got:?}"
            ),
        }
        for t in 1..7 {
            assert!(info.audio[t].is_none(), "{name}: track {t} unexpected");
        }
    }
}

#[test]
#[ignore = "inventory regeneration only; paste output into the pinned table"]
fn regen_inventory() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(corpus())
        .expect("read GAMEGFX")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("smk")))
        .collect();
    paths.sort();
    for p in paths {
        let data = std::fs::read(&p).expect("read smk");
        let name = p.file_name().unwrap().to_string_lossy().to_uppercase();
        match SmkStream::open(&data) {
            Ok(s) => {
                let i = s.info();
                let audio: Vec<String> = i
                    .audio
                    .iter()
                    .enumerate()
                    .filter_map(|(n, t)| {
                        t.map(|m| {
                            format!(
                                "{}:{:?}/{}/{}/{}",
                                n, m.codec, m.channels, m.bitdepth, m.rate_hz
                            )
                        })
                    })
                    .collect();
                println!(
                    "{name} {}x{} frames={} us={} ring={} ys={:?} audio=[{}]",
                    i.width,
                    i.height,
                    i.frames,
                    i.us_per_frame,
                    i.ring_frame,
                    i.y_scale,
                    audio.join(",")
                );
            }
            Err(e) => println!("{name} OPEN-REJECT {e}"),
        }
    }
}
