//! .MRS music score container + event stream decoder.
//!
//! Format: docs/RE-EXW-MUSIC.md sections 2 and 2b. The grammar there was
//! verified byte-exact against all 5 shipped files in BEDLAM/SOUND/MIDI.
//! One event = u16 LE delta (10 ms ticks at the 100 Hz pump) + opcode byte
//! + 1 argument byte. See [MrsEvent] for the opcode table.

use crate::{i16le, u16le, AssetsError, CodecError};

/// Resample ratio table @EXW VA 0x00454174 (DGROUP, file offset 0x52774 in
/// BEDLAM.EXW), u32 16.16 fixed point, indexed by the variant-1 note byte.
/// Equal tempered semitone steps relative to 0x54 = 1.0 (0x18 = 0x800 =
/// 2^-5 exactly), clamped to 0 below 0x18 and to 0x2d410 (+18 semitones)
/// at and above 0x66. The game converts it to a DirectSound frequency as
/// (ratio * 11025) >> 16 in SubVoiceStart (FUN_0044c4a8). [EXW, byte-exact]
pub const RATIO_TABLE: [u32; 0x7F] = [
    0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000,
    0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000, 0x00000,
    0x00000, 0x00000, 0x00000, 0x00000, 0x00800, 0x00879, 0x008fa, 0x00983, 0x00a14, 0x00aad,
    0x00b50, 0x00bfc, 0x00cb2, 0x00d74, 0x00e41, 0x00f1a, 0x01000, 0x010f3, 0x011f5, 0x01306,
    0x01428, 0x0155b, 0x016a0, 0x017f8, 0x01965, 0x01ae8, 0x01c82, 0x01e33, 0x01fff, 0x021e7,
    0x023eb, 0x0260d, 0x02850, 0x02ab6, 0x02d40, 0x02ff1, 0x032cb, 0x035d1, 0x03904, 0x03c68,
    0x03fff, 0x043cd, 0x047d6, 0x04c1b, 0x050a1, 0x0556d, 0x05a82, 0x05fe3, 0x06595, 0x06ba2,
    0x07208, 0x078d0, 0x07ffe, 0x0879b, 0x08fab, 0x09836, 0x0a143, 0x0aada, 0x0b502, 0x0bfc5,
    0x0cb2e, 0x0d744, 0x0e410, 0x0f1a0, 0x10000, 0x10f37, 0x11f57, 0x1306f, 0x14289, 0x155b7,
    0x16a07, 0x17f8e, 0x1965f, 0x1ae88, 0x1c820, 0x1e340, 0x1fffd, 0x21e70, 0x23eb0, 0x260dc,
    0x28512, 0x2ab6e, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410,
    0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410,
    0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410, 0x2d410,
];

/// Loop/timing chunk index. Data convention (all 5 shipped files): chunk 0
/// is a 2-byte disabled stub and chunk 1 is the 6-byte loop timer whose
/// initial tick delay equals the song length in 10 ms ticks.
pub const LOOP_CHUNK: usize = 1;

/// Start-offset table value marking a disabled chunk.
pub const CHUNK_DISABLED: u16 = 0xFFFF;

/// Parsed .MRS container header. Chunk event data stays in the source
/// buffer; use [Mrs::chunk_range] to slice it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mrs {
    /// W0: chunk count.
    pub chunk_count: usize,
    /// W1: channel count (1 in every shipped file; W1 > 1 layout unverified).
    pub channel_count: usize,
    /// Chunk data sizes in bytes (W0 entries).
    pub sizes: Vec<u16>,
    /// Per-chunk variant words: 0 = variant-0 (event byte = instrument),
    /// else variant-1 (instrument = variant + 7, event byte = note selector).
    pub variants: Vec<u16>,
    /// Start-offset table (W0*W1): byte offset of the first event within
    /// the chunk data block; 0xFFFF = chunk disabled.
    pub start_offsets: Vec<u16>,
    /// Initial tick delay table (W0*W1): overrides the first event delta.
    pub tick_delays: Vec<u16>,
    /// Table C (W0*W1): written by load_midi, read by nothing in EXW
    /// (write-only dead data; kept for rebuild/inspection completeness).
    pub table_c: Vec<u16>,
    /// Byte offset of the first chunk data block.
    pub data_off: usize,
}

