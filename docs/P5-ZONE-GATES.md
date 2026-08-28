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

---

## 6. The all-37-mission load-census GAP TABLE (unit `p5-mission-load-census`, D176)

**Provenance:** unit `p5-mission-load-census` (D176), run 2026-08-27 at
HEAD `6355fba`; RE-PINNED 2026-08-28 (p5-select-shell-g1, D183 — the
SELECT seam rows, §6.2/G1). Every one of the 37 ledger missions was
driven through OUR engine load seams READ-ONLY from
`game-data/BEDLAM`: `GameHost::stage_episode_slot` +
`GameHost::load_mission` (the canonical mission-view load seam:
Terrain + AngleTable + `MissionView::from_mission_bytes` + MapOverlay
+ MRK spawns) where the episode slot reaches the mission,
`GameHost::stage_select_mission` + `load_mission` for the MP-only
missions 6-7 (§6.2/G1), `MissionScene::stage` + the claim bank
directly as the defensive fallback; then the destroy family
(.BDG/.POS/.TRT via `stage_destroy_family`), the pickup surface (.TOT
via `stage_pickup_surface` + the hazard stamper), the critter family
(.NME via `stage_critters`), the bedlam-assets parser family over
every runtime file (grid16/grid8/pad/mrk/pos/trt/nme/bdg + the zone
min/lnk/lng/cgr/bin family), and a short scripted frame run (FSM
Boot→Mission + 9 frames host-side, activate + 8 × tick/present
direct-side; panics caught and reported). The executable artifact is
`engine/bedlam-game/tests/mission_load_census.rs`
(`census_matches_pinned_table` — corpus-gated, pins the table below;
`census_print_table --ignored --nocapture` prints the full columns).
`sha256sum -c MANIFEST.sha256 --quiet` clean BEFORE and AFTER; no
Ghidra run; no corpus write.

**Confidence tags:** every row is VERIFIED by the executable census
(machine-derived, deterministic, re-runnable). The three gap CLASSES
are the census verdict (D176; G1 since landed, D183; G3 since
RESOLVED-NO-SWAP, D184/RE-EXW-SIM §7c.9 — the EXW walk is the
anchor, the 8street reference superseded).

### 6.1 Headline

**ALL 37 missions LOAD through our engine.** Zero load failures, zero
parser refusals, zero frame-run panics. The destroy family, the pickup
surface and every parser accept all 37 missions' runtime file
families. **Since the G2 PERSONNEL/POI landing (§7j.77/D191) ALL 37
missions load with ZERO gaps** — the eleven Personnel-hosting rows
(ZONEE M1-5, ZONEF M1-5, ZONEG M1) flipped clean on top of the G1
SELECT shell (the ten MP-only missions 6-7 of zones B-F, all
`select:clean` since D183 — their .NME files are 16-byte empties),
the G2 Shooters landing (ZONED-M5, D185), the G2 Chasers landing
(the twelve Chasers-only hosts, D186) and the G2 CloseCombat landing
(ZONEC-M3, D189). The ledger is therefore
UNCHANGED: no mission is unloadable-by-corpus, so no disposition
moves (dispositions flip only on zone-parity evidence, §3/§5).

### 6.2 The three named gap classes

