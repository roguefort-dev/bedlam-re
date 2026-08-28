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
1. [READY] [id=p5-zone-d-disposition] [gate=p5-zone-d-disposition] P5
   follow-up — ZONE C is CLOSED (15/37 green, D193) and the
   disposition family continues through the CLOSED-ZONE LIST (f4ab798:
   the ZONES (ZoneSpec-slice) array in
   engine/bedlam-game/tests/zone_mission_parity.rs — a zone's unit
   APPENDS its spec, never re-instantiates the const, so no closed
   zone's gate evidence is ever stranded): ZONED next, a pure
   ZONES-append unit in the §9 shape. (a) LAND the ZONED evidence:
   APPEND the ZONED ZoneSpec (letter D, missions 1-7, dims 100x100,
   committed flows NONE — no committed .scen stages zone D, so the
   generated per-mission battery IS the whole criterion-1 leg; M1-5
   through the episode-slot mask seam, M6-7 through the SELECT seam;
   linear = clamp(5·(4−2)+m−1, 1, 26) = m+9; M5 carries the G3
   variant BIN ZONED/MISSION5.BIN — runtime-dead editor residue, the
   D184 no-swap pin, no extra leg beyond the zone-level fetch-chain
   assert), document the §10 criterion table in P5-ZONE-GATES (the §9
   pattern), flip ZONED-MISSION1..MISSION7 to green WITH the
   cross-artifact rule (the flip and its evidence in the same commit),
   re-baseline the ledger test pin 15/37 to 22/37 in the SAME commit
   (the D28 fingerprint discipline, D192/D193 precedent), and wire the
   p5-zone-d gate into docs/required-gates.toml P5 required_gates
   (offline evidence commands only; zone D stages no committed flow so
   the gate carries no scenario corpus). Bounds: bedlam-core +
   bedlam-game suites green; the census stays 37/37; zero canonical
   chain movement; fmt + clippy; the gates validator all-green (13
   gates; note .state/STATE.md is an s0-dispositions tracked path —
   park uncommitted STATE.md edits while running the HEAD-bound
   battery, the D193 lesson); MANIFEST clean; no Ghidra run;
   Nudge-Worker trailer. If the battery surfaces a REAL engine gap on
   a ZONED mission, stop at the structured finding — the gap becomes
   its own unit, the ledger stays unchanged, and the failure artifact
   records it.
## Done
1. DONE (2026-08-28, worker 4016c154 claim 1, commits f4ab798 +
   dcfdcc8 + this bookkeeping commit, all PUSHED): P5
   `p5-zone-c-disposition` — ZONE C CLOSED: the SECOND 7-mission
   zone flips green (the ledger 15/37; D193) and the harness carries
   the CLOSED-ZONE LIST. (a) HARNESS SHAPE (f4ab798): the D192
   single-ZONE-const suite lifted to ZONES: &[ZoneSpec] (B then C) so
   a zone's disposition unit APPENDS its spec and the closed set
   never loses its executable evidence — a per-zone const
   re-instantiation would strand every earlier p5-zone-{b..} gate on
   a suite that no longer exercises its zone; both gates run the same
   command over the same file. (b) LAND THE ZONEC EVIDENCE (dcfdcc8,
   the cross-artifact rule): the FIRST PURE instantiation — zone C
   ships NO committed .scen flows (scenarios-tree grep verified), so
   the generated per-mission battery IS the whole criterion-1 leg:
   all 21 flows (P5CM1A..P5CM7C — boot, 120-frame passive, 48-frame
   full-staging destroy+pickup+platforms+critters) full declared
   budgets, dumps verify, two-run byte identity — NO engine gap
   surfaced on any ZONEC mission (the stopping condition did not
   trigger); the anchor TS/T0 statics re-derived from each TOT
   header (100x100, 160004 B — all seven) + the §7j.64 formula (zone
   C: linear = m+4); the T1 spot table per zone (FULL_MASK
   arithmetic, start_score, the 25-name fetch chain with the D184
   zone-level CGR/BIN/LNK pin EDITOR\ZONEC\MISSIONC.*, the seam
   domains at stage/zone-cell 3 incl. campaign-clears-select); the
   criterion-6 SAVED/OPTIONS import tests stay FILE-LEVEL (the
   shipped slot-0 campaign IS ZONEB/MISSION1 — hardcoded to zone B;
   the zone-C campaign staging rides criterion 2's seam legs).
   P5-ZONE-GATES §9 documents the per-criterion table + the §8
   closed-zone-list tail note; the ledger flips ZONEC-MISSION1..7
   green (catalog_refs = []); the p5-zone-c gate joins P5
   required_gates (12 gates); the ledger test pin re-baselined 8/37
   to 15/37 + the ZONEC 7/7 line (deliberate, same commit). Verified:
   zone_mission_parity 5/5 (both zones, 10.85s), bedlam-game suites
   green (canonical_dump_gate 13/13 — ZERO canonical chain movement;
   differ_gate 4/4; determinism; census 37/37 unchanged), bedlam-core
   201/0, diffharness 104/0, fmt + clippy clean, check-p5-zone-ledger
   OK + hermetic suite 18/18, gates-validator suite 22/22 + the
   HEAD-bound battery all-green at dcfdcc8 (12 gates), MANIFEST clean
   before AND after every corpus read, no Ghidra run. Queued: the
   ZONED disposition unit as the new head (the pure ZONES-append;
   after it ZONEE, ZONEF, ZONEG close the ledger). Finish note: the
   worker died at a transport error after the flip commit but before
   the bookkeeping commit — the watchdog repair 3709375 adopted the
   commits verbatim, fixed a prose bracket in this rewrite (the
   D180 class, third recurrence — D194), landed this bookkeeping
   commit, and pushed.
