//! The D31-D37 asset wiring (P4 step 1).
//!
//! The scene FSM never loads anything itself - hosts fetch names
//! through their ByteSource and hand bytes to the staging calls
//! (DESIGN-GAME sec 8). This module is that fetch layer: given the
//! scene the host just ENTERED, which corpus files the landed D31-D37
//! sites need and in what order, plus the staging calls themselves.
//!
//! Provenance: the names all come from bedlam-game movies.rs (per-site
//! RE tags D31-D37); the boot pair order GTLOG-then-LOGO and the
//! briefing drop-then-backdrop order are EXW-verified there. Nothing
//! here re-derives RE facts - it composes the already-wired chain.
//!
//! Music (.MRS/.MRW) is deliberately NOT staged here: it rides the
//! D27 MusicPump and lands with platform audio output (shell step 2).

use bedlam_game::{ByteSource, GameError, GameHost, Region, Scene, BRIEFING_DROP_NAME};

/// The language file handed to the D35 loading-font pass. The EXW
/// language index (004eba1c) arithmetic is not yet RE-pinned, so the
/// shell defaults to English and treats the choice as configuration;
/// pinning the index selection is queued with the P5 language work.
pub const DEFAULT_LANGUAGE: &str = "LANGUAGE.ENG";

/// FullFONT bank + FULLPAL ramp file names (D35 staging seam).
const FULLFONT_NAME: &str = "FULLFONT.BIN";
const FULLPAL_NAME: &str = "FULLPAL.PAL";

/// What the shell wires, where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainConfig {
    pub region: Region,
    pub language: &'static str,
}

impl Default for ChainConfig {
    fn default() -> Self {
        ChainConfig {
            region: Region::Uk,
            language: DEFAULT_LANGUAGE,
        }
    }
}

/// Asset names the wired sites need for a scene ENTER transition,
/// in fetch order (order matters for the briefing pair: drop first).
/// OWNED strings: the host-selected cutscene / briefing / mission
/// names are runtime Strings, so the whole list is allocated.
///
/// Pure name arithmetic over host-selected names - unit-pinned. The
/// boot scene needs nothing per-transition: its pair is staged once
/// at construction ([`stage_boot`]) because the host boots there.
/// The Title fetch set is the D41 menu staging (D42.7): the movie,
/// the language table, the FULLFONT bank, the FULLPAL ramp and the
/// MENU1/MENU2 SFX pair. The Mission fetch set is the DESIGN-GAME
/// sec 11 staging (the host-selected ZONE/MISSION files + the
/// GAMEGFX family tail; `EDITOR` sub-paths resolved by the source).
pub fn scene_assets(
    scene: Scene,
    config: ChainConfig,
    cutscene: &str,
    briefing: Option<&str>,
    mission: &[String],
) -> Vec<String> {
    match scene {
        Scene::Title => {
            let mut v = vec![bedlam_game::movies::title_name().to_string()];
            v.push(config.language.to_string());
            v.push(FULLFONT_NAME.to_string());
            v.push(FULLPAL_NAME.to_string());
            v.extend(bedlam_game::menu::MENU_SFX_NAMES.map(String::from));
            v
        }
        Scene::Brief => match briefing {
            Some(backdrop) => vec![BRIEFING_DROP_NAME.to_string(), backdrop.to_string()],
            None => Vec::new(),
        },
        Scene::Cutscene => vec![
            cutscene.to_string(),
            bedlam_game::movies::interlude_name().to_string(),
            config.region.loading_bin().to_string(),
            config.region.loading_pal().to_string(),
            FULLFONT_NAME.to_string(),
            FULLPAL_NAME.to_string(),
            config.language.to_string(),
        ],
        Scene::Shop => vec![bedlam_game::movies::shop_name().to_string()],
        // The mission files as the host selected them (fetch order =
        // the load_mission path families; see
        // bedlam_game::mission::mission_asset_names).
        Scene::Mission => mission.to_vec(),
        // Boot: staged at construction; the other scenes own no
        // wired assets.
        Scene::Boot | Scene::Options | Scene::Select | Scene::Debrief | Scene::Quit => Vec::new(),
    }
}