impl Mrs {
    /// Byte range of chunk event data within the source file.
    pub fn chunk_range(&self, i: usize) -> Option<(usize, usize)> {
        if i >= self.chunk_count {
            return None;
        }
        let start = self.data_off + self.sizes[..i].iter().map(|s| *s as usize).sum::<usize>();
        Some((start, start + self.sizes[i] as usize))
    }

    /// Song length in 10 ms ticks = loop chunk initial tick delay
    /// ([LOOP_CHUNK]); None when the file has no loop chunk.
    pub fn song_length_ticks(&self) -> Option<u16> {
        self.tick_delays.get(LOOP_CHUNK).copied()
    }
}

/// Parse an .MRS container. Fails unless the file size equals
/// data_off + sum(sizes) exactly (holds for every shipped file).
pub fn parse_mrs(data: &[u8]) -> Result<Mrs, AssetsError> {
    if data.len() < 4 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let w0 = u16le(data, 0) as usize;
    let w1 = u16le(data, 2) as usize;
    let data_off = 4 + 4 * w0 + 6 * w1 * w0;
    if data_off > data.len() {
        return Err(AssetsError::CountOverruns {
            count: w0,
            len: data.len(),
        });
    }
    let words =
        |n: usize, base: usize| -> Vec<u16> { (0..n).map(|i| u16le(data, base + 2 * i)).collect() };
    let sizes = words(w0, 4);
    let variants = words(w0, 4 + 2 * w0);
    let n = w0 * w1;
    let start_offsets = words(n, 4 + 4 * w0);
    let tick_delays = words(n, 4 + 4 * w0 + 2 * n);
    let table_c = words(n, 4 + 4 * w0 + 4 * n);
    let sum: usize = sizes.iter().map(|s| *s as usize).sum();
    if data_off + sum != data.len() {
        return Err(AssetsError::ContainerSize {
            data_off,
            sum,
            len: data.len(),
        });
    }
    Ok(Mrs {
        chunk_count: w0,
        channel_count: w1,
        sizes,
        variants,
        start_offsets,
        tick_delays,
        table_c,
        data_off,
    })
}

/// A NOTE-ON (volume != 0xFF) or NOTE-OFF (volume == 0xFF) event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MrsNote {
    /// True for variant-1 chunks (instrument = variant + 7, note byte
    /// selects the ratio table entry).
    pub variant1: bool,
    /// Variant 0: the event byte itself. Variant 1: variant word + 7.
    pub inst: u8,
    /// 16.16 resample ratio: RATIO_TABLE[note byte] for variant 1,
    /// 0x10000 (1.0) for variant 0.
    pub ratio: u32,
    /// Variant 1: note byte - 0x54 (the game stores this in 0045b044 and
    /// releases the matching sub-voice on note-off). Variant 0: the raw
    /// event byte (note-off never appears in variant-0 shipped chunks).
    pub note_tag: i16,
    /// Volume byte (observed 9..42); 0xFF = NOTE-OFF release.
    pub volume: u8,
}

impl MrsNote {
    /// Volume byte 0xFF: release the sub-voice whose note tag matches.
    pub fn is_note_off(&self) -> bool {
        self.volume == 0xFF
    }

    /// DirectSound playback frequency, as SubVoiceStart computes it:
    /// (ratio * 11025) >> 16.
    pub fn frequency_hz(&self) -> u32 {
        ((self.ratio as u64 * 11025) >> 16) as u32
    }
}

/// One decoded event (the +1 argument byte is consumed for every opcode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MrsEvent {
    Note(MrsNote),
    /// 0x7F: copies chunk state to the shadow song slot and stops it
    /// (unused by shipped data).
    SongEnd,
    /// 0x80..=0xFD: idle gate; the pump skips (unused by shipped data).
    Rest,
    /// 0xFE (conditional: only with the never-set loop flag; dead in EXW)
    /// / 0xFF (unconditional): MrsChunkStart re-inits all chunks of
    /// `channel` from the header tables.
    Restart {
        conditional: bool,
        channel: u8,
    },
}

