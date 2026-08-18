//! MusicPump - the bedlam-assets -> bedlam-audio bridge (DESIGN-GAME
//! sec 5; DESIGN-AUDIO sec 7). This is the ONLY place the two crates
//! meet: Mrs streams walk into absolute-tick MusicScripts the Mixer
//! dispatches on its internal Q16 grid. SFX bypass this path entirely
//! (host-event-timed, DESIGN-AUDIO sec 7).

use bedlam_assets::music::{parse_mrs, Mrs, MrsEvent, MrsWalkEnd};
use bedlam_audio::{MusicCommand, MusicScript};

use crate::fsm::Scene;
use crate::GameError;

/// Per-scene music track (corpus fact: the 5 shipped .MRS files are
/// named for their screens). None = no scripted track (title = the
/// TITLE.SMK video; mission music = DESIGN-GAME open question Q2).
pub fn track_name(scene: Scene) -> Option<&'static str> {
    match scene {
        Scene::Options => Some("OPTIONS.MRS"),
        Scene::Brief => Some("BRIEF.MRS"),
        Scene::Select => Some("SELECT.MRS"),
        Scene::Debrief => Some("DEBRIEF.MRS"),
        Scene::Shop => Some("SHOP.MRS"),
        _ => None,
    }
}

/// Terminal condition of a built script (MrsWalkEnd minus the byte
/// offsets - the mixer only needs the kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptTerminal {
    Freeze,
    Eof,
    Truncated,
    Restart,
    SongEnd,
    Budget,
}

impl ScriptTerminal {
    fn from_walk(end: &MrsWalkEnd) -> ScriptTerminal {
        match end {
            MrsWalkEnd::Freeze { .. } => ScriptTerminal::Freeze,
            MrsWalkEnd::Eof { .. } => ScriptTerminal::Eof,
            MrsWalkEnd::Truncated { .. } => ScriptTerminal::Truncated,
            MrsWalkEnd::Restart { .. } => ScriptTerminal::Restart,
            MrsWalkEnd::SongEnd { .. } => ScriptTerminal::SongEnd,
            MrsWalkEnd::Budget { .. } => ScriptTerminal::Budget,
        }
    }
}

/// Side facts of a built script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptMeta {
    /// How the walk ended.
    pub terminal: ScriptTerminal,
    /// NoteOn + NoteOff commands in the script.
    pub note_events: usize,
    /// Absolute tick of the last decoded event.
    pub end_tick: u32,
}

/// Build the absolute-tick MusicScript for one chunk (the DESIGN-AUDIO
/// sec 7 mapping): deltas accumulate; volume 0xFF decodes to NoteOff;
/// rests advance the clock only; the walk terminal lands in the meta so
/// the host can decide the loop (SongEnd/Restart re-init the walk).
pub fn build_script(mrs: &Mrs, chunk: usize) -> Result<(MusicScript, ScriptMeta), GameError> {
    let (events, end) = mrs.walk(chunk).ok_or(GameError::BadMusicChunk { chunk })?;
    let mut script = MusicScript::new();
    let mut tick: u32 = 0;
    let mut note_events = 0usize;
    for ev in &events {
        match ev {
            MrsEvent::Note {
                delta,
                volume,
                instrument,
                ratio,
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
                script.push(tick, cmd)?;
                note_events += 1;
            }
            MrsEvent::Rest { delta } => tick += u32::from(*delta),
            // Terminals carry one last delta; the walk already stopped.
            MrsEvent::SongEnd { delta } | MrsEvent::Restart { delta, .. } => {
                tick += u32::from(*delta);
            }
        }
    }
    Ok((
        script,
        ScriptMeta {
            terminal: ScriptTerminal::from_walk(&end),
            note_events,
            end_tick: tick,
        },
    ))
}

/// First enabled chunk whose walk contains at least one Note event
/// (D27: skips the chunk-1 loop timer of the shipped corpus).
fn first_melody_chunk(mrs: &Mrs) -> Option<usize> {
    (0..mrs.chunk_count)
        .filter(|&c| !mrs.is_disabled(c))
        .find(|&c| {
            mrs.walk(c)
                .is_some_and(|(evs, _)| evs.iter().any(|e| matches!(e, MrsEvent::Note { .. })))
        })
}

/// Host-side pump state for one loaded track: the parsed Mrs, the chosen
/// chunk and its pre-built script (rebuilt on restart = the loop).
#[derive(Debug, Clone)]
pub struct MusicPump {
    mrs: Mrs,
    chunk: usize,
    script: MusicScript,
    meta: ScriptMeta,
    restarts: u32,
}

