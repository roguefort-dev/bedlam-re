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
//!    with exactly the expected coverage findings (blink-cursor +
//!    move-target-words rows are E-only — the move-target span is
//!    spliced into the O1 robot-bank row, so its E row has no raw
//!    counterpart — and ZERO robot field gaps: the D90 splice sources
//!    the target trio) + the one T2 frame-counter note (the
//!    O1 counter carries menu frames — never matches E by construction).
//!    No engine-bug/structural findings: the mapped-field contract
//!    holds on the real corpus.
//! 2. DOUBLE-RUN (fabricated O1 vs perturbed copy): the DH-G1 verdict
//!    shape — PASS with counter/RNG divergence budgeted, FAIL on any
//!    other byte diff.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::PathBuf;

use canonical::run_canonical;
use diffharness::differ::{report_text, run_diff, Class, DiffConfig, Mode, Verdict};
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

/// Fabricate the O1-side frame for one E canonical frame.
/// `menu_frames` offsets the frame-counter (the never-resetting O1
/// counter carries the menu walk); `rng_wander` perturbs the RNG words.
fn inv_frame(
    e: &FrameRecord,
    map_wh: Option<(u32, u32)>,
    menu_frames: u32,
    rng_wander: u32,
) -> FrameRecord {
    let mut f = FrameRecord::new(e.frame_no, e.injection_applied);
    for w in &e.watches {
        let id = w.id.as_str();
        let bytes: Vec<u8> = match id {
            // E-only rows: the O1 plan cannot carry them (blink-cursor
            // is a registry gap; move-target-words is CONSUMED into the
            // robot-bank splice on the O1 side, so the E row stays
            // E-only at the row level; the W12-S4 debris/splash T3
            // rows have no EXD alias pinned yet — the stitcher's
            // O1-address rule excludes them, the differ reports the
            // E-only row as a coverage finding, never fabricated).
            "blink-cursor" => continue,
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
                let mut span = vec![0u8; 0x30];
                span[0x00..0x04].copy_from_slice(&hv.to_le_bytes());
                span[0x2c..0x30].copy_from_slice(&wv.to_le_bytes());
                span
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
    let scen = diffharness::runner::Scenario::parse(scen_src).unwrap();
    let header = DumpHeader::new(Channel::O1ExdDosboxX, sha256(b"exd-test"), scen.id.clone());
    stitch(
        &scen,
        &diffharness::runner::Transcript { frames },
        &header,
        &registry(),
    )
    .expect("fabricated transcript stitches")
    .bytes
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
    // the T1 slice: blink-cursor + move-target-words rows are E-only
    // (the latter because the O1 side consumes its span into the
    // robot-bank splice) and the robot field gaps are ZERO — the D90
    // splice sources the record-external target trio (34 canonical
    // leaves, all 34 shared, RE-EXD-MAP §8). S2 (D91) is the first
    // scenario whose splice carries a live present=1 span both ways:
    // the staged walker's target (22,73) fabricates into the EXD
    // x[1]/y[1] u32 pair and splices back — same 2 row-level
    // findings, zero field gaps. S3 (W12-S3, D103) adds the T2 slice:
    // the two full bank rows fabricate as the bare spans and parse
    // back through the shared field walk — still exactly the 2
    // row-level findings, zero field gaps, zero T2 diffs.
    for (id, frames_total, pinned_chain, expect_coverage) in [
        ("S0", 3u64, "8901789a88cf61fe", 0u64),
        ("S1", 401u64, "1c4e7b4c9d9b0947", 2u64),
        ("S2", 17u64, "809f4961b7757da4", 2u64),
        // Re-pinned at the W12-S4-prep landing (D104, §7j.39/9) —
        // the artillery burst-pair application draws the shared
        // stream (was 49193732e6dbc546).
        ("S3", 133u64, "9a11efa03baafb64", 2u64),
        // W12-S4 (DESIGN §7 S4 row): the destroy rows fabricate as
        // the guest banks and parse back through the destroy
        // normalizers — the T1 destroy rows join the exact-exact
        // set, the debris/splash T3 rows are E-only (no EXD alias
        // yet — 2 more row-level coverage findings, documented
        // never fabricated).
        ("S4", 49u64, "35fa3a9234cbff37", 2u64 + 2),
        // W12-S5 (DESIGN §7 S5 row, D108): the ZONEB scenarios carry
        // no T3 tier (nothing fires/dies/explodes in the walks), so
        // the debris/splash rows never ride — exactly the 2 S1-class
        // row-level findings. The REAL-staged mirror rows (every
        // tile active) fabricate as the full 100x100 guest grid and
        // parse back through the same compact-tile filter; the zone
        // row exercises the D108 cell−1 convention end-to-end.
        ("S5", 16u64, "a4659f25d453b6a1", 2u64),
        ("S5B", 19u64, "93e976587a98d2a1", 2u64),
        // W12-S5C (D108's observability follow-up): the pre-damaged
        // walker run — same tier set as S5/S5B (T0/T1/TS: the
        // artillery's debris/splash staging stays unwatched, no T3
        // rows ride), so again exactly the 2 S1-class row-level
        // findings. The destroy-chain cascade the burst rings
        // detonate rides the SAME aliased T1 rows (the compact-tile
        // filter + the destroy normalizers) — zero field gaps.
        ("S5C", 55u64, "786fd87565b67f4a", 2u64),
        // W12-S6 (§7j.40, D112): the pad step-on extraction run —
        // T0/T1/T3/TS. The T3 dropship-frame row is E-only (no EXD
        // alias), so exactly the 2 S1-class findings + 1 more. The
        // beacon-family row's post-deploy latch {0,0,19,70,31} and
        // the surviving claims fabricate through the u16-cell map
        // and parse back exactly; the swept robot's state-5/stop-1e6
        // words ride the aliased robot bank — zero field gaps.
        ("S6", 75u64, "c96f0735df1059ea", 2u64 + 1),
        // W12-S7 (§7j.41, D113): the platform-dynamics lifecycle —
        // T0/T1/T3/TS (the S4 tier set: destroy staged, so the T1
        // destroy rows + both platform banks ride, and the T3
        // debris/splash rows carry the k7 destroy debris — no
        // dropship, no T2 banks). Exactly the 2 S1-class row-level
        // findings + the debris/splash E-only pair (like S4). The
        // platform rows fabricate as the identity spans their
        // normalizers define (both channels carry the same form);
        // the creep-grown mirror words parse back through the
        // compact-tile filter — zero field gaps.
        ("S7", 1361u64, "ecdce5472df6a324", 2u64 + 2),
        // W12-S8 (§7j.42, D114): the critter-engagement lifecycle —
        // T0/T1/T2/T3/TS (the projectile bank rides the 0x68 fire
        // cycle — ALIASED, S3 pinned the T2 form; the critter bank
        // itself + the effect rows are E-ONLY). No destroy staging:
        // the debris/splash rows never ride. Exactly the 2 S1-class
        // row-level findings + the critter-bank/effect-rows pair —
        // zero field gaps (the 0x68 records fabricate through the
        // same bare-span T2 form).
        ("S8", 121u64, "44d806b81bd1b1ff", 2u64 + 2),
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
            o1_frames.push(inv_frame(ef, map_wh, 2000, 0));
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
        // blink-cursor + move-target-words rows are E-only (S1): the
        // splice sources every robot leaf, so exactly the 2 row-level
        // findings remain — no field-level gaps.
        assert_eq!(res.count(Class::Coverage), expect_coverage, "{id}");
        if expect_coverage > 0 {
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "blink-cursor" && f.class == Class::Coverage));
            assert!(res
                .findings
                .iter()
                .any(|f| f.row == "move-target-words" && f.class == Class::Coverage));
        }
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
            .map(|ef| inv_frame(ef, map_wh, 2003, 0x1357_1357))
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
            .map(|ef| inv_frame(ef, map_wh, 2000, 0))
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
