//! Whole-corpus static differential oracle for the DAT-addressing table
//! pair — diffharness registry row `static-yline-zbase`
//! (EXW 0x4ea900 + 0x4eaacc / EXD 0x8b78c + 0x107718).
//!
//! Expected side: an independent byte-level transcription of the EXW
//! table-build loops in `FUN_0041dc5a` @0x41ddaa..0x41dde2 (re-verified
//! instruction-by-instruction 2026-08-25, docs/RE-EXW-SIM.md §7c.3):
//! `y_line[y] = y*w` for y in 0..h−1 — **h dwords at 0x4ea900, NOT
//! h+1** (the loop bound is `h*4` under `jl` @0x41ddbe; no boundary
//! entry at h is ever staged or read) — and `z_base[z] = z*w*h` for
//! z in 0..7, 8 dwords at 0x4eaacc (stored factored `w*(z*h)`, offset
//! pre-incremented; the store-base cell 0x4eaac8 belongs to the
//! adjacent screen-scale family and is never a table entry). The second
//! producer pair @0x4466bd..0x4466f8 (FUN_0044661b, brief-screen
//! loadout) and the EXD twin @0x2e713..0x2e74b run both loops
//! instruction-for-instruction. The original reads `w`/`h` from the
//! TOT header words (movsx @0x41dd67/0x41dd7a).
//!
//! Actual side: the Rust target retains NO y_line/z_base bank — it
//! indexes `dat[z*w*h + y*w + x]` inline (`Terrain::dat_type`). The
//! tables are a pure `(w, h)` function whose whole semantic content is
//! the retained dims (`Terrain::size`), so this gate compares the
//! oracle against a TEST-ONLY representation built from the target's
//! retained dims — never fabricated engine output — and pins the
//! corpus invariants that make that reduction sound, first among them
//! **TOT[0..4] == DAT[0..4] on every shipped mission** (the original
//! builds the tables from the TOT header while the Rust loader takes
//! its dims from the DAT header; the divergence is real but
//! unobservable on a corpus where the two headers agree — asserted
//! here, not assumed).
//!
//! Scope: valid shipped corpus only. Not a malformed-input spec. No
//! production parser, loader, or terrain helper is reused on the
//! expected side (bytes only).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::mission::Terrain;

