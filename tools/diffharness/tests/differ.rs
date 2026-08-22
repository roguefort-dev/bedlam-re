//! W7 differ tests (DESIGN-DIFFHARNESS.md §6/§6a, RE-EXD-MAP §8).
//!
//! Anti-fabrication discipline: the O1-layout fixtures below are
//! hand-built from the RE-EXD-MAP §8 offset table (an independent
//! transcription, not a re-run of the normalizer), and the canonical
//! robot literal is the W6 gate's hand-encoded 98-byte fixture — the
//! pinned CONTRACT this module must match.

use diffharness::differ::{
    manifest_json, normalize_frame, report_text, run_diff, Class, DiffConfig, FieldVal, Mode,
    Verdict,
};
use diffharness::dump::{encode_dump, Channel, DumpHeader, FrameRecord, WatchRecord};
use diffharness::hash::sha256;
use diffharness::registry;
use diffharness::runner::{stitch, Scenario, Transcript};

fn reg() -> Vec<diffharness::Watch> {
    registry()
}

fn frame(no: u64, watches: Vec<WatchRecord>) -> FrameRecord {
    let mut f = FrameRecord::new(no, false);
    f.watches = watches;
    f
}

/// Stitch a transcript into dump bytes for one channel.
fn dump_bytes(scenario_src: &str, frames: Vec<FrameRecord>, channel: Channel) -> Vec<u8> {
    let scen = Scenario::parse(scenario_src).expect("scenario parses");
    let header = DumpHeader::new(channel, sha256(b"test-build"), scen.id.clone());
    stitch(&scen, &Transcript { frames }, &header, &reg())
        .expect("transcript stitches")
        .bytes
}

// ---------------------------------------------------------------------
// 1. The O1 normalizer: hand-built EXD-layout fixtures
// ---------------------------------------------------------------------

/// A 0xA8 EXD robot record carrying every §8-mapped field (D88 back
/// half). The eight parameters set the W7-era front fields; the back
/// half is written with the SAME constants as `canon_robot_bank` so
/// the two normalizers' rows can be compared field-for-field on the
/// shared/mapped set (the record layout stays opaque to the
/// normalizer beyond the pinned offsets).
#[allow(clippy::too_many_arguments)]
fn exd_robot_record(
    x: i32,
    y: i32,
    z: i32,
    state: u16,
    drop: i32,
    stop: i32,
    hp: i32,
    alive: i32,
) -> Vec<u8> {
    let mut r = vec![0u8; 0xA8];
    r[0x00..0x04].copy_from_slice(&x.to_le_bytes());
    r[0x04..0x08].copy_from_slice(&y.to_le_bytes());
    r[0x08..0x0C].copy_from_slice(&z.to_le_bytes());
    r[0x0C..0x0E].copy_from_slice(&state.to_le_bytes());
    // Back half: the canon_robot_bank constants (independent §8
    // transcription of the same table).
    r[0x0E..0x10].copy_from_slice(&42u16.to_le_bytes()); // dir_byte
    r[0x10..0x12].copy_from_slice(&3u16.to_le_bytes()); // facing
    r[0x12..0x14].copy_from_slice(&7u16.to_le_bytes()); // anim
    r[0x18..0x1A].copy_from_slice(&2u16.to_le_bytes()); // variant
    for k in 0..8 {
        let v = (0x0102 + k * 0x0202) as u16; // 0x0102 .. 0x0F10
        r[0x1A + 2 * k..0x1C + 2 * k].copy_from_slice(&v.to_le_bytes());
    }
    r[0x2A..0x2C].copy_from_slice(&0u16.to_le_bytes()); // kind
    r[0x2E..0x30].copy_from_slice(&2u16.to_le_bytes()); // hit_flash
    r[0x30..0x32].copy_from_slice(&(-25i16).to_le_bytes()); // armor (i16)
    r[0x34..0x36].copy_from_slice(&100u16.to_le_bytes()); // alarm
    r[0x74..0x78].copy_from_slice(&stop.to_le_bytes());
    r[0x78..0x7C].copy_from_slice(&hp.to_le_bytes());
    r[0x7C..0x80].copy_from_slice(&alive.to_le_bytes());
    r[0x80..0x84].copy_from_slice(&drop.to_le_bytes()); // D88: +0x80 gate word
    r[0x88..0x8C].copy_from_slice(&32i32.to_le_bytes()); // shield
    r[0x8C..0x90].copy_from_slice(&5i32.to_le_bytes()); // shield_charges
    r[0x94..0x98].copy_from_slice(&10i32.to_le_bytes()); // battery
    r[0x98..0x9C].copy_from_slice(&2000i32.to_le_bytes()); // armor_pool
    r[0x9C..0x9E].copy_from_slice(&0u16.to_le_bytes()); // death_flag
    r[0xA0..0xA4].copy_from_slice(&0i32.to_le_bytes()); // shield_boost
    r[0xA4..0xA8].copy_from_slice(&(-99i32).to_le_bytes()); // alarm_ctr
    r
}

