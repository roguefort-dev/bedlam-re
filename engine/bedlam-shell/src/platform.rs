//! The P7 platform profile (PLAN §6 P7 "SteamDeck defaults stretch";
//! docs/P7-PORTS.md §5, the D221 contract; implementation D224).
//!
//! THE PROFILE IS A DEFAULT, NOT A MODE TOGGLE (D200 layering, the
//! D215 posture): the platform class is identified ONCE at window
//! startup and selects only the DEFAULT scale arm of the
//! [`PresentConfig`] the GPU scale path already consumes — never
//! `ModeConfig`, never `SimConfig`, never a hash, and BOTH pacing
//! arms accept it identically. The user's `--scale`/`--filter`
//! words keep their exact landed semantics: an explicit `--scale`
//! ALWAYS wins over the profile default (the default fills the gap
//! the user left), and the FILTER default stays Nearest on every
//! platform (the contract overrides the scale arm only).
//!
//! IDENTIFICATION (the mechanism this unit records, per the §5
//! contract): the SteamDeck class is the DMI identity of the
//! hardware itself, read-only from the standard sysfs DMI files —
//! `board_vendor` = "Valve" AND `product_name` = "Jupiter" (the
//! 1280x800 LCD deck) or "Galileo" (the 1280x800 OLED deck). Both
//! fields must match (trimmed, case-insensitively — firmware string
//! casing varies); anything else — missing files (non-Linux
//! platforms have no sysfs DMI tree), other vendors, other products
//! — classifies FAIL-CLOSED as [`PlatformClass::Generic`]. The env
//! is deliberately NOT consulted: `STEAMDECK=1` is a Steam session
//! fact, not a hardware fact, and a desktop with the variable
//! exported is not a SteamDeck. A probe that cannot read DMI (or
//! reads nothing) is never fatal — it is a Generic machine.
//!
//! THE DEFAULT THE PROFILE SELECTS (D224): on the SteamDeck's
//! 1280x800 16:10 panel the shipped Integer default would present
//! 640x480 centered — pillarbox bars by default, exactly the
//! posture the contract forbids. The profile default is the
//! fill-the-panel arm [`ScaleMode::Stretch`]: the WHOLE frame maps
//! onto the WHOLE panel edge to edge (no bars, no crop — the 4:3
//! aspect is absorbed by the non-uniform scale). This unit lands
//! the explicit aspect-distorting Stretch arm rather than reusing
//! the Fill crop (Fill covers the panel but CENTER-CROPS the frame,
//! hiding the top and bottom of the game's own 480 rows); the
//! choice is recorded in the registry row's note per the §5
//! contract. Generic platforms keep Integer + Nearest bit-for-bit
//! (the D215 pin `scaling_defaults_to_the_shipped_integer_nearest`
//! stays green — [`PlatformClass::default`] is Generic and
//! `PresentConfig::default()` is untouched).
//!
//! PARITY BOUNDS (D17 b): the profile selects NOTHING in the host
//! beyond the default of the already-landed scale knob — the
//! canonical 640x480 indexed frame + palette ride unchanged, the
//! headless path never probes DMI (it owns no surface), and the
//! sim config + every hash are identical under every class/CLI
//! combination (pinned by
//! `profile_selection_never_touches_the_sim_or_the_hashed_
//! trajectory`).

use std::path::Path;

use bedlam_platform::scale::ScaleMode;

/// The hardware platform class the shell starts on (P7, D224): the
/// recorded platform profile that selects the DEFAULT scale arm.
/// DEFAULT = [`PlatformClass::Generic`] — the shipped Integer +
/// Nearest posture bit-for-bit (the honest classification of every
/// machine the identification cannot prove is a SteamDeck,
/// fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlatformClass {
    /// Any machine that is not a SteamDeck (the default): the
    /// default scale arm stays the shipped Integer.
    #[default]
    Generic,
    /// A SteamDeck (Valve board, Jupiter/Galileo product): the
    /// default scale arm becomes the fill-the-panel Stretch.
    SteamDeck,
}

