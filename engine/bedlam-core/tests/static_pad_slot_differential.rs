//! Whole-corpus static differential oracle for the runtime `.PAD` staged
//! slot bank — diffharness registry row `static-pad-slots`
//! (EXW 0x4e44f8 / EXD 0xf63c, extent 0x1f38 = 999×8).
//!
//! Expected side: an independent byte-level transcription of the EXW PAD
//! staging loop `FUN_0041dc5a` @0x41de44..0x41df03 (re-verified
//! instruction-by-instruction 2026-08-25, docs/RE-EXW-SIM.md §7c.5):
//! whole-bank memset-0, then per record — stage `x` BEFORE the 0xFFFF
//! check, exit on terminator leaving `{active=0, x=0xFFFF, y=0, z=0}`,
//! else read `y`/`z`, set `active=1`, stamp the DAT volume. The DAT
//! stamp itself is a different row (`static-dat-volume`, covered by
//! `static_loader_differential`); this gate pins the SLOT BANK.
//!
//! Actual side: the Rust target's retained bank `Terrain::pad_slots`
//! (the live run, file order, active implicitly 1) materialized into the
//! same 8-byte record form. The inactive surface (terminator slot bytes,
//! all-zero tail) is unretained by Rust and is asserted against the
//! statically pinned constants instead — never fabricated as Rust output.
//!
//! Scope: valid shipped corpus only. Not a malformed-input spec. No
//! production parser, loader, or terrain helper is reused on the expected
//! side (bytes only).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::mission::Terrain;

const SHIPPED_MISSION_COUNT: usize = 37;
const SLOT_COUNT: usize = 999;
const PAD_LEN: usize = SLOT_COUNT * 6;
const BANK_LEN: usize = SLOT_COUNT * 8;

#[derive(Debug)]
struct MissionFiles {
    identity: String,
    tot: PathBuf,
    dat: PathBuf,
    pad: PathBuf,
    cgr: PathBuf,
}

fn editor_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR")
}

fn is_numbered_mission_stem(stem: &str) -> bool {
    let Some(number) = stem.strip_prefix("MISSION") else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

/// Enumerate the corpus from its shipped ZONE*/MISSION<number> shape,
/// following the same sorted `read_dir` convention as the sibling corpus
/// gates. PAD/TOT/CGR companions hang off the DAT stems.
fn shipped_missions() -> Option<Vec<MissionFiles>> {
    let root = editor_root();
    if !root.is_dir() {
        eprintln!("game-data corpus not found - skipping");
        return None;
    }

    let mut zones: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read EDITOR corpus directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    zones.sort();

    let mut missions = Vec::new();
    for zone_dir in zones {
        let Some(zone_name) = zone_dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(zone_suffix) = zone_name.strip_prefix("ZONE") else {
            continue;
        };
        if zone_suffix.len() != 1 || !zone_suffix.as_bytes()[0].is_ascii_uppercase() {
            continue;
        }

        let mut dat_files: Vec<PathBuf> = fs::read_dir(&zone_dir)
            .unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && {
                    path.extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("dat"))
                        && path
                            .file_stem()
                            .and_then(|value| value.to_str())
                            .is_some_and(is_numbered_mission_stem)
                }
            })
            .collect();
        dat_files.sort();

        let zone_cgr = zone_dir.join(format!("MISSION{zone_suffix}.CGR"));
        for dat in dat_files {
            let stem = dat
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("mission DAT has a UTF-8 stem");
            missions.push(MissionFiles {
                identity: format!("{zone_name}/{stem}"),
                tot: dat.with_extension("TOT"),
                pad: dat.with_extension("PAD"),
                cgr: zone_cgr.clone(),
                dat,
            });
        }
    }
    Some(missions)
}

fn canonical_mission_identities() -> BTreeSet<String> {
    let mut identities =
        BTreeSet::from(["ZONEA/MISSION1".to_string(), "ZONEG/MISSION1".to_string()]);
    for zone in ['B', 'C', 'D', 'E', 'F'] {
        for mission in 1..=7 {
            identities.insert(format!("ZONE{zone}/MISSION{mission}"));
        }
    }
    identities
}

/// Read only the format-pinned TOT header fields (signed LE words at
/// +0/+2); used solely for the live-record bounds assertion.
fn shipped_tot_dimensions(tot: &[u8]) -> Result<(u16, u16), String> {
    if tot.len() < 4 {
        return Err(format!(
            "shipped-corpus precondition: TOT header is truncated: {} bytes",
            tot.len()
        ));
    }
    Ok((
        i16::from_le_bytes([tot[0], tot[1]]) as u16,
        i16::from_le_bytes([tot[2], tot[3]]) as u16,
    ))
}

