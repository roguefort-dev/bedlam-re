//! Typed OPTIONS.BDL view (DESIGN-GAME sec 6). OPTIONS.BDL is the ONLY
//! config file the engine reads: SETUP-owned (written by SETUP.EXE,
//! read at boot, RE-EXW-MUSIC sec 4 + census sec 6). CONFIG.BDL is an
//! installer SB record never read by EXW and is deliberately NOT
//! modelled anywhere in this workspace.

use bedlam_assets::bdl::parse_options_bdl;

use crate::host::{ByteSink, ByteSource};
use crate::GameError;

/// Asset name of the options file (the SETUP-owned name).
pub const OPTIONS_NAME: &str = "OPTIONS.BDL";

/// Volume domain: the EXW UI writes 0..100 (FUN_0044c630).
pub const VOLUME_MAX: u32 = 100;

/// Options record size: 9 dwords + 8 name bytes + 1 drive byte.
pub const OPTIONS_LEN: usize = 41;

/// Typed, domain-validated configuration view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConfig {
    /// Player name: exactly 8 sanitized graphic chars (assets bdl.rs).
    pub player_name: String,
    /// Music/UI volume 0..=100 (validated on parse).
    pub volume: u32,
    /// Raw language code (B2 LANGUAGE.* select; value space is
    /// DESIGN-GAME open question Q3 - kept raw until the P2g UI RE).
    pub language: u32,
    /// Flag fields: nonzero original dword -> true. Exact value
    /// semantics are DESIGN-GAME open question Q1.
    pub backbuffer: bool,
    pub actionpan: bool,
    pub cd_audio: bool,
    pub midi: bool,
    pub sound: bool,
    pub code_no_title: bool,
    /// Installer drive letter byte.
    pub installdrive: u8,
}

impl Default for GameConfig {
    /// Neutral defaults [design]: zero flags, English code 0, dotted
    /// name, volume max (a fresh SETUP install; real default unknown
    /// until Q1).
    fn default() -> Self {
        GameConfig {
            player_name: "........".to_string(),
            volume: VOLUME_MAX,
            language: 0,
            backbuffer: false,
            actionpan: false,
            cd_audio: false,
            midi: false,
            sound: false,
            code_no_title: false,
            installdrive: b'C',
        }
    }
}

impl GameConfig {
    /// Validate and type a parsed record.
    pub fn from_options(o: &bedlam_assets::bdl::OptionsBdl) -> Result<GameConfig, GameError> {
        if o.volume > VOLUME_MAX {
            return Err(GameError::InvalidVolume { value: o.volume });
        }
        Ok(GameConfig {
            player_name: o.playername.clone(),
            volume: o.volume,
            language: o.language,
            backbuffer: o.backbuffer != 0,
            actionpan: o.actionpan != 0,
            cd_audio: o.cd_audio != 0,
            midi: o.midi != 0,
            sound: o.sound != 0,
            code_no_title: o.code_no_title != 0,
            installdrive: o.installdrive,
        })
    }

    /// Parse + validate raw OPTIONS.BDL bytes.
    pub fn from_bytes(data: &[u8]) -> Result<GameConfig, GameError> {
        Self::from_options(&parse_options_bdl(data)?)
    }

