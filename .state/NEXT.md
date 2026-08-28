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
1. [READY] [id=p5-zone-g-disposition] [gate=p5-zone-g-disposition] P5
   follow-up — ZONE F is CLOSED (36/37 green, D197) and ZONEG is the
   LAST disposition of the ledger: one mission closes P5 at 37/37.
   A pure ZONES-append unit in the §12 shape. (a) LAND the ZONEG
   evidence: APPEND the ZONEG ZoneSpec (letter G, missions 1..=1,
   dims 100x25 — the census-pinned non-square mission, TOT 40004 B
   = 4 + 16*w*h with w*h = 2500, header re-verified first-hand;
   committed flows NONE — no committed .scen stages zone G, so the
   generated per-mission battery IS the whole criterion-1 leg; M1
   through the episode-slot mask seam; linear =
   clamp(5*(7-2)+m-1, 1, 26) = m+24, M1 = 25 — one below the clamp
   ceiling 26, no bite; zone G ships NO mission-number variant bank
   — only the zone-level MISSIONG.BIN 2443943 B, so NO G3 no-swap
   extra leg, just the zone-level fetch-chain assert; note M1's NME
   is a real 1144 B bank, not the 16-byte MP empty), document the
   §13 criterion table in P5-ZONE-GATES (the §12 pattern), flip
   ZONEG-MISSION1 to green WITH the cross-artifact rule (the flip
   and its evidence in the same commit), re-baseline the ledger
   test pin 36/37 to 37/37 in the SAME commit (the D28 fingerprint
   discipline, D192..D197 precedent), and wire the p5-zone-g gate
   into docs/required-gates.toml P5 required_gates (offline
   evidence commands only; zone G stages no committed flow so the
   gate carries no scenario corpus). Bounds: bedlam-core +
   bedlam-game suites green; the census stays 37/37; zero canonical
   chain movement; fmt + clippy; the gates validator all-green
   (16 gates; .state/STATE.md is an s0-dispositions tracked path —
   park uncommitted STATE.md edits while running the HEAD-bound
   battery, the D193/D194 lesson); MANIFEST clean; no Ghidra run;
   Nudge-Worker trailer. If the battery surfaces a REAL engine gap
   on the ZONEG mission, stop at the structured finding — the gap
   becomes its own unit, the ledger stays unchanged, and the
   failure artifact records it. After ZONEG: the ledger reads
   37/37 and P5 moves to its phase-close disposition.
## Done
1. DONE (2026-08-28, claim 1 — substantive commits 99bb89a +
   29cfc3f by worker b5bce035, which the 09:49 watchdog pass
   terminated mid HEAD-bound battery over the stale unacked
   224613cc no-progress marker; claim released, and watchdog
   repair 314485 independently re-validated and finished the
   bookkeeping + push): P5 `p5-zone-f-disposition` — ZONE F
   CLOSED: the FIFTH 7-mission zone flips green (the ledger 36/37;
   D197) and the disposition is the THIRD PURE ZONES-APPEND (the
   §10/§11/§12 shape). (a) THE APPEND (99bb89a): the ZONEF ZoneSpec
   joined the ZONES list after B, C, D, E (letter F, missions
   1..=7, dims 100x100, committed flows NONE) and nothing else; the
   battery: P5FM1A..P5FM7C all 21 flows full declared budgets,
   dumps verify, two-run byte identity — NO engine gap on any ZONEF
   mission; zones B (21 + the committed S5/S5B/S5C), C (21), D (21)
   and E (21) re-verified in place. (b) THE FLIP (29cfc3f, the
   cross-artifact rule): ledger ZONEF-MISSION1..7 green
   (catalog_refs = []); P5-ZONE-GATES §12 criterion table (linear =
   m+19, M1..M7 = 20..26 — M7 exactly touches the clamp ceiling 26,
   the first ledger mission to reach it; zone F ships NO
   mission-number variant bank — only the zone-level MISSIONF.BIN
   1464679 B, so the D184 no-swap pin needs no variant caveat); the
   p5-zone-f gate joins P5 required_gates (15 gates); the ledger
   test pin re-baselined 29/37 to 36/37 + the ZONEF 7/7 line; D197.
   Re-validated first-hand by the finishing repair session at
   29cfc3f: ledger OK 36/37 + ZONEF 7/7, hermetic 18/18, strict
   queue parser rc=0, zone_mission_parity 5/5 (26.75s, all five
   ZONES-const zones incl. F), MANIFEST clean before and after
   every corpus read, no Ghidra run. Queued: the ZONEG disposition
   unit as the new head — one mission closes the ledger at 37/37;
   after it P5 moves to its phase-close disposition.
