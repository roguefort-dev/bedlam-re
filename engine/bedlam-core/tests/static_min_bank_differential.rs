//! Whole-corpus static differential oracle for the `.MIN` territory-mask
//! bank — diffharness registry row `static-min-bank`
//! (EXW 0x4edd9c / EXD 0x107538, arena extent 0x7530 = 30000 B).
//!
//! Expected side: an independent byte-level transcription of the EXW
//! loader (FUN_0041dc5a @0x41dcd8..0x41dcf3, re-verified
//! instruction-by-instruction 2026-08-25, docs/RE-EXW-SIM.md §7j.62):
//! a verbatim whole-file read into the never-memset arena bank (no
//! header skip, no transform, no 0x7530 cap in the original), plus the
//! sole consumer's projection — the 4×4 territory stamp FUN_00402ab8
//! (mask byte 0 → transparent, else XLAT through the MAPTRAN ramp
//! selected by the robot-proximity variant byte; dest row stride 640,
//! row advance 0x27c = 640−4) indexed `cw = LNK/LNG_word[TOT word]`
//! with cw==0 skipped (caller 0x408a8e..0x408ae3).
//!
//! Actual side: NONE, by documented design. The bank is
//! presentation-half (D17): its only consumer writes backbuffer pixels —
//! never engine state, never in the hash surface — and bedlam-core
//! retains nothing of it. A retained `Vec<u8>` with zero Rust consumers
//! would be fabricated parity, so no seam is added; this gate pins the
//! ORIGINAL side exactly (loader transcription, consumer projection,
//! corpus identities, and the stale-tail-never-read bound) and records
//! the Rust absence as the row's parity status.
//!
//! Scope: valid shipped corpus only. Not a malformed-input spec. No
//! production parser, loader, or terrain helper is reused on the
//! expected side (bytes only).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The arena allocation: `mov eax,0x7530; call 0x41db89; mov
/// ds:0x4edd9c,eax` @0x41dabd..0x41dac7 (EXD twin @0x2e3e6..0x2e3f0).
const BANK_LEN: usize = 0x7530;
/// Mask entry granularity: the reader addresses `bank + cw*0x10` and
/// walks 16 bytes (4×4).
const ENTRY_LEN: usize = 16;
/// The LNK/LNG lookup images are 16384 B = 8192 words (FORMATS §5/§7).
const LINK_WORDS: usize = 8192;
/// Backbuffer row stride of the stamp destination.
const SCREEN_STRIDE: usize = 640;

const SHIPPED_ZONE_COUNT: usize = 7;
const SHIPPED_MISSION_COUNT: usize = 37;

#[derive(Debug)]
struct ZoneFiles {
    letter: char,
    identity: String,
    min: PathBuf,
    lnk: PathBuf,
    lng: PathBuf,
    /// Sorted `MISSION<number>` stems (from the DAT census convention).
    missions: Vec<String>,
    dir: PathBuf,
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
/// gates. The `.MIN`/`.LNK`/`.LNG` files are ZONE-scoped (MISSIONX.*).
fn shipped_zones() -> Option<Vec<ZoneFiles>> {
    let root = editor_root();
    if !root.is_dir() {
        eprintln!("game-data corpus not found - skipping");
        return None;
    }

    let mut zone_dirs: Vec<PathBuf> = fs::read_dir(&root)
        .expect("read EDITOR corpus directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    zone_dirs.sort();

    let mut zones = Vec::new();
    for dir in zone_dirs {
        let Some(zone_name) = dir.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(zone_suffix) = zone_name.strip_prefix("ZONE") else {
            continue;
        };
        if zone_suffix.len() != 1 || !zone_suffix.as_bytes()[0].is_ascii_uppercase() {
            continue;
        }
        let letter = zone_suffix.as_bytes()[0] as char;
        if !dir.join(format!("MISSION{letter}.MIN")).is_file() {
            continue;
        }

        let mut missions: Vec<String> = fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("dat"))
                    && entry
                        .path()
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .is_some_and(is_numbered_mission_stem)
            })
            .filter_map(|entry| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .map(str::to_owned)
            })
            .collect();
        missions.sort();

        zones.push(ZoneFiles {
            letter,
            identity: zone_name.to_owned(),
            min: dir.join(format!("MISSION{letter}.MIN")),
            lnk: dir.join(format!("MISSION{letter}.LNK")),
            lng: dir.join(format!("MISSION{letter}.LNG")),
            missions,
            dir,
        });
    }
    Some(zones)
}

