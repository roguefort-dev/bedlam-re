# NEXT - task queue (top first; rewrite this file at end of every run)

QUEUE CONVENTION (2026-08-22, D106): a completed unit's entry MOVES to
the '## Done' log at end of run - never stays in '## Now' as 'N. DONE ...'
(the scheduler mechanically skips a first-word DONE marker, but the
renumbered queue keeps every open item claimable by number).
## Now
1. [READY] [id=p5-critter-state-g2-wanderers] [gate=p5-critter-state-g2-wanderers] P5
   follow-up — the FIRST G2 critter-state unit from the census
   (docs/P5-ZONE-GATES §6.2/G2, D176): Wanderers (.NME section
   2, critter state 1 — 24 of the 26 refusing missions host it, the
   most common state; grammar head = bedlam-assets misc.rs
   NmeSectionKind::Wanderers). (a) RE FIRST (objdump-only, no Ghidra
   run): decode the EXW wander-AI family (the FUN_00416458 loader
   walk is already anchored §7j.18; the state-1 controller + its
   wander/sine semantics + difficulty scaling), write the RE notes as
   a committed docs/RE-EXW-SIM §7j addendum BEFORE any engine change.
   (b) Land the controller bounded in bedlam-core critter.rs +
   extend stage_critters to accept the section; RE-BASELINE the
   census pins deliberately (mission_load_census rows whose refusals
   drop) in the same commit with the census_print_table output as the
   provenance. (c) Do NOT touch canonical chains unless a scenario
   exercises the section (none does today — the critters key stages
   ZONEA-shaped content only); document any chain decision. Bounds:
   census_matches_pinned_table green after the deliberate re-pin; the
   full bedlam-game suite green; fmt + clippy; gates-validator 22/22;
   MANIFEST clean before AND after; Nudge-Worker trailer.
2. [READY] [id=ci-cross-os-repair] [gate=ci-cross-os-repair] infra
   follow-up (queued by the D178 zone-A evidence run): the CI matrix —
   the designed cross-OS enforcement channel for criterion 5
   (docs/P5-ZONE-GATES §7 table row 5) — is RED repo-wide for
   ENVIRONMENT reasons, >=100 consecutive failing runs, every failure
   BEFORE any test executes (verified via gh on 2026-08-27): (a) the
   ubuntu build job dies at `cargo clippy --workspace --all-targets`
   on the alsa-sys v0.4.0 build script (cpal needs libasound2-dev +
   pkg-config on the runner) — fix .github/workflows/ci.yml with an
   apt install step (or make the alsa dependency optional for CI);
   (b) the miri job dies on `mission_corpus_gate` file-isolation (a
   corpus-gated suite that must skip cleanly without game-data — find
   the unconditional open/stat on the skip path or set the isolation
   error policy) — the job's charter is bedlam-core+bedlam-audio UB
   detection, keep it that scope; (c) windows-latest only ever
   fail-fast-cancels behind ubuntu — verify it goes green once (a)
   lands. Evidence: a pushed commit whose ci run is green on BOTH
   matrix legs (gh run view), restoring the ubuntu+windows enforcement
   of hash_fixture + the determinism suites. Bounds: workflow/test-
   skip changes only, no engine behavior change, no game-data touch,
   MANIFEST clean, gates-validator 22/22 stays green, Nudge-Worker
   trailer.
