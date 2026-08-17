//! Bedlam music formats: the .MRS score container + event-stream grammar
//! and the .MRW instrument waveform bank.
//!
//! Grammar and provenance: docs/RE-EXW-MUSIC.md sections 2/2b/3 [EXW+DATA,
//! byte-validated against all 5 shipped files]. One sequencer tick = 10 ms
//! (100 Hz pump, see MRS_TICK_MS).

use crate::{i16le, u16le, AssetsError};

/// One sequencer tick in milliseconds (the 100 Hz MusicPump decrements each
/// chunk delta word once per tick).
pub const MRS_TICK_MS: u64 = 10;

/// .MRS chunk-disable marker in the start-offset table (table A): the pump
/// writes 0xffff into the chunk delta and the chunk never fires. Chunk 0 is
/// disabled in every shipped file.
pub const MRS_DISABLED: u16 = 0xffff;

/// Resample ratio table @00454174 (DGROUP, EXW file offset 0x52774), 128
/// dwords of 16.16 fixed point indexed by the raw note byte. Chromatic
/// ladder: 0 below byte 0x18 (-60 semitone floor = 0x800), 1.0 = 0x10000 at
/// byte 0x54, +18-semitone ceiling 0x2d410 at 0x66 repeated through 0x7F.
/// The clamps are physical table contents, not code. Variant-1 note events
/// load ratio = RATIO_TABLE[byte] (SubVoiceStart feeds it to
/// IDirectSoundBuffer::SetFrequency as (ratio * 11025) >> 16).
/// [EXW image extraction 2026-08-17, values verbatim]
pub const RATIO_TABLE: [u32; 128] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, // 0x00
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, // 0x08
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, // 0x10
    0x00000800, 0x00000879, 0x000008fa, 0x00000983, 0x00000a14, 0x00000aad, 0x00000b50,
    0x00000bfc, // 0x18
    0x00000cb2, 0x00000d74, 0x00000e41, 0x00000f1a, 0x00001000, 0x000010f3, 0x000011f5,
    0x00001306, // 0x20
    0x00001428, 0x0000155b, 0x000016a0, 0x000017f8, 0x00001965, 0x00001ae8, 0x00001c82,
    0x00001e33, // 0x28
    0x00001fff, 0x000021e7, 0x000023eb, 0x0000260d, 0x00002850, 0x00002ab6, 0x00002d40,
    0x00002ff1, // 0x30
    0x000032cb, 0x000035d1, 0x00003904, 0x00003c68, 0x00003fff, 0x000043cd, 0x000047d6,
    0x00004c1b, // 0x38
    0x000050a1, 0x0000556d, 0x00005a82, 0x00005fe3, 0x00006595, 0x00006ba2, 0x00007208,
    0x000078d0, // 0x40
    0x00007ffe, 0x0000879b, 0x00008fab, 0x00009836, 0x0000a143, 0x0000aada, 0x0000b502,
    0x0000bfc5, // 0x48
    0x0000cb2e, 0x0000d744, 0x0000e410, 0x0000f1a0, 0x00010000, 0x00010f37, 0x00011f57,
    0x0001306f, // 0x50
    0x00014289, 0x000155b7, 0x00016a07, 0x00017f8e, 0x0001965f, 0x0001ae88, 0x0001c820,
    0x0001e340, // 0x58
    0x0001fffd, 0x00021e70, 0x00023eb0, 0x000260dc, 0x00028512, 0x0002ab6e, 0x0002d410,
    0x0002d410, // 0x60
    0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410,
    0x0002d410, // 0x68
    0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410,
    0x0002d410, // 0x70
    0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410, 0x0002d410,
    0x0002d410, // 0x78
];

