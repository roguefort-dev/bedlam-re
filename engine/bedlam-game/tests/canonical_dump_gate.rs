//! W6 canonical dump gate (DESIGN-DIFFHARNESS.md §6a/§10-W6, D85) —
//! the verification half of the engine dump emitter. Three tiers:
//!
//! 1. SYNTHETIC GRAMMAR FIXTURE (no corpus): a hand-built
//!    [`canonical::TickState`] emitted through
//!    [`canonical::emit_frame`] and compared byte-for-byte against
//!    HAND-ENCODED literals written straight from the §6a grammar
//!    table — an independent transcription, not a re-run of
//!    `robot_bank_blob` (the independence is the point: it pins the
//!    CONTRACT W7's normalizer must match). The frame digest is
//!    pinned; a grammar drift moves it loudly.
//! 2. SYNTHETIC SIM RUN (no corpus): a headless MissionSim (flat
//!    terrain, zeroed angle table, pinned seed) ticked three frames
//!    through the SAME emit_frame + `runner::stitch` + `encode_dump`
//!    path the live channel uses; `decode_dump` verifies the stream
//!    and the chain digest is pinned + double-run byte-identical.
//! 3. CORPUS-GATED S0/S1 (skips when game-data is absent): full
//!    `run_canonical` drives over the REAL shipped ZONEA/MISSION1 —
//!    3/401 records, pinned chain digests, byte-identical double
//!    runs — plus the scenario-step seam gates: boot difficulty
//!    consumed (money seed via menu::start_score), walk-phase
//!    non-boot steps rejected naming the P2e seam, command payloads
//!    CONSUMED by the W12-S3-prep fire seam (a ≥14 B record stages
//!    into the sim ring; short payloads fail loud; with no staged
//!    weapon slots nothing fires — the no-inject invariant), pad
//!    rejected naming the S6 seam, P-pause banned mid-scenario,
//!    and the order seam arming at the tile-exact robot.
//! 4. CORPUS-GATED S2 (the W8-s2 order→walk slice, D91): the
//!    `markers` staging key banks the mission_corpus_gate walker at
//!    (18,73) beside the MRK robot; `order 21 73 1` arms the beacon
//!    at the MRK robot's tile and the WALKER consumes it (spread
//!    slot 1 = (22,73)) — the first corpus scenario with a live
//!    present=1 move-target window, the arrival snap (one tile short
//!    of the slot target, west approach), and the beacon/claims
//!    clear on all-state-3. Pinned chain; byte-identical double run.
//!
//! PIN DISCIPLINE: the digest/chain pins below are fingerprints of
//! deliberate engine/dump behavior — they move only when the engine
//! behavior or the §6a grammar changes, and then they are re-baselined
//! DELIBERATELY with a commit message saying why (the fingerprint
//! discipline, D28). Dumps stay runtime-only (§3 hygiene); git carries
//! only the digests asserted here.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::PathBuf;

use bedlam_core::mission::{AngleTable, MissionSim, Order, Robot, Terrain, ORDER_WINDOW};
use bedlam_core::weapon::WeaponSlot;
use canonical::{emit_frame, run_canonical, TickState};
use diffharness::dump::{canonicalize_frame, decode_dump, frame_digest, Channel, DumpHeader};
use diffharness::hash::sha256;
use diffharness::registry;
use diffharness::runner::{stitch, Scenario, Transcript};

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
// 1. The synthetic §6a grammar fixture
// ---------------------------------------------------------------------

/// The fixture robot: every field a distinct value so a field-order or
/// width drift flips bytes (negatives chosen for pos_y, stop_dist,
/// armor, alarm_ctr to pin sign extension).
fn fixture_robot() -> Robot {
    Robot {
        pos_x: 0x0010_0000,
        pos_y: -256,
        z: 65,
        state: 6,
        dir_byte: 42,
        facing: 3,
        anim: 7,
        variant: 2,
        probe_z: [
            0x0102, 0x0304, 0x0506, 0x0708, 0x090A, 0x0B0C, 0x0D0E, 0x0F10,
        ],
        stop_dist: -3,
        target: Some((0x0012_3400, -256)),
        alive: true,
        drop_countdown: 0x99,
        hp: 5000,
        armor: -25,
        hit_flash: 2,
        alarm: 100,
        alarm_ctr: -99,
        shield: 32,
        shield_charges: 5,
        shield_boost: 0,
        battery: 10,
        armor_pool: 2000,
        kind: 0,
        death_flag: 0,
        weapons: [WeaponSlot::default(); 7],
        weapon_mask: 0,
    }
}

