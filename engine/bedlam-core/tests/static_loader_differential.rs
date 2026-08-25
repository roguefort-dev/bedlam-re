//! Whole-corpus differential gate for the statically verified EXW TOT-driven
//! DAT/PAD load transform (docs/RE-EXW-SIM.md sec 7c.3-5).
//!
//! Scope: valid shipped-corpus post-load DAT-volume parity only. This is not a
//! malformed-input behavior specification and does not differentially test the
//! CGR transform (the matching zone CGR is supplied only to construct Terrain).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::mission::Terrain;

const SHIPPED_MISSION_COUNT: usize = 37;
const SHIPPED_PAD_RECORD_COUNT: usize = 999;
const SHIPPED_PAD_LEN: usize = SHIPPED_PAD_RECORD_COUNT * 6;
const MAX_SHIPPED_MAP_DIMENSION: i16 = 100;

#[derive(Debug)]
struct MissionFiles {
    identity: String,
    tot: PathBuf,
    dat: PathBuf,
    pad: PathBuf,
    zone_cgr: PathBuf,
}

fn editor_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR")
}

fn is_numbered_mission_dat(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    if !extension.eq_ignore_ascii_case("dat") {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(number) = stem.strip_prefix("MISSION") else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

/// Enumerate the corpus from its shipped ZONE*/MISSION<number>.DAT shape,
/// following the same sorted `read_dir` convention as the other corpus gates.
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
            .filter(|path| path.is_file() && is_numbered_mission_dat(path))
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
                dat,
                zone_cgr: zone_cgr.clone(),
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

/// Read only the format-pinned TOT header fields used by EXW: signed little-
/// endian words at +0/+2. The exact TOT volume length and 100x100 arena bound
/// are shipped-corpus sanity checks, not production parser behavior.
fn shipped_tot_dimensions(tot: &[u8]) -> Result<(u16, u16), String> {
    if tot.len() < 4 {
        return Err(format!(
            "shipped-corpus precondition: TOT header is truncated: {} bytes",
            tot.len()
        ));
    }
    let width = i16::from_le_bytes([tot[0], tot[1]]);
    let height = i16::from_le_bytes([tot[2], tot[3]]);
    if !(1..=MAX_SHIPPED_MAP_DIMENSION).contains(&width)
        || !(1..=MAX_SHIPPED_MAP_DIMENSION).contains(&height)
    {
        return Err(format!(
            "shipped-corpus precondition: TOT dimensions are not positive/sane: {width}x{height}"
        ));
    }
    let width = width as u16;
    let height = height as u16;
    let cells = usize::from(width) * usize::from(height);
    let required_len = 4 + 8 * cells * size_of::<u16>();
    if tot.len() != required_len {
        return Err(format!(
            "shipped-corpus precondition: TOT length is {}, expected {required_len} for {width}x{height}",
            tot.len()
        ));
    }
    Ok((width, height))
}

fn shipped_dat_header_dimensions(dat: &[u8]) -> Result<(u16, u16), String> {
    if dat.len() < 4 {
        return Err(format!(
            "shipped-corpus precondition: DAT header is truncated: {} bytes",
            dat.len()
        ));
    }
    Ok((
        u16::from_le_bytes([dat[0], dat[1]]),
        u16::from_le_bytes([dat[2], dat[3]]),
    ))
}

/// Independent transcription of load_mission@0041dc5a's DAT/PAD transform.
/// Dimensions come from the independently parsed TOT header, as they do in
/// EXW. This deliberately operates on bytes only and shares no production
/// parser or loader helper.
fn exw_dat_pad_oracle(width: u16, height: u16, dat: &[u8], pad: &[u8]) -> Result<Vec<u8>, String> {
    if dat.len() < 4 {
        return Err(format!(
            "shipped-corpus precondition: DAT header is truncated: {} bytes",
            dat.len()
        ));
    }
    let cells = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(|| format!("DAT dimensions overflow: {width}x{height}"))?;
    let payload_len = 8usize
        .checked_mul(cells)
        .ok_or_else(|| format!("DAT payload length overflows: {width}x{height}"))?;
    let required_len = 4usize
        .checked_add(payload_len)
        .ok_or_else(|| format!("DAT file length overflows: {width}x{height}"))?;
    if dat.len() != required_len {
        return Err(format!(
            "shipped-corpus precondition: DAT length is {}, expected {required_len} from TOT dimensions {width}x{height}",
            dat.len()
        ));
    }
    if pad.len() != SHIPPED_PAD_LEN {
        return Err(format!(
            "shipped-corpus precondition: PAD length is {}, expected {SHIPPED_PAD_LEN} bytes ({SHIPPED_PAD_RECORD_COUNT} six-byte records)",
            pad.len()
        ));
    }

    let mut planes = dat[4..].to_vec();
    for value in &mut planes[..7 * cells] {
        if *value >= 0x80 {
            *value = 0;
        }
    }

    let mut found_terminator = false;
    for record_index in 0..SHIPPED_PAD_RECORD_COUNT {
        let offset = record_index * 6;
        let end = offset + 6;
        let record = pad.get(offset..end).ok_or_else(|| {
            format!(
                "shipped-corpus precondition: PAD record {record_index} at byte {offset} is truncated: {} bytes remain",
                pad.len() - offset
            )
        })?;
        let x = u16::from_le_bytes([record[0], record[1]]);
        if x == 0xffff {
            found_terminator = true;
            // EXW stops here. Do not constrain ignored tail records: the
            // shipped ZONEB/MISSION3 PAD contains one orphan after this mark.
            break;
        }
        let y = u16::from_le_bytes([record[2], record[3]]);
        let level = u16::from_le_bytes([record[4], record[5]]);
        // Corpus precondition only: EXW's PAD write is unchecked. This gate
        // deliberately says nothing about malformed/out-of-bounds PAD input.
        if x >= width || y >= height || level >= 8 {
            return Err(format!(
                "shipped-corpus precondition: PAD record {record_index} is out of range: ({x}, {y}, level {level}) for {width}x{height}x8"
            ));
        }
        let index =
            usize::from(level) * cells + usize::from(y) * usize::from(width) + usize::from(x);
        planes[index] = 0xff;
    }
    if !found_terminator {
        return Err(format!(
            "shipped-corpus precondition: PAD has no 0xffff-x terminator in its {SHIPPED_PAD_RECORD_COUNT} records"
        ));
    }

    let mut serialized = Vec::with_capacity(required_len);
    serialized.extend_from_slice(&width.to_le_bytes());
    serialized.extend_from_slice(&height.to_le_bytes());
    serialized.extend_from_slice(&planes);
    Ok(serialized)
}

fn serialize_terrain(terrain: &Terrain) -> Vec<u8> {
    let (width, height) = terrain.size();
    let mut serialized = Vec::with_capacity(4 + 8 * (width * height) as usize);
    serialized.extend_from_slice(&(width as u16).to_le_bytes());
    serialized.extend_from_slice(&(height as u16).to_le_bytes());
    for level in 0..8 {
        for y in 0..height {
            for x in 0..width {
                serialized.push(
                    terrain
                        .raw_dat_byte(x, y, level)
                        .expect("canonical terrain coordinate is in range"),
                );
            }
        }
    }
    serialized
}

fn byte_context(offset: usize, width: u16, height: u16) -> String {
    match offset {
        0..=1 => format!("width header byte {offset}"),
        2..=3 => format!("height header byte {}", offset - 2),
        _ => {
            let cells = usize::from(width) * usize::from(height);
            let payload_offset = offset - 4;
            let level = payload_offset / cells;
            let tile = payload_offset % cells;
            let y = tile / usize::from(width);
            let x = tile % usize::from(width);
            format!("plane {level}, x {x}, y {y}")
        }
    }
}

fn byte_value(bytes: &[u8], offset: usize) -> String {
    bytes
        .get(offset)
        .map_or_else(|| "<end>".to_string(), |value| format!("{value:#04x}"))
}

fn assert_loader_bytes_equal(
    identity: &str,
    width: u16,
    height: u16,
    expected: &[u8],
    actual: &[u8],
) {
    let shared_len = expected.len().min(actual.len());
    let mismatch = (0..shared_len)
        .find(|&offset| expected[offset] != actual[offset])
        .or_else(|| (expected.len() != actual.len()).then_some(shared_len));
    if let Some(offset) = mismatch {
        panic!(
            "{identity}: loader mismatch at byte offset {offset} ({context}): expected {expected_byte}, actual {actual_byte}",
            context = byte_context(offset, width, height),
            expected_byte = byte_value(expected, offset),
            actual_byte = byte_value(actual, offset),
        );
    }
}

#[test]
fn all_missions_exw_dat_pad_loader_matches_terrain() {
    let Some(missions) = shipped_missions() else {
        return;
    };
    assert_eq!(
        missions.len(),
        SHIPPED_MISSION_COUNT,
        "enumerated every shipped numbered mission DAT"
    );
    let identities: BTreeSet<String> = missions
        .iter()
        .map(|mission| mission.identity.clone())
        .collect();
    assert_eq!(
        identities.len(),
        missions.len(),
        "enumerated mission identities contain no duplicates"
    );
    assert_eq!(
        identities,
        canonical_mission_identities(),
        "enumerated exact canonical shipped mission identity set"
    );

    let mut dimension_census = BTreeMap::new();
    for mission in missions {
        let tot = fs::read(&mission.tot)
            .unwrap_or_else(|error| panic!("{}: read TOT: {error}", mission.identity));
        let dat = fs::read(&mission.dat)
            .unwrap_or_else(|error| panic!("{}: read DAT: {error}", mission.identity));
        let pad = fs::read(&mission.pad)
            .unwrap_or_else(|error| panic!("{}: read PAD: {error}", mission.identity));
        let cgr = fs::read(&mission.zone_cgr)
            .unwrap_or_else(|error| panic!("{}: read zone CGR: {error}", mission.identity));

        let (width, height) = shipped_tot_dimensions(&tot)
            .unwrap_or_else(|error| panic!("{}: {error}", mission.identity));
        let dat_dimensions = shipped_dat_header_dimensions(&dat)
            .unwrap_or_else(|error| panic!("{}: {error}", mission.identity));
        assert_eq!(
            dat_dimensions,
            (width, height),
            "{}: shipped DAT header must agree with independently parsed TOT dimensions",
            mission.identity
        );
        let expected = exw_dat_pad_oracle(width, height, &dat, &pad).unwrap_or_else(|error| {
            panic!("{}: oracle rejected corpus: {error}", mission.identity)
        });
        *dimension_census.entry((width, height)).or_insert(0usize) += 1;

        let terrain = Terrain::from_mission_bytes(&dat, &pad, &cgr)
            .unwrap_or_else(|| panic!("{}: Terrain rejected corpus", mission.identity));
        assert_eq!(
            terrain.size(),
            (i32::from(width), i32::from(height)),
            "{}: Terrain dimensions",
            mission.identity
        );
        let actual = serialize_terrain(&terrain);
        assert_loader_bytes_equal(&mission.identity, width, height, &expected, &actual);
    }

    assert_eq!(
        dimension_census,
        [((25, 75), 1usize), ((100, 25), 1), ((100, 100), 35)]
            .into_iter()
            .collect(),
        "shipped mission dimension census"
    );
}
