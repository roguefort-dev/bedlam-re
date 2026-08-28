//! The bounded frame-pacing MEASUREMENT binary (PLAN §6 closing QoL
//! instrument, D219): "An automated scheduled CI benchmark checks
//! 240Hz frame pacing against a pinned hardware profile and
//! thresholds; an unavailable profile creates no task and only
//! excludes that platform attestation."
//!
//! Runs the SAME pacing path as the window present loop
//! (`bedlam_shell::pacing::CadenceDriver` — clock advance -> due
//! pumps -> present gate -> alpha -> recompose) against a wall
//! clock, as a 240 Hz-CADENCE PROXY: each iteration sleeps to the
//! next display-period boundary, then measures the pacing path,
//! recording the inter-frame delta exactly as the loop's
//! `about_to_wait` would. The display's own vsync wait is the one
//! piece a surface-less benchmark cannot include — the profile's
//! p95 budget (1.25 display periods) covers the pacing path +
//! wake jitter, and the real 240 Hz attestation this instrument
//! feeds runs on the pinned machine (see
//! `.github/workflows/frame-pacing.yml`).
//!
//! PROFILE-GATED, fail-closed to skip-clean: the benchmark only
//! measures on a machine that declares the pinned hardware profile
//! via `BEDLAM_PACING_PROFILE`; ANY other machine (every CI runner
//! included) prints an explicit no-attestation note and exits 0 —
//! never a false red, never a task. Exit 1 exists ONLY for a
//! matched machine whose measured cadence exceeded the pinned
//! thresholds.
//!
//! Bounded: `PacingProfile::sample_frames` iterations (2400 = 10 s
//! of 240 Hz cadence), no window, no GPU, no corpus.

use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bedlam_core::mode::ModeConfig;

use bedlam_shell::pacing::{
    benchmark_report, profile_for, CadenceDriver, PacingProfile, PacingTrace, ProfileSelection,
    PROFILE_ENV,
};

fn main() -> ExitCode {
    let declared = std::env::var(PROFILE_ENV).ok();
    let selection = profile_for(declared.as_deref());
    let trace = match &selection {
        // The ONLY measurement path: a matched pinned profile.
        ProfileSelection::Matched(profile) => measure(profile),
        // The unavailable-profile posture: nothing to measure —
        // skip clean (the report carries the no-attestation note).
        ProfileSelection::Unavailable => CadenceDriver::new(ModeConfig::MODERN).into_trace(),
    };
    let outcome = benchmark_report(selection, &trace);
    for line in &outcome.lines {
        println!("{line}");
    }
    ExitCode::from(u8::try_from(outcome.exit_code).unwrap_or(1))
}

/// The bounded 240 Hz-cadence proxy measurement: self-paced to the
/// pinned display period, measuring the pacing path's frame cost
/// (the driver consumes each MEASURED delta, exactly as the loop
/// does). Bounded by `sample_frames` iterations.
fn measure(profile: &PacingProfile) -> PacingTrace {
    let period = Duration::from_nanos(profile.period_ns());
    let mut driver = CadenceDriver::new(ModeConfig::MODERN);
    let start = Instant::now();
    let mut frame_begin = start;
    for frame in 0..profile.sample_frames {
        // Sleep to the next display-period boundary (the 240 Hz
        // cadence proxy; a machine whose pacing path already
        // overruns the period skips the sleep and the overrun
        // lands in the measured deltas).
        let target = start + period * (frame + 1);
        let now = Instant::now();
        if now < target {
            sleep(target - now);
        }
        let begin = Instant::now();
        let delta_ns =
            u64::try_from(begin.duration_since(frame_begin).as_nanos()).unwrap_or(u64::MAX);
        frame_begin = begin;
        // One present-loop-shaped frame over the measured delta.
        driver.frame(delta_ns);
    }
    driver.into_trace()
}
