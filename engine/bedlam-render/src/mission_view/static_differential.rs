//! Whole-corpus static differential oracle for the mission-view load seam.
//!
//! The expected side is a byte-level transcription of the source-pinned
//! formats in `docs/RE-EXW-MISSIONVIEW.md` sections 1, 2, and 4. It does not
//! use `Terrain`, `dat_plane_bytes`, a production loader/normalizer, or a
//! production asset codec. Scope is valid shipped TOT, pre-PAD swept DAT,
//! and the seven runtime-selected lettered BIN/LNK banks. Malformed input,
//! `.LNG`, numbered BIN/LNK banks, MIN/PAD retention, composition, and timing
//! are deliberately outside this gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{MissionView, Sprite};

const MISSION_IDENTITIES: [&str; 37] = [
    "ZONEA/MISSION1",
    "ZONEB/MISSION1",
    "ZONEB/MISSION2",
    "ZONEB/MISSION3",
    "ZONEB/MISSION4",
    "ZONEB/MISSION5",
    "ZONEB/MISSION6",
    "ZONEB/MISSION7",
    "ZONEC/MISSION1",
    "ZONEC/MISSION2",
    "ZONEC/MISSION3",
    "ZONEC/MISSION4",
    "ZONEC/MISSION5",
    "ZONEC/MISSION6",
    "ZONEC/MISSION7",
    "ZONED/MISSION1",
    "ZONED/MISSION2",
    "ZONED/MISSION3",
    "ZONED/MISSION4",
    "ZONED/MISSION5",
    "ZONED/MISSION6",
    "ZONED/MISSION7",
    "ZONEE/MISSION1",
    "ZONEE/MISSION2",
    "ZONEE/MISSION3",
    "ZONEE/MISSION4",
    "ZONEE/MISSION5",
    "ZONEE/MISSION6",
    "ZONEE/MISSION7",
    "ZONEF/MISSION1",
    "ZONEF/MISSION2",
    "ZONEF/MISSION3",
    "ZONEF/MISSION4",
    "ZONEF/MISSION5",
    "ZONEF/MISSION6",
    "ZONEF/MISSION7",
    "ZONEG/MISSION1",
];

#[derive(Debug, Clone, Copy)]
struct BankSpec {
    zone: char,
    bin_len: usize,
    sprite_count: usize,
    fmt7_count: usize,
    inactive_count: usize,
}

const BANK_SPECS: [BankSpec; 7] = [
    BankSpec {
        zone: 'A',
        bin_len: 2_041_594,
        sprite_count: 1_450,
        fmt7_count: 1_441,
        inactive_count: 83,
    },
    BankSpec {
        zone: 'B',
        bin_len: 2_443_943,
        sprite_count: 1_872,
        fmt7_count: 1_863,
        inactive_count: 47,
    },
    BankSpec {
        zone: 'C',
        bin_len: 2_076_553,
        sprite_count: 1_743,
        fmt7_count: 1_734,
        inactive_count: 50,
    },
    BankSpec {
        zone: 'D',
        bin_len: 2_041_594,
        sprite_count: 1_450,
        fmt7_count: 1_441,
        inactive_count: 83,
    },
    BankSpec {
        zone: 'E',
        bin_len: 1_968_763,
        sprite_count: 1_455,
        fmt7_count: 1_446,
        inactive_count: 68,
    },
    BankSpec {
        zone: 'F',
        bin_len: 1_464_679,
        sprite_count: 989,
        fmt7_count: 980,
        inactive_count: 40,
    },
    BankSpec {
        zone: 'G',
        bin_len: 2_443_943,
        sprite_count: 1_872,
        fmt7_count: 1_863,
        inactive_count: 47,
    },
];

const LNK_LEN: usize = 8192 * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Header {
    format: u16,
    dy: u16,
    dx: u16,
    gate: u16,
    rows: u16,
}

#[derive(Debug, Clone, Copy)]
struct OraclePixel {
    row: i32,
    column: i32,
    palette: u8,
    source_offset: usize,
}

#[derive(Debug)]
struct OracleSprite {
    header: Header,
    pixels: Vec<OraclePixel>,
}

fn editor_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM/EDITOR")
}

fn le_u16(bytes: &[u8], offset: usize, context: &str) -> u16 {
    let field = bytes.get(offset..offset + 2).unwrap_or_else(|| {
        panic!("{context}: truncated little-endian u16 at source byte offset {offset}")
    });
    u16::from_le_bytes(field.try_into().expect("two-byte field"))
}

