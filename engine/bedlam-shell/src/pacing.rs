//! The P6 frame-pacing benchmark harness (PLAN §6 closing QoL
//! instrument, D219): "An automated scheduled CI benchmark checks
//! 240Hz frame pacing against a pinned hardware profile and
//! thresholds; an unavailable profile creates no task and only
//! excludes that platform attestation."
//!
//! Two halves, deliberately split by hermeticity:
//!
//! 1. THE PURE CADENCE MATH (hermetic, gate-covered): a delta
//!    trace — measured or synthetic frame deltas in nanoseconds —
//!    replayed through the EXACT present-loop arithmetic
//!    ([`CadenceDriver::frame`]: `FixedStepClock::advance` decides
//!    pumps due, each pump runs the fixed dt through
//!    `GameHost::pump_frame`, then the loop's own gate/alpha
//!    decisions [`crate::window::present_due`] /
//!    [`crate::window::present_camera_alpha`] answer, the present
//!    recomposing at the accumulator fraction). Summarized into
//!    the feel-proxy metrics the plan sentence names: pump cadence
//!    (pumps per delta, dropped pumps), present-gate answers
//!    (presents vs held frames) and the recompose alpha cadence,
//!    plus the p95 frame-time percentile of the measured trace.
//!    One loop-shape fact the replay records faithfully: a
//!    zero-PUMP frame never calls `pump_frame`, so the gate
//!    inherits the last pump's answer — after the first tick it
//!    answers YES on every frame in BOTH arms (the classic arm's
//!    frame-locked hold lands at CONTENT level: one NEW image per
//!    executed tick, unchanged frames re-presented), with the
//!    alpha cadence the arm-visible difference.
//! 2. THE BOUNDED MEASUREMENT (wall-clock, profile-gated): the
//!    `examples/frame-pacing.rs` binary runs the SAME driver
//!    against a wall clock — a 240 Hz-cadence proxy loop (sleep to
//!    the next display period, measure the pacing path) — but ONLY
//!    on a machine that declares the PINNED HARDWARE PROFILE
//!    ([`PINNED_240HZ`]); any other machine takes the
//!    UNAVAILABLE-PROFILE POSTURE: skip clean (exit 0 + an
//!    explicit no-attestation note), never a false red, never a
//!    task. The scheduled CI workflow
//!    (`.github/workflows/frame-pacing.yml`) runs that binary on a
//!    cron; GitHub runners never declare the profile, so the job
//!    exercises exactly the skip-clean posture.
//!
//! Provenance: a DECISION/instrument (D219), not an RE artifact —
//! every original-behavior fact cited here is already landed and
//! anchored (the classic arm's frame-locked present-coupled pacing
//! is RE-EXW-PACER §3 [verified]; the accumulator-fraction camera
//! composition is docs/RE-EXW-CAMERA.md §5 [verified]). The
//! harness drives the real landed seams and adds ZERO new binary
//! claims.
//!
//! Bounds: presentation-bucket only (D17 b) — the driver pumps a
//! bare host (default config, empty palette, no corpus asset ever
//! staged), records gate/alpha answers and never hashes anything;
//! no engine change (the harness needed no read-only seam beyond
//! crate-visible helpers); the hashed trajectory is arm-invariant
//! under the replay (unit-pinned below).

use bedlam_core::input::InputFrame;
use bedlam_core::mode::ModeConfig;
use bedlam_core::sim::SimConfig;
use bedlam_game::{GameConfig, GameHost};

use crate::clock::{FixedStepClock, SUBTICKS_PER_PUMP};
use crate::window::{present_camera_alpha, present_due};

/// The environment variable a machine uses to DECLARE itself the
/// pinned hardware profile (the only channel; nothing probes the
/// hardware — a machine that does not declare the identity is
/// unavailable by definition, so CI runners and stray machines can
/// never produce a false attestation).
pub const PROFILE_ENV: &str = "BEDLAM_PACING_PROFILE";

