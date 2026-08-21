//! diffharness — P4.2 differential-harness support code (DESIGN-DIFFHARNESS.md).
//!
//! W2 scope: the watch registry (`watches.toml`, the DESIGN §4 watch set as
//! data) plus the minimal parser that loads it. The registry's semantic
//! validity guard (every anchor string resolves to a ledger row heading in
//! its named doc) lives in `tests/registry_anchors.rs` — the mechanical
//! anti-ghost guard demanded by the B2 ghost-fabrication lesson.
//!
//! The parser below understands only the TOML subset the registry uses:
//! comments, `[[watch]]` table headers, and `key = "quoted string"` /
//! `key = true|false` pairs. Anything else is a hard error — the registry is
//! data with a schema, not free-form prose.

use std::collections::BTreeSet;
use std::fmt;

/// One registry row (one watched memory object or event hook).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    /// Stable kebab-case id used in dump records and diff reports.
    pub id: String,
    /// Tier: S0 (frame-tail trigger), T0..T4 (watch tiers), TS (static
    /// one-shot), TI (injection surface — watched AND written).
    pub tier: String,
    /// EXW (Win95, canon) address expression, e.g. "0x4c69e4 + i*0xA8".
    /// Empty only for EXD-only rows (documented via `note`).
    pub exw_addr: String,
    /// EXD (DOS oracle) address expression. Empty = explicit gap; never
    /// guessed (DESIGN §4 EXD-aliasing rule).
    pub exd_addr: String,
    /// true = EXD keeps this bank behind a pointer cell; the runner must
    /// read through the pointer (RE-EXD-MAP §7 divergence seed #4).
    pub indirect: bool,
    /// Watched byte extent (expression form, e.g. "count*0xA8", "w*h*2").
    pub extent: String,
    /// Layout reference: field offsets / record grammar for the field map.
    pub layout: String,
    /// Provenance of the EXD alias: "verified" (disasm-anchored by W1),
    /// "derived" (arithmetic consequence of verified rows), "gap" (T0/T1
    /// row the W1 map explicitly left open), "unmapped" (T2+ / TI rows;
    /// aliasing is a later unit per the W1 ticket).
    pub exd_status: String,
    /// Which doc the anchor heading lives in (RE-EXW-SIM, RE-EXD-MAP, ...).
    pub anchor_doc: String,
    /// Exact ledger row heading / markdown heading string in `anchor_doc`.
    pub anchor: String,
    /// Optional free-form note (partial-coverage gaps, EXD-only rows).
    pub note: String,
}

#[derive(Debug)]
pub struct ParseError {
    line_no: usize,
    line: String,
    reason: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "watches.toml:{}: {} (line: {})",
            self.line_no, self.reason, self.line
        )
    }
}

fn err(line_no: usize, line: &str, reason: &str) -> ParseError {
    ParseError {
        line_no,
        line: line.to_string(),
        reason: reason.to_string(),
    }
}

/// The registry's known keys, for unknown-key detection.
const KNOWN_KEYS: [&str; 11] = [
    "id",
    "tier",
    "exw_addr",
    "exd_addr",
    "indirect",
    "extent",
    "layout",
    "exd_status",
    "anchor_doc",
    "anchor",
    "note",
];

fn unquote(line_no: usize, line: &str, raw: &str) -> Result<String, ParseError> {
    let v = raw.trim();
    if !v.starts_with('"') || !v.ends_with('"') || v.len() < 2 {
        return Err(err(
            line_no,
            line,
            "value must be a double-quoted string or true/false",
        ));
    }
    let inner = &v[1..v.len() - 1];
    if inner.contains('"') {
        return Err(err(
            line_no,
            line,
            "escaped/embedded quotes are not supported",
        ));
    }
    Ok(inner.to_string())
}

