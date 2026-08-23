//! W7 differ gate (DESIGN-DIFFHARNESS.md §6/§6a, D87) — the differ
//! verified against the REAL engine dumps.
//!
//! Corpus-gated (skips without game-data). Uses the W6 canonical
//! emitter (`run_canonical`) on S0/S1 — the dumps whose chains are
//! PINNED in canonical_dump_gate.rs — then fabricates the O1-side
//! transcripts through the INVERSE of the O1 normalizer (test-only;
//! any normalizer map change breaks this coupling loudly) and runs
//! the differ:
//!
//! 1. CROSS-CHANNEL (E vs fabricated O1): verdict PASS-WITH-NOTES
//!    with exactly the expected coverage findings (move-target-words
//!    is the one E-only row — its 0x60 span is spliced into the O1
//!    robot-bank row, so the E row has no raw counterpart; blink-
//!    cursor fabricates identity since D132 closed the EXD twin
//!    0x10e108, exactly like the D136 sfx precedent, so both
//!    channels carry it — and ZERO robot field gaps: the D90 splice
//!    sources the target trio) + the one T2 frame-counter note (the
//!    O1 counter carries menu frames — never matches E by construction).
//!    No engine-bug/structural findings: the mapped-field contract
//!    holds on the real corpus.
//! 2. DOUBLE-RUN (fabricated O1 vs perturbed copy): the DH-G1 verdict
//!    shape — PASS with counter/RNG divergence budgeted, FAIL on any
//!    other byte diff.
//! 3. O2 TIEBREAK ARBITRATION (W11-prep): a fabricated O2 side (the
//!    `inv_frame` output stitched under Channel::O2ExwWine — valid
//!    because `normalize_o2_row`'s alias list takes EXW guest forms
//!    identical to EXD, `EXW_ROBOT_MAP == EXD_ROBOT_MAP`, and
//!    static-map-wh fabricates the D137-pinned EXW 0x28 span) drives
//!    all four `compare_field` T1-exact lanes on a perturbed
//!    `money`: O2-with-O1 → EngineBug "engine is the outlier";
//!    O2-with-E → OriginalDivergence (verdict back to
//!    PASS-WITH-NOTES); all-three-differ → EngineBug "wrong against
//!    both oracles"; no tiebreak → EngineBug "provisional"; plus an
//!    E-vs-O2 cross proving the pinned row compares CLEAN.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::PathBuf;

use canonical::run_canonical;
use diffharness::differ::{
    report_text, run_diff, Class, DiffConfig, DiffResult, FieldVal, Finding, Mode, Verdict,
};
use diffharness::dump::{decode_dump, Channel, DumpHeader, FrameRecord};
use diffharness::hash::sha256;
use diffharness::registry;
use diffharness::runner::stitch;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn scen_path(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/diffharness/scenarios")
        .join(format!("{id}.scen"))
}

fn corpus_present() -> bool {
    root().join("EDITOR/ZONEA/MISSION1.TOT").is_file()
}

// ---------------------------------------------------------------------
// The inverse normalizer (canonical -> EXD raw; test-only)
// ---------------------------------------------------------------------

