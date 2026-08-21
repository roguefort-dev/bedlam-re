//! Miscellaneous bank formats: .min tile colors, .lnk/.lng remaps, .nme
//! name tables, .bdg badge/badge-art records, and the ascii-percentage
//! helper for text files. (.MRW lives in music.rs with .MRS.)

use crate::{hex_head, u16le, AssetsError};

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

/// One of the eight fixed .nme sections, in loader order — the read schedule
/// of the EXW mission-load dispatcher FUN_00416458 (RE-EXW-SIM §7j.18):
/// after staging ".NME" it reads exactly eight `u16 count + count*rec`
/// sections; each feeds the critter bank 0x4cff98 (sections 1-7, one
/// critter-ACTOR state each) or the POI/personnel bank 0x4dabdc (section 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmeSectionKind {
    /// 10 B records: critter state 2 (sine-walk shooter). Fields:
    /// w1 = spawn base (adds difficulty), w2 = mirror flag, w3/w4 = x/y tile.
    Shooters,
    /// 10 B records: critter state 1 (wander). w3/w4 = x/y tile; the loader
    /// searches the DAT volume downward from z=6 for the standing level.
    Wanderers,
    /// 8 B records: critter state 5 (mixed-AI). w1 = probe level, w2/w3 = x/y.
    MixedState5,
    /// 8 B records: critter state 4 (mixed-AI seek steppers).
    SeekSteppers,
    /// 10 B records: critter state 3 (chase; stores home x/y).
    Chasers,
    /// 8 B records: critter state 6 (mixed-AI, ballistic).
    BallisticState6,
    /// 6 B records: critter state 7 (close combat). w1/w2 = x/y tile.
    CloseCombat,
    /// 8 B records: personnel/POI bank — spawns FOUR POIs per record (state 5
    /// ESCAPE, flee to the exit slots). w1 = probe level, w2/w3 = x/y.
    Personnel,
}

impl NmeSectionKind {
    /// The loader order — also the order of `NmeFile::sections`.
    pub const ALL: [NmeSectionKind; 8] = [
        NmeSectionKind::Shooters,
        NmeSectionKind::Wanderers,
        NmeSectionKind::MixedState5,
        NmeSectionKind::SeekSteppers,
        NmeSectionKind::Chasers,
        NmeSectionKind::BallisticState6,
        NmeSectionKind::CloseCombat,
        NmeSectionKind::Personnel,
    ];

    /// Record width in bytes, fixed per section position.
    pub fn width(self) -> usize {
        match self {
            NmeSectionKind::Shooters | NmeSectionKind::Wanderers | NmeSectionKind::Chasers => 10,
            NmeSectionKind::CloseCombat => 6,
            _ => 8,
        }
    }
}

/// One walked .nme section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NmeSection {
    /// A section in the fixed schedule: `count` records of the kind's width;
    /// `sample` holds up to 32 records as u16 word vectors (words past EOF
    /// read as 0, mirroring the zero-filled staging buffer).
    Section {
        kind: NmeSectionKind,
        at: usize,
        count: usize,
        sample: Vec<Vec<u16>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NmeFile {
    pub size: usize,
    pub sections: Vec<NmeSection>,
    /// Bytes consumed by the 8-section schedule (can exceed `size` only if a
    /// count overruns the file, which no shipped file does).
    pub consumed: usize,
    /// Bytes after section 8 that the game loader never reads
    /// (ZONEA/MISSION1.NME carries 16 orphan bytes; the shipped corpus is
    /// otherwise byte-exact).
    pub orphan_tail: usize,
}

/// Walk an .nme file with the EXW loader's exact fixed schedule: eight
/// `u16 count + count*width` sections in the order of `NmeSectionKind::ALL`.
/// Counts read past EOF are 0 (the staging buffer is zero-filled .bss), so
/// the walk never fails.
pub fn parse_nme(data: &[u8]) -> NmeFile {
    let mut p = 0usize;
    let mut sections = Vec::with_capacity(8);
    for kind in NmeSectionKind::ALL {
        let w = kind.width();
        let at = p;
        let count = if p + 2 <= data.len() {
            u16le(data, p) as usize
        } else {
            0
        };
        p += 2;
        let mut sample = Vec::new();
        for i in 0..count.min(32) {
            let b = p + i * w;
            let mut words = Vec::with_capacity(w / 2);
            for k in 0..w / 2 {
                let off = b + k * 2;
                words.push(if off + 2 <= data.len() {
                    u16le(data, off)
                } else {
                    0
                });
            }
            sample.push(words);
        }
        p += count * w;
        sections.push(NmeSection::Section {
            kind,
            at,
            count,
            sample,
        });
    }
    NmeFile {
        size: data.len(),
        sections,
        consumed: p,
        orphan_tail: data.len().saturating_sub(p),
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
    fn nme_fixed_schedule_consumes_exactly() {
        // one record in each of the eight fixed sections (10/10/8/8/10/8/6/8)
        let mut d = Vec::new();
        for (i, kind) in NmeSectionKind::ALL.iter().enumerate() {
            d.extend_from_slice(&(1u16 + i as u16).to_le_bytes()); // nonzero count
            for _ in 0..(1 + i) {
                d.extend(std::iter::repeat_n(0xA0 | i as u8, kind.width()));
            }
        }
        let n = parse_nme(&d);
        assert_eq!(n.sections.len(), 8);
        assert_eq!(n.consumed, d.len());
        assert_eq!(n.orphan_tail, 0);
        for (i, sec) in n.sections.iter().enumerate() {
            let NmeSection::Section {
                kind,
                count,
                sample,
                ..
            } = sec;
            assert_eq!(*kind, NmeSectionKind::ALL[i]);
            assert_eq!(*count, 1 + i);
            assert_eq!(sample.len(), 1 + i);
            assert_eq!(sample[0].len(), kind.width() / 2);
            let fill = (0xA0 | i as u8) as u16;
            assert!(sample[0].iter().all(|w| *w == (fill << 8) | fill));
        }
    }

    #[test]
    fn nme_counts_past_eof_read_zero() {
        // seven zero counts + an 8th section of 2 records, nothing more:
        // the schedule stops cleanly at EOF (staging buffer reads as 0)
        let mut d = vec![0u8; 14];
        d.extend_from_slice(&2u16.to_le_bytes());
        d.extend_from_slice(&[0x11; 16]);
        let n = parse_nme(&d);
        assert_eq!(n.consumed, d.len());
        assert_eq!(n.orphan_tail, 0);
        assert_eq!(n.sections.len(), 8);
        let last = n.sections.last().unwrap();
        let NmeSection::Section { kind, count, .. } = last;
        assert_eq!(*kind, NmeSectionKind::Personnel);
        assert_eq!(*count, 2);
    }

    #[test]
    fn nme_orphan_tail_is_reported() {
        // section 8 leaves unread trailing bytes
        let mut d = vec![0u8; 16];
        d.extend_from_slice(&[0xEE; 4]);
        let n = parse_nme(&d);
        assert_eq!(n.orphan_tail, 4);
        assert_eq!(n.consumed, 16);
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
            let _ = parse_nme(&d);
            let _ = parse_bdg(&d);
            let _ = ascii_pct(&d);
        }
    }
}
