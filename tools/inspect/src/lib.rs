//! inspect library surface: format dumpers shared by the corpus walker
//! (src/main.rs) and the decode-song bin (src/bin/decode-song.rs).

pub mod formats;

/// Lowercased file stem of a repo-relative path (same rule the walker uses).
pub fn stem_of(rel: &str) -> String {
    let base = rel.rsplit("/").next().unwrap_or(rel);
    match base.rfind(".") {
        Some(i) => base[..i].to_string(),
        None => base.to_string(),
    }
}

/// Parent directory of a repo-relative path ("." at the root).
pub fn parent_dir_of(rel: &str) -> String {
    match rel.rfind("/") {
        Some(i) => rel[..i].to_string(),
        None => String::from("."),
    }
}