/// Robot canonical record (94 B) -> EXD 0xA8 record with the 31
/// RE-EXD-MAP §8-mapped leaf fields placed (everything else zero;
/// the canonical target trio is record-external — §5 move-target
/// arrays — so it is never placed).
fn inv_robot_bank(canon: &[u8]) -> Vec<u8> {
    assert!(canon.len() >= 4);
    let n = u32::from_le_bytes(canon[0..4].try_into().unwrap()) as usize;
    assert_eq!(canon.len(), 4 + n * 94);
    let mut out = Vec::with_capacity(n * 0xA8);
    for i in 0..n {
        let rec = &canon[4 + i * 94..4 + (i + 1) * 94];
        // §6a order: alive@0, pos_x@1, pos_y@5, z@9, state@13,
        // dir_byte@15, facing@17, anim@19, variant@21, probe_z@23(16B),
        // stop_dist@39, target@43(9B), drop@52, hp@56, armor@60,
        // hit_flash@62, alarm@64, kind@66, shield@68, charges@72,
        // boost@76, battery@80, pool@84, ctr@88, death@92.
        let mut r = vec![0u8; 0xA8];
        let u16at = |p: usize| u16::from_le_bytes(rec[p..p + 2].try_into().unwrap());
        let i32at = |p: usize| i32::from_le_bytes(rec[p..p + 4].try_into().unwrap());
        r[0x00..0x04].copy_from_slice(&i32at(1).to_le_bytes());
        r[0x04..0x08].copy_from_slice(&i32at(5).to_le_bytes());
        r[0x08..0x0C].copy_from_slice(&i32at(9).to_le_bytes());
        r[0x0C..0x0E].copy_from_slice(&u16at(13).to_le_bytes());
        r[0x0E..0x10].copy_from_slice(&u16at(15).to_le_bytes());
        r[0x10..0x12].copy_from_slice(&u16at(17).to_le_bytes());
        r[0x12..0x14].copy_from_slice(&u16at(19).to_le_bytes());
        r[0x18..0x1A].copy_from_slice(&u16at(21).to_le_bytes());
        for k in 0..8 {
            r[0x1A + 2 * k..0x1C + 2 * k].copy_from_slice(&u16at(23 + 2 * k).to_le_bytes());
        }
        r[0x2A..0x2C].copy_from_slice(&u16at(66).to_le_bytes());
        r[0x2E..0x30].copy_from_slice(&u16at(62).to_le_bytes());
        r[0x30..0x32]
            .copy_from_slice(&i16::from_le_bytes(rec[60..62].try_into().unwrap()).to_le_bytes());
        r[0x34..0x36].copy_from_slice(&u16at(64).to_le_bytes());
        r[0x74..0x78].copy_from_slice(&i32at(39).to_le_bytes());
        r[0x78..0x7C].copy_from_slice(&i32at(56).to_le_bytes());
        let alive = rec[0];
        r[0x7C..0x80].copy_from_slice(&(alive as i32).to_le_bytes());
        r[0x80..0x84].copy_from_slice(&i32at(52).to_le_bytes()); // D88: +0x80
        r[0x88..0x8C].copy_from_slice(&i32at(68).to_le_bytes());
        r[0x8C..0x90].copy_from_slice(&i32at(72).to_le_bytes());
        r[0x94..0x98].copy_from_slice(&i32at(80).to_le_bytes());
        r[0x98..0x9C].copy_from_slice(&i32at(84).to_le_bytes());
        r[0x9C..0x9E].copy_from_slice(&u16at(92).to_le_bytes());
        r[0xA0..0xA4].copy_from_slice(&i32at(76).to_le_bytes());
        r[0xA4..0xA8].copy_from_slice(&i32at(88).to_le_bytes());
        out.extend(r);
    }
    out
}