#[test]
fn synthetic_grammar_pins_the_6a_bytes() {
    let robot = fixture_robot();
    let mut claims = [false; 12];
    claims[0] = true;
    claims[2] = true;
    let order = Order {
        tile: (31, 46, 3),
        window: 0x0103,
        claims,
    };
    let pads = [0xABu8, 0xCD, 0xEF];
    // The staged claim bank (S0-11b): a compact hand-made image —
    // the row is a raw byte span (no count prefix, no field map;
    // the D136 static-map-wh fixed-extent precedent).
    let claims: [u8; 8] = [0, 1, 1, 0, 0, 0, 1, 0];
    let st = TickState {
        frame_no: 7,
        rand_a_state: 0x0123_4567_89AB_CDEF,
        rand_b_state: 0xFEDC_BA98_7654_3210,
        score: 1234,
        money: 2500,
        difficulty: 3,
        zone: 0,
        mission: 1,
        mode: 0,
        linear: 17,
        robots: std::slice::from_ref(&robot),
        order: Some(order),
        beacon_latch: None,
        claims_latch: [false; 12],
        dropship: None,
        selected: 0,
        blink_cursor: 2,
        order_target: (31, 46, 3),
        armor_pads: &pads,
        map_wh: Some((300, 150)),
        claim_bank: &claims,
        player_type: 0,
        weapon_bank: &[],
        enemy_bank: &[],
        critter: None,
        destroy: None,
    };
    let tiers: Vec<String> = ["T0", "T1", "TS"].iter().map(|s| s.to_string()).collect();
    let frame = emit_frame(&st, &tiers, true, true);

    assert_eq!(frame.frame_no, 7);
    assert!(frame.injection_applied);

    // --- T0 rows (§6a): u32 scalars, little-endian ---
    assert_eq!(frame.watch("frame-counter"), Some(&[0x07, 0, 0, 0][..]));
    assert_eq!(
        frame.watch("rng-state-a"),
        Some(&[0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01][..])
    );
    assert_eq!(
        frame.watch("rng-state-b"),
        Some(&[0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE][..])
    );
    assert_eq!(frame.watch("score"), Some(&1234u32.to_le_bytes()[..]));
    assert_eq!(frame.watch("money"), Some(&2500u32.to_le_bytes()[..]));
    assert_eq!(frame.watch("difficulty"), Some(&3u32.to_le_bytes()[..]));
    assert_eq!(frame.watch("zone"), Some(&0u32.to_le_bytes()[..]));
    assert_eq!(frame.watch("mission"), Some(&1u32.to_le_bytes()[..]));
    assert_eq!(frame.watch("mode"), Some(&0u32.to_le_bytes()[..]));
    assert_eq!(
        frame.watch("linear-mission-m"),
        Some(&17u32.to_le_bytes()[..])
    );
    // The SFX master gate (D136): the sound-on construction constant.
    assert_eq!(
        frame.watch("sfx-master-gate"),
        Some(&1u32.to_le_bytes()[..])
    );

    // --- T1 rows ---
    // robot-bank: u32 count + the state_hash field order
    // (alive u8, pos_x i32, pos_y i32, z i32, state u16, dir_byte u16,
    // facing u16, anim u16, variant u16, probe_z u16×8, stop_dist i32,
    // present u8, tx i32, ty i32, drop_countdown i32, hp i32,
    // armor i16, hit_flash u16, alarm u16, kind u16, shield i32,
    // shield_charges i32, shield_boost i32, battery i32,
    // armor_pool i32, alarm_ctr i32, death_flag u16) — 98 bytes total.
    let expect_robot_bank: [u8; 98] = [
        // count = 1
        0x01, 0x00, 0x00, 0x00, //
        // alive
        0x01, //
        // pos_x 0x00100000
        0x00, 0x00, 0x10, 0x00, //
        // pos_y -256 (0xFFFFFF00)
        0x00, 0xFF, 0xFF, 0xFF, //
        // z 65
        0x41, 0x00, 0x00, 0x00, //
        // state 6, dir_byte 42, facing 3, anim 7, variant 2
        0x06, 0x00, 0x2A, 0x00, 0x03, 0x00, 0x07, 0x00, 0x02, 0x00, //
        // probe_z 0x0102..0x0F10 (u16 LE each)
        0x02, 0x01, 0x04, 0x03, 0x06, 0x05, 0x08, 0x07, 0x0A, 0x09, 0x0C, 0x0B, 0x0E, 0x0D, 0x10,
        0x0F, //
        // stop_dist -3
        0xFD, 0xFF, 0xFF, 0xFF, //
        // target present 1, tx 0x00123400, ty -256
        0x01, 0x00, 0x34, 0x12, 0x00, 0x00, 0xFF, 0xFF, 0xFF, //
        // drop_countdown 0x99, hp 5000
        0x99, 0x00, 0x00, 0x00, 0x88, 0x13, 0x00, 0x00, //
        // armor -25 (i16), hit_flash 2, alarm 100, kind 0
        0xE7, 0xFF, 0x02, 0x00, 0x64, 0x00, 0x00, 0x00, //
        // shield 32, charges 5, boost 0
        0x20, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
        // battery 10, armor_pool 2000
        0x0A, 0x00, 0x00, 0x00, 0xD0, 0x07, 0x00, 0x00, //
        // alarm_ctr -99, death_flag 0
        0x9D, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
    ];
    assert_eq!(frame.watch("robot-bank"), Some(&expect_robot_bank[..]));

    // selection-triple: the 4-byte selected-idx alias (D83 form).
    assert_eq!(
        frame.watch("selection-triple"),
        Some(&0u32.to_le_bytes()[..])
    );
    // blink-cursor: u32.
    assert_eq!(frame.watch("blink-cursor"), Some(&2u32.to_le_bytes()[..]));

    // per-player-selected: 4 × {x i32, y i32, z i32}; player 0 =
    // selected robot pos>>8 (Q5) + z; players 1..3 zero.
    let mut expect_players: Vec<u8> = Vec::new();
    expect_players.extend_from_slice(&4096i32.to_le_bytes()); // 0x00100000>>8
    expect_players.extend_from_slice(&(-1i32).to_le_bytes()); // -256>>8
    expect_players.extend_from_slice(&65i32.to_le_bytes());
    expect_players.extend_from_slice(&[0u8; 36]);
    assert_eq!(
        frame.watch("per-player-selected"),
        Some(&expect_players[..])
    );

    // order-target: i32 ×3 (the seam write persists).
    let mut expect_target = Vec::new();
    expect_target.extend_from_slice(&31i32.to_le_bytes());
    expect_target.extend_from_slice(&46i32.to_le_bytes());
    expect_target.extend_from_slice(&3i32.to_le_bytes());
    assert_eq!(frame.watch("order-target"), Some(&expect_target[..]));

    // move-target-words: u32 count + per-robot {present u8, tx i32, ty i32}.
    let expect_moves: [u8; 13] = [
        0x01, 0x00, 0x00, 0x00, // count
        0x01, // present
        0x00, 0x34, 0x12, 0x00, // tx
        0x00, 0xFF, 0xFF, 0xFF, // ty
    ];
    assert_eq!(frame.watch("move-target-words"), Some(&expect_moves[..]));

    // beacon-family: flag u32, timer u32, tile i32×3.
    let expect_beacon: [u8; 20] = [
        0x01, 0x00, 0x00, 0x00, // flag = armed
        0x03, 0x01, 0x00, 0x00, // window 0x0103
        0x1F, 0x00, 0x00, 0x00, // tile x 31
        0x2E, 0x00, 0x00, 0x00, // tile y 46
        0x03, 0x00, 0x00, 0x00, // tile z 3
    ];
    assert_eq!(frame.watch("beacon-family"), Some(&expect_beacon[..]));

    // spread-claims: u16 ×12 (claims 0 and 2 set).
    let mut expect_claims = Vec::new();
    expect_claims.extend_from_slice(&1u16.to_le_bytes());
    expect_claims.extend_from_slice(&0u16.to_le_bytes());
    expect_claims.extend_from_slice(&1u16.to_le_bytes());
    expect_claims.extend_from_slice(&[0u8; 18]);
    assert_eq!(frame.watch("spread-claims"), Some(&expect_claims[..]));

    // no-extract-latch (D136): u32 count (the robot-bank count) +
    // count all-zero u32 words — one robot, never claimed (SP).
    let expect_latch: [u8; 8] = [0x01, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(frame.watch("no-extract-latch"), Some(&expect_latch[..]));

    // the +0x18 byte family: u32 len + raw bytes (both rows, same bank).
    let expect_pads: [u8; 7] = [0x03, 0x00, 0x00, 0x00, 0xAB, 0xCD, 0xEF];
    assert_eq!(frame.watch("typedb-fade-byte"), Some(&expect_pads[..]));
    assert_eq!(frame.watch("armor-pad-reads"), Some(&expect_pads[..]));

    // --- TS row (anchor frame only) ---
    let expect_wh: [u8; 8] = [
        0x2C, 0x01, 0x00, 0x00, // w 300
        0x96, 0x00, 0x00, 0x00, // h 150
    ];
    assert_eq!(frame.watch("static-map-wh"), Some(&expect_wh[..]));

    // static-claim-bank (S0-11b, §7j.63): the RAW arena image — no
    // count prefix, no field map (the O1/O2 plans dump the same
    // fixed 10000-B span through the pointer cells; byte
    // passthrough on every channel).
    assert_eq!(frame.watch("static-claim-bank"), Some(&claims[..]));

    // The registry order is imposed by encode; here we just pin that
    // every emitted id exists as a registry row (stitch's job, but
    // fail early with a clear name).
    let reg = registry();
    for w in &frame.watches {
        assert!(
            reg.iter().any(|r| r.id == w.id),
            "emitted id {} is not a registry row",
            w.id
        );
    }

    // Pinned frame digest (canonicalized to registry order first).
    let mut canon = frame.clone();
    canonicalize_frame(&mut canon, &reg).expect("registry covers every emitted id");
    let digest = frame_digest(&canon).expect("digest computes");
    assert_eq!(
        digest.to_string(),
        "d2e92edc4f6e50c4",
        "grammar drift: re-derive the hand bytes above, then re-pin (deliberately)"
    );
}

// ---------------------------------------------------------------------
// 2. The synthetic MissionSim run (dump decode + pinned chain)
// ---------------------------------------------------------------------

fn synthetic_frames() -> Vec<diffharness::dump::FrameRecord> {
    let terrain = Terrain::from_parts(4, 4, vec![0u8; 8 * 4 * 4], Vec::new())
        .expect("4×4 flat terrain is well-formed");
    let angles = AngleTable::from_thresholds(&[0u16; 64]).expect("64 thresholds");
    let mut sim = MissionSim::new(terrain, angles, 0x1E240);
    let r0 = sim.spawn_robot((1, 1, 0));
    let _r1 = sim.spawn_robot((2, 2, 0));
    assert!(sim.arm_order_at_robot(r0));

    let tiers: Vec<String> = ["T0", "T1", "TS"].iter().map(|s| s.to_string()).collect();
    let mut frames = Vec::new();
    for i in 0..3u64 {
        sim.advance_frame();
        let st = TickState {
            frame_no: sim.frame() - 1,
            rand_a_state: sim.rand_a_state(),
            rand_b_state: 0xDEAD_BEEF_CAFE_F00D,
            score: 0,
            money: 4000,
            difficulty: 0,
            zone: 0,
            mission: 1,
            mode: 0,
            linear: 1,
            robots: sim.robots(),
            order: sim.order(),
            beacon_latch: sim.beacon_tile_latch(),
            claims_latch: sim.beacon_claims_latch(),
            dropship: None,
            selected: 0,
            blink_cursor: 0,
            order_target: (0, 0, 0),
            armor_pads: sim.armor_pads(),
            map_wh: (i == 0).then_some((4, 4)),
            claim_bank: if i == 0 { sim.claim_bank() } else { &[] },
            player_type: sim.player_type(),
            weapon_bank: sim.weapon_bank(),
            enemy_bank: sim.enemy_bank(),
            critter: None,
            destroy: None,
        };
        frames.push(emit_frame(&st, &tiers, false, i == 0));
    }
    frames
}

#[test]
fn synthetic_sim_dump_decodes_with_pinned_chain() {
    let src = "scenario = \"SX\"\ntiers = T0,T1,TS\nanchor = mission-start\nframes = 2\n";
    let scen = Scenario::parse(src).expect("synthetic scenario parses");
    let header = DumpHeader::new(Channel::Engine, sha256(b"synthetic"), "SX");
    let stitched = stitch(
        &scen,
        &Transcript {
            frames: synthetic_frames(),
        },
        &header,
        &registry(),
    )
    .expect("synthetic transcript stitches");

    // Frame-count contract: anchor + frames = 3.
    assert_eq!(stitched.manifest.frame_count, 3);
    // Determinism: identical inputs stitch to identical bytes.
    let again = stitch(
        &scen,
        &Transcript {
            frames: synthetic_frames(),
        },
        &header,
        &registry(),
    )
    .expect("second stitch");
    assert_eq!(
        stitched.bytes, again.bytes,
        "byte-deterministic by construction"
    );

    // Decode verifies every digest + the chain against the bytes.
    let dump = decode_dump(&stitched.bytes).expect("dump decodes + verifies");
    assert_eq!(dump.header.channel, Channel::Engine);
    assert_eq!(dump.header.scenario, "SX");
    assert_eq!(dump.trailer.frame_count, 3);
    assert_eq!(
        dump.trailer.chain.to_string(),
        stitched.manifest.chain_digest
    );
    // frame_no strictly increasing from the anchor (0, 1, 2).
    assert_eq!(
        dump.frames.iter().map(|f| f.frame_no).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    // The TS row rides the anchor only.
    assert!(dump.frames[0].watch("static-map-wh").is_some());
    assert!(dump.frames[1].watch("static-map-wh").is_none());
    // The armed order is live sim state, not fixture data: two alive
    // robots → the 0x197 window at arm time; the anchor emit runs
    // AFTER the first advance_frame, so the window shows 0x197−1.
    let beacon = dump.frames[0].watch("beacon-family").expect("T1 row");
    assert_eq!(&beacon[0..4], &1u32.to_le_bytes()[..]);
    assert_eq!(
        &beacon[4..8],
        &u32::from(ORDER_WINDOW - 1).to_le_bytes()[..]
    );

    assert_eq!(
        stitched.manifest.chain_digest, "b61d0647c3b65717",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
}

// ---------------------------------------------------------------------
// 3. Corpus-gated S0/S1 + the scenario seam gates
// ---------------------------------------------------------------------

#[test]
fn corpus_s0_s1_canonical_runs() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // S0: boot → mission start, 3 records (anchor + 2).
    let s0 = fs::read_to_string(scen_path("S0")).expect("S0.scen committed");
    let run0 = run_canonical(&s0, &root).expect("S0 canonical run");
    assert_eq!(run0.manifest.frame_count, 3);
    assert_eq!(run0.manifest.chain_digest, "c766cc682b73a32c");
    let run0b = run_canonical(&s0, &root).expect("S0 canonical re-run");
    assert_eq!(run0.bytes, run0b.bytes, "byte-identical double run");

    let dump = decode_dump(&run0.bytes).expect("S0 dump verifies");
    assert_eq!(dump.header.channel, Channel::Engine);
    assert_eq!(dump.header.scenario, "S0");
    assert_eq!(
        dump.frames.iter().map(|f| f.frame_no).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    // Fresh campaign: money 3500 (the §7j.64/C name-entry seed at the
    // boot-default difficulty 1 — S0-12b/D154; was 4000 at the
    // mis-modeled d=0 default), the ZONEA statics ride the anchor.
    assert_eq!(
        dump.frames[0].watch("money"),
        Some(&3500u32.to_le_bytes()[..])
    );
    assert!(dump.frames[0].watch("static-map-wh").is_some());
    assert!(dump.frames[1].watch("static-map-wh").is_none());
    // ZONEA/MISSION1 is 25×75 tiles (w·h = 1875 — the W1 cross-check:
    // TOT 4+16wh = 30004, DAT 4+8wh = 15004 file bytes).
    assert_eq!(
        dump.frames[0].watch("static-map-wh"),
        Some(&[25u32.to_le_bytes(), 75u32.to_le_bytes()].concat()[..])
    );

    // S1: mission-start passive, 401 records (anchor + 400).
    let s1 = fs::read_to_string(scen_path("S1")).expect("S1.scen committed");
    let run1 = run_canonical(&s1, &root).expect("S1 canonical run");
    assert_eq!(run1.manifest.frame_count, 401);
    assert_eq!(run1.manifest.chain_digest, "ed7deab5e3df5ba8");
    let run1b = run_canonical(&s1, &root).expect("S1 canonical re-run");
    assert_eq!(run1.bytes, run1b.bytes, "byte-identical double run");
}

/// One robot leaf from a canonical robot-bank blob (94 B records
/// after the u32 count — the §6a order).
struct RobotView<'a> {
    rec: &'a [u8],
}

impl RobotView<'_> {
    fn i32(&self, p: usize) -> i32 {
        i32::from_le_bytes(self.rec[p..p + 4].try_into().unwrap())
    }
    fn u16(&self, p: usize) -> u16 {
        u16::from_le_bytes(self.rec[p..p + 2].try_into().unwrap())
    }
    fn tile(&self) -> (i32, i32) {
        (self.i32(1) >> 13, self.i32(5) >> 13)
    }
    fn snapped(&self) -> bool {
        self.i32(1) & 0x1FFF == 0 && self.i32(5) & 0x1FFF == 0
    }
    fn state(&self) -> u16 {
        self.u16(13)
    }
    fn present(&self) -> u8 {
        self.rec[43]
    }
    /// The move-target in TILE units (Q5 >> 5) for the asserts.
    fn target_tile(&self) -> (i32, i32) {
        (self.i32(44) >> 5, self.i32(48) >> 5)
    }
}

fn robots_of(bank: &[u8]) -> Vec<RobotView<'_>> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 94, "canonical robot-bank shape");
    (0..n)
        .map(|i| RobotView {
            rec: &bank[4 + i * 94..4 + (i + 1) * 94],
        })
        .collect()
}

fn beacon_u32(beacon: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(beacon[p..p + 4].try_into().unwrap())
}

fn claims_set(claims: &[u8]) -> Vec<u16> {
    claims
        .chunks(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

/// Active (kind ≠ 0) weapon-bank records as (slot, kind, owner, tick,
/// class) — the §6a T2 row grammar (0x36 records after the u32 count,
/// byte layout = the guest record).
fn weapons_of(bank: &[u8]) -> Vec<(usize, u16, i32, i32, i32)> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 0x36, "weapon-anim-bank row shape");
    let mut out = Vec::new();
    for i in 0..n {
        let rec = &bank[4 + i * 0x36..4 + (i + 1) * 0x36];
        let kind = u16::from_le_bytes(rec[0..2].try_into().unwrap());
        if kind != 0 {
            let owner = i32::from_le_bytes(rec[2..6].try_into().unwrap());
            let tick = i32::from_le_bytes(rec[0xA..0xE].try_into().unwrap());
            let class = i32::from_le_bytes(rec[0x2A..0x2E].try_into().unwrap());
            out.push((i, kind, owner, tick, class));
        }
    }
    out
}

