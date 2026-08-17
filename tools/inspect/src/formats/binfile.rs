//! CLI dump shims over the bedlam-assets crate: file I/O, JSON docs and PNG
//! rendering stay here; all parsing lives in the library.

use crate::formats::pal;
use crate::stem_of;
use bedlam_assets as assets;
use bedlam_assets::AssetsError;
use serde::Serialize;
use std::fs;
use std::path::Path;

fn write_json(out_dir: &Path, name: &str, doc: &serde_json::Value) -> bool {
    let _ = fs::create_dir_all(out_dir);
    fs::write(
        out_dir.join(name),
        serde_json::to_string_pretty(doc).unwrap_or_default(),
    )
    .is_ok()
}

/// Same field order as the legacy ImageMeta so the emitted JSON is identical.
#[derive(Serialize)]
struct ImageMeta {
    i: usize,
    off: u32,
    flags: u16,
    hot: Option<(i16, i16)>,
    w: u16,
    h: u16,
    codec: String,
    ok: bool,
}

fn render(
    out_dir: &Path,
    stem: &str,
    i: usize,
    px: &[u8],
    w: u32,
    h: u32,
    p: &[[u8; 3]; 256],
) -> bool {
    let mut img = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let ci = px[(y * w + x) as usize] as usize;
            img.put_pixel(x, y, image::Rgb([p[ci][0], p[ci][1], p[ci][2]]));
        }
    }
    img.save(out_dir.join(format!("{}_{:04}_{}x{}.png", stem, i, w, h)))
        .is_ok()
}

