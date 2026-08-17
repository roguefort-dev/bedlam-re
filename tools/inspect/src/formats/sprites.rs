use crate::formats::pal;
use crate::{hex_head, stem_of};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct EntryMeta {
    idx: usize,
    offset: usize,
    len: usize,
    w: u16,
    h: u16,
    fits_wh: bool,
    head: String,
}

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 6 {
        return (
            String::from("heuristic-failed"),
            format!("too small: {}B", data.len()),
        );
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let base = 2usize;
    if count == 0 {
        return (String::from("empty"), String::from("count=0"));
    }
    if data.len() < base + count * 4 {
        return (
            String::from("heuristic-failed"),
            format!("count {} overruns {}B file", count, data.len()),
        );
    }
    let mut offs: Vec<usize> = Vec::with_capacity(count);
    for i in 0..count {
        let b = base + i * 4;
        offs.push(u32::from_le_bytes([data[b], data[b + 1], data[b + 2], data[b + 3]]) as usize);
    }
    let palette = pal::sibling_vga770(path);
    let png_dir = out_dir.join(format!("{}.sprites", stem_of(rel)));
    if palette.is_some() {
        let _ = fs::create_dir_all(&png_dir);
    }
    let mut metas: Vec<EntryMeta> = Vec::with_capacity(count);
    let mut fits = 0usize;
    let mut rendered = 0usize;
    let max_png = 256usize;
    let max_px = 4_000_000usize;
    for (i, &off) in offs.iter().enumerate() {
        let start = base + off;
        let end = if i + 1 < offs.len() {
            base + offs[i + 1]
        } else {
            data.len()
        };
        let bad = start >= data.len() || end > data.len() || end < start;
        let (len, w, h, ok, head) = if bad {
            (0, 0, 0, false, String::new())
        } else {
            let l = end - start;
            let (w, h) = if l >= 4 {
                (
                    u16::from_le_bytes([data[start], data[start + 1]]),
                    u16::from_le_bytes([data[start + 2], data[start + 3]]),
                )
            } else {
                (0, 0)
            };
            let ok = l >= 4 && w > 0 && h > 0 && 4 + (w as usize) * (h as usize) == l;
            let hsend = std::cmp::min(end, start + 8);
            (l, w, h, ok, hex_head(&data[start..hsend], 8))
        };
        if ok {
            fits += 1;
        }
        if ok && palette.is_some() && rendered < max_png && (w as usize) * (h as usize) <= max_px {
            let p = palette.unwrap();
            let mut img = image::RgbImage::new(w as u32, h as u32);
            let mut k = start + 4;
            for y in 0..h as u32 {
                for x in 0..w as u32 {
                    let ci = data[k] as usize;
                    k += 1;
                    img.put_pixel(x, y, image::Rgb([p[ci][0], p[ci][1], p[ci][2]]));
                }
            }
            if img
                .save(png_dir.join(format!("{:04}_{}x{}.png", i, w, h)))
                .is_ok()
            {
                rendered += 1;
            }
        }
        metas.push(EntryMeta {
            idx: i,
            offset: off,
            len,
            w,
            h,
            fits_wh: ok,
            head,
        });
    }
    let meta_path = out_dir.join(format!("{}.sprites.json", stem_of(rel)));
    let _ = fs::write(
        &meta_path,
        serde_json::to_string_pretty(&metas).unwrap_or_default(),
    );
    let cov = (fits * 100) / count;
    let paldesc = if palette.is_some() { "sibling" } else { "none" };
    if fits == count {
        (
            String::from("parsed"),
            format!(
                "count={} wh-fit=100% rendered={} pal={}",
                count, rendered, paldesc
            ),
        )
    } else if fits > 0 {
        (
            String::from("partial"),
            format!(
                "count={} wh-fit={}% rendered={} pal={}",
                count, cov, rendered, paldesc
            ),
        )
    } else {
        (
            String::from("heuristic-failed"),
            format!("count={} no wh-fit; head: {}", count, hex_head(&data, 32)),
        )
    }
}

pub fn cgr_directory(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 6 {
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    if count == 0 || data.len() < 2 + count * 4 {
        return (
            String::from("heuristic-failed"),
            format!("count {} vs {}B", count, data.len()),
        );
    }
    let dir_end = 2 + count * 4;
    let mut offs: Vec<u32> = Vec::with_capacity(count);
    for i in 0..count {
        let b = 2 + i * 4;
        offs.push(u32::from_le_bytes([
            data[b],
            data[b + 1],
            data[b + 2],
            data[b + 3],
        ]));
    }
    let mut monotonic = true;
    for w in offs.windows(2) {
        if w[0] >= w[1] {
            monotonic = false;
        }
    }
    let candidates: [usize; 3] = [0, 2, dir_end];
    let mut chosen: Option<usize> = None;
    for base in candidates {
        let minv = offs[0] as usize + base;
        let maxv = offs[count - 1] as usize + base;
        if minv >= dir_end && maxv <= data.len() {
            chosen = Some(base);
            break;
        }
    }
    let base = match chosen {
        Some(b) => b,
        None => {
            return (
                String::from("heuristic-failed"),
                format!(
                    "no offset base fits (count {} first {} last {} len {})",
                    count,
                    offs[0],
                    offs[count - 1],
                    data.len()
                ),
            )
        }
    };
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for i in 0..count {
        let start = base + offs[i] as usize;
        let end = if i + 1 < count {
            base + offs[i + 1] as usize
        } else {
            data.len()
        };
        let len = end.saturating_sub(start);
        entries.push(serde_json::json!({
            "i": i, "off": offs[i], "len": len,
            "head": if start + 8 <= data.len() { hex_head(&data[start..start + 8], 8) } else { String::new() },
        }));
    }
    let doc = serde_json::json!({
        "file": rel, "count": count, "offset_base": base, "dir_end": dir_end,
        "monotonic": monotonic, "entries": entries,
    });
    let _ = fs::create_dir_all(out_dir);
    let ok = fs::write(
        out_dir.join(format!("{}.cgr.json", stem_of(rel))),
        serde_json::to_string_pretty(&doc).unwrap_or_default(),
    )
    .is_ok();
    (
        if ok {
            String::from("directory-parsed")
        } else {
            String::from("error")
        },
        format!(
            "count={} base={} monotonic={} (pixel codec open)",
            count, base, monotonic
        ),
    )
}