| Class | Kind | Missions | Content | Sizing |
|-------|------|----------|---------|--------|
| G1 | episode-slot seam (semantic) | 10: all zones B–F missions 6–7 | **RESOLVED 2026-08-28** (`p5-select-shell-g1`, D183, RE-EXW-SIM §7j.73): the missions are MP-ONLY — no stage mask ever expressed them. The SELECT screen's MP write arm (0x43edc2..0x43ee43) writes `{zone 2..6, mission 1..2}` and `build_mission_paths` @0x4467df adds 5 at load → `ZONE{B..F}/MISSION{6,7}.*`. Landed as the sibling seam `GameHost::stage_select_mission` (the +5 = `SELECT_MP_FILE_OFFSET`); the census stages all ten through it (`select:clean` rows); the save-import mask domain widened to the EXW five-bit save/SELECT shape (`SELECT_FULL_MASK`, the restore tests bits 1/2/4/8/0x10 — the D178 loud bit-4 rejection retired) and `mission_number_for_mask` saturates at 5 (the SP SELECT domain — the campaign path can never name an MP file). | LANDED |
| G2 | critter family scope (semantic) | 13: ZONEC-M3, ZONEE M1–5, ZONEF M1–5, ZONEG-M1 | **RESOLVED 2026-08-28 (`p5-personnel-poi-s8`, D191, RE-EXW-SIM §7j.77) — ALL CLASSES LANDED, G2 EMPTY:** the .NME sections the controller does not model ran out — Wanderers (kind 1, §7j.71/D179), BallisticState6 (6, §7j.72/D182), Shooters (2, §7j.74/D185), Chasers (3, §7j.75/D186) and CloseCombat (7, §7j.76/D189) had already dropped their components from every row, and the S8 PERSONNEL/POI landing staged the last one: the separate 0x4dabdc bank (four POIs per 8-B record, the exact three-draw schedule — x/y in-tile scatter RandA&0x1F + heading RandA&7, the w1-level floor probe, state 1 IDLE, hp the literal 0x32 with NO m-scalar) + the controller subset (the z re-settle + nearest-exit prologue, the idle/settle/walk-out 1/16 machine with the robot aim, the flee/escape lane over the host-staged exit seam — the counter, the 0x32 panic cell, the elevator dwell reset, +5000 — and the panic tail) + the FUN_0040dc1b damage-lane seam. The §7j.18 seed list was corrected in the same unit (the +4/+6 store transposition: personnel spawn IDLE, not ESCAPE). The eleven hosting rows (ZONEE M1-5, ZONEF M1-5, ZONEG M1) all FLIPPED CLEAN — 37/37 load clean. | LANDED (D191) |
| G3 | zone-BIN variant naming (RESOLVED) | 3: ZONEB-MISSION6, ZONED-MISSION5, ZONEE-MISSION6 | **RESOLVED 2026-08-28 — NO SWAP (`p5-zone-bin-variant-g3`, D184, RE-EXW-SIM §7c.9):** the corpus ships mission-number terrain banks `ZONEB/MISSION6.BIN`, `ZONED/MISSION5.BIN`, `ZONEE/MISSION6.BIN` beside the zone-level `MISSION{L}.BIN`, but the EXW NEVER opens them — `build_mission_paths` @0x44670c builds path2 (the `.CGR/.BIN/.MIN/.LNG/.LNK` base) as `EDITOR\ZONE{L}\MISSION{L}` unconditionally (zone letter twice, no itoa, no conditional), both `.BIN` concat sites (load_mission @0x41dcbc + the brief twin FUN_0044661b @0x446644) sit on path2, a whole-image string census finds no other mission-path builder, and the EXD twin agrees. Data corroboration: only zone-level `.MIN` ship (16× the zone-BIN counts, never the variant counts); ZONEB M6 ≡ ZONED M5 byte-identical (a shared dev bank). Our `mission_asset_names` zone-level rule is VERIFIED CORRECT — engine untouched, census NOT re-pinned (the loads were already zone-level and green); the three variant files are editor-side residue. | CLOSED (no engine change) |

### 6.3 Per-mission table (the census output, pinned)

