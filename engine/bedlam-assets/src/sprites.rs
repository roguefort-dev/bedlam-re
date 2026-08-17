//! BIN image banks: `u16` count, `count` `u32` offsets relative to the end of
//! each slot's directory entry, then per-image headers with optional hotspot.

use crate::codecs::{decode_raw, decode_rle16};
use crate::{i16le, u16le, u32le, AssetsError};

/// One entry of a BIN image bank.
///
/// `codec` mirrors the inspect CLI's per-image field exactly: `"rle16"`,
/// `"raw"`, `"empty-slot"`, or a decode-failure description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteImage {
    /// Raw directory offset value (relative to its slot end).
    pub off: u32,
    pub flags: u16,
    pub hot: Option<(i16, i16)>,
    pub w: u16,
    pub h: u16,
    pub codec: String,
    /// True when the pixels decoded (or the slot is legitimately empty).
    pub ok: bool,
    /// Decoded `w*h` 8-bit palette indices, when `ok` and not an empty slot.
    pub pixels: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteBank {
    pub count: usize,
    pub images: Vec<SpriteImage>,
}

struct Header {
    flags: u16,
    hot: Option<(i16, i16)>,
    w: u16,
    h: u16,
    px: usize,
}

fn parse_header(data: &[u8], p: usize) -> Option<Header> {
    if p + 8 > data.len() {
        return None;
    }
    let flags = u16le(data, p);
    let has_hot = flags & 2 != 0;
    if has_hot {
        if p + 12 > data.len() {
            return None;
        }
        let w = i16le(data, p + 6);
        let h = i16le(data, p + 8);
        if w < 0 || h < 0 || w > 4096 || h > 4096 {
            return None;
        }
        Some(Header {
            flags,
            hot: Some((i16le(data, p + 2), i16le(data, p + 4))),
            w: w as u16,
            h: h as u16,
            px: p + 10,
        })
    } else {
        let w = i16le(data, p + 2);
        let h = i16le(data, p + 4);
        if w < 0 || h < 0 || w > 4096 || h > 4096 {
            return None;
        }
        Some(Header {
            flags,
            hot: None,
            w: w as u16,
            h: h as u16,
            px: p + 6,
        })
    }
}

