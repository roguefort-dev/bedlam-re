//! S0-13 strict-coverage oracle for the RNG pair + the dither-noise
//! bank (RE-EXW-SIM §7j.65, D155): `rng-state-a`, `rng-state-b`,
//! `static-dither-noise`.
//!
//! Two halves, the static-oracle convention (S0-07..S0-12 pattern):
//!
//! 1. ORIGINAL-SIDE TRANSCRIPTION (corpus-free): the §7j.65
//!    instruction-level decode of `ghidra-project/exw-text-objdump.txt`
//!    hand-transcribed below — the RandA/RandB step functions
//!    (0x402975/0x4029b6, identical modulo the state cell), the seed
//!    plants (GameMain boot both 0x41c0cd/0x41c0d3, MissionShell
//!    reseed A-only 0x447728), the dither boot fill (0x447b13..0x447b3a)
//!    and per-frame churn (0x448147..0x448195). The transcription is
//!    the coverage: it pins the ORIGINAL's init/evolution semantics in
//!    code, independently of the engine.
//! 2. E-SIDE CLASSIFICATION (corpus-gated): these rows are the charter
//!    T3 statistical class — the E values are PCG32 STAND-IN streams
//!    (never the original LCG), the differ compares DRAW COUNTS and
//!    never bits (`Class::AcceptedT3`), and the noise bank is
//!    presentation-half (D17) that never enters the dump. The E half
//!    pins exactly those seam facts — row presence/form, the
//!    `seed=0x1e240` stand-in pin, stream liveness, and the
//!    deliberately-absent dither row — and pointedly does NOT assert
//!    bit equality with the transcribed chains (Rust determinism is
//!    not the oracle for a T3 row).
//!
//! This test lives in bedlam-game because the E half is the canonical
//! harness (`parity_harness/canonical.rs`, re-exported the
//! canonical_dump_gate way).

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::{Path, PathBuf};

use canonical::run_canonical;
use diffharness::dump::{decode_dump, Channel};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR").is_dir()
}

// ---------------------------------------------------------------------
// 1. Original-side transcription (§7j.65) — the coverage half
// ---------------------------------------------------------------------

/// §7j.65/B — GameMain plants BOTH seeds at boot (0x41c0ad `mov
/// eax,0x1e240` → A, 0x41c0a8 `mov edi,0x39447` → B).
const ORIGINAL_SEED_A: u32 = 123456; // 0x1E240 (stored 0x41c0d3)
const ORIGINAL_SEED_B: u32 = 234567; // 0x39447 (stored 0x41c0cd)

/// The RandA/RandB step function, transcribed
/// instruction-for-instruction (§7j.65/A; the two EXW functions
/// 0x402975..0x4029b5 / 0x4029b6..0x4029f6 differ ONLY in their state
/// cell — A 0x4ede48, B 0x4ede4c).
struct OriginalLcg {
    state: u32,
}

impl OriginalLcg {
    fn new(seed: u32) -> OriginalLcg {
        OriginalLcg { state: seed }
    }

    /// One draw. Returns the NEW HIGH WORD — the EXW return value
    /// (eax: `movzx` zeroed the top half at entry and every later
    /// operation is 16-bit, so eax = the new hi, a u16; consumers
    /// mask it: `test al,3`, `and eax,0x1ff`).
    fn step(&mut self) -> u16 {
        let s = self.state;
        let lo = (s & 0xFFFF) as u16;
        let hi = (s >> 16) as u16;
        // The byte shuffle 0x402987..0x40298f: the 40-bit chain
        // dl:ax:bx := S << 8 (dl = hi15..8; ax = hi7..0 | lo15..8;
        // bx = (lo7..0) << 8). `xor bl,bl` also zeroes CF.
        let dl: u8 = (hi >> 8) as u8;
        let mut ax: u16 = (hi << 8) | (lo >> 8);
        let mut bx: u16 = lo << 8;
        // rcr dl,1 / rcr ax,1 / rcr bx,1 (0x402991..0x402996): the
        // 40-bit chain rotated right exactly 1 through CF. The
        // incoming CF is 0 (the xor) and chain bit 0 is 0 (bl was
        // zeroed), so this is chain := chain >> 1.
        let carry_dl = dl & 1 != 0;
        let dl = dl >> 1; // CF-in was 0
        let carry_ax = ax & 1 != 0;
        ax = (ax >> 1) | (u16::from(carry_dl) << 15);
        let carry_bx = bx & 1 != 0;
        bx = (bx >> 1) | (u16::from(carry_ax) << 15);
        // dl' = S >> 25 is DISCARDED — never read again (this is why
        // the closed form is a SHIFT-7, not a wrap rotate) — and the
        // outgoing carry_bx is the final CF, also never consumed.
        let _ = (dl, carry_bx);
        // add bx,di / adc ax,si (0x402999..0x40299c) — the first
        // add/adc pair adds back the ORIGINAL state (di=lo, si=hi
        // saved at 0x402983/0x402985).
        let (b1, c1) = bx.overflowing_add(lo);
        let a1 = ax.wrapping_add(hi).wrapping_add(u16::from(c1));
        // add bx,0x62e9 / adc ax,0x3619 (0x40299f..0x4029a4) — the
        // second pair adds the constant 0x361962E9 (the middle CF
        // from the first adc is destroyed by the plain add).
        let (b2, c3) = b1.overflowing_add(0x62E9);
        let a2 = a1.wrapping_add(0x3619).wrapping_add(u16::from(c3));
        self.state = (u32::from(a2) << 16) | u32::from(b2);
        a2
    }