`load` = the load seam that staged the mission (host = episode-slot +
`load_mission`; select = the SELECT mission-choice seam
`stage_select_mission` + `load_mission`, §6.2/G1; direct =
`MissionScene::stage`, the defensive fallback no ledger row uses).
`destroy`/`pickup`/
`parsers`/`frames` are ok for ALL 37 rows (§6.1) and omitted; `critter
gap` names the refused .NME sections (G2). RE-PINNED 2026-08-27
(p5-critter-state-g2-wanderers, D179): the Wanderers landing dropped
the `WanderersxNN` component from every refusal row — the
`census_print_table` output at that commit is the provenance (the
test's PINNED table and this table updated together, deliberately;
no row flipped to clean: every Wanderers-hosting mission still
hosts another unmodeled state). RE-PINNED AGAIN 2026-08-28
(p5-critter-state-g2-ballistic6, D182): the BallisticState6 landing
(§7j.72 — kind 6 staged through the shared k5/6 mixed body, ONE each
and draw-free) dropped the BallisticState6xNN component from all 26
hosting rows; the provenance is
docs/evidence/p5-g2-ballistic6-census-table.txt; again no row
flipped to clean (every host still carries
Shooters/Chasers/CloseCombat/Personnel). RE-PINNED A THIRD TIME
2026-08-28 (p5-select-shell-g1, D183): the SELECT shell landing moved
the ten B-F missions 6/7 from the direct fallback to `select`
(`select:clean` — ten rows flipped clean, the §6.1 headline update);
the provenance is docs/evidence/p5-g1-select-census-table.txt.
RE-PINNED A FOURTH TIME 2026-08-28 (p5-critter-state-g2-shooters,
D185): the Shooters landing (§7j.74 — kind 2 staged through the new
sine-walk shooter body, hp 175+(175·m)/27, the ±2-tile scatter +
the map-bounds drop gate + the 5-draw spawn budget) dropped the
`ShootersxNN` component from all 17 hosting rows and FLIPPED
ZONED-MISSION5 CLEAN (its only unmodeled section — the queue's "no
row flips clean" expectation falsified by this one row, documented
+ deliberate); the provenance is
docs/evidence/p5-g2-shooters-census-table.txt. RE-PINNED A FIFTH
TIME 2026-08-28 (p5-critter-state-g2-chasers-r2, D186): the
Chasers landing (§7j.75 — kind 3 staged through the
distance-ladder body: the species triple role, the 4-rule ladder,
the 8-sector snap aim, the every-frame 0x67 fire, the walk table
[0,0,1,1,0,0,0,1,1,1], the wall-follow ladder, hp 1500+(1500·m)/27,
the whole chain draw-free) dropped the `ChasersxNN` component from
all 17 hosting rows and FLIPPED the twelve Chasers-ONLY hosts CLEAN
(ZONEB M1-5, ZONEC M1/M2/M4/M5, ZONED M1-4 — 24/37 load clean);
the provenance is
docs/evidence/p5-g2-chasers-census-table.txt. RE-PINNED A SIXTH
TIME 2026-08-28 (p5-critter-state-g2-closecombat, D189): the
CloseCombat landing (§7j.76 — kind 7 staged ACTIVE mode 3 with the
steer-aim-move engage, the two-conjunct 32/16/8 beam 0x69 fire, the
knock drift + ballistic landing machine, hp 2500+(2500·m)/27, the
engage chain draw-free) dropped the `CloseCombatxNN` component from
all 8 hosting rows and FLIPPED ZONEC-MISSION3 CLEAN (the one
CloseCombat-ONLY host — 25/37 load clean; the queue's "every host
also carries Personnel" expectation wrong for this row, documented
+ deliberate); the provenance is
docs/evidence/p5-g2-closecombat-census-table.txt. RE-PINNED A SEVENTH
TIME 2026-08-28 (p5-personnel-poi-s8, D191): the S8 PERSONNEL/POI
landing (§7j.77 — the poi bank + the controller subset, the separate
0x4dabdc bank with the exact three-draw schedule) dropped the
`PersonnelxNN` component from all 11 hosting rows and FLIPPED EVERY
ONE CLEAN — 37/37 load clean, G2 EMPTY; the provenance is
docs/evidence/p5-g2-personnel-census-table.txt.