/// Parse the registry source (TOML subset) into rows, preserving file order.
pub fn parse_registry(src: &str) -> Result<Vec<Watch>, ParseError> {
    let mut rows: Vec<Watch> = Vec::new();
    let mut pending: Vec<(String, String, usize, String)> = Vec::new(); // (key, value, line_no, raw line)
    let mut in_table = false;

    for (idx, raw_line) in src.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[watch]]" {
            flush(&mut rows, &mut pending)?;
            in_table = true;
            continue;
        }
        if line.starts_with("[[") || line.starts_with('[') {
            return Err(err(line_no, line, "only [[watch]] tables are supported"));
        }
        let Some(eq) = line.find('=') else {
            return Err(err(line_no, line, "expected key = value"));
        };
        let key = line[..eq].trim().to_string();
        let value_raw = line[eq + 1..].trim().to_string();
        let value = match value_raw.as_str() {
            "true" => "true".to_string(),
            "false" => "false".to_string(),
            _ => unquote(line_no, line, &value_raw)?,
        };
        let in_table_now = in_table;
        if !in_table_now {
            return Err(err(line_no, line, "key/value outside a [[watch]] table"));
        }
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(err(line_no, line, "unknown registry key"));
        }
        pending.push((key, value, line_no, line.to_string()));
    }
    flush(&mut rows, &mut pending)?;
    Ok(rows)
}

fn flush(
    rows: &mut Vec<Watch>,
    pending: &mut Vec<(String, String, usize, String)>,
) -> Result<(), ParseError> {
    if pending.is_empty() {
        return Ok(());
    }
    let get = |k: &str| -> Option<String> {
        pending
            .iter()
            .find(|(key, _, _, _)| key == k)
            .map(|(_, v, _, _)| v.clone())
    };
    let req = |k: &str| -> Result<String, ParseError> {
        get(k).ok_or_else(|| {
            err(
                pending[0].2,
                pending[0].3.as_str(),
                &format!("missing required key `{k}`"),
            )
        })
    };
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (key, _, line_no, line) in pending.iter() {
        if !seen.insert(key.as_str()) {
            return Err(err(*line_no, line, "duplicate key in [[watch]] table"));
        }
    }
    let watch = Watch {
        id: req("id")?,
        tier: req("tier")?,
        exw_addr: req("exw_addr")?,
        exd_addr: get("exd_addr").unwrap_or_default(),
        indirect: get("indirect").as_deref() == Some("true"),
        extent: req("extent")?,
        layout: req("layout")?,
        exd_status: req("exd_status")?,
        anchor_doc: req("anchor_doc")?,
        anchor: req("anchor")?,
        note: get("note").unwrap_or_default(),
    };
    rows.push(watch);
    pending.clear();
    Ok(())
}

/// Convenience: parse the committed registry file (via include_str!).
pub fn registry() -> Vec<Watch> {
    parse_registry(WATCHES_TOML).expect("committed watches.toml must parse")
}

/// The committed registry, embedded at compile time.
pub const WATCHES_TOML: &str = include_str!("../watches.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_registry_parses_and_ids_are_unique() {
        let rows = registry();
        assert!(
            rows.len() >= 60,
            "expected the full DESIGN §4 watch set (>=60 rows), got {}",
            rows.len()
        );
        let mut ids = BTreeSet::new();
        for r in &rows {
            assert!(ids.insert(r.id.as_str()), "duplicate id {}", r.id);
        }
    }

    #[test]
    fn rejects_unknown_key_and_bare_value() {
        let bad_key = "[[watch]]\nid = \"x\"\nfoo = \"1\"\n";
        assert!(parse_registry(bad_key).is_err());
        let bare = "[[watch]]\nid = x\n";
        assert!(parse_registry(bare).is_err());
        let outside = "id = \"x\"\n";
        assert!(parse_registry(outside).is_err());
    }

    #[test]
    fn parses_indirect_flag_and_note() {
        let src = "[[watch]]\nid = \"obj\"\ntier = \"T1\"\nexw_addr = \"0x1\"\nexd_addr = \"0x2\"\nindirect = true\nextent = \"4\"\nlayout = \"u32\"\nexd_status = \"verified\"\nanchor_doc = \"RE-EXD-MAP\"\nanchor = \"object instances\"\nnote = \"ptr cell\"\n";
        let rows = parse_registry(src).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].indirect);
        assert_eq!(rows[0].note, "ptr cell");
    }
}
