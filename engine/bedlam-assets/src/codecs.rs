//! Pixel codecs shared by BIN sprite banks (word RLE) and CGR tile banks
//! (byte RLE), plus the raw fallback and the canonical encoders.

use crate::CodecError;

/// BIN sprite codec: big-endian word stream.
/// - word with 0x8000 set: skip `(word & 0x0FFF)` pixels (leave as 0)
/// - otherwise: literal `(word & 0x0FFF)` bytes follow
/// - word with 0x4000 set: end of row
///
/// Output is `w*h` bytes, zero-initialized. `guard` bounds total word count so
/// hostile streams terminate instead of looping.
pub fn decode_rle16(data: &[u8], w: usize, h: usize) -> Result<Vec<u8>, CodecError> {
    let mut out = vec![0u8; w * h];
    let mut p = 0usize;
    let mut guard = 0usize;
    for row in 0..h {
        let mut x = 0usize;
        loop {
            if p + 2 > data.len() || guard > 4_000_000 {
                return Err(CodecError::Rle16Overrun);
            }
            guard += 1;
            let word = u16::from_le_bytes([data[p], data[p + 1]]);
            p += 2;
            if word & 0x8000 != 0 {
                x += (word & 0x0FFF) as usize;
            } else {
                let n = (word & 0x0FFF) as usize;
                if p + n > data.len() {
                    return Err(CodecError::LiteralOverrun);
                }
                for k in 0..n {
                    if x + k < w {
                        out[row * w + x + k] = data[p + k];
                    }
                }
                x += n;
                p += n;
            }
            if word & 0x4000 != 0 {
                break;
            }
        }
    }
    Ok(out)
}

/// CGR 32-wide tile codec: byte stream.
/// - byte with 0x40 set: end of line (next row)
/// - byte with 0x80 set: skip `((b & 0x3F) + 1)` pixels (leave as 0)
/// - otherwise: literal `((b & 0x3F) + 1)` bytes follow
///
/// Succeeds only when at least `h` end-of-line markers were consumed.
pub fn decode_byterle(data: &[u8], w: usize, h: usize) -> Result<Vec<u8>, CodecError> {
    let mut out = vec![0u8; w * h];
    let mut x = 0usize;
    let mut y = 0usize;
    let mut p = 0usize;
    let mut guard = 0usize;
    while y < h && p < data.len() && guard < 1_000_000 {
        guard += 1;
        let b = data[p];
        p += 1;
        if b & 0x40 != 0 {
            y += 1;
            x = 0;
        } else if b & 0x80 != 0 {
            x += (b & 0x3F) as usize + 1;
        } else {
            let n = (b & 0x3F) as usize + 1;
            if p + n > data.len() {
                return Err(CodecError::LiteralOverrun);
            }
            for k in 0..n {
                if x < w && y < h {
                    out[y * w + x] = data[p + k];
                }
                x += 1;
            }
            p += n;
        }
    }
    if y >= h {
        Ok(out)
    } else {
        Err(CodecError::ByterleIncomplete)
    }
}

/// Uncompressed pixel copy; fails when fewer than `w*h` bytes remain.
pub fn decode_raw(data: &[u8], w: usize, h: usize) -> Result<Vec<u8>, CodecError> {
    if w * h > data.len() {
        return Err(CodecError::RawOverrun);
    }
    Ok(data[..w * h].to_vec())
}

fn px_at(pixels: &[u8], w: usize, row: usize, x: usize) -> u8 {
    // Total on any input: out-of-range positions read as 0.
    row.checked_mul(w)
        .and_then(|r| r.checked_add(x))
        .and_then(|i| pixels.get(i).copied())
        .unwrap_or(0)
}

/// Absurd dimension guard for the encoders: anything above this cannot occur in
/// any Bedlam format (dims are i16/u16 bounded) and would only serve to make
/// the encoder allocate unboundedly. Such input yields an empty stream.
const ENCODE_AREA_CAP: usize = 1 << 26;