impl MusicPump {
    /// Parse the bytes and select the MELODY chunk: the first enabled
    /// chunk whose walk yields at least one Note event. Corpus shape
    /// (bedlam-assets corpus test + RE-EXW-MUSIC sec 2): chunk 0 is
    /// disabled in every shipped file and chunk 1 is the LOOP TIMER (a
    /// single unconditional Restart whose delta == its table-B entry ==
    /// the song length), so "first enabled" alone would sequence the
    /// timer and stay silent; melody streams start at chunk 2 (D27).
    pub fn new(mrs_bytes: &[u8]) -> Result<MusicPump, GameError> {
        let mrs = parse_mrs(mrs_bytes)?;
        let chunk = first_melody_chunk(&mrs).ok_or(GameError::BadMusicChunk { chunk: 0 })?;
        let (script, meta) = build_script(&mrs, chunk)?;
        Ok(MusicPump {
            mrs,
            chunk,
            script,
            meta,
            restarts: 0,
        })
    }

    /// The pre-built script (attach to the Mixer on scene change).
    pub fn script(&self) -> &MusicScript {
        &self.script
    }

    /// Build facts of the current script.
    pub fn meta(&self) -> ScriptMeta {
        self.meta
    }

    /// The chunk the pump walks.
    pub fn chunk(&self) -> usize {
        self.chunk
    }

    /// Loop: rebuild the script from the same bytes (the MrsChunkStart
    /// re-init analog). Deterministic: same bytes, same script.
    pub fn restart(&mut self) -> Result<(), GameError> {
        let (script, meta) = build_script(&self.mrs, self.chunk)?;
        self.script = script;
        self.meta = meta;
        self.restarts += 1;
        Ok(())
    }

    /// Restarts since construction.
    pub fn restarts(&self) -> u32 {
        self.restarts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedlam_assets::music::RATIO_TABLE;

    /// Synthetic two-chunk Mrs: chunk 0 disabled, chunk 1 variant 1 with
    /// one NoteOn, one NoteOff, one rest, then the freeze word.
    fn synth_mrs_bytes() -> Vec<u8> {
        let mut stream: Vec<u8> = Vec::new();
        stream.extend_from_slice(&3u16.to_le_bytes()); // delta 3
        stream.push(0x60); // note byte (variant 1 -> ratio from table)
        stream.push(10); // volume
        stream.extend_from_slice(&2u16.to_le_bytes()); // delta 2
        stream.push(0x60);
        stream.push(0xFF); // note-off volume
        stream.extend_from_slice(&4u16.to_le_bytes()); // delta 4
        stream.push(0x90); // rest opcode
        stream.push(0);
        stream.extend_from_slice(&0xFFFFu16.to_le_bytes()); // freeze
        let mrs = Mrs {
            chunk_count: 2,
            chan_count: 1,
            sizes: vec![0, stream.len() as u16],
            variants: vec![0, 1],
            start_offsets: vec![0xFFFF, 0],
            tick_delays: vec![0, 9],
            table_c: vec![0, 0],
            data_off: 28,
            streams: vec![Vec::new(), stream],
        };
        mrs.to_bytes()
    }

    #[test]
    fn track_table_matches_the_corpus_names() {
        assert_eq!(track_name(Scene::Options), Some("OPTIONS.MRS"));
        assert_eq!(track_name(Scene::Brief), Some("BRIEF.MRS"));
        assert_eq!(track_name(Scene::Select), Some("SELECT.MRS"));
        assert_eq!(track_name(Scene::Debrief), Some("DEBRIEF.MRS"));
        assert_eq!(track_name(Scene::Shop), Some("SHOP.MRS"));
        for scene in [
            Scene::Boot,
            Scene::Title,
            Scene::Mission,
            Scene::Cutscene,
            Scene::Quit,
        ] {
            assert_eq!(track_name(scene), None, "{scene:?}");
        }
    }

    #[test]
    fn build_script_maps_the_grammar() {
        let bytes = synth_mrs_bytes();
        let mrs = parse_mrs(&bytes).expect("synthetic Mrs must parse");
        let err = build_script(&mrs, 0).unwrap_err();
        assert!(matches!(err, GameError::BadMusicChunk { chunk: 0 }));
        let (script, meta) = build_script(&mrs, 1).unwrap();
        // delta 3 note-on, delta 2 note-off, rest advances to 9.
        assert_eq!(
            script.events(),
            [
                (
                    3,
                    MusicCommand::NoteOn {
                        instrument: 8, // variant 1 + 7
                        ratio: RATIO_TABLE[0x60],
                        volume: 10,
                    }
                ),
                (5, MusicCommand::NoteOff { instrument: 8 }),
            ]
        );
        assert_eq!(meta.terminal, ScriptTerminal::Freeze);
        assert_eq!(meta.note_events, 2);
        assert_eq!(meta.end_tick, 9);
    }

    #[test]
    fn pump_selects_the_melody_chunk_and_restarts_identically() {
        let bytes = synth_mrs_bytes();
        let mut pump = MusicPump::new(&bytes).unwrap();
        assert_eq!(pump.chunk(), 1);
        let before = pump.script().clone();
        let meta = pump.meta();
        pump.restart().unwrap();
        assert_eq!(pump.restarts(), 1);
        assert_eq!(pump.script().events(), before.events());
        assert_eq!(pump.meta(), meta, "rebuild is deterministic");
    }
}
