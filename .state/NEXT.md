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
1. [READY] [id=p5-zone-bin-variant-g3] [gate=p5-zone-bin-variant-g3] P5
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
2. [READY] [id=p5-critter-state-g2-shooters] [gate=p5-critter-state-g2-shooters] P5
   follow-up — the NEXT G2 critter-state unit from the census
   residue (docs/P5-ZONE-GATES §6.2/G2, D179/D182 lineage): the
   kind-2 SHOOTERS state — the most-hosted unmodeled section
   (ZONEB M2/M4/M5, ZONEC M1/M3/M5, ZONED M1-M5, ZONEE M1-M5,
   ZONEF/ZONEG: 17 hosting missions beside the Chaser-only rows).
   (a) RE FIRST (objdump-only from the committed
   exw-critterpoi-loader.txt + exw-text-objdump.txt, no Ghidra run):
   the .NME S1 loader walk for kind 2 (the §7j.18/§7j.71 method —
   stamps, counts, draws, the hp scalar on the 0x46ae8c linear m
   per the closed imul census) + the k2 controller body (the kind
   table 0x412f18 case 2), committed as RE notes BEFORE any engine
   change. (b) Land stage_critters section 2 acceptance + the
   controller in bedlam-core::critter with unit tests; re-pin the
   census (the ShootersxNN component drops; expect NO row flip
   clean — Chasers remain on every host). Bounds: census green
   after the re-pin; bedlam-core + bedlam-game suites green; no
   canonical chain movement (ZONEA/M1 hosts no S1) unless a row
   changes (then documented + deliberate); fmt + clippy;
   gates-validator 22/22; MANIFEST clean; no Ghidra run;
   Nudge-Worker trailer.
