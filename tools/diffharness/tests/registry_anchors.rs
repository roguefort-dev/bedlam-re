//! W2 registry validity guard — the mechanical anti-ghost check.
//!
//! DESIGN-DIFFHARNESS §4: "no unanchored address ever enters the registry."
//! This test enforces it mechanically: every `anchor` string in
//! watches.toml must resolve, as an EXACT string, to a ledger row heading
//! (the first cell of a markdown table row) or a markdown heading inside
//! the row's `anchor_doc`. If a doc row is renamed or an anchor is
//! fabricated, this test fails — the same spirit as the B2
//! ghost-fabrication lesson.
//!
//! It also checks the registry's schema invariants (tier set, exd_status
//! vs exd_addr consistency, indirect-pointer rules, gap discipline).

use diffharness::{registry, Watch};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

/// anchor_doc id -> repo doc path (relative to the crate manifest dir).
const ANCHOR_DOCS: &[(&str, &str)] = &[
    ("RE-EXW-SIM", "docs/RE-EXW-SIM.md"),
    ("RE-EXD-MAP", "docs/RE-EXD-MAP.md"),
    ("RE-EXW-INPUT", "docs/RE-EXW-INPUT.md"),
    ("RE-EXW-PACER", "docs/RE-EXW-PACER.md"),
    ("RE-EXW-GAMETHREAD", "docs/RE-EXW-GAMETHREAD.md"),
];

const TIERS: [&str; 8] = ["S0", "T0", "T1", "TS", "T2", "T3", "T4", "TI"];
const EXD_STATUSES: [&str; 4] = ["verified", "derived", "gap", "unmapped"];

fn doc_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The ledger universe of one doc: every markdown heading text plus the
/// first cell of every table row (the "ledger row heading" form).
fn ledger_universe(src: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in src.lines() {
        let line = line.trim_end();
        let t = line.trim_start();
        if t.starts_with('#') {
            let text = t.trim_start_matches('#').trim();
            if !text.is_empty() {
                set.insert(text.to_string());
            }
        } else if t.starts_with('|') {
            let cells: Vec<&str> = t.split('|').collect();
            // cells[0] is empty (leading '|'); the first real cell is [1]
            if cells.len() >= 3 {
                let first = cells[1].trim();
                // skip separator rows like |---|---|
                if !first.is_empty()
                    && !first
                        .chars()
                        .all(|c| c == '-' || c == ':' || c == ' ' || c == '=')
                {
                    set.insert(first.to_string());
                }
            }
        }
    }
    set
}

fn universes() -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut map = BTreeMap::new();
    for (id, rel) in ANCHOR_DOCS {
        let src = fs::read_to_string(doc_path(rel))
            .unwrap_or_else(|e| panic!("anchor doc {rel} unreadable: {e}"));
        map.insert(*id, ledger_universe(&src));
    }
    map
}

fn contains_hex_token(s: &str) -> bool {
    s.split(|c: char| !(c.is_ascii_alphanumeric())).any(|tok| {
        tok.len() >= 3 && tok.starts_with("0x") && tok[2..].chars().all(|c| c.is_ascii_hexdigit())
    })
}

