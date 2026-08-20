//! The EXW loading-text font drawer (P5, D35): FUN_0043c87c and its
//! callees, reproduced over the validated sprites parser. All facts
//! verified from the Ghidra listing (ghidra-project/exw-font-drawer.txt)
//! plus objdump of FUN_0042471f and the FUN_00410493 stub table,
//! 2026-08-20; corpus pinned in bedlam-assets tests/font_gate.rs.
//!
//! Drawer semantics (FUN_0043c87c(EAX=str, EDX=bank, EBX=y, ECX=base)):
//! - two passes over the NUL-terminated string: measure, then draw;
//! - per byte c: c >= 0x80 first remaps through FUN_00410493 to a
//!   base ASCII char plus an accent id; then k = char - 0x21;
//!   k < 0 (space / control) advances the pen 9 px; else the glyph at
//!   entry base + k blits (RLE16, transparent: zero source bytes are
//!   skipped - FUN_00401ca2 EDX=1 path) and the pen advances w + 2
//!   (FUN_00402a12: w = u16 at entry + 6 in the flags-0x0003 layout);
//! - hotspot (flags bit 1): u16@+2 adds to the DEST ROW, u16@+4 to
//!   the DEST COLUMN - lowercase glyphs carry dy=5, mid punctuation
//!   dy=10, low punctuation dy=15 (baseline anchoring);
//! - x0 = 0x140 - total/2 (screen center 320, SAR by 1);
//! - an accent id 1..=4 additionally blits the overlay glyph at
//!   entry base + 0x6b + id (238 diaeresis, 239 acute, 240 grave,
//!   241 circumflex) at the SAME pen position, before the advance;
//! - blits write directly through the surface pointer in EXW; here
//!   every pixel is bounds-clipped to the plane [deviation: Rust
//!   never writes out of bounds; corpus draws fit the 640x480 plane].
//!
//! The FUN_00410493 remap (match table 0x4103f4 scanned by REPNE
//! SCASB, jumptable 0x410413 indexed by 31 - match_index): k = c-0x80
//! maps to (base char, accent id). Cross-validated against the CP437
//! accent set the corpus actually uses: acute a/e/i/o/u, grave
//! a/e/u, circumflex a/e/i/o/u, diaeresis a/i/u, c-cedilla ->
//! glyph 0x81, sharp-s -> glyph 0x80. Two shipped quirks kept
//! verbatim [verified: stub bodies objdump 0x4104c0..0x410650,
//! 2026-08-20]: CP437 e-diaeresis (0x89, k=0x09) and
//! o-diaeresis (0x94, k=0x14) have NO letter+diaeresis stubs -
//! their stubs (0x410616 / 0x4105eb) leave EDX at the prologue
//! default 0x2d, so EXW draws the dash glyph under the diaeresis
//! overlay.

use bedlam_assets::sprites::parse_bin_images;

use crate::GameError;

/// Glyph entry base of the loading font [verified: ECX=0x82 on all
/// four LAB_0041c69e draws; entry 130 = 0x21 in the corpus].
pub(crate) const GLYPH_BASE: usize = 0x82;

/// Entry offset of the accent overlays from the glyph base
/// [verified: the drawer adds 0x6b to the remapped char].
pub(crate) const ACCENT_GLYPH_OFF: usize = 0x6b;

/// First / last byte the glyph table covers: chars 0x21..=0x81 map
/// to entries GLYPH_BASE..=GLYPH_BASE + 0x60 (0x7f..=0x81 are the
/// extended glyphs the remap returns for c-cedilla / sharp-s).
const FIRST_CHAR: u8 = 0x21;
const LAST_CHAR: u8 = 0x81;

/// Advance for bytes below 0x21 (space and controls) [verified: pen
/// += 9].
pub(crate) const SPACE_ADVANCE: i32 = 9;

/// Extra pen advance after each glyph [verified: pen += w + 2].
pub(crate) const GLYPH_GAP: i32 = 2;

/// Screen center the row is centered on [verified: x0 = 0x140 -
/// total/2].
pub(crate) const CENTER_X: i32 = 0x140;

