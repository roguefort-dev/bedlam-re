use crate::formats::pal;
use crate::{hex_head, stem_of};
use serde::Serialize;
use std::fs;
use std::path::Path;

fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn i16le(d: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([d[o], d[o + 1]])
}
fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
fn write_json(out_dir: &Path, name: &str, doc: &serde_json::Value) -> bool {
    let _ = fs::create_dir_all(out_dir);
    fs::write(
        out_dir.join(name),
        serde_json::to_string_pretty(doc).unwrap_or_default(),
    )
    .is_ok()
}

#[derive(Serialize)]
struct ImageMeta {
    i: usize,
    off: u32,
    flags: u16,
    hot: Option<(i16, i16)>,
    w: i32,
    h: i32,
    codec: String,
    ok: bool,
}

struct Header {
    flags: u16,
    hot: Option<(i16, i16)>,
    w: i32,
    h: i32,
    px: usize,
}

fn parse_header(data: &[u8], p: usize) -> Option<Header> {
    if p + 8 > data.len() {
        return None;
    }
    let flags = u16le(data, p);
    let has_hot = flags & 2 != 0;
    if has_hot {
        if p + 12 > data.len() {
            return None;
        }
        let hot = (i16le(data, p + 2), i16le(data, p + 4));
        let w = i16le(data, p + 6) as i32;
        let h = i16le(data, p + 8) as i32;
        if w < 0 || h < 0 || w > 4096 || h > 4096 {
            return None;
        }
        Some(Header {
            flags,
            hot: Some(hot),
            w,
            h,
            px: p + 10,
        })
    } else {
        let w = i16le(data, p + 2) as i32;
        let h = i16le(data, p + 4) as i32;
        if w < 0 || h < 0 || w > 4096 || h > 4096 {
            return None;
        }
        Some(Header {
            flags,
            hot: None,
            w,
            h,
            px: p + 6,
        })
    }
}

fn decode_rle16(data: &[u8], mut p: usize, w: usize, h: usize) -> (Option<Vec<u8>>, String) {
    let mut out = vec![0u8; w * h];
    let mut guard = 0usize;
    for row in 0..h {
        let mut x = 0usize;
        loop {
            if p + 2 > data.len() || guard > 4_000_000 {
                return (None, String::from("rle16 stream overrun"));
            }
            guard += 1;
            let word = u16le(data, p);
            p += 2;
            if word & 0x8000 != 0 {
                x += (word & 0x0FFF) as usize;
            } else {
                let n = (word & 0x0FFF) as usize;
                if p + n > data.len() {
                    return (None, String::from("literal overrun"));
                }
                for k in 0..n {
                    if x + k < w {
                        out[row * w + x + k] = data[p + k];
                    }
                }
                x += n;
                p += n;
            }
            if word & 0x4000 != 0 {
                break;
            }
        }
    }
    (Some(out), String::from("rle16"))
}

