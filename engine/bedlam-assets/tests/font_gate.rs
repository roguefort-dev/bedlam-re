//! FULLFONT loading-text gate (P5, D35): the LAB_0041c69e zone
//! transition tail draws the loading text row through the EXW font
//! drawer FUN_0043c87c (Ghidra listing ghidra-project/exw-font-drawer.txt,
//! objdump of FUN_0042471f and the FUN_00410493 stubs, 2026-08-20).
//! This gate pins the three assets that flow consumes:
//!
//! - FULLFONT.BIN (GAMEGFX): 390-entry BIN bank, every decodable
//!   entry flags 0x0003 (RLE16 | hotspot); the drawer uses glyph
//!   entry = 0x82 + (c - 0x21) for chars 0x21..=0x7e (entries 130..=223),
//!   accent-remap extras 0x7f..=0x81 (entries 224..=226), and the four
//!   accent overlays at entry 0x82 + 0x6b + id = 238..=241 (id 1..=4:
//!   diaeresis, acute, grave, circumflex - pixel-verified shapes).
//!   ASCII-run glyph pixels are exactly {233..=244} U {0} - inside the
//!   FULLPAL ramp entries 224..=255.
//! - FULLPAL.PAL (GAMEGFX): exactly 98 B = 2-byte lead (e0 20 = first
//!   entry 224, count 32) + 96 B of 6-bit triples. The EXW tail copies
//!   those 96 bytes (24 dwords + 0 tail, ECX=0x60) from the load
//!   buffer +2 into DAC buffer +0x2a2 = fade-target entries 224..=255,
//!   AFTER the pre-text 0x3f fill of the same range.
//! - LANGUAGE.{ENG,FRE,GER,ITL,SPA,DCH}: the [MENU_ITEMS] table (96
//!   entries, FUN_00424679/FUN_004245e6/FUN_0042463d semantics,
//!   language.rs). The loading row draws entries 0x45 / 0x46 /
//!   (zone+0x51) with zone 1..=6 -> 0x52..=0x57, plus 0x58 when
//!   zone == 6. FRE/GER/SPA entries carry high-bit accent bytes that
//!   go through the FUN_00410493 remap (bedlam-game font.rs).
//!
//! Regen path: the ignored test is the ONLY documented way to print
//! the tables. game-data access is read-only; the run is bracketed by
//! MANIFEST.sha256 checks at the shell level.

use std::path::{Path, PathBuf};

use bedlam_assets::language::{parse_menu_items, MENU_ITEMS_COUNT};
use bedlam_assets::pal::parse_font_ramp;
use bedlam_assets::sprites::parse_bin_images;

fn bedlam_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game-data/BEDLAM")
}

fn sha_head(data: &[u8]) -> u64 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let d = h.finalize();
    u64::from_be_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]])
}