/// Parse a BIN image bank. SINTABLE.BIN is special-cased by the CLI before
/// this parser runs; everything else goes through here.
///
/// Individual bad entries do not fail the parse (the bank is walked to the
/// extent possible, exactly like the original tool); only a truncated
/// directory or a bogus header count is an `Err`.
pub fn parse_bin_images(data: &[u8]) -> Result<SpriteBank, AssetsError> {
    if data.len() < 6 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let count = u16le(data, 0) as usize;
    if count == 0 || 2 + count * 4 > data.len() {
        return Err(AssetsError::CountOverruns {
            count,
            len: data.len(),
        });
    }
    let mut images = Vec::with_capacity(count);
    for i in 0..count {
        let slot = 2 + i * 4;
        let off = u32le(data, slot);
        let start = slot + off as usize;
        let hdr = if start + 8 <= data.len() {
            parse_header(data, start)
        } else {
            None
        }
        .unwrap_or(Header {
            flags: 0,
            hot: None,
            w: 0,
            h: 0,
            px: start,
        });
        if hdr.w == 0 {
            images.push(SpriteImage {
                off,
                flags: 0,
                hot: None,
                w: 0,
                h: 0,
                codec: String::from("empty-slot"),
                ok: true,
                pixels: None,
            });
            continue;
        }
        let w = hdr.w as usize;
        let h = hdr.h as usize;
        let (pixels, codec) = if hdr.flags & 1 != 0 {
            match decode_rle16(&data[hdr.px..], w, h) {
                Ok(px) => (Some(px), String::from("rle16")),
                Err(e) => (None, e.to_string()),
            }
        } else {
            match decode_raw(&data[hdr.px..], w, h) {
                Ok(px) => (Some(px), String::from("raw")),
                Err(e) => (None, e.to_string()),
            }
        };
        let ok = pixels.is_some();
        images.push(SpriteImage {
            off,
            flags: hdr.flags,
            hot: hdr.hot,
            w: hdr.w,
            h: hdr.h,
            codec,
            ok,
            pixels,
        });
    }
    Ok(SpriteBank { count, images })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_image(flags: u16, w: u16, h: u16, fill: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&flags.to_le_bytes());
        v.extend_from_slice(&(w as i16).to_le_bytes());
        v.extend_from_slice(&(h as i16).to_le_bytes());
        v.extend(std::iter::repeat_n(fill, w as usize * h as usize));
        v
    }

    fn bank(images: &[Vec<u8>]) -> Vec<u8> {
        // directory offset convention: start = slot + off, so off = target - slot
        let count = images.len() as u16;
        let mut abs = 2 + images.len() * 4;
        let mut abs_pos = Vec::new();
        for img in images {
            abs_pos.push(abs);
            abs += img.len();
        }
        let mut v = count.to_le_bytes().to_vec();
        for (i, pos) in abs_pos.iter().enumerate() {
            v.extend_from_slice(&((pos - (2 + i * 4)) as u32).to_le_bytes());
        }
        for img in images {
            v.extend_from_slice(img);
        }
        v
    }

    #[test]
    fn parse_raw_and_empty_slots() {
        let mut data = bank(&[raw_image(0, 3, 2, 0xAB), vec![0u8; 4]]);
        data.extend_from_slice(&[0xEE, 0xFF]); // trailing slack
        let b = parse_bin_images(&data).unwrap();
        assert_eq!(b.count, 2);
        assert_eq!(b.images.len(), 2);
        let a = &b.images[0];
        assert_eq!((a.w, a.h), (3, 2));
        assert_eq!(a.codec, "raw");
        assert!(a.ok);
        assert_eq!(a.pixels.as_deref(), Some(&[0xABu8; 6][..]));
        // second entry: 4 header bytes present but w=h=0 -> empty-slot
        let c = &b.images[1];
        assert_eq!(c.codec, "empty-slot");
        assert!(c.ok);
        assert!(c.pixels.is_none());
    }

    #[test]
    fn parse_hotspot_header() {
        let mut img = Vec::new();
        img.extend_from_slice(&2u16.to_le_bytes()); // flags: has hot
        img.extend_from_slice(&(-3i16).to_le_bytes()); // hot x
        img.extend_from_slice(&7i16.to_le_bytes()); // hot y
        img.extend_from_slice(&2i16.to_le_bytes()); // w
        img.extend_from_slice(&1i16.to_le_bytes()); // h
        img.extend_from_slice(&[9, 8]);
        let data = bank(&[img]);
        let b = parse_bin_images(&data).unwrap();
        assert_eq!(b.images[0].hot, Some((-3, 7)));
        assert_eq!((b.images[0].w, b.images[0].h), (2, 1));
        assert_eq!(b.images[0].pixels.as_deref(), Some(&[9u8, 8][..]));
    }

    #[test]
    fn parse_rle16_image() {
        // one row: skip 1, literal 2 (0x11 0x22 inline), EOL
        let mut img = Vec::new();
        img.extend_from_slice(&1u16.to_le_bytes()); // flags: rle16
        img.extend_from_slice(&3i16.to_le_bytes()); // w
        img.extend_from_slice(&1i16.to_le_bytes()); // h
        img.extend_from_slice(&0x8001u16.to_le_bytes()); // skip 1
        img.extend_from_slice(&0x0002u16.to_le_bytes()); // literal 2
        img.extend_from_slice(&[0x11, 0x22]); // literal bytes (inline)
        img.extend_from_slice(&0x4000u16.to_le_bytes()); // EOL
        let data = bank(&[img]);
        let b = parse_bin_images(&data).unwrap();
        assert_eq!(b.images[0].codec, "rle16");
        assert_eq!(b.images[0].pixels.as_deref(), Some(&[0u8, 0x11, 0x22][..]));
    }

    #[test]
    fn missing_header_counts_as_empty_slot() {
        // offset points past EOF: header absent -> empty-slot, ok
        let mut data = vec![1u8, 0];
        data.extend_from_slice(&1000u32.to_le_bytes());
        let b = parse_bin_images(&data).unwrap();
        assert_eq!(b.images[0].codec, "empty-slot");
        assert!(b.images[0].ok);
    }

    #[test]
    fn oversized_dims_rejected_as_no_header() {
        // w = 5000 (> 4096) -> header invalid -> treated as empty slot
        let mut img = Vec::new();
        img.extend_from_slice(&0u16.to_le_bytes());
        img.extend_from_slice(&5000i16.to_le_bytes());
        img.extend_from_slice(&1i16.to_le_bytes());
        let data = bank(&[img]);
        let b = parse_bin_images(&data).unwrap();
        assert_eq!(b.images[0].codec, "empty-slot");
    }

    #[test]
    fn raw_overrun_marks_entry_bad() {
        // header claims 4x4 but only 2 pixel bytes follow
        let mut img = Vec::new();
        img.extend_from_slice(&0u16.to_le_bytes());
        img.extend_from_slice(&4i16.to_le_bytes());
        img.extend_from_slice(&4i16.to_le_bytes());
        img.extend_from_slice(&[1, 2]);
        let data = bank(&[img]);
        let b = parse_bin_images(&data).unwrap();
        assert!(!b.images[0].ok);
        assert_eq!(b.images[0].codec, "raw overrun");
        assert!(b.images[0].pixels.is_none());
    }

    #[test]
    fn rejects_small_and_bad_count() {
        assert_eq!(
            parse_bin_images(&[1, 2, 3]),
            Err(AssetsError::TooSmall { len: 3 })
        );
        assert_eq!(
            parse_bin_images(&[0, 0, 0, 0, 0, 0]),
            Err(AssetsError::CountOverruns { count: 0, len: 6 })
        );
        // count 1000 needs 4002B of directory
        let mut d = vec![0xE8u8, 0x03];
        d.resize(100, 0);
        assert_eq!(
            parse_bin_images(&d),
            Err(AssetsError::CountOverruns {
                count: 1000,
                len: 100
            })
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 99u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 3, 6, 7, 64, 1024, 65536] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_bin_images(&d);
        }
    }
}
