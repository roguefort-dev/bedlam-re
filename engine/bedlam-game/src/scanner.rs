//! EXW 0x41ec81 radar reveal and 0x402572 marker blitter.
use crate::GameError;
use bedlam_assets::sprites::{parse_bin_images, SpriteBank};
use bedlam_render::ui_bank::draw_sprite;

/// Marker position in the selected robot's radar space, centered at (64,64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Marker {
    pub icon: usize,
    pub x: i32,
    pub y: i32,
}

/// Verified map, active-pad and robot passes of EXW 0x41ee20.
/// Object/TRT/critter/arrival/objective producers are not included here yet.
pub fn mission_markers(
    sim: &bedlam_core::mission::MissionSim,
    selected: usize,
    squad: std::ops::Range<usize>,
    level: u8,
) -> Vec<Marker> {
    let Some(anchor) = sim.robots().get(selected) else {
        return Vec::new();
    };
    let project = |p: i32| ((p >> 8) + 16) >> 4;
    let center = (project(anchor.pos_x), project(anchor.pos_y));
    let (w, h) = sim.terrain.size();
    let lo = (((center.0 >> 1) - 32).max(0), ((center.1 >> 1) - 32).max(0));
    let hi = (
        ((center.0 >> 1) + 32).min(w - 1),
        ((center.1 >> 1) + 32).min(h - 1),
    );
    let mut markers = Vec::new();
    let mut push = |icon, x: i32, y: i32| {
        let (dx, dy) = (x - center.0, y - center.1);
        if dx.abs() < 128 && dy.abs() < 128 {
            markers.push(Marker {
                icon,
                x: 64 + dx,
                y: 64 + dy,
            });
        }
    };
    for y in lo.1..hi.1 {
        for x in lo.0..hi.0 {
            if level >= 1 && sim.platform_strength_word(x, y) != 0 {
                push(7, 2 * x + 1, 2 * y + 1);
            } else if sim
                .claim_bank()
                .get((y * w + x) as usize)
                .copied()
                .unwrap_or(0)
                != 0
            {
                push(13, 2 * x + 1, 2 * y + 1);
            }
        }
    }
    for slot in 0..sim.terrain.pad_slot_count() {
        let (x, y, _) = sim.terrain.pad_slot(slot).expect("retained pad slot");
        if x == 0 && y == 0 {
            break;
        }
        if x > lo.0 && x < hi.0 && y > lo.1 && y < hi.1 {
            push(12, 2 * x + 1, 2 * y + 1);
        }
    }
    for (index, robot) in sim.robots().iter().enumerate() {
        if robot.alive {
            push(
                if squad.contains(&index) { 1 } else { 2 },
                project(robot.pos_x),
                project(robot.pos_y),
            );
        }
    }
    markers
}

#[derive(Debug)]
pub struct Scanner {
    bytes: Vec<u8>,
    sprites: SpriteBank,
    image: Vec<u8>,
    radius: u8,
    pressed: bool,
}

impl Scanner {
    pub fn draw_backdrop(&self, plane: &mut [u8]) {
        draw_sprite(plane, 640, &self.bytes, 18, 494, 195, true);
    }

    pub fn load(bytes: Vec<u8>) -> Result<Self, GameError> {
        let sprites = parse_bin_images(&bytes)?;
        if sprites.count != 19 || sprites.images.iter().any(|im| !im.ok) {
            return Err(GameError::BadMissionAsset {
                what: "SCANNER.BIN",
                reason: "expected nineteen valid radar sprites",
            });
        }
        Ok(Self {
            bytes,
            sprites,
            image: vec![0; 128 * 128],
            radius: 64,
            pressed: false,
        })
    }

    /// One MissionShell radar call. Markers are captured only at the refresh
    /// boundary. Returns true on release, for the original backdrop redraw.
    pub fn draw(
        &mut self,
        plane: &mut [u8],
        cursor: (i32, i32),
        mouse_down: bool,
        markers: &[Marker],
    ) -> bool {
        if plane.len() != 640 * 480 {
            return false;
        }
        let held_inside =
            mouse_down && (494..=625).contains(&cursor.0) && (195..=326).contains(&cursor.1);
        if self.pressed {
            if held_inside {
                draw_sprite(plane, 640, &self.bytes, 17, 494, 195, true);
            } else {
                self.radius = 64;
                self.image.fill(0);
                self.pressed = false;
                draw_sprite(plane, 640, &self.bytes, 18, 494, 195, true);
                return true;
            }
            return false;
        }
        self.pressed = held_inside;
        if self.radius == 64 {
            self.copy_square(plane);
            self.image.fill(0);
            self.radius = 0;
            for marker in markers {
                self.mark(*marker);
            }
        } else {
            self.radius = (self.radius + 4).min(64);
            self.copy_square(plane);
            if self.radius != 64 {
                self.outline(plane);
            }
        }
        draw_sprite(plane, 640, &self.bytes, 0, 496, 195, true);
        false
    }