/// The DMI facts the classification reads (PLAIN DATA — the pure
/// classifier takes these so every arm is hermetically testable;
/// [`PlatformFacts::from_dmi`] is the one impure reader at the
/// binary's startup). `None` = the file is missing or unreadable
/// (never fatal; on platforms without a sysfs DMI tree everything
/// is `None` and the class is Generic).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlatformFacts {
    /// `/sys/.../board_vendor` — "Valve" on a SteamDeck.
    pub board_vendor: Option<String>,
    /// `/sys/.../product_name` — "Jupiter" (LCD) / "Galileo" (OLED).
    pub product_name: Option<String>,
}

impl PlatformFacts {
    /// Read the DMI facts from a DMI sysfs directory (the standard
    /// `/sys/devices/virtual/dmi/id` tree). READ-ONLY, best-effort:
    /// a missing or unreadable file is `None`, any IO error is
    /// `None` — the classification can never fail, only degrade to
    /// Generic. Contents are read as raw bytes trimmed of the
    /// trailing newline the kernel writes.
    pub fn from_dmi_dir(dir: impl AsRef<Path>) -> PlatformFacts {
        let read = |name: &str| -> Option<String> {
            std::fs::read(dir.as_ref().join(name))
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty())
        };
        PlatformFacts {
            board_vendor: read("board_vendor"),
            product_name: read("product_name"),
        }
    }

    /// Read the DMI facts from the live system tree (the binary's
    /// one startup probe; window host only).
    pub fn from_dmi() -> PlatformFacts {
        PlatformFacts::from_dmi_dir("/sys/devices/virtual/dmi/id")
    }
}

/// PURE: classify the platform from the DMI facts (the function
/// under test). SteamDeck iff `board_vendor` is "Valve" AND
/// `product_name` is "Jupiter" or "Galileo" (trimmed,
/// case-insensitive — firmware casing varies). FAIL-CLOSED: every
/// other combination, missing field or empty string classifies
/// Generic (a profile default is never guessed onto an unproven
/// machine).
pub fn classify_platform(facts: &PlatformFacts) -> PlatformClass {
    let is_valve = facts
        .board_vendor
        .as_deref()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("Valve"));
    let is_deck_product = facts.product_name.as_deref().is_some_and(|p| {
        let p = p.trim();
        p.eq_ignore_ascii_case("Jupiter") || p.eq_ignore_ascii_case("Galileo")
    });
    if is_valve && is_deck_product {
        PlatformClass::SteamDeck
    } else {
        PlatformClass::Generic
    }
}

/// The one startup probe (window host only): read the live DMI
/// facts and classify. Never fails, never fatal — an unreadable DMI
/// tree is a Generic machine with a note-free posture.
pub fn detect_platform() -> PlatformClass {
    classify_platform(&PlatformFacts::from_dmi())
}

/// PURE: the platform profile's DEFAULT scale arm (D224, the §5
/// contract). Generic keeps the shipped Integer bit-for-bit;
/// SteamDeck selects the fill-the-panel Stretch. This is only the
/// DEFAULT — the user's explicit `--scale` word always wins
/// ([`startup_scale_selection`]), and the filter default stays
/// Nearest on every platform.
pub fn profile_default_scale(class: PlatformClass) -> ScaleMode {
    match class {
        PlatformClass::Generic => ScaleMode::Integer,
        PlatformClass::SteamDeck => ScaleMode::Stretch,
    }
}

/// PURE: the startup scale selection — the user's explicit `--scale`
/// word wins over the platform profile default, exactly as the §5
/// contract pins ("overridable by the same `--scale`/`--filter` CLI
/// that already exists"). `None` (no flag given) falls to
/// [`profile_default_scale`]: Integer on Generic (the shipped
/// posture bit-for-bit), Stretch on SteamDeck (fill the panel).
pub fn startup_scale_selection(class: PlatformClass, cli: Option<ScaleMode>) -> ScaleMode {
    cli.unwrap_or_else(|| profile_default_scale(class))
}

#[cfg(test)]
mod platform_profile_tests {
    //! The p7-steamdeck-default unit (D224): the PLATFORM PROFILE —
    //! the SteamDeck class identified at startup (DMI facts, pure
    //! classifier) selecting only the default of the already-landed
    //! D215 scale surface. Every pin below is the no-arbitration
    //! posture: the profile is a platform knob OUT of ModeConfig
    //! that selects nothing in the host beyond the PresentConfig
    //! default the GPU scale path consumes.