/// One decoded glyph: raster plus its blit anchor (dy adds to the
/// dest row, dx to the dest column - the sprites-parser hotspot in
/// EXW blit order).
#[derive(Debug, Clone)]
struct Glyph {
    w: i32,
    h: i32,
    dy: i32,
    dx: i32,
    pixels: Box<[u8]>,
}

impl Glyph {
    /// Extract entry `entry` of a parsed bank; None for empty or
    /// undecodable entries (the drawer then skips the glyph - EXW
    /// would blit whatever the slot decodes to; empty slots decode
    /// to nothing there too).
    fn from_bank(bank: &bedlam_assets::sprites::SpriteBank, entry: usize) -> Option<Glyph> {
        let im = bank.images.get(entry)?;
        let px = im.pixels.as_deref()?;
        let (dy, dx) = im.hot.unwrap_or((0, 0));
        Some(Glyph {
            w: i32::from(im.w),
            h: i32::from(im.h),
            dy: i32::from(dy),
            dx: i32::from(dx),
            pixels: px.to_vec().into_boxed_slice(),
        })
    }
}

/// The FUN_00410493 accent remap: (base char, accent id). Accent ids
/// select the overlay glyph ACCENT_GLYPH_OFF + id: 1 = diaeresis,
/// 2 = acute, 3 = grave, 4 = circumflex; 0 = none. Unlisted k values
/// (including k > 0x78) fall through to the dash + diaeresis arm the
/// prologue leaves in EDX / DAT_0046ccd0.
pub(crate) fn remap_high(c: u8) -> (u8, u8) {
    match c.wrapping_sub(0x80) {
        0x00 => (0x80, 0), // sharp-s glyph
        0x01 => (0x75, 1), // u + diaeresis
        0x02 => (0x65, 2), // e + acute
        0x03 => (0x61, 4), // a + circumflex
        0x04 => (0x61, 1), // a + diaeresis
        0x05 => (0x61, 3), // a + grave
        0x07 => (0x81, 0), // c-cedilla glyph
        0x08 => (0x65, 4), // e + circumflex
        0x09 => (0x2d, 1), // QUIRK: e-diaeresis -> dash + diaeresis (stub 0x410616)
        0x0a => (0x65, 3), // e + grave
        0x0b => (0x69, 1), // i + diaeresis
        0x0c => (0x69, 4), // i + circumflex
        0x0d => (0x69, 3), // i + grave
        0x0e => (0x67, 1), // g + diaeresis
        0x13 => (0x6f, 4), // o + circumflex
        0x14 => (0x2d, 1), // QUIRK: o-diaeresis -> dash + diaeresis
        0x15 => (0x6f, 3), // o + grave
        0x16 => (0x75, 4), // u + circumflex
        0x17 => (0x75, 3), // u + grave
        0x19 => (0x2d, 1), // unmapped -> dash + diaeresis
        0x1a => (0x75, 1), // u + diaeresis (alt code)
        0x20 => (0x61, 2), // a + acute
        0x21 => (0x69, 2), // i + acute
        0x22 => (0x6f, 2), // o + acute
        0x23 => (0x75, 2), // u + acute
        0x25 => (0x7c, 0), // extended glyph (pipe slot)
        0x27 => (0x2d, 1), // unmapped -> dash + diaeresis
        0x28 => (0x7e, 0), // inverted question mark (tilde slot)
        0x2d => (0x7f, 0), // inverted exclamation glyph
        0x61 => (0x7b, 0), // sharp-s (brace slot)
        0x78 => (0x2d, 1), // unmapped -> dash + diaeresis
        _ => (0x2d, 1),    // prologue default: dash, shadow still 1
    }
}

/// The loading font: glyph entries for chars 0x21..=0x81 plus the
/// four accent overlays.
#[derive(Debug, Default)]
pub(crate) struct LoadingFont {
    glyphs: Vec<Option<Glyph>>,  // indexed by c - FIRST_CHAR
    accents: [Option<Glyph>; 4], // indexed by accent id - 1
}

