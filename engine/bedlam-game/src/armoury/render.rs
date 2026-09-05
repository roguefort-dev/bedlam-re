//! Original armoury artwork and immediate text, EXW 0x440e45/0x43fe8a.
use super::{catalog::CATEGORIES, transactions::Transactions};
use crate::{ByteSource, GameError};
use bedlam_assets::sprites::{parse_bin_images, SpriteBank};
use bedlam_render::Vga6;
const W: usize = 640;
const H: usize = 480;

fn bad(what: &'static str) -> GameError {
    GameError::BadArmouryAsset {
        what,
        reason: "missing or undecodable image",
    }
}

struct Font {
    bank: SpriteBank,
    coverage: Vec<Vec<bool>>,
}
impl Font {
    fn load(
        source: &mut dyn ByteSource,
        name: &'static str,
        count: usize,
    ) -> Result<Self, GameError> {
        let bytes = source.load(name)?;
        let bank = parse_bin_images(&bytes)?;
        if bank.images.len() != count || bank.images.iter().any(|im| !im.ok) {
            return Err(bad(name));
        }
        let mut coverage = Vec::new();
        for (i, im) in bank.images.iter().enumerate() {
            let start = 2 + i * 4 + im.off as usize + if im.flags & 2 != 0 { 10 } else { 6 };
            coverage.push(if im.w == 0 {
                Vec::new()
            } else if im.flags & 1 != 0 {
                bedlam_assets::codecs::decode_rle16_coverage(
                    bytes.get(start..).ok_or_else(|| bad(name))?,
                    im.w as usize,
                    im.h as usize,
                )
                .map_err(|_| bad(name))?
            } else {
                vec![true; im.w as usize * im.h as usize]
            });
        }
        Ok(Self { bank, coverage })
    }
    fn width(&self, text: &[u8]) -> i32 {
        text.iter()
            .map(|&c| {
                let c = if c >= 128 {
                    crate::font::remap_high(c).0
                } else {
                    c
                };
                if c < 33 {
                    3
                } else {
                    self.bank
                        .images
                        .get((c - 33) as usize)
                        .map_or(0, |im| i32::from(im.w) + 1)
                }
            })
            .sum()
    }
    fn glyph(&self, plane: &mut [u8], entry: usize, x: i32, y: i32, color: u8) {
        let Some(im) = self.bank.images.get(entry) else {
            return;
        };
        let (dy, dx) = im.hot.unwrap_or((0, 0));
        for row in 0..im.h as usize {
            for col in 0..im.w as usize {
                let x = x + i32::from(dx) + col as i32;
                let y = y + i32::from(dy) + row as i32;
                if (0..640).contains(&x)
                    && (0..480).contains(&y)
                    && self.coverage[entry][row * im.w as usize + col]
                {
                    plane[y as usize * W + x as usize] = color;
                }
            }
        }
    }
    fn text(&self, plane: &mut [u8], text: &[u8], mut x: i32, y: i32, color: u8) {
        for &c in text {
            let (c, accent) = if c >= 128 {
                crate::font::remap_high(c)
            } else {
                (c, 0)
            };
            if c < 33 {
                x += 3;
                continue;
            }
            let entry = (c - 33) as usize;
            self.glyph(plane, entry, x, y, color);
            if accent != 0 {
                self.glyph(plane, 0x71 + accent as usize, x, y, color);
            }
            x += self
                .bank
                .images
                .get(entry)
                .map_or(0, |im| i32::from(im.w) + 1);
        }
    }
}

