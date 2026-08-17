//! Mission-layer formats: 8-plane grids (.map/.tot/.col/.dat), trigger
//! tables (.trt), markers (.mrk), start positions (.pos), pads (.pad) and
//! path headers (.pth).

use crate::{u16le, AssetsError};

/// An 8-plane cell grid. `grid8` files widen their `u8` planes to `u16`
/// values (0..=255) so both grid kinds share this shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    /// Always 8 planes, each `w*h` cells.
    pub planes: Vec<Vec<u16>>,
}

impl Grid {
    /// Per-plane summary used by the inspect CLI's `planes_stat` JSON.
    /// Empty grids report min/max/uniq of 0 (the legacy tool panicked).
    pub fn plane_stats(&self) -> Vec<PlaneStat> {
        (0..8).map(|p| plane_stat(p, &self.planes[p])).collect()
    }

    /// Byte-identical rebuild of the original .map/.tot/.col file.
    pub fn to_bytes_grid16(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + self.w * self.h * 16);
        v.extend_from_slice(&(self.w as u16).to_le_bytes());
        v.extend_from_slice(&(self.h as u16).to_le_bytes());
        for plane in &self.planes {
            for cell in plane {
                v.extend_from_slice(&cell.to_le_bytes());
            }
        }
        v
    }

    /// Byte-identical rebuild of the original .dat file.
    pub fn to_bytes_grid8(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + self.w * self.h * 8);
        v.extend_from_slice(&(self.w as u16).to_le_bytes());
        v.extend_from_slice(&(self.h as u16).to_le_bytes());
        for plane in &self.planes {
            for cell in plane {
                v.push(*cell as u8);
            }
        }
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneStat {
    pub plane: usize,
    pub min: u16,
    pub max: u16,
    pub uniq: usize,
    pub top: Vec<(u16, usize)>,
}

fn plane_stat(plane: usize, vals: &[u16]) -> PlaneStat {
    if vals.is_empty() {
        return PlaneStat {
            plane,
            min: 0,
            max: 0,
            uniq: 0,
            top: Vec::new(),
        };
    }
    let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    let mut min = u16::MAX;
    let mut max = 0u16;
    for v in vals {
        *counts.entry(*v).or_insert(0) += 1;
        min = min.min(*v);
        max = max.max(*v);
    }
    let mut list: Vec<(u16, usize)> = counts.into_iter().collect();
    list.sort_by_key(|e| std::cmp::Reverse(e.1));
    list.truncate(6);
    PlaneStat {
        plane,
        min,
        max,
        uniq: vals.iter().collect::<std::collections::BTreeSet<_>>().len(),
        top: list,
    }
}

/// Parse a grid16 file: 4-byte `w`,`h` header then 8 `u16` planes.
/// Required size: `4 + w*h*16`.
pub fn parse_grid16(data: &[u8]) -> Result<Grid, AssetsError> {
    if data.len() < 4 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let w = u16le(data, 0) as usize;
    let h = u16le(data, 2) as usize;
    let need = 4 + w * h * 16;
    if data.len() != need {
        return Err(AssetsError::SizeFormula {
            len: data.len(),
            expected: need,
            w,
            h,
        });
    }
    let mut planes = Vec::with_capacity(8);
    for p in 0..8 {
        let mut plane = Vec::with_capacity(w * h);
        for i in 0..w * h {
            plane.push(u16le(data, 4 + p * w * h * 2 + i * 2));
        }
        planes.push(plane);
    }
    Ok(Grid { w, h, planes })
}

/// Parse a grid8 file: 4-byte `w`,`h` header then 8 `u8` planes.
/// Required size: `4 + w*h*8`.
pub fn parse_grid8(data: &[u8]) -> Result<Grid, AssetsError> {
    if data.len() < 4 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let w = u16le(data, 0) as usize;
    let h = u16le(data, 2) as usize;
    let need = 4 + w * h * 8;
    if data.len() != need {
        return Err(AssetsError::SizeFormula {
            len: data.len(),
            expected: need,
            w,
            h,
        });
    }
    let mut planes = Vec::with_capacity(8);
    for p in 0..8 {
        let mut plane = Vec::with_capacity(w * h);
        for i in 0..w * h {
            plane.push(data[4 + p * w * h + i] as u16);
        }
        planes.push(plane);
    }
    Ok(Grid { w, h, planes })
}