/// Independent transcription of the EXW PAD staging loop
/// (`FUN_0041dc5a` @0x41de44..0x41df03; EXD twin 0x2e7a0..0x2e85d is
/// identical). Returns `(bank, live_count)` where `bank` is the exact
/// 999×8 staged image and `live_count` the terminator index. Shares no
/// code with the production parser.
fn exw_pad_staging_oracle(pad: &[u8]) -> Result<(Vec<u8>, usize), String> {
    if pad.len() != PAD_LEN {
        return Err(format!(
            "shipped-corpus precondition: PAD length is {}, expected {PAD_LEN} bytes ({SLOT_COUNT} six-byte records)",
            pad.len()
        ));
    }
    // Pinned pre-zero: FUN_00402965(ecx=0x1f38, edi=0x4e44f8) @0x41de62.
    let mut bank = vec![0u8; BANK_LEN];
    let mut slot = 0usize;
    while slot < SLOT_COUNT {
        let base = 6 * slot;
        // x is staged BEFORE the terminator check @0x41defa.
        let x = u16::from_le_bytes([pad[base], pad[base + 1]]);
        bank[8 * slot + 2..8 * slot + 4].copy_from_slice(&x.to_le_bytes());
        if x == 0xFFFF {
            // Terminator slot stays {0, 0xFFFF, 0, 0}: y/z are never
            // read and the active word is never written. Slots after
            // this one keep their pre-zero bytes; their file records
            // are never read.
            return Ok((bank, slot));
        }
        let y = u16::from_le_bytes([pad[base + 2], pad[base + 3]]);
        let z = u16::from_le_bytes([pad[base + 4], pad[base + 5]]);
        bank[8 * slot + 4..8 * slot + 6].copy_from_slice(&y.to_le_bytes());
        bank[8 * slot + 6..8 * slot + 8].copy_from_slice(&z.to_le_bytes());
        // Active word := 1 @0x41de8c.
        bank[8 * slot..8 * slot + 2].copy_from_slice(&1u16.to_le_bytes());
        slot += 1;
    }
    Err(format!(
        "shipped-corpus precondition: PAD has no 0xFFFF-x terminator in its {SLOT_COUNT} records"
    ))
}

/// Materialize the Rust target's RETAINED bank (the live run, file
/// order, active implicitly 1) into the same 8-byte record form. This is
/// a test-only representation of `Terrain::pad_slots`; Rust retains no
/// terminator/tail state, so those slots stay zero here and are checked
/// against the pinned constants via the oracle side instead.
fn rust_staged_bank(terrain: &Terrain) -> (Vec<u8>, usize) {
    let count = terrain.pad_slot_count();
    let mut bank = vec![0u8; BANK_LEN];
    for slot in 0..count {
        let (x, y, z) = terrain.pad_slot(slot).unwrap_or_else(|| {
            panic!("Rust pad_slots live run shorter than its count at slot {slot}")
        });
        bank[8 * slot..8 * slot + 2].copy_from_slice(&1u16.to_le_bytes());
        bank[8 * slot + 2..8 * slot + 4].copy_from_slice(&(x as u16).to_le_bytes());
        bank[8 * slot + 4..8 * slot + 6].copy_from_slice(&(y as u16).to_le_bytes());
        bank[8 * slot + 6..8 * slot + 8].copy_from_slice(&(z as u16).to_le_bytes());
    }
    (bank, count)
}

fn field_name(byte_in_slot: usize) -> &'static str {
    match byte_in_slot {
        0..=1 => "active",
        2..=3 => "x",
        4..=5 => "y",
        _ => "z",
    }
}

