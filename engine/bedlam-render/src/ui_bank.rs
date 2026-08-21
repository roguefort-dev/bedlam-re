//! UI sprite banks — the GENERAL/SMLFONT/NUMBERS/SCANNER .BIN
//! family [RE-EXW-SIM sec 6c.8c, added 2026-08-21].
//!
//! Every UI bank uses the one .BIN layout [RESEARCH-8STREET .BIN
//! row, verified against the shipped bytes]: a directory of `int16
//! count` entries at `2 + 4*id`, each an `int32` offset RELATIVE TO
//! ITS OWN SLOT, the sprite record at `slot + offset`:
//! `{int16 flags; if flags & 2: int16 yhot, int16 xhot; int16 w;
//! int16 h; data}`. The blit is EXW `FUN_00401ca2` [asm
//! 0x401ca2, verified]: flags bit0 = RLE (control `int16` — bit15
//! set = skip run (word & 0xFFF), bit14 set = end-of-line, else a
//! literal run of (word & 0xFFF) bytes), bit0 clear = raw `w*h`
//! rows; the transp flag keeps only nonzero bytes in both modes;
//! flags bit1 adds the hotspot to (x, y). Width/height zero draws
//! nothing (the EXW early-out). A decode that runs off the record
//! simply stops (the EXW trusts the data; we do not).

/// A resolved sprite record (checked, borrowed).
struct UiSprite<'a> {
    flags: u16,
    yhot: i32,
    xhot: i32,
    w: i32,
    h: i32,
    data: &'a [u8],
}