/// Independent transcription of the EXW `.MIN` load: the whole file
/// lands in the bank verbatim. Returns the file prefix as the bank's
/// defined surface; the bytes beyond it are stale arena (never memset,
/// never observable through the corpus — proven by the cw bound below).
///
/// Divergence note: the original `FUN_0041cc7f` read has NO cap against
/// the 0x7530 allocation — a larger file would clobber the following
/// arena blocks unchecked. The oracle models only the shipped domain
/// and rejects anything larger, so a synthetic overflow fails loudly
/// here instead of silently "passing".
fn exw_min_bank_prefix(min: &[u8]) -> Result<&[u8], String> {
    if min.len() > BANK_LEN {
        return Err(format!(
            "MIN file is {} bytes, over the {BANK_LEN}-byte arena bank (the original \
             load is uncapped and would clobber the arena; never shipped)",
            min.len()
        ));
    }
    Ok(min)
}

/// The lookup `cw = word@(image + 2*type)` (the caller reads the dword
/// at 0x45cdd8 + 2*type and takes its upper word — the word at
/// 0x45cdda + 2*type, i.e. image word #type). The original is
/// unchecked; the oracle stays inside the 8192-word image.
fn exw_link_lookup(image: &[u8], type_word: u16) -> Option<u16> {
    let index = usize::from(type_word);
    if index >= LINK_WORDS || 2 * index + 2 > image.len() {
        return None;
    }
    Some(u16::from_le_bytes([image[2 * index], image[2 * index + 1]]))
}

/// The stamp source of the sole runtime reader FUN_00402ab8: the 16
/// mask bytes at `bank + cw*0x10`. Returns `None` when the entry would
/// reach past the file prefix into the stale arena tail (a dead read —
/// unreachable on the shipped corpus, asserted by the projection).
fn exw_mask_entry(prefix: &[u8], cw: u16) -> Option<[u8; ENTRY_LEN]> {
    let start = usize::from(cw) * ENTRY_LEN;
    prefix
        .get(start..start + ENTRY_LEN)
        .and_then(|slice| slice.try_into().ok())
}

/// The full stamp projection with an identity ramp (ramp[b] = b): for
/// each mask byte != 0 the destination pixel at (x+c, y+r) takes the
/// mask byte value (XLAT with the test ramp); 0 bytes leave the
/// destination untouched. Modelled on a small window to keep the
/// 640-stride arithmetic observable.
fn exw_territory_stamp(prefix: &[u8], cw: u16) -> Option<Vec<Option<u8>>> {
    let entry = exw_mask_entry(prefix, cw)?;
    let mut pixels = vec![None; 4 * SCREEN_STRIDE];
    for row in 0..4usize {
        for col in 0..4usize {
            let byte = entry[row * 4 + col];
            if byte != 0 {
                pixels[row * SCREEN_STRIDE + col] = Some(byte);
            }
        }
    }
    Some(pixels)
}

