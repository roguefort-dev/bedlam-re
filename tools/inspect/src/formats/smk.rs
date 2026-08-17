use crate::stem_of;
use bedlam_assets as assets;
use std::fs;
use std::path::Path;

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let h = match assets::smk::parse_smk_header(&data) {
        Ok(h) => h,
        Err(_) => {
            return (
                String::from("unknown-variant"),
                format!("head: {}", assets::hex_head(&data, 8)),
            )
        }
    };
    let doc = serde_json::json!({
        "file": rel,
        "magic": h.magic,
        "width": h.width,
        "height": h.height,
        "frames": h.frames,
        "ms_per_frame_raw": h.ms_per_frame_raw,
        "fps_desc": h.fps_desc(),
        "flags": h.flags,
        "audio_sizes": h.audio_sizes,
        "tree_sizes": h.tree_sizes,
        "audio_rates": h.audio_rates,
        "filesize": data.len(),
    });
    let _ = fs::create_dir_all(out_dir);
    let p = out_dir.join(format!("{}.smk.json", stem_of(rel)));
    match fs::write(&p, serde_json::to_string_pretty(&doc).unwrap()) {
        Ok(_) => (
            String::from("parsed"),
            format!(
                "{}x{} frames={} {}",
                h.width,
                h.height,
                h.frames,
                h.fps_desc()
            ),
        ),
        Err(e) => (String::from("error"), format!("write failed: {}", e)),
    }
}