    fn copy_square(&self, plane: &mut [u8]) {
        let r = usize::from(self.radius);
        let start = 64 - r;
        for row in start..64 + r {
            let src = row * 128 + start;
            let dst = (197 + row) * 640 + 496 + start;
            plane[dst..dst + 2 * r].copy_from_slice(&self.image[src..src + 2 * r]);
        }
    }

    fn outline(&self, plane: &mut [u8]) {
        let r = usize::from(self.radius);
        if r == 0 {
            return;
        }
        let x = 496 + 64 - r;
        let y = 195 + 64 - r;
        for xx in x..x + 2 * r {
            plane[y * 640 + xx] = 7;
            plane[(y + 2 * r - 1) * 640 + xx] = 7;
        }
        for yy in y..y + 2 * r {
            plane[yy * 640 + x] = 7;
            plane[yy * 640 + x + 2 * r] = 7;
        }
    }

    fn mark(&mut self, marker: Marker) {
        if !(1..=13).contains(&marker.icon) {
            return;
        }
        let im = &self.sprites.images[marker.icon];
        let Some(pixels) = &im.pixels else {
            return;
        };
        let (hy, hx) = im.hot.unwrap_or((0, 0));
        // The original specialized marker blitter reads unsigned hotspots.
        let x = marker.x + i32::from(hx as u16) - 2;
        let y = marker.y + i32::from(hy as u16) - 2;
        for row in 0..i32::from(im.h) {
            for col in 0..i32::from(im.w) {
                let color = pixels[(row * i32::from(im.w) + col) as usize];
                if color != 0 && (0..128).contains(&(x + col)) && (0..128).contains(&(y + row)) {
                    self.image[((y + row) * 128 + x + col) as usize] = color;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scanner() -> Scanner {
        Scanner::load(
            std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../game-data/BEDLAM/GAMEGFX/SCANNER.BIN"),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn refresh_captures_then_reveals_old_snapshot_for_seventeen_calls() {
        let mut scanner = scanner();
        let mut plane = vec![200; 640 * 480];
        let markers = [Marker {
            icon: 1,
            x: 64,
            y: 64,
        }];
        scanner.draw(&mut plane, (0, 0), false, &markers);
        assert_eq!(scanner.radius, 0);
        let captured = scanner.image.clone();
        assert!(captured.iter().any(|&p| p != 0));
        for r in (4..=64).step_by(4) {
            scanner.draw(&mut plane, (0, 0), false, &[]);
            assert_eq!(scanner.radius, r);
            assert_eq!(scanner.image, captured, "markers stay cached between scans");
        }
        scanner.draw(&mut plane, (0, 0), false, &[]);
        assert_eq!(scanner.radius, 0);
        assert!(scanner.image.iter().all(|&p| p == 0));
        assert_eq!(plane[0], 200);
    }

    #[test]
    fn press_hold_release_resets_scan_and_requests_backdrop() {
        let mut scanner = scanner();
        let mut plane = vec![0; 640 * 480];
        assert!(!scanner.draw(&mut plane, (494, 195), true, &[]));
        assert_eq!(scanner.radius, 0, "first press still scans");
        assert!(!scanner.draw(&mut plane, (625, 326), true, &[]));
        assert_eq!(scanner.radius, 0, "held press freezes scan");
        assert!(scanner.draw(&mut plane, (626, 326), true, &[]));
        assert_eq!(scanner.radius, 64);
        assert!(!scanner.pressed);
    }

    #[test]
    fn marker_clipping_and_transparency_preserve_other_pixels() {
        let mut scanner = scanner();
        scanner.image.fill(200);
        scanner.mark(Marker {
            icon: 1,
            x: 0,
            y: 0,
        });
        assert!(scanner.image[..128 * 3].iter().any(|&p| p != 200));
        assert_eq!(scanner.image[127 * 128 + 127], 200);
        let image = scanner.image.clone();
        scanner.mark(Marker {
            icon: 1,
            x: -128,
            y: -128,
        });
        assert_eq!(scanner.image, image);
    }
}
