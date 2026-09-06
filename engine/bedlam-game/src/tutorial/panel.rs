//! Original hint glyphs, authored rows and staggered color animation.
use crate::{ByteSource, GameError};
use bedlam_assets::sprites::{parse_bin_images, SpriteBank};

const COLORS: [u8; 8] = [103, 98, 146, 164, 36, 66, 255, 77];
const W: usize = 640;
const H: usize = 480;

#[derive(Debug)]
pub struct HintPanel {
    font: SpriteBank,
    coverage: Vec<Vec<bool>>,
    messages: Vec<Vec<Vec<u8>>>,
}

impl HintPanel {
    pub fn load(source: &mut dyn ByteSource, language: &str) -> Result<Self, GameError> {
        let bytes = source.load("TINYFONT.BIN")?;
        let font = parse_bin_images(&bytes)?;
        if font.count != 118 || font.images.iter().any(|im| !im.ok) {
            return Err(GameError::BadLoadingAsset {
                what: "TINYFONT.BIN",
                reason: "missing or undecodable hint glyph",
            });
        }
        let coverage = crate::mission_room::coverage_for_bank(&bytes, &font, "TINYFONT.BIN")?;
        let bytes = source.load(language)?;
        let messages = (0..15)
            .map(|id| {
                crate::mission_room::panel::description(
                    &bytes,
                    format!("[BOOT_CAMP_{id:03}]").as_bytes(),
                )
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            font,
            coverage,
            messages,
        })
    }

    fn width(&self, line: &[u8]) -> i32 {
        line.iter()
            .map(|&c| {
                let (c, _) = remap(c);
                if c < 33 {
                    3
                } else {
                    i32::from(self.font.images[(c - 33) as usize].w) + 1
                }
            })
            .sum()
    }

    /// Age is one on the first ticker/draw call. The caller supplies the
    /// mission's active 256-entry darkening table (EXW 0x402a73).
    pub fn draw(&self, plane: &mut [u8], id: usize, age: u32, dark: &[u8; 256]) {
        let Some(lines) = self.messages.get(id).filter(|v| !v.is_empty()) else {
            return;
        };
        if age == 0 || plane.len() != W * H {
            return;
        }
        let max_width = lines.iter().map(|line| self.width(line)).max().unwrap_or(0);
        let n = lines.len() as i32;
        let cols = (max_width + 4) / 5 + 2;
        let rows = ((n - 1) * 9 + 10) / 7 + 2;
        let x = 240 - max_width / 2;
        let y = 200;
        for yy in y..y + rows * 7 {
            for xx in x..x + cols * 5 {
                if (0..640).contains(&xx) && (0..480).contains(&yy) {
                    let pixel = &mut plane[yy as usize * W + xx as usize];
                    *pixel = dark[*pixel as usize];
                }
            }
        }
        for col in 0..cols {
            let delay = (col - cols / 2).unsigned_abs() / 4;
            if let Some(color) = color(age, delay) {
                let top = if col == 0 {
                    97
                } else if col == cols - 1 {
                    98
                } else {
                    95
                };
                let bottom = if col == 0 {
                    99
                } else if col == cols - 1 {
                    100
                } else {
                    95
                };
                self.glyph(plane, top, x + col * 5, y, color);
                self.glyph(plane, bottom, x + col * 5, y + (rows - 1) * 7, color);
            }
        }
        for row in 0..rows - 2 {
            let delay = ((cols / 2 + row.min(rows - 3 - row)) / 4) as u32;
            if let Some(color) = color(age, delay) {
                self.glyph(plane, 96, x, y + (row + 1) * 7, color);
                self.glyph(plane, 96, x + (cols - 1) * 5, y + (row + 1) * 7, color);
            }
        }
        let gap = rows * 7 - n * 9;
        for (i, line) in lines.iter().enumerate() {
            let Some(color) = color(age, i as u32) else {
                continue;
            };
            let yy = y
                + gap / 2
                + if i == 0 {
                    1
                } else {
                    2 + (gap & 1) + 9 * i as i32
                };
            let mut xx = x + (5 * cols - max_width) / 2;
            for &c in line {
                let (c, accent) = remap(c);
                if c < 33 {
                    xx += 3;
                    continue;
                }
                let entry = (c - 33) as usize;
                self.glyph(plane, entry, xx, yy, color);
                if accent != 0 {
                    self.glyph(plane, 0x71 + accent as usize, xx, yy, color);
                }
                xx += i32::from(self.font.images[entry].w) + 1;
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
                let xx = x + i32::from(dx) + col as i32;
                let yy = y + i32::from(dy) + row as i32;
                if self.coverage[entry][row * im.w as usize + col]
                    && (0..640).contains(&xx)
                    && (0..480).contains(&yy)
                {
                    plane[yy as usize * W + xx as usize] = color;
                }
            }
        }
    }
}

fn remap(c: u8) -> (u8, u8) {
    if c >= 128 {
        crate::font::remap_high(c)
    } else {
        (c, 0)
    }
}

fn color(age: u32, delay: u32) -> Option<u8> {
    age.checked_sub(delay + 1)
        .map(|frame| COLORS[frame.min(7) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Corpus;
    impl ByteSource for Corpus {
        fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
            let root =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM");
            std::fs::read(if name.starts_with("LANGUAGE.") {
                root.join(name)
            } else {
                root.join("GAMEGFX").join(name)
            })
            .map_err(|_| GameError::AssetMissing { name: name.into() })
        }
    }

    #[test]
    fn all_localized_hints_load_and_draw_with_staggered_original_colors() {
        let dark = [12u8; 256];
        for language in ["ENG", "GER", "SPA", "FRE", "ITL", "DCH"] {
            let panel = HintPanel::load(&mut Corpus, &format!("LANGUAGE.{language}")).unwrap();
            for id in 0..15 {
                assert!(!panel.messages[id].is_empty());
                let mut early = vec![200; W * H];
                panel.draw(&mut early, id, 1, &dark);
                assert!(early.contains(&103), "first color {language}/{id}");
                assert!(!early.contains(&77));
                let mut settled = vec![200; W * H];
                panel.draw(&mut settled, id, 100, &dark);
                assert!(settled.contains(&77), "settled color {language}/{id}");
                assert!(settled.contains(&12));
                assert_eq!(settled[0], 200);
                assert!(settled.iter().all(|p| [200, 12, 77].contains(p)));
            }
        }
        assert_eq!(color(3, 3), None);
        assert_eq!(color(4, 3), Some(103));
        assert_eq!(color(11, 3), Some(77));
    }
}
