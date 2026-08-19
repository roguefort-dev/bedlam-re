//! Movie selection (P5, D32): which .SMK plays where, and which
//! region-variant file backs it. Pure name arithmetic over hashed state
//! (the episode stage) and host config (the region flag) - selection is
//! PRESENTATION-side (D17 bucket b): it never feeds back into the hash,
//! and the host stays byte-source-free (DESIGN-GAME sec 8: callers load
//! the named bytes through their injected ByteSource and hand them to
//! [`crate::host::GameHost::load_cutscene`] / `load_movie`).
//!
//! Provenance anchors (docs/RE-EXW-GAMETHREAD.md, GameMain zone-complete
//! block LAB_0041c69e): the zone-complete handler plays
//! FUN_0044567c("GAMEGFX\\ZONEDONE.SMK", 0) on every zone completion,
//! except the endgame arm (`_DAT_004edd8c == 7`) which plays
//! FUN_0044567c("GAMEGFX\\END.SMK", 0); after the movie it loads
//! BETWEEN.BIN then the region-variant loading screen
//! FUN_0041cc7f("GAMEGFX\\LOAD_UK.BIN"/"LOAD_US.BIN" per the language
//! flag DAT_0046ae64) with LOADPAL.PAL/LOADPALU.PAL [verified].

use crate::fsm::MAX_STAGE;

/// The speech-region flag - the reimplementation of EXW DAT_0046ae64,
/// the dword that selects LOAD_UK.BIN vs LOAD_US.BIN (and the
/// region-variant movies below). UK/US is a SHIPPING variant of the same
/// corpus, not a runtime option the original exposes mid-game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Uk,
    Us,
}