/// Raster layer. Input, reveal timing and owned weapon icon animation are
/// separate scene work; callers choose the active artwork category explicitly.
pub struct ArmouryRenderer {
    artwork: SpriteBank,
    tiny: Font,
    small: Font,
    palette: [Vga6; 256],
    plane: Vec<u8>,
}
impl ArmouryRenderer {
    pub fn load(source: &mut dyn ByteSource) -> Result<Self, GameError> {
        let artwork = parse_bin_images(&source.load("SHOPLITE.BIN")?)?;
        if artwork.images.len() != 10
            || artwork.images.iter().any(|im| {
                im.w != 640 || im.h != 480 || im.pixels.as_ref().is_none_or(|p| p.len() != W * H)
            })
        {
            return Err(bad("SHOPLITE.BIN"));
        }
        Ok(Self {
            artwork,
            tiny: Font::load(source, "TINYFONT.BIN", 118)?,
            small: Font::load(source, "SMLFONT.BIN", 63)?,
            palette: crate::loading::loading_palette(&source.load("SHOPPAL.PAL")?)?,
            plane: vec![0; W * H],
        })
    }
    pub fn palette(&self) -> &[Vga6; 256] {
        &self.palette
    }
    pub fn pixels(&self) -> &[u8] {
        &self.plane
    }
    pub fn draw(&mut self, state: &Transactions, category: Option<usize>) {
        let category = category.filter(|&c| c < CATEGORIES.len());
        self.plane.copy_from_slice(
            self.artwork.images[category.map_or(0, |c| c + 1)]
                .pixels
                .as_ref()
                .expect("validated artwork"),
        );
        if let Some(c) = category {
            let cat = CATEGORIES[c];
            let (x, y) = cat.panel_origin();
            // Text positions are original. Border animation is added by the scene.
            for (i, item) in cat.items.iter().enumerate() {
                let name = if state.catalog().available(c, i) {
                    crate::mission::weapon_name(item.name)
                } else {
                    "CLASSIFIED"
                };
                self.tiny.text(
                    &mut self.plane,
                    name.as_bytes(),
                    x + 5,
                    y + 7 + 9 * i as i32,
                    5,
                );
                self.tiny.text(
                    &mut self.plane,
                    format!("{:03}/{:03}", item.price, item.amount).as_bytes(),
                    x + 5 + cat.columns * 5 - 44,
                    y + 7 + 9 * i as i32,
                    5,
                );
            }
        }
        if let Some(cart) = state.cart() {
            let item = state
                .catalog()
                .item(cart.category, cart.item)
                .expect("validated cart");
            self.center(crate::mission::weapon_name(item.name), 0x129);
            self.center(&format!("CASH:{} AMT:{}", state.cash(), cart.amount), 0x141);
        } else {
            self.center(&format!("BALANCE:{}", state.balance()), 0x129);
        }
    }
    fn center(&mut self, text: &str, y: i32) {
        self.small.text(
            &mut self.plane,
            text.as_bytes(),
            0x22d - self.small.width(text.as_bytes()) / 2,
            y,
            0xfd,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armoury::catalog::{Catalog, Mode};
    struct Source;
    impl ByteSource for Source {
        fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../game-data/BEDLAM/GAMEGFX")
                .join(name);
            std::fs::read(path).map_err(|_| GameError::AssetMissing { name: name.into() })
        }
    }
    #[test]
    fn original_artwork_and_fonts_render_the_pending_purchase() {
        let mut renderer = ArmouryRenderer::load(&mut Source).unwrap();
        let mut state = Transactions::new(Catalog::new(Mode::Campaign, 1, [0; 15]).unwrap(), 3500);
        renderer.draw(&state, None);
        let entry = renderer.pixels().to_vec();
        assert!(state.select(0, 0));
        renderer.draw(&state, Some(0));
        assert_ne!(renderer.pixels(), entry);
        // The actual small font must fit the right-hand display; SHOPFONT does not.
        assert!(renderer.small.width(b"NEEDLER CANNON #1") < 174);
        assert!(renderer.pixels()[297 * W..308 * W].contains(&253));
        for category in 0..9 {
            renderer.draw(&state, Some(category));
            assert_eq!(renderer.pixels().len(), W * H);
        }
        renderer.draw(&state, Some(usize::MAX)); // Invalid category safely restores base artwork.
    }
}