/// One decoded .MRS chunk-stream event (grammar sec 2b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MrsEvent {
    /// Note trigger, opcode 0x00..=0x7E + volume byte. volume == 0xFF is
    /// NOTE-OFF (releases the instrument base sub-voice). Decoding follows
    /// the chunk variant: variant 0 -> instrument = byte, ratio 1.0, tag 0;
    /// variant != 0 -> instrument = variant + 7, ratio = RATIO_TABLE[byte],
    /// tag = byte - 0x54.
    Note {
        delta: u16,
        byte: u8,
        volume: u8,
        instrument: u16,
        ratio: u32,
        tag: i16,
    },
    /// Song end (opcode 0x7F): the pump copies every chunk into the shadow
    /// song slot and stops. One discarded operand byte follows.
    SongEnd { delta: u16 },
    /// Rest / idle gate (opcode 0x80..=0xFD): the pump skips dispatch while
    /// the state keeps bit 7. One consumed operand byte follows.
    Rest { delta: u16 },
    /// Pattern restart (opcode 0xFE conditional on the loop flag, 0xFF
    /// unconditional): MrsChunkStart re-inits all chunks of the channel
    /// byte from the header tables. The walk stops here; later bytes would
    /// repeat.
    Restart {
        delta: u16,
        chan: u8,
        conditional: bool,
    },
}

/// Terminal condition of a chunk-stream walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MrsWalkEnd {
    /// Next delta word is signed-negative (0xFFxx): the pump never
    /// decrements it, so the chunk stalls forever = natural stop. Every
    /// shipped stream ends with this 2-byte freeze word.
    Freeze { at: usize },
    /// Stream exhausted exactly after the last whole event.
    Eof { at: usize },
    /// An event header crossed the stream edge (corrupt or foreign data).
    Truncated { at: usize },
    /// Walk stopped on a restart event; at is just past its operand byte.
    Restart { at: usize },
    /// Walk stopped on a song-end event; at is just past its operand byte.
    SongEnd { at: usize },
    /// Rewrite/rewind budget exhausted (crafted backward-jump streams; the
    /// original has no guard and would loop the pump forever).
    Budget { at: usize },
}

/// Walk one chunk event stream exactly like MrsNextEvent: start at byte
/// start (the table-A start offset; 0xffff means disabled and should not
/// be walked), decode whole events until a terminal condition. Never panics
/// and always terminates on arbitrary bytes (budget guard on the unused
/// >30000 backward-reposition path).
pub fn walk_mrs_chunk(stream: &[u8], start: usize, variant: u16) -> (Vec<MrsEvent>, MrsWalkEnd) {
    let mut events: Vec<MrsEvent> = Vec::new();
    let mut pos: i64 = start as i64;
    let mut rewinds: u32 = 0;
    loop {
        if pos < 0 || pos as u64 + 2 > stream.len() as u64 {
            let at = pos.clamp(0, stream.len() as i64) as usize;
            let end = if pos == stream.len() as i64 {
                MrsWalkEnd::Eof { at }
            } else {
                MrsWalkEnd::Truncated { at }
            };
            return (events, end);
        }
        let p = pos as usize;
        let delta = i16le(stream, p);
        if delta < 0 {
            return (events, MrsWalkEnd::Freeze { at: p });
        }
        if delta > 30000 {
            // Backward stream reposition: pos -= delta*4 - 0x1d4be, then
            // re-read. Unused by shipped data.
            if rewinds >= 8 {
                return (events, MrsWalkEnd::Budget { at: p });
            }
            rewinds += 1;
            pos = p as i64 - (delta as i64 * 4 - 0x1d4be);
            continue;
        }
        if p + 3 > stream.len() {
            return (events, MrsWalkEnd::Truncated { at: p });
        }
        let op = stream[p + 2];
        if p + 4 > stream.len() {
            return (events, MrsWalkEnd::Truncated { at: p });
        }
        let operand = stream[p + 3];
        match op {
            0x00..=0x7E => {
                let (instrument, ratio, tag) = if variant == 0 {
                    (op as u16, 0x1_0000u32, 0i16)
                } else {
                    (
                        variant.wrapping_add(7),
                        RATIO_TABLE[op as usize],
                        op as i16 - 0x54,
                    )
                };
                events.push(MrsEvent::Note {
                    delta: delta as u16,
                    byte: op,
                    volume: operand,
                    instrument,
                    ratio,
                    tag,
                });
                pos = p as i64 + 4;
            }
            0x7F => {
                events.push(MrsEvent::SongEnd {
                    delta: delta as u16,
                });
                return (events, MrsWalkEnd::SongEnd { at: p + 4 });
            }
            0x80..=0xFD => {
                events.push(MrsEvent::Rest {
                    delta: delta as u16,
                });
                pos = p as i64 + 4;
            }
            _ => {
                events.push(MrsEvent::Restart {
                    delta: delta as u16,
                    chan: operand,
                    conditional: op == 0xFE,
                });
                return (events, MrsWalkEnd::Restart { at: p + 4 });
            }
        }
        if events.len() >= 1 << 20 {
            return (events, MrsWalkEnd::Budget { at: p });
        }
    }
}

