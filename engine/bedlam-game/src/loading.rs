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
//! FUN_0043c87c(str[0x45], bank, 0x96, 0x82);            // text, row 150
//! FUN_0043c87c(str[0x46], bank, 0xb4, 0x82);            // row 180
//! FUN_0043c87c(table + (zone+0x51)*0x30, bank, 0xd2, 0x82); // row 210
//! if (zone == 6) FUN_0043c87c(str[0x58], bank, 0x104, 0x82); // row 260
//! FUN_00425a03(); 0x60 bytes FULLPAL buf+2 -> DAC buf+0x2a2; // font ramp
//! FUN_0041cbf0(_DAT_004edbf8, 10);                      // FadeSetup 10 steps
//! ```
//!
//! FUN_0043c87c argument semantics [verified D35 from the Ghidra
//! listing ghidra-project/exw-font-drawer.txt + the FUN_00401ca2
//! body]: EAX = string, EDX = the FULLFONT bank, EBX = the draw ROW,
//! ECX = the glyph entry base (0x82 = entry 130 = char 0x21). x0 is
//! computed INSIDE the drawer: pen = 0x140 - total_width/2 - each
//! line centers on screen x 320. The four EBX values 0x96/0xb4/0xd2/
//! 0x104 are four text ROWS (150/180/210/260), NOT x columns - D34
//! recorded the pair swapped; corrected here (D35).
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
//! - Palette tail, D35 model: the EXW tail fills DAC commit-buffer
//!   bytes 0x2a2..=0x301 = entries 224..=255 with 0x3f and commits
//!   [verified fill + FUN_004258d0] - a TRANSIENT pre-text state -
//!   then, after the four text draws, copies the FULLPAL.PAL ramp
//!   (load buffer +2, 0x60 bytes) over the same region and arms the
//!   fade [verified: MOVSD 24 dwords + 0 tail into buf+0x2a2, then
//!   FUN_0041cbf0]. The fade TARGET therefore carries the ramp in
//!   entries 224..=255, which is what this flow reproduces; under the
//!   from-black fade design the transient 0x3f fill is never
//!   displayed.
//! - Text rows, D35: the four FUN_0043c87c draws (rows 150/180/210,
//!   +260 when the completed EXW zone == 6) land on the
//!   loading-screen raster at Loading entry, strings from the
//!   LANGUAGE [MENU_ITEMS] table (entries 0x45, 0x46, zone+0x51, and
//!   0x58 for zone 6), glyphs from FULLFONT.BIN entry 0x82 + (c -
//!   0x21) with the FUN_00410493 accent remap (crate::font). The
//!   geometry stays pinned as flow state (`text_row`) for
//!   introspection; the draws themselves run through the staged
//!   font.
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

/// The transient tail fill: 6-bit (0x3f, 0x3f, 0x3f). The EXW tail
/// commits this into DAC entries 224..=255 BEFORE the text draws; the
/// ramp copy overwrites the same bytes in the fade target, so under
/// the from-black fade design it is never displayed (kept as the
/// verified RE fact, not applied to the target).
pub const TAIL_COLOR: Vga6 = [0x3f, 0x3f, 0x3f];

/// Loading-text draw rows, draws 1..=3 [verified D35: the
/// FUN_0043c87c EBX arg is the blit ROW via FUN_00401ca2 ECX; 0x96 /
/// 0xb4 / 0xd2 = 150 / 180 / 210. D34 recorded these as x columns
/// with y = 0x82; 0x82 is the GLYPH ENTRY BASE (the ECX arg) and x0
/// is computed inside the drawer - the pair was swapped, corrected in
/// D35].
pub const TEXT_ROWS: [i32; 3] = [0x96, 0xb4, 0xd2];

/// The zone-6-only fourth draw row [verified: `zone == 6` -> 0x104 =
/// 260; the string is table entry 0x58].
pub const TEXT_ROW_ZONE6: i32 = 0x104;

