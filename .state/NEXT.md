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
1. [READY] [id=p5-zone-e-disposition] [gate=p5-zone-e-disposition] P5
   follow-up — ZONE D is CLOSED (22/37 green, D195) and the
   disposition family continues through the CLOSED-ZONE LIST
   (681db03 + 6c14940: the ZONES const in
   engine/bedlam-game/tests/zone_mission_parity.rs — a zone's unit
   APPENDS its spec, never re-instantiates the const, so no closed
   zone's gate evidence is ever stranded): ZONEE next, a pure
   ZONES-append unit in the §10 shape. (a) LAND the ZONEE evidence:
   APPEND the ZONEE ZoneSpec (letter E, missions 1-7, dims 100x100
   — all seven TOT headers re-verified 160004 B, committed flows
   NONE — no committed .scen stages zone E, so the generated
   per-mission battery IS the whole criterion-1 leg; M1-5 through
   the episode-slot mask seam, M6-7 through the SELECT seam; linear
   = clamp(5·(5−2)+m−1, 1, 26) = m+14, M1..M7 = 15..21 no clamp
   bite; M6 carries the THIRD G3 variant bank ZONEE/MISSION6.BIN —
   runtime-dead editor residue, the D184 no-swap pin, no extra leg
   beyond the zone-level fetch-chain assert), document the §11
   criterion table in P5-ZONE-GATES (the §10 pattern), flip
   ZONEE-MISSION1..MISSION7 to green WITH the cross-artifact rule
   (the flip and its evidence in the same commit), re-baseline the
   ledger test pin 22/37 to 29/37 in the SAME commit (the D28
   fingerprint discipline, D192/D193/D195 precedent), and wire the
   p5-zone-e gate into docs/required-gates.toml P5 required_gates
   (offline evidence commands only; zone E stages no committed flow
   so the gate carries no scenario corpus). Bounds: bedlam-core +
   bedlam-game suites green; the census stays 37/37; zero canonical
   chain movement; fmt + clippy; the gates validator all-green (14
   gates; .state/STATE.md is an s0-dispositions tracked path — park
   uncommitted STATE.md edits while running the HEAD-bound battery,
   the D193/D194 lesson); MANIFEST clean; no Ghidra run;
   Nudge-Worker trailer. If the battery surfaces a REAL engine gap
   on a ZONEE mission, stop at the structured finding — the gap
   becomes its own unit, the ledger stays unchanged, and the failure
   artifact records it. After ZONEE: ZONEF, then ZONEG close the
   ledger.
## Done
1. DONE (2026-08-28, worker 34b13b42 claim 1, commits 681db03 +
   6c14940 + this bookkeeping commit, all PUSHED): P5
   `p5-zone-d-disposition` — ZONE D CLOSED: the THIRD 7-mission
   zone flips green (the ledger 22/37; D195) and the disposition
   lands as the FIRST PURE ZONES-APPEND (the shape E/F/G take — no
   harness, grammar, or engine change). (a) THE APPEND (681db03):
   the ZONED ZoneSpec joined the `ZONES: &[ZoneSpec]` list in
   engine/bedlam-game/tests/zone_mission_parity.rs (letter D,
   missions 1..=7, dims 100x100, committed flows NONE — verified no
   committed .scen stages zone D) and nothing else; the battery run
   at that state: P5DM1A..P5DM7C all 21 flows full declared
   budgets, dumps verify, two-run byte identity — NO engine gap
   surfaced on any ZONED mission (the stopping condition did not
   trigger); zones B (21 + S5/S5B/S5C) and C (21) re-verified in
   place. (b) THE FLIP (6c14940, the cross-artifact rule): the
   ledger flips ZONED-MISSION1..7 green (catalog_refs = []); §10
   documents the per-criterion table — the anchor statics
   re-derived from each TOT header (100x100, 160004 B — all seven)
   + the §7j.64 formula (zone D: linear = clamp(5·(4−2)+m−1, 1, 26)
   = m+9, M1..M7 = 10..16), the T1 spot table (FULL_MASK arithmetic,
   start_score, the 25-name fetch chain with the D184 zone-level
   EDITOR\ZONED\MISSIOND.* pin — the G3 variant ZONED/MISSION5.BIN
   re-verified byte-identical to ZONEB/MISSION6.BIN, runtime-dead
   editor residue, NO extra leg — the seam domains at stage/zone
   cell 4 incl. campaign-clears-select), criterion 6 FILE-LEVEL
   (the shipped slot-0 campaign IS ZONEB/MISSION1), the load-bearing
   DM carve-out on M6/M7; the p5-zone-d gate joins P5
   required_gates (13 gates); the ledger test pin re-baselined 15/37
   to 22/37 + the ZONED 7/7 line (deliberate, same commit); D195.
   Verified: zone_mission_parity 5/5 (three zones, 16.24s),
   bedlam-game suites green (canonical_dump_gate 13/13 — ZERO
   canonical chain movement; differ_gate 4/4; determinism; census
   37/37 unchanged), bedlam-core 201/0, diffharness 104/0, fmt +
   clippy clean (the bedlam-core claim-bank test warnings pre-exist
   from D151, untouched), check-p5-zone-ledger OK 22/37 + hermetic
   suite 18/18, gates-validator suite 22/22 + the HEAD-bound
   battery all-green at 6c14940 (13 gates), MANIFEST clean before
   AND after every corpus read, no Ghidra run. Queued: the ZONEE
   disposition unit as the new head; after it ZONEF, ZONEG close
   the ledger.