    /// The §7j.65/A closed form the instruction chain reduces to
    /// (the decode proof: both must agree everywhere).
    fn closed_form_step(s: u32) -> u32 {
        (s << 7).wrapping_add(s).wrapping_add(0x3619_62E9)
    }
}

/// §7j.65/B — MissionShell reseeds A ONLY (0x447728, the first body
/// instruction); B is carried across missions within a session.
fn original_mission_reseed(a: &mut OriginalLcg) {
    a.state = ORIGINAL_SEED_A;
}

/// The boot fill byte rule (§7j.65/C, 0x447b27..0x447b32): one RandB
/// draw per byte, `ret & 3 == 0 ? 0xFF : 0x00`.
fn original_noise_byte(rng: &mut OriginalLcg) -> u8 {
    if rng.step() & 3 == 0 {
        0xFF
    } else {
        0x00
    }
}

/// The churn iteration (§7j.65/C, 0x44815d..0x44818d): advance the
/// cursor FIRST (wrap ≥ 0x800 → 0), then draw, then write the byte at
/// the normalized cursor. Returns the (cursor, byte) written.
fn original_churn_byte(rng: &mut OriginalLcg, cursor: &mut usize) -> (usize, u8) {
    *cursor += 1;
    if *cursor >= 0x800 {
        *cursor = 0;
    }
    (*cursor, original_noise_byte(rng))
}

#[test]
fn original_step_matches_the_closed_form() {
    // The decode cross-proof: the instruction-faithful transcription
    // and the closed form S' = ((S<<7) + S + 0x361962E9) mod 2^32
    // agree on every state walked below — both chains plus edge
    // patterns (the shuffle/rcr algebra of §7j.65/A is total, but
    // this pins it empirically at the oracle level).
    let mut edge = vec![0u32, 1, 0x7FFF, 0x8000, 0xFFFF, 0x1E240, 0x39447];
    let mut a = OriginalLcg::new(ORIGINAL_SEED_A);
    for _ in 0..64 {
        edge.push(a.state);
        a.step();
    }
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..64 {
        edge.push(b.state);
        b.step();
    }
    for s in edge {
        let mut t = OriginalLcg::new(s);
        t.step();
        assert_eq!(
            t.state,
            OriginalLcg::closed_form_step(s),
            "closed form at S=0x{s:08x}"
        );
    }
}

#[test]
fn original_first_eight_states_pinned() {
    // Hand-computed from the §7j.65/A decode (the §7j.65 notes quote
    // the first two of each; these eight are the oracle's literals).
    let a_chain: [u32; 8] = [
        123456, 923559209,  // 0x370C6529
        4082654354, // 0xF3585C92
        3584034939, // 0xD5A0087B
        3686639844, // 0xDBBDA8E4
        4037770701, // 0xF0AB7DCD
        2089010998, // 0x7C83C736
        4102079775, // 0xF480C51F
    ];
    let b_chain: [u32; 8] = [
        234567, 937892528,  // 0x37E71AB0
        1636685209, // 0x618DD599
        1586627842, // 0x5E920502
        3719162091, // 0xDDADE8EB
        3938173268, // 0xEABBC154
        2125844029, // 0x7EB5CE3D
        263606182,  // 0x0FB64FA6
    ];
    let mut a = OriginalLcg::new(ORIGINAL_SEED_A);
    for &expect in &a_chain {
        assert_eq!(a.state, expect, "A chain");
        a.step();
    }
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for &expect in &b_chain {
        assert_eq!(b.state, expect, "B chain");
        b.step();
    }
}

