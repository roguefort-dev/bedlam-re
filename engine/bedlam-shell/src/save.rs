//! The P6 QoL save-slot platform surface (D213; PLAN §6 "save slots +
//! metadata + opt-in autosave"): SLOT SELECTION, METADATA
//! PRESENTATION and the OPT-IN AUTOSAVE POLICY as platform knobs OUT
//! of `ModeConfig` (D200 layering), grounded in `docs/RE-EXW-SAVE.md`
//! (committed BEFORE this code, the RE-FIRST rule).
//!
//! The original save surface it re-anchors: FIVE 180-B slots
//! (0x4eae58, stride 0xB4) persisted as ONE 900-B registry value
//! "SAVEGAME" — never a mid-mission write, never automatic: the
//! exhaustive EXW writer census (RE-EXW-SAVE sec 4) shows EXACTLY TWO
//! savegame writers, the save screen's slot commit (0x446e98, reached
//! only from the SINGLE-PLAYER campaign-shell SAVE button) and the
//! first-run five-EMPTY initialization (0x44706c). The load side is
//! the title-menu "Start Saved Game" flow. The engine's READ side is
//! already byte-faithful and import-only (bedlam-game save.rs, the
//! §7j.70 seam); this module adds the platform presentation and
//! policy over it — the sim, the ModeConfig and every hash are
//! untouched by construction (the knobs never enter `SimConfig`;
//! pinned in window.rs).
//!
//! The surface lands INERT by design (the D201 seam precedent): the
//! new versioned save FORMAT writer is future engine work and lands
//! config-not-state when it does (a restore ADOPTS the saved session
//! — the title-menu shape — never a mid-run mutation); until then
//! these knobs select and describe, they do not write.

use bedlam_game::{import_saved_slot, SaveSlotImport, SAVED_SLOTS};

use crate::window::WindowOptions;

/// One save slot in the original's own FIVE-slot domain
/// (RE-EXW-SAVE sec 1; `SAVED_SLOTS`). 0-based like the restore's
/// slot dispatch (0x43c26e `imul edx,0xB4`); `Display` is the
/// player-facing 1-based "Slot N".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveSlotId(u8);

impl SaveSlotId {
    /// The first slot (index 0) — the platform default selection.
    pub const FIRST: SaveSlotId = SaveSlotId(0);

    /// The last slot (index 4; the original's fifth row).
    pub const LAST: SaveSlotId = SaveSlotId(SAVED_SLOTS as u8 - 1);

    /// Checked construction: only the five original slots exist,
    /// anything else is `None` (never a guess).
    pub fn new(index: u8) -> Option<SaveSlotId> {
        if index < SAVED_SLOTS as u8 {
            Some(SaveSlotId(index))
        } else {
            None
        }
    }

    /// The 0-based slot index (the restore's edx domain).
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl Default for SaveSlotId {
    fn default() -> SaveSlotId {
        SaveSlotId::FIRST
    }
}

impl std::fmt::Display for SaveSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Slot {}", self.0 + 1)
    }
}

/// The EXW save-game level text, byte-faithful (RE-EXW-SAVE sec 5;
/// FUN_004473cd): `""` for a zero zone, else ONE leading space, the
/// stage letter `'A' + zone - 1` (zone 1 = ZONEA .. 7 = ZONEG) and
/// one digit `'1'..'5'` per SET mask bit, ascending (bit 0 = '1',
/// bit 1 = '2', .. bit 4 = '5'). Examples: zone 3 mask 0b10011 =
/// `" C135"`; zone 2 mask 0b11111 = `" B12345"`; zone 1 mask 0b1 =
/// `" A1"`.
pub fn save_level_text(zone: u8, mask: u8) -> String {
    if zone == 0 {
        return String::new();
    }
    let mut text = String::with_capacity(7);
    text.push(' ');
    // 0x4473f2 `add al,0x40`: the zone word becomes its letter.
    text.push((b'A' + zone - 1) as char);
    for bit in 0..5 {
        if mask & (1 << bit) != 0 {
            text.push((b'1' + bit) as char);
        }
    }
    text
}

/// The line the original's save/load menus build for a USED slot
/// (RE-EXW-SAVE sec 5; the menu-3 construction 0x445dc7..0x445e37):
/// the slot name space-padded (0x20) out to 8 characters, then the
/// level text — e.g. `"PLAYER   C135"`.
pub fn slot_menu_line(name: &str, zone: u8, mask: u8) -> String {
    menu_line(name, &save_level_text(zone, mask))
}

fn menu_line(name: &str, level_text: &str) -> String {
    let mut line = String::with_capacity(8 + level_text.len());
    line.push_str(name);
    for _ in name.chars().count()..8 {
        line.push(' ');
    }
    line.push_str(level_text);
    line
}

