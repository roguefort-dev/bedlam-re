# NEXT - task queue (top first; rewrite this file at end of every run)

QUEUE CONVENTION (2026-08-22, D106): a completed unit's entry MOVES to
the '## Done' log at end of run - never stays in '## Now' as 'N. DONE ...'
(the scheduler mechanically skips a first-word DONE marker, but the
renumbered queue keeps every open item claimable by number).
AUTHORING RULE (D180, second recurrence after D177): every
[status]/[id]/[gate]/[probe]/[retry] metadata tag stays WHOLE on the
item's first numbered line, prose starting same-line after the tags —
never wrap INSIDE a tag; the strict parser rejects it (rc=2,
INVALID-DEADLOCKED) and the worker dies at its own finish line.
## Now
1. [READY] [id=p5-zone-c-disposition] [gate=p5-zone-c-disposition] P5
   follow-up — ZONE B is CLOSED (8/37 green, D192) and the disposition
   family continues through the GENERALIZED harness (2980e8b: grammar
   v1.8 mission key + the ZoneSpec-parameterized suite): ZONEC next, a
   pure instantiation unit. (a) LAND the ZONEC evidence: instantiate
   the ZoneSpec const in engine/bedlam-game/tests/zone_mission_parity.rs
   for zone C (letter C, missions 1-7, dims 100x100, committed flows
   NONE — no committed .scen stages zone C, so the generated per-mission
   battery IS the whole criterion-1 leg; M1-5 through the episode-slot
   mask seam, M6-7 through the SELECT seam), document the §9 criterion
   table in P5-ZONE-GATES (new section, the §8 pattern), flip
   ZONEC-MISSION1..MISSION7 to green WITH the cross-artifact rule (the
   flip and its evidence in the same commit), re-baseline the ledger
   test pin 8/37 to 15/37 in the SAME commit (the D28 fingerprint
   discipline, D192 precedent), and wire the p5-zone-c gate into
   docs/required-gates.toml P5 required_gates (offline evidence
   commands only). Bounds: bedlam-core + bedlam-game suites green; the
   census stays 37/37; zero canonical chain movement; fmt + clippy;
   the gates validator all-green (24 gates); MANIFEST clean; no Ghidra
   run; Nudge-Worker trailer. If the battery surfaces a REAL engine gap
   on a ZONEC mission, stop at the structured finding — the gap becomes
   its own unit, the ledger stays unchanged, and the failure artifact
   records it.
## Done
1. DONE (2026-08-28, worker 2a33d196 claim 1, commits 2980e8b +
   6ba8aae + this bookkeeping commit, all PUSHED): P5
   `p5-zone-b-disposition` — ZONE B CLOSED: the FIRST 7-mission zone
   flips green (the ledger 8/37; D192). (a) GENERALIZE (2980e8b): the
   D178 ZONEA parity shape lifted to a per-zone/per-mission
   PARAMETERIZED suite — engine/bedlam-game/tests/zone_mission_parity.rs
   (ZoneSpec: letter, mission range, committed flows, TOT dims) — so
   the P5-ZONE-GATES §1 criterion table is executable for ANY ledger
   mission; the scenario grammar gained the v1.8 `mission = 1..=7`
   header key (requires `zone`; 6..7 pinned to zones B..F — the SELECT
   write-arm domain; fail-loud range/duplicate/pairing gates + parser
   tests + the dbx-plan `_e_staging` seam note), and the canonical
   runner's staging arm selects the seam per the §7j.73 finding:
   campaigns 1..=5 through the episode slot at the completion mask
   whose first-uncompleted sub is exactly the mission (mask
   (1<<(m-1))-1), the MP files 6..=7 through the SELECT write pair
   ALONE (stage_select_mission — campaign staging would clear it).
   (b) LAND THE ZONEB EVIDENCE (6ba8aae, the cross-artifact rule): all
   seven missions' battery — boot + 120-frame passive + 48-frame
   full-staging (destroy+pickup+platforms+critters) — full declared
   budgets, dumps verify, two-run byte identity; the committed ZONEB
   flows S5/S5B/S5C re-run; the anchor TS/T0 statics re-derived from
   each TOT header (100x100, 160004 B) + the §7j.64 formula (linear =
   clamp(n-1,1,26) — M1/M2 both floor at 1); the T1 spot table
   (FULL_MASK arithmetic, start_score, the per-mission 25-name fetch
   chain with the D184 zone-level CGR/BIN/LNK pin, the seam domains
   incl. campaign-clears-select); the shipped SAVED.BDL slot-0 campaign
   = ZONEB/MISSION1 import + bounded fuzz; the DM carve-out
   LOAD-BEARING on M6/M7 (the MP-only files: maps load through the
   SELECT seam + local SP semantics re-derived). P5-ZONE-GATES §8
   documents the per-criterion table; the ledger flips
   ZONEB-MISSION1..7 green (catalog_refs = []); the p5-zone-b gate
   joins P5 required_gates; the ledger test pin re-baselined 1/37 to
   8/37 + the ZONEB 7/7 line (deliberate, same commit); D192. Verified:
   bedlam-game 21 suites green (canonical_dump_gate 13/13 — ZERO
   canonical chain movement; differ_gate 4/4; determinism; census 37/37
   unchanged; zonea_mission1_parity 6/6; zone_mission_parity 5/5),
   bedlam-core + diffharness green incl. the new parser tests, fmt +
   clippy clean on the touched crates, check-p5-zone-ledger OK +
   hermetic suite 18/18, the HEAD-bound gates validator battery green
   at 6ba8aae, inspect baseline ok (1069 files), MANIFEST clean before
   AND after every corpus read, no Ghidra run. Queued: the ZONEC
   disposition unit as the new head (the pure ZoneSpec instantiation;
   after it ZONED, ZONEE, ZONEF, ZONEG close the ledger).
