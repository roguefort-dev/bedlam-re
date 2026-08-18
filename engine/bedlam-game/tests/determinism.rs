//! FSM determinism gates (DESIGN-GAME sec 9/10, deliverable c): the
//! same WALL-TIME input script yields the identical scene-state hash
//! chain at 15 / 60 / 240 Hz host frame rates, a pure-FSM replay
//! reproduces the chain, and a different script diverges.
//!
//! The script phases are aligned to 16-sub-tick boundaries (4 sim
//! ticks) so the coarsest host (15 Hz = 16 sub-ticks per frame) can
//! represent every phase change exactly; finer hosts subdivide the
//! same phase without ever splitting it (D12/D17 quantization + the
//! per-tick action derivation D26 are what make this exact).

use bedlam_core::input::InputFrame;
use bedlam_game::host::GameHost;
use bedlam_game::{GameConfig, SceneFsm};
use bedlam_render::Vga6;

/// Deterministic per-PHASE mouse script (LCG over the phase index; no
/// RNG crate). A phase lasts 4 sim ticks = 16 sub-ticks = 1/15 s.
fn phase_mouse(phase: u64) -> u8 {
    let x = phase
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    match (x >> 33) % 6 {
        0 | 4 => 0, // idle
        1 => 1,     // left held for the whole phase (press edge at entry)
        2 => 0,
        3 => 2, // right held
        _ => 1,
    }
}

/// The wall-time input at tick t (constant inside each 4-tick phase).
fn input_at_tick(t: u64, salt: u64) -> InputFrame {
    InputFrame {
        buttons: 0,
        mouse_dx: 0,
        mouse_dy: 0,
        mouse_buttons: phase_mouse(t / 4 + salt),
    }
}

fn palette() -> [Vga6; 256] {
    [[0, 0, 0]; 256]
}

/// Drive one host at a given frame rate until `ticks` sim ticks have
/// executed; fold the scene hash on the 20-tick grid into a chain.
/// 20 = lcm(4-tick phase, the 10-tick sampling urge): every host rate
/// in these tests advances a whole 1/2/4 ticks per frame, so each lands
/// exactly on the grid - a coarser sampling would read some hosts
/// mid-frame (tick 12 vs 10) and false-diverge.
fn run_chain(subticks_per_frame: u32, ticks: u64, salt: u64) -> Vec<u64> {
    let mut host = GameHost::new(
        &GameConfig::default(),
        &bedlam_core::sim::SimConfig::default(),
        palette(),
    );
    let mut chain = Vec::new();
    let mut tick_cursor = 0u64;
    loop {
        let frame_input = input_at_tick(tick_cursor, salt);
        let executed = host.pump_frame(subticks_per_frame, &frame_input);
        tick_cursor += executed as u64;
        // Sample only on frames that EXECUTED ticks: a host faster than
        // 60 Hz (or a banked remainder frame) executes 0 ticks and the
        // cursor rests on the same multiple of 20 (0 included) - pushing
        // there would duplicate the checkpoint hash and false-diverge.
        if executed > 0 && tick_cursor.is_multiple_of(20) {
            chain.push(host.scene_hash().0);
        }
        if tick_cursor >= ticks {
            break;
        }
    }
    chain
}

#[test]
fn identical_scene_hash_chain_at_15_60_240hz() {
    // 15 Hz = 16 sub-ticks/frame, 60 Hz = 4, 240 Hz = 1.
    let a = run_chain(16, 600, 0);
    let b = run_chain(4, 600, 0);
    let c = run_chain(1, 600, 0);
    assert_eq!(a.len(), 30);
    assert_eq!(b, a, "60 Hz host diverged from the 15 Hz host");
    assert_eq!(c, a, "240 Hz host diverged from the 15 Hz host");
}

#[test]
fn odd_host_rates_quantize_to_the_same_chain() {
    // 30 Hz (8 sub-ticks) and 120 Hz (2) pay the same sub-tick total
    // per phase pair; the banking remainder cannot split a phase.
    let a = run_chain(8, 300, 0);
    let b = run_chain(2, 300, 0);
    assert_eq!(a.len(), 15);
    assert_eq!(b, a);
}

#[test]
fn a_different_script_diverges() {
    let a = run_chain(4, 200, 0);
    let b = run_chain(4, 200, 1); // one phase of shift = different script
    assert_ne!(a, b, "salted script must diverge");
}

#[test]
fn pure_fsm_replay_reproduces_the_chain() {
    // The host chain is a pure function of the tick input sequence:
    // driving the bare SceneFsm with the same per-tick inputs must
    // land on the identical hash at every checkpoint.
    let mut fsm = SceneFsm::new();
    let mut expected = Vec::new();
    for t in 0..600 {
        fsm.tick(&input_at_tick(t, 0));
        if (t + 1) % 20 == 0 {
            expected.push(fsm.scene_hash().0);
        }
    }
    let mut replay = SceneFsm::new();
    let mut got = Vec::new();
    for t in 0..600 {
        replay.tick(&input_at_tick(t, 0));
        if (t + 1) % 20 == 0 {
            got.push(replay.scene_hash().0);
        }
    }
    assert_eq!(got, expected, "replay determinism");
    assert_eq!(run_chain(4, 600, 0), expected, "host == pure FSM");
}