/// Independent read of the TOT word stream: u16 W + u16 H header, then
/// 8 plane-major layers of W·H u16 words (the lookup key surface).
fn exw_tot_words(tot: &[u8]) -> Result<Vec<u16>, String> {
    if tot.len() < 4 {
        return Err("TOT header truncated".to_owned());
    }
    let width = u16::from_le_bytes([tot[0], tot[1]]);
    let height = u16::from_le_bytes([tot[2], tot[3]]);
    let expected = 4 + usize::from(width) * usize::from(height) * 8 * 2;
    if tot.len() != expected {
        return Err(format!(
            "TOT length {} != header-derived {} ({}x{}x8 words)",
            tot.len(),
            expected,
            width,
            height
        ));
    }
    let mut words = Vec::with_capacity(expected / 2);
    for chunk in tot[4..].chunks_exact(2) {
        words.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(words)
}

struct ZoneCensus {
    letter: char,
    min_len: usize,
    entries: usize,
    lnk_reachable: usize,
    lng_reachable: usize,
    union_reachable: usize,
    max_cw_lnk: u16,
    max_cw_lng: u16,
    zero_entries: usize,
    distinct_bytes: usize,
    max_byte: u8,
}

/// The pinned §7j.62 D census row per zone, minus the zone letter.
#[derive(Debug, PartialEq)]
struct CensusPin {
    min_len: usize,
    entries: usize,
    lnk_reachable: usize,
    lng_reachable: usize,
    union_reachable: usize,
    max_cw_lnk: u16,
    max_cw_lng: u16,
    zero_entries: usize,
    distinct_bytes: usize,
    max_byte: u8,
}

impl CensusPin {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        min_len: usize,
        entries: usize,
        lnk_reachable: usize,
        lng_reachable: usize,
        union_reachable: usize,
        max_cw_lnk: u16,
        max_cw_lng: u16,
        zero_entries: usize,
        distinct_bytes: usize,
        max_byte: u8,
    ) -> Self {
        Self {
            min_len,
            entries,
            lnk_reachable,
            lng_reachable,
            union_reachable,
            max_cw_lnk,
            max_cw_lng,
            zero_entries,
            distinct_bytes,
            max_byte,
        }
    }
}

fn census_of(census: &ZoneCensus) -> CensusPin {
    CensusPin {
        min_len: census.min_len,
        entries: census.entries,
        lnk_reachable: census.lnk_reachable,
        lng_reachable: census.lng_reachable,
        union_reachable: census.union_reachable,
        max_cw_lnk: census.max_cw_lnk,
        max_cw_lng: census.max_cw_lng,
        zero_entries: census.zero_entries,
        distinct_bytes: census.distinct_bytes,
        max_byte: census.max_byte,
    }
}

