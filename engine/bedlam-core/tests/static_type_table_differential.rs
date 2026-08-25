//! Whole-corpus static differential oracle for the .BDG type-table
//! staging — diffharness registry row `static-type-table`
//! (EXW 0x4dedf2 / EXD 0x108428, 282 × 0x4E rows).
//!
//! Expected side: an independent byte-level transcription of the EXW
//! loader leg in `FUN_0041a4f8` @0x41a5d6..0x41a7ef (re-verified
//! instruction-by-instruction 2026-08-25, docs/RE-EXW-SIM.md §7j.61):
//! the whole 21996-B table AND 0x9C40 B of the bank arena are
//! memset-0 before every load; exactly 282 records are walked; the
//! raw control word is staged at row+0 BEFORE the `== 1` test (≠1 →
//! a 2-byte empty row whose +2..+0x4E stays memset-0 — head, count,
//! effects, and all four bank pointer slots); active rows stage
//! W/H/D u16 @+2/+4/+6, hp i32 @+8, chain u16 @+0xC, type i32
//! @+0xE, five 8-B effect entries @+0x16+8m, then read FOUR banks of
//! 2·W·H·D bytes each into CONSECUTIVE arena slots in DISK ORDER
//! (cursor += bank bytes per read), storing the slot pointers in the
//! interleaved row order +0x3E ← bank 1 (CURRENT TOT), +0x46 ←
//! bank 2 (UNDER TOT), +0x42 ← bank 3 (CURRENT DAT), +0x4A ←
//! bank 4 (UNDER DAT) [§7j.32/1]. After the banks, `count@+0x12 :=
//! number of NONZERO staged selectors` is computed on ACTIVE rows
//! only (empty rows keep the memset 0).
//!
//! Actual side: the Rust target's retained bank — the production
//! `ObjectTypeTable::from_bdg_bytes` (staged verbatim into
//! `MissionSim::object_types` by `stage_destroy_family`). Two
//! original surfaces are deliberately NOT retained and no seam is
//! fabricated for them: the count word (write-only in the original —
//! displacement 0x4dee04 has exactly ONE .text site, the loader
//! store @0x41a61d, §7j.61/B) and the staged control word @+0
//! (write-only; the 0/1 classification is retained instead). The
//! count is asserted here as a pure function of the RETAINED effect
//! selectors — the derivation identity the original computes — and
//! the control word as the corpus-pinned 0/1 classification.
//!
//! Scope: valid shipped corpus only. Not a malformed-input spec. No
//! production parser, loader, or destroy-family helper is reused on
//! the expected side (bytes only).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::destroy::{ObjectType, ObjectTypeTable};

const SHIPPED_MISSION_COUNT: usize = 37;
/// 0x11A — the loader's record cap (EXW `cmp ecx,0x11a` @0x41a638;
/// EXD bounds the row offset at 0x55EC = 282·ROW_BYTES, same count).
const OBJECT_TYPE_SLOTS: usize = 282;
/// The in-memory row stride; 282·ROW_BYTES = 0x55EC is exactly the
/// loader's table memset span (§7j.61/A1 — pinned in the corpus gate).
const ROW_BYTES: usize = 0x4E;
/// The arena memset bound the loader pre-clears (0x9C40).
const ARENA_MEMSET_BYTES: usize = 0x9C40;

#[derive(Debug)]
struct MissionFiles {
    identity: String,
    bdg: PathBuf,
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
/// gates.
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

