# Bedlam (1996) — P6 Modernization: ModeConfig Seam, Triage Rubric, Behavior Catalog

**Scope:** the P6 opener per `docs/PLAN.md` §6 (P6). This document pins
(1) the ModeConfig seam decision VERBATIM from the plan, (2) the bug-triage
rubric VERBATIM from the plan (normative for every catalog entry),
(3) the committed original-behavior catalog format
(`docs/P6-BEHAVIOR-CATALOG.toml`, schema `p6-behavior-catalog-v1`) that the
P5 mission ledger's `catalog_refs` feed, (4) the catalog seeding policy, and
(5) how the first P6 required gate (`p6-modernization-scaffold`) lands in
`docs/required-gates.toml` BEFORE any behavior change (the D175 scaffold
pattern: the machine-checkable contract precedes the work it checks).

**Provenance:** unit `p6-modernization-scaffold` (D200). No engine change,
no harness change, no Ghidra run; this unit lands decisions + contract
artifacts only. The P5 ledger fact cited below (37/37 green, all
`catalog_refs` empty) is machine-verified by `tools/check-p5-zone-ledger.py`
(the `p5-zone-gate-scaffold` gate).

**Confidence tags:** the quotes in §1/§2 are VERBATIM plan text (byte-bound
by git history; amend here whenever the plan text changes — the checker does
NOT textually bind these quotes, the git history does). The catalog format,
seeding policy, and gate wiring are a DECISION (D200), not an RE claim. The
post-P5 seeding fact (empty refs) is VERIFIED (ledger + checker, 2026-08-28).

---

## 1. The ModeConfig seam (VERBATIM from PLAN §6, P6)

The following is quoted byte-for-byte from `docs/PLAN.md` §6
("P6 — Modernization (default = modern; classic available)"). It is the
architecture decision P6 implements; D200 makes it binding:

> Architecture (simplified by the 99% target): fixes land directly in the engine —
> there is no bug-complete-faithful core to preserve. Classic mode shrinks to a small
> purist toggle set covering feel-contested items only (timing lock, control scheme,
> selected catalog entries classified for preservation by a deterministic rubric and
> decision record, with regression tests). Mode is one immutable ModeConfig
> injected at sim construction; test surface = the purist toggles, not 2^features.

Consequences D200 records as binding (all grounded in the quote above and
the Determinism Charter in PLAN §3):

- **Fixes land directly in the engine.** There is no dual-core split (no
  "bug-complete-faithful core + modern shell"): the one engine carries every
  fix, and classic mode selects original behavior through the small toggle
  set only.
- **Classic mode = a small purist toggle set**, exactly three sources:
  (i) the timing lock, (ii) the control scheme, and (iii) selected
  original-behavior catalog entries classified for preservation by the
  deterministic rubric (§2) with a decision record and regression tests.
  Nothing else is a classic/modern axis.
- **Mode is ONE immutable `ModeConfig` injected at sim construction.** It is
  never mutated mid-run; a mode change is a new sim. The purist toggles it
  carries are the catalog's `purist_toggle` ids (§3) plus the two plan-named
  axes (timing lock, control scheme) whose concrete ids land with the first
  P6 engine unit that implements the seam.
- **The test surface is the purist toggles, never the full feature
  cross-product.** P6 tests parameterize over the small toggle set (the
  closed catalog entries that carry a `purist_toggle`), never over
  2^(modern features).
- **`ModeConfig` covers sim-behavior-affecting choices only.** Presentation
  and platform options (window mode, vsync, resolution, scaling mode, HD
  pack, refresh rate) are NOT mode toggles: per the plan, display rate
  NEVER enters the sim (Determinism Charter); the logic tick stays fixed at
  the original rate in every mode.

Bounds of this unit: the seam is DECIDED here; no engine code lands with it
(the first P6 engine unit implements `ModeConfig` and the toggle plumbing).

## 2. The bug-triage rubric (VERBATIM from PLAN §6, P6)