#[test]
fn corpus_s3_command_fire() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // S3 (DESIGN §7 S3 row + §10-W12; D103): the loadout key stages
    // robot 0 with one slot per inline-spawn class + the marker
    // robot with the rocket; 8 COMMAND records exercise the fire
    // gates, the cooldown cadences, the per-record ammo gate, the
    // auto-rearm cascade, and the spawn/active/free lifecycle of
    // every modeled record family. 133 records (anchor + 132),
    // pinned chain.
    let s3 = fs::read_to_string(scen_path("S3")).expect("S3.scen committed");
    let run = run_canonical(&s3, &root).expect("S3 canonical run");
    assert_eq!(run.manifest.frame_count, 133);
    // Re-pinned ONCE at the W12-S4-prep landing [D104, §7j.39/9]:
    // the S3 artillery volleys reach the burst window, and the
    // landed burst-pair application draws the shared stream (the
    // per-pair script-blast k6 1-in-8 gate + the k11 50% gate +
    // the stager's k11 SFX-gate draw) whether or not destructibles
    // are staged; the 0xF mine's class-0 expiry also no longer
    // frees the record (the raw-asm disburser no-op). The chain
    // moved from 49193732e6dbc546 BEFORE any O1 S3 capture exists
    // (the D103 dbx-plan T2-tier unit precedes any live S3).
    assert_eq!(
        run.manifest.chain_digest, "88e5d849cfb91c09",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s3, &root).expect("S3 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S3 dump verifies");

    // --- the T2 rows: FULL banks every frame ------------------------
    for f in &dump.frames {
        let w = f.watch("weapon-anim-bank").expect("T2 weapon row");
        assert_eq!(w.len(), 4 + 400 * 0x36, "the full 400-slot bank");
        let p = f.watch("projectile-bank").expect("T2 projectile row");
        assert_eq!(p.len(), 4 + 50 * 0x22, "the full 50-slot bank");
        // No enemy fire rides S3 (the 0x22 producers are the critter
        // family, an E-gap): the row stays the all-free zero blob.
        assert!(p[4..].iter().all(|&x| x == 0), "projectile bank free");
    }

    // --- anchor frame 0: inert banks, TS rides -----------------------
    let f0 = &dump.frames[0];
    assert!(weapons_of(f0.watch("weapon-anim-bank").unwrap()).is_empty());
    assert!(f0.watch("static-map-wh").is_some());

    // --- frame 1: the full volley (15 records) -----------------------
    let f1 = &dump.frames[1];
    assert!(f1.injection_applied);
    let v1 = weapons_of(f1.watch("weapon-anim-bank").unwrap());
    let mut kinds: Vec<u16> = v1.iter().map(|&(_, k, ..)| k).collect();
    kinds.sort_unstable();
    assert_eq!(
        kinds,
        vec![
            9, 0xA, 0xB, // artillery (record type = the slot id)
            0xF, 0xF, // prox mines 0x10 -> 2x type 0xF
            0x13, 0x13, // pressure mines 0x14 -> 2x type 0x13
            0x1A, 0x1A, 0x1A, 0x1A, // bouncy 0x1B -> 4x type 0x1A
            0x1F, 0x1F, 0x1F, 0x1F, // sticky 0x1D -> 4x type 0x1F
        ],
        "one record per inline-spawn class, first-free slot order"
    );
    assert!(v1.iter().all(|&(_, _, o, ..)| o == 0), "owner = robot 0");
    // The mines/grenades jittered their RandA draws: the rng row moved.
    assert_ne!(
        f0.watch("rng-state-a").unwrap(),
        f1.watch("rng-state-a").unwrap()
    );
    // The order-target row mirrors the COMMAND triple (raw Q5 words).
    assert_eq!(
        f1.watch("order-target").unwrap(),
        &[
            736i32.to_le_bytes(),
            2336i32.to_le_bytes(),
            0i32.to_le_bytes()
        ]
        .concat()[..]
    );

    // --- frame 10: volley 2 (12 records; the artillery slots stay
    //     disarmed — the unconditional one-shot per arm) --------------
    let f10 = &dump.frames[10];
    let v2 = weapons_of(f10.watch("weapon-anim-bank").unwrap());
    assert_eq!(v2.len(), 17, "5 survivors + the 12 volley-2 records");
    assert_eq!(
        v2.iter().filter(|&&(_, k, ..)| k == 9).count(),
        1,
        "artillery 9 still the volley-1 record (no refire)"
    );

    // --- frame 11: the rocket (owner 1 = the staged marker robot) ----
    let f11 = &dump.frames[11];
    let rockets: Vec<_> = weapons_of(f11.watch("weapon-anim-bank").unwrap())
        .into_iter()
        .filter(|&(_, k, ..)| k == 0x24)
        .collect();
    assert_eq!(rockets.len(), 1);
    assert_eq!(rockets[0].2, 1, "the rocket's owner is robot 1");

    // --- frames 20/28/36: the AUTO-REARM CASCADE — the volley-2 ammo
    //     spend emptied the mask, slot 0 re-armed, and each command
    //     fires exactly one artillery walking slots 9 -> 0xA -> 0xB ---
    for (fi, kind) in [(20usize, 9u16), (28, 0xA), (36, 0xB)] {
        let f = &dump.frames[fi];
        assert!(f.injection_applied);
        let fresh: Vec<_> = weapons_of(f.watch("weapon-anim-bank").unwrap())
            .into_iter()
            .filter(|&(_, k, _, t, _)| k == kind && t <= 4)
            .collect();
        assert_eq!(fresh.len(), 1, "frame {fi}: one fresh artillery {kind:#x}");
        assert_eq!(fresh[0].2, 0);
    }

    // --- frame 44: the ALL-EMPTY command — the mask is 0, nothing
    //     fires (the no-op arm path) -----------------------------------
    let before = weapons_of(dump.frames[43].watch("weapon-anim-bank").unwrap()).len();
    let f44 = weapons_of(dump.frames[44].watch("weapon-anim-bank").unwrap());
    assert!(dump.frames[44].injection_applied);
    assert_eq!(f44.len(), before, "the empty-mask command fires nothing");

    // --- frame 45: rocket 2 (ammo to 0; no rearm — the all-empty
    //     auto-rearm path) ---------------------------------------------
    let rockets: Vec<_> = weapons_of(dump.frames[45].watch("weapon-anim-bank").unwrap())
        .into_iter()
        .filter(|&(_, k, ..)| k == 0x24)
        .collect();
    assert_eq!(rockets.len(), 1);
    assert_eq!(rockets[0].2, 1);

    // --- frame 100: the class ladder — only mines remain (grenades +
    //     rockets + artillery freed; the mines' 4-cycle class
    //     decrement mid-ladder) -----------------------------------------
    // Re-derived at the S4-prep re-pin [D104]: the burst-pair RNG
    // draws shifted the later volleys' spawn jitter, so the frame-100
    // ladder now shows the volley-2 0xF mines at classes {0,0,1,1}
    // and the 0x13s at {1,1}.
    let v100 = weapons_of(dump.frames[100].watch("weapon-anim-bank").unwrap());
    assert!(v100.iter().all(|&(_, k, ..)| k == 0xF || k == 0x13));
    let mut classes: Vec<_> = v100.iter().map(|&(_, _, _, _, c)| c).collect();
    classes.sort_unstable();
    assert_eq!(classes, vec![0, 0, 1, 1, 1, 1], "class 4 -> {classes:?}");

    // --- the tail: the class-0 lifecycle split [§7j.39/3 — the raw
    //     disburser map]: every spawned record freed EXCEPT the 0xF
    //     mines, which persist past their class-0 quadrant (the 0xF
    //     disburser arm is an asm no-op; the mine-proximity family
    //     that eventually frees them is the documented E-gap — the
    //     S4+ scenario/differ coverage names it) ------------------------
    let flast = dump.frames.last().unwrap();
    let tail = weapons_of(flast.watch("weapon-anim-bank").unwrap());
    assert!(
        tail.iter().all(|&(_, k, ..)| k == 0xF),
        "only the persistent 0xF mines remain: {tail:?}"
    );
    assert_eq!(tail.len(), 4);
    // Two already cycled past class 0 (class −1, forever re-arming);
    // two sit between the 3rd and 4th expiry (class 0).
    let mut tail_classes: Vec<_> = tail.iter().map(|&(_, _, _, _, c)| c).collect();
    tail_classes.sort_unstable();
    assert_eq!(tail_classes, vec![-1, -1, 0, 0]);

    // --- the loadout seam gates ----------------------------------------
    // A robot index past the staged bank fails loud (never guessed).
    let bad = "scenario = \"SB\"\ntiers = T0\nframes = 1\nloadout = 5,0x01,9:2\nuntil-anchor mission-start\nstep 1\n";
    let err = run_canonical(bad, &root).unwrap_err();
    assert!(
        err.to_string().contains("not in the bank"),
        "loadout rejection names the bank bound: {err}"
    );
}

#[test]
fn corpus_s2_order_walk() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // S2 (DESIGN §7, the P4 slice; D91): the markers key stages the
    // mission_corpus_gate walker; `order 21 73 1` arms at the MRK
    // robot. 17 records (anchor + 16), pinned chain.
    let s2 = fs::read_to_string(scen_path("S2")).expect("S2.scen committed");
    let run = run_canonical(&s2, &root).expect("S2 canonical run");
    assert_eq!(run.manifest.frame_count, 17);
    assert_eq!(
        run.manifest.chain_digest, "dfb8e457003e36f6",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s2, &root).expect("S2 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S2 dump verifies");

    // --- anchor frame 0: two idle robots, no order -------------------
    let f0 = &dump.frames[0];
    let bank = f0.watch("robot-bank").expect("T1 robot bank");
    let rs = robots_of(bank);
    assert_eq!(rs.len(), 2, "MRK robot + the staged marker (D91)");
    assert_eq!(rs[0].tile(), (21, 73), "ZONEA/MISSION1 MRK record 0");
    assert_eq!(rs[1].tile(), (18, 73), "the staged walker marker");
    assert!([rs[0].state(), rs[1].state()].iter().all(|&s| s == 0));
    assert!([rs[0].present(), rs[1].present()].iter().all(|&p| p == 0));
    assert_eq!(beacon_u32(f0.watch("beacon-family").unwrap(), 0), 0);
    assert!(f0.watch("static-map-wh").is_some(), "TS rides the anchor");
    // The TS tier is why S2 carries it: the fabricated-O1 differ leg
    // needs the anchor statics for the len-0 grid equivalence.

    // --- frame 1 (the order step): arm + consume in one pump --------
    let f1 = &dump.frames[1];
    assert!(f1.injection_applied);
    // The SEAM write persists on the order-target row (the step's
    // z=1; the BEACON tile z is the robot's z=31 — different sources,
    // both pinned §5c/§6a).
    assert_eq!(
        f1.watch("order-target"),
        Some(&[21i32.to_le_bytes(), 73i32.to_le_bytes(), 1i32.to_le_bytes()].concat()[..])
    );
    let beacon = f1.watch("beacon-family").unwrap();
    assert_eq!(beacon_u32(beacon, 0), 1, "order armed");
    assert_eq!(
        beacon_u32(beacon, 4),
        u32::from(bedlam_core::mission::ORDER_WINDOW) - 1,
        "window 0x197 minus the arming pump's decrement (the W6 SO \
         gate's single-robot window-0 clear does NOT fire at 2 alive)"
    );
    assert_eq!(beacon_u32(beacon, 8), 21);
    assert_eq!(beacon_u32(beacon, 12), 73);
    assert_eq!(beacon_u32(beacon, 16), 31);
    // claims: slot 0 = the clicked robot's own tile, slot 1 = the
    // walker's (22,73) — claimed in the SAME pump's phases.
    assert_eq!(
        claims_set(f1.watch("spread-claims").unwrap()),
        vec![1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    let rs = robots_of(f1.watch("robot-bank").unwrap());
    assert_eq!(rs[0].state(), 3, "the clicked robot takes state 3");
    assert_eq!(rs[0].tile(), (21, 73));
    assert!(rs[0].snapped(), "the armer snaps slot 0 to the tile origin");
    assert_eq!(rs[0].present(), 0, "the clicked robot gets no target");
    assert_eq!(rs[1].state(), 4, "the walker consumed the order");
    assert_eq!(rs[1].present(), 1, "THE present=1 target window opens");
    assert_eq!(rs[1].target_tile(), (22, 73), "spread slot 1 = +1 x");
    assert_eq!(rs[1].i32(39), 1_000_000, "ORDER_STOP_DIST go-all-the-way");
    // The move-target-words row carries the same window (the E-side
    // form the D90 splice mirrors on O1).
    let mv = f1.watch("move-target-words").unwrap();
    assert_eq!(u32::from_le_bytes(mv[0..4].try_into().unwrap()), 2);
    assert_eq!(mv[4], 0, "robot 0 absent");
    assert_eq!(mv[4 + 9], 1, "robot 1 present");

    // --- the walk window (frames 1..6): state 4, present 1, monotone
    for f in &dump.frames[1..7] {
        let rs = robots_of(f.watch("robot-bank").unwrap());
        assert_eq!(rs[1].state(), 4, "walking");
        assert_eq!(rs[1].present(), 1);
        assert_eq!(rs[1].target_tile(), (22, 73));
        assert_eq!(beacon_u32(f.watch("beacon-family").unwrap(), 0), 1);
    }
    // eastbound tile crossings (0.75 tile/frame on the real deck)
    let tile_at = |f: &diffharness::dump::FrameRecord, i: usize| {
        robots_of(f.watch("robot-bank").unwrap())[i].tile()
    };
    assert_eq!(tile_at(&dump.frames[1], 1), (18, 73));
    assert_eq!(tile_at(&dump.frames[2], 1), (19, 73));
    assert_eq!(tile_at(&dump.frames[4], 1), (20, 73));
    assert_eq!(tile_at(&dump.frames[6], 1), (21, 73));

    // --- frame 7: the arrival clear ----------------------------------
    let f7 = &dump.frames[7];
    let rs = robots_of(f7.watch("robot-bank").unwrap());
    assert_eq!(rs[1].state(), 3, "state 4 -> 3 on arrival");
    assert_eq!(rs[1].tile(), (21, 73), "one tile SHORT of the slot target");
    assert!(rs[1].snapped(), "arrival snaps pos &= !0x1FFF");
    assert_eq!(rs[1].i32(9), 31, "stays on the real deck");
    // the walker KEEPS its target (state-4 arrival clears neither
    // target nor stop_dist) — present=1 persists to the last frame.
    assert_eq!(rs[1].present(), 1);
    assert_eq!(rs[1].target_tile(), (22, 73));
    // the ORDER clears once every alive robot is state-3: flag 0,
    // window 0, claims all 0 (the beacon-family/claims transition).
    let beacon = f7.watch("beacon-family").unwrap();
    assert_eq!(beacon_u32(beacon, 0), 0);
    assert_eq!(beacon_u32(beacon, 4), 0);
    assert_eq!(claims_set(f7.watch("spread-claims").unwrap()), vec![0; 12]);
    // ...and the steady state holds to the end.
    let flast = dump.frames.last().unwrap();
    let rs = robots_of(flast.watch("robot-bank").unwrap());
    assert!([rs[0].state(), rs[1].state()].iter().all(|&s| s == 3));
    assert_eq!(beacon_u32(flast.watch("beacon-family").unwrap(), 0), 0);
}

#[test]
fn canonical_seam_gates() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();

    // Walk-phase non-boot steps name the missing P2e seam. (This
    // fires before staging, but stays in the gated test.)
    let walk = "scenario = \"SW\"\ntiers = T0\nframes = 1\nkeystore 0x1f=1\nuntil-anchor mission-start\nstep 1\n";
    let err = run_canonical(walk, &root).unwrap_err();
    assert!(
        err.to_string().contains("P2e"),
        "walk rejection names the seam: {err}"
    );

    // BOOT difficulty is the one consumable walk step: money seed via
    // the engine's own formula (4000 − 500·2 = 3000) + the pin.
    let sd = "scenario = \"SD\"\ntiers = T0,TS\nframes = 2\nboot difficulty=2\nuntil-anchor mission-start\nstep 2\n";
    let run = run_canonical(sd, &root).expect("boot difficulty consumed");
    assert_eq!(run.manifest.frame_count, 3);
    assert!(run.manifest.pins.iter().any(|p| p == "difficulty=2"));
    let dump = decode_dump(&run.bytes).expect("SD dump verifies");
    assert_eq!(
        dump.frames[0].watch("money"),
        Some(&3000u32.to_le_bytes()[..])
    );
    assert_eq!(
        dump.frames[0].watch("difficulty"),
        Some(&2u32.to_le_bytes()[..])
    );

    // COMMAND is CONSUMED by the W12-S3-prep fire seam (§7j.37): a
    // well-formed 14-B record (marker 01, id 0, flags 2 = the ORDER
    // arm, target words 0) stages into the sim's ring — with NO
    // staged weapon slots nothing fires (the no-inject invariant)
    // and the run completes.
    let cmd =
        "scenario = \"SC\"\ntiers = T0\nframes = 1\nuntil-anchor mission-start\ncommand 01 00 00 00 00 02 00 00 00 00 00 00 00 00\n";
    let stitched = run_canonical(cmd, &root);
    assert!(
        stitched.is_ok(),
        "command record consumed: {:?}",
        stitched.err()
    );
    // A short payload fails loud naming the record grammar.
    let short =
        "scenario = \"SH\"\ntiers = T0\nframes = 1\nuntil-anchor mission-start\ncommand 01\n";
    let err = run_canonical(short, &root).unwrap_err();
    assert!(
        err.to_string().contains("14 B"),
        "short command payload names the record grammar: {err}"
    );

    // PAD is CONSUMED by the W12-S6 extraction seam (§7j.40): the
    // target tile is read from the staged .PAD slot bank, the
    // order-target seam records it, and the run completes. A slot
    // outside the mission's live record run fails LOUD naming the
    // slot (the D86 capgen contract).
    let pad = "scenario = \"SP\"\ntiers = T0,T1\nframes = 1\nuntil-anchor mission-start\npad 0\n";
    let stitched = run_canonical(pad, &root);
    assert!(stitched.is_ok(), "pad step consumed: {:?}", stitched.err());
    {
        let dump = decode_dump(&stitched.unwrap().bytes).expect("SP dump verifies");
        // ZONEA/MISSION1 slot 0 = (5, 61, 0): the triple rides the
        // anchor+1 frame's order-target row (the seam write).
        assert_eq!(
            dump.frames[1].watch("order-target"),
            Some(
                &[5i32, 61, 0]
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<u8>>()[..]
            )
        );
    }
    let bad = "scenario = \"SB\"\ntiers = T0\nframes = 1\nuntil-anchor mission-start\npad 900\n";
    let err = run_canonical(bad, &root).unwrap_err();
    assert!(
        err.to_string().contains("slot 900"),
        "pad rejection names the missing slot: {err}"
    );

    // P-pause (scan 0x19) is banned mid-scenario (DESIGN §2).
    let pp =
        "scenario = \"SK\"\ntiers = T0\nframes = 1\nuntil-anchor mission-start\nkeystore 0x19=1\n";
    let err = run_canonical(pp, &root).unwrap_err();
    assert!(
        err.to_string().contains("0x19"),
        "P-pause rejection names the scan: {err}"
    );

    // ORDER: the click-order seam — target recorded (order-target
    // row) + the armer at the tile-exact alive robot (ZONEA/MISSION1
    // robot 0 spawns at tile (21, 73), mission_scene_gate's click
    // twin). NOTE the canonical runner stages NO network marker
    // override (the host default), so ZONEA carries a SINGLE-robot
    // squad: the armer's window-0 single-robot special case then
    // clears the order on the SAME pump's window tick (the ledger
    // behavior — 0x4eabb2 = 0 when one alive, cleared next tick), so
    // the beacon flag reads 0 in the dump; the ARM itself is proven
    // by robot 0's state-3 + tile-origin snap in the robot bank.
    // (The surviving-order case is the synthetic two-robot fixture
    // above.)
    let so = "scenario = \"SO\"\ntiers = T0,T1\nframes = 2\nuntil-anchor mission-start\norder 21 73 1\nstep 1\n";
    let run = run_canonical(so, &root).expect("order seam runs");
    let dump = decode_dump(&run.bytes).expect("SO dump verifies");
    assert_eq!(dump.frames.len(), 3);
    let injected = &dump.frames[1];
    assert!(injected.injection_applied);
    assert_eq!(
        injected.watch("order-target"),
        Some(&[21i32.to_le_bytes(), 73i32.to_le_bytes(), 1i32.to_le_bytes()].concat()[..])
    );
    let bank = injected.watch("robot-bank").expect("robot bank row");
    // u32 count, alive u8, pos_x i32, pos_y i32, z i32, then state u16.
    let count = u32::from_le_bytes(bank[0..4].try_into().unwrap());
    let pos_x = i32::from_le_bytes(bank[5..9].try_into().unwrap());
    let state = u16::from_le_bytes(bank[17..19].try_into().unwrap());
    assert_eq!(count, 1, "no network override markers staged");
    assert_eq!(state, 3, "the order ARMED robot 0 (state 3)");
    assert_eq!(pos_x, 21 << 13, "snap to the tile origin");
    let beacon = injected.watch("beacon-family").expect("beacon row");
    assert_eq!(
        &beacon[0..4],
        &0u32.to_le_bytes()[..],
        "single-robot window-0 order clears on the arming pump's tick"
    );
}