/// Fabricate the guest-channel frame for one E canonical frame.
/// `menu_frames` offsets the frame-counter (the never-resetting O1
/// counter carries the menu walk); `rng_wander` perturbs the RNG words.
/// `chan` selects the guest raw form of the channel-split rows —
/// static-map-wh (D137/§7j.60): the EXD cells are 0x2c apart with h
/// LOW (the O1 0x30 span) while the EXW cells are 0x24 apart with w
/// LOW (the O2 0x28 span, w@+0x00/h@+0x24).
fn inv_frame(
    e: &FrameRecord,
    map_wh: Option<(u32, u32)>,
    menu_frames: u32,
    rng_wander: u32,
    chan: Channel,
) -> FrameRecord {
    let mut f = FrameRecord::new(e.frame_no, e.injection_applied);
    for w in &e.watches {
        let id = w.id.as_str();
        let bytes: Vec<u8> = match id {
            // E-only rows: the O1 plan cannot carry them
            // (move-target-words is CONSUMED into the robot-bank
            // splice on the O1 side, so the E row stays E-only at
            // the row level; the W12-S4 debris/splash T3 rows have
            // no EXD alias pinned yet — the stitcher's O1-address
            // rule excludes them, the differ reports the E-only row
            // as a coverage finding, never fabricated). blink-cursor
            // is NO LONGER in this set: D132 closed the EXD twin
            // 0x10e108 (a plain 4-B u32 cell — capture-plans/S1.json
            // carries the row), so it fabricates identity through
            // the scalar catch-all below and compares clean (the
            // D136 sfx-master-gate precedent).
            "debris-stager" | "splash-records" => continue,
            // The W12-S6 dropship row: no EXD alias (exd_status
            // unmapped) — the stitcher's O1-address rule excludes
            // it, the differ reports the E-only row as a coverage
            // finding, never fabricated.
            "dropship-frame" => continue,
            // The W12-S8 critter-family rows: no EXD alias (the
            // critter bank + the effect rows are unmapped) — E-only
            // coverage findings, never fabricated on O1.
            "critter-bank" | "effect-rows" => continue,
            "rng-state-a" | "rng-state-b" => {
                let v = u64::from_le_bytes(w.bytes[..8].try_into().unwrap()) as u32;
                v.wrapping_add(rng_wander).to_le_bytes().to_vec()
            }
            "frame-counter" => {
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                (v + menu_frames).to_le_bytes().to_vec()
            }
            "zone" => {
                // D108 (§6a): the O1 cell is the 1-based guest SET
                // (zone_index+1); the E canonical row is the 0-based
                // slot index — fabricate the true cell so the O1
                // normalizer's cell−1 canonicalization round-trips.
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                (v + 1).to_le_bytes().to_vec()
            }
            "robot-bank" => inv_robot_bank(&w.bytes),
            // The no-extract latch (D133/D136): E canonical = u32
            // count + count*4 zero words; the O1 raw form = the bare
            // count-driven span (dbx-plan's $robot_count*4) — strip
            // the prefix so the O1 normalizer's len/4 round-trips.
            "no-extract-latch" => {
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 4);
                w.bytes[4..].to_vec()
            }
            // The T2 banks (W12-S3): E canonical = u32 count + the
            // records; the O1 raw form = the bare span (no count cell
            // on the guest — the free-slot walk is the bound).
            "weapon-anim-bank" => {
                assert_eq!(w.bytes.len(), 4 + 400 * 0x36);
                w.bytes[4..].to_vec()
            }
            "projectile-bank" => {
                assert_eq!(w.bytes.len(), 4 + 50 * 0x22);
                w.bytes[4..].to_vec()
            }
            "move-target-words" => {
                // canonical u32 count + n*9 records -> the 0x60 EXD
                // span: x[i]/y[i] u32 by ABSOLUTE id, -1 = none, the
                // tail slots stay at the spawn -1 fill.
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 9);
                let mut span = vec![0xFFu8; 0x60];
                for i in 0..n.min(12) {
                    let rec = &w.bytes[4 + i * 9..];
                    if rec[0] != 0 {
                        let (px, py) = (4 * i, 0x30 + 4 * i);
                        span[px..px + 4].copy_from_slice(
                            &i32::from_le_bytes(rec[1..5].try_into().unwrap()).to_le_bytes(),
                        );
                        span[py..py + 4].copy_from_slice(
                            &i32::from_le_bytes(rec[5..9].try_into().unwrap()).to_le_bytes(),
                        );
                    }
                }
                span
            }
            "beacon-family" => {
                // canonical u32 x5 -> the five u16 cells.
                w.bytes
                    .chunks(4)
                    .flat_map(|c| (u32::from_le_bytes(c.try_into().unwrap()) as u16).to_le_bytes())
                    .collect()
            }
            "typedb-fade-byte" | "armor-pad-reads" => {
                let len = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + len);
                if len == 0 {
                    // len 0 == all-zero w*h grid (§6a equivalence).
                    let (mw, mh) = map_wh.expect("anchor statics precede grid rows");
                    vec![0u8; (mw * mh) as usize]
                } else {
                    w.bytes[4..].to_vec()
                }
            }
            // The destroy-family rows (W12-S4): E canonical -> the
            // O1 guest raw forms.
            "object-instances" => {
                // canonical {slot,x,y,z,id,flags,hp} 23-B -> the
                // 0x14-stride guest records up to the LIVE count
                // (dead id==-1 tail fills to 2000; the count cell
                // bounds the guest walk).
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 23);
                let mut bank = vec![0u8; 4 + 2000 * 0x14];
                for slot in 0..2000u32 {
                    bank[4 + slot as usize * 0x14 + 0xC..4 + slot as usize * 0x14 + 0x10]
                        .copy_from_slice(&(-1i32).to_le_bytes());
                }
                for i in 0..n {
                    let rec = &w.bytes[4 + i * 23..];
                    let slot = u16::from_le_bytes(rec[0..2].try_into().unwrap()) as usize;
                    let id = i32::from_le_bytes(rec[14..18].try_into().unwrap()) & 0xFF
                        | i32::from(rec[18] & 0x40 != 0) << 14;
                    let g = &mut bank[4 + slot * 0x14..];
                    g[0..4].copy_from_slice(&rec[2..6]);
                    g[4..8].copy_from_slice(&rec[6..10]);
                    g[8..12].copy_from_slice(&rec[10..14]);
                    g[0xC..0x10].copy_from_slice(&id.to_le_bytes());
                    g[0x10..0x14].copy_from_slice(&rec[19..23]);
                }
                bank[0..4].copy_from_slice(&(n as u32).to_le_bytes());
                bank
            }
            "trt-array" => {
                // canonical {active,hp,x,y,z} 20-B -> the 0x20-stride
                // guest records (hp@+0x10, x@+0x14, y@+0x18, z@+0x1C).
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 20);
                let mut bank = vec![0u8; 4 + n * 0x20];
                bank[0..4].copy_from_slice(&(n as u32).to_le_bytes());
                for i in 0..n {
                    let rec = &w.bytes[4 + i * 20..];
                    let g = &mut bank[4 + i * 0x20..];
                    g[0..4].copy_from_slice(&rec[0..4]);
                    g[0x10..0x20].copy_from_slice(&rec[4..20]);
                }
                bank
            }
            "typedb-mirror-rows" => {
                // compact-active {tile, 8x(word,seen)} -> the full
                // 0x1E-stride w*h guest rows (the changed tiles
                // placed, all others zero).
                let n = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(w.bytes.len(), 4 + n * 26);
                let (mw, mh) = map_wh.expect("anchor statics precede grid rows");
                let mut grid = vec![0u8; (mw * mh) as usize * 0x1E];
                for i in 0..n {
                    let rec = &w.bytes[4 + i * 26..];
                    let tile = u16::from_le_bytes(rec[0..2].try_into().unwrap()) as usize;
                    for z in 0..8 {
                        let wv = u16::from_le_bytes(rec[2 + z * 3..4 + z * 3].try_into().unwrap());
                        grid[tile * 0x1E + 2 * z..tile * 0x1E + 2 * z + 2]
                            .copy_from_slice(&wv.to_le_bytes());
                        grid[tile * 0x1E + 0x10 + z] = rec[4 + z * 3];
                    }
                }
                grid
            }
            "tile-word-grid" | "platform-strength" => {
                // Both channels carry the same span — identity.
                w.bytes.clone()
            }
            "static-map-wh" => {
                let wv = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
                let hv = u32::from_le_bytes(w.bytes[4..8].try_into().unwrap());
                match chan {
                    // O1: the EXD 0x30 span (h@+0x00 low cell 0x10748c,
                    // w@+0x2c high cell 0x1074b8).
                    Channel::O1ExdDosboxX => {
                        let mut span = vec![0u8; 0x30];
                        span[0x00..0x04].copy_from_slice(&hv.to_le_bytes());
                        span[0x2c..0x30].copy_from_slice(&wv.to_le_bytes());
                        span
                    }
                    // O2 (D137/§7j.60): the EXW 0x28 span — cells 0x24
                    // apart with w LOW (0x4eddec) / h HIGH (0x4eddf0).
                    Channel::O2ExwWine => {
                        let mut span = vec![0u8; 0x28];
                        span[0x00..0x04].copy_from_slice(&wv.to_le_bytes());
                        span[0x24..0x28].copy_from_slice(&hv.to_le_bytes());
                        span
                    }
                    // inv_frame fabricates GUEST channels only; the
                    // Engine re-stitch passes the real E frames.
                    Channel::Engine | Channel::O3Street => {
                        unreachable!("inv_frame fabricates guest channels only")
                    }
                }
            }
            // identity rows (scalars, selection-triple, order-target,
            // per-player, spread-claims).
            _ => w.bytes.clone(),
        };
        f.push_watch(id, bytes);
    }
    f
}

