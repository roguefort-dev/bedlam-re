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
1. [READY] [id=p5-zone-f-disposition] [gate=p5-zone-f-disposition] P5
   follow-up — ZONE E is CLOSED (29/37 green, D196) and the
   disposition family continues through the CLOSED-ZONE LIST (9410e0d
   + 9190a87: the ZONES const in
   engine/bedlam-game/tests/zone_mission_parity.rs — a zone's unit
   APPENDS its spec, never re-instantiates the const, so no closed
   zone's gate evidence is ever stranded): ZONEF next, a pure
   ZONES-append unit in the §11 shape. (a) LAND the ZONEF evidence:
   APPEND the ZONEF ZoneSpec (letter F, missions 1-7, dims 100x100
   — all seven TOT headers re-verified 160004 B, committed flows
   NONE — no committed .scen stages zone F, so the generated
   per-mission battery IS the whole criterion-1 leg; M1-5 through
   the episode-slot mask seam, M6-7 through the SELECT seam; linear
   = clamp(5·(6−2)+m−1, 1, 26) = m+19, M1..M7 = 20..26 — M7 exactly
   touches the clamp ceiling 26, no bite; zone F ships NO
   mission-number variant bank at all — only the zone-level
   MISSIONF.BIN 1464679 B, so NO G3 no-swap extra leg, just the
   zone-level fetch-chain assert), document the §12 criterion table
   in P5-ZONE-GATES (the §11 pattern), flip ZONEF-MISSION1..MISSION7
   to green WITH the cross-artifact rule (the flip and its evidence
   in the same commit), re-baseline the ledger test pin 29/37 to
   36/37 in the SAME commit (the D28 fingerprint discipline,
   D192/D193/D195/D196 precedent), and wire the p5-zone-f gate into
   docs/required-gates.toml P5 required_gates (offline evidence
   commands only; zone F stages no committed flow so the gate
   carries no scenario corpus). Bounds: bedlam-core + bedlam-game
   suites green; the census stays 37/37; zero canonical chain
   movement; fmt + clippy; the gates validator all-green (15 gates;
   .state/STATE.md is an s0-dispositions tracked path — park
   uncommitted STATE.md edits while running the HEAD-bound battery,
   the D193/D194 lesson); MANIFEST clean; no Ghidra run;
   Nudge-Worker trailer. If the battery surfaces a REAL engine gap
   on a ZONEF mission, stop at the structured finding — the gap
   becomes its own unit, the ledger stays unchanged, and the failure
   artifact records it. After ZONEF: ZONEG closes the ledger.
## Done
1. DONE (2026-08-28, claim 1 — substantive commits 9410e0d + 9190a87
   by worker e9edfba4 which died waiting on its background battery;
   claim re-acquired, independently re-validated, battery re-run and
   bookkeeping/push by worker 224613cc): P5 `p5-zone-e-disposition`
   — ZONE E CLOSED: the FOURTH 7-mission zone flips green (the
   ledger 29/37; D196) and the disposition is the SECOND PURE
   ZONES-APPEND (the §10/§11 shape). (a) THE APPEND (9410e0d): the
   ZONEE ZoneSpec joined the ZONES list after B, C, D (letter E,
   missions 1..=7, dims 100x100, committed flows NONE) and nothing
   else; the battery: P5EM1A..P5EM7C all 21 flows full declared
   budgets, dumps verify, two-run byte identity — NO engine gap on
   any ZONEE mission; zones B (21 + S5/S5B/S5C), C (21), D (21)
   re-verified in place. (b) THE FLIP (9190a87, the cross-artifact
   rule): ledger ZONEE-MISSION1..7 green (catalog_refs = []);
   P5-ZONE-GATES §11 criterion table (linear = m+14, M1..M7 =
   15..21; the D184 zone-level EDITOR\ZONEE\MISSIONE.* pin; the
   THIRD G3 variant ZONEE/MISSION6.BIN 1508806 B sha256-DISTINCT —
   its OWN bank, not the ZONEB twin ZONED/MISSION5.BIN is —
   runtime-dead editor residue, no extra leg); the p5-zone-e gate
   joins P5 required_gates (14 gates); the ledger test pin
   re-baselined 22/37 to 29/37 + the ZONEE 7/7 line; D196.
   Re-validated first-hand by the finishing session: ledger OK
   29/37, hermetic 18/18, gates-validator suite 22/22, bedlam-game
   suites green (canonical_dump_gate 13/13 zero chain movement,
   differ 4/4, determinism, census 37/37), bedlam-core, diffharness
   104/0, fmt + clippy, the HEAD-bound battery ALL 14 GATES PASSED
   at 9190a87, MANIFEST clean before and after every corpus read,
   no Ghidra run. Queued: the ZONEF disposition unit as the new
   head; after it ZONEG closes the ledger.