    /// Canonical 41-byte serialization (byte-exact when the name holds
    /// graphic chars only - the sanitizer maps everything else away).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = vec![0u8; OPTIONS_LEN];
        v[0..4].copy_from_slice(&u32::from(self.backbuffer).to_le_bytes());
        v[4..8].copy_from_slice(&u32::from(self.actionpan).to_le_bytes());
        v[8..12].copy_from_slice(&self.language.to_le_bytes());
        v[12..16].copy_from_slice(&u32::from(self.cd_audio).to_le_bytes());
        let mut name = [b'.'; 8];
        for (i, b) in self.player_name.as_bytes().iter().take(8).enumerate() {
            name[i] = *b;
        }
        v[16..24].copy_from_slice(&name);
        v[24..28].copy_from_slice(&self.volume.to_le_bytes());
        v[28..32].copy_from_slice(&u32::from(self.code_no_title).to_le_bytes());
        v[32..36].copy_from_slice(&u32::from(self.midi).to_le_bytes());
        v[36..40].copy_from_slice(&u32::from(self.sound).to_le_bytes());
        v[40] = self.installdrive;
        v
    }

    /// Music master volume: the EXW UI 0..100 -> >>1 -> 0..50 mapping
    /// (FUN_0044c630; DESIGN-AUDIO fact 3 domain).
    pub fn music_master(&self) -> u8 {
        (self.volume >> 1) as u8
    }

    /// Load OPTIONS.BDL through an injected byte source.
    pub fn load<S: ByteSource>(source: &mut S) -> Result<GameConfig, GameError> {
        let bytes = source.load(OPTIONS_NAME)?;
        GameConfig::from_bytes(&bytes)
    }

    /// Store through an injected byte sink (the SETUP role).
    pub fn store<S: ByteSink>(&self, sink: &mut S) -> Result<(), GameError> {
        sink.store(OPTIONS_NAME, &self.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory byte source/sink pair for the trait paths.
    struct Mem {
        files: Vec<(String, Vec<u8>)>,
    }

    impl ByteSource for Mem {
        fn load(&mut self, name: &str) -> Result<Vec<u8>, GameError> {
            self.files
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b.clone())
                .ok_or(GameError::AssetMissing {
                    name: name.to_string(),
                })
        }
    }

    impl ByteSink for Mem {
        fn store(&mut self, name: &str, bytes: &[u8]) -> Result<(), GameError> {
            match self.files.iter_mut().find(|(n, _)| n == name) {
                Some((_, b)) => *b = bytes.to_vec(),
                None => self.files.push((name.to_string(), bytes.to_vec())),
            }
            Ok(())
        }
    }

    fn synth(volume: u32) -> Vec<u8> {
        let mut v = vec![0u8; OPTIONS_LEN];
        v[8..12].copy_from_slice(&2u32.to_le_bytes()); // language
        v[16..24].copy_from_slice(b"KATO....");
        v[24..28].copy_from_slice(&volume.to_le_bytes());
        v[36..40].copy_from_slice(&1u32.to_le_bytes()); // sound on
        v[40] = b'D';
        v
    }

    #[test]
    fn parse_types_and_validates() {
        let cfg = GameConfig::from_bytes(&synth(80)).unwrap();
        assert_eq!(cfg.player_name, "KATO....", "zeros sanitize to dots");
        assert_eq!(cfg.volume, 80);
        assert_eq!(cfg.language, 2);
        assert!(cfg.sound);
        assert!(!cfg.midi);
        assert_eq!(cfg.installdrive, b'D');
        assert_eq!(cfg.music_master(), 40, "UI >>1 master mapping");
        let err = GameConfig::from_bytes(&synth(101)).unwrap_err();
        assert!(matches!(err, GameError::InvalidVolume { value: 101 }));
        let short = GameConfig::from_bytes(&synth(80)[..10]);
        assert!(short.is_err(), "truncated options must fail typed");
    }

    #[test]
    fn volume_master_edges() {
        assert_eq!(
            GameConfig::from_bytes(&synth(VOLUME_MAX))
                .unwrap()
                .music_master(),
            50
        );
        assert_eq!(
            GameConfig::from_bytes(&synth(99)).unwrap().music_master(),
            49
        );
        assert_eq!(GameConfig::from_bytes(&synth(0)).unwrap().music_master(), 0);
    }

    #[test]
    fn byte_round_trip_is_logical() {
        let cfg = GameConfig::from_bytes(&synth(80)).unwrap();
        let bytes = cfg.to_bytes();
        assert_eq!(bytes.len(), OPTIONS_LEN);
        assert_eq!(GameConfig::from_bytes(&bytes).unwrap(), cfg);
        // The synthesized name is graphic-only, so bytes are exact.
        assert_eq!(bytes[16..24].to_vec(), b"KATO....".to_vec());
        assert_eq!(bytes, synth(80));
    }

    #[test]
    fn load_and_store_cross_the_traits() {
        let mut mem = Mem {
            files: vec![("OPTIONS.BDL".to_string(), synth(60))],
        };
        let cfg = GameConfig::load(&mut mem).unwrap();
        assert_eq!(cfg.volume, 60);
        let mut sink = Mem { files: Vec::new() };
        cfg.store(&mut sink).unwrap();
        assert_eq!(sink.files.len(), 1);
        assert_eq!(sink.files[0].1, cfg.to_bytes());
        let missing = GameConfig::load(&mut Mem { files: Vec::new() });
        assert!(matches!(
            missing,
            Err(GameError::AssetMissing { name }) if name == "OPTIONS.BDL"
        ));
    }
}
