//! Post-cutscene loading flow (P5, D34): the EXW zone-transition tail
//! of GameMain LAB_0041c69e, reproduced as a presentation-side
//! sequence on the Cutscene -> Select transition.
//!
//! EXW tail [verified, docs/RE-EXW-GAMETHREAD.md; assets pinned in
//! bedlam-assets tests/loading_gate.rs], non-endgame arm only
//! (`_DAT_004edd8c != 7`):
//!
//! ```text
//! FUN_0044567c("GAMEGFX\\ZONEDONE.SMK", 0);            // the cutscene (D32)
//! FUN_0041db89(310000);                                 // scratch alloc
//! FUN_0041cc7f("GAMEGFX\\BETWEEN.BIN", ...);            // interlude still
//! FUN_00401e39(0,1,0,0);                                // draw entry 0
//! FUN_0042597c(...);                                    // present
//! FUN_0041db89(300000);                                 // scratch alloc
//! FUN_0041cc7f("GAMEGFX\\LOAD_UK|US.BIN", ...);         // region variant
//! FUN_0041cc7f("GAMEGFX\\LOADPAL|LOADPALU.PAL", _DAT_004edbf8);
//! .. DAC buffer bytes 0x2a2..0x301 = 0x3f; FUN_004258d0(buf);
//! FUN_0043c87c(.., 0x96, 0x82);                         // text row, y=130
//! FUN_0043c87c(.., 0xb4, 0x82);
//! FUN_0043c87c(&DAT_0046af5c + (zone+0x51)*0x30, 0xd2, 0x82);
//! if (zone == 6) FUN_0043c87c(.., 0x104, 0x82);         // 4th column
//! FUN_00425a03(); .. font ramp into buf+0x2a2 ..
//! FUN_0041cbf0(_DAT_004edbf8, 10);                      // FadeSetup 10 steps
//! ```
//!
//! Reproduction mapping [design where tagged]:
//! - BETWEEN (the interlude still, entry 0) owns the Cutscene plane
//!   once the cutscene MOVIE finishes - EXW shows it for the
//!   un-RE-ed decode window between the movie runner returning and
//!   the LOAD assets being ready; here that window is the user-paced
//!   gap between movie end and the advance that leaves Cutscene. A
//!   skip-advance before movie end bypasses the interlude visual but
//!   still runs the loading screen (the EXW tail is unconditional
//!   after the movie call, finish or skip) [design: no BETWEEN hold
//!   duration is RE-ed].
//! - The loading screen (LOAD_UK/US.BIN entry 0 + LOADPAL/LOADPALU.PAL
//!   selected by movies::Region per EXW DAT_0046ae64) owns the Select
//!   plane on the Cutscene -> Select transition, fading in from black
//!   over FadeSetup 10 steps at 50 Hz on the same x240-us integer
//!   accumulator grid the movie player uses (20 ms = 4_800_000 units;
//!   steps land on whole elapsed periods, never rounded) [design:
//!   from-black; the EXW FadeStep body is only partially RE-ed -
//!   RE-EXW-TICK D15].
//! - Palette tail: DAC commit-buffer bytes 0x2a2..=0x301 = palette
//!   entries 224..=255 forced to 6-bit (0x3f, 0x3f, 0x3f) [verified
//!   fill; the later font-ramp copy into the same region belongs to
//!   the FULLFONT text pass, queued].
//! - Text row: four possible draws at y = 0x82, x = 0x96/0xb4/0xd2
//!   (+0x104 when the completed EXW zone == 6). The glyph pass needs
//!   FULLFONT.BIN + FUN_0043c87c semantics (not yet RE-ed); this unit
//!   pins the geometry as flow state (`text_row`) so the font pass
//!   consumes it without re-deriving the zone logic [design].
//! - The 310000/300000 FUN_0041db89 allocs are EXW decode scratch
//!   (just under the 640x480 = 307200 rasters); the Rust analog is
//!   the decoded Vec itself - internal representation, parity budget
//!   T3, not reproduced literally.
//! - BETWEEN is presented under the STANDING host palette: EXW makes
//!   no DAC change between the movie runner returning and the LOADPAL
//!   commit (the last FULLPAL.PAL load sits earlier in the same
//!   LAB_0041c69e block). Which palette the DAC actually held at
//!   BETWEEN-present time (FULLPAL vs the movie last upload) is not
//!   RE-ed [open: FUN_0044567c exit path].
//! - The endgame arm (`_DAT_004edd8c == 7`: END.SMK then credits,
//!   FUN_0041c9f0) loads NO BETWEEN/LOAD assets [verified code path]:
//!   a staged flow is dropped at Cutscene/Select when the episode
//!   stage has reached MAX_STAGE (the post-END continuation shape the
//!   test FSM allows).
//!
//! D17 bucket b: the whole flow is presentation. It never touches the
//! sim, the scene hash, or any hashed bucket; its planes ride the
//! existing MovieFrame seam (a full-screen 640x480 raster centers at
//! the origin, i.e. the 1:1 no-letterbox blit the loading gate pins).