#[test]
fn o1_robot_bank_maps_the_pinned_fields() {
    let mut blob = Vec::new();
    blob.extend(exd_robot_record(
        21 << 13,
        73 << 13,
        65,
        3,
        41,
        1000000,
        5000,
        1,
    ));
    blob.extend(exd_robot_record(0, 0, 0, 0, 0, 0, 0, 0));
    let f = frame(0, vec![WatchRecord::new("robot-bank", blob)]);
    let rows = normalize_frame(&f, Channel::O1ExdDosboxX, &reg()).expect("normalizes");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.id, "robot-bank");
    // Field set = count + exactly the 31 §8-mapped leaf fields per
    // robot (probe_z = 8 leaves; the canonical target trio is NOT
    // record-sourced — RE-EXD-MAP §8 move-target note).
    let mut expect: Vec<String> = vec!["count".to_string()];
    for i in 0..2 {
        for n in [
            "pos_x",
            "pos_y",
            "z",
            "state",
            "dir_byte",
            "facing",
            "anim",
            "variant",
            "probe_z[0]",
            "probe_z[1]",
            "probe_z[2]",
            "probe_z[3]",
            "probe_z[4]",
            "probe_z[5]",
            "probe_z[6]",
            "probe_z[7]",
            "kind",
            "hit_flash",
            "armor",
            "alarm",
            "stop_dist",
            "hp",
            "alive",
            "drop_countdown",
            "shield",
            "shield_charges",
            "battery",
            "armor_pool",
            "death_flag",
            "shield_boost",
            "alarm_ctr",
        ] {
            expect.push(format!("robot[{i}].{n}"));
        }
    }
    let names: Vec<&str> = row.fields.iter().map(|(n, _)| n.as_str()).collect();
    let expect_ref: Vec<&str> = expect.iter().map(|s| s.as_str()).collect();
    assert_eq!(names, expect_ref);
    // Values (hand-derived from the fixture bytes).
    let get = |n: &str| row.field(n).cloned();
    assert_eq!(get("count"), Some(FieldVal::Int(2)));
    assert_eq!(
        get("robot[0].pos_x"),
        Some(FieldVal::Int((21 << 13) as i128))
    );
    assert_eq!(
        get("robot[0].pos_y"),
        Some(FieldVal::Int((73 << 13) as i128))
    );
    assert_eq!(get("robot[0].z"), Some(FieldVal::Int(65)));
    assert_eq!(get("robot[0].state"), Some(FieldVal::Int(3)));
    // D88: drop_countdown reads the +0x80 phase-gate dword.
    assert_eq!(get("robot[0].drop_countdown"), Some(FieldVal::Int(41)));
    assert_eq!(get("robot[0].stop_dist"), Some(FieldVal::Int(1000000)));
    assert_eq!(get("robot[0].hp"), Some(FieldVal::Int(5000)));
    assert_eq!(get("robot[0].alive"), Some(FieldVal::Int(1)));
    assert_eq!(get("robot[1].alive"), Some(FieldVal::Int(0)));
    // Back-half reads incl. the sign-extended i16 armor + the u16
    // zero-extends + the negative dword.
    assert_eq!(get("robot[0].dir_byte"), Some(FieldVal::Int(42)));
    assert_eq!(get("robot[0].facing"), Some(FieldVal::Int(3)));
    assert_eq!(get("robot[0].anim"), Some(FieldVal::Int(7)));
    assert_eq!(get("robot[0].variant"), Some(FieldVal::Int(2)));
    assert_eq!(get("robot[0].probe_z[0]"), Some(FieldVal::Int(0x0102)));
    assert_eq!(get("robot[0].probe_z[7]"), Some(FieldVal::Int(0x0F10)));
    assert_eq!(get("robot[0].kind"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[0].hit_flash"), Some(FieldVal::Int(2)));
    assert_eq!(get("robot[0].armor"), Some(FieldVal::Int(-25)));
    assert_eq!(get("robot[0].alarm"), Some(FieldVal::Int(100)));
    assert_eq!(get("robot[0].shield"), Some(FieldVal::Int(32)));
    assert_eq!(get("robot[0].shield_charges"), Some(FieldVal::Int(5)));
    assert_eq!(get("robot[0].battery"), Some(FieldVal::Int(10)));
    assert_eq!(get("robot[0].armor_pool"), Some(FieldVal::Int(2000)));
    assert_eq!(get("robot[0].death_flag"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[0].shield_boost"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[0].alarm_ctr"), Some(FieldVal::Int(-99)));
    // Anti-fabrication: the canonical target trio is record-external
    // (§5 move-target arrays) — never invented from record bytes.
    assert!(get("robot[0].target_present").is_none());
    assert!(get("robot[0].target_x").is_none());
    assert!(get("robot[0].target_y").is_none());
}

#[test]
fn o1_row_forms() {
    // beacon-family: five u16 cells -> canonical u32 words.
    let mut beacon = Vec::new();
    for v in [1u16, 0x197, 31, 46, 3] {
        beacon.extend_from_slice(&v.to_le_bytes());
    }
    // static-map-wh: 0x2c+4 span, h at +0, w at +0x2c.
    let mut span = vec![0u8; 0x30];
    span[0x00..0x04].copy_from_slice(&75u32.to_le_bytes());
    span[0x2c..0x30].copy_from_slice(&25u32.to_le_bytes());
    // typedb grid: all-zero -> len 0; nonzero -> len + bytes.
    let zeros = vec![0u8; 1875];
    let mut nonzero = vec![0u8; 8];
    nonzero[3] = 0xAB;
    let f = frame(
        0,
        vec![
            WatchRecord::new("beacon-family", beacon),
            WatchRecord::new("static-map-wh", span),
            WatchRecord::new("typedb-fade-byte", zeros),
            WatchRecord::new("armor-pad-reads", nonzero.clone()),
            WatchRecord::new("rng-state-a", 0xDEADBEEFu32.to_le_bytes()),
            WatchRecord::new("score", 1234u32.to_le_bytes()),
        ],
    );
    let rows = normalize_frame(&f, Channel::O1ExdDosboxX, &reg()).unwrap();
    let get = |id: &str, n: &str| rows.iter().find(|r| r.id == id).unwrap().field(n).cloned();
    assert_eq!(get("beacon-family", "flag"), Some(FieldVal::Int(1)));
    assert_eq!(get("beacon-family", "timer"), Some(FieldVal::Int(0x197)));
    assert_eq!(get("beacon-family", "tile.x"), Some(FieldVal::Int(31)));
    assert_eq!(get("beacon-family", "tile.y"), Some(FieldVal::Int(46)));
    assert_eq!(get("beacon-family", "tile.z"), Some(FieldVal::Int(3)));
    assert_eq!(get("static-map-wh", "w"), Some(FieldVal::Int(25)));
    assert_eq!(get("static-map-wh", "h"), Some(FieldVal::Int(75)));
    assert_eq!(get("typedb-fade-byte", "len"), Some(FieldVal::Int(0)));
    assert_eq!(
        get("armor-pad-reads", "len"),
        Some(FieldVal::Int(8)),
        "the len-0 equivalence applies only to all-zero grids"
    );
    assert_eq!(
        get("rng-state-a", "value"),
        Some(FieldVal::Int(0xDEAD_BEEF))
    );
    assert_eq!(get("score", "value"), Some(FieldVal::Int(1234)));
}

#[test]
fn o1_move_target_splice_into_robot_bank() {
    // The D90 form: a 0x60 span whose slots carry the per-robot x/y
    // u32 pairs by ABSOLUTE id (-1 = none). Robot 0 holds a Q5 target
    // (tile<<5 units), robot 1 cleared, slots >= count never read.
    let mut span = vec![0xFFu8; 0x60];
    span[0x00..0x04].copy_from_slice(&0x0012_3400i32.to_le_bytes()); // x[0]
    span[0x30..0x34].copy_from_slice(&(-256i32).to_le_bytes()); // y[0]
    span[0x04..0x08].copy_from_slice(&(-1i32).to_le_bytes()); // x[1] = none
    span[0x34..0x38].copy_from_slice(&(-1i32).to_le_bytes()); // y[1]
    let mut blob = Vec::new();
    blob.extend(exd_robot_record(
        21 << 13,
        73 << 13,
        65,
        3,
        41,
        1000000,
        5000,
        1,
    ));
    blob.extend(exd_robot_record(0, 0, 0, 0, 0, 0, 0, 0));
    let f = frame(
        0,
        vec![
            WatchRecord::new("robot-bank", blob),
            WatchRecord::new("move-target-words", span),
        ],
    );
    let rows = normalize_frame(&f, Channel::O1ExdDosboxX, &reg()).unwrap();
    // The span carries NO standalone raw row — consumed by the splice.
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "robot-bank");
    let row = &rows[0];
    let get = |n: &str| row.field(n).cloned();
    // Canonicalized trio: present x!=−1, absent -> (0, 0, 0).
    assert_eq!(get("robot[0].target_present"), Some(FieldVal::Int(1)));
    assert_eq!(get("robot[0].target_x"), Some(FieldVal::Int(0x0012_3400)));
    assert_eq!(get("robot[0].target_y"), Some(FieldVal::Int(-256)));
    assert_eq!(get("robot[1].target_present"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[1].target_x"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[1].target_y"), Some(FieldVal::Int(0)));
    // Spliced at the canonical position: right after stop_dist,
    // before hp (the CANON_ROBOT_FIELDS order).
    let names: Vec<&str> = row.fields.iter().map(|(n, _)| n.as_str()).collect();
    let at = names
        .iter()
        .position(|n| *n == "robot[0].stop_dist")
        .unwrap();
    assert_eq!(
        &names[at + 1..at + 4],
        [
            "robot[0].target_present",
            "robot[0].target_x",
            "robot[0].target_y"
        ]
    );
    assert_eq!(names[at + 4], "robot[0].hp");
}

