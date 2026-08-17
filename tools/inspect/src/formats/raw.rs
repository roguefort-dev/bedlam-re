use crate::stem_of;
use bedlam_assets as assets;
use std::fs;
use std::path::Path;

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let rate: u32 = 11025;
    let wav = assets::audio::wav_wrap(&data, rate);
    let _ = fs::create_dir_all(out_dir);
    let p = out_dir.join(format!("{}.wav", stem_of(rel)));
    match fs::write(&p, &wav) {
        Ok(_) => (
            String::from("parsed"),
            format!(
                "{}B pcm8 -> wav {}Hz mono (rate unverified, pending EXD HMI init check)",
                data.len(),
                rate
            ),
        ),
        Err(e) => (String::from("error"), format!("write failed: {}", e)),
    }
}
