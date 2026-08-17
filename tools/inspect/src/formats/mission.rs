//! CLI dump shims for mission-layer formats (parsing lives in bedlam-assets).

use crate::formats::pal;
use crate::stem_of;
use bedlam_assets as assets;
use bedlam_assets::AssetsError;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Same field order as the legacy PlaneStat so the emitted JSON is identical.
#[derive(Serialize)]
struct PlaneStat {
    plane: usize,
    min: u16,
    max: u16,
    uniq: usize,
    top: Vec<(u16, usize)>,
}

fn cli_stats(g: &assets::mission::Grid) -> Vec<PlaneStat> {
    g.plane_stats()
        .into_iter()
        .map(|s| PlaneStat {
            plane: s.plane,
            min: s.min,
            max: s.max,
            uniq: s.uniq,
            top: s.top,
        })
        .collect()
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

fn grid_err(e: AssetsError) -> (String, String) {
    match e {
        AssetsError::TooSmall { len } => (String::from("heuristic-failed"), format!("{}B", len)),
        AssetsError::SizeFormula {
            len,
            expected,
            w,
            h,
        } => (
            String::from("heuristic-failed"),
            format!("size {} != formula {} (w={} h={})", len, expected, w, h),
        ),
        other => (String::from("heuristic-failed"), other.to_string()),
    }
}

pub fn grid16(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let g = match assets::mission::parse_grid16(&data) {
        Ok(g) => g,
        Err(e) => return grid_err(e),
    };
    let stats = cli_stats(&g);
    plane_png(
        path,
        out_dir,
        rel,
        g.w as u32,
        g.h as u32,
        &g.planes[0],
        "plane0",
    );
    let doc = serde_json::json!({
        "file": rel, "kind": "grid16", "w": g.w, "h": g.h, "planes": 8, "planes_stat": stats,
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
        format!(
            "{}x{} 8 u16 planes; uniq counts {}",
            g.w,
            g.h,
            nz.join(", ")
        ),
    )
}

pub fn grid8(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let g = match assets::mission::parse_grid8(&data) {
        Ok(g) => g,
        Err(e) => return grid_err(e),
    };
    let stats = cli_stats(&g);
    plane_png(
        path,
        out_dir,
        rel,
        g.w as u32,
        g.h as u32,
        &g.planes[0],
        "plane0",
    );
    let doc = serde_json::json!({
        "file": rel, "kind": "grid8", "w": g.w, "h": g.h, "planes": 8, "planes_stat": stats,
    });
    let ok = write_json(out_dir, &format!("{}.grid8.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}x{} 8 u8 planes", g.w, g.h),
    )
}

pub fn trt(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let t = match assets::mission::parse_trt(&data) {
        Ok(t) => t,
        Err(AssetsError::NotMultiple { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} not 2+12n", len),
            )
        }
        Err(AssetsError::CountMismatch { count }) => {
            return (
                String::from("heuristic-failed"),
                format!("count {} mismatch", count),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut types: BTreeMap<u16, usize> = BTreeMap::new();
    for (i, r) in t.records.iter().enumerate() {
        *types.entry(r.kind).or_insert(0) += 1;
        recs.push(serde_json::json!({
            "i": i, "x": r.x, "y": r.y, "type": r.kind,
            "rest": assets::hex_head(&r.rest, 6),
        }));
    }
    let doc =
        serde_json::json!({ "file": rel, "count": t.count, "records": recs, "type_counts": types });
    let ok = write_json(out_dir, &format!("{}.trt.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{} records (x,y,type); type counts {:?}", t.count, types),
    )
}

pub fn mrk(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let m = match assets::mission::parse_mrk(&data) {
        Ok(m) => m,
        Err(AssetsError::NotMultiple { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} not 16n", len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut types: BTreeMap<u16, usize> = BTreeMap::new();
    for (i, r) in m.records.iter().enumerate() {
        *types.entry(r.kind).or_insert(0) += 1;
        recs.push(serde_json::json!({
            "i": i, "flag": r.flag, "x": r.x, "y": r.y, "type": r.kind,
            "rest": assets::hex_head(&r.rest, 8),
        }));
    }
    let n = m.records.len();
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
    let p = match assets::mission::parse_pos(&data) {
        Ok(p) => p,
        Err(AssetsError::NotMultiple { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} not 16n", len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let n = p.slots.len();
    let mut used = 0usize;
    let mut sample: Vec<serde_json::Value> = Vec::new();
    for (i, s) in p.slots.iter().enumerate() {
        if s.empty {
            continue;
        }
        used += 1;
        if sample.len() < 16 {
            sample.push(serde_json::json!({
                "i": i,
                "u16x4": s.u16x4,
                "u16x4b": s.u16x4b,
                "head": assets::hex_head(&s.raw, 16),
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
    let p = match assets::mission::parse_pad(&data) {
        Ok(p) => p,
        Err(AssetsError::NotMultiple { len }) => {
            return (
                String::from("heuristic-failed"),
                format!("len {} not 6n", len),
            )
        }
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let mut recs: Vec<serde_json::Value> = Vec::new();
    let mut fill = 0usize;
    for (i, slot) in p.slots.iter().enumerate() {
        match slot {
            None => fill += 1,
            Some(r) => recs.push(serde_json::json!({
                "i": i, "x": r.x, "y": r.y, "type": r.kind,
            })),
        }
    }
    let n = p.slots.len();
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
    let p = assets::mission::parse_pth(&data);
    let doc = serde_json::json!({ "file": rel, "size": data.len(), "count": p.count, "head": assets::hex_head(&p.head, 16) });
    let ok = write_json(out_dir, &format!("{}.pth.json", stem_of(rel)), &doc);
    (
        if ok {
            String::from("parsed")
        } else {
            String::from("error")
        },
        format!("{}B count={}", data.len(), p.count),
    )
}
