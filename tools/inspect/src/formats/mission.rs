use crate::{hex_head, stem_of};
use crate::formats::pal;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct PlaneStat {
    plane: usize,
    min: u16,
    max: u16,
    uniq: usize,
    top: Vec<(u16, usize)>,
}

fn top_values(vals: &[u16], n: usize) -> Vec<(u16, usize)> {
    let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for v in vals {
        *counts.entry(*v).or_insert(0) += 1;
    }
    let mut list: Vec<(u16, usize)> = counts.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.truncate(n);
    list
}

fn write_json(out_dir: &Path, name: &str, doc: &serde_json::Value) -> bool {
    let _ = fs::create_dir_all(out_dir);
    fs::write(
        out_dir.join(name),
        serde_json::to_string_pretty(doc).unwrap_or_default(),
    )
    .is_ok()
}

fn plane_png(path: &Path, out_dir: &Path, rel: &str, w: u32, h: u32, plane: &[u16], tag: &str) {
    if let Some(p) = pal::sibling_vga770(path) {
        let mut img = image::RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let ci = (plane[(y * w + x) as usize] & 0xFF) as usize;
                img.put_pixel(x, y, image::Rgb([p[ci][0], p[ci][1], p[ci][2]]));
            }
        }
        let _ = fs::create_dir_all(out_dir);
        let _ = img.save(out_dir.join(format!("{}.{}.png", stem_of(rel), tag)));
    }
}

pub fn grid16(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 4 {
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let w = u16::from_le_bytes([data[0], data[1]]) as usize;
    let h = u16::from_le_bytes([data[2], data[3]]) as usize;
    let need = 4 + w * h * 16;
    if data.len() != need {
        return (
            String::from("heuristic-failed"),
            format!("size {} != formula {} (w={} h={})", data.len(), need, w, h),
        );
    }
    let mut stats: Vec<PlaneStat> = Vec::new();
    let mut plane0: Vec<u16> = Vec::with_capacity(w * h);
    for p in 0..8 {
        let mut plane = Vec::with_capacity(w * h);
        for i in 0..w * h {
            let o = 4 + p * w * h * 2 + i * 2;
            plane.push(u16::from_le_bytes([data[o], data[o + 1]]));
        }
        let stat = PlaneStat {
            plane: p,
            min: *plane.iter().min().unwrap(),
            max: *plane.iter().max().unwrap(),
            uniq: plane
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            top: top_values(&plane, 6),
        };
        if p == 0 {
            plane0 = plane.clone();
        }
        stats.push(stat);
    }
    plane_png(path, out_dir, rel, w as u32, h as u32, &plane0, "plane0");
    let doc = serde_json::json!({
        "file": rel, "kind": "grid16", "w": w, "h": h, "planes": 8, "planes_stat": stats,
    });
    let ok = write_json(out_dir, &format!("{}.grid16.json", stem_of(rel)), &doc);
    let nz: Vec<String> = stats
        .iter()
        .map(|s| format!("p{}:u{}", s.plane, s.uniq))
        .collect();
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}x{} 8 u16 planes; uniq counts {}", w, h, nz.join(", ")),
    )
}

pub fn grid8(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 4 {
        return (String::from("heuristic-failed"), format!("{}B", data.len()));
    }
    let w = u16::from_le_bytes([data[0], data[1]]) as usize;
    let h = u16::from_le_bytes([data[2], data[3]]) as usize;
    let need = 4 + w * h * 8;
    if data.len() != need {
        return (
            String::from("heuristic-failed"),
            format!("size {} != formula {} (w={} h={})", data.len(), need, w, h),
        );
    }
    let mut stats: Vec<PlaneStat> = Vec::new();
    let mut plane0: Vec<u16> = Vec::new();
    for p in 0..8 {
        let mut plane = Vec::with_capacity(w * h);
        for i in 0..w * h {
            plane.push(data[4 + p * w * h + i] as u16);
        }
        let stat = PlaneStat {
            plane: p,
            min: *plane.iter().min().unwrap(),
            max: *plane.iter().max().unwrap(),
            uniq: plane
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            top: top_values(&plane, 6),
        };
        if p == 0 {
            plane0 = plane.clone();
        }
        stats.push(stat);
    }
    plane_png(path, out_dir, rel, w as u32, h as u32, &plane0, "plane0");
    let doc = serde_json::json!({
        "file": rel, "kind": "grid8", "w": w, "h": h, "planes": 8, "planes_stat": stats,
    });
    let ok = write_json(out_dir, &format!("{}.grid8.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}x{} 8 u8 planes", w, h),
    )
}

fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

pub fn trt(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 2 || (data.len() - 2) % 12 != 0 {
        return (
            String::from("heuristic-failed"),
            format!("len {} not 2+12n", data.len()),
        );
    }
    let n = u16le(&data, 0) as usize;
    if n * 12 + 2 != data.len() {
        return (
            String::from("heuristic-failed"),
            format!("count {} mismatch", n),
        );
    }
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut types: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for i in 0..n {
        let b = 2 + i * 12;
        let x = u16le(&data, b);
        let y = u16le(&data, b + 2);
        let t = u16le(&data, b + 4);
        *types.entry(t).or_insert(0) += 1;
        recs.push(serde_json::json!({
            "i": i, "x": x, "y": y, "type": t,
            "rest": hex_head(&data[b + 6..b + 12], 6),
        }));
    }
    let doc = serde_json::json!({ "file": rel, "count": n, "records": recs, "type_counts": types });
    let ok = write_json(out_dir, &format!("{}.trt.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} records (x,y,type); type counts {:?}", n, types),
    )
}

pub fn mrk(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
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
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut types: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for i in 0..n {
        let b = i * 16;
        let flag = u16le(&data, b);
        let x = u16le(&data, b + 2);
        let y = u16le(&data, b + 4);
        let t = u16le(&data, b + 6);
        *types.entry(t).or_insert(0) += 1;
        recs.push(serde_json::json!({
            "i": i, "flag": flag, "x": x, "y": y, "type": t,
            "rest": hex_head(&data[b + 8..b + 16], 8),
        }));
    }
    let doc = serde_json::json!({ "file": rel, "count": n, "records": recs, "type_counts": types });
    let ok = write_json(out_dir, &format!("{}.mrk.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} x 16B slots; type counts {:?}", n, types),
    )
}

pub fn pos(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
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
    let mut used = 0usize;
    let mut sample: Vec<serde_json::Value> = Vec::new();
    for i in 0..n {
        let b = i * 16;
        let empty = data[b..b + 16].iter().all(|x| *x == 0xFF);
        if empty {
            continue;
        }
        used += 1;
        if sample.len() < 16 {
            sample.push(serde_json::json!({
                "i": i,
                "u16x4": [u16le(&data, b), u16le(&data, b+2), u16le(&data, b+4), u16le(&data, b+6)],
                "u16x4b": [u16le(&data, b+8), u16le(&data, b+10), u16le(&data, b+12), u16le(&data, b+14)],
                "head": hex_head(&data[b..b + 16], 16),
            }));
        }
    }
    let doc = serde_json::json!({ "file": rel, "slots": n, "used": used, "empty_marker": "all-FF", "sample": sample });
    let ok = write_json(out_dir, &format!("{}.pos.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}/{} slots used (empty = all-FF)", used, n),
    )
}

pub fn pad(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() % 6 != 0 {
        return (
            String::from("heuristic-failed"),
            format!("len {} not 6n", data.len()),
        );
    }
    let n = data.len() / 6;
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut fill = 0usize;
    for i in 0..n {
        let b = i * 6;
        let is_fill = data[b..b + 6].iter().all(|x| *x == 0xFF);
        if is_fill {
            fill += 1;
            continue;
        }
        recs.push(serde_json::json!({
            "i": i, "x": u16le(&data, b), "y": u16le(&data, b+2), "type": u16le(&data, b+4),
        }));
    }
    let doc = serde_json::json!({ "file": rel, "slots": n, "records": recs, "fill_slots": fill });
    let ok = write_json(out_dir, &format!("{}.pad.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}/{} records (rest 0xFF fill)", recs.len(), n),
    )
}

pub fn pth(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let count = if data.len() >= 2 { u16le(&data, 0) } else { 0 };
    let doc = serde_json::json!({ "file": rel, "size": data.len(), "count": count, "head": hex_head(&data, 16) });
    let ok = write_json(out_dir, &format!("{}.pth.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}B count={}", data.len(), count),
    )
}
