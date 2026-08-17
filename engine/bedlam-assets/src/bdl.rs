//! BDL config files: SAVED.BDL (5 save slots), HISCORE.BDL (10 scores) and
//! OPTIONS.BDL. The stem decides which layout applies (CLI-side dispatch).

use crate::{u16le, u32le, AssetsError};

/// Name fields are fixed-width byte arrays; non-graphic, non-space bytes are
/// displayed as '.' (same sanitization the tool has always used).
pub(crate) fn sanitize_name(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|c| {
            if c.is_ascii_graphic() || *c == b' ' {
                *c as char
            } else {
                char::from(46u8)
            }
        })
        .collect()
}

/// One 180-byte SAVED.BDL slot. `raw` is kept for byte-identical rebuilds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSlot {
    pub raw: [u8; 180],
    pub name: String,
    pub completed_mask: u32,
    pub zone: u16,
    pub money: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedBdl {
    pub slots: Vec<SavedSlot>,
}

impl SavedBdl {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(900);
        for s in &self.slots {
            v.extend_from_slice(&s.raw);
        }
        v
    }
}

/// Parse SAVED.BDL (exactly 900 bytes = 5 x 180B slots).
pub fn parse_saved_bdl(data: &[u8]) -> Result<SavedBdl, AssetsError> {
    if data.len() != 900 {
        return Err(AssetsError::WrongSize { len: data.len() });
    }
    let mut slots = Vec::with_capacity(5);
    for s in 0..5 {
        let b = s * 180;
        let mut raw = [0u8; 180];
        raw.copy_from_slice(&data[b..b + 180]);
        slots.push(SavedSlot {
            name: sanitize_name(&raw[..8]),
            completed_mask: u32le(&raw, 8),
            zone: u16le(&raw, 12),
            money: u32le(&raw, 18),
            raw,
        });
    }
    Ok(SavedBdl { slots })
}

/// One 12-byte HISCORE.BDL entry: `u32` score then 8 name bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiscoreEntry {
    pub raw: [u8; 12],
    pub score: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiscoreBdl {
    pub scores: Vec<HiscoreEntry>,
}

impl HiscoreBdl {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(120);
        for s in &self.scores {
            v.extend_from_slice(&s.raw);
        }
        v
    }
}

