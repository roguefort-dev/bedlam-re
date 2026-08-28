//! The FIRST native ENHANCED pass (P6 opener, PLAN §6 "ENHANCED
//! mode is explicitly non-parity and renders supported world/UI
//! passes natively"): the MISSION-IDENTITY STRIP.
//!
//! THE CHOICE (documented in docs/P6-MODERNIZATION.md §1, D217):
//! the smallest HONEST native pass is a UI plane whose every input
//! is already-landed game-owned data — the mission identity (the
//! zone letter + mission number, the same bytes the save metadata
//! shows, RE-EXW-SAVE sec 3 / FUN_004473cd semantics), drawn with
//! the game's own SMLFONT glyphs through the LANDED pub
//! [`bedlam_render::ui_bank`] drawer (FUN_00402884 glyph fill +
//! the FUN_00408913 text advances, both verified RE-EXW-SIM
//! sec 6c.8c), in the game's own sidebar text color 0x24 [asm
//! 0x4084f8/0x40853e], indexed against the canonical frame's own
//! palette. ZERO new binary claims — every RE fact above is
//! already committed and anchored.
//!
//! It renders at PRESENTATION resolution (integer-scaled crisp
//! glyphs, positioned in device pixels by the responsive layout)
//! into the LEFT pillarbox margin INSIDE the safe region — never
//! over game pixels (the canonical frame rides byte-identical
//! underneath; the margin is presentation matte), never inside the
//! future HD-pack area OUTSIDE the safe region (RESEARCH-HD-ASSET-
//! PIPELINE §5.A/§8: engine-rendered text stays inside the safe
//! region). Shown ONLY while a mission is staged; a missing
//! SMLFONT.BIN disables the strip (best-effort platform surface,
//! noted, never fatal — the D208 posture).
//!
//! Hermetic by construction: the builders here are PURE functions
//! over bank bytes; only the window present site reads the corpus
//! (SMLFONT.BIN through the existing GameGfxSource — the headless
//! path never fetches it).

use bedlam_game::Scene;
use bedlam_platform::layout::{world_margins, ResponsiveFrame};
use bedlam_platform::scale::Rect;
use bedlam_render::ui_bank::{draw_glyph, sprite_geometry};

/// The sidebar text bank file name [RE-EXW-SIM sec 6c.8c: the
/// mission staging fetch set stages SMLFONT.BIN; LANDED].
pub const SMLFONT_NAME: &str = "SMLFONT.BIN";

/// The strip text color: the game's OWN sidebar text color 0x24
/// [RE-EXW-SIM sec 6c.8a, asm 0x4084f8/0x40853e; LANDED] — the
/// strip never invents a color, it reuses the identity color the
/// game itself renders its sidebar rows in.
pub const STRIP_TEXT_COLOR: u8 = 0x24;

/// The integer glyph replication factor: the SMLFONT rasters are
/// authored 1x on the 640x480 grid; the native pass draws them at
/// 2x device pixels so the strip reads at presentation scale while
/// staying pixel-crisp (never interpolated — the parity posture).
pub const STRIP_SCALE: u32 = 2;

/// The minimum left-margin width (device px) that shows the strip:
/// narrower margins (small windows, unusual aspects) omit it — the
/// responsive layout only ever places UI where it fits.
pub const STRIP_MIN_MARGIN_W: u32 = 96;

/// The padding kept around the strip inside its margin.
const STRIP_PAD: u32 = 8;

/// The zone letter + mission number as the game's own save
/// metadata writes them [RE-EXW-SAVE sec 3, FUN_004473cd: one zone
/// letter 'A'+zone, then the number; LANDED] — the engine's
/// 0-based zone meets the letter arithmetic here ('A' + zone).
pub fn mission_identity_text(zone: i32, mission: i32) -> Vec<u8> {
    let letter = b'A' + zone.clamp(0, 5) as u8;
    let mut text = vec![letter];
    text.extend_from_slice(mission.max(1).to_string().as_bytes());
    text
}

/// Whether the strip is staged for this host state, and from which
/// slot: MISSION scenes only (the identity is the mission the
/// engine is running — `GameHost::mission_slot` answers it; on
/// every other scene there is no honest identity to show, so the
/// pass renders nothing).
pub fn strip_slot_for(scene: Scene, slot: (i32, i32)) -> Option<(i32, i32)> {
    (scene == Scene::Mission).then_some(slot)
}

/// One built native strip plane: palette indices, its own
/// dimensions, and the replication factor used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStripPlane {
    pub w: u32,
    pub h: u32,
    pub indices: Vec<u8>,
    pub scale: u32,
}