fn plane_sha(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// Hot order is the EXW blit order: u16@+2 adds to the DEST ROW,
/// u16@+4 to the DEST COLUMN (FUN_00401ca2).
/// (entry, hotspot (dy, dx), w, h, pixel sha256).
type PinnedGlyph = (usize, (i16, i16), u16, u16, &'static str);

const PINNED_GLYPHS: [PinnedGlyph; 10] = [
    // ASCII run: entry = 0x82 + (c - 0x21). 130 = bang, 162 = A, 223 = ~.
    (
        130,
        (0, 0),
        5,
        24,
        "efec2be3019bca0a342afd0e5a6bdc68ed0b50a1ad3edcb3d58cd0bb94e5a9ab",
    ),
    (
        162,
        (0, 0),
        15,
        19,
        "035b7c755c060d0eb766850306316b8c89146a6468a00ca37dbf82d8312ed228",
    ),
    (
        223,
        (0, 0),
        15,
        19,
        "7d89e82c77626dbcebc5d74193dbcc1f60a8fb034b7d58ba4d3ceb6e1860c4af",
    ),
    // accent-remap extras: 0x7f / 0x80 / 0x81 glyphs (entries 224..=226).
    (
        224,
        (0, 0),
        5,
        19,
        "938a5aa829262c7a28be6ef3950c4446cea7bbde67d9be75146094061222b6c4",
    ),
    (
        225,
        (0, 0),
        14,
        24,
        "abe16c977494e4b37c5916a40190e8d65dd155af1e8e3dfbc7675ee7c3dd007d",
    ),
    (
        226,
        (5, 0),
        15,
        19,
        "26b8c9caf1f0b56a879a996214bed6fedb6bc94ec2f6ef79315522c72fb423d1",
    ),
    // accent overlays: entry 0x82 + 0x6b + id, id 1..=4
    // (238 diaeresis, 239 acute, 240 grave, 241 circumflex).
    (
        238,
        (0, 0),
        15,
        4,
        "fc8e0c218c70e1539426b3a94173349fe6167c61c53e6f28612fa9f558da4417",
    ),
    (
        239,
        (0, 3),
        8,
        6,
        "fcfa2d34ec0db7416526623b122394786b71286e6a6382686058fbdc1e0f4a1c",
    ),
    (
        240,
        (0, 3),
        8,
        6,
        "ce7d1f6e694db646e87389347592df51568cc20a54ab29980f3911467adad1e4",
    ),
    (
        241,
        (0, 1),
        11,
        6,
        "97b92332c3212cf824a77d34739eecc7c00d2eb4610f47f0ac731fbfd048fcc3",
    ),
];

const FULLFONT_SHA_HEAD: u64 = 0x5279_99e6_dc6a_ca69;

#[test]
fn fullfont_bank_is_pinned() {
    let path = bedlam_dir().join("GAMEGFX/FULLFONT.BIN");
    if !path.is_file() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("FULLFONT.BIN: {e}"));
    assert_eq!(sha_head(&data), FULLFONT_SHA_HEAD, "file identity");
    let bank = parse_bin_images(&data).unwrap_or_else(|e| panic!("parse rejected: {e}"));
    assert_eq!(bank.count, 390, "390-entry multi-font bank");
    assert_eq!(bank.images.len(), 390);
    let ok = bank
        .images
        .iter()
        .filter(|im| im.ok && im.pixels.is_some())
        .count();
    let empty = bank
        .images
        .iter()
        .filter(|im| im.codec == "empty-slot")
        .count();
    assert_eq!(ok, 333, "decoded glyph count");
    assert_eq!(empty, 57, "empty slots");
    assert!(
        bank.images
            .iter()
            .all(|im| im.flags == 0x0003 || im.codec == "empty-slot"),
        "decoded entries are RLE16 | hotspot"
    );
    for (entry, hot, w, h, psha) in PINNED_GLYPHS {
        let im = bank
            .images
            .get(entry)
            .unwrap_or_else(|| panic!("entry {entry}"));
        assert_eq!(im.hot, Some(hot), "entry {entry} hotspot (dy,dx)");
        assert_eq!((im.w, im.h), (w, h), "entry {entry} shape");
        let px = im
            .pixels
            .as_deref()
            .unwrap_or_else(|| panic!("entry {entry} pixels"));
        assert_eq!(plane_sha(px), psha, "entry {entry} content");
    }
    // The full ASCII run is present and pixel values sit inside the
    // FULLPAL ramp (224..=255): exactly {0} U {233..=244}.
    let mut vals = std::collections::BTreeSet::new();
    for im in &bank.images[130..=223] {
        let px = im
            .pixels
            .as_deref()
            .unwrap_or_else(|| panic!("ASCII entry undecoded"));
        vals.extend(px.iter().copied());
    }
    let expect: std::collections::BTreeSet<u8> = [0u8].into_iter().chain(233..=244).collect();
    assert_eq!(vals, expect, "ASCII-run pixel values");
    // Baseline anchoring: hotspot dy>0 drops short glyphs (x-height
    // letters dy=5, mid punctuation dy=10, low punctuation dy=15).
    let dys: std::collections::BTreeSet<i16> = bank.images[130..=223]
        .iter()
        .map(|im| im.hot.unwrap().0)
        .collect();
    assert_eq!(dys, [0, 5, 10, 15].into_iter().collect(), "baseline dy set");
}

#[test]
fn fullpal_ramp_is_pinned() {
    let path = bedlam_dir().join("GAMEGFX/FULLPAL.PAL");
    if !path.is_file() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("FULLPAL.PAL: {e}"));
    assert_eq!(data.len(), 98, "2-byte lead + 96 ramp bytes");
    assert_eq!(&data[..2], &[0xe0, 0x20], "lead: first entry 224, count 32");
    let ramp = parse_font_ramp(&data).unwrap_or_else(|e| panic!("parse rejected: {e}"));
    assert_eq!(ramp[0], [0, 0, 0], "entry 224 black (glyph shadow rows)");
    assert_eq!(
        ramp[9],
        [63, 63, 63],
        "entry 233 white (brightest glyph shade)"
    );
    assert_eq!(ramp[31], [0, 22, 47], "entry 255 deep blue (ramp end)");
    let flat: Vec<u8> = ramp.iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(
        plane_sha(&flat),
        "35035a79305bcec19c8243f7669086facfea89c83737c99c9a003d5829eca8fd",
        "ramp content"
    );
    assert_eq!(
        parse_font_ramp(&data[..97]),
        Err(bedlam_assets::AssetsError::WrongSize { len: 97 }),
        "exact-size format"
    );
}