use bedlam_render::Vga6;

use crate::fsm::MAX_STAGE;
use crate::movie::{UNITS_PER_SUBTICK, UNITS_PER_US};
use crate::GameError;

/// FadeSetup step count [verified: FUN_0041cbf0(pal, 10)].
pub const FADE_STEPS: u16 = 10;

/// One fade step = 20 ms at 50 Hz, in x240-us units (20_000 * 240).
/// Non-integer on the 240 Hz sub-tick grid (4.8 sub-ticks), so steps
/// land on an accumulator exactly like movie frame periods.
pub const FADE_STEP_UNITS: u64 = 20_000 * UNITS_PER_US;

/// First DAC entry of the forced tail (buffer byte 0x2a2 = entry
/// (0x2a2-2)/3 = 224 exactly) [verified fill range 0x2a2..=0x301].
pub const TAIL_FIRST_ENTRY: usize = 224;

/// Last DAC entry of the forced tail (buffer byte 0x301 is the final
/// byte of entry 255 - the 0x302-byte commit buffer ends there).
pub const TAIL_LAST_ENTRY: usize = 255;

/// The forced tail value: 6-bit (0x3f, 0x3f, 0x3f).
pub const TAIL_COLOR: Vga6 = [0x3f, 0x3f, 0x3f];

/// Loading-screen text row y coordinate [verified: FUN_0043c87c arg
/// 0x82 on all four draws].
pub const TEXT_Y: i32 = 0x82;

/// Text row x coordinates, draws 1..=3 [verified: 0x96 / 0xb4 / 0xd2].
pub const TEXT_XS: [i32; 3] = [0x96, 0xb4, 0xd2];

/// The zone-6-only fourth draw [verified: `if (zone == 6)` -> 0x104].
pub const TEXT_X_ZONE6: i32 = 0x104;

/// Which x columns the loading text row draws for the just-completed
/// EXW zone: three always, the fourth only for zone 6 (the transition
/// INTO the endgame zone). Zone 7 never reaches this code (the
/// endgame arm has no loading screen) - it folds onto the 3-column
/// baseline [design: defensive].
pub fn text_x_columns(exw_zone: u8) -> &'static [i32] {
    if exw_zone == 6 {
        &[TEXT_XS[0], TEXT_XS[1], TEXT_XS[2], TEXT_X_ZONE6]
    } else {
        &TEXT_XS
    }
}

/// Whether the zone-transition tail runs at this episode stage.
///
/// EXW reads the zone counter `_DAT_004edd8c` BEFORE its post-tail
/// increment, so the just-completed zone in this FSM terms is `stage - 1`
/// (Episode::complete already advanced it - the same
/// reconciliation as movies::cutscene_name). The tail exists only in
/// the non-endgame arm: stages 2..=7 (completed zones 1..=6). Stage 8
/// = MAX_STAGE is the endgame/credits arm, which loads no BETWEEN or
/// LOAD assets [verified code path]; stage 1 never stands on Cutscene.
pub fn flow_armed_at_stage(stage: u8) -> bool {
    (2..MAX_STAGE).contains(&stage)
}

/// One decoded full-screen still (BETWEEN.BIN / LOAD_*.BIN entry 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Still {
    pub(crate) pixels: Box<[u8]>,
    pub(crate) w: u32,
    pub(crate) h: u32,
}