/// Stage the boot attract pair (GTLOG then LOGO, D36) on a freshly
/// constructed host. Returns the fetched asset names in order.
pub fn stage_boot(
    host: &mut GameHost,
    source: &mut dyn ByteSource,
    config: ChainConfig,
) -> Result<Vec<String>, GameError> {
    let [gtlog, logo] = bedlam_game::boot_pair(config.region);
    let gtlog_bytes = source.load(gtlog)?;
    let logo_bytes = source.load(logo)?;
    host.load_boot_attract(&gtlog_bytes, &logo_bytes)?;
    Ok(vec![gtlog.to_string(), logo.to_string()])
}

/// Wire the scene the host just ENTERED: fetch the scene assets and
/// stage them. Returns the fetched names (empty when the scene owns
/// no D31-D37 assets - e.g. a boot-camp Brief has no lettered
/// backdrop in the corpus). The language file joins the Cutscene and
/// Title fetch sets (the D35 pass stages with the loading flow; the
/// D41 menu stages from it directly).
pub fn stage_scene(
    host: &mut GameHost,
    source: &mut dyn ByteSource,
    config: ChainConfig,
) -> Result<Vec<String>, GameError> {
    let scene = host.scene();
    let cutscene = host.cutscene_name().to_string();
    let briefing = host.briefing_name();
    let mission = host.mission_asset_names();
    let names = scene_assets(scene, config, &cutscene, briefing.as_deref(), &mission);
    if names.is_empty() {
        return Ok(names);
    }
    let bytes: Vec<Vec<u8>> = names
        .iter()
        .map(|n| source.load(n))
        .collect::<Result<_, _>>()?;
    match scene {
        Scene::Title => {
            host.load_movie(Scene::Title, &bytes[0])?;
            host.load_title_menu(&bytes[1], &bytes[2], &bytes[3], &bytes[4], &bytes[5])?;
        }
        Scene::Brief => host.load_briefing(&bytes[0], &bytes[1])?,
        Scene::Cutscene => {
            host.load_cutscene(&bytes[0])?;
            host.load_interlude(&bytes[1])?;
            host.load_loading_screen(&bytes[2], &bytes[3])?;
            host.load_loading_font(&bytes[4], &bytes[5], &bytes[6])?;
        }
        Scene::Shop => host.load_shop(&bytes[0])?,
        // Fetch order = load_mission order: TOT, DAT, PAD, CGR, BIN,
        // LNK, SINTABLE, DANTE, GAMEPAL, MRK. Single player: no
        // robots override, no staged markers (the 0x46cbe0 network
        // seam).
        Scene::Mission => host.load_mission(
            &bytes[0],
            &bytes[1],
            &bytes[2],
            &bytes[3],
            &bytes[4],
            &bytes[5],
            &bytes[6],
            &bytes[7],
            &bytes[8],
            &bytes[9],
            None,
            &[],
        )?,
        _ => unreachable!("scene_assets returned empty for this scene"),
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-scene fetch sets, exactly: the wired chain.
    #[test]
    fn scene_assets_pin_the_chain() {
        let uk = ChainConfig::default();
        let no_mission: Vec<String> = Vec::new();
        assert_eq!(
            scene_assets(Scene::Title, uk, "ZONEDONE.SMK", None, &no_mission),
            vec![
                "TITLE.SMK".to_string(),
                "LANGUAGE.ENG".to_string(),
                "FULLFONT.BIN".to_string(),
                "FULLPAL.PAL".to_string(),
                "MENU1.RAW".to_string(),
                "MENU2.RAW".to_string(),
            ]
        );
        // A lettered briefing stage: drop first, then the backdrop.
        assert_eq!(
            scene_assets(
                Scene::Brief,
                uk,
                "ZONEDONE.SMK",
                Some("BRF_B1.SMK"),
                &no_mission
            ),
            vec!["BRF_DROP.SMK".to_string(), "BRF_B1.SMK".to_string()]
        );
        // Boot camp: no lettered backdrop in the corpus, nothing to
        // fetch (the host stages no pair).
        assert!(scene_assets(Scene::Brief, uk, "ZONEDONE.SMK", None, &no_mission).is_empty());
        // The zone-transition chain: cutscene + interlude + region
        // loading screen + D35 font pass, in staging order.
        assert_eq!(
            scene_assets(Scene::Cutscene, uk, "ZONEDONE.SMK", None, &no_mission),
            vec![
                "ZONEDONE.SMK".to_string(),
                "BETWEEN.BIN".to_string(),
                "LOAD_UK.BIN".to_string(),
                "LOADPAL.PAL".to_string(),
                "FULLFONT.BIN".to_string(),
                "FULLPAL.PAL".to_string(),
                "LANGUAGE.ENG".to_string(),
            ]
        );
        let mut us = uk;
        us.region = Region::Us;
        assert_eq!(
            scene_assets(Scene::Cutscene, us, "END.SMK", None, &no_mission)[2..4],
            ["LOAD_US.BIN".to_string(), "LOADPALU.PAL".to_string()]
        );
        assert_eq!(
            scene_assets(Scene::Shop, uk, "ZONEDONE.SMK", None, &no_mission),
            vec!["SHOP.SMK".to_string()]
        );
        // The mission fetch set is EXACTLY the host's selection, in
        // the host's order (DESIGN-GAME sec 11 staging order, GAMEPAL
        // in the GAMEGFX tail before the markers).
        let mission: Vec<String> = bedlam_game::mission_asset_names(0, 1);
        assert_eq!(
            scene_assets(Scene::Mission, uk, "ZONEDONE.SMK", None, &mission),
            vec![
                "ZONEA/MISSION1.TOT".to_string(),
                "ZONEA/MISSION1.DAT".to_string(),
                "ZONEA/MISSION1.PAD".to_string(),
                "ZONEA/MISSIONA.CGR".to_string(),
                "ZONEA/MISSIONA.BIN".to_string(),
                "ZONEA/MISSIONA.LNK".to_string(),
                "SINTABLE.BIN".to_string(),
                "DANTE.BIN".to_string(),
                "GAMEPAL.PAL".to_string(),
                "ZONEA/MISSION1.MRK".to_string(),
            ]
        );
        for scene in [
            Scene::Boot,
            Scene::Options,
            Scene::Select,
            Scene::Debrief,
            Scene::Quit,
        ] {
            assert!(
                scene_assets(scene, uk, "ZONEDONE.SMK", None, &no_mission).is_empty(),
                "{scene:?}"
            );
        }
    }

    /// A fresh host selects the boot-camp ZONEA/MISSION1 set: the
    /// mission names come from the episode slot (stage 1, mask 0),
    /// and the boot-scene transition itself still fetches nothing.
    #[test]
    fn fresh_host_mission_names_are_zonea_mission1() {
        let cfg = ChainConfig::default();
        let host = bedlam_game::host::GameHost::new(
            &bedlam_game::config::GameConfig::default(),
            &bedlam_core::sim::SimConfig::default(),
            [[0u8, 0, 0]; 256],
        );
        let cutscene = host.cutscene_name().to_string();
        let briefing = host.briefing_name();
        let mission = host.mission_asset_names();
        assert_eq!(
            mission.first().map(String::as_str),
            Some("ZONEA/MISSION1.TOT")
        );
        assert_eq!(mission.len(), 10);
        assert!(
            scene_assets(host.scene(), cfg, &cutscene, briefing.as_deref(), &mission).is_empty(),
            "boot transition fetches nothing"
        );
        assert_eq!(cfg.language, "LANGUAGE.ENG");
        assert_eq!(DEFAULT_LANGUAGE, "LANGUAGE.ENG");
    }
}
