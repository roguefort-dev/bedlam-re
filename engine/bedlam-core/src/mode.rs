//! The P6 mode seam: ONE immutable [`ModeConfig`] injected at sim
//! construction (docs/PLAN.md §6 "P6 — Modernization"; docs/
//! P6-MODERNIZATION.md §1; D200 binding, D201 implementation).
//!
//! Contract (D200, verbatim consequences in P6-MODERNIZATION §1):
//!
//! - **Mode is one immutable `ModeConfig` injected at sim construction.**
//!   It rides [`crate::sim::SimConfig`] into [`crate::sim::Sim::new`]
//!   and never mutates afterwards: the type has NO `&mut self` method
//!   and no public field — the only way to a different mode is a
//!   different `ModeConfig`, and the only way to a different mode
//!   mid-run is a NEW sim ("a mode change is a new sim").
//! - **Default = modern.** `ModeConfig::default()` and the MODERN preset
//!   are the all-modern-arms configuration; the canonical chains and
//!   every parity harness run under it.
//! - **Classic = a small purist toggle set** over the FEEL-CONTESTED
//!   axes only. This unit lands exactly the two plan-named axes —
//!   [`PuristToggle::TimingLock`] and [`PuristToggle::ControlScheme`]
//!   — plus (later, per-entry) `closed-preserve-classic` catalog
//!   entries whose `purist_toggle` ids join the same namespace. The
//!   catalog itself stays EMPTY until entries exist with recorded
//!   evidence (D200 seeding policy); no toggle here is a catalog
//!   entry.
//! - **Presentation/platform options are NOT mode toggles**: window
//!   mode, vsync, resolution, scaling, refresh rate etc. never enter
//!   `ModeConfig` and never enter the sim (Determinism Charter,
//!   PLAN §3). The timing-lock axis selects a PACING POLICY
//!   (frame-locked classic pacing vs the modern accumulator), not a
//!   display rate — the logic tick stays fixed at the original rate
//!   in every arm, and no arm of any toggle carries a Hz.
//!
//! Layering: `ModeConfig` is CONFIG, not state. Like the seed and time
//! base it is deliberately NOT part of [`crate::sim::Sim::state_hash`]
//! and NOT serialized into snapshots/replays (the formats stay
//! byte-stable; FORMAT_VERSION does not move in this unit). A restore
//! adopts the mode of the `SimConfig` it is restored under
//! (see [`crate::sim::Sim::restore`]) — restoring IS constructing a new
//! sim. The initial two axes are host-side policies (pacing, input
//! mapping) with zero in-sim consumers, so the sim trajectory is
//! arm-independent in this unit; a later unit that gives an axis (or a
//! catalog toggle) an in-sim consumer diverges the arms there, and
//! THAT unit decides whether the replay header starts recording the
//! mode (with a FORMAT_VERSION bump then, not now).

/// One arm of a purist-toggle axis: the modern arm (the default — the
/// modernization the plan asks for) or the classic arm (the purist
/// preservation of original behavior).
///
/// Naming follows the rubric vocabulary: `gameplay-coupled` catalog
/// entries are "classic preserves / modern fixes" (P6-MODERNIZATION
/// §2). Every axis defaults to [`ToggleArm::Modern`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ToggleArm {
    /// The modern arm: the fix / new behavior. Default everywhere.
    #[default]
    Modern,
    /// The classic arm: purist preservation of the original behavior.
    Classic,
}

impl ToggleArm {
    /// `true` iff this is the classic (purist) arm.
    pub const fn is_purist(self) -> bool {
        matches!(self, ToggleArm::Classic)
    }
}

/// A purist-toggle axis: one feel-contested classic/modern choice.
///
/// The two variants of this unit are the plan-named axes (PLAN §6 P6:
/// "timing lock, control scheme"). The set may GROW only through
/// catalog entries classified `closed-preserve-classic` by the §2
/// rubric (their `purist_toggle` id names the axis) — never ad hoc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PuristToggle {
    /// The timing-lock axis: classic = the original frame-locked,
    /// presentation-coupled pacing; modern = the time-based
    /// accumulator (PLAN §6 P6 first bullet). Pacing policy only —
    /// the logic tick rate is fixed at the original rate in BOTH
    /// arms; no display rate ever enters the sim.
    TimingLock,
    /// The control-scheme axis: classic = the original scheme;
    /// modern = WASD, 1-4 hotkeys, full remap, wheel zoom, gamepad
    /// (PLAN §6 P6 "Modern controls ... original scheme selectable").
    ControlScheme,
}

