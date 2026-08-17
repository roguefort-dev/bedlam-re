//! .MRS song dumper: container + event-stream walk summary (see
//! bedlam-assets music module and docs/RE-EXW-MUSIC.md sec 2/2b).

use crate::stem_of;
use bedlam_assets as assets;
use bedlam_assets::music::{Mrs, MrsEvent, MrsWalkEnd};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Per-instrument histogram over every enabled chunk of the song.
pub struct InstStat {
    pub instrument: u16,
    pub note_ons: usize,
    pub note_offs: usize,
    pub min_vol: u8,
    pub max_vol: u8,
    pub min_ratio: u32,
    pub max_ratio: u32,
}

pub fn inst_stats(m: &Mrs) -> Vec<InstStat> {
    let mut v: Vec<InstStat> = Vec::new();
    for chunk in 0..m.chunk_count {
        let Some((events, _)) = m.walk(chunk) else {
            continue;
        };
        for e in events {
            if let MrsEvent::Note {
                volume,
                instrument,
                ratio,
                ..
            } = e
            {
                let off = volume == 0xff;
                match v.iter_mut().find(|s| s.instrument == instrument) {
                    Some(s) => {
                        if off {
                            s.note_offs += 1;
                        } else {
                            s.note_ons += 1;
                            s.min_vol = s.min_vol.min(volume);
                            s.max_vol = s.max_vol.max(volume);
                        }
                        s.min_ratio = s.min_ratio.min(ratio);
                        s.max_ratio = s.max_ratio.max(ratio);
                    }
                    None => v.push(InstStat {
                        instrument,
                        note_ons: usize::from(!off),
                        note_offs: usize::from(off),
                        min_vol: volume,
                        max_vol: volume,
                        min_ratio: ratio,
                        max_ratio: ratio,
                    }),
                }
            }
        }
    }
    v.sort_by_key(|s| s.instrument);
    v
}

fn end_name(end: &MrsWalkEnd) -> String {
    match end {
        MrsWalkEnd::Freeze { .. } => String::from("freeze"),
        MrsWalkEnd::Eof { .. } => String::from("eof"),
        MrsWalkEnd::Truncated { .. } => String::from("truncated"),
        MrsWalkEnd::Restart { .. } => String::from("restart"),
        MrsWalkEnd::SongEnd { .. } => String::from("song-end"),
        MrsWalkEnd::Budget { .. } => String::from("budget"),
    }
}

/// Full song document (JSON): layout tables, per-chunk walk stats and the
/// instrument histogram, optionally joined with the sibling .MRW bank.
pub fn song_doc(
    m: &Mrs,
    size: usize,
    mrw: Option<(&assets::music::Mrw, usize)>,
) -> serde_json::Value {
    let stats = inst_stats(m);
    let instruments: Vec<serde_json::Value> = stats
        .iter()
        .map(|s| {
            let wave = mrw
                .and_then(|(b, _)| b.wave_range(s.instrument as usize))
                .map(|(a, z)| json!({ "off": a, "size": z - a }));
            json!({
                "id": s.instrument,
                "note_ons": s.note_ons,
                "note_offs": s.note_offs,
                "min_vol": s.min_vol,
                "max_vol": s.max_vol,
                "min_ratio": s.min_ratio,
                "max_ratio": s.max_ratio,
                "wave": wave,
            })
        })
        .collect();
    let chunk_table: Vec<serde_json::Value> = (0..m.chunk_count)
        .map(|i| {
            json!({
                "i": i,
                "size": m.sizes[i],
                "variant": m.variants[i],
                "start": format!("{:#06x}", m.start_offsets[i]),
                "ticks": m.tick_delays[i],
                "disabled": m.is_disabled(i),
            })
        })
        .collect();
    let chunks_walked: Vec<serde_json::Value> = (0..m.chunk_count)
        .filter_map(|i| {
            let (events, end) = m.walk(i)?;
            let notes = events
                .iter()
                .filter(|e| matches!(e, MrsEvent::Note { volume, .. } if *volume != 0xff))
                .count();
            let offs = events
                .iter()
                .filter(|e| matches!(e, MrsEvent::Note { volume, .. } if *volume == 0xff))
                .count();
            let rests = events
                .iter()
                .filter(|e| matches!(e, MrsEvent::Rest { .. }))
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
            json!({
                "i": i,
                "events": events.len(),
                "notes": notes,
                "note_offs": offs,
                "rests": rests,
                "delta_sum": delta_sum,
                "end": end_name(&end),
            })
            .into()
        })
        .collect();
    json!({
        "size": size,
        "chunks": m.chunk_count,
        "channels": m.chan_count,
        "song_len_ticks": m.song_len_ticks(),
        "song_len_ms": m.song_len_ms(),
        "mrw_instruments": mrw.map(|(b, _)| b.count),
        "chunk_table": chunk_table,
        "chunks_walked": chunks_walked,
        "instruments": instruments,
    })
}

/// Inspect-walker dump: parse, walk, write <stem>.song.json, report status.
pub fn dump_mrs(path: &Path, out_dir: &Path, rel: &str) -> (String, String) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (String::from("error"), format!("read failed: {e}")),
    };
    let m = match assets::music::parse_mrs(&data) {
        Ok(m) => m,
        Err(e) => return (String::from("heuristic-failed"), e.to_string()),
    };
    let mrw = fs::read(path.with_extension("MRW"))
        .ok()
        .and_then(|d| assets::music::parse_mrw(&d).ok().map(|b| (b, 0)));
    let stats = inst_stats(&m);
    let doc = song_doc(&m, data.len(), mrw.as_ref().map(|(b, _)| (b, 0)));
    let _ = fs::create_dir_all(out_dir);
    let ok = fs::write(
        out_dir.join(format!("{}.song.json", stem_of(rel))),
        serde_json::to_string_pretty(&doc).unwrap_or_default(),
    )
    .is_ok();
    let len = m.song_len_ticks().unwrap_or(0);
    let detail = format!(
        "{} chunks x {} chan, {} instruments, loop {} ticks ({}.{:02} s){}",
        m.chunk_count,
        m.chan_count,
        stats.len(),
        len,
        len / 100,
        len % 100,
        if ok { "" } else { " (json write failed)" }
    );
    (String::from("parsed"), detail)
}