        let mut bdg_files: Vec<PathBuf> = fs::read_dir(&zone_dir)
            .unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("bdg"))
                    && path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .is_some_and(is_numbered_mission_stem)
            })
            .collect();
        bdg_files.sort();

        for bdg in bdg_files {
            let stem = bdg
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("mission BDG has a UTF-8 stem");
            missions.push(MissionFiles {
                identity: format!("{zone_name}/{stem}"),
                bdg,
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

/// One staged row as the EXW loader leaves it (§7j.61), with the four
/// bank pointer slots represented by their ARENA OFFSETS — the
/// reproducible content of a pointer (the arena base address itself is
/// environment state, not staged data).
#[derive(Debug, Clone, Default)]
struct OracleRow {
    /// The raw disk control word, staged at +0 before the ==1 test.
    control: u16,
    active: bool,
    w: u16,
    h: u16,
    d: u16,
    hp: i32,
    chain: u16,
    kind: i32,
    /// The load-computed count@+0x12 (nonzero-selector count) —
    /// computed on ACTIVE rows only; empty rows keep the memset 0.
    count: u16,
    /// (selector, dx, dy, dz) per effect entry, disk order.
    effects: [[u16; 4]; 5],
    /// The four banks in DISK ORDER, as u16 cell words.
    banks: [Vec<u16>; 4],
    /// Arena offsets stored into the row's pointer slots
    /// +0x3E/+0x46/+0x42/+0x4A respectively (the §7j.32 interleave).
    bank_slot_arena_offsets: [usize; 4],
}

/// The staged image of one mission's type table + arena.
#[derive(Debug, Default)]
struct OracleTable {
    rows: Vec<OracleRow>,
    /// Total arena bytes consumed (4 banks per active row).
    arena_bytes: usize,
}

fn rd16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

fn rd32(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(at..at + 4)?.try_into().expect("i32 head field"),
    ))
}

/// Independent transcription of the EXW .BDG staging loop. Returns
/// `Err` on any shipped-corpus precondition violation: a truncated
/// head/bank (the walk would read past EOF), or a desynced grammar
/// (the 282-record walk does not end exactly at EOF). The corpus is
/// EOF-exact on all 37 files, so any Err is a walk/grammar divergence,
/// never shipped data.
fn exw_staging_oracle(bytes: &[u8]) -> Result<OracleTable, String> {
    let mut rows: Vec<OracleRow> = Vec::with_capacity(OBJECT_TYPE_SLOTS);
    let mut pos = 0usize;
    let mut arena_cursor = 0usize;
    while rows.len() < OBJECT_TYPE_SLOTS {
        if pos == bytes.len() {
            return Err(format!(
                "shipped-corpus precondition: file ends after {} records (need exactly {})",
                rows.len(),
                OBJECT_TYPE_SLOTS
            ));
        }
        // The control read lands at row+0 BEFORE the ==1 test (§7j.61/A3).
        let control =
            rd16(bytes, pos).ok_or_else(|| format!("truncated control word at byte {pos}"))?;
        if control != 1 {
            // Empty row: 2 disk bytes; +2..+0x4E of the row stays memset-0
            // (head, count, effects, NULL bank slots). The count loop is
            // skipped on this path (jne 0x41a623 bypasses it).
            rows.push(OracleRow {
                control,
                ..OracleRow::default()
            });
            pos += 2;
            continue;
        }
        let w = rd16(bytes, pos + 0x02).ok_or_else(|| format!("truncated W at {pos}"))?;
        let h = rd16(bytes, pos + 0x04).ok_or_else(|| format!("truncated H at {pos}"))?;
        let d = rd16(bytes, pos + 0x06).ok_or_else(|| format!("truncated D at {pos}"))?;
        let hp = rd32(bytes, pos + 0x08).ok_or_else(|| format!("truncated hp at {pos}"))?;
        let chain = rd16(bytes, pos + 0x0C).ok_or_else(|| format!("truncated chain at {pos}"))?;
        let kind = rd32(bytes, pos + 0x0E).ok_or_else(|| format!("truncated type at {pos}"))?;
        let mut effects = [[0u16; 4]; 5];
        for (m, entry) in effects.iter_mut().enumerate() {
            let base = pos + 0x12 + 8 * m;
            for (word, field) in entry.iter_mut().enumerate() {
                *field = rd16(bytes, base + 2 * word)
                    .ok_or_else(|| format!("truncated effect {m} at {base}"))?;
            }
        }
        // Bank byte count = 2*W*H*D, recomputed from the staged words.
        let cells = w as usize * h as usize * d as usize;
        let bank_bytes = cells * 2;
        if pos + 0x3A + 4 * bank_bytes > bytes.len() {
            return Err(format!(
                "truncated template banks at {pos} (needs {} bank bytes)",
                4 * bank_bytes
            ));
        }
        // Four consecutive arena reads in DISK ORDER; the slot stores
        // interleave: +0x3E <- bank1, +0x46 <- bank2, +0x42 <- bank3,
        // +0x4A <- bank4 (§7j.61/A7).
        let mut banks = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        let mut offsets = [0usize; 4];
        for bank_index in 0..4 {
            let start = pos + 0x3A + bank_index * bank_bytes;
            banks[bank_index] = (0..cells)
                .map(|cell| rd16(bytes, start + 2 * cell).expect("bank word in range"))
                .collect();
            offsets[bank_index] = arena_cursor;
            arena_cursor += bank_bytes;
        }
        // count@+0x12 := nonzero staged selectors (ACTIVE rows only).
        let count = effects
            .iter()
            .filter(|entry| entry[0] != 0)
            .count()
            .min(u16::MAX as usize) as u16;
        rows.push(OracleRow {
            control,
            active: true,
            w,
            h,
            d,
            hp,
            chain,
            kind,
            count,
            effects,
            banks,
            bank_slot_arena_offsets: offsets,
        });
        pos += 0x3A + 4 * bank_bytes;
    }
    if pos != bytes.len() {
        return Err(format!(
            "grammar desync: the 282-record walk ended at byte {pos} of {} \
             (the original's bounded loop would misread every later record)",
            bytes.len()
        ));
    }
    Ok(OracleTable {
        rows,
        arena_bytes: arena_cursor,
    })
}

fn oracle_count_of(actual_row: &ObjectType) -> u16 {
    actual_row
        .effects
        .iter()
        .filter(|entry| entry.selector != 0)
        .count()
        .min(u16::MAX as usize) as u16
}

/// Field-exact comparison of the staged image against the Rust target's
/// retained row bank, naming the first differing field. The count word
/// is compared through the derivation identity (the original's count is
/// a pure function of the retained selectors — §7j.61/E), and the raw
/// control word through the corpus-pinned 0/1 classification.
fn assert_rows_match(identity: &str, oracle: &OracleTable, actual: &ObjectTypeTable) {
    assert_eq!(
        oracle.rows.len(),
        OBJECT_TYPE_SLOTS,
        "{identity}: oracle walked exactly 282 records"
    );
    assert_eq!(
        actual.rows.len(),
        OBJECT_TYPE_SLOTS,
        "{identity}: Rust target retains exactly 282 rows"
    );
    for (index, (expected, staged)) in oracle.rows.iter().zip(&actual.rows).enumerate() {
        if !expected.active {
            assert_eq!(
                expected.control, 0,
                "{identity}: row {index}: corpus empty rows carry control word 0"
            );
            assert_eq!(
                staged,
                &ObjectType::default(),
                "{identity}: row {index}: an empty row must stage as the memset-0 default"
            );
            continue;
        }
        let label = format!("{identity}: row {index}");
        assert_eq!(staged.w, expected.w, "{label}: W");
        assert_eq!(staged.h, expected.h, "{label}: H");
        assert_eq!(staged.d, expected.d, "{label}: D");
        assert_eq!(staged.hp, expected.hp, "{label}: hp");
        assert_eq!(staged.chain, expected.chain, "{label}: chain");
        assert_eq!(staged.kind, expected.kind, "{label}: type");
        for (m, (expected_entry, staged_entry)) in
            expected.effects.iter().zip(&staged.effects).enumerate()
        {
            assert_eq!(
                staged_entry.selector, expected_entry[0],
                "{label}: effect {m} selector"
            );
            assert_eq!(staged_entry.dx, expected_entry[1], "{label}: effect {m} dx");
            assert_eq!(staged_entry.dy, expected_entry[2], "{label}: effect {m} dy");
            assert_eq!(staged_entry.dz, expected_entry[3], "{label}: effect {m} dz");
        }
        // The §7j.32 disk->slot mapping: disk bank 1 = CURRENT TOT,
        // 2 = UNDER TOT, 3 = CURRENT DAT, 4 = UNDER DAT.
        assert_eq!(
            staged.bank_current_tot, expected.banks[0],
            "{label}: bank slot +0x3E (CURRENT TOT) must hold disk bank 1"
        );
        assert_eq!(
            staged.bank_under_tot, expected.banks[1],
            "{label}: bank slot +0x46 (UNDER TOT) must hold disk bank 2"
        );
        assert_eq!(
            staged.bank_current_dat, expected.banks[2],
            "{label}: bank slot +0x42 (CURRENT DAT) must hold disk bank 3"
        );
        assert_eq!(
            staged.bank_under_dat, expected.banks[3],
            "{label}: bank slot +0x4A (UNDER DAT) must hold disk bank 4"
        );
        // The two deliberately-unretained surfaces: the count word
        // (write-only in the original) and the control word@+0. The
        // count is pinned through the derivation identity; the control
        // through the classification.
        assert_eq!(
            staged.count, 0,
            "{label}: the Rust target deliberately leaves count unretained (0)"
        );
        assert_eq!(
            expected.count,
            oracle_count_of(staged),
            "{label}: the original's count@+0x12 == the nonzero-selector count of the RETAINED effects"
        );
    }
}

/// The arena interleave arithmetic: consecutive disk-order slots per
/// active row, cursor advancing by one bank per read (§7j.61/A7).
fn assert_arena_layout(identity: &str, oracle: &OracleTable) {
    let mut cursor = 0usize;
    for (index, row) in oracle.rows.iter().enumerate() {
        if !row.active {
            continue;
        }
        let cells = row.w as usize * row.h as usize * row.d as usize;
        let bank_bytes = cells * 2;
        assert_eq!(
            row.bank_slot_arena_offsets,
            [
                cursor,
                cursor + bank_bytes,
                cursor + 2 * bank_bytes,
                cursor + 3 * bank_bytes
            ],
            "{identity}: row {index}: the four slot pointers address consecutive \
             disk-order arena slots"
        );
        cursor += 4 * bank_bytes;
    }
    assert_eq!(
        cursor, oracle.arena_bytes,
        "{identity}: arena cursor ends at the total bank bytes"
    );
    assert!(
        oracle.arena_bytes < ARENA_MEMSET_BYTES,
        "{identity}: arena span {} stays under the 0x9C40 memset bound",
        oracle.arena_bytes
    );
}

#[test]
fn all_missions_type_table_matches_exw_staging_oracle() {
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

    // Corpus-wide census recomputed independently by the oracle run and
    // pinned against the §7j.61/D numbers.
    let mut total_rows = 0usize;
    let mut active_rows = 0usize;
    let mut empty_rows = 0usize;
    let mut dims: BTreeSet<(u16, u16, u16)> = BTreeSet::new();
    let mut selector_census: BTreeMap<u16, usize> = BTreeMap::new();
    let mut count_census: BTreeMap<u16, usize> = BTreeMap::new();
    let mut arena_bytes_by_mission: BTreeMap<String, usize> = BTreeMap::new();
    let mut hp_min = i32::MAX;
    let mut chain_domain = BTreeSet::new();
    let mut banks_differ = 0usize;
    let mut per_mission_active: BTreeMap<String, usize> = BTreeMap::new();

    for mission in &missions {
        let bytes = fs::read(&mission.bdg)
            .unwrap_or_else(|error| panic!("{}: read BDG: {error}", mission.identity));
        let oracle = exw_staging_oracle(&bytes).unwrap_or_else(|error| {
            panic!("{}: oracle rejected corpus: {error}", mission.identity)
        });
        let actual = ObjectTypeTable::from_bdg_bytes(&bytes)
            .unwrap_or_else(|| panic!("{}: Rust target rejected corpus", mission.identity));

        assert_rows_match(&mission.identity, &oracle, &actual);
        assert_arena_layout(&mission.identity, &oracle);

        let active = oracle.rows.iter().filter(|row| row.active).count();
        total_rows += oracle.rows.len();
        active_rows += active;
        empty_rows += oracle.rows.len() - active;
        per_mission_active.insert(mission.identity.clone(), active);
        arena_bytes_by_mission.insert(mission.identity.clone(), oracle.arena_bytes);

        for row in &oracle.rows {
            if !row.active {
                continue;
            }
            dims.insert((row.w, row.h, row.d));
            hp_min = hp_min.min(row.hp);
            chain_domain.insert(row.chain);
            *count_census.entry(row.count).or_insert(0) += 1;
            for entry in &row.effects {
                *selector_census.entry(entry[0]).or_insert(0) += 1;
            }
            if !(row.banks[0] == row.banks[1]
                && row.banks[1] == row.banks[2]
                && row.banks[2] == row.banks[3])
            {
                banks_differ += 1;
            }
        }
    }

    // Whole-corpus pins (§7j.61/D).
    assert_eq!(
        OBJECT_TYPE_SLOTS * ROW_BYTES,
        0x55EC,
        "the loader's table memset span is exactly 282 x 0x4E bytes"
    );
    assert_eq!(total_rows, 10434, "37 files x exactly 282 records");
    assert_eq!(active_rows, 7907, "active rows corpus-wide");
    assert_eq!(empty_rows, 2527, "empty rows corpus-wide (all control 0)");
    assert_eq!(dims.len(), 113, "distinct footprint tuples");
    assert_eq!(*dims.iter().max().unwrap(), (10, 10, 5), "max footprint");
    assert_eq!(
        selector_census.get(&0).copied().unwrap_or(0),
        23976,
        "zero-selector entries"
    );
    let mut selector_domain: Vec<u16> = selector_census.keys().copied().collect();
    selector_domain.sort();
    assert_eq!(
        selector_domain,
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        "selectors on disk are 0 and exactly 1..9"
    );
    assert_eq!(
        selector_census.get(&1).copied().unwrap_or(0),
        11098,
        "selector-1 entries"
    );
    assert_eq!(
        selector_census.get(&9).copied().unwrap_or(0),
        56,
        "selector-9 entries"
    );
    let mut expected_count_census = BTreeMap::new();
    for (count, entries) in [
        (0u16, 554usize),
        (1, 3755),
        (2, 1304),
        (3, 884),
        (4, 506),
        (5, 904),
    ] {
        expected_count_census.insert(count, entries);
    }
    assert_eq!(
        count_census, expected_count_census,
        "nonzero-selector count census over active rows (554 active rows stage count 0)"
    );
    assert_eq!(
        hp_min, -1,
        "negative hp exists on disk (the i32 field is signed)"
    );
    assert_eq!(
        chain_domain,
        BTreeSet::from([0u16, 1]),
        "chain domain is {{0,1}}"
    );
    assert_eq!(
        banks_differ, 7904,
        "rows whose four banks are not all identical (the interleave check has teeth)"
    );
    let arena_spans: Vec<usize> = arena_bytes_by_mission.values().copied().collect();
    assert_eq!(
        arena_spans.iter().min().copied().unwrap(),
        6728,
        "smallest arena span"
    );
    assert_eq!(
        arena_bytes_by_mission["ZONEF/MISSION1"], 27288,
        "largest arena span is ZONEF/MISSION1"
    );
    assert_eq!(
        arena_spans.iter().max().copied().unwrap(),
        27288,
        "largest arena span value"
    );

    // Per-mission anchors.
    assert_eq!(
        per_mission_active["ZONEA/MISSION1"], 197,
        "ZONEA/MISSION1 active rows"
    );
    let zonea = fs::read(editor_root().join("ZONEA/MISSION1.BDG")).expect("ZONEA/MISSION1.BDG");
    let oracle = exw_staging_oracle(&zonea).expect("ZONEA/MISSION1 oracle");
    let record0 = &oracle.rows[0];
    assert!(record0.active, "ZONEA/MISSION1 record 0 is active");
    assert_eq!(
        (record0.w, record0.h, record0.d),
        (1, 1, 1),
        "record 0 dims"
    );
    assert_eq!(record0.hp, 150, "record 0 hp");
    assert_eq!(record0.chain, 1, "record 0 chain");
    assert_eq!(record0.kind, 15, "record 0 type");
    assert_eq!(
        record0.effects,
        [[1, 0, 0, 0], [0; 4], [0; 4], [0; 4], [0; 4]],
        "record 0 carries exactly one effect entry"
    );
    assert_eq!(record0.count, 1, "record 0 count");
    assert_eq!(
        (
            record0.banks[0].as_slice(),
            record0.banks[1].as_slice(),
            record0.banks[2].as_slice(),
            record0.banks[3].as_slice()
        ),
        (&[53u16][..], &[1189][..], &[2][..], &[0][..]),
        "record 0 four bank words (CURRENT TOT / UNDER TOT / CURRENT DAT / UNDER DAT)"
    );
    assert_eq!(
        record0.bank_slot_arena_offsets,
        [0, 2, 4, 6],
        "record 0 owns the arena head with consecutive slots"
    );
    let record19 = &oracle.rows[19];
    assert!(
        record19.effects.iter().all(|entry| entry[0] == 1),
        "record 19 carries five selector-1 entries"
    );
    assert_eq!(record19.count, 5, "record 19 count");
    let zero_count_active = oracle
        .rows
        .iter()
        .filter(|row| row.active && row.count == 0)
        .count();
    assert_eq!(
        zero_count_active, 26,
        "ZONEA/MISSION1 active rows with count 0"
    );

    let zonef = fs::read(editor_root().join("ZONEF/MISSION1.BDG")).expect("ZONEF/MISSION1.BDG");
    let oracle_f = exw_staging_oracle(&zonef).expect("ZONEF/MISSION1 oracle");
    let record184 = &oracle_f.rows[184];
    assert_eq!(
        (record184.w, record184.h, record184.d),
        (10, 10, 5),
        "the corpus-max footprint record (FORMATS §16 erratum, §7j.61/D)"
    );
    assert_eq!(
        record184.banks.iter().map(Vec::len).collect::<Vec<usize>>(),
        vec![500; 4],
        "the (10,10,5) record stages four 500-word banks"
    );
}

/// Sensitivity proof for the oracle (temporary in-memory mutations; the
/// corpus on disk is never touched):
/// 1. a bank byte bump inside record 0's CURRENT-TOT bank moves exactly
///    that staged word in BOTH the oracle and the Rust target — the two
///    keep agreeing on the mutated input while the differential against
///    the un-mutated side fails (the bank content is load-bearing, and
///    the slot mapping cannot absorb it: record 0's four words are
///    pairwise distinct);
/// 2. an hp byte bump moves the staged hp in both (head fields are
///    load-bearing too);
/// 3. a selector value bump 1 -> 2 moves the staged selector in both
///    while the DERIVED count stays 1 (the count identity sees only
///    selector presence, never the value);
/// 4. a control flip 1 -> 0 on an active row DESYNCS the grammar — the
///    oracle's EOF-exact walk fails and the Rust target returns None
///    (both fail loud; the original would misread every later record);
/// 5. a control flip 0 -> 1 on an empty row makes the walk read a bogus
///    head past EOF — both sides reject again;
/// 6. trailing bytes: the Rust target rejects (stricter than the
///    original, whose bounded 282-record loop never reads them; the
///    corpus is EOF-exact so this is unreachable shipped behavior).
#[test]
fn type_table_oracle_is_sensitive_to_field_and_structure_mutations() {
    let zonea_path = editor_root().join("ZONEA/MISSION1.BDG");
    if !zonea_path.is_file() {
        eprintln!("game-data corpus not found - skipping");
        return;
    }
    let clean_bytes = fs::read(&zonea_path).expect("read ZONEA/MISSION1.BDG");
    let clean_oracle = exw_staging_oracle(&clean_bytes).expect("clean oracle");
    let clean_actual = ObjectTypeTable::from_bdg_bytes(&clean_bytes).expect("clean target");

    // 1. Bank byte bump: record 0's bank 1 (CURRENT TOT) word 53 -> 54.
    //    Record 0's four bank words are pairwise distinct (53/1189/2/0),
    //    so no slot permutation can absorb this mutation.
    assert_ne!(
        (
            clean_oracle.rows[0].banks[0][0],
            clean_oracle.rows[0].banks[1][0],
            clean_oracle.rows[0].banks[2][0],
            clean_oracle.rows[0].banks[3][0]
        ),
        (54, 1189, 2, 0),
        "record 0 bank words are pairwise distinct pre-mutation"
    );
    let mut bank_mut = clean_bytes.clone();
    bank_mut[0x3A] = bank_mut[0x3A].wrapping_add(1);
    let bank_oracle = exw_staging_oracle(&bank_mut).expect("bank-mutated walk stays EOF-exact");
    let bank_actual = ObjectTypeTable::from_bdg_bytes(&bank_mut).expect("target accepts mutation");
    assert_eq!(
        bank_oracle.rows[0].banks[0][0], 54,
        "oracle staged the bump"
    );
    assert_eq!(
        bank_actual.rows[0].bank_current_tot,
        vec![54u16],
        "Rust target staged the bump in the +0x3E slot"
    );
    // The differential against the un-mutated side FAILS: the staged
    // bank word moved.
    assert_ne!(
        bank_actual.rows[0].bank_current_tot, clean_actual.rows[0].bank_current_tot,
        "the differential detects the bank bump"
    );
    assert_eq!(
        bank_actual.rows[0].bank_current_tot, bank_oracle.rows[0].banks[0],
        "mutated oracle and mutated target keep agreeing"
    );

    // 2. hp byte bump: 150 -> 151 (low byte of the i32 at +8).
    let mut hp_mut = clean_bytes.clone();
    hp_mut[0x08] = hp_mut[0x08].wrapping_add(1);
    let hp_oracle = exw_staging_oracle(&hp_mut).expect("hp-mutated walk stays EOF-exact");
    let hp_actual = ObjectTypeTable::from_bdg_bytes(&hp_mut).expect("target accepts hp mutation");
    assert_eq!(hp_oracle.rows[0].hp, 151, "oracle staged the hp bump");
    assert_eq!(hp_actual.rows[0].hp, 151, "target staged the hp bump");
    assert_eq!(hp_oracle.rows[0].count, clean_oracle.rows[0].count);
    assert_ne!(hp_actual.rows[0].hp, clean_actual.rows[0].hp);

    // 3. Selector value bump: record 0's effect 0 selector 1 -> 2.
    let mut sel_mut = clean_bytes.clone();
    sel_mut[0x12] = 2;
    let sel_oracle = exw_staging_oracle(&sel_mut).expect("selector-mutated walk stays EOF-exact");
    let sel_actual = ObjectTypeTable::from_bdg_bytes(&sel_mut).expect("target accepts selector");
    assert_eq!(
        sel_oracle.rows[0].effects[0][0], 2,
        "oracle staged the selector"
    );
    assert_eq!(
        sel_actual.rows[0].effects[0].selector, 2,
        "target staged it"
    );
    assert_eq!(
        sel_oracle.rows[0].count, 1,
        "the derived count is presence-only: 1 -> 2 keeps it 1"
    );
    assert_eq!(
        oracle_count_of(&sel_actual.rows[0]),
        1,
        "the count identity stays satisfied under the mutation"
    );
    assert_ne!(
        sel_actual.rows[0].effects[0], clean_actual.rows[0].effects[0],
        "the field differential detects the selector bump"
    );

    // 4. Control flip 1 -> 0 on record 0: the walk consumes 2 bytes for
    //    the now-empty row and reinterprets every later record against
    //    shifted bytes — the EOF-exact precondition fails (the walk ends
    //    early: 21940 != 21988), and the Rust target returns None.
    let mut ctrl_mut = clean_bytes.clone();
    ctrl_mut[0] = 0;
    assert!(
        exw_staging_oracle(&ctrl_mut).is_err(),
        "the oracle rejects the desynced grammar (282 records walked, EOF not reached exactly)"
    );
    assert!(
        ObjectTypeTable::from_bdg_bytes(&ctrl_mut).is_none(),
        "the Rust target rejects the desynced grammar"
    );

    // 5. Control flip 0 -> 1 on the FIRST EMPTY row: the bogus active
    //    row swallows the following empty rows' bytes (a 0x3A-byte read
    //    for what was a run of 2-byte rows), so the file runs OUT of
    //    records before 282 — the oracle's exactly-282 precondition
    //    rejects. The Rust target instead ACCEPTS the EOF-short walk
    //    (fewer than 282 rows): a divergence from the original, whose
    //    bounded 282-record loop would memset-pad the tail rows empty —
    //    unreachable on the shipped corpus (every file is EOF-exact at
    //    exactly 282), and still caught by the differential's row-extent
    //    check.
    let first_empty = clean_oracle
        .rows
        .iter()
        .position(|row| !row.active)
        .expect("ZONEA/MISSION1 has empty rows");
    // Recover that row's disk byte offset with a fresh bounded walk.
    let mut offset = 0usize;
    for row in &clean_oracle.rows[..first_empty] {
        offset += if row.active {
            0x3A + 8 * row.w as usize * row.h as usize * row.d as usize
        } else {
            2
        };
    }
    assert_eq!(
        rd16(&clean_bytes, offset),
        Some(0),
        "the first empty row's disk control is 0"
    );
    let mut empty_mut = clean_bytes.clone();
    empty_mut[offset] = 1;
    assert!(
        exw_staging_oracle(&empty_mut).is_err(),
        "the oracle rejects the record-short file (fewer than 282 records)"
    );
    let short_target = ObjectTypeTable::from_bdg_bytes(&empty_mut)
        .expect("the Rust target accepts an EOF-short walk (documented divergence)");
    assert!(
        short_target.rows.len() < OBJECT_TYPE_SLOTS,
        "the differential's row-extent check fails on the mutated input"
    );

    // 6. Trailing bytes: the Rust target fails loud where the original
    //    simply never reads them (bounded 282-record loop). Documented
    //    divergence; unreachable on the shipped corpus.
    let mut trailing = clean_bytes.clone();
    trailing.extend_from_slice(&[0, 0]);
    assert!(ObjectTypeTable::from_bdg_bytes(&trailing).is_none());
}
