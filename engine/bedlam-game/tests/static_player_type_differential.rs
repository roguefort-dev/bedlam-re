//! S0-16 strict-coverage oracle for the player TYPE row
//! `static-player-type` (RE-EXW-SIM §7j.68, D159).
//!
//! Two halves, the static-oracle convention (S0-07..S0-15 pattern):
//!
//! 1. ORIGINAL-SIDE TRANSCRIPTION: the §7j.68 instruction-level
//!    decode hand-transcribed below — the boot writer on BOTH
//!    channels (the CINEMATICS sandwich + the ordinal-identical
//!    tail), the whole writer census (EXW 6 = boot + 5 MP-lobby
//!    sites, four of them the −1 error exit; EXD 2 = boot twin +
//!    the MP serial-sync writer), the save-family READ-only proof,
//!    and the spawn-consumer transcription (the kind stamp, the
//!    first-robot cell, the "my robot" gate). Corpus-gated byte
//!    probes re-derive the census against the actual BEDLAM.EXW
//!    image (the raw-dword scan, the D133 technique).
//! 2. E-SIDE PINS (both-sides closure, the D154 seam class — NOT
//!    the D157 no-fabricated-parity class): the sim genuinely
//!    models the cell (`MissionSim::player_type`, constant 0, no
//!    setter — the census's writer set is boot+MP, both outside
//!    the mission sim) and three real gates consume it. The row's
//!    E half is the canonical ANCHOR emission (u16 LE, 00 00) —
//!    pinned here as bytes on the real S0 run, and pinned as
//!    WIRED-TO-THE-CELL (a hand-built state with a nonzero type
//!    emits different bytes — the row is not a hardcoded zero).
//!
//! MP/config value semantics are EXCLUDED from this row's closure
//! by the task charter (a later named task owns the lobby/sync
//! families); the census below pins the writer SITES only.

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::{Path, PathBuf};

use bedlam_core::mission::{AngleTable, MissionSim, Robot, Terrain};
use canonical::{emit_frame, run_canonical, TickState};
use diffharness::dump::{decode_dump, Channel};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR").is_dir()
}

// ---------------------------------------------------------------------
// 1a. The cell + the boot writer (§7j.68/A + /B1 + /C1)
// ---------------------------------------------------------------------

/// The registry cells: EXW 0x4edb90 / EXD 0x1075c0. DWORD-written,
/// WORD-consumed (the two spawn kind stamps are the only word
/// reads); the high words 0x4edb92/0x1075c2 have ZERO references —
/// extent 2 captures the consumed word.
const EXW_CELL: u32 = 0x4EDB90;
const EXD_CELL: u32 = 0x1075C0;

/// The EXW boot writer block (§7j.68/B1), hand-transcribed from
/// `ghidra-project/exw-text-objdump.txt` 0x41c327..0x41c351:
/// CINEMATICS [0x46cca4] := 1 around the sound-init call
/// FUN_0043a144 (D134; the §7d.3 "bootattract" gloss superseded),
/// then `xor eax,eax` and the unconditional TYPE := 0 store, then
/// the radio-warning-post successor — the whole tail that the EXD
/// twin preserves instruction-for-instruction in ordinal position.
const EXW_BOOT_WRITER: &[(u32, &[u8])] = &[
    // 41c339  mov [0x46cca4],edi        ; CINEMATICS := 1
    (0x41C339, &[0x89, 0x3D, 0xA4, 0xCC, 0x46, 0x00]),
    // 41c33f  call 0x43a144             ; the sound init
    (0x41C33F, &[0xE8, 0x00, 0xDE, 0x01, 0x00]),
    // 41c344  xor eax,eax               ; THE VALUE: 0
    (0x41C344, &[0x31, 0xC0]),
    // 41c346  mov [0x46cca4],ebx        ; CINEMATICS restored
    (0x41C346, &[0x89, 0x1D, 0xA4, 0xCC, 0x46, 0x00]),
    // 41c34c  mov [0x4edb90],eax        ; TYPE := 0 (dword store)
    (0x41C34C, &[0xA3, 0x90, 0xDB, 0x4E, 0x00]),
    // 41c351  call 0x42391d             ; warning-post successor
    (0x41C351, &[0xE8, 0xC7, 0x75, 0x00, 0x00]),
];

