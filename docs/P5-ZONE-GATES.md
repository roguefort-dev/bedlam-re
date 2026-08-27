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
HEAD `6355fba`. Every one of the 37 ledger missions was driven through
OUR engine load seams READ-ONLY from `game-data/BEDLAM`:
`GameHost::stage_episode_slot` + `GameHost::load_mission` (the
canonical mission-view load seam: Terrain + AngleTable +
`MissionView::from_mission_bytes` + MapOverlay + MRK spawns) where the
episode slot reaches the mission, `MissionScene::stage` + the claim
bank directly where it cannot; then the destroy family
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
are the census verdict (D176); G3's override rule is LIKELY
(corpus + RESEARCH-8STREET §3 — the 8street reference is NOT evidence
until re-anchored to EXW, per the 8street policy).

### 6.1 Headline

**ALL 37 missions LOAD through our engine.** Zero load failures, zero
parser refusals, zero frame-run panics. The destroy family, the pickup
surface and every parser accept all 37 missions' runtime file
families. The only mission that loads with ZERO gaps is
`ZONEA-MISSION1` — exactly the mission the S0–S8 canonical corpus
exercises. Every other mission loads with named gaps, all of them
SEMANTIC (engine scope), none parser-sized. The ledger is therefore
UNCHANGED: no mission is unloadable-by-corpus, so no disposition
moves (dispositions flip only on zone-parity evidence, §3/§5).

### 6.2 The three named gap classes