/// Field-exact comparison: the shared surface (live run, all four fields)
/// must match byte-for-byte; the inactive surface is compared against the
/// pinned original constants — the terminator slot `{0, 0xFFFF, 0, 0}`
/// and the all-zero tail.
fn assert_pad_banks_match(
    identity: &str,
    oracle: &[u8],
    live: usize,
    actual: &[u8],
    actual_live: usize,
) {
    assert_eq!(
        live, actual_live,
        "{identity}: oracle live-run length vs Rust pad_slot_count"
    );
    for slot in 0..live {
        let expected: [u8; 8] = oracle[8 * slot..8 * slot + 8]
            .try_into()
            .expect("oracle bank slot");
        let staged: [u8; 8] = actual[8 * slot..8 * slot + 8]
            .try_into()
            .expect("rust bank slot");
        for (byte, (&expected_byte, staged_byte)) in expected.iter().zip(staged).enumerate() {
            assert_eq!(
                expected_byte, staged_byte,
                "{identity}: staged slot {slot} field {} differs (oracle {expected_byte:#04x} vs Rust {staged_byte:#04x})",
                field_name(byte)
            );
        }
        assert_eq!(
            expected[0..2],
            [0x01, 0x00],
            "{identity}: live slot {slot} active word must be 1 on both sides"
        );
    }
    // Pinned inactive surface (original-only bytes; Rust retains none):
    let terminator: [u8; 8] = oracle[8 * live..8 * live + 8]
        .try_into()
        .expect("oracle terminator slot");
    assert_eq!(
        terminator,
        [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
        "{identity}: terminator slot {live} must be the pinned {{active=0, x=0xFFFF, y=0, z=0}}"
    );
    assert!(
        oracle[8 * (live + 1)..].iter().all(|byte| *byte == 0),
        "{identity}: all {} tail slots after the terminator must be pre-zero",
        SLOT_COUNT - live - 1
    );
}

#[test]
fn all_missions_pad_slot_bank_matches_exw_staging_oracle() {
    let Some(missions) = shipped_missions() else {
        return;
    };
    assert_eq!(
        missions.len(),
        SHIPPED_MISSION_COUNT,
        "enumerated every shipped numbered mission"
    );
    let identities: BTreeSet<String> = missions
        .iter()
        .map(|mission| mission.identity.clone())
        .collect();
    assert_eq!(identities.len(), missions.len(), "no duplicate identities");
    assert_eq!(
        identities,
        canonical_mission_identities(),
        "enumerated exact canonical shipped mission identity set"
    );

    let mut level_census: BTreeMap<u16, usize> = BTreeMap::new();
    let mut live_counts: BTreeMap<String, usize> = BTreeMap::new();
    for mission in &missions {
        let tot = fs::read(&mission.tot)
            .unwrap_or_else(|error| panic!("{}: read TOT: {error}", mission.identity));
        let dat = fs::read(&mission.dat)
            .unwrap_or_else(|error| panic!("{}: read DAT: {error}", mission.identity));
        let pad = fs::read(&mission.pad)
            .unwrap_or_else(|error| panic!("{}: read PAD: {error}", mission.identity));
        let cgr = fs::read(&mission.cgr)
            .unwrap_or_else(|error| panic!("{}: read zone CGR: {error}", mission.identity));

        let (oracle, live) = exw_pad_staging_oracle(&pad).unwrap_or_else(|error| {
            panic!("{}: oracle rejected corpus: {error}", mission.identity)
        });

        // Live records stay inside the TOT volume (pinned: the original
        // write is unchecked, shipped values are in range).
        let (width, height) = shipped_tot_dimensions(&tot)
            .unwrap_or_else(|error| panic!("{}: {error}", mission.identity));
        for slot in 0..live {
            let x = u16::from_le_bytes([oracle[8 * slot + 2], oracle[8 * slot + 3]]);
            let y = u16::from_le_bytes([oracle[8 * slot + 4], oracle[8 * slot + 5]]);
            let z = u16::from_le_bytes([oracle[8 * slot + 6], oracle[8 * slot + 7]]);
            assert!(
                usize::from(x) < usize::from(width)
                    && usize::from(y) < usize::from(height)
                    && usize::from(z) < 8,
                "{}: live PAD record {slot} ({x}, {y}, z {z}) is out of the {width}x{height}x8 TOT volume",
                mission.identity
            );
            *level_census.entry(z).or_insert(0) += 1;
        }

        // The Rust target's retained bank.
        let terrain = Terrain::from_mission_bytes(&dat, &pad, &cgr)
            .unwrap_or_else(|| panic!("{}: Terrain rejected corpus", mission.identity));
        let (actual, actual_live) = rust_staged_bank(&terrain);
        assert_pad_banks_match(&mission.identity, &oracle, live, &actual, actual_live);

        live_counts.insert(mission.identity.clone(), live);
    }

    // Pinned corpus census (FORMATS-MISSION §10 VERIFIED; independently
    // recomputed by the oracle above — a corpus identity regression or a
    // decode-order bug breaks one of the two sides).
    assert_eq!(
        level_census,
        [
            (0u16, 310usize),
            (1, 173),
            (2, 51),
            (3, 50),
            (4, 62),
            (5, 47),
            (6, 8)
        ]
        .into_iter()
        .collect(),
        "shipped live-record level tally"
    );
    assert_eq!(
        live_counts.values().copied().min(),
        Some(2),
        "minimum shipped live-run length"
    );
    assert_eq!(
        live_counts.values().copied().max(),
        Some(114),
        "maximum shipped live-run length"
    );
    assert_eq!(
        live_counts["ZONEA/MISSION1"], 114,
        "ZONEA/MISSION1 live-run length (§7j.40/1 census)"
    );
    assert_eq!(
        live_counts["ZONEB/MISSION3"], 6,
        "ZONEB/MISSION3 live-run length (one orphan record ignored after the terminator)"
    );
}

/// Sensitivity proof for the oracle (temporary in-memory mutations; the
/// corpus on disk is never touched):
/// 1. a live-field byte flip moves exactly that slot's field;
/// 2. a tail byte after the terminator (the ZONEB/M3 orphan) changes
///    NOTHING — a parser that over-reads (the D112 dead-break bug class)
///    fails this;
/// 3. rewriting the terminator record live extends the run by exactly
///    one, re-staging the old terminator slot active.
#[test]
fn pad_staging_oracle_is_sensitive_to_live_bytes_and_blind_to_the_tail() {
    let zonea = editor_root().join("ZONEA/MISSION1.PAD");
    let zoneb = editor_root().join("ZONEB/MISSION3.PAD");
    if !zonea.is_file() || !zoneb.is_file() {
        eprintln!("game-data corpus not found - skipping");
        return;
    }
    let mut pad_a = fs::read(&zonea).expect("read ZONEA/MISSION1.PAD");
    let mut pad_b = fs::read(&zoneb).expect("read ZONEB/MISSION3.PAD");
    let (bank_a, live_a) = exw_pad_staging_oracle(&pad_a).expect("ZONEA/MISSION1 oracle");
    let (bank_b, live_b) = exw_pad_staging_oracle(&pad_b).expect("ZONEB/MISSION3 oracle");
    assert_eq!(live_a, 114);
    assert_eq!(live_b, 6);

    // (1) flip a live x byte (slot 3, x low byte): exactly the staged x
    // field of slot 3 changes, the run length does not.
    let mut mutated = pad_a.clone();
    mutated[6 * 3] ^= 0x01;
    let (bank_m, live_m) = exw_pad_staging_oracle(&mutated).expect("mutated live oracle");
    assert_eq!(
        live_m, live_a,
        "a live-field flip must not move the terminator"
    );
    let differing: Vec<usize> = (0..BANK_LEN)
        .filter(|&offset| bank_a[offset] != bank_m[offset])
        .collect();
    assert_eq!(
        differing,
        vec![8 * 3 + 2],
        "a live x-byte flip must change exactly slot 3's x field"
    );

    // (2) the ZONEB/M3 orphan record sits at index 7 (record (51,16,3)),
    // one past the terminator at 6 — flipping ANY of its six bytes must
    // leave the staged bank byte-identical.
    let orphan = 7usize;
    let (before_x, before_y, before_z) = (
        u16::from_le_bytes([pad_b[6 * orphan], pad_b[6 * orphan + 1]]),
        u16::from_le_bytes([pad_b[6 * orphan + 2], pad_b[6 * orphan + 3]]),
        u16::from_le_bytes([pad_b[6 * orphan + 4], pad_b[6 * orphan + 5]]),
    );
    assert_ne!(
        (before_x, before_y, before_z),
        (0xFFFF, 0xFFFF, 0xFFFF),
        "the shipped orphan record must be live-looking, not terminator fill"
    );
    for byte in 0..6 {
        pad_b[6 * orphan + byte] ^= 0x01;
    }
    let (bank_b2, live_b2) = exw_pad_staging_oracle(&pad_b).expect("mutated tail oracle");
    assert_eq!(
        live_b2, live_b,
        "a tail mutation must not move the terminator"
    );
    assert_eq!(
        bank_b, bank_b2,
        "mutating every byte of the post-terminator orphan must not change the staged bank"
    );

    // (3) rewriting the terminator record live (index 114 := (1,1,0))
    // extends the run by one: slot 114 stages {1, 1, 1, 0} and the next
    // 0xFFFF fill record becomes the terminator.
    pad_a[6 * live_a..6 * live_a + 6].copy_from_slice(&[0x01, 0x00, 0x01, 0x00, 0x00, 0x00]);
    let (bank_c, live_c) = exw_pad_staging_oracle(&pad_a).expect("terminator-extended oracle");
    assert_eq!(
        live_c,
        live_a + 1,
        "a live terminator record extends the run by one"
    );
    let slot: [u8; 8] = bank_c[8 * live_a..8 * live_a + 8]
        .try_into()
        .expect("re-staged terminator slot");
    assert_eq!(
        slot,
        [0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00],
        "the rewritten terminator slot must stage {{active=1, x=1, y=1, z=0}}"
    );
    let next: [u8; 8] = bank_c[8 * live_c..8 * live_c + 8]
        .try_into()
        .expect("new terminator slot");
    assert_eq!(
        next,
        [0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00],
        "the next 0xFFFF fill record becomes the new terminator"
    );
}