/// An event plus the 10 ms tick countdown consumed before it fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedEvent {
    pub delta: u16,
    pub event: MrsEvent,
}

/// Why the walk stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStop {
    /// The stream ended exactly on an event boundary.
    EndExact,
    /// A negative (0xFFxx) delta word: the pump never decrements it, so the
    /// chunk stalls forever (natural stop; every shipped stream ends this
    /// way or exact).
    Freeze,
    /// A 0x7F song-end opcode.
    SongEnd,
}

/// Result of walking one chunk event stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedStream {
    pub events: Vec<TimedEvent>,
    /// Bytes consumed, including the terminating freeze word / last event.
    pub consumed: usize,
    pub stop: StreamStop,
    /// Sum of event deltas (the initial wait uses the header tick delay).
    pub delta_sum: u32,
}

/// Decode one chunk event stream with the MrsNextEvent (FUN_00402e74) walk:
/// u16 LE delta + opcode + argument, repeated. Stops on exact end, freeze
/// word, or song-end. Deltas 30001..=32767 reposition the read position
/// backward by delta*4 - 0x1d4be and re-read (loop-back encoding, unused by
/// shipped data; a bounded reposition count guards against bad streams).
pub fn decode_stream(stream: &[u8], variant: u16) -> Result<DecodedStream, AssetsError> {
    let mut pos = 0usize;
    let mut events = Vec::new();
    let mut delta_sum = 0u32;
    let stop;
    let mut repositions = 0usize;
    loop {
        if pos == stream.len() {
            stop = StreamStop::EndExact;
            break;
        }
        if pos + 2 > stream.len() {
            return Err(CodecError::MrsEventTruncated.into());
        }
        let delta = i16le(stream, pos);
        if delta < 0 {
            stop = StreamStop::Freeze;
            pos += 2;
            break;
        }
        if delta > 30000 {
            repositions += 1;
            if repositions > 4096 {
                return Err(CodecError::MrsRepositionLoop.into());
            }
            let new_pos = pos as i64 - delta as i64 * 4 + 0x1D4BE;
            if new_pos < 0 || new_pos as usize >= stream.len() {
                return Err(CodecError::MrsEventTruncated.into());
            }
            pos = new_pos as usize;
            continue;
        }
        if pos + 4 > stream.len() {
            return Err(CodecError::MrsEventTruncated.into());
        }
        let b = stream[pos + 2];
        let arg = stream[pos + 3];
        let event = match b {
            0x00..=0x7E => {
                if variant == 0 {
                    MrsEvent::Note(MrsNote {
                        variant1: false,
                        inst: b,
                        ratio: 0x10000,
                        note_tag: b as i16,
                        volume: arg,
                    })
                } else {
                    MrsEvent::Note(MrsNote {
                        variant1: true,
                        inst: (variant + 7) as u8,
                        ratio: RATIO_TABLE[b as usize],
                        note_tag: b as i16 - 0x54,
                        volume: arg,
                    })
                }
            }
            0x7F => MrsEvent::SongEnd,
            0x80..=0xFD => MrsEvent::Rest,
            0xFE => MrsEvent::Restart {
                conditional: true,
                channel: arg,
            },
            0xFF => MrsEvent::Restart {
                conditional: false,
                channel: arg,
            },
        };
        events.push(TimedEvent {
            delta: delta as u16,
            event,
        });
        delta_sum += delta as u32;
        pos += 4;
        if let MrsEvent::SongEnd = event {
            stop = StreamStop::SongEnd;
            break;
        }
    }
    Ok(DecodedStream {
        events,
        consumed: pos,
        stop,
        delta_sum,
    })
}