/// Stitch fabricated O1 frames into dump bytes.
fn stitch_o1(scen_src: &str, frames: Vec<FrameRecord>) -> Vec<u8> {
    stitch_chan(Channel::O1ExdDosboxX, scen_src, frames)
}

/// Stitch fabricated frames into dump bytes for ANY channel (the
/// O1-address rule binds only O1; O2 and Engine accept every registry
/// row the scenario tiers carry — the W11-prep tiebreak side needs
/// both).
fn stitch_chan(channel: Channel, scen_src: &str, frames: Vec<FrameRecord>) -> Vec<u8> {
    let scen = diffharness::runner::Scenario::parse(scen_src).unwrap();
    let header = DumpHeader::new(channel, sha256(b"exd-test"), scen.id.clone());
    stitch(
        &scen,
        &diffharness::runner::Transcript { frames },
        &header,
        &registry(),
    )
    .expect("fabricated transcript stitches")
    .bytes
}

/// Perturb the `money` watch by `delta` in every frame that carries it
/// (S1 keeps it at the boot 4000, so small deltas cannot underflow).
fn perturb_money(mut frames: Vec<FrameRecord>, delta: i64) -> Vec<FrameRecord> {
    for f in frames.iter_mut() {
        if let Some(w) = f.watches.iter_mut().find(|w| w.id == "money") {
            let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
            w.bytes = v.wrapping_add(delta as u32).to_le_bytes().to_vec();
        }
    }
    frames
}

