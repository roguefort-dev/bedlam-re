//! SMK2/SMK4 video header (the first 104 bytes). Full frame decoding is a
//! separate future task; this captures the header fields the tool reports.

use crate::{u32le, AssetsError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmkHeader {
    /// "SMK2" or "SMK4".
    pub magic: String,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    /// Raw frame-interval field; negative values encode microseconds.
    pub ms_per_frame_raw: i32,
    pub flags: u32,
    pub audio_sizes: Vec<u32>,
    pub tree_sizes: Vec<u32>,
    pub audio_rates: Vec<u32>,
}

impl SmkHeader {
    /// Same fps description string the tool has always emitted.
    pub fn fps_desc(&self) -> String {
        let ms_raw = self.ms_per_frame_raw;
        if ms_raw > 0 {
            format!("{} fps", 1000 / ms_raw.max(1))
        } else {
            // cast before negate so i32::MIN cannot overflow (the legacy tool
            // wrapped/panicked there; every real file has a sane value)
            let us = (-(ms_raw as i64)) * 10;
            format!("{} fps (us-per-frame encoding: {}us)", 1_000_000 / us, us)
        }
    }
}

/// Parse an SMK2/SMK4 header (requires at least 104 bytes and a known magic).
pub fn parse_smk_header(data: &[u8]) -> Result<SmkHeader, AssetsError> {
    if data.len() < 104 || (&data[0..4] != b"SMK2" && &data[0..4] != b"SMK4") {
        return Err(AssetsError::BadMagic);
    }
    let audio_sizes: Vec<u32> = (0..7).map(|i| u32le(data, 24 + i * 4)).collect();
    let tree_sizes: Vec<u32> = vec![
        u32le(data, 52),
        u32le(data, 56),
        u32le(data, 60),
        u32le(data, 64),
    ];
    let audio_rates: Vec<u32> = (0..7).map(|i| u32le(data, 68 + i * 4)).collect();
    Ok(SmkHeader {
        magic: String::from_utf8_lossy(&data[0..4]).to_string(),
        width: u32le(data, 4),
        height: u32le(data, 8),
        frames: u32le(data, 12),
        ms_per_frame_raw: u32le(data, 16) as i32,
        flags: u32le(data, 20),
        audio_sizes,
        tree_sizes,
        audio_rates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic(ms: i32) -> Vec<u8> {
        let mut d = vec![0u8; 104];
        d[0..4].copy_from_slice(b"SMK2");
        d[4..8].copy_from_slice(&320u32.to_le_bytes());
        d[8..12].copy_from_slice(&200u32.to_le_bytes());
        d[12..16].copy_from_slice(&100u32.to_le_bytes());
        d[16..20].copy_from_slice(&(ms as u32).to_le_bytes());
        d[20..24].copy_from_slice(&0xABCDu32.to_le_bytes());
        for i in 0..7 {
            d[24 + i * 4..28 + i * 4].copy_from_slice(&((i as u32) * 100).to_le_bytes());
            d[68 + i * 4..72 + i * 4].copy_from_slice(&((i as u32) * 7).to_le_bytes());
        }
        for (i, off) in [52usize, 56, 60, 64].iter().enumerate() {
            d[*off..*off + 4].copy_from_slice(&((i as u32) * 11).to_le_bytes());
        }
        d
    }

    #[test]
    fn parse_header_fields() {
        let d = synthetic(40);
        let h = parse_smk_header(&d).unwrap();
        assert_eq!(h.magic, "SMK2");
        assert_eq!((h.width, h.height, h.frames), (320, 200, 100));
        assert_eq!(h.ms_per_frame_raw, 40);
        assert_eq!(h.fps_desc(), "25 fps");
        assert_eq!(h.flags, 0xABCD);
        assert_eq!(h.audio_sizes, vec![0, 100, 200, 300, 400, 500, 600]);
        assert_eq!(h.tree_sizes, vec![0, 11, 22, 33]);
        assert_eq!(h.audio_rates, vec![0, 7, 14, 21, 28, 35, 42]);
    }

    #[test]
    fn negative_ms_is_us_encoding() {
        let h = parse_smk_header(&synthetic(-1)).unwrap();
        assert_eq!(h.fps_desc(), "100000 fps (us-per-frame encoding: 10us)");
        let h = parse_smk_header(&synthetic(-417)).unwrap();
        // us = 4170 -> 1000000/4170 = 239
        assert_eq!(h.fps_desc(), "239 fps (us-per-frame encoding: 4170us)");
        // i32::MIN must not panic
        let h = parse_smk_header(&synthetic(i32::MIN)).unwrap();
        assert!(h.fps_desc().contains("us-per-frame encoding"));
    }

    #[test]
    fn rejects_short_and_bad_magic() {
        assert_eq!(parse_smk_header(b"SMK2"), Err(AssetsError::BadMagic));
        let mut d = synthetic(40);
        d[0..4].copy_from_slice(b"XVID");
        assert_eq!(parse_smk_header(&d), Err(AssetsError::BadMagic));
    }

    #[test]
    fn smk4_magic_accepted() {
        let mut d = synthetic(40);
        d[0..4].copy_from_slice(b"SMK4");
        assert_eq!(parse_smk_header(&d).unwrap().magic, "SMK4");
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 31337u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 4, 103, 104, 105, 4096] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_smk_header(&d);
        }
    }
}