/// The pinned hardware profile — committed data (machine class,
/// refresh, thresholds, sample size) the scheduled benchmark
/// attests against (PLAN §6). `id` is the declared identity string
/// compared against [`PROFILE_ENV`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacingProfile {
    /// The declared identity (the exact [`PROFILE_ENV`] value that
    /// matches this profile).
    pub id: &'static str,
    /// The machine class this profile pins (human-readable).
    pub machine_class: &'static str,
    /// The display refresh the budget is stated at (Hz).
    pub display_refresh_hz: u32,
    /// The p95 frame-time budget at that refresh: 1.25 display
    /// periods (PLAN §6 "no stutter under p95 frame-time budget at
    /// that refresh" — a concrete committed threshold).
    pub p95_frame_time_budget_ns: u64,
    /// The bounded measurement length in frames (the wall-clock
    /// benchmark's hard iteration cap).
    pub sample_frames: u32,
}

impl PacingProfile {
    /// One display period at the pinned refresh (ns).
    pub fn period_ns(&self) -> u64 {
        1_000_000_000 / u64::from(self.display_refresh_hz)
    }
}

/// THE pinned hardware profile: the operator desktop class with a
/// 240 Hz vsync-locked display (the plan's own "checks 240Hz frame
/// pacing against a pinned hardware profile"). Budget = 1.25 x the
/// 240 Hz period = 5_208_333 ns; sample = 2400 frames (10 s of
/// 240 Hz cadence — bounded in iterations AND wall time).
pub const PINNED_240HZ: PacingProfile = PacingProfile {
    id: "pinned-240hz-desk-v1",
    machine_class: "operator desktop class, 240 Hz vsync-locked \
                    display, modern x86-64 desktop",
    display_refresh_hz: 240,
    p95_frame_time_budget_ns: 5_208_333,
    sample_frames: 2400,
};

/// Whether the running machine carries a pinned profile the
/// benchmark may attest against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSelection {
    /// The machine declared the pinned identity: measure and
    /// attest (or fail — a real red only ever on this path).
    Matched(&'static PacingProfile),
    /// No pinned profile on this machine: skip clean, exit 0, no
    /// attestation (the plan's unavailable-profile posture).
    Unavailable,
}

/// Resolve the profile posture from the declared identity
/// (typically `std::env::var(PROFILE_ENV)`). Exact-match only.
pub fn profile_for(declared: Option<&str>) -> ProfileSelection {
    match declared {
        Some(id) if id == PINNED_240HZ.id => ProfileSelection::Matched(&PINNED_240HZ),
        _ => ProfileSelection::Unavailable,
    }
}

/// One replayed host frame: the measured delta fed in and every
/// cadence answer the present loop derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePacing {
    /// The measured inter-frame delta this frame was fed (ns).
    pub delta_ns: u64,
    /// Pumps the fixed-step clock found due for this delta.
    pub pumps_due: u32,
    /// Sim ticks the due pumps actually executed (each canonical
    /// pump is one 60 Hz tick; a stalled frame executes the
    /// anti-spiral-clamped count).
    pub ticks_executed: u32,
    /// The present-gate answer after the frame's pumps (the loop's
    /// `present_due`). NOTE — the LOOP-SHAPE truth this records:
    /// the gate is consulted after the frame's pump batch, and a
    /// ZERO-PUMP frame never calls `pump_frame`, so it inherits the
    /// last pump's answer; under the loop shape every pump executes
    /// its one canonical tick, so after the first tick the gate
    /// answers YES on every frame in BOTH arms (the classic arm's
    /// frame-locked hold lands at CONTENT level — a zero-pump frame
    /// re-presents the unchanged canonical image, exactly one NEW
    /// image per executed tick — while `camera_alpha` below stays
    /// the arm-visible difference).
    pub present: bool,
    /// The recompose alpha the present site would use (the loop's
    /// `present_camera_alpha`: `Some(fraction)` on a presenting
    /// modern-arm frame, `None` otherwise).
    pub camera_alpha: Option<f32>,
}

/// The pacing-path frame driver — the exact present-loop shape
/// (`about_to_wait` measure/advance/pump pass + the
/// `RedrawRequested` present gate/alpha/recompose), over a bare
/// host (default config, empty palette, no corpus asset staged —
/// the cadence harness needs no game data). Shared by the hermetic
/// replay ([`replay_cadence`]) and the profile-gated wall-clock
/// measurement (`examples/frame-pacing.rs`), so the benchmark can
/// never drift from the loop it measures.
pub struct CadenceDriver {
    clock: FixedStepClock,
    host: GameHost,
    frames: Vec<FramePacing>,
}