/// The EXD boot twin (§7j.68/C1): the same CINEMATICS sandwich on
/// the cell pair [0x1194d8]≡[0x46cca4] around the config/sound
/// init FUN_0004be7d (the D134 function twin), `xor ebx,ebx`, the
/// dword store, then the warning-post twin call — the instruction
/// ORDINAL of the EXW tail exactly.
const EXD_BOOT_WRITER: &[(u32, &[u8])] = &[
    // 2cc6a  mov [0x1194d8],edx        ; CINEMATICS := 1
    (0x2CC6A, &[0x89, 0x15, 0xD8, 0x94, 0x11, 0x00]),
    // 2cc70  call 0x4be7d              ; the EXD sound/config init
    (0x2CC70, &[0xE8, 0x08, 0xF2, 0x01, 0x00]),
    // 2cc75  mov [0x1194d8],ebx        ; CINEMATICS restored
    (0x2CC75, &[0x89, 0x1D, 0xD8, 0x94, 0x11, 0x00]),
    // 2cc7b  xor ebx,ebx               ; THE VALUE: 0
    (0x2CC7B, &[0x31, 0xDB]),
    // 2cc84  mov [0x1075c0],ebx        ; TYPE := 0 (dword store)
    (0x2CC84, &[0x89, 0x1D, 0xC0, 0x75, 0x10, 0x00]),
];

#[test]
fn boot_writer_semantics_match_the_transcription() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    // EXW: VA = file + 0x400C00 (the D133 BEGTEXT anchor).
    let exw = fs::read(root().join("BEDLAM.EXW")).expect("BEDLAM.EXW");
    for &(va, want) in EXW_BOOT_WRITER {
        let off = (va - 0x400C00) as usize;
        assert_eq!(
            &exw[off..off + want.len()],
            want,
            "EXW boot-writer byte at {va:#x} drifted — the §7j.68/B1 pin is stale"
        );
    }
    // The fresh-SP value is pinned by the transcription itself: the
    // ONLY SP-path writer stores a register loaded by `xor reg,reg`
    // immediately before — value 0, unconditional, before any
    // mission/screen runs.
    let xor_idx = EXW_BOOT_WRITER
        .iter()
        .position(|&(va, _)| va == 0x41C344)
        .unwrap();
    let store_idx = EXW_BOOT_WRITER
        .iter()
        .position(|&(va, _)| va == 0x41C34C)
        .unwrap();
    assert_eq!(store_idx, xor_idx + 2, "the store is the xor's consumer");

    // EXD twin: the raw file is LE/LX-packed (raw offsets ≠ VAs —
    // the linear image the census read is a scratch artifact of
    // tools/exd-relod.py), so the twin's bytes are pinned by
    // SELF-CONSISTENCY instead: the sandwich stores carry the
    // CINEMATICS cell 0x1194d8's disp32, the value xor and the
    // TYPE store (the cell's own disp32) close the sequence in
    // the exact EXW ordinal.
    assert_eq!(EXD_BOOT_WRITER.len(), 5);
    let (v_cinema1, b_cinema1) = EXD_BOOT_WRITER[0];
    let (v_init, b_init) = EXD_BOOT_WRITER[1];
    let (v_cinema2, b_cinema2) = EXD_BOOT_WRITER[2];
    let (v_xor, b_xor) = EXD_BOOT_WRITER[3];
    let (v_store, b_store) = EXD_BOOT_WRITER[4];
    let cinema_le: [u8; 4] = 0x1194D8u32.to_le_bytes();
    assert!(
        b_cinema1.ends_with(&cinema_le) && b_cinema2.ends_with(&cinema_le),
        "both sandwich stores target the CINEMATICS cell 0x1194d8"
    );
    assert_eq!(b_init[0], 0xE8, "the config/sound init is a call");
    assert_eq!(b_xor, &[0x31, 0xDB], "the value xor: ebx := 0");
    let mut expected_store: Vec<u8> = vec![0x89, 0x1D];
    expected_store.extend_from_slice(&EXD_CELL.to_le_bytes());
    assert_eq!(
        b_store,
        &expected_store[..],
        "the TYPE store: mov [{EXD_CELL:#x}],ebx (the cell's own disp32)"
    );
    // The EXW ordinal exactly: sandwich-on, init, sandwich-off,
    // xor, store (§7j.68/C1).
    assert!(
        v_cinema1 < v_init && v_init < v_cinema2 && v_cinema2 < v_xor && v_xor < v_store,
        "the EXD tail preserves the EXW instruction ordinal"
    );
}

// ---------------------------------------------------------------------
// 1b. The whole writer census (§7j.68/B + /C) — site pins + the
//     raw-dword re-derivation against the actual image
// ---------------------------------------------------------------------

