//! Original single-player mission room. EXW provenance and draw semantics:
//! docs/RE-EXW-MISSION-ROOM.md, FUN_0043e7d4 and helpers.

use crate::{ByteSource, GameError};
use bedlam_assets::sprites::{parse_bin_images, SpriteBank, SpriteImage};
use bedlam_core::input::InputFrame;
use bedlam_render::Vga6;

mod panel;
use panel::RoomPanel;

const W: usize = 640;
const H: usize = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomAction {
    None,
    Armoury { zone: u8, mission: u8 },
    Briefing { zone: u8, mission: u8 },
    Back,
}

/// Entry starts with no mission selected. Completion flags are supplied by
/// campaign state; clicking another zone never changes that state.
pub struct MissionRoom {
    panel: RoomPanel,
    panel_age: u32,
    selector: SpriteBank,
    regions: SpriteBank,
    coverage: Vec<Vec<bool>>,
    mask: Vec<u8>,
    blend: Vec<u8>,
    dark: Vec<u8>,
    palette: [Vga6; 256],
    plane: Vec<u8>,
    zone: u8,
    completed: [bool; 27],
    selected: Option<u8>,
    cursor: (i32, i32),
    previous_left: bool,
    door: u8,
    frame: u32,
}

fn bad(what: &'static str, reason: &'static str) -> GameError {
    GameError::BadMissionRoomAsset { what, reason }
}

fn bank(bytes: &[u8], count: usize, name: &'static str) -> Result<SpriteBank, GameError> {
    let result = parse_bin_images(bytes)?;
    if result.images.len() != count
        || result.images.iter().any(|im| {
            im.pixels
                .as_ref()
                .is_none_or(|p| p.len() != usize::from(im.w) * usize::from(im.h))
        })
    {
        return Err(bad(name, "missing or undecodable image"));
    }
    Ok(result)
}

fn full_screen(image: &SpriteImage, name: &'static str) -> Result<(), GameError> {
    if (image.w, image.h) != (W as u16, H as u16) {
        return Err(bad(name, "expected 640x480 image"));
    }
    Ok(())
}

fn coverage_for_bank(
    bytes: &[u8],
    images: &SpriteBank,
    name: &'static str,
) -> Result<Vec<Vec<bool>>, GameError> {
    images
        .images
        .iter()
        .enumerate()
        .map(|(i, im)| {
            if im.w == 0 {
                return Ok(Vec::new());
            }
            let start = 2 + i * 4 + im.off as usize + if im.flags & 2 != 0 { 10 } else { 6 };
            let payload = bytes
                .get(start..)
                .ok_or_else(|| bad(name, "invalid image offset"))?;
            if im.flags & 1 != 0 {
                bedlam_assets::codecs::decode_rle16_coverage(payload, im.w as usize, im.h as usize)
                    .map_err(|_| bad(name, "invalid coverage spans"))
            } else {
                Ok(vec![true; im.w as usize * im.h as usize])
            }
        })
        .collect()
}

impl MissionRoom {
    pub fn load(
        source: &mut dyn ByteSource,
        zone: u8,
        completed: [bool; 27],
        language_name: &str,
    ) -> Result<Self, GameError> {
        if !(1..=6).contains(&zone) {
            return Err(bad("campaign zone", "outside selectable zones 1..6"));
        }
        let selector = bank(&source.load("SELECTOR.BIN")?, 15, "SELECTOR.BIN")?;
        full_screen(&selector.images[0], "SELECTOR.BIN")?;
        let palette = crate::loading::loading_palette(&source.load("SELECTOR.PAL")?)?;
        let normal = source.load("NORMAL.BIN")?;
        let regions = bank(&normal, 35, "NORMAL.BIN")?;
        let coverage = coverage_for_bank(&normal, &regions, "NORMAL.BIN")?;
        let mask_bank = bank(&source.load("SELMONT.BIN")?, 1, "SELMONT.BIN")?;
        full_screen(&mask_bank.images[0], "SELMONT.BIN")?;
        let mask = mask_bank.images[0].pixels.clone().expect("validated");
        if mask.iter().any(|&id| id > 26) {
            return Err(bad("SELMONT.BIN", "unknown mission id"));
        }
        let blend = source.load("TXPAL3.PAL")?;
        let dark = source.load("SELDARK.PAL")?;
        if blend.len() != 65536 || dark.len() != 256 {
            return Err(bad("translation tables", "invalid table length"));
        }
        let panel = RoomPanel::load(source, zone, language_name)?;
        let mut room = Self {
            panel,
            panel_age: 0,
            selector,
            regions,
            coverage,
            mask,
            blend,
            dark,
            palette,
            plane: vec![0; W * H],
            zone,
            completed,
            selected: None,
            cursor: (320, 240),
            previous_left: false,
            door: 0,
            frame: 0,
        };
        room.render();
        Ok(room)
    }