impl CadenceDriver {
    /// A driver over a bare host under `mode` (the immutable
    /// mode rides `SimConfig` as config, never state — D201).
    pub fn new(mode: ModeConfig) -> CadenceDriver {
        CadenceDriver {
            clock: FixedStepClock::host(),
            host: GameHost::new(
                &GameConfig::default(),
                &SimConfig {
                    mode,
                    ..SimConfig::default()
                },
                [[0u8, 0, 0]; 256],
            ),
            frames: Vec::new(),
        }
    }

    /// Run one present-loop-shaped frame over a MEASURED delta:
    /// the clock answers pumps due, each pump hands the host the
    /// fixed dt (a neutral input snapshot — the cadence math is
    /// input-independent), then the loop's own gate and alpha
    /// decisions are recorded and the presenting frame recomposes
    /// at the accumulator fraction (exactly the present site's
    /// order: gate, alpha, recompose).
    pub fn frame(&mut self, delta_ns: u64) -> FramePacing {
        let pumps_due = self.clock.advance(delta_ns);
        let mut ticks_executed = 0u32;
        for _ in 0..pumps_due {
            ticks_executed += self
                .host
                .pump_frame(SUBTICKS_PER_PUMP, &InputFrame::default());
        }
        // The present site's exact order: the gate first (a held
        // frame writes nothing, so classic zero-tick frames do not
        // recompose), then the alpha + recompose.
        let present = present_due(&self.host);
        let camera_alpha = if present {
            present_camera_alpha(&self.host, &self.clock)
        } else {
            None
        };
        if let Some(alpha) = camera_alpha {
            self.host.recompose(alpha);
        }
        let frame = FramePacing {
            delta_ns,
            pumps_due,
            ticks_executed,
            present,
            camera_alpha,
        };
        self.frames.push(frame);
        frame
    }

    /// Pumps the anti-spiral clamp DISCARDED so far (a non-zero
    /// count means the measured cadence stalled).
    pub fn pumps_dropped(&self) -> u64 {
        self.clock.ticks_dropped()
    }

    /// The bare host (presentation-bucket reads only).
    pub fn host(&self) -> &GameHost {
        &self.host
    }

    /// Consume the driver into its recorded trace.
    pub fn into_trace(self) -> PacingTrace {
        PacingTrace {
            pumps_dropped: self.pumps_dropped(),
            frames: self.frames,
        }
    }
}

/// A full replayed cadence: every frame's pacing answers plus the
/// clock's dropped-pump total.
#[derive(Debug, Clone, PartialEq)]
pub struct PacingTrace {
    /// Per-frame answers, in replay order.
    pub frames: Vec<FramePacing>,
    /// Pumps discarded by the anti-spiral clamp over the trace.
    pub pumps_dropped: u64,
}

/// Replay a delta trace through the pacing path under `mode` —
/// the hermetic benchmark core (pure: no clock reads, no window,
/// no corpus; the same driver the wall-clock measurement runs).
pub fn replay_cadence(mode: ModeConfig, deltas_ns: &[u64]) -> PacingTrace {
    let mut driver = CadenceDriver::new(mode);
    for &delta_ns in deltas_ns {
        driver.frame(delta_ns);
    }
    driver.into_trace()
}

/// Nearest-rank percentile over the frame deltas (pure; `percent`
/// in 1..=100). The p95 of the measured trace is the feel-proxy
/// "no stutter" metric the profile budget bounds.
pub fn percentile_ns(deltas_ns: &[u64], percent: u64) -> u64 {
    assert!(
        (1..=100).contains(&percent),
        "percentile percent must be 1..=100"
    );
    assert!(!deltas_ns.is_empty(), "percentile of an empty trace");
    let mut sorted = deltas_ns.to_vec();
    sorted.sort_unstable();
    let rank = (percent * sorted.len() as u64).div_ceil(100) as usize;
    sorted[rank - 1]
}

