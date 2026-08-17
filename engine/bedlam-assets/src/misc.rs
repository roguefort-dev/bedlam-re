//! Miscellaneous bank formats: .min tile colors, .lnk/.lng remaps, .mrw
//! instrument chunks, .nme name tables, .bdg badge/badge-art records, and the
//! ascii-percentage helper for text files.

use crate::{hex_head, i16le, u16le, u32le, AssetsError};

/// .min file: `n*16` bytes of per-tile color data. Kept raw (the only parsed
/// fact is the size divisibility and the tile count).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinFile {
    pub bytes: Vec<u8>,
}

impl MinFile {
    pub fn tile_count(&self) -> usize {
        self.bytes.len() / 16
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// Parse a .min file (must be a multiple of 16 bytes).
pub fn parse_min(data: &[u8]) -> Result<MinFile, AssetsError> {
    if !data.len().is_multiple_of(16) {
        return Err(AssetsError::NotMultiple { len: data.len() });
    }
    Ok(MinFile {
        bytes: data.to_vec(),
    })
}

/// .lnk/.lng file: exactly 16384 bytes = 8192 `u16` remap entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LnkRemap {
    pub entries: Vec<u16>,
}

impl LnkRemap {
    /// Entries that map to their own index.
    pub fn identity_count(&self) -> usize {
        self.entries
            .iter()
            .enumerate()
            .filter(|(i, v)| **v as usize == *i)
            .count()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.entries.len() * 2);
        for e in &self.entries {
            v.extend_from_slice(&e.to_le_bytes());
        }
        v
    }
}

/// Parse a .lnk/.lng remap (must be exactly 16384 bytes).
pub fn parse_lnk_lng(data: &[u8]) -> Result<LnkRemap, AssetsError> {
    if data.len() != 16384 {
        return Err(AssetsError::WrongSize { len: data.len() });
    }
    let mut entries = Vec::with_capacity(8192);
    for i in 0..8192 {
        entries.push(u16le(data, i * 2));
    }
    Ok(LnkRemap { entries })
}

/// One .mrw chunk directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrwChunk {
    pub off: u32,
    pub size: u32,
    /// `off + size` lies within the file.
    pub fits: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mrw {
    pub count: usize,
    pub chunks: Vec<MrwChunk>,
}

/// Parse an .mrw chunk table: `u16` count then `count` `(off u32, size u32)`
/// pairs. Chunk payloads stay in the source buffer (the CLI wraps them).
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
        let off = u32le(data, b);
        let size = u32le(data, b + 4);
        let fits = off as usize + size as usize <= data.len();
        chunks.push(MrwChunk { off, size, fits });
    }
    Ok(Mrw { count, chunks })
}

