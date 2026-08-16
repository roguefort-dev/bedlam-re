use crate::stem_of;
use std::fs;
use std::path::Path;

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    let rate: u32 = 11025;
    let mut wav: Vec<u8> = Vec::with_capacity(44 + data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&8u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
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
