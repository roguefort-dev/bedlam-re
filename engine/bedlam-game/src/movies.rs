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
    pub fn loading_bin(self) -> &'static str {
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

/// The briefing backdrop for the hashed episode slot (D33): stage ->
/// zone letter, lowest-unset mask bit + 1 -> sub (the SAME sub
/// arithmetic `Episode::complete` applies [design, DESIGN-GAME open
/// Q5]). Letter map [design, anchored both ways]: stage 1 is the
/// BootCamp intro and stages 7..=8 the endgame zone / post-endgame
/// ceiling (EXW zone counter 1..7, 7 = endgame, RE-EXW-GAMETHREAD
/// fact table) - neither has a lettered backdrop in the corpus (no
/// BRF_A / BRF_G files exist), so they select None = no briefing
/// movie; stages 2..=6 map onto EXW zones 2..=6 = letters B..=F,
/// exactly the 25-file BRF_{B..F}{1..5} corpus domain (the linear
/// formula clamp((zone-2)*5 + level - 1, 1, 26) walks zones 2..6 =
/// the 25 lettered levels). The B2 4-sub FULL_MASK cadence selects
/// BRF_*{1..4} only; BRF_*5 stays corpus-resident but
/// cadence-unreachable (the EXW 5-level cadence files, like B2's
/// mostly-absent MISSION5, census sec 1); a transitional full mask
/// (0b1111) still lands inside the corpus domain (sub 5), and masks
/// with bit 4+ set are not playable subs (briefing_name rejects
/// them -> None).
pub fn briefing_name_for_slot(stage: u8, mask: u8) -> Option<String> {
    if !(2..=6).contains(&stage) {
        return None;
    }
    let letter = char::from(b'B' + (stage - 2));
    // Lowest unset bit (the complete() arithmetic); the < 8 guard
    // keeps a saturated mask (u8 all-bits) from shifting past the
    // width - it lands on sub 9, which briefing_name rejects.
    let mut sub = 0u8;
    while sub < 8 && mask >> sub & 1 != 0 {
        sub += 1;
    }
    briefing_name(letter, sub + 1)
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

    #[test]
    fn briefing_slot_map_letters_the_campaign_and_skips_the_doms() {
        // Boot camp (stage 1), the endgame zone (7) and the
        // post-endgame ceiling (8) select no backdrop - no BRF_A /
        // BRF_G exists in the corpus. Fresh lettered stages 2..=6
        // select B..=F sub 1.
        for stage in [0u8, 1, 7, 8, 9, u8::MAX] {
            assert_eq!(briefing_name_for_slot(stage, 0), None, "stage {stage}");
        }
        for (i, letter) in ['B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            let stage = i as u8 + 2;
            assert_eq!(
                briefing_name_for_slot(stage, 0).as_deref(),
                Some(format!("BRF_{letter}1.SMK").as_str()),
                "stage {stage}"
            );
        }
    }

    #[test]
    fn briefing_slot_sub_follows_the_mask_bits() {
        // Observable FULL_MASK cadence at one zone stage: masks
        // 0, 1, 3, 7 select subs 1..=4 (lowest-unset bit + 1 - the
        // Episode::complete arithmetic). A transitional full mask
        // still lands in the 1..=5 corpus domain; bit 4+ set is not
        // a playable sub (briefing_name rejects -> None).
        let expect = |sub: u8| Some(format!("BRF_C{sub}.SMK"));
        assert_eq!(briefing_name_for_slot(3, 0b000), expect(1));
        assert_eq!(briefing_name_for_slot(3, 0b001), expect(2));
        assert_eq!(briefing_name_for_slot(3, 0b011), expect(3));
        assert_eq!(briefing_name_for_slot(3, 0b111), expect(4));
        assert_eq!(briefing_name_for_slot(3, 0b1111), expect(5));
        assert_eq!(briefing_name_for_slot(3, 0b1_1111), None);
    }

    #[test]
    fn briefing_slot_map_stays_inside_the_corpus_domain() {
        // Every Some over the whole slot domain is one of the 25
        // corpus BRF names (cross-check against the domain, not a
        // recomputation of the map itself).
        let corpus: Vec<String> = ('B'..='F')
            .flat_map(|z| (1..=5u8).map(move |s| format!("BRF_{z}{s}.SMK")))
            .collect();
        for stage in 0..=10u8 {
            for mask in [0u8, 1, 2, 3, 7, 15, 31, 255] {
                if let Some(name) = briefing_name_for_slot(stage, mask) {
                    assert!(
                        corpus.contains(&name),
                        "stage {stage} mask {mask}: {name} not in the corpus domain"
                    );
                }
            }
        }
    }
}
