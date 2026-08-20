//! 770-byte VGA palette files (.pal) and the 98-byte FULLPAL font ramp.

use crate::AssetsError;

/// A 256-entry RGB palette, already expanded to 8-bit components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette(pub [[u8; 3]; 256]);

/// Parse a 770-byte VGA palette: 2-byte lead-in then 256 RGB triples in 6-bit
/// components, expanded via `(v << 2) | (v >> 4)`.
///
/// Accepts any buffer of at least 770 bytes (the inspect CLI gates on exactly
/// 770 before calling; sibling-palette lookup uses the same rule).
pub fn parse_vga770(data: &[u8]) -> Result<Palette, AssetsError> {
    if data.len() < 770 {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    let mut p = [[0u8; 3]; 256];
    for i in 0..256 {
        for c in 0..3 {
            let v6 = data[2 + i * 3 + c] & 0x3F;
            p[i][c] = (v6 << 2) | (v6 >> 4);
        }
    }
    Ok(Palette(p))
}

/// Entries in the FULLPAL.PAL font ramp [verified: the LAB_0041c69e
/// tail copies 0x60 bytes = 24 dwords + 0 tail from the FULLPAL load
/// buffer +2 into DAC buffer +0x2a2 = entries 224..=255].
pub const FONT_RAMP_ENTRIES: usize = 32;

/// Parse a 98-byte FULLPAL.PAL: 2-byte lead-in (`e0 20` on the
/// corpus = first entry 224, count 32) then 32 RGB triples in 6-bit
/// components, masked like [`parse_vga770`] (EXW copies the bytes
/// raw into the 6-bit DAC buffer; the mask is a no-op on 6-bit data).
///
/// The ramp replaces palette entries 224..=255 of the loading-screen
/// fade target AFTER the pre-text 0x3f fill (bedlam-game loading).
pub fn parse_font_ramp(data: &[u8]) -> Result<[[u8; 3]; FONT_RAMP_ENTRIES], AssetsError> {
    let expected = 2 + FONT_RAMP_ENTRIES * 3;
    if data.len() != expected {
        return Err(AssetsError::WrongSize { len: data.len() });
    }
    let mut out = [[0u8; 3]; FONT_RAMP_ENTRIES];
    for (i, entry) in out.iter_mut().enumerate() {
        for c in 0..3 {
            entry[c] = data[2 + i * 3 + c] & 0x3F;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; 770];
        for i in 0..256usize {
            for c in 0..3usize {
                d[2 + i * 3 + c] = ((i * 3 + c) & 0x3F) as u8;
            }
        }
        d
    }

    #[test]
    fn parse_expands_six_bit() {
        let d = synthetic();
        let pal = parse_vga770(&d).unwrap();
        // entry 0: components 0,1,2 -> 0, (1<<2)|(1>>4)=4, (2<<2)|(2>>4)=8
        assert_eq!(pal.0[0], [0, 4, 8]);
        // entry 1: components 3,4,5
        assert_eq!(pal.0[1], [12, 16, 20]);
        // top-of-range: 0x3F -> (0x3F<<2)|(0x3F>>4) = 0xFC | 0x3 = 0xFF
        assert_eq!((0x3Fu8 << 2) | (0x3Fu8 >> 4), 0xFF);
        // high bits above 0x3F are masked off
        let mut d2 = d.clone();
        d2[2] = 0xFF; // low 6 bits = 0x3F
        let pal2 = parse_vga770(&d2).unwrap();
        assert_eq!(pal2.0[0][0], 0xFF);
    }

    #[test]
    fn parse_rejects_short_buffers() {
        assert_eq!(
            parse_vga770(&vec![0u8; 769]),
            Err(AssetsError::TooSmall { len: 769 })
        );
        assert_eq!(parse_vga770(&[]), Err(AssetsError::TooSmall { len: 0 }));
    }

    #[test]
    fn font_ramp_masks_six_bit_and_pins_size() {
        let mut d = vec![0xE0u8, 0x20];
        for i in 0..FONT_RAMP_ENTRIES {
            for c in 0..3 {
                d.push(((i * 3 + c) & 0x3F) as u8);
            }
        }
        assert_eq!(d.len(), 98);
        let ramp = parse_font_ramp(&d).unwrap();
        assert_eq!(ramp[0], [0, 1, 2]);
        assert_eq!(
            ramp[31],
            [
                ((31 * 3) & 0x3F) as u8,
                ((31 * 3 + 1) & 0x3F) as u8,
                ((31 * 3 + 2) & 0x3F) as u8,
            ],
        );
        // High bits masked like parse_vga770.
        let mut d2 = d.clone();
        d2[2] = 0xFF;
        assert_eq!(parse_font_ramp(&d2).unwrap()[0][0], 0x3F);
        // Exact size gate: 97 or 99 bytes rejected.
        assert_eq!(
            parse_font_ramp(&d[..97]),
            Err(AssetsError::WrongSize { len: 97 })
        );
        let mut long = d.clone();
        long.push(0);
        assert_eq!(
            parse_font_ramp(&long),
            Err(AssetsError::WrongSize { len: 99 })
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 42u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 769, 770, 771, 2048] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_vga770(&d);
        }
        for len in [0usize, 1, 97, 98, 99, 4096] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_font_ramp(&d);
        }
    }
}