    use super::*;
    use bedlam_platform::scale::{scale_rect, uv_rect, PresentConfig};

    fn facts(vendor: Option<&str>, product: Option<&str>) -> PlatformFacts {
        PlatformFacts {
            board_vendor: vendor.map(str::to_string),
            product_name: product.map(str::to_string),
        }
    }

    /// THE IDENTIFICATION: the SteamDeck is the Valve board + the
    /// Jupiter (LCD) / Galileo (OLED) product, matched trimmed and
    /// case-insensitively; EVERYTHING ELSE — including each fact
    /// alone, other products, other vendors, missing or empty
    /// fields — is Generic, fail-closed.
    #[test]
    fn steamdeck_is_the_valve_jupiter_or_galileo_dmi_identity() {
        assert_eq!(
            classify_platform(&facts(Some("Valve"), Some("Jupiter"))),
            PlatformClass::SteamDeck,
            "the LCD deck"
        );
        assert_eq!(
            classify_platform(&facts(Some("Valve"), Some("Galileo"))),
            PlatformClass::SteamDeck,
            "the OLED deck"
        );
        // Firmware casing/whitespace variance is absorbed, never fatal.
        assert_eq!(
            classify_platform(&facts(Some(" valve "), Some("jupiter"))),
            PlatformClass::SteamDeck
        );
        assert_eq!(
            classify_platform(&facts(Some("VALVE"), Some("GALILEO\n"))),
            PlatformClass::SteamDeck
        );
    }

    #[test]
    fn classification_fails_closed_to_generic() {
        // The vendor alone or the product alone never profiles.
        assert_eq!(
            classify_platform(&facts(Some("Valve"), Some("Index"))),
            PlatformClass::Generic
        );
        assert_eq!(
            classify_platform(&facts(Some("ASUS"), Some("Jupiter"))),
            PlatformClass::Generic
        );
        // Missing or empty fields (the unreadable-DMI posture).
        assert_eq!(
            classify_platform(&facts(None, Some("Jupiter"))),
            PlatformClass::Generic
        );
        assert_eq!(
            classify_platform(&facts(Some("Valve"), None)),
            PlatformClass::Generic
        );
        assert_eq!(
            classify_platform(&facts(None, None)),
            PlatformClass::Generic
        );
        assert_eq!(
            classify_platform(&facts(Some(""), Some("Jupiter"))),
            PlatformClass::Generic
        );
        assert_eq!(
            classify_platform(&facts(Some("Valve"), Some(""))),
            PlatformClass::Generic
        );
        // Near-miss products never match (whitespace/casing variance
        // IS absorbed by design — a padded "Jupiter" matches — but a
        // different word never does).
        for product in ["Sepulcher", "Galilean", "jupite", "Deck", "Jupiter1"] {
            assert_eq!(
                classify_platform(&facts(Some("Valve"), Some(product))),
                PlatformClass::Generic,
                "product {product:?} is not a deck product"
            );
        }
    }