#[test]
fn o1_move_target_span_guards() {
    let robot = WatchRecord::new("robot-bank", exd_robot_record(0, 0, 0, 0, 0, 0, 0, 0));
    // Wrong length: loud BadLength naming the pinned 0x60 form.
    let err = normalize_frame(
        &frame(
            0,
            vec![WatchRecord::new("move-target-words", vec![0u8; 13])],
        ),
        Channel::O1ExdDosboxX,
        &reg(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("move-target-words"), "{err}");
    assert!(err.to_string().contains("0x60"), "{err}");
    // A span without the robot-bank row has no bound — never guessed.
    let err = normalize_frame(
        &frame(
            0,
            vec![WatchRecord::new("move-target-words", vec![0xFFu8; 0x60])],
        ),
        Channel::O1ExdDosboxX,
        &reg(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("the robot-bank row in the same frame"),
        "{err}"
    );
    // More robots than span slots: loud, never truncated.
    let mut blob = Vec::new();
    for _ in 0..13 {
        blob.extend(exd_robot_record(0, 0, 0, 0, 0, 0, 0, 0));
    }
    let err = normalize_frame(
        &frame(
            0,
            vec![
                WatchRecord::new("robot-bank", blob),
                WatchRecord::new("move-target-words", vec![0xFFu8; 0x60]),
            ],
        ),
        Channel::O1ExdDosboxX,
        &reg(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("≤ 12"), "{err}");
    // Sanity: the bank row alone (no span) still normalizes — an old
    // plan without the row leaves the trio uncovered, not broken.
    let rows = normalize_frame(&frame(0, vec![robot]), Channel::O1ExdDosboxX, &reg()).unwrap();
    assert!(rows[0].field("robot[0].target_present").is_none());
}

// ---------------------------------------------------------------------
// 2. The E normalizer: the pinned §6a canonical grammar
// ---------------------------------------------------------------------

/// The W6 gate's hand-encoded 98-byte canonical robot-bank fixture
/// (count 1; see engine canonical_dump_gate.rs — an independent
/// transcription of the §6a table, re-typed here).
fn canon_robot_bank() -> Vec<u8> {
    vec![
        0x01, 0x00, 0x00, 0x00, // count
        0x01, // alive
        0x00, 0x00, 0x10, 0x00, // pos_x 0x00100000
        0x00, 0xFF, 0xFF, 0xFF, // pos_y -256
        0x41, 0x00, 0x00, 0x00, // z 65
        0x06, 0x00, 0x2A, 0x00, 0x03, 0x00, 0x07, 0x00, 0x02,
        0x00, // state/dir/facing/anim/variant
        0x02, 0x01, 0x04, 0x03, 0x06, 0x05, 0x08, 0x07, 0x0A, 0x09, 0x0C, 0x0B, 0x0E, 0x0D, 0x10,
        0x0F, // probe_z
        0xFD, 0xFF, 0xFF, 0xFF, // stop_dist -3
        0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0xFF, 0xFF, 0xFF, // target present/tx/ty
        0x99, 0x00, 0x00, 0x00, 0x88, 0x13, 0x00, 0x00, // drop 0x99, hp 5000
        0xE7, 0xFF, 0x02, 0x00, 0x64, 0x00, 0x00,
        0x00, // armor -25, flash 2, alarm 100, kind 0
        0x20, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // shield family
        0x0A, 0x00, 0x00, 0x00, 0xD0, 0x07, 0x00, 0x00, // battery, armor_pool
        0x9D, 0xFF, 0xFF, 0xFF, 0x00, 0x00, // alarm_ctr -99, death_flag
    ]
}

#[test]
fn engine_robot_bank_parses_the_full_canonical_record() {
    let f = frame(0, vec![WatchRecord::new("robot-bank", canon_robot_bank())]);
    let rows = normalize_frame(&f, Channel::Engine, &reg()).unwrap();
    let row = &rows[0];
    let get = |n: &str| row.field(n).cloned();
    assert_eq!(
        row.fields.len(),
        1 + 34,
        "count + 34 record fields (probe_z x8, target x3)"
    );
    assert_eq!(get("count"), Some(FieldVal::Int(1)));
    assert_eq!(get("robot[0].alive"), Some(FieldVal::Int(1)));
    assert_eq!(get("robot[0].pos_x"), Some(FieldVal::Int(0x0010_0000)));
    assert_eq!(get("robot[0].pos_y"), Some(FieldVal::Int(-256)));
    assert_eq!(get("robot[0].z"), Some(FieldVal::Int(65)));
    assert_eq!(get("robot[0].state"), Some(FieldVal::Int(6)));
    assert_eq!(get("robot[0].dir_byte"), Some(FieldVal::Int(42)));
    assert_eq!(get("robot[0].facing"), Some(FieldVal::Int(3)));
    assert_eq!(get("robot[0].anim"), Some(FieldVal::Int(7)));
    assert_eq!(get("robot[0].variant"), Some(FieldVal::Int(2)));
    assert_eq!(get("robot[0].probe_z[0]"), Some(FieldVal::Int(0x0102)));
    assert_eq!(get("robot[0].probe_z[7]"), Some(FieldVal::Int(0x0F10)));
    assert_eq!(get("robot[0].stop_dist"), Some(FieldVal::Int(-3)));
    assert_eq!(get("robot[0].target_present"), Some(FieldVal::Int(1)));
    assert_eq!(get("robot[0].target_x"), Some(FieldVal::Int(0x0012_3400)));
    assert_eq!(get("robot[0].target_y"), Some(FieldVal::Int(-256)));
    assert_eq!(get("robot[0].drop_countdown"), Some(FieldVal::Int(0x99)));
    assert_eq!(get("robot[0].hp"), Some(FieldVal::Int(5000)));
    assert_eq!(get("robot[0].armor"), Some(FieldVal::Int(-25)));
    assert_eq!(get("robot[0].hit_flash"), Some(FieldVal::Int(2)));
    assert_eq!(get("robot[0].alarm"), Some(FieldVal::Int(100)));
    assert_eq!(get("robot[0].kind"), Some(FieldVal::Int(0)));
    assert_eq!(get("robot[0].shield"), Some(FieldVal::Int(32)));
    assert_eq!(get("robot[0].shield_charges"), Some(FieldVal::Int(5)));
    assert_eq!(get("robot[0].battery"), Some(FieldVal::Int(10)));
    assert_eq!(get("robot[0].armor_pool"), Some(FieldVal::Int(2000)));
    assert_eq!(get("robot[0].alarm_ctr"), Some(FieldVal::Int(-99)));
    assert_eq!(get("robot[0].death_flag"), Some(FieldVal::Int(0)));
}

/// The shared-field contract: the O1 normalizer's output for a robot
/// the EXD record describes must EQUAL the E canonical parse of the
/// same robot's canonical record (on the §8-mapped fields — including
/// the D90 spliced target trio, sourced from the record-external
/// §5 span).
#[test]
fn o1_and_engine_robot_rows_agree_on_shared_fields() {
    // Canonical fixture robot (see canon_robot_bank): pos 0x100000/-256,
    // z 65, state 6, drop 0x99, stop -3, hp 5000, alive 1, target
    // present @ (0x0012_3400, -256).
    let exd = exd_robot_record(0x0010_0000, -256, 65, 6, 0x99, -3, 5000, 1);
    let mut span = vec![0xFFu8; 0x60];
    span[0x00..0x04].copy_from_slice(&0x0012_3400i32.to_le_bytes()); // x[0]
    span[0x30..0x34].copy_from_slice(&(-256i32).to_le_bytes()); // y[0]
    let f_o1 = frame(
        0,
        vec![
            WatchRecord::new("robot-bank", exd),
            WatchRecord::new("move-target-words", span),
        ],
    );
    let f_e = frame(0, vec![WatchRecord::new("robot-bank", canon_robot_bank())]);
    let o1 = normalize_frame(&f_o1, Channel::O1ExdDosboxX, &reg()).unwrap();
    let e = normalize_frame(&f_e, Channel::Engine, &reg()).unwrap();
    for (name, _) in &o1[0].fields {
        let a = o1[0].field(name);
        let b = e[0].field(name);
        assert_eq!(
            a, b,
            "shared field {name} must agree across the two normalizers"
        );
    }
    // The splice closed the last gap: every O1 leaf is shared now.
    assert!(o1[0].field("robot[0].target_x").is_some());
    assert_eq!(o1[0].fields.len(), e[0].fields.len());
}

// ---------------------------------------------------------------------
// 3. Modes: double-run (DH-G1) + cross-channel with O2 arbitration
// ---------------------------------------------------------------------

const SCEN_T0: &str = "scenario = \"DX\"\ntiers = T0\nframes = 2\n";

fn t0_frame(no: u64, counter: u32, rng_a: u32, rng_b: u32, score: u32) -> FrameRecord {
    frame(
        no,
        vec![
            WatchRecord::new("frame-counter", counter.to_le_bytes().to_vec()),
            WatchRecord::new("rng-state-a", rng_a.to_le_bytes().to_vec()),
            WatchRecord::new("rng-state-b", rng_b.to_le_bytes().to_vec()),
            WatchRecord::new("score", score.to_le_bytes().to_vec()),
        ],
    )
}

#[test]
fn double_run_passes_modulo_counter_and_rng() {
    let a = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 100, 11, 21, 500),
            t0_frame(1, 101, 12, 22, 500),
            t0_frame(2, 102, 13, 23, 500),
        ],
        Channel::O1ExdDosboxX,
    );
    // Second run: counter drifts by 3 each frame (never resets), the
    // RNG blobs wander freely, everything else identical.
    let b = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 103, 91, 81, 500),
            t0_frame(1, 106, 92, 82, 500),
            t0_frame(2, 109, 93, 83, 500),
        ],
        Channel::O1ExdDosboxX,
    );
    let res = run_diff(&a, &b, None, &DiffConfig::new(Mode::DoubleRun), &reg()).unwrap();
    assert_eq!(res.verdict, Verdict::Pass, "{}", report_text(&res));
    // No value findings at all; the T2 counter diffs are suppressed by
    // the quantum, the T3 rows never compare.
    assert!(res.findings.is_empty());
    assert!(!res.suppressed.is_empty(), "counter deltas are counted");
    assert_eq!(res.count(Class::EngineBug), 0);
}