/// Decode entry 0 of a loading-flow BIN bank through the validated
/// sprites parser. The corpus banks are single-image (loading gate),
/// but any decodable entry 0 is accepted: EXW draws `FUN_00401e39(0,
/// ..)` = entry 0, whatever the bank holds.
pub(crate) fn decode_entry0(bin: &[u8]) -> Result<Still, GameError> {
    let bank = bedlam_assets::sprites::parse_bin_images(bin)?;
    let img = bank.images.first().ok_or(GameError::BadLoadingAsset {
        what: "image bank",
        reason: "no entries",
    })?;
    let want = img.w as usize * img.h as usize;
    let pixels =
        img.pixels
            .as_deref()
            .filter(|p| p.len() == want)
            .ok_or(GameError::BadLoadingAsset {
                what: "image bank entry 0",
                reason: "undecoded raster",
            })?;
    Ok(Still {
        pixels: pixels.to_vec().into_boxed_slice(),
        w: u32::from(img.w),
        h: u32::from(img.h),
    })
}

/// Fold a 770-byte LOADPAL/LOADPALU.PAL file to the canonical 6-bit
/// palette and force the EXW tail. The validated parser expands 6-bit
/// components to 8 bits via `(v << 2) | (v >> 4)`; folding back with
/// `>> 2` is lossless for every 6-bit value (the identical argument
/// MoviePlayer::palette makes for the Smacker PALMAP), so the round
/// trip pins the file-owned 6-bit values exactly. Then entries
/// TAIL_FIRST_ENTRY..=TAIL_LAST_ENTRY become the uniform 0x3f tail -
/// the byte-range fill the EXW tail performs on the DAC commit buffer
/// before FUN_004258d0 commits it.
pub(crate) fn loading_palette(pal770: &[u8]) -> Result<[Vga6; 256], GameError> {
    let parsed = bedlam_assets::pal::parse_vga770(pal770)?;
    let mut out = [[0u8; 3]; 256];
    for (dst, src) in out.iter_mut().zip(parsed.0) {
        *dst = [src[0] >> 2, src[1] >> 2, src[2] >> 2];
    }
    for entry in &mut out[TAIL_FIRST_ENTRY..=TAIL_LAST_ENTRY] {
        *entry = TAIL_COLOR;
    }
    Ok(out)
}

/// Fade palette at `step` of FADE_STEPS: integer lerp from black,
/// `(component * step) / FADE_STEPS` per 6-bit channel. Step 0 is all
/// black, step FADE_STEPS is exactly the target; monotone in between
/// and drift-free (pure integer math) [design: from-black fade-in;
/// per-step SetPaletteRGB on all 256 entries is the compose-doc
/// FadeStep analog, expressed as palette_dirty on the movie plane].
pub fn fade_palette(target: &[Vga6; 256], step: u16) -> [Vga6; 256] {
    let mut out = [[0u8; 3]; 256];
    if step == 0 {
        return out;
    }
    let s = u32::from(step);
    for (dst, c) in out.iter_mut().zip(target) {
        for k in 0..3 {
            dst[k] = ((u32::from(c[k]) * s) / u32::from(FADE_STEPS)) as u8;
        }
    }
    out
}

/// The presentation phase of the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingPhase {
    /// Decoded and waiting for the Cutscene tail (inert everywhere).
    Staged,
    /// The BETWEEN interlude owns the Cutscene plane (movie over).
    Between,
    /// The loading screen owns the Select plane (fading in / held).
    Loading,
}

/// The loading-screen text row pinned into flow state for the future
/// FULLFONT glyph pass: y coordinate plus the zone-dependent x columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRow {
    pub y: i32,
    pub xs: &'static [i32],
}

/// The whole post-cutscene flow: both stills, the fade engine, and the
/// pinned text row. Owned by the GameHost; staged by
/// [`crate::host::GameHost::load_interlude`] /
/// [`crate::host::GameHost::load_loading_screen`], phase-driven by the
/// host scene sync.
#[derive(Debug)]
pub(crate) struct LoadingFlow {
    pub(crate) phase: LoadingPhase,
    pub(crate) between: Option<Still>,
    pub(crate) screen: Option<Still>,
    pub(crate) target: Option<[Vga6; 256]>,
    /// Fade steps completed, 0..=FADE_STEPS.
    pub(crate) fade_step: u16,
    /// Elapsed x240-us units since the last fade step boundary.
    fade_acc: u64,
    /// Whether the flow has stood on its Cutscene (gates the Select
    /// arming: only a post-cutscene Select runs the loading screen).
    pub(crate) saw_cutscene: bool,
    /// The text row for the loading screen (set on Loading entry).
    pub(crate) text_row: Option<TextRow>,
}