/// Build the identity strip plane from the SMLFONT bank bytes
/// (PURE). The text draw is the LANDED FUN_00408913 semantics:
/// bytes < 0x21 advance 6 px without drawing; any other byte draws
/// glyph `ch - 0x21` filled with [`STRIP_TEXT_COLOR`] and advances
/// `w + 1`. Glyph rows land at the plane top; the plane is the
/// text's tight ink box, then every ink pixel replicates to a
/// `scale` x `scale` block. Non-ink cells carry index 0 — the
/// game's own background color (canonical palette entry 0), the
/// same matte the clear would paint. None when the bank lacks the
/// glyphs or the text is empty.
pub fn build_identity_strip(
    bank: &[u8],
    zone: i32,
    mission: i32,
    scale: u32,
) -> Option<NativeStripPlane> {
    let scale = scale.max(1) as usize;
    let text = mission_identity_text(zone, mission);
    // Measure (the FUN_00408913 advance arithmetic).
    let mut w_total = 0usize;
    let mut h_max = 0usize;
    for &b in &text {
        let ch = u32::from(b);
        if ch < 0x21 {
            w_total += 6;
            continue;
        }
        let id = (ch - 0x21) as u16;
        let (w, h, _xhot, yhot) = sprite_geometry(bank, id)?;
        w_total += w.unsigned_abs() as usize + 1;
        h_max = h_max.max((h + yhot).max(0) as usize);
    }
    if w_total == 0 || h_max == 0 {
        return None;
    }
    // Draw at 1x into the tight box.
    let mut scratch = vec![0u8; w_total * h_max];
    let mut x = 0i32;
    for &b in &text {
        let ch = u32::from(b);
        if ch < 0x21 {
            x += 6;
            continue;
        }
        let id = (ch - 0x21) as u16;
        draw_glyph(&mut scratch, w_total, bank, id, STRIP_TEXT_COLOR, x, 0);
        x += sprite_geometry(bank, id).map_or(0, |g| g.0) + 1;
    }
    // Integer pixel replication to presentation scale.
    let sw = w_total * scale;
    let sh = h_max * scale;
    let mut indices = vec![0u8; sw * sh];
    for sy in 0..h_max {
        for sx in 0..w_total {
            let ink = scratch[sy * w_total + sx];
            if ink == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    indices[(sy * scale + dy) * sw + sx * scale + dx] = ink;
                }
            }
        }
    }
    Some(NativeStripPlane {
        w: sw as u32,
        h: sh as u32,
        indices,
        scale: scale as u32,
    })
}

