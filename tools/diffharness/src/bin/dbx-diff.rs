//! dbx-diff — the W7 differ CLI (DESIGN-DIFFHARNESS.md §6).
//!
//! Compares two (or three, with `--tiebreak`) W3 dump files and writes
//! the human report (+ the fingerprint manifest with `--manifest`).
//! Mode defaults from the dump channels: two O1 dumps → `double-run`
//! (the DH-G1 verdict instrument); anything else → `cross-channel`.
//!
//! Dumps stay under runtime/ (§3 hygiene); the manifest is the
//! git-carried fingerprint.

use std::process::ExitCode;

use diffharness::differ::{report_text, run_diff, DiffConfig, Mode};
use diffharness::dump::Channel;
use diffharness::registry;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut a: Option<String> = None;
    let mut b: Option<String> = None;
    let mut tiebreak: Option<String> = None;
    let mut mode: Option<Mode> = None;
    let mut quantum: Option<i64> = None;
    let mut report_path: Option<String> = None;
    let mut manifest_path: Option<String> = None;
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--tiebreak" => tiebreak = it.next().cloned(),
            "--mode" => match it.next().map(|s| s.as_str()) {
                Some("double-run") => mode = Some(Mode::DoubleRun),
                Some("cross") | Some("cross-channel") => mode = Some(Mode::CrossChannel),
                other => {
                    eprintln!("dbx-diff: unknown --mode {other:?} (want double-run|cross)");
                    return ExitCode::FAILURE;
                }
            },
            "--t2-quantum" => match it.next().and_then(|v| v.parse::<i64>().ok()) {
                Some(q) if q >= 0 => quantum = Some(q),
                _ => {
                    eprintln!("dbx-diff: --t2-quantum needs a non-negative integer");
                    return ExitCode::FAILURE;
                }
            },
            "--report" => report_path = it.next().cloned(),
            "--manifest" => manifest_path = it.next().cloned(),
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("dbx-diff: unknown flag {other:?}");
                print_help();
                return ExitCode::FAILURE;
            }
            other => {
                if a.is_none() {
                    a = Some(other.to_string());
                } else if b.is_none() {
                    b = Some(other.to_string());
                } else {
                    eprintln!("dbx-diff: unexpected extra argument {other:?}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    let (Some(a_path), Some(b_path)) = (a, b) else {
        print_help();
        return ExitCode::FAILURE;
    };

    let read = |p: &str| match std::fs::read(p) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("dbx-diff: cannot read {p}: {e}");
            std::process::exit(2);
        }
    };
    let a_bytes = read(&a_path);
    let b_bytes = read(&b_path);
    let t_bytes = tiebreak.as_ref().map(|p| read(p));

    // Mode default from the dump channels (decode is cheap + verifies
    // integrity before the differ runs).
    let channel_of = |bytes: &[u8]| -> Channel {
        match diffharness::dump::decode_dump(bytes) {
            Ok(d) => d.header.channel,
            Err(e) => {
                eprintln!("dbx-diff: dump does not verify: {e}");
                std::process::exit(2);
            }
        }
    };
    let mode = mode.unwrap_or(match (channel_of(&a_bytes), channel_of(&b_bytes)) {
        (Channel::O1ExdDosboxX, Channel::O1ExdDosboxX) => Mode::DoubleRun,
        _ => Mode::CrossChannel,
    });

    let mut cfg = DiffConfig::new(mode);
    if let Some(q) = quantum {
        cfg.t2_quantum = q;
    }
    let reg = registry();
    let res = match run_diff(&a_bytes, &b_bytes, t_bytes.as_deref(), &cfg, &reg) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("dbx-diff: {e}");
            return ExitCode::FAILURE;
        }
    };

    let text = report_text(&res);
    match &report_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &text) {
                eprintln!("dbx-diff: cannot write report {p}: {e}");
                return ExitCode::FAILURE;
            }
        }
        None => print!("{text}"),
    }
    if let Some(p) = &manifest_path {
        if let Err(e) = std::fs::write(p, diffharness::differ::manifest_json(&res)) {
            eprintln!("dbx-diff: cannot write manifest {p}: {e}");
            return ExitCode::FAILURE;
        }
    }
    eprintln!(
        "dbx-diff: verdict {} ({} findings)",
        res.verdict.name(),
        res.findings.len()
    );
    match res.verdict {
        diffharness::differ::Verdict::Fail => ExitCode::FAILURE,
        _ => ExitCode::SUCCESS,
    }
}

fn print_help() {
    println!(
        "dbx-diff — the P4.2 W7 differ (DESIGN-DIFFHARNESS.md sec 6)\n\
         \n\
         USAGE: dbx-diff <a.bdl> <b.bdl> [--tiebreak <o2.bdl>]\n\
                 [--mode double-run|cross] [--t2-quantum N]\n\
                 [--report out.txt] [--manifest out.json]\n\
         \n\
         Two O1 dumps default to double-run (the DH-G1 verdict: identical\n\
         modulo the frame-counter/RNG classes); anything else defaults to\n\
         cross-channel (per-field classes + coverage findings + O2\n\
         arbitration when --tiebreak is given)."
    );
}