#[test]
fn s0_s1_cross_and_double_run() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    let reg = registry();

    // S0 = T0+TS only (no T1 rows -> no coverage asymmetry); S1 adds
    // the T1 slice: the move-target-words row is E-only (the O1 side
    // consumes its span into the robot-bank splice; blink-cursor
    // fabricates identity since D132 closed the EXD twin 0x10e108 —
    // the real capture plans dump the row, so the fabricated side
    // must too, exactly like the D136 sfx precedent) and the robot
    // field gaps are ZERO — the D90
    // splice sources the record-external target trio (34 canonical
    // leaves, all 34 shared, RE-EXD-MAP §8). S2 (D91) is the first
    // scenario whose splice carries a live present=1 span both ways:
    // the staged walker's target (22,73) fabricates into the EXD
    // x[1]/y[1] u32 pair and splices back — same 1 row-level
    // finding, zero field gaps. S3 (W12-S3, D103) adds the T2 slice:
    // the two full bank rows fabricate as the bare spans and parse
    // back through the shared field walk — still exactly the 1
    // row-level finding, zero field gaps, zero T2 diffs.
    for (id, frames_total, pinned_chain, expect_coverage) in [
        ("S0", 3u64, "dac1cfd17bc7ede3", 0u64),
        ("S1", 401u64, "a18cb11ac8e4314e", 1u64),
        ("S2", 17u64, "d6649ce272ad6d96", 1u64),
        // Re-pinned at the W12-S4-prep landing (D104, §7j.39/9) —
        // the artillery burst-pair application draws the shared
        // stream (was 49193732e6dbc546).
        ("S3", 133u64, "f4f5b4351e976ed5", 1u64),
        // W12-S4 (DESIGN §7 S4 row): the destroy rows fabricate as
        // the guest banks and parse back through the destroy
        // normalizers — the T1 destroy rows join the exact-exact
        // set, the debris/splash T3 rows are E-only (no EXD alias
        // yet — 2 more row-level coverage findings, documented
        // never fabricated).
        ("S4", 49u64, "63ab5ac7679f6de7", 1u64 + 2),
        // W12-S5 (DESIGN §7 S5 row, D108): the ZONEB scenarios carry
        // no T3 tier (nothing fires/dies/explodes in the walks), so
        // the debris/splash rows never ride — exactly the 1 S1-class
        // row-level finding (move-target-words only, post-D132).
        // The REAL-staged mirror rows (every
        // tile active) fabricate as the full 100x100 guest grid and
        // parse back through the same compact-tile filter; the zone
        // row exercises the D108 cell−1 convention end-to-end.
        ("S5", 16u64, "8a718339e0702fd6", 1u64),
        ("S5B", 19u64, "b72f57e0b8e7042b", 1u64),
        // W12-S5C (D108's observability follow-up): the pre-damaged
        // walker run — same tier set as S5/S5B (T0/T1/TS: the
        // artillery's debris/splash staging stays unwatched, no T3
        // rows ride), so again exactly the 1 S1-class row-level
        // finding. The destroy-chain cascade the burst rings
        // detonate rides the SAME aliased T1 rows (the compact-tile
        // filter + the destroy normalizers) — zero field gaps.
        ("S5C", 55u64, "de5b80a6177aecdd", 1u64),
        // W12-S6 (§7j.40, D112): the pad step-on extraction run —
        // T0/T1/T3/TS. The T3 dropship-frame row is E-only (no EXD
        // alias), so exactly the 1 S1-class finding + 1 more. The
        // beacon-family row's post-deploy latch {0,0,19,70,31} and
        // the surviving claims fabricate through the u16-cell map
        // and parse back exactly; the swept robot's state-5/stop-1e6
        // words ride the aliased robot bank — zero field gaps.
        ("S6", 75u64, "c27bff339929339d", 1u64 + 1),
        // W12-S7 (§7j.41, D113): the platform-dynamics lifecycle —
        // T0/T1/T3/TS (the S4 tier set: destroy staged, so the T1
        // destroy rows + both platform banks ride, and the T3
        // debris/splash rows carry the k7 destroy debris — no
        // dropship, no T2 banks). Exactly the 1 S1-class row-level
        // finding + the debris/splash E-only pair (like S4). The
        // platform rows fabricate as the identity spans their
        // normalizers define (both channels carry the same form);
        // the creep-grown mirror words parse back through the
        // compact-tile filter — zero field gaps.
        ("S7", 1361u64, "b0db22840310e82a", 1u64 + 2),
        // W12-S8 (§7j.42, D114): the critter-engagement lifecycle —
        // T0/T1/T2/T3/TS (the projectile bank rides the 0x68 fire
        // cycle — ALIASED, S3 pinned the T2 form; the critter bank
        // itself + the effect rows are E-ONLY). No destroy staging:
        // the debris/splash rows never ride. Exactly the 1 S1-class
        // row-level finding + the critter-bank/effect-rows pair —
        // zero field gaps (the 0x68 records fabricate through the
        // same bare-span T2 form).
        ("S8", 121u64, "29fa2f400a10974b", 1u64 + 2),
    ] {
        let src = fs::read_to_string(scen_path(id)).unwrap();
        let e_run = run_canonical(&src, &root).unwrap();
        assert_eq!(e_run.manifest.frame_count, frames_total);
        assert_eq!(
            e_run.manifest.chain_digest, pinned_chain,
            "the pinned E content"
        );
        let e_dump = decode_dump(&e_run.bytes).unwrap();

        // Fabricate the O1 side (+2000 menu frames on the counter).
        let mut o1_frames = Vec::new();
        let mut map_wh: Option<(u32, u32)> = None;
        for ef in &e_dump.frames {
            if let Some(b) = ef.watch("static-map-wh") {
                map_wh = Some((
                    u32::from_le_bytes(b[..4].try_into().unwrap()),
                    u32::from_le_bytes(b[4..8].try_into().unwrap()),
                ));
            }
            o1_frames.push(inv_frame(ef, map_wh, 2000, 0, Channel::O1ExdDosboxX));
        }
        let o1_bytes = stitch_o1(&src, o1_frames);

        // ---- cross-channel: the mapped-field contract holds ----
        let res = run_diff(
            &e_run.bytes,
            &o1_bytes,
            None,
            &DiffConfig::new(Mode::CrossChannel),
            &reg,
        )
        .unwrap();
        assert_eq!(
            res.verdict,
            Verdict::PassWithNotes,
            "{id} cross verdict\n{}",
            report_text(&res)
        );
        assert_eq!(res.count(Class::EngineBug), 0, "{id}");
        assert_eq!(res.count(Class::Structural), 0, "{id}");
        // move-target-words is the one E-only row (S1+): the splice
        // sources every robot leaf, so exactly the 1 row-level
        // finding remains — no field-level gaps.
        assert_eq!(res.count(Class::Coverage), expect_coverage, "{id}");
        if expect_coverage > 0 {
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "move-target-words" && f.class == Class::Coverage));
        }
        // The D132-alignment guard: blink-cursor is carried by BOTH
        // channels (identity u32; the named O1 normalizer arm), so it
        // must never appear as a finding — row- or field-level.
        assert!(
            res.findings.iter().all(|f| f.row != "blink-cursor"),
            "{id}: blink-cursor must compare clean post-D132\n{}",
            report_text(&res)
        );
        if id == "S4" || id == "S7" {
            // The debris/splash rows have no EXD alias yet — exactly
            // the 2 extra row-level findings (E-only rows, never
            // fabricated O1 bytes).
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "debris-stager" && f.class == Class::Coverage));
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "splash-records" && f.class == Class::Coverage));
            // The aliased destroy rows compare exact-exact: ZERO
            // field-level findings on them.
            for r in [
                "object-instances",
                "trt-array",
                "tile-word-grid",
                "platform-strength",
                "typedb-mirror-rows",
            ] {
                assert!(
                    res.findings
                        .iter()
                        .all(|f| f.row != r || f.class == Class::Coverage),
                    "{id}: row {r} must be gap-or-clean, got {}",
                    res.findings
                        .iter()
                        .filter(|f| f.row == r)
                        .map(|f| format!("{:?}", f.class))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
        }
        // The never-resetting O1 counter is the single T2 note.
        assert_eq!(res.count(Class::T2Reported), 1, "{id}");
        // The anchor statics normalized identically (w/h from the span).
        assert!(res.findings.iter().all(|f| f.row != "static-map-wh"));

        // ---- double-run: the DH-G1 verdict shape ----
        let o1_frames_b: Vec<FrameRecord> = e_dump
            .frames
            .iter()
            .map(|ef| inv_frame(ef, map_wh, 2003, 0x1357_1357, Channel::O1ExdDosboxX))
            .collect();
        let o1_bytes_b = stitch_o1(&src, o1_frames_b);
        let res = run_diff(
            &o1_bytes,
            &o1_bytes_b,
            None,
            &DiffConfig::new(Mode::DoubleRun),
            &reg,
        )
        .unwrap();
        assert_eq!(
            res.verdict,
            Verdict::Pass,
            "{id} double-run modulo counter/RNG\n{}",
            report_text(&res)
        );
        assert!(res.findings.is_empty(), "{id}");

        // ...and FAILS on any other byte diff (money on S1 stays 4000;
        // perturb the fabricated score row's neighbor through money).
        let mut bad: Vec<FrameRecord> = e_dump
            .frames
            .iter()
            .map(|ef| inv_frame(ef, map_wh, 2000, 0, Channel::O1ExdDosboxX))
            .collect();
        for f in bad.iter_mut() {
            if let Some(w) = f.watches.iter_mut().find(|w| w.id == "money") {
                let v = u32::from_le_bytes(w.bytes[..4].try_into().unwrap()) - 7;
                w.bytes = v.to_le_bytes().to_vec();
            }
        }
        let bad_bytes = stitch_o1(&src, bad);
        let res = run_diff(
            &o1_bytes,
            &bad_bytes,
            None,
            &DiffConfig::new(Mode::DoubleRun),
            &reg,
        )
        .unwrap();
        assert_eq!(res.verdict, Verdict::Fail, "{id}");
        assert_eq!(res.first_divergence().unwrap().row, "money");
    }
}