fn le_u32(bytes: &[u8], offset: usize, context: &str) -> u32 {
    let field = bytes.get(offset..offset + 4).unwrap_or_else(|| {
        panic!("{context}: truncated little-endian u32 at source byte offset {offset}")
    });
    u32::from_le_bytes(field.try_into().expect("four-byte field"))
}

fn discover_numbered_tot_identities(root: &Path) -> BTreeSet<String> {
    let mut identities = BTreeSet::new();
    for zone in 'A'..='G' {
        let zone_name = format!("ZONE{zone}");
        let zone_dir = root.join(&zone_name);
        for entry in
            fs::read_dir(&zone_dir).unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
        {
            let path = entry
                .unwrap_or_else(|error| panic!("read entry in {zone_name}: {error}"))
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("TOT") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(number) = stem.strip_prefix("MISSION") else {
                continue;
            };
            if !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) {
                identities.insert(format!("{zone_name}/{stem}"));
            }
        }
    }
    identities
}

fn discover_lettered_banks(root: &Path, extension: &str) -> BTreeSet<String> {
    let mut identities = BTreeSet::new();
    for zone in 'A'..='G' {
        let zone_name = format!("ZONE{zone}");
        let zone_dir = root.join(&zone_name);
        for entry in
            fs::read_dir(&zone_dir).unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
        {
            let path = entry
                .unwrap_or_else(|error| panic!("read entry in {zone_name}: {error}"))
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some(extension) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(suffix) = stem.strip_prefix("MISSION") else {
                continue;
            };
            if suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'A'..=b'G') {
                identities.insert(format!("{zone_name}/{stem}.{extension}"));
            }
        }
    }
    identities
}

fn expected_bank_identities(extension: &str) -> BTreeSet<String> {
    ('A'..='G')
        .map(|zone| format!("ZONE{zone}/MISSION{zone}.{extension}"))
        .collect()
}

/// Independently parses the signed TOT header and exact eight-plane volume.
fn tot_dimensions(identity: &str, tot: &[u8]) -> (usize, usize) {
    assert!(tot.len() >= 4, "{identity}: TOT header truncated");
    let width = i16::from_le_bytes([tot[0], tot[1]]);
    let height = i16::from_le_bytes([tot[2], tot[3]]);
    assert!(
        width > 0 && height > 0,
        "{identity}: signed TOT dimensions at +0/+2 are {width}x{height}"
    );
    let (width, height) = (width as usize, height as usize);
    let cells = width
        .checked_mul(height)
        .unwrap_or_else(|| panic!("{identity}: TOT dimensions overflow"));
    assert_eq!(
        tot.len(),
        4 + 16 * cells,
        "{identity}: exact TOT length for signed dimensions {width}x{height}"
    );
    (width, height)
}

/// Applies only the pre-PAD EXW DAT sweep consumed by MissionView: planes
/// 0..6 map bytes >= 0x80 to zero; plane 7 is retained byte-for-byte.
fn swept_dat_planes(identity: &str, dat: &[u8], width: usize, height: usize) -> Vec<u8> {
    assert!(dat.len() >= 4, "{identity}: DAT header truncated");
    let dat_width = i16::from_le_bytes([dat[0], dat[1]]);
    let dat_height = i16::from_le_bytes([dat[2], dat[3]]);
    assert_eq!(
        (dat_width, dat_height),
        (width as i16, height as i16),
        "{identity}: DAT signed header +0/+2 agrees with TOT"
    );
    let cells = width * height;
    assert_eq!(
        dat.len(),
        4 + 8 * cells,
        "{identity}: exact DAT length for {width}x{height}"
    );
    let mut planes = dat[4..].to_vec();
    for value in &mut planes[..7 * cells] {
        if *value >= 0x80 {
            *value = 0;
        }
    }
    planes
}

fn oracle_lnk(identity: &str, lnk: &[u8]) -> Vec<u16> {
    assert_eq!(lnk.len(), LNK_LEN, "{identity}: exact runtime LNK length");
    (0..8192)
        .map(|index| le_u16(lnk, 2 * index, identity))
        .collect()
}