/// Canonical RLE16 encoder, inverse of [`decode_rle16`].
///
/// Per row: zero runs become skip words, non-zero runs become literals
/// (chunked to 0x0FFF), then one 0x4000 end-of-row word. Total for any input:
/// never panics; `pixels` shorter than `w*h` is treated as zero-padded.
pub fn encode_rle16(w: usize, h: usize, pixels: &[u8]) -> Vec<u8> {
    if w.saturating_mul(h) > ENCODE_AREA_CAP {
        return Vec::new();
    }
    let mut out = Vec::new();
    for row in 0..h {
        let mut x = 0usize;
        while x < w {
            if px_at(pixels, w, row, x) == 0 {
                let mut z = 0usize;
                while x + z < w && px_at(pixels, w, row, x + z) == 0 {
                    z += 1;
                }
                let mut rem = z;
                while rem > 0 {
                    let n = rem.min(0x0FFF);
                    out.extend_from_slice(&(0x8000u16 | n as u16).to_le_bytes());
                    rem -= n;
                }
                x += z;
            } else {
                let mut n = 0usize;
                while x + n < w && px_at(pixels, w, row, x + n) != 0 {
                    n += 1;
                }
                let mut rem = n;
                let mut s = x;
                while rem > 0 {
                    let c = rem.min(0x0FFF);
                    out.extend_from_slice(&(c as u16).to_le_bytes());
                    for k in 0..c {
                        out.push(px_at(pixels, w, row, s + k));
                    }
                    rem -= c;
                    s += c;
                }
                x += n;
            }
        }
        out.extend_from_slice(&0x4000u16.to_le_bytes());
    }
    out
}