| Mission | Dims | Load | Critter gap (refused sections) |
|---------|------|------|-------------------------------|
| ZONEA-MISSION1 | 25×75 | host | — (clean) |
| ZONEB-MISSION1 | 100×100 | host | — (clean — the Chasers landing §7j.75/D186) |
| ZONEB-MISSION2 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEB-MISSION3 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEB-MISSION4 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEB-MISSION5 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEB-MISSION6 | 100×100 | select | — (empty .NME; G3 resolved D184: variant BIN runtime-dead) |
| ZONEB-MISSION7 | 100×100 | select | — (empty .NME) |
| ZONEC-MISSION1 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEC-MISSION2 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEC-MISSION3 | 100×100 | host | — (clean — the CloseCombat landing §7j.76/D189) |
| ZONEC-MISSION4 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEC-MISSION5 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONEC-MISSION6 | 100×100 | select | — (empty .NME) |
| ZONEC-MISSION7 | 100×100 | select | — (empty .NME) |
| ZONED-MISSION1 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONED-MISSION2 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONED-MISSION3 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONED-MISSION4 | 100×100 | host | — (clean — §7j.75/D186) |
| ZONED-MISSION5 | 100×100 | host | — (clean — the Shooters landing §7j.74/D185; G3 resolved D184: variant BIN runtime-dead) |
| ZONED-MISSION6 | 100×100 | select | — (empty .NME) |
| ZONED-MISSION7 | 100×100 | select | — (empty .NME) |
| ZONEE-MISSION1 | 100×100 | host | — (clean — the S8 Personnel landing §7j.77/D191) |
| ZONEE-MISSION2 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEE-MISSION3 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEE-MISSION4 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEE-MISSION5 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEE-MISSION6 | 100×100 | select | — (empty .NME; G3 resolved D184: variant BIN runtime-dead) |
| ZONEE-MISSION7 | 100×100 | select | — (empty .NME) |
| ZONEF-MISSION1 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEF-MISSION2 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEF-MISSION3 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEF-MISSION4 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEF-MISSION5 | 100×100 | host | — (clean — §7j.77/D191) |
| ZONEF-MISSION6 | 100×100 | select | — (empty .NME) |
| ZONEF-MISSION7 | 100×100 | select | — (empty .NME) |
| ZONEG-MISSION1 | 100×25 | host | — (clean — §7j.77/D191) |

Dims cross-check: every TOT header matches the §2 zone table
(25×75 / 100×100 / 100×25) — a second, independent re-derivation of
the §2 size arithmetic (VERIFIED).

### 6.4 Per-zone rollup (zone-work sizing)

| Zone | Missions | Load | G1 | G2 critter states to model | G3 |
|------|----------|------|----|---------------------------|----|
| A | 1 | 1 host | 0 | none (clean) | — |
| B | 7 | 7 clean (5 host + 2 select) | landed (D183) | none — Wanderers landed (D179), Shooters LANDED (D185), Chasers LANDED (D186, M1-5) | resolved: no swap (D184) |
| C | 7 | 7 clean (5 host + 2 select) | landed (D183) | none — Wanderers landed (D179), Shooters LANDED (D185), Chasers LANDED (D186, M1/M2/M4/M5), CloseCombat LANDED (D189, M3) | — |
| D | 7 | 7 clean (5 host + 2 select) | landed (D183) | none — Wanderers landed (D179), Shooters LANDED (D185, M5), Chasers LANDED (D186, M1-4) | resolved: no swap (D184) |
| E | 7 | 7 clean (5 host + 2 select) | landed (D183) | none — Wanderers landed (D179), Shooters LANDED (D185), Chasers LANDED (D186), CloseCombat LANDED (D189), Personnel LANDED (D191) | resolved: no swap (D184) |
| F | 7 | 7 clean (5 host + 2 select) | landed (D183) | none — Wanderers landed (D179), Chasers LANDED (D186, M1), CloseCombat LANDED (D189, M1), Personnel LANDED (D191) | — |
| G | 1 | 1 host (clean) | 0 | none — Wanderers landed (D179), Shooters LANDED (D185), Chasers LANDED (D186), CloseCombat LANDED (D189), Personnel LANDED (D191) | — |