// ---------------------------------------------------------------------
// S4 — the destroy family (W12-S4, DESIGN §7 S4 row)
// ---------------------------------------------------------------------

/// One live object instance from the canonical object-instances
/// blob (23-B records after the u32 count — the §6a destroy rows).
struct ObjView {
    slot: u16,
    x: i32,
    y: i32,
    id: i32,
    destroyed: bool,
    hp: i32,
}

fn objects_of(bank: &[u8]) -> Vec<ObjView> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 23, "object-instances row shape");
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &bank[4 + i * 23..4 + (i + 1) * 23];
        out.push(ObjView {
            slot: u16::from_le_bytes(rec[0..2].try_into().unwrap()),
            x: i32::from_le_bytes(rec[2..6].try_into().unwrap()),
            y: i32::from_le_bytes(rec[6..10].try_into().unwrap()),
            id: i32::from_le_bytes(rec[14..18].try_into().unwrap()),
            destroyed: rec[18] & 0x40 != 0,
            hp: i32::from_le_bytes(rec[19..23].try_into().unwrap()),
        });
    }
    out
}

/// The trt-array row: u32 count + {active, hp, x, y, z} i32 records.
fn trt_of(bank: &[u8]) -> Vec<(i32, i32, i32, i32, i32)> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 20, "trt-array row shape");
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let rec = &bank[4 + i * 20..4 + (i + 1) * 20];
        let rd = |p: usize| i32::from_le_bytes(rec[p..p + 4].try_into().unwrap());
        out.push((rd(0), rd(4), rd(8), rd(12), rd(16)));
    }
    out
}

/// Active debris records as (slot, kind, delay, seq) — the 42-B
/// §6a records after the u32 count (active u8 @0, x/y/z i32 @1/5/9,
/// init_a @13, init_b @17, seq @21, kind @25, phys @29, delay @33,
/// param @37, table u8 @41).
fn debris_of(bank: &[u8]) -> Vec<(usize, i32, i32, i32)> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 42, "debris-stager row shape");
    let mut out = Vec::new();
    for i in 0..n {
        let rec = &bank[4 + i * 42..4 + (i + 1) * 42];
        if rec[0] == 0 {
            continue;
        }
        let rd = |p: usize| i32::from_le_bytes(rec[p..p + 4].try_into().unwrap());
        out.push((i, rd(25), rd(33), rd(21)));
    }
    out
}

/// Active splash records (age ≠ 0) as (x, y, z, delay).
fn splashes_of(bank: &[u8]) -> Vec<(i16, i16, i16, u16)> {
    let n = u32::from_le_bytes(bank[0..4].try_into().unwrap()) as usize;
    assert_eq!(bank.len(), 4 + n * 10, "splash-records row shape");
    let mut out = Vec::new();
    for i in 0..n {
        let rec = &bank[4 + i * 10..4 + (i + 1) * 10];
        let age = u16::from_le_bytes(rec[8..10].try_into().unwrap());
        if age == 0 {
            continue;
        }
        out.push((
            i16::from_le_bytes(rec[0..2].try_into().unwrap()),
            i16::from_le_bytes(rec[2..4].try_into().unwrap()),
            i16::from_le_bytes(rec[4..6].try_into().unwrap()),
            u16::from_le_bytes(rec[6..8].try_into().unwrap()),
        ));
    }
    out
}