    pub fn selected(&self) -> Option<u8> {
        self.selected
    }
    pub fn cursor(&self) -> (i32, i32) {
        self.cursor
    }
    pub fn pixels(&self) -> &[u8] {
        &self.plane
    }
    pub fn palette(&self) -> &[Vga6; 256] {
        &self.palette
    }
    pub fn door_frame(&self) -> u8 {
        self.door
    }

    fn selected_index(&self) -> Option<usize> {
        self.selected.map(|m| {
            if self.zone == 1 {
                0
            } else {
                (self.zone as usize - 2) * 5 + m as usize
            }
        })
    }

    fn available(&self) -> bool {
        self.selected_index().is_some_and(|i| !self.completed[i])
    }

    pub fn tick(&mut self, input: &InputFrame) -> RoomAction {
        use bedlam_core::frame::{CURSOR_MAX_X, CURSOR_MAX_Y, CURSOR_MIN_X, CURSOR_MIN_Y};
        self.cursor.0 =
            (self.cursor.0 + i32::from(input.mouse_dx)).clamp(CURSOR_MIN_X, CURSOR_MAX_X);
        self.cursor.1 =
            (self.cursor.1 + i32::from(input.mouse_dy)).clamp(CURSOR_MIN_Y, CURSOR_MAX_Y);
        let left = input.mouse_buttons & 1 != 0;
        let click = left && !self.previous_left;
        self.previous_left = left;
        // Shell's Escape semantic bit; same cinema/menu input binding.
        if input.buttons & (1 << 9) != 0 {
            return RoomAction::Back;
        }
        let (x, y) = self.cursor;
        if click {
            if self.available() && self.door == 4 {
                let mission = self.selected.expect("available");
                // EXW really repeats x for the lower-bound comparison.
                if x > 227 && x < 284 && y < 128 {
                    return RoomAction::Armoury {
                        zone: self.zone,
                        mission,
                    };
                }
                if self.zone > 1 && x > 458 && x < 542 && y < 176 {
                    return RoomAction::Briefing {
                        zone: self.zone,
                        mission,
                    };
                }
            }
            let id = self.mask[y as usize * W + x as usize];
            if let Some((zone, mission)) = mission_for_id(id) {
                if zone == self.zone {
                    if self.selected != Some(mission) {
                        self.panel_age = 0;
                    }
                    self.selected = Some(mission);
                }
            }
        }
        self.door = if self.available() {
            (self.door + 1).min(4)
        } else {
            self.door.saturating_sub(1)
        };
        self.frame = self.frame.wrapping_add(1);
        self.panel_age = self.panel_age.saturating_add(1);
        self.render();
        RoomAction::None
    }