The load/parse layer needs NO work for any zone and NO mission
carries a gap: every G2 class LANDED (the S8 personnel/POI bank was
the last, D191) and the G1 SELECT shell LANDED (D183, §6.2 — the ten
MP missions stage through `stage_select_mission`). Zone parity work
continues per PLAN §6 as the per-zone DISPOSITION evidence (the §7
ZONEA pattern: criterion tables + ledger flips + gate wiring), not
loader work. The census test stays as the regression guard:
any loader change that flips a row fails
`census_matches_pinned_table` until deliberately re-baselined (the D28
fingerprint rule).

---

## 7. Zone A closure — ZONEA-MISSION1 green (D178)

**Provenance:** unit `p5-zonea-mission1-parity` (D178), run 2026-08-27
at HEAD `94d2c8b`+ (the RE-note commit; the evidence lands with the
flip). Zone A has exactly ONE mission (§2), so the zone closure and
the mission disposition are the same fact: `ZONEA-MISSION1` flips
`green` in `docs/P5-MISSION-LEDGER.toml` IN THE SAME COMMIT as this
evidence, and the `p5-zone-a` completion gate is wired into
`docs/required-gates.toml` (the §4 rule-5 cross-artifact check stays
green: zone A is fully green the moment the flip lands).

**The §1 criterion table, per criterion** (every command below is in
the `p5-zone-a` gate, executable offline under the validator's bwrap
containment):