/// One walked .nme section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NmeSection {
    /// Zero-count section that ends the file exactly.
    Zero { sec: usize, at: usize },
    /// Unparseable tail: neither 10-byte nor 8-byte records fit.
    Tail { sec: usize, count: usize, at: usize },
    /// Records section; `sample` holds up to 32 records as `i16` word vectors.
    Records {
        sec: usize,
        count: usize,
        rec: usize,
        at: usize,
        sample: Vec<Vec<i16>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NmeFile {
    pub size: usize,
    pub sections: Vec<NmeSection>,
}

fn next_count_plausible(d: &[u8], p: usize) -> bool {
    if p + 2 > d.len() {
        return p == d.len();
    }
    let c = u16le(d, p) as usize;
    c < 4000
}

/// Walk an .nme file as a sequence of `u16 count` + `count*rec` sections,
/// with the 8street rec8/rec10 heuristic: prefer 10-byte records when the
/// following section's count looks plausible, else 8-byte. Never fails; at
/// most 16 sections are walked.
pub fn parse_nme(data: &[u8]) -> NmeFile {
    let mut p = 0usize;
    let mut sections = Vec::new();
    let mut sec = 0usize;
    while p + 2 <= data.len() && sec < 16 {
        let count = u16le(data, p) as usize;
        if count == 0 && p + 2 == data.len() {
            sections.push(NmeSection::Zero { sec, at: p });
            break;
        }
        let rec10 = p + 2 + count * 10 <= data.len();
        let rec8b = p + 2 + count * 8 <= data.len();
        let chosen = if rec10 && rec8b {
            if next_count_plausible(data, p + 2 + count * 10) {
                10
            } else {
                8
            }
        } else if rec10 {
            10
        } else if rec8b {
            8
        } else {
            0
        };
        if chosen == 0 {
            sections.push(NmeSection::Tail { sec, count, at: p });
            break;
        }
        let mut sample = Vec::new();
        for i in 0..count.min(32) {
            let b = p + 2 + i * chosen;
            let mut words = Vec::new();
            for k in 0..chosen / 2 {
                words.push(i16le(data, b + k * 2));
            }
            sample.push(words);
        }
        sections.push(NmeSection::Records {
            sec,
            count,
            rec: chosen,
            at: p,
            sample,
        });
        p += 2 + count * chosen;
        sec += 1;
    }
    NmeFile {
        size: data.len(),
        sections,
    }
}

/// One .bdg record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BdgRecord {
    Inactive,
    /// Active record: dims `w`x`h`x`dep`, `0x36`-byte head, total stride
    /// `0x36 + 3*2*w*h*dep`. `head_hex` is the hex form of the 0x36 bytes.
    Active {
        w: usize,
        h: usize,
        dep: usize,
        blobs3: usize,
        head_hex: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BdgFile {
    pub size: usize,
    pub records: Vec<BdgRecord>,
}

/// Walk a .bdg file: `u16 flag==1` starts an active record (stride
/// `0x36 + 6*w*h*dep`), anything else is a 2-byte inactive slot. At most 282
/// records are walked. Never fails.
pub fn parse_bdg(data: &[u8]) -> BdgFile {
    let mut p = 0usize;
    let mut records = Vec::new();
    while p + 2 <= data.len() && records.len() < 282 {
        let flag = u16le(data, p);
        if flag != 1 {
            records.push(BdgRecord::Inactive);
            p += 2;
            continue;
        }
        if p + 0x36 > data.len() {
            break;
        }
        let w = u16le(data, p + 2) as usize;
        let h = u16le(data, p + 4) as usize;
        let dep = u16le(data, p + 6) as usize;
        let blob = 2 * w * h * dep;
        let total = 0x36 + 3 * blob;
        records.push(BdgRecord::Active {
            w,
            h,
            dep,
            blobs3: blob * 3,
            head_hex: hex_head(&data[p..p + 0x36], 0x36),
        });
        p += total;
    }
    BdgFile {
        size: data.len(),
        records,
    }
}

/// Percentage (0..=100) of bytes that are ASCII, as the text dumper reports it.
pub fn ascii_pct(data: &[u8]) -> usize {
    data.iter().filter(|b| b.is_ascii()).count() * 100 / data.len().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_round_trip() {
        let d = vec![7u8; 48];
        let m = parse_min(&d).unwrap();
        assert_eq!(m.tile_count(), 3);
        assert_eq!(m.to_bytes(), d);
        assert_eq!(
            parse_min(&[0u8; 17]),
            Err(AssetsError::NotMultiple { len: 17 })
        );
    }

    #[test]
    fn lnk_round_trip_and_identity() {
        let mut d = Vec::with_capacity(16384);
        for i in 0..8192u16 {
            // first 100 entries map to themselves, the rest to 0
            d.extend_from_slice(&(if i < 100 { i } else { 0 }).to_le_bytes());
        }
        let l = parse_lnk_lng(&d).unwrap();
        assert_eq!(l.entries.len(), 8192);
        assert_eq!(l.identity_count(), 100);
        assert_eq!(l.to_bytes(), d);
        assert_eq!(
            parse_lnk_lng(&vec![0u8; 16383]),
            Err(AssetsError::WrongSize { len: 16383 })
        );
        assert_eq!(
            parse_lnk_lng(&vec![0u8; 16385]),
            Err(AssetsError::WrongSize { len: 16385 })
        );
    }

    #[test]
    fn mrw_directory() {
        let mut d = 2u16.to_le_bytes().to_vec();
        d.extend_from_slice(&2u32.to_le_bytes()); // off 2
        d.extend_from_slice(&4u32.to_le_bytes()); // size 4
        d.extend_from_slice(&[0xAA; 4]); // payload at 2..6
        d.extend_from_slice(&100u32.to_le_bytes()); // off 100 (outside)
        d.extend_from_slice(&8u32.to_le_bytes());
        let m = parse_mrw(&d).unwrap();
        assert_eq!(m.count, 2);
        assert!(m.chunks[0].fits);
        assert_eq!((m.chunks[0].off, m.chunks[0].size), (2, 4));
        assert!(!m.chunks[1].fits);
        assert_eq!(parse_mrw(&[0u8; 9]), Err(AssetsError::TooSmall { len: 9 }));
        let mut big = 5u16.to_le_bytes().to_vec();
        big.resize(20, 0); // count 5 needs 42B
        assert_eq!(
            parse_mrw(&big),
            Err(AssetsError::CountOverruns { count: 5, len: 20 })
        );
    }

    #[test]
    fn nme_walks_rec10_then_zero() {
        // section: count=1, 10-byte record, then zero count at exact end
        let mut d = 1u16.to_le_bytes().to_vec();
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        d.extend_from_slice(&0u16.to_le_bytes());
        let n = parse_nme(&d);
        assert_eq!(n.sections.len(), 2);
        match &n.sections[0] {
            NmeSection::Records {
                count, rec, sample, ..
            } => {
                assert_eq!(*count, 1);
                assert_eq!(*rec, 10);
                assert_eq!(sample[0], vec![0x0201, 0x0403, 0x0605, 0x0807, 0x0A09]);
            }
            other => panic!("expected Records, got {other:?}"),
        }
        assert!(matches!(n.sections[1], NmeSection::Zero { .. }));
    }

    #[test]
    fn nme_tail_when_nothing_fits() {
        // count=5000: neither 5000*10 nor 5000*8 fits in 6 bytes
        let d = (5000u16).to_le_bytes().to_vec();
        let n = parse_nme(&d);
        assert_eq!(n.sections.len(), 1);
        assert!(matches!(
            n.sections[0],
            NmeSection::Tail { count: 5000, .. }
        ));
    }

    #[test]
    fn bdg_walk() {
        let rec = |out: &mut Vec<u8>| {
            out.extend_from_slice(&1u16.to_le_bytes()); // active
            out.extend_from_slice(&2u16.to_le_bytes()); // w
            out.extend_from_slice(&2u16.to_le_bytes()); // h
            out.extend_from_slice(&1u16.to_le_bytes()); // dep
            out.extend_from_slice(&[0x55; 0x36 - 8]); // rest of head (0x36 total)
            out.extend(std::iter::repeat_n(0x77, 3 * 2 * 2 * 2)); // blob payload
        };
        let mut d = Vec::new();
        d.extend_from_slice(&0u16.to_le_bytes()); // inactive
        rec(&mut d);
        rec(&mut d);
        let b = parse_bdg(&d);
        assert_eq!(b.records.len(), 3);
        assert!(matches!(b.records[0], BdgRecord::Inactive));
        match &b.records[1] {
            BdgRecord::Active {
                w, h, dep, blobs3, ..
            } => {
                assert_eq!((*w, *h, *dep), (2, 2, 1));
                assert_eq!(*blobs3, 3 * 2 * 2 * 2);
            }
            other => panic!("expected Active, got {other:?}"),
        }
        assert_eq!(b.size, d.len());
    }

    #[test]
    fn bdg_truncated_head_breaks_cleanly() {
        let mut d = 1u16.to_le_bytes().to_vec();
        d.extend_from_slice(&[0u8; 4]);
        let b = parse_bdg(&d);
        assert!(b.records.is_empty());
    }

    #[test]
    fn ascii_pct_helper() {
        assert_eq!(ascii_pct(b"hello"), 100);
        assert_eq!(ascii_pct(&[0xFFu8; 4]), 0);
        assert_eq!(ascii_pct(&[]), 0); // no div-by-zero
        assert_eq!(ascii_pct(&[b'a', 0xF0, b'b', 0xF1]), 50);
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 2026u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 5, 10, 16, 256, 16384, 20000] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_min(&d);
            let _ = parse_lnk_lng(&d);
            let _ = parse_mrw(&d);
            let _ = parse_nme(&d);
            let _ = parse_bdg(&d);
            let _ = ascii_pct(&d);
        }
    }
}