    /// The DMI reader is best-effort over a REAL directory: present
    /// files (with the kernel's trailing newline + surrounding
    /// whitespace) read trimmed, absent files are None — never an
    /// error, never fatal.
    #[test]
    fn dmi_reader_is_best_effort_and_trims() {
        let dir = std::env::temp_dir().join(format!(
            "bedlam-dmi-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dmi dir");
        std::fs::write(dir.join("board_vendor"), b"Valve\n").expect("write vendor");
        std::fs::write(dir.join("product_name"), b"Jupiter\n").expect("write product");
        let read_back = PlatformFacts::from_dmi_dir(&dir);
        assert_eq!(read_back, facts(Some("Valve"), Some("Jupiter")));
        std::fs::remove_dir_all(&dir).ok();
        // An absent tree reads as no facts at all.
        assert_eq!(
            PlatformFacts::from_dmi_dir(dir.join("does-not-exist")),
            PlatformFacts::default()
        );
    }

    /// THE DEFAULT THE PROFILE SELECTS: Generic keeps the shipped
    /// Integer bit-for-bit; SteamDeck selects the fill-the-panel
    /// Stretch arm (D224 — the explicit aspect arm, NOT the Fill
    /// crop; recorded in the registry row's note).
    #[test]
    fn profile_default_scale_per_class() {
        assert_eq!(
            profile_default_scale(PlatformClass::Generic),
            ScaleMode::Integer
        );
        assert_eq!(
            profile_default_scale(PlatformClass::SteamDeck),
            ScaleMode::Stretch
        );
        assert_eq!(
            profile_default_scale(PlatformClass::default()),
            ScaleMode::Integer,
            "the default class keeps the default arm"
        );
    }

    /// THE CLI ALWAYS WINS (the §5 contract: "overridable by the
    /// same --scale/--filter CLI that already exists"): an explicit
    /// word is honored on EVERY class — a SteamDeck user can ask
    /// for Integer bars, a Generic user can ask for Stretch — and
    /// only the missing word falls to the profile default (Integer
    /// on Generic = PresentConfig::default() bit-for-bit, Stretch
    /// on SteamDeck).
    #[test]
    fn the_cli_word_always_wins_over_the_profile_default() {
        for class in [PlatformClass::Generic, PlatformClass::SteamDeck] {
            for word in [
                ScaleMode::Integer,
                ScaleMode::Fit,
                ScaleMode::Fill,
                ScaleMode::Stretch,
            ] {
                assert_eq!(
                    startup_scale_selection(class, Some(word)),
                    word,
                    "explicit --scale wins on {class:?}"
                );
            }
        }
        assert_eq!(
            startup_scale_selection(PlatformClass::Generic, None),
            ScaleMode::Integer
        );
        assert_eq!(
            startup_scale_selection(PlatformClass::SteamDeck, None),
            ScaleMode::Stretch
        );
    }

    /// THE USER-VISIBLE POSTURE THE CONTRACT PINS: on the deck's
    /// 1280x800 16:10 panel the profile default fills the panel
    /// edge to edge — the whole frame onto the whole target, no
    /// bars (Integer/ Fit would pillarbox), no crop (Fill would
    /// hide 80 rows top + bottom of the game's own 480).
    #[test]
    fn the_steamdeck_default_fills_the_panel_edge_to_edge() {
        let stretch = profile_default_scale(PlatformClass::SteamDeck);
        let rect = scale_rect(stretch, 640, 480, 1280, 800);
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (0, 0, 1280, 800));
        assert_eq!(uv_rect(stretch, 640, 480, 1280, 800), [0.0, 0.0, 1.0, 1.0]);
        // The generic default keeps bars exactly as shipped (Integer
        // on 1280x800 = 1x 640x480 centered: 320 px side bars).
        let integer = profile_default_scale(PlatformClass::Generic);
        let bars = scale_rect(integer, 640, 480, 1280, 800);
        assert_eq!((bars.x, bars.w), (320, 640), "the generic default pillars");
        // The Fill arm fills the panel but crops — NOT the profile
        // default (the whole-frame posture is the recorded choice).
        let fill_uv = uv_rect(ScaleMode::Fill, 640, 480, 1280, 800);
        assert!(fill_uv[1] > 0.0 && fill_uv[3] < 1.0, "fill crops rows");
    }

    /// The profile touches ONLY the default of the scale knob: the
    /// composed configs still carry the parity palette expansion
    /// under every class, and the Generic default round-trips
    /// `PresentConfig::default()` bit-for-bit (the D215 pin's
    /// premise stays true on every platform).
    #[test]
    fn the_profile_touches_only_the_scale_default() {
        use bedlam_render::VgaExpand;
        for class in [PlatformClass::Generic, PlatformClass::SteamDeck] {
            let cfg = crate::window::scaling_present_config(
                startup_scale_selection(class, None),
                Default::default(),
            );
            assert_eq!(cfg.expand, VgaExpand::Original, "{class:?}");
            assert_eq!(cfg.filter, bedlam_platform::scale::FilterMode::Nearest);
        }
        let generic = crate::window::scaling_present_config(
            startup_scale_selection(PlatformClass::Generic, None),
            Default::default(),
        );
        assert_eq!(generic, PresentConfig::default());
    }
}