| Class | Kind | Missions | Content | Sizing |
|-------|------|----------|---------|--------|
| G1 | episode-slot seam (semantic) | 10: all zones B–F missions 6–7 | `GameHost::stage_episode_slot` cannot stage them: `FULL_MASK` pins FOUR sub-slots per stage (B2 @0x81d9a, RE-pinned; mask ⊆ 0b1111), so a slot can derive missions 1–5 only. The census staged them DIRECTLY (`MissionScene::stage` + claim bank — the `load_mission` body verbatim); they load and run clean. The fix is the SELECT mission-choice shell (the original's sub-mission picker), its own unit. | one shell-modeling unit + wiring |
| G2 | critter family scope (semantic) | 26: zones B–F missions 1–5 (25) + ZONEG-MISSION1 | `.NME` hosts critter sections the controller does not model (`stage_critters` accepts MixedState5 + SeekSteppers only, §7j.42/6): the refusals name Shooters (state 2), Wanderers (1), Chasers (3), BallisticState6, CloseCombat (7), and the personnel/POI bank (S8). ZONEA-MISSION1 passes (MixedState5x6 + SeekSteppersx5 — the modeled slice); the ten 16-byte all-zero .NME missions (all B–F missions 6/7) pass trivially. Not parser-sized: each critter state is AI modeling, its own unit(s). | per-state units; per-mission counts in §6.3 |
| G3 | zone-BIN variant naming (RE open) | 3: ZONEB-MISSION6, ZONED-MISSION5, ZONEE-MISSION6 | The corpus ships mission-number terrain banks `ZONEB/MISSION6.BIN`, `ZONED/MISSION5.BIN`, `ZONEE/MISSION6.BIN` beside the zone-level `MISSION{L}.BIN`; our fetch always builds `MISSION{L}.BIN` (`mission_asset_names`). The census loaded those missions with the zone-level bank (loads + frames clean). The override rule is the open RESEARCH-8STREET §3 question — unresolved against EXW; if the original swaps the bank per mission, our terrain sprites for those three missions are wrong until then. Its own RE unit. | one RE unit (EXW-anchored) |

### 6.3 Per-mission table (the census output, pinned)

`load` = the load seam that staged the mission (host = episode-slot +
`load_mission`; direct = `MissionScene::stage`, G1). `destroy`/`pickup`/
`parsers`/`frames` are ok for ALL 37 rows (§6.1) and omitted; `critter
gap` names the refused .NME sections (G2).

| Mission | Dims | Load | Critter gap (refused sections) |
|---------|------|------|-------------------------------|
| ZONEA-MISSION1 | 25×75 | host | — (clean) |
| ZONEB-MISSION1 | 100×100 | host | Wanderersx24, Chasersx10, BallisticState6x9 |
| ZONEB-MISSION2 | 100×100 | host | Shootersx3, Wanderersx22, Chasersx6, BallisticState6x5 |
| ZONEB-MISSION3 | 100×100 | host | Wanderersx18, Chasersx7, BallisticState6x12 |
| ZONEB-MISSION4 | 100×100 | host | Shootersx1, Wanderersx13, Chasersx12, BallisticState6x21 |
| ZONEB-MISSION5 | 100×100 | host | Shootersx1, Wanderersx28, Chasersx16, BallisticState6x12 |
| ZONEB-MISSION6 | 100×100 | direct (G1) | — (empty .NME; G3: MISSION6.BIN variant) |
| ZONEB-MISSION7 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEC-MISSION1 | 100×100 | host | Shootersx1, Wanderersx13, Chasersx10, BallisticState6x13 |
| ZONEC-MISSION2 | 100×100 | host | Wanderersx22, Chasersx13, BallisticState6x13 |
| ZONEC-MISSION3 | 100×100 | host | Shootersx4, Wanderersx18, Chasersx9, BallisticState6x21, CloseCombatx4 |
| ZONEC-MISSION4 | 100×100 | host | Wanderersx19, Chasersx15, BallisticState6x23 |
| ZONEC-MISSION5 | 100×100 | host | Shootersx1, Wanderersx12, Chasersx2, BallisticState6x22 |
| ZONEC-MISSION6 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEC-MISSION7 | 100×100 | direct (G1) | — (empty .NME) |
| ZONED-MISSION1 | 100×100 | host | Shootersx4, Wanderersx33, Chasersx9, BallisticState6x18 |
| ZONED-MISSION2 | 100×100 | host | Shootersx8, Wanderersx20, Chasersx7, BallisticState6x9 |
| ZONED-MISSION3 | 100×100 | host | Shootersx8, Wanderersx2, Chasersx4, BallisticState6x21 |
| ZONED-MISSION4 | 100×100 | host | Shootersx8, Wanderersx2, Chasersx4, BallisticState6x16 |
| ZONED-MISSION5 | 100×100 | host | Shootersx4, Wanderersx12, BallisticState6x17 |
| ZONED-MISSION6 | 100×100 | direct (G1) | — (empty .NME) |
| ZONED-MISSION7 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEE-MISSION1 | 100×100 | host | Shootersx4, Wanderersx18, Chasersx6, BallisticState6x17, CloseCombatx5, Personnelx12 |
| ZONEE-MISSION2 | 100×100 | host | Shootersx1, Wanderersx34, Chasersx5, BallisticState6x2, CloseCombatx5, Personnelx12 |
| ZONEE-MISSION3 | 100×100 | host | Shootersx3, Wanderersx28, Chasersx5, BallisticState6x11, CloseCombatx6, Personnelx12 |
| ZONEE-MISSION4 | 100×100 | host | Shootersx4, Wanderersx23, Chasersx8, BallisticState6x8, CloseCombatx8, Personnelx12 |
| ZONEE-MISSION5 | 100×100 | host | Shootersx5, Wanderersx27, Chasersx13, BallisticState6x5, CloseCombatx4, Personnelx13 |
| ZONEE-MISSION6 | 100×100 | direct (G1) | — (empty .NME; G3: MISSION6.BIN variant) |
| ZONEE-MISSION7 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEF-MISSION1 | 100×100 | host | Wanderersx12, Chasersx3, BallisticState6x43, CloseCombatx4, Personnelx9 |
| ZONEF-MISSION2 | 100×100 | host | Wanderersx28, BallisticState6x12, Personnelx9 |
| ZONEF-MISSION3 | 100×100 | host | Wanderersx24, BallisticState6x16, Personnelx9 |
| ZONEF-MISSION4 | 100×100 | host | Wanderersx11, BallisticState6x17, Personnelx9 |
| ZONEF-MISSION5 | 100×100 | host | Wanderersx42, BallisticState6x53, Personnelx19 |
| ZONEF-MISSION6 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEF-MISSION7 | 100×100 | direct (G1) | — (empty .NME) |
| ZONEG-MISSION1 | 100×25 | host | Shootersx3, Wanderersx20, Chasersx23, BallisticState6x18, CloseCombatx6, Personnelx9 |

Dims cross-check: every TOT header matches the §2 zone table
(25×75 / 100×100 / 100×25) — a second, independent re-derivation of
the §2 size arithmetic (VERIFIED).

### 6.4 Per-zone rollup (zone-work sizing)

| Zone | Missions | Load | G1 | G2 critter states to model | G3 |
|------|----------|------|----|---------------------------|----|
| A | 1 | 1 host | 0 | none (clean) | — |
| B | 7 | 5 host + 2 direct | 2 | Shooters (M2,M4,M5), Wanderers, Chasers, BallisticState6 | MISSION6.BIN |
| C | 7 | 5 host + 2 direct | 2 | Shooters (M1,M3,M5), Wanderers, Chasers, BallisticState6, CloseCombat (M3) | — |
| D | 7 | 5 host + 2 direct | 2 | Shooters, Wanderers, Chasers, BallisticState6 | MISSION5.BIN |
| E | 7 | 5 host + 2 direct | 2 | Shooters, Wanderers, Chasers, BallisticState6, CloseCombat, Personnel | MISSION6.BIN |
| F | 7 | 5 host + 2 direct | 2 | Wanderers, Chasers (M1), BallisticState6, CloseCombat (M1), Personnel | — |
| G | 1 | 1 host | 0 | Shooters, Wanderers, Chasers, BallisticState6, CloseCombat, Personnel | — |

The load/parse layer needs NO work for any zone: zone parity work is
the G1 SELECT shell, the G2 critter states (+ the S8 personnel/POI
bank), and the G3 BIN-variant RE — all queued as their own units
(`.state/NEXT.md`). The census test stays as the regression guard:
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
| 5 | cross-OS replay hash equality (OUR engine) | The replay-hash fixtures run as gate commands: `hash_fixture` (600-tick fixed script, 13 milestone StateHash pins + the FNV-1a chain `0x760d221bec3b3b99`) + the `determinism` suites (15/60/240 Hz rate invariance + pure-FSM replay identity) + `zonea_replay_stitch_is_stable`. Verified this run on TWO TOOLCHAINS (stable + nightly, identical pins). The cross-OS channel is the ubuntu+windows CI matrix (`cargo test --workspace`, `.github/workflows/ci.yml`) — currently RED repo-wide for ENVIRONMENT reasons predating this unit (alsa-sys needs libasound2-dev on ubuntu; the miri job trips file-isolation on a corpus-gated suite; ≥100 consecutive runs, every failure before any test executes) — a queued CI-repair unit, NOT a determinism finding: the hashed state is integer-only, little-endian by format contract, float-free (Miri-clean), so the pinned chains are OS-invariant by construction with CI as the enforcement channel. | GREEN (fixtures + cross-toolchain); CI channel repair queued |
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

P5 stays `pending` (1/37 missions green; B–G open — the G1/G2/G3 gap
classes of §6.2 are their queued units).