#[test]
fn corpus_s4_destroy_family() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S4 (DESIGN §7 S4 row + §10-W12; the W12-S4 unit): the
    // `destroy = 1` staging key (grammar v1.4) stages the mission's
    // own .BDG/.POS/.TRT (211 live instances, 3 turrets), the
    // `markers` key stages the trap gunner + the artillery gunner,
    // and the loadout arms the grenade + artillery legs. 49
    // records (anchor + 48), pinned chain.
    let s4 = fs::read_to_string(scen_path("S4")).expect("S4.scen committed");
    let run = run_canonical(&s4, &root).expect("S4 canonical run");
    assert_eq!(run.manifest.frame_count, 49);
    assert_eq!(
        run.manifest.chain_digest, "1357af61ef082cb5",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s4, &root).expect("S4 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S4 dump verifies");

    // --- the row shapes: fixed-extent grids, full banks ---------------
    for f in &dump.frames {
        let objs = f.watch("object-instances").expect("T1 object row");
        assert_eq!(
            objs.len(),
            4 + 211 * 23,
            "211 live ZONEA/M1 instances every frame"
        );
        assert_eq!(
            f.watch("tile-word-grid").unwrap().len(),
            25 * 75 * 2,
            "the 25x75 grid span"
        );
        assert_eq!(f.watch("platform-strength").unwrap().len(), 25 * 75 * 2);
        assert!(f
            .watch("platform-strength")
            .unwrap()
            .iter()
            .all(|&b| b == 0));
        assert_eq!(f.watch("trt-array").unwrap().len(), 4 + 3 * 20);
        assert_eq!(f.watch("debris-stager").unwrap().len(), 4 + 128 * 42);
        assert_eq!(f.watch("splash-records").unwrap().len(), 4 + 250 * 10);
        // No T2 tier rides S4 (the weapon banks are S3's surface).
        assert!(f.watch("weapon-anim-bank").is_none());
    }

    // --- the anchor frame: the TRAP leg -------------------------------
    // The staged marker robot 1 stands ON the tile-0x62 trap cell
    // (14,16): the phase-1 armor pass runs the trap lane at the
    // FIRST active tick — resolver 100 (NO score flag) destroys the
    // hp-0 id-21 object (.POS slot 78), staging the five k12 trap
    // debris + the sel-9 k20 + the 3x3 splash ring, and the destroy
    // tail's RESTORE writes the id-21 under-TOT word into the
    // (empty-staged) mirror bank.
    let f0 = &dump.frames[0];
    let objs0 = objects_of(f0.watch("object-instances").unwrap());
    assert_eq!(objs0.len(), 211);
    let trap = &objs0[78];
    assert_eq!((trap.x, trap.y, trap.id), (14, 16, 21));
    assert!(trap.destroyed, "the trap fires at the anchor frame");
    assert_eq!(trap.hp, 0);
    assert_eq!(
        u32::from_le_bytes(f0.watch("score").unwrap()[..4].try_into().unwrap()),
        0,
        "the trap resolver passes NO score flag"
    );
    let d0 = debris_of(f0.watch("debris-stager").unwrap());
    assert_eq!(d0.len(), 6, "5x k12 + the k20");
    assert_eq!(d0.iter().filter(|&(_, k, ..)| *k == 12).count(), 5);
    assert_eq!(d0.iter().filter(|&(_, k, ..)| *k == 20).count(), 1);
    // The 3x3 splash ring: all nine (x,y) around (12..14, 14..16).
    let sp0 = splashes_of(f0.watch("splash-records").unwrap());
    assert_eq!(sp0.len(), 9);
    let mut tiles: Vec<(i16, i16)> = sp0.iter().map(|&(x, y, ..)| (x, y)).collect();
    tiles.sort_unstable();
    let expect: Vec<(i16, i16)> = {
        let mut v = Vec::new();
        for y in 14..17i16 {
            for x in 12..15i16 {
                v.push((x, y));
            }
        }
        v.sort_unstable();
        v
    };
    assert_eq!(tiles, expect, "the sel-9 3x3 ring at (12..15, 14..17)");
    // The restore: the mirror row at tile 414 = (14,16) carries the
    // id-21 under-TOT word 0x316 at z0 (seen 0 — the under-DAT
    // volume is nonzero at that cell).
    let m0 = f0.watch("typedb-mirror-rows").unwrap();
    assert_eq!(u32::from_le_bytes(m0[0..4].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(m0[4..6].try_into().unwrap()), 414);
    assert_eq!(u16::from_le_bytes(m0[6..8].try_into().unwrap()), 0x316);

    // --- frames 15..26: the SURVIVOR leg (pure multi-hit subtract) ----
    // Robot 1's two grenade volleys land hits of 75 on the slot-2
    // id-18 structure (hp 1800). Debris-physics re-baseline (D115,
    // §7j.44): the trap-leg debris knocks robot 1 px-level before
    // the volleys, so 3 of the old 5 blast boxes now reach the
    // footprint — 1800 -> 1575, monotone, NEVER destroyed, no
    // restore on its tiles, no score, and its grid word (3) stays
    // stamped.
    let mut last = 1800;
    for f in dump.frames.iter().take(27) {
        let hp = objects_of(f.watch("object-instances").unwrap())
            .into_iter()
            .find(|o| o.slot == 2)
            .unwrap()
            .hp;
        assert!(hp <= last && hp > 0, "monotone subtract, never destroyed");
        assert_eq!(hp % 75, 0, "every hit is exactly the 0x1A damage 75");
        last = hp;
    }
    assert_eq!(last, 1575, "1800 - 3*75 (the knock-shifted volleys)");
    let objs48 = objects_of(dump.frames[48].watch("object-instances").unwrap());
    let surv = objs48.iter().find(|o| o.slot == 2).unwrap();
    assert!(!surv.destroyed);
    let g48 = dump.frames[48].watch("tile-word-grid").unwrap();
    assert_eq!(
        u16::from_le_bytes(
            g48[(18 * 25 + 21) * 2..(18 * 25 + 21) * 2 + 2]
                .try_into()
                .unwrap()
        ),
        3,
        "the survivor's footprint word stays stamped (idx 2 + 1)"
    );

    // --- frame 32: the ARTILLERY ring-0 TURRET destroy ----------------
    // Robot 2's barrage bursts at its own tile (10,34): pair-ring 0
    // (the full 3x3 incl. the center) script-blasts the .TRT turret
    // (10,33) at 5000: active 0, hp -4741 (the 5000 blast against the
    // m=1 tier hp 259 = 250+250/27 — the §7j.64/D derived cell the
    // S0-12b seam pins; was -4750 at the linear-0 tier), the RUBBLE
    // stamp (zone-1 word 0x20 + seen 1) in the mirror row. The blast
    // box also spends 12 x 312 of robot 2's own hp (5000 -> 1256) —
    // the faithful self-damage of a point-blank barrage.
    let f32 = &dump.frames[32];
    let trt32 = trt_of(f32.watch("trt-array").unwrap());
    assert_eq!(trt32[2], (0, -4741, 10, 33, 1), "the turret dies at ring 0");
    assert_eq!(trt32[0], (1, 259, 14, 15, 1), "the other turrets hold");
    let m32 = f32.watch("typedb-mirror-rows").unwrap();
    let m32_tiles: Vec<u16> = (0..u32::from_le_bytes(m32[0..4].try_into().unwrap()) as usize)
        .map(|i| u16::from_le_bytes(m32[4 + i * 26..6 + i * 26].try_into().unwrap()))
        .collect();
    assert!(m32_tiles.contains(&835), "the rubble tile (10,33)");
    let rubble = &m32[4 + m32_tiles.iter().position(|&t| t == 835).unwrap() * 26..];
    assert_eq!(
        u16::from_le_bytes(rubble[2 + 3..4 + 3].try_into().unwrap()),
        0x20,
        "the zone-1 rubble word at z1"
    );
    assert_eq!(rubble[4 + 3], 1, "seen := 1");
    let rb32 = robots_of(f32.watch("robot-bank").unwrap());
    assert_eq!(
        rb32[2].i32(56),
        1254,
        "12 blast hits + one mag-2 debris chip (D115)"
    );

    // --- frames 35..38: the CHAIN-WALK cascade ------------------------
    // Pair-ring 4 script-blasts (14,29) at frame 35: the perimeter
    // walks cascade the chainable cluster in ONE frame — the
    // debris-physics re-baseline (D115, §7j.44) knock-shifted
    // robot 1's volleys, so the burst boxes also reach slots
    // 79/89/90 (ELEVEN same-frame destroys) — then the ring-6
    // frame destroys (16,40) and (9,27) at 38. Tail: 15 destroyed,
    // score 605, the splash bank SATURATED (250 max-age
    // eviction exercised) and the debris ring at 60 live records
    // — the tick now FREES finished chunks (the old never-free
    // saturation at 128 is gone; the lifecycle is the §7j.44
    // observable).
    let f35 = &dump.frames[35];
    let objs35 = objects_of(f35.watch("object-instances").unwrap());
    let destroyed35: Vec<u16> = objs35
        .iter()
        .filter(|o| o.destroyed)
        .map(|o| o.slot)
        .collect();
    assert_eq!(
        destroyed35,
        vec![78, 79, 89, 90, 97, 98, 100, 101, 102, 103, 207],
        "the trap + the ten-object same-frame cascade"
    );
    let objs_t = objects_of(dump.frames[48].watch("object-instances").unwrap());
    let destroyed_t: Vec<u16> = objs_t
        .iter()
        .filter(|o| o.destroyed)
        .map(|o| o.slot)
        .collect();
    assert_eq!(
        destroyed_t,
        vec![78, 79, 89, 90, 97, 98, 99, 100, 101, 102, 103, 106, 171, 172, 207],
        "15 destroyed at the tail (the trap + the widened cascade + rings 5/6)"
    );
    assert_eq!(
        u32::from_le_bytes(
            dump.frames[48].watch("score").unwrap()[..4]
                .try_into()
                .unwrap()
        ),
        605,
        "the destroy awards folded into the campaign score"
    );
    assert_eq!(
        debris_of(dump.frames[48].watch("debris-stager").unwrap()).len(),
        60,
        "the tick frees finished chunks (the §7j.44 lifecycle)"
    );
    assert_eq!(
        splashes_of(dump.frames[48].watch("splash-records").unwrap()).len(),
        250
    );
    // The restores accumulated in the mirror row (20 changed tiles:
    // the trap + the rubble + the widened cascade footprints).
    assert_eq!(
        u32::from_le_bytes(
            dump.frames[48].watch("typedb-mirror-rows").unwrap()[0..4]
                .try_into()
                .unwrap()
        ),
        20
    );

    // --- the staging seam gates ---------------------------------------
    // A destroy key with a wrong value fails loud at the grammar
    // (the runner test pins it); the canonical layer fails loud on
    // a malformed corpus file — pinned by the destroy_gate parser
    // rows. Here: the destroy-less S0 shape carries NO destroy rows
    // (the S0/S1/S2/S3 pinned chains in this file are the
    // no-inject invariant; S3 re-pinned f4f5b4351e976ed5 by the debris-physics unit, D115;
    // every pin re-baselined once more by the S0-11b claim-bank TS row).
    let s0 = fs::read_to_string(scen_path("S0")).unwrap();
    let run0 = run_canonical(&s0, &root).unwrap();
    let dump0 = decode_dump(&run0.bytes).unwrap();
    assert!(dump0.frames[0].watch("object-instances").is_none());
    assert_eq!(run0.manifest.chain_digest, "c766cc682b73a32c");
}

/// One typedb-mirror-rows cell (word, seen) at (tile, z) — the
/// compact-active §6a form `{tile u16, 8×(word u16, seen u8)}`.
fn mirror_cell(blob: &[u8], tile: usize, z: usize) -> (u16, u8) {
    let n = u32::from_le_bytes(blob[0..4].try_into().unwrap()) as usize;
    for i in 0..n {
        let rec = &blob[4 + i * 26..];
        if u16::from_le_bytes(rec[0..2].try_into().unwrap()) as usize == tile {
            let off = 2 + z * 3;
            return (
                u16::from_le_bytes(rec[off..off + 2].try_into().unwrap()),
                rec[off + 2],
            );
        }
    }
    panic!("tile {tile} not in the compact mirror row");
}

#[test]
fn corpus_s5_pickup_cases_1_2_4() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S5 (DESIGN §7 S5 row + §10-W12, D108 — the W12-S5 unit): the
    // grammar v1.5 keys stage ZONEB/MISSION1 (`zone = "B"` → the
    // episode-slot host seam, set-2 pickup surface + destroy family
    // + the hazard stamper) and the row-21 z3 walk fires cases
    // 1/2/4 — the only spot in the shipped corpus where cases 1 and
    // 2 co-occur walkably. 16 records (anchor + 15), pinned chain.
    let s5 = fs::read_to_string(scen_path("S5")).expect("S5.scen committed");
    let run = run_canonical(&s5, &root).expect("S5 canonical run");
    assert_eq!(run.manifest.frame_count, 16);
    assert_eq!(
        run.manifest.chain_digest, "0e844d916452ee78",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s5, &root).expect("S5 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S5 dump verifies");

    // --- the staged-session rows --------------------------------------
    // The zone row = the staged ZONEB slot INDEX (1; §6a zone
    // convention, D108); the anchor statics are the 100x100 map;
    // the mirror rows carry the REAL staged surface (every tile
    // active — the S4 empty-mirror divergence closes here).
    for f in &dump.frames {
        assert_eq!(
            u32::from_le_bytes(f.watch("zone").unwrap()[..4].try_into().unwrap()),
            1,
            "the staged ZONEB episode slot (0-based index)"
        );
        assert_eq!(
            u32::from_le_bytes(
                f.watch("typedb-mirror-rows").unwrap()[0..4]
                    .try_into()
                    .unwrap()
            ),
            10_000,
            "every ZONEB tile is active in the compact mirror row"
        );
        assert_eq!(f.watch("tile-word-grid").unwrap().len(), 100 * 100 * 2);
        assert_eq!(
            f.watch("object-instances").unwrap().len(),
            4 + 1096 * 23,
            "1096 live ZONEB/M1 instances every frame"
        );
        assert_eq!(
            f.watch("trt-array").unwrap().len(),
            4 + 19 * 20,
            "19 turrets (the m=1 tier hp 259 = 250+250/27 — the S0-12b derived-cell seam)"
        );
    }
    let wh = dump.frames[0].watch("static-map-wh").unwrap();
    assert_eq!(
        u32::from_le_bytes(wh[..4].try_into().unwrap()),
        100,
        "the ZONEB map width"
    );
    assert_eq!(
        u32::from_le_bytes(wh[4..8].try_into().unwrap()),
        100,
        "the ZONEB map height"
    );

    // --- the consume protocol: cases 1, 2, 4 ---------------------------
    // Anchor: the corridor cells stage their REAL TOT words (seen 0
    // — the swept DAT volume is nonzero under them). f1: case 1 at
    // (26,21) — word := table-C floor 0x48F, seen := 1, DAT := 0,
    // drop_countdown := 1000 (the reinforcement arm). f2: case 2 at
    // (27,21) — shield := 1000 (then the 2/frame phase-0 decay).
    // f4: case 4 at (28,21) — the award draw folds into the score
    // (this seed draws the score row, 1000). f5: the arrival —
    // state 4→3 snapped at the (28,21) origin, target retained.
    let m = |f: usize, tile: usize| {
        mirror_cell(dump.frames[f].watch("typedb-mirror-rows").unwrap(), tile, 3)
    };
    let t26 = 21 * 100 + 26;
    let t27 = 21 * 100 + 27;
    let t28 = 21 * 100 + 28;
    assert_eq!(m(0, t26), (0x76, 0), "the c1 cell stages its TOT word");
    assert_eq!(m(0, t27), (0x7e, 0));
    assert_eq!(m(0, t28), (0x82, 0));
    let score = |f: usize| {
        u32::from_le_bytes(
            dump.frames[f].watch("score").unwrap()[..4]
                .try_into()
                .unwrap(),
        )
    };
    let money = |f: usize| {
        u32::from_le_bytes(
            dump.frames[f].watch("money").unwrap()[..4]
                .try_into()
                .unwrap(),
        )
    };
    let walker_i32 =
        |f: usize, p: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].i32(p);
    let walker_state = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].state();
    let walker_tile = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].tile();
    let walker_snapped =
        |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].snapped();
    let walker_present =
        |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].present();
    let walker_target =
        |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].target_tile();
    // f1: the claim + the case-1 fire.
    assert_eq!(m(1, t26), (0x48F, 1), "case 1 consumed at frame 1");
    assert_eq!(walker_i32(1, 52), 1000, "case 1 arms the drop");
    assert_eq!(walker_state(1), 4, "the walker claims the order");
    assert_eq!(walker_present(1), 1);
    assert_eq!(walker_target(1), (29, 21), "spread slot 1");
    // f2: the case-2 fire.
    assert_eq!(m(2, t27), (0x48F, 1), "case 2 consumed at frame 2");
    assert_eq!(walker_i32(2, 68), 1000, "case 2 fills the shield");
    assert_eq!(walker_i32(3, 68), 998, "the 2/frame phase-0 decay");
    // f4: the case-4 fire + the award fold.
    assert_eq!(m(4, t28), (0x48F, 1), "case 4 consumed at frame 4");
    assert_eq!(score(3), 0);
    assert_eq!(score(4), 1000, "the case-4 award folds into the score");
    assert_eq!(money(15), 3500, "no money draw on this seed");
    // f5: the arrival.
    assert_eq!(walker_state(5), 3, "state 4→3 at the arrival");
    assert_eq!(walker_tile(5), (28, 21));
    assert!(walker_snapped(5), "snapped at the tile origin");
    assert_eq!(walker_present(5), 1, "the target is retained");
    // EXACTLY the three corridor cells consume: the full mirror
    // (10,000 tiles × 8 planes) changes at precisely those cells —
    // no other cell on the walker's path decodes under set 2.
    let cells_of = |blob: &[u8]| -> Vec<u16> {
        let n = u32::from_le_bytes(blob[0..4].try_into().unwrap()) as usize;
        (0..n)
            .map(|i| u16::from_le_bytes(blob[4 + i * 26..6 + i * 26].try_into().unwrap()))
            .collect()
    };
    let full_cells = |blob: &[u8]| -> std::collections::BTreeMap<(u16, u8), (u16, u8)> {
        let n = u32::from_le_bytes(blob[0..4].try_into().unwrap()) as usize;
        let mut out = std::collections::BTreeMap::new();
        for i in 0..n {
            let rec = &blob[4 + i * 26..];
            let t = u16::from_le_bytes(rec[0..2].try_into().unwrap());
            for z in 0..8u8 {
                let off = 2 + z as usize * 3;
                out.insert(
                    (t, z),
                    (
                        u16::from_le_bytes(rec[off..off + 2].try_into().unwrap()),
                        rec[off + 2],
                    ),
                );
            }
        }
        out
    };
    let tiles_a = cells_of(dump.frames[0].watch("typedb-mirror-rows").unwrap());
    assert_eq!(tiles_a.len(), 10_000);
    let a = full_cells(dump.frames[0].watch("typedb-mirror-rows").unwrap());
    let b = full_cells(dump.frames[15].watch("typedb-mirror-rows").unwrap());
    let diff: Vec<((u16, u8), (u16, u8))> = a
        .iter()
        .filter(|(k, v)| b.get(*k) != Some(*v))
        .map(|(k, _)| (*k, b.get(k).copied().unwrap_or((0, 0))))
        .collect();
    assert_eq!(
        diff,
        vec![
            (((t26 as u16), 3u8), (0x48F, 1)),
            ((t27 as u16, 3), (0x48F, 1)),
            ((t28 as u16, 3), (0x48F, 1)),
        ],
        "the whole-map consume census: exactly the corridor trio"
    );
}