#[test]
fn double_run_fails_on_any_other_byte_diff() {
    let a = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 100, 11, 21, 500),
            t0_frame(1, 101, 12, 22, 500),
            t0_frame(2, 102, 13, 23, 500),
        ],
        Channel::O1ExdDosboxX,
    );
    let b = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 100, 11, 21, 500),
            t0_frame(1, 101, 12, 22, 620),
            t0_frame(2, 102, 13, 23, 620),
        ],
        Channel::O1ExdDosboxX,
    );
    let res = run_diff(&a, &b, None, &DiffConfig::new(Mode::DoubleRun), &reg()).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    let first = res.first_divergence().unwrap();
    assert_eq!(first.row, "score");
    assert_eq!(first.first_frame, 1);
    assert_eq!(first.a, Some(FieldVal::Int(500)));
    assert_eq!(first.b, Some(FieldVal::Int(620)));
    // T3 draw counts: the rng rows change on both sides equally here.
    assert_eq!(res.count(Class::Structural), 0);
}

#[test]
fn t3_draw_count_mismatch_is_structural() {
    // Side B's rng-a never changes after frame 0 (fewer draws).
    let a = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 100, 11, 21, 500),
            t0_frame(1, 101, 12, 22, 500),
            t0_frame(2, 102, 13, 23, 500),
        ],
        Channel::O1ExdDosboxX,
    );
    let b = dump_bytes(
        SCEN_T0,
        vec![
            t0_frame(0, 100, 11, 21, 500),
            t0_frame(1, 101, 11, 22, 500),
            t0_frame(2, 102, 11, 23, 500),
        ],
        Channel::O1ExdDosboxX,
    );
    let res = run_diff(&a, &b, None, &DiffConfig::new(Mode::DoubleRun), &reg()).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    let finding = res
        .findings
        .iter()
        .find(|f| f.row == "rng-state-a")
        .expect("draw-count mismatch reported");
    assert_eq!(finding.class, Class::Structural);
    assert!(finding.detail.contains("draw-count"));
}

