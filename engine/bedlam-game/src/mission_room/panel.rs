//! EXW 0x447216 / 0x43e274 / 0x43f855 text and 0x440888 border.
use super::{bad, coverage_for_bank, ByteSource, GameError, SpriteBank, H, W};

const COLORS: [[u8; 8]; 3] = [
    [129, 130, 130, 130, 130, 130, 130, 130],
    [129, 130, 131, 132, 132, 132, 132, 132],
    [1, 130, 131, 132, 133, 134, 135, 136],
];

pub(super) struct RoomPanel {
    font: SpriteBank,
    coverage: Vec<Vec<bool>>,
    descriptions: Vec<Vec<Vec<u8>>>,
}

impl RoomPanel {
    pub(super) fn load(
        source: &mut dyn ByteSource,
        zone: u8,
        language_name: &str,
    ) -> Result<Self, GameError> {
        let bytes = source.load("TINYFONT.BIN")?;
        let font = bedlam_assets::sprites::parse_bin_images(&bytes)?;
        // The shipped bank intentionally leaves entries 105, 106 and 113 empty.
        if font.count != 118 || font.images.iter().any(|im| !im.ok) {
            return Err(bad("TINYFONT.BIN", "missing or undecodable glyph"));
        }
        let coverage = coverage_for_bank(&bytes, &font, "TINYFONT.BIN")?;
        let language = source.load(language_name)?;
        let mut descriptions = Vec::new();
        for mission in 1..=if zone == 1 { 1 } else { 5 } {
            let heading = format!("[OVERVIEW_{}{}]", char::from(b'A' + zone - 1), mission);
            descriptions.push(description(&language, heading.as_bytes())?);
        }
        Ok(Self {
            font,
            coverage,
            descriptions,
        })
    }

    pub(super) fn draw(&self, plane: &mut [u8], mission: Option<u8>, state: usize, age: u32) {
        for col in 0i32..41 {
            let x = 1 + 5 * col;
            let delay = (col - 20).unsigned_abs();
            let Some(color) = color(state, age, delay) else {
                continue;
            };
            self.glyph(
                plane,
                match col {
                    0 => 97,
                    40 => 98,
                    _ => 95,
                },
                x,
                1,
                color,
            );
            self.glyph(
                plane,
                match col {
                    0 => 99,
                    40 => 100,
                    _ => 95,
                },
                x,
                113,
                color,
            );
        }
        for row in 1u32..16 {
            let delay = 8 + (row - 1).min(15 - row);
            if let Some(color) = color(state, age, delay) {
                self.glyph(plane, 96, 1, 1 + 7 * row as i32, color);
                self.glyph(plane, 96, 201, 1 + 7 * row as i32, color);
            }
        }
        let Some(lines) = mission.and_then(|m| self.descriptions.get(m as usize - 1)) else {
            return;
        };
        for (i, line) in lines.iter().enumerate() {
            let delay = if i == 0 { 1 } else { 3 * i as u32 };
            let Some(color) = color(state, age, delay) else {
                continue;
            };
            let y = if i == 0 { 8 } else { 11 + 10 * i as i32 };
            let mut x = 8;
            for &c in line {
                let (c, accent) = if c >= 128 {
                    crate::font::remap_high(c)
                } else {
                    (c, 0)
                };
                if c < 0x21 {
                    x += 3;
                    continue;
                }
                let entry = (c - 0x21) as usize;
                if let Some(glyph) = self.font.images.get(entry) {
                    self.glyph(plane, entry, x, y, color);
                    if accent != 0 {
                        self.glyph(plane, 0x71 + accent as usize, x, y, color);
                    }
                    x += i32::from(glyph.w) + 1;
                }
            }
        }
    }

    fn glyph(&self, plane: &mut [u8], entry: usize, x: i32, y: i32, color: u8) {
        let Some(im) = self.font.images.get(entry) else {
            return;
        };
        let (dy, dx) = im.hot.unwrap_or((0, 0));
        for row in 0..im.h as usize {
            for col in 0..im.w as usize {
                let (x, y) = (
                    x + i32::from(dx) + col as i32,
                    y + i32::from(dy) + row as i32,
                );
                if self.coverage[entry][row * im.w as usize + col]
                    && (0..W as i32).contains(&x)
                    && (0..H as i32).contains(&y)
                {
                    plane[y as usize * W + x as usize] = color;
                }
            }
        }
    }
}

fn color(state: usize, age: u32, delay: u32) -> Option<u8> {
    age.checked_sub(delay + 1)
        .map(|phase| COLORS[state][phase.min(7) as usize])
}

/// Preserve authored rows and byte encodings; there is no runtime word wrap.
pub(crate) fn description(data: &[u8], heading: &[u8]) -> Result<Vec<Vec<u8>>, GameError> {
    let start = data
        .windows(heading.len())
        .position(|w| w == heading)
        .map(|p| p + heading.len())
        .ok_or_else(|| bad("LANGUAGE overview", "missing heading"))?;
    let opener = data[start..]
        .iter()
        .position(|&b| b >= 0x21)
        .map(|p| p + start)
        .filter(|&p| data[p] == b'[')
        .ok_or_else(|| bad("LANGUAGE overview", "missing body"))?;
    let open = opener + 1;
    let end = data[open..]
        .iter()
        .position(|&b| b == b']')
        .map(|p| p + open)
        .ok_or_else(|| bad("LANGUAGE overview", "unterminated body"))?;
    let mut rows = Vec::new();
    if data[open..end].contains(&b'[') {
        return Err(bad("LANGUAGE overview", "nested body"));
    }
    for raw in data[open..end].split(|b| *b < 0x20) {
        let row = raw
            .iter()
            .position(|b| *b >= 0x21)
            .map(|p| &raw[p..])
            .unwrap_or_default();
        if row.is_empty() {
            continue;
        }
        if row.len() > 63 || rows.len() >= 16 {
            return Err(bad("LANGUAGE overview", "text exceeds original row bank"));
        }
        rows.push(row.to_vec());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_rows_keep_accents_spaces_and_reject_missing_or_unbounded_bodies() {
        assert_eq!(
            description(
                b"[OVERVIEW_A1]\r\n[\n FIRST  ROW \n\tCAF\x82\n]",
                b"[OVERVIEW_A1]"
            )
            .unwrap(),
            vec![b"FIRST  ROW ".to_vec(), b"CAF\x82".to_vec()]
        );
        for bytes in [
            b"[OTHER][text]".as_slice(),
            b"[OVERVIEW_A1][unfinished".as_slice(),
        ] {
            assert!(description(bytes, b"[OVERVIEW_A1]").is_err());
        }
        assert_eq!(color(2, 3, 3), None);
        assert_eq!(color(2, 4, 3), Some(1));
        assert_eq!(color(2, 20, 3), Some(136));
    }
}
