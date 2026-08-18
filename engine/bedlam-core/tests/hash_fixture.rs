//! Committed per-tick state-hash fixture - the PLAN sec 7 gate
//! ("cross-OS per-tick hash equality is a CI job from the first playable
//! tick"), applied from tick 0 of the skeleton so the pin exists before
//! any gameplay lands on top.
//!
//! A fixed input script (pure integer arithmetic on the tick index - no
//! PRNG, no ambient anything) drives a fixed config for FIXED_TICKS
//! ticks; the per-tick StateHash sequence is asserted against constants
//! committed below. The test runs in the ordinary cargo test matrix
//! (ubuntu + windows in ci.yml), so any OS-, rustc- or layout-dependent
//! drift in hashed sim state fails loud here, per tick.
//!
//! Regeneration contract: every constant below is a paste of the output
//! of print_fixture (cargo test -p bedlam-core --test hash_fixture --
//! --ignored --nocapture) after an INTENTIONAL hashed-state change (for
//! example appending a field in P4+, which must also bump
//! FORMAT_VERSION). Unintended changes must FAIL, never be papered
//! over: investigate before regenerating.
//!
//! The script exercises every InputFrame field (only buttons bit 0 has
//! a placeholder sim effect today) plus a fade-satellite window
//! (fading armed after tick 100, disarmed after tick 200), so the 300Hz
//! microstep satellites are inside the pinned sequence.

use bedlam_core::hash::{Fnv1a64, StateHash};
use bedlam_core::input::InputFrame;
use bedlam_core::sim::{Sim, SimConfig};
use bedlam_core::time::TimeBase;

/// Fixture seed. Provenance nod: the original game seeds its two RNGs
/// 123456/234567 (EXW 004ede48/004ede4c, B2 0x11ef1c/0x11ef18) - this
/// fixture uses the first of those. The parity harness later pins real
/// per-mission seeds; this constant is only about being fixed forever.
const SEED: u64 = 123_456;

/// Script length: 600 ticks = 10 game-seconds at the nominal 60Hz base.
const FIXED_TICKS: u64 = 600;

/// Fading is armed (Sim::set_fading(true)) once FADE_ON_AFTER ticks
/// have completed, and disarmed after FADE_OFF_AFTER.
const FADE_ON_AFTER: u64 = 100;
const FADE_OFF_AFTER: u64 = 200;

/// (tick, state hash) pins. Ticks chosen to localize a first divergence:
/// 0 initial state, 1 first service event (microstep 3), 2 first
/// fade-phase microstep (6, fade still disarmed), 3 service phase wrap,
/// 5 first palette cycle (microstep 24), 24 pal phase wrap, 60 one
/// game-second, 100/101 fade window opens (first fade step fires at
/// microstep 504 inside tick 101), 200/201 fade window closes, 300, 600
/// final state.
const EXPECTED_MILESTONES: &[(u64, u64)] = &[
    (0, 0x0d6947f680d24ff8),
    (1, 0xe8380f65a0d000cf),
    (2, 0xdff65b093ba6ea72),
    (3, 0x45f9eee0585a9ba3),
    (5, 0x0417af27b3eb6d5f),
    (24, 0x35361898139c0fe7),
    (60, 0x4efba3aae00c7efa),
    (100, 0x70073f5cb24626c4),
    (101, 0x409af364361f8632),
    (200, 0x9de2e5c1cdbcfe7e),
    (201, 0xdf0c8821b6090477),
    (300, 0x7a5ca587ffafc6d3),
    (600, 0x36cb1c274412a043),
];

/// FNV-1a 64 over every per-tick state hash in order (ticks 0..=600,
/// tick 0 = pre-tick initial state). One constant pins the entire
/// 601-entry sequence; the milestones above localize a divergence.
const EXPECTED_CHAIN: u64 = 0x760d221bec3b3b99;

/// The fixed input script: tick t (1-based, the tick about to run) maps
/// to a deterministic InputFrame. Values are arbitrary; being fixed and
/// reproducible everywhere is the entire point.
fn script_input(tick: u64) -> InputFrame {
    let t = tick as u32;
    InputFrame {
        buttons: t.wrapping_mul(0x9E37_79B9) ^ (t >> 3),
        mouse_dx: ((tick % 97) as i16) - 48,
        mouse_dy: (((tick / 3) % 53) as i16) - 26,
        mouse_buttons: ((tick >> 1) & 0x3) as u8,
    }
}

/// Run the fixture script; returns the per-tick state hashes, tick 0
/// (initial) first, so entry i is the hash after i ticks.
fn run_script() -> Vec<u64> {
    let config = SimConfig {
        seed: SEED,
        time_base: TimeBase::NOMINAL,
    };
    let mut sim = Sim::new(&config);
    let mut hashes = Vec::with_capacity(FIXED_TICKS as usize + 1);
    hashes.push(sim.state_hash().0);
    for t in 1..=FIXED_TICKS {
        if t == FADE_ON_AFTER + 1 {
            sim.set_fading(true);
        }
        if t == FADE_OFF_AFTER + 1 {
            sim.set_fading(false);
        }
        sim.tick(&script_input(t));
        hashes.push(sim.state_hash().0);
    }
    hashes
}

/// FNV-1a 64 over the hash sequence - same construction the crate uses
/// everywhere, so the chain is byte-reproducible on every OS/toolchain.
fn sequence_chain(hashes: &[u64]) -> u64 {
    let mut h = Fnv1a64::new();
    for &hash in hashes {
        h.write_u64(hash);
    }
    h.finish()
}

#[test]
fn per_tick_hash_fixture() {
    let hashes = run_script();
    assert_eq!(hashes.len(), FIXED_TICKS as usize + 1);
    for &(tick, expected) in EXPECTED_MILESTONES {
        assert_eq!(
            hashes[tick as usize], expected,
            "state hash diverged at tick {tick} (cross-OS/toolchain drift or hashed-state layout change)"
        );
    }
    let chain = sequence_chain(&hashes);
    assert_eq!(
        chain, EXPECTED_CHAIN,
        "per-tick hash sequence diverged from the committed chain"
    );
}

#[test]
#[ignore = "generator: prints the constants to paste after INTENTIONAL changes"]
fn print_fixture() {
    let hashes = run_script();
    for &(tick, _) in EXPECTED_MILESTONES {
        println!("    ({tick}, 0x{:016x}),", hashes[tick as usize]);
    }
    println!(
        "const EXPECTED_CHAIN: u64 = 0x{:016x};",
        sequence_chain(&hashes)
    );
    let _: Option<StateHash> = None; // keep the import honest in generator mode
}
