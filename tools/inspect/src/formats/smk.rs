use crate::stem_of;
use std::fs;
use std::path::Path;

fn u32at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn i32at(d: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() < 104 || (&data[0..4] != b"SMK2" && &data[0..4] != b"SMK4") {
        return (
            String::from("unknown-variant"),
            format!("head: {}", crate::hex_head(&data, 8)),
        );
    }
    let audio_sizes: Vec<u32> = (0..7).map(|i| u32at(&data, 24 + i * 4)).collect();
    let tree_sizes: Vec<u32> = vec![
        u32at(&data, 52),
        u32at(&data, 56),
        u32at(&data, 60),
        u32at(&data, 64),
    ];
    let audio_rates: Vec<u32> = (0..7).map(|i| u32at(&data, 68 + i * 4)).collect();
    let ms_raw = i32at(&data, 16);
    let fps_desc = if ms_raw > 0 {
        format!("{} fps", 1000 / ms_raw.max(1))
    } else {
        let us = (-ms_raw as i64) * 10;
        format!("{} fps (us-per-frame encoding: {}us)", 1000000 / us, us)
    };
    let doc = serde_json::json!({
        "file": rel,
        "magic": String::from_utf8_lossy(&data[0..4]).to_string(),
        "width": u32at(&data, 4),
        "height": u32at(&data, 8),
        "frames": u32at(&data, 12),
        "ms_per_frame_raw": ms_raw,
        "fps_desc": fps_desc,
        "flags": u32at(&data, 20),
        "audio_sizes": audio_sizes,
        "tree_sizes": tree_sizes,
        "audio_rates": audio_rates,
        "filesize": data.len(),
    });
    let _ = fs::create_dir_all(out_dir);
    let p = out_dir.join(format!("{}.smk.json", stem_of(rel)));
    match fs::write(&p, serde_json::to_string_pretty(&doc).unwrap()) {
        Ok(_) => (
            String::from("parsed"),
            format!(
                "{}x{} frames={} {}",
                u32at(&data, 4),
                u32at(&data, 8),
                u32at(&data, 12),
                fps_desc
            ),
        ),
        Err(e) => (String::from("error"), format!("write failed: {}", e)),
    }
}