/// Parsed .MRS score container (layout sec 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mrs {
    /// W0: chunk count.
    pub chunk_count: usize,
    /// W1: channel count (1 in every shipped file).
    pub chan_count: usize,
    /// Per-chunk event-stream sizes in bytes.
    pub sizes: Vec<u16>,
    /// Per-chunk variant word (0 = raw-instrument chunks, else instrument
    /// = variant + 7 and the note byte selects ratio/tag).
    pub variants: Vec<u16>,
    /// Table A: first-event byte offset per chunk x channel; 0xffff =
    /// disabled chunk.
    pub start_offsets: Vec<u16>,
    /// Table B: initial tick delay per chunk x channel (overrides the first
    /// event delta; chunk 1 = the song length).
    pub tick_delays: Vec<u16>,
    /// Table C: written by load_midi, read by nothing in EXW (dead data,
    /// kept for byte-exact rebuilds).
    pub table_c: Vec<u16>,
    /// Byte offset of the first chunk stream.
    pub data_off: usize,
    /// Chunk event streams, back to back.
    pub streams: Vec<Vec<u8>>,
}

impl Mrs {
    /// Table entry index for (chunk, chan) in the W0*W1 tables.
    fn tbl(&self, chunk: usize, chan: usize) -> Option<usize> {
        if chunk < self.chunk_count && chan < self.chan_count {
            Some(chunk * self.chan_count + chan)
        } else {
            None
        }
    }

    /// Table-A start offset for a chunk (chan 0 in every shipped file).
    pub fn start_offset(&self, chunk: usize, chan: usize) -> Option<u16> {
        self.tbl(chunk, chan).map(|i| self.start_offsets[i])
    }

    /// Table-B initial tick delay for a chunk.
    pub fn tick_delay(&self, chunk: usize, chan: usize) -> Option<u16> {
        self.tbl(chunk, chan).map(|i| self.tick_delays[i])
    }

    /// True when the chunk is disabled (start offset 0xffff).
    pub fn is_disabled(&self, chunk: usize) -> bool {
        self.start_offset(chunk, 0) == Some(MRS_DISABLED)
    }

    /// Song length in 10 ms ticks: the table-B delay of chunk 1, the loop
    /// timer chunk (verified == its stream first delta in all shipped
    /// files). None when the layout has no such chunk.
    pub fn song_len_ticks(&self) -> Option<u16> {
        self.tick_delay(1, 0)
    }

    /// Song length in whole milliseconds.
    pub fn song_len_ms(&self) -> Option<u64> {
        self.song_len_ticks().map(|t| t as u64 * MRS_TICK_MS)
    }