/// The complete EXW writer set (§7j.68/B): the boot store + FIVE
/// sites in the MP lobby FUN_00448ef1. Everything else in the
/// 113-site census is a read. (site, operand bytes)
const EXW_WRITERS: &[(u32, &[u8])] = &[
    (0x41C34C, &[0xA3, 0x90, 0xDB, 0x4E, 0x00]), // boot := 0
    (0x44918A, &[0x89, 0x35, 0x90, 0xDB, 0x4E, 0x00]), // lobby err := esi(−1)
    (0x4493E0, &[0x89, 0x3D, 0x90, 0xDB, 0x4E, 0x00]), // lobby err := edi(−1)
    (0x4497F1, &[0x89, 0x1D, 0x90, 0xDB, 0x4E, 0x00]), // lobby err := ebx(−1)
    (
        0x4498E6,
        &[0xC7, 0x05, 0x90, 0xDB, 0x4E, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],
    ), // lobby err := −1 literal
    (0x449A5C, &[0xA3, 0x90, 0xDB, 0x4E, 0x00]), // lobby ok := ordinal
];

/// The save-family reader bands that the READ-only proof excludes
/// from the writer set (§7j.68/D3): the restore FUN_0044745e
/// 0x43c37a..0x43c7ac and the SAVED.BDL writer FUN_0044693a
/// 0x4469cc..0x446e0a only ever LOAD the cell (the type is never
/// saved — the save derives the name from it, the restore copies
/// the row INTO type·0x62).
const EXW_SAVE_READER_BANDS: &[(u32, u32)] = &[(0x43C37A, 0x43C7AC), (0x4469CC, 0x446E0A)];

#[test]
fn exw_census_is_closed_by_the_raw_dword_scan() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let exw = fs::read(root().join("BEDLAM.EXW")).expect("BEDLAM.EXW");
    // (1) Every literal occurrence of the cell dword in the file
    // image maps to a .text disp32 operand — count them.
    let needle: [u8; 4] = EXW_CELL.to_le_bytes();
    let mut hits: Vec<usize> = Vec::new();
    let mut off = 0;
    while off + 4 <= exw.len() {
        if exw[off..off + 4] == needle {
            hits.push(off);
        }
        off += 1;
    }
    assert_eq!(hits.len(), 113, "the §7j.68 census count is 113");
    // (2) The high word 0x4edb92 is NEVER referenced — the dword
    // writers own it (nonzero only transiently, MP error paths).
    let needle2: [u8; 4] = 0x4EDB92u32.to_le_bytes();
    assert!(
        !exw.windows(4).any(|w| w == needle2),
        "0x4edb92 must have zero references (dword-written, word-consumed)"
    );
    // (3) Every WRITER site's operand bytes are still the pinned
    // store forms; every other hit is a load/cmp/imul disp32 at
    // VA = file + 0x400C00 whose site is NOT in the writer set.
    for &(va, want) in EXW_WRITERS {
        // The disp32 starts 1..2 bytes into the instruction; probe
        // the full pinned operand from the instruction head.
        let head = (va - 0x400C00) as usize;
        assert_eq!(
            &exw[head..head + want.len()],
            want,
            "EXW writer at {va:#x} drifted"
        );
    }
    let writer_vas: Vec<u32> = EXW_WRITERS.iter().map(|&(va, _)| va).collect();
    for &h in &hits {
        let va = h as u32 + 0x400C00;
        // A writer's disp32 lands at va+1 (mov r/m) or va+2 (c7 05).
        let is_writer = writer_vas.iter().any(|&w| va == w + 1 || va == w + 2);
        if !is_writer {
            // Must be a READ-form disp32: the byte before the disp
            // is the modrm of a load/cmp/imul (3b/8b/a1/6b/83...),
            // never a store opcode (a1 is mov eax,moffs = LOAD).
            let op = exw[h - 1];
            assert!(
                matches!(op, 0x05 | 0x0D | 0x15 | 0x1D | 0x2D | 0x35 | 0x3D | 0xA1)
                    || exw[h - 2] == 0x6B // imul r,rm32,imm — disp in modrm tail
                    || exw[h - 2] == 0x3B,
                "unexpected writer form at va {va:#x} (opcode byte {op:#x}) — \
                 census NOT closed, §7j.68/B is stale"
            );
        }
    }
    // (4) The save-family bands contain NO writer site.
    for &(lo, hi) in EXW_SAVE_READER_BANDS {
        assert!(
            !writer_vas.iter().any(|&w| w >= lo && w <= hi),
            "a writer landed inside the save band {lo:#x}..{hi:#x} — the \
             READ-only proof (§7j.68/D3) is stale"
        );
    }
}