#[test]
fn corpus_s5b_pickup_case_3() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S5B (DESIGN §7 S5 row, D108 — the case-3 half of the
    // two-scenario split): the row-10 z3 walk consumes five corridor
    // cells + the (76,9) side cell the diagonal probe reach collects
    // (6 total: 5× case 4 + case 3). Case 3's hp body is
    // value-invisible here — the walker spawns AT the 5000 clamp
    // (the D108 observability note) — the consume + dispatch still
    // ride the rows. 19 records (anchor + 18), pinned chain.
    let s5b = fs::read_to_string(scen_path("S5B")).expect("S5B.scen committed");
    let run = run_canonical(&s5b, &root).expect("S5B canonical run");
    assert_eq!(run.manifest.frame_count, 19);
    assert_eq!(
        run.manifest.chain_digest, "288f9d39f602bd82",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s5b, &root).expect("S5B canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S5B dump verifies");

    let m = |f: usize, tile: usize| {
        mirror_cell(dump.frames[f].watch("typedb-mirror-rows").unwrap(), tile, 3)
    };
    let score = |f: usize| {
        u32::from_le_bytes(
            dump.frames[f].watch("score").unwrap()[..4]
                .try_into()
                .unwrap(),
        )
    };
    let t74 = 10 * 100 + 74;
    let t75 = 10 * 100 + 75;
    let t76 = 10 * 100 + 76;
    let t77 = 10 * 100 + 77;
    let t78 = 10 * 100 + 78;
    let t769 = 9 * 100 + 76;
    // The consume schedule (word := 0x48F + seen := 1 at each).
    assert_eq!(m(0, t76), (0x7b, 0), "the case-3 word stages");
    assert_eq!(m(1, t74), (0x48F, 1), "case 4 at frame 1");
    assert_eq!(m(3, t75), (0x48F, 1), "case 4 at frame 3");
    assert_eq!(m(5, t769), (0x48F, 1), "the (76,9) side cell at ~f5");
    assert_eq!(m(6, t76), (0x48F, 1), "CASE 3 at frame 6");
    assert_eq!(m(8, t77), (0x48F, 1), "case 4 at frame 8");
    assert_eq!(m(10, t78), (0x48F, 1), "case 4 at frame 10 (diagonal)");
    // The case-4 award folds: the draws on this seed are all score
    // (1000/1000/5000/10000/10000 — RNG-pinned by the chain).
    assert_eq!(score(0), 0);
    assert_eq!(score(1), 1000);
    assert_eq!(score(3), 2000);
    assert_eq!(score(5), 7000);
    assert_eq!(score(8), 17000);
    assert_eq!(score(10), 27000);
    // The case-3 body: hp stays at the 5000 clamp (value-invisible;
    // the consume census below carries the dispatch proof).
    let hp = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].i32(56);
    assert_eq!((hp(5), hp(7)), (5000, 5000));
    // The arrival at frame 12: state 4→3 snapped at (78,9).
    let state = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].state();
    let tile = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap())[2].tile();
    assert_eq!(state(11), 4);
    assert_eq!(state(12), 3, "the arrival clear at frame 12");
    assert_eq!(tile(12), (78, 9));
    // The whole-map consume census: exactly the six cells.
    let full = |blob: &[u8]| -> std::collections::BTreeMap<(u16, u8), (u16, u8)> {
        let n = u32::from_le_bytes(blob[0..4].try_into().unwrap()) as usize;
        let mut out = std::collections::BTreeMap::new();
        for i in 0..n {
            let rec = &blob[4 + i * 26..];
            let t = u16::from_le_bytes(rec[0..2].try_into().unwrap());
            for z in 0..8u8 {
                let off = 2 + z as usize * 3;
                out.insert(
                    (t, z),
                    (
                        u16::from_le_bytes(rec[off..off + 2].try_into().unwrap()),
                        rec[off + 2],
                    ),
                );
            }
        }
        out
    };
    let a = full(dump.frames[0].watch("typedb-mirror-rows").unwrap());
    let b = full(dump.frames[18].watch("typedb-mirror-rows").unwrap());
    let diff: Vec<(u16, u8)> = a
        .iter()
        .filter(|(k, v)| b.get(*k) != Some(*v))
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(
        diff,
        vec![
            (t769 as u16, 3u8),
            (t74 as u16, 3),
            (t75 as u16, 3),
            (t76 as u16, 3),
            (t77 as u16, 3),
            (t78 as u16, 3),
        ],
        "the whole-map consume census: the corridor five + the (76,9) side cell"
    );
}

#[test]
fn corpus_s5c_pickup_case_3_predamaged() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S5C (DESIGN §7 S5 row, D108's observability follow-up — the
    // W12-S5C unit): the S4 artillery pattern spends the walker's hp
    // below the clamp BEFORE the walk, so apply_pickup case 3's
    // +2500 lands IN FULL — 1256 → 3756, the exact PICKUP_HEALTH
    // increment, no clamp (S5B could only show the dispatch: its
    // walker spawns AT 5000). The gunner rides the same burst box
    // (staged ON the walker's tile), takes the same 3744, claims its
    // own spread slot, and walks one robot behind the whole way —
    // it reaches no unconsumed case-3 cell, hp 1256 through the
    // tail. 55 records (anchor + 54), pinned chain.
    let s5c = fs::read_to_string(scen_path("S5C")).expect("S5C.scen committed");
    let run = run_canonical(&s5c, &root).expect("S5C canonical run");
    assert_eq!(run.manifest.frame_count, 55);
    assert_eq!(
        run.manifest.chain_digest, "84b88562afa6fa54",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s5c, &root).expect("S5C canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S5C dump verifies");

    let m = |f: usize, tile: usize| {
        mirror_cell(dump.frames[f].watch("typedb-mirror-rows").unwrap(), tile, 3)
    };
    let money = |f: usize| {
        u32::from_le_bytes(
            dump.frames[f].watch("money").unwrap()[..4]
                .try_into()
                .unwrap(),
        )
    };
    let score = |f: usize| {
        u32::from_le_bytes(
            dump.frames[f].watch("score").unwrap()[..4]
                .try_into()
                .unwrap(),
        )
    };
    let bank = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap());
    let hp = |f: usize, i: usize| bank(f)[i].i32(56);
    let state = |f: usize, i: usize| bank(f)[i].state();
    let tile = |f: usize, i: usize| bank(f)[i].tile();
    let t74 = 10 * 100 + 74;
    let t75 = 10 * 100 + 75;
    let t76 = 10 * 100 + 76;
    let t77 = 10 * 100 + 77;
    let t78 = 10 * 100 + 78;
    let t769 = 9 * 100 + 76;
    // The staged corridor (same ZONEB set-2 surface as S5B).
    assert_eq!(m(0, t74), (0x83, 0));
    assert_eq!(m(0, t75), (0x83, 0));
    assert_eq!(m(0, t76), (0x7b, 0), "the case-3 word stages");
    assert_eq!(m(0, t77), (0x83, 0));
    assert_eq!(m(0, t78), (0x83, 0));
    assert_eq!(m(0, t769), (0x83, 0));
    assert_eq!((score(0), money(0)), (0, 3500));
    // THE OBSERVABILITY SPEND (the S4 artillery pattern): the
    // frame-1 command's three records (9/0xA/0xB) all walk their
    // list-0 3x3 at tick 0x20 = frame 32; the four pairs whose blast
    // boxes reach a marker-staged robot (+0xF00 offset) spend the
    // walker AND the gunner 3x4x312 = 3744 each; the 0xB's outer
    // ring spends the clicker 624 at frame 36. Everyone survives;
    // all of it lands while the robots are state 0/3 (the hp path —
    // a state-4 robot converts damage to a shield tick).
    assert_eq!((hp(31, 1), hp(31, 2), hp(31, 3)), (5000, 5000, 5000));
    assert_eq!(
        (hp(32, 1), hp(32, 2), hp(32, 3)),
        (5000, 1256, 1256),
        "the burst box spends walker+gunner 3744 at frame 32"
    );
    // Debris-physics re-baseline (D115, §7j.44): the burst's
    // spread chunks add three mag-2 debris hits around the burst
    // frames (one at 35, two at 36) — the clicker ends 6 lower.
    assert_eq!(hp(36, 1), 4370, "624 ring + 3x2 debris chips");
    // The order arms at frame 37 (after the burst windows close).
    assert_eq!((state(37, 1), state(37, 2), state(37, 3)), (3, 4, 4));
    // The consume schedule: five case-4 cells + the case-3 cell.
    assert_eq!(m(37, t74), (0x48F, 1), "case 4 at frame 37");
    assert_eq!(m(39, t75), (0x48F, 1), "case 4 at frame 39");
    assert_eq!(m(44, t77), (0x48F, 1), "case 4 at frame 44");
    assert_eq!(m(46, t78), (0x48F, 1), "case 4 at frame 46");
    assert_eq!(m(41, t769), (0x48F, 1), "the (76,9) side cell at frame 41");
    // THE HEADLINE: case 3 at (76,10) fires at frame 41 and the
    // healed robot's hp body is VALUE-VISIBLE — the exact +2500,
    // unclamped (the S5B observability gap closed). DEBRIS-PHYSICS
    // RE-BASELINE (D115, §7j.44): the burst-spread chunks chip
    // BOTH stacked robots (1256 → 1246 by f36) and the knock
    // nudges their px paths, so the case-3 consume order FLIPS —
    // the GUNNER (r3) probes the cell first (f41, 1246 → 3746)
    // and the WALKER (r2) becomes the unhealed negative control
    // (case 3 is single-use). The heal value stays the exact
    // +2500 unclamped — the scenario's purpose is intact; an O1
    // capture arbitrates the consume-order flip.
    assert_eq!(m(40, t76), (0x7b, 0), "not yet consumed at frame 40");
    assert_eq!(m(41, t76), (0x48F, 1), "CASE 3 at frame 41");
    assert_eq!(hp(40, 2), 1246);
    assert_eq!(
        hp(41, 3),
        3746,
        "case 3 heals the pre-damaged gunner the exact +2500, unclamped"
    );
    assert_eq!(
        hp(41, 2),
        1246,
        "the walker (one probe behind, case 3 already consumed) never heals"
    );
    assert_eq!(hp(54, 3), 3746, "gunner hp through the tail");
    assert_eq!(
        money(41),
        3650,
        "the side cell's case-4 draw folds same-frame"
    );
    assert_eq!((score(54), money(54)), (2667, 3710), "the pinned tail");
    // The arrival at frame 48: walker state 4→3 snapped at (78,10)
    // (one tile short of its spread slot — the west-approach
    // ARRIVE_RADIUS semantics, the S2 precedent).
    assert_eq!(state(47, 2), 4);
    assert_eq!(state(48, 2), 3, "the arrival clear at frame 48");
    assert_eq!(tile(48, 2), (78, 10));
    assert!(bank(48)[2].snapped(), "snapped at the tile origin");
}