/// The feel-proxy summary of a replayed trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacingSummary {
    /// Host frames replayed.
    pub frames: u64,
    /// Pumps the clock found due (executed + clamped).
    pub pumps_due: u64,
    /// Sim ticks the pumps executed.
    pub ticks_executed: u64,
    /// Zero-tick host frames (high-refresh present cadence).
    pub zero_tick_frames: u64,
    /// Frames the present gate answered YES on.
    pub presents: u64,
    /// Frames the gate HELD (classic zero-tick frames).
    pub held_frames: u64,
    /// Pumps discarded by the anti-spiral clamp (a stall).
    pub pumps_dropped: u64,
    /// Nearest-rank p95 of the measured frame deltas (ns).
    pub p95_frame_time_ns: u64,
    /// Worst measured frame delta (ns).
    pub max_frame_time_ns: u64,
}

/// Summarize a trace into the feel-proxy metrics.
pub fn summarize(trace: &PacingTrace) -> PacingSummary {
    let mut summary = PacingSummary {
        frames: trace.frames.len() as u64,
        pumps_due: 0,
        ticks_executed: 0,
        zero_tick_frames: 0,
        presents: 0,
        held_frames: 0,
        pumps_dropped: trace.pumps_dropped,
        p95_frame_time_ns: 0,
        max_frame_time_ns: 0,
    };
    let mut deltas = Vec::with_capacity(trace.frames.len());
    for frame in &trace.frames {
        summary.pumps_due += u64::from(frame.pumps_due);
        summary.ticks_executed += u64::from(frame.ticks_executed);
        summary.zero_tick_frames += u64::from(frame.ticks_executed == 0);
        summary.presents += u64::from(frame.present);
        summary.held_frames += u64::from(!frame.present);
        deltas.push(frame.delta_ns);
        summary.max_frame_time_ns = summary.max_frame_time_ns.max(frame.delta_ns);
    }
    if !deltas.is_empty() {
        summary.p95_frame_time_ns = percentile_ns(&deltas, 95);
    }
    summary
}

/// The benchmark verdict against a profile posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacingVerdict {
    /// No pinned profile on this machine — nothing is attested and
    /// nothing fails (the plan's unavailable-profile posture).
    NoAttestation,
    /// The measured cadence meets the pinned thresholds.
    Attested {
        /// Measured p95 frame time (ns).
        p95_frame_time_ns: u64,
        /// Pumps discarded by the anti-spiral clamp.
        pumps_dropped: u64,
    },
    /// The pinned profile matched but a threshold failed — the
    /// only red the instrument can produce, and only ever on the
    /// pinned machine.
    Exceeded {
        /// Measured p95 frame time (ns).
        p95_frame_time_ns: u64,
        /// The profile's p95 budget it exceeded (ns).
        budget_ns: u64,
        /// Pumps discarded by the anti-spiral clamp.
        pumps_dropped: u64,
    },
}

/// Apply the pinned thresholds: p95 frame time within budget AND
/// zero dropped pumps (the anti-spiral clamp firing IS stutter).
/// An unavailable profile is `NoAttestation` regardless of the
/// measured numbers — never a false red.
pub fn verdict(selection: ProfileSelection, summary: &PacingSummary) -> PacingVerdict {
    let ProfileSelection::Matched(profile) = selection else {
        return PacingVerdict::NoAttestation;
    };
    let within =
        summary.p95_frame_time_ns <= profile.p95_frame_time_budget_ns && summary.pumps_dropped == 0;
    if within {
        PacingVerdict::Attested {
            p95_frame_time_ns: summary.p95_frame_time_ns,
            pumps_dropped: summary.pumps_dropped,
        }
    } else {
        PacingVerdict::Exceeded {
            p95_frame_time_ns: summary.p95_frame_time_ns,
            budget_ns: profile.p95_frame_time_budget_ns,
            pumps_dropped: summary.pumps_dropped,
        }
    }
}

/// The printed benchmark outcome (report lines + process exit
/// code) — the entire behavior of the measurement binary except
/// the wall-clock measurement itself, kept pure so the hermetic
/// suite pins every posture including the skip-clean one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkOutcome {
    /// The exit code the measurement binary should exit with:
    /// 0 for no-attestation and attested, 1 only for exceeded.
    pub exit_code: i32,
    /// The report lines (printed in order).
    pub lines: Vec<String>,
}

