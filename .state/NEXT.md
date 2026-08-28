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
1. [READY] [id=p5-critter-state-g2-chasers] [gate=p5-critter-state-g2-chasers] P5
   follow-up — the FOLLOWING G2 critter-state unit from the census
   residue (docs/P5-ZONE-GATES §6.2/G2): the kind-3 CHASERS state
   (the ChasersxNN census component; hosts beside Shooters on
   ZONEB/ZONEC/ZONEE and as the sole refuser on rows where the
   Shooters unit leaves it last). (a) RE FIRST (objdump-only from
   the committed exw-critterpoi-loader.txt + exw-text-objdump.txt,
   no Ghidra run): the .NME S5 loader walk for kind 3 (the
   §7j.18/§7j.71/§7j.72 method — stamps, counts, draws, the hp
   scalar on the 0x46ae8c linear m per the closed imul census) +
   the k3 controller body (the kind table 0x412f18 case 3),
   committed as RE notes BEFORE any engine change. (b) Land
   stage_critters section 5 acceptance + the controller in
   bedlam-core::critter with unit tests; re-pin the census (the
   ChasersxNN component drops; expect NO row flip clean unless a
   host carries no other unmodeled state — then documented +
   deliberate). Bounds: census green after the re-pin; bedlam-core
   + bedlam-game suites green; no canonical chain movement
   (ZONEA/M1 hosts no S5) unless a row changes (then documented +
   deliberate); fmt + clippy; gates-validator 22/22; MANIFEST
   clean; no Ghidra run; Nudge-Worker trailer.
2. [READY] [id=p5-critter-state-g2-closecombat] [gate=p5-critter-state-g2-closecombat] P5
   follow-up — the NEXT G2 critter-state unit after the Chasers
   unit (docs/P5-ZONE-GATES §6.2/G2 residue): the kind-7
   CLOSE-COMBAT state (the CloseCombatxNN census component; hosts
   ZONEC-M3, ZONEE-M1..M5, ZONEF-M1, ZONEG-M1). (a) RE FIRST
   (objdump-only from the committed exw-critterpoi-loader.txt +
   exw-text-objdump.txt, no Ghidra run): the .NME S7 loader walk
   for kind 7 (6-B records, the §7j.18/§7j.71/§7j.74 method —
   stamps, counts, draws, the hp scalar 0x9C4 on the 0x46ae8c
   linear m) + the k7 controller body (the kind table 0x412f18
   case 7 — the §7j.42 k7 gloss: steer + sin/cos move, engage
   leash (d+1)·0x40+600, point-blank dist<0x50 projectile 0x69 at
   the 32/16/8-frame cadence), committed as RE notes BEFORE any
   engine change. (b) Land stage_critters section 7 acceptance +
   the controller in bedlam-core::critter with unit tests; re-pin
   the census (the CloseCombatxNN component drops; expect NO row
   flip clean unless a host carries no other unmodeled state —
   then documented + deliberate). Bounds: census green after the
   re-pin; bedlam-core + bedlam-game suites green; no canonical
   chain movement (ZONEA/M1 hosts no S7) unless a row changes
   (then documented + deliberate); fmt + clippy; gates-validator
   22/22; MANIFEST clean; no Ghidra run; Nudge-Worker trailer.