impl Region {
    /// Loading-screen image bank [verified: FUN_0041cc7f call site,
    /// LAB_0041c69e].
    pub fn loading_bin(self) -> &"""'static str {
        match self {
            Region::Uk => "LOAD_UK.BIN",
            Region::Us => "LOAD_US.BIN",
        }
    }

    /// Loading-screen palette [verified: same call site].
    pub fn loading_pal(self) -> &'static str {
        match self {
            Region::Uk => "LOADPAL.PAL",
            Region::Us => "LOADPALU.PAL",
        }
    }
}

/// Title attract movie (D31 wired; 640x320 letterbox, non-ring).
pub fn title_name() -> &'static str {
    "TITLE.SMK"
}

/// Publisher logo movie, region-variant [corpus: LOGO_UK/US.SMK exist as
/// the only variant pair; the EXW playback site is not yet RE'd].
pub fn logo_name(region: Region) -> &'static str {
    match region {
        Region::Uk => "LOGO_UK.SMK",
        Region::Us => "LOGO_US.SMK",
    }
}

/// Gremlin-style studio card, region-variant [corpus: GTLOG_UK/US.SMK;
/// playback site not yet RE'd].
pub fn gtlog_name(region: Region) -> &'static str {
    match region {
        Region::Uk => "GTLOG_UK.SMK",
        Region::Us => "GTLOG_US.SMK",
    }
}

/// Mission-failed movie [corpus: GAMEOVER.SMK; the EXW fail arm is the
/// outcome switch around FUN_0044771c, not yet RE'd to the movie call].
pub fn gameover_name() -> &'static str {
    "GAMEOVER.SMK"
}

/// Shop backdrop movie [corpus: SHOP.SMK, 61-frame ring].
pub fn shop_name() -> &'static str {
    "SHOP.SMK"
}

/// The zone-complete cutscene movie for a Cutscene scene entered with
/// the episode at `stage` [verified vs EXW LAB_0041c69e]: EXW reads the
/// zone counter BEFORE its post-movie increment (`_DAT_004edd8c == 7`
/// endgame), while this FSM advances the stage inside
/// `Episode::complete()` - i.e. ALREADY - so the just-completed slot is
/// `stage - 1` and the endgame arm (`completed == 7`) is exactly
/// `stage >= MAX_STAGE`. A completion at the capped ceiling (slot 8,
/// unreachable in EXW because the endgame ends the game) re-picks END
/// [design].
pub fn cutscene_name(stage: u8) -> &'static str {
    if stage >= MAX_STAGE {
        "END.SMK"
    } else {
        "ZONEDONE.SMK"
    }
}

/// Briefing backdrop movie for zone letter + sub [corpus: BRF_B1..F5
/// exist, 512-frame rings; B2 census ties the BRF_* backdrops to the
/// mission-select screens. The zone-number-to-letter map is not yet
/// RE'd, so the letter is taken verbatim].
pub fn briefing_name(zone_letter: char, sub: u8) -> Option<String> {
    let z = zone_letter.to_ascii_uppercase();
    if !matches!(z, 'B'..='F') || !(1..=5).contains(&sub) {
        return None;
    }
    Some(format!("BRF_{z}{sub}.SMK"))
}

/// Drop-ship briefing interlude [corpus: BRF_DROP.SMK, 30-frame
/// non-ring].
pub const BRIEFING_DROP_NAME: &str = "BRF_DROP.SMK";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_backs_the_loading_bin_pair() {
        assert_eq!(Region::Uk.loading_bin(), "LOAD_UK.BIN");
        assert_eq!(Region::Us.loading_bin(), "LOAD_US.BIN");
        assert_eq!(Region::Uk.loading_pal(), "LOADPAL.PAL");
        assert_eq!(Region::Us.loading_pal(), "LOADPALU.PAL");
    }

    #[test]
    fn region_varies_only_the_variant_movies() {
        assert_eq!(logo_name(Region::Uk), "LOGO_UK.SMK");
        assert_eq!(logo_name(Region::Us), "LOGO_US.SMK");
        assert_eq!(gtlog_name(Region::Uk), "GTLOG_UK.SMK");
        assert_eq!(gtlog_name(Region::Us), "GTLOG_US.SMK");
        assert_eq!(title_name(), "TITLE.SMK");
        assert_eq!(gameover_name(), "GAMEOVER.SMK");
        assert_eq!(shop_name(), "SHOP.SMK");
    }

    #[test]
    fn cutscene_picks_zonedone_until_the_stage_ceiling() {
        // Stages 1..=7 (slots 2..=7 completed) -> ZONEDONE.SMK;
        // stage 8 = the endgame completion (slot 7 = EXW zone 7) -> END.
        for stage in 1..=7u8 {
            assert_eq!(cutscene_name(stage), "ZONEDONE.SMK", "stage {stage}");
        }
        assert_eq!(cutscene_name(8), "END.SMK");
        assert_eq!(cutscene_name(u8::MAX), "END.SMK", "cap repeats END");
    }

    #[test]
    fn cutscene_boundary_matches_the_episode_cap() {
        // The selection flips exactly at MAX_STAGE (fsm::Episode caps
        // there), so the endgame arm is reachable and nothing beyond it
        // exists.
        assert_eq!(cutscene_name(MAX_STAGE), "END.SMK");
        assert_eq!(cutscene_name(MAX_STAGE - 1), "ZONEDONE.SMK");
    }

    #[test]
    fn briefing_names_cover_the_corpus_and_reject_the_rest() {
        for z in 'B'..='F' {
            for sub in 1..=5u8 {
                assert_eq!(
                    briefing_name(z, sub).as_deref(),
                    Some(format!("BRF_{z}{sub}.SMK").as_str())
                );
            }
        }
        assert_eq!(briefing_name('b', 3).as_deref(), Some("BRF_B3.SMK"));
        assert_eq!(briefing_name('A', 1), None);
        assert_eq!(briefing_name('G', 1), None);
        assert_eq!(briefing_name('C', 0), None);
        assert_eq!(briefing_name('C', 6), None);
        assert_eq!(BRIEFING_DROP_NAME, "BRF_DROP.SMK");
    }
}