const SCEN_T0TS: &str = "scenario = \"CX\"\ntiers = T0,TS\nframes = 2\n";

/// An E-side frame: canonical rows.
fn e_frame(no: u64, money: u32) -> FrameRecord {
    let mut f = frame(
        no,
        vec![
            WatchRecord::new("frame-counter", (no as u32).to_le_bytes().to_vec()),
            WatchRecord::new(
                "rng-state-a",
                0x1234_5678_9ABC_DEF0u64.to_le_bytes().to_vec(),
            ),
            WatchRecord::new("money", money.to_le_bytes().to_vec()),
        ],
    );
    if no == 0 {
        let mut wh = Vec::new();
        wh.extend_from_slice(&25u32.to_le_bytes());
        wh.extend_from_slice(&75u32.to_le_bytes());
        f.push_watch("static-map-wh", wh);
    }
    f
}

/// The O1-side fabrication (the INVERSE of the O1 normalizer —
/// test-only; any map change breaks this coupling loudly).
fn o1_frame(no: u64, money: u32) -> FrameRecord {
    let mut f = frame(
        no,
        vec![
            WatchRecord::new("frame-counter", (5000 + no as u32).to_le_bytes().to_vec()),
            WatchRecord::new("rng-state-a", 0xABCD_EF01u32.to_le_bytes().to_vec()),
            WatchRecord::new("money", money.to_le_bytes().to_vec()),
        ],
    );
    if no == 0 {
        let mut span = vec![0u8; 0x30];
        span[0x00..0x04].copy_from_slice(&75u32.to_le_bytes());
        span[0x2c..0x30].copy_from_slice(&25u32.to_le_bytes());
        f.push_watch("static-map-wh", span);
    }
    f
}