    /// Walk chunk (chan 0; every shipped file has W1 = 1). None when the
    /// chunk index is out of range or disabled.
    pub fn walk(&self, chunk: usize) -> Option<(Vec<MrsEvent>, MrsWalkEnd)> {
        let start = self.start_offset(chunk, 0)?;
        if start == MRS_DISABLED {
            return None;
        }
        Some(walk_mrs_chunk(
            &self.streams[chunk],
            start as usize,
            self.variants[chunk],
        ))
    }

    /// Byte-identical rebuild of the original file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v =
            Vec::with_capacity(self.data_off + self.streams.iter().map(|s| s.len()).sum::<usize>());
        v.extend_from_slice(&(self.chunk_count as u16).to_le_bytes());
        v.extend_from_slice(&(self.chan_count as u16).to_le_bytes());
        for s in &self.sizes {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for s in &self.variants {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for s in &self.start_offsets {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for s in &self.tick_delays {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for s in &self.table_c {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for s in &self.streams {
            v.extend_from_slice(s);
        }
        v
    }
}

/// Parse a .MRS score container. Requires the exact shipped layout:
/// header tables fit and data_off + sum(sizes) == file size (true for all
/// five shipped files).
pub fn parse_mrs(data: &[u8]) -> Result<Mrs, AssetsError> {
    if data.len() < 4 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let chunk_count = u16le(data, 0) as usize;
    let chan_count = u16le(data, 2) as usize;
    let n = chunk_count
        .checked_mul(chan_count)
        .ok_or(AssetsError::CountOverruns {
            count: chunk_count,
            len: data.len(),
        })?;
    let need = 4u64 + 4u64 * chunk_count as u64 + 6u64 * n as u64;
    if need > data.len() as u64 {
        return Err(AssetsError::CountOverruns {
            count: chunk_count,
            len: data.len(),
        });
    }
    let data_off = need as usize;
    let sizes: Vec<u16> = (0..chunk_count).map(|i| u16le(data, 4 + 2 * i)).collect();
    let variants = (0..chunk_count)
        .map(|i| u16le(data, 4 + 2 * chunk_count + 2 * i))
        .collect();
    let t = 4 + 4 * chunk_count;
    let start_offsets = (0..n).map(|i| u16le(data, t + 2 * i)).collect();
    let tick_delays = (0..n).map(|i| u16le(data, t + 2 * n + 2 * i)).collect();
    let table_c = (0..n).map(|i| u16le(data, t + 4 * n + 2 * i)).collect();
    let total: u64 = sizes.iter().map(|&s| s as u64).sum();
    let expected = data_off as u64 + total;
    if expected != data.len() as u64 {
        return Err(AssetsError::MrsLayout {
            len: data.len(),
            expected: expected as usize,
        });
    }
    let mut streams = Vec::with_capacity(chunk_count);
    let mut off = data_off;
    for &s in &sizes {
        streams.push(data[off..off + s as usize].to_vec());
        off += s as usize;
    }
    Ok(Mrs {
        chunk_count,
        chan_count,
        sizes,
        variants,
        start_offsets,
        tick_delays,
        table_c,
        data_off,
        streams,
    })
}

/// One .MRW instrument directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrwChunk {
    pub off: u32,
    pub size: u32,
    /// off + size lies within the file.
    pub fits: bool,
}

/// Parsed .MRW instrument bank: u16 instrument count then count entries of
/// (off u32, size u32); offsets are relative to file start + 2 and the
/// waveform data is 11025 Hz 8-bit mono PCM (records may share offsets:
/// waveforms are deduplicated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mrw {
    pub count: usize,
    pub chunks: Vec<MrwChunk>,
}

impl Mrw {
    /// Byte range (start, end) of the instrument i waveform within the
    /// source buffer (11025 Hz 8-bit mono), or None when the index is out
    /// of range or the record does not fit.
    pub fn wave_range(&self, i: usize) -> Option<(usize, usize)> {
        let ch = self.chunks.get(i)?;
        if !ch.fits {
            return None;
        }
        Some((ch.off as usize, (ch.off + ch.size) as usize))
    }

    /// Exhaustive-layout check: every record fits and max(off + size)
    /// equals len exactly (true for all five shipped banks).
    pub fn exhaustive(&self, len: usize) -> bool {
        let mut max_end = 0u64;
        for c in &self.chunks {
            if !c.fits {
                return false;
            }
            max_end = max_end.max(c.off as u64 + c.size as u64);
        }
        max_end == len as u64
    }
}

/// Parse an .MRW instrument bank: u16 count then count entries of
/// (off u32, size u32). Chunk payloads stay in the source buffer (the CLI
/// wraps them into WAV).
pub fn parse_mrw(data: &[u8]) -> Result<Mrw, AssetsError> {
    if data.len() < 10 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let count = u16le(data, 0) as usize;
    if 2 + count * 8 > data.len() {
        return Err(AssetsError::CountOverruns {
            count,
            len: data.len(),
        });
    }
    let mut chunks = Vec::with_capacity(count);
    for i in 0..count {
        let b = 2 + i * 8;
        let off = crate::u32le(data, b);
        let size = crate::u32le(data, b + 4);
        let fits = off as usize + size as usize <= data.len();
        chunks.push(MrwChunk { off, size, fits });
    }
    Ok(Mrw { count, chunks })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_mrs(w1: usize, variants: &[u16], a: &[u16], b: &[u16], streams: &[&[u8]]) -> Vec<u8> {
        let w0 = streams.len();
        let mut v = Vec::new();
        v.extend_from_slice(&(w0 as u16).to_le_bytes());
        v.extend_from_slice(&(w1 as u16).to_le_bytes());
        for s in streams {
            v.extend_from_slice(&(s.len() as u16).to_le_bytes());
        }
        for x in variants {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in a {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in b {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in a {
            // table C: distinct filler derived from table A
            v.extend_from_slice(&(x.wrapping_add(0x1234)).to_le_bytes());
        }
        for s in streams {
            v.extend_from_slice(s);
        }
        v
    }

    #[test]
    fn ratio_table_anchors() {
        assert_eq!(RATIO_TABLE[0x54], 0x10000, "1.0 at 0x54");
        assert_eq!(RATIO_TABLE[0x53], 0xf1a0, "one semitone below");
        assert_eq!(RATIO_TABLE[0x66], 0x2d410, "+18 semitone ceiling");
        assert_eq!(RATIO_TABLE[0x7F], 0x2d410, "ceiling repeated to end");
        assert_eq!(RATIO_TABLE[0x18], 0x800, "-60 semitone floor");
        assert_eq!(RATIO_TABLE[0x17], 0, "zero below 0x18");
        // monotone nondecreasing over the whole ladder
        for i in 1..RATIO_TABLE.len() {
            assert!(RATIO_TABLE[i] >= RATIO_TABLE[i - 1], "idx {i:#x}");
        }
    }

    #[test]
    fn note_events_variant0_and_variant1() {
        // delta 5, note byte 0x03 (inst 3 var0), volume 0x20; then freeze
        let stream = [0x05, 0x00, 0x03, 0x20, 0xff, 0xff];
        let (ev, end) = walk_mrs_chunk(&stream, 0, 0);
        assert_eq!(
            ev,
            vec![MrsEvent::Note {
                delta: 5,
                byte: 0x03,
                volume: 0x20,
                instrument: 3,
                ratio: 0x10000,
                tag: 0
            }]
        );
        assert_eq!(end, MrsWalkEnd::Freeze { at: 4 });
        // variant 1: instrument 1+7=8, ratio from table, tag = byte-0x54
        let (ev, _) = walk_mrs_chunk(&stream, 0, 1);
        assert_eq!(
            ev[0],
            MrsEvent::Note {
                delta: 5,
                byte: 0x03,
                volume: 0x20,
                instrument: 8,
                ratio: 0,
                tag: 0x03 - 0x54
            }
        );
        // note-off volume marker is preserved verbatim
        let off = [0x00, 0x00, 0x30, 0xff, 0xff, 0xff];
        let (ev, _) = walk_mrs_chunk(&off, 0, 1);
        match &ev[0] {
            MrsEvent::Note { volume, .. } => assert_eq!(*volume, 0xff),
            other => panic!("expected Note, got {other:?}"),
        }
    }

    #[test]
    fn rest_songend_restart_terminals() {
        let rest = [0x07, 0x00, 0x80, 0x00, 0xff, 0xff];
        let (ev, end) = walk_mrs_chunk(&rest, 0, 0);
        assert_eq!(ev, vec![MrsEvent::Rest { delta: 7 }]);
        assert_eq!(end, MrsWalkEnd::Freeze { at: 4 });
        let se = [0x00, 0x00, 0x7f, 0x00];
        let (ev, end) = walk_mrs_chunk(&se, 0, 0);
        assert_eq!(ev, vec![MrsEvent::SongEnd { delta: 0 }]);
        assert_eq!(end, MrsWalkEnd::SongEnd { at: 4 });
        let rs = [0x0b, 0x00, 0xff, 0x02, 0x00, 0x00];
        let (ev, end) = walk_mrs_chunk(&rs, 0, 0);
        assert_eq!(
            ev,
            vec![MrsEvent::Restart {
                delta: 11,
                chan: 2,
                conditional: false
            }]
        );
        assert_eq!(end, MrsWalkEnd::Restart { at: 4 });
        let rc = [0x00, 0x00, 0xfe, 0x01, 0x00, 0x00];
        let (ev, _) = walk_mrs_chunk(&rc, 0, 0);
        assert!(matches!(
            ev[0],
            MrsEvent::Restart {
                conditional: true,
                ..
            }
        ));
    }

    #[test]
    fn eof_truncated_and_start_offset() {
        // exact EOF without freeze word
        let s = [0x00, 0x00, 0x10, 0x20];
        let (ev, end) = walk_mrs_chunk(&s, 0, 0);
        assert_eq!(ev.len(), 1);
        assert_eq!(end, MrsWalkEnd::Eof { at: 4 });
        // truncated note (volume byte missing)
        let t = [0x00, 0x00, 0x10];
        let (_, end) = walk_mrs_chunk(&t, 0, 0);
        assert_eq!(end, MrsWalkEnd::Truncated { at: 0 });
        // nonzero start offset skips leading bytes
        let s2 = [0xaa, 0xbb, 0x01, 0x00, 0x33, 0x22, 0xff, 0xff];
        let (ev, end) = walk_mrs_chunk(&s2, 2, 0);
        assert_eq!(
            ev,
            vec![MrsEvent::Note {
                delta: 1,
                byte: 0x33,
                volume: 0x22,
                instrument: 0x33,
                ratio: 0x10000,
                tag: 0
            }]
        );
        assert_eq!(end, MrsWalkEnd::Freeze { at: 6 });
        // start beyond the stream truncates
        let (_, end) = walk_mrs_chunk(&s2, 9, 0);
        assert_eq!(end, MrsWalkEnd::Truncated { at: 8 });
    }

    #[test]
    fn rewind_path_terminates() {
        // delta 30001 at pos 0 -> pos -= 30001*4 - 0x1d4be = 6 -> negative
        let mut s = vec![0u8; 64];
        s[0..2].copy_from_slice(&30001i16.to_le_bytes());
        s[2] = 0x10;
        s[3] = 0x20;
        let (ev, end) = walk_mrs_chunk(&s, 0, 0);
        assert!(ev.is_empty());
        assert_eq!(end, MrsWalkEnd::Truncated { at: 0 });
        // a bounce that lands before the stream also terminates
        let mut b = vec![0u8; 16];
        b[6..8].copy_from_slice(&30003i16.to_le_bytes());
        let (_, end) = walk_mrs_chunk(&b, 6, 0);
        assert!(matches!(end, MrsWalkEnd::Truncated { .. }));
    }

    #[test]
    fn container_parse_rebuild_and_errors() {
        let streams: [&[u8]; 3] = [
            &[0xff, 0xff],
            &[0x4b, 0x01, 0xff, 0x00, 0xff, 0xff],
            &[0x00, 0x00, 0x05, 0x20, 0xff, 0xff],
        ];
        let d = build_mrs(1, &[0, 0, 0], &[0xffff, 0, 0], &[0, 331, 0], &streams);
        let m = parse_mrs(&d).unwrap();
        assert_eq!(m.chunk_count, 3);
        assert_eq!(m.chan_count, 1);
        assert!(m.is_disabled(0));
        assert_eq!(m.song_len_ticks(), Some(331));
        assert_eq!(m.song_len_ms(), Some(3310));
        assert_eq!(m.to_bytes(), d);
        // chunk 1 walks to the loop restart, chunk 2 to a freeze
        let (ev, end) = m.walk(1).unwrap();
        assert!(matches!(ev[0], MrsEvent::Restart { chan: 0, .. }));
        assert_eq!(end, MrsWalkEnd::Restart { at: 4 });
        let (ev, end) = m.walk(2).unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(end, MrsWalkEnd::Freeze { at: 4 });
        assert!(m.walk(0).is_none());
        // trailing garbage fails the exact-size rule
        let mut bad = d.clone();
        bad.push(0);
        assert_eq!(
            parse_mrs(&bad),
            Err(AssetsError::MrsLayout {
                len: bad.len(),
                expected: d.len()
            })
        );
        assert_eq!(parse_mrs(&[0, 3]), Err(AssetsError::TooSmall { len: 2 }));
        // oversized W0 overruns the header
        let mut over = Vec::new();
        over.extend_from_slice(&1000u16.to_le_bytes());
        over.extend_from_slice(&1u16.to_le_bytes());
        over.resize(64, 0);
        assert_eq!(
            parse_mrs(&over),
            Err(AssetsError::CountOverruns {
                count: 1000,
                len: 64
            })
        );
    }

    #[test]
    fn mrw_directory_and_wave_access() {
        // two records: directory = bytes 2..18, payload after it
        let mut d = 2u16.to_le_bytes().to_vec();
        d.extend_from_slice(&18u32.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&100u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&[0xAA; 4]);
        let m = parse_mrw(&d).unwrap();
        assert_eq!(m.count, 2);
        assert_eq!((m.chunks[0].off, m.chunks[0].size), (18, 4));
        assert_eq!(m.wave_range(0), Some((18, 22)));
        assert_eq!(&d[18..22], &[0xAA; 4][..]);
        assert!(m.wave_range(1).is_none());
        assert!(m.wave_range(2).is_none());
        assert!(!m.exhaustive(d.len()));
        // exhaustive when max end == len (dir = 2+8 bytes, payload 3)
        let mut e = 1u16.to_le_bytes().to_vec();
        e.extend_from_slice(&10u32.to_le_bytes());
        e.extend_from_slice(&3u32.to_le_bytes());
        e.extend_from_slice(&[1, 2, 3]);
        let m2 = parse_mrw(&e).unwrap();
        assert!(m2.exhaustive(e.len()));
        assert_eq!(parse_mrw(&[0u8; 9]), Err(AssetsError::TooSmall { len: 9 }));
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 0x00BE_D1A4u64;
        let mut next = move || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 3, 4, 5, 8, 34, 100, 104, 256, 1000] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_mrs(&d);
            let _ = parse_mrw(&d);
            let start = (next() as usize) % 300;
            let variant = (next() as u16) % 8;
            let _ = walk_mrs_chunk(&d, start, variant);
        }
    }
}
