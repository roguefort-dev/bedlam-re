//! Whole-corpus differential gate for the shipped runtime-selected zone CGR
//! transformation.
//!
//! Scope is limited to the seven lettered `ZONE?/MISSION?.CGR` banks selected
//! at runtime. This does not specify malformed-input behavior, RLE/editor
//! rendering, floor-search rules, or any numbered mission CGR.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::mission::Terrain;

const SHIPPED_CGR_LEN: usize = 132_354;
const SHIPPED_CGR_COUNT: usize = 128;
const HEIGHT_MAP_LEN: usize = 32 * 32;

#[derive(Debug)]
struct ZoneCgr {
    identity: String,
    path: PathBuf,
}

fn editor_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR")
}

fn expected_identities() -> BTreeSet<String> {
    ('A'..='G')
        .map(|zone| format!("ZONE{zone}/MISSION{zone}.CGR"))
        .collect()
}

fn is_lettered_mission_cgr(path: &Path) -> bool {
    if path.extension().and_then(|value| value.to_str()) != Some("CGR") {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(suffix) = stem.strip_prefix("MISSION") else {
        return false;
    };
    suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'A'..=b'G')
}

/// Discover only letter-selected CGRs; `MISSION1.CGR` and other numbered
/// mission banks are deliberately outside this gate.
fn shipped_zone_cgrs() -> Option<Vec<ZoneCgr>> {
    let root = editor_root();
    if !root.is_dir() {
        eprintln!("game-data corpus not found - skipping");
        return None;
    }

    let mut banks = Vec::new();
    for zone in 'A'..='G' {
        let zone_name = format!("ZONE{zone}");
        let zone_dir = root.join(&zone_name);
        let entries = fs::read_dir(&zone_dir)
            .unwrap_or_else(|error| panic!("read shipped {zone_name} directory: {error}"));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| panic!("read entry in shipped {zone_name}: {error}"))
                .path();
            if path.is_file() && is_lettered_mission_cgr(&path) {
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_else(|| panic!("{zone_name}: CGR file name is not UTF-8"));
                banks.push(ZoneCgr {
                    identity: format!("{zone_name}/{file_name}"),
                    path,
                });
            }
        }
    }
    banks.sort_by(|left, right| left.identity.cmp(&right.identity));
    Some(banks)
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("four-byte slice"),
    )
}

/// Independent byte transcription of the shipped runtime-selected CGR bank
/// layout. It intentionally shares no asset codec or production CGR helper.
fn exw_height_oracle(identity: &str, cgr: &[u8]) -> Vec<[u8; HEIGHT_MAP_LEN]> {
    assert_eq!(cgr.len(), SHIPPED_CGR_LEN, "{identity}: shipped CGR length");
    assert_eq!(
        u16::from_le_bytes([cgr[0], cgr[1]]) as usize,
        SHIPPED_CGR_COUNT,
        "{identity}: shipped CGR count"
    );

    let mut concatenated = Vec::with_capacity(SHIPPED_CGR_COUNT * HEIGHT_MAP_LEN);
    for slot in 0..SHIPPED_CGR_COUNT {
        let directory_slot = 2 + 4 * slot;
        let stored_offset = le_u32(cgr, directory_slot) as usize;
        let expected_offset = 512 + 1026 * slot;
        assert_eq!(
            stored_offset, expected_offset,
            "{identity}: slot {slot} directory offset at byte {directory_slot}"
        );

        let record_start = directory_slot + stored_offset;
        let expected_record_start = 514 + 1030 * slot;
        assert_eq!(
            record_start, expected_record_start,
            "{identity}: slot {slot} record start"
        );
        assert_eq!(
            &cgr[record_start..record_start + 6],
            &[0x00, 0x00, 0x20, 0x00, 0x20, 0x00],
            "{identity}: slot {slot} six-byte header at byte {record_start}"
        );

        let payload_start = record_start + 6;
        let payload_end = payload_start + HEIGHT_MAP_LEN;
        concatenated.extend_from_slice(&cgr[payload_start..payload_end]);
        if slot == SHIPPED_CGR_COUNT - 1 {
            assert_eq!(
                payload_end,
                cgr.len(),
                "{identity}: slot {slot} payload must end at EOF"
            );
        }
    }
    assert_eq!(
        concatenated.len(),
        SHIPPED_CGR_COUNT * HEIGHT_MAP_LEN,
        "{identity}: concatenated oracle height-map length"
    );

    concatenated
        .chunks_exact(HEIGHT_MAP_LEN)
        .map(|payload| payload.try_into().expect("1024-byte height map"))
        .collect()
}

#[test]
fn all_shipped_zone_cgr_banks_match_exw_height_oracle() {
    let Some(banks) = shipped_zone_cgrs() else {
        return;
    };

    let identities: BTreeSet<String> = banks.iter().map(|bank| bank.identity.clone()).collect();
    assert_eq!(
        identities.len(),
        banks.len(),
        "shipped runtime-selected CGR identities contain no duplicates"
    );
    assert_eq!(
        identities,
        expected_identities(),
        "exact shipped runtime-selected CGR identity set"
    );

    let mut dat = vec![0x01, 0x00, 0x01, 0x00];
    dat.extend_from_slice(&[0; 8]);
    let pad = [0xff; 6];

    for bank in banks {
        let cgr = fs::read(&bank.path)
            .unwrap_or_else(|error| panic!("{}: read shipped CGR: {error}", bank.identity));
        let oracle_maps = exw_height_oracle(&bank.identity, &cgr);
        let actual = Terrain::from_mission_bytes(&dat, &pad, &cgr)
            .unwrap_or_else(|| panic!("{}: Terrain rejected shipped CGR", bank.identity));
        let expected = Terrain::from_parts(1, 1, vec![0; 8], oracle_maps)
            .expect("synthetic 1x1 Terrain parts are valid");

        assert_eq!(
            actual, expected,
            "{}: full Terrain differs from the independent shipped-CGR oracle",
            bank.identity
        );
    }
}