fn parse_header(bank: &[u8], start: usize, identity: &str, sprite_id: usize) -> Header {
    let context = format!("{identity}: sprite {sprite_id}");
    Header {
        format: le_u16(bank, start, &context),
        dy: le_u16(bank, start + 2, &context),
        dx: le_u16(bank, start + 4, &context),
        gate: le_u16(bank, start + 6, &context),
        rows: le_u16(bank, start + 8, &context),
    }
}

/// Simple test-local interpreter for FUN_00401471's three documented format
/// classes. It advances a row/column cursor and records source offsets for
/// diagnostics rather than sharing the production decoder's control flow.
fn decode_bin_record_oracle(
    identity: &str,
    sprite_id: usize,
    bank: &[u8],
    start: usize,
    end: usize,
) -> OracleSprite {
    let header = parse_header(bank, start, identity, sprite_id);
    let context = || format!("{identity}: sprite {sprite_id}, format {}", header.format);
    let mut pixels = Vec::new();

    if header.gate == 0 || header.rows == 0 {
        assert_eq!(
            header.gate,
            0,
            "{}: inactive header gate field at source byte offset {}",
            context(),
            start + 6
        );
        assert_eq!(
            header.rows,
            0,
            "{}: inactive header rows field at source byte offset {}",
            context(),
            start + 8
        );
        // The nine radar scratch records are the documented special case:
        // a six-byte {fmt, 64, 64} head followed immediately by 4096 raw
        // bytes. Their first four raw bytes are zero, so the terrain decoder
        // observes gate==rows==0 and returns before treating them as pixels.
        let expected_len = if header.format == 0 { 6 + 64 * 64 } else { 10 };
        assert_eq!(
            end - start,
            expected_len,
            "{}: source-pinned inactive record length at byte range {start}..{end}",
            context()
        );
        return OracleSprite { header, pixels };
    }

    let mut source = start + 10;
    match header.format {
        0 => {
            assert_eq!(
                end - source,
                64 * 64,
                "{}: fmt0 raw payload length at source byte offset {source}",
                context()
            );
            for row in 0..64i32 {
                for column in 0..64i32 {
                    let palette = bank[source];
                    if palette != 0 {
                        pixels.push(OraclePixel {
                            row: i32::from(header.dy) + row,
                            column: i32::from(header.dx) + column,
                            palette,
                            source_offset: source,
                        });
                    }
                    source += 1;
                }
            }
        }
        1..=3 => {
            let mut row = i32::from(header.dy);
            let mut completed_rows = 0usize;
            let mut column = i32::from(header.dx);
            while completed_rows < usize::from(header.rows) {
                let control_offset = source;
                let control = le_u16(bank, source, &context());
                source += 2;
                if control & 0x8000 != 0 {
                    if control & 0x4000 != 0 {
                        completed_rows += 1;
                        row += 1;
                        column = i32::from(header.dx);
                    } else {
                        column += i32::from(control & 0x0fff);
                    }
                    continue;
                }
                let run = usize::from(control & 0x0fff);
                let literal_end = source.checked_add(run).unwrap_or_else(|| {
                    panic!(
                        "{}: literal length overflow at source byte offset {control_offset}",
                        context()
                    )
                });
                assert!(
                    literal_end <= end,
                    "{}: literal at source byte offset {control_offset} overruns record end {end}",
                    context()
                );
                for palette in &bank[source..literal_end] {
                    if *palette != 0 {
                        pixels.push(OraclePixel {
                            row,
                            column,
                            palette: *palette,
                            source_offset: source,
                        });
                    }
                    source += 1;
                    column += 1;
                }
                if control & 0x4000 != 0 {
                    completed_rows += 1;
                    row += 1;
                    column = i32::from(header.dx);
                }
            }
        }
        _ => {
            let mut row = i32::from(header.dy);
            let mut completed_rows = 0usize;
            let mut column = i32::from(header.dx);
            while completed_rows < usize::from(header.rows) {
                let control_offset = source;
                let control = *bank.get(source).unwrap_or_else(|| {
                    panic!(
                        "{}: missing control at source byte offset {source}",
                        context()
                    )
                });
                source += 1;
                if control & 0x80 != 0 {
                    if control & 0x40 != 0 {
                        completed_rows += 1;
                        row += 1;
                        column = i32::from(header.dx);
                    } else {
                        column += i32::from(control & 0x3f) + 1;
                    }
                    continue;
                }
                let run = usize::from(control & 0x3f) + 1;
                let literal_end = source.checked_add(run).unwrap_or_else(|| {
                    panic!(
                        "{}: literal length overflow at source byte offset {control_offset}",
                        context()
                    )
                });
                assert!(
                    literal_end <= end,
                    "{}: literal at source byte offset {control_offset} overruns record end {end}",
                    context()
                );
                for palette in &bank[source..literal_end] {
                    if *palette != 0 {
                        pixels.push(OraclePixel {
                            row,
                            column,
                            palette: *palette,
                            source_offset: source,
                        });
                    }
                    source += 1;
                    column += 1;
                }
                if control & 0x40 != 0 {
                    completed_rows += 1;
                    row += 1;
                    column = i32::from(header.dx);
                }
            }
        }
    }
    assert_eq!(
        source,
        end,
        "{}: decoder consumed through source byte {source}, record ends at {end}",
        context()
    );
    OracleSprite { header, pixels }
}