3. [READY] [id=p5-select-shell-g1] [gate=p5-select-shell-g1] P5
   follow-up — the G1 SELECT mission-choice shell from the census
   (docs/P5-ZONE-GATES §6.2/G1, D176): make missions 6-7 of a
   7-mission zone reachable through the engine staging seam. (a) RE
   FIRST (objdump-only): how the original selects a sub-mission past
   the 4-bit stage mask (the EXW SELECT screen family 0x50953 / the
   mission-slot writes; the census pinned FULL_MASK=15 = B2 @0x81d9a
   as the SAVE shape — find the runtime mission-number source the
   SELECT screen writes), committed as RE notes BEFORE any engine
   change. (b) Land the host seam bounded (a stage_episode_slot
   extension or a sibling seam — RE decides; keep the canonical S5
   zone-staging semantics intact), extend the census to stage B-F
   missions 6-7 through it, re-baseline the census pins deliberately.
   (c) No canonical chain movement unless the seam changes an emitted
   row (then documented + deliberate). Bounds: census green after the
   re-pin; bedlam-game suite green; fmt + clippy; gates-validator
   22/22; MANIFEST clean; no Ghidra run; Nudge-Worker trailer.
   NOTE (D178): the original-save import seam (bedlam-game save.rs,
   §7j.70) deliberately REJECTS SELECT-shaped masks (mask bits past
   FULL_MASK) loud until this unit lands — widening that domain is
   part of this seam's acceptance.
4. [READY] [id=p5-zone-bin-variant-g3] [gate=p5-zone-bin-variant-g3] P5
   follow-up — the G3 zone-BIN variant RE unit from the census
   (docs/P5-ZONE-GATES §6.2/G3, D176): decide EXW-anchored whether
   ZONEB/MISSION6, ZONED/MISSION5, ZONEE/MISSION6 load the
   mission-number .BIN (ZONEB/MISSION6.BIN etc.) instead of the
   zone-level MISSION{L}.BIN (the open RESEARCH-8STREET §3 question —
   re-anchor to EXW/EXD addresses, never copy 8street code). (a)
   RE ONLY first (objdump, no Ghidra): the load_mission BIN name
   construction (FUN_0041dc5a family, §7c.1) + any per-mission
   override; record the verdict + anchors in FORMATS-MISSION (§0/§5
   as fits) + a DECISIONS entry; close the RESEARCH-8STREET §3
   question. (b) If the verdict is a swap: land the bounded
   mission_asset_names change + census re-pin; if not: record the
   zone-level rule as VERIFIED and leave the engine untouched.
   Bounds: census green (re-pinned only if the swap lands); MANIFEST
   clean; no Ghidra run; Nudge-Worker trailer.
## Done
1. DONE (2026-08-27, worker 42041a21 claim 1, commits 94d2c8b + 70897c5,
   both PUSHED): P5 `p5-zonea-mission1-parity` — ZONEA-MISSION1 flipped
   GREEN (the FIRST zone-parity disposition, D178) with its executable
   evidence + the p5-zone-a completion gate wired. (a) RE FIRST
   (94d2c8b): RE-EXW-SIM §7j.70 pins the SAVED.BDL restore header walk
   EXW-side (slot stride 0xB4=180, name@+0, mask dword@+8, zone SIGNED
   word@+0xC -> 0x4edd8c @0x43c2b8, score@+0xE, money@+0x12,
   difficulty@+0x16; empty predicate = zero dword@+0x0C; the mask
   replay) — the 8street layout now EXW-anchored. (b) EVIDENCE +
   FLIP + GATE (70897c5, same commit per the queue contract):
   tests/zonea_mission1_parity.rs (the §1 criterion table executable:
   the 8 ZONEA S-scenarios S0-S4/S6-S8 run full declared budgets
   crash-free + two-run byte identity; T1 spot table; anchor TS
   statics independently re-derived from the TOT header + the
   §7j.64/D154 fresh scalars; the REAL SAVED/OPTIONS.BDL import —
   slot 0 PLAYER/zone 2/mask 0/money 580/difficulty 1 -> stages
   ZONEB-MISSION1, four EMPTY slots rejected — + bounded deterministic
   fuzz; DM carve-out noted); bedlam-game save.rs = the read-only
   bounds-checked original-save import seam (5 lib units) +
   GameHost::import_saved_slot staging via the D51 seam; ledger
   ZONEA-MISSION1 -> green (catalog_refs empty, legitimate); p5-zone-a
   wired into P5 required_gates (2 offline evidence commands); checker
   suite pin re-baselined 0/37 -> 1/37 deliberately; P5-ZONE-GATES §7
   = the closure table; DECISIONS D178. Cross-OS honestly recorded:
   hash_fixture + determinism verified on TWO toolchains (stable +
   nightly, identical pins); the ubuntu+windows CI channel RED
   repo-wide for ENVIRONMENT reasons (alsa-sys; miri isolation; >=100
   runs, all pre-test) — repair QUEUED as item 2 (ci-cross-os-repair),
   not a determinism finding. Verified: bedlam-game release 245/0
   (234 + 11 new); checker OK + suite 18/18; gates-validator 22/22;
   the bound P5 phase validation status=passed at 70897c5 under real
   bwrap containment (both P5 gates, all 4 commands rc=0, plan_complete
   correctly false); fmt + clippy -D warnings clean on the touched
   crate; MANIFEST clean before AND after every corpus run; no Ghidra
   run; no canonical-chain movement (canonical_dump_gate pins
   re-asserted unchanged). Queued: items 1-4 above (the census G2/G1/G3
   units unchanged + the new CI repair).
