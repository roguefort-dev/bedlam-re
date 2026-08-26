# Bedlam (1996) — P5 Per-Zone Parity Gates + Mission Ledger

**Scope:** the P5 opener per `docs/PLAN.md` §6 (P5). This document pins
(1) the per-zone acceptance shape VERBATIM from the plan, (2) the read-only
enumeration of the 37 shipped missions it applies to, (3) the committed
per-mission disposition ledger format (`docs/P5-MISSION-LEDGER.toml`), and
(4) how per-zone completion gates land in `docs/required-gates.toml` as each
zone closes (the P4 pattern).

**Provenance:** unit `p5-zone-gate-scaffold` (D175). The corpus enumeration
below was performed READ-ONLY against `game-data/` with
`sha256sum -c MANIFEST.sha256 --quiet` clean BEFORE and AFTER (2026-08-27).
No Ghidra run; no engine change.

**Confidence tags:** the enumeration and size arithmetic are VERIFIED
(corpus + `MANIFEST.sha256` + cross-checked against `docs/FORMATS-MISSION.md`
§0, which independently records the same 1+7·5+1 census). The ledger format
and gate wiring are a DECISION (D175), not an RE claim.

---

## 1. Per-zone acceptance shape (VERBATIM from PLAN §6, P5)

The following is quoted byte-for-byte from `docs/PLAN.md` §6
("P5 — Parity completion (per-zone gates)"). It is the acceptance contract
every zone must satisfy; this section is normative for P5 and must track the
plan text (amend here whenever the plan text changes — the checker does NOT
textually bind this quote, the git history does):

> 37 missions playable; AI, weapons, shop, briefings, speech+music+SFX, save/load,
> languages. Multiplayer-only (deathmatch) content carved out of the parity exit
> (defined: maps load + local semantics correct; full DM = future work with netplay).
> Acceptance per zone (playthrough-based, per the 0b budget): all scripted flows
> complete without crashes; T1 game rules verified against RE/8street; perceptual frame
> checks at key moments (T2); differential harness spot-checks for structure (not
> tick-complete); cross-OS replay hash equality of OUR engine (internal determinism).
> Original save compatibility: declared IN — original SAVED/OPTIONS.BDL import is
> read-only, bounds-checked, fuzzed; new saves use the new versioned format.
> Original-behavior catalog is a P5 artifact: a committed, schema-validated per-bug
> ledger (repro, affected missions, severity, gameplay-coupling) that feeds P6 triage;
> an automated completeness gate validates every zone.

Acceptance therefore decomposes per zone into:

| # | Criterion | Class |
|---|-----------|-------|
| 1 | All scripted flows complete without crashes | playthrough |
| 2 | T1 game rules verified against RE/8street | differential/static |
| 3 | Perceptual frame checks at key moments (T2) | diagnostic band (0b budget) |
| 4 | Differential harness spot-checks for structure (not tick-complete) | spot-check |
| 5 | Cross-OS replay hash equality of OUR engine | internal determinism |
| 6 | Original SAVED/OPTIONS.BDL import read-only, bounds-checked, fuzzed | save seam |
| 7 | Multiplayer deathmatch carved out (maps load + local semantics correct) | scope carve-out |

Criterion 7 is a carve-out, not a check: DM-only behavior is out of the parity
exit; the mission's map must still load and its local (single-player) semantics
must still be correct.

---

## 2. The 37 shipped missions (VERIFIED, read-only enumeration)

Enumerated from `game-data/BEDLAM/EDITOR/ZONE{A..G}/MISSION*.TOT` (the `.TOT`
mission-total file is the runtime-loaded mission identity — FORMATS-MISSION
§0.2 runtime extension census; the zone-level lettered `MISSION{A..G}.*` files
carry no `.TOT` and are not missions). The zone directories are the only
mission trees in the corpus (`cd-root` carries none).

| Zone | Missions | Mission TOT sizes (B) | Map dims (TOT header) |
|------|----------|----------------------|----------------------|
| A | MISSION1 | 30004 | 25 × 75 |
| B | MISSION1–7 (7) | 160004 each | 100 × 100 |
| C | MISSION1–7 (7) | 160004 each | 100 × 100 |
| D | MISSION1–7 (7) | 160004 each | 100 × 100 |
| E | MISSION1–7 (7) | 160004 each | 100 × 100 |
| F | MISSION1–7 (7) | 160004 each | 100 × 100 |
| G | MISSION1 | 40004 | 100 × 25 |

**Total: exactly 37 missions (1 + 7·5 + 1).** Arithmetic self-check: the TOT
layout is `u16 w + u16 h + 8 × w·h u16 planes` (FORMATS-MISSION §2, VERIFIED),
i.e. `4 + 16·w·h` bytes — 25·75→30004, 100·100→160004, 100·25→40004; every
enumerated file matches its zone's expected size exactly. Every per-file
digest is pinned by the committed `MANIFEST.sha256` (checked clean before and
after this enumeration). This matches the independent FORMATS-MISSION §0
census (same 1+7·5+1 shape, same dims, 354 375 total tiles).

