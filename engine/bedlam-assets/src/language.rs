//! LANGUAGE.* text tables (P5, D35): the EXW boot loader
//! (FUN_0041c050 language arm) loads the region LANGUAGE.<xxx> file
//! and fills a 96-entry, 0x30-stride string table at 0x0046af5c from
//! its [MENU_ITEMS] section, then a second table from [WARNINGS].
//!
//! Pinned EXW semantics (all verified from the Ghidra listing +
//! disassembly, ghidra-project/exw-font-strings.txt +
//! exw-menu-parse.txt, 2026-08-20):
//! - heading seek FUN_00424679("[MENU_ITEMS]"): scan for a `[`, match
//!   the following heading bytes, repeat on mismatch;
//! - after the heading: separator skip FUN_0042471f = skip while
//!   byte < 0x21 (0x424729 `cmp edx,0x21 / jge`): CR, LF, TAB and
//!   even SPACE are separators;
//! - content opener FUN_004245e6: advance to the next `[` (the
//!   section body block), skip separators after it;
//! - entry loop FUN_0042463d: skip separators, then copy the run of
//!   bytes >= 0x20 (interior and pre-tab spaces kept; TAB/CR/LF
//!   terminate), NUL-terminate into the slot; GameMain runs it until
//!   dst == base + 0x1200, i.e. exactly 96 entries;
//! - the file buffer bound is base + 81000 (an alloc size, larger
//!   than every shipped file); this parser bounds by buffer end and
//!   stops at EOF instead [deviation: typed Ok, never panics].
//!
//! Bytes stay bytes: non-English entries carry high-bit accent
//! encodings (CP437-adjacent) that are not UTF-8.

use crate::AssetsError;

/// Entries in the [MENU_ITEMS] table [verified: 0x1200 / 0x30].
pub const MENU_ITEMS_COUNT: usize = 96;

const HEADING: &[u8] = b"[MENU_ITEMS]";
const SEP_BELOW: u8 = 0x21; // separator = byte < 0x21 (FUN_0042471f)
const ENTRY_MIN: u8 = 0x20; // entry byte = >= 0x20 (FUN_0042463d)

/// Skip separator bytes (< 0x21) from `p`, bounded by the buffer.
fn skip_seps(data: &[u8], mut p: usize) -> usize {
    while p < data.len() && data[p] < SEP_BELOW {
        p += 1;
    }
    p
}

/// Parse the [MENU_ITEMS] entries of a LANGUAGE.* file, EXW table
/// order. Returns up to [`MENU_ITEMS_COUNT`] entries (fewer only if
/// the buffer ends first). The loading-row draws consume entries
/// 0x45/0x46/0x52..=0x58 (bedlam-game loading/font).
pub fn parse_menu_items(data: &[u8]) -> Result<Vec<Vec<u8>>, AssetsError> {
    if data.len() < HEADING.len() {
        return Err(AssetsError::TooSmall { len: data.len() });
    }
    // Heading seek: first window equal to "[MENU_ITEMS]". (The EXW
    // scanner restarts at the next `[` on mismatch - equivalent for
    // any heading without an inner `[`.)
    let mut p = data
        .windows(HEADING.len())
        .position(|w| w == HEADING)
        .map(|p| p + HEADING.len())
        .ok_or(AssetsError::SectionNotFound)?;
    p = skip_seps(data, p);
    // Content opener: the next `[` after the heading separators
    // (FUN_004245e6; no-op when already on it).
    p = data[p..]
        .iter()
        .position(|&b| b == b'[')
        .map(|i| p + i + 1)
        .ok_or(AssetsError::SectionNotFound)?;
    let mut out = Vec::new();
    while out.len() < MENU_ITEMS_COUNT {
        p = skip_seps(data, p);
        if p >= data.len() {
            break;
        }
        let start = p;
        while p < data.len() && data[p] >= ENTRY_MIN {
            p += 1;
        }
        out.push(data[start..p].to_vec());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal EXW-shaped section: heading, blank line, content
    /// block, tab-padded lines, section close.
    fn synth() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"[DM_OVERVIEW_B1]\r\n\r\n[\r\nother section\r\n]\r\n\r\n");
        v.extend_from_slice(b"[MENU_ITEMS]\r\n\r\n[\r\n");
        v.extend_from_slice(b"Difficulty: SIMPLE\r\n");
        v.extend_from_slice(b"Difficulty: STANDARD\t\t\r\n");
        v.extend_from_slice(b"Number of Players: 2 \t\t\r\n");
        v.extend_from_slice(b"  leading spaces eaten\r\n");
        v.extend_from_slice(b"Congratulations !!!\r\n");
        v.extend_from_slice(b"]\t\t\t\t\r\n\r\n[WARNINGS]\r\n");
        v
    }

    #[test]
    fn parses_exw_shaped_sections() {
        let ent = parse_menu_items(&synth()).unwrap();
        assert_eq!(
            ent[0],
            b"Difficulty: SIMPLE".to_vec(),
            "entry 0 (cross-checked: FUN_00446522 reads it)"
        );
        assert_eq!(
            ent[1],
            b"Difficulty: STANDARD".to_vec(),
            "trailing tabs stripped"
        );
        assert_eq!(
            ent[2],
            b"Number of Players: 2 ".to_vec(),
            "pre-tab space kept, tabs stripped"
        );
        assert_eq!(
            ent[3],
            b"leading spaces eaten".to_vec(),
            "leading spaces skipped (<0x21)"
        );
        assert_eq!(ent[4], b"Congratulations !!!".to_vec());
        assert_eq!(
            ent[5],
            b"]".to_vec(),
            "section close copies as an entry (EXW)"
        );
    }

    #[test]
    fn caps_at_96_and_stops_at_eof() {
        // A file whose section has just 2 lines: 2 entries, no panic.
        let mut v = Vec::new();
        v.extend_from_slice(b"[MENU_ITEMS]\r\n[\r\none\r\ntwo\r\n");
        let ent = parse_menu_items(&v).unwrap();
        assert_eq!(ent, vec![b"one".to_vec(), b"two".to_vec()]);
        // 200 lines: capped at MENU_ITEMS_COUNT.
        let mut big = Vec::new();
        big.extend_from_slice(b"[MENU_ITEMS]\r\n[\r\n");
        for i in 0..200u32 {
            big.extend_from_slice(format!("line{i}\r\n").as_bytes());
        }
        assert_eq!(parse_menu_items(&big).unwrap().len(), MENU_ITEMS_COUNT);
    }

    #[test]
    fn rejects_missing_heading_and_tiny_buffers() {
        assert_eq!(
            parse_menu_items(b"no heading here"),
            Err(AssetsError::SectionNotFound)
        );
        assert_eq!(
            parse_menu_items(b"[MEN"),
            Err(AssetsError::TooSmall { len: 4 })
        );
        // Heading found but no content opener.
        assert_eq!(
            parse_menu_items(b"[MENU_ITEMS]\r\n\r\nEND"),
            Err(AssetsError::SectionNotFound)
        );
    }

    #[test]
    fn no_panic_on_randomish_input() {
        let mut s = 7u64;
        let mut next = move || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 33) as u8
        };
        for len in [0usize, 1, 13, 100, 8192, 65536] {
            let d: Vec<u8> = (0..len).map(|_| next()).collect();
            let _ = parse_menu_items(&d);
        }
    }
}