#[test]
fn corpus_s6_pad_extraction() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S6 (DESIGN §7 S6 row, §10-W12; §7j.40 the trigger chain, D86
    // the pad op, D112): the scripted .PAD step-on arms the beacon
    // through the REAL producer (the pad-script armer, not the click
    // seam) and the dropship runs its full cycle — deploy at the
    // trigger frame (the single-robot window-0 expiry), descent,
    // the extraction sweep (state 3 → 5), the RNG-jittered dwell,
    // the departure drift, and the completion freeze. 75 records
    // (anchor + 74), pinned chain.
    let s6 = fs::read_to_string(scen_path("S6")).expect("S6.scen committed");
    let run = run_canonical(&s6, &root).expect("S6 canonical run");
    assert_eq!(run.manifest.frame_count, 75);
    assert_eq!(
        run.manifest.chain_digest, "1f26e1343d7296d4",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s6, &root).expect("S6 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S6 dump verifies");

    let bank = |f: usize| robots_of(dump.frames[f].watch("robot-bank").unwrap());
    let state = |f: usize| bank(f)[0].state();
    let tile = |f: usize| bank(f)[0].tile();
    let stop = |f: usize| bank(f)[0].i32(39);
    let beacon = |f: usize| -> Vec<i32> {
        dump.frames[f]
            .watch("beacon-family")
            .unwrap()
            .chunks_exact(4)
            .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };
    let claims = |f: usize| -> Vec<u16> {
        dump.frames[f]
            .watch("spread-claims")
            .unwrap()
            .chunks_exact(2)
            .map(|w| u16::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };
    let craft = |f: usize| -> Vec<i32> {
        dump.frames[f]
            .watch("dropship-frame")
            .unwrap()
            .chunks_exact(4)
            .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };
    let target = |f: usize| -> Vec<i32> {
        dump.frames[f]
            .watch("order-target")
            .unwrap()
            .chunks_exact(4)
            .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    };

    // THE PAD OP (D86): the triple is the .PAD slot 0x12 record
    // (19,70,0) in raw tile words, written at frame 1 and mirrored
    // to the tail (the seam write persists; nothing rewrites it).
    assert_eq!(target(0), vec![0, 0, 0]);
    assert_eq!(target(1), vec![19, 70, 0]);
    assert_eq!(target(74), vec![19, 70, 0]);

    // The command-driven walk (the bit0 SELECT auto-arm — the
    // original's own move mechanism, NOT the click seam): leg 1
    // walks west state 1, arrives state 0 inside tile (19,73);
    // leg 2 re-arms and walks north state 1.
    assert_eq!((tile(0), state(0)), ((21, 73), 0));
    assert_eq!(state(2), 1, "leg 1 armed at frame 2");
    assert_eq!(state(6), 0, "leg 1 arrival: state 1 -> idle");
    assert_eq!(tile(6), (19, 73));
    assert_eq!(state(9), 1, "leg 2 armed at frame 9");
    assert_eq!(tile(12), (19, 71), "the approach row, one tile south");
    // No beacon before the crossing (the click never arms it).
    assert_eq!(beacon(12), vec![0, 0, 0, 0, 0]);
    assert_eq!(craft(12), vec![0, 0, 0, 0, 0, 0, 0]);

    // THE TRIGGER at frame 13: the walker crosses (19,70) mid-walk
    // (state 1 + target present — the dispatcher's dual gate), the
    // probe matches slot 0x12 at LEVEL 0, the armer halts it state 3
    // SNAPPED at the beacon tile origin, and the same frame's
    // MissionShell beacon block deploys (window 0 for one alive
    // robot; FUN_0041faf0 clears ONLY the flag/window pair).
    assert_eq!(state(13), 3, "the pad armer's halt");
    assert_eq!(tile(13), (19, 70));
    assert!(bank(13)[0].snapped(), "snapped at the beacon tile origin");
    // The beacon-family row post-deploy: the SURVIVING tile words
    // 0x4eabb4/6/8 (§7j.40/4) — flag 0, window 0, tile (19,70,31).
    assert_eq!(beacon(13), vec![0, 0, 19, 70, 31]);
    assert_eq!(beacon(74), vec![0, 0, 19, 70, 31], "the latch persists");
    // The claims 0x4eabba are NEVER released (§7j.20/3): slot 0 (the
    // halted walker's own claim) survives the deploy.
    assert_eq!(claims(13), vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(claims(74), vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // The deployed craft: {active 1, phase 1, x = 19·0x20, y =
    // 70·0x20, alt 0x200, group 0, dwell 0}.
    assert_eq!(craft(13), vec![1, 1, 608, 2240, 512, 0, 0]);

    // The descent (phase 1): −0x20 while alt ≥ 0x101, then the
    // (alt>>2)·3 shrink, group flip 0/1.
    assert_eq!(craft(14), vec![1, 1, 608, 2240, 480, 1, 0]);
    assert_eq!(craft(21), vec![1, 1, 608, 2240, 256, 0, 0]);
    assert_eq!(craft(22), vec![1, 1, 608, 2240, 192, 1, 0]);
    assert_eq!(craft(34), vec![1, 1, 608, 2240, 3, 1, 0]);

    // THE LANDING at frame 35: phase 2 + dwell 10 + the EXTRACTION
    // SWEEP — the halted walker state 3 → 5, stop_dist 1e6 (the
    // +0x74 order-target write; the extracted counter rides the
    // accessor below).
    assert_eq!(craft(35), vec![1, 2, 608, 2240, 0, 0, 10]);
    assert_eq!(state(35), 5, "the sweep takes the halted walker");
    assert_eq!(stop(35), 1_000_000);
    assert_eq!(stop(74), 1_000_000, "swept robots keep the order target");
    // The dwell: the RandA()&7==0 altitude jitter (a shared-stream
    // draw) — exactly one hit in the 10 frames, at frame 40.
    assert_eq!(craft(39), vec![1, 2, 608, 2240, 0, 0, 6]);
    assert_eq!(craft(40), vec![1, 2, 608, 2240, 1, 1, 5], "the jitter draw");
    assert_eq!(craft(44), vec![1, 2, 608, 2240, 0, 1, 1]);

    // The departure (phase 3): the dwell's last frame (45) only
    // flips the phase word; the alt/group math starts at 46 —
    // alt += (alt>>2)+1, x −= group·4, the group ramp; completion
    // freezes the record at frame 69.
    assert_eq!(craft(45), vec![1, 3, 608, 2240, 0, 0, 0]);
    assert_eq!(craft(46), vec![1, 3, 608, 2240, 1, 1, 0]);
    assert_eq!(craft(47), vec![1, 3, 604, 2240, 2, 2, 0], "the group drift");
    assert_eq!(craft(68), vec![1, 3, 244, 2240, 453, 5, 0]);
    assert_eq!(
        craft(69),
        vec![0, 3, 224, 2240, 567, 4, 0],
        "alt > 0x200: inactive + complete"
    );
    assert_eq!(craft(74), craft(69), "the frozen record to the tail");
}

// ---------------------------------------------------------------------
// S7 — the platform-dynamics lifecycle (W12-S7, §7j.41/D113)
// ---------------------------------------------------------------------

#[test]
fn corpus_s7_platform_dynamics() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    // S7 (DESIGN §7 S7 row + §10-W12; §7j.41 the decode, D113): the
    // full platform lifecycle in one run — the TRIGGER build (the
    // zone-1 code-5 instance at .POS slot 74, destroyed by the
    // frame-1 artillery burst at f32), the WEAKEN ring gates
    // (300→150 the spread, →75 the site latch), the DESTROY (the
    // spent tiles' k7 debris), and the CREEP (the armed epilogue
    // tick growing a 25-tile 199 bridge from f449). 1361 records
    // (anchor + 1360), pinned chain.
    let s7 = fs::read_to_string(scen_path("S7")).expect("S7.scen committed");
    let run = run_canonical(&s7, &root).expect("S7 canonical run");
    assert_eq!(run.manifest.frame_count, 1361);
    assert_eq!(
        run.manifest.chain_digest, "5d7217beb232d64b",
        "engine/dump behavior drift: re-baseline deliberately with a commit saying why"
    );
    let run_b = run_canonical(&s7, &root).expect("S7 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    let dump = decode_dump(&run.bytes).expect("S7 dump verifies");

    // The platform-strength + tile-word-grid views (tile-major 25
    // wide): the field snapshot helper.
    let field = |f: usize| -> Vec<(i32, i32, i32)> {
        let bank = dump.frames[f].watch("platform-strength").unwrap();
        let mut out = Vec::new();
        for t in 0..(bank.len() / 2) {
            let s = u16::from_le_bytes([bank[2 * t], bank[2 * t + 1]]);
            if s != 0 {
                out.push(((t % 25) as i32, (t / 25) as i32, s as i32));
            }
        }
        out
    };
    let grid7d4 = |f: usize, x: i32, y: i32| {
        let grid = dump.frames[f].watch("tile-word-grid").unwrap();
        u16::from_le_bytes([
            grid[2 * (y * 25 + x) as usize],
            grid[2 * (y * 25 + x) as usize + 1],
        ])
    };
    // The typedb-mirror view: the water z-word + seen at (tile, z).
    let mirror = |f: usize, x: i32, y: i32, z: usize| -> (u16, u8) {
        let words = dump.frames[f].watch("typedb-mirror-rows").unwrap();
        // COMPACT-ACTIVE: u32 count + {tile u16, 8×word u16, 8×seen
        // u8} runs (26 B each).
        let n = u32::from_le_bytes(words[0..4].try_into().unwrap()) as usize;
        for i in 0..n {
            let off = 4 + i * 26;
            if off + 26 > words.len() {
                break;
            }
            let tile = u16::from_le_bytes([words[off], words[off + 1]]) as i32;
            if tile == y * 25 + x {
                // INTERLEAVED per z: {word u16, seen u8} (3 B).
                let w = u16::from_le_bytes([words[off + 2 + 3 * z], words[off + 2 + 3 * z + 1]]);
                let sn = words[off + 2 + 3 * z + 2];
                return (w, sn);
            }
        }
        (0, 0)
    };
    let k7_count = |f: usize| -> usize {
        let d = dump.frames[f].watch("debris-stager").unwrap();
        // u32 count + 42-B records; kind @ +25 (i32 after
        // active/x/y/z/init_a/init_b/seq).
        let n = u32::from_le_bytes(d[0..4].try_into().unwrap()) as usize;
        let mut k7 = 0;
        for i in 0..n {
            let o = 4 + i * 42;
            if o + 42 > d.len() {
                break;
            }
            if d[o] == 1 && i32::from_le_bytes(d[o + 25..o + 29].try_into().unwrap()) == 7 {
                k7 += 1;
            }
        }
        k7
    };

    // THE BUILD at f32: the trigger ring around (3,57) — five tiles
    // build, the gunner's quadrant blocks three, and the burst's
    // ring-0 pair 7 destroys the fresh (4,56) the same frame.
    assert_eq!(
        field(31),
        vec![],
        "no platforms before the burst (the trigger object stands)"
    );
    // THE DEBRIS-DAMAGE LANE (D115, §7j.44): the f32 destroy's
    // chunk field (five k12 mag-25 + the five-effect/TRT/k6 mag-2
    // spreads, phys-6 countdowns after their delays) rolls over
    // the gunner STANDING at (3,57) — 19 hp-change frames
    // f32..f50, waxing/waning with the countdowns, then static
    // through the creep tail (the last event f50).
    let hp = |f: usize| -> i32 {
        let bank = dump.frames[f].watch("robot-bank").unwrap();
        robots_of(bank)[1].i32(56)
    };
    assert_eq!(hp(31), 5000);
    assert_eq!(hp(32), 4996, "the first physics frame (two mag-2 chunks)");
    assert_eq!(hp(33), 4940, "the k12 delays land (4x25 + chips)");
    assert_eq!(hp(37), 4541, "the chunk-field peak window");
    assert_eq!(hp(50), 3752, "the last event (1248 total debris spend)");
    assert_eq!(hp(60), 3752, "static through the tail");
    assert_eq!(hp(1360), 3752, "static at the last frame");
    assert_eq!(
        field(32),
        vec![(2, 56, 300), (3, 56, 300), (2, 57, 300), (2, 58, 300)],
        "BUILD: the strength-300 trigger ring minus the pair-7 tile"
    );
    assert_eq!(grid7d4(32, 2, 56), 0x7d4, "the platform grid word");
    // The water z-structure: word 0x25D (the zone-1 stamped base)
    // at z2, seen 0 (volume 2 — the FUN_0042394a semantics).
    assert_eq!(mirror(32, 2, 56, 2), (0x25D, 0));
    assert_eq!(
        mirror(32, 2, 56, 1),
        (226, 0),
        "the plateau word below (volume 1 → seen 0)"
    );
    // The pair-7 destroy: 5× k7 debris staged.
    assert_eq!(k7_count(32), 5, "the (4,56) fresh platform's k7 debris");

    // THE WEAKEN + SPREAD at f33: two 75-hits per tile take the
    // 300s to 150 — the §7j.41/3 ring gate (old ≥ 200 ∧ new < 200)
    // fires and the 150-rings build the north row + rebuild (4,56).
    assert_eq!(
        field(33),
        vec![
            (2, 55, 150),
            (3, 55, 150),
            (4, 55, 150),
            (2, 56, 150),
            (3, 56, 150),
            (4, 56, 150),
            (2, 57, 150),
            (2, 58, 300),
        ],
        "WEAKEN to 150 + the SPREAD rings (the north row + the (4,56) rebuild)"
    );
    // f34: the second gate (old ≥ 100 ∧ new < 100) — the 75s + the
    // creep-site latch.
    assert_eq!(
        field(34)
            .iter()
            .map(|&(x, y, s)| (x, y, s))
            .collect::<Vec<_>>(),
        vec![
            (2, 55, 150),
            (3, 55, 150),
            (4, 55, 150),
            (2, 56, 75),
            (3, 56, 75),
            (4, 56, 150),
            (2, 57, 75),
            (2, 58, 300),
        ],
        "the second weaken: 75 (the site latches)"
    );
    // THE DESTROY at f35: the 75-tiles die (75 − 75 ≤ 0), 5× k7
    // each — 15 more debris records beside the f32 five.
    assert_eq!(
        field(35),
        vec![
            (2, 55, 150),
            (3, 55, 150),
            (4, 55, 150),
            (4, 56, 150),
            (2, 58, 300)
        ],
        "DESTROY: the spent tiles clear (both banks + the water word)"
    );
    assert_eq!(k7_count(35), 20, "5 + 3×5 k7 debris records");
    assert_eq!(
        grid7d4(35, 2, 56),
        0,
        "the destroyed tile's grid word clears"
    );
    assert_eq!(mirror(35, 2, 56, 2), (0, 0), "the water z-word clears");

    // THE CREEP: the armed epilogue tick's first tip ring lands at
    // f449 — (2,57) at 199 — and the bridge grows through the tail.
    assert_eq!(field(448).iter().filter(|c| c.2 == 199).count(), 0);
    let f449 = field(449);
    assert_eq!(
        f449.iter().find(|c| c.2 == 199),
        Some(&(2, 57, 199)),
        "the first creep build"
    );
    // The growth: 5 tiles by f474, and the saturated tail (the last
    // change f1240): 22 creep tiles across (3..9, 53..57).
    assert_eq!(field(474).iter().filter(|c| c.2 == 199).count(), 5);
    let tail = field(1360);
    let n199 = tail.iter().filter(|c| c.2 == 199).count();
    assert_eq!(n199, 22, "the saturated 199 bridge");
    assert_eq!(tail.len(), 27, "22 creep + the 5 survivors");
    assert_eq!(field(1360), field(1240), "static past the last growth");
    // The creep tiles carry the water z-word too (the 199 build is
    // the same FUN_004228ce write half).
    assert_eq!(grid7d4(1360, 2, 57), 0x7d4);
    assert_eq!(mirror(1360, 2, 57, 2), (0x25D, 0));
}

// ---------------------------------------------------------------------
// W12-S8 (D114): the critter-engagement scenario.
// ---------------------------------------------------------------------

/// Read one critter field out of the critter-bank blob (the
/// emitter's pinned 74 B record layout — the differ's
/// `critter-bank` normalizer mirrors it).
fn critter_field(b: &[u8], i: usize, off: usize, len: usize) -> i64 {
    let o = 4 + i * 74 + off;
    match len {
        2 => i16::from_le_bytes([b[o], b[o + 1]]) as i64,
        _ => i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as i64,
    }
}

/// Mode histogram of the critter bank at one frame.
fn critter_modes(b: &[u8]) -> std::collections::BTreeMap<u16, usize> {
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let mut m = std::collections::BTreeMap::new();
    for i in 0..n {
        let mode = critter_field(b, i, 8, 2) as u16;
        *m.entry(mode).or_insert(0) += 1;
    }
    m
}

#[test]
fn corpus_s8_critter_engagement() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let root = root();
    let s8 = fs::read_to_string(scen_path("S8")).expect("S8.scen committed");
    let run = canonical::run_canonical(&s8, &root).expect("S8 canonical run");
    assert_eq!(run.manifest.frame_count, 121);
    let run_b = canonical::run_canonical(&s8, &root).expect("S8 canonical re-run");
    assert_eq!(run.bytes, run_b.bytes, "byte-identical double run");
    // Chain pin (the fingerprint discipline, D28: moves only on a
    // deliberate engine/dump change, re-baselined loudly).
    assert_eq!(run.manifest.chain_digest, "10c78a7144cf6d3d");

    let dump = decode_dump(&run.bytes).expect("S8 dump verifies");
    assert_eq!(dump.header.scenario, "S8");
    let frames = &dump.frames;

    // THE DEBRIS-DAMAGE LANE (D115, §7j.44) at the S0-12b boot
    // difficulty 1 (D154): the 0x68 lane is now the §7j.15/1
    // difficulty-scaled row (d+1)·75 = 150/hit (was 75 at the
    // mis-modeled d=0 default); the burst windows' spread chunks
    // (mag-2, phys-6 countdowns) still chip −2 on the frames after
    // each hit pair. First hit f5 (−304 = 2×150 + two chips), the
    // pair cadence then lands 150+chip mixes; the burst frames
    // 34/35 spend −776/−626. Pinned tail: 1132 by f39.
    let hp8 = |f: usize| -> i32 {
        let bank = frames[f].watch("robot-bank").unwrap();
        robots_of(bank)[1].i32(56)
    };
    assert_eq!(hp8(4), 5000, "the last pre-hit frame");
    assert_eq!(hp8(5), 4696, "2x150 + two mag-2 chips");
    assert_eq!(hp8(39), 1132, "the tail after the burst windows close");

    // The staging: 17 critters (7 kind-5 + 10 kind-4, §7j.42/5 — the
    // d=1 spawn roll (RandA&1)+1 banks one extra kind-5 vs the d=0
    // staging, S0-12b/D154), the mode split {8: 7, 9: 10} at the
    // anchor.
    let b = frames[0].watch("critter-bank").expect("T2 critter row");
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    assert_eq!(n, 17);
    let mut kinds = std::collections::BTreeMap::new();
    for i in 0..n {
        let kind = critter_field(b, i, 0, 2) as u16;
        *kinds.entry(kind).or_insert(0) += 1;
    }
    assert_eq!(kinds.get(&5), Some(&7));
    assert_eq!(kinds.get(&4), Some(&10));
    // All ACTIVE from frame 0 (neither ZONEA family spawns
    // dormant): kind-5 engage-family modes (the anchor snapshot
    // sits AFTER the first controller pass — the near pack has
    // already transitioned/fired once), kind-4 mode 9, hp scaled
    // base+base·1/27 = 155/207 (the §7j.42 difficulty staging).
    for i in 0..7 {
        let mode = critter_field(b, i, 8, 2);
        assert!(
            matches!(mode, 2 | 3 | 8),
            "kind-5 engage-family mode (got {mode})"
        );
        assert_eq!(critter_field(b, i, 6, 2), 155, "kind-5 hp 150+150/27");
    }
    for i in 7..17 {
        assert_eq!(critter_field(b, i, 8, 2), 9, "kind-4 mode 9");
        assert_eq!(critter_field(b, i, 6, 2), 207, "kind-4 hp 200+200/27");
    }

    // The fire cycle: 0x68 records appear in the ALIASED
    // projectile bank from the first frames (the mode-2 spawn).
    let mut fired = 0usize;
    for f in frames.iter().take(32) {
        if let Some(pb) = f.watch("projectile-bank") {
            let pn = u32::from_le_bytes([pb[0], pb[1], pb[2], pb[3]]) as usize;
            for i in 0..pn {
                let o = 4 + i * 0x22;
                if u16::from_le_bytes([pb[o], pb[o + 1]]) == 0x68 {
                    fired += 1;
                    break;
                }
            }
        }
    }
    assert!(
        fired >= 10,
        "the 0x68 fire cycle ran ({fired} firing frames in f0..32)"
    );

    // The gunner (robot 1) takes the 0x68 damage: hp < 5000 from
    // f5 (150/hit through the walker, owner −1).
    // (robot-bank blob: 4 + n*94, hit_flash u16 at record +62 —
    // asserted via the first hit-flash > 0 before f32. STRIDE FIX
    // (S0-12b/D154): the old walk read stride 0x54/+0x2E — a wrong
    // record offset that passed on a neighbor field; the 94-B
    // record +62 is the pinned §6a hit_flash.)
    let mut flashed = 0usize;
    for f in frames.iter().take(31) {
        if let Some(rb) = f.watch("robot-bank") {
            let (rec, stride) = (1usize, 94usize);
            let o = 4 + rec * stride + 62;
            if u16::from_le_bytes([rb[o], rb[o + 1]]) > 0 {
                flashed += 1;
            }
        }
    }
    assert!(
        flashed > 0,
        "the gunner's hit-flash bumped (0x68 damage landed)"
    );

    // The deaths: the burst (f33..38) kills the approached pack +
    // the walked-in kind-4s — hp ≤ 0 and mode 6 (the dive) by
    // f39; the effect-rows bank fully turned over (80 live rows).
    let b39 = frames[39].watch("critter-bank").expect("critter row f39");
    let m39 = critter_modes(b39);
    let diving = m39.get(&6).copied().unwrap_or(0);
    assert!(
        diving >= 8,
        "the burst deaths (>= 8 divers at f39, got {diving})"
    );
    let mut dead = 0usize;
    for i in 0..17 {
        if critter_field(b39, i, 6, 2) <= 0 {
            dead += 1;
        }
    }
    assert_eq!(dead, diving, "every diver is hp<=0");
    let er = frames[39].watch("effect-rows").expect("T3 effect row f39");
    let ern = u32::from_le_bytes([er[0], er[1], er[2], er[3]]) as usize;
    let live = (0..ern)
        .filter(|i| {
            let o = 4 + i * 28 + 24;
            i32::from_le_bytes([er[o], er[o + 1], er[o + 2], er[o + 3]]) != 0
        })
        .count();
    assert!(live >= 80, "the LRU bank turned over ({live} live rows)");

    // The dying tail: the dives run their countdown-6 leash, the
    // mode-7 counters (0x28) run out, and the survivors' dormancy
    // (mode 0xB) holds through the end (the d=1 respawn table
    // 900 frames out — S0-12b/D154).
    let b110 = frames[110].watch("critter-bank").expect("critter row f110");
    let m110 = critter_modes(b110);
    assert!(!m110.contains_key(&7), "the dying window closed by f110");
    assert!(!m110.contains_key(&6), "the dives completed by f110");
    let b119 = frames[120].watch("critter-bank").expect("critter row f119");
    let m119 = critter_modes(b119);
    let dormant = m119.get(&0xB).copied().unwrap_or(0);
    assert_eq!(dormant, diving, "the dead are dormant at f120");
    // The south pack + the far kind-4s survive: modes 8/9 remain.
    assert!(
        m119.get(&8).copied().unwrap_or(0) >= 3,
        "the south pack survives"
    );
    assert!(
        m119.get(&9).copied().unwrap_or(0) >= 4,
        "the far kind-4s seek on"
    );

    // NO-INJECT re-assertion: S0..S7 chains stay byte-identical
    // (the family is armed ONLY by `critters = 1` — the canonical
    // suite's own pins cover each; this asserts the dump carries
    // no critter rows without the key).
    let s1 = fs::read_to_string(scen_path("S1")).expect("S1.scen committed");
    let run1 = canonical::run_canonical(&s1, &root).expect("S1 canonical run");
    assert_eq!(run1.manifest.chain_digest, "ed7deab5e3df5ba8");
    assert!(decode_dump(&run1.bytes)
        .expect("S1 verifies")
        .frames
        .iter()
        .all(|f| f.watch("critter-bank").is_none()));
}
