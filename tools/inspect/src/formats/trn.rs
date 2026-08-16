use crate::stem_of;
use std::fs;
use std::path::Path;

pub fn dump(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {}", e)),
    };
    if data.len() == 256 {
        let vals: Vec<u16> = data.iter().map(|b| *b as u16).collect();
        let _ = fs::create_dir_all(out_dir);
        let doc = serde_json::json!({ "file": rel, "size": data.len(), "values": vals });
        let p = out_dir.join(format!("{}.trn.json", stem_of(rel)));
        match fs::write(&p, serde_json::to_string_pretty(&doc).unwrap()) {
            Ok(_) => {
                let perm = data.iter().filter(|v| **v != data[0]).count();
                (
                    String::from("parsed"),
                    format!("256B remap LUT, non-uniform entries: {}", perm),
                )
            }
            Err(e) => (String::from("error"), format!("write failed: {}", e)),
        }
    } else {
        (format!("unknown-variant-{}B", data.len()), String::new())
    }
}