/// (lang, file sha head, [0x45, 0x46, 0x52..=0x58]); 0x27 in the
/// bytes below is the escaped apostrophe of the corpus text.
const LANGS: [(&str, u64, [&[u8]; 9]); 6] = [
    (
        "ENG",
        0xe1b9_65de_229d_ae81,
        [
            b"Congratulations !!!" as &[u8],
            b"Now move out to",
            b"The Airport",
            b"The Industrial Sector",
            b"The Docklands",
            b"The Suburbs",
            b"The City Centre",
            b"The Biomex Nest",
            b"Destroy all BioCapsules",
        ],
    ),
    (
        "FRE",
        0x5f84_9070_38ea_8c0d,
        [
            b"F\x82licitations !!!" as &[u8],
            b"Maintenant, allez vers la zone",
            b"L\x27a\x82roport",
            b"Le secteur industriel",
            b"Les docks",
            b"La banlieue",
            b"Le centre ville",
            b"Le nid de Bio-robots",
            b"D\x82truire toutes les Bio-capsules",
        ],
    ),
    (
        "GER",
        0x5280_d67e_c3ae_a090,
        [
            b"Herzlichen Gl\x81ckwunsch!!!" as &[u8],
            b"Gehen Sie nun ",
            b"zum Flughafen",
            b"zum Industriegebiet",
            b"zum Hafen",
            b"zur Vorstadt",
            b"zur Gro\xe1stadt",
            b"zum Nest der Biomechs",
            b"Zerst\x94ren Sie alle Bio-Kapseln",
        ],
    ),
    (
        "ITL",
        0x13e5_e8ef_2ae6_51ea,
        [
            b"Congratulazioni!!!" as &[u8],
            b"Ora vai a",
            b"L\x27aeroporto",
            b"La zona industriale",
            b"Il porto",
            b"I sobborghi",
            b"Il centro cittadino",
            b"Il nido dei biomeccanici",
            b"Distruggi tutte le bio-capsule",
        ],
    ),
    (
        "SPA",
        0xa4aa_e879_5c13_6100,
        [
            b"\xad\xad\xadEnhorabuena!!!" as &[u8],
            b"Ahora puedes pasar a ",
            b"El aeropuerto",
            b"Los sectores industriales",
            b"El puerto",
            b"Los suburbios",
            b"El centro de la ciudad",
            b"El nido de los bio-robots",
            b"Destruye todas las c\xa0psulas biol\xa2gicas",
        ],
    ),
    (
        "DCH",
        0x62c4_8d55_4a0c_ab96,
        [
            b"Gefeliciteerd !!!" as &[u8],
            b"Ga nu door naar",
            b"De luchthaven",
            b"De industie-sector",
            b"De dokken",
            b"De voorsteden",
            b"Het stadscentrum",
            b"Het biomex-nest",
            b"Vernietig alle biocapsules",
        ],
    ),
];

#[test]
fn language_menu_items_are_pinned() {
    let dir = bedlam_dir();
    if !dir.is_dir() {
        eprintln!("skipping: {} not present", dir.display());
        return;
    }
    for (lang, fsha, entries) in LANGS {
        let path = dir.join(format!("LANGUAGE.{lang}"));
        let data = std::fs::read(&path).unwrap_or_else(|e| panic!("LANGUAGE.{lang}: {e}"));
        assert_eq!(sha_head(&data), fsha, "{lang} file identity");
        let table = parse_menu_items(&data)
            .unwrap_or_else(|e| panic!("LANGUAGE.{lang} parse rejected: {e}"));
        assert_eq!(table.len(), MENU_ITEMS_COUNT, "{lang} table size");
        if lang == "ENG" {
            assert_eq!(
                &table[0], b"Difficulty: SIMPLE",
                "entry 0 (FUN_00446522 reads it)"
            );
        } else {
            assert!(!table[0].is_empty(), "{lang} entry 0 non-empty");
        }
        let idx = [0x45usize, 0x46, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58];
        for (i, want) in idx.iter().zip(entries) {
            assert_eq!(&table[*i], want, "{lang} entry {i:#04x}");
        }
    }
}