| # | Criterion | Evidence (machine) | Status |
|---|-----------|--------------------|--------|
| 1 | scripted flows crash-free | `zonea_mission1_parity::zonea_scripted_flows_complete_crash_free`: every ZONEA-shaped S-scenario — S0 (boot→mission), S1 (400-frame passive), S2 (order→walk), S3 (weapon fire), S4 (destroy family), S6 (extraction), S7 (platform dynamics), S8 (critter engagement) — runs its full declared frame budget through the canonical runner; the W3 dump verifies; two runs byte-identical. The per-scenario CHAIN digests are additionally pinned in `canonical_dump_gate` (a gate command). S5/S5B/S5C are ZONEB's pickup corridors — zone B's evidence, not A's. | GREEN |
| 2 | T1 rules vs RE/8street | The deep oracle suites run as gate commands: `mission_corpus_gate` (loader/climb/order/snap rules on the real ZONEA bytes), plus the spot table `zonea_t1_rules_spot` (the B2 @0x81d9a FULL_MASK table, the first-unset-bit mission selection, the §7j.64/C economy seed 4000−500·d, the 25-name fetch chain) and `zonea_structural_spot_check` (the anchor TS statics re-derived INDEPENDENTLY from the TOT header bytes + the §7j.64/D154 fresh-campaign scalars). The full D145–D164 static differential set remains the P4 `s0-dispositions` gate (all-RE oracles; unchanged). | GREEN |
| 3 | perceptual frame checks at key moments (T2) | Diagnostic band per §0b: thresholds + owner feel sign-off, never pixel-exact gates. Machine stand-ins at the key moments: `mission_scene_gate` spawn/mid-walk frame-hash pins + the GAMEPAL fold pins (palette identity, 254/256 non-black) — a gate command. Owner feel sign-off stays the operator diagnostic process (not machine-checkable, not a gate). | GREEN (machine band) — sign-off tracked as operator diagnostic |
| 4 | differ structural spot-check | `differ_gate` (the cross-channel differ on the real S0/S1 dumps: PASS-WITH-NOTES with exactly the budgeted findings, zero structural findings) + `zonea_mission1_parity::zonea_structural_spot_check` (structural contract: anchor frame, monotone frame_no, record count = declared budget + 1). Not tick-complete by design (§0b). | GREEN |
| 5 | cross-OS replay hash equality (OUR engine) | The replay-hash fixtures run as gate commands: `hash_fixture` (600-tick fixed script, 13 milestone StateHash pins + the FNV-1a chain `0x760d221bec3b3b99`) + the `determinism` suites (15/60/240 Hz rate invariance + pure-FSM replay identity) + `zonea_replay_stitch_is_stable`. Verified this run on TWO TOOLCHAINS (stable + nightly, identical pins). The cross-OS channel is the ubuntu+windows CI matrix (`cargo test --workspace`, `.github/workflows/ci.yml`) — REPAIRED GREEN (D181 `ci-cross-os-repair`, run 33123147228 at a168d69: ubuntu+windows matrix legs + miri + diffharness ALL green after >=100 consecutive red-for-environment runs; the windows leg now runs and passes the determinism/replay suites on MSVC). The repair itself validated the channel immediately: the first-ever windows test run caught a REAL cross-profile engine bug — the debris seq-table pointer-identity fallback (all kinds walked table 0 in every release build; the canonical S4 pin had encoded it; re-pinned deliberately, see D181) — plus the CRLF artifact-compare class (fixed by the repo-wide LF checkout policy) — never a determinism finding of the hashed state itself: the hashed state is integer-only, little-endian by format contract, float-free (Miri-clean), so the pinned chains are OS-invariant by construction with CI as the enforcement channel. | GREEN (fixtures + cross-toolchain + CI matrix) |
| 6 | original SAVED/OPTIONS.BDL import read-only, bounds-checked, fuzzed | RE-EXW-SIM §7j.70 pins the restore grammar EXW-side (this was 8street-cited before). Engine: `bedlam-game save.rs` — the bounds-checked header walk (exact 900 B, slot < 5, the EXW zero-dword@+0x0C empty predicate, zone/mask inside the modeled episode space, never guess) + `GameHost::import_saved_slot` staging through the D51 seam; money/score/difficulty RETURNED (sim-side), nothing ever written back. Tests: the real shipped files (slot 0 = "PLAYER"/zone 2/mask 0/money 580/difficulty 1 → stages ZONEB-MISSION1; four EMPTY slots rejected loud; OPTIONS 41 B → volume 75, name "Player") + bounded deterministic fuzz (full header bit-flip sweep per slot, truncations, size attacks, random images; OPTIONS bit-flip sweep through the typed view) — Ok/Err only, never a panic. | GREEN |
| 7 | DM carve-out | A carve-out, not a check: deathmatch is MODE-level (the same maps under netplay; ZONEA-MISSION1 hosts no DM-only content — the corpus has no DM map variant, FORMATS-MISSION §0). The carve-out legs that ARE checked: the map loads (criterion 1's flows stage it) and local SP semantics are correct (criteria 1-2). Full DM/netplay = future work, out of the parity exit. | NOTED |

**Ledger:** `ZONEA-MISSION1` → `green`, `catalog_refs = []` (no
original-behavior divergences observed on this mission; a green
mission may legitimately carry zero refs, §3). Zone A status derives
green (1/1).

**Gate wiring:** `p5-zone-a` joins P5's `required_gates` (after
`p5-zone-gate-scaffold`) carrying the zone's evidence commands:

1. `/usr/bin/cargo test --release --locked --offline -p bedlam-game
   --test zonea_mission1_parity --test canonical_dump_gate --test
   differ_gate --test mission_scene_gate --test determinism`
2. `/usr/bin/cargo test --release --locked --offline -p bedlam-core
   --test hash_fixture --test mission_corpus_gate`

P5 stays `pending` (1/37 missions green — the zone-parity ledger;
37/37 LOAD clean per the §6 census after the S8 Personnel landing —
B–G open on the DISPOSITION side only: every G2 class LANDED (G1
2026-08-28 D183, Shooters D185, Chasers D186, CloseCombat D189,
Personnel D191 — G2 EMPTY); the per-zone parity evidence is the
queued unit family).
