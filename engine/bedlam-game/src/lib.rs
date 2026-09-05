//! bedlam-game: scene FSM + host pump - the composition layer of the
//! Bedlam reimplementation (P3 skeleton; docs/DESIGN-GAME.md is the
//! design note this implements, mirroring the DESIGN-RENDER /
//! DESIGN-AUDIO flow). The LAST P3 charter crate.
//!
//! Contract (PLAN P3 + DESIGN-GAME secs 1/8):
//! - NO per-mission game logic: mission quirks are data (a P5
//!   hypothesis). This crate owns the scene machine, the typed config
//!   view and the per-frame pump wiring core + render + audio + assets.
//! - Hermetic like the siblings: no fs / clock / threads; every byte
//!   crossing the boundary passes through the injected ByteSource /
//!   ByteSink traits, so the FSM stays pure and replays from tests
//!   unchanged.
//! - Determinism boundary (D17 + D26): scene state is HASHED on the
//!   same fixed 60 Hz tick grid as the sim; the host accumulator, frame
//!   state, mixer and rendered frames stay in the per-frame bucket (b),
//!   unhashed.
//! - thiserror only; no new dependencies.

#![forbid(unsafe_code)]

pub mod armoury;
pub mod boot;
pub mod brief;
pub mod config;
mod font;
pub mod fsm;
pub mod host;
pub mod loading;
pub mod menu;
pub mod mission;
pub mod mission_room;
pub mod movie;
pub mod movies;
pub mod music;
pub mod save;

pub use boot::{BootAttract, BootPhase};
pub use brief::{BriefIntro, BriefPhase};
pub use config::{GameConfig, OPTIONS_LEN, OPTIONS_NAME, VOLUME_MAX};
pub use fsm::{
    Episode, Scene, SceneAction, SceneFsm, SelectSlot, BOOT_TICKS, FULL_MASK, MAX_LINEAR,
    MAX_STAGE, SELECT_FULL_MASK,
};
pub use host::{ByteSink, ByteSource, GameHost};
pub use loading::{LoadingPhase, TextRow};
pub use mission::{
    mission_asset_names, mission_number_for_mask, robots_per_player, zone_for_stage, MissionScene,
    SELECT_MP_FILE_OFFSET,
};
pub use movie::MoviePlayer;
pub use movies::{
    boot_pair, briefing_name, briefing_name_for_slot, cutscene_name, gameover_name, gtlog_name,
    logo_name, shop_name, title_name, Region, BRIEFING_DROP_NAME,
};
pub use music::{build_script, track_name, MusicPump, ScriptMeta, ScriptTerminal};
pub use save::{
    import_saved_slot, stage_imported_episode, SaveSlotImport, SAVED_LEN, SAVED_NAME, SAVED_SLOTS,
};

/// Serialization tag of the hashed scene-state view (DESIGN-GAME sec 7).
pub const SCENE_HASH_TAG: &[u8; 4] = b"BDLG";

/// Errors for the composition crate. thiserror only; host misuse
/// returns Err, never panics (panic = engine bug, PLAN P3).
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    /// Asset parse failure (propagated from bedlam-assets).
    #[error(transparent)]
    Assets(#[from] bedlam_assets::AssetsError),
    /// Audio failure (propagated from bedlam-audio).
    #[error(transparent)]
    Audio(#[from] bedlam_audio::AudioError),
    /// The requested music chunk is disabled or out of range.
    #[error("music chunk {chunk} is disabled or out of range")]
    BadMusicChunk { chunk: usize },
    /// OPTIONS.BDL volume outside 0..=100.
    #[error("options volume {value} out of range 0..=100")]
    InvalidVolume { value: u32 },
    /// A byte source could not produce the named asset.
    #[error("asset {name} missing from the byte source")]
    AssetMissing { name: String },
    /// A loading-flow asset decoded structurally but its entry-0
    /// raster is missing or the wrong size (host staging rejects it).
    #[error("loading-flow asset {what}: {reason}")]
    BadLoadingAsset {
        what: &'static str,
        reason: &'static str,
    },
    /// A title-menu asset decoded structurally but the menu cannot
    /// be built from it (short [MENU_ITEMS] table, undecodable font
    /// base, bad ramp).
    #[error("title-menu asset {what}: {reason}")]
    BadMenuAsset {
        what: &'static str,
        reason: &'static str,
    },
    #[error("mission-room asset {what}: {reason}")]
    BadMissionRoomAsset {
        what: &'static str,
        reason: &'static str,
    },
    /// A mission asset decoded structurally but the scene cannot be
    /// staged from it (malformed terrain/viewport bytes, short MRK or
    /// SINTABLE) — DESIGN-GAME sec 11 staging.
    #[error("mission asset {what}: {reason}")]
    BadMissionAsset {
        what: &'static str,
        reason: &'static str,
    },
    /// Original SAVED.BDL slot index outside 0..5 (the staging buffer
    /// holds exactly five slots, RE-EXW-SIM §7j.70).
    #[error("save slot index {slot} out of range 0..5")]
    SaveSlotIndex { slot: usize },
    /// Original SAVED.BDL slot is empty — the EXW restore's zero
    /// dword@+0x0C predicate (the 0x43c558 exit arm, §7j.70).
    #[error("save slot {slot} is empty (dword@+0x0C == 0)")]
    SaveSlotEmpty { slot: usize },
    /// Original SAVED.BDL slot carries a campaign state outside the
    /// modeled episode space (zone signed word or mask dword; §7j.70).
    #[error("save slot {slot} campaign state zone {zone}/mask {mask:#x} is outside the modeled episode space")]
    SaveSlotInvalid { slot: usize, zone: i32, mask: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_displays_are_stable() {
        assert_eq!(
            GameError::BadMusicChunk { chunk: 7 }.to_string(),
            "music chunk 7 is disabled or out of range"
        );
        assert_eq!(
            GameError::InvalidVolume { value: 250 }.to_string(),
            "options volume 250 out of range 0..=100"
        );
        assert_eq!(
            GameError::AssetMissing {
                name: "OPTIONS.BDL".to_string()
            }
            .to_string(),
            "asset OPTIONS.BDL missing from the byte source"
        );
    }

    #[test]
    fn scene_tag_is_pinned() {
        assert_eq!(SCENE_HASH_TAG, b"BDLG");
    }
}
