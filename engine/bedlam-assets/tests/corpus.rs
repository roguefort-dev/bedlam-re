//! Corpus integration test: runs every bedlam-assets parser over a
//! deterministic sample of game-data, asserting no panics, lossless rebuilds
//! for the byte-exact formats, and codec round-trips for sampled images and
//! tiles. Read-only: never writes anything under game-data/.

use std::fs;
use std::path::{Path, PathBuf};

use bedlam_assets as assets;
use bedlam_assets::AssetsError;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data")
}

/// Deterministic recursive walk, sorted like the inspect CLI's.
fn walk_sorted(dir: &Path, out: &mut Vec<PathBuf>) {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            walk_sorted(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn stem_of(p: &Path) -> String {
    let base = p
        .file_name()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    match base.rfind('.') {
        Some(i) => base[..i].to_lowercase(),
        None => base.to_lowercase(),
    }
}

/// Extension families we force-include (one file each, if present).
const FAMILIES: &[&str] = &[
    "pal", "trn", "bin", "cgr", "min", "lnk", "mrw", "mrs", "map", "tot", "col", "dat", "trt",
    "mrk", "pos", "pad", "pth", "raw", "smk", "bdl", "nme", "bdg",
];

/// Deterministic sample: first file of each family, then every Nth remaining
/// file until ~80 total.
fn sample(all: &[PathBuf]) -> Vec<PathBuf> {
    let mut sample: Vec<PathBuf> = Vec::new();
    let mut forced_exts: Vec<String> = Vec::new();
    for fam in FAMILIES {
        if let Some(first) = all.iter().find(|p| ext_of(p) == *fam) {
            sample.push(first.clone());
            forced_exts.push((*fam).to_string());
        }
    }
    let rest: Vec<&PathBuf> = all
        .iter()
        .filter(|p| !sample.iter().any(|s| s == *p))
        .collect();
    let remaining = 80usize.saturating_sub(sample.len());
    if remaining > 0 && !rest.is_empty() {
        let step = (rest.len() / remaining).max(1);
        for f in rest.iter().step_by(step) {
            if sample.len() >= 80 {
                break;
            }
            sample.push((*f).clone());
        }
    }
    sample
}

#[test]
fn corpus_parses_rebuilds_and_round_trips() {
    let root = corpus_root();
    if !root.is_dir() {
        eprintln!("corpus not found - skipping");
        return;
    }
    let mut all = Vec::new();
    walk_sorted(&root, &mut all);
    assert!(!all.is_empty(), "corpus dir exists but is empty");

    let sel = sample(&all);
    eprintln!(
        "corpus: {} files total, sample {} (families forced: {})",
        all.len(),
        sel.len(),
        FAMILIES.len()
    );

    let mut ok = 0usize;
    let mut err = 0usize;
    let mut rebuilt = 0usize;
    let mut rle16_round_trips = 0usize;
    let mut byterle_round_trips = 0usize;
    let mut byterle_coded_found = 0usize;
    let mut saw_bin = false;
    let mut saw_cgr = false;
    let mut saw_mrs = false;

    for p in &sel {
        let data = match fs::read(p) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let ext = ext_of(p);
        let stem = stem_of(p);
        // run the appropriate parser; Err is fine, panic is not (test fails)
        match ext.as_str() {
            "pal" => {
                let _ = assets::pal::parse_vga770(&data);
                ok += 1;
            }
            "trn" => match assets::trn::parse_trn(&data) {
                Ok(t) => {
                    assert_eq!(t.to_bytes(), data, "trn rebuild {}", p.display());
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "bin" => match assets::sprites::parse_bin_images(&data) {
                Ok(bank) => {
                    saw_bin = true;
                    for img in &bank.images {
                        if rle16_round_trips >= 20 || img.codec != "rle16" {
                            continue;
                        }
                        if let Some(px) = &img.pixels {
                            let w = img.w as usize;
                            let h = img.h as usize;
                            assert_eq!(
                                assets::codecs::decode_rle16(
                                    &assets::codecs::encode_rle16(w, h, px),
                                    w,
                                    h
                                )
                                .unwrap(),
                                *px,
                                "rle16 round trip {}",
                                p.display()
                            );
                            rle16_round_trips += 1;
                        }
                    }
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "cgr" => match assets::tiles::parse_cgr_tiles(&data) {
                Ok(bank) => {
                    saw_cgr = true;
                    for tile in &bank.tiles {
                        if tile.codec == "byterle" {
                            byterle_coded_found += 1;
                        }
                        // Round-trip corpus tile pixels through the byterle
                        // codec. (The shipped corpus stores every tile raw;
                        // byterle_coded_found tracks any exception.)
                        if byterle_round_trips >= 20 || !tile.ok {
                            continue;
                        }
                        if let Some(px) = &tile.pixels {
                            let (w, h) = (tile.w, tile.h);
                            assert_eq!(
                                assets::codecs::decode_byterle(
                                    &assets::codecs::encode_byterle(w, h, px),
                                    w,
                                    h
                                )
                                .unwrap(),
                                *px,
                                "byterle round trip {}",
                                p.display()
                            );
                            byterle_round_trips += 1;
                        }
                    }
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "min" => match assets::misc::parse_min(&data) {
                Ok(m) => {
                    assert_eq!(m.to_bytes(), data, "min rebuild {}", p.display());
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "lnk" | "lng" => match assets::misc::parse_lnk_lng(&data) {
                Ok(l) => {
                    assert_eq!(l.to_bytes(), data, "lnk rebuild {}", p.display());
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "mrw" => {
                let _ = assets::music::parse_mrw(&data);
                ok += 1;
            }
            "mrs" => match assets::music::parse_mrs(&data) {
                Ok(m) => {
                    saw_mrs = true;
                    assert_eq!(m.to_bytes(), data, "mrs rebuild {}", p.display());
                    validate_mrs_song(&m, p, &stem);
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "map" | "tot" | "col" => match assets::mission::parse_grid16(&data) {
                Ok(g) => {
                    assert_eq!(g.to_bytes_grid16(), data, "grid16 rebuild {}", p.display());
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "dat" => match assets::mission::parse_grid8(&data) {
                Ok(g) => {
                    assert_eq!(g.to_bytes_grid8(), data, "grid8 rebuild {}", p.display());
                    rebuilt += 1;
                    ok += 1;
                }
                Err(_) => err += 1,
            },
            "trt" => {
                let _ = assets::mission::parse_trt(&data);
                ok += 1;
            }
            "mrk" => {
                let _ = assets::mission::parse_mrk(&data);
                ok += 1;
            }
            "pos" => {
                let _ = assets::mission::parse_pos(&data);
                ok += 1;
            }
            "pad" => {
                let _ = assets::mission::parse_pad(&data);
                ok += 1;
            }
            "pth" => {
                let _ = assets::mission::parse_pth(&data);
                ok += 1;
            }
            "raw" => {
                let wav = assets::audio::wav_wrap(&data, 11025);
                assert_eq!(&wav[0..4], b"RIFF");
                assert_eq!(&wav[8..12], b"WAVE");
                ok += 1;
            }
            "smk" => {
                let _ = assets::smk::parse_smk_header(&data);
                // structural validator + backend open across the corpus
                if let Ok(mut s) = assets::smk::SmkStream::open(&data) {
                    let _ = s.first_frame();
                }
                ok += 1;
            }
            "bdl" => match stem.as_str() {
                "saved" => match assets::bdl::parse_saved_bdl(&data) {
                    Ok(s) => {
                        assert_eq!(s.to_bytes(), data, "saved rebuild {}", p.display());
                        rebuilt += 1;
                        ok += 1;
                    }
                    Err(_) => err += 1,
                },
                "hiscore" => match assets::bdl::parse_hiscore_bdl(&data) {
                    Ok(h) => {
                        assert_eq!(h.to_bytes(), data, "hiscore rebuild {}", p.display());
                        rebuilt += 1;
                        ok += 1;
                    }
                    Err(_) => err += 1,
                },
                "options" => {
                    let _ = assets::bdl::parse_options_bdl(&data);
                    ok += 1;
                }
                _ => ok += 1,
            },
            "nme" => {
                let _ = assets::misc::parse_nme(&data);
                ok += 1;
            }
            "bdg" => {
                let _ = assets::misc::parse_bdg(&data);
                ok += 1;
            }
            _ => {
                // not an asset format: still feed the bytes to nothing here
            }
        }
    }

    eprintln!(
        "corpus sample: {} files, ok={} err={} rebuilt={} rle16_rt={} byterle_rt={} (byterle-coded tiles in sample: {})",
        sel.len(),
        ok,
        err,
        rebuilt,
        rle16_round_trips,
        byterle_round_trips,
        byterle_coded_found
    );

    // The families were force-included, so these must have executed.
    if all.iter().any(|p| ext_of(p) == "bin") {
        assert!(saw_bin, "no bin file parsed from sample");
        assert!(rle16_round_trips > 0, "no rle16 round trip executed");
    }
    if all.iter().any(|p| ext_of(p) == "cgr") {
        assert!(saw_cgr, "no cgr file parsed from sample");
        assert!(byterle_round_trips > 0, "no byterle round trip executed");
    }
    if all.iter().any(|p| ext_of(p) == "mrs") {
        assert!(saw_mrs, "no mrs file parsed from sample");
    }
}

/// Shipped-corpus invariants for one .MRS song (all verified byte-exact
/// against the five shipped files; see docs/RE-EXW-MUSIC.md sec 2/2b):
/// chunk 0 disabled, chunk 1 = loop timer whose table-B delay is the song
/// length and equals its first event delta, every enabled stream ends in a
/// 2-byte freeze word (or the loop restart + freeze word), and every note
/// instrument is in range for the sibling .MRW bank.
fn validate_mrs_song(m: &assets::music::Mrs, p: &Path, stem: &str) {
    use assets::music::{MrsEvent, MrsWalkEnd};

    assert!(m.chunk_count >= 2, "{}: no chunks", p.display());
    assert!(m.is_disabled(0), "{}: chunk 0 not disabled", p.display());

    // known shipped song lengths in 10 ms ticks [DATA]
    let known_len = match stem {
        "brief" => Some(331),
        "shop" => Some(400),
        "select" => Some(1476),
        "debrief" => Some(1600),
        "options" => Some(3388),
        _ => None,
    };
    if let Some(len) = known_len {
        assert_eq!(m.song_len_ticks(), Some(len), "{}: song len", p.display());
    }

    // chunk 1 = loop timer: unconditional restart on channel 0, delta
    // equal to the table-B override (the song length)
    let (ev1, end1) = m.walk(1).expect("chunk 1 walkable");
    match (&ev1[0], &end1) {
        (
            MrsEvent::Restart {
                delta,
                chan: 0,
                conditional: false,
            },
            MrsWalkEnd::Restart { at },
        ) => {
            assert_eq!(*at, m.sizes[1] as usize - 2, "{}: c1 tail", p.display());
            assert_eq!(
                Some(*delta),
                m.tick_delay(1, 0),
                "{}: c1 delta != table B",
                p.display()
            );
        }
        other => panic!("{}: chunk 1 shape {other:?}", p.display()),
    }

    // sibling .MRW bank: every note instrument must be < n_inst
    let mrw_data = fs::read(p.with_extension("MRW")).ok();
    let bank = mrw_data
        .as_deref()
        .and_then(|d| assets::music::parse_mrw(d).ok());

    for chunk in 0..m.chunk_count {
        let size = m.sizes[chunk] as usize;
        if m.is_disabled(chunk) {
            continue;
        }
        let (ev, end) = m
            .walk(chunk)
            .unwrap_or_else(|| panic!("{}: chunk {chunk} walk failed", p.display()));
        let at = match end {
            MrsWalkEnd::Freeze { at } | MrsWalkEnd::Restart { at } => at,
            other => panic!("{}: chunk {chunk} ended {other:?}", p.display()),
        };
        assert_eq!(
            at,
            size - 2,
            "{}: chunk {chunk} terminal freeze word",
            p.display()
        );
        if let Some(b) = &bank {
            for e in &ev {
                if let MrsEvent::Note { instrument, .. } = e {
                    assert!(
                        (*instrument as usize) < b.count,
                        "{}: chunk {chunk} inst {instrument} >= n_inst {}",
                        p.display(),
                        b.count
                    );
                }
            }
        }
    }
}

/// Feeding random-ish bytes (tiny LCG, no rand dep) through every parser must
/// never panic. Err results are fine — they are the whole point.
#[test]
fn corpus_free_fuzz_no_panics() {
    let mut s = 0xC0FFEEu64;
    let mut next = move || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as u8
    };
    let sizes = [
        0usize, 1, 2, 3, 4, 5, 6, 9, 11, 12, 16, 17, 41, 120, 121, 256, 900, 901, 104, 1024, 16384,
    ];
    for len in sizes {
        let d: Vec<u8> = (0..len).map(|_| next()).collect();
        let _ = assets::pal::parse_vga770(&d);
        let _ = assets::trn::parse_trn(&d);
        let _ = assets::sprites::parse_bin_images(&d);
        let _ = assets::tiles::parse_cgr_tiles(&d);
        let _ = assets::mission::parse_grid16(&d);
        let _ = assets::mission::parse_grid8(&d);
        let _ = assets::mission::parse_trt(&d);
        let _ = assets::mission::parse_mrk(&d);
        let _ = assets::mission::parse_pos(&d);
        let _ = assets::mission::parse_pad(&d);
        let _ = assets::mission::parse_pth(&d);
        let _ = assets::misc::parse_min(&d);
        let _ = assets::misc::parse_lnk_lng(&d);
        let _ = assets::music::parse_mrw(&d);
        let _ = assets::music::parse_mrs(&d);
        let _ = assets::misc::parse_nme(&d);
        let _ = assets::misc::parse_bdg(&d);
        let _ = assets::smk::parse_smk_header(&d);
        let _ = assets::smk::SmkStream::open(&d);
        let _ = assets::bdl::parse_saved_bdl(&d);
        let _ = assets::bdl::parse_hiscore_bdl(&d);
        let _ = assets::bdl::parse_options_bdl(&d);
        let _ = assets::audio::wav_wrap(&d, 11025);
        // and a few random w/h + mrs walk params through the codecs directly
        let w = (next() as usize % 33) + 1;
        let h = (next() as usize % 9) + 1;
        let start = (next() as usize) % 300;
        let variant = (next() as u16) % 8;
        let _ = assets::music::walk_mrs_chunk(&d, start, variant);
        let _ = assets::codecs::decode_rle16(&d, w, h);
        let _ = assets::codecs::decode_byterle(&d, w, h);
        let _ = assets::codecs::decode_raw(&d, w, h);
    }
    // sanity: the error type is constructible/comparable as documented
    assert_eq!(
        AssetsError::WrongSize { len: 3 },
        AssetsError::WrongSize { len: 3 }
    );
}