fn cross_pair(money_e: u32, money_o1: u32) -> (Vec<u8>, Vec<u8>) {
    let e = dump_bytes(
        SCEN_T0TS,
        vec![
            e_frame(0, money_e),
            e_frame(1, money_e),
            e_frame(2, money_e),
        ],
        Channel::Engine,
    );
    let o1 = dump_bytes(
        SCEN_T0TS,
        vec![
            o1_frame(0, money_o1),
            o1_frame(1, money_o1),
            o1_frame(2, money_o1),
        ],
        Channel::O1ExdDosboxX,
    );
    (e, o1)
}

#[test]
fn cross_channel_clean_pass_is_pass() {
    let (e, o1) = cross_pair(3000, 3000);
    let res = run_diff(&e, &o1, None, &DiffConfig::new(Mode::CrossChannel), &reg()).unwrap();
    // The frame-counter row is T2-report-only by construction (the O1
    // counter never resets — menu frames included — so a live O1 value
    // never matches E's 0..N): the clean run verdict is PASS-WITH-NOTES
    // carrying exactly that one note, nothing failing.
    assert_eq!(res.verdict, Verdict::PassWithNotes, "{}", report_text(&res));
    assert_eq!(res.paired_frames, 3);
    assert_eq!(res.count(Class::T2Reported), 1);
    assert!(res
        .findings
        .iter()
        .all(|f| f.row == "frame-counter" && f.class == Class::T2Reported));
    // Event timing: money never changes on either side.
    let (_, _, ca, cb) = res.timing.get("money").unwrap();
    assert_eq!((*ca, *cb), (0, 0));
}