The following is quoted byte-for-byte from `docs/PLAN.md` §6 (P6). It is
normative for every catalog entry; §3 encodes it as machine rules:

> Bug triage rubric (per catalog entry): crash/data-loss → fix everywhere;
> gameplay-coupled → classic preserves / modern fixes; cosmetic → fix in modern.
> Fixed = deviation from the catalog established by mechanically applying the rubric
> and recording regression evidence — not vibes.

The rubric as a decision table (the mechanical form §3 enforces):

| Catalog `class`        | Fix policy                                            | Terminal disposition      |
|------------------------|-------------------------------------------------------|---------------------------|
| `crash-data-loss`      | fixed everywhere (modern AND classic)                 | `closed-fix-everywhere`   |
| `gameplay-coupled`     | classic preserves / modern fixes (a purist toggle)    | `closed-preserve-classic` |
| `cosmetic`             | fixed in modern (no toggle; classic is not a look)    | `closed-fix-modern`       |

"Fixed" means: the deviation from the original is established by
mechanically applying the rubric to a catalog entry AND recording regression
evidence — the entry's `evidence` field names the test/gate/document anchor
that proves the fix (for `closed-preserve-classic` entries, the regression
evidence must cover BOTH arms: the modern fix and the classic preservation
through the entry's `purist_toggle`). No vibes.

## 3. The original-behavior catalog format (`p6-behavior-catalog-v1`)

Artifact: `docs/P6-BEHAVIOR-CATALOG.toml`, TOML (the repo convention —
`required-gates.toml`, `watches.toml`, the P5 ledger; stdlib `tomllib`, no
deps), schema string `p6-behavior-catalog-v1` fail-closed. One `[[entry]]`
per cataloged original behavior:

| Field           | Type   | Rule |
|-----------------|--------|------|
| `id`            | str    | unique, non-empty, whitespace-free — the target the P5 ledger's `catalog_refs` point at |
| `title`         | str    | non-empty one-line summary |
| `class`         | str    | one of `crash-data-loss`, `gameplay-coupled`, `cosmetic` (the rubric §2) |
| `observed`      | str    | one of `original` (repro'd observation of the original game), `divergence` (repro'd divergence of our engine from the original) |
| `repro`         | str    | non-empty deterministic repro / oracle evidence pointer (scenario, command, doc §) |
| `missions`      | [str]  | non-empty, duplicate-free, every id a `docs/P5-MISSION-LEDGER.toml` mission id ("affected missions") |
| `disposition`   | str    | one of `open`, `closed-fix-everywhere`, `closed-fix-modern`, `closed-preserve-classic` |
| `evidence`      | str    | non-empty iff closed (the regression-evidence anchor: test id / gate / doc §); empty or absent on open entries |
| `purist_toggle` | str    | present iff `closed-preserve-classic`; non-empty, whitespace-free, unique across the catalog — the `ModeConfig` toggle id that preserves the original behavior in classic mode |
| `provenance`    | str    | non-empty: DECISIONS D-id and/or RE-notes anchor + confidence tag (VERIFIED/LIKELY per repo convention) |

Mechanical rules (fail-closed, `tools/check-p6-behavior-catalog.py`):

- **R1 rubric-as-code:** a CLOSED entry's disposition must be the terminal
  disposition of its class (§2 table). An `open` entry may carry any class
  (observed + classed, fix not yet implemented/evidenced).
- **R2 evidence discipline:** closed ⇒ `evidence` non-empty; open ⇒ no
  evidence (one source of truth — an entry is closed exactly when its
  regression evidence exists).
- **R3 toggle discipline:** `purist_toggle` present iff the disposition is
  `closed-preserve-classic`, and unique across the catalog (one toggle per
  preserved behavior, one behavior per toggle).
- **R4 mission grounding:** `missions` ⊆ the P5 ledger's mission ids. The
  ledger (itself corpus-pinned by the P5 checker) is the single mission
  identity source; this checker does NOT re-enumerate `game-data/`.
- **R5 the P5 feed joins:** every `catalog_refs` value in
  `docs/P5-MISSION-LEDGER.toml` resolves to a catalog entry id (bidirectional
  with R4 — the plan's "feeds P6 triage" made mechanical).
- **R6 scaffold-first manifest:** if `docs/required-gates.toml` P6
  `required_gates` is non-empty, its FIRST entry is `p6-modernization-scaffold`
  and a `[[gate]]` with that id exists (P6 behavior gates can never be wired
  ahead of the contract that grades them — the D175 rule's P6 analogue).
- **R7 phase-gate consistency:** manifest P6 `status = "green"` requires ZERO
  open catalog entries (P6 cannot close with untriaged behaviors; necessary,
  not sufficient — the full P6 exit is PLAN §6).

Layering (one source of truth per fact): ledger schema/corpus binding =
`tools/check-p5-zone-ledger.py`; mission identity for THIS checker = the
ledger; rubric/toggle/manifest-P6 = this checker. The checker reads only
committed docs — it performs NO corpus read (hermetic; `game-data/` never
appears in `tracked_paths`/`corpus`, the never-git-tracked rule).

## 4. Seeding policy (D200)

The catalog seeds **EMPTY**, and both evidence-backed entry sources are
accepted:

- All 37 ledger missions closed green with `catalog_refs = []`
  (machine-verified). P5 parity work recorded zero divergences and zero
  repro'd original-behavior observations worth classifying — the empty
  catalog is the honest post-P5 state (the D175 "0/37 is the honest
  scaffold state" principle).
- Entries land ONLY on recorded evidence with a `repro`:
  `observed = "original"` (an observation of the original game through the
  pinned oracles, an 8street navigation reference re-anchored to EXW/EXD
  addresses per repo policy, or an RE-verified mechanism with doc anchor +
  confidence) or `observed = "divergence"` (a repro'd divergence of our
  engine from the original, found during P6+ work). Speculative or
  retrospective seeding — forum posts, unanchored memory, "probably a bug"
  — is forbidden.
- Why `original` observations must be first-class: after P5 parity our
  engine FAITHFULLY REPRODUCES original behaviors, so an original bug
  surfaces as NO divergence at all. A divergence-only policy would starve
  the catalog of exactly the feel-contested items classic mode exists to
  preserve. The expected dominant entry source is therefore
  `observed = "original"`.

## 5. Gate wiring (the first P6 required gate)

Per the D175 pattern the contract lands before any behavior change:

- `docs/required-gates.toml` P6 `required_gates = ["p6-modernization-scaffold"]`
  (the FIRST entry; R6 enforces it stays first once more P6 gates land).
- Gate commands = the fail-closed checker + its hermetic test suite
  (`tools/check-p6-behavior-catalog.py`, `tools/test-p6-behavior-catalog.py`),
  `tracked_paths` = this doc, the catalog, both tools, and the manifest.
  No `corpus` key; no `writable` (the suite fixtures live under HOME, the
  validator scratch convention).
- The gate validates the CONTRACT (format + rubric-as-code + joins), not P6
  completion: it is green from the moment it lands (0 entries is the honest
  scaffold state). P6 status stays `pending` until the phase's actual exit.
- Later P6 gates (the ModeConfig seam implementation, catalog entries +
  regression evidence, modernization surfaces per PLAN §6) land as evidence
  lands, each behind the scaffold.

## 6. P6 acceptance surface (pointer, not re-statement)

The full P6 phase definition (time-based simulation + high-refresh present,
modern controls, the rubric, resolution independence/GPU, optional HD asset
pipeline, QoL + feel proxies) is PLAN §6 (P6) — this doc does not restate it
beyond the §1/§2 verbatim quotes it operationalizes. Every P6 unit cites the
plan paragraph it implements; divergences from the plan are DECISIONS.md
entries, never silent.
