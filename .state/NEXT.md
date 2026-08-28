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
1. [READY] [id=p6-modeconfig-seam] [gate=p6-modeconfig-seam] P6 seam
   implementation per PLAN §6 + D200 — the FIRST engine unit behind
   the p6-modernization-scaffold contract: land the ONE immutable
   ModeConfig injected at sim construction (constructed once,
   injected, never mutated mid-run; a mode change is a new sim) with
   the initial purist toggle set covering the two plan-named
   FEEL-CONTESTED axes only (timing lock, control scheme); the
   catalog purist_toggle join stays EMPTY until entries exist (the
   D200 seeding policy: recorded evidence with a repro only); default
   mode = modern; presentation/platform options stay OUTSIDE
   ModeConfig (display rate never enters the sim, Determinism
   Charter). Test surface = the purist toggles (both arms), never the
   feature cross-product. Wire the completion gate p6-modeconfig-seam
   as the SECOND P6 required_gates entry (R6 of
   tools/check-p6-behavior-catalog.py keeps the scaffold first — the
   manifest rule enforces it). Bounds: no canonical-chain movement
   (canonical_dump_gate 13/13, zone_mission_parity 5/5,
   determinism green before AND after — a modern default must leave
   the canonical S0-S8 chains byte-identical or the seam is wrong);
   fmt + clippy on touched crates; the gates-validator suite stays
   green; MANIFEST clean before and after any corpus read; no Ghidra
   run; commit with the unit's own Nudge-Worker trailer.
## Done
1. DONE (2026-08-28, claim 1 — commit e0bc7fb by worker 6e45232f,
   PUSHED, plus this bookkeeping commit): P6 opener
   `p6-modernization-scaffold` — the modernization CONTRACT scaffold
   landed per PLAN §6 + the D175 pattern (the machine-checkable
   contract BEFORE any behavior change it grades; bounds honored: no
   engine change, no harness change, no Ghidra run). (a) D200 in
   docs/DECISIONS.md decides the ModeConfig seam per PLAN §6
   verbatim: fixes land directly in the engine (no bug-complete-
   faithful core), classic mode = a small purist toggle set covering
   FEEL-CONTESTED items only (timing lock, control scheme, preserved
   catalog entries), mode = ONE immutable ModeConfig injected at sim
   construction, test surface = the purist toggles never the feature
   cross-product, presentation options are NOT mode toggles. (b)
   docs/P6-MODERNIZATION.md commits the bug-triage rubric VERBATIM
   from PLAN §6 (crash/data-loss fixed everywhere; gameplay-coupled
   classic-preserves/modern-fixes; cosmetic fixed in modern; fixed =
   mechanically applying the rubric + regression evidence, not
   vibes), the catalog format spec (schema p6-behavior-catalog-v1,
   fields + the R1-R7 mechanical rules), and the seeding policy. (c)
   docs/P6-BEHAVIOR-CATALOG.toml seeds EMPTY (the honest post-P5
   state: 37/37 green, all catalog_refs empty; entries land only on
   recorded evidence with a repro, observed = original or
   divergence — post-parity an original bug is a faithful
   reproduction, so original observations are the expected dominant
   source). (d) tools/check-p6-behavior-catalog.py +
   tools/test-p6-behavior-catalog.py: the fail-closed checker (30
   cases hermetic, all fail-closed: rubric-as-code over all six
   wrong-class closures, evidence discipline both directions, toggle
   discipline, mission grounding, the BIDIRECTIONAL ledger
   catalog_refs join, manifest scaffold-first + P6-green rules) wired
   as the FIRST P6 required gate in docs/required-gates.toml (P6
   pending with exactly one required gate; reads ONLY committed docs,
   no corpus key). Verified first-hand: checker OK (entries 0, 37
   ledger ids, 0 refs resolve) pre- and post-commit; suite 30/30;
   gates-validator suite 22/22; check-p5-zone-ledger OK 37/37 with
   the edited manifest; controls green BEFORE (clean HEAD 0c81387)
   AND AFTER (commit e0bc7fb): zone_mission_parity 5/5 (27.33s /
   27.20s) + canonical_dump_gate 13/13 (4.50s / 4.45s); the bounded
   HEAD-bound phase verdict at e0bc7fb via validate-required-gates.py
   --phase P6 = status passed, both gate commands rc=0 under bwrap
   containment; MANIFEST clean before AND after every corpus read; no
   canonical-chain movement. Queued: the p6-modeconfig-seam engine
   unit as the new head.