pub fn bin_images(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    if stem_of(rel).to_uppercase() == "SINTABLE" {
        return (
            String::from("parsed"),
            String::from("256-entry sine LUT (amp 32767; not an image bank)"),
        );
    }
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let bank = match assets::sprites::parse_bin_images(&data) {
        Ok(b) => b,
        Err(AssetsError::TooSmall { len }) => {
            return (String::from("heuristic-failed"), format!("{}B", len))
        }
        Err(AssetsError::CountOverruns { count, len }) => {
            return (
                String::from("heuristic-failed"),
                format!("count {} vs {}B", count, len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let palette = pal::sibling_vga770(path);
    let png_dir = out_dir.join(format!("{}.images", stem_of(rel)));
    if palette.is_some() {
        let _ = fs::create_dir_all(&png_dir);
    }
    let mut metas: Vec<ImageMeta> = Vec::with_capacity(bank.count);
    let mut ok_count = 0usize;
    let mut rendered = 0usize;
    for (i, img) in bank.images.iter().enumerate() {
        if img.ok {
            ok_count += 1;
        }
        if let (Some(px), Some(p)) = (&img.pixels, palette.as_ref()) {
            if rendered < 512
                && (img.w as usize) * (img.h as usize) <= 1_000_000
                && render(
                    &png_dir,
                    &stem_of(rel),
                    i,
                    px,
                    img.w as u32,
                    img.h as u32,
                    p,
                )
            {
                rendered += 1;
            }
        }
        metas.push(ImageMeta {
            i,
            off: img.off,
            flags: img.flags,
            hot: img.hot,
            w: img.w,
            h: img.h,
            codec: img.codec.clone(),
            ok: img.ok,
        });
    }
    let doc =
        serde_json::json!({ "file": rel, "count": bank.count, "ok": ok_count, "images": metas });
    let _ = write_json(out_dir, &format!("{}.bin.json", stem_of(rel)), &doc);
    let cov = ok_count * 100 / bank.count;
    let status = if cov >= 50 {
        String::from("parsed")
    } else {
        String::from("partial")
    };
    (
        status,
        format!(
            "count={} decoded={} rendered={} pal={}",
            bank.count,
            ok_count,
            rendered,
            palette.is_some()
        ),
    )
}

pub fn cgr_tiles(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let bank = match assets::tiles::parse_cgr_tiles(&data) {
        Ok(b) => b,
        Err(AssetsError::TooSmall { len }) => {
            return (String::from("heuristic-failed"), format!("{}B", len))
        }
        Err(AssetsError::CountOverruns { count, len }) => {
            return (
                String::from("heuristic-failed"),
                format!("count {} vs {}B", count, len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let palette = pal::sibling_vga770(path);
    let png_dir = out_dir.join(format!("{}.tiles", stem_of(rel)));
    if palette.is_some() {
        let _ = fs::create_dir_all(&png_dir);
    }
    let mut metas: Vec<serde_json::Value> = Vec::new();
    let mut ok_count = 0usize;
    let mut rendered = 0usize;
    for (i, t) in bank.tiles.iter().enumerate() {
        if t.short {
            metas.push(serde_json::json!({ "i": i, "ok": false, "why": "short" }));
            continue;
        }
        if t.ok {
            ok_count += 1;
        }
        if let (Some(px), Some(palv)) = (&t.pixels, palette.as_ref()) {
            if rendered < 160
                && render(&png_dir, &stem_of(rel), i, px, t.w as u32, t.h as u32, palv)
            {
                rendered += 1;
            }
        }
        metas.push(serde_json::json!({
            "i": i, "off": t.off, "w0": t.w0, "ok": t.ok, "codec": t.codec, "w": t.w, "h": t.h,
        }));
    }
    let doc =
        serde_json::json!({ "file": rel, "count": bank.count, "ok": ok_count, "tiles": metas });
    let _ = write_json(out_dir, &format!("{}.cgr.json", stem_of(rel)), &doc);
    let status = if ok_count * 2 >= bank.count {
        String::from("parsed")
    } else {
        String::from("partial")
    };
    (
        status,
        format!(
            "count={} tiles-ok={} rendered={}",
            bank.count, ok_count, rendered
        ),
    )
}

pub fn min_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let m = match assets::misc::parse_min(&data) {
        Ok(m) => m,
        Err(AssetsError::NotMultiple { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} not 16n", len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let doc = serde_json::json!({ "file": rel, "tile_count": m.tile_count(), "bytes": data });
    let ok = write_json(out_dir, &format!("{}.min.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} tile colors (16B each)", m.tile_count()),
    )
}

pub fn lnk_lng(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let l = match assets::misc::parse_lnk_lng(&data) {
        Ok(l) => l,
        Err(AssetsError::WrongSize { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} != 16384", len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let identity = l.identity_count();
    let doc = serde_json::json!({ "file": rel, "entries": l.entries, "identity_count": identity });
    let ok = write_json(out_dir, &format!("{}.lnk.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("8192 u16 remap; identity {}", identity),
    )
}

pub fn mrw(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let m = match assets::misc::parse_mrw(&data) {
        Ok(m) => m,
        Err(AssetsError::TooSmall { len }) => {
            return (String::from("heuristic-failed"), format!("{}B", len))
        }
        Err(AssetsError::CountOverruns { count, .. }) => {
            return (String::from("heuristic-failed"), format!("count {}", count))
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let mut chunks: Vec<serde_json::Value> = Vec::new();
    let wav_dir = out_dir.to_path_buf();
    let _ = fs::create_dir_all(&wav_dir);
    for (i, ch) in m.chunks.iter().enumerate() {
        chunks.push(serde_json::json!({
            "i": i, "offset": ch.off, "size": ch.size, "fits": ch.fits
        }));
        if ch.fits && ch.size > 0 && ch.size < 4_000_000 {
            let pcm = &data[ch.off as usize..(ch.off + ch.size) as usize];
            let wav = assets::audio::wav_wrap(pcm, 11025);
            let _ = fs::write(
                wav_dir.join(format!("{}_chunk{:02}.wav", stem_of(rel), i)),
                &wav,
            );
        }
    }
    let doc = serde_json::json!({ "file": rel, "count": m.count, "chunks": chunks, "rate": 11025, "fmt": "8-bit mono" });
    let ok = write_json(out_dir, &format!("{}.mrw.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} instrument chunks (11025Hz 8-bit mono)", m.count),
    )
}

pub fn saved_bdl(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let s = match assets::bdl::parse_saved_bdl(&data) {
        Ok(s) => s,
        Err(AssetsError::WrongSize { len }) => {
            return (
                String::from("unknown-variant"),
                format!("len {} != 900", len),
            )
        }
        Err(e) => return (String::from("unknown-variant"), e.to_string()),
    };
    let mut slots: Vec<serde_json::Value> = Vec::new();
    for (sl, slot) in s.slots.iter().enumerate() {
        slots.push(serde_json::json!({
            "slot": sl, "name": slot.name,
            "completed_mask": format!("{:08x}", slot.completed_mask),
            "zone": slot.zone, "money": slot.money,
        }));
    }
    let doc = serde_json::json!({ "file": rel, "size": 900, "slots": slots });
    let ok = write_json(out_dir, &format!("{}.saved.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        String::from("5 save slots decoded"),
    )
}

pub fn hiscore_bdl(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let h = match assets::bdl::parse_hiscore_bdl(&data) {
        Ok(h) => h,
        Err(AssetsError::WrongSize { len }) => {
            return (
                String::from("unknown-variant"),
                format!("len {} != 120", len),
            )
        }
        Err(e) => return (String::from("unknown-variant"), e.to_string()),
    };
    let mut scores: Vec<serde_json::Value> = Vec::new();
    for (sl, e) in h.scores.iter().enumerate() {
        scores.push(serde_json::json!({ "rank": sl + 1, "score": e.score, "name": e.name }));
    }
    let doc = serde_json::json!({ "file": rel, "scores": scores });
    let ok = write_json(out_dir, &format!("{}.hiscore.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        String::from("10 hi-scores decoded"),
    )
}

pub fn options_bdl(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let o = match assets::bdl::parse_options_bdl(&data) {
        Ok(o) => o,
        Err(AssetsError::TooSmall { len }) => {
            return (String::from("unknown-variant"), format!("len {} < 41", len))
        }
        Err(e) => return (String::from("unknown-variant"), e.to_string()),
    };
    let doc = serde_json::json!({
        "file": rel, "size": data.len(),
        "backbuffer": o.backbuffer, "actionpan": o.actionpan,
        "language": o.language, "cd_audio": o.cd_audio,
        "playername": o.playername, "volume": o.volume,
        "code_no_title": o.code_no_title, "midi": o.midi, "sound": o.sound,
        "installdrive": o.installdrive,
    });
    let ok = write_json(out_dir, &format!("{}.options.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("options struct; player={}", o.playername),
    )
}

pub fn nme_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let n = assets::misc::parse_nme(&data);
    let mut sections: Vec<serde_json::Value> = Vec::new();
    for sec in &n.sections {
        let v = match sec {
            assets::misc::NmeSection::Zero { sec, at } => serde_json::json!({
                "sec": sec, "count": 0, "at": at
            }),
            assets::misc::NmeSection::Tail { sec, count, at } => serde_json::json!({
                "sec": sec, "count": count, "at": at, "note": "tail"
            }),
            assets::misc::NmeSection::Records {
                sec,
                count,
                rec,
                at,
                sample,
            } => {
                let recs: Vec<serde_json::Value> = sample
                    .iter()
                    .enumerate()
                    .map(|(i, w)| serde_json::json!({ "i": i, "w": w }))
                    .collect();
                serde_json::json!({
                    "sec": sec, "count": count, "rec": rec, "at": at, "sample": recs
                })
            }
        };
        sections.push(v);
    }
    let doc = serde_json::json!({ "file": rel, "size": n.size, "sections": sections });
    let ok = write_json(out_dir, &format!("{}.nme.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!(
            "{} sections (8street layout, fields partial)",
            n.sections.len()
        ),
    )
}

pub fn bdg_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let b = assets::misc::parse_bdg(&data);
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut active = 0usize;
    for (idx, rec) in b.records.iter().enumerate() {
        match rec {
            assets::misc::BdgRecord::Inactive => {
                recs.push(serde_json::json!({ "i": idx, "active": false }));
            }
            assets::misc::BdgRecord::Active {
                w,
                h,
                dep,
                blobs3,
                head_hex,
            } => {
                active += 1;
                recs.push(serde_json::json!({
                    "i": idx, "active": true, "w": w, "h": h, "d": dep,
                    "blobs": blobs3, "head": head_hex,
                }));
            }
        }
    }
    let doc = serde_json::json!({ "file": rel, "size": b.size, "records": recs });
    let ok = write_json(out_dir, &format!("{}.bdg.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} records, {} active", b.records.len(), active),
    )
}

pub fn text_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let printable = assets::misc::ascii_pct(&data);
    let ok = write_json(
        out_dir,
        &format!("{}.txt.json", stem_of(rel)),
        &serde_json::json!({ "file": rel, "size": data.len(), "ascii_pct": printable }),
    );
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}B, {}% ascii (designer notes)", data.len(), printable),
    )
}
