//! Original SAVED.BDL import seam (PLAN §6 P5 save-compatibility
//! criterion; RE-EXW-SIM §7j.70). The original's save-load restore arm
//! (slot dispatch 0x43c26e) is the EXW anchor for the whole grammar:
//! 5 x 180-B slots, name 8 B @+0x00, completed-missions bitmask dword
//! @+0x08, zone SIGNED word @+0x0C (-> the zone cell 0x4edd8c write
//! 0x43c2b8), hiscore/score dword @+0x0E, money dword @+0x12,
//! difficulty SIGNED word @+0x16, weapon rows from +0x18.
//!
//! This seam is IMPORT ONLY and READ-ONLY: the import never writes
//! anything (new saves use the new versioned format, PLAN §6 P5), is
//! bounds-checked at every step (exact file length, slot index, the
//! EXW empty-slot predicate, stage/mask domain — never guess), and
//! returns money/score/difficulty instead of staging them (they stay
//! sim-side, DESIGN-GAME sec 3; the §7j.64 cell census).
//!
//! The staging half goes through [`crate::fsm::SceneFsm::
//! stage_episode_slot`] — the D51 host seam that models exactly the
//! restore's zone-cell write + mask replay. Missions 6-7 (mask bits
//! past FULL_MASK) stay out of the modeled episode space until the
//! SELECT shell lands (the census G1 class, P5-ZONE-GATES §6.2); such
//! a slot is rejected loud, never guessed.

use bedlam_assets::bdl::parse_saved_bdl;

use crate::fsm::{Episode, FULL_MASK, MAX_STAGE};
use crate::GameError;

/// Asset name of the original save file.
pub const SAVED_NAME: &str = "SAVED.BDL";

/// SAVED.BDL is exactly 900 bytes (5 x 180; the 0x43c26e stride
/// arithmetic closes on the shipped file, §7j.70).
pub const SAVED_LEN: usize = 900;

/// Slot count (the 0x4eae58 staging buffer holds exactly five).
pub const SAVED_SLOTS: usize = 5;

/// One imported original save slot (the §7j.70 header fields only;
/// the weapon/chassis rows are loadout state the engine does not
/// model, §7j.67/E).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSlotImport {
    /// Slot index the import read (0-based, like the restore's edx).
    pub slot: usize,
    /// Sanitized 8-char slot name (assets bdl.rs rule).
    pub name: String,
    /// Campaign stage slot (the restore's zone cell value, 1-based:
    /// 1 = ZONEA .. 7 = ZONEG, 8 = the endgame arm).
    pub stage: u8,
    /// Completed-sub bitmask of that stage (dword @+0x08; the restore
    /// replays it bit-by-bit through the sub-marking calls).
    pub mask: u8,
    /// Hiscore/score dword @+0x0E (returned, never staged).
    pub score: u32,
    /// Money dword @+0x12 (returned, never staged).
    pub money: u32,
    /// Difficulty SIGNED word @+0x16 (returned, never staged).
    pub difficulty: i16,
}

/// The EXW empty-slot predicate: the restore tests the DWORD at
/// slot+0x0C (the zone word widened) against zero and takes the
/// 0x43c558 exit arm — an "EMPTY" slot is never restored.
fn slot_is_empty(raw: &[u8; 180]) -> bool {
    raw[0x0C] == 0 && raw[0x0D] == 0 && raw[0x0E] == 0 && raw[0x0F] == 0
}

/// Import one original SAVED.BDL slot, read-only and bounds-checked.
///
/// Every rejection is loud ([`GameError`]), never a guess: wrong file
/// size (the assets parser), slot index out of 0..5, the EXW empty
/// predicate, and a campaign state outside the modeled episode space
/// (stage not in 1..=8, mask not a sub-mask of `FULL_MASK[stage]` —
/// the missions-6/7 SELECT shape stays rejected until G1 lands).
pub fn import_saved_slot(data: &[u8], slot: usize) -> Result<SaveSlotImport, GameError> {
    if slot >= SAVED_SLOTS {
        return Err(GameError::SaveSlotIndex { slot });
    }
    let saved = parse_saved_bdl(data)?;
    let raw = &saved.slots[slot].raw;
    if slot_is_empty(raw) {
        return Err(GameError::SaveSlotEmpty { slot });
    }
    // The restore reads the zone word movsx (SIGNED, 0x43c2b3); a
    // negative or zero zone is outside the modeled space by construct
    // (the empty predicate already took the exact-zero dword).
    let zone = i32::from(i16::from_le_bytes([raw[0x0C], raw[0x0D]]));
    let mask_word = u32::from_le_bytes([raw[0x08], raw[0x09], raw[0x0A], raw[0x0B]]);
    if !(1..=i32::from(MAX_STAGE)).contains(&zone)
        || mask_word > u32::from(u8::MAX)
        || (mask_word as u8) & !FULL_MASK[zone as usize] != 0
    {
        return Err(GameError::SaveSlotInvalid {
            slot,
            zone,
            mask: mask_word,
        });
    }
    Ok(SaveSlotImport {
        slot,
        name: saved.slots[slot].name.clone(),
        stage: zone as u8,
        mask: mask_word as u8,
        score: u32::from_le_bytes([raw[0x0E], raw[0x0F], raw[0x10], raw[0x11]]),
        money: u32::from_le_bytes([raw[0x12], raw[0x13], raw[0x14], raw[0x15]]),
        difficulty: i16::from_le_bytes([raw[0x16], raw[0x17]]),
    })
}