#[test]
fn every_anchor_resolves_to_a_ledger_row_heading() {
    let uni = universes();
    let rows = registry();
    assert!(!rows.is_empty());
    let mut failures: Vec<String> = Vec::new();
    for w in &rows {
        let Some(u) = uni.get(w.anchor_doc.as_str()) else {
            failures.push(format!(
                "{}: unknown anchor_doc `{}` (known: {:?})",
                w.id, w.anchor_doc, ANCHOR_DOCS
            ));
            continue;
        };
        if !u.contains(&w.anchor) {
            failures.push(format!(
                "{}: anchor `{}` does NOT resolve in {} — ghost or stale heading",
                w.id, w.anchor, w.anchor_doc
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} anchor failures:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

#[test]
fn registry_schema_invariants_hold() {
    let rows = registry();
    let mut failures: Vec<String> = Vec::new();

    for w in &rows {
        let Watch { id, tier, .. } = w;
        if !TIERS.contains(&tier.as_str()) {
            failures.push(format!("{id}: unknown tier `{tier}`"));
        }
        if !EXD_STATUSES.contains(&w.exd_status.as_str()) {
            failures.push(format!("{id}: unknown exd_status `{}`", w.exd_status));
        }
        // exd_addr <-> exd_status consistency (gaps are explicit, never guessed)
        let has_exd = !w.exd_addr.is_empty();
        match w.exd_status.as_str() {
            "verified" | "derived" if !has_exd => {
                failures.push(format!(
                    "{id}: exd_status {} but exd_addr empty",
                    w.exd_status
                ));
            }
            "gap" | "unmapped" if has_exd => {
                failures.push(format!(
                    "{id}: exd_status {} but exd_addr non-empty (`{}`)",
                    w.exd_status, w.exd_addr
                ));
            }
            _ => {}
        }
        // W2 scope rule: T2-T4 and TI rows stay EXD-empty (W1 ticket)
        if ["T2", "T3", "T4", "TI"].contains(&tier.as_str()) && has_exd {
            failures.push(format!("{id}: tier {tier} must stay exd-empty in W2"));
        }
        // indirect = EXD pointer cell: meaningless without an EXD alias
        if w.indirect && !has_exd {
            failures.push(format!(
                "{id}: indirect=true requires exd_addr (pointer cell)"
            ));
        }
        if w.indirect && !(w.exd_addr.contains("pointer cell") || w.layout.contains("pointer cell"))
        {
            failures.push(format!(
                "{id}: indirect row should name its pointer cell in exd_addr or layout (`{}` / `{}`)",
                w.exd_addr, w.layout
            ));
        }
        // Every row must name a real address on the EXW side (EXD-only
        // exceptions must carry a note explaining why).
        if w.exw_addr.is_empty() && w.note.is_empty() {
            failures.push(format!("{id}: empty exw_addr needs an explanatory note"));
        }
        if !w.exw_addr.is_empty() && !contains_hex_token(&w.exw_addr) {
            failures.push(format!(
                "{id}: exw_addr has no 0x-hex token (`{}`)",
                w.exw_addr
            ));
        }
        if !w.exd_addr.is_empty() && !contains_hex_token(&w.exd_addr) {
            failures.push(format!(
                "{id}: exd_addr has no 0x-hex token (`{}`)",
                w.exd_addr
            ));
        }
        if w.extent.is_empty() || w.layout.is_empty() || w.anchor.is_empty() {
            failures.push(format!("{id}: extent/layout/anchor must be non-empty"));
        }
    }

    // Tier coverage: the DESIGN §4 sets must all be present.
    let mut by_tier: BTreeMap<&str, usize> = BTreeMap::new();
    for w in &rows {
        *by_tier.entry(w.tier.as_str()).or_default() += 1;
    }
    for need in ["S0", "T0", "T1", "TS", "T2", "T3", "T4", "TI"] {
        if !by_tier.contains_key(need) {
            failures.push(format!("missing tier {need} entirely"));
        }
    }
    // The six tagged T0/T1 gaps must be present AND exd-empty (ticket W2).
    let gap_ids = [
        "difficulty",
        "sfx-master-gate",
        "blink-cursor",
        "order-target",
        "no-extract-latch",
    ];
    for gid in gap_ids {
        match rows.iter().find(|w| w.id == gid) {
            Some(w) => {
                if !w.exd_addr.is_empty() {
                    failures.push(format!("{gid}: tagged gap must stay exd-empty"));
                }
                if w.exd_status != "gap" {
                    failures.push(format!("{gid}: tagged gap must carry exd_status=gap"));
                }
            }
            None => failures.push(format!("tagged gap row {gid} missing")),
        }
    }
    // selection cursor/squad is the sixth tagged gap (partial fill allowed
    // for the selected-idx cell, but the gap must be documented).
    match rows.iter().find(|w| w.id == "selection-triple") {
        Some(w) => {
            if !w.note.contains("gap") {
                failures
                    .push("selection-triple: cursor/squad gap must be documented in note".into());
            }
        }
        None => failures.push("selection-triple row missing".into()),
    }

    assert!(
        failures.is_empty(),
        "{} invariant failures:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