#[test]
fn all_zones_min_bank_projection_matches_exw_oracle() {
    let Some(zones) = shipped_zones() else {
        return;
    };
    assert_eq!(
        zones.len(),
        SHIPPED_ZONE_COUNT,
        "enumerated every shipped zone (.MIN is zone-scoped)"
    );
    let identities: BTreeSet<String> = zones.iter().map(|z| z.identity.clone()).collect();
    assert_eq!(
        identities,
        ['A', 'B', 'C', 'D', 'E', 'F', 'G']
            .iter()
            .map(|l| format!("ZONE{l}"))
            .collect(),
        "enumerated the exact canonical zone set"
    );

    let mut total_missions = 0usize;
    let mut max_type_corpus = 0u16;
    let mut mission_max_cw: BTreeMap<String, u16> = BTreeMap::new();
    let mut censuses: Vec<ZoneCensus> = Vec::new();
    let mut min_bytes: BTreeMap<char, Vec<u8>> = BTreeMap::new();

    for zone in &zones {
        let min = fs::read(&zone.min)
            .unwrap_or_else(|error| panic!("{}: read MIN: {error}", zone.identity));
        let lnk = fs::read(&zone.lnk)
            .unwrap_or_else(|error| panic!("{}: read LNK: {error}", zone.identity));
        let lng = fs::read(&zone.lng)
            .unwrap_or_else(|error| panic!("{}: read LNG: {error}", zone.identity));
        assert_eq!(
            lnk.len(),
            LINK_WORDS * 2,
            "{}: LNK image size",
            zone.identity
        );
        assert_eq!(
            lng.len(),
            LINK_WORDS * 2,
            "{}: LNG image size",
            zone.identity
        );
        let prefix = exw_min_bank_prefix(&min)
            .unwrap_or_else(|error| panic!("{}: oracle rejected corpus: {error}", zone.identity));
        assert_eq!(
            prefix.len() % ENTRY_LEN,
            0,
            "{}: shipped MIN length is a whole number of 16-B entries",
            zone.identity
        );
        min_bytes.insert(zone.letter, min.clone());

        let mut cw_lnk: BTreeSet<u16> = BTreeSet::new();
        let mut cw_lng: BTreeSet<u16> = BTreeSet::new();
        for mission in &zone.missions {
            let tot_path = zone.dir.join(format!("{mission}.TOT"));
            let tot = fs::read(&tot_path)
                .unwrap_or_else(|error| panic!("{}: read {mission}.TOT: {error}", zone.identity));
            let words = exw_tot_words(&tot)
                .unwrap_or_else(|error| panic!("{}: {mission}: {error}", zone.identity));
            let types: BTreeSet<u16> = words.iter().copied().collect();
            let zone_max = types.iter().copied().max().unwrap_or(0);
            max_type_corpus = max_type_corpus.max(zone_max);
            let mut mission_max = 0u16;
            for &type_word in &types {
                // Every shipped type stays inside the 8192-word image.
                let looked = exw_link_lookup(&lnk, type_word).unwrap_or_else(|| {
                    panic!(
                        "{}: {mission}: type {type_word} outside the LNK image",
                        zone.identity
                    )
                });
                cw_lnk.insert(looked);
                let looked_lng = exw_link_lookup(&lng, type_word).unwrap_or_else(|| {
                    panic!(
                        "{}: {mission}: type {type_word} outside the LNG image",
                        zone.identity
                    )
                });
                cw_lng.insert(looked_lng);
                if looked != 0 {
                    mission_max = mission_max.max(looked);
                }
            }
            mission_max_cw.insert(format!("{}/{}", zone.identity, mission), mission_max);
            total_missions += 1;
        }

        // The stale-tail-never-read bound: EVERY nonzero reachable cw
        // under BOTH language gates must address a whole 16-B entry
        // inside the file prefix.
        for (label, set) in [("LNK", &cw_lnk), ("LNG", &cw_lng)] {
            for &cw in set.iter().filter(|cw| **cw != 0) {
                assert!(
                    exw_mask_entry(prefix, cw).is_some(),
                    "{}: reachable {label} cw {cw} reads past the {}-byte file prefix \
                     into the stale arena tail",
                    zone.identity,
                    prefix.len()
                );
            }
        }

        let union: BTreeSet<u16> = cw_lnk.union(&cw_lng).copied().collect();
        let nz_union: Vec<u16> = union.iter().copied().filter(|cw| *cw != 0).collect();
        let zero_entries = nz_union
            .iter()
            .filter(|cw| exw_mask_entry(prefix, **cw).expect("bounded entry") == [0; ENTRY_LEN])
            .count();
        let mut distinct: BTreeSet<u8> = BTreeSet::new();
        for cw in &nz_union {
            distinct.extend(exw_mask_entry(prefix, *cw).expect("bounded entry"));
        }
        censuses.push(ZoneCensus {
            letter: zone.letter,
            min_len: prefix.len(),
            entries: prefix.len() / ENTRY_LEN,
            lnk_reachable: cw_lnk.iter().filter(|cw| **cw != 0).count(),
            lng_reachable: cw_lng.iter().filter(|cw| **cw != 0).count(),
            union_reachable: nz_union.len(),
            max_cw_lnk: cw_lnk
                .iter()
                .copied()
                .filter(|cw| *cw != 0)
                .max()
                .unwrap_or(0),
            max_cw_lng: cw_lng
                .iter()
                .copied()
                .filter(|cw| *cw != 0)
                .max()
                .unwrap_or(0),
            zero_entries,
            distinct_bytes: distinct.len(),
            max_byte: distinct.iter().copied().max().unwrap_or(0),
        });
    }

    assert_eq!(total_missions, SHIPPED_MISSION_COUNT, "mission coverage");
    assert_eq!(
        max_type_corpus, 1868,
        "corpus TOT type maximum (well inside the 8192-word LNK/LNG image)"
    );

    // Pinned corpus census (RE-EXW-SIM §7j.62 D; independently
    // recomputed by the oracle above — a corpus identity regression or
    // a transcription bug breaks one of the two sides).
    let expected_census: BTreeMap<char, CensusPin> = [
        (
            'A',
            CensusPin::new(23200, 1450, 349, 337, 349, 1356, 1356, 9, 119, 254),
        ),
        (
            'B',
            CensusPin::new(29952, 1872, 1180, 1146, 1180, 1868, 1868, 11, 181, 254),
        ),
        (
            'C',
            CensusPin::new(27888, 1743, 1054, 1026, 1055, 1741, 1706, 12, 180, 254),
        ),
        (
            'D',
            CensusPin::new(23200, 1450, 1008, 988, 1008, 1356, 1356, 10, 176, 254),
        ),
        (
            'E',
            CensusPin::new(23280, 1455, 949, 929, 954, 1398, 1400, 9, 154, 254),
        ),
        (
            'F',
            CensusPin::new(15824, 989, 632, 632, 632, 960, 960, 9, 170, 254),
        ),
        (
            'G',
            CensusPin::new(29952, 1872, 271, 271, 271, 1834, 1834, 2, 100, 223),
        ),
    ]
    .into_iter()
    .collect();
    for census in &censuses {
        let want = expected_census
            .get(&census.letter)
            .unwrap_or_else(|| panic!("unexpected zone {}", census.letter));
        assert_eq!(
            census_of(census),
            *want,
            "ZONE{} census mismatch",
            census.letter
        );
        assert!(
            usize::from(census.max_cw_lnk) * 16 + 16 <= census.min_len,
            "ZONE{}: tightest LNK bound",
            census.letter
        );
    }

    // ZONEA ≡ ZONED byte-for-byte (the one shared-content pair).
    assert_eq!(
        min_bytes[&'A'], min_bytes[&'D'],
        "ZONEA/MISSIONA.MIN and ZONED/MISSIOND.MIN are byte-identical"
    );

    // The language gate is not cosmetic: LNK and LNG reach different
    // entries (and ZONEE's union exceeds both single-gate sets).
    let zone_e = censuses.iter().find(|c| c.letter == 'E').expect("ZONEE");
    assert!(
        zone_e.union_reachable > zone_e.lnk_reachable
            && zone_e.union_reachable > zone_e.lng_reachable,
        "ZONEE union reachable set exceeds both single-gate sets"
    );
    let zone_a = censuses.iter().find(|c| c.letter == 'A').expect("ZONEA");
    assert_ne!(
        zone_a.lnk_reachable, zone_a.lng_reachable,
        "ZONEA LNK vs LNG reachable sets differ"
    );

    // Per-mission max reachable cw (LNK gate) — the mission-granular
    // corpus identity pin (§7j.62 D).
    let expected_mission_max: BTreeMap<String, u16> = [
        ("ZONEA/MISSION1", 1356u16),
        ("ZONEB/MISSION1", 1868),
        ("ZONEB/MISSION2", 1868),
        ("ZONEB/MISSION3", 1868),
        ("ZONEB/MISSION4", 1868),
        ("ZONEB/MISSION5", 1868),
        ("ZONEB/MISSION6", 1812),
        ("ZONEB/MISSION7", 1814),
        ("ZONEC/MISSION1", 1706),
        ("ZONEC/MISSION2", 1706),
        ("ZONEC/MISSION3", 1706),
        ("ZONEC/MISSION4", 1706),
        ("ZONEC/MISSION5", 1741),
        ("ZONEC/MISSION6", 1633),
        ("ZONEC/MISSION7", 1633),
        ("ZONED/MISSION1", 1356),
        ("ZONED/MISSION2", 1356),
        ("ZONED/MISSION3", 1356),
        ("ZONED/MISSION4", 1356),
        ("ZONED/MISSION5", 1356),
        ("ZONED/MISSION6", 1344),
        ("ZONED/MISSION7", 1344),
        ("ZONEE/MISSION1", 1398),
        ("ZONEE/MISSION2", 1390),
        ("ZONEE/MISSION3", 1384),
        ("ZONEE/MISSION4", 1361),
        ("ZONEE/MISSION5", 1375),
        ("ZONEE/MISSION6", 1347),
        ("ZONEE/MISSION7", 1347),
        ("ZONEF/MISSION1", 960),
        ("ZONEF/MISSION2", 960),
        ("ZONEF/MISSION3", 960),
        ("ZONEF/MISSION4", 960),
        ("ZONEF/MISSION5", 960),
        ("ZONEF/MISSION6", 915),
        ("ZONEF/MISSION7", 809),
        ("ZONEG/MISSION1", 1834),
    ]
    .into_iter()
    .map(|(identity, max)| (identity.to_owned(), max))
    .collect();
    assert_eq!(mission_max_cw, expected_mission_max, "per-mission max cw");

    // Reader transcription spot-proof (ZONEA, first nonzero reachable
    // cw): the stamp writes exactly the nonzero mask bytes at their
    // (row*640+col) offsets and leaves transparent cells untouched.
    let prefix_a = exw_min_bank_prefix(&min_bytes[&'A']).expect("ZONEA prefix");
    let mut first_nz = None;
    for entry in 1..(prefix_a.len() / ENTRY_LEN) as u16 {
        if exw_mask_entry(prefix_a, entry).expect("entry") != [0; ENTRY_LEN] {
            first_nz = Some(entry);
            break;
        }
    }
    let cw = first_nz.expect("a nonzero entry early in ZONEA's bank");
    let stamped = exw_territory_stamp(prefix_a, cw).expect("stamped entry");
    let entry = exw_mask_entry(prefix_a, cw).expect("entry");
    for row in 0..4usize {
        for col in 0..4usize {
            let want = (entry[row * 4 + col] != 0).then_some(entry[row * 4 + col]);
            assert_eq!(
                stamped[row * SCREEN_STRIDE + col],
                want,
                "stamp pixel ({row},{col}) of ZONEA entry {cw}"
            );
        }
    }
    // Everything outside the 4×4 block is untouched.
    assert!(
        stamped
            .iter()
            .enumerate()
            .filter(|(offset, pixel)| {
                pixel.is_some() && !(offset / SCREEN_STRIDE < 4 && offset % SCREEN_STRIDE < 4)
            })
            .count()
            == 0,
        "no writes outside the 4x4 stamp block"
    );

    // The documented Rust absence: bedlam-core exposes no `.MIN`
    // surface at all (presentation-half, D17). Nothing to compare
    // against — this row's parity status is the original-side pin plus
    // the queued display-phase gap, never a fabricated bank.
}