#[test]
fn cross_channel_engine_bug_then_o2_arbitration() {
    let (e, o1) = cross_pair(3000, 2999);

    // No tiebreak: provisional engine-bug.
    let res = run_diff(&e, &o1, None, &DiffConfig::new(Mode::CrossChannel), &reg()).unwrap();
    assert_eq!(res.verdict, Verdict::Fail);
    let f = res.findings.iter().find(|f| f.row == "money").unwrap();
    assert_eq!(f.class, Class::EngineBug);
    assert!(f.detail.contains("provisional"));

    // O2 agrees with O1 (EXD == EXW canon; E is the outlier).
    let o2 = dump_bytes(
        SCEN_T0TS,
        vec![o1_frame(0, 2999), o1_frame(1, 2999), o1_frame(2, 2999)],
        Channel::O2ExwWine,
    );
    let res = run_diff(
        &e,
        &o1,
        Some(&o2),
        &DiffConfig::new(Mode::CrossChannel),
        &reg(),
    )
    .unwrap();
    assert_eq!(res.verdict, Verdict::Fail, "engine-bug still fails");
    let f = res.findings.iter().find(|f| f.row == "money").unwrap();
    assert_eq!(f.class, Class::EngineBug);
    assert!(f.detail.contains("outlier"));

    // O2 agrees with E (EXD diverges from EXW canon) -> original
    // divergence, a NOTE not a failure.
    let o2 = dump_bytes(
        SCEN_T0TS,
        vec![o1_frame(0, 3000), o1_frame(1, 3000), o1_frame(2, 3000)],
        Channel::O2ExwWine,
    );
    let res = run_diff(
        &e,
        &o1,
        Some(&o2),
        &DiffConfig::new(Mode::CrossChannel),
        &reg(),
    )
    .unwrap();
    assert_eq!(res.verdict, Verdict::PassWithNotes);
    let f = res.findings.iter().find(|f| f.row == "money").unwrap();
    assert_eq!(f.class, Class::OriginalDivergence);
    assert!(f.detail.contains("DIVERGENCES"));
}

// ---------------------------------------------------------------------
// 4. Coverage, alignment, reports
// ---------------------------------------------------------------------