## Done
1. DONE (2026-08-28, worker 5ee1c0ce claim 1, commits 199373a +
   cb3a3f5, both PUSHED): P5 `p5-critter-state-g2-shooters` — the
   G2 SHOOTERS state LANDED (the census Shooters class CLOSED with
   ONE deliberate row flip, D185). (a) RE FIRST (199373a, §7j.74,
   objdump-only from the committed exw-critterpoi-loader.txt +
   exw-text-objdump.txt — no Ghidra run, no corpus read): the .NME
   S1 loader walk made exact — spawn count w1+d CLAMPED >=1, two
   scatter(5) draws per attempt, the MAP-BOUNDS DROP GATE (a NEW
   pin: out-of-map attempts leave no critter but consume both
   draws), the stamps (species 1, z FIXED 0xC000 Q13, heading 0,
   anim RandA&7, variant pick(4)+3 NEGATED by the w2 flag, hp
   0xAF+(m·0xAF)/27 at the 0x4165db imul site, the dead +0x72
   timer stamp (RandA&0x1F)−0xF), draw budget 2/dropped attempt +
   5/landed critter; the k2 controller body 0x415216..0x415466 —
   species substeps, the heading precession by the SIGNED variant,
   the (cos/sin·0x14)>>8 sine walk (no bounds gate/wall probe/z
   change), the 1/128 SQUAWK pulse gate (FUN_0043a48e draw-free,
   the 0x4152bd reader identified) + the 1/4 fire gate (RandA&3==0
   — CORRECTS §7j.17's "every 4th substep"), the fire arm (the
   bounded robot-slot pick over 0x46ccbc + the +0x7C alive gate,
   FUN_0041286f pinned as the FIRST-FREE 0x4cc654 allocator, the
   ±0x1F00 jitter aim, the 2-D octile range gate 300−(2−d)·0x40
   with dz DEAD for the gate, the 0x65 stamp with the RAW
   direction>>5 velocity — NOT normalized); the kind-2 z cell is
   Q13 (the documented exception to the record's Q5-z rule). (b)
   ENGINE (cb3a3f5): stage_critters accepts section 1 + the k2
   body in bedlam-core::critter with 10 unit tests; the variant
   record field NOT serialized (no blob change) and no canonical
   scenario stages S1 — ZERO chain movement. (c) CENSUS RE-PIN
   (deliberate, D28): the ShootersxNN component dropped from all
   17 hosting rows AND ZONED-MISSION5 FLIPPED CLEAN — the one host
   whose only unmodeled section was Shootersx4 (the queue's "no
   row flips clean" expectation falsified by that row, documented
   + deliberate; provenance docs/evidence/p5-g2-shooters-census-
   table.txt); G2 residue = Chasers + CloseCombat + the S8
   personnel bank (25 missions); 12/37 load clean, the ledger
   stays 1/37 green; P5-ZONE-GATES §6.1/§6.2/§6.3/§6.4
   re-baselined; D185. Verified: bedlam-core + bedlam-game release
   suites green (canonical_dump_gate 13/13 + differ_gate 4/4 +
   determinism green); fmt + clippy clean on the touched crates;
   gates-validator 22/22; inspect baseline ok (1069 files);
   MANIFEST clean before AND after every corpus read; no Ghidra
   run. Queued: items 1-2 above (the Chasers unit is the new head;
   the CloseCombat unit second — after it, the S8 personnel bank
   is the last G2 class).
1. DONE (2026-08-28, worker cef2f815 claim 1, commits 51933bd +
   d4f7609, both PUSHED): P5 `p5-zone-bin-variant-g3` — the G3
   zone-BIN variant RE unit CLOSED with a NO-SWAP verdict (D184):
   the EXW runtime ALWAYS loads the zone-level MISSION{L}.BIN —
   the three shipped mission-number variant banks
   (ZONEB/MISSION6.BIN, ZONED/MISSION5.BIN, ZONEE/MISSION6.BIN)
   are runtime-dead editor residue. (a) RE FIRST (51933bd, §7c.9,
   objdump-only from the committed exw/exd-text-objdump.txt — no
   Ghidra run): build_mission_paths@0x44670c..0x446907 walked
   whole — path2@0x4dca8c (the .CGR/.BIN/.MIN/.LNG/.LNK base) =
   EDITOR\ + ZONE + chr(0x40+[0x4edd8c]) + \MISSION +
   chr(0x40+[0x4edd8c]): the zone letter appended TWICE, NO itoa,
   NO conditional (the function's only branch stays the G1/D183 +5
   on path1's mission number when [0x4edb88]==2); the .BIN
   consumers are exactly TWO, both on path2, both builder-fresh
   (load_mission@0x41dcbc tag 0x4587e8 + the brief-reload twin
   FUN_0044661b@0x446644 tag 0x45979a, tags byte-read from the
   PE); the joined name lives in the concat-private 0x40-B buffer
   0x4dca4c (one 3×0x40 family, only concat@0x41dbed touches it);
   a complete 29-site path-buffer census (path1 = .TOT/.DAT/.PAD +
   .MRK/.NME/.TRT/.POS/.BDG + the GAMEGFX\BRF_{L}{level} movie
   scratch + the save-path reuse; path2 = the five family tags
   only) + a whole-image ASCII string census (NO hardcoded
   ZONE?\MISSIONn.* literal anywhere; the 8street boot check
   EDITOR/ZONEA/MISSIONA.BIN is reconstruction-side, not an EXW
   literal); the EXD twin agrees (load block 0x2e5c3, builder
   0x58606, .BIN on path2 0x92f34, tag table byte-verified at
   linear 0x862a9 = file 0x9eaa9 delta -0x18800, builder tail
   letter-only into the epilogue jmp 0x51d12). Data corroboration
   (read-only, MANIFEST clean before AND after): only zone-level
   .MIN ship, each 16× the ZONE-level BIN count (B 1872 / D 1450 /
   E 1455 — never the variant counts 1443/1443/1120; a swap would
   desync the minimap walk); ZONEB/MISSION6.BIN ≡
   ZONED/MISSION5.BIN byte-identical (sha256 5735b08a3e08853e...,
   2,189,466 B, count 1443 — a shared dev/deathmatch bank);
   ZONEE/MISSION6.BIN (1,508,806 B, count 1120) likewise distinct
   from MISSIONE.BIN (1,968,763 B, count 1455). (b) VERDICT =
   NOT a swap: engine UNTOUCHED (mission_asset_names'
   {ZONE{L}/MISSION{L}.BIN} rule VERIFIED correct as-is); census
   NOT re-pinned (the loads were already zone-level and green; the
   G3 mention was a docs-side open flag). (c) PROPAGATION
   (d4f7609): RESEARCH-8STREET OPEN QUESTIONS #3 ANSWERED + the
   §1.0/§1.1/§7 8street glosses corrected (the "loaded only when
   the mission has its own" gloss was wrong — superseded by the
   EXW anchor per the 8street policy); FORMATS-MISSION §0.2 (the
   zone-level-only rule) + §23 (the MIN count corroboration);
   P5-ZONE-GATES §6.2/G3 CLOSED (no engine change) + §6.3 row
   notes + §6.4 rollup + the confidence tags; DECISIONS D184.
   Verified: gates-validator 22/22; no Rust change (fmt/clippy
   N/A); MANIFEST clean before AND after every corpus read; no
   Ghidra run; no ledger or canonical-chain movement (docs-only
   unit). Queued: the G2 Shooters unit as the new head + the G2
   Chasers unit second (both consumed above).
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
   Ghidra run. Queued: the G3 BIN-variant RE head + the G2
   Shooters unit (both consumed above). Watchdog repair 1007791
   (2026-08-28): this worker was grace-killed at the 240s boundary
   while re-verifying a51d4f2 — the gates-validator battery runs
   after the bookkeeping commit by contract — so the repair landed
   the push, archived the false preflight-mismatch failure, and
   widened the boundary grace 240 to 900 in this repair commit.