/// Trigger table record: x, y, type plus 6 still-unmapped bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrtRecord {
    pub x: u16,
    pub y: u16,
    pub kind: u16,
    pub rest: [u8; 6],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trt {
    pub count: usize,
    pub records: Vec<TrtRecord>,
}

impl Trt {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(2 + self.records.len() * 12);
        v.extend_from_slice(&(self.count as u16).to_le_bytes());
        for r in &self.records {
            v.extend_from_slice(&r.x.to_le_bytes());
            v.extend_from_slice(&r.y.to_le_bytes());
            v.extend_from_slice(&r.kind.to_le_bytes());
            v.extend_from_slice(&r.rest);
        }
        v
    }
}

/// Parse a trigger table: `u16` count then `count` 12-byte records
/// (size must be exactly `2 + count*12`).
pub fn parse_trt(data: &[u8]) -> Result<Trt, AssetsError> {
    if data.len() < 2 || !(data.len() - 2).is_multiple_of(12) {
        return Err(AssetsError::NotMultiple { len: data.len() });
    }
    let n = u16le(data, 0) as usize;
    if n * 12 + 2 != data.len() {
        return Err(AssetsError::CountMismatch { count: n });
    }
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        let b = 2 + i * 12;
        let mut rest = [0u8; 6];
        rest.copy_from_slice(&data[b + 6..b + 12]);
        records.push(TrtRecord {
            x: u16le(data, b),
            y: u16le(data, b + 2),
            kind: u16le(data, b + 4),
            rest,
        });
    }
    Ok(Trt { count: n, records })
}

/// Marker record: flag, x, y, type plus 8 still-unmapped bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrkRecord {
    pub flag: u16,
    pub x: u16,
    pub y: u16,
    pub kind: u16,
    pub rest: [u8; 8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mrk {
    pub records: Vec<MrkRecord>,
}

impl Mrk {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.records.len() * 16);
        for r in &self.records {
            v.extend_from_slice(&r.flag.to_le_bytes());
            v.extend_from_slice(&r.x.to_le_bytes());
            v.extend_from_slice(&r.y.to_le_bytes());
            v.extend_from_slice(&r.kind.to_le_bytes());
            v.extend_from_slice(&r.rest);
        }
        v
    }
}

/// Parse a marker file: `n*16`-byte records (`n` implied by size).
pub fn parse_mrk(data: &[u8]) -> Result<Mrk, AssetsError> {
    if !data.len().is_multiple_of(16) {
        return Err(AssetsError::NotMultiple { len: data.len() });
    }
    let n = data.len() / 16;
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        let b = i * 16;
        let mut rest = [0u8; 8];
        rest.copy_from_slice(&data[b + 8..b + 16]);
        records.push(MrkRecord {
            flag: u16le(data, b),
            x: u16le(data, b + 2),
            y: u16le(data, b + 4),
            kind: u16le(data, b + 6),
            rest,
        });
    }
    Ok(Mrk { records })
}

/// One 16-byte .pos slot. `empty` slots are all-FF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosSlot {
    pub raw: [u8; 16],
    pub empty: bool,
    pub u16x4: [u16; 4],
    pub u16x4b: [u16; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosFile {
    pub slots: Vec<PosSlot>,
}

/// Parse a start-position file: `n*16`-byte slots, all-FF marks empty.
pub fn parse_pos(data: &[u8]) -> Result<PosFile, AssetsError> {
    if !data.len().is_multiple_of(16) {
        return Err(AssetsError::NotMultiple { len: data.len() });
    }
    let n = data.len() / 16;
    let mut slots = Vec::with_capacity(n);
    for i in 0..n {
        let b = i * 16;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(&data[b..b + 16]);
        let empty = raw.iter().all(|x| *x == 0xFF);
        let u16x4 = [
            u16le(&raw, 0),
            u16le(&raw, 2),
            u16le(&raw, 4),
            u16le(&raw, 6),
        ];
        let u16x4b = [
            u16le(&raw, 8),
            u16le(&raw, 10),
            u16le(&raw, 12),
            u16le(&raw, 14),
        ];
        slots.push(PosSlot {
            raw,
            empty,
            u16x4,
            u16x4b,
        });
    }
    Ok(PosFile { slots })
}

/// One 6-byte .pad record (all-FF slots are fill, not records).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadRecord {
    pub x: u16,
    pub y: u16,
    pub kind: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadFile {
    /// One entry per 6-byte slot; `None` for 0xFF fill slots.
    pub slots: Vec<Option<PadRecord>>,
}

/// Parse a pad file: `n*6`-byte records.
pub fn parse_pad(data: &[u8]) -> Result<PadFile, AssetsError> {
    if !data.len().is_multiple_of(6) {
        return Err(AssetsError::NotMultiple { len: data.len() });
    }
    let n = data.len() / 6;
    let mut slots = Vec::with_capacity(n);
    for i in 0..n {
        let b = i * 6;
        let chunk = &data[b..b + 6];
        if chunk.iter().all(|x| *x == 0xFF) {
            slots.push(None);
        } else {
            slots.push(Some(PadRecord {
                x: u16le(data, b),
                y: u16le(data, b + 2),
                kind: u16le(data, b + 4),
            }));
        }
    }
    Ok(PadFile { slots })
}

/// Path file: little more than a `u16` count and the head bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pth {
    pub count: u16,
    /// First (up to) 16 bytes of the file, for the CLI's head field.
    pub head: Vec<u8>,
}