impl PuristToggle {
    /// Every axis of the current toggle set, in declaration order.
    /// The test surface parameterizes over THIS list (both arms of
    /// each axis), never the feature cross-product (D200).
    pub const ALL: [PuristToggle; 2] = [PuristToggle::TimingLock, PuristToggle::ControlScheme];

    /// The stable toggle id — the catalog-join namespace. Rules
    /// mirror the catalog's `purist_toggle` field rules (checker R3):
    /// non-empty, whitespace-free, unique across the set. RESERVED:
    /// these two plan-named ids are taken; a future catalog entry's
    /// `purist_toggle` id must not collide with them (checker-side
    /// enforcement lands with the first catalog entry).
    pub const fn id(self) -> &'static str {
        match self {
            PuristToggle::TimingLock => "timing-lock",
            PuristToggle::ControlScheme => "control-scheme",
        }
    }

    /// Parse a toggle id (the inverse of [`PuristToggle::id`]);
    /// unknown ids return `None`.
    pub fn from_id(id: &str) -> Option<PuristToggle> {
        match id {
            "timing-lock" => Some(PuristToggle::TimingLock),
            "control-scheme" => Some(PuristToggle::ControlScheme),
            _ => None,
        }
    }
}

/// The mode a simulation runs under: ONE immutable value injected at
/// sim construction (via [`crate::sim::SimConfig`]), never mutated
/// mid-run — a mode change is a new sim (D200/D201).
///
/// Construct it with [`ModeConfig::default`] (modern),
/// [`ModeConfig::CLASSIC`], or per-axis through the consuming
/// [`ModeConfig::with`] builder:
///
/// ```
/// use bedlam_core::mode::{ModeConfig, PuristToggle, ToggleArm};
///
/// // Default = modern (every axis on the modern arm).
/// assert_eq!(ModeConfig::default(), ModeConfig::MODERN);
///
/// // Classic preset = every axis on the classic arm.
/// assert!(ModeConfig::CLASSIC.is_purist(PuristToggle::TimingLock));
///
/// // Per-axis: modern controls, purist timing lock — built as a NEW
/// // value; the original is never mutated (there is no mutation API).
/// let mixed = ModeConfig::default()
///     .with(PuristToggle::TimingLock, ToggleArm::Classic);
/// assert!(mixed.is_purist(PuristToggle::TimingLock));
/// assert!(!mixed.is_purist(PuristToggle::ControlScheme));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModeConfig {
    timing_lock: ToggleArm,
    control_scheme: ToggleArm,
}

impl ModeConfig {
    /// The MODERN preset: every axis on the modern arm. This is the
    /// default mode (PLAN §6: "default = modern") — the canonical
    /// chains and parity harnesses run under it.
    pub const MODERN: ModeConfig = ModeConfig {
        timing_lock: ToggleArm::Modern,
        control_scheme: ToggleArm::Modern,
    };

    /// The CLASSIC preset: every axis of the current purist toggle
    /// set on the classic arm (the purist configuration; grows only
    /// as the toggle set grows, per-axis).
    pub const CLASSIC: ModeConfig = ModeConfig {
        timing_lock: ToggleArm::Classic,
        control_scheme: ToggleArm::Classic,
    };

    /// A copy of this config with ONE axis set to `arm` — the only
    /// way to change anything. CONSUMES `self` and returns a new
    /// value: the source is never mutated (there is no `&mut self`
    /// anywhere on this type; immutability is the type's shape, not a
    /// convention).
    pub const fn with(self, axis: PuristToggle, arm: ToggleArm) -> ModeConfig {
        match axis {
            PuristToggle::TimingLock => ModeConfig {
                timing_lock: arm,
                control_scheme: self.control_scheme,
            },
            PuristToggle::ControlScheme => ModeConfig {
                timing_lock: self.timing_lock,
                control_scheme: arm,
            },
        }
    }

    /// The arm one axis is set to.
    pub const fn arm(self, axis: PuristToggle) -> ToggleArm {
        match axis {
            PuristToggle::TimingLock => self.timing_lock,
            PuristToggle::ControlScheme => self.control_scheme,
        }
    }

