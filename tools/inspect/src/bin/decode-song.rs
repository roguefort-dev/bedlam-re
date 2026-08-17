//! decode-song: dump a Bedlam .MRS song (duration + instrument table) to
//! stdout. Auto-joins the sibling .MRW bank when present.
//!
//! Usage: decode-song <file.MRS> [more.MRS ...]

use bedlam_assets::music::{MrsEvent, MrsWalkEnd};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: decode-song <file.MRS> [more.MRS ...]");
        std::process::exit(2);
    }
    let mut failed = false;
    for a in &args {
        if !decode_one(Path::new(a)) {
            failed = true;
        }
    }
    if failed {
        std::process::exit(1);
    }
}

fn decode_one(path: &Path) -> bool {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: read failed: {e}", path.display());
            return false;
        }
    };
    let m = match bedlam_assets::music::parse_mrs(&data) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return false;
        }
    };
    let mrw_bytes = fs::read(path.with_extension("MRW")).ok();
    let bank = mrw_bytes
        .as_deref()
        .and_then(|d| bedlam_assets::music::parse_mrw(d).ok());

    let ticks = m.song_len_ticks().unwrap_or(0);
    println!(
        "{}: {} chunks x {} chan, song length {} ticks ({}.{:02} s)",
        path.display(),
        m.chunk_count,
        m.chan_count,
        ticks,
        ticks / 100,
        ticks % 100
    );
    for i in 0..m.chunk_count {
        if m.is_disabled(i) {
            println!(
                "  chunk {i}: disabled (size {}, variant {})",
                m.sizes[i], m.variants[i]
            );
            continue;
        }
        match m.walk(i) {
            Some((events, end)) => {
                let notes = events
                    .iter()
                    .filter(|e| matches!(e, MrsEvent::Note { volume, .. } if *volume != 0xff))
                    .count();
                let offs = events
                    .iter()
                    .filter(|e| matches!(e, MrsEvent::Note { volume, .. } if *volume == 0xff))
                    .count();
                let restarts = events
                    .iter()
                    .filter(|e| matches!(e, MrsEvent::Restart { .. }))
                    .count();
                let delta_sum: u64 = events
                    .iter()
                    .map(|e| match e {
                        MrsEvent::Note { delta, .. }
                        | MrsEvent::SongEnd { delta }
                        | MrsEvent::Rest { delta }
                        | MrsEvent::Restart { delta, .. } => u64::from(*delta),
                    })
                    .sum();
                println!(
                    "  chunk {i}: variant {}, {} events ({} notes, {} offs, {} restarts), delta sum {}, end {}",
                    m.variants[i],
                    events.len(),
                    notes,
                    offs,
                    restarts,
                    delta_sum,
                    end_name(&end)
                );
            }
            None => println!("  chunk {i}: walk unavailable"),
        }
    }
    match &bank {
        Some(b) => {
            println!(
                "instrument table (bank {}):",
                path.with_extension("MRW").display()
            );
            for s in inspect::formats::music::inst_stats(&m) {
                let (a, z) = b.wave_range(s.instrument as usize).unwrap_or((0, 0));
                println!(
                    "  inst {:>2}: {:>4} notes {:>3} offs  vol {:#04x}..{:#04x}  ratio {:>5.3}..{:>5.3}  wave {:>7} B @ {:#x}",
                    s.instrument,
                    s.note_ons,
                    s.note_offs,
                    s.min_vol,
                    s.max_vol,
                    s.min_ratio as f64 / 65536.0,
                    s.max_ratio as f64 / 65536.0,
                    z - a,
                    a
                );
            }
            println!(
                "  bank: {} instruments, exhaustive: {}",
                b.count,
                b.exhaustive(mrw_bytes.as_ref().map(|v| v.len()).unwrap_or(0))
            );
        }
        None => println!("no sibling .MRW bank found (instrument join skipped)"),
    }
    true
}

fn end_name(end: &MrsWalkEnd) -> String {
    match end {
        MrsWalkEnd::Freeze { .. } => String::from("freeze (natural stop)"),
        MrsWalkEnd::Eof { .. } => String::from("eof"),
        MrsWalkEnd::Truncated { .. } => String::from("truncated"),
        MrsWalkEnd::Restart { .. } => String::from("restart (loop point)"),
        MrsWalkEnd::SongEnd { .. } => String::from("song-end"),
        MrsWalkEnd::Budget { .. } => String::from("budget"),
    }
}