#[test]
fn exd_census_two_writers_and_the_sync_strings() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let exd = fs::read(root().join("BEDLAM.EXD")).expect("BEDLAM.EXD");
    // (1) The boot twin's operand bytes (the objdump transcription
    // re-derived from the linear image is §7j.68/C1; here the two
    // marker strings pin the MP-sync path's identity in the file).
    assert_eq!(
        exd.windows(b"Quit from sychronising".len())
            .filter(|w| *w == b"Quit from sychronising")
            .count(),
        1,
        "the 'Quit from sychronising' string (the original's own \
         typo) must occur exactly once — the MP sync writer's family"
    );
    assert_eq!(
        exd.windows(b"Found %i players, but could only sync %i !".len())
            .filter(|w| *w == b"Found %i players, but could only sync %i !")
            .count(),
        1,
        "the 'Found %i players, but could only sync %i !' string"
    );
    // (2) The transcribed census: EXD holds 117 .text sites of the
    // cell, EXACTLY 2 writers (the boot twin + the sync writer);
    // the DOS port has NO lobby family. Pinned as data — the
    // objdump census itself is §7j.68/C (exd-relod linear image).
    const EXD_SITES: usize = 117;
    const EXD_WRITER_COUNT: usize = 2;
    assert_eq!(EXD_WRITER_COUNT, 2);
    assert_eq!(EXD_SITES, 117);
}

// ---------------------------------------------------------------------
// 1c. The spawn-consumer transcription (§7j.68/D1 + /D2)
// ---------------------------------------------------------------------

/// The spawn kind stamp (instruction-exact EXW 0x40cdec ⟷ EXD
/// 0x1db19): `eax := ((i<<2)+i)<<2 + i` (= 21·i), then
/// `WORD[base + eax·8] := WORD[cell]` — the 0xA8-stride robot
/// record's kind field (+0x2A) receives the player TYPE word.
fn spawn_kind_record_index(i: u32) -> u32 {
    let eax = i;
    let eax = (eax << 2) + i; // shl 2 + add
    let eax = (eax << 2) + i; // shl 2 + add
    eax * 8
}

#[test]
fn spawn_consumers_transcribe_instruction_exact() {
    // The kind stamp arithmetic: 21·i·8 = i·0xA8 exactly.
    for i in 0..12u32 {
        assert_eq!(spawn_kind_record_index(i), i * 0xA8);
    }
    // The first-robot cell (EXW 0x40cdfb ⟷ EXD 0x1db28):
    // first_robot := TYPE · robot_count; the selected offset := 0.
    let first_robot = |typ: u32, count: u32| typ.wrapping_mul(count);
    assert_eq!(first_robot(0, 12), 0, "SP: TYPE 0 → robot 0 is mine");
    assert_eq!(first_robot(5, 12), 60, "MP ordinal 5 in a 12-robot bank");
    // The "my robot" gate (the dominant reader family): the record
    // word at +0x2A — read as the dword at +0x28 shifted arithmetically
    // right 16 — equals the (dword-read) type cell.
    let my_robot = |rec_word_2a: u16, typ: u16| rec_word_2a == typ;
    assert!(my_robot(0, 0), "SP: every spawned robot (kind 0) is mine");
    assert!(!my_robot(1, 0), "a kind-1 robot is not mine at TYPE 0");
    assert!(
        my_robot(5, 5),
        "at TYPE 5 the kind-5 robots become mine — \
         the falsification direction: a nonzero type CHANGES gate outcomes"
    );
}

// ---------------------------------------------------------------------
// 2. E-side pins (both-sides closure, D159)
// ---------------------------------------------------------------------

fn synth_sim() -> MissionSim {
    let terrain = Terrain::from_parts(16, 16, vec![0u8; 8 * 16 * 16], Vec::new()).expect("terrain");
    let angles = AngleTable::from_thresholds(&[0u16; 64]).expect("threshold table");
    MissionSim::new(terrain, angles, 0xDEAD_BEEF)
}

#[test]
fn e_side_sim_constant_zero_robots_kind_zero() {
    // The sim's model IS the constant: constructed 0 (the census's
    // only SP-path writer is the GameMain boot store, outside the
    // mission sim), and NO setter exists — verified structurally by
    // the mission-loop run below (a writer would move the value).
    let mut sim = synth_sim();
    assert_eq!(sim.player_type(), 0);
    let idx = sim.spawn_robot((2, 3, 0));
    // The spawn stamp model: Rust robots construct kind 0 — the SP
    // form of `kind := WORD[player_type]`.
    assert_eq!(sim.robots()[idx].kind, 0);
    assert_eq!(sim.robots()[idx].kind, sim.player_type());
    // The mission loop never writes the cell.
    for _ in 0..64 {
        sim.advance_frame();
    }
    assert_eq!(sim.player_type(), 0, "no mission-loop writer exists");
    assert_eq!(sim.robots()[idx].kind, 0);
}