impl LoadingFlow {
    pub(crate) fn staged() -> LoadingFlow {
        LoadingFlow {
            phase: LoadingPhase::Staged,
            between: None,
            screen: None,
            target: None,
            fade_step: 0,
            fade_acc: 0,
            saw_cutscene: false,
            text_row: None,
        }
    }

    /// Enter the Between phase (host: on Cutscene with the cutscene
    /// movie finished or absent).
    pub(crate) fn enter_between(&mut self) {
        self.phase = LoadingPhase::Between;
    }

    /// Enter the Loading phase (host: on the Cutscene -> Select
    /// transition). Arms the fade and pins the text row for the
    /// just-completed EXW zone (= stage - 1).
    pub(crate) fn enter_loading(&mut self, exw_zone: u8) {
        self.phase = LoadingPhase::Loading;
        self.fade_step = 0;
        self.fade_acc = 0;
        self.text_row = Some(TextRow {
            y: TEXT_Y,
            xs: text_x_columns(exw_zone),
        });
    }

    /// Feed one host-frame dt (sub-ticks on the 240 Hz grid). Only the
    /// Loading phase consumes time: each fully elapsed 20 ms steps the
    /// fade once, banking the remainder - the same accumulator
    /// discipline as MoviePlayer, so any host chunking yields the same
    /// palette at the same wall time. The step count saturates at
    /// FADE_STEPS (the screen then holds at full brightness until the
    /// scene is left).
    pub(crate) fn advance(&mut self, dt_subticks: u32) {
        if self.phase != LoadingPhase::Loading {
            return;
        }
        self.fade_acc += u64::from(dt_subticks) * UNITS_PER_SUBTICK;
        while self.fade_step < FADE_STEPS && self.fade_acc >= FADE_STEP_UNITS {
            self.fade_step += 1;
            self.fade_acc -= FADE_STEP_UNITS;
        }
        if self.fade_step == FADE_STEPS {
            self.fade_acc = 0;
        }
    }
}

/// A still plane ready for the MovieFrame seam: borrowed raster plus
/// the owned palette the still is presented under this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plane<'a> {
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) pixels: &'a [u8],
    pub(crate) palette: [Vga6; 256],
}

