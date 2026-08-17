//! CGR tile banks: `u16` count, `count` `u32` offsets relative to the end of
//! each slot's directory entry. Per tile: if the first `u16` is >= 4 the tile
//! is a 32-wide byte-RLE block whose row count sits at `start+8`; otherwise it
//! is a raw `tw`x`th` block after a 6-byte mini-header.

use crate::codecs::decode_byterle;
use crate::{u16le, u32le, AssetsError};

/// One entry of a CGR tile bank.
///
/// When `short` is true the tile header itself lay past EOF and only
/// `{i, ok:false, why:"short"}` is reported. `codec` mirrors the inspect
/// CLI's per-tile field: `"byterle"`, `"raw"`, `"hdr rows=N"`,
/// `"raw dims WxH"`, or a decode-failure description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub off: u32,
    pub w0: u16,
    pub ok: bool,
    pub codec: String,
    pub w: usize,
    pub h: usize,
    pub pixels: Option<Vec<u8>>,
    pub short: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBank {
    pub count: usize,
    pub tiles: Vec<Tile>,
}

/// Parse a CGR tile bank. Bad tiles do not fail the parse; only a truncated
/// file or bogus header count is an `Err`.
///
/// Note: when `w0 >= 4` but fewer than 10 bytes remain at the tile start the
/// row-count field is unreadable; the tile is reported as `hdr rows=0`
/// (the legacy tool would have crashed reading it).
pub fn parse_cgr_tiles(data: &[u8]) -> Result<TileBank, AssetsError> {
    if data.len() < 12 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let count = u16le(data, 0) as usize;
    if count == 0 || 2 + count * 4 > data.len() {
        return Err(AssetsError::CountOverruns {
            count,
            len: data.len(),
        });
    }
    let mut tiles = Vec::with_capacity(count);
    for i in 0..count {
        let slot = 2 + i * 4;
        let off = u32le(data, slot);
        let start = slot + off as usize;
        if start + 6 > data.len() {
            tiles.push(Tile {
                off,
                w0: 0,
                ok: false,
                codec: String::new(),
                w: 0,
                h: 0,
                pixels: None,
                short: true,
            });
            continue;
        }
        let w0 = u16le(data, start);
        if w0 >= 4 {
            let rows = if start + 10 <= data.len() {
                u16le(data, start + 8) as usize
            } else {
                0
            };
            if rows == 0 || rows > 64 {
                tiles.push(Tile {
                    off,
                    w0,
                    ok: false,
                    codec: format!("hdr rows={}", rows),
                    w: 32,
                    h: rows,
                    pixels: None,
                    short: false,
                });
            } else {
                let (pixels, codec) = match decode_byterle(&data[start + 10..], 32, rows) {
                    Ok(px) => (Some(px), String::from("byterle")),
                    Err(e) => (None, e.to_string()),
                };
                let ok = pixels.is_some();
                tiles.push(Tile {
                    off,
                    w0,
                    ok,
                    codec,
                    w: 32,
                    h: rows,
                    pixels,
                    short: false,
                });
            }
        } else {
            let tw = u16le(data, start + 2) as usize;
            let th = u16le(data, start + 4) as usize;
            if tw == 0 || th == 0 || tw > 256 || th > 256 || start + 6 + tw * th > data.len() {
                tiles.push(Tile {
                    off,
                    w0,
                    ok: false,
                    codec: format!("raw dims {}x{}", tw, th),
                    w: tw,
                    h: th,
                    pixels: None,
                    short: false,
                });
            } else {
                tiles.push(Tile {
                    off,
                    w0,
                    ok: true,
                    codec: String::from("raw"),
                    w: tw,
                    h: th,
                    pixels: Some(data[start + 6..start + 6 + tw * th].to_vec()),
                    short: false,
                });
            }
        }
    }
    Ok(TileBank { count, tiles })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank(tiles: &[Vec<u8>]) -> Vec<u8> {
        // directory offset convention: start = slot + off, so off = target - slot
        let count = tiles.len() as u16;
        let mut abs = 2 + tiles.len() * 4;
        let mut abs_pos = Vec::new();
        for t in tiles {
            abs_pos.push(abs);
            abs += t.len();
        }
        let mut v = count.to_le_bytes().to_vec();
        for (i, pos) in abs_pos.iter().enumerate() {
            v.extend_from_slice(&((pos - (2 + i * 4)) as u32).to_le_bytes());
        }
        for t in tiles {
            v.extend_from_slice(t);
        }
        v
    }

    fn byterle_tile(rows: usize, body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&32u16.to_le_bytes()); // w0 >= 4 branch
        v.extend_from_slice(&[0u8; 6]); // pad so rows lands at start+8
        v.extend_from_slice(&(rows as u16).to_le_bytes());
        v.extend_from_slice(body); // rle body at start+10
        v
    }

    fn raw_tile(tw: u16, th: u16, body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&3u16.to_le_bytes()); // w0 < 4 branch
        v.extend_from_slice(&tw.to_le_bytes());
        v.extend_from_slice(&th.to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn parse_byterle_tile() {
        // two rows of 32 zeros: skip-32 + EOL twice
        let body = [0x9Fu8, 0x40, 0x9F, 0x40]; // 0x80|31 -> skip 32
        let data = bank(&[byterle_tile(2, &body)]);
        let b = parse_cgr_tiles(&data).unwrap();
        assert_eq!(b.count, 1);
        let t = &b.tiles[0];
        assert_eq!(t.codec, "byterle");
        assert_eq!((t.w, t.h), (32, 2));
        assert!(t.ok);
        assert_eq!(t.pixels.as_deref(), Some(&[0u8; 64][..]));
    }

    #[test]
    fn parse_raw_tile() {
        let data = bank(&[raw_tile(4, 2, &[1, 2, 3, 4, 5, 6, 7, 8])]);
        let b = parse_cgr_tiles(&data).unwrap();
        let t = &b.tiles[0];
        assert_eq!(t.codec, "raw");
        assert_eq!((t.w, t.h), (4, 2));
        assert_eq!(t.pixels.as_deref(), Some(&[1u8, 2, 3, 4, 5, 6, 7, 8][..]));
    }

    #[test]
    fn rows_out_of_range_reported() {
        let data = bank(&[byterle_tile(65, &[])]);
        let b = parse_cgr_tiles(&data).unwrap();
        assert!(!b.tiles[0].ok);
        assert_eq!(b.tiles[0].codec, "hdr rows=65");
        assert_eq!(b.tiles[0].w, 32);
    }

    #[test]
    fn short_tile_reported() {
        // two slots: the second points far past EOF -> short
        let mut data = 2u16.to_le_bytes().to_vec();
        data.extend_from_slice(&8u32.to_le_bytes()); // slot0 -> raw 2x2 tile at abs 10
        data.extend_from_slice(&5000u32.to_le_bytes()); // slot1 -> way out
        data.extend_from_slice(&3u16.to_le_bytes()); // w0 < 4
        data.extend_from_slice(&2u16.to_le_bytes()); // tw
        data.extend_from_slice(&2u16.to_le_bytes()); // th
        data.extend_from_slice(&[1, 2, 3, 4]);
        let b = parse_cgr_tiles(&data).unwrap();
        assert!(b.tiles[0].ok);
        assert!(b.tiles[1].short);
        assert!(!b.tiles[1].ok);
    }

    #[test]
    fn bad_raw_dims_reported() {
        let data = bank(&[raw_tile(300, 2, &[0; 8])]); // tw > 256
        let b = parse_cgr_tiles(&data).unwrap();
        assert!(!b.tiles[0].ok);
        assert_eq!(b.tiles[0].codec, "raw dims 300x2");
    }

    #[test]
    fn rejects_small_and_bad_count() {
        assert_eq!(
            parse_cgr_tiles(&[0u8; 11]),
            Err(AssetsError::TooSmall { len: 11 })
        );
        let mut d = vec![0u8; 12];
        d[0] = 2; // count=2 needs 10B dir, has 12B -> ok actually
        assert!(parse_cgr_tiles(&d).is_ok());
        d[0] = 200;
        d[1] = 0; // count=200 needs 802B
        assert_eq!(
            parse_cgr_tiles(&d),
            Err(AssetsError::CountOverruns {
                count: 200,
                len: 12
            })
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 5u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 11, 12, 13, 256, 4096] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_cgr_tiles(&d);
        }
    }
}