fn assert_sprite_pixels_equal(
    identity: &str,
    sprite_id: usize,
    format: u16,
    expected: &[OraclePixel],
    actual: &[(i32, i32, u8)],
) {
    let common = expected.len().min(actual.len());
    for index in 0..common {
        let expected_pixel = expected[index];
        let actual_pixel = actual[index];
        let expected_tuple = (
            expected_pixel.row,
            expected_pixel.column,
            expected_pixel.palette,
        );
        if expected_tuple != actual_pixel {
            panic!(
                "{identity}: sprite {sprite_id}, format {format}, output {index}, source byte offset {source}: expected row {er}, column {ec}, palette {ep:#04x}; actual row {ar}, column {ac}, palette {ap:#04x}",
                source = expected_pixel.source_offset,
                er = expected_pixel.row,
                ec = expected_pixel.column,
                ep = expected_pixel.palette,
                ar = actual_pixel.0,
                ac = actual_pixel.1,
                ap = actual_pixel.2,
            );
        }
    }
    if expected.len() != actual.len() {
        if let Some(pixel) = expected.get(common) {
            panic!(
                "{identity}: sprite {sprite_id}, format {format}: production output ended at {common} pixels; next expected row {}, column {}, palette {:#04x}, source byte offset {}",
                pixel.row, pixel.column, pixel.palette, pixel.source_offset
            );
        }
        let (row, column, palette) = actual[common];
        panic!(
            "{identity}: sprite {sprite_id}, format {format}: production emitted extra pixel {common}: row {row}, column {column}, palette {palette:#04x} after oracle record end"
        );
    }
}

