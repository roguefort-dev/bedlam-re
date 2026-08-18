//! Music-bridge corpus gates (DESIGN-GAME sec 9, deliverable c): over
//! the 5 shipped .MRS files, the bedlam-assets -> bedlam-audio bridge
//! (music.rs) must agree with an independent walk-side recomputation
//! event-for-event, select the MELODY chunk (D27 - never the chunk-1
//! loop timer), end every enabled stream in a freeze, keep chunk 0
//! disabled, and mix byte-identically under any host chunking.
//! Skips when game-data is absent (CI).

use std::fs;
use std::path::{Path, PathBuf};

use bedlam_assets::music::{parse_mrs, Mrs, MrsEvent, MrsWalkEnd};
use bedlam_audio::{Mixer, MusicCommand};
use bedlam_game::MusicPump;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data")
}

fn mrs_files() -> Option<Vec<PathBuf>> {
    let dir = corpus_root().join("BEDLAM/SOUND/MIDI");
    let mut v: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mrs")))
            .collect(),
        Err(_) => {
            eprintln!("game-data corpus not found - skipping");
            return None;
        }
    };
    v.sort();
    assert_eq!(v.len(), 5, "expected the 5 shipped .MRS files");
    Some(v)
}

/// Independent absolute-tick recomputation from the walk side (the
/// semantics under test): the script tick of event i is the running
/// sum of the decoded deltas of events 1..=i (event 1 sits at its own
/// delta; DESIGN-GAME sec 5 - deltas accumulate from stream start).
fn reference_events(mrs: &Mrs, chunk: usize) -> Vec<(u32, MusicCommand)> {
    let (events, _end) = mrs.walk(chunk).expect("enabled chunk walks");
    let mut out = Vec::new();
    let mut tick = 0u32;
    for ev in &events {
        match ev {
            MrsEvent::Note {
                delta,
                instrument,
                ratio,
                volume,
                ..
            } => {
                tick += u32::from(*delta);
                let cmd = if *volume == 0xFF {
                    MusicCommand::NoteOff {
                        instrument: *instrument,
                    }
                } else {
                    MusicCommand::NoteOn {
                        instrument: *instrument,
                        ratio: *ratio,
                        volume: *volume,
                    }
                };
                out.push((tick, cmd));
            }
            MrsEvent::Rest { delta } => tick += u32::from(*delta),
            // Terminals only extend the end tick.
            MrsEvent::SongEnd { delta } | MrsEvent::Restart { delta, .. } => {
                tick += u32::from(*delta)
            }
        }
    }
    out
}

#[test]
fn bridge_matches_the_walk_on_the_corpus() {
    let Some(files) = mrs_files() else { return };
    for path in files {
        let bytes = fs::read(&path).expect("read mrs");
        let mrs = parse_mrs(&bytes).expect("parse mrs");

        // Corpus invariants (RE-EXW-MUSIC sec 3b, assets corpus test):
        // chunk 0 disabled everywhere; every ENABLED stream ends in the
        // terminal freeze word - directly (Freeze) or via the chunk-1
        // loop timer, a single unconditional pattern RESTART followed
        // by the freeze word (Restart). Mirrors validate_mrs_song.
        assert!(mrs.is_disabled(0), "chunk 0 must be disabled: {path:?}");
        for c in 0..mrs.chunk_count {
            if !mrs.is_disabled(c) {
                let (_, end) = mrs.walk(c).expect("walk");
                assert!(
                    matches!(end, MrsWalkEnd::Freeze { .. } | MrsWalkEnd::Restart { .. }),
                    "chunk {c} of {path:?} ended in {end:?}"
                );
            }
        }

        // The pump selects the MELODY chunk, not the chunk-1 loop timer.
        let pump = MusicPump::new(&bytes).expect("melody chunk exists");
        assert!(pump.chunk() >= 2, "D27: melody starts at chunk 2: {path:?}");

        // Event-for-event walk-vs-script equivalence.
        let reference = reference_events(&mrs, pump.chunk());
        let got: Vec<(u32, MusicCommand)> = pump.script().events().to_vec();
        assert_eq!(got, reference, "bridge mismatch in {path:?}");
        assert!(!got.is_empty(), "melody chunk has notes: {path:?}");
        let mut last = 0u32;
        for &(t, _) in &got {
            assert!(t >= last, "non-decreasing ticks in {path:?}");
            last = t;
        }

        // Volumes sit in the observed stream band: note-on volumes are
        // u7 values (0xFF is the note-off marker, never a volume).
        for &(t, cmd) in &got {
            if let MusicCommand::NoteOn { volume, .. } = cmd {
                assert!(
                    volume <= 0x7F,
                    "volume {volume:#x} outside u7 band at tick {t}: {path:?}"
                );
            }
        }
        let meta = pump.meta();
        assert_eq!(meta.note_events, got.len());
        assert_eq!(meta.terminal, bedlam_game::ScriptTerminal::Freeze);
        assert_eq!(meta.end_tick, last, "end tick = last event tick: {path:?}");

        // Restart rebuilds the identical script (the loop analog).
        let mut p2 = MusicPump::new(&bytes).unwrap();
        p2.restart().unwrap();
        assert_eq!(p2.script().events(), pump.script().events());
    }
}

#[test]
fn bridged_scripts_mix_chunking_invariantly() {
    let Some(files) = mrs_files() else { return };
    for path in files {
        let bytes = fs::read(&path).expect("read mrs");
        let pump = MusicPump::new(&bytes).expect("bridge");
        let script = pump.script().clone();

        // Song bounds: last event tick + a 10-tick ring-out margin.
        let last_tick = script.events().last().map(|&(t, _)| t).unwrap_or(0);
        let total_frames = last_tick as usize * 11025 / 100 + 1103;

        let mixed = |chunk: usize| -> Vec<i16> {
            let mut mixer = Mixer::new();
            mixer.set_master_volume(100);
            let mut wave = Vec::with_capacity(256);
            for i in 0..256u16 {
                wave.push((i.wrapping_mul(37) % 256) as u8);
            }
            for (_t, cmd) in script.events() {
                if let MusicCommand::NoteOn { instrument, .. } = cmd {
                    mixer.load_wave(*instrument, &wave).expect("load wave");
                }
            }
            mixer.load_script(script.clone());
            let mut out = Vec::new();
            let mut chunkbuf = vec![0i16; 2 * chunk];
            while out.len() / 2 < total_frames {
                let n = mixer.render(&mut chunkbuf).expect("render");
                if n == 0 {
                    break;
                }
                out.extend_from_slice(&chunkbuf[..2 * n]);
            }
            // Each chunking overshoots the song bound by up to chunk-1
            // frames (whole-buffer renders); compare the exact common
            // prefix, like the bedlam-audio determinism suite does.
            out.truncate(2 * total_frames);
            out
        };

        let one = mixed(1);
        let seven = mixed(7);
        let big = mixed(64);
        assert_eq!(one, seven, "7-frame chunking diverged: {path:?}");
        assert_eq!(one, big, "64-frame chunking diverged: {path:?}");
        assert!(!one.is_empty());
    }
}
