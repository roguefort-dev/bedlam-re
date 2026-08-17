//! Determinism integration tests (docs/PLAN.md sec 7 charter, P3 gate).
//!
//! These are the acceptance tests for the bedlam-core skeleton: identical
//! seeds + inputs => identical per-tick hash streams; replays and snapshots
//! round-trip bit-exactly; tampering is detected; the D16 60 Hz time base
//! holds.

use bedlam_core::input::InputFrame;
use bedlam_core::replay::Replay;
use bedlam_core::rng::Pcg32;
use bedlam_core::sim::{Sim, SimConfig};
use bedlam_core::time::{TimeBase, NOMINAL_TICK_HZ};
use bedlam_core::{CoreError, FORMAT_VERSION};

/// Deterministic input generator: the test harness itself uses the crate
/// PRNG on its own stream, so the "random" inputs are reproducible.
fn random_input(rng: &mut Pcg32) -> InputFrame {
    InputFrame {
        buttons: rng.next_u32(),
        mouse_dx: rng.next_u32() as i16,
        mouse_dy: rng.next_u32() as i16,
        mouse_buttons: rng.next_u32() as u8,
    }
}

#[test]
fn same_seed_same_inputs_same_hash_stream() {
    let config = SimConfig {
        seed: 0x00C0_FFEE,
        time_base: TimeBase::NOMINAL,
    };
    let mut a = Sim::new(&config);
    let mut b = Sim::new(&config);
    let mut gen = Pcg32::new(123_456, 999); // generator stream, unrelated to sim
    let mut previous: Option<bedlam_core::hash::StateHash> = None;
    let mut nonconstant = false;
    for _ in 0..500 {
        let input = random_input(&mut gen);
        a.tick(&input);
        b.tick(&input);
        let (ha, hb) = (a.state_hash(), b.state_hash());
        assert_eq!(ha, hb, "hash streams diverged at tick {}", a.tick_index());
        if previous.is_some_and(|prev| prev != ha) {
            nonconstant = true;
        }
        previous = Some(ha);
    }
    assert!(nonconstant, "hash stream was constant");
}

#[test]
fn different_seed_diverges() {
    let mut inputs = Vec::with_capacity(100);
    let mut gen = Pcg32::new(42, 4242);
    for _ in 0..100 {
        inputs.push(random_input(&mut gen));
    }
    let config_a = SimConfig {
        seed: 1,
        time_base: TimeBase::NOMINAL,
    };
    let config_b = SimConfig {
        seed: 2,
        time_base: TimeBase::NOMINAL,
    };
    let mut a = Sim::new(&config_a);
    let mut b = Sim::new(&config_b);
    // Initial (tick 0) hashes differ...
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "different seeds produced identical initial state"
    );
    let mut diverged = false;
    for input in &inputs {
        a.tick(input);
        b.tick(input);
        if a.state_hash() != b.state_hash() {
            diverged = true;
        }
    }
    assert!(diverged, "streams never diverged across 100 ticks");
}

#[test]
fn replay_round_trip_is_bit_exact() {
    let config = SimConfig::default();
    let mut sim = Sim::new(&config);
    let mut gen = Pcg32::new(777, 5);
    let mut inputs = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let input = random_input(&mut gen);
        inputs.push(input);
        sim.tick(&input);
    }
    let final_hash = sim.state_hash();

    let replay = Replay {
        version: FORMAT_VERSION,
        tick_hz: config.time_base.tick_hz,
        seed: config.seed,
        initial_state_hash: sim.initial_state_hash(),
        inputs,
    };
    let parsed = Replay::parse(&replay.to_bytes()).expect("valid replay must parse");
    assert_eq!(parsed.version, replay.version);
    assert_eq!(parsed.tick_hz, replay.tick_hz);
    assert_eq!(parsed.seed, replay.seed);
    assert_eq!(parsed.initial_state_hash, replay.initial_state_hash);
    assert_eq!(parsed.inputs, replay.inputs);
    assert_eq!(parsed.inputs.len(), 1000);

    // Re-simulate from the parsed recording: bit-exact final hash.
    let replay_config = SimConfig {
        seed: parsed.seed,
        time_base: TimeBase::new(parsed.tick_hz).unwrap(),
    };
    let mut replayed = Sim::new(&replay_config);
    assert_eq!(replayed.initial_state_hash(), parsed.initial_state_hash);
    for frame in &parsed.inputs {
        replayed.tick(frame);
    }
    assert_eq!(replayed.state_hash(), final_hash);
}

#[test]
fn snapshot_restore_continues_identically() {
    let config = SimConfig {
        seed: 0x5EED_5EED,
        time_base: TimeBase::NOMINAL,
    };
    let mut sim = Sim::new(&config);
    let mut gen = Pcg32::new(31337, 60);
    let mut inputs = Vec::with_capacity(700);
    for _ in 0..700 {
        inputs.push(random_input(&mut gen));
    }
    for input in &inputs[..300] {
        sim.tick(input);
    }
    assert_eq!(sim.tick_index(), 300);

    let snapshot_bytes = sim.snapshot().to_bytes();
    let mut restored = Sim::restore(&snapshot_bytes, &config).expect("snapshot must restore");
    assert_eq!(restored.tick_index(), 300);

    for (i, input) in inputs[300..].iter().enumerate() {
        sim.tick(input);
        restored.tick(input);
        assert_eq!(
            sim.state_hash(),
            restored.state_hash(),
            "restored sim diverged at tick {}",
            301 + i
        );
    }
    assert_eq!(sim.tick_index(), 700);
    assert_eq!(restored.tick_index(), 700);
}

#[test]
fn tampered_snapshot_detected() {
    let config = SimConfig::default();
    let mut sim = Sim::new(&config);
    let mut gen = Pcg32::new(9, 10);
    for _ in 0..250 {
        let input = random_input(&mut gen);
        sim.tick(&input);
    }
    let bytes = sim.snapshot().to_bytes();
    // Header is 32 bytes; the state region starts right after. Flip a byte
    // well inside it (rng stream field) — not magic/version/length fields.
    let mut tampered = bytes.clone();
    tampered[40] ^= 0x80;
    match Sim::restore(&tampered, &config) {
        Err(CoreError::HashMismatch { .. }) => {}
        other => panic!("expected HashMismatch, got {other:?}"),
    }
    // Untouched bytes still restore fine right next to the tampered copy.
    assert!(Sim::restore(&bytes, &config).is_ok());
}

#[test]
fn nominal_time_base_is_60hz() {
    // D16 canary: original is vsync-present-paced; parity sim is fixed 60 Hz.
    assert_eq!(NOMINAL_TICK_HZ, 60);
    assert_eq!(TimeBase::default().tick_hz, 60);
    assert_eq!(TimeBase::NOMINAL.tick_hz, 60);
    assert_eq!(SimConfig::default().time_base.tick_hz, 60);
}

#[test]
fn zero_tick_hz_rejected() {
    assert!(matches!(TimeBase::new(0), Err(CoreError::BadTickHz(0))));
}