/// Parse + decode chunk `chunk` of an .MRS file in one step. Returns Ok(None)
/// when the chunk is disabled (start offset 0xFFFF) or out of range.
pub fn decode_mrs_chunk(
    data: &[u8],
    mrs: &Mrs,
    chunk: usize,
) -> Result<Option<DecodedStream>, AssetsError> {
    let Some((a, b)) = mrs.chunk_range(chunk) else {
        return Ok(None);
    };
    let start = mrs.start_offsets[chunk];
    if start == CHUNK_DISABLED {
        return Ok(None);
    }
    let start = start as usize;
    if start >= b - a {
        return Err(CodecError::MrsEventTruncated.into());
    }
    decode_stream(&data[a + start..b], mrs.variants[chunk]).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic W1=1 container from header words + chunk blocks.
    fn build_mrs(
        sizes: &[u16],
        variants: &[u16],
        start: &[u16],
        ticks: &[u16],
        tablec: &[u16],
        chunk_bytes: &[&[u8]],
    ) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        for s in sizes {
            v.extend_from_slice(&s.to_le_bytes());
        }
        for x in variants {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in start {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in ticks {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for x in tablec {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for c in chunk_bytes {
            v.extend_from_slice(c);
        }
        v
    }

    fn be16(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }

    #[test]
    fn ratio_table_anchors() {
        assert_eq!(RATIO_TABLE.len(), 0x7F);
        assert_eq!(RATIO_TABLE[0], 0);
        assert_eq!(RATIO_TABLE[0x17], 0);
        assert_eq!(RATIO_TABLE[0x18], 0x800); // 2^-5 exactly
        assert_eq!(RATIO_TABLE[0x54], 0x10000); // 1.0
        assert_eq!(RATIO_TABLE[0x55], 0x10f37); // +1 semitone
        assert_eq!(RATIO_TABLE[0x66], 0x2d410); // +18 st ceiling
        assert_eq!(RATIO_TABLE[0x7E], 0x2d410); // clamped above
        for w in RATIO_TABLE.windows(2).skip(0x18).take(0x66 - 0x18) {
            assert!(w[0] < w[1], "table must rise over 0x18..=0x66");
        }
    }

    #[test]
    fn frequency_math_matches_subvoicestart() {
        let n = MrsNote {
            variant1: true,
            inst: 8,
            ratio: 0x10000,
            note_tag: 0,
            volume: 0x25,
        };
        assert_eq!(n.frequency_hz(), 11025);
        let hi = MrsNote {
            ratio: 0x2d410,
            ..n
        };
        assert_eq!(hi.frequency_hz(), 31182); // (185360 * 11025) >> 16
        assert!(!hi.is_note_off());
    }

    #[test]
    fn parse_synthetic_brief_like() {
        let c0 = [0xFFu8, 0xFF];
        let c1 = [0x4Bu8, 0x01, 0xFF, 0x00, 0xFF, 0xFF];
        let c2 = [
            0x00u8, 0x00, 0x54, 0x25, 0x0B, 0x00, 0x54, 0xFF, 0x00, 0x00, 0x55, 0x2A, 0xFF, 0xFF,
        ];
        let d = build_mrs(
            &[2, 6, 14],
            &[0, 0, 1],
            &[CHUNK_DISABLED, 0, 0],
            &[0, 331, 0],
            &[0, 1, 2],
            &[&c0, &c1, &c2],
        );
        assert_eq!(d.len(), 34 + 2 + 6 + 14);
        let m = parse_mrs(&d).unwrap();
        assert_eq!(m.chunk_count, 3);
        assert_eq!(m.channel_count, 1);
        assert_eq!(m.data_off, 34);
        assert_eq!(m.sizes, vec![2, 6, 14]);
        assert_eq!(m.variants, vec![0, 0, 1]);
        assert_eq!(m.start_offsets, vec![CHUNK_DISABLED, 0, 0]);
        assert_eq!(m.tick_delays, vec![0, 331, 0]);
        assert_eq!(m.table_c, vec![0, 1, 2]);
        assert_eq!(m.chunk_range(0), Some((34, 36)));
        assert_eq!(m.chunk_range(1), Some((36, 42)));
        assert_eq!(m.chunk_range(2), Some((42, 56)));
        assert_eq!(m.chunk_range(3), None);
        assert_eq!(m.song_length_ticks(), Some(331));

        // chunk 0: disabled -> Ok(None)
        assert_eq!(decode_mrs_chunk(&d, &m, 0).unwrap(), None);
        // out of range -> Ok(None)
        assert_eq!(decode_mrs_chunk(&d, &m, 9).unwrap(), None);
    }

    #[test]
    fn decode_loop_chunk_restart_then_freeze() {
        let c1 = [0x4Bu8, 0x01, 0xFF, 0x00, 0xFF, 0xFF];
        let s = decode_stream(&c1, 0).unwrap();
        assert_eq!(
            s.events,
            vec![TimedEvent {
                delta: 331,
                event: MrsEvent::Restart {
                    conditional: false,
                    channel: 0
                }
            }]
        );
        assert_eq!(s.stop, StreamStop::Freeze);
        assert_eq!(s.consumed, 6);
        assert_eq!(s.delta_sum, 331);
    }

    #[test]
    fn decode_variant1_melody() {
        let c2 = [
            0x00u8, 0x00, 0x54, 0x25, 0x0B, 0x00, 0x54, 0xFF, 0x00, 0x00, 0x55, 0x2A, 0xFF, 0xFF,
        ];
        let s = decode_stream(&c2, 1).unwrap();
        assert_eq!(s.consumed, c2.len());
        assert_eq!(s.stop, StreamStop::Freeze);
        assert_eq!(s.delta_sum, 11);
        assert_eq!(s.events.len(), 3);
        match &s.events[0].event {
            MrsEvent::Note(n) => {
                assert!(n.variant1);
                assert_eq!(n.inst, 8); // variant 1 + 7
                assert_eq!(n.ratio, 0x10000);
                assert_eq!(n.note_tag, 0);
                assert_eq!(n.volume, 0x25);
                assert!(!n.is_note_off());
            }
            other => panic!("expected Note, got {other:?}"),
        }
        match &s.events[1].event {
            MrsEvent::Note(n) => {
                assert!(n.is_note_off());
                assert_eq!(n.note_tag, 0);
            }
            other => panic!("expected Note, got {other:?}"),
        }
        match &s.events[2].event {
            MrsEvent::Note(n) => {
                assert_eq!(n.inst, 8);
                assert_eq!(n.ratio, RATIO_TABLE[0x55]);
                assert_eq!(n.note_tag, 1);
                assert_eq!(n.volume, 0x2A);
            }
            other => panic!("expected Note, got {other:?}"),
        }
    }

    #[test]
    fn decode_variant0_melody() {
        // delta 0, inst 1, vol 0x23; delta 46, inst 0, vol 0x21; freeze
        let mut c = Vec::new();
        c.extend_from_slice(&be16(0));
        c.extend_from_slice(&[0x01, 0x23]);
        c.extend_from_slice(&be16(46));
        c.extend_from_slice(&[0x00, 0x21]);
        c.extend_from_slice(&be16(0xFFFF));
        let s = decode_stream(&c, 0).unwrap();
        assert_eq!(s.stop, StreamStop::Freeze);
        assert_eq!(s.consumed, 10);
        assert_eq!(s.delta_sum, 46);
        match &s.events[0].event {
            MrsEvent::Note(n) => {
                assert!(!n.variant1);
                assert_eq!(n.inst, 1);
                assert_eq!(n.ratio, 0x10000);
                assert_eq!(n.volume, 0x23);
            }
            other => panic!("expected Note, got {other:?}"),
        }
    }

    #[test]
    fn rest_and_song_end_paths() {
        // rest (0x80) then song end (0x7F)
        let mut c = Vec::new();
        c.extend_from_slice(&be16(1));
        c.extend_from_slice(&[0x80, 0x00]);
        c.extend_from_slice(&be16(0));
        c.extend_from_slice(&[0x7F, 0x00]);
        let s = decode_stream(&c, 0).unwrap();
        assert_eq!(s.stop, StreamStop::SongEnd);
        assert_eq!(s.consumed, 8);
        assert!(matches!(s.events[0].event, MrsEvent::Rest));
        assert!(matches!(s.events[1].event, MrsEvent::SongEnd));

        // exact end, no freeze word
        let e = [0x00u8, 0x00, 0x54, 0x25];
        let s2 = decode_stream(&e, 1).unwrap();
        assert_eq!(s2.stop, StreamStop::EndExact);
        assert_eq!(s2.consumed, 4);

        // conditional restart 0xFE on channel 2
        let mut r = Vec::new();
        r.extend_from_slice(&be16(5));
        r.extend_from_slice(&[0xFE, 0x02]);
        r.extend_from_slice(&be16(0xFFFF));
        let s3 = decode_stream(&r, 0).unwrap();
        assert_eq!(
            s3.events[0].event,
            MrsEvent::Restart {
                conditional: true,
                channel: 2
            }
        );
        assert_eq!(s3.stop, StreamStop::Freeze);
    }

    #[test]
    fn reposition_paths() {
        // 30001 word at pos 4 jumps to 4 - 6 = -2 -> error
        let bad = [0x00u8, 0x00, 0x00, 0x00, 0x31, 0x75];
        assert_eq!(
            decode_stream(&bad, 0),
            Err(AssetsError::Codec(CodecError::MrsEventTruncated))
        );

        // 30001 word at pos 8 jumps back to pos 2 and re-reads there:
        // events fire at 0, 4, then (after jump) at 2 and 6, freeze at 10.
        let lp = [
            0x00u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, 0x75, 0xFF, 0xFF,
        ];
        let s = decode_stream(&lp, 0).unwrap();
        assert_eq!(s.stop, StreamStop::Freeze);
        assert_eq!(s.consumed, 12);
        assert_eq!(s.events.len(), 4);
        assert_eq!(s.events[3].delta, 0);
        match s.events[3].event {
            MrsEvent::Note(n) => assert_eq!(n.inst, 0x31),
            other => panic!("expected Note, got {other:?}"),
        }
    }

    #[test]
    fn container_and_stream_errors() {
        assert_eq!(
            parse_mrs(&[0u8, 0, 0]),
            Err(AssetsError::TooSmall { len: 3 })
        );
        // W0=5 needs data_off = 4 + 20 + 30 = 54
        let mut big = Vec::new();
        big.extend_from_slice(&be16(5));
        big.extend_from_slice(&be16(1));
        big.resize(20, 0);
        assert_eq!(
            parse_mrs(&big),
            Err(AssetsError::CountOverruns { count: 5, len: 20 })
        );
        // size formula violated: chop one byte off a valid container
        let c0 = [0xFFu8, 0xFF];
        let mut d = build_mrs(&[2], &[0], &[CHUNK_DISABLED], &[0], &[0], &[&c0]);
        let full_len = d.len();
        assert_eq!(parse_mrs(&d).map(|m| m.chunk_count), Ok(1));
        d.truncate(full_len - 1);
        assert_eq!(
            parse_mrs(&d),
            Err(AssetsError::ContainerSize {
                data_off: 4 + 4 + 6,
                sum: 2,
                len: full_len - 1,
            })
        );
        // truncated event
        assert_eq!(
            decode_stream(&[0x00, 0x00, 0x54], 1),
            Err(AssetsError::Codec(CodecError::MrsEventTruncated))
        );
        assert_eq!(
            decode_stream(&[0x00, 0x00], 0),
            Err(AssetsError::Codec(CodecError::MrsEventTruncated))
        );
        // lone freeze word: valid, no events
        let s = decode_stream(&[0xFF, 0xFF], 0).unwrap();
        assert_eq!(s.stop, StreamStop::Freeze);
        assert!(s.events.is_empty());
        assert_eq!(s.consumed, 2);
    }

    #[test]
    fn start_offset_beyond_chunk_is_an_error() {
        let blk = [0u8; 4];
        let d = build_mrs(&[4], &[0], &[5], &[0], &[0], &[&blk]);
        let m = parse_mrs(&d).unwrap();
        assert_eq!(
            decode_mrs_chunk(&d, &m, 0),
            Err(AssetsError::Codec(CodecError::MrsEventTruncated))
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 0x5EEDu64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 2, 3, 4, 6, 8, 34, 100, 3048] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_mrs(&d);
            for variant in [0u16, 1, 3] {
                let _ = decode_stream(&d, variant);
            }
            if let Ok(m) = parse_mrs(&d) {
                for i in 0..m.chunk_count + 1 {
                    let _ = decode_mrs_chunk(&d, &m, i);
                }
            }
        }
    }
}