/// Sensitivity proof for the oracle (temporary in-memory mutations; the
/// corpus on disk is never touched):
/// 1. a byte flip inside a REACHABLE mask entry moves exactly that
///    stamp pixel;
/// 2. a byte flip in an entry beyond the max reachable cw changes NO
///    reachable surface (the loader copies it, the runtime never reads
///    it — the dead-tail proof);
/// 3. repointing a live LNK lookup at an entry past the file prefix is
///    caught by the projection bound (the would-be stale read);
/// 4. a synthetic >0x7530 file is rejected (the original's uncapped
///    read would clobber the arena — never shipped, fails loudly).
#[test]
fn min_bank_oracle_is_sensitive_to_reachable_bytes_and_blind_to_the_tail() {
    let zone = editor_root().join("ZONEA");
    let min_path = zone.join("MISSIONA.MIN");
    let lnk_path = zone.join("MISSIONA.LNK");
    let tot_path = zone.join("MISSION1.TOT");
    if !min_path.is_file() || !lnk_path.is_file() || !tot_path.is_file() {
        eprintln!("game-data corpus not found - skipping");
        return;
    }
    let min = fs::read(&min_path).expect("read ZONEA/MISSIONA.MIN");
    let mut lnk = fs::read(&lnk_path).expect("read ZONEA/MISSIONA.LNK");
    let tot = fs::read(&tot_path).expect("read ZONEA/MISSION1.TOT");
    let prefix = exw_min_bank_prefix(&min).expect("ZONEA prefix");
    let entries = (prefix.len() / ENTRY_LEN) as u16;

    // Reachable set of this mission under the LNK gate.
    let words = exw_tot_words(&tot).expect("ZONEA TOT");
    let types: BTreeSet<u16> = words.iter().copied().collect();
    let cw_set: BTreeSet<u16> = types
        .iter()
        .filter_map(|t| exw_link_lookup(&lnk, *t))
        .collect();
    let max_cw = cw_set
        .iter()
        .copied()
        .filter(|cw| *cw != 0)
        .max()
        .expect("nz cw");
    assert_eq!(max_cw, 1356, "ZONEA/MISSION1 max reachable cw");

    // (1) flip the mask byte at (row 1, col 1) of the entry for the
    // smallest nonzero reachable cw: exactly that stamp pixel moves.
    let cw = *cw_set
        .iter()
        .filter(|cw| **cw != 0)
        .min()
        .expect("min nz cw");
    let entry_offset = usize::from(cw) * ENTRY_LEN;
    let mut mutated = min.clone();
    mutated[entry_offset + 5] ^= 0x01;
    let mutated_prefix = exw_min_bank_prefix(&mutated).expect("mutated prefix");
    let before = exw_territory_stamp(prefix, cw).expect("stamp before");
    let after = exw_territory_stamp(mutated_prefix, cw).expect("stamp after");
    let differing: Vec<usize> = (0..before.len())
        .filter(|&offset| before[offset] != after[offset])
        .collect();
    assert_eq!(
        differing,
        vec![SCREEN_STRIDE + 1],
        "a mask-byte flip at entry+5 (row 1, col 1) moves exactly that stamp pixel"
    );

    // (2) flip a byte of an entry BEYOND the max reachable cw (the
    // loaded-but-dead tail): no reachable entry changes.
    let dead_cw = entries - 1;
    assert!(
        dead_cw > max_cw,
        "the last entry lies beyond the reachable max"
    );
    assert!(
        !cw_set.contains(&dead_cw),
        "the last entry is not reachable itself"
    );
    let mut dead_mutated = min.clone();
    let dead_offset = usize::from(dead_cw) * ENTRY_LEN;
    dead_mutated[dead_offset] ^= 0xFF;
    let dead_prefix = exw_min_bank_prefix(&dead_mutated).expect("dead prefix");
    for reachable in cw_set.iter().filter(|cw| **cw != 0) {
        assert_eq!(
            exw_mask_entry(prefix, *reachable),
            exw_mask_entry(dead_prefix, *reachable),
            "dead-tail flip must not touch reachable entry {reachable}"
        );
    }

    // (3) repoint ONE live LNK lookup (a type present in the TOT) at an
    // entry one past the file: the projection must flag the stale read.
    let live_type = *types
        .iter()
        .find(|t| exw_link_lookup(&lnk, **t) == Some(cw))
        .expect("the type mapping to the min nz cw");
    lnk[2 * usize::from(live_type)..2 * usize::from(live_type) + 2]
        .copy_from_slice(&entries.to_le_bytes());
    let poisoned: BTreeSet<u16> = types
        .iter()
        .filter_map(|t| exw_link_lookup(&lnk, *t))
        .collect();
    assert_eq!(
        poisoned.iter().copied().filter(|cw| *cw != 0).max(),
        Some(entries),
        "the poisoned lookup now addresses one past the file"
    );
    assert!(
        exw_mask_entry(prefix, entries).is_none(),
        "an entry one past the {}-byte file is a stale-tail read",
        prefix.len()
    );

    // (4) a synthetic over-bank file is rejected loudly.
    let oversize = vec![0u8; BANK_LEN + 1];
    assert!(
        exw_min_bank_prefix(&oversize).is_err(),
        "the oracle rejects an over-bank MIN file (uncapped in the original)"
    );
}