#[test]
fn coverage_asymmetry_is_reported_never_silent() {
    // E carries robot-bank (canonical 1 robot); O1 carries the same
    // row from an EXD record + a TS static the E side never emits.
    let mut e_frames = Vec::new();
    let mut o1_frames = Vec::new();
    for no in 0..3u64 {
        let mut ef = frame(no, vec![WatchRecord::new("robot-bank", canon_robot_bank())]);
        if no == 0 {
            ef.push_watch("score", 10u32.to_le_bytes().to_vec());
        }
        e_frames.push(ef);
        let mut of = frame(
            no,
            vec![
                // The EXD record mirrors the canonical fixture on the
                // §8-mapped fields (see canon_robot_bank).
                WatchRecord::new(
                    "robot-bank",
                    exd_robot_record(0x0010_0000, -256, 65, 6, 0x99, -3, 5000, 1),
                ),
                WatchRecord::new("score", 10u32.to_le_bytes().to_vec()),
            ],
        );
        if no == 0 {
            of.push_watch("static-type-table", vec![0x11, 0x22, 0x33, 0x44]);
        }
        o1_frames.push(of);
    }
    let scen = "scenario = \"CV\"\ntiers = T0,T1,TS\nframes = 2\n";
    let e = dump_bytes(scen, e_frames, Channel::Engine);
    let o1 = dump_bytes(scen, o1_frames, Channel::O1ExdDosboxX);
    let res = run_diff(&e, &o1, None, &DiffConfig::new(Mode::CrossChannel), &reg()).unwrap();

    // Row-level: static-type-table is O1-only (an E-gap).
    assert!(res
        .findings
        .iter()
        .any(|f| f.row == "static-type-table" && f.class == Class::Coverage));
    // Field-level: the unmapped canonical robot fields are coverage
    // findings (34 canonical record fields - 31 mapped = 3 gaps: the
    // target trio, record-external per RE-EXD-MAP §8).
    let robot_gaps = res
        .findings
        .iter()
        .filter(|f| f.row == "robot-bank" && f.class == Class::Coverage && f.field != "count")
        .count();
    assert_eq!(robot_gaps, 3, "34 canonical record fields - 31 mapped");
    // And nothing was fabricated into a VALUE compare: no engine-bug
    // findings on the shared fields (the EXD record was built to match
    // the canonical one on the mapped set).
    assert_eq!(res.count(Class::EngineBug), 0, "{}", report_text(&res));
    assert_eq!(res.verdict, Verdict::PassWithNotes);
}

#[test]
fn constant_shift_is_detected_and_reported() {
    let a = dump_bytes(
        SCEN_T0TS,
        vec![e_frame(0, 7), e_frame(1, 7), e_frame(2, 7)],
        Channel::Engine,
    );
    // B runs the same scenario two frames later (anchor shift +2).
    let b = dump_bytes(
        "scenario = \"CX\"\ntiers = T0,TS\nframes = 2\n",
        vec![e_frame(2, 7), e_frame(3, 7), e_frame(4, 7)],
        Channel::Engine,
    );
    let res = run_diff(&a, &b, None, &DiffConfig::new(Mode::CrossChannel), &reg()).unwrap();
    assert_eq!(res.shift, 2);
    assert_eq!(res.paired_frames, 3);
    // The shift itself is a T1-timing note, not a failure.
    assert_eq!(res.verdict, Verdict::PassWithNotes, "{}", report_text(&res));
    assert!(res.findings.iter().any(|f| f.detail.contains("shift")));
}

#[test]
fn report_and_manifest_are_deterministic() {
    let (e, o1) = cross_pair(3000, 2990);
    let cfg = DiffConfig::new(Mode::CrossChannel);
    let r1 = run_diff(&e, &o1, None, &cfg, &reg()).unwrap();
    let r2 = run_diff(&e, &o1, None, &cfg, &reg()).unwrap();
    assert_eq!(report_text(&r1), report_text(&r2));
    assert_eq!(manifest_json(&r1), manifest_json(&r2));
    let text = report_text(&r1);
    assert!(text.contains("BEDLAM DIFF REPORT"));
    assert!(text.contains("mode: cross-channel"));
    assert!(text.contains("METER:"));
    assert!(text.contains("FIRST DIVERGENCE:"));
    assert!(text.contains("VERDICT: FAIL"));
    assert!(text.contains(&r1.a.chain_digest));
    assert!(text.contains(&r1.b.chain_digest));
    let json = manifest_json(&r1);
    assert!(json.contains("\"verdict\": \"FAIL\""));
    assert!(json.contains("\"chain_digest\": \""));
    assert!(json.contains("\"first_divergence\": {\"frame\": 0, \"row\": \"money\""));
}

#[test]
fn scenario_mismatch_is_an_error() {
    let a = dump_bytes(
        SCEN_T0TS,
        vec![e_frame(0, 7), e_frame(1, 7), e_frame(2, 7)],
        Channel::Engine,
    );
    let b = dump_bytes(
        "scenario = \"OTHER\"\ntiers = T0,TS\nframes = 2\n",
        vec![e_frame(0, 7), e_frame(1, 7), e_frame(2, 7)],
        Channel::Engine,
    );
    let err = run_diff(&a, &b, None, &DiffConfig::new(Mode::CrossChannel), &reg()).unwrap_err();
    assert!(err.to_string().contains("scenario mismatch"));
}

#[test]
fn encode_dump_canonicalizes_watch_order() {
    // The differ relies on dumps carrying registry order; encode_dump
    // already enforces it — spot-check one frame both ways.
    let scen = Scenario::parse(SCEN_T0).unwrap();
    let mut f = t0_frame(0, 1, 2, 3, 4);
    f.watches.reverse();
    let bytes = encode_dump(
        &DumpHeader::new(Channel::O1ExdDosboxX, sha256(b"x"), "DX"),
        &[f],
        &reg(),
    )
    .unwrap();
    let d = diffharness::dump::decode_dump(&bytes).unwrap();
    assert_eq!(d.frames[0].watches[0].id, "frame-counter");
    let _ = scen;
}