#[test]
fn original_mission_reseed_resets_a_only() {
    // §7j.65/B — the write census: 0x447728 re-pins A := 0x1e240 at
    // EVERY MissionShell entry; B has no reseed site (boot-only
    // 0x41c0cd), so a session's B stream is continuous across
    // missions while A restarts at each.
    let mut a = OriginalLcg::new(ORIGINAL_SEED_A);
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..5 {
        a.step();
        b.step();
    }
    assert_ne!(a.state, ORIGINAL_SEED_A);
    // Mission 2 entry: A back at the seed (its next draw re-walks the
    // pinned chain head), B keeps evolving.
    original_mission_reseed(&mut a);
    assert_eq!(a.state, ORIGINAL_SEED_A);
    assert_eq!(a.step(), 14092, "A after reseed re-walks the chain head");
    let mut fresh_b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..5 {
        fresh_b.step();
    }
    assert_eq!(
        b.state, fresh_b.state,
        "B is continuous across the mission boundary"
    );
}

#[test]
fn original_boot_fill_transcription() {
    // §7j.65/C — the fill (0x447b13..0x447b3a): exactly 2048 RandB
    // draws from the SESSION-continuous B stream, one per byte, the
    // cursor untouched. On a fresh boot B starts at its seed 234567.
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    let mut bank = [0u8; 0x800];
    for byte in bank.iter_mut() {
        *byte = original_noise_byte(&mut b);
    }
    // Hand-computed literals (the fill after the fresh-boot B seed):
    // the first 16 bytes are 11 zeros, 0xFF at 11, two zeros, 0xFF
    // at 14 and 15.
    let first16: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0, 0, 0xFF, 0xFF];
    assert_eq!(&bank[..16], &first16);
    // Content is strictly binary {0x00, 0xFF} (§7j.65/C).
    assert!(bank.iter().all(|&x| x == 0 || x == 0xFF));
    // The white census: 526/2048 = 25.68% on this exact draw sequence
    // (the population mean is 25% — `ret&3 == 0`).
    assert_eq!(bank.iter().filter(|&&x| x == 0xFF).count(), 526);
    // The B state after the fill's 2048 draws.
    assert_eq!(b.state, 2774850631, "post-fill B state (0xA564DC47)");
}

#[test]
fn original_churn_transcription() {
    // §7j.65/C — the churn (0x448147..0x448195): 15 draws per frame,
    // advance-then-draw-then-write, cursor wrapping at 0x800. Frame 1
    // of a fresh mission continues the post-fill B stream (the fill
    // is the staging block; the churn is the per-frame epilogue).
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..0x800 {
        b.step();
    }
    assert_eq!(b.state, 2774850631, "post-fill B state");
    let mut cursor = 0usize; // the staging clear 0x4478f7 (ecx = 0)
    let mut written = Vec::with_capacity(15);
    for _ in 0..15 {
        written.push(original_churn_byte(&mut b, &mut cursor));
    }
    // Hand-computed: cursors 1..=15 (advance-first from 0), bytes as
    // drawn from the continuing stream.
    let expect: &[(usize, u8)] = &[
        (1, 0xFF),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
        (8, 0xFF),
        (9, 0xFF),
        (10, 0),
        (11, 0),
        (12, 0),
        (13, 0xFF),
        (14, 0xFF),
        (15, 0),
    ];
    assert_eq!(written.as_slice(), expect);
    assert_eq!(b.state, 4113433838, "post-churn-frame B (0xF52E04EE)");
    // The wrap: from cursor 0x7FF the next advance lands at 0
    // (0x44816a `cmp ecx,0x800; jge` → the 0 store at 0x448178).
    let mut cur = 0x7FF;
    let (c, _) = original_churn_byte(&mut b, &mut cur);
    assert_eq!(c, 0, "cursor wrap 0x7FF -> 0");
    // The full-ring refresh identity: 136 frames * 15 = 2040 < 2048
    // bytes <= 137 * 15 = 2055 — ceil(2048/15) = 137 frames.
    assert_eq!(136 * 15, 2040);
    assert_eq!(2055 - 0x800, 7, "137 frames overrun the ring by 7 bytes");
}