impl<'a> UiSprite<'a> {
    /// Resolve sprite `id` in `bank` — entry `2 + 4*id`, record at
    /// `entry + u32[entry]` [FUN_00401ca2, verified].
    fn resolve(bank: &'a [u8], id: u16) -> Option<UiSprite<'a>> {
        let entry = 2usize + 4 * id as usize;
        let off = u32::from_le_bytes(bank.get(entry..entry + 4)?.try_into().ok()?) as usize;
        let rec = bank.get(entry.checked_add(off)?..)?;
        let word = |i: usize| -> Option<i32> {
            Some(u16::from_le_bytes([*rec.get(2 * i)?, *rec.get(2 * i + 1)?]) as i32)
        };
        let flags = word(0)?;
        let (mut yhot, mut xhot, mut p) = (0, 0, 2usize);
        if flags & 2 != 0 {
            yhot = word(1)?;
            xhot = word(2)?;
            p = 6;
        }
        let w = word(p / 2)?;
        let h = word(p / 2 + 1)?;
        Some(UiSprite {
            flags: flags as u16,
            yhot,
            xhot,
            w,
            h,
            data: rec.get(p + 4..)?,
        })
    }
}

/// The sprite geometry `(w, h, xhot, yhot)` — the layout metric
/// (`FUN_00402a12` is the width-at-record+6 lookup, i.e. these
/// records carry hotspots).
pub fn sprite_geometry(bank: &[u8], id: u16) -> Option<(i32, i32, i32, i32)> {
    let s = UiSprite::resolve(bank, id)?;
    Some((s.w, s.h, s.xhot, s.yhot))
}

/// Draw sprite `id` from `bank` onto `plane` (`stride` wide, row
/// major) at (x, y) plus the record hotspot, honoring the EXW
/// transp flag. Returns whether the sprite resolved with a nonzero
/// extent (the EXW early-out); out-of-plane pixels clip.
pub fn draw_sprite(
    plane: &mut [u8],
    stride: usize,
    bank: &[u8],
    id: u16,
    x: i32,
    y: i32,
    transparent: bool,
) -> bool {
    let Some(s) = UiSprite::resolve(bank, id) else {
        return false;
    };
    if s.w == 0 || s.h == 0 {
        return false;
    }
    let rows = plane.len() / stride;
    let x0 = x + s.xhot;
    let y0 = y + s.yhot;
    let mut set = |px: i32, py: i32, b: u8| {
        if px >= 0 && py >= 0 && (px as usize) < stride && (py as usize) < rows {
            plane[py as usize * stride + px as usize] = b;
        }
    };
    let get = |p: usize| -> Option<u8> { s.data.get(p).copied() };
    if s.flags & 1 != 0 {
        // RLE: control int16 — bit15 skip (word & 0xFFF), bit14
        // end-of-line, else literal run [FUN_00401ca2, verified].
        let mut p = 0usize;
        'rows: for row in 0..s.h {
            let mut col = 0i32;
            loop {
                let (Some(lo), Some(hi)) = (get(p), get(p + 1)) else {
                    break 'rows;
                };
                p += 2;
                let w = u16::from_le_bytes([lo, hi]);
                if w & 0x8000 != 0 {
                    if w & 0x4000 != 0 {
                        break; // end of line
                    }
                    col += i32::from(w & 0x0FFF);
                } else {
                    for _ in 0..usize::from(w & 0x0FFF) {
                        let Some(b) = get(p) else {
                            break 'rows;
                        };
                        p += 1;
                        if b != 0 || !transparent {
                            set(x0 + col, y0 + row, b);
                        }
                        col += 1;
                    }
                }
            }
        }
    } else {
        // Raw w*h rows; the transp flag keeps only nonzero bytes.
        let mut p = 0usize;
        'rows: for row in 0..s.h {
            for col in 0..s.w {
                let Some(b) = get(p) else {
                    break 'rows;
                };
                p += 1;
                if b != 0 || !transparent {
                    set(x0 + col, y0 + row, b);
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One synth sprite: (id, flags, xhot, yhot, w, h, rows).
    struct SynthSprite<'a> {
        id: u16,
        flags: u16,
        xhot: i32,
        yhot: i32,
        w: i32,
        h: i32,
        rows: &'a [u8],
    }

    /// Build a synth bank of `count` sprites; `solid` lists the
    /// non-empty records. The directory layout mirrors the shipped
    /// banks: every entry's offset is relative to its own slot.
    fn synth_bank(count: u16, solid: &[SynthSprite<'_>]) -> Vec<u8> {
        let mut bank = vec![0u8; 2 + 4 * count as usize];
        bank[0..2].copy_from_slice(&count.to_le_bytes());
        for s in solid {
            let entry = 2 + 4 * s.id as usize;
            let mut rec = Vec::new();
            rec.extend_from_slice(&s.flags.to_le_bytes());
            if s.flags & 2 != 0 {
                rec.extend_from_slice(&(s.yhot as u16).to_le_bytes());
                rec.extend_from_slice(&(s.xhot as u16).to_le_bytes());
            }
            rec.extend_from_slice(&(s.w as u16).to_le_bytes());
            rec.extend_from_slice(&(s.h as u16).to_le_bytes());
            rec.extend_from_slice(s.rows);
            // Offsets are relative to the slot, so append the record
            // and store (record start - entry).
            let start = bank.len();
            bank.extend_from_slice(&rec);
            let off = (start as u32) - entry as u32;
            bank[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
        }
        bank
    }

    #[test]
    fn raw_sprite_blits_with_transparency_and_hotspot() {
        // 3x2 raw sprite with flags 3 (hotspot + RLE bit set is NOT
        // used here: flags 2 = hotspot only), pixel values 1,0,2 /
        // 3,0,4; hotspot (+10, +1).
        let rows: &[u8] = &[1, 0, 2, 3, 0, 4];
        let bank = synth_bank(
            4,
            &[SynthSprite {
                id: 2,
                flags: 2,
                xhot: 10,
                yhot: 1,
                w: 3,
                h: 2,
                rows,
            }],
        );
        assert_eq!(sprite_geometry(&bank, 2), Some((3, 2, 10, 1)));
        let mut plane = vec![0u8; 8 * 16];
        assert!(draw_sprite(&mut plane, 16, &bank, 2, 5, 7, true));
        // (5,7) + hotspot (10,1) = (15,8) — off-plane, clipped.
        assert!(plane.iter().all(|&b| b == 0));
        // At (0,0): lands at (10,1); transparent drops the zeros.
        let mut plane = vec![0u8; 8 * 16];
        assert!(draw_sprite(&mut plane, 16, &bank, 2, 0, 0, true));
        assert_eq!(plane[16 + 10], 1);
        assert_eq!(plane[16 + 12], 2);
        assert_eq!(plane[2 * 16 + 10], 3);
        assert_eq!(plane[2 * 16 + 12], 4);
        assert_eq!(plane[16 + 11], 0, "transparent zero skipped");
        // Opaque: zeros overwrite too.
        let mut plane = vec![9u8; 8 * 16];
        assert!(draw_sprite(&mut plane, 16, &bank, 2, -10, -1, false));
        assert_eq!(plane[0], 1);
        assert_eq!(plane[1], 0, "opaque writes the zero byte");
        assert_eq!(plane[2], 2);
    }

    #[test]
    fn rle_sprite_skips_runs_and_ends_lines() {
        // flags 1 (RLE, no hotspot): w=4 h=2.
        // Row 0: skip 1, literal 2 (A,B), EOL.
        // Row 1: literal 1 (C), skip 2, EOL.
        let rows: &[u8] = &[
            0x01, 0x80, // skip 1
            0x02, 0x00, 0xAA, 0xBB, // literal 2
            0x00, 0xC0, // EOL
            0x01, 0x00, 0xCC, // literal 1
            0x02, 0x80, // skip 2
            0x00, 0xC0, // EOL
        ];
        let bank = synth_bank(
            1,
            &[SynthSprite {
                id: 0,
                flags: 1,
                xhot: 0,
                yhot: 0,
                w: 4,
                h: 2,
                rows,
            }],
        );
        let mut plane = vec![0u8; 4 * 2];
        assert!(draw_sprite(&mut plane, 4, &bank, 0, 0, 0, true));
        assert_eq!(plane, vec![0, 0xAA, 0xBB, 0, 0xCC, 0, 0, 0]);
    }

    #[test]
    fn zero_extent_and_missing_ids_early_out() {
        let bank = synth_bank(
            2,
            &[
                SynthSprite {
                    id: 0,
                    flags: 3,
                    xhot: 0,
                    yhot: 0,
                    w: 0,
                    h: 5,
                    rows: &[9; 5],
                },
                SynthSprite {
                    id: 1,
                    flags: 3,
                    xhot: 0,
                    yhot: 0,
                    w: 5,
                    h: 0,
                    rows: &[],
                },
            ],
        );
        let mut plane = vec![0u8; 4];
        assert!(!draw_sprite(&mut plane, 4, &bank, 0, 0, 0, true), "w=0");
        assert!(!draw_sprite(&mut plane, 4, &bank, 1, 0, 0, true), "h=0");
        assert!(!draw_sprite(&mut plane, 4, &bank, 9, 0, 0, true), "no id 9");
        assert!(
            !draw_sprite(&mut plane, 4, &[], 0, 0, 0, true),
            "short bank"
        );
        assert!(plane.iter().all(|&b| b == 0));
    }

    #[test]
    fn decode_running_off_the_record_stops_clean() {
        // RLE sprite claiming w=4 h=4 but only one literal run of 1.
        let rows: &[u8] = &[0x01, 0x00, 0x77];
        let bank = synth_bank(
            1,
            &[SynthSprite {
                id: 0,
                flags: 1,
                xhot: 0,
                yhot: 0,
                w: 4,
                h: 4,
                rows,
            }],
        );
        let mut plane = vec![0u8; 4 * 4];
        assert!(draw_sprite(&mut plane, 4, &bank, 0, 0, 0, true));
        assert_eq!(plane[0], 0x77);
        assert_eq!(plane.iter().filter(|&&b| b != 0).count(), 1);
    }

    /// The shipped banks (corpus-gated, skipped on CI): the sidebar
    /// geometry the EXW draws with — GENERAL.BIN portraits 0x12..0x17
    /// 48x48, row body 0x47/0x49 108x11, count well 0x4A/0x4C 27x11,
    /// SMLFONT 63 glyphs 5x7 [RE-EXW-SIM 6c.8].
    #[test]
    fn shipped_sidebar_geometry_matches_the_decode() {
        let Ok(general) = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../game-data/BEDLAM/GAMEGFX/GENERAL.BIN"),
        ) else {
            eprintln!("corpus absent - skipping (CI)");
            return;
        };
        let count = u16::from_le_bytes([general[0], general[1]]);
        assert_eq!(count, 153, "the 8street GENERAL.BIN census");
        for id in [0x12u16, 0x13, 0x14, 0x15, 0x16, 0x17] {
            assert_eq!(
                sprite_geometry(&general, id),
                Some((48, 48, 0, 0)),
                "portrait {id:#x}"
            );
        }
        for id in [0x47u16, 0x49] {
            assert_eq!(
                sprite_geometry(&general, id),
                Some((108, 11, 0, 0)),
                "row body {id:#x}"
            );
        }
        for id in [0x4Au16, 0x4C] {
            assert_eq!(
                sprite_geometry(&general, id),
                Some((27, 11, 0, 0)),
                "count well {id:#x}"
            );
        }
        // The row body actually decodes into the plane (RLE flags 3).
        let mut plane = vec![0u8; 108 * 11];
        assert!(draw_sprite(&mut plane, 108, &general, 0x49, 0, 0, true));
        assert!(
            plane.iter().any(|&b| b != 0),
            "the unarmed row body carries pixels"
        );
    }
}
