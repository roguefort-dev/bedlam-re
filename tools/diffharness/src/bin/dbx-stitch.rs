//! dbx-stitch — W4 stitcher CLI (DESIGN-DIFFHARNESS.md §3/§10-W4).
//!
//! Converts a channel capture transcript (DBXCAP v1, see `runner`) into
//! the W3 dump + digest manifest. The capture channel itself is not yet
//! re-pinned (RUNTIME.md "DH-G0 channel audit": the flathub DOSBox-X pin
//! has no debugger and log-only JS); whatever instrument lands at DH-G0
//! only needs to emit DBXCAP.
//!
//! Usage:
//! ```text
//! dbx-stitch <scenario.scen> <capture.dbxcap> --build <binary> [--out-dir DIR]
//!            [--pin k=v ...] [--build-sha256 <64hex>] [--channel o1|o2]
//! ```
//! Outputs (under `--out-dir`, default: alongside the transcript):
//! `<scenario>.bdld` (the dump; asset-derived, runtime/-only per D77)
//! and `<scenario>.manifest.json` (the committed fingerprint form).
//! The manifest is also printed to stdout.
//!
//! Channels (D139): `o1` (default) = an EXD/DOSBox-X capture — every
//! transcript id validates against the registry `exd_addr` rule;
//! `o2` = an EXW/Wine spot-check capture (W11) — ids validate against
//! `exw_addr` instead (the EXD-only rows reject loud), and the build
//! identity should be the watched EXW binary.

use diffharness::dump::{Channel, DumpHeader};
use diffharness::hash::{hex_lower, sha256};
use diffharness::registry;
use diffharness::runner::{self, Scenario, Transcript};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn die(msg: &str) -> ExitCode {
    eprintln!("dbx-stitch: error: {msg}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let scen_path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => return die("usage: dbx-stitch <scenario.scen> <capture.dbxcap> --build <binary> [--out-dir DIR] [--pin k=v] [--build-sha256 <hex>] [--channel o1|o2]"),
    };
    let cap_path = match args.next() {
        Some(p) => PathBuf::from(p),
        None => return die("missing <capture.dbxcap> argument"),
    };
    let mut build_path: Option<PathBuf> = None;
    let mut build_sha_hex: Option<String> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut pins: Vec<String> = Vec::new();
    let mut channel = Channel::O1ExdDosboxX;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--build" => match args.next() {
                Some(p) => build_path = Some(PathBuf::from(p)),
                None => return die("--build needs a path"),
            },
            "--build-sha256" => match args.next() {
                Some(h) => build_sha_hex = Some(h),
                None => return die("--build-sha256 needs 64 hex digits"),
            },
            "--out-dir" => match args.next() {
                Some(d) => out_dir = Some(PathBuf::from(d)),
                None => return die("--out-dir needs a path"),
            },
            "--pin" => match args.next() {
                Some(p) => pins.push(p),
                None => return die("--pin needs k=v"),
            },
            "--channel" => match args.next().as_deref() {
                Some("o1") => channel = Channel::O1ExdDosboxX,
                Some("o2") => channel = Channel::O2ExwWine,
                Some(other) => return die(&format!("--channel expects o1 or o2, got {other:?}")),
                None => return die("--channel expects o1 or o2"),
            },
            other => return die(&format!("unknown argument {other:?}")),
        }
    }

    let scen_src = match std::fs::read_to_string(&scen_path) {
        Ok(s) => s,
        Err(e) => return die(&format!("cannot read scenario {scen_path:?}: {e}")),
    };
    let scenario = match Scenario::parse(&scen_src) {
        Ok(s) => s,
        Err(e) => return die(&format!("{e}")),
    };
    let cap_src = match std::fs::read_to_string(&cap_path) {
        Ok(s) => s,
        Err(e) => return die(&format!("cannot read transcript {cap_path:?}: {e}")),
    };
    let transcript = match Transcript::parse(&cap_src) {
        Ok(t) => t,
        Err(e) => return die(&format!("{e}")),
    };

    // build_sha256: the watched binary. Prefer hashing the file; accept a
    // literal (for runs where the binary is not reachable).
    let build_sha256: [u8; 32] = if let Some(path) = &build_path {
        match std::fs::read(path) {
            Ok(bytes) => sha256(&bytes),
            Err(e) => return die(&format!("cannot read build binary {path:?}: {e}")),
        }
    } else if let Some(hex) = &build_sha_hex {
        match parse_hex32(hex) {
            Some(b) => b,
            None => return die("--build-sha256 must be 64 hex digits"),
        }
    } else {
        return die("missing build identity: pass --build <binary> or --build-sha256 <hex>");
    };

    let mut header = DumpHeader::new(channel, build_sha256, scenario.id.clone());
    for p in pins {
        header.push_pin(p);
    }

    let stitched = match runner::stitch(&scenario, &transcript, &header, &registry()) {
        Ok(s) => s,
        Err(e) => return die(&format!("{e}")),
    };

    let out_dir = out_dir.unwrap_or_else(|| {
        cap_path
            .parent()
            .map(Path::to_path_buf)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        return die(&format!("cannot create {out_dir:?}: {e}"));
    }
    let dump_path = out_dir.join(format!("{}.bdld", scenario.id));
    let manifest_path = out_dir.join(format!("{}.manifest.json", scenario.id));
    if let Err(e) = std::fs::write(&dump_path, &stitched.bytes) {
        return die(&format!("cannot write {dump_path:?}: {e}"));
    }
    if let Err(e) = std::fs::write(&manifest_path, stitched.manifest.to_json()) {
        return die(&format!("cannot write {manifest_path:?}: {e}"));
    }
    print!("{}", stitched.manifest.to_json());
    eprintln!(
        "dbx-stitch: wrote {} ({} B) + manifest (sha256 {}, chain {})",
        dump_path.display(),
        stitched.bytes.len(),
        &hex_lower(&sha256(&stitched.bytes))[..16],
        stitched.manifest.chain_digest,
    );
    ExitCode::SUCCESS
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim().to_ascii_lowercase();
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let b = s.as_bytes();
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = (b[i * 2] as char).to_digit(16)? as u8;
        let lo = (b[i * 2 + 1] as char).to_digit(16)? as u8;
        *slot = hi << 4 | lo;
    }
    Some(out)
}