## Done
1. DONE (2026-08-28, worker 05e14378 claim 1, commits a5c3a71 +
   3d64ca5, both PUSHED): P5 `p5-select-shell-g1` — the G1 SELECT
   mission-choice shell LANDED: missions 6-7 of zones B-F (the ten
   MP-only missions) stage through the engine, the census G1 class
   CLOSED. (a) RE FIRST (a5c3a71, §7j.73, objdump-only from the
   committed exw-text-objdump.txt — no Ghidra run): the runtime
   mission-number source = the SELECT screen's write pair
   {zone 0x4edd8c, mission 0x4edd88} read from its strategic-map
   PIXEL→ID grid — the SP arm (0x43ee48..0x43ee9d) writes missions
   1..5 per zone ONLY (26 hot spots = ZONEA{1} + 5×{B..F} =
   MAX_LINEAR; zone G is the campaign-advance endgame, no hot
   spot), the MP arm (0x43edc2..0x43ee43) writes BOTH cells from
   10 list rows → {zone 2..6, mission 1..2}, and
   build_mission_paths @0x4467df ADDS 5 to the mission cell in
   mode 2 — **missions 6-7 are the MP-ONLY files, NOT campaign
   sub-missions: no stage mask can ever express them (the G1
   answer)**. The save-restore replay tests FIVE mask bits
   (0x43c2bf..0x43c36c, subs 1..5) — the EXW save domain is
   0b11111 (the B2 FULL_MASK=15 table is B2's own 4-sub campaign);
   the 27-record completion bank (0x4decae, 0x144/0xC — one
   record per linear mission) is the SELECT screen's own state
   (FUN_004474ef/44751c). (b) ENGINE (3d64ca5): the SIBLING seam
   GameHost::stage_select_mission (the MP write pair domain
   zone 2..=6, mission 1..=2 — never guess) + mission_slot applies
   the +5 (SELECT_MP_FILE_OFFSET); the pair is staging-ONLY state
   (NOT in scene_hash — the D31 pattern, pinned by test) and
   campaign staging CLEARS it; Episode::stage_slot's accepted
   domain widened to SELECT_FULL_MASK [0,1,31×7] while
   Episode::complete still walks FULL_MASK (canonical S5 semantics
   INTACT); mission_number_for_mask saturates at 5 (the SP SELECT
   domain — the campaign path can never name an MP file,
   property-tested); save.rs widened (the D178 rider): bit-4 masks
   import + stage, bits past 0x10 stay rejected loud. (c) CENSUS
   RE-PIN (deliberate, D28): the ten B-F missions-6/7 rows moved
   from the direct fallback to the SELECT seam — all select:clean
   (provenance docs/evidence/p5-g1-select-census-table.txt);
   P5-ZONE-GATES §6.1/§6.2/§6.3/§6.4 re-baselined (G1 LANDED, the
   headline gains the ten clean rows); D183. Verified: bedlam-game
   release 249/0 (+4 net new tests; canonical_dump_gate 13/13 +
   differ_gate 4/4 + determinism green — NO canonical chain moved),
   bedlam-core 154/0, diffharness 103/0, fmt + clippy clean on the
   touched crate, gates-validator 22/22, inspect baseline ok (1069
   files), MANIFEST clean before AND after every corpus run, no
   Ghidra run. Queued: items 1-2 above (the G3 BIN variant is the
   new head; the G2 Shooters unit second). Watchdog repair 1007791
   (2026-08-28): this worker was grace-killed at the 240s boundary
   while re-verifying a51d4f2 — the gates-validator battery runs
   after the bookkeeping commit by contract — so the repair landed
   the push, archived the false preflight-mismatch failure, and
   widened the boundary grace 240 to 900 in this repair commit.
1. DONE (2026-08-28, worker b03463e5 claim 1, commits e590bd6 +
   715a066 + 0aa7cf0, all PUSHED): P5 `p5-critter-state-g2-ballistic6`
   — the SECOND G2 critter-state unit: BallisticState6 landed
   engine-side through the shared k5/6 body + the D179 RIDER (the
   S3/S4 hp scalars aligned to the linear mission m) + the deliberate
   S8 chain re-baseline (D182, §7j.72). (a) RE FIRST (e590bd6,
   decompile-only from the COMMITTED exw-critterpoi-loader.txt — no
   Ghidra run): §7j.72 walks the S6 staging block exact — ONE per
   8-B record at EVERY difficulty (no inner spawn loop, the S3/S7
   multiplier preambles absent), ZERO stream draws, the S3 stamps
   verbatim (kind 6, species 3, mode 8, anim 5, heading 0x72, the
   w1-level floor probe, countdown 0), no home stamps, hp
   0x96+(m·0x96)/0x1B on the SAME 0x46ae8c cell — the §7j.71/1 imul
   census now closes: EVERY section reads m, none difficulty.
   (b) ENGINE (same commit): stage_critters accepts section 6 (the
   E S3 block verbatim, kind 6, ONE draw-free spawn; the 5|6 dispatch
   arm predates it from W12-S8) + the S3/S4 scalar swap to
   MissionSim::linear; 3 new unit tests (d=3/m=5 proves hp 177/237
   where the difficulty form said 166/222; m=0 gives the base).
   (c) CENSUS + GATES (715a066): the BallisticState6xNN component
   dropped from all 26 hosting refusal rows, NO row flipped clean
   (every host still carries Shooters/Chasers/CloseCombat/Personnel;
   ZONEA-MISSION1 stays the sole clean row); the print-table output
   committed as docs/evidence/p5-g2-ballistic6-census-table.txt; the
   S8 canonical chain re-baselined deliberately (canonical_dump_gate
   corpus_s8 + differ_gate's S8 row, 10c78a7144cf6d3d ->
   bac6a3053cedfebd — S8 stages no destroy so m=0 and the staged hp
   moved 155/207 -> 150/200; the 0-when-unstaged class D179 accepted
   for S2, documented §7j.72/4; the death-timeline inequality asserts
   survived unmoved); P5-ZONE-GATES 6.2/G2 + 6.3 + 6.4 re-baselined;
   0aa7cf0 is the fmt pass on the new tests. Verified: bedlam-core
   release 155/0 (18 critter tests), bedlam-game release 245/0
   (census + canonical_dump_gate 13/13 + differ_gate 4/4),
   diffharness 103/0, fmt clean, clippy clean on touched files
   (the 5 remaining workspace warnings pre-exist), gates-validator
   22/22 + the bound P5 phase validation status=passed at 0aa7cf0,
   MANIFEST clean before AND after every corpus run. Queued: items
   1-2 above (the G1 SELECT shell is the new head).
1. DONE (2026-08-28, worker 0e1d4854 claim 1, commits 177c953 +
   a18d9c3 + a168d69, all PUSHED): infra `ci-cross-os-repair` — the
   CI matrix REPAIRED GREEN on BOTH legs + miri + diffharness (run
   33123147228 at a168d69, verified via gh run view; the ubuntu+windows
   enforcement of the determinism/replay suites restored — the windows
   leg runs and passes them on MSVC). (a) 177c953: the two
   ENVIRONMENT fixes — a Linux-only apt step (libasound2-dev +
   pkg-config) so the alsa-sys build script survives on ubuntu, and
   MIRIFLAGS=-Zmiri-isolation-error=warn so the corpus-gated suites'
   skip probes return clean errors instead of aborting under miri
   isolation (verified locally: corpus-less clone 759/0 + all 8
   corpus binaries green under the warn policy; miri GREEN on CI
   33122184098/33123147228). (b) a18d9c3: the first-ever windows test
   run caught a REAL engine bug the channel existed to catch —
   stage_debris resolved the seq-table index by std::ptr::eq with an
   .unwrap_or(0) fallback, and a release-profile probe proved the
   pointer match NEVER succeeds in release builds (all 20 kinds walked
   table 0; debug passed by constant-merge luck; the canonical S4 pin
   had encoded the bug). Fixed by content equality + expect, two
   regression pins landed (per-kind staged index + per-kind terminator
   walk), and the S4 chain re-baselined DELIBERATELY
   (canonical_dump_gate + differ_gate, 1357af61ef082cb5 ->
   21520352000ca4bf — the one canonical chain movement, in the same
   commit as the fix, D181 items 4-6). Verified: bedlam-game release
   245/0, diffharness 103/0, bedlam-core release 151/0. (c) a168d69:
   the CRLF class — 11 diffharness s*_plan_matches_committed_artifact
   pins failed on windows ONLY from autocrlf rewriting committed
   artifacts at checkout (include_str! embedded CRLF); fixed by a
   repo-wide .gitattributes eol=lf policy (zero CR blobs tracked, no
   renormalization), verified in an autocrlf=true clone (LF checkout,
   corpus-less workspace 761/0). Bookkeeping: P5-ZONE-GATES §7 row 5
   -> GREEN (fixtures + cross-toolchain + CI matrix); D181 records the
   whole arc. Bounds check: workflow + test-policy changes plus the
   one bounded engine correctness fix the acceptance criterion forced
   (documented, pinned, re-baselined); no game-data touch; MANIFEST
   clean before AND after every corpus run; gates-validator 22/22;
   bound P5 phase validation status=passed at a18d9c3; no Ghidra run.
   Queued: items 1-3 above (ballistic6 stays the head with its S3/S4
   hp rider).
1. DONE (2026-08-27, worker 58b640c3 claim 1, commits 2195999 +
   49aeeeb + c60c0ba, all PUSHED): P5 `p5-critter-state-g2-wanderers`
   — the FIRST G2 critter-state unit: the kind-1 Wanderer landed
   engine-side whole + the census re-pinned deliberately (D179,
   §7j.71). (a) RE FIRST (2195999 + 49aeeeb, objdump-only from the
   committed exw-text-objdump.txt + the §7j.18 loader decompile, no
   Ghidra run): §7j.71 pins the k1 body 0x414c96..0x415216 whole —
   the door-tile gate (FUN_004186fc, the §7j.12 type-DB variant byte
   — documented E-gap), the suicide trigger FUN_00417e2f (nearest
   robot < 0x30 px → presence 0 + 8 debris/splash pairs = 5 draws × 8
   = 40; the EXPLICIT return convention correcting §7j.17/2's
   EAX-leak hypothesis), the (countdown, DIR) substep machine with
   the IDLE SQUASH semantics (the 8..15/12..27 re-pick constants
   never take effect — the runtime pause is 2 substeps), the DIR
   table @0x412f08 {0→y−6, 1→x+6, 2→y+6, 3→x−6}, the ±6 RAW-px
   steppers (kind 1 is px-scale — the §7j.17 Q13 gloss corrected),
   the 8-sample wall probe (footprint from 0x4543e4/0x454404;
   floor_z==z ∧ RAW tile ≤ 3), the toward-robot picker
   (y-axis ties), the FUN_00418250 death quirk, and the S2 loader
   walk (DIR seed −1 — new pin; one draw per spawned critter; the
   level-6-down z search). **The .NME hp scalar = base+base·
   [0x46ae8c]/27 = the LINEAR MISSION m for EVERY section — §7j.18's
   difficulty gloss corrected.** (b) THE LANDING (c60c0ba):
   stage_critters accepts S2 (hp via MissionSim::linear); the k1
   controller in bedlam-core::critter + 11 new unit tests; new
   record fields dir/frame/z_restore NOT in the canonical blob. The
   S3/S4 hp scalars HOLD difficulty deliberately (the S8 chain stages
   ZONEA S3+S4; no scenario exercises S2 → NO canonical chain
   movement; the alignment queued as the item-2 rider). (c) THE
   CENSUS RE-PIN: WanderersxNN dropped from every G2 refusal row (no
   row flipped clean — every host carries another state); the
   census_print_table output committed as provenance
   (docs/evidence/p5-g2-wanderers-census-table.txt) + P5-ZONE-GATES
   §6.2/G2 + §6.3 updated. Verified: bedlam-core 88/88 (77+11);
   bedlam-game release 245/0 (census 1/1 re-pinned,
   canonical_dump_gate 13/13 unchanged, zonea_mission1_parity 6/6,
   determinism + differ green); fmt + clippy -D warnings clean on
   the touched lib; gates-validator 22/22; inspect baseline ok;
   MANIFEST clean before AND after; Nudge-Worker trailer. Queued:
   items 1-4 above (ci-cross-os-repair stays the head; the new
   ballistic6 G2 hop second with the S3/S4 hp rider).
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