    fn render(&mut self) {
        self.plane
            .copy_from_slice(self.selector.images[0].pixels.as_ref().expect("validated"));
        // EXW 0x43eb38..4f: SELDARK over (1,1), 205 columns, 119 rows.
        for y in 1..120 {
            for x in 1..206 {
                let p = &mut self.plane[y * W + x];
                *p = self.dark[*p as usize];
            }
        }
        let panel_state = if self.available() { 2 } else { 0 };
        self.panel
            .draw(&mut self.plane, self.selected, panel_state, self.panel_age);
        for i in 0..26 {
            let (zone, _) = mission_for_id(i as u8 + 1).expect("region");
            if self.completed[i] {
                self.region(i, true, false);
            } else if zone == self.zone {
                self.region(i, false, false);
            }
        }
        if self.frame > 16 && self.available() {
            self.region(
                self.selected_index().expect("available"),
                false,
                self.frame & 7 >= 3,
            );
        }
        blit(
            &mut self.plane,
            &self.selector.images[5 + 2 * self.door as usize],
            218,
            20,
        );
        let briefing = if self.zone > 1 {
            6 + 2 * self.door as usize
        } else {
            6
        };
        blit(&mut self.plane, &self.selector.images[briefing], 447, 4);
        // NORMAL stores y,x placement in its hotspot words.
        blit(
            &mut self.plane,
            &self.regions.images[26 + (self.frame as usize / 2).min(8)],
            0,
            0,
        );
    }

    fn region(&mut self, i: usize, completed: bool, copy: bool) {
        let im = &self.regions.images[i];
        let (y, x) = im.hot.unwrap_or((0, 0));
        let px = im.pixels.as_ref().expect("validated");
        for row in 0..im.h as usize {
            for col in 0..im.w as usize {
                let n = row * im.w as usize + col;
                let (dx, dy) = (i32::from(x) + col as i32, i32::from(y) + row as i32);
                if !(0..W as i32).contains(&dx)
                    || !(0..H as i32).contains(&dy)
                    || !self.coverage[i][n]
                {
                    continue;
                }
                let dst = &mut self.plane[dy as usize * W + dx as usize];
                if completed {
                    if px[n] != 0 {
                        *dst = self.dark[*dst as usize];
                    }
                } else if copy {
                    *dst = px[n];
                } else {
                    *dst = self.blend[px[n] as usize * 256 + *dst as usize];
                }
            }
        }
    }
}

/// SP mask ids cover A1 then B..F, five missions each (EXW 0x43ee48..edc).
fn mission_for_id(id: u8) -> Option<(u8, u8)> {
    match id {
        1 => Some((1, 1)),
        2..=26 => Some(((id - 2) / 5 + 2, (id - 2) % 5 + 1)),
        _ => None,
    }
}