/// Build the benchmark report for a profile posture over a
/// measured trace (the trace is ignored on the unavailable path —
/// there is nothing to measure without the pinned machine).
pub fn benchmark_report(selection: ProfileSelection, trace: &PacingTrace) -> BenchmarkOutcome {
    match selection {
        ProfileSelection::Unavailable => BenchmarkOutcome {
            exit_code: 0,
            lines: vec![
                format!(
                    "frame-pacing: pinned profile '{}' UNAVAILABLE on this machine",
                    PINNED_240HZ.id
                ),
                "frame-pacing: no attestation recorded (the platform is \
                 excluded from the 240Hz pacing attestation) - skipping, exit 0"
                    .to_string(),
            ],
        },
        ProfileSelection::Matched(profile) => {
            let summary = summarize(trace);
            let mut lines = vec![
                format!(
                    "frame-pacing: pinned profile '{}' MATCHED (machine class: {})",
                    profile.id, profile.machine_class
                ),
                format!(
                    "frame-pacing: {} frames at the {} Hz cadence proxy \
                     (period {} ns, p95 budget {} ns)",
                    summary.frames,
                    profile.display_refresh_hz,
                    profile.period_ns(),
                    profile.p95_frame_time_budget_ns
                ),
                format!(
                    "frame-pacing: p95 frame time {} ns / max {} ns / \
                     pumps {} / ticks {} / presents {} / held {} / dropped {}",
                    summary.p95_frame_time_ns,
                    summary.max_frame_time_ns,
                    summary.pumps_due,
                    summary.ticks_executed,
                    summary.presents,
                    summary.held_frames,
                    summary.pumps_dropped
                ),
            ];
            match verdict(selection, &summary) {
                PacingVerdict::Attested { .. } => {
                    lines.push("frame-pacing: VERDICT: ATTESTED (exit 0)".to_string());
                    BenchmarkOutcome {
                        exit_code: 0,
                        lines,
                    }
                }
                PacingVerdict::Exceeded {
                    p95_frame_time_ns,
                    budget_ns,
                    pumps_dropped,
                } => {
                    lines.push(format!(
                        "frame-pacing: VERDICT: EXCEEDED (p95 {} ns > budget \
                         {} ns, dropped pumps {}) - exit 1",
                        p95_frame_time_ns, budget_ns, pumps_dropped
                    ));
                    BenchmarkOutcome {
                        exit_code: 1,
                        lines,
                    }
                }
                PacingVerdict::NoAttestation => unreachable!("matched selection attests"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedlam_core::mode::ModeConfig;

    /// The exact 240 Hz vsync period the synthetic cadences use
    /// (4_166_666 ns — the same constant the clock tests pin).
    const NS_240HZ: u64 = 4_166_666;

    fn cadence_240hz(frames: usize) -> Vec<u64> {
        vec![NS_240HZ; frames]
    }

    /// The pump arithmetic at 240 Hz: 240 frames bank and execute
    /// exactly 59 pumps (181 zero-tick frames), nothing dropped —
    /// hand-derived and pinned by the clock tests; this pins the
    /// REPLAY carries the same arithmetic through the real host
    /// pumps (every due pump executes its one canonical tick).
    #[test]
    fn replay_pumps_match_the_clock_arithmetic_at_240hz() {
        let trace = replay_cadence(ModeConfig::MODERN, &cadence_240hz(240));
        assert_eq!(trace.pumps_dropped, 0);
        let summary = summarize(&trace);
        assert_eq!(summary.frames, 240);
        assert_eq!(summary.pumps_due, 59);
        assert_eq!(summary.ticks_executed, 59);
        assert_eq!(summary.zero_tick_frames, 181);
        // Every executed pump is exactly one canonical tick on the
        // unstalled cadence.
        assert!(trace.frames.iter().all(|f| f.pumps_due == f.ticks_executed));
        // The measured-delta metrics: an ideal cadence's p95 IS the
        // period, under the 1.25x budget.
        assert_eq!(summary.p95_frame_time_ns, NS_240HZ);
        assert_eq!(summary.max_frame_time_ns, NS_240HZ);
    }

    /// The present-gate answers, both arms, on the 240 Hz cadence —
    /// the LOOP-SHAPE truth: the gate is consulted after each
    /// frame's pump batch, and a zero-PUMP frame inherits the last
    /// pump's answer (stale-true after the first tick, since every
    /// loop pump executes its canonical tick), so BOTH arms answer
    /// YES on every frame. The arms differ in the PRESENT
    /// COMPOSITION: MODERN carries an alpha on every present
    /// (every frame recomposes from latest state), CLASSIC never
    /// does — and the classic arm's visible refresh follows the
    /// fixed logic tick at CONTENT level: exactly 59 of 240 frames
    /// present a NEW image (the tick frames), the other 181
    /// re-present the unchanged canonical image.
    #[test]
    fn present_gate_answers_both_arms_at_240hz() {
        let modern = replay_cadence(ModeConfig::MODERN, &cadence_240hz(240));
        assert!(modern.frames.iter().all(|f| f.present));
        assert!(modern.frames.iter().all(|f| f.camera_alpha.is_some()));
        assert_eq!(summarize(&modern).presents, 240);
        assert_eq!(summarize(&modern).held_frames, 0);

        let classic = replay_cadence(ModeConfig::CLASSIC, &cadence_240hz(240));
        let summary = summarize(&classic);
        // The gate answers YES on every frame in the loop shape
        // (the stale-answer fact above).
        assert!(classic.frames.iter().all(|f| f.present));
        assert_eq!(summary.presents, 240);
        assert_eq!(summary.held_frames, 0);
        // The arm difference: no recompose alpha, ever.
        assert!(classic.frames.iter().all(|f| f.camera_alpha.is_none()));
        // The classic VISIBLE refresh: exactly the tick frames
        // present a new image (59 of 240 — the 60 Hz content
        // cadence inside the 240 Hz present cadence).
        assert_eq!(
            classic
                .frames
                .iter()
                .filter(|f| f.ticks_executed > 0)
                .count(),
            59
        );
        assert_eq!(summary.zero_tick_frames, 181);
    }

    /// The recompose alpha cadence at 240 Hz (the modern arm's
    /// animation-cadence metric): within each inter-tick window the
    /// accumulator fraction sweeps up through ~0.25/0.5/0.75/1.0,
    /// never extrapolates past 1.0, and restarts low the frame
    /// after a tick fires. On the 60 Hz steady state (the original
    /// display class) the fraction pins to exactly 1.0 — the
    /// interpolated camera IS the parity camera there.
    #[test]
    fn alpha_cadence_sweeps_the_pending_tick_at_240hz() {
        let modern = replay_cadence(ModeConfig::MODERN, &cadence_240hz(12));
        let alphas: Vec<f32> = modern
            .frames
            .iter()
            .map(|f| f.camera_alpha.expect("modern presents with alpha"))
            .collect();
        // Frame 5 is the first tick frame (the clock's pinned
        // banking): frames 0..5 sweep the first pending tick.
        assert!((alphas[0] - 0.25).abs() < 1e-6);
        assert!((alphas[1] - 0.50).abs() < 1e-6);
        assert!((alphas[2] - 0.75).abs() < 1e-6);
        assert!((alphas[3] - 1.00).abs() < 1e-6);
        assert_eq!(modern.frames[4].ticks_executed, 1, "frame 5 fires the tick");
        // The tick frame's own present restarts the sweep low.
        assert!(alphas[4] < 0.5, "sweep restarts after the tick");
        // Never extrapolates past either endpoint.
        assert!(alphas.iter().all(|&a| (0.0..=1.0).contains(&a)));

        // 60 Hz steady state: the fraction is exactly 1.0 from the
        // first executed tick on.
        let hz60 = replay_cadence(ModeConfig::MODERN, &vec![16_666_666; 60]);
        for frame in &hz60.frames[1..] {
            assert_eq!(frame.camera_alpha, Some(1.0));
        }
    }

    /// A stall: the anti-spiral clamp executes 4 pumps, DISCARDS
    /// 596, and the replay records the drop — the verdict treats a
    /// non-zero dropped count as stutter EVEN with a p95 inside
    /// the budget (the clamp firing is the stutter).
    #[test]
    fn stall_clamps_and_the_drop_is_a_verdict_failure() {
        let mut deltas = vec![10_000_000_000u64];
        deltas.extend(cadence_240hz(239));
        let trace = replay_cadence(ModeConfig::MODERN, &deltas);
        assert_eq!(trace.frames[0].pumps_due, 4);
        assert_eq!(trace.frames[0].ticks_executed, 4);
        assert_eq!(trace.pumps_dropped, 596);
        let summary = summarize(&trace);
        // The p95 over 239 ideal + 1 stall frame stays at the
        // period (the stall is one sample of 240) — the drop is
        // the only failing signal.
        assert_eq!(summary.p95_frame_time_ns, NS_240HZ);
        let matched = profile_for(Some(PINNED_240HZ.id));
        assert_eq!(
            verdict(matched, &summary),
            PacingVerdict::Exceeded {
                p95_frame_time_ns: NS_240HZ,
                budget_ns: PINNED_240HZ.p95_frame_time_budget_ns,
                pumps_dropped: 596,
            }
        );
    }

    /// The p95 threshold itself: a trace whose p95 frame time
    /// exceeds the budget fails ONLY under a matched profile —
    /// the same summary under an unavailable profile is
    /// NoAttestation (never a false red).
    #[test]
    fn over_budget_p95_fails_only_on_the_pinned_profile() {
        let mut deltas = vec![6_000_000u64; 120];
        deltas.extend(cadence_240hz(120));
        let trace = replay_cadence(ModeConfig::MODERN, &deltas);
        let summary = summarize(&trace);
        assert_eq!(summary.p95_frame_time_ns, 6_000_000);
        assert!(summary.p95_frame_time_ns > PINNED_240HZ.p95_frame_time_budget_ns);
        assert_eq!(
            verdict(profile_for(Some(PINNED_240HZ.id)), &summary),
            PacingVerdict::Exceeded {
                p95_frame_time_ns: 6_000_000,
                budget_ns: PINNED_240HZ.p95_frame_time_budget_ns,
                pumps_dropped: 0,
            }
        );
        assert_eq!(
            verdict(profile_for(None), &summary),
            PacingVerdict::NoAttestation
        );
        assert_eq!(
            verdict(profile_for(Some("some-other-machine")), &summary),
            PacingVerdict::NoAttestation
        );
    }

    /// The healthy posture end-to-end: the ideal 240 Hz cadence
    /// under the matched profile ATTESTS.
    #[test]
    fn ideal_cadence_attests_on_the_pinned_profile() {
        let trace = replay_cadence(ModeConfig::MODERN, &cadence_240hz(240));
        assert_eq!(
            verdict(profile_for(Some(PINNED_240HZ.id)), &summarize(&trace)),
            PacingVerdict::Attested {
                p95_frame_time_ns: NS_240HZ,
                pumps_dropped: 0,
            }
        );
    }

    /// Profile matching is EXACT: only the pinned id matches;
    /// nothing declared, or any other identity, is unavailable.
    #[test]
    fn profile_matching_is_exact() {
        assert_eq!(profile_for(None), ProfileSelection::Unavailable);
        assert_eq!(profile_for(Some("")), ProfileSelection::Unavailable);
        assert_eq!(
            profile_for(Some("pinned-240hz-desk-v2")),
            ProfileSelection::Unavailable
        );
        assert_eq!(
            profile_for(Some(PINNED_240HZ.id)),
            ProfileSelection::Matched(&PINNED_240HZ)
        );
    }

    /// The pinned profile data is self-consistent: 240 Hz, period
    /// 4_166_666 ns, budget exactly 1.25 periods on the floor,
    /// bounded sample size.
    #[test]
    fn pinned_profile_data_is_self_consistent() {
        assert_eq!(PINNED_240HZ.display_refresh_hz, 240);
        assert_eq!(PINNED_240HZ.period_ns(), 4_166_666);
        assert_eq!(
            PINNED_240HZ.p95_frame_time_budget_ns, 5_208_333,
            "1.25 x period on the floor"
        );
        assert!((1..=10_000).contains(&PINNED_240HZ.sample_frames));
        assert_eq!(PROFILE_ENV, "BEDLAM_PACING_PROFILE");
    }

    /// Nearest-rank percentile, hand-checked: rank =
    /// ceil(p/100 * n) over the sorted values.
    #[test]
    fn nearest_rank_percentile() {
        let values = [10u64, 30, 20, 40, 50, 60, 70, 80, 90, 100];
        // n=10, p95: rank ceil(9.5)=10 -> sorted[9] = 100.
        assert_eq!(percentile_ns(&values, 95), 100);
        // p50: rank ceil(5)=5 -> sorted[4] = 50.
        assert_eq!(percentile_ns(&values, 50), 50);
        // p1: rank ceil(0.1)=1 -> sorted[0] = 10.
        assert_eq!(percentile_ns(&values, 1), 10);
        let single = [7u64];
        assert_eq!(percentile_ns(&single, 95), 7);
    }

    /// The benchmark report's three postures: unavailable means
    /// exit 0 with the explicit no-attestation note (the plan's
    /// posture, mechanically pinned); attested means exit 0;
    /// exceeded means exit 1.
    #[test]
    fn benchmark_report_postures() {
        let healthy = replay_cadence(ModeConfig::MODERN, &cadence_240hz(240));
        let skip = benchmark_report(ProfileSelection::Unavailable, &healthy);
        assert_eq!(skip.exit_code, 0);
        assert!(skip
            .lines
            .iter()
            .any(|l| l.contains("UNAVAILABLE") && l.contains(PINNED_240HZ.id)));
        assert!(skip
            .lines
            .iter()
            .any(|l| l.contains("no attestation") && l.contains("exit 0")));

        let attested = benchmark_report(profile_for(Some(PINNED_240HZ.id)), &healthy);
        assert_eq!(attested.exit_code, 0);
        assert!(attested.lines.iter().any(|l| l.contains("ATTESTED")));

        let mut deltas = vec![6_000_000u64; 120];
        deltas.extend(cadence_240hz(120));
        let stutter = replay_cadence(ModeConfig::MODERN, &deltas);
        let exceeded = benchmark_report(profile_for(Some(PINNED_240HZ.id)), &stutter);
        assert_eq!(exceeded.exit_code, 1);
        assert!(exceeded.lines.iter().any(|l| l.contains("EXCEEDED")));
    }

    /// The replay is trajectory-neutral across the pacing arms
    /// (the D203 property at the harness boundary): the SAME delta
    /// trace through both arms yields the identical executed-tick
    /// totals, sim tick count, state hash and scene hash — the
    /// cadence answers differ (present/alpha), the hashed
    /// trajectory does not.
    #[test]
    fn cadence_replay_never_touches_the_hashed_trajectory_across_arms() {
        let mut deltas = cadence_240hz(48);
        deltas.insert(9, 40_000_000);
        let modern = replay_cadence(ModeConfig::MODERN, &deltas);
        let classic = replay_cadence(ModeConfig::CLASSIC, &deltas);
        let (sm, sc) = (summarize(&modern), summarize(&classic));
        assert_eq!(sm.ticks_executed, sc.ticks_executed);
        assert_eq!(sm.pumps_due, sc.pumps_due);
        assert_eq!(sm.pumps_dropped, sc.pumps_dropped);
        // The arms differ in the PRESENT COMPOSITION only (the
        // alpha cadence): same tick totals, same pump totals, same
        // dropped pumps — and the modern arm interpolates while
        // the classic arm never does.
        assert!(modern.frames.iter().any(|f| f.camera_alpha.is_some()));
        assert!(classic.frames.iter().all(|f| f.camera_alpha.is_none()));
        assert_eq!(
            hashed_endpoints(ModeConfig::MODERN, &deltas),
            hashed_endpoints(ModeConfig::CLASSIC, &deltas)
        );
    }

    /// The hashed endpoints of a replayed script (re-replayed with
    /// the driver kept alive — GameHost is not Clone by design).
    fn hashed_endpoints(
        mode: ModeConfig,
        deltas: &[u64],
    ) -> (
        bedlam_core::time::Tick,
        bedlam_core::hash::StateHash,
        bedlam_core::hash::StateHash,
    ) {
        let mut driver = CadenceDriver::new(mode);
        for &delta in deltas {
            driver.frame(delta);
        }
        let host = driver.host();
        (
            host.driver().sim().tick_index(),
            host.driver().sim().state_hash(),
            host.scene_hash(),
        )
    }

    /// The replay is deterministic: the same trace replays
    /// bit-identically (the harness itself adds no nondeterminism
    /// — every input is the committed delta list).
    #[test]
    fn replay_is_deterministic() {
        let mut deltas = cadence_240hz(24);
        deltas.insert(3, 25_000_000);
        let a = replay_cadence(ModeConfig::MODERN, &deltas);
        let b = replay_cadence(ModeConfig::MODERN, &deltas);
        assert_eq!(a, b);
    }
}