2. DONE (2026-08-28, claim 1 — commit f608207 by worker ec090fa6,
   PUSHED, plus this bookkeeping commit): P5 phase-close
   bookkeeping `p5-phase-close` — the P5 phase status FLIPPED
   pending->green in docs/required-gates.toml (P0-P5 green,
   P6-P7 pending; plan_complete correctly stays false), then the
   bound phase verdict RE-EMITTED at the flip commit with the
   exact P4-shaped command: /usr/bin/python3
   tools/validate-required-gates.py --root . --report
   .state/p5-gates-report.json --phase P5 --phase-output
   .state/P5-COMPLETE — ALL 8 P5 GATES GREEN at f608207 (report
   status=passed, bounded, offline, containment
   bwrap-unshare-net-pid-ro, every command rc=0;
   .state/P5-COMPLETE phase-complete-v1 re-bound to the flip
   commit + manifest sha256 7efb1041..., producer
   required-gates-validator, emitted by the validator itself).
   Pre-flip first-hand checks: check-p5-zone-ledger OK — the
   ledger reads 37/37 (A 1/1, B..F 7/7 each, G 1/1) and the P5
   status-green cross-artifact rule holds; the tracked tree
   stayed clean through the whole HEAD-bound battery (the
   D193/D194 lesson — STATE.md/NEXT.md edits parked until
   after); MANIFEST clean before AND after every corpus read; no
   Ghidra run. The queue then carried the P6 opener as the head
   (the p4-phase-close/5347a37 pattern): p6-modernization-scaffold
   per PLAN §6, so required work stays active.
3. DONE (2026-08-28, claim 1 — substantive commits 0829187 + 65505ea
   by worker ebf6cfca, both PUSHED): P5 `p5-zone-g-disposition` —
   ZONE G CLOSED, THE LEDGER READS 37/37: the LAST ledger mission
   flips green and P5's mission side is DONE (D199); the disposition
   is the FIFTH PURE ZONES-APPEND (the §9..§12 shape) with the ONE
   census-forced seam delta. (a) THE APPEND (0829187): the ZONEG
   ZoneSpec joined the ZONES list after B, C, D, E, F (letter G,
   missions 1..=1, dims 100x25 — the census-pinned NON-SQUARE
   mission, TOT 40004 B re-verified first-hand; committed flows
   NONE) and the SELECT write-pair legs of zone_t1_rules_spot now
   derive from the zone's own mission range (zone G's zone cell 7
   is OUTSIDE the SELECT write arm's 2..=6 domain and no MP file
   ships for G, §7j.73 — zones B..=F exercise the identical legs
   they always did; the write-arm reject domain still checks (7,1)
   loud); the battery: P5GM1A/B/C all 3 flows full declared budgets
   (3/121/49 records), dumps verify, two-run byte identity — NO
   engine gap on the ZONEG mission; zones B (21 + the committed
   S5/S5B/S5C), C (21), D (21), E (21) and F (21) re-verified in
   place. (b) THE FLIP (65505ea, the cross-artifact rule): ledger
   ZONEG-MISSION1 green (catalog_refs = []), the ledger 37/37
   (A 1/1, B..F 7/7 each, G 1/1 — EVERY shipped mission green);
   P5-ZONE-GATES §13 criterion table (linear = m+24, M1 = 25 one
   below the clamp ceiling; the zone-level MISSIONG.BIN 2443943 B
   fetch-chain pin with NO variant caveat — zone G ships no
   mission-number variant bank; the real 1144 B .NME bank; the
   zone-A-shaped DM carve-out); the p5-zone-g gate joins P5
   required_gates (16 gates); the ledger test pin re-baselined
   36/37 to 37/37 + the ZONEG 1/1 line (deliberate, same commit);
   D199. Verified first-hand at the flip commit: ledger OK 37/37 +
   ZONEG 1/1, hermetic 18/18, strict queue parser rc=0,
   zone_mission_parity 5/5 (six zones, 27.43s), canonical_dump_gate
   13/13 zero chain movement, differ_gate 4/4, determinism 4/4,
   mission_load_census green (census stays 37/37), bedlam-core
   hash_fixture + mission_corpus_gate green, fmt + clippy clean on
   the touched crate (the 7 bedlam-core warnings pre-exist from
   D151, untouched), the HEAD-bound validator battery ALL 16 GATES
   PASSED at 65505ea (bounded, offline, incl. p5-zone-g's both
   commands rc=0; the global report's status=failed/plan_complete=
   false is ONLY the pending P5-P7 phase-status semantics, not a
   gate failure — the same documented ZONEB note), MANIFEST clean
   before and after every corpus read, no Ghidra run. Queued: the
   P5 phase-close disposition as the new head (the P4 pattern).
