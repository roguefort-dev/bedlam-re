use crate::{hex_head, stem_of};
use std::fs;
use std::path::Path;

pub fn parse_vga770(data: &[u8]) -> Option<[[u8; 3]; 256]> {
    if data.len() < 770 {
        return None;
    }
    let mut p = [[0u8; 3]; 256];
    for i in 0..256 {
        for c in 0..3 {
            let v6 = data[2 + i * 3 + c] & 0x3F;
            p[i][c] = (v6 << 2) | (v6 >> 4);
        }
    }
    Some(p)
}

pub fn sibling_vga770(path: &Path) -> Option<[[u8; 3]; 256]> {
    let dir = path.parent()?;
    let mut cands: Vec<std::path::PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|e| e.eq_ignore_ascii_case("pal"))
                .unwrap_or(false)
        })
        .collect();
    cands.sort();
    for c in cands {
        if let Ok(d) = fs::read(&c) {
            if d.len() == 770 {
                return parse_vga770(&d);
            }
        }
    }
    None
}

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() == 770 {
        let pal = parse_vga770(&data).unwrap();
        let mut img = image::RgbImage::new(256, 256);
        for idx in 0..256usize {
            let cx = ((idx % 16) * 16) as u32;
            let cy = ((idx / 16) * 16) as u32;
            let rgb = image::Rgb([pal[idx][0], pal[idx][1], pal[idx][2]]);
            for y in 0..16u32 {
                for x in 0..16u32 {
                    img.put_pixel(cx + x, cy + y, rgb);
                }
            }
        }
        let _ = fs::create_dir_all(out_dir);
        let png = out_dir.join(format!("{}.swatch.png", stem_of(rel)));
        match img.save(&png) {
            Ok(_) => (
                String::from("parsed"),
                String::from("770B vga palette (6-bit expanded), swatch written"),
            ),
            Err(e) => (
                String::from("parsed"),
                format!("770B palette, png save failed: {}", e),
            ),
        }
    } else {
        (
            format!("unknown-variant-{}B", data.len()),
            format!("head: {}", hex_head(&data, 16)),
        )
    }
}