/// The line for an EMPTY slot — the original's own string
/// (0x45980f, RE-EXW-SAVE secs 2/5).
pub const EMPTY_SLOT_LINE: &str = "EMPTY";

/// Presentation metadata for one imported slot — the original's
/// WHOLE metadata surface (RE-EXW-SAVE sec 5): the name, the level
/// text, and the record's score/money/difficulty (carried by the
/// slot record; the menu line itself shows only name + level text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    /// The slot this metadata describes.
    pub slot: SaveSlotId,
    /// The sanitized 8-char slot name.
    pub name: String,
    /// The EXW-faithful `" Z<digits>"` stage/done text.
    pub level_text: String,
    /// Hiscore/score dword @+0x0E.
    pub score: u32,
    /// Money dword @+0x12.
    pub money: u32,
    /// Difficulty SIGNED word @+0x16.
    pub difficulty: i16,
}

impl SaveSlotMetadata {
    /// Derive the presentation from an engine import (PURE).
    pub fn from_import(import: &SaveSlotImport) -> SaveSlotMetadata {
        SaveSlotMetadata {
            slot: SaveSlotId::new(import.slot as u8)
                .expect("the engine import domain is exactly the five slots"),
            name: import.name.clone(),
            level_text: save_level_text(import.stage, import.mask),
            score: import.score,
            money: import.money,
            difficulty: import.difficulty,
        }
    }

    /// The menu line for this slot (name padded to 8 + level text).
    pub fn menu_line(&self) -> String {
        menu_line(&self.name, &self.level_text)
    }
}

/// One row of the save/load screen list: a USED slot or an EMPTY one
/// (the original's five-row list; the empty predicate is the zone
/// dword at +0x0C, RE-EXW-SAVE sec 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSlotRow {
    /// A slot the empty predicate accepts.
    Used(SaveSlotMetadata),
    /// A slot whose zone dword is zero — shown as `EMPTY`.
    Empty,
}

impl SaveSlotRow {
    /// The text the original shows for this row.
    pub fn line(&self) -> String {
        match self {
            SaveSlotRow::Used(meta) => meta.menu_line(),
            SaveSlotRow::Empty => String::from(EMPTY_SLOT_LINE),
        }
    }
}

/// Summarize a whole original SAVED.BDL image into the five-row
/// save/load list (PURE; the menu-3/4 construction over the engine's
/// import-only seam). Every rejection stays loud (`GameError`) — the
/// image must be the exactly-900-B five-slot store; an EMPTY slot is
/// not an error, it is a row.
pub fn summarize_saved_bdl(data: &[u8]) -> Result<Vec<SaveSlotRow>, bedlam_game::GameError> {
    let mut rows = Vec::with_capacity(SAVED_SLOTS);
    for index in 0..SAVED_SLOTS as u8 {
        let slot = SaveSlotId::new(index).expect("the loop bound is the slot domain");
        rows.push(match import_saved_slot(data, slot.index()) {
            Ok(import) => SaveSlotRow::Used(SaveSlotMetadata::from_import(&import)),
            Err(bedlam_game::GameError::SaveSlotEmpty { .. }) => SaveSlotRow::Empty,
            Err(e) => return Err(e),
        });
    }
    Ok(rows)
}

/// The OPT-IN autosave policy (P6 QoL; RE-EXW-SAVE secs 4/6): `Off`
/// is the default and the shipped posture — the original NEVER
/// autosaves (the exhaustive writer census). `On` is a MODERN
/// platform addition whose save opportunities mirror the original's
/// own save gating (sec 3): single-player, campaign boundary — the
/// armed save button between missions — never mid-mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutosavePolicy {
    /// The shipped posture (default): no automatic saves.
    #[default]
    Off,
    /// Opt-in: autosave the campaign state to the selected slot at
    /// the original's own save opportunities.
    On(SaveSlotId),
}

impl AutosavePolicy {
    /// Whether the policy is opted in.
    pub fn is_on(&self) -> bool {
        matches!(self, AutosavePolicy::On(_))
    }

    /// The slot an autosave targets (None while Off).
    pub fn slot(&self) -> Option<SaveSlotId> {
        match self {
            AutosavePolicy::Off => None,
            AutosavePolicy::On(slot) => Some(*slot),
        }
    }

    /// The save-opportunity gate (PURE), mirroring the original's
    /// save-screen gating (RE-EXW-SAVE sec 3): offered ONLY in
    /// single-player (`0x4edb88 == 0` at the SAVE button) and ONLY at
    /// a campaign boundary (the campaign shell between missions —
    /// never mid-mission, never in the coop/h2h variants). `Off`
    /// answers false unconditionally — autosave is NEVER the default.
    pub fn should_autosave(&self, single_player: bool, at_campaign_boundary: bool) -> bool {
        match self {
            AutosavePolicy::Off => false,
            AutosavePolicy::On(_) => single_player && at_campaign_boundary,
        }
    }
}