/// Stage the imported campaign state onto a fresh episode, exactly
/// the modeling of the restore arm's effect (the zone cell write plus
/// the sub-marking replay): the stage/mask pair goes through the
/// D51 seam's own validation, so the accepted import space and the
/// staging space can never drift apart.
pub fn stage_imported_episode(episode: &mut Episode, import: &SaveSlotImport) -> bool {
    episode.stage_slot(import.stage, import.mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal well-formed 900-B image with one non-empty slot.
    fn image_with_slot(slot: usize, zone: u16, mask: u32) -> Vec<u8> {
        let mut d = vec![0u8; SAVED_LEN];
        let b = slot * 180;
        d[b..b + 5].copy_from_slice(b"SLOTS");
        d[b + 0x0C..b + 0x0E].copy_from_slice(&zone.to_le_bytes());
        d[b + 0x08..b + 0x0C].copy_from_slice(&mask.to_le_bytes());
        d
    }

    #[test]
    fn imports_header_fields() {
        let mut d = image_with_slot(2, 3, 0b0110);
        let b = 2 * 180;
        d[b + 0x0E..b + 0x12].copy_from_slice(&0xA40B_u32.to_le_bytes());
        d[b + 0x12..b + 0x16].copy_from_slice(&580_u32.to_le_bytes());
        d[b + 0x16..b + 0x18].copy_from_slice(&1_i16.to_le_bytes());
        let import = import_saved_slot(&d, 2).unwrap();
        assert_eq!(import.name, "SLOTS...");
        assert_eq!(import.stage, 3);
        assert_eq!(import.mask, 0b0110);
        assert_eq!(import.score, 0xA40B);
        assert_eq!(import.money, 580);
        assert_eq!(import.difficulty, 1);
        // Staging through the episode seam: stage 3 mask 0b0110 is a
        // valid sub-mask of FULL_MASK[3] = 15.
        let mut episode = Episode::boot();
        assert!(stage_imported_episode(&mut episode, &import));
        assert_eq!((episode.stage(), episode.mask()), (3, 0b0110));
    }

    #[test]
    fn empty_predicate_matches_exw() {
        // Zone dword zero -> empty, even with a nonzero name/money.
        let mut d = image_with_slot(1, 0, 0);
        let b = 180;
        d[b + 0x12..b + 0x16].copy_from_slice(&999_u32.to_le_bytes());
        assert!(matches!(
            import_saved_slot(&d, 1),
            Err(GameError::SaveSlotEmpty { slot: 1 })
        ));
    }

    #[test]
    fn rejects_out_of_model_states() {
        // Zone past MAX_STAGE, negative zone (movsx), mask past the
        // FULL_MASK sub-mask (the missions-6/7 SELECT shape), and the
        // slot index bound — all loud, none guessed.
        assert!(matches!(
            import_saved_slot(&image_with_slot(0, 9, 0), 0),
            Err(GameError::SaveSlotInvalid { .. })
        ));
        assert!(matches!(
            import_saved_slot(&image_with_slot(0, 0xFFFF, 0), 0),
            Err(GameError::SaveSlotInvalid { .. })
        ));
        assert!(matches!(
            import_saved_slot(&image_with_slot(0, 2, 0x10), 0),
            Err(GameError::SaveSlotInvalid { .. })
        ));
        assert!(matches!(
            import_saved_slot(&vec![0u8; SAVED_LEN], SAVED_SLOTS),
            Err(GameError::SaveSlotIndex { slot: 5 })
        ));
        // The size check rides the assets parser (900 exactly).
        assert!(import_saved_slot(&vec![0u8; SAVED_LEN - 1], 0).is_err());
        assert!(import_saved_slot(&vec![0u8; SAVED_LEN + 1], 0).is_err());
    }

    #[test]
    fn every_stage_one_mask_accepts() {
        // The full modeled space imports + stages cleanly (stage 1 is
        // the boot/intro slot with FULL_MASK 1).
        for stage in 1..=MAX_STAGE {
            for mask in 0..=FULL_MASK[stage as usize] {
                let d = image_with_slot(0, stage as u16, mask as u32);
                let import = import_saved_slot(&d, 0).unwrap();
                assert_eq!((import.stage, import.mask), (stage, mask));
                let mut episode = Episode::boot();
                assert!(stage_imported_episode(&mut episode, &import));
            }
        }
    }

    #[test]
    fn no_panic_on_fuzzed_images() {
        // Bounded deterministic fuzz over the 900-B shape: random
        // payloads, truncations at header-sensitive offsets and size
        // attacks. Ok/Err only, never a panic.
        let mut s = 0x5AED_u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(0x0BED);
            (s >> 33) as u8
        };
        for slot in 0..SAVED_SLOTS {
            let d: Vec<u8> = (0..SAVED_LEN).map(|_| next()).collect();
            let _ = import_saved_slot(&d, slot);
        }
        for len in [
            0usize,
            1,
            0x0C,
            0x0D,
            0x0E,
            0x10,
            0x11,
            0x16,
            0x18,
            179,
            180,
            181,
            SAVED_LEN - 1,
            SAVED_LEN,
            SAVED_LEN + 1,
        ] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = import_saved_slot(&d, 0);
            let _ = import_saved_slot(&d, SAVED_SLOTS - 1);
        }
        // Bit flips over the header window of a valid slot: any
        // mutation still yields Ok or Err, and a flip that keeps the
        // state in-model round-trips through staging.
        let base = image_with_slot(3, 2, 0b0001);
        for bit in 0..(0x18 * 8) {
            let mut d = base.clone();
            d[bit / 8] ^= 1 << (bit % 8);
            if let Ok(import) = import_saved_slot(&d, 3) {
                let mut episode = Episode::boot();
                let _ = stage_imported_episode(&mut episode, &import);
            }
        }
    }
}