impl LoadingFont {
    /// Extract the drawer tables from a FULLFONT.BIN image. The bank
    /// parses through the generic sprites decoder; a bank that
    /// rejects, or holds no glyph at entry GLYPH_BASE, is a staging
    /// error. Missing individual glyphs stay None (skipped at draw).
    pub(crate) fn from_bank(bin: &[u8]) -> Result<LoadingFont, GameError> {
        let bank = parse_bin_images(bin).map_err(GameError::Assets)?;
        if bank.count < GLYPH_BASE + ACCENT_GLYPH_OFF + 4 {
            return Err(GameError::BadLoadingAsset {
                what: "font bank",
                reason: "too few entries for the loading font",
            });
        }
        let first = Glyph::from_bank(&bank, GLYPH_BASE).ok_or(GameError::BadLoadingAsset {
            what: "font bank",
            reason: "entry 0x82 (first glyph) undecoded",
        })?;
        let mut glyphs = Vec::with_capacity((LAST_CHAR - FIRST_CHAR + 1) as usize);
        glyphs.push(Some(first));
        for entry in GLYPH_BASE + 1..=GLYPH_BASE + (LAST_CHAR - FIRST_CHAR) as usize {
            glyphs.push(Glyph::from_bank(&bank, entry));
        }
        let accents = [
            Glyph::from_bank(&bank, GLYPH_BASE + ACCENT_GLYPH_OFF + 1),
            Glyph::from_bank(&bank, GLYPH_BASE + ACCENT_GLYPH_OFF + 2),
            Glyph::from_bank(&bank, GLYPH_BASE + ACCENT_GLYPH_OFF + 3),
            Glyph::from_bank(&bank, GLYPH_BASE + ACCENT_GLYPH_OFF + 4),
        ];
        Ok(LoadingFont { glyphs, accents })
    }

    fn glyph_for(&self, c: u8) -> Option<&Glyph> {
        if !(FIRST_CHAR..=LAST_CHAR).contains(&c) {
            return None;
        }
        self.glyphs[(c - FIRST_CHAR) as usize].as_ref()
    }

    /// Resolve one byte to (effective char, glyph, accent id) per the
    /// drawer walk: high bytes remap first, then the effective char
    /// indexes the table.
    fn resolve(&self, c: u8) -> (u8, Option<&Glyph>, u8) {
        let (eff, accent) = if c >= 0x80 { remap_high(c) } else { (c, 0) };
        (eff, self.glyph_for(eff), accent)
    }

    /// The measure pass: total pen advance of the string. An
    /// effective char below 0x21 (space / control bytes; remap
    /// bases are always >= 0x2d) advances SPACE_ADVANCE; anything
    /// else advances slot_width + GLYPH_GAP - EXW reads the width
    /// from the bank slot, and an empty slot reads 0, so a missing
    /// glyph still advances GLYPH_GAP [verified: FUN_0043c87c
    /// measure loop, ADD EBP,0x9 vs ADD EAX,0x2 after the
    /// FUN_00402a12 width read].
    pub(crate) fn measure(&self, text: &[u8]) -> i32 {
        let mut total = 0;
        for &c in text {
            let (eff, glyph, _) = self.resolve(c);
            total += match glyph {
                Some(g) => g.w + GLYPH_GAP,
                None if eff < FIRST_CHAR => SPACE_ADVANCE,
                None => GLYPH_GAP,
            };
        }
        total
    }

    /// Pen start for a centered draw [verified: 0x140 - total/2].
    pub(crate) fn pen_start(&self, text: &[u8]) -> i32 {
        CENTER_X - self.measure(text) / 2
    }

    /// The draw pass: blit the string onto `plane` (row-major,
    /// `stride` bytes per row) at row `y`, centered. Zero source
    /// pixels are skipped (transparent blit); every destination
    /// pixel is bounds-clipped.
    pub(crate) fn draw(&self, plane: &mut [u8], stride: usize, text: &[u8], y: i32) {
        let rows = (plane.len() / stride) as i32;
        let mut pen = self.pen_start(text);
        for &c in text {
            let (eff, glyph, accent) = self.resolve(c);
            match glyph {
                None => {
                    pen += if eff < FIRST_CHAR {
                        SPACE_ADVANCE
                    } else {
                        GLYPH_GAP
                    }
                }
                Some(g) => {
                    blit(plane, stride, rows, g, pen, y);
                    if accent != 0 {
                        if let Some(ov) = &self.accents[accent as usize - 1] {
                            blit(plane, stride, rows, ov, pen, y);
                        }
                    }
                    pen += g.w + GLYPH_GAP;
                }
            }
        }
    }
}