/// Confirm a window host's save knobs are the shipped defaults (the
/// FIRST slot, the Off policy) — the pin behind
/// `WindowOptions::new`.
pub fn window_save_surface_is_shipped(opts: &WindowOptions) -> bool {
    opts.save_slot == SaveSlotId::FIRST && opts.autosave == AutosavePolicy::Off
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal well-formed 900-B image (the engine save.rs test
    /// shape): slot `slot` carrying name "SLOTS", zone, mask.
    fn image_with_slot(slot: usize, zone: u16, mask: u32) -> Vec<u8> {
        use bedlam_game::SAVED_LEN;
        let mut d = vec![0u8; SAVED_LEN];
        let b = slot * 180;
        d[b..b + 5].copy_from_slice(b"SLOTS");
        d[b + 0x08..b + 0x0C].copy_from_slice(&mask.to_le_bytes());
        d[b + 0x0C..b + 0x0E].copy_from_slice(&zone.to_le_bytes());
        d
    }

    #[test]
    fn slot_domain_is_the_original_five() {
        // RE-EXW-SAVE sec 1: FIVE slots, stride 0xB4, the staging
        // buffer 0x4eae58 holds exactly five.
        assert_eq!(SaveSlotId::FIRST.index(), 0);
        assert_eq!(SaveSlotId::LAST.index(), SAVED_SLOTS - 1);
        for index in 0..5u8 {
            assert_eq!(
                SaveSlotId::new(index).map(|s| s.index()),
                Some(index as usize)
            );
        }
        assert_eq!(SaveSlotId::new(5), None);
        assert_eq!(SaveSlotId::new(255), None);
        assert_eq!(SaveSlotId::default(), SaveSlotId::FIRST);
        assert_eq!(SaveSlotId::FIRST.to_string(), "Slot 1");
        assert_eq!(SaveSlotId::LAST.to_string(), "Slot 5");
    }

    #[test]
    fn save_level_text_is_exw_faithful() {
        // FUN_004473cd (RE-EXW-SAVE sec 5): "" for a zero zone; one
        // leading space + the stage letter + one digit per SET bit,
        // ascending 1..5.
        assert_eq!(save_level_text(0, 0), "");
        assert_eq!(save_level_text(0, 0b11111), "");
        assert_eq!(save_level_text(1, 0b1), " A1");
        assert_eq!(save_level_text(3, 0b10011), " C125");
        assert_eq!(save_level_text(2, 0b11111), " B12345");
        assert_eq!(save_level_text(7, 0b101), " G13");
        // Zone 8 is the endgame arm of the modeled episode space; the
        // text builder only ever sees zones the import accepted.
        assert_eq!(save_level_text(8, 0), " H");
    }

    #[test]
    fn slot_menu_line_pads_the_name_to_eight() {
        // The menu-3 construction: name space-padded (0x20) out to 8
        // characters, then the level text.
        assert_eq!(slot_menu_line("PLAYER", 2, 0b111), "PLAYER   B123");
        assert_eq!(slot_menu_line("ABCDEFGH", 1, 0b1), "ABCDEFGH A1");
        // A short name pads; a full 8-char name does not.
        assert_eq!(slot_menu_line("AB", 1, 0), "AB       A");
    }

    #[test]
    fn empty_row_is_the_exw_empty_line() {
        assert_eq!(EMPTY_SLOT_LINE, "EMPTY");
        assert_eq!(SaveSlotRow::Empty.line(), "EMPTY");
    }

    #[test]
    fn summarize_walks_all_five_rows() {
        // One used slot (slot 1) among five empties: the exact list
        // the original's save/load screens build.
        let mut d = image_with_slot(1, 3, 0b0110);
        let b = 180;
        d[b + 0x0E..b + 0x12].copy_from_slice(&0xA40B_u32.to_le_bytes());
        d[b + 0x12..b + 0x16].copy_from_slice(&580_u32.to_le_bytes());
        d[b + 0x16..b + 0x18].copy_from_slice(&1_i16.to_le_bytes());
        let rows = summarize_saved_bdl(&d).unwrap();
        assert_eq!(rows.len(), SAVED_SLOTS);
        assert_eq!(rows[0], SaveSlotRow::Empty);
        let SaveSlotRow::Used(meta) = &rows[1] else {
            panic!("slot 1 is used");
        };
        assert_eq!(meta.slot.index(), 1);
        assert_eq!(meta.name, "SLOTS...");
        assert_eq!(meta.level_text, " C23");
        assert_eq!(meta.score, 0xA40B);
        assert_eq!(meta.money, 580);
        assert_eq!(meta.difficulty, 1);
        assert_eq!(meta.menu_line(), "SLOTS... C23");
        assert_eq!(rows[2], SaveSlotRow::Empty);
        assert_eq!(rows[3], SaveSlotRow::Empty);
        assert_eq!(rows[4], SaveSlotRow::Empty);
        assert_eq!(rows[4].line(), "EMPTY");
    }

    #[test]
    fn summarize_maps_every_header_field() {
        // The metadata surface is the whole §7j.70 header: name,
        // stage/mask (as the level text), score, money, difficulty.
        let mut d = image_with_slot(3, 5, 0b10101);
        let b = 3 * 180;
        d[b + 0x0E..b + 0x12].copy_from_slice(&123456_u32.to_le_bytes());
        d[b + 0x12..b + 0x16].copy_from_slice(&999_u32.to_le_bytes());
        d[b + 0x16..b + 0x18].copy_from_slice(&(-2_i16).to_le_bytes());
        let rows = summarize_saved_bdl(&d).unwrap();
        let SaveSlotRow::Used(meta) = &rows[3] else {
            panic!("slot 3 is used");
        };
        assert_eq!(meta.level_text, " E135");
        assert_eq!(meta.score, 123456);
        assert_eq!(meta.money, 999);
        assert_eq!(meta.difficulty, -2);
    }

    #[test]
    fn summarize_rejects_broken_images_loud() {
        // Every rejection stays the engine import's loud GameError —
        // never a guess, never a silent row.
        assert!(summarize_saved_bdl(&[]).is_err());
        assert!(summarize_saved_bdl(&vec![0u8; 899]).is_err());
        assert!(summarize_saved_bdl(&vec![0u8; 901]).is_err());
        // Out-of-model campaign state: loud, not a row.
        assert!(summarize_saved_bdl(&image_with_slot(2, 9, 0)).is_err());
    }

    #[test]
    fn autosave_is_never_the_default() {
        // RE-EXW-SAVE sec 4: the shipped game never autosaves — Off
        // is the default at every layer.
        assert_eq!(AutosavePolicy::default(), AutosavePolicy::Off);
        assert!(!AutosavePolicy::default().is_on());
        assert_eq!(AutosavePolicy::default().slot(), None);
        let dir = std::path::PathBuf::from("gfx");
        let opts = crate::window::WindowOptions::new(&dir);
        assert!(window_save_surface_is_shipped(&opts));
        assert_eq!(opts.autosave, AutosavePolicy::Off);
        assert_eq!(opts.save_slot, SaveSlotId::FIRST);
    }

    #[test]
    fn autosave_gates_mirror_the_original_save_screen() {
        // RE-EXW-SAVE sec 3: the original's save opportunity is the
        // campaign-shell SAVE button — single-player ONLY
        // (0x4edb88 == 0), between missions ONLY. Off never saves.
        let off = AutosavePolicy::Off;
        assert!(!off.should_autosave(true, true));
        assert!(!off.should_autosave(true, false));
        assert!(!off.should_autosave(false, true));
        let on = AutosavePolicy::On(SaveSlotId::LAST);
        assert!(on.is_on());
        assert_eq!(on.slot(), Some(SaveSlotId::LAST));
        // The gate: both conditions, exactly the original's own.
        assert!(on.should_autosave(true, true));
        assert!(!on.should_autosave(true, false)); // mid-mission: never
        assert!(!on.should_autosave(false, true)); // coop/h2h: never
        assert!(!on.should_autosave(false, false));
    }

    #[test]
    fn metadata_from_import_over_a_full_domain_sweep() {
        // Every stage/mask pair the engine accepts presents cleanly
        // (the level text is total over the modeled space).
        use bedlam_game::{Episode, MAX_STAGE, SELECT_FULL_MASK};
        for stage in 1..=MAX_STAGE {
            for mask in 0..=SELECT_FULL_MASK[stage as usize] {
                let d = image_with_slot(0, stage as u16, mask as u32);
                let import = import_saved_slot(&d, 0).unwrap();
                let mut episode = Episode::boot();
                assert!(bedlam_game::stage_imported_episode(&mut episode, &import));
                let meta = SaveSlotMetadata::from_import(&import);
                assert_eq!(meta.level_text, save_level_text(stage, mask));
                // The sanitized name is already 8 chars ("SLOTS..."):
                // no padding, the level text appends directly.
                let mut expect = String::from("SLOTS...");
                expect.push_str(&save_level_text(stage, mask));
                assert_eq!(meta.menu_line(), expect);
            }
        }
    }
}