/// Which text rows the loading screen draws for the just-completed
/// EXW zone: three always, the fourth only for zone 6 (the transition
/// INTO the endgame zone). Zone 7 never reaches this code (the
/// endgame arm has no loading screen) - it folds onto the 3-row
/// baseline [design: defensive].
pub fn text_rows(exw_zone: u8) -> &'static [i32] {
    if exw_zone == 6 {
        &[TEXT_ROWS[0], TEXT_ROWS[1], TEXT_ROWS[2], TEXT_ROW_ZONE6]
    } else {
        &TEXT_ROWS
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
/// palette. The validated parser expands 6-bit components to 8 bits
/// via `(v << 2) | (v >> 4)`; folding back with `>> 2` is lossless
/// for every 6-bit value (the identical argument MoviePlayer::palette
/// makes for the Smacker PALMAP), so the round trip pins the
/// file-owned 6-bit values exactly. The EXW 0x3f tail fill does NOT
/// reach the fade target (the FULLPAL ramp overwrites it - see
/// `LoadingFlow::enter_loading`); without a staged ramp the tail
/// keeps the folded file values.
pub(crate) fn loading_palette(pal770: &[u8]) -> Result<[Vga6; 256], GameError> {
    let parsed = bedlam_assets::pal::parse_vga770(pal770)?;
    let mut out = [[0u8; 3]; 256];
    for (dst, src) in out.iter_mut().zip(parsed.0) {
        *dst = [src[0] >> 2, src[1] >> 2, src[2] >> 2];
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

/// The loading-screen text rows pinned into flow state (D35): the
/// zone-dependent draw rows. Introspection only - the draws themselves
/// run at Loading entry through the staged font (crate::font).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRow {
    pub rows: &'static [i32],
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
    /// The text rows for the loading screen (set on Loading entry).
    pub(crate) text_row: Option<TextRow>,
    /// The staged loading font (FULLFONT.BIN through crate::font),
    /// drawn onto the loading screen at Loading entry.
    pub(crate) font: Option<crate::font::LoadingFont>,
    /// The staged LANGUAGE [MENU_ITEMS] table (bedlam-assets
    /// language parser).
    pub(crate) table: Option<Vec<Vec<u8>>>,
    /// The staged FULLPAL.PAL font ramp (entries 224..=255 of the
    /// fade target).
    pub(crate) ramp: Option<[[u8; 3]; 32]>,
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
            font: None,
            table: None,
            ramp: None,
        }
    }

    /// Enter the Between phase (host: on Cutscene with the cutscene
    /// movie finished or absent).
    pub(crate) fn enter_between(&mut self) {
        self.phase = LoadingPhase::Between;
    }

    /// Enter the Loading phase (host: on the Cutscene -> Select
    /// transition). Arms the fade, pins the text rows for the
    /// just-completed EXW zone (= stage - 1), runs the four text
    /// draws onto the loading-screen raster, and copies the staged
    /// font ramp into the fade-target tail - the EXW order: still ->
    /// 0x3f commit -> text draws -> ramp copy -> FadeSetup.
    pub(crate) fn enter_loading(&mut self, exw_zone: u8) {
        self.phase = LoadingPhase::Loading;
        self.fade_step = 0;
        self.fade_acc = 0;
        let rows = text_rows(exw_zone);
        self.text_row = Some(TextRow { rows });
        self.draw_loading_text(rows, exw_zone);
        self.apply_font_ramp();
    }

    /// The four LAB_0041c69e text draws [verified D35]: table entries
    /// 0x45, 0x46, zone+0x51 (0x52..=0x57 for zones 1..=6), and 0x58
    /// for zone 6 only - each blitted through the staged font,
    /// centered on x0 = 0x140 - total/2, at its row. Any missing part
    /// (no staged font / table / screen) skips the draws [deviation:
    /// EXW always has all three; a host that staged none has nothing
    /// to draw]. A table shorter than an index skips that draw
    /// [deviation: EXW reads its 0x30-stride slot regardless].
    fn draw_loading_text(&mut self, rows: &'static [i32], exw_zone: u8) {
        let font = self.font.as_ref();
        let table = self.table.as_ref();
        let Some(still) = self.screen.as_mut() else {
            return;
        };
        let (Some(font), Some(table)) = (font, table) else {
            return;
        };
        let indices = [0x45usize, 0x46, usize::from(exw_zone) + 0x51, 0x58];
        // rows holds 4 entries only for zone 6; zip stops at 3
        // otherwise, exactly the EXW `if (zone == 6)` guard.
        for (&row, &idx) in rows.iter().zip(indices.iter()) {
            if let Some(text) = table.get(idx) {
                font.draw(&mut still.pixels, still.w as usize, text, row);
            }
        }
    }

    /// Copy the staged FULLPAL ramp over the fade-target tail (DAC
    /// buffer +0x2a2 = entries 224..=255) [verified D35: 0x60 bytes
    /// MOVSD from the FULLPAL load buffer +2, then FadeSetup]. The
    /// transient 0x3f fill never reaches the target; without a staged
    /// ramp the folded LOADPAL tail values stand.
    fn apply_font_ramp(&mut self) {
        if let (Some(ramp), Some(target)) = (self.ramp, self.target.as_mut()) {
            target[TAIL_FIRST_ENTRY..=TAIL_LAST_ENTRY].copy_from_slice(&ramp);
        }
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
    fn palette_folds_losslessly_across_the_whole_range() {
        let pal = loading_palette(&synth_pal()).unwrap();
        // Every entry: the file-owned 6-bit values survive the
        // expand-then-fold round trip exactly - including the tail
        // range (the 0x3f fill is a transient EXW DAC state, not the
        // fade target; the ramp overwrites the target tail).
        let expect = (0..256u32).map(|i| {
            [
                ((i * 3) & 0x3f) as u8,
                ((i * 3 + 1) & 0x3f) as u8,
                ((i * 3 + 2) & 0x3f) as u8,
            ]
        });
        for (i, (got, want)) in pal.iter().zip(expect).enumerate() {
            assert_eq!(got, &want, "entry {i}");
        }
        // Boundary exactness: 0x2a2 = entry 224 byte 0, 0x301 = entry
        // 255 byte 2 - the 32 ramp-replaced entries span exactly the
        // EXW fill/copy range.
        assert_eq!(TAIL_LAST_ENTRY - TAIL_FIRST_ENTRY + 1, 32);
        // Short palette file: typed rejection via the parser.
        assert!(loading_palette(&[0u8; 769]).is_err());
    }

    #[test]
    fn enter_loading_draws_the_rows_and_applies_the_ramp() {
        use crate::font::synth;
        // Full-width still so the centered pens land on-plane: 640 x
        // 300 (rows 150/180/210 + glyph extents fit; the zone-6 row
        // 260 + 24-row glyph extents fits too).
        let mut flow = LoadingFlow::staged();
        flow.screen = Some(decode_entry0(&synth_bin(640, 300, 0x10)).unwrap());
        flow.target = Some(loading_palette(&synth_pal()).unwrap());
        flow.font = Some(crate::font::LoadingFont::from_bank(&synth::font_bin()).unwrap());
        flow.table = Some(
            bedlam_assets::language::parse_menu_items(&synth::language_bin(b"Congrats!")).unwrap(),
        );
        flow.ramp = Some(bedlam_assets::pal::parse_font_ramp(&synth::fullpal_bin()).unwrap());
        let ramp = bedlam_assets::pal::parse_font_ramp(&synth::fullpal_bin()).unwrap();
        flow.enter_loading(6);
        // The fade target tail now carries the ramp, not the folded
        // file values and not the 0x3f transient.
        let target = flow.target.unwrap();
        assert_eq!(&target[TAIL_FIRST_ENTRY..=TAIL_LAST_ENTRY], &ramp[..]);
        assert_eq!(target[0], [0, 1, 2], "pre-tail entry 0 stays folded");
        // The four rows drew: congrats (bang glyphs, fill 0xF0) at
        // row 150, move-out (E..U glyphs, fills 0xF1/0xF2) at 180,
        // the zone-6 table string at 210 and the 0x58 string at 260.
        // 0x10 = the still fill; glyph fills are 0xF0..=0xF5.
        let still = flow.screen.as_ref().unwrap();
        let stride = still.w as usize;
        for row in [150usize, 180, 210, 260] {
            let band = &still.pixels[row * stride..(row + 2) * stride];
            assert!(
                band.iter().any(|&v| (0xF0..=0xF5).contains(&v)),
                "row {row}: glyphs drew"
            );
        }
        // Above the first row: untouched still fill.
        let clean = &still.pixels[100 * stride..110 * stride];
        assert!(clean.iter().all(|&v| v == 0x10));
        // Without a staged font the raster stays pristine (a host
        // that staged no font has nothing to draw) - fresh flow.
        let mut bare = LoadingFlow::staged();
        bare.screen = Some(decode_entry0(&synth_bin(640, 300, 0x10)).unwrap());
        bare.target = Some(loading_palette(&synth_pal()).unwrap());
        bare.enter_loading(3);
        assert!(
            bare.screen
                .as_ref()
                .unwrap()
                .pixels
                .iter()
                .all(|&v| v == 0x10),
            "no staged font: no draws"
        );
        // ...and without a ramp the folded tail stands.
        assert_eq!(
            bare.target.unwrap()[255],
            [
                ((255 * 3) & 0x3f) as u8,
                ((255 * 3 + 1) & 0x3f) as u8,
                ((255 * 3 + 2) & 0x3f) as u8
            ]
        );
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
    fn text_rows_add_the_fourth_draw_only_for_zone_six() {
        assert_eq!(text_rows(1), &[150, 180, 210]);
        assert_eq!(text_rows(5), &[150, 180, 210]);
        assert_eq!(text_rows(6), &[150, 180, 210, 260]);
        // Zone 7 = the endgame arm (never draws); defensive baseline.
        assert_eq!(text_rows(7), &[150, 180, 210]);
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
                rows: &[150, 180, 210, 260]
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