fn blit(dst: &mut [u8], im: &SpriteImage, x: i32, y: i32) {
    let (dy, dx) = im.hot.unwrap_or((0, 0));
    let (x, y) = (x + i32::from(dx), y + i32::from(dy));
    let pixels = im.pixels.as_ref().expect("validated");
    for row in 0..im.h as usize {
        for col in 0..im.w as usize {
            let p = pixels[row * im.w as usize + col];
            let (x, y) = (x + col as i32, y + row as i32);
            if p != 0 && (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
                dst[y as usize * W + x as usize] = p;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Corpus(std::path::PathBuf);
    impl ByteSource for Corpus {
        fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
            std::fs::read(if name.starts_with("LANGUAGE.") {
                self.0.parent().unwrap().join(name)
            } else {
                self.0.join(name)
            })
            .map_err(|_| GameError::AssetMissing { name: name.into() })
        }
    }
    fn room(completed: [bool; 27]) -> MissionRoom {
        MissionRoom::load(
            &mut Corpus(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../game-data/BEDLAM/GAMEGFX"),
            ),
            1,
            completed,
            "LANGUAGE.ENG",
        )
        .unwrap()
    }
    fn click(room: &mut MissionRoom, x: i32, y: i32) -> RoomAction {
        let (cx, cy) = room.cursor();
        room.tick(&InputFrame {
            mouse_dx: (x - cx) as i16,
            mouse_dy: (y - cy) as i16,
            ..Default::default()
        });
        room.tick(&InputFrame {
            mouse_buttons: 1,
            ..Default::default()
        })
    }
    #[test]
    fn original_boot_camp_region_opens_armoury_only_after_selection() {
        let mut r = room([false; 27]);
        assert_eq!(click(&mut r, 255, 80), RoomAction::None);
        assert_eq!(r.selected(), None);
        assert_eq!(click(&mut r, 255, 315), RoomAction::None);
        assert_eq!(r.selected(), Some(1));
        assert!(r.door_frame() < 4);
        for _ in 0..20 {
            r.tick(&InputFrame::default());
        }
        assert_eq!(r.door_frame(), 4);
        assert_eq!(
            click(&mut r, 500, 80),
            RoomAction::None,
            "Boot Camp has no briefing door"
        );
        assert_eq!(
            click(&mut r, 255, 80),
            RoomAction::Armoury {
                zone: 1,
                mission: 1
            }
        );
    }
    #[test]
    fn completed_and_other_zone_regions_cannot_launch_a_mission() {
        let mut completed = [false; 27];
        completed[0] = true;
        let mut r = room(completed);
        assert_eq!(click(&mut r, 255, 315), RoomAction::None);
        for _ in 0..8 {
            r.tick(&InputFrame::default());
        }
        assert_eq!(r.door_frame(), 0);
        assert_eq!(click(&mut r, 255, 80), RoomAction::None);
        let mut r = room([false; 27]);
        let p = r.mask.iter().position(|&id| id == 2).unwrap();
        assert_eq!(
            click(&mut r, (p % W) as i32, (p / W) as i32),
            RoomAction::None
        );
        assert_eq!(r.selected(), None);
        assert_eq!(
            r.tick(&InputFrame {
                buttons: 1 << 9,
                ..Default::default()
            }),
            RoomAction::Back
        );
    }
    #[test]
    fn translation_blit_preserves_skips_and_uses_original_hotspot_order() {
        let mut r = room([false; 27]);
        r.plane.fill(17);
        r.region(0, false, false);
        let im = &r.regions.images[0];
        let (y, x) = im.hot.unwrap();
        let pixels = im.pixels.as_ref().unwrap();
        for row in 0..im.h as usize {
            for col in 0..im.w as usize {
                let n = row * im.w as usize + col;
                let dst = (y as usize + row) * W + x as usize + col;
                let expected = if r.coverage[0][n] {
                    r.blend[pixels[n] as usize * 256 + 17]
                } else {
                    17
                };
                assert_eq!(r.pixels()[dst], expected);
            }
        }
        assert_eq!(r.pixels()[0], 17);
    }

    #[test]
    fn every_shipped_language_loads_all_campaign_descriptions() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/GAMEGFX");
        for language in ["ENG", "FRE", "GER", "SPA", "ITL", "DCH"] {
            for zone in 1..=6 {
                panel::RoomPanel::load(
                    &mut Corpus(root.clone()),
                    zone,
                    &format!("LANGUAGE.{language}"),
                )
                .unwrap_or_else(|e| panic!("{language} zone {zone}: {e}"));
            }
        }
    }

    #[test]
    fn selected_description_and_border_reveal_in_the_original_color_ramp() {
        let mut r = room([false; 27]);
        click(&mut r, 255, 315);
        let early = r.pixels().to_vec();
        for _ in 0..40 {
            r.tick(&InputFrame::default());
        }
        // Settled border corner and glyph strokes use selected color 136.
        assert_eq!(r.pixels()[4 * W + 3], 136);
        let text_pixels = (8..100)
            .flat_map(|y| &r.pixels()[y * W + 8..y * W + 200])
            .filter(|&&p| p == 136)
            .count();
        assert!(
            text_pixels > 700,
            "original description missing: {text_pixels} selected-color pixels"
        );
        assert_ne!(
            &r.pixels()[..W * 120],
            &early[..W * 120],
            "panel must reveal over time"
        );
    }
}
