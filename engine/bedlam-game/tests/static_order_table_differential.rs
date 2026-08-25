//! S0-15 strict-coverage oracle for the order/weapon table row
//! `static-order-table` (RE-EXW-SIM §7j.67, D157).
//!
//! Two halves, the static-oracle convention (S0-07..S0-14 pattern):
//!
//! 1. ORIGINAL-SIDE TRANSCRIPTION (corpus-free): the §7j.67
//!    instruction-level decode of `ghidra-project/exw-text-objdump.txt`
//!    hand-transcribed below — the 12×0x62 = 0x498 geometry pinned from
//!    both ends (the boot memset immediate and the successor chassis
//!    base), the boot/episode/reset/recapture/restore writer cycle, the
//!    §6c.6 spawn-copy + default-order-bits derivation, the SAVED.BDL
//!    word map, and the shop-exit MP mirror. The transcription is the
//!    coverage: it pins the ORIGINAL's init/evolution semantics in
//!    code, independently of the engine.
//! 2. E-SIDE CLASSIFICATION (corpus-gated): the row closes
//!    ORIGINAL-SIDE ONLY under the charter no-fabricated-parity class
//!    (the D149/D155 precedent) — the loadout is host-session state
//!    whose producers (shop, save-load, MP exchange) are outside the E
//!    engine; E has no loadout model and the canonical robot-bank
//!    record is the 94-B modeled subset with NEITHER the +0x36/+0x38
//!    group words NOR the +0x6E order-bits word, so an E emission
//!    would be fabricated parity. The E half pins exactly those seam
//!    facts — the deliberately-absent row and the 94-B record guard —
//!    and pointedly does NOT assert any table bytes on E.
//!
//! This test lives in bedlam-game because the E half is the canonical
//! harness (`parity_harness/canonical.rs`, re-exported the
//! canonical_dump_gate way).

#[path = "../examples/parity_harness/canonical.rs"]
mod canonical;

use std::fs;
use std::path::{Path, PathBuf};

use canonical::run_canonical;
use diffharness::dump::{decode_dump, Channel};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn corpus_present() -> bool {
    root().join("EDITOR").is_dir()
}

// ---------------------------------------------------------------------
// 1. Original-side transcription (§7j.67) — the coverage half
// ---------------------------------------------------------------------

/// §7j.67/A — the table geometry: 12 rows × 0x62, EXW 0x4de664 (the
/// chassis successor at 0x4deafc is ADJACENT) / EXD 0x91ee4 (a 0x90-B
/// path buffer at 0x9237c separates it from the chassis twin 0x9240c).
const EXW_BASE: u32 = 0x4DE664;
const EXD_BASE: u32 = 0x91EE4;
const ROW_STRIDE: usize = 0x62;
const ROW_COUNT: usize = 12;
const TABLE_BYTES: usize = ROW_STRIDE * ROW_COUNT; // 0x498
const EXW_CHASSIS_BASE: u32 = 0x4DEAFC;
const EXD_CHASSIS_BASE: u32 = 0x9240C;

/// §7j.67/B1 — the GameMain boot zero-init site list (the whole-span
/// memset; `call 0x43a48d` is a single-`ret` no-op stub).
const BOOT_ZERO_INIT_EXW: [u32; 4] = [0x41C3D6, 0x41C3DB, 0x41C3E0, 0x41C3E5];
const BOOT_ZERO_INIT_EXD: [u32; 4] = [0x2CD0F, 0x2CD14, 0x2CD19, 0x2CD1E];