/// One transparent, clipped blit of `g` at (x, y) + its anchor.
fn blit(plane: &mut [u8], stride: usize, rows: i32, g: &Glyph, x: i32, y: i32) {
    for r in 0..g.h {
        let dy = y + g.dy + r;
        if dy < 0 || dy >= rows {
            continue;
        }
        for c in 0..g.w {
            let dx = x + g.dx + c;
            if dx < 0 || dx >= stride as i32 {
                continue;
            }
            let v = g.pixels[(r * g.w + c) as usize];
            if v != 0 {
                plane[dy as usize * stride + dx as usize] = v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Raw single-glyph image bytes (flags 0 = raw raster, no
    /// hotspot), w x h filled with `fill` except a zero first column.
    fn glyph_bytes(w: u16, h: u16, fill: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&(w as i16).to_le_bytes());
        v.extend_from_slice(&(h as i16).to_le_bytes());
        for _r in 0..h {
            for c in 0..w {
                v.push(if c == 0 { 0 } else { fill });
            }
        }
        v
    }

    /// Hotspot glyph (flags 2 = raw + hotspot), solid fill.
    fn hotspot_glyph_bytes(w: u16, h: u16, dy: i16, dx: i16, fill: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&dy.to_le_bytes());
        v.extend_from_slice(&dx.to_le_bytes());
        v.extend_from_slice(&(w as i16).to_le_bytes());
        v.extend_from_slice(&(h as i16).to_le_bytes());
        v.extend(std::iter::repeat_n(fill, w as usize * h as usize));
        v
    }

    /// Bank of `count` slots where entries `entries` (in order) hold
    /// the given images and every other slot is an empty (zeroed
    /// header) slot at the end of the file.
    fn bank(count: usize, entries: &[usize], images: &[Vec<u8>]) -> Vec<u8> {
        assert_eq!(entries.len(), images.len());
        let mut starts = Vec::new();
        let mut data_start = 2 + count * 4;
        for img in images {
            starts.push(data_start);
            data_start += img.len();
        }
        let empty_start = data_start;
        let mut v = (count as u16).to_le_bytes().to_vec();
        for entry in 0..count {
            let off = match entries.iter().position(|e| *e == entry) {
                Some(k) => (starts[k] - (2 + entry * 4)) as u32,
                None => (empty_start - (2 + entry * 4)) as u32,
            };
            v.extend_from_slice(&off.to_le_bytes());
        }
        for img in images {
            v.extend_from_slice(img);
        }
        v.extend(std::iter::repeat_n(0u8, 8)); // zeroed header: empty slot
        v
    }

    /// Synth font: 0x21 glyph 3x2 fill 0xF0 (first column zero) at
    /// entry 0x82, 0x22 glyph 4x2 fill 0xF1 at 0x83, and the
    /// diaeresis overlay (2x1 fill 0xF2, dy=-1) at 0x82+0x6b+1.
    fn synth_font() -> LoadingFont {
        let count = GLYPH_BASE + ACCENT_GLYPH_OFF + 5;
        let images = vec![
            glyph_bytes(3, 2, 0xF0),
            glyph_bytes(4, 2, 0xF1),
            hotspot_glyph_bytes(2, 1, -1, 0, 0xF2),
        ];
        let entries = vec![
            GLYPH_BASE,
            GLYPH_BASE + 1,
            GLYPH_BASE + ACCENT_GLYPH_OFF + 1,
        ];
        let data = bank(count, &entries, &images);
        LoadingFont::from_bank(&data).unwrap()
    }

    #[test]
    fn remap_matches_the_stub_table() {
        // CP437 accents the corpus strings actually use.
        assert_eq!(remap_high(0x82), (0x65, 2), "e acute (FRE)");
        assert_eq!(remap_high(0x8A), (0x65, 3), "e grave");
        assert_eq!(remap_high(0x83), (0x61, 4), "a circumflex");
        assert_eq!(remap_high(0x84), (0x61, 1), "a diaeresis");
        assert_eq!(remap_high(0x81), (0x75, 1), "u diaeresis (GER)");
        assert_eq!(remap_high(0x94), (0x2d, 1), "o diaeresis quirk (GER)");
        assert_eq!(
            remap_high(0x89),
            (0x2d, 1),
            "e diaeresis quirk (stub 0x410616)"
        );
        assert_eq!(remap_high(0xA0), (0x61, 2), "a acute (SPA)");
        assert_eq!(remap_high(0xA2), (0x6f, 2), "o acute (SPA)");
        assert_eq!(remap_high(0xAD), (0x7f, 0), "inverted bang (SPA)");
        assert_eq!(remap_high(0x87), (0x81, 0), "c cedilla");
        assert_eq!(remap_high(0xE1), (0x7b, 0), "sharp s (GER city centre)");
        assert_eq!(remap_high(0xFF), (0x2d, 1), "out of range default");
        // The wrapper never calls this for c < 0x80, but the table is
        // total: k = 0 means c = 0x80 exactly.
        assert_eq!(remap_high(0x80), (0x80, 0));
    }

    #[test]
    fn measure_and_pen_follow_the_drawer_rule() {
        let f = synth_font();
        // 0x21 w=3 -> 5 per glyph; space 9; 0x22 w=4 -> 6; 0x61 has
        // no glyph in the synth bank -> GLYPH_GAP only.
        assert_eq!(f.measure(&[0x21]), 5);
        assert_eq!(f.measure(&[0x21, 0x20, 0x21]), 5 + 9 + 5);
        assert_eq!(f.measure(&[0x21, 0x22]), 5 + 6);
        assert_eq!(f.measure(&[0x61]), GLYPH_GAP, "empty slot: gap only");
        assert_eq!(f.measure(&[0x7F]), GLYPH_GAP, "table char, empty slot");
        assert_eq!(f.measure(&[0x01]), SPACE_ADVANCE, "control byte");
        assert_eq!(f.measure(&[0x20]), SPACE_ADVANCE, "space byte");
        // e-acute remaps to base e (empty slot in the synth bank): the
        // advance follows the EFFECTIVE char - slot width 0 + gap.
        assert_eq!(f.measure(&[0x82]), GLYPH_GAP);
        // An out-of-range high byte remaps to dash (0x2d): empty slot
        // in the synth bank -> gap, NOT the space advance.
        assert_eq!(f.measure(&[0xFF]), GLYPH_GAP);
        assert_eq!(f.pen_start(&[0x21, 0x20, 0x21]), CENTER_X - 19 / 2);
    }

    #[test]
    fn draw_blits_transparent_clipped_and_overlaid() {
        let f = synth_font();
        // 4x3 plane: 0x21 (3x2, first column zero) at pen CENTER_X-2
        // is far right -> fully clipped; draw with a tiny measure by
        // using draw at stride 4 on a 4-wide plane via a 1-glyph font.
        let mut plane = vec![0u8; 12];
        f.draw(&mut plane, 4, &[0x21], 0);
        // pen = 320 - 5/2 = 318 -> everything off-plane.
        assert!(
            plane.iter().all(|&v| v == 0),
            "centered pen off a 4px plane clips"
        );
        // Force on-plane coverage with bracketing spaces that land the
        // pen exactly at 0: 5 + 9*70 + 5 = 640 -> pen = 320 - 320 = 0.
        // Glyph 0x21 is 3 wide with a ZERO first column -> dest cols
        // 1..2 carry the fill; the trailing glyph sits at pen 635,
        // fully clipped off the 4-wide plane.
        let mut text = vec![0x21u8];
        text.extend(std::iter::repeat_n(0x20, 70));
        text.push(0x21);
        let mut plane = vec![0u8; 12];
        f.draw(&mut plane, 4, &text, 0);
        assert_eq!(&plane[0..4], &[0, 0xF0, 0xF0, 0], "row 0, cols 1..2");
        assert_eq!(&plane[4..8], &[0, 0xF0, 0xF0, 0], "row 1");
        assert_eq!(&plane[8..12], &[0; 4], "row 2 untouched");
        // Accent overlay: e-acute (0x82) has no base glyph in the
        // synth bank, so nothing draws; use a font WITH an e glyph.
        let count = GLYPH_BASE + ACCENT_GLYPH_OFF + 5;
        let e_char = 0x65usize; // e
        let e_entry = GLYPH_BASE + (e_char - FIRST_CHAR as usize);
        let images = vec![
            glyph_bytes(3, 2, 0xF0),                // bang: entry 0x82 gate
            glyph_bytes(3, 2, 0xF3),                // e
            hotspot_glyph_bytes(2, 1, -1, 1, 0xF2), // diaeresis overlay
        ];
        let entries = vec![
            GLYPH_BASE,
            e_entry,
            GLYPH_BASE + ACCENT_GLYPH_OFF + 2, // accent 2 = acute (e-acute)
        ];
        let f2 = LoadingFont::from_bank(&bank(count, &entries, &images)).unwrap();
        // e-acute measures 5 (e glyph 3 + gap): 5 + 9*70 + 5 = 640 ->
        // pen 0. The e glyph (transparent first column) fills row 1
        // cols 1..2; the ACUTE overlay (2x1, dy -1, dx +1) lands row 0
        // cols 1..2; the trailing e-acute at pen 635 clips off-plane.
        let mut text = vec![0x82u8];
        text.extend(std::iter::repeat_n(0x20, 70));
        text.push(0x82);
        let mut plane = vec![0xFFu8; 16]; // 4x4
        f2.draw(&mut plane, 4, &text, 1);
        // The e glyph (glyph_bytes) has a transparent first column.
        assert_eq!(&plane[4..8], &[0xFF, 0xF3, 0xF3, 0xFF], "base glyph row 1");
        assert_eq!(&plane[12..16], &[0xFF; 4], "row 3 untouched");
        assert_eq!(
            &plane[0..4],
            &[0xFF, 0xF2, 0xF2, 0xFF],
            "overlay above base"
        );
    }

    #[test]
    fn from_bank_rejects_short_and_headless_banks() {
        // Directory too small / garbage: the parser rejects.
        assert!(LoadingFont::from_bank(&[1u8, 0, 4, 0, 0, 0, 8]).is_err());
        // Structurally fine (count covers the font window) but every
        // slot points past EOF: staging error on the first glyph.
        let empty = {
            let mut v = 300u16.to_le_bytes().to_vec();
            for _ in 0..300 {
                v.extend_from_slice(&2000u32.to_le_bytes());
            }
            v
        };
        match LoadingFont::from_bank(&empty) {
            Err(GameError::BadLoadingAsset {
                what: "font bank",
                reason: "entry 0x82 (first glyph) undecoded",
            }) => {}
            other => panic!("wrong error: {other:?}"),
        }
    }
}

/// Shared synthetic assets for the loading-flow tests (font bank +
/// LANGUAGE file + FULLPAL ramp), shaped exactly like the corpus.
#[cfg(test)]
pub(crate) mod synth {
    use super::*;

    /// Raw solid-fill glyph, w x h, flags 0 (raw, no hotspot).
    pub(crate) fn raw_glyph(w: u16, h: u16, fill: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&(w as i16).to_le_bytes());
        v.extend_from_slice(&(h as i16).to_le_bytes());
        v.extend(std::iter::repeat_n(fill, w as usize * h as usize));
        v
    }

    /// Bank of `count` slots; `entries` (in order) hold `images`,
    /// every other slot points at a zeroed trailing header (empty).
    pub(crate) fn bank(count: usize, entries: &[usize], images: &[Vec<u8>]) -> Vec<u8> {
        assert_eq!(entries.len(), images.len());
        let mut starts = Vec::new();
        let mut data_start = 2 + count * 4;
        for img in images {
            starts.push(data_start);
            data_start += img.len();
        }
        let empty_start = data_start;
        let mut v = (count as u16).to_le_bytes().to_vec();
        for entry in 0..count {
            let off = match entries.iter().position(|e| *e == entry) {
                Some(k) => (starts[k] - (2 + entry * 4)) as u32,
                None => (empty_start - (2 + entry * 4)) as u32,
            };
            v.extend_from_slice(&off.to_le_bytes());
        }
        for img in images {
            v.extend_from_slice(img);
        }
        v.extend(std::iter::repeat_n(0u8, 8));
        v
    }

    /// A minimal but complete loading font bank: the five chars of
    /// the synth strings below (bang e i o u space-free) plus the
    /// diaeresis/acute overlays. Glyph values use fills 0xF0.. so
    /// compositing is observable against a 0x10 still.
    pub(crate) fn font_bin() -> Vec<u8> {
        let count = GLYPH_BASE + ACCENT_GLYPH_OFF + 5;
        // chars: 0x21 bang, 0x45 E, 0x49 I, 0x4f O, 0x55 U, 0x65 e,
        // 0x69 i, 0x6f o, 0x75 u, 0x2d dash.
        let chars: [(u8, u8); 10] = [
            (0x21, 0xF0),
            (0x45, 0xF1),
            (0x49, 0xF1),
            (0x4f, 0xF1),
            (0x55, 0xF1),
            (0x65, 0xF2),
            (0x69, 0xF2),
            (0x6f, 0xF2),
            (0x75, 0xF2),
            (0x2d, 0xF3),
        ];
        let mut entries = Vec::new();
        let mut images = Vec::new();
        for (c, fill) in chars {
            entries.push(GLYPH_BASE + (c - FIRST_CHAR) as usize);
            images.push(raw_glyph(3, 2, fill));
        }
        // Accent overlays 238 (diaeresis) and 239 (acute): 2x1, dy=-1,
        // dx 0 and 1, distinct fills.
        let overlay = |dx: i16, fill: u8| {
            let mut g = Vec::new();
            g.extend_from_slice(&2u16.to_le_bytes()); // flags: hotspot
            g.extend_from_slice(&(-1i16).to_le_bytes()); // dy
            g.extend_from_slice(&dx.to_le_bytes());
            g.extend_from_slice(&2i16.to_le_bytes()); // w
            g.extend_from_slice(&1i16.to_le_bytes()); // h
            g.extend_from_slice(&[fill]);
            g
        };
        entries.push(GLYPH_BASE + ACCENT_GLYPH_OFF + 1);
        images.push(overlay(0, 0xF4));
        entries.push(GLYPH_BASE + ACCENT_GLYPH_OFF + 2);
        images.push(overlay(1, 0xF5));
        bank(count, &entries, &images)
    }

    /// A LANGUAGE-shaped file whose [MENU_ITEMS] section places the
    /// five loading strings at the EXW indices (entries 0x45, 0x46,
    /// 0x52..=0x58), ASCII-only here (the accent path is unit-tested
    /// in the font module itself).
    pub(crate) fn language_bin(congrats: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"[OTHER]\r\n[\r\nx\r\n]\r\n\r\n");
        v.extend_from_slice(b"[MENU_ITEMS]\r\n\r\n[\r\n");
        // Filler entries up to each index, then the string itself.
        let mut emitted = 0usize;
        let mut put = |want: usize, text: &[u8]| {
            while emitted < want {
                v.extend_from_slice(b"filler line\r\n");
                emitted += 1;
            }
            v.extend_from_slice(text);
            v.extend_from_slice(b"\r\n");
            emitted += 1;
        };
        put(0x45, congrats);
        put(0x46, b"Now move out to");
        put(0x52, b"The Airport");
        put(0x53, b"The Industrial Sector");
        put(0x54, b"The Docklands");
        put(0x55, b"The Suburbs");
        put(0x56, b"The City Centre");
        put(0x57, b"The Biomex Nest");
        put(0x58, b"Destroy all BioCapsules");
        v.extend_from_slice(b"]\r\n");
        v
    }

    /// A 98-byte FULLPAL-shaped ramp: entry 224+0..=8 black, then a
    /// distinct 6-bit value per remaining entry.
    pub(crate) fn fullpal_bin() -> Vec<u8> {
        let mut v = vec![0xE0u8, 0x20];
        for i in 0..32u16 {
            let c = if i < 9 { 0 } else { ((i * 7) & 0x3F) as u8 };
            v.extend_from_slice(&[c, c, c]);
        }
        assert_eq!(v.len(), 98);
        v
    }
}
