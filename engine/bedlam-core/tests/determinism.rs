//! Determinism integration tests (docs/PLAN.md sec 7 charter, P3 gate).
//!
//! These are the acceptance tests for the bedlam-core skeleton: identical
//! seeds + inputs => identical per-tick hash streams; replays and snapshots
//! round-trip bit-exactly; tampering is detected; the D16 60 Hz time base
//! holds; and the D17 timing model is proven — same wall-time input script
//! => identical SIM hash at 15/60/240 Hz host, with frame-rate-driven
//! state excluded from the hash.

use bedlam_core::frame::SimDriver;
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

/// D17 acceptance: the same WALL-TIME input script drives three hosts at
/// 15/60/240 Hz; the sim hashes must be identical at the 500 ms and 1 s
/// checkpoints.
///
/// The script is keyed to wall time (constant input for the first 500 ms
/// = 120 sub-ticks = 30 ticks, then a second constant input to 1 s = 240
/// sub-ticks = 60 ticks), NOT to host-frame index — an index-keyed script
/// would feed different inputs at different rates and prove nothing.
///
/// 15 Hz cannot land 500 ms on whole frames (7.5 frames), so that host
/// quantizes its boundary frame to a half frame of 8 sub-ticks; the point
/// under test is that this pacing difference never reaches the sim, which
/// only sees whole 60 Hz ticks + the per-tick input sequence.
#[test]
fn same_script_same_sim_hash_at_15_60_240hz() {
    let config = SimConfig {
        seed: 0x00D1_7600,
        time_base: TimeBase::NOMINAL,
    };
    let phase1 = InputFrame {
        buttons: 1, // bit 0 held: the placeholder actor advances
        mouse_dx: 3,
        mouse_dy: -2,
        mouse_buttons: 1,
    };
    let phase2 = InputFrame {
        buttons: 0,
        mouse_dx: -7,
        mouse_dy: 9,
        mouse_buttons: 2,
    };

    fn drive_half_second(
        driver: &mut SimDriver,
        origin: u32,
        dts: &[u32],
        phase1: InputFrame,
        phase2: InputFrame,
    ) {
        let mut start = origin;
        for &dt in dts {
            // Input keyed by ABSOLUTE frame start time: < 120 sub-ticks
            // (= 500 ms on the 240 Hz grid) is phase 1, after that phase 2.
            let input = if start < 120 { phase1 } else { phase2 };
            driver.advance(dt, &input);
            start += dt;
        }
        assert_eq!(start, origin + 120, "must land the checkpoint exactly");
    }

    let mut a = SimDriver::new(&config); // 15 Hz host
    let mut b = SimDriver::new(&config); // 60 Hz host
    let mut c = SimDriver::new(&config); // 240 Hz host

    let a_dts = [16, 16, 16, 16, 16, 16, 16, 8]; // 7 whole frames + half frame
    let b_dts = [4; 30];
    let c_dts = [1; 120];

    drive_half_second(&mut a, 0, &a_dts, phase1, phase2);
    drive_half_second(&mut b, 0, &b_dts, phase1, phase2);
    drive_half_second(&mut c, 0, &c_dts, phase1, phase2);

    // Checkpoint 1: 500 ms = 30 ticks, identical hashes.
    assert_eq!(a.sim().tick_index(), 30);
    assert_eq!(b.sim().tick_index(), 30);
    assert_eq!(c.sim().tick_index(), 30);
    assert_eq!(
        a.sim().state_hash(),
        b.sim().state_hash(),
        "15 Hz vs 60 Hz diverged at 500 ms"
    );
    assert_eq!(
        b.sim().state_hash(),
        c.sim().state_hash(),
        "60 Hz vs 240 Hz diverged at 500 ms"
    );
    // Phase 1 actually reached the sim: bit 0 held for 30 ticks.
    assert_eq!(a.sim().actor().0, 30);

    drive_half_second(&mut a, 120, &a_dts, phase1, phase2);
    drive_half_second(&mut b, 120, &b_dts, phase1, phase2);
    drive_half_second(&mut c, 120, &c_dts, phase1, phase2);

    // Checkpoint 2: 1 s = 60 ticks, still identical; phase 2 (bit 0 up)
    // advanced the actor no further.
    assert_eq!(a.sim().tick_index(), 60);
    assert_eq!(b.sim().tick_index(), 60);
    assert_eq!(c.sim().tick_index(), 60);
    assert_eq!(a.sim().state_hash(), b.sim().state_hash());
    assert_eq!(b.sim().state_hash(), c.sim().state_hash());
    assert_eq!(a.sim().actor().0, 30);
}