#[test]
fn alarm_gate_fires_on_the_player_type_robot() {
    // The §7g.1 consumer, behavioral: `alarm_ctr > 100 ∧ kind ==
    // player_type` → alarm := 100, ctr := 0. With damage 0 the
    // alarm path still runs (alarm first, then absorb/subtract).
    let mut sim = synth_sim();
    let idx = sim.spawn_robot((2, 3, 0));
    for _ in 0..33 {
        let _ = sim.apply_damage(idx, 0, -1);
    }
    assert_eq!(sim.robots()[idx].alarm_ctr, 99, "33 hits × 3 = 99, no trip");
    assert_eq!(sim.robots()[idx].alarm, 0);
    let _ = sim.apply_damage(idx, 0, -1); // ctr 102 > 100 ∧ 0 == 0
    assert_eq!(
        sim.robots()[idx].alarm,
        100,
        "the gate trips at kind == player_type"
    );
    assert_eq!(sim.robots()[idx].alarm_ctr, 0);
    // The sensitivity direction (transcribed in
    // spawn_consumers_transcribe_instruction_exact): with a nonzero
    // player_type this kind-0 robot would NOT trip — that arm is
    // unreachable in SP precisely because the fresh-SP value is 0.
}

/// A minimal hand-built TickState — the §6a-fixture shape (the
/// canonical_dump_gate way), tiers TS + anchor only.
fn ts_state(player_type: u16) -> TickState<'static> {
    static NO_ROBOTS: [Robot; 0] = [];
    TickState {
        frame_no: 0,
        rand_a_state: 0,
        rand_b_state: 0,
        score: 0,
        money: 0,
        difficulty: 0,
        zone: 0,
        mission: 1,
        mode: 0,
        linear: 1,
        robots: &NO_ROBOTS,
        order: None,
        beacon_latch: None,
        claims_latch: [false; 12],
        dropship: None,
        selected: 0,
        blink_cursor: 0,
        order_target: (0, 0, 0),
        armor_pads: &[],
        map_wh: Some((4, 4)),
        claim_bank: &[],
        player_type,
        weapon_bank: &[],
        enemy_bank: &[],
        critter: None,
        destroy: None,
    }
}

#[test]
fn anchor_row_is_wired_to_the_cell_not_hardcoded() {
    let tiers: Vec<String> = ["TS"].iter().map(|s| s.to_string()).collect();
    // The emitted row is the CELL, not a constant: distinct
    // player_type values produce distinct row bytes.
    let f0 = emit_frame(&ts_state(0), &tiers, false, true);
    assert_eq!(f0.watch("static-player-type"), Some(&[0u8, 0][..]));
    let f1 = emit_frame(&ts_state(1), &tiers, false, true);
    assert_eq!(f1.watch("static-player-type"), Some(&[1u8, 0][..]));
    let f2 = emit_frame(&ts_state(0x1234), &tiers, false, true);
    assert_eq!(f2.watch("static-player-type"), Some(&[0x34, 0x12][..]));
    // Anchor-only: the row rides the anchor frame, never a
    // mission frame.
    let mid = emit_frame(&ts_state(0), &tiers, false, false);
    assert_eq!(mid.watch("static-player-type"), None);
}

#[test]
fn corpus_s0_anchor_row_carries_the_pinned_zero() {
    if !corpus_present() {
        eprintln!("skip: game-data corpus absent (CI)");
        return;
    }
    let s0 = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/diffharness/scenarios/S0.scen"),
    )
    .expect("S0.scen committed");
    let run = run_canonical(&s0, &root()).expect("S0 canonical run");
    let dump = decode_dump(&run.bytes).expect("S0 dump verifies");
    assert_eq!(dump.header.channel, Channel::Engine);
    // The row is PRESENT on the anchor frame with the pinned
    // fresh-SP bytes — the E half of the both-sides closure (the
    // O1 plan dumps the same 2 bytes at CS:001075C0, the O2 plan
    // at 0x004EDB90).
    let anchor = &dump.frames[0];
    assert_eq!(
        anchor.watch("static-player-type"),
        Some(&[0u8, 0][..]),
        "the anchor row must be the pinned fresh-SP 00 00 (§7j.68/E)"
    );
    // Anchor-only: no later frame repeats it.
    assert!(dump.frames[1..]
        .iter()
        .all(|f| f.watch("static-player-type").is_none()));
}