/// Parse a path header. Never fails: short files report count 0.
pub fn parse_pth(data: &[u8]) -> Pth {
    Pth {
        count: if data.len() >= 2 { u16le(data, 0) } else { 0 },
        head: data.iter().take(16).copied().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid16_round_trip_and_stats() {
        let w = 3usize;
        let h = 2usize;
        let mut d = Vec::new();
        d.extend_from_slice(&(w as u16).to_le_bytes());
        d.extend_from_slice(&(h as u16).to_le_bytes());
        for p in 0..8u16 {
            for i in 0..(w * h) as u16 {
                let v = p * 100 + i;
                d.extend_from_slice(&v.to_le_bytes());
            }
        }
        let g = parse_grid16(&d).unwrap();
        assert_eq!((g.w, g.h), (3, 2));
        assert_eq!(g.planes.len(), 8);
        assert_eq!(g.planes[0], vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(g.planes[7], vec![700, 701, 702, 703, 704, 705]);
        assert_eq!(g.to_bytes_grid16(), d);
        let stats = g.plane_stats();
        assert_eq!(stats.len(), 8);
        assert_eq!(stats[0].min, 0);
        assert_eq!(stats[0].max, 5);
        assert_eq!(stats[0].uniq, 6);
        assert_eq!(stats[0].top[0], (0, 1)); // all counts 1, key order preserved
    }

    #[test]
    fn grid8_round_trip() {
        let mut d = vec![2u8, 0, 2, 0]; // 2x2
        for p in 0..8 {
            for i in 0..4 {
                d.push(p * 32 + i);
            }
        }
        let g = parse_grid8(&d).unwrap();
        assert_eq!(g.planes[7], vec![224, 225, 226, 227]);
        assert_eq!(g.to_bytes_grid8(), d);
    }

    #[test]
    fn grids_reject_bad_sizes() {
        assert_eq!(parse_grid16(&[1, 2]), Err(AssetsError::TooSmall { len: 2 }));
        // claims 2x2 (needs 4+64=68) but has 8
        assert_eq!(
            parse_grid16(&[2, 0, 2, 0, 0, 0, 0, 0]),
            Err(AssetsError::SizeFormula {
                len: 8,
                expected: 68,
                w: 2,
                h: 2
            })
        );
        assert_eq!(
            parse_grid8(&[2, 0, 2, 0, 0, 0, 0, 0]),
            Err(AssetsError::SizeFormula {
                len: 8,
                expected: 36,
                w: 2,
                h: 2
            })
        );
    }

    #[test]
    fn empty_grid_is_ok_not_panic() {
        // w=h=0 -> size 4 exactly; legacy tool panicked on min/max of empty
        let g = parse_grid16(&[0, 0, 0, 0]).unwrap();
        assert_eq!(g.plane_stats()[0].uniq, 0);
        let g8 = parse_grid8(&[0, 0, 0, 0]).unwrap();
        assert_eq!(g8.to_bytes_grid8(), vec![0, 0, 0, 0]);
    }

    #[test]
    fn trt_round_trip() {
        let mut d = 2u16.to_le_bytes().to_vec();
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&2u16.to_le_bytes());
        d.extend_from_slice(&9u16.to_le_bytes());
        d.extend_from_slice(&[0xAA; 6]);
        d.extend_from_slice(&3u16.to_le_bytes());
        d.extend_from_slice(&4u16.to_le_bytes());
        d.extend_from_slice(&9u16.to_le_bytes());
        d.extend_from_slice(&[0xBB; 6]);
        let t = parse_trt(&d).unwrap();
        assert_eq!(t.count, 2);
        assert_eq!(
            (t.records[0].x, t.records[0].y, t.records[0].kind),
            (1, 2, 9)
        );
        assert_eq!(t.records[0].rest, [0xAA; 6]);
        assert_eq!(t.to_bytes(), d);
    }

    #[test]
    fn trt_rejects_malformed() {
        assert_eq!(parse_trt(&[0u8]), Err(AssetsError::NotMultiple { len: 1 }));
        // len ok (2+12) but count says 2
        let mut d = 2u16.to_le_bytes().to_vec();
        d.extend_from_slice(&[0; 12]);
        assert_eq!(parse_trt(&d), Err(AssetsError::CountMismatch { count: 2 }));
        // 3 extra bytes -> not 2+12n
        let mut d2 = 0u16.to_le_bytes().to_vec();
        d2.extend_from_slice(&[0; 15]);
        assert_eq!(parse_trt(&d2), Err(AssetsError::NotMultiple { len: 17 }));
    }

    #[test]
    fn mrk_round_trip() {
        let mut d = Vec::new();
        for i in 0..3u16 {
            d.extend_from_slice(&(i + 1).to_le_bytes()); // flag
            d.extend_from_slice(&(i * 10).to_le_bytes());
            d.extend_from_slice(&(i * 20).to_le_bytes());
            d.extend_from_slice(&(i * 30).to_le_bytes());
            d.extend_from_slice(&[i as u8; 8]);
        }
        let m = parse_mrk(&d).unwrap();
        assert_eq!(m.records.len(), 3);
        assert_eq!(m.records[2].kind, 60);
        assert_eq!(m.to_bytes(), d);
        assert_eq!(
            parse_mrk(&[0u8; 15]),
            Err(AssetsError::NotMultiple { len: 15 })
        );
    }

    #[test]
    fn pos_empty_detection() {
        let mut d = vec![0u8; 32];
        for (i, b) in d[..16].iter_mut().enumerate() {
            *b = i as u8;
        }
        for b in &mut d[16..32] {
            *b = 0xFF;
        }
        let p = parse_pos(&d).unwrap();
        assert_eq!(p.slots.len(), 2);
        assert!(!p.slots[0].empty);
        assert_eq!(p.slots[0].u16x4, [0x0100, 0x0302, 0x0504, 0x0706]);
        assert!(p.slots[1].empty);
        assert_eq!(
            parse_pos(&[0u8; 17]),
            Err(AssetsError::NotMultiple { len: 17 })
        );
    }

    #[test]
    fn pad_fill_slots() {
        let mut d = Vec::new();
        d.extend_from_slice(&7u16.to_le_bytes());
        d.extend_from_slice(&8u16.to_le_bytes());
        d.extend_from_slice(&9u16.to_le_bytes());
        d.extend_from_slice(&[0xFF; 6]);
        let p = parse_pad(&d).unwrap();
        assert_eq!(p.slots.len(), 2);
        assert_eq!(
            p.slots[0].as_ref().map(|r| (r.x, r.y, r.kind)),
            Some((7, 8, 9))
        );
        assert!(p.slots[1].is_none());
        assert_eq!(
            parse_pad(&[0u8; 7]),
            Err(AssetsError::NotMultiple { len: 7 })
        );
    }

    #[test]
    fn pth_never_fails() {
        let p = parse_pth(&[5, 0, 1, 2, 3]);
        assert_eq!(p.count, 5);
        assert_eq!(p.head, vec![5, 0, 1, 2, 3]);
        let p = parse_pth(&[]);
        assert_eq!(p.count, 0);
        assert!(p.head.is_empty());
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 1234u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 3, 4, 5, 16, 17, 65536] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_grid16(&d);
            let _ = parse_grid8(&d);
            let _ = parse_trt(&d);
            let _ = parse_mrk(&d);
            let _ = parse_pos(&d);
            let _ = parse_pad(&d);
            let _ = parse_pth(&d);
        }
    }
}