1. DONE (2026-08-27, worker 7e59f4d7 claim 1, commit 4803d58,
   PUSHED): P5 `p5-mission-load-census` — the all-37-mission
   READ-ONLY load census through our engine loaders, sizing the zone
   work (D176, docs/P5-ZONE-GATES.md §6). (a) The executable census
   engine/bedlam-game/tests/mission_load_census.rs (corpus-gated,
   deterministic): per mission the canonical 25-name fetch +
   GameHost::load_mission through stage_episode_slot (or
   MissionScene::stage + claim bank directly where the slot cannot
   reach), then the destroy family (BDG/POS/TRT), the pickup surface
   (TOT), the critter family (NME), the full bedlam-assets parser
   family over every runtime file, and a scripted frame run (FSM
   Boot→Mission + 9 frames host-side; activate + 8 tick/present
   direct-side; panics caught + recorded). The pinned table =
   census_matches_pinned_table (D28 fingerprint discipline);
   census_print_table --ignored prints the full columns. (b) VERDICT:
   ALL 37 LOAD — zero load failures, parser refusals, or frame-run
   panics; destroy/pickup/parsers/frames ok on EVERY row; every TOT
   header independently re-derives the §2 dims (25x75/100x100/100x25).
   ZONEA-MISSION1 is the ONLY zero-gap mission (the canonical
   corpus's own). Three named SEMANTIC gap classes, NONE
   parser-sized: G1 episode-slot seam (B-F missions 6-7; FULL_MASK=15
   = B2 @0x81d9a — staged direct, load+run clean; fix = the SELECT
   shell, queued item 3), G2 critter states (26 missions refuse
   Shooters/Wanderers/Chasers/BallisticState6/CloseCombat/Personnel;
   ZONEA-M1 + the ten 16-B empty-NME missions pass; queued item 1 =
   Wanderers first), G3 zone-BIN variant (ZONEB-M6/ZONED-M5/ZONEE-M6
   ship mission-number .BIN banks; override rule unresolved vs EXW,
   RESEARCH-8STREET §3 — queued item 4). (c) NO loader change landed
   (nothing parser-sized); ledger UNCHANGED (no mission
   unloadable-by-corpus — dispositions flip only on parity evidence).
   Verified: census 1/1 pinned + the print probe; canonical_dump_gate
   13/13; bedlam-game release 234/0; fmt + clippy clean;
   gates-validator suite 22/22; the bound P5 phase validation GREEN
   at 4803d58 (p5-zone-gate-scaffold: checker + 18-case suite rc=0);
   MANIFEST clean before AND after; no Ghidra run; no canonical-chain
   movement (test-only addition). Queued: items 2-4 above
   (zonea-mission1-parity stays the head as item 1).
1. DONE (2026-08-27, worker 05e2d7ae claim 1, commits 953b6af +
   5e8e78f, both PUSHED): P5 opener `p5-zone-gate-scaffold` — the
   per-zone parity LEDGER + the first P5 required gate LANDED (D175).
   (a) The 37 shipped missions enumerated READ-ONLY from game-data/
   BEDLAM/EDITOR/ZONE*/MISSION*.TOT (ZONEA M1; ZONEB..F M1-7 each;
   ZONEG M1; TOT size arithmetic 4+16·w·h self-checked: 30004/160004/
   40004 all match; MANIFEST clean before AND after; corpus
   untouched). (b) docs/P5-ZONE-GATES.md: the per-zone acceptance
   shape VERBATIM from PLAN §6 P5 + the seven-criterion decomposition
   (DM carve-out as scope, not check) + the ledger format spec. (c)
   docs/P5-MISSION-LEDGER.toml (schema p5-mission-ledger-v1): 37
   mission rows, ALL pending, catalog_refs reserved as the P6 triage
   feed; zone status DERIVED, never stored. (d) The fail-closed
   checker tools/check-p5-zone-ledger.py + the 18-case hermetic suite
   tools/test-p5-zone-ledger.py: ledger completeness/internal
   consistency, corpus re-enumeration pinned to the shipped zone
   shape (drift fails loud), ledger set == corpus set, and
   cross-artifact manifest safety (p5-zone-{a..g} gates require their
   zone fully green; P5 status green requires 37/37 — closing the
   empty-green-phase hole in the validator's all-gates-pass
   semantics). (e) docs/required-gates.toml: P5 required_gates =
   ["p5-zone-gate-scaffold"] as the FIRST entry (checker + suite);
   NO game-data path in tracked_paths/corpus — the checker reads the
   corpus read-only at runtime, the MANIFEST.sha256 contract. VERIFIED:
   bound `validate-required-gates.py --phase P5` at HEAD 5e8e78f
   status=passed (gate green under real bwrap containment, both
   commands rc=0); checker OK 0/37; suite 18/18; gates-validator
   22/22; canonical_dump_gate 13/13 (controls — no engine change);
   manifest TOML re-parsed (9 gates, 8 phase rows); tools committed
   mode 100755; no Ghidra run. Queued: p5-mission-load-census (1) +
   p5-zonea-mission1-parity (2).
[post-P4 note] (the five-unit P4 machine contract is fully
consumed and the bound phase verdict landed; the controller's
empty-queue path now owns the P0-P7 completion decision and P5+
queue content is operator/controller work — superseded 2026-08-27 by
the p4-phase-status-green item above keeping required work active
instead of idling on the completion beacon)
1. DONE (2026-08-27, worker eeba31cf claim 1, commit 972748d,
   PUSHED): P4 closure bookkeeping `p4-phase-status-green` — the P4
   phase status FLIPPED pending->green in docs/required-gates.toml
   (P0-P4 green, P5-P7 pending; plan_complete correctly stays
   false), then the bound phase verdict RE-EMITTED at the flip
   commit with the exact mandated command: /usr/bin/python3
   tools/validate-required-gates.py --root . --report
   .state/p4-gates-report.json --phase P4 --phase-output
   .state/P4-COMPLETE — all 8 P4 gates GREEN at 972748d (report
   status=passed, bounded, offline; .state/P4-COMPLETE
   phase-complete-v1 re-bound to the new HEAD + manifest sha256
   734a540c..., emitted by the validator itself). Pre-flip checks:
   gates-validator command 22/22 green at d84f8d0 (the 17550e2
   full-run gates-validator failure was fixed BY d84f8d0; its
   p4-machine-verdict False was only the dependency short-circuit),
   MANIFEST clean before AND after, TOML re-parsed (8 phase rows).
   .state/STATE.md phase line updated (P4 GREEN / P5 UNDERWAY; the
   stale 2026-08-18 P3-era duplicate phase line collapsed and marked
   historical). First P5 unit queued: p5-zone-gate-scaffold (the
   37-mission per-zone parity ledger per PLAN §6). No engine change;
   no canonical chain movement by construction; no Ghidra run.