/// §7j.67/B — the writer census: SIX families (the §7d.2 list had
/// three + the §7j.45 mirror; the two GameMain families were missing).
const WRITER_FAMILY_SITES_EXW: &[(&str, &[u32])] = &[
    // B1 boot zero-init (whole span)
    ("boot-memset", &[0x41C3D6, 0x41C3DB]),
    // B2 episode-reset memset (block 0x41ca06, called 0x41c5f1)
    ("episode-reset", &[0x41CA06, 0x41CA0B, 0x41CA10]),
    // B3 post-mission recapture (block 0x41ca2e; the idiv-by-squad-size
    // loadout re-pool, calls 0x41c665/0x41c682/0x41c689)
    ("recapture", &[0x41CA2E, 0x41CAE2, 0x41CB0B, 0x41CB24]),
    // B4 save-load restore (FUN_0044745e case 2)
    (
        "save-restore",
        &[
            0x43C3C3, 0x43C3D0, 0x43C3E5, 0x43C3F5, 0x43C402, 0x43C41A, 0x43C42A,
        ],
    ),
    // B5 shop mutation (FUN_00440e45 buy/clear/ammo/staging/sell-all)
    (
        "shop",
        &[
            0x441485, 0x441498, 0x4414A2, 0x4414AB, 0x4417F3, 0x4417FB, 0x441808, 0x441817,
            0x44181F, 0x44182E, 0x44183D, 0x4418DA, 0x4418F7, 0x441E1D, 0x441E27, 0x441E31,
            0x441E3F, 0x442821, 0x442838, 0x44284D, 0x442862, 0x442876, 0x442886, 0x442B97,
        ],
    ),
    // B6 shop-exit MP mirror (0x442b97 name / 0x442ba7 ammo via the
    // +0xE loop carry — the +2 word, never the 0x4de658 latch)
    ("mp-mirror", &[0x442B97, 0x442BA7]),
];

/// §7j.67/C — the reader census: FIVE families, each with a 1:1 EXD
/// twin (the two walks are ordinal-identical).
const READER_FAMILY_SITES_EXW: &[(&str, &[u32])] = &[
    // C1 spawn copy (load_markers, §6c.6)
    ("spawn-copy", &[0x40CEFD, 0x40CF18, 0x40CF33]),
    // C2 MP respawn re-copy (FUN_0040e230, §7j.24)
    ("mp-respawn", &[0x40E97C, 0x40E997]),
    // C3 shop reads (row text 0x4403d3 feeds FUN_00420260; the
    // auto-loadout search 0x443823/0x443859 with eax=(t<<2−t)<<4+t)
    (
        "shop",
        &[
            0x4402CD, 0x4403D3, 0x441479, 0x441B6D, 0x442A46, 0x443823, 0x443859,
        ],
    ),
    // C4 the SAVED.BDL writer walk (FUN_0044693a, 7 groups × 7 words)
    (
        "save-writer",
        &[
            0x446CE1, 0x446CE8, 0x446CFE, 0x446D14, 0x446D2A, 0x446D40, 0x446D56, 0x446D6C,
        ],
    ),
    // C5 MP lobby exchange (FUN_00448ef1 — READ direction only: the
    // §7d.2(c) "writer" gloss corrected; the outgoing 0x4dd4a0 staging)
    ("mp-lobby", &[0x449F94, 0x449FBD]),
];

/// The EXD twin anchors (first site of every family, the ordinal
/// identity §7j.67/C pins).
const TWIN_SITES_EXD: &[u32] = &[
    0x1DBC1, // spawn copy
    0x1F690, // MP respawn
    0x2CD0F, // boot memset
    0x2D2D6, // episode reset
    0x2D398, // recapture
    0x4E583, // save restore
    0x52464, // shop
    0x58BEF, // save writer
    0x5B3FC, // MP lobby
];

/// §7j.67/B1 — the boot zero-init: `mov ecx,0x498; mov edi,<base>`;
/// call <no-op stub>; call <memset-0>`. The whole span zeroes.
fn boot_image() -> Vec<u8> {
    vec![0u8; TABLE_BYTES]
}

/// One table row: 7 groups × 7 words (+0 name, +2 ammo, +4 artifact,
/// +6 price, +8 category, +0xA item, +0xC owned — the §7j.45 layout).
fn row_word(row: &[u8], group: usize, word: usize) -> u16 {
    let off = group * 0x0E + word * 2;
    u16::from_le_bytes([row[off], row[off + 1]])
}