/// D17 (c): satellite clocks as exact integer substeps of the 60 Hz tick.
#[test]
fn satellite_substep_rates() {
    // 100 Hz service = 5 events per 3 ticks.
    let mut sim = Sim::new(&SimConfig::default());
    for _ in 0..3 {
        sim.tick(&InputFrame::default());
    }
    assert_eq!(sim.service_ticks(), 5);
    assert_eq!(sim.service_phase(), 0);

    // 12.5 Hz palette cycle = 5 cycles per 24 ticks (and 40 service
    // events over the same span: 24 * 5/3).
    let mut sim = Sim::new(&SimConfig::default());
    for _ in 0..24 {
        sim.tick(&InputFrame::default());
    }
    assert_eq!(sim.pal_cycles(), 5);
    assert_eq!(sim.pal_phase(), 0);
    assert_eq!(sim.service_ticks(), 40);

    // 50 Hz fade stepper = 5 steps per 6 ticks, but only while fading.
    let mut fading = Sim::new(&SimConfig::default());
    fading.set_fading(true);
    for _ in 0..6 {
        fading.tick(&InputFrame::default());
    }
    assert_eq!(fading.fade_steps(), 5);
    assert_eq!(fading.fade_phase(), 0);
    assert_eq!(fading.service_ticks(), 10); // satellites coexist

    let mut idle = Sim::new(&SimConfig::default());
    for _ in 0..6 {
        idle.tick(&InputFrame::default());
    }
    assert_eq!(idle.fade_steps(), 0, "not fading: stepper stays idle");
    assert!(!idle.fading());
}

/// D17 (b): frame-rate-driven state is excluded from the sim hash. Mouse
/// deltas move the cursor (per-frame bucket) but never touch the sim; only
/// buttons reach the sim placeholder in this skeleton.
#[test]
fn frame_state_excluded_from_hash() {
    let config = SimConfig::default();
    let mut a = SimDriver::new(&config);
    let mut b = SimDriver::new(&config);
    let moving = InputFrame {
        mouse_dx: 100,
        ..InputFrame::default()
    };
    let still = InputFrame::default();

    // One tick each (4 sub-ticks): cursors diverge, hashes do not.
    a.advance(4, &moving);
    b.advance(4, &still);
    assert_ne!(a.frame().cursor_x, b.frame().cursor_x);
    assert_eq!(a.sim().state_hash(), b.sim().state_hash());

    // Six more moving frames: A clamps at the 640x480 edge, B stays put,
    // and the hashes STILL match — dt-driven data never enters the hash.
    for _ in 0..6 {
        a.advance(4, &moving);
        b.advance(4, &still);
    }
    assert_eq!(a.frame().cursor_x, 639, "cursor clamps into game space");
    assert_eq!(b.frame().cursor_x, 0);
    assert_eq!(a.frame().cursor_y, 0);
    assert_eq!(b.frame().cursor_y, 0);
    assert_eq!(a.sim().state_hash(), b.sim().state_hash());
    assert_eq!(a.sim().tick_index(), 7);
    assert_eq!(b.sim().tick_index(), 7);
}

/// D17 (a): the accumulator banks remainders; a tick fires exactly when
/// the 4th sub-tick of the pair is banked.
#[test]
fn accumulator_remainder_banks() {
    let mut driver = SimDriver::new(&SimConfig::default());
    let input = InputFrame::default();

    // 7 sub-ticks in ones (a 240 Hz pattern): the tick fires at sub-tick
    // 4, the remaining 3 rest in the accumulator.
    let mut executed = 0;
    for i in 0..7 {
        executed += driver.advance(1, &input);
        assert_eq!(
            driver.sim().tick_index(),
            u64::from(executed),
            "after sub-tick {}",
            i + 1
        );
    }
    assert_eq!(executed, 1);
    assert_eq!(driver.sim().tick_index(), 1);

    // One more sub-tick completes the banked 3 + 1 = 4: the second tick
    // fires (which is only possible if the remainder was banked).
    assert_eq!(driver.advance(1, &input), 1);
    assert_eq!(driver.sim().tick_index(), 2);
}