/// Canonical byte-RLE encoder, inverse of [`decode_byterle`].
///
/// Per row: zero runs become skip bytes, non-zero runs become literals
/// (chunked to 64), then one 0x40 end-of-line byte. Total for any input.
pub fn encode_byterle(w: usize, h: usize, pixels: &[u8]) -> Vec<u8> {
    if w.saturating_mul(h) > ENCODE_AREA_CAP {
        return Vec::new();
    }
    let mut out = Vec::new();
    for row in 0..h {
        let mut x = 0usize;
        while x < w {
            if px_at(pixels, w, row, x) == 0 {
                let mut z = 0usize;
                while x + z < w && px_at(pixels, w, row, x + z) == 0 {
                    z += 1;
                }
                let mut rem = z;
                while rem > 0 {
                    let n = rem.min(64);
                    out.push(0x80 | (n as u8 - 1));
                    rem -= n;
                }
                x += z;
            } else {
                let mut n = 0usize;
                while x + n < w && px_at(pixels, w, row, x + n) != 0 {
                    n += 1;
                }
                let mut rem = n;
                let mut s = x;
                while rem > 0 {
                    let c = rem.min(64);
                    out.push(c as u8 - 1);
                    for k in 0..c {
                        out.push(px_at(pixels, w, row, s + k));
                    }
                    rem -= c;
                    s += c;
                }
                x += n;
            }
        }
        out.push(0x40);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle16_basic_round_trip() {
        let px: Vec<u8> = vec![1, 2, 3, 0, 0, 0, 4, 0, 5, 5, 5, 5, 0, 0, 0, 0];
        let enc = encode_rle16(4, 4, &px);
        assert_eq!(decode_rle16(&enc, 4, 4).unwrap(), px);
    }

    #[test]
    fn rle16_zero_width_and_height() {
        assert!(encode_rle16(0, 0, &[]).is_empty());
        assert_eq!(decode_rle16(&[], 0, 0).unwrap(), Vec::<u8>::new());
        // 0-width rows still emit (and require) their end-of-row word.
        let enc = encode_rle16(0, 3, &[]);
        assert_eq!(enc.len(), 6);
        assert_eq!(decode_rle16(&enc, 0, 3).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn rle16_all_zero_rows() {
        let px = vec![0u8; 64 * 8];
        let enc = encode_rle16(64, 8, &px);
        // each row: skip 64 + EOL => 4 bytes
        assert_eq!(enc.len(), 8 * 4);
        assert_eq!(decode_rle16(&enc, 64, 8).unwrap(), px);
    }

    #[test]
    fn rle16_max_runs_chunked() {
        let w = 5000usize; // > 0x0FFF per single word
        let mut px = vec![0u8; w * 2];
        for (i, v) in px.iter_mut().enumerate() {
            *v = if i % 2 == 0 { 7 } else { 9 };
        }
        let enc = encode_rle16(w, 2, &px);
        assert_eq!(decode_rle16(&enc, w, 2).unwrap(), px);
    }

    #[test]
    fn rle16_all_max_values_row() {
        let px = vec![255u8; 300];
        let enc = encode_rle16(300, 1, &px);
        assert_eq!(decode_rle16(&enc, 300, 1).unwrap(), px);
    }

    #[test]
    fn rle16_truncated_stream_err() {
        assert_eq!(decode_rle16(&[], 2, 2), Err(CodecError::Rle16Overrun));
        // literal claiming more bytes than remain
        let mut d = vec![0x03u8, 0x00];
        d.push(1);
        assert_eq!(decode_rle16(&d, 4, 1), Err(CodecError::LiteralOverrun));
    }

    #[test]
    fn rle16_decode_word_semantics() {
        // skip 3, literal 2 (bytes A B inline), EOL => row: 0 0 0 A B
        let mut d: Vec<u8> = Vec::new();
        for w in [0x8003u16, 0x0002u16] {
            d.extend_from_slice(&w.to_le_bytes());
        }
        d.extend_from_slice(&[0xAA, 0xBB]);
        d.extend_from_slice(&0x4000u16.to_le_bytes());
        assert_eq!(decode_rle16(&d, 5, 1).unwrap(), vec![0, 0, 0, 0xAA, 0xBB]);
    }

    #[test]
    fn byterle_basic_round_trip() {
        let px: Vec<u8> = vec![9, 0, 0, 8, 8, 0, 7, 7, 7, 0, 0, 1];
        let enc = encode_byterle(4, 3, &px);
        assert_eq!(decode_byterle(&enc, 4, 3).unwrap(), px);
    }

    #[test]
    fn byterle_zero_width_and_height() {
        assert!(encode_byterle(0, 0, &[]).is_empty());
        assert_eq!(decode_byterle(&[], 0, 0).unwrap(), Vec::<u8>::new());
        let enc = encode_byterle(0, 2, &[]);
        assert_eq!(enc, vec![0x40, 0x40]);
        assert_eq!(decode_byterle(&enc, 0, 2).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn byterle_all_zero_rows() {
        let px = vec![0u8; 32 * 64];
        let enc = encode_byterle(32, 64, &px);
        // each row: skip 32 (one byte) + EOL
        assert_eq!(enc.len(), 64 * 2);
        assert_eq!(decode_byterle(&enc, 32, 64).unwrap(), px);
    }

    #[test]
    fn byterle_max_run_chunking() {
        let w = 200usize; // > 64 per chunk
        let mut px = vec![0u8; w * 3];
        for (i, v) in px.iter_mut().enumerate() {
            *v = ((i % 251) + 1) as u8; // all non-zero
        }
        let enc = encode_byterle(w, 3, &px);
        assert_eq!(decode_byterle(&enc, w, 3).unwrap(), px);
    }

    #[test]
    fn byterle_missing_eol_is_incomplete() {
        // one literal of 2 bytes, no EOL, h=1
        assert_eq!(
            decode_byterle(&[0x01, 5, 6], 2, 1),
            Err(CodecError::ByterleIncomplete)
        );
    }

    #[test]
    fn byterle_literal_overrun_err() {
        assert_eq!(
            decode_byterle(&[0x05, 1], 8, 1),
            Err(CodecError::LiteralOverrun)
        );
    }

    #[test]
    fn raw_round_trip_and_err() {
        assert_eq!(decode_raw(&[1, 2, 3, 4], 2, 2).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(decode_raw(&[1], 2, 2), Err(CodecError::RawOverrun));
        assert_eq!(decode_raw(&[], 0, 0).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn encoders_are_total_on_short_buffers() {
        // fewer pixels than w*h: no panic, zero-padded semantics
        let enc = encode_rle16(8, 2, &[1, 2]);
        assert_eq!(decode_rle16(&enc, 8, 2).unwrap()[..2], [1, 2]);
        let enc = encode_byterle(8, 2, &[1, 2]);
        assert_eq!(decode_byterle(&enc, 8, 2).unwrap()[..2], [1, 2]);
        // absurd dims: empty stream, no allocation storm
        assert!(encode_rle16(usize::MAX, 2, &[]).is_empty());
        assert!(encode_byterle(2, usize::MAX, &[]).is_empty());
    }

    /// Deterministic pseudo-random round-trips via a tiny LCG (no rand dep).
    #[test]
    fn fuzz_round_trip_lcg() {
        let mut seed = 0x1234_5678_9abc_def0u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as u8
        };
        for _ in 0..64 {
            let w = (next() as usize % 40) + 1;
            let h = (next() as usize % 12) + 1;
            let sparse = next() % 4 == 0; // exercise long zero runs vs dense literals
            let px: Vec<u8> = (0..w * h)
                .map(|_| {
                    if sparse && next() % 8 != 0 {
                        0
                    } else {
                        next().max(1) // keep literals non-zero in sparse mode
                    }
                })
                .collect();
            let enc16 = encode_rle16(w, h, &px);
            assert_eq!(decode_rle16(&enc16, w, h).unwrap(), px, "rle16 {w}x{h}");
            let encb = encode_byterle(w, h, &px);
            assert_eq!(decode_byterle(&encb, w, h).unwrap(), px, "byterle {w}x{h}");
        }
    }
}