fn set_row_word(row: &mut [u8], group: usize, word: usize, v: u16) {
    let off = group * 0x0E + word * 2;
    row[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

/// §6c.6/§7j.67/C1 — the spawn-copy loop transcribed: robot
/// +0x36 := group word0, +0x38 := +0x3A := group word1, and the
/// default order bits = `1 << first group whose word0 ≠ 0` (found
/// latch: the first nonzero word0 wins, later ones do not re-arm).
fn spawn_copy(row: &[u8]) -> ([u16; 7], [u16; 7], u16) {
    let mut w36 = [0u16; 7];
    let mut w38 = [0u16; 7];
    let mut bits: u16 = 0;
    let mut found = false;
    for g in 0..7 {
        let word0 = row_word(row, g, 0);
        let word1 = row_word(row, g, 1);
        w36[g] = word0;
        w38[g] = word1;
        if !found && word0 != 0 {
            bits |= 1 << g;
            found = true;
        }
    }
    (w36, w38, bits)
}

/// §7j.67/C4/B4 — the SAVED.BDL word map: the writer stages and the
/// restore rewrites the SAME 49 words in the SAME order (word (g,w)
/// ↔ row byte offset g·0x0E + w·2 — the +0xA/+0xC words ride the
/// 0x4de660/0x4de662 loop-carry displacements on the restore side).
fn save_stage_row(row: &[u8]) -> Vec<u16> {
    (0..7)
        .flat_map(|g| (0..7).map(move |w| row_word(row, g, w)))
        .collect()
}

fn save_restore_row(words: &[u16; 49]) -> Vec<u8> {
    let mut row = vec![0u8; ROW_STRIDE];
    for g in 0..7 {
        for w in 0..7 {
            set_row_word(&mut row, g, w, words[g * 7 + w]);
        }
    }
    row
}

/// §7j.67/B3 — the post-mission recapture idiv: `v idiv squad_size`
/// per (player, group). quotient ≠ 0 → the ammo word (+2) := q and
/// the item word (+0xA) := the catalog lookup FUN_0041cb38 (not
/// transcribed — it needs the 0x4ea2ac/0x4ea2b0 catalog tables);
/// quotient == 0 → the `xor edx,eax` quirk leaves word@+0 := r.
fn recapture_divide(pooled: u32, squad_size: u32) -> (u16, u16) {
    assert!(squad_size > 0, "the divisor is [0x46cbd8], never 0 in play");
    let q = pooled / squad_size;
    let r = pooled % squad_size;
    if q == 0 {
        (r as u16, 0)
    } else {
        (0, q as u16) // +0 untouched; +2 := q (the item word follows)
    }
}

/// §7j.67/B6/C5 — the shop-exit MP mirror word pair: the 0x80-stride
/// record (first byte skipped) carries 7 (name, ammo) pairs which the
/// mirror writes to words +0/+2 — the exact cells the lobby read
/// stages back out.
fn mirror_write_pair(record_pair: (u16, u16), row: &mut [u8], group: usize) {
    set_row_word(row, group, 0, record_pair.0);
    set_row_word(row, group, 1, record_pair.1);
}

fn lobby_read_pair(row: &[u8], group: usize) -> (u16, u16) {
    (row_word(row, group, 0), row_word(row, group, 1))
}

#[test]
fn original_geometry_pins_the_extent_from_both_ends() {
    // The stride arithmetic and the immediate agree.
    assert_eq!(ROW_STRIDE, 0x62);
    assert_eq!(TABLE_BYTES, 0x498);
    assert_eq!(TABLE_BYTES / ROW_STRIDE, ROW_COUNT);
    // EXW: the successor chassis table (its own 0x150 = 12×0x1C boot
    // memset) sits DIRECTLY at the order table end — adjacency is the
    // upper-bound proof. EXD: a 0x90-B path buffer intervenes.
    assert_eq!(EXW_BASE as usize + TABLE_BYTES, EXW_CHASSIS_BASE as usize);
    assert_eq!(
        EXD_BASE as usize + TABLE_BYTES,
        0x9237Cusize,
        "the EXD order table ends at the 0x9237c path buffer"
    );
    assert_eq!(EXD_CHASSIS_BASE - 0x9237C, 0x90);
    // The boot memset sites exist and are ordered (ecx → edi → stub →
    // memset at BOTH channels).
    let mut sorted = BOOT_ZERO_INIT_EXW;
    sorted.sort_unstable();
    assert_eq!(
        sorted, BOOT_ZERO_INIT_EXW,
        "the EXW boot sequence is ascending"
    );
    let mut sorted = BOOT_ZERO_INIT_EXD;
    sorted.sort_unstable();
    assert_eq!(
        sorted, BOOT_ZERO_INIT_EXD,
        "the EXD boot sequence is ascending"
    );
    // 12 rows also matches the 12-slot robot bank (D129) and the
    // 12×0x1C chassis rows — the type domain 0..11.
    assert_eq!(0x150 / 0x1C, ROW_COUNT);
}

#[test]
fn original_boot_image_is_the_all_zero_fresh_session_state() {
    // §7j.67/D: boot memset → no SAVED.BDL on a fresh session → the
    // shop mutates nothing without purchases → the MissionShell-entry
    // image is 1176 zero bytes, deterministic.
    let img = boot_image();
    assert_eq!(img.len(), TABLE_BYTES);
    assert!(img.iter().all(|&b| b == 0));
}

#[test]
fn original_spawn_copy_and_default_order_bits_transcription() {
    // Fresh row: zero group words, NO default order bit (the §6c.6
    // derivation finds no nonzero word0 — §7d.4's no-rows consequence).
    let (w36, w38, bits) = spawn_copy(&boot_image()[..ROW_STRIDE]);
    assert!(w36.iter().all(|&w| w == 0));
    assert!(w38.iter().all(|&w| w == 0));
    assert_eq!(bits, 0);

    // Sensitivity direction 1 — the table content IS semantically
    // real: a purchased row arms the first nonzero group only.
    let mut row = vec![0u8; ROW_STRIDE];
    set_row_word(&mut row, 3, 0, 0x0007); // group 3 name word ≠ 0
    set_row_word(&mut row, 3, 1, 150); // …and its ammo
    set_row_word(&mut row, 5, 0, 0x0002); // later nonzero word0 …
    let (w36, w38, bits) = spawn_copy(&row);
    assert_eq!(bits, 1 << 3, "the FIRST nonzero word0 wins the default bit");
    assert_eq!(w36[3], 0x0007);
    assert_eq!(w38[3], 150, "word1 is copied to BOTH +0x38 and +0x3A");
    assert_eq!(w36[5], 0x0002);
    // …and never re-arms (found latch): bits has exactly one bit set.
    assert_eq!(bits.count_ones(), 1);

    // Sensitivity direction 2 — ammo alone (word1 ≠ 0, word0 == 0)
    // arms NOTHING: the derivation keys ONLY on the name word.
    let mut row = vec![0u8; ROW_STRIDE];
    set_row_word(&mut row, 2, 1, 999);
    let (_, w38, bits) = spawn_copy(&row);
    assert_eq!(bits, 0);
    assert_eq!(w38[2], 999, "the ammo still copies into the robot record");
}

#[test]
fn original_save_word_map_round_trips_field_exact() {
    // §7j.67/C4/B4 — the SAVED.BDL writer stages and the restore
    // rewrites the same 49 words: round-trip identity, including the
    // +0xA item and +0xC owned words that ride the loop-carry
    // displacements on the restore side.
    let mut row = vec![0u8; ROW_STRIDE];
    for g in 0..7 {
        for w in 0..7 {
            set_row_word(&mut row, g, w, (0x1000 + g * 7 + w) as u16);
        }
    }
    let words = save_stage_row(&row);
    assert_eq!(words.len(), 49);
    let back = save_restore_row(&<[u16; 49]>::try_from(words.as_slice()).unwrap());
    assert_eq!(back, row, "the save grammar is a word-for-word identity");

    // Sensitivity: flipping the item word (g=2, w=5 → the +0xA cell)
    // moves EXACTLY the two bytes at 2·0x0E + 0x0A.
    let mut words2 = words.clone();
    words2[2 * 7 + 5] ^= 0xFFFF;
    let back2 = save_restore_row(&<[u16; 49]>::try_from(words2.as_slice()).unwrap());
    let diff: Vec<usize> = (0..ROW_STRIDE).filter(|&i| back2[i] != row[i]).collect();
    assert_eq!(diff, vec![2 * 0x0E + 0x0A, 2 * 0x0E + 0x0A + 1]);
}

#[test]
fn original_recapture_idiv_and_the_zero_fixed_point() {
    // §7j.67/B3 — the idiv table: q==0 leaves the remainder in word@+0
    // (the `xor edx,eax` quirk); q!=0 writes the quotient to +2.
    assert_eq!(recapture_divide(0, 3), (0, 0));
    assert_eq!(recapture_divide(2, 3), (2, 0)); // q=0, r=2 → +0 := 2
    assert_eq!(recapture_divide(3, 3), (0, 1)); // q=1 → +2 := 1
    assert_eq!(recapture_divide(7, 3), (0, 2));
    assert_eq!(recapture_divide(9, 3), (0, 3));
    assert_eq!(recapture_divide(11, 3), (0, 3)); // remainder 2 DISCARDED here

    // THE ZERO FIXED POINT (§7j.67/D): a fresh mission copies zero
    // ammo into every robot (+0x38 words = 0), the recapture pools
    // zeros, and 0 idiv squad_size = (q=0, r=0) → word@+0 := 0 — the
    // all-zero image is closed under the whole writer cycle. Absent
    // shop/save/MP input the table can never leave zero.
    let mut next = boot_image();
    for p in 0..ROW_COUNT {
        for g in 0..7 {
            let pooled = 0u32; // fresh robots carry zero ammo words
            let (w0, w1) = recapture_divide(pooled, 4); // any squad size
            let row = &mut next[p * ROW_STRIDE..(p + 1) * ROW_STRIDE];
            set_row_word(row, g, 0, w0);
            set_row_word(row, g, 1, w1);
        }
    }
    assert_eq!(next, boot_image(), "zero is the recapture fixed point");

    // And a NONZERO pooled ammo breaks the fixed point — the
    // falsification direction (a played campaign's post-mission rows
    // are genuinely loadout-bearing).
    let (w0, w1) = recapture_divide(9, 3);
    assert_ne!((w0, w1), (0, 0));
}

#[test]
fn original_mp_mirror_round_trips_the_exchange_pair() {
    // §7j.67/B6/C5 — the mirror writes (name, ammo) to words +0/+2;
    // the lobby read stages exactly those words back out.
    let mut row = vec![0u8; ROW_STRIDE];
    for g in 0..7 {
        mirror_write_pair(((0x20 + g as u16) & 0xFF, 100 * g as u16), &mut row, g);
    }
    for g in 0..7 {
        assert_eq!(
            lobby_read_pair(&row, g),
            ((0x20 + g as u16) & 0xFF, 100 * g as u16)
        );
    }
    // The pair lives at the row's first four bytes of each group — the
    // exact cells the spawn copy consumes as word0/word1.
    let (w36, w38, _) = spawn_copy(&row);
    for g in 0..7 {
        assert_eq!(w36[g], (0x20 + g as u16) & 0xFF);
        assert_eq!(w38[g], 100 * g as u16);
    }
}

#[test]
fn original_writer_reader_census_pins() {
    // §7j.67/B/C — the census shape: SIX writer families, FIVE reader
    // families, NINE EXD twin anchors (one per family, ordinal-identical
    // walks). Every pinned address is inside its channel's family
    // region; the boot/memset pair bookends the table window.
    assert_eq!(
        WRITER_FAMILY_SITES_EXW.len(),
        6,
        "boot, episode-reset, recapture, save-restore, shop, mp-mirror"
    );
    assert_eq!(
        READER_FAMILY_SITES_EXW.len(),
        5,
        "spawn-copy, mp-respawn, shop, save-writer, mp-lobby"
    );
    assert_eq!(TWIN_SITES_EXD.len(), 9);
    // The boot memset writes the WHOLE span — the extent pin's anchor.
    assert_eq!(BOOT_ZERO_INIT_EXW[0], 0x41C3D6);
    assert_eq!(BOOT_ZERO_INIT_EXD[0], 0x2CD0F);
    // The recapture and mirror are the two families §7d.2 missed /
    // mis-glossed (D157): their pins must survive any future re-census.
    let recapture = WRITER_FAMILY_SITES_EXW
        .iter()
        .find(|(n, _)| *n == "recapture")
        .unwrap();
    assert_eq!(recapture.1, [0x41CA2E, 0x41CAE2, 0x41CB0B, 0x41CB24]);
    let mirror = WRITER_FAMILY_SITES_EXW
        .iter()
        .find(|(n, _)| *n == "mp-mirror")
        .unwrap();
    assert_eq!(mirror.1, [0x442B97, 0x442BA7]);
    let lobby = READER_FAMILY_SITES_EXW
        .iter()
        .find(|(n, _)| *n == "mp-lobby")
        .unwrap();
    assert_eq!(
        lobby.1,
        [0x449F94, 0x449FBD],
        "FUN_00448ef1 READS the table — never writes it"
    );
    // All writer sites fall within the EXW family regions (GameMain,
    // FUN_0044745e, FUN_00440e45) — a coarse monotonic sanity band.
    for (_, sites) in WRITER_FAMILY_SITES_EXW.iter() {
        for &a in sites.iter() {
            assert!(
                (0x41C3D6..=0x443000).contains(&a),
                "writer site {a:#x} outside the census band"
            );
        }
    }
}

// ---------------------------------------------------------------------
// 2. E-side classification (corpus-gated S0 + S1 runs) — the charter
//    no-fabricated-parity seam facts (D157; the D149/D155 precedent).
//    NOT byte comparisons: E emits NOTHING for this row, by design.
// ---------------------------------------------------------------------

#[test]
fn corpus_s0_order_table_row_is_the_documented_original_only_gap() {
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

    // The row is DELIBERATELY ABSENT on E: the loadout is host-session
    // state whose producers (shop, save-load, MP exchange) are outside
    // the engine; there is no loadout model to emit and fabricating an
    // image would be fake parity (§7j.67/E). Any future emitter
    // re-opens this loudly.
    assert!(
        dump.frames
            .iter()
            .all(|f| f.watch("static-order-table").is_none()),
        "static-order-table must stay absent on E (charter original-side row, D157)"
    );

    // The structural guard runs on a T1-carrying scenario (S0 is
    // T0+TS only — no robot-bank rows): the S1 record is the 94-B
    // modeled subset: it contains NEITHER the +0x36/+0x38/+0x3A
    // order-group words NOR the +0x6E order-bits word the original's
    // spawn copy plants. The record length is the loud tripwire:
    // adding any of those fields changes it (and re-opens the
    // classification).
    let s1 = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/diffharness/scenarios/S1.scen"),
    )
    .expect("S1.scen committed");
    let run1 = run_canonical(&s1, &root()).expect("S1 canonical run");
    let dump1 = decode_dump(&run1.bytes).expect("S1 dump verifies");
    for frame in &dump1.frames {
        let blob = frame
            .watch("robot-bank")
            .unwrap_or_else(|| panic!("robot-bank row"));
        assert!(blob.len() >= 4, "robot-bank count word");
        let count = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
        assert!(count > 0, "the staged S1 squad is nonempty");
        assert_eq!(
            blob.len(),
            4 + count * 94,
            "the modeled robot record is exactly 94 B (hit_flash@+62, death_flag@+92) — \
             no order-table consumer fields"
        );
    }
}