const SHIPPED_MISSION_COUNT: usize = 37;
const Z_BASE_ENTRIES: usize = 8;

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

        let mut dat_files: Vec<PathBuf> = fs::read_dir(&zone_dir)
            .unwrap_or_else(|error| panic!("read {zone_name}: {error}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("dat"))
                    && path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .is_some_and(is_numbered_mission_stem)
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

/// Independent transcription of the EXW y_line/z_base build loops,
/// reading `w`/`h` from the TOT header words exactly as the original
/// does (`movsx` @0x41dd67/0x41dd7a). Returns the exact staged images:
/// `h*4` y_line bytes (one LE dword per entry, `y*w`) and 32 z_base
/// bytes (`z*w*h`). Shares no code with the production loader.
fn exw_table_oracle(tot: &[u8]) -> Result<(Vec<u8>, [u8; 32]), String> {
    if tot.len() < 4 {
        return Err(format!(
            "shipped-corpus precondition: TOT header is truncated: {} bytes",
            tot.len()
        ));
    }
    // movsx s16 words: w @+0, h @+2 (cells 0x4eddec / 0x4eddf0).
    let width = i16::from_le_bytes([tot[0], tot[1]]) as i32;
    let height = i16::from_le_bytes([tot[2], tot[3]]) as i32;
    if width <= 0 || height <= 0 {
        return Err(format!(
            "shipped-corpus precondition: TOT dims are {width}x{height}"
        ));
    }
    // y_line loop @0x41ddaa..0x41ddbe: offset and value start at 0,
    // stride ecx = w, bound ebx = h*4 under jl -> h entries, value y*w.
    let mut y_line = Vec::with_capacity(4 * height as usize);
    let mut value: i32 = 0;
    let mut offset: i32 = 0;
    while offset < height * 4 {
        y_line.extend_from_slice(&(value as u32).to_le_bytes());
        offset += 4;
        value += width;
    }
    // z_base loop @0x41ddc0..0x41dde2: ecx = h, ebx = w; each iteration
    // computes w * (z*h) from the PRE-increment accumulator, bumps the
    // store offset by 4 FIRST, and stores at 0x4eaac8+eax for
    // eax in 4..=0x20 -> the 8 dwords at 0x4eaacc..=0x4eaae8.
    let mut z_base = [0u8; 32];
    let mut accumulator: i32 = 0; // z*h
    for z in 0..Z_BASE_ENTRIES {
        let staged = (width as i64) * (accumulator as i64);
        z_base[4 * z..4 * z + 4].copy_from_slice(&(staged as u32).to_le_bytes());
        accumulator += height;
    }
    Ok((y_line, z_base))
}

/// The Rust target's TEST-ONLY representation: the same table byte form
/// built from the retained dims (`Terrain::size` — the 0x4eddec /
/// 0x4eddf0 content). Rust retains no bank and no Rust consumer reads
/// these tables; this exists solely so the differential has a target
/// to compare against, and is NOT engine output.
fn rust_repr_tables(terrain: &Terrain) -> (Vec<u8>, [u8; 32]) {
    let (width, height) = terrain.size();
    let mut y_line = Vec::with_capacity(4 * height as usize);
    for y in 0..height {
        y_line.extend_from_slice(&((y * width) as u32).to_le_bytes());
    }
    let mut z_base = [0u8; 32];
    for z in 0..Z_BASE_ENTRIES {
        let staged = (z as i64) * (width as i64) * (height as i64);
        z_base[4 * z..4 * z + 4].copy_from_slice(&(staged as u32).to_le_bytes());
    }
    (y_line, z_base)
}

fn u32_at(blob: &[u8], entry: usize) -> u32 {
    u32::from_le_bytes(
        blob[4 * entry..4 * entry + 4]
            .try_into()
            .expect("table entry"),
    )
}

/// Byte/field-exact comparison of the two table images, naming every
/// differing entry.
fn assert_tables_match(identity: &str, oracle: &(Vec<u8>, [u8; 32]), actual: &(Vec<u8>, [u8; 32])) {
    assert_eq!(
        oracle.0.len(),
        actual.0.len(),
        "{identity}: y_line extents differ (oracle {} bytes vs Rust {})",
        oracle.0.len(),
        actual.0.len()
    );
    for entry in 0..oracle.0.len() / 4 {
        assert_eq!(
            u32_at(&oracle.0, entry),
            u32_at(&actual.0, entry),
            "{identity}: y_line[{entry}] differs"
        );
    }
    for entry in 0..Z_BASE_ENTRIES {
        assert_eq!(
            u32_at(&oracle.1, entry),
            u32_at(&actual.1, entry),
            "{identity}: z_base[{entry}] differs"
        );
    }
}

#[test]
fn all_missions_yline_zbase_tables_match_exw_staging_oracle() {
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

    let mut dims_census: BTreeMap<String, (i32, i32)> = BTreeMap::new();
    for mission in &missions {
        let tot = fs::read(&mission.tot)
            .unwrap_or_else(|error| panic!("{}: read TOT: {error}", mission.identity));
        let dat = fs::read(&mission.dat)
            .unwrap_or_else(|error| panic!("{}: read DAT: {error}", mission.identity));
        let pad = fs::read(&mission.pad)
            .unwrap_or_else(|error| panic!("{}: read PAD: {error}", mission.identity));
        let cgr = fs::read(&mission.cgr)
            .unwrap_or_else(|error| panic!("{}: read zone CGR: {error}", mission.identity));

        // Pinned invariant the reduction stands on: the original builds
        // the tables from the TOT header, the Rust loader reads the DAT
        // header — on the shipped corpus the two headers are identical.
        assert_eq!(
            &tot[0..4],
            &dat[0..4],
            "{}: TOT and DAT w/h headers disagree (the original's table dims \
             and the Rust loader dims would diverge)",
            mission.identity
        );

        let oracle = exw_table_oracle(&tot).unwrap_or_else(|error| {
            panic!("{}: oracle rejected corpus: {error}", mission.identity)
        });
        let width = i16::from_le_bytes([tot[0], tot[1]]) as i32;
        let height = i16::from_le_bytes([tot[2], tot[3]]) as i32;
        let plane = (width * height) as i64;

        // Exact staged extents and boundary entries [0x41ddaa..0x41dde2].
        assert_eq!(
            oracle.0.len(),
            4 * height as usize,
            "{}: y_line extent is h dwords (h = {height}), never h+1",
            mission.identity
        );
        assert_eq!(
            u32_at(&oracle.0, 0),
            0,
            "{}: y_line[0] == 0",
            mission.identity
        );
        assert_eq!(
            u32_at(&oracle.0, height as usize - 1),
            ((height - 1) * width) as u32,
            "{}: y_line[h-1] == (h-1)*w == plane size - one row",
            mission.identity
        );
        assert_eq!(
            oracle.1.len(),
            4 * Z_BASE_ENTRIES,
            "{}: z_base extent is exactly 8 dwords",
            mission.identity
        );
        assert_eq!(
            u32_at(&oracle.1, 0),
            0,
            "{}: z_base[0] == 0",
            mission.identity
        );
        assert_eq!(
            u32_at(&oracle.1, Z_BASE_ENTRIES - 1),
            (7 * plane) as u32,
            "{}: z_base[7] == 7*w*h (the plane-7 base)",
            mission.identity
        );
        // The consumer-index identity: the original's decomposition
        // z_base[z] + y_line[y] + x spans exactly the 8-plane volume.
        assert_eq!(
            u32_at(&oracle.1, Z_BASE_ENTRIES - 1) as i64
                + u32_at(&oracle.0, height as usize - 1) as i64
                + (width as i64 - 1),
            8 * plane - 1,
            "{}: z_base[7] + y_line[h-1] + (w-1) must address the volume's last byte",
            mission.identity
        );
        // The DAT file is exactly header + 8 planes (the arena bound).
        assert_eq!(
            dat.len(),
            4 + 8 * plane as usize,
            "{}: DAT file size is 4 + 8*w*h",
            mission.identity
        );

        // The Rust target: load through the production loader, then the
        // test-only representation of the (unretained) tables.
        let terrain = Terrain::from_mission_bytes(&dat, &pad, &cgr)
            .unwrap_or_else(|| panic!("{}: Terrain rejected corpus", mission.identity));
        assert_eq!(
            terrain.size(),
            (width, height),
            "{}: Rust retained dims vs the independent TOT parse",
            mission.identity
        );
        let actual = rust_repr_tables(&terrain);
        assert_tables_match(&mission.identity, &oracle, &actual);

        dims_census.insert(mission.identity.clone(), (width, height));
    }

    // Pinned corpus census (independently recomputed by the oracle run):
    // three non-square missions, 35 squares at the 100x100 maximum.
    assert_eq!(
        dims_census["ZONEA/MISSION1"],
        (25, 75),
        "ZONEA/MISSION1 dims"
    );
    assert_eq!(
        dims_census["ZONEG/MISSION1"],
        (100, 25),
        "ZONEG/MISSION1 dims"
    );
    let squares = dims_census
        .values()
        .filter(|&(w, h)| *w == 100 && *h == 100)
        .count();
    assert_eq!(squares, 35, "missions at the 100x100 maximum");
    assert_eq!(
        dims_census.len(),
        SHIPPED_MISSION_COUNT,
        "census covers every mission"
    );

    // Extreme-value pins (the largest and smallest staged images).
    let (y_a, z_a) = exw_table_oracle(&fs::read(editor_root().join("ZONEA/MISSION1.TOT")).unwrap())
        .expect("ZONEA/MISSION1 oracle");
    assert_eq!(y_a.len(), 300, "ZONEA y_line is 75 entries (h, not h+1)");
    assert_eq!(u32_at(&y_a, 74), 1850, "ZONEA y_line[74] == 74*25");
    assert_eq!(u32_at(&z_a, 7), 13125, "ZONEA z_base[7] == 7*1875");
    let (y_g, z_g) = exw_table_oracle(&fs::read(editor_root().join("ZONEG/MISSION1.TOT")).unwrap())
        .expect("ZONEG/MISSION1 oracle");
    assert_eq!(y_g.len(), 100, "ZONEG y_line is 25 entries");
    assert_eq!(u32_at(&y_g, 24), 2400, "ZONEG y_line[24] == 24*100");
    assert_eq!(u32_at(&z_g, 7), 17500, "ZONEG z_base[7] == 7*2500");
    let (y_b, z_b) = exw_table_oracle(&fs::read(editor_root().join("ZONEB/MISSION1.TOT")).unwrap())
        .expect("ZONEB/MISSION1 oracle");
    assert_eq!(y_b.len(), 400, "ZONEB y_line is 100 entries");
    assert_eq!(u32_at(&y_b, 99), 9900, "ZONEB y_line[99] == 99*100");
    assert_eq!(u32_at(&z_b, 7), 70000, "ZONEB z_base[7] == 7*10000");
}

/// Sensitivity proof for the oracle (temporary in-memory mutations; the
/// corpus on disk is never touched):
/// 1. a TOT w-byte bump changes every y_line entry y>=1 and every
///    z_base entry z>=1 (the zero anchors stay), keeps the extents, and
///    makes the differential FAIL against the un-mutated Rust side —
///    a TOT/DAT header disagreement is rejected, never absorbed;
/// 2. a TOT h-byte bump grows the y_line EXTENT by one entry while
///    leaving every existing entry byte-identical (the h-entry count is
///    load-bearing, not just the values);
/// 3. a DAT w-byte bump makes the Rust loader reject the file outright
///    (the Rust side is sensitive to its own dims source).
#[test]
fn yline_zbase_oracle_is_sensitive_to_dims_and_extents() {
    let zonea_tot = editor_root().join("ZONEA/MISSION1.TOT");
    let zonea_dat = editor_root().join("ZONEA/MISSION1.DAT");
    let zonea_pad = editor_root().join("ZONEA/MISSION1.PAD");
    let zonea_cgr = editor_root().join("ZONEA/MISSION1.CGR");
    if !zonea_tot.is_file() || !zonea_dat.is_file() {
        eprintln!("game-data corpus not found - skipping");
        return;
    }
    let tot = fs::read(&zonea_tot).expect("read ZONEA/MISSION1.TOT");
    let dat = fs::read(&zonea_dat).expect("read ZONEA/MISSION1.DAT");
    let pad = fs::read(&zonea_pad).expect("read ZONEA/MISSION1.PAD");
    let cgr = fs::read(&zonea_cgr).expect("read ZONEA/MISSION1.CGR");
    let (y_orig, z_orig) = exw_table_oracle(&tot).expect("ZONEA/MISSION1 oracle");
    let terrain =
        Terrain::from_mission_bytes(&dat, &pad, &cgr).expect("Rust target loads ZONEA/MISSION1");
    let actual = rust_repr_tables(&terrain);

    // (1) w bump 25 -> 26: values move at y>=1 / z>=1, extents do not,
    // and the mutated oracle no longer matches the Rust side.
    let mut bumped = tot.clone();
    bumped[0] = 26;
    let (y_w, z_w) = exw_table_oracle(&bumped).expect("w-bumped oracle");
    assert_eq!(
        y_w.len(),
        y_orig.len(),
        "a w bump must not move y_line extent"
    );
    let y_moved: Vec<usize> = (0..y_orig.len() / 4)
        .filter(|&e| u32_at(&y_orig, e) != u32_at(&y_w, e))
        .collect();
    assert_eq!(
        y_moved,
        (1..y_orig.len() / 4).collect::<Vec<_>>(),
        "a w bump must change exactly y_line[1..h] (y_line[0] stays 0)"
    );
    let z_moved: Vec<usize> = (0..Z_BASE_ENTRIES)
        .filter(|&e| u32_at(&z_orig, e) != u32_at(&z_w, e))
        .collect();
    assert_eq!(
        z_moved,
        (1..Z_BASE_ENTRIES).collect::<Vec<_>>(),
        "a w bump must change exactly z_base[1..8] (z_base[0] stays 0)"
    );
    assert_ne!(
        (y_w.clone(), z_w),
        actual,
        "a TOT/DAT header disagreement must fail the differential"
    );

    // (2) h bump 75 -> 76: the y_line extent grows by one entry and the
    // shared prefix stays byte-identical; z_base moves at z>=1.
    let mut bumped = tot.clone();
    bumped[2] = 76;
    let (y_h, z_h) = exw_table_oracle(&bumped).expect("h-bumped oracle");
    assert_eq!(
        y_h.len(),
        y_orig.len() + 4,
        "an h bump must grow the y_line extent by exactly one entry"
    );
    assert_eq!(
        &y_h[..y_orig.len()],
        &y_orig[..],
        "an h bump must leave every existing y_line entry byte-identical"
    );
    assert_eq!(
        u32_at(&y_h, 75),
        75 * 25,
        "the new last entry is 75*25 (index h-1 — no entry at h is ever staged)"
    );
    assert_ne!(z_orig, z_h, "an h bump must move z_base (z*w*h)");

    // (3) DAT w bump: the Rust loader's own dims source — the file no
    // longer matches 4 + 8*w*h, so the load is rejected, not absorbed.
    let mut dat_bumped = dat.clone();
    dat_bumped[0] = 26;
    assert!(
        Terrain::from_mission_bytes(&dat_bumped, &pad, &cgr).is_none(),
        "the Rust loader must reject a DAT header that disagrees with its plane bytes"
    );
}
