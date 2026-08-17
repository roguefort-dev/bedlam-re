mod formats;

use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
pub struct FileReport {
    pub path: String,
    pub kind: String,
    pub status: String,
    pub detail: String,
}

pub fn stem_of(rel: &str) -> String {
    let base = rel.rsplit("/").next().unwrap_or(rel);
    match base.rfind(".") {
        Some(i) => base[..i].to_string(),
        None => base.to_string(),
    }
}

pub fn parent_dir_of(rel: &str) -> String {
    match rel.rfind("/") {
        Some(i) => rel[..i].to_string(),
        None => String::from("."),
    }
}

pub fn hex_head(data: &[u8], n: usize) -> String {
    data.iter()
        .take(n)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let root = PathBuf::from(if args.len() > 1 {
        args[1].clone()
    } else {
        String::from(".")
    });
    let out = PathBuf::from(if args.len() > 2 {
        args[2].clone()
    } else {
        String::from("derived")
    });
    fs::create_dir_all(&out).expect("create out dir");
    let mut reports: Vec<FileReport> = Vec::new();
    walk(&root, &root, &out, &mut reports);
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for r in &reports {
        *by.entry(format!("{}:{}", r.kind, r.status)).or_insert(0) += 1;
    }
    let doc = serde_json::json!({
        "root": root.to_string_lossy(),
        "total": reports.len(),
        "breakdown": by,
        "files": reports,
    });
    fs::write(
        out.join("summary.json"),
        serde_json::to_string_pretty(&doc).unwrap(),
    )
    .unwrap();
    println!(
        "inspect v0: {} files -> {}",
        reports.len(),
        out.join("summary.json").display()
    );
}

fn dispatch(
    ext: &str,
    stem: &str,
    p: &Path,
    dirout: &Path,
    rel: &str,
) -> (&'static str, (String, String)) {
    match ext {
        "pal" => ("pal", formats::pal::dump(p, dirout, rel)),
        "trn" => ("trn", formats::trn::dump(p, dirout, rel)),
        "bin" => ("bin", formats::binfile::bin_images(p, dirout, rel)),
        "cgr" => ("cgr", formats::binfile::cgr_tiles(p, dirout, rel)),
        "min" => ("min", formats::binfile::min_file(p, dirout, rel)),
        "lnk" | "lng" => ("lnk", formats::binfile::lnk_lng(p, dirout, rel)),
        "mrw" => ("mrw", formats::binfile::mrw(p, dirout, rel)),
        "map" | "tot" | "col" => ("grid16", formats::mission::grid16(p, dirout, rel)),
        "dat" => ("grid8", formats::mission::grid8(p, dirout, rel)),
        "trt" => ("trt", formats::mission::trt(p, dirout, rel)),
        "mrk" => ("mrk", formats::mission::mrk(p, dirout, rel)),
        "pos" => ("pos", formats::mission::pos(p, dirout, rel)),
        "pad" => ("pad", formats::mission::pad(p, dirout, rel)),
        "pth" => ("pth", formats::mission::pth(p, dirout, rel)),
        "raw" => ("pcm8", formats::raw::dump(p, dirout, rel)),
        "smk" => ("smk", formats::smk::dump(p, dirout, rel)),
        "bdl" => match stem {
            "saved" => ("bdl", formats::binfile::saved_bdl(p, dirout, rel)),
            "hiscore" => ("bdl", formats::binfile::hiscore_bdl(p, dirout, rel)),
            "options" => ("bdl", formats::binfile::options_bdl(p, dirout, rel)),
            _ => (
                "bdl",
                (
                    String::from("unknown-variant"),
                    String::from("CONFIG.BDL layout open"),
                ),
            ),
        },
        "nme" => ("nme", formats::binfile::nme_file(p, dirout, rel)),
        "bdg" => ("bdg", formats::binfile::bdg_file(p, dirout, rel)),
        "txt" | "rst" => ("text", formats::binfile::text_file(p, dirout, rel)),
        "eng" | "fre" | "ger" | "itl" | "spa" | "dch" => {
            let secs = fs::read_to_string(p)
                .map(|t| t.matches("[").count())
                .unwrap_or(0);
            (
                "language",
                (
                    String::from("parsed"),
                    format!("{} text sections (INI-style DB)", secs),
                ),
            )
        }
        "mrs" => (
            "mrs",
            (
                String::from("partial"),
                String::from("music score/sequencer data; event encoding open (EXW RE)"),
            ),
        ),
        "wav" => (
            "audio-cdda",
            (
                String::from("parsed"),
                String::from("44.1kHz stereo PCM track (CDDA)"),
            ),
        ),
        "exe" | "exd" | "exw" | "dll" | "386" | "ico" | "inf" | "log" => (
            "runtime",
            (
                String::from("classified"),
                String::from("program/runtime file, not data"),
            ),
        ),
        "bld" | "ctg" => (
            "editor-unknown",
            (
                String::from("no-loader"),
                String::from("editor file; no runtime loader found (8street negative)"),
            ),
        ),
        _ => ("pending", (String::from("queued"), String::new())),
    }
}

fn walk(root: &Path, dir: &Path, out: &Path, reports: &mut Vec<FileReport>) {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            walk(root, &p, out, reports);
            continue;
        }
        let rel = p
            .strip_prefix(root)
            .unwrap_or(&p)
            .to_string_lossy()
            .replace(char::from(92u8), "/");
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = stem_of(&rel).to_lowercase();
        let dirout = out.join(parent_dir_of(&rel));
        let (kind, res) = dispatch(&ext, &stem, &p, &dirout, &rel);
        reports.push(FileReport {
            path: rel,
            kind: kind.to_string(),
            status: res.0,
            detail: res.1,
        });
    }
}