impl LoadingFlow {
    /// The plane owning the screen in the current phase, if any.
    /// Between presents the interlude under the standing host palette
    /// (the DAC-out analog); Loading presents the screen under the
    /// current fade step of the tail-forced target palette.
    pub(crate) fn plane(&self, host_palette: &[Vga6; 256]) -> Option<Plane<'_>> {
        match self.phase {
            LoadingPhase::Staged => None,
            LoadingPhase::Between => self.between.as_ref().map(|still| Plane {
                w: still.w,
                h: still.h,
                pixels: &still.pixels,
                palette: *host_palette,
            }),
            LoadingPhase::Loading => {
                let target = self.target?;
                let still = self.screen.as_ref()?;
                Some(Plane {
                    w: still.w,
                    h: still.h,
                    pixels: &still.pixels,
                    palette: fade_palette(&target, self.fade_step),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Single-image raw BIN bank (the sprites-test convention): a
    /// plain raster behind a hotspot-free header, full-screen shape.
    fn synth_bin(w: u16, h: u16, fill: u8) -> Vec<u8> {
        let mut img = Vec::new();
        img.extend_from_slice(&0u16.to_le_bytes()); // flags: raw
        img.extend_from_slice(&(w as i16).to_le_bytes());
        img.extend_from_slice(&(h as i16).to_le_bytes());
        img.extend(std::iter::repeat_n(fill, w as usize * h as usize));
        let mut v = 1u16.to_le_bytes().to_vec();
        v.extend_from_slice(&4u32.to_le_bytes()); // off: entry 0 -> start 6
        v.extend_from_slice(&img);
        v
    }

    /// 770-byte palette file with per-entry distinct 6-bit content.
    fn synth_pal() -> Vec<u8> {
        let mut d = vec![0u8; 770];
        for i in 0..256usize {
            for c in 0..3usize {
                d[2 + i * 3 + c] = ((i * 3 + c) & 0x3f) as u8;
            }
        }
        d
    }

    #[test]
    fn entry0_decodes_through_the_validated_parser() {
        let still = decode_entry0(&synth_bin(4, 3, 0xAB)).unwrap();
        assert_eq!((still.w, still.h), (4, 3));
        assert_eq!(&still.pixels[..], &[0xABu8; 12][..]);
        // Multi-entry bank: entry 0 wins (EXW draws entry 0).
        let a = synth_bin(2, 2, 0x11);
        let first = &a[6..]; // image 0 bytes (header + raster)
        let mut img2 = Vec::new();
        img2.extend_from_slice(&0u16.to_le_bytes());
        img2.extend_from_slice(&2i16.to_le_bytes());
        img2.extend_from_slice(&2i16.to_le_bytes());
        img2.extend([0x22u8; 4]);
        let mut bank = 2u16.to_le_bytes().to_vec();
        bank.extend_from_slice(&4u32.to_le_bytes()); // slot 0 -> start 6
        bank.extend_from_slice(&(first.len() as u32).to_le_bytes()); // slot 1
        bank.extend_from_slice(first);
        bank.extend_from_slice(&img2);
        let still = decode_entry0(&bank).unwrap();
        assert_eq!(&still.pixels[..], &[0x11u8; 4][..], "entry 0, not 1");
    }

    #[test]
    fn undecodable_banks_are_rejected_with_the_typed_error() {
        // Empty-slot entry (no raster): structurally parsed,
        // staging-rejected.
        let empty = {
            let mut v = 1u16.to_le_bytes().to_vec();
            v.extend_from_slice(&100u32.to_le_bytes()); // off past EOF
            v
        };
        match decode_entry0(&empty) {
            Err(GameError::BadLoadingAsset {
                what: "image bank entry 0",
                reason: "undecoded raster",
            }) => {}
            other => panic!("wrong error: {other:?}"),
        }
        // Zero-entry bank and garbage: the parser rejects.
        assert!(decode_entry0(&[0u8, 0, 0, 0, 0, 0]).is_err());
        assert!(decode_entry0(&[1u8, 2, 3]).is_err());
    }

    #[test]
    fn palette_folds_losslessly_and_forces_the_tail() {
        let pal = loading_palette(&synth_pal()).unwrap();
        // Entries below the tail: the file-owned 6-bit values survive
        // the expand-then-fold round trip exactly.
        let expect_pre_tail = (0..TAIL_FIRST_ENTRY as u32).map(|i| {
            [
                ((i * 3) & 0x3f) as u8,
                ((i * 3 + 1) & 0x3f) as u8,
                ((i * 3 + 2) & 0x3f) as u8,
            ]
        });
        for (i, (got, want)) in pal.iter().zip(expect_pre_tail).enumerate() {
            assert_eq!(got, &want, "entry {i}");
        }
        // The tail: forced 0x3f regardless of file content.
        for entry in &pal[TAIL_FIRST_ENTRY..=TAIL_LAST_ENTRY] {
            assert_eq!(entry, &TAIL_COLOR);
        }
        // Boundary exactness: 0x2a2 = entry 224 byte 0, 0x301 = entry
        // 255 byte 2 - the 32 forced entries span exactly the fill.
        assert_eq!(TAIL_LAST_ENTRY - TAIL_FIRST_ENTRY + 1, 32);
        // Short palette file: typed rejection via the parser.
        assert!(loading_palette(&[0u8; 769]).is_err());
    }

    #[test]
    fn fade_lerps_from_black_to_the_target_in_integer_steps() {
        let mut target = [[0u8; 3]; 256];
        target[0] = [12, 34, 56];
        target[255] = [63, 63, 63];
        assert_eq!(fade_palette(&target, 0), [[0u8; 3]; 256], "step 0: black");
        assert_eq!(fade_palette(&target, FADE_STEPS), target, "full: exact");
        assert_eq!(
            fade_palette(&target, 5)[0],
            [
                (12u32 * 5 / 10) as u8,
                (34 * 5 / 10) as u8,
                (56 * 5 / 10) as u8
            ],
            "half: integer lerp"
        );
        // Monotone per component across all steps.
        for entry in [0usize, 255] {
            for c in 0..3 {
                let mut prev = 0u8;
                for step in 0..=FADE_STEPS {
                    let v = fade_palette(&target, step)[entry][c];
                    assert!(v >= prev, "entry {entry} ch {c} step {step}");
                    prev = v;
                }
            }
        }
    }

    #[test]
    fn fade_paces_50hz_steps_on_the_subtick_accumulator() {
        // 60 Hz host: 4 sub-ticks per pump. Step boundaries at 4.8
        // sub-ticks: pumps 2..=6 fire steps 1..=5, pump 7 banks
        // without a step, pumps 8..=12 fire steps 6..=10. Twelve pumps
        // = 48 sub-ticks = 200 ms = 10 steps at 50 Hz exactly.
        for (pump, want) in [
            (1usize, 0u16),
            (2, 1),
            (3, 2),
            (4, 3),
            (5, 4),
            (6, 5),
            (7, 5),
            (8, 6),
            (9, 7),
            (10, 8),
            (11, 9),
            (12, 10),
        ] {
            let mut f = LoadingFlow::staged();
            f.enter_loading(0);
            for _ in 0..pump {
                f.advance(4);
            }
            assert_eq!(f.fade_step, want, "after {pump} pumps");
        }
        // Saturated: extra time changes nothing.
        let mut f = LoadingFlow::staged();
        f.enter_loading(0);
        f.advance(4 * 100);
        assert_eq!(f.fade_step, FADE_STEPS);
        // Chunking invariance: the same TOTAL dt (48 sub-ticks = 200
        // ms) lands on step 10 however it was chunked.
        let mut one = LoadingFlow::staged();
        one.enter_loading(0);
        for _ in 0..48 {
            one.advance(1);
        }
        let mut big = LoadingFlow::staged();
        big.enter_loading(0);
        for _ in 0..2 {
            big.advance(24);
        }
        assert_eq!(one.fade_step, big.fade_step);
        assert_eq!(one.fade_step, 10);
        // Staged flow consumes no time.
        let mut idle = LoadingFlow::staged();
        idle.advance(4 * 100);
        assert_eq!(idle.fade_step, 0);
    }

    #[test]
    fn text_columns_add_the_fourth_draw_only_for_zone_six() {
        assert_eq!(text_x_columns(1), &[150, 180, 210]);
        assert_eq!(text_x_columns(5), &[150, 180, 210]);
        assert_eq!(text_x_columns(6), &[150, 180, 210, 260]);
        // Zone 7 = the endgame arm (never draws); defensive baseline.
        assert_eq!(text_x_columns(7), &[150, 180, 210]);
        assert_eq!(TEXT_Y, 130);
    }

    #[test]
    fn flow_arms_only_on_the_zone_transition_stages() {
        for stage in 2..=7u8 {
            assert!(flow_armed_at_stage(stage), "stage {stage}");
        }
        for stage in [0u8, 1, MAX_STAGE, MAX_STAGE + 1, u8::MAX] {
            assert!(!flow_armed_at_stage(stage), "stage {stage}");
        }
    }

    #[test]
    fn planes_follow_the_phase() {
        let between = Still {
            pixels: vec![0xB7u8; 6].into_boxed_slice(),
            w: 3,
            h: 2,
        };
        let screen = Still {
            pixels: vec![0x10u8; 6].into_boxed_slice(),
            w: 3,
            h: 2,
        };
        let mut target = [[0u8; 3]; 256];
        target[0] = [60, 60, 60];
        let host_pal = [[7u8, 8, 9]; 256];

        let mut flow = LoadingFlow::staged();
        assert!(flow.plane(&host_pal).is_none(), "staged: no plane");
        assert!(flow.text_row.is_none());

        flow.between = Some(between);
        flow.enter_between();
        let plane = flow.plane(&host_pal).unwrap();
        assert_eq!(plane.pixels, &[0xB7u8; 6][..]);
        assert_eq!(plane.palette, host_pal, "interlude under host palette");

        flow.screen = Some(screen);
        flow.target = Some(target);
        flow.enter_loading(6);
        assert_eq!(
            flow.text_row,
            Some(TextRow {
                y: 130,
                xs: &[150, 180, 210, 260]
            })
        );
        let plane = flow.plane(&host_pal).unwrap();
        assert_eq!(plane.pixels, &[0x10u8; 6][..]);
        assert_eq!(plane.palette, [[0u8; 3]; 256], "fade step 0: black");
        flow.advance(4 * 12);
        let plane = flow.plane(&host_pal).unwrap();
        assert_eq!(plane.palette, target, "faded to the target");
    }
}