// ---------------------------------------------------------------------
// The O2 tiebreak fabrication (W11-prep, 2026-08-23): the differ's
// four arbitration lanes driven headless (DESIGN sec 6 "Arbitration
// lanes GATED").
// ---------------------------------------------------------------------

/// The aggregated `money.value` finding of a diff (exactly one
/// expected — the fabrication introduces no other T1 diffs).
fn money_finding(res: &DiffResult) -> &Finding {
    res.findings
        .iter()
        .find(|f| f.row == "money" && f.field == "value")
        .expect("the money.value finding")
}

#[test]
fn s1_o2_tiebreak_arbitration() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    let reg = registry();

    let src = fs::read_to_string(scen_path("S1")).unwrap();
    let e_run = run_canonical(&src, &root).unwrap();
    assert_eq!(
        e_run.manifest.chain_digest, "a18cb11ac8e4314e",
        "the pinned E content (S1)"
    );
    let e_dump = decode_dump(&e_run.bytes).unwrap();

    // The fabrications are CHANNEL-SPLIT since the D137 pin: every
    // aliased row's EXW guest form IS the EXD form
    // (normalize_o2_row delegates to normalize_o1_row) and the robot
    // map is the same table (EXW_ROBOT_MAP == EXD_ROBOT_MAP) — but
    // static-map-wh is NOT (EXW cells 0x24 apart w LOW vs the EXD
    // 0x2c h LOW), so the O1 and O2 dumps stitch from their own
    // inv_frame channel forms.
    let mut map_wh: Option<(u32, u32)> = None;
    let mut fab_o1: Vec<FrameRecord> = Vec::new();
    let mut fab_o2: Vec<FrameRecord> = Vec::new();
    for ef in &e_dump.frames {
        if let Some(b) = ef.watch("static-map-wh") {
            map_wh = Some((
                u32::from_le_bytes(b[..4].try_into().unwrap()),
                u32::from_le_bytes(b[4..8].try_into().unwrap()),
            ));
        }
        fab_o1.push(inv_frame(ef, map_wh, 2000, 0, Channel::O1ExdDosboxX));
        fab_o2.push(inv_frame(ef, map_wh, 2000, 0, Channel::O2ExwWine));
    }
    let o1_bytes = stitch_chan(Channel::O1ExdDosboxX, &src, fab_o1.clone());
    let o2_bytes = stitch_chan(Channel::O2ExwWine, &src, fab_o2.clone());

    // Perturbed variants: O1 -7 (the wrong oracle reading), O2 -3
    // (a third reading neither side holds), and an Engine re-stitch
    // of the real E frames with money -7 (the engine-is-wrong case).
    let o1_bad = stitch_chan(
        Channel::O1ExdDosboxX,
        &src,
        perturb_money(fab_o1.clone(), -7),
    );
    let o2_bad = stitch_chan(Channel::O2ExwWine, &src, perturb_money(fab_o2, -3));
    let e_bad = stitch_chan(
        Channel::Engine,
        &src,
        perturb_money(e_dump.frames.clone(), -7),
    );

    let cross = DiffConfig::new(Mode::CrossChannel);

    // ---- the D137 flip: E vs the fabricated O2 DIRECTLY — the
    // static-map-wh row now parses through the real O2 normalizer
    // (the 0x28 EXW span) and must COMPARE CLEAN (pre-pin, the
    // zero-field arm rendered it as 2 field-level coverage gaps).
    // The verdict shape mirrors the E-vs-O1 cross exactly: coverage
    // = move-target-words only, the +2000 counter is the one T2
    // note, zero EngineBug/Structural. ----
    let res = run_diff(&e_run.bytes, &o2_bytes, None, &cross, &reg).unwrap();
    assert_eq!(
        res.verdict,
        Verdict::PassWithNotes,
        "E vs O2 with the D137-pinned static-map-wh form\n{}",
        report_text(&res)
    );
    assert_eq!(res.count(Class::EngineBug), 0);
    assert_eq!(res.count(Class::Structural), 0);
    assert_eq!(res.count(Class::Coverage), 1); // move-target-words only
    assert_eq!(res.count(Class::T2Reported), 1);
    assert!(
        res.findings.iter().all(|f| f.row != "static-map-wh"),
        "static-map-wh compares clean through the O2 normalizer\n{}",
        report_text(&res)
    );

    // ---- baseline: a present tiebreak dump changes NOTHING while no
    // T1 diff exists (the O2 side is only read at arbitration time) ----
    let res = run_diff(&e_run.bytes, &o1_bytes, Some(&o2_bytes), &cross, &reg).unwrap();
    assert_eq!(
        res.verdict,
        Verdict::PassWithNotes,
        "baseline with idle tiebreak\n{}",
        report_text(&res)
    );
    assert_eq!(res.count(Class::EngineBug), 0);
    assert_eq!(res.count(Class::OriginalDivergence), 0);
    assert_eq!(res.count(Class::Coverage), 1); // move-target-words only
    assert!(res.findings.iter().all(|f| f.row != "money"));
    let tb = res.tiebreak.as_ref().expect("the tiebreak fingerprint");
    assert_eq!(tb.channel, "O2:EXW/Wine");
    assert_eq!(tb.scenario, "S1");

    // ---- lane (a): O2 sides with O1 against a perturbed E ----
    let res = run_diff(&e_bad, &o1_bytes, Some(&o2_bytes), &cross, &reg).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    assert_eq!(
        res.count(Class::EngineBug),
        1,
        "money only\n{}",
        report_text(&res)
    );
    let f = money_finding(&res);
    assert_eq!(f.class, Class::EngineBug);
    assert_eq!(
        f.detail,
        "O2/EXW canon agrees with O1: the engine (E) is the outlier"
    );
    assert_eq!(f.a, Some(FieldVal::Int(3993))); // E' = 4000-7
    assert_eq!(f.b, Some(FieldVal::Int(4000))); // O1/O2 canon
    assert_eq!(res.first_divergence().unwrap().row, "money");

    // ---- lane (b): O2 sides with E against a perturbed O1 — the
    // re-class BUDGETS the diff (verdict back to PASS-WITH-NOTES) ----
    let res = run_diff(&e_run.bytes, &o1_bad, Some(&o2_bytes), &cross, &reg).unwrap();
    assert_eq!(
        res.verdict,
        Verdict::PassWithNotes,
        "original-divergence is budgeted\n{}",
        report_text(&res)
    );
    assert_eq!(res.count(Class::EngineBug), 0);
    assert_eq!(res.count(Class::OriginalDivergence), 1);
    let f = money_finding(&res);
    assert_eq!(f.class, Class::OriginalDivergence);
    assert_eq!(
        f.detail,
        "O2/EXW canon agrees with E: EXD diverges from EXW (engine keeps EXW; log to docs/DIVERGENCES.md)"
    );
    assert_eq!(f.a, Some(FieldVal::Int(4000))); // E/O2 canon
    assert_eq!(f.b, Some(FieldVal::Int(3993))); // O1' = 4000-7

    // ---- lane (c): all three channels hold different readings ----
    let res = run_diff(&e_run.bytes, &o1_bad, Some(&o2_bad), &cross, &reg).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    assert_eq!(res.count(Class::EngineBug), 1);
    let f = money_finding(&res);
    assert_eq!(f.class, Class::EngineBug);
    assert_eq!(
        f.detail,
        "all three channels differ (E wrong against both oracles)"
    );
    assert_eq!(f.a, Some(FieldVal::Int(4000))); // E
    assert_eq!(f.b, Some(FieldVal::Int(3993))); // O1' (O2' = 3997 off-stage)

    // ---- lane (d): no tiebreak dump supplied — provisional ----
    let res = run_diff(&e_run.bytes, &o1_bad, None, &cross, &reg).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    assert_eq!(res.count(Class::EngineBug), 1);
    let f = money_finding(&res);
    assert_eq!(f.class, Class::EngineBug);
    assert_eq!(
        f.detail,
        "provisional engine-bug: no O2 tiebreak dump supplied"
    );
    assert!(res.tiebreak.is_none());
}
