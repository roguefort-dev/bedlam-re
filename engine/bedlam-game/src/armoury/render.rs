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

/// Raster layer. The scene supplies input state and animation ages.
pub struct ArmouryRenderer {
    artwork: SpriteBank,
    icons: SpriteBank,
    controls: SpriteBank,
    dark: [u8; 256],
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
        let icons = parse_bin_images(&source.load("WEAPICON.BIN")?)?;
        if icons.images.len() != 96 || icons.images.iter().any(|im| !im.ok || im.pixels.is_none()) {
            return Err(bad("WEAPICON.BIN"));
        }
        let controls = parse_bin_images(&source.load("CONLITE.BIN")?)?;
        if controls.images.len() != 6
            || controls
                .images
                .iter()
                .any(|im| !im.ok || im.pixels.is_none())
        {
            return Err(bad("CONLITE.BIN"));
        }
        Ok(Self {
            dark: source
                .load("DARKPALS.PAL")?
                .try_into()
                .map_err(|_| bad("DARKPALS.PAL"))?,
            icons,
            controls,
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
    /// Render the settled state, useful for previews and restoration.
    pub fn draw(&mut self, state: &Transactions, category: Option<usize>) {
        self.draw_animated(state, category, &[12; 7], &[9; 2]);
    }

    /// Ages are supplied by the scene clock, never advanced by repainting.
    /// Weapon age zero is text phase zero and icon counter one.
    pub fn draw_animated(
        &mut self,
        state: &Transactions,
        category: Option<usize>,
        weapon_ages: &[u8; 7],
        equipment_ages: &[u8; 2],
    ) {
        self.draw_frame(state, category, weapon_ages, equipment_ages, u32::MAX);
    }

    pub fn draw_frame(
        &mut self,
        state: &Transactions,
        category: Option<usize>,
        weapon_ages: &[u8; 7],
        equipment_ages: &[u8; 2],
        panel_age: u32,
    ) {
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
            for row in y..y + 7 * cat.rows {
                for col in x..x + 5 * cat.columns {
                    let offset = row as usize * W + col as usize;
                    self.plane[offset] = self.dark[self.plane[offset] as usize];
                }
            }
            self.border(cat, panel_age);
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
        const COLORS: [u8; 10] = [4, 64, 74, 58, 42, 128, 160, 158, 1, 207];
        const X: [i32; 7] = [318, 92, 546, 156, 484, 236, 406];
        const Y: [i32; 7] = [89, 145, 145, 202, 202, 181, 181];
        for (slot, row) in state.weapons().iter().enumerate() {
            let Some(row) = row else {
                continue;
            };
            let age = weapon_ages[slot];
            self.tiny.text(
                &mut self.plane,
                crate::mission::weapon_name(row.name).as_bytes(),
                538,
                342 + 10 * slot as i32,
                COLORS[age.min(9) as usize],
            );
            let entry = row.category * 12 + age.min(11) as usize;
            self.icon(entry, X[slot] - 29, Y[slot] - 27);
        }
        for (slot, row) in state.equipment().iter().enumerate() {
            if let Some(row) = row {
                self.tiny.text(
                    &mut self.plane,
                    crate::mission::weapon_name(row.name).as_bytes(),
                    547,
                    417 + 10 * slot as i32,
                    COLORS[equipment_ages[slot].min(9) as usize],
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
    fn border(&mut self, category: super::catalog::Category, age: u32) {
        const COLORS: [u8; 12] = [4, 225, 222, 230, 221, 5, 10, 235, 228, 158, 1, 5];
        let color = |delay: u32| {
            age.checked_sub(delay + 1)
                .map(|phase| COLORS[phase.min(11) as usize])
        };
        let (x, y) = category.panel_origin();
        for column in 0..category.columns {
            if let Some(color) = color((column - category.columns / 2).unsigned_abs()) {
                let top = if column == 0 {
                    97
                } else if column == category.columns - 1 {
                    98
                } else {
                    95
                };
                let bottom = if column == 0 {
                    99
                } else if column == category.columns - 1 {
                    100
                } else {
                    95
                };
                self.tiny
                    .glyph(&mut self.plane, top, x + 5 * column, y, color);
                self.tiny.glyph(
                    &mut self.plane,
                    bottom,
                    x + 5 * column,
                    y + 7 * (category.rows - 1),
                    color,
                );
            }
        }
        for i in 0..category.rows - 2 {
            let delay = category.columns / 2 + i.min(category.rows - 3 - i);
            if let Some(color) = color(delay as u32) {
                self.tiny
                    .glyph(&mut self.plane, 96, x, y + 7 * (i + 1), color);
                self.tiny.glyph(
                    &mut self.plane,
                    96,
                    x + 5 * (category.columns - 1),
                    y + 7 * (i + 1),
                    color,
                );
            }
        }
    }

    /// The original lights a control while the mouse button is held, not on
    /// hover. DONE's animation-ready gate is supplied by the scene.
    pub fn highlight(&mut self, cursor: (i32, i32), held: bool, ready: bool) {
        use super::controls::Control;
        if !held {
            return;
        }
        let Some(control) = Control::at(cursor) else {
            return;
        };
        if control == Control::Done && !ready {
            return;
        }
        let (entry, x, y) = control.image();
        Self::blit(&mut self.plane, &self.controls.images[entry], x, y);
    }
    fn icon(&mut self, entry: usize, x: i32, y: i32) {
        Self::blit(&mut self.plane, &self.icons.images[entry], x, y);
    }
    fn blit(plane: &mut [u8], im: &bedlam_assets::sprites::SpriteImage, x: i32, y: i32) {
        let pixels = im.pixels.as_ref().expect("validated icon");
        let (dy, dx) = im.hot.unwrap_or((0, 0));
        for row in 0..im.h as usize {
            for col in 0..im.w as usize {
                let pixel = pixels[row * im.w as usize + col];
                let x = x + i32::from(dx) + col as i32;
                let y = y + i32::from(dy) + row as i32;
                if pixel != 0 && (0..640).contains(&x) && (0..480).contains(&y) {
                    plane[y as usize * W + x as usize] = pixel;
                }
            }
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
    fn controls_light_only_while_pressed_and_done_waits_for_readiness() {
        let mut renderer = ArmouryRenderer::load(&mut Source).unwrap();
        let state = Transactions::new(Catalog::new(Mode::Campaign, 1, [0; 15]).unwrap(), 3500);
        renderer.draw(&state, None);
        let baseline = renderer.pixels().to_vec();
        renderer.highlight((500, 350), false, true);
        assert_eq!(renderer.pixels(), baseline);
        renderer.highlight((500, 350), true, true);
        assert_ne!(renderer.pixels(), baseline);
        renderer.draw(&state, None);
        renderer.highlight((590, 460), true, false);
        assert_eq!(renderer.pixels(), baseline);
        renderer.highlight((590, 460), true, true);
        assert_ne!(renderer.pixels(), baseline);
    }

    #[test]
    fn purchased_weapon_and_equipment_render_and_repaint_without_advancing() {
        let mut renderer = ArmouryRenderer::load(&mut Source).unwrap();
        let mut state = Transactions::new(Catalog::new(Mode::Campaign, 2, [1; 15]).unwrap(), 3500);
        renderer.draw(&state, None);
        let empty = renderer.pixels().to_vec();
        assert!(state.select(0, 0));
        assert!(state.buy());
        assert!(state.select(8, 1));
        assert!(state.buy());
        renderer.draw_animated(&state, None, &[0; 7], &[0; 2]);
        let first = renderer.pixels().to_vec();
        renderer.draw_animated(&state, None, &[0; 7], &[0; 2]);
        assert_eq!(renderer.pixels(), first);
        renderer.draw(&state, None);
        assert_ne!(renderer.pixels(), first);
        assert_ne!(renderer.pixels()[70 * W..115 * W], empty[70 * W..115 * W]);
        assert!(renderer.pixels()[342 * W..350 * W].contains(&207));
        assert!(renderer.pixels()[417 * W..425 * W].contains(&207));
        assert!(state.sell_weapon(0));
        assert!(state.sell_equipment(0));
        state.cancel();
        renderer.draw(&state, None);
        assert_eq!(renderer.pixels(), empty);
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
        let settled = renderer.pixels().to_vec();
        renderer.draw_frame(&state, Some(0), &[12; 7], &[9; 2], 0);
        let initial = renderer.pixels().to_vec();
        assert_ne!(initial, settled);
        renderer.draw_frame(&state, Some(0), &[12; 7], &[9; 2], 0);
        assert_eq!(renderer.pixels(), initial);
        renderer.draw_frame(&state, Some(0), &[12; 7], &[9; 2], 40);
        assert_eq!(renderer.pixels(), settled);
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