/// The EXW drawer arithmetic over the REAL bank widths (FUN_0043c87c:
/// space advance 9, glyph advance w + 2, x0 = 0x140 - total/2). This
/// is the independent data-side check of the bedlam-game measure:
/// pinned totals from the corpus probe, including one accented
/// language (FRE 0x82 remaps to base e, no width change).
#[test]
fn loading_row_measures_over_the_real_bank() {
    let font = bedlam_dir().join("GAMEGFX/FULLFONT.BIN");
    if !font.is_file() {
        eprintln!("skipping: {} not present", font.display());
        return;
    }
    let data = std::fs::read(&font).unwrap();
    let bank = parse_bin_images(&data).unwrap();
    let width = |c: u8| -> i32 { i32::from(bank.images[0x82 + (c as i32 - 0x21) as usize].w) };
    let measure = |s: &[u8]| -> i32 {
        let mut t = 0;
        for &c in s {
            let c = if c >= 0x80 { b'e' } else { c };
            if c < 0x21 {
                t += 9;
            } else {
                t += width(c) + 2;
            }
        }
        t
    };
    let read = |name: &str| std::fs::read(bedlam_dir().join(name)).unwrap();
    let eng = parse_menu_items(&read("LANGUAGE.ENG")).unwrap();
    let fre = parse_menu_items(&read("LANGUAGE.FRE")).unwrap();
    for (table, idx, want) in [
        (&eng, 0x45usize, 252i32),
        (&eng, 0x46, 233),
        (&eng, 0x52, 157),
        (&eng, 0x53, 295),
        (&eng, 0x57, 233),
        (&eng, 0x58, 326),
        (&fre, 0x45, 202),
    ] {
        assert_eq!(measure(&table[idx]), want, "entry {idx:#04x}");
    }
    // x0 = 320 - total/2 for the zone-6 fourth draw (ENG).
    assert_eq!(320 - measure(&eng[0x58]) / 2, 157);
}

#[test]
#[ignore = "inventory regeneration only; paste output into the pinned tables"]
fn regen_inventory() {
    let dir = bedlam_dir();
    let font = std::fs::read(dir.join("GAMEGFX/FULLFONT.BIN")).expect("FULLFONT.BIN");
    println!(
        "== FULLFONT.BIN len={} sha_head={:016x}",
        font.len(),
        sha_head(&font)
    );
    match parse_bin_images(&font) {
        Ok(b) => {
            let ok = b
                .images
                .iter()
                .filter(|i| i.ok && i.pixels.is_some())
                .count();
            let empty = b.images.iter().filter(|i| i.codec == "empty-slot").count();
            println!("   count={} ok={} empty={}", b.count, ok, empty);
            for e in [130usize, 162, 223, 224, 225, 226, 238, 239, 240, 241] {
                let im = &b.images[e];
                println!(
                    "   [{e}] hot={:?} w={} h={} pix_sha={}",
                    im.hot,
                    im.w,
                    im.h,
                    im.pixels.as_deref().map(plane_sha).unwrap_or_default()
                );
            }
            let mut vals = std::collections::BTreeSet::new();
            for im in &b.images[130..=223] {
                vals.extend(im.pixels.as_deref().into_iter().flatten().copied());
            }
            println!("   ASCII pixel set: {:?}", vals);
        }
        Err(e) => println!("   PARSE-REJECT {e}"),
    }
    let fp = std::fs::read(dir.join("GAMEGFX/FULLPAL.PAL")).expect("FULLPAL.PAL");
    println!(
        "== FULLPAL.PAL len={} lead={:#x},{:#x}",
        fp.len(),
        fp[0],
        fp[1]
    );
    match parse_font_ramp(&fp) {
        Ok(r) => {
            let flat: Vec<u8> = r.iter().flat_map(|c| c.iter().copied()).collect();
            println!("   flat_sha={}", plane_sha(&flat));
            for (i, e) in r.iter().enumerate() {
                println!("   entry {}: {:?}", 224 + i, e);
            }
        }
        Err(e) => println!("   PAL-REJECT {e}"),
    }
    for lang in ["ENG", "FRE", "GER", "ITL", "SPA", "DCH"] {
        let d = std::fs::read(dir.join(format!("LANGUAGE.{lang}"))).expect("language file");
        println!(
            "== LANGUAGE.{lang} len={} sha_head={:016x}",
            d.len(),
            sha_head(&d)
        );
        match parse_menu_items(&d) {
            Ok(t) => {
                println!("   entries={}", t.len());
                for i in [0usize, 0x45, 0x46, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58] {
                    println!("   [{i:#04x}] {:?}", t[i]);
                }
            }
            Err(e) => println!("   LANG-REJECT {e}"),
        }
    }
}