The enumeration is the completeness basis for the ledger: the checker
(`tools/check-p5-zone-ledger.py`) re-derives it from the corpus at runtime and
fails closed on any drift.

---

## 3. The per-mission disposition ledger

Committed artifact: **`docs/P5-MISSION-LEDGER.toml`**, schema
`p5-mission-ledger-v1`. One `[[mission]]` row per shipped mission, keyed by
`id = "ZONE{L}-MISSION{n}"` with `zone` ∈ A..G and `mission` ≥ 1.

- **disposition** ∈ `{pending, green}` — nothing else. Every mission starts
  `pending`. A mission flips to `green` only when its zone's acceptance shape
  (§1 above) holds for it. A mission that fails verification simply stays
  `pending` (there is no `failed` state: the ledger records closure, not
  attempt history — history lives in `.state/NEXT.md` Done entries and
  DECISIONS.md).
- **catalog_refs** — list of original-behavior catalog entry ids observed on
  that mission (the per-bug catalog is the PLAN §6 P5 artifact that feeds P6
  triage). Empty while `pending` work has not begun; a `green` mission may
  legitimately carry zero refs (no divergences found). Refs are non-empty,
  unique, whitespace-free strings.
- Zone completion status is DERIVED (all missions of the zone `green`), never
  stored — one source of truth per fact.
- Schema evolution: adding fields or dispositions bumps the schema string and
  the checker together (fail-closed on unknown schema), the required-gates-v1
  pattern.

The ledger is machine-checked by `tools/check-p5-zone-ledger.py`
(fail-closed; see §4) and human-audited via its per-zone summary output.

---

## 4. Gate wiring (the P4 pattern applied to P5)

`docs/required-gates.toml` phase P5 starts with exactly ONE required gate:

- **`p5-zone-gate-scaffold`** — runs the ledger checker against the real
  ledger and the read-only corpus, plus the checker's fail-closed test suite.
  This gate validates ledger COMPLETENESS + internal consistency
  (exactly the 37 enumerated missions; valid zones/dispositions/ids; the
  ledger set equals the corpus set), NOT zone completion. It is green from
  the moment the scaffold lands (all 37 missions `pending` is a valid,
  expected state).

Checker consistency rules (all fail-closed):

1. Ledger schema must be `p5-mission-ledger-v1`.
2. The corpus enumeration must yield exactly the pinned zone shape
   (A:1, B–F:7 each, G:1; total 37). Corpus drift (a mission added or
   removed anywhere in `game-data/BEDLAM/EDITOR`) fails loudly until the
   ledger and the pin are deliberately re-baselined.
3. Ledger rows ↔ corpus missions must match exactly (missing row, extra
   row, duplicate id, id/zone/mission mismatch all fail).
4. Dispositions must be `pending` or `green`; `catalog_refs` entries
   non-empty, unique, whitespace-free.
5. Cross-artifact safety with the manifest: a per-zone completion gate id
   `p5-zone-{a..g}` present in P5's `required_gates` requires that zone to
   be fully `green` in the ledger (a zone gate can never be wired ahead of
   its closure), and the manifest's P5 phase `status = "green"` requires
   ALL 37 missions `green` (premature phase flips fail even with an empty
   gate list).
6. game-data paths must NOT appear in the gate's `tracked_paths` or
   `corpus` (game-data is never git-tracked); the checker reads the corpus
   READ-ONLY at runtime, the same contract `MANIFEST.sha256` already
   enforces.

**Per-zone completion gates land as each zone closes:** when a zone's
missions are all flipped `green`, a `p5-zone-{a..g}` gate is added to P5's
`required_gates` carrying that zone's executable acceptance evidence (the
zone's scripted-flow/T1/spot-check commands). Rule 5 keeps wiring and ledger
consistent in both directions of time.

**P5 phase status stays `pending` until every zone closes** (validator
semantics + rule 5). P5 completion emits only the HEAD/manifest-bound
`.state/P5-COMPLETE` via the validator's `--phase P5` path; global plan
completion stays controller-owned.

---

## 5. Disposition lifecycle (operational)

1. Zone work units flip their missions' dispositions in the SAME commit as
   the evidence (gate commands green), citing D-numbers in `catalog_refs`.
2. The scaffold gate re-runs on every validation; any inconsistency
   (half-flipped zone, id typo, corpus drift) blocks the whole P5 phase.
3. The original-behavior catalog (per-bug ledger: repro, affected missions,
   severity, gameplay-coupling) is a separate committed P5 artifact per PLAN
   §6; `catalog_refs` values are its entry ids, so the catalog feeds P6
   triage directly from the mission ledger.