    /// `true` iff the axis sits on the classic (purist) arm.
    pub const fn is_purist(self, axis: PuristToggle) -> bool {
        self.arm(axis).is_purist()
    }
}

impl Default for ModeConfig {
    /// Default mode = modern (PLAN §6). Identical to
    /// [`ModeConfig::MODERN`].
    fn default() -> Self {
        ModeConfig::MODERN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The toggle ids are the catalog-join namespace: non-empty,
    /// whitespace-free, unique across the set, and round-trip through
    /// `from_id`. Unknown ids parse to `None` (fail-closed).
    #[test]
    fn toggle_ids_round_trip_unique_and_stable() {
        let mut seen: Vec<&str> = Vec::new();
        for axis in PuristToggle::ALL {
            let id = axis.id();
            assert!(!id.is_empty());
            assert!(
                !id.chars().any(char::is_whitespace),
                "id {id:?} must be whitespace-free"
            );
            assert!(!seen.contains(&id), "duplicate toggle id {id:?}");
            seen.push(id);
            assert_eq!(PuristToggle::from_id(id), Some(axis));
        }
        assert_eq!(PuristToggle::from_id("not-a-toggle"), None);
        assert_eq!(PuristToggle::from_id(""), None);
        // The plan-named axis ids this unit pins (D201): stable
        // strings, the catalog must not collide with them later.
        assert_eq!(PuristToggle::TimingLock.id(), "timing-lock");
        assert_eq!(PuristToggle::ControlScheme.id(), "control-scheme");
    }

    /// Default = modern on every axis of the current set (PLAN §6
    /// "default = modern"). The classic preset = purist on every
    /// axis. Checked per-axis over `ALL` — both arms of the toggle
    /// set, never the feature cross-product (D200).
    #[test]
    fn default_is_modern_and_classic_preset_is_purist() {
        assert_eq!(ModeConfig::default(), ModeConfig::MODERN);
        for axis in PuristToggle::ALL {
            assert_eq!(ModeConfig::default().arm(axis), ToggleArm::Modern);
            assert!(!ModeConfig::default().is_purist(axis));
            assert_eq!(ModeConfig::CLASSIC.arm(axis), ToggleArm::Classic);
            assert!(ModeConfig::CLASSIC.is_purist(axis));
        }
        assert_ne!(ModeConfig::MODERN, ModeConfig::CLASSIC);
    }

    /// The builder overrides exactly ONE axis and leaves the source
    /// untouched — the immutability proof at value level: a mode
    /// change is a NEW ModeConfig (there is no mutation path; a mode
    /// change mid-run is a new sim).
    #[test]
    fn with_overrides_one_axis_and_never_mutates_the_source() {
        for axis in PuristToggle::ALL {
            let base = ModeConfig::default();
            let purist = base.with(axis, ToggleArm::Classic);
            // The new value flips exactly the chosen axis...
            assert!(purist.is_purist(axis));
            for other in PuristToggle::ALL {
                if other != axis {
                    assert!(!purist.is_purist(other), "only {axis:?} moves");
                }
            }
            // ...and the source is unchanged (Copy semantics, no &mut).
            assert_eq!(base, ModeConfig::MODERN);
            // Reverting via `with` is the identity on the source arm.
            let reverted = purist.with(axis, ToggleArm::Modern);
            assert_eq!(reverted, ModeConfig::MODERN);
            // One axis off classic = everything else stays classic.
            let one_off = ModeConfig::CLASSIC.with(axis, ToggleArm::Modern);
            for other in PuristToggle::ALL {
                assert_eq!(
                    one_off.arm(other),
                    if other == axis {
                        ToggleArm::Modern
                    } else {
                        ToggleArm::Classic
                    }
                );
            }
        }
    }

    /// Both arms, both axes, through the full constructor surface:
    /// `arm()`/`is_purist()` agree everywhere.
    #[test]
    fn arm_queries_agree_for_both_arms_of_every_axis() {
        for axis in PuristToggle::ALL {
            for arm in [ToggleArm::Modern, ToggleArm::Classic] {
                let config = ModeConfig::default().with(axis, arm);
                assert_eq!(config.arm(axis), arm);
                assert_eq!(config.is_purist(axis), arm.is_purist());
            }
        }
    }
}