fn assert_bin_matches_oracle(identity: &str, spec: BankSpec, bank: &[u8], view: &MissionView) {
    assert_eq!(bank.len(), spec.bin_len, "{identity}: exact BIN length");
    assert_eq!(
        view.bank.len(),
        bank.len(),
        "{identity}: MissionView retained BIN length"
    );
    if let Some(offset) = bank
        .iter()
        .zip(&view.bank)
        .position(|(expected, actual)| expected != actual)
    {
        panic!(
            "{identity}: retained BIN mismatch at absolute source offset {offset}: expected {:#04x}, actual {:#04x}",
            bank[offset], view.bank[offset]
        );
    }
    let count = usize::from(le_u16(bank, 0, identity));
    assert_eq!(
        count, spec.sprite_count,
        "{identity}: sprite count word at byte 0"
    );
    let directory_end = 2 + 4 * count;
    let mut starts = Vec::with_capacity(count);
    for sprite_id in 0..count {
        let entry = 2 + 4 * sprite_id;
        let relative = le_u32(bank, entry, identity) as usize;
        let start = entry.checked_add(relative).unwrap_or_else(|| {
            panic!(
                "{identity}: sprite {sprite_id} self-relative directory overflow at byte {entry}"
            )
        });
        assert!(
            start < bank.len(),
            "{identity}: sprite {sprite_id} directory entry at byte {entry} resolves out of file to {start}"
        );
        starts.push(start);
    }
    assert_eq!(
        starts.first().copied(),
        Some(directory_end),
        "{identity}: first self-relative record starts after exact directory"
    );
    assert!(
        starts.windows(2).all(|pair| pair[0] < pair[1]),
        "{identity}: self-relative record starts are strictly ordered"
    );

    let mut format_census = BTreeMap::<u16, usize>::new();
    let mut inactive_count = 0usize;
    for sprite_id in 0..count {
        let start = starts[sprite_id];
        let end = starts.get(sprite_id + 1).copied().unwrap_or(bank.len());
        let oracle = decode_bin_record_oracle(identity, sprite_id, bank, start, end);
        *format_census.entry(oracle.header.format).or_default() += 1;
        if oracle.header.gate == 0 || oracle.header.rows == 0 {
            inactive_count += 1;
        }

        let production = Sprite::resolve(&view.bank, sprite_id as u16).unwrap_or_else(|| {
            panic!("{identity}: production failed to resolve sprite {sprite_id}")
        });
        let production_start = view.bank.len() - production.data.len();
        assert_eq!(
            production_start,
            start,
            "{identity}: sprite {sprite_id} self-relative directory resolution from entry byte {}",
            2 + 4 * sprite_id
        );
        for (field, relative_offset, expected) in [
            ("format", 0usize, oracle.header.format),
            ("dy", 2, oracle.header.dy),
            ("dx", 4, oracle.header.dx),
            ("gate", 6, oracle.header.gate),
            ("rows", 8, oracle.header.rows),
        ] {
            let actual = le_u16(production.data, relative_offset, identity);
            assert_eq!(
                actual,
                expected,
                "{identity}: sprite {sprite_id}, format {}, header {field} at source byte offset {}",
                oracle.header.format,
                start + relative_offset
            );
        }

        let expected_header = (oracle.header.gate != 0 && oracle.header.rows != 0).then_some((
            oracle.header.format,
            i32::from(oracle.header.dy),
            i32::from(oracle.header.dx),
            i32::from(oracle.header.rows),
        ));
        assert_eq!(
            production.header(),
            expected_header,
            "{identity}: sprite {sprite_id}, format {}, production header transform (gate source byte {}, rows source byte {})",
            oracle.header.format,
            start + 6,
            start + 8
        );

        let mut actual_pixels = Vec::new();
        let actual_header = production.for_each_pixel(|row, column, palette| {
            actual_pixels.push((row, column, palette));
        });
        assert_eq!(
            actual_header, expected_header,
            "{identity}: sprite {sprite_id}, format {}, decoder header result",
            oracle.header.format
        );
        assert_sprite_pixels_equal(
            identity,
            sprite_id,
            oracle.header.format,
            &oracle.pixels,
            &actual_pixels,
        );
    }

    assert_eq!(
        format_census,
        BTreeMap::from([(0, 9), (7, spec.fmt7_count)]),
        "{identity}: exact shipped format census (u16-RLE formats 1..3 are absent)"
    );
    assert_eq!(
        inactive_count, spec.inactive_count,
        "{identity}: exact gate==rows==0 sprite census"
    );
}