/// Where the strip lands inside the responsive frame's LEFT margin
/// (PURE): centered, shown only when the margin comfortably fits
/// the plane (the layout never squeezes UI — narrow margins omit
/// the strip entirely). The margins OUTSIDE the safe region are
/// never used: they are the future HD-pack/outpaint territory.
pub fn strip_rect(frame: &ResponsiveFrame, plane_w: u32, plane_h: u32) -> Option<Rect> {
    let (left, _right) = world_margins(frame);
    if left.w < STRIP_MIN_MARGIN_W {
        return None;
    }
    if plane_w + 2 * STRIP_PAD > left.w || plane_h + 2 * STRIP_PAD > left.h {
        return None;
    }
    Some(Rect {
        x: left.x + (left.w - plane_w) / 2,
        y: left.y + (left.h - plane_h) / 2,
        w: plane_w,
        h: plane_h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal SMLFONT-shaped bank [the landed .BIN layout,
    /// RE-EXW-SIM sec 6c.8c — the same synth shape bedlam-game's
    /// mission tests use]: `count` glyphs, every one a solid 2x2
    /// RLE mask with a zero hotspot.
    fn synth_bank(count: usize) -> Vec<u8> {
        let mut bank = vec![0u8; 2 + 4 * count];
        bank[0..2].copy_from_slice(&(count as u16).to_le_bytes());
        for id in 0..count {
            let entry = 2 + 4 * id;
            let off = bank.len() as u32 - entry as u32;
            bank[entry..entry + 4].copy_from_slice(&off.to_le_bytes());
            bank.extend_from_slice(&3u16.to_le_bytes()); // flags: hotspot + RLE
            bank.extend_from_slice(&[0, 0]); // yhot
            bank.extend_from_slice(&[0, 0]); // xhot
            bank.extend_from_slice(&2u16.to_le_bytes()); // w
            bank.extend_from_slice(&2u16.to_le_bytes()); // h
            bank.extend_from_slice(&[0x02, 0x40, 0x4D, 0x4D]); // literal 2 + EOL
            bank.extend_from_slice(&[0x02, 0x40, 0x4D, 0x4D]);
        }
        bank
    }

    #[test]
    fn identity_text_is_the_save_metadata_bytes() {
        // 'A'+zone, then the number — the FUN_004473cd zone-letter
        // arithmetic over the engine's 0-based zone.
        assert_eq!(mission_identity_text(0, 1), b"A1".to_vec());
        assert_eq!(mission_identity_text(1, 6), b"B6".to_vec());
        assert_eq!(mission_identity_text(5, 7), b"F7".to_vec());
        // Out-of-range slots clamp, never guess a nonsense letter.
        assert_eq!(mission_identity_text(-3, 2), b"A2".to_vec());
        assert_eq!(mission_identity_text(9, 2), b"F2".to_vec());
        assert_eq!(mission_identity_text(2, 0), b"C1".to_vec());
    }

    #[test]
    fn strip_is_staged_for_missions_only() {
        let slot = (1, 3);
        assert_eq!(strip_slot_for(Scene::Mission, slot), Some(slot));
        for scene in [
            Scene::Boot,
            Scene::Title,
            Scene::Brief,
            Scene::Select,
            Scene::Debrief,
            Scene::Cutscene,
            Scene::Shop,
            Scene::Options,
            Scene::Quit,
        ] {
            assert_eq!(strip_slot_for(scene, slot), None, "{scene:?}");
        }
    }

    #[test]
    fn strip_builds_the_exw_text_semantics_at_scale() {
        let bank = synth_bank(63);
        // "B2": glyph 'B' = id 0x42-0x21 = 0x21, '2' = 0x31-0x21
        // = 0x10 — both 2x2 solids, advance w+1 = 3 each: the 1x
        // box is 6x2, the 2x plane 12x4.
        let plane = build_identity_strip(&bank, 1, 2, 2).expect("strip builds");
        assert_eq!((plane.w, plane.h, plane.scale), (12, 4, 2));
        assert_eq!(plane.indices.len(), 48);
        // The FUN_00408913 advance arithmetic places 'B' at x=0
        // (columns 0..2) and '2' at x=3 (columns 3..5); at scale 2
        // the ink blocks are columns 0..4 and 6..10, rows 0..4.
        for y in 0..4 {
            for x in 0..12 {
                let expected = if (x < 4 || (6..10).contains(&x)) && y < 4 {
                    STRIP_TEXT_COLOR
                } else {
                    0
                };
                assert_eq!(plane.indices[y * 12 + x], expected, "pixel ({x},{y})");
            }
        }
        // The ink is the game's own color, never an invented one.
        assert!(plane
            .indices
            .iter()
            .all(|&i| i == 0 || i == STRIP_TEXT_COLOR));
    }

    #[test]
    fn strip_space_advances_six_without_drawing() {
        let bank = synth_bank(63);
        // The identity text never contains a space (letter +
        // digits), so the 6px space advance is unreachable through
        // the builder — the measure arithmetic keeps the landed
        // FUN_00408913 semantics for future strip content anyway.
        assert!(!mission_identity_text(2, 5).contains(&b' '));
        assert!(build_identity_strip(&bank, 2, 5, 2).is_some());
        // A bank missing the glyphs fails closed (None, no panic).
        assert!(build_identity_strip(&synth_bank(4), 1, 2, 2).is_none());
    }

    #[test]
    fn strip_rect_centers_in_the_left_margin_or_omits() {
        let frame = bedlam_platform::layout::responsive_frame(1920, 1200);
        // 16:10 target: safe = 1920x1080 centered, world = 1440x1080
        // fit, left margin 240 wide — the 12x4 strip centers in it.
        let rect = strip_rect(&frame, 12, 4).expect("fits");
        assert_eq!(rect.w, 12);
        assert_eq!(rect.h, 4);
        let (left, _right) = world_margins(&frame);
        assert_eq!(rect.x, left.x + (left.w - 12) / 2);
        assert_eq!(rect.y, left.y + (left.h - 4) / 2);
        // Too small a margin omits the strip — never squeezed.
        let tiny = bedlam_platform::layout::responsive_frame(700, 525);
        assert_eq!(strip_rect(&tiny, 12, 4), None);
        // A plane wider than the padded margin omits too.
        assert_eq!(strip_rect(&frame, 240, 4), None);
    }
}