/// Parse HISCORE.BDL (exactly 120 bytes = 10 x 12B entries).
pub fn parse_hiscore_bdl(data: &[u8]) -> Result<HiscoreBdl, AssetsError> {
    if data.len() != 120 {
        return Err(AssetsError::WrongSize { len: data.len() });
    }
    let mut scores = Vec::with_capacity(10);
    for s in 0..10 {
        let b = s * 12;
        let mut raw = [0u8; 12];
        raw.copy_from_slice(&data[b..b + 12]);
        scores.push(HiscoreEntry {
            score: u32le(&raw, 0),
            name: sanitize_name(&raw[4..12]),
            raw,
        });
    }
    Ok(HiscoreBdl { scores })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsBdl {
    pub backbuffer: u32,
    pub actionpan: u32,
    pub language: u32,
    pub cd_audio: u32,
    pub playername: String,
    pub volume: u32,
    pub code_no_title: u32,
    pub midi: u32,
    pub sound: u32,
    pub installdrive: u8,
}

/// Parse OPTIONS.BDL (at least 41 bytes; trailing bytes are ignored).
pub fn parse_options_bdl(data: &[u8]) -> Result<OptionsBdl, AssetsError> {
    if data.len() < 41 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    Ok(OptionsBdl {
        backbuffer: u32le(data, 0),
        actionpan: u32le(data, 4),
        language: u32le(data, 8),
        cd_audio: u32le(data, 12),
        playername: sanitize_name(&data[16..24]),
        volume: u32le(data, 24),
        code_no_title: u32le(data, 28),
        midi: u32le(data, 32),
        sound: u32le(data, 36),
        installdrive: data[40],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_round_trip_and_fields() {
        let mut d = vec![0u8; 900];
        for s in 0..5usize {
            let b = s * 180;
            for (i, byte) in b"KATO".iter().enumerate() {
                d[b + i] = *byte;
            }
            d[b + 8..b + 12].copy_from_slice(&(0xDEAD_BEEFu32).to_le_bytes());
            d[b + 12..b + 14].copy_from_slice(&((s + 1) as u16).to_le_bytes());
            d[b + 18..b + 22].copy_from_slice(&((s * 1000) as u32).to_le_bytes());
        }
        let sv = parse_saved_bdl(&d).unwrap();
        assert_eq!(sv.slots.len(), 5);
        assert_eq!(sv.slots[0].name, "KATO...."); // zero bytes sanitize to '.'
        assert_eq!(sv.slots[0].completed_mask, 0xDEAD_BEEF);
        assert_eq!(sv.slots[2].zone, 3);
        assert_eq!(sv.slots[3].money, 3000);
        assert_eq!(sv.to_bytes(), d);
    }

    #[test]
    fn saved_sanitizes_names() {
        let mut d = vec![0xFFu8; 900];
        d[1] = b'A';
        d[2] = b' ';
        let sv = parse_saved_bdl(&d).unwrap();
        assert_eq!(sv.slots[0].name, ".A ....."); // 0xFF->'.', 'A', ' ', then dots
    }

    #[test]
    fn saved_rejects_wrong_size() {
        assert_eq!(
            parse_saved_bdl(&vec![0u8; 899]),
            Err(AssetsError::WrongSize { len: 899 })
        );
        assert_eq!(
            parse_saved_bdl(&vec![0u8; 901]),
            Err(AssetsError::WrongSize { len: 901 })
        );
    }

    #[test]
    fn hiscore_round_trip_and_fields() {
        let mut d = vec![0u8; 120];
        for s in 0..10usize {
            let b = s * 12;
            d[b..b + 4].copy_from_slice(&((10 - s) as u32 * 100).to_le_bytes());
            d[b + 4] = b'H';
            d[b + 5] = b'I';
        }
        let h = parse_hiscore_bdl(&d).unwrap();
        assert_eq!(h.scores.len(), 10);
        assert_eq!(h.scores[0].score, 1000);
        assert_eq!(h.scores[0].name, "HI......");
        assert_eq!(h.scores[9].score, 100);
        assert_eq!(h.to_bytes(), d);
        assert_eq!(
            parse_hiscore_bdl(&[0u8; 121]),
            Err(AssetsError::WrongSize { len: 121 })
        );
    }

    #[test]
    fn options_fields() {
        let mut d = vec![0u8; 41];
        d[0..4].copy_from_slice(&1u32.to_le_bytes());
        d[4..8].copy_from_slice(&2u32.to_le_bytes());
        d[8..12].copy_from_slice(&3u32.to_le_bytes());
        d[12..16].copy_from_slice(&4u32.to_le_bytes());
        for (i, byte) in b"PLAYER".iter().enumerate() {
            d[16 + i] = *byte;
        }
        d[24..28].copy_from_slice(&200u32.to_le_bytes());
        d[28..32].copy_from_slice(&1u32.to_le_bytes());
        d[32..36].copy_from_slice(&7u32.to_le_bytes());
        d[36..40].copy_from_slice(&8u32.to_le_bytes());
        d[40] = b'C';
        let o = parse_options_bdl(&d).unwrap();
        assert_eq!(o.backbuffer, 1);
        assert_eq!(o.actionpan, 2);
        assert_eq!(o.language, 3);
        assert_eq!(o.cd_audio, 4);
        assert_eq!(o.playername, "PLAYER.."); // zero-padded name field
        assert_eq!(o.volume, 200);
        assert_eq!(o.code_no_title, 1);
        assert_eq!(o.midi, 7);
        assert_eq!(o.sound, 8);
        assert_eq!(o.installdrive, b'C');
        assert_eq!(
            parse_options_bdl(&[0u8; 40]),
            Err(AssetsError::TooSmall { len: 40 })
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 777u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 40, 41, 119, 120, 121, 899, 900, 901] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_saved_bdl(&d);
            let _ = parse_hiscore_bdl(&d);
            let _ = parse_options_bdl(&d);
        }
    }
}