#[test]
fn all_shipped_missions_exw_tot_dat_bin_lnk_transform_matches_mission_view() {
    let root = editor_root();
    if !root.is_dir() {
        eprintln!("game-data corpus not found - skipping");
        return;
    }

    let expected_missions: BTreeSet<String> = MISSION_IDENTITIES
        .iter()
        .map(|identity| (*identity).to_string())
        .collect();
    assert_eq!(
        discover_numbered_tot_identities(&root),
        expected_missions,
        "exact 37 shipped numbered mission identities"
    );
    assert_eq!(
        discover_lettered_banks(&root, "BIN"),
        expected_bank_identities("BIN"),
        "exact seven runtime-selected lettered BIN identities; numbered banks excluded"
    );
    assert_eq!(
        discover_lettered_banks(&root, "LNK"),
        expected_bank_identities("LNK"),
        "exact seven runtime-selected lettered LNK identities; numbered banks and LNG excluded"
    );

    let specs: BTreeMap<char, BankSpec> = BANK_SPECS
        .iter()
        .copied()
        .map(|spec| (spec.zone, spec))
        .collect();
    let mut compared_bins = BTreeSet::new();

    for identity in MISSION_IDENTITIES {
        let zone = identity.as_bytes()[4] as char;
        let spec = specs[&zone];
        let mission_base = root.join(identity);
        let tot = fs::read(mission_base.with_extension("TOT"))
            .unwrap_or_else(|error| panic!("{identity}: read TOT: {error}"));
        let dat = fs::read(mission_base.with_extension("DAT"))
            .unwrap_or_else(|error| panic!("{identity}: read DAT: {error}"));
        let bank_identity = format!("ZONE{zone}/MISSION{zone}.BIN");
        let lnk_identity = format!("ZONE{zone}/MISSION{zone}.LNK");
        let bin = fs::read(root.join(&bank_identity))
            .unwrap_or_else(|error| panic!("{identity}: read {bank_identity}: {error}"));
        let lnk = fs::read(root.join(&lnk_identity))
            .unwrap_or_else(|error| panic!("{identity}: read {lnk_identity}: {error}"));

        let (width, height) = tot_dimensions(identity, &tot);
        let cells = width * height;
        let swept_dat = swept_dat_planes(identity, &dat, width, height);
        let expected_lnk = oracle_lnk(&lnk_identity, &lnk);
        let view = MissionView::from_mission_bytes(&tot, &swept_dat, &bin, &lnk)
            .unwrap_or_else(|| panic!("{identity}: MissionView rejected valid shipped bytes"));

        assert_eq!(
            (view.width, view.height),
            (width as i32, height as i32),
            "{identity}: private MissionView dimensions from signed TOT +0/+2"
        );
        assert_eq!(
            view.size(),
            (width as i32, height as i32),
            "{identity}: public MissionView dimensions from signed TOT +0/+2"
        );
        assert_eq!(
            view.words.len(),
            8 * cells,
            "{identity}: private words state length for {cells} mission cells"
        );
        assert_eq!(
            view.seen.len(),
            8 * cells,
            "{identity}: private seen state length for {cells} mission cells"
        );
        assert_eq!(
            view.bias.len(),
            cells,
            "{identity}: private bias state length for {cells} mission cells"
        );
        if let Some((tile, bias)) = view
            .bias
            .iter()
            .copied()
            .enumerate()
            .find(|(_, bias)| *bias != 0)
        {
            let x = tile % width;
            let y = tile / width;
            panic!(
                "{identity}: initial private bias byte at tile {tile} (x {x}, y {y}) is {bias:#04x}, expected 0x00"
            );
        }

        for tile in 0..cells {
            let x = tile % width;
            let y = tile / width;
            for level in 0..8usize {
                let tot_offset = 4 + 2 * (level * cells + tile);
                let expected_word = le_u16(&tot, tot_offset, identity);
                let state_index = 8 * tile + level;
                assert_eq!(
                    view.words[state_index],
                    expected_word,
                    "{identity}: private TOT mirror mismatch at tile {tile} (x {x}, y {y}), level {level}, TOT source byte offset {tot_offset}"
                );
                assert_eq!(
                    view.word(tile, level),
                    expected_word,
                    "{identity}: public TOT mirror mismatch at tile {tile} (x {x}, y {y}), level {level}, TOT source byte offset {tot_offset}"
                );

                let dat_plane_offset = level * cells + tile;
                let dat_source_offset = 4 + dat_plane_offset;
                let expected_seen =
                    u8::from(expected_word != 0 && swept_dat[dat_plane_offset] == 0);
                assert_eq!(
                    view.seen[state_index],
                    expected_seen,
                    "{identity}: private seen mismatch at tile {tile} (x {x}, y {y}), level {level}; TOT source byte offset {tot_offset}, swept DAT source byte offset {dat_source_offset}"
                );
                assert_eq!(
                    view.seen(tile, level),
                    expected_seen,
                    "{identity}: public seen mismatch at tile {tile} (x {x}, y {y}), level {level}; TOT source byte offset {tot_offset}, swept DAT source byte offset {dat_source_offset}"
                );
            }
        }

        assert_eq!(
            view.lnk.len(),
            8192,
            "{identity}/{lnk_identity}: production LNK word count"
        );
        for (index, expected) in expected_lnk.iter().copied().enumerate() {
            assert_eq!(
                view.lnk[index],
                expected,
                "{identity}/{lnk_identity}: private LNK index {index}, source byte offset {}",
                2 * index
            );
            assert_eq!(
                view.lnk_step(index as u16),
                expected,
                "{identity}/{lnk_identity}: public LNK step at index {index}, source byte offset {}",
                2 * index
            );
        }

        if compared_bins.insert(zone) {
            assert_bin_matches_oracle(&bank_identity, spec, &bin, &view);
        }
    }

    assert_eq!(
        compared_bins,
        ('A'..='G').collect(),
        "all seven runtime-selected BIN banks compared exactly once"
    );
}