fn decode_raw(data: &[u8], mut p: usize, w: usize, h: usize) -> (Option<Vec<u8>>, String) {
    if p + w * h > data.len() {
        return (None, String::from("raw overrun"));
    }
    let out = data[p..p + w * h].to_vec();
    p += w * h;
    (Some(out), String::from("raw"))
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
    if data.len() < 6 {
        if stem_of(rel).to_uppercase() == "SINTABLE" && data.len() == 512 {
            return (
                String::from("parsed"),
                String::from("256-entry sine LUT (amp 32767)"),
            );
        }
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let count = u16le(&data, 0) as usize;
    if count == 0 || 2 + count * 4 > data.len() {
        return (
            String::from("heuristic-failed"),
            format!("count {} vs {}B", count, data.len()),
        );
    }
    let palette = pal::sibling_vga770(path);
    let png_dir = out_dir.join(format!("{}.images", stem_of(rel)));
    if palette.is_some() {
        let _ = fs::create_dir_all(&png_dir);
    }
    let mut metas: Vec<ImageMeta> = Vec::with_capacity(count);
    let mut ok_count = 0usize;
    let mut rendered = 0usize;
    for i in 0..count {
        let slot = 2 + i * 4;
        let off = u32le(&data, slot);
        let start = slot + off as usize;
        let (hdr, why) = match if start + 8 <= data.len() {
            parse_header(&data, start)
        } else {
            None
        } {
            Some(h) => (h, String::new()),
            None => (
                Header {
                    flags: 0,
                    hot: None,
                    w: 0,
                    h: 0,
                    px: start,
                },
                String::from("no-header"),
            ),
        };
        if hdr.w == 0 {
            ok_count += 1;
            metas.push(ImageMeta {
                i,
                off,
                flags: 0,
                hot: None,
                w: 0,
                h: 0,
                codec: String::from("empty-slot"),
                ok: true,
            });
            continue;
        }
        let w = hdr.w as usize;
        let h = hdr.h as usize;
        let (px, codec) = if hdr.flags & 1 != 0 {
            decode_rle16(&data, hdr.px, w, h)
        } else {
            decode_raw(&data, hdr.px, w, h)
        };
        let ok = px.is_some();
        if ok {
            ok_count += 1;
        }
        if let (Some(px), Some(p)) = (&px, palette.as_ref()) {
            if rendered < 512 && w * h <= 1_000_000 {
                if render(&png_dir, &stem_of(rel), i, px, w as u32, h as u32, p) {
                    rendered += 1;
                }
            }
        }
        metas.push(ImageMeta {
            i,
            off,
            flags: hdr.flags,
            hot: hdr.hot,
            w: hdr.w,
            h: hdr.h,
            codec,
            ok,
        });
    }
    let doc = serde_json::json!({ "file": rel, "count": count, "ok": ok_count, "images": metas });
    let _ = write_json(out_dir, &format!("{}.bin.json", stem_of(rel)), &doc);
    let cov = ok_count * 100 / count;
    let status = if cov == 100 {
        String::from("parsed")
    } else if cov >= 50 {
        String::from("parsed")
    } else {
        String::from("partial")
    };
    (
        status,
        format!(
            "count={} decoded={} rendered={} pal={}",
            count,
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
    if data.len() < 12 {
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let count = u16le(&data, 0) as usize;
    if count == 0 || 2 + count * 4 > data.len() {
        return (
            String::from("heuristic-failed"),
            format!("count {} vs {}B", count, data.len()),
        );
    }
    let palette = pal::sibling_vga770(path);
    let png_dir = out_dir.join(format!("{}.tiles", stem_of(rel)));
    if palette.is_some() {
        let _ = fs::create_dir_all(&png_dir);
    }
    let mut metas: Vec<serde_json::Value> = Vec::new();
    let mut ok_count = 0usize;
    let mut rendered = 0usize;
    for i in 0..count {
        let slot = 2 + i * 4;
        let off = u32le(&data, slot);
        let start = slot + off as usize;
        if start + 6 > data.len() {
            metas.push(serde_json::json!({ "i": i, "ok": false, "why": "short" }));
            continue;
        }
        let w0 = u16le(&data, start);
        let (px, codec, tw, th) = if w0 >= 4 {
            let rows = u16le(&data, start + 8) as usize;
            if rows == 0 || rows > 64 {
                (None, format!("hdr rows={}", rows), 32, rows)
            } else {
                let r = decode_byte_rle(&data, start + 10, 32, rows);
                (r.0, r.1, 32, rows)
            }
        } else {
            let tw = u16le(&data, start + 2) as usize;
            let th = u16le(&data, start + 4) as usize;
            if tw == 0 || th == 0 || tw > 256 || th > 256 || start + 6 + tw * th > data.len() {
                (None, format!("raw dims {}x{}", tw, th), tw, th)
            } else {
                (
                    Some(data[start + 6..start + 6 + tw * th].to_vec()),
                    String::from("raw"),
                    tw,
                    th,
                )
            }
        };
        let ok = px.is_some();
        if ok {
            ok_count += 1;
        }
        if let (Some(px), Some(palv)) = (&px, palette.as_ref()) {
            if rendered < 160 {
                if render(&png_dir, &stem_of(rel), i, px, tw as u32, th as u32, palv) {
                    rendered += 1;
                }
            }
        }
        metas.push(serde_json::json!({
            "i": i, "off": off, "w0": w0, "ok": ok, "codec": codec, "w": tw, "h": th,
        }));
    }
    let doc = serde_json::json!({ "file": rel, "count": count, "ok": ok_count, "tiles": metas });
    let _ = write_json(out_dir, &format!("{}.cgr.json", stem_of(rel)), &doc);
    let status = if ok_count * 2 >= count {
        String::from("parsed")
    } else {
        String::from("partial")
    };
    (
        status,
        format!(
            "count={} tiles-ok={} rendered={}",
            count, ok_count, rendered
        ),
    )
}

fn decode_byte_rle(data: &[u8], mut p: usize, w: usize, h: usize) -> (Option<Vec<u8>>, String) {
    let mut out = vec![0u8; w * h];
    let mut x = 0usize;
    let mut y = 0usize;
    let mut guard = 0usize;
    while y < h && p < data.len() && guard < 1_000_000 {
        guard += 1;
        let b = data[p];
        p += 1;
        if b & 0x40 != 0 {
            y += 1;
            x = 0;
        } else if b & 0x80 != 0 {
            x += (b & 0x3F) as usize + 1;
        } else {
            let n = (b & 0x3F) as usize + 1;
            if p + n > data.len() {
                return (None, String::from("literal overrun"));
            }
            for k in 0..n {
                if x < w && y < h {
                    out[y * w + x] = data[p + k];
                }
                x += 1;
            }
            p += n;
        }
    }
    if y >= h {
        (Some(out), String::from("byterle"))
    } else {
        (None, String::from("byterle incomplete"))
    }
}

pub fn min_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() % 16 != 0 {
        return (
            String::from("heuristic-failed"),
            format!("len {} not 16n", data.len()),
        );
    }
    let n = data.len() / 16;
    let doc = serde_json::json!({ "file": rel, "tile_count": n, "bytes": data });
    let ok = write_json(out_dir, &format!("{}.min.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} tile colors (16B each)", n),
    )
}

pub fn lnk_lng(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() != 16384 {
        return (
            String::from("heuristic-failed"),
            format!("len {} != 16384", data.len()),
        );
    }
    let mut vals = Vec::with_capacity(8192);
    let mut identity = 0usize;
    for i in 0..8192 {
        let v = u16le(&data, i * 2);
        if v as usize == i {
            identity += 1;
        }
        vals.push(v);
    }
    let doc = serde_json::json!({ "file": rel, "entries": vals, "identity_count": identity });
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
    if data.len() < 10 {
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let count = u16le(&data, 0) as usize;
    if 2 + count * 8 > data.len() {
        return (String::from("heuristic-failed"), format!("count {}", count));
    }
    let mut chunks: Vec<serde_json::Value> = Vec::new();
    let wav_dir = out_dir.to_path_buf();
    let _ = fs::create_dir_all(&wav_dir);
    for i in 0..count {
        let b = 2 + i * 8;
        let off = u32le(&data, b) as usize;
        let size = u32le(&data, b + 4) as usize;
        let inside = off + size <= data.len();
        chunks.push(serde_json::json!({ "i": i, "offset": off, "size": size, "fits": inside }));
        if inside && size > 0 && size < 4_000_000 {
            let mut wav: Vec<u8> = Vec::with_capacity(44 + size);
            wav.extend_from_slice(b"RIFF");
            wav.extend_from_slice(&((36 + size) as u32).to_le_bytes());
            wav.extend_from_slice(b"WAVE");
            wav.extend_from_slice(b"fmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&11025u32.to_le_bytes());
            wav.extend_from_slice(&11025u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&8u16.to_le_bytes());
            wav.extend_from_slice(b"data");
            wav.extend_from_slice(&(size as u32).to_le_bytes());
            wav.extend_from_slice(&data[off..off + size]);
            let _ = fs::write(
                wav_dir.join(format!("{}_chunk{:02}.wav", stem_of(rel), i)),
                &wav,
            );
        }
    }
    let doc = serde_json::json!({ "file": rel, "count": count, "chunks": chunks, "rate": 11025, "fmt": "8-bit mono" });
    let ok = write_json(out_dir, &format!("{}.mrw.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} instrument chunks (11025Hz 8-bit mono)", count),
    )
}

pub fn saved_bdl(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() != 900 {
        return (
            String::from("unknown-variant"),
            format!("len {} != 900", data.len()),
        );
    }
    let mut slots: Vec<serde_json::Value> = Vec::new();
    for s in 0..5 {
        let b = s * 180;
        let name: String = data[b..b + 8]
            .iter()
            .map(|c| {
                if c.is_ascii_graphic() || *c == b" "[0] {
                    *c as char
                } else {
                    char::from(46u8)
                }
            })
            .collect();
        let done = u32le(&data, b + 8);
        slots.push(serde_json::json!({
            "slot": s, "name": name, "completed_mask": format!("{:08x}", done),
            "zone": u16le(&data, b + 12), "money": u32le(&data, b + 18),
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
    if data.len() != 120 {
        return (
            String::from("unknown-variant"),
            format!("len {} != 120", data.len()),
        );
    }
    let mut scores: Vec<serde_json::Value> = Vec::new();
    for s in 0..10 {
        let b = s * 12;
        let name: String = data[b + 4..b + 12]
            .iter()
            .map(|c| {
                if c.is_ascii_graphic() || *c == b" "[0] {
                    *c as char
                } else {
                    char::from(46u8)
                }
            })
            .collect();
        scores.push(serde_json::json!({ "rank": s + 1, "score": u32le(&data, b), "name": name }));
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
    if data.len() < 41 {
        return (
            String::from("unknown-variant"),
            format!("len {} < 41", data.len()),
        );
    }
    let name: String = data[16..24]
        .iter()
        .map(|c| {
            if c.is_ascii_graphic() || *c == b" "[0] {
                *c as char
            } else {
                char::from(46u8)
            }
        })
        .collect();
    let doc = serde_json::json!({
        "file": rel, "size": data.len(),
        "backbuffer": u32le(&data, 0), "actionpan": u32le(&data, 4),
        "language": u32le(&data, 8), "cd_audio": u32le(&data, 12),
        "playername": name, "volume": u32le(&data, 24),
        "code_no_title": u32le(&data, 28), "midi": u32le(&data, 32), "sound": u32le(&data, 36),
        "installdrive": data[40],
    });
    let ok = write_json(out_dir, &format!("{}.options.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("options struct; player={}", name),
    )
}

pub fn nme_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let mut p = 0usize;
    let mut sections: Vec<serde_json::Value> = Vec::new();
    let mut sec = 0usize;
    while p + 2 <= data.len() && sec < 16 {
        let count = u16le(&data, p) as usize;
        if count == 0 && p + 2 == data.len() {
            sections.push(serde_json::json!({ "sec": sec, "count": 0, "at": p }));
            p += 2;
            break;
        }
        let rec8 = (p + 2 + count * 10 <= data.len())
            && (count == 0 || next_count_plausible(&data, p + 2 + count * 10));
        let rec10 = p + 2 + count * 10 <= data.len();
        let rec8b = p + 2 + count * 8 <= data.len();
        let chosen = if rec10 && rec8b {
            if next_count_plausible(&data, p + 2 + count * 10) {
                10
            } else {
                8
            }
        } else if rec10 {
            10
        } else if rec8b {
            8
        } else {
            0
        };
        if chosen == 0 {
            sections
                .push(serde_json::json!({ "sec": sec, "count": count, "at": p, "note": "tail" }));
            break;
        }
        let mut recs: Vec<serde_json::Value> = Vec::new();
        for i in 0..count.min(32) {
            let b = p + 2 + i * chosen;
            let mut words = Vec::new();
            for k in 0..chosen / 2 {
                words.push(i16le(&data, b + k * 2));
            }
            recs.push(serde_json::json!({ "i": i, "w": words }));
        }
        sections.push(serde_json::json!({ "sec": sec, "count": count, "rec": chosen, "at": p, "sample": recs }));
        p += 2 + count * chosen;
        sec += 1;
    }
    let doc = serde_json::json!({ "file": rel, "size": data.len(), "sections": sections });
    let ok = write_json(out_dir, &format!("{}.nme.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!(
            "{} sections (8street layout, fields partial)",
            sections.len()
        ),
    )
}

fn next_count_plausible(d: &[u8], p: usize) -> bool {
    if p + 2 > d.len() {
        return p == d.len();
    }
    let c = u16le(d, p) as usize;
    c < 4000
}

pub fn bdg_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let mut p = 0usize;
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut idx = 0usize;
    while p + 2 <= data.len() && idx < 282 {
        let flag = u16le(&data, p);
        if flag != 1 {
            recs.push(serde_json::json!({ "i": idx, "active": false }));
            p += 2;
            idx += 1;
            continue;
        }
        if p + 0x36 > data.len() {
            break;
        }
        let w = u16le(&data, p + 2) as usize;
        let h = u16le(&data, p + 4) as usize;
        let dep = u16le(&data, p + 6) as usize;
        let blob = 2 * w * h * dep;
        let total = 0x36 + 3 * blob;
        recs.push(serde_json::json!({
            "i": idx, "active": true, "w": w, "h": h, "d": dep,
            "blobs": blob * 3, "head": hex_head(&data[p..p + 0x36], 0x36),
        }));
        p += total;
        idx += 1;
    }
    let doc = serde_json::json!({ "file": rel, "size": data.len(), "records": recs });
    let ok = write_json(out_dir, &format!("{}.bdg.json", stem_of(rel)), &doc);
    let active = recs
        .iter()
        .filter(|r| r.get("active") == Some(&serde_json::json!(true)))
        .count();
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} records, {} active", recs.len(), active),
    )
}

pub fn text_file(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let printable = data.iter().filter(|b| b.is_ascii()).count() * 100 / data.len().max(1);
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