#[test]
fn original_blit_reseed_and_seed_formula() {
    // §7j.65/C/D — the blit (§7i/1) only READS the bank: per row, the
    // look-ahead `src_off + 2*width - 0x800 >= 0` re-picks the read
    // offset `RandB() & 0x1ff` (0x401b22..0x401b39) — a bank READ
    // offset, never a bank write. The per-blit seed (§7i/3,
    // FUN_0041ec59 called with 0x7f6) is `(RandB() & 0x7fff) / 15`
    // clamped <= 0x7f5.
    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..0x800 {
        b.step();
    }
    // Hand-computed from the post-fill stream.
    let mut offsets = Vec::new();
    for _ in 0..4 {
        offsets.push(b.step() as u32 & 0x1FF);
    }
    assert_eq!(offsets, [492, 55, 479, 415]);
    assert!(offsets.iter().all(|&o| o < 0x200), "& 0x1ff bounds");

    let mut b = OriginalLcg::new(ORIGINAL_SEED_B);
    for _ in 0..0x800 {
        b.step();
    }
    let mut seeds = Vec::new();
    for _ in 0..4 {
        let v = (b.step() as u32 & 0x7FFF) / 15;
        seeds.push(v.min(0x7F5));
    }
    assert_eq!(seeds, [237, 993, 202, 846]);
    // The divisor identity (§7i/3): 0x8000 / 0x7f6 - 1 = 15, and the
    // cap 0x7f5 is reachable in-range (0x7fff/15 = 2184 unclamped).
    assert_eq!(0x8000 / 0x7F6 - 1, 15);
    assert_eq!(0x7FFF / 15, 2184);
    assert!(seeds.iter().all(|&v| v <= 0x7F5));
}

// ---------------------------------------------------------------------
// 2. E-side classification (corpus-gated S0 run) — the charter T3
//    seam facts. NOT bit comparisons: the E rows are PCG32 stand-ins
//    (differ `Class::AcceptedT3` — draw counts only, never bits).
// ---------------------------------------------------------------------

#[test]
fn corpus_s0_rng_rows_are_the_documented_t3_standins() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let s0 = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/diffharness/scenarios/S0.scen"),
    )
    .expect("S0.scen committed");
    let run = run_canonical(&s0, &root()).expect("S0 canonical run");
    let dump = decode_dump(&run.bytes).expect("S0 dump verifies");
    assert_eq!(dump.header.channel, Channel::Engine);
    // The stand-in seam pin: BOTH E streams are seeded with 0x1E240 —
    // the ORIGINAL's per-mission A-reseed constant (0x447728). The
    // original's B (234567, boot-only) has no E mirror: the engine's
    // shared mission stream re-arms per mission (mission.rs, the §7i/4
    // per-frame order seam) — a documented charter-T3 divergence,
    // never bit-compared.
    assert!(
        dump.header.pins.iter().any(|p| p == "seed=0x1e240"),
        "the canonical seed pin (the original's per-mission reseed constant)"
    );
    // The rows exist in the canonical u64 form on every T0 frame.
    for frame in &dump.frames {
        for id in ["rng-state-a", "rng-state-b"] {
            let bytes = frame.watch(id).unwrap_or_else(|| panic!("row {id}"));
            assert_eq!(bytes.len(), 8, "{id} canonical form is 8 bytes");
        }
    }
    // Stream liveness: across the scenario's frames the streams move
    // (the rows are live draws, not static fabrications) — the signal
    // the differ's T3 draw-count comparison consumes. The VALUES are
    // deliberately unpinned: they are the stand-in's own states.
    let first: Vec<Vec<u8>> = ["rng-state-a", "rng-state-b"]
        .iter()
        .map(|id| dump.frames[0].watch(id).unwrap().to_vec())
        .collect();
    let moved = dump.frames.iter().any(|frame| {
        ["rng-state-a", "rng-state-b"]
            .iter()
            .enumerate()
            .any(|(i, id)| frame.watch(id) != Some(first[i].as_slice()))
    });
    assert!(moved, "at least one stand-in stream evolves across frames");
    // The dither row is DELIBERATELY ABSENT on E: the noise bank is
    // presentation-half (D17 — writes backbuffer pixels, never engine
    // state, never the dump/hash surface; §7j.65/E). The row is
    // O1-side coverage; fabricating an E emission would be fake
    // parity. Any future emitter re-opens this loudly.
    assert!(
        dump.frames
            .iter()
            .all(|f| f.watch("static-dither-noise").is_none()),
        "static-dither-noise must stay absent on E (D17 presentation half)"
    );
}
