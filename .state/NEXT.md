# NEXT - task queue (top first; rewrite this file at end of every run)

QUEUE CONVENTION (2026-08-22, D106): a completed unit's entry MOVES to
the '## Done' log at end of run - never stays in '## Now' as 'N. DONE ...'
(the scheduler mechanically skips a first-word DONE marker, but the
renumbered queue keeps every open item claimable by number).
## Now
1. [P4/static-parity/S0-15] the `static-order-table` TS row (order
   table EXD 0x91ee4 / EXW twin): whole-writer/reader census +
   independent static-oracle coverage (the S0-07..S0-14 pattern), or
   a loud named gap if a seam stays unmodeled. NOTE: the worktree
   carries interrupted O1-boot WIP (dbx-plan.rs boot_trap/entry +
   dbx-capgen.py + dosbox-harness.sh + RUNTIME.md + the capture-plan
   boot_trap deltas, owner ≠ current worker): inspect, preserve,
   adopt per AGENTS shared-worktree rules; stage explicit task paths
   only.

## Done
1. DONE (2026-08-25, worker 9c711d0c claim 1, commits a335220 +
   4e0af96, PUSHED): P4/static-parity/S0-14 the s0-trigger/
   frame-counter ordering RESOLVED + the DYNAMIC-ONLY placement class
   (D156, RE-EXW-SIM §7j.66). (a) RE first (a335220): the EXW
   MissionShell tail decoded whole — pause gate 0x4485de, the
   NORMAL-path PresentEnd CALL 0x4486c9 (THE O2 dump point;
   PresentEnd FUN_00425a03 has 62 direct call sites, so the function
   entry the registry exw_addr names is NOT a usable trigger — the
   W11 deferral resolved, the O2 plan regen moves trigger.site
   0x00425A03 -> 0x004486C9), pause-path present 0x44861f, and the
   register-form counter increment 0x4486ce-da ALWAYS after the flip
   — the EXD 0x5a6eb/0x5a6f0-fd twin order IDENTICAL; exactly one
   present + one inc per pass both paths. D81 CORRECTED: the eight
   bounded cinematic screens RESET the counter (xor+mov, the INC-only
   census trap) and reuse it as their 100/200/300-frame duration
   timer; the five interactive menu screens count cumulatively — so
   C0 = the mission-entry counter value is a DETERMINISTIC FUNCTION
   OF THE SCRIPTED MENU WALK, not a boot-frame total (T2 budget
   consequence unchanged; the EXD menu reset family = open
   cross-check, not blocking). Dumped value = pre-increment =
   C0 + (k-1); O1/O2 = E + C0 (E already emits sim.frame()-1).
   (b) The classification (D156): s0-trigger (extent 0, the dump
   point itself) + frame-counter (the T2 timing cell) carry no
   statically-closeable state — they close by MECHANISM under the new
   dynamic-only placement disposition, tracked separately from static
   closure: strict S0 = 22 static-closed + 2 dynamic-only + 3 static
   remaining (S0-15/16/17) = 27. (c) Oracle (4e0af96):
   static_frame_counter_differential.rs, 6 tests — the tail
   transcription as a state machine (pre-increment, one inc per pass,
   pause presents but does not fire the BP), the census pins (13+1
   increments, the eight cinematic rows, 62 call sites, the walk
   model falsifying the boot-total reading), the differ tie-in (the
   transcribed O1 model vs E lands exactly on Class::T2Reported —
   PassWithNotes, zero engine-bug/structural/coverage; identical-
   script double-run byte-exact), the canonical E assertion (counter
   == frame_no strictly from 0). watches.toml s0-trigger/
   frame-counter layouts re-anchored (plan-neutral — layout strings
   never feed plans); differ.rs alignment note corrected in place;
   DESIGN §6a row + the s0-trigger E-gap note. Verified: workspace
   release tests green (incl. differ_gate 4 corpus lanes +
   canonical_dump_gate 13), fmt + clippy clean on touched files,
   MANIFEST clean before AND after (no corpus read). The unrelated
   O1-boot WIP preserved untouched (unit staged only its own paths).
   Queued: S0-15 as item 1.
1. DONE (2026-08-25, worker 77b1c512 claim 1, commits b2e522c +
   dc6c99d, PUSHED): P4/static-parity/S0-13 the RNG pair +
   dither-noise rows independently covered (D155, RE-EXW-SIM §7j.65).
   (a) RE first (b2e522c): RandA/RandB @0x402975/0x4029b6 decoded
   whole — the 40-bit dl:ax:bx shuffle+rcr chain with the S>>25
   DISCARD gives the closed form S' = ((S<<7)+S+0x361962E9) mod 2^32
   (shift-7, NOT a wrap rotate — the 8street "ror33ish" gloss
   retired/re-anchored); return = the NEW HI word (u16). Complete
   writer censuses: boot plants BOTH seeds (0x41c0cd B=234567 /
   0x41c0d3 A=123456), MissionShell reseeds A ONLY (0x447728); dither
   cursor := 0 per mission (0x4478f7), fill = exactly 2048 RandB
   draws (0x447b13..3a), churn = 15 draws/frame advance-then-draw
   (0x448147..95), blit reads only; call census 158 A / 27 B direct.
   (b) Oracle (dc6c99d): bedlam-game/tests/static_rng_differential.rs
   — the instruction-faithful step cross-proven against the closed
   form (128 states + edges), first-eight literals both chains, the
   A-only reseed seam, the fill/churn/blit-seed literal tables (post-
   fill B 0xA564DC47, 526/2048 white, churn frame → 0xF52E04EE);
   sensitivity proven BOTH directions in-memory (one-ulp add-tail →
   cross-proof fails; wrong shuffle → 5 literal pins fail). (c) E-side
   CLASSIFICATION, not Rust-determinism-as-oracle: the canonical
   seed=0x1e240 stand-in pin, 8-byte row presence + liveness, and
   static-dither-noise DELIBERATELY absent on E (D17 presentation
   half, the D149 no-fabricated-parity precedent) — the rows close
   ORIGINAL-side under the charter T3 never-bit-compared class.
   watches.toml layout notes corrected plan-neutral; DESIGN-
   DIFFHARNESS RNG row re-anchored. Strict S0 coverage 22/27 (5 rows
   remain: S0-14..S0-17 + the s0-trigger tier row). Verified:
   workspace release tests green, fmt + clippy clean, MANIFEST clean
   pre+post; the unrelated O1-boot WIP untouched. Queued: S0-14 as
   item 1.
1. DONE (2026-08-25, worker 52f0a9f0 claim 1, commit 0e7d245,
   PUSHED): P4/static-parity/S0-12b the fresh-session campaign/config
   seam LANDED (D154) — the three D153 gaps closed BOTH sides. (a)
   canonical.rs: fresh difficulty default 1 (§7j.64/A 0x41c14a; `boot
   difficulty=d` now OVERRIDES a default — an explicit d=0 is
   expressible again), the campaign seed on EVERY run (money 3500 at
   d=1 through start_score + the sim damage rows at the original's
   fresh tier), and linear-mission-m emitted through the DERIVED cell
   clamp(5·(zone−2)+mission−1, 1, 26) from the CURRENT mission_slot()
   (§7j.64/D) — never episode().linear(); the destroy staging's TRT hp
   tier selector reads the same derived value (m=1 tier 259).
   (b) Acceptance: the three LOUD gap assertions in
   `static_campaign_config_differential.rs` flipped to equality pins
   (verified failing-then-passing around the seam); all eight §7j.64/G
   rows closed both sides. (c) The deliberate full-chain re-baseline:
   all 11 canonical corpus chains re-pinned (S0 5ab9df44ca3ba0c6, S1
   0224dcc5f4631460, S2 04dfa60b7262a474, S3 95375e99ba27990a, S4
   a8deea56f9308102, S5 359d9131fb51a86c, S5B 18a27532aeb7858e, S5C
   0095d08b9f92d51b, S6 7c4437ee14e9c7ab, S7 f8e83317ca7c5f8a, S8
   0d1482d01f57b2b1; synthetic 9e5efdc3fff70d88 unchanged) — live O1
   comparisons pin against these from 0e7d245; differ_gate coverage
   counts UNCHANGED on every scenario. (d) Difficulty-1 content
   re-derivations: S4 turret tier 250→259 (ring-0 destroy −4741), S5C
   money folds (3650/3710), S8 restaged whole (17 critters = 7 kind-5
   + 10 kind-4, hp 155/207, the 0x68 lane 150/hit, 900-frame respawn
   table) + ONE latent test bug fixed (the S8 hit-flash walk read
   stride 0x54/+0x2E — the 94-B record +62 is the pinned hit_flash).
   (e) Docs: DESIGN §6a/§6/§7/§10-W12 corrected in place + the D108
   supersession note + D154 + the §7j.64 landing note. Verified:
   workspace release tests green (incl. canonical_dump_gate 13 +
   differ_gate 4 corpus lanes), fmt + clippy clean, MANIFEST clean
   pre+post. The unrelated O1-boot WIP preserved untouched. Strict S0
   coverage stays 19/27. Queued: S0-13 (rng/dither) as item 1.
1. DONE (2026-08-25, worker 0f91b0d7 claim 1, commits cda35f2 +
   ea745fd, PUSHED): P4/static-parity/S0-12 the eight fresh-session
   T0 campaign/config rows independently covered — FIVE closed both
   sides, THREE named gaps queued (D153, RE-EXW-SIM §7j.64). (a) RE
   first (cda35f2): whole-objdump write-form censuses per cell +
   instruction-level decodes — the GameMain boot head (mode:=0
   0x41c145, DIFFICULTY:=1 0x41c14a — the §7j.15 "campaign-start"
   gloss CORRECTED), the episode slot boot (zone:=1/mission:=1/
   score:=FUN_0043a5fc-fresh-0), the name-entry fresh arm (money :=
   4000−500·d, 0x43aaca — fresh boot = 3500), the linear-mission-m
   DERIVATION clamp 5·(zone−2)+mission−1 ∈[1,26] (0x41c520..556, 3
   writes all GameMain — NOT a counter; fresh = 1), and the SOUND
   loader default 1 (bounds [0,1] default ecx=1). (b) Oracle
   (ea745fd): `bedlam-game/tests/static_campaign_config_differential.rs`
   (first static oracle in bedlam-game — the rows' E half IS the
   canonical harness): the original-side transcription consts + the
   linear derivation spot table + all-37-mission census (3 floors,
   max 26, sum 482) + the E-side comparison on the S0 anchor —
   score/mission/mode/zone/sfx CLOSED; difficulty/money/linear pinned
   LOUD with the original value named in each message (S0-12b flips
   them); the boot-key seam proof (boot difficulty=1 → money 3500
   through start_score — the gap is the DEFAULT, not the mechanism).
   (c) watches.toml money/difficulty/linear layout notes corrected
   (plan-neutral — zero plan bytes moved). (d) Coverage: strict S0
   now 19/27 rows; the mandated 27-row registry re-audit CLOSED the
   predecessor's off-by-one (27 = s0-trigger + 11 T0 + 15 TS;
   remainder was 16 at D152, now 8: S0-13..S0-17). Verified: 35
   release suites green (bedlam-game + diffharness + bedlam-core),
   fmt + clippy clean, MANIFEST clean pre+post. The unrelated
   O1-boot WIP untouched. Queued: S0-12b (the seam unit) as item 1.
2. DONE (2026-08-25, worker ee030ded claim 1, commit 545e7f6,
    PUSHED): P4/static-parity/S0-12a dbx-plan `static-min-bank` extent
    RESOLVED (D152) — the deferred arm
    `"static-cgr-volume" | "static-bin-terrain" | "static-min-bank"`
    split; `static-min-bank` → `Form::PtrCell { cell, len_expr:
    "0x7530" }` under a dedicated arm guarding the new watches.toml
    extent "0x7530 (30000 B)" (moved off "bank-sized" per the pinned
    ArenaAlloc 7j.62/D149); new resolve symbol `min_ptr` in both
    PtrCell maps; all 13 capture-plan artifacts regenerated (the
    `_deferred` entry gone, anchor row `{ "id": "static-min-bank",
    "addr": "$min_ptr", "len": "0x7530" }` + resolve row added on both
    channels); count asserts re-pinned (s0 21/5, s1 21+17/5, o2 37/6
    symmetry, s2 21+17, s3 +10/8, s4 +10/19) + a min_ptr/0x7530 span
    assert. cgr/bin-terrain stay deferred (sizes unpinned). NOT a
    strict-coverage row (infra hop): strict S0 stays 11/27.
    Verification: full diffharness suite green BOTH on the exact
    committed content (scratch-crate simulation of staged blobs) and
    on the live worktree; fmt + clippy clean; MANIFEST clean. The
    unrelated O1-boot WIP (dbx-plan.rs boot_trap/entry, capgen,
    RUNTIME.md, artifact boot_trap deltas) was deliberately NOT
    staged and remains intact in the worktree for its owner. Queued:
    S0-12 (the eight T0 campaign/config rows) as the new head.
2. DONE (2026-08-25, worker ab778f23 claim 1, commit 7760294,
   PUSHED): P4/static-parity/S0-11b THE CLAIM-BANK STAGING SEAM —
   `static-claim-bank` closed BOTH sides (D151). (a) The rect farm
   promoted tests/data→`bedlam-core/src/claim_rects.rs` (byte-identical,
   pinned by the new `promoted_rect_farm_is_byte_identical` oracle
   test); `MissionSim::stage_claim_bank(zone_set, mission)` = the
   §7j.63/C initializer transcription (memset-0 + the ACTIVE-PREFIX
   door-rect stamp, line[y]=y·map_w from terrain.size()), called at
   EVERY `GameHost::load_mission` (the unconditional 0x447b85 call —
   no scenario key, no RNG draws, state_hash untouched). (b) Reader
   gates: `stage_splash` + `platform_tile_build` read the byte in the
   §7j.63 gate order; the THIRD §7j.63/F reader (the FUN_0042382c
   death-blast smoke producer) is HOST-SEAMED presentation (§7j.24) —
   no sim gate fabricated (the F phrase corrected, §7j.63/F-bis);
   `claim_seam_tests` pins the refusal on rec-0's (2,51) tile + the
   unstaged/unclaimed/A-M2 controls. (c) Oracle parity BOTH sides:
   `claim_staging_matches_the_independent_image` 37/37 shipped
   missions byte-identical (synthetic terrain of the TOT dims). (d)
   The canonical `static-claim-bank` TS row emits the RAW 10000-B
   image, anchor frame only — byte passthrough on all channels, ZERO
   differ changes, differ_gate coverage pins pass UNCHANGED. (e) ALL
   canonical chains re-baselined deliberately (fixture
   1335f953d7da3c82, synthetic 9e5efdc3fff70d88, S0 b9b57b68e95f482a,
   S1 da833e535f833dcc, S2 43110d921137da19, S3 fdd9fae3de7a3ef9,
   S4 f35b5e45b26891ea, S5 744950e2d3753d04, S5B 28bfea820bfb05ac,
   S5C be8cf733f1d078c2, S6 80066717ee97b67f, S7 9b81586f58687994,
   S8 acced68c68c14fa6 — live-session O1 comparisons pin against
   these from 7760294). (f) Corpus reachability ANSWERED by the
   UNCHANGED timeline asserts (S4 splash ring, S7 builds/k7/creep,
   S6/S8 events): no staged S0..S8 scenario lands on a claimed tile —
   the chains moved via the TS row ONLY. Strict S0 coverage stays
   11/27 (the row was counted by D150; this closes its Rust half —
   the FIRST S0 static row closed both sides). Workspace green
   (bedlam-core 239 + bedlam-game 41 + differ_gate 4/4 corpus 672s +
   full-workspace run; the 4 remaining clippy test warnings pre-exist
   at HEAD); fmt clean; MANIFEST clean. Queued: S0-12a dbx-plan
   min-bank extent (item 1).

## Backlog (not yet started)
- S0 static-parity closure baseline: strict independent coverage is
  22/27 rows (D155) — the 11 TS rows from bd91c10, 56918c5, 390acb9,
  cd70efe, 920aec2, fcb8fb2, cec30a7, 2646ce8, 76a14c6 + the eight
  T0 campaign/config rows from ea745fd (S0-12, closed both sides by
  the D154 seam) + the three RNG/dither rows from dc6c99d (S0-13,
  closed original-side under the charter T3 class — D155). The
  27-row registry: tier-S0 s0-trigger + 11 T0 + 15 TS; the remainder
  (5 rows) is S0-14..S0-17 + the s0-trigger tier row. `static-min-bank` (S0-10) is
  CLOSED original-side only: Rust retention deliberately none —
  presentation-half D17; the display-phase producer stays queued,
  not covered (D149); its dbx-plan extent infra hop landed separately
  as S0-12a (545e7f6, D152 — not a row). S0-12's three named gaps
  (difficulty/money fresh defaults, the linear derived-cell) LANDED
  as S0-12b (0e7d245, D154) — all eight T0 campaign/config rows now
  closed BOTH sides (the E half through the canonical seam, not a
  second oracle).
- [P4/static-parity/S0-14] resolve `s0-trigger`/`frame-counter` ordering
  and classify dynamic-only row placement separately from static closure.
- [P4/static-parity/S0-15] `static-order-table` — independently
  reconstruct fresh-session/loadout/order-table post-init state and
  compare it against the Rust target. Do not fold this row invisibly
  into the generic T0 campaign/config unit.
- [P4/static-parity/S0-16] `static-player-type` — independently pin the
  original fresh-SP value and writer semantics, then compare them with
  Rust construction. MP/config variants are explicitly excluded from
  this slice and require a later named task before any MP closure claim.
- [P4/static-parity/S0-17] `static-cursor-clamp` — statically verify the
  EXD-only 240x320 clamp maxima constants/formula and compare them with
  the DOS/classic-input adapter. If no such target exists, explicitly
  classify the row as hardware/input-profile-only rather than semantic
  engine state; never silently count it as parity-covered.
- [P4.2/W7-followups] after the differ core: the T2/T3 field maps on
  the E side (projectile/critter banks, effects/debris rings) as
  their producer families land in-engine (S3+ pairing per §10-W12);
  the O2/Wine tiebreak channel (W11); O3 8street comparator (W10).
- CLOSED by 7j.27: the DROPSHIP ring producers (writer census,
   animator map, 7×5 grid correction, latch census, the 0x4c71f4
   pass head). CLOSED by 7j.26: the [0x4ede24]/[0x4ede28] "7×7 screen-address
  table" question — it is the terrain RESTAMP list (count + 3-dword
  {dest row, tile-x, tile-y} records, blitted via FUN_00401471;
  writer FUN_00440a2d = the scroll/camera restamp stager, confirming
  the hypothesis). CLOSED 2026-08-23 by 7j.49/D121: FUN_00440dc2 =
  the BRIEF objective-minimap SNAPSHOTTER (sole caller FUN_0043dc65,
  the per-objective brief panel; the drawer sites 0x440d1c/0x440d93
  belong to callee FUN_00440c34; BRIEF-only — the whole
  FUN_00440a2d/FUN_00440c34 family never runs during the mission
  render pass; [0x4ede24] is a per-screen cell reuse). CLOSED by 7j.17: the [0x4edd60] height-bank family and the
  projectile z-encoding census. CLOSED by 7j.24: the critter
  death-handler family. CLOSED by 7j.25: the destroy-tail
  effect-entry map + the 160-vs-0xA8 stride anomaly + the
  .POS/.BDG loaders + the .BDG grammar (FORMATS §12/§16).
  CLOSED 2026-08-23 by 7j.50/D122: projectile type 0x69 vs the
  FUN_00419aff damage table (else path dumped — inline jump tree,
  no memory table; the beam re-keys to literal 0x65, terrain-only,
  never robots; no caller ever passes 0x69).
- CLOSED 2026-08-23 by 7j.46/D117: the per-zone FUN_00433980 case
  table (all zones/modes + the ride-record bank grammar + the 21
  beacon slots + the zone-F/G EXIT pairs + zone E verified negative)
  and the FUN_00424a6f message system (the LANGUAGE.* section walk,
  the 15 BOOT_CAMP ids, the latch/timer semantics) — the promoted
  item 2 completed (commit fcf97c3).
- The 0x4787c4/0x47879c hot-rect record — CLOSED 2026-08-22
  (§7j.31/D95): ONE 0x20-stride array base 0x4787bc, grammar +
  7-writer census + octile picker + class dispatcher landed; SP
  click-orders never robot-targeted; new pins 0x46cc00/0x4ddb20
  (watch-set candidates for click parity, additive when needed).
- RETIRED 2026-08-22 (D93/§7j.29): the ".MOFO loader" — never
  existed (string-tail misparse). REMAINING from this bullet:
  the .BLD record walk (names/graphics
  side; FORMATS §17 — CLOSED 2026-08-22 by §7j.33/D97, editor-only)
  + the .BDG template-bank plane↔mirror-word mapping — the
  parenthetical here ("@+0x3E/+0x42 readers still open") was STALE
  and caused the 2026-08-23 queue re-queue caught by hygiene #3:
  CLOSED 2026-08-22 by §7j.32/D96 (+0x46/+0x4A the only consumed
  banks; +0x3E/+0x42 = dead editor payload; independently
  re-verified at HEAD 2026-08-23 by D118).
- RETIRED 2026-08-23 (D115/§7j.44): the "debris-stager ENGINE
  widening" bullet — k2/k8 scorch + the k1/k20 ring landed with
  the 7j.11 stager, the +0x20 physics classes landed as the
  FUN_0040de9c family (commit cebc178; the 0x454510 census task
  closed BY DISPROOF — no param table exists).
- Keyboard latch wiring for the sidebar (F1/F2/F3, keys 1..7,
  MSpace; RE-EXW-INPUT line 95) - blocked on the P2e InputFrame
  button bit-map assignment.
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence
  — CLOSED 2026-08-23 (§7j.56/D128: the family is REGISTRY-
  backed, HKCU\Software\Mirage\Bedlam\1.00 via
  RegCreateKeyExA/RegQueryValueExA/RegSetValueExA; the
  "CONFIG.BDL" name RETIRED — zero binary refs; the loader is
  FUN_004252c0 @boot + the saver FUN_0042540c at the
  name-entry exit, both decoded),
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- Mission SFX tier (MENU1/MENU2-style mixer instruments; the
  bank→name DATA PREREQUISITE DELIVERED 2026-08-22 by §7j.30/D94 —
  202 durable assignments, zero unnamed cells) + the order SFX 0x2A armer click + the
  damage/alarm SFX families (7g.1) + the pickup SFX 0x43a48e
  entries (7h.2) + the select-ack SFX pair 0xC+k/0xF (7j.6) + the
  debris arrival-SFX pair FUN_00421e60/FUN_00421dec — CLOSED
  2026-08-23 (§7j.52/D124, commit 01d380b: BOOM1/2/3 trio +
  RICOCHT1..4 quad, RandB pick — item 4's "RandA" corrected,
  stage-time trigger, corpus reach = k5 only; the §7j.42
  FUN_00421ed6 [identity open] gloss closed in the same unit,
  commit 2728351). The select-ack pair + armer click — CLOSED
  2026-08-23 (§7j.53/D125, commit 38a8463: FUN_004239ef decoded
  whole = the RADIO-WARNING poster, 4-channel queue 0x4eb954 +
  consumer FUN_00423a85; the 53 ids = the LANGUAGE.*
  [WARNINGS] lines, all 55 sites named; the "select-ack" pair is
  the DANGER-TARGETTED/BOMBARDMENT warning; the armer click 0x2A
  = "EVACUATION COMMENCED"; take A/B = RandA bit0; FORMATS §22 =
  the LANGUAGE.* container grammar).
  NOTE 7j.17 pinned new FUN_0043a48e banks: _DAT_004edf94/
  _DAT_004edfe4/_DAT_004edfac (robot fire) and
  _DAT_004edffc/_DAT_004edff0/_DAT_004edfa8 (critters/POI).
  NOTE 7j.20: the beacon armer's SFX is FUN_004239ef(0x2a,3).
  NOTE 7j.25/7j.30 CLOSED: the destroy-thud pair 0x4edfb8/
  0x4edfbc = DEADMAN1/DEADMAN2.RAW and the FULL bank-name walk
  landed as §7j.30 (commit a0f291c, D94).
- The pickup tile-word PRODUCER — PROMOTED to the Now queue
  (7h.3, item 3): the 0x4796bc mirror-row semantics it needed
  landed complete in §7j.34/D98.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items: CLOSED 2026-08-22 (§7j.34/
  D98): the type-DB tail producers (the door animator family + all
  reader anchors + +0x1D padding). CLOSED 2026-08-22 (§7j.36/D101):
  the BIN u32[bank+0] header word (sprite COUNT → the write-only cell
  0x46cdb8; the [0x4ede1c] bank's content consumers = the vestigial
  radar stamp — no differ row). CLOSED 2026-08-22 (§7j.35/D100):
  the u32[0x456ca8] anim sequence + the water flag producer
  (STATIC ping-pong const + flag ≡ 1 for every mission — the
  0x12d/0x12e/0x12f flush remaps may hard-code water-ON; a stale
  re-queue of this closed unit was caught + removed by queue
  hygiene 2026-08-22, D111). SEC 8 IS NOW FULLY CLOSED (all four
  items). CLOSED: u32[0x4dd444] (7e.4 - the PALTRAN
  ramps); +0x18 producer (7j.8/7j.9 - FUN_00422287, reader raw,
  ring landed D57).
- MISSIONVIEW sec 5d tail notes: ROBNUMS name plates,
  Shield/Variant bank staging — CLOSED 2026-08-23 (§7j.48/D120,
  commit dd8d5e2: TELEPORT/SHIELD label corrections, banks
  alloc+load at every MissionShell head — SP included, ROBNUMS =
  dead data, TINYFONT plates MP-gated, unstaged-flush clause
  RETIRED — no bank-zero skip exists anywhere in enqueue/flush).
- RE-EXW-SIM sec 9 open items 2-3: CLOSED 2026-08-23 (§7j.45/D116,
  commit 47357ca — the FUN_00440e45 SHOP identity + the robots()
  extra-phase/state-1 producers; the promotion note superseded).
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DESIGN DOC LANDED 2026-08-22 (docs/DESIGN-DIFFHARNESS.md, D77, commit
  7bc2c9d) — oracle topology (O1 EXD/DOSBox-X primary instrument, O2
  EXW/Wine canon tiebreak, O3 8street second comparator), tiered watch
  set (every address ledger-anchored), seam injection (COMMAND records/
  orders/.PAD step-on — never raw input), canonical-record differ, gates
  DH-G0..G3, build order W1..W12 (W1 = EXD import + address map, now the
  head item). The doc also arbitrates the two 7j hypotheses (the debris 2k
  start delay and the blink-cursor-from-spawn question) + the 7j.9 overlap
  last-write-wins read of the five rings. NOTE 7j.20: the harness
  must model the mission-start pod-descent stagger (w@+0x2C =
  1+k·(2000−m·1000/27)) — the first seconds of any mission have
  the robots frozen in pods (7j.27: descent ≈41 frames, pod phase
  2 = one tick, release = state 6) — and arm extraction via a scripted
  .PAD step-on, not a click. NOTE 7j.22: weapon fire needs
  injected COMMAND records (FUN_00449c94/0x4dd4a0) or order
  dispatch, not raw input — the fire family is fully anchored
  for it (per-type cadences + damage tables). NOTE 7j.25: the
  destroy family is now fully decoded end-to-end (resolver →
  restore → 5-effect loop → chain walks), ready for the harness.
- TOT semantics follow-up: FORMATS sec 2 plane 6/7 (the ~2000-slot
  POS linkage) — CLOSED 2026-08-23 (§7j.47/D119, commit dc6f5bf):
  planes 6/7 = ordinary z-levels of the word stack (tall-structure
  tops; they DRAW ungated — no z≥6 gate in any consumer), the
  POS-slot linkage REFUTED, FORMATS §2 closed.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-25: P4/static-parity/S0-11b COMPLETE (worker ab778f23
  claim 1, commit 7760294, D151). The D150-queued claim-bank staging
  seam LANDED — `static-claim-bank` closed BOTH sides (the FIRST S0
  static row so closed). Rect farm promoted to
  `bedlam-core/src/claim_rects.rs` (pinned byte-identical to the
  oracle copy); `MissionSim::stage_claim_bank` = the §7j.63/C
  initializer transcription, run at EVERY `load_mission`;
  `stage_splash`/`platform_tile_build` read the byte (the third
  §7j.63 reader is host-seamed presentation — no gate fabricated);
  oracle actual-side test 37/37 missions byte-identical; the
  canonical TS row emits the raw 10000-B image (passthrough all
  channels, differ untouched); ALL canonical chains re-baselined
  (S0 b9b57b68e95f482a .. S8 acced68c68c14fa6 — live O1 comparisons
  pin against these from 7760294); corpus reachability answered NO
  (no staged S0..S8 scenario lands on a claimed tile — chains moved
  via the row only, proven by the unchanged timeline asserts).
- 2026-08-25: P4/static-parity/S0-11 COMPLETE (worker eeafac37 claim 1,
  commits 2646ce8 docs + 76a14c6 test; D150). The `static-claim-bank`
  row independently covered ORIGINAL-SIDE with a CONCRETE Rust staging
  gap queued (S0-11b, the new item 1). RE-EXW-SIM §7j.63: the
  0x46af58/0x119564 bank decoded whole — 7-site EXW census with a
  7-for-7 EXD twin census; the §7j.10 "ORDER marker family 0x425556"
  gloss RETIRED (it is FUN_004254e1's inner store — the MISSION-LOAD
  initializer: memset-0 of the whole 10000-B bank + the stamp of the
  ACTIVE PREFIX of the 45×0x10 door-rect list, no bounds checks); the
  rect source = FUN_0042c4a0's per-zone/mission HARDCODED store farm
  (zone table 0x42c484, mode gate, mission tables ×5 for zones 2..6,
  ==1-only zones 1/7) after the 0x447b7b whole-bank memset; a NEW 4th
  reader found (the radar marker-0xd gate 0x41f191); arena side
  re-verified (7th per-mission bump block after the 0x41d955 cursor
  reset; staleness moot). Oracle: 368-row pinned rect farm
  (tests/data/claim_rects.rs, concrete-interpreter transcription,
  three cases hand-verified) + the independent all-37-mission
  initializer transcription with census pins (per-mission counts, the
  exact ZONEA/M1 59-tile set, total 3049, 10 all-zero missions) +
  four-part sensitivity. Rust side deliberately absent (claim==0
  hardcode) — both halves of its justification disproven, destroy.rs
  comments corrected; the staging seam (gates + canonical TS row +
  chain re-baselining) queued as S0-11b. Strict S0 coverage 11/27.
- 2026-08-25: P4/static-parity/S0-10 COMPLETE (worker 95c99db8 claim 1,
  commits 0ebb184 docs + cec30a7 test; D149). The `static-min-bank`
  row independently covered ORIGINAL-SIDE; Rust retention deliberately
  NONE (presentation-half D17 — backbuffer-only, never engine state;
  no seam fabricated). RE-EXW-SIM §7j.62: arena alloc 0x7530 with NO
  memset @0x41dabd..0x41dac7 (EXD 0x2e3f0), verbatim whole-file load of
  the ZONE-scoped EDITOR\ZONEX\MISSIONX.MIN @0x41dcd8..0x41dcf3 (EXD
  0x2e641; no header/transform/cap), displacement census closed at 3
  sites with ONE reader = the 4×4 territory stamp FUN_00402ab8 (EXD
  0x12df3), cw = LNK/LNG word[TOT word], cw==0 transparent, XLAT
  MAPTRAN[variant] — the FIRST verified runtime consumer of the LNK
  permutation (FORMATS §5 anchor). Corpus: 7 zone files (A≡D), per-zone
  reachable sets under BOTH language gates (A 349 / B 1180 / C 1055 /
  D 1008 / E 954 / F 632 / G 271 union nz), max cw 1868, and the
  STALE-TAIL-NEVER-READ proof (every reachable cw·16+16 ≤ file size).
  Oracle `static_min_bank_differential.rs`: loader+projection
  transcription, per-mission max-cw identity pins (37), A≡D pin,
  LNK≠LNG variant pins, TOT type max 1868<8192, reader spot-proof;
  sensitivity = reachable-byte flip / dead-tail blindness / poisoned
  LNK bound catch / >0x7530 rejection. fmt+clippy clean, suite green,
  MANIFEST clean before AND after. Queued: S0-12a (dbx-plan extent →
  PtrCell 0x7530; file in flight with unrelated O1-boot WIP) and the
  display-phase map-overlay producer (D17/D50). Strict S0 coverage
  10/27.
- 2026-08-25: P4/static-parity/S0-09 COMPLETE (worker e473f5db claim 1,
  commits bad9ff6 docs + fcb8fb2 test, both PUSHED; D148). The
  `static-type-table` row independently covered: the EXW .BDG loader
  leg (FUN_0041a4f8 @0x41a5d6..0x41a7ef) + EXD twin FUN_0002adb4
  re-verified instruction-by-instruction (RE-EXW-SIM §7j.61) —
  table+arena memset pre-zero, control word staged at +0 before the
  test, count@+0x12 = nonzero selectors on ACTIVE rows only and
  WRITE-ONLY (displacement 0x4dee04 has one site = the loader store),
  four banks into consecutive arena slots in disk order (the §7j.32
  interleave lives only in the row pointer slots); ONE FORMATS §16
  corpus erratum (footprint max is (10,10,5), 113 distinct tuples, not
  (3,3,3)); the all-37-mission field-exact oracle vs the retained
  `ObjectTypeTable::from_bdg_bytes` with census pins (10434/7907/2527
  rows, selector domain 0+1..9, count census, arena span 6728..27288,
  hp min −1) and a six-part mutation sensitivity proof; the two
  write-only surfaces (count word, control word@+0) deliberately
  unretained, no seam fabricated. Strict S0 independent coverage is
  9/27 rows; next slice: S0-10 `static-min-bank` (queued as item 1).
- 2026-08-25: P4/static-parity/S0-08 COMPLETE (worker 2b25b994 claim 1,
  commits 8a054b3 docs + 920aec2 test, both PUSHED; D147). The
  `static-yline-zbase` row independently covered: both EXW table-build
  loops pinned instruction-by-instruction with ONE GLOSS CORRECTED —
  y_line is h dwords at 0x4ea900 (y·w for y in 0..h−1, bound h·4 under
  jl @0x41ddbe; the old §7c.3 "h+1 dwords" boundary entry is never
  staged or read), z_base is exactly 8 dwords at 0x4eaacc..0x4eaae8
  (z·w·h stored factored; the store-base cells 0x4eaac8/EXD 0x107714
  are the adjacent screen-scale family, not entries — watches.toml
  exd_addr/layout corrected). Census: the four stores
  0x41ddb1/0x41ddd9/0x4466c7/0x4466ef are the ONLY writers; SECOND
  producer pair @0x4466bd..0x4466f8 (FUN_0044661b, brief-screen
  loadout via 0x43d1a5: FULLFONT/BRIEF/palettes/SFX + mission
  TOT/BIN/DAT into fresh arenas) re-runs both loops verbatim, no
  0x302 copy/sweep/PAD on that path. EXD twin 0x2e713..0x2e74b:
  y_line 0x8b78c (h dwords), z_base 0x107718..0x107734. Rust retains
  NO bank (inline z·w·h + y·w + x in Terrain::dat_type) — the row's
  parity content reduces to the retained dims + exact extents, so the
  oracle compares a TOT-header-only transcription against a test-only
  representation from Terrain::size() across all 37 missions and pins
  the reduction invariant TOT[0..4]==DAT[0..4] on every shipped
  mission (original reads TOT header, Rust the DAT header), dims
  {25×75 ZONEA/M1, 100×100 ×35, 100×25 ZONEG/M1}, sizes 4+8·w·h, the
  volume identity z_base[7]+y_line[h−1]+(w−1)==8·w·h−1, zero anchors.
  No production seam (the tables are pure (w,h) functions; retaining
  them would be fabricated parity). Mutation sensitivity: w bump moves
  exactly y_line[1..h]/z_base[1..8] and fails the differential; h bump
  grows the y_line extent by one over a byte-identical prefix; DAT
  header bump → loader rejects. Workspace tests green, fmt+clippy
  clean, MANIFEST clean pre+post (un-piped root-CWD checks), no
  Ghidra run (objdump-only). Strict S0 coverage 8/27; next: S0-09
  `static-type-table` (queued as item 1).
- 2026-08-25: P4/static-parity/S0-07 COMPLETE (worker f25d060f claim 1,
  commits 848e2f7 + cd70efe, both PUSHED; D146). The retained PAD-slot
  bank row `static-pad-slots` independently covered: EXW staging loop
  pinned instruction-by-instruction (memset-0 pre-parse @0x41de62,
  x staged before the 0xFFFF check, terminator slot {0,0xFFFF,0,0},
  all-zero never-read tail; EXD twin 0x2e7a0 identical), watches.toml
  layout corrected, and an all-37-mission bytes-only oracle compared
  field-exact against Terrain::pad_slots with the pinned corpus census
  (level tally 701, live-run 2..114, ZONEA/M1 114, ZONEB/M3 orphan
  ignored) + 3-part mutation sensitivity (live field, tail-blind
  orphan, terminator extension). Rust live-run-only retention
  documented as the justified seam. Strict S0 coverage 7/27; next:
  S0-08 `static-yline-zbase` (queued as item 1).
- 2026-08-25: P4.2/DH-G0-live S0 LIVE SESSION RETIRED AS A T1
  SEMANTIC PREREQUISITE (D145). Independent deterministic static
  oracles are now the default. Operator capture was NOT completed and
  remains optional for channel/address, hardware, timing, or perceptual
  qualification; it no longer blocks semantic parity work.
- 2026-08-24: P4.2/W10-impl-b THE DIFFER O3 FIELD MAP + O3-SEAM
  CLASSIFICATION unit COMPLETE (worker 59d0e7d5 claim 2, commits
  7d28bc2 spec + 55d2dc6 impl, both PUSHED; D144; spec =
  O3-8STREET-COMPARATOR §5a + DESIGN §10-W10 amendment). The LAST
  in-repo W10 unit — W10 in-repo work is COMPLETE. (1) normalize_o3_row
  = normalize_o2_row VERBATIM (the reconstruction rebuilds EXW state,
  same cells/layouts, D142 §3); the UnsupportedChannel rejection is
  gone; the D90 splice + lone-span guard apply on O3 identically.
  (2) Class::O3Seam (name o3-seam, NON-failing) via TWO matchers:
  row-id (sfx-master-gate: OPTIONS.BDL vs HKCU registry D128 vs
  CONFIG.BDL D134 vs E-constant-1 D136) + the registry row's exw_addr
  BASE CELL (the whole D128 §7j.56 config family: 0x4ede58/0x4ede5c
  SOUND, 0x4eb93c SPEECH always-on, 0x4edbd8 ACTIONPAN, 0x46cca4
  CINEMATICS, 0x4eba1c LANGUAGE, 0x4e444c DEFAULTNAME) — future
  config rows are caught automatically. Equality silent; divergence
  reports the ledger reason verbatim; coverage asymmetry on seam rows
  classifies o3-seam too; seam rows EXCLUDED from tiebreak arbitration.
  The volume-key pair (0xC8/D0→0x48/0x50) + CDDA documented as
  deliberate NO-classifier omissions (trigger/behavior deviations,
  never seams — a live O3 arrow-key drift is a genuine finding).
  (3) GATES: differ_gate O3 lane extended — real-S0 fabricated self-
  cross PASS 0 findings; seeded sfx-master-gate → exactly one o3-seam
  (PASS-WITH-NOTES, zero EngineBug/Structural); the same seed on money
  FAILs EngineBug (selectivity); a synthetic ACTIONPAN registry row
  seam-classifies via the cell matcher end-to-end through the real O3
  stitch rule; the same pair under O2 headers → plain EngineBug FAIL
  (the class binds the O3 channel only). New fast crate unit
  o3_field_map_and_seam_ledger (O3==O2 normalization identity + every
  matcher form); runner stitch_o3_channel_rules (e) flipped to the
  landed state. No engine change; no registry change (synthetic row
  test-local); no plan artifacts. VERIFIED: diffharness 96 green,
  differ_gate corpus 4/4 green (201s), fmt+clippy clean (workspace,
  all targets), MANIFEST.sha256 clean before AND after the corpus
  run, no Ghidra run. Queued: NOTHING — the unattended P4.2 queue is
  empty; item 1 (the operator S0 live session) + the W10/W11 operator
  pieces carry the rest.
- 2026-08-24: P4.2/W10-impl-a DBX-STITCH --CHANNEL O3 + THE O3
  ANTI-GHOST RULE unit COMPLETE (worker a42f254c claim 2, commits
  f584eab impl + 4b27761 docs, both PUSHED; D143). The stitch side
  of the D142 O3 intake closed: runner::stitch binds the O3 channel
  to the registry exw_addr cell — the O2 MIRROR (the 8street
  reconstruction rebuilds EXW state, so a row with no EXW canon cell
  can never appear in an O3 dump): the one live-registry EXD-only
  row (static-cursor-clamp, TS) rejects LOUD on O3 exactly as on O2
  (StitchError::NoExwAddress, the D139 pattern), EXD-gap rows with
  live EXW cells (debris-stager T3) stay LEGAL there (per-channel
  mirrors; the O1 exd_addr rule untouched); dbx-stitch accepts
  --channel o3 (o1 default + O1/O2 behavior byte-identical; manifest
  names "O3:8street"; Channel::O3Street encode/decode was already
  channel-complete since D78). GATES: new runner unit
  stitch_o3_channel_rules (O2-form transcript stitches + decodes
  channel-marked code 3; EXD-only row refuses loud; the mirror both
  ways on debris-stager; determinism byte-identical re-stitch; the
  differ UnsupportedChannel rejection ASSERTED as the documented
  D142 §5 gap until W10-impl-b) + new differ_gate lane
  s0_o3_transcript_stitch_channel_rule (the REAL S0 E run, chain
  dac1cfd17bc7ede3, fabricated through inv_frame's new O3Street arm
  — O3 raw forms = the O2 forms — stitched THROUGH the enforced
  rule, byte-identical re-stitch + chain; the EXD-only row refuses
  at stitch) + CLI smoke (o3 manifest/dump + determinism, ghost-row
  refusal, o4 error). Docs: O3-8STREET-COMPARATOR §5/§8 + DESIGN
  §10-W10 amended + DECISIONS D143. No engine change (test-file
  fabrication lane only); no registry/plan changes. VERIFIED:
  diffharness full suite green, bedlam-game 191 green (differ_gate 4
  lanes, canonical_dump_gate 13, corpus read), fmt + clippy clean,
  MANIFEST clean before AND after the corpus runs, no Ghidra run.
  Queued: item 2 = the differ O3 field map + o3-seam classification
  (the LAST unattended-safe W10 remainder).
- 2026-08-24: P4.2/W10-prep THE O3 8STREET COMPARATOR FEASIBILITY
  unit COMPLETE (worker 5ae99a92 claim 2, commit 5740555, PUSHED;
  D142; docs/O3-8STREET-COMPARATOR.md + DESIGN-DIFFHARNESS §10-W10
  addendum + DECISIONS D142). The landing study for the last
  unstarted W item: pinned rebuild target (8street/Bedlam @
  a8622e6, tree f9df7045, bedlam.asm/.inc sha256s; NO top-level
  license -> local test-only, nothing enters this repo); build
  toolchain (clang -m32 + JWasm + vendored libsmacker + i686
  SDL2/SDL_mixer >=2.0.12; their CI recipe proves reproducibility);
  FIRST build operator-gated (sudo + network), compile.sh alone
  unattended after; O3 runs against a STAGED corpus copy (the
  reconstruction WRITES SAVES/ + BEDLAM.LOG — game-data/ never its
  working folder). HEADLINE RE: 8street resolves every cell by
  SYMBOL NAME (never address arithmetic); bedlam_data.inc is a
  sequential mirror of EXW .data(0x454000)/.bss(0x45B000..0x4EFB60)
  with an 8-transition DRIFT LEDGER (first defect: seven anonymous
  dd where IDA names imply four at 0x4DC6CC..E0; exact only below
  0x4DC6D0); CROSS-VALIDATED — simulated emission positions of the
  semantic symbols re-anchor onto independently-pinned registry
  cells (current_money==money, difficulty, robots_available==the
  D89 per-player cell all delta-0; game_mode==mode; zone/zone_level,
  rnd_seed1/2==rng-state-a/b, sound_enable==sfx-master-gate all
  landing at EXACTLY the ledger's -208). Hook family: H1 frame tail
  = game_level wait loc_448730:99697 (== EXW 0x425a03), H2 anchor =
  loop-head first entry loc_447E6A:98943, H3 the D77 §3 seams via
  three row-resolution cases (named symbol / anonymous-filler label
  via the ledger / C++-shell externs; frame-counter 0x46ae68 is DEAD
  in 8street — the hook numbers frames, an equivalence seam), H4 the
  hook emits DBXCAP v1 DIRECTLY so stitch->encode->chain->differ are
  reused unchanged. Remaining W10 work split: two in-repo units
  (queued as items 2+3) + the operator-gated rebuild (parked until a
  three-way tiebreak is wanted). Docs only; MANIFEST clean before
  AND after the clone reads.
- 2026-08-24: P4.2/D140-followup THE O1 DBX-CAPGEN FRAME-1 DEDUPE
  FIX unit COMPLETE (worker 9f4a1111 claim 2, commit c65d1e8,
  PUSHED; D141). The D140(2) landmine closed BEFORE the operator S0
  live session: dbx-capgen.py's frame-1 path concatenated
  anchor_watches + watches literally, but on every committed plan
  the per-frame rows are a SUBSET of anchor_watches, so every live
  O1 session would emit DUPLICATE watch ids at frame 1 and `diff
  stitch` rejects the transcript (canonicalize_frame
  DuplicateWatchId, dump.rs). FIX: frame 1 now dumps the DEDUPED
  union keep-first via the new module-level dedupe_frame1_rows()
  (the exact capgen-o2 semantics); the transcript summary line
  reports the deduped count; the module docstring updated. VERIFIED
  headless (no game, no corpus read, MANIFEST clean): (a) the
  committed check tools/runtime/capgen-frame1-dedupe-check.py
  imports the REAL shipped function (never a copy) and proves over
  all 13 committed plans: unique frame-1 ids, frame-1 == the anchor
  list in anchor order, every per-frame id rides the anchor set,
  the landmine expression absent from the source (the raw concat
  would have duplicated 11-30 ids per plan = every per-frame row);
  (b) ALL dbgprobe probes re-GREEN through the changed path (gate,
  flow, inject, walk, pad both legs) — the probe plans DO carry
  anchor_watches but their anchor/per-frame id sets are DISJOINT
  (zero overlap, checked — the D140 "probe plans carry no
  anchor_watches" gloss corrected in RUNTIME.md), which is why the
  gates never tripped it pre-fix; (c) py_compile clean. No Rust
  change. Docs: RUNTIME.md D140 finding note closed out + DECISIONS
  D141. No plan bytes changed — no re-stage needed beyond D134.
  Queued: item 2 = the W10-prep O3 comparator feasibility note
  (the unattended P4.2 queue was otherwise empty; only operator-
  gated work remains).
- 2026-08-24: P4.2/W11-prep THE CAPGEN-O2 TRANSCRIPT EMITTER
  SKELETON unit COMPLETE (worker 3b207215 claim 2, commits dba16f3
  notes + 0d45531 impl, both PUSHED; D140). (a) THE CONTRACT SPLIT
  + DBXFEED v1: the future W11 ptrace driver (operator-gated)
  services the D138 o2 plan (trigger hits @0x425a03 +
  process_vm_readv per row) and logs a DBXFEED v1 read/write log
  (hit blocks; hit 0 = optional boot position; hit 1 = the ANCHOR —
  where the feed starts IS the driver's mission-load policy);
  tools/runtime/capgen-o2.py is the pure plan interpreter +
  transcript emitter validating the feed against ONE shared
  plan_walk 1:1 — every read's addr+len, hit numbering (= capture
  frames, anchor = 1, resolving the D138 comment's loose wording),
  and the FULL inject arithmetic re-derived (plain /
  op:command ring appends from the logged count read / op:pad
  step-ons with the D86 mark check). --synthesize-feed is the
  reference mini-driver (deterministic LCG bytes per (addr,hit),
  consistent resolve statics + prefix counts + the +1-per-hit frame
  counter) — generator and checker can never diverge. SYNTHETIC
  feeds mark the transcript (anti-ghost); frame-counter drift warns
  + records a transcript comment. (b) HEADLINE FINDING (D140(2)):
  on EVERY committed plan the per-frame rows are a SUBSET of
  anchor_watches — the anchor list IS the frame-1 row set; a
  literal anchor+watches concatenation emits DUPLICATE ids and the
  stitcher's canonicalize_frame rejects DuplicateWatchId. capgen-o2
  dedupes keep-first; the SAME landmine sits in the O1 dbx-capgen
  frame-1 path (queued as the new item 2 — every live O1 session
  would fail at `diff stitch`; the dbgprobe gates never see it, the
  Rust fabrication lanes build from E frames whose ids are unique
  by construction). (c) SMOKE ALL GREEN
  (tools/runtime/capgen-o2-smoke.sh; unattended-safe: no Wine, no
  ptrace, no game, no corpus read; MANIFEST clean pre+post):
  dbx-plan --channel o2 byte-pins S1-o2.json; the FULL 401-frame
  S1-o2 chain synthesize → emit → dbx-stitch --channel o2 against
  the REAL S1 scenario (20.5 MB dump, chain b436fa77642c94fc,
  manifest O2:EXW/Wine, frame_count 401) → dbx-diff self-cross
  PASS 0 findings (decode + normalize_o2_row intake); the EXD-only
  row (static-cursor-clamp) spliced into the transcript REFUSES at
  the CLI (NoExwAddress); a feed truncated at hit 401 refuses at
  emit; S3-o2 compiles its 8 op:command injects on EXW cells and
  the chain runs end-to-end (frame-1 injected flag, chain
  52f6044c2033cb34); emitter determinism byte-identical. THE
  HEADLESS O2 LOOP IS COMPLETE (plan → driver-feed → transcript →
  stitch → differ); only operator-gated W11 work remains (the
  ptrace driver + the S0 live session). diffharness 82 tests
  green, fmt+clippy clean, no Ghidra run, no corpus write. Queued:
  item 2 = the O1 dbx-capgen frame-1 dedupe fix.
- 2026-08-24: P4.2/W11-prep THE DBX-STITCH O2 TRANSCRIPT CHANNEL
  SUPPORT unit COMPLETE (worker 74bae49c claim 2, commits 1cc53b4 +
  ab0738b, both PUSHED; D139). (a) THE CHANNEL-THREADED ANTI-GHOST
  RULE: runner::stitch validates every transcript id per the DUMP
  HEADER's channel — O1 keeps the exd_addr rule verbatim, O2 gains
  the mirror (StitchError::NoExwAddress, carrying the row's note).
  PER-CHANNEL MIRRORS, never global: a T3 EXD-gap row with a live
  EXW cell (debris-stager 0x476fbc) dumps LEGALLY on O2 (the EXW
  cell is the canon there + the D138 plan emits it); the ONE
  live-registry EXD-only row (static-cursor-clamp, TS — empty
  exw_addr) rejects LOUD on O2 and stays legal on O1 (EXD pair
  0x1074ac/0x1074b0). Pre-D139 the O2 path enforced NOTHING (only
  the differ downstream would surface a bogus row as coverage
  noise). Engine/O3 carry no address rule. (b) dbx-stitch
  --channel o1|o2 (o1 default, O1 behavior byte-identical — the W3
  machinery was already channel-agnostic by DESIGN §3, which is why
  the D87 fabricated tiebreak lanes were correct by construction);
  CLI smoke-verified both ways (o2 manifest "O2:EXW/Wine"; the clamp
  transcript FAILS o2 with the note, PASSES o1). (c) VERIFIED: new
  runner unit stitch_o2_channel_rules (the D138 row forms
  end-to-end: the 8-byte ADJACENT map-wh span w@+0x00/h@+0x04
  stitches + decodes channel-marked; the LOUD rejection; the mirror
  both ways) + new differ_gate lane s0_o2_transcript_stitch_
  channel_rule (the REAL S0 run dac1cfd17bc7ede3 fabricated through
  the channel-aware inv_frame, stitched under O2 THROUGH the
  enforced rule, decoded with the 8-byte span intact; the EXD-only
  row refuses; the same row on O1 forms stitches clean). Full
  differ_gate 3/3 corpus green (829s — the S0..S8 cross/double-run
  lane + all four tiebreak lanes UNCHANGED: the only empty-exw
  registry row is static-cursor-clamp itself, so the rule guards
  never breaks the existing fabrications); canonical_dump_gate
  13/13; diffharness 99; bedlam-game lib 132; fmt+clippy clean;
  MANIFEST clean pre+post; no Ghidra run. THE O2 HEADLESS TRIANGLE
  (plan D138 <-> differ D137/D138 <-> stitch D139) IS CHANNEL-
  COMPLETE — only the operator-gated W11 ptrace driver remains.
  Queued: item 2 = the capgen O2 transcript emitter skeleton (the
  plan->driver->transcript->stitch chain proven headless on a
  synthetic feed).
- 2026-08-24: P4.2/W11-prep THE DBX-PLAN O2 CHANNEL SUPPORT unit
  COMPLETE (worker c44a3c8b claim 2, commits c57eae3 RE-notes +
  b199ece impl, both PUSHED; D138 + D137-CORRECTION). (a) dbx-plan
  --channel o2: every watch/resolve/step address swaps to the
  registry exw_addr canon cell in flat 0x form; the DOSBox
  boot/arm/env machinery replaced by the registry-derived trigger
  object {site 0x425a03, frame_counter 0x46ae68}; resolve_at=anchor
  + frames contract channel-neutral; walk-phase keystore scenarios
  REFUSED on o2 (the BPLM menu walk is DOSBox/O1 machinery);
  mission-phase steps emit on the EXW seam cells. o1 default
  BYTE-IDENTICAL to all 12 committed plans (gates prove it);
  capture-plans/S1-o2.json committed + byte-pinned (36 anchor / 28
  per-frame / 7 deferred — static-cursor-clamp is the EXD-only
  deferral; the EXD-unmapped T2/T3 rows deferred on BOTH channels —
  channel-symmetric emission set). (b) HEADLINE — THE D137 SPAN
  ARITHMETIC CORRECTION: the new registry-derived span assert
  CAUGHT that D137(2)/§7j.60 C-D's "EXW cells 0x24 apart / O2 = the
  0x28 span h@+0x24" was ARITHMETICALLY IMPOSSIBLE for the cells it
  quotes (0x4eddf0−0x4eddec = 4 — adjacent u32s, stride cell right
  after; 0x4eddec+0x24 = 0x4ede10 ≠ 0x4eddf0). CORRECTED PIN: O2
  form = the 8-byte span @0x4eddec, w@+0x00/h@+0x04 (the
  field-order asymmetry vs the EXD 0x30 h-LOW span SURVIVES — O2 is
  still not O1 relabelled). Fixed everywhere the wrong form had
  landed: §7j.60 C/D, RE-EXD-MAP §5b, watches.toml layout note,
  DESIGN §10-W7 + §10-W11, normalize_o2_row (need 8 — the 0x28 arm
  would have failed every REAL live O2 capture structurally), the
  differ.rs fixtures, the differ_gate inv_frame fabrication; the
  corrected triangle re-verified green on corpus. (c) TWO REGISTRY
  CORRECTIONS the address swap forced (committed-pin citations, no
  new RE): robot-bank + no-extract-latch exw_addr count-cell
  parentheticals 0x46ccbc→0x46cbd8 (the PER-PLAYER 0x11958c twin
  per the W8-prep count-mapping; 0x46ccbc = the TOTAL/cap twin —
  SP values coincide, the semantic binding is now correct), and
  selection-triple's EXW pick = cells[1] 0x46cbdc with geometry
  asserts (the EXW list is field-ordered base/selected/size but
  NOT ascending — the D132 pairing; O1 keeps cells[0] 0x11954c).
  VERIFIED: diffharness 98 (4 new O2 tests incl. the artifact pin +
  the walk refusal + the EXW-cell step emission), differ_gate 2/2
  (696s corpus), canonical_dump_gate 13/13, bedlam-game lib 132,
  fmt+clippy clean, MANIFEST clean pre+post, no Ghidra run, no
  corpus write. Queued: dbx-stitch O2 transcript support (item 2 —
  the last headless-reachable W11 piece before the driver).
- 2026-08-24: P4.2/W11-prep THE O2 STATIC-MAP-WH PIN unit COMPLETE
  (worker 05178a0c claim 2, commits 1438ca6 RE notes/D137/registry/
  DESIGN/watches amendments by predecessor a3532435 + 0ea13b8 impl,
  both PUSHED — the impl unit adopted + validated + completed
  interrupted predecessor WIP found uncommitted over 1438ca6; the
  full diff re-read and every call site checked before staging,
  unrelated dirt untouched). The LAST deliberate zero-field differ
  row closes: normalize_o2_row's static-map-wh arm parses the
  D137-pinned EXW form — the 0x28 span @0x4eddec, w@+0x00/h@+0x24
  (exact-length need BOTH directions: the EXD 0x30 span REJECTED
  on O2 and the 0x28 span on O1 — the reversed field order vs
  address order can never silently mis-parse). differ_gate
  fabrication CHANNEL-SPLIT since the pin: inv_frame takes the
  channel (EXD 0x30 span under O1, EXW 0x28 under O2; Engine/O3
  unreachable — guest channels only). NEW direct E-vs-O2 cross in
  s1_o2_tiebreak_arbitration proves the row COMPARES CLEAN through
  the real O2 normalizer (coverage exactly 1 = move-target-words,
  zero EngineBug/Structural, no static-map-wh finding); all four
  tiebreak lanes re-verified on their own channel forms; the cross
  suite's S0 expect_coverage stays 0. New o2_row_forms unit (the
  0x28 parse + symmetric cross-form rejection) + tools/differ
  o2_frame fabrication for the tiebreak lanes. Green: differ_gate
  2/2 (697s, corpus), diffharness 43 (incl. the new unit), fmt +
  clippy clean, MANIFEST.sha256 clean before AND after; no Ghidra
  run, no corpus write. Queued next: item 2 = dbx-plan O2 channel
  support (the headless W11 prerequisite — plans are still
  EXD/CS:-form only).
- 2026-08-23: P4.2/W11-prep DIFFER_GATE O2 TIEBREAK FABRICATION
  unit COMPLETE (worker 7956a0e8 claim 2, commits 04cd6b0 RE/design
  note + 4591f52 impl, both PUSHED). All four compare_field
  T1-exact arbitration lanes driven headless by the new
  s1_o2_tiebreak_arbitration test: ONE inv_frame fabrication
  stitched under BOTH O1ExdDosboxX and O2ExwWine (valid because
  normalize_o2_row's alias list takes EXD-identical EXW guest
  forms, EXW_ROBOT_MAP == EXD_ROBOT_MAP per the sec 8 back-half
  probe, and the O2 static-map-wh row ignores its bytes pending
  the W11 pin); the engine-is-wrong lane re-stitches the REAL E
  frames under Channel::Engine with money perturbed (stitch_o1
  generalized to stitch_chan — the O1-address rule binds only
  O1). Lanes asserted class+detail+a/b VERBATIM: (a) O2 sides
  with O1 vs perturbed E -> EngineBug "the engine (E) is the
  outlier" FAIL; (b) O2 sides with E vs perturbed O1 ->
  OriginalDivergence "EXD diverges from EXW" PASS-WITH-NOTES
  (budgeted); (c) all three differ -> EngineBug "E wrong against
  both oracles"; (d) no tiebreak -> EngineBug "provisional"; plus
  the idle-tiebreak baseline (coverage stays exactly 1 on S1,
  tiebreak fingerprint O2:EXW/Wine + S1). NO production change —
  the arbitration logic verified as-written (W11's live channel
  inherits a proven arbiter). Full differ_gate suite green
  (2 tests, 693s incl. the S0..S8 corpus gate), fmt+clippy
  clean, MANIFEST.sha256 clean after the corpus-reading run;
  no Ghidra run. Queued next: item 2 = the O2 static-map-wh pin
  (the last deliberate zero-field row).
- 2026-08-23: P4/RE THE EXD SFX-MASTER-GATE TWIN CENSUS unit
  COMPLETE (worker 2a9f1b9f claim 2, commits 5178420 notes +
  d341c65 impl + the D134 DECISIONS entry riding the impl commit;
  docs+registry+plans; objdump-only from the committed exd/exw
  listings + read-only string probes of game-data (vma↔fileoff
  via the DEADMAN1 anchor); no Ghidra run, no corpus write;
  MANIFEST.sha256 clean before AND after; 93 diffharness tests +
  13 canonical_dump_gate green; fmt+clippy clean; PUSHED). LANDED
  + RE-VALIDATED by the respawned slot worker e104cbd0 claim 2
  (commit b0e105a, D134 landing note in DECISIONS): every census
  family independently re-derived before adoption, and the census
  COUNTS corrected with history preserved — EXW 18 / EXD 17
  literal sites (not 19/18; 13 reader sites one-for-one, EXW-only
  {0x43a16c, 0x42530a, 0x4253f3} vs EXD-only {0x4c593, 0x12767}),
  + two gloss fixes (0x43a79e = the options-handler drop-flag
  pair, NOT inside FUN_0043a48e; 0x4c9a0/0x4c9a9 ⟷ 0x43a795/
  0x43a79e same order — the "arg order swapped" note retracted).
  THE W1
  REGISTRY GAP SET IS NOW EMPTY — the last of the four W1 schema
  gaps closed. CLOSED with the verdict set: (1) THE TWIN =
  [0x10743c], pinned by the queue's own anchor — the EXD
  BOOM-trio twin FUN_00032de9 (gate @0x32df1) is
  shape-identical to EXW FUN_00421e60 (@0x421e68) incl. the
  RandB idiv-3 dispatch + the shared play tail `call 0x4c584`
  @0x32f95 = THE PLAY TWIN FUN_0004c584 ⟷ FUN_0043a48e. (2)
  CENSUSES one-for-one (EXW 19 / EXD 18 literal sites): the
  arrival-family FIVE (RICOCHT/BOOM/GRUNT — cells REVERSED vs
  EXW —/DEATH/HURT), the music-sequencer TRIO, the radio-warning
  consumer (0x34a8e ⟷ 0x423af7 — independently confirming
  [0x10766c]≡SPEECH [0x4eb93c] + [0x107444]≡[0x4ede5c] via the
  EXW arg-order), the driver-sync wait, the play dispatcher's
  own gates (fail → [0x1195f4]:=1 ⟷ [0x46ae78]:=1), the
  MissionShell volume-key pair (the EXACT ×0x147≫7 scale pinning
  [0x1081f0]≡[0x4ddb2c]); EXD-only: the frame-tick music hook
  0x12767. (3) THE WRITERS + CONFIG DIVERGENCE: EXW init
  FUN_0043a144 (sole caller GameMain 0x41c33f) with the value
  from the REGISTRY "SOUND" key (boot load 0x42530a, saver
  0x4253f3) ⟷ EXD init FUN_0004be7d (callers boot 0x2cc70 +
  title 0x5b03f) parsing the FILE CONFIG.BDL (install-dir buffer
  0x9237c + the name strings 0x867ea/0x867f5/0x867f9) — the DOS
  file-config vs Win32 registry port seam; both branch pairs
  write the SAME tandem cells (sister gate, SPEECH, the 0xfe000
  arena, the instruction-exact 16-entry voice-table fill loop
  0x8b938 ⟷ 0x4eada8). (4) THE FUN_0004c121 BANK-NAME WALK
  pinned (names past the "SOUND\SFX\" prefix; GRUNT rides the
  MissionShell-head walk @0x59b79 with BEAMIN/THROW/PEXPLODE/
  BIOFIRE/CACODETH/SQUAWK) + 19 §5g cascade aliases. (5)
  REGISTRY/PLANS: watches.toml filled + registry_anchors gap set
  EMPTIED (new hard no-gap check + D134 citation); dbx-plan emits
  the row on every T0 scenario; the runner's NoExdAddress fixture
  now fabricates a synthetic gap; ALL 12 capture plans
  regenerated (S0/S0W INCLUDED — the row is T0; deferred counts
  7→6/10→9/21→20/24→23); E's W6 row list untouched (the row
  stays a documented E-gap, the D133 no-extract-latch precedent
  — the emission decision queued as item 3). One S0
  fingerprint-step companion recorded (a sound-DISABLED capture
  machine dumps 0 — one dbgprobe read settles it). Queued:
  item 2 = the EXW bank-cell twin cross-check (the §5g
  leftovers), item 3 = the E-gap emission decision.
- 2026-08-23: P4/RE THE EXD BLINK-CURSOR TWIN CENSUS unit
  COMPLETE (worker c653b51a claim 2 — adopted + re-validated +
  landed the interrupted 4fe7f1e9 WIP; substrate committed by it
  as tools/exd-relod.py 8f641d3; docs+registry commit f9986b0,
  D132, RE-EXD-MAP §5/§5e + §7j.59.E addendum; objdump-only from
  the committed-tool relocation listing, no Ghidra run (the
  pgrep hits were this worker's own prompt string), no corpus
  write; MANIFEST.sha256 clean before AND after;
  registry_anchors 2/2 green; dbx-plan 31/31 incl. the
  committed-artifact reproduction; fmt+clippy clean; PUSHED).
  CLOSED with the verdict set: (1) THE TWIN = [0x0010e108],
  EXACTLY 7 .text sites one-for-one with the §7j.59/D131 EXW
  census — writers: the three idle-arm strips 0x1cef1 :=ecx(1) /
  0x1cf2c :=2 imm / 0x1cf72 :=ecx(3) (⟷ 0x40c1d7/0x40c217/
  0x40c254; posts (0xC,0)+(0xF,·,1)/(0xD,1)/(0xE,2) via the
  warning-post twin FUN_00034972 ≡ FUN_004239ef; size gates
  [0x11958c]>1/>2; the imm-vs-ecx split is a codegen swap,
  k=1 imm on EXD vs k=2 imm on EXW) + the impact-completion
  clear 0x34f7f :=ebx(0) (⟷ 0x423fef, in the shell-resolver
  FUN_00034d89 ≡ FUN_00423e1c after the 3×3 nine-blast
  FUN_00035406 loops) + the MissionShell reset 0x59842 :=ecx(0)
  (⟷ 0x447871, between map-overlay [0x1075bc] and salvo latch
  [0x1081fc]); readers: the portrait blink gate 0x186dc (⟷
  0x407428, inside FUN_000180a1 ≡ FUN_004072bf — identical
  (frame[0x1195f0]&3)+0x51 sprite, literal 1/2/3 x-dispatch
  0x1F0/0x222/0x254, y=0xD, bank [0x1074fc] ≡ [0x4edd7c],
  draw FUN_000111fa ≡ FUN_00401ca2, 0 AND >3 draw nothing) + the
  chase-camera gate 0x34e25 (⟷ 0x423e91 — identical
  ([base]+[cursor]−1)·0xA8 → kind@+0x2A (dword@+0x28 sar16)
  == player-type [0x1075c0] ∧ ≠selected → cut FUN_0003552e ≡
  FUN_004245c9). Idle table 0x8105c {400,300,200,5000} +
  respawn {1500,900,600} BYTE-IDENTICAL to EXW 0x454ee8/0x81050.
  (2) THE SELECTION-TRIPLE LABEL-SWAP CORRECTION: EXD 0x11954c
  ≡ EXW 0x46cbdc (SELECTED SLOT — auto-switch 0x5a117/0x5a124 ⟷
  0x448109/0x448111), EXD 0x11955c ≡ EXW 0x46cbd4 (SQUAD BASE —
  arm-strip compare 0x1cecc, chase read order 0x34e20→0x34e30 ⟷
  0x423e8c→0x423e9c, global index 0x5a871 ⟷ 0x4480c1); 0x11958c
  ≡ 0x46cbd8 per W8-prep; ALL THREE cells mapped, gap CLOSED.
  (3) TEN §5e ALIASES: salvo latch 0x1081fc; 8-shell bank
  0x8f0b4 with the grammar pinned BOTH sides {x w@+0, y w@+2,
  fall w@+4 seed 0xFF, start-delay w@+6, valid w@+8} (scatter
  0x1d00e..0x1d038 ⟷ 0x40c323..0x40c348, last two stores
  swapped codegen order; the 0x8f0b2/0x4ea236 sites = the
  dword@base−2≫16 x-read idiom — LANDING-RUN correction of the
  first draft's 0x8f0b2/+2/+4 convention, re-verified
  instruction-exact); map-overlay 0x1075bc; zoom 0x107448;
  idle table 0x8105c; GENERAL.BIN ptr 0x1074fc; + the four
  function twins. (4) watches.toml blink-cursor exd_addr filled
  + gap closed, selection-triple re-aliased; dbx-plan emits the
  4-B cell; S1..S8/S5B/S5C capture-plans regenerated; registry
  gap set = {sfx-master-gate, no-extract-latch} only. Engine
  consequence NONE. Queued: item 2 = the EXD no-extract-latch
  twin census, item 3 = the EXD SFX-master-gate twin census.
  NEXT: item 2.
- 2026-08-23: P4/RE THE [0x4dc5d0] BLINK/EFFECT-LIST PRODUCER
  CENSUS unit COMPLETE (worker 0329338f claim 2, commit
  64543b6, D131, §7j.59, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt, no Ghidra run, no
  corpus read; MANIFEST clean before AND after;
  registry_anchors 2/2 green; PUSHED). CLOSED with the verdict
  set: (1) THE MECHANICAL CENSUS — exactly SEVEN .text
  references to 0x4dc5d0 (whole-objdump grep, no other
  addressing form): 5 writers (0x40c1d7 :=ebx=1, 0x40c217
  :=edi=2, 0x40c254 :=3 imm, 0x423fef :=ecx=0, 0x447871
  :=ecx=0) + 2 readers (0x407428 the §6c.6d portrait gate,
  0x423e91 the §7j.54 chase-camera impact gate). (2) VALUE
  GRAMMAR {0,1,2,3}: the three :=k+1 writes = the UNROLLED
  per-slot strips of the robots() idle-arm tail (k=0 no size
  gate → (0xC,0)+(0xF,0) → :=1; k=1 size>1 → (0xD,1)+(0xF,1)
  → :=2; k=2 size>2 → (0xE,2)+(0xF,2) → :=3; shared salvo
  tail +0x70:=0, [0x4de658]:=0x80, 8-shell scatter) — value =
  the ENDANGERED robot's squad slot+1; the 2026-08-21 item-6
  "SELECTED robot's SLOT" gloss CORRECTED (SP coincidence only
  — in MP every idle robot arms; arm gate ≠ write gate).
  (3) {1,2,3} GATE PINNED vs the effect-row family: the
  consumer is a LITERAL x-dispatch (1→0x1F0/2→0x222/3→0x254,
  sprite (frame&3)+0x51 GENERAL.BIN y=0xD; 0 AND >3 both draw
  NOTHING — >3 dead-defensive); 1/2/3 are NOT blink classes /
  FLAGS ids — the 10×16-B effect-row array 0x4dc5d4..0x4dc67c
  is DISJOINT (allocator scans 0x4dc5e0+k*0x10 only; §6c.6d
  "sprite-list field" renamed "warning field"); the impact-gate
  reader PROVES index semantics arithmetically ([0x46cbd4]+
  ([0x4dc5d0]−1)×0xA8 → player-type ∧ ≠selected → chase
  camera). (4) LIFECYCLE: 0 at mission entry → :=slot+1 at
  the idle arm → 0 at FIRST shell impact (≈ arm+40..54
  frames) → re-arm after the 0x80 cooldown + fresh threshold;
  ordering resets the idle counter. (5) ENGINE/DIFFER
  CONSEQUENCE NONE (SP-UI presentation, zero sim reads/RNG);
  the S1 blink-cursor-from-spawn hypothesis now STATICALLY
  decidable — constant 0 on every corpus scenario; DESIGN
  watch + hypothesis rows annotated. Deliverables: §7j.59
  A–E + the amendment-item-6 correction/supersession + §6c.6d
  gate/engine-seam fixes + D131. Queued: item 2 = the EXD
  blink-cursor twin census.
- 2026-08-23: P4/RE THE ROBOT +0x9C DEATH-FLAG READER CENSUS
  unit COMPLETE (worker 18039414 claim 2, commit 6a3abcd,
  D129, §7j.57, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt, no Ghidra run, no
  corpus read; MANIFEST.sha256 clean, registry_anchors 2/2
  green; PUSHED). CLOSED with the verdict set: (1) BOTH
  PRODUCERS PINNED = 1 — the SP/other tail 0x40eac0 (edx := 1
  @0x40eab4; reached when SP [0x4edb88]==0 ∨ no-extract latch
  [idx*4+0x46aed4]≠0) and the MP respawn tail 0x40e82a (edi
  := 1 @0x40e807; MP ∧ latch==0) — the queue "MP-respawn
  reset" phrasing was a MISNOMER, corrected in place: the
  respawn re-init does NOT clear +0x9C, the respawned MP slot
  STAYS death-flagged (harmless — the sole reader is
  SP-only). (2) THE SOLE READER = the SP SQUAD-WIPE FAIL
  DETECTOR FUN_0044764c..0x44770a (decoded whole; sole caller
  MissionShell 0x44870d gated [0x4dc67c]==0 = extraction NOT
  complete — a wiped squad post-extraction never fails): MP →
  ret 0; walks squad [0x46cbd4]..+[0x46cbd8]−1, FIRST +0x9C==0
  → ret 0; all dead ∧ [0x4ede34]==0x1E0 (death wipe at
  terminal 480) → FUN_0042391d + FUN_00425a03 (+cond
  FUN_0042595a) + FUN_00425bf5 + the [0x46cca4]-gated anim
  string 0x459852 → ret 1 → MissionShell ret 3 (the
  fail/debrief transition; ret 2 = launch). +0x9C = the
  MISSION-FAIL liveness oracle, DISTINCT from +0x7C alive /
  +0x78 hp (both re-staged by MP respawn, this never). (3)
  LIFECYCLE CLOSED — no literal zero-writer exists; the clear
  is the mission-staging WHOLE-BANK ZERO-FILL: FUN_0040cca2
  @0x40cd29..38 — ecx := 0x7E0; edi := 0x4c69e4; FUN_0041cd42
  (file rewind [0x4eba20]; edi/ecx callee-saved) then
  FUN_00402965 (the §7j.21 memset-0) zeroes 0x7E0 = 12·0xA8 =
  the WHOLE 12-SLOT BANK (NEW FACT: the robot bank is 12
  slots); the ONLY immediate-load of 0x4c69e4 in the binary —
  no save-load bulk copy touches the bank; every mission entry
  starts flag-clean. (4) THE §7j.55 SIDEBAR QUESTION ANSWERED
  NO — the heat-family sidebar row pass never reads +0x9C
  ([0x46ccec] sole reader 0x407205: it is a FLASH-COUNTDOWN in
  the [0x46ccf0]/[0x46ccf8] timer family, ≠0 → dec →
  FUN_00408403; writers death :=3 / cook-off :=2 /
  click-select :=2); the "dead-robot per-frame handling"
  hypothesis retired. ENGINE CONSEQUENCE NONE — E already
  conforms (death_flag := 1 SP subset + fresh per-mission
  records ≡ the zero-fill; death_flag already a +0x9C U16
  field leaf of the T1 robot-bank differ row). Deliverables:
  §7j.57 + the §3 +0x9C row + 2 ledger rows (squad-wipe fail
  detector, robot-bank zero-fill) + the §7j.45 item-6 closure
  + D129. Queued: item 2 = the [0x4ede34] temp-viewport
  census (renumbered, fed with this unit's producer/consumer
  facts). NEXT: item 2.
- 2026-08-23: P4/RE THE [0x4edbd8] CAMERA-GATE CELL + [0x4ede54]
  ZOOM CELL unit COMPLETE (worker 21e88d3b claim 2, commit
  d80fd8b, D128, §7j.56, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt + read-only string/import
  probes of game-data/cd-root/BEDLAM.EXW (.idata parsed to name
  IAT 0x4f010c = RegQueryValueExA); no Ghidra run, no corpus
  write; MANIFEST.sha256 clean before AND after;
  registry_anchors 2/2 green; PUSHED). CLOSED with the verdict
  set: (1) [0x4edbd8] = the "ACTIONPAN" value of the REGISTRY
  key HKCU\Software\Mirage\Bedlam\1.00 — the whole §7j.54
  chase-camera subsystem's enable bit: 4-site census = the two
  known readers EXACTLY (FUN_00403938 0x4039b0 camera-slot swap;
  robots() 0x40b875 recenter gate w/ the [0x4de654] leg
  0x40b885 — address refined, the double-je @0x40b87f a dead
  Watcom artifact) + the boot loader registration 0x42535c +
  the saver read 0x42545c (name-entry exit 0x43b03b +
  0x41c59b); .bss; bounds [0,1], DEFAULT 1 ⇒ pans ENABLED on
  default installs; NO game-state/mission-phase/UI writer —
  session-constant. (2) THE CONFIG FAMILY IS REGISTRY I/O
  (.idata pinned: FUN_0044ed40 = RegCreateKeyExA(HKCU,
  "Software\Mirage\Bedlam\1.00", KEY_ALL_ACCESS) → hKey
  [0x4ef770]; FUN_0044ede4 = the bounded loader —
  RegQueryValueExA writes the cell DIRECTLY, absent/malformed
  ⇒ the ecx default @0x44ee23..27, out-of-bounds ⇒ same;
  FUN_0044ed98 = query-then-RegSetValueExA self-heal writer;
  FUN_0044eee0 = the REG_SZ create-if-missing for
  DEFAULTNAME="Player"; family pattern cross-checked via
  INSTALLDRIVE ['A'..'Z'] default 'C' + SOUND [0, volume]) —
  the "CONFIG.BDL" gloss RETIRED (the string has ZERO binary
  refs; on-disk CONFIG.BDL/OPTIONS.BDL = DOS leftovers EXW
  never opens; SAVED.BDL the only referenced .BDL; TITLEMENU
  §4 corrected history-preserved). (3) [0x4ede54] = the
  VIEWPORT ZOOM height (backbuffer rows, clamp [0xF0,0x1E0] =
  [240,480]), NOT a plain speed factor — 26-site census:
  writers = the ±0x10 zoom-key handler (the FUN_0042034c tail
  0x4204ea..0x420548; scan 0x4E/0x0D in vs 0x4A/0x0C out;
  keystore 0x4edc92/0x4edc51/0x4edc8e/0x4edc50) + the
  MissionShell leftover-edx init 0x447883 (the 0x1E0
  @0x44784a does not provably survive FUN_004034ef
  (`imul edx,edx,0x26`)/FUN_0041d954 (xor tails) — benign:
  ≥480 dispatches 1:1 + first keypress re-clamps) + the temp
  save/restore pair in FUN_00401107's [0x4ede34] path
  (0x4012c7/0x4012e5/0x4012f1); readers = the Q16 magnify
  zoom blitter FUN_00401107 (scale (v<<16)/480 → 0x454060/68
  + halves 0x45405c/64, source offset (480−v)/2; ≥480 → 1:1
  rep-movs; [0x4edba0] map-overlay ≠0 → the map path; the two
  MissionShell render sites 0x447ca0/0x448094) + the recenter
  speed (cursor−240)·v/480 @0x40b89e/0x40b8c5 + the cursor
  un-zoom mappers 0x4106a1/0x4106d4/0x419a41. (4) DIFFER:
  zoom = ZERO rows (no corpus keypresses, deterministic per
  mission, zero RNG/robot-bank bytes, presentation-only);
  ACTIONPAN = one LIVE-CHANNEL CONFUND recorded (default-1
  pans live; a stale registry 0 on the O1 machine silently
  disables them while E models them — the S0 fingerprint step
  notes it now, D128 folded into queue item 1). [0x4ede34]
  census pointer recorded (9 sites, identity open — queued as
  item 3). Deliverables: §7j.56 + 2 ledger rows + the §7j.54
  address refinement + the TITLEMENU correction + D128.
  Queued: item 2 = the robot +0x9C death-flag census
  (unchanged), item 3 = the NEW [0x4ede34] temp-viewport
  census. NEXT: item 2 (the +0x9C death-flag census).
- 2026-08-23: P4/RE THE HEAT-MACHINE WARNING FAMILY unit COMPLETE
  (worker 19d79ca9 claim 2, commit 18e59ed, D127, §7j.55,
  docs-only; objdump-only from ghidra-project/exw-text-objdump.txt,
  no Ghidra run, no corpus read — the reachability corpus facts ride
  the committed §7j.53/§7j.9/§7j.41/§7j.10 evidence; MANIFEST.sha256
  clean before AND after; registry_anchors 2/2 green; PUSHED).
  CLOSED with the verdict set: (1) FUN_004100b7 (0x4100b7..0x4102b6,
  sole caller robots() phase-1 0x40bc72 amount 0x14 on-scorch) = the
  HEAT-IN machine — the +0x98 DAMPER (equipment stat 0x2C ×200,
  spawn 0x40d013 / MP-respawn 0x40ea59) absorbs first (≤0 → zero +
  "DAMPER EXHAUSTED" ids 0x2E..0x30 ONCE + return, no heat that
  pass); pool==0 → word@+0x30 += amt i16-wrap clamp 0xBB8, with
  EDGE-triggered OLD-vs-NEW crossings: 0x753 → "TEMPERATURE
  CRITICAL" ids 6/7/8 (@0x41025e/80/0x4102ac), 0x9C4 → "HAS
  OVERHEATED" ids 3/4/5 (@0x4101d7/f9/21d), old ≥ 0x9C4 →
  FUN_004102b6 EVERY pass, old ≥ 0x753 → early return; rising heat
  escalates CRITICAL→OVERHEATED; one huge add posts BOTH (overheat
  first); standard idx==[0x46cbd4]+k ∧ [0x46cbd8]>k dispatch, one
  post per event. (2) FUN_004102b6 (0x4102b6..0x4103ed, sole caller
  0x41019a) = the AMMO COOK-OFF — RandA&0x7F==0 (1/128), w=RandA&7<7,
  drain = max(1, ammo@+0x38+8w >>3), floor 1 (empty slot → 1 quirk),
  player-type → [0x46ccec]:=2, +0x32==0 → "LOSING AMMO" ids
  0x31..0x33 + cooldown := 100 (one per 100 frames). (3) THE +0x32
  CELL CLOSED: sole writer = the cook-off tail 0x4103e3, sole reader
  = its own gate 0x41036e, decay = the robots() pre-walk trio
  0x40bab7..0x40bac6; the §3 "scorched tiles re-burn" gloss RETIRED
  (it is the LOSING-AMMO cooldown); §7j.45 Part B's
  "producer unknown" residue closed; +0x34/+0xA4 = ZERO family
  traffic (FUN_0040e230's cells). (4) TERMINOLOGY ARBITRATED:
  §7j.45 item 4's "armor/pool/charge ticks" vocabulary SUPERSEDED
  (history-preserved corrections in §3 +0x30/+0x32 rows, §7f.4
  item 1, §7j.45 item 4): +0x30 = HEAT accumulator, +0x98 = DAMPER,
  the FUN_0040807f "armor bar" = the HEAT gauge (scale 2500 IS the
  overheat threshold), the "design intent unclear" tag RETIRED;
  the 0x40e6e2 text match = the seven-order-words walk, NOT a +0x30
  site (displacement-aware filtering). (5) CORPUS REACHABILITY =
  UNREACHABLE BY CONSTRUCTION: scorch byte (rings 1/2/4 + platform
  +4, clamp 7, fade −1/frame) arms a tile ≤7 frames → ≤+140 heat
  per event; crossing 0x753 needs ≥14 same-tile re-scorches within
  ~94 frames under a PARKED robot — no corpus scenario (S0..S8)
  comes close; below 0x9C4 ZERO RNG + only in-span robot-bank bytes
  E models verbatim → pinned chains hold; E's omissions
  (warnings/cook-off/+0x32 decay) unobservable; recorded seam: a
  future sustained-scorch scenario MUST add FUN_004102b6 verbatim.
  The "armor-pad-reads" watch id keeps its legacy name (anchor
  load-bearing). Deliverables: §7j.55 + the heat-machine ledger row
  + 3 gloss corrections + D127. Queued: the [0x4edbd8] camera-gate
  cell unit (item 2, unchanged) + the NEW robot +0x9C death-flag
  reader census unit (item 3 — pre-queue census found exactly one
  reader @0x447697 MissionShell + the two producers; §7j.45 item 6
  left it open). NEXT: item 2 (the camera-gate cell unit).
- 2026-08-23: P4/RE THE DEBRIS ARRIVAL-SFX PAIR unit COMPLETE
  (worker a553aa84 claim 2, commits 01d380b + 2728351, D124,
  §7j.52, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt + one read-only raw-dword
  scan of game-data/cd-root/BEDLAM.EXW; no Ghidra run; MANIFEST
  clean before AND after; registry_anchors 2/2 green; PUSHED).
  CLOSED with the verdict set: (1) FUN_00421e60 (118 B, 11
  callers, all inside the FUN_00420608 kind legs) = the
  BOOM1/BOOM2/BOOM3 spawn trio — [0x4ede58]≠0 gate, RandB()
  signed-idiv-3 pick (cells 0x4edf64/68/6c), play
  FUN_0043a48e(handle,0,x,y,priority 2); FUN_00421dec (116 B, 2
  callers = k2/k8) = the RICOCHT1..4 quad — RandB()&3 jump
  table @0x421ddc (cells 0x4edf98/9c/a0/a4), priority 1, one
  voice-steal class BELOW the BOOM trio; every cell named via
  the §7j.30 anchor. (2) THE RNG CORRECTION: §7j.11 item 4's
  "RandA()%3" was the WRONG draw — both bodies call RandB
  (FUN_004029b6, state 0x4ede4c; RandA 0x402975/0x4ede48 is
  drawn ONLY by k11's ~50% al&1 play gate) — corrected in place,
  history preserved; bank pick = T4 (unmodeled), k11's gate = a
  modeled RandA draw-count. (3) THE TRIGGER: all 13 sites fire
  at DEBRIS-STAGE time (entity creation, BEFORE the record
  fields — "arrival" = arrival on the field); 12 of 13 share
  one shape: per-leg in-map bounds recheck of the raw Q5 args
  (x/y≥0, x<[0x4eddec]<<5, y<[0x4eddf0]<<5; fail → ret-8, no
  record, no SFX) then the UNCONDITIONAL call; the 13th (k11
  @0x420e93) adds the RandA&1 gate (two RNGs on one leg).
  Kind→leg map re-verified byte-exact vs jump table @0x4205b8
  (6+12 and 1+13/14/15 body-sharing). Caller census COMPLETE:
  raw-dword scan → ZERO refs — the 13 direct calls are the
  whole graph. (4) CORPUS REACH: k5 via apply_damage remains
  the only reachable producer → the only reachable arrival-SFX
  site is k5's e60 leg @0x421364 (one RandB + one BOOM at the
  death position, priority 2); FUN_00421dec has NO
  corpus-reachable caller. (5) Adjacent census: third sibling
  FUN_00421ed6 = the GRUNT1/2/3 trio (RandB()%3 →
  0x4ee000/04/08, p2; callers 0x413ba0/0x413f2a = the §7j.42
  k5/6 engage-cycle juice) — the §7j.42 [identity open] gloss
  closed in place (commit 2728351); the arrival-SFX family is
  now four decode-complete members. Engine consequence NONE
  today; the beyond-k5 E-side stager must draw one RandB per
  staging (T4) at the spawn position.
  Queue: 1 = [BLOCKED] S0 live session (operator-gated,
  unchanged), 2 = the FUN_004239ef SFX-message dispatcher unit.
  NEXT: the FUN_004239ef dispatcher unit (item 2).
- 2026-08-23: P4/RE THE FUN_00419756 IDENTITY unit COMPLETE
  (worker 9a23356a claim 2, commit 224188f, D123, docs-only;
  objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra
  run, no corpus read; MANIFEST.sha256 clean before AND after,
  registry_anchors 2/2 green, PUSHED). CLOSED with the verdict set:
  (1) THE IDENTITY — FUN_00419756(x,y,z Q13) = a first-alive
  ROBOT-BANK OCCUPANCY BOX: walks 0x4c69e4/0xA8 (count [0x46ccbc],
  ALIVE gate +0x7C≠0), returns 1 on the FIRST record with
  |Δ(x>>8)|<0x10 ∧ |Δ(y>>8)|<0x10 ∧ |z@+8 raw − z>>8|<0x20 — of
  the queue's four candidates ONLY the robot bank is right, and
  it is a BOX not octile (no FUN_0041ebf8; presence predicate,
  first-match-in-bank-order, not a nearest-scan); NOT critters,
  NOT TRT structures, NOT tile words; sole caller 0x4123ae (no
  jump-table refs). (2) THE SCALE MATCH — all three axes
  normalize to Q5 (32/tile): thresholds = ±<0.5 tile lateral,
  ±<1 z level; robot z@+8 is STORED Q5 (§3 +0x08) so the raw
  compare is scale-matching, not a quirk; FUN_004197d4's robot
  lane uses the IDENTICAL box (0x419856/76/93). (3) THE CLASS-3
  VERDICT — CONFIRMED the "hit an actor but no robot damage"
  leg, and stronger: NO damage query of ANY kind on the path
  (FUN_004126dc disburser → kind-8 debris + state := 0; no
  FUN_00419aff, no 0x41a894/0x41bc1c, no 0x40e230) — ALIVE
  ROBOTS are a pure BLOCKER for the TRT bolt; its (d+1)·300
  damage is EXCLUSIVELY the class-2 terrain contact (the §7j.50
  residual closed: the bolt interacts with the squad as an
  obstruction, never a target). Two §7j.50/6 gloss fixes landed
  history-preserved: the probe takes all THREE args (z = the
  record's unstepped z), and "vz≠0 → break" is really
  skip-height-probe-only (substeps continue; the §7j.16 spawn vz
  0x14 = a ~2-frame terrain-arming delay, occupancy tested every
  substep); the write-back reverts the contact substep BEFORE
  the class dispatch → class-3 debris spawns pre-contact. (4)
  ENGINE CONSEQUENCE NONE today (T2-class bank, no watch rows);
  the future E-side TRT fire routine must reproduce the blocker
  box verbatim (else a death-POSITION divergence: bolts fly
  through the squad) + the pre-contact debris position + zero
  damage. Deliverables: §7j.51 + the ledger row "TRT-bolt
  robot-occupancy probe" + D123. Queued: the debris arrival-SFX
  pair unit (item 2, renumbered from 3).
- 2026-08-23: P4/RE THE PROJECTILE-TYPE-0x69 DAMAGE-TABLE unit
  COMPLETE (worker 6bb948aa claim 2, commit 897f524, D122,
  docs-only; objdump-only from ghidra-project/exw-text-objdump.txt,
  no Ghidra run, no corpus read; MANIFEST.sha256 clean,
  registry_anchors 2/2 green, PUSHED). CLOSED with the verdict set:
  (1) FUN_00419aff ELSE PATH DUMPED — NO memory table (inline
  binary jump tree; else = default eax=1 via 4 fall-through stubs
  0x419b57/0x419c2c/0x419c50/0x419c5e + the Watcom CROSS-FUNCTION
  SHARED-EPILOGUE gadget 0x418aa1 reached by 5 arms: 2 carrying
  the default 1 [w<2, 0x1B..0x23], 3 carrying the d≠2 products
  50·(d+1)/300·(d+1)/75·(d+1) — the §7j.17 key table re-verified
  instruction-exact incl. the ≥0x69 final else). (2) THE 0x69
  VERDICT: the per-level BEAM column is a 0x4cc654-bank STATE
  (producer = the k7 close-combat leg @0x4135a2 behind the
  d-indexed 32/16/8-frame fire gates, {z=6, TTL 0x18, +0x1A=0});
  its impact handler NEVER queries the table at its own id — it
  passes the LITERAL 0x65 (0x41215a) → 50/100/200 by d,
  TERRAIN-ONLY via FUN_0041a894; the probe counter OSCILLATES
  (k := min(k+1,7) top, k−− on contact 0x4120e9) so a blocked
  level re-damages EVERY FRAME (debris K0x14 + RandA±7 + SFX per
  contact) until the TTL-0x18 silent death; NEVER robots. No
  caller anywhere passes 0x69 (29-site census); the 7j.16 "else 1"
  guess CORRECTED. (3) The "(d+1)·300 as type 0x66" hypothesis
  REFUTED for 0x69 — that key is the TRT-bolt state 0x66 ALONE
  (producer FUN_00417698 @0x417a5c `[eax*2+0x4cc654]`; guided
  stepper ≤10 substeps, contact classes 1/2/3, class-2 terrain
  damage key 0x66 + 0x41bc1c) and 0x66 ALSO never damages robots
  (FUN_004197d4 admits 0x65/0x67/0x68 only — own-state keys for
  0x67/0x68 via the [+0x4cc652]>>16 trick, literal 0x65 for the
  0x65 state). (4) COMPLETE 25-site state-word census: 4 readers +
  12 zero-writes + exactly FIVE producers (k2 0x65 @0x41540e / TRT
  0x66 @0x417a5c / k3 0x67 @0x414b79 / k5-6 0x68 @0x413def / k7
  0x69 @0x4135a2); tick dispatch = jump table 0x411ffc on
  state−0x65 ∈ 0..4; FUN_004126dc's 0x69 arm = silent
  shared-epilogue return (defensive). Deliverables: §7j.50 + 3
  rewritten ledger rows (projectile tick / weapon damage table /
  disburser) + the §7j.14 self-type gloss corrected + §7j.16/§7j.28
  pointers + the 7j.18 residue CLOSED (3 sites). Queued: the
  FUN_00419756 identity unit (item 2) + the debris arrival-SFX
  pair unit (item 3).
- 2026-08-23: P4/RE THE FUN_00440dc2 IDENTITY unit COMPLETE (worker
  21c18e9e claim 2, D121/§7j.49, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt, no Ghidra run; read-only
  DGROUP string probes + a raw-dword pointer scan of BEDLAM.EXW
  (zero hits for 0x00440dc2/0x00440a2d/0x00440c34 — no jump-table
  refs); MANIFEST.sha256 clean before AND after; registry_anchors
  2/2 green). CLOSED with the verdict set: (1) CALLER CENSUS
  COMPLETE — exactly ONE call site 0x43dfb3 inside FUN_0043dc65 =
  the per-objective BRIEF panel renderer; strict closed trio
  FUN_0043dc65 → FUN_00440dc2 → {FUN_00440a2d, FUN_00440c34}; the
  "jmp into the caller" red flag DECODED as Watcom MULTI-ENTRY
  SHARED-EPILOGUE GADGETS (0x43c801 6-pop / 0x43c802 5-pop /
  0x43f49e) — all three return normally. (2) IDENTITY: FUN_0043d00b
  = the MISSION BRIEF screen (GameMain 0x41c4d5, ret 2 = launch);
  objective bank 24×14 B @0x4e9628 (+0/+2 marker x/y, +4/+6 TOT
  row/col, +8 counter, +0xA latch; staged by the BRIEF text parser
  0x43e5b1..0x43e7b2; "OBJECTIVE_%c%c" strings 0x4592b6/0x4592c1);
  FUN_00440dc2 = the OBJECTIVE-MINIMAP SNAPSHOTTER — stager +
  mirror materializer + FULL backbuffer zero (FUN_00440a2d), 8-z
  iso draw dest−z·0x5000 (FUN_00440c34, the real owner of
  0x440d1c/0x440d93), then a plain 2× DOWNSAMPLE
  bb[(64+2r)·0x280+64+2c] → the 256×256 cache [0x46cbb0]
  (alloc 0x10100), flag [0x4dc6c0], consumer = the flag-gated
  transparent palette-remap blit FUN_00402a28 @0x43d9a2; the
  `mov ecx,0x10000` @0x440de2 is the pre-set zero-fill count, not
  a stager arg. (3) MID-FRAME/§1 ORDERING CLOSED BY SCREEN
  LIFECYCLE — FUN_00403938 is called only from MissionShell
  (0x447c9b/0x448094); the BRIEF holds its OWN [0x4ede18] alloc,
  so no mission frame can be wiped and no in-game path exists.
  (4) GLOSS CORRECTIONS landed with history preserved: 7j.26
  "scroll/camera restamp stager" → BRIEF minimap window stager;
  [0x4ede24] = per-screen cell reuse (BRIEF 49×12 list vs mission
  1296×12 viewport cache, FUN_0041d954 = the in-game producer);
  7j.36 cluster-(b) drawer = FUN_00440c34 BRIEF-only; 7j.16 lead
  RESOLVED. Engine consequence NONE (BRIEF screen outside the P4
  diff scope; no new watch rows). Queued: the projectile-0x69
  damage-table unit.
- 2026-08-23: P4/RE THE MISSIONVIEW §5d TAIL unit COMPLETE (worker
  328b7651 claim 2, commit dd8d5e2, D120, docs-only; objdump-only
  from ghidra-project/exw-text-objdump.txt, no Ghidra run;
  ADOPTED + VALIDATED the interrupted predecessor WIP in
  RE-EXW-MISSIONVIEW.md §5d — every claim re-verified at the asm
  level before landing; read-only corpus probes on
  game-data/BEDLAM/GAMEGFX TELEPORT/SHIELD/ROBNUMS/TINYFONT
  headers: 10/4/9/118 imgs; MANIFEST.sha256 clean before AND
  after; registry_anchors 2/2 green). CLOSED with the verdict
  set: (1) §5d label CORRECTIONS — 0x46af38 = TELEPORT.BIN (10
  imgs; the state-5/6 draw is the BEAM: mode 0x12e, sy−0x48,
  clamp(10−wobble/4,0..9) @0x403de6..0x403e71), 0x46af44 =
  SHIELD.BIN (4 imgs; the +0x88-gated draw @0x403ef4..0x403f29 =
  the shield, RandA()&3 spawn + (+1)&3 shimmer @0x403cf7);
  (2) STAGING — alloc FUN_0041d954 @0x447860 (TELEPORT 0x6d60 /
  NUMBERS 0xfa0 / FLAGS 0x3a98 / ROBNUMS 0xbb8 / SHIELD 0x1b58) +
  LoadFile FUN_0041df10 @0x447b3f (TELEPORT@0x41df99, SHIELD
  @0x41dfe9, ROBNUMS@0x41dff9), both single-site on the
  straight-line MissionShell (FUN_0044771c) head — EVERY mission,
  SP included, NO gate; (3) ROBNUMS.BIN = DEAD DATA (sole reader
  = its own load site 0x41dffe; the plates draw TINYFONT
  0x46cdb0, 118 glyphs, ASCII−0x21, shared with map markers +
  sidebar text); (4) NAME-PLATE GRAMMAR — gate [0x4edb88]≠0
  @0x403fb9 (SP never; the ==2 arm @0x403c62 is the 7j.31 MP
  hot-rect, a different consumer), glyph g = [0x4e4458+id*9+i],
  skip g > 0x40 (jl arm dead), x = sx + u32[0x4e44c8+id*4] + 6·i,
  mode 0x12c; 0x4e44c8 = id-indexed CENTERING = 32−3·strlen
  (writer 0x447ce0..0x447d85: memset, toupper(FUN_0044f067)-copy
  from raw 0x4e43e0 storing c−0x21); (5) UNSTAGED-FLUSH RETIRED —
  no bank-zero skip in FUN_0040798e (only bx/by<0) or FUN_0040179b
  (only the unknown-mode RET @0x4017e0; drawn modes deref the bank
  unchecked) — an unstaged bank would FAULT, and per (2) can never
  occur; E needs no unstaged-skip logic (lazy staging is
  unobservable). Ledger row + §5d/§8 notes updated. PUSHED dd8d5e2.
  Queued: the FUN_00440dc2 identity unit (item 2).
- 2026-08-23: P4/RE THE TOT PLANE-6/7 SEMANTICS unit COMPLETE (worker
   f29066bd claim 2, commit dc6f5bf, D119, docs-only; objdump-only from
   ghidra-project/exw-text-objdump.txt, no Ghidra run; read-only corpus
   probes over game-data TOT/DAT/POS in /tmp/opencode — manifest clean
   before AND after). CLOSED with the verdict triple: (a) RENDERER —
   plane-6/7 mirror words DO draw, NO z≥6 gate in ANY consumer family:
   the FUN_00403938 restamp z-stack loop 0x4067cf..0x406c73 runs z 0..7
   (outer `cmp 8` @0x406863, chain @0x40695c) with the Block-1 restart
   draw @0x406882..0x406941 gated on word≠0 ALONE (no seen — seen only
   short-circuits the contiguous Block-2 chain; the cursor k resets to 0
   per record @0x406c00/08); init_tiles stages all 8 planes (`cmp 8`
   @0x407fce); the overlay scanner 0x408a49..0x408ade walks planes 1..7
   and the range consumer 0x42035c..0x4203a5 planes 0..7; (b) CENSUS —
   36/37 missions (only ZONEG/M1 zero), 8 016+2 882 words in 9 296
   cells (6 504 overlay on planes 0..5 / 2 792 standalone), value
   domain IDENTICAL to planes 1..5 (35..1868 vs 33..1868), DAT bytes
   overwhelmingly 1 (~93% seen=1 at load); the words are per-level
   sprite ids of TALL STRUCTURES (ZONEA/M1 (17,25) = the one zone-A
   cell: column [454,1354,1355,1356] at z=4..7 — the "1355/1356
   adjacent integers" are the z-6/z-7 sprite ids; ZONEB ramps 1866/
   1867/1868 + 1755→1753 + 1153..1161 sequential runs); (c) the
   ~2000-entry target-table hypothesis REFUTED (planes 1..5 reach 1868
   too — the nearness is the word grammar; .POS resolutions 9 217 live/
   1 681 empty = coincidence, ZONEA's pair hits EMPTY slots; p7==p6+1
   at only 83/9 296). Deliverables: RE-EXW-SIM §7j.47 + the ledger row
   "TOT plane-6/7 semantics" + FORMATS §2 planes-6/7 paragraph CLOSED +
   §12 cross-ref + the cross-file LIKELY row → REFUTED + D119. Engine
   consequence NONE (E already stages every nonzero plane word per
   D107; no new watch rows). registry_anchors 2/2 green. Queued: the
   MISSIONVIEW §5d tail unit (item 2 — ROBNUMS name plates + Shield/
   Variant bank staging; pre-queue grep performed per D118).
- 2026-08-23: P4/RE QUEUE HYGIENE #3 — THE .BDG TEMPLATE-BANK ↔
  RESTORE-WORD MAPPING item REMOVED AS ALREADY-CLOSED (D96/§7j.32,
  2026-08-22, commits 4210f55 + f554bee; worker e26508a9 claim 2,
  D118; docs-only). The queued item text was stale pre-D96 state
  copied from the Backlog's RETIRED-D93 bullet (the closure sat in
  the Done log AND D96 at queue-write time); the bullet's stale
  parenthetical is now annotated CLOSED in place (the D111 lesson
  extended: a COPIED stale parenthetical defeats the headline grep
  — re-check the bullet before copying). The closure re-verified
  genuinely green at HEAD with FRESH evidence, every leg
  reproducing: the loader disk order (stores +0x3E/+0x46/+0x42/
  +0x4A at 0x41a727/0x41a742/0x41a75d/0x41a77c), the restore
  three writes instruction-exact (mirror word ← +0x46 0x41ab59→
  0x41ab6b; seen := +0x4A word==0 0x41ab72→0x41ab80; DAT volume ←
  +0x4A&0xFF 0x41ab8a→0x41abdb; index (z'·H+i)·W+j), the
  zero-reader census both legs (absolute 0x4dee30/0x4dee34 =
  loader stores only; 6×[reg+0x3e] + 12×[reg+0x42] displacement
  sites, none type-table-relative; arena 0x46ad5c loader-only),
  and the corpus numbers byte-identical from a fresh parser
  (ZONEA/M1: 211 typed instances, 435 cells — 434/435, 11/435,
  434/435, 155/435 + the (14,29,z1) overlap cell
  last-.POS-slot-wins). NEW method note (D118/3): the TOT
  word-plane header is WORD-unit (planes start at BYTE 4 = WORD
  2) — a byte-unit +4×2 double-count yields a false 67/435 (this
  run's own first pass); the u8 DAT path is immune. Backlog
  hygiene: the D93-bullet parenthetical annotated CLOSED;
  registry_anchors 2/2 green; manifest clean before AND after;
  no Ghidra run, no corpus write. Queued: the TOT plane-6/7
  semantics unit (item 2, pre-queue grep performed per D118).
- 2026-08-23: P4/RE THE FUN_00433980 CASE TABLE + FUN_00424a6f
   MESSAGE TABLE unit COMPLETE (worker 0c2df9b4 claim 2, commit
   fcf97c3, D117; docs-only; clean objdump windows on the read-only
   binary — the flat exw-text-objdump MISPARSES the 0x43301c..0x433963
   table farm, so targeted `objdump -d/-s --start/--stop-address`
   re-disassembly + a static Watcom-cascade walker were used; no
   Ghidra run, no corpus write — manifest clean before AND after;
   registry_anchors 2/2 green; PUSHED). §7j.46 landed with: (a) the
   FULL per-zone case table as §8-bis (every zone A..G × SP/H2H ×
   mission × .PAD slot → action; the zone table @0x433964; mode
   [0x4edb88] / mission [0x4edd88] gates; mission tables B 0x4331d0,
   D 0x433650, F 0x433950); (b) the RIDE-RECORD BANK grammar — the
   7j.19/7j.21 "dword tables 0x4dcdbc..0x4dd330" = one 0x24-stride
   bank {+0/+4 dest tile, +0x18 latch :=10, +0x1C rider gate}, 16
   records, y-stamp shared tail 0x43475f, +0x84 arrival-plat 0..0xE;
   (c) the action census — 21 SP BEACON slots, DOOR rects 0..0x25,
   zone-F/G DOOR+FUN_0041fa51 EXIT pairs (the "sole case 0x1B" gloss
   retired), zone E VERIFIED NEGATIVE (overlay restage only, no
   probe/cases; 5-pop-thunk quirk recorded); (d) FUN_00424a6f = the
   ZONE-A-M1-ONLY message shower (sole caller 0x433d07): SP-only,
   show-once latch 0x4eb5f8+2·id, name = BOOT_CAMP_%03i sections of
   the LANGUAGE.{ENG..DCH} blob [0x46cbb4] (FUN_00424679 section
   finder; LANGUAGE.ENG = 421 sections; the 15 BOOT_CAMP ids = the
   zone-A M1 message slots exactly); (e) the timer semantics —
   [0x4eaac0] := 0xFDE8, ticker/drawer FUN_00425010 (MissionShell
   0x448381) decrements, the FUN_00409138 COMMAND sites
   0x40a2bc/0x40a396 DISMISS (≥8/44 frames), 0x40c570 gates the
   state-0 write while showing; the producers' "msgs 9/10/0xB/..."
   clarified as FUN_004239ef SFX ids, not text. Ledger: the
   pad-trigger dispatcher row rewritten + 2 new rows (zone-A message
   shower, ride-record bank). Queued: the .BDG template-bank ↔
   restore-word mapping unit (item 2).
- 2026-08-23: P4/RE THE RE-EXW-SIM §9 ITEMS 2-3 REMAINDER unit
   COMPLETE (worker c607288e claim 2, commit 47357ca, D116; docs-only;
   objdump-only + read-only DGROUP string probes, no Ghidra run, no
   corpus write — manifest clean before AND after; registry_anchors
   2/2 green; PUSHED). §9 item 2 CLOSED by §7j.45 Part A: FUN_00440e45
   = THE SHOP, instruction-exact — the §7d asset list lands exactly
   (WEAPICON/CONLITE/SHOPFONT/SHOPLITE + DARKPALS/SHOPPAL + SHOP.SMK
   gated by the NEW animations pin [0x46cca4] + SOUND\MIDI\SHOP), the
   MONEY FLOOR (>=100 at entry), the MP/zone-7 16-dword LOCKOUT array
   0x46cd48..80, the 9-category CATALOG grammar @0x4ea288
   (immediate-staged; cat 8 = the equipment chassis 0x2A..0x2E with the
   0x2D/0x2E mutex), the full buy/sell/auto-loadout/confirm machine,
   the WEAPON-GROUP LAYOUT CORRECTION (+6 price/+8 category/+0xA item —
   §7d sat one slot low), and the MP SHOP SYNC CONFIRMED: the exit
   appends the type-4 COMMAND record (FUN_00449c94(4, 0x4e43e0), the
   63-B staging struct MissionShell/save consume) then walks players
   p < [0x46cbe0] mirroring each record's 7 (name,ammo) pairs into
   0x4de664+p*0x62 (a FOURTH weapon-table writer family). §9 item 3
   CLOSED by §7j.45 Part B: the phase-0 pre-pass timer decays (+0x32
   BURN cooldown :=100 via the FUN_004100b7 scorch lane; +0x34 ALARM
   cooldown; +0xA4 alarm counter DOES decay 1/frame — the D90 question
   closed; the queue's "0x4c6a8c" = zero sites, the intended pair
   +0x88/+0x8C), the SHIELD machine (+0x88 points: -2/frame, 0x20 per
   charge/state-3, 0x2710 INVULN during the +0xA0 flash + the player
   palette strobe; +0x8C CHARGES sourced from the equipment-chassis
   row word+2 via the 0x40cc8c jump table), +0x70 = the REINFORCEMENT
   delay with the NEW pending gate [0x4de658] (:=0x80 at the arm), the
   0x7d3 tile gate CORRECTED (countdown-dependent phase bound), and
   the STATE-1 CENSUS: exactly ONE producer — FUN_00409138's COMMAND
   bit0 arm (0x40a37b); NO patrol semantics (SP never produces state
   1 — why S6 needed the inject seam). Deliverables: §7j.45 + 10
   ledger/§3 rows + §5/§7d corrections + GAMETHREAD gloss
   retirements + D116. Queued: the FUN_00433980 case table +
   FUN_00424a6f message table unit (item 2).
- 2026-08-23: P4.2/debris-physics THE FUN_0040de9c FAMILY unit
   COMPLETE (D115; the §7j.44 RE decode d467471 + the engine leg
   cebc178 landed by predecessor a5ef2370 claim 2, which died at
   session end mid-re-baseline leaving the gate pins uncommitted;
   continuation worker 07ce0c25 claim 2 ADOPTED the WIP +
   INDEPENDENTLY re-verified + completed the re-baseline/docs/
   queue legs, commits b2c89af + c4af24b): the tick FUN_00420549
   (delay/anim/free lifecycle + the phys gate, MissionShell
   epilogue slot) + the pass — the +0x20 phys word is a COUNTDOWN
   (dec-on-exit; the 0x454510 "param table" DISPROVEN, closing
   7j.11/5), mag = kind==12?25:2, knock_mult = min(phys,3),
   critter radius = min(16·phys+0x20,0x60); the ROBOT lane (the
   W12-S8 FUN_0040db9e dispatcher: damage + facing −1 + the
   ≤3-px knock, five-k5 death tail), the TERRAIN-GATED critter
   lane (3-row plane-0 dword probe; per-kind get/set scales; the
   §7j.24 register-gloss correction — edx = knock/sin-cos factor,
   ecx = the 2/25 hp subtraction), the POI squash lane E-only.
   RE-BASELINE (the physics turn-on makes chunks mutators on the
   aliased robot bank — mines/grenades expire to k12 mag-25 so
   even NON-destroy scenarios move): S3 → 9a11efa03baafb64, S4 →
   35fa3a9234cbff37, S5C → 786fd87565b67f4a (the case-3 consume
   flips to the gunner — the +2500 heal stays exact/unclamped,
   the scenario's purpose intact; an O1 capture arbitrates), S7
   → ecdce5472df6a324, S8 → 44d806b81bd1b1ff; S0/S1/S2/S5/S5B/
   S6 BYTE-IDENTICAL (staging-key discipline holds). The
   observability pairing: corpus_s4 (the knock-widened cascade,
   15 destroyed + the freed-ring lifecycle — 60 live at the tail,
   the old never-free 128 saturation gone), corpus_s7 (the
   standing gunner's chunk-field schedule: 19 hp-change frames
   f32..f50, 1248 total spend), corpus_s8 (the burst-window
   chips, end 3041). No new S-variant needed (the queue's
   conditional satisfied — debris damage DOES land in-scenario);
   the differ contract unchanged (zero new rows). Verified:
   workspace 54 suites green, fmt+clippy clean, manifest clean
   before AND after, PUSHED. Queued: the RE-EXW-SIM §9 items 2-3
   remainder unit (item 2).
- 2026-08-23: P4.2/W12-S8 THE E-SIDE CRITTER-ENGAGEMENT PRODUCER
   unit COMPLETE (D114; the §7j.42 RE decode b3e78cb+05f0d95 by
   predecessor f9af5743 claim 2; the engine leg 8786c9e + the
   differ/plan/docs legs ebf1d0b by predecessor 40dd9473 claim 2,
   which died at session end AFTER pushing but BEFORE the queue
   rewrite; continuation worker b22cba4a claim 2 adopted the
   pushed state, INDEPENDENTLY re-verified green, and completed
   the queue leg): bedlam-core::critter (the 0x4cff98 bank + the
   .NME staging host seam with fail-loud kind refusal, the k4
   seek steppers + the k5/6 mixed-AI body per the FIVE §7j.43
   corrections, the mode-2 0x68 fire cycle with the 3-D aim, the
   odd-pass FUN_004197d4 walker + FUN_004190bc applier, the §7j.24
   death handlers + the 80-row LRU effect bank, the bounty gate),
   grammar v1.7 `critters = 1` (staging+arm; the loader/controller
   RNG draws = the budgeted E-side stream gap on unarmed
   scenarios), S8.scen (T0/T1/T2/T3/TS, 121 records, chain
   b5ae3f8be91c7449, double-run byte-identical; the (18,13)
   FLAT-row engage instrument + the 9-death burst window),
   corpus_s8_critter_engagement + differ_gate S8 row (cross
   PASS-WITH-NOTES, exactly the 2 S1-class + the critter/effect
   E-only pair, zero field gaps) + capture-plans/S8.json
   byte-pinned (36 anchor + 27 per-frame, 26 deferred unaliased
   rows, 1 command inject). THIS RUN RE-VERIFIED: canonical_dump_
   gate 13/13 (S0..S7 byte-identical + S8 pinned), differ_gate
   green, dbx-plan 31/31 (both S8 plan tests), workspace 54
   suites green, fmt clean, clippy clean — 3 leftover lints in
   the S8 canonical test fixed (identity op + 2× get().is_none()
   → contains_key), manifest clean before AND after. THE W12
   SERIES (S3..S8) IS COMPLETE — the queue successor is the
   FUN_0040de9c debris-physics family (item 2).
- 2026-08-23: P4.2/W12-S7 THE PLATFORM-DYNAMICS PRODUCER unit
   COMPLETE (D113; the §7j.41 RE decode 984a078 + the engine leg
   ea2f259 + the scenario leg b9cbcf3 landed by predecessor worker
   56d80c42 claim 2, which died at session end BEFORE the
   differ/plan/docs legs; continuation worker 0b66f6a6 claim 2
   adopted the pushed state + completed them, commits 4c6c068
   (differ_gate S7 row + the stale scenario-comment creep schedule
   corrected to the pinned timeline) + 13bae85 (dbx-plan
   `platforms` _e_staging note + capture-plans/S7.json, 34 anchor +
   25 per-frame, 5 command injects, byte-pinned) + the docs commit
   (D113 + DESIGN §7/§10-W12 + the §8 ledger rows rewritten with
   the §7j.41 corrections + §7j.41 LANDED note)). S7.scen: the full
   lifecycle in ONE ZONEA/M1 run — the FUN_00422600 zone-code
   trigger build at .POS slot 74 (3,57,2) (the gunner's own
   quadrant blocks 3 of 8 ring tiles — the live-robot gate
   OBSERVED), the same burst's pair-7 destroy (first k7), the
   corrected weaken ring gates (300→150 SPREAD, 150→75 site
   latch), the destroy tail (k7 census 5→20), and the armed creep
   (grammar v1.6 `platforms = 1`; THE PER-FRAME RandA GATE-DRAW
   finding — the original consumes one gate draw per frame even
   unarmed, an E-side stream gap on S0..S6 until a deliberate
   re-baseline; the live-capture note folded into queue item 1).
   1361 records, chain b41db389f3ad8947, double-run
   byte-identical; corpus_s7_platform_dynamics gates the full
   timeline (the 0x25D water word + seen-0 volume-2 semantics);
   differ_gate S7 row (cross PASS-WITH-NOTES, exactly the 2
   S1-class + the debris/splash E-only pair, zero field gaps).
   S0..S6 chains re-asserted BYTE-IDENTICAL; workspace suites
   green, fmt+clippy clean (one predecessor clippy lint fixed),
   manifest clean both sides, PUSHED. Queued: W12-S8 the
   critter-engagement producer unit (item 2).
- 2026-08-23: P4.2/W12-S6 THE EXTRACTION SCENARIO unit COMPLETE
   (worker 4d92bb13 claim 2, commits bcf5396 (scenario+harness+gate)
   + 0545e2e (differ+plan) + the docs commit, D112; the §7j.40 RE
   decode 631bd28 + the engine extraction family edafd02 landed by
   predecessor 8d32d85d whose interrupted harness WIP was ADOPTED +
   completed this run). S6.scen (T0/T1/T3/TS, 75 records, chain
   c96f0735df1059ea, double-run byte-identical): the walk is
   COMMAND-driven (bit0 SELECT = state 1 + move-target, NO pending
   order — a click never arms the beacon and E's `order` blocks the
   pad trigger, §7j.40/5); `pad 18` = slot 0x12 = (19,70,0), the
   census GROUND pad (the queue's `pad 8`/(5,61,0) gloss predates
   the verified census — slot 8 is LEVEL 1, unreachable for a
   ground robot; deviation recorded in D112); two legs cross the
   pad mid-walk → sub-tick probe → state-3 halt + same-frame deploy
   (single-robot window 0) → descent f14..34 → sweep f35 (state
   3→5, stop 1e6) → RandA-jittered dwell f36..44 → departure drift
   → complete f69. THE .PAD TERMINATOR BUG fixed (the dead `x ==
   -1` break on a `u16 as i32` read — the slot bank carried the
   0xFFFF fill past the live run; the D86 missing-slot rejection
   now fires; ZONEA/M1 = exactly 114 live slots). corpus_s6 gate
   pins the full timeline; the dropship-frame differ normalizer (7
   i32 leaves, E-only — a coverage finding, never fabricated);
   differ_gate S6 row (cross PASS-WITH-NOTES, 2 S1-class + the
   dropship row, zero field gaps); capture-plans/S6.json compiled +
   byte-pinned (3 injects: the pad op + two command records; NO
   staging seams). S0..S5C chains re-asserted BYTE-IDENTICAL;
   workspace 54 suites green, fmt+clippy clean, manifest clean both
   sides, PUSHED. Queued: W12-S7-prep the platform-dynamics
   producer unit (item 2).
- 2026-08-22: QUEUE HYGIENE unit #2 (worker e444e1cd claim 2,
   D111): the claimed queue item 2 (the MISSIONVIEW §8
   water-flag/anim remainder) was found ALREADY CLOSED at HEAD —
   closed by worker 57ba8753 claim 2 as D100/RE-EXW-SIM §7j.35
   (commits bee4336 + 60f7d3b, ~15 units BEFORE S5C), but the
   S5C unit's queue note (105d9aa) re-queued the closed unit as
   item 2 by mistake (a hand-written stale queue note, NOT a
   scheduler bug — D106's DONE-marker fix doesn't apply;
   nudge-free-items.py untouched; a Done-log-aware semantic
   dedupe was judged too fuzzy to automate). Re-verified green
   at HEAD this run: §7j.35 covers every deliverable the
   re-queued item asked for (the u32[0x456ca8] family = a STATIC
   DGROUP ping-pong const, 2 readers/0 writers; the [0x4edbd4]
   water-flag 3-writer census ≡ 1 every mission; the 7j.12
   zone-table off-by-one ledger correction; MISSIONVIEW §8 all
   FOUR items closed; the §0b verdict = nothing new to watch;
   D100 recorded) — plus INDEPENDENT spot-checks: objdump re-grep
   confirms 0x456ca8 exactly {0x40691a,0x406a2c} readers/zero
   writers and the 0x4edbd4 {0x4252d8 persistent, 0x41c649/
   0x41c65a bracket} writer census; the file-image read at
   0x552a8 confirms u32[16] = {0..7,7..0} byte-exact;
   registry_anchors 2/2 green; MANIFEST clean before AND after
   the read-only corpus probe. Stale Now item removed (W12-S6
   renumbered to item 2), the stale Backlog PROMOTED note folded
   to CLOSED. No engine/doc/tool change beyond the queue +
   DECISIONS D111.
- 2026-08-22: P4.2/W12-S5C THE CASE-3 OBSERVABILITY VARIANT unit
  COMPLETE (worker 82d5a27f claim 2, commit c27b3db, D110; scenario
  + tests + plan, unattended-safe; NO engine change, no Ghidra run,
  corpus read-only — manifest clean). CLOSED the D108
  value-invisibility gap: S5B's walker spawned AT the 5000 clamp so
  apply_pickup case 3's +2500 read 5000→5000; S5C.scen spends the
  walker to 1256 BEFORE the walk with the S4 artillery pattern (a
  third marker stages the gunner ON the walker's tile (73,10,3) —
  ≤5 tiles from the order tile, inside ORDER_RADIUS; loadout
  9/0xA/0xB 1 ammo each; the frame-1 command fires all three
  records at the gunner's tile; the §7j.23 robot lane 312/pair
  box-reaches a +0xF00-offset robot from exactly FOUR list-0 pairs
  {T,T+1}×{Ty,Ty+1} per burst → 3×4×312 = 3744 at frame 32 on the
  walker AND the gunner, both survive at 1256; the 0xB outer ring
  spends the clicker 624 at f36; all damage pre-order at state 0/3
  — the hp path, a state-4 robot converts damage to a shield tick).
  `order 78 10 3` arms at f37; CASE 3 AT FRAME 41 heals the EXACT
  +2500 UNCLAMPED (1256 → 3756 — better than the ticket's
  clamp-tolerant bar); the gunner claims its own spread slot and
  walks one robot behind (lower index moves first — reaches no
  unconsumed cell, hp 1256 through the tail = the same-run negative
  control); arrival f48 snapped (78,10). CAVEAT recorded: the burst
  rings detonate the destroy CHAIN CASCADE (232 off-corridor mirror
  cells change — S5B's six-cell whole-map census does NOT hold;
  asserts target the corridor cells + the hp schedule; the differ
  passes with exactly the 2 S1-class findings — the cascade rides
  the SAME aliased T1 rows). 55 records, chain e0999fcb3455d3ef
  pinned + double-run byte-identical (canonical_dump_gate
  corpus_s5c_pickup_case_3_predamaged); differ_gate S5C row;
  dbx-plan compiles tiers T0/T1/TS (4 inject rows: the frame-1
  command append CS:0009255C + the frame-37 order triple
  CS:0010E0A4/A8/AC; the loadout seam in _e_staging; the command
  record's triple = the order tile in RAW Q5 words (2496,320,3));
  capture-plans/S5C.json committed + byte-pinned. S0..S5B chains
  re-asserted BYTE-IDENTICAL (54 suites green, fmt+clippy clean,
  manifest clean). PUSHED. Queued: the MISSIONVIEW §8 water-flag/
  anim remainder (item 2, re-queued from the D99 plan — the S5
  series superseded it) + the W12-S6 extraction scenario unit
  (item 3).
- 2026-08-22: P4.2/dbx-plan-tiers THE T2/T3 TIER COMPILE unit
  COMPLETE (worker 33a28c84 claim 2, commits a784e49 (dbx-plan +
  capture-plans) + 690d8b0 (capgen prefix + flow probe) + 4db7ba1
  (D109 docs), D109; tooling, unattended-safe; no engine change, no
  Ghidra run, no corpus write — manifest clean). SUPPORTED_TIERS
  widens to T0/T1/T2/T3/TS: S3 (T2) and S4 (T0/T1/T3) capture plans
  COMPILE — capture-plans/S3+S4.json committed + byte-pinned by
  tests (S3: 36 anchor + 27 per-frame, 8 command inject rows, the
  loadout seam recorded with a DECIMAL mask — the D103 hex literal
  made loadout plans unparseable JSON, surfaced by the first
  compilable one; S4: 34 + 25, 3 injects). The two aliased T2 banks
  emit as the FULL fixed spans the differ's O1 normalizers pin
  (weapon-anim 0x980d4 × 400*0x36 = 0x5460, projectile 0x10e174 ×
  50*0x22 = 0x6A4; no count cell on the guest); ALL unaliased T2/T3
  rows (mortar/critter/POI + the 14 T3 rows incl. debris-stager +
  splash-records) STAY refused — explicit _deferred coverage gaps,
  never emitted (tests pin the refusal); a future aliased row needs
  a deliberate form (indirect/count-driven extents die loudly).
  THE COUNT-PREFIX GRAMMAR: the differ pins trt/object O1 blobs as
  u32 count + records but the count cells (0x11949c/0x119554) are
  not contiguous with the banks — capgen watch rows gain a `prefix`
  {addr,len} sub-row (dump 4 B first, concatenate; the flow probe
  gains the probe-flow-prefix row + assertions, GREEN headless; all
  four dbgprobe modes re-verified) and dbx-plan emits Prefixed for
  trt-array + object-instances (robot-bank stays the bare span its
  normalizer defines). OBJECT-INSTANCES now dumps the WHOLE
  2000*0x14 bank + count prefix (the D108 ZONEB .POS live-past-dead
  holes — the count-bounded span dropped 32 live objects and broke
  the count field; $obj_count retires as a resolve symbol). S1/S2/
  S5/S5B plans REGENERATED (prefix + full-bank rows); S0/S0W
  byte-identical (artifact tests green). Workspace 54 suites / 632
  tests green, fmt+clippy clean, registry_anchors green. LIVE-SESSION
  NOTE folded into queue item 1: re-stage any plan after D109.
  Queued next: W12-S5C (item 2, renumbered).
- 2026-08-22: P4.2/W12-S5 THE S5.SCEN + CANONICAL-CHAIN unit
  COMPLETE (worker c2aba48b claim 2, commits 66ad013 (RE/design
  notes first: the ZONEB/M1 corridor census, the order-window
  two-scenario-split analysis, D108) + 3626010 (grammar+engine+
  scenarios+tests+plans)). Grammar v1.5 `zone = "B"` (the
  GameHost::stage_episode_slot D51 seam — fsm Episode::stage_slot +
  SceneFsm + host wrappers; the campaign-advance/save-load shells;
  mask 0 → MISSION1, linear the fresh-slot 0) + `pickup = 1` (the
  mission's own .TOT through stage_pickup_surface AFTER the destroy
  staging + the §7j.12/6 hazard stamper — the original mission-load
  order; 30 ZONEB hazard grid cells). S5.scen = the row-21 z3
  corridor (cases 1/2/4 at (26,21)/(27,21)/(28,21) — the ONLY c1+c2
  co-walkable spot in the corpus; clicker (28,21,3) + walker
  (25,21,3), order 28 21 3 → slot-1 (29,21); consumes at frames
  1/2/4 = drop_countdown 1000 / shield 1000 / score+1000, arrival
  frame 5; 16 records, chain a4659f25d453b6a1) + S5B.scen = the
  row-10 z3 corridor (case 3 at (76,10) + 4× c4 + the (76,9)
  diagonal side cell = 6 consumes; arrival frame 12 at (78,9); 19
  records, chain 93e976587a98d2a1) — the SPLIT is forced by the
  order-window semantics (c1↔c3 are 61 octagonal tiles apart; a
  second order needs the first cleared = all-alive-state-3, which
  mid-scenario robots can never reach, or the 0x197-frame window ≈
  407 idle frames × ~340 KB/record of REAL mirror rows). The
  typedb-mirror-rows go REAL on pickup runs (15,102 words + 52,715
  seen, every tile active; S4's empty-mirror divergence closes; the
  S4 chain untouched — S4 sets no pickup key). DIFFER FIXES the
  ZONEB surface exposed: (a) the O1 zone-row normalizer maps cell−1
  (§6a zone convention: the guest cell 0x4edd8c/0x107500 is the
  1-based set per D99, E canonical the 0-based slot index; + the
  differ unit test + the differ_gate inv fabrication cell+1); (b)
  the O1 object-instances walk covers the WHOLE dumped span
  (ZONEB/M1 .POS carries live slots past dead holes: 1096 live,
  max slot 1128, first hole at 303 — the count-bounded walk
  silently dropped 32 live objects); (c) the field-union join is
  hash-indexed (the mirror rows carry ~170k fields/frame; the old
  linear union + per-name lookup was quadratic — the real-dump diff
  went 5+ min → 3 s). dbx-plan: the zone+pickup seams record in
  _e_staging (multi-entry stagings are strict JSON again — the
  join fix; S0/S0W/S1/S2 plans BYTE-IDENTICAL, re-verified);
  capture-plans/S5+S5B.json compiled + committed (34 anchor + 25
  per-frame rows, tiers T0/T1/TS — no T2/T3 unit needed).
  VERIFIED: workspace 54 suites / 629 tests green incl. the S0..S4
  chains BYTE-IDENTICAL (8901789a88cf61fe / 1c4e7b4c9d9b0947 /
  809f4961b7757da4 / e29f76f5585401e1 / 2ddd15ea50c8a14d) +
  differ_gate S5/S5B rows (cross PASS-WITH-NOTES exactly the 2
  S1-class findings, double-run PASS modulo counter/RNG, FAIL on
  money) + canonical_dump_gate corpus_s5/corpus_s5b (whole-map
  consume censuses: exactly the corridor cells); fmt+clippy clean,
  registry_anchors green, manifest clean before AND after the
  corpus runs; PUSHED. Queued: dbx-plan-tiers (item 2) + the
  W12-S5C case-3 observability variant (item 3, the pre-damaged
  walker follow-up).
- 2026-08-22: P4.2/W12-S5-prep THE E-SIDE PICKUP PRODUCER unit
  COMPLETE (worker f32193a2 claim 2, commits ad43c12 (RE notes
  §7h.5 first) + 7a2dfeb (engine+tests) + D107). bedlam-core::
  mission: `stage_pickup_surface` (the init_tiles@00407e11 host
  seam — the .TOT volume parses, EVERY nonzero plane word stages
  into the mirror, the DAT byte gates ONLY the seen flag, zone :=
  zone_index+1 per D99), the clear→move→test→fire consume
  protocol in robots_phase (last_trigger := None @0x40bef2 →
  robot_move → test @0x40bf0b → fire_pickup: DAT byte := 0 /
  mirror word := PICKUP_FLOOR_WORD (table C) / seen := 1 → the
  dispatch; the FOUR probe-latch sites were already modeled in
  floor_z), apply_pickup WIDENED (case 4 draws row+amount on the
  shared stream + the pending (score,money) award the MissionShell
  folds beside the destroy fold + strip arm; cases 8/9 effect ids
  0xC/0xD host-seamed — no shipped cells stage them), PICKUP_AWARDS
  moved core-side. §7h.5 DERIVATION: the range/floor tables are
  zone_index-0-BASED (the 0x454a04..0x454ac8 DGROUP family is one
  contiguous 7-dword/0x1C-stride run — no head slots, so
  base+(cell−1)·4; corpus-confirmed) — and the PRE-EXISTING
  destroy.rs zone tables (RUBBLE/HAZARD/WATER) are raw-cell with
  heads, FLAGGED for the S5/S7 differ to arbitrate (corpus-dead).
  VERIFIED: 4 new synthetic tests + the corpus gate (ZONEA/M1
  stages the real TOT with ZERO fire traffic — the D99 census
  re-derived live: 80 cells, the exact word multiset pinned, the
  staged walk hash-trace-identical to the bare walk; ZONEB/M1
  positive control: 199 cells/152 in-range/case-4 dominant),
  canonical_dump_gate 7/7 with S0..S4 chains BYTE-IDENTICAL
  (8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 /
  49193732e6dbc546 / 2ddd15ea50c8a14d), differ_gate 7/7,
  destroy_gate 16/16, weapon_fire_gate 28/28, registry_anchors
  2/2, workspace green, fmt+clippy clean, manifest clean both
  sides. Queued: W12-S5 the S5.scen unit (item 2).
- 2026-08-22: QUEUE HYGIENE unit (worker 78203f4f claim 2, D106):
  the claimed queue item 2 (W12-S4) was found ALREADY CLOSED at
  HEAD (b8925a9, D105, pushed) but left as one of five stale
  "2. DONE ..." blocks in Now that the scheduler kept respawning
  (marker-blind enumeration). This unit re-verified the closure
  green at HEAD (differ_gate 7/7, destroy_gate 16/16,
  canonical_dump_gate full-chain assert incl. S4
  2ddd15ea50c8a14d, weapon_fire_gate 28/28, registry_anchors
  2/2, MANIFEST clean), taught nudge-free-items.py to skip a
  first-word DONE marker (+ test-nudge-queue.sh case), removed
  the five stale DONE blocks (all already in the Done log), and
  renumbered the open items: 2 = W12-S5-prep, 3 = dbx-plan-tiers.
- 2026-08-22: P4.2/W12-S4 THE S4.SCEN + CANONICAL-CHAIN unit
  COMPLETE (engine+tests by a predecessor session left
  uncommitted by session death — ADOPTED + FIXED + VALIDATED +
  COMMITTED + PUSHED by continuation worker 65f39dff claim 2,
  commit b8925a9, D105). Grammar v1.4 `destroy = 1` (an
  EQUIVALENCE seam — the original loads the same .BDG/.POS/.TRT
  natively; dbx-plan records it in _e_staging, never
  fabricates, with the pre-S5 empty-mirror divergence noted) +
  S4.scen (trap/artillery-cascade/survivor legs, 49 records,
  chain 2ddd15ea50c8a14d byte-identical double run) + the
  canonical destroy rows on T1/T3 (23-B objects keyed by .POS
  slot, 20-B TRT, shared grid spans, COMPACT-ACTIVE mirror with
  the nonzero-tile filter on both channels, FULL-bank
  debris/splash = E-only coverage rows) + the differ O1
  normalizers + differ_gate S4 cross PASS-WITH-NOTES (4 E-only
  rows, zero field gaps) + the MissionShell destroy-score fold.
  Continuation fixes to the WIP: the trt fabricator slice
  overrun, the count-cell stride guards, the mirror
  compact-tail parser layout cross, clippy lints, the dbx-plan
  destroy leg + its record-never-fabricate test. S0/S1/S2/S3
  chains re-asserted BYTE-IDENTICAL; workspace 617 green,
  fmt+clippy clean, registry_anchors green, manifest clean both
  sides. Queued: W12-S5-prep (item 3) + the dbx-plan T2/T3
  tier unit (item 4).
- 2026-08-22: P4.2/W12-S4-prep THE E-SIDE IMPACT-APPLICATION +
  DESTROY-RESOLVER PRODUCER unit COMPLETE (RE 7j.38/7j.39 by
  worker 460d294e claim 2, commits dcc8865 + acf09ff; the
  engine+tests built by worker d57a4dec claim 2 and left
  uncommitted by session death — ADOPTED + INDEPENDENTLY
  RE-VALIDATED + COMMITTED + PUSHED by continuation worker
  3e93a4b1 claim 2, commit ad26952, D104). bedlam-core::destroy:
  the mission-load STAGING (the .BDG EOF-exact parser pinned
  against all 37 shipped files / the .POS instances with the
  footprint+hp re-stamp / the .TRT turrets / the 0x7d2/0x7d3
  hazard stamper / the empty-staged TOT-mirror+seen banks, all
  D51 host seams), the two RESOLVERS (FUN_0041a894 objects +
  the platform 0x7d4 entry + FUN_0041bc1c structures with the
  rubble stamp), the destroy TAIL (objective notify → GER gate
  → the +0x46/+0x4A template RESTORE → the five-effect loop
  (draw table 8/8/8/8/8/0/0/72/9 same-seed-asserted) → the
  score award → the four perimeter CHAIN walks), the 20-kind
  debris stager, the splash stager + water-z probe, the script
  blast, the tile-0x62 trap lane, both disbursers (the 7j.14
  0xF-persist / 0x65-clear corrections), the §7j.39/2
  weapon-tick impact call orders wired (0x29 REVERSED
  faithful). NONE enters state_hash (the W6 split). D104
  differ contract: the armor/fade rows canonicalize BOTH
  channels to the last-nonzero prefix. VERIFIED:
  destroy_gate 16/16, weapon_fire_gate 28/28, S0/S1/S2 chains
  BYTE-IDENTICAL (the no-inject invariant), S3 re-pinned ONCE
  e29f76f5585401e1 before any O1 S3 capture; workspace green,
  fmt+clippy clean, registry_anchors green, manifest clean
  both sides. Queued: the W12-S4 S4.scen + canonical-chain
  unit (item 3).
- 2026-08-22: P4.2/W12-S3 THE S3.SCEN + CANONICAL-CHAIN unit
  COMPLETE (engine+tests by worker 0bef7bae claim 2, commits
  774eed4 + ae8be6b + a928ad8 + af5c2b8 — the EXD-twin Ghidra
  hop, the COMMAND payload +7/+9/+0xB off-by-one fix, grammar
  v1.3 `loadout`, the canonical bank rows + S3.scen + chain pin
  49193732e6dbc546; the differ/registry leg was left uncommitted
  by session death and ADOPTED + VALIDATED + COMPLETED by
  continuation worker 16ebe0c4 claim 2, commits 51fa937 (both-
  channel bank normalizers + EXD aliases 0x980d4/0x10e174 +
  registry_anchors exception + differ_gate S3 row) + f211684
  (dbx-plan records the loadout seam in _e_staging, never
  fabricates; extract_injects tolerates step-less plans) + d407ca6
  (D103 + DESIGN §7 v1.3 note + §10-W12 S3-LANDED)). S3 covers
  every inline-spawn class over 133 records (bullets/shell/0x17/
  homing = documented E-gaps surfaced as differ coverage); S3
  differ = cross PASS-WITH-NOTES (2 E-only rows, zero gaps, zero
  T2 diffs); S0/S1/S2 chains BYTE-IDENTICAL (no-inject invariant
  re-asserted); workspace tests green, fmt+clippy clean,
  registry_anchors green, manifest clean both sides. Queued:
  W12-S4-prep the E-side impact-application + destroy-resolver
  producer unit (item 2).
- 2026-08-22: P4.2/W12-S3-prep THE E-SIDE WEAPON-FIRE COMMAND
  PRODUCER unit COMPLETE (worker 95ab9206 claim 2, commits 5cf5078
  (RE 7j.37) + 5f2963a (engine+tests) + 642be37 (D102 docs); adopted
  + independently re-validated after the session died post-push by
  continuation worker bae2e091 claim 2). bedlam-core/weapon.rs: the
  COMMAND ring + the FUN_00409138 consumer subset (fire gates
  mask ∧ cooldown==0 ∧ ammo≠0 verified; inline spawns field-exact
  artillery/mines/grenades/rocket; family routing; auto-rearm;
  recharge pass), the 400×0x36 weapon + 50×0x22 projectile banks
  (out of state_hash — the S3 T2 watch surface), the per-type ticks
  (bullets net-TWO committed steps + free only at tick>99 (corrects
  7j.22); artillery burst window BY TYPE; ballistic incl. 0x17
  3-clone split; rocket launch delay; homing 0x29 steering exact),
  enemy_tick, the FUN_00419aff damage table with the d=2 flat
  override; Robot weapons[7]+mask host-seamed; W5 `command` step
  CONSUMED in canonical.rs; advance_frame = the MissionShell order.
  RE-VALIDATED this run: weapon_fire_gate 28/28, canonical_dump_gate
  5/5 with S0/S1/S2 chains BYTE-IDENTICAL
  (8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4),
  differ_gate, registry_anchors, workspace 100%, fmt+clippy clean,
  manifest clean both sides. E-gaps documented (AI-family spawn
  internals [hypothesis-routed], mortar, impact application = S4,
  disbursers, SFX = T4, 0x22 producers, FUN_004197d4, trail ring).
  Queued: the S3.scen + canonical-chain unit (item 2).
- 2026-08-22: P4/RE THE [0x4ede1c] BIN-BANK CONTENT CONSUMERS unit
  COMPLETE (worker d6b238f4 claim 2, commit cd304c6, D101,
  docs-only; objdump-only from exw-text-objdump.txt + read-only
  corpus probes in /tmp/opencode, no Ghidra run). CLOSED with the
  grammar + census + verdict triple: (a) container grammar pinned
  instruction-exact (FUN_00401471 0x401477..0x4014c8) + corpus
  11/11 banks — u16[bank+0] = sprite COUNT → WRITE-ONLY cell
  0x46cdb8; directory entry = bank+2+4·id, sprite = entry +
  u32[entry] SELF-relative (MISSIONVIEW §4 "bank+4+id*4 bank-
  relative" gloss CORRECTED; FORMATS §18 assumed→VERIFIED);
  records u16 fmt/dy/dx/gate/rows + stream, gate==0/rows==0 →
  draws nothing, FUN_0040167a ignores gate, ALL real terrain =
  fmt 7 + exactly 9 fmt-0 scratch records per bank; (b) reader
  census complete (12 [0x4ede1c] sites): terrain loop (4) + the
  restamp drawer FUN_00440dc2 + FUN_00401010 the RADAR STAMP (the
  bank's ONLY runtime writer: 5× downsample + 2:1 iso deshear of
  the 480×480 viewport into ids u32[0x454b00+4·set]..+8) — VESTIGIAL:
  stubs gate=rows=0 forever, LNK identity ×63 ids ×7 zones, never
  drawn (A–D TOT refs render nothing; stamp runs every present);
  (c) §0b verdict: render-only presentation, NO differ watch row.
  Deliverables: RE-EXW-SIM §7j.36 + 2 new + 2 rewritten ledger
  rows + MISSIONVIEW §1/§4 + FORMATS §18 + D101. registry_anchors
  green; manifest clean both sides; PUSHED. Queued: the W12-S3-prep
  E-side weapon-fire COMMAND producer unit (item 2).
- 2026-08-22: P4/RE THE 7h.3 PICKUP TILE-WORD PRODUCER unit COMPLETE
  (worker f461ea05 claim 2, commit 187f0aa, D99, docs-only; objdump-only
  from exw-text-objdump.txt + read-only corpus probes in /tmp/opencode,
  no Ghidra run). CLOSED with the staging headline: init_tiles@00407e11
  copies EVERY nonzero TOT plane word into the 0x4796bc mirror (the DAT
  byte gates ONLY the seen flag — the §2/§7j.16 gloss corrected; the
  DAT==0 word gate is the FUN_00440a2d restamp path alone). The
  get_z_pos type-3 probe latch = FOUR writer sites {z/x/y}→
  0x4dc688/8c/90 (z / z+1 / z−2 empty-search / slope z+1), last-write-
  wins; SOLE consumer = the robots() move-toward-target clear(0x40bef2)
  →robot_move(0x40bf06)→test(0x40bf0b)→fire protocol (DAT byte := 0,
  mirror word := floor word 0x454a90+4·set, seen := 1, MP-only
  0x4dc6ac/b0/b4 stage via FUN_00425647, then FUN_0040eba0); any of
  the 9 probes of one move sub-tick collects (±0.34..0.38 tile reach,
  no standing-on). TERRAIN SET [0x4edd8c] = zone_index+1 CONFIRMED
  (path zone letter 'A'+set−1 @0x446771/79/d2; GameMain boot 1,
  campaign episode advance ++ @0x41c9e5 walking sets 1..7 = zones
  A..G, save-load restore @0x43c2b8, MP picker rows 1..10 → sets 2..6
  MP-ONLY). CORPUS VERDICT: ZONEA/M1 = 80 DAT==3 cells, ZERO in the
  set-1 pickup range (0x81..0x84/0x53D are set-2/5 shapes — inert
  under set 1); ZONEB (set 2) 601 pickup cells, ZONEF (set 6) 149,
  zones C/D/E/G none — S0/S1/S2 NEVER fire the machinery, so the
  engine seam stays host-seamed BY CORPUS FACT (D98 pattern; P4.2
  hooks on the S5 row: the pickup leg must run ZONEB/ZONEF + the
  E-side producer list). Deliverables: RE-EXW-SIM §7h.4 + the §7h
  seam note superseded + 1 rewritten + 2 new ledger rows + §9 item 4
  refresh; FORMATS-MISSION §2 staging correction + §4 type-3
  substrate note; DESIGN-DIFFHARNESS S5 row; D99. registry_anchors
  green; manifest clean before AND after the corpus probes; PUSHED.
  Queued: the MISSIONVIEW §8 water-flag/anim remainder (item 2) + the
  [0x4ede1c] BIN-bank content consumers (item 3).
- 2026-08-22: P4/RE THE MISSIONVIEW §8 TYPE-DB TAIL PRODUCERS unit
  COMPLETE (worker a42c6027 claim 2, commit 3530df5, D98,
  docs-only; objdump-only from ghidra-project/exw-text-objdump.txt,
  no Ghidra run, no corpus read). CLOSED with the door-machine
  headline: +0x19 = the door/scenery TARGET-TAG byte, +0x1A =
  {bit7 phase, low7 frame counter} — the 15-frame sliding-door
  animator FUN_00423081 (sole caller MissionShell epilogue
  0x44808f; DAT door-frame bytes 0x40+2n even / 0x5F−2n odd at
  the walk-down level; nibble wrap → finish pairs
  FUN_004236c6+00423740 (close: DAT seen 1/0 + z-stack PUSH-UP,
  plane0 cleared when S+E neighbors are door tiles) /
  FUN_00423650+004235fb (open: DAT 0 + z-stack DROP); counter
  stops at low7==+0x19; state≥3 auto doors XOR bit7 + 0x14 pause
  forever, states 1/2 script-toggled via FUN_004223b8's 86
  FUN_00433980 callers; renderer slide bias −nibble·0x500
  0x406c5c; [0x4eaae8] = a 9th z-plane offset). RECT GRAMMAR
  RESOLVED {+0 state,+2 x0,+4 y0,+6 w,+8 h,+0xA variant,+0xC cd,
  +0xE sfx} — the 7j.12 "word@+2" qualifier and the 7j.21 w/y/h
  permutation corrected. Reader anchors: scorch→damage 0x40bc60
  (FUN_004100b7(robot,0x14)), fire-anchor 0x4110cb, renderer
  adjacency 0x406bd6/0x406bf9, neighbor test 0x4237c5/da, the
  second +0x1B/+0x1C stamp/clear walks (0x448b4f/61,
  0x448d65/6c); +0x1D zero traffic CONFIRMED (71-site census).
  Deliverables: RE-EXW-SIM §7j.34 + 2 rewritten + 1 new ledger
  row + MISSIONVIEW §2/§8.1 + FORMATS §2 + D98. registry_anchors
  green, manifest clean, PUSHED. Queued: the 7h.3 pickup
  tile-word producer unit (item 3 → next slot 2).
- 2026-08-22: P4/FORMATS THE .BLD RECORD WALK unit COMPLETE
  (worker fc88ecf3 claim 2, commit 6897326, D97, docs-only;
  objdump-only from exw-text-objdump.txt + read-only corpus
  probes in /tmp/opencode, no Ghidra run). CLOSED with a
  negative-result headline: "BLD" (case-insensitive) appears
  in ZERO shipped executables (EXW/EXD/EXE/cd-root/DIRECTX×3)
  — there is NO .BLD loader; "SAVED.BDL" @0x4597d6 = the
  savegame. .BLD is the EDITOR-SOURCE format that compiles to
  .BDG (record j ≡ BDG non-empty record j; BLD 197 = 282 − 85
  EMPTY rows on ZONEA/M1). Grammar VERIFIED: length = 137 +
  64·W·H + variable tail (subsumes the 201+64k deltas); four
  template-bank slots of 16·W·H B each whose values ARE the
  BDG banks (+0x3E/+0x42/+0x46/+0x4A); head u32s = H/hp/
  chain/type; name@+0x60; arrays cap at 16 u16s; NOT
  self-delimiting (no terminator/count — needs the sibling
  BDG's W,H); zero fill after the last record; 7 286/7 907
  records byte-validated (ZONEA/C/D/E + ZONEF M2/M4/M7 fully;
  ZONEB/G + ZONEF M6 desync at variable-tail records —
  bounded, documented). RUNTIME CENSUS: FUN_0041dc5a = the
  mission family loader (.TOT/.DAT/.CGR/.BIN/.MIN/.LNG-or-.
  LNK by [0x4eba1c], .PAD; 8-entry 5-B-stride tag table
  0x4587d9..0x4587fc) + path builder FUN_0044670c
  ("EDITOR\ZONE{n}\MISSION{m}"); editor-only set = .BLD/.CTG/
  .COL/.MAP/.PTH/.TXT (FORMATS §0.2 — .CTG never loaded!).
  BONUS: zone D DOES ship mission-level BLDs (§0 row fixed);
  zone-level BLDs byte-shared A≡F and B≡G. Deliverables:
  RE-EXW-SIM §7j.33 + 2 ledger rows + FORMATS §0/§0.2/§16/§17/
  §19/§20/§21 + D97. registry_anchors green, manifest clean
  both sides, PUSHED. Queued: the MISSIONVIEW §8 type-DB tail
  producers unit (item 3 → next slot 2).
- 2026-08-22: P4/RE THE .BDG TEMPLATE-BANK READER unit COMPLETE
  (worker ce347a0e claim 2, commit 4210f55, D96, docs-only;
  objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra
  run; read-only corpus probes in /tmp/opencode). CLOSED with a
  negative-result headline: +0x3E/+0x42 have ZERO readers — they
  are the editor's CURRENT-state pair (bank1 ≡ shipped TOT word,
  bank3 ≡ shipped DAT byte at every .POS footprint, 434/435
  ZONEA/M1 cells; the 1 miss = a genuine footprint overlap,
  last-.POS-slot-wins), already baked into the shipped mission
  files — the runtime spawn-stamp hypothesis is RETIRED. The
  loader DISK ORDER is interleaved vs the slot layout
  (+0x3E,+0x46,+0x42,+0x4A; 0x41a71d..0x41a782); the destroy
  restore consumes ONLY the UNDER pair (+0x46 → TOT-mirror plane
  words +2·z; +0x4A → seen=(word==0) @+0x10+z + DAT volume low
  byte; linear (z'·H+i)·W+j; z ∈ [z0, min(z0+D,8))) —
  re-verified instruction-exact. BONUS DECODES: (a) the 0x1E-B
  TOT-mirror tile-record grammar unified (plane words +0x00..0x0F,
  seen +0x10..0x17, +0x18 scorch, +0x19 variant<<4, +0x1A door
  bit7, +0x1B/+0x1C = the OBJECT-HEIGHT pair (z0, z0+D) — the
  MISSIONVIEW §8.1 unknown producer found = the objective family;
  +0x1D zero traffic); (b) FUN_0044889a/FUN_00448b80 = the
  OBJECTIVE-BUILDING family (zone-7 gate [0x4edd8c]==7, counter
  [0x46cce0] over instance types 0x44..0x47, heights stamped/
  cleared, at zero SFX FUN_004239ef(0x28,3)/(0x29,3) +
  extraction-arm cells 0x46cd00/0x46ccfc/0x46ccc4); (c) .POS word
  2 = the BASE Z LEVEL (FORMATS §12 kind-gloss corrected);
  (d) FUN_0041bc1c TRT death stamp (per-zone rubble word table
  0x454a04 + k15 debris + splash). Deliverables: RE-EXW-SIM §7j.32
  + 3 rewritten + 4 new ledger rows + FORMATS §2/§12/§16 +
  MISSIONVIEW §2 update + D96. registry_anchors green, manifest
  clean both sides, PUSHED. Queued: the .BLD record walk (item 3).
- 2026-08-22: P4/RE THE HOT-RECT RECORD unit COMPLETE (worker
  aa62f5ed claim 2, commit 5abeaad, D95, docs-only; objdump-only
  from ghidra-project/exw-text-objdump.txt, no Ghidra run).
  HYPOTHESIS CONFIRMED (one refinement): the 0x4787c4/0x47879c
  family is ONE 0x20-stride record array — base 0x4787bc (rec 0),
  count [0x46ccd8], cap 0x77, per-frame reset @0x403a9a; 0x47879c
  = base−0x20 = the dispatcher's 1-based view. Grammar: +0/+4
  world corner, +8/+0xC hit-box ORIGIN (NOT center — the picker
  adds w/2,h/2), +0x10 z, +0x14 w, +0x18 h, +0x1C type. Full
  traffic census: 7 writer sites ALL in renderer FUN_00403938 (w1
  0x403c87 robots gated [0x4edb88]==2 ∧ ≠local-player — MP-ONLY,
  type (idx+1)|0x1000, z+0x21, corner tile+0xB; w2-w7 critter
  .NME draw paths, type idx+1 plain, z ∈ {raw,+0x20,+0x10,>>8}, w
  ∈ {0x3C,0x40}) + picker FUN_00419943 (octile priority
  FUN_0041ebf8, early-out <4, returns i+1; ground fallback = iso
  (mx−0xF0)·k/0x1E0 + TRT active-scan → 0x2000|(idx+1)) +
  dispatcher FUN_00410644 (MissionShell @0x448021; type cell
  [0x46cc00] NEW pin; bit12 robot corner+z; bit13 TRT via the
  −0xC-bias base 0x4cccec ×0x20+0x10 — the 7j.28 "critter
  0x4cccec" gloss CORRECTED to TRT; else critter z =
  FUN_004128ec>>8+0x15; tail [0x4ddb20]|=2 order latch NEW pin +
  mouse consume). SEAM CONSEQUENCES: SP click-orders can NEVER be
  robot-targeted (E seam must not fabricate them — S2's ground
  seam validated); order-target units are per-class formulas vs
  the D82 cells 0x4dd484/88/8c. Deliverables: RE-EXW-SIM §7j.31 +
  3 ledger rows (supersedes the §7j.16 skeleton rows) + D95.
  registry_anchors green, manifest clean both sides, PUSHED.
  Queued: the .BDG template-bank reader unit (item 2) + the .BLD
  record walk (item 3).
- 2026-08-22: P4/RE THE SFX BANK-NAME WALK unit COMPLETE (worker
  7972b334 claim 2, commit a0f291c, D94, docs-only; objdump-only
  from exw-text-objdump.txt + DGROUP re-read from the binary, no
  Ghidra run; adopted + validated the interrupted 09:55 WIP dump
  exw-banknames.txt with an independent extractor — 17 widened-
  window rows verified, 1 artifact row rejected). DELIVERABLE: the
  COMPLETE bank→name map — 202 durable assignments, ZERO unnamed
  durable cells (RE-EXW-SIM §7j.30 + 2 ledger rows): mission set
  FUN_0043a1d3 = 27 registers incl. the MIDIGUN-duplicate quirk at
  0x4edf70; screen sets share cells (MENU1/2 + BEEP1/4/5/7 +
  TEXTBOX1 + DOOROPEN/CLSE); mission-extra BEAMIN/THROW/BIOFIRE/
  PEXPLODE/CACODETH/SQUAWK/GRUNT1..3; speech = 53 8-B {A,B}
  records at 0x4ee014 (95 files, pair slot-order FLIP at SPCH16,
  11 empty +4 slots, playback bypasses the steal path via
  0x44c8c4); GFX families + language G-variant gates (index
  0x4eba1c==1, edition [0x4edd8c]>4 → GRILLA) + per-ROLE palette
  cells (0x4edbf8 ×6 names). SFX CELLS HOLD VOICE-BASE HANDLES:
  FUN_0043a36e/39c = 1-/4-voice registers (staging cell 0x46af0c),
  FUN_0043a48e = the play/steal function (listener 0x4edde4/8,
  priority/age arrays 0x4ee1c2/0x4ee2e2, default vol 0x7f/pan
  0x8000 at −1,−1). All prior bank pins re-confirmed cell-exact.
  The unnamed 0x46afXX cells characterized (0x46af4c = the DAT
  volume pointer; 0x46af58 = 0x2710-B arena; 0x46af0c = the load
  staging cell; 0x46af5c = struct base). Manifest clean both
  sides. Queued: the hot-rect record unit (item 2).
- 2026-08-22: P4/FORMATS THE .MOFO LOADER unit COMPLETE as a
  NEGATIVE RESULT (worker 0a08a5e1 claim 2, commit 03e8c3b, D93,
  docs-only; objdump-only from the existing exw-text-objdump.txt, no
  Ghidra run). THE .MOFO LOADER DOES NOT EXIST: 0x457a4c "MOFO\0"
  is the dead tail of the fatal string "Buggered direction in
  MOFO" @0x457a3c (DGROUP bytes re-read from the binary; ZERO code
  refs — full .text immediate scan + the empty Ghidra XREF block);
  no ".MOFO" byte sequence in EXW or EXD; no *.MOFO file in the
  corpus (manifest verified both sides). The extension-tag family
  is CLOSED at .NME/.TRT/.POS/.BDG @0x457a57..0x457a6d (one ref
  each → the four CLOSED loaders 7j.15/7j.18/7j.25; the 7j.15
  ".MOFO" gloss corrected). BONUS PIN: the string's sole consumer
  FUN_00415490 = the mode-9 SEEK per-step target-acquisition
  dispatcher — dword@+0x10 dual-purpose (wander heading 0..255 /
  mode-9 direction 0..3 seeded RandA()&3 at the 0xB-dormant wake),
  `cmp 3; ja fatal` → the standard fatal idiom (fade-cancel
  0x420100 + print 0x44d2ac + FATAL EXIT 0x44d2da); 4-way tables
  0x415480 (acquisition: tight −4..+0xF ahead on the walk axis,
  |Δ|<0x18 cross + z; c3 robot-y-RAW quirk) + 0x412ef8 (steppers
  y−1/x+1/y+1/x−1); hit → target w@+0x7A + mode := 2 + anim := 0.
  Deliverables: RE-EXW-SIM §7j.29 + 2 ledger rows + FORMATS §0.1.
  Queued: the SFX bank-name walk (item 2).
- 2026-08-22: P4.2/W9 GATES/CI WIRING unit COMPLETE (worker cd3ebd73
  claim 2, commit 5026afc, D92). (a) CI LEG: the named
  `diffharness` job in .github/workflows/ci.yml (`cargo test -p
  diffharness` + `cargo test -p bedlam-game --test
  canonical_dump_gate --test differ_gate`) — DESIGN §9 DH-G3 section
  landed with the CI-proves-vs-live-session split (CI: compile +
  skip-cleanly + the corpus-free tests; the pinned chains
  8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 run on
  corpus-present machines; original-side NEVER in CI). (b) THE SWEEP
  was empirical, not grep-only: fresh corpus-free git clone (faithful
  CI-checkout sim; game-data never committed) + `cargo test
  --workspace --no-fail-fast` → exactly ONE non-skipping corpus
  dependency: menu_gate, 3 of 5 tests panicking via corpus_host()'s
  expect on the absent corpus. (c) FIX: menu_gate gains
  corpus_present() (LANGUAGE.ENG marker) + the three guards in the
  file's own pattern; the expects stay as corrupt-corpus tripwires.
  Post-fix: clone 52/52 targets green, workspace 565 tests green
  (all 5 menu_gate tests executing for real with the corpus), fmt/
  clippy clean, manifest clean both sides. Recipe in DESIGN §9:
  re-run the clone test whenever a new corpus gate lands (the named
  job enforces it). Queued: the .MOFO loader unit (item 2) + the SFX
  bank-name walk (item 3).
- 2026-08-22: P4.2/W8-s2 THE S2 ORDER SCENARIO unit COMPLETE (worker
  7faaeb53 claim 2, commits a9e6964 + 786c9fb, D91). (a) STAGING
  DESIGN (docs-first): grammar v1.2 `markers = x,y,z[; ...]` header
  key — the walk seam (the click-order moves only the OTHER robots
  in the order radius; the clicked robot snaps to spread slot 0, and
  D89 pins the SP squad at 1 robot on EXW/EXD/E alike). E stages via
  the EXISTING load_mission(staged_markers) seam (no staging-rule
  change, MRK+markers ≤ 12); O1 records the seam in the plan's
  `_e_staging` field and NEVER fabricates (a dbx-plan test pins that
  no inject row touches the robot bank/count). Also landed the
  predecessor 3595c744's uncommitted D90 journal. (b) S2.scen:
  markers 18,73,1 (the mission_corpus_gate walker) + order 21 73 1 +
  frames 16, tiers T0,T1,TS. (c) canonical_dump_gate
  corpus_s2_order_walk: 17 records, chain 809f4961b7757da4 pinned,
  double-run byte-identical; asserts the arm frame (window 0x197−1
  after the arming pump's decrement — the single-robot window-0
  clear does NOT fire at 2 alive; claims slots 0+1; clicked robot
  state-3 snapped + no target; walker state-4, present=1 target
  (22,73) Q5, stop_dist 1000000), the walk window (frames 1..6,
  monotone 18→21), the arrival clear at frame 7 (state 4→3, snapped
  ONE TILE SHORT at the (21,73) origin — the west-approach
  ARRIVE_RADIUS semantics; beacon 0 + claims all 0 on all-state-3;
  target RETAINED, present=1 persists), the move-target-words row
  form, and the steady tail. (d) differ_gate: S2 row in the loop —
  fabricated O1 carries the present=1 span both ways through the D90
  splice; cross PASS-WITH-NOTES (exactly the 2 E-only rows, ZERO
  robot field gaps), double-run PASS modulo counter/RNG, FAIL on
  money. (e) dbx-plan compiles S2: order-target 3-cell write at
  frame 1 + `_e_staging`; capture-plans/S2.json committed +
  byte-pinned. Workspace 52 suites green (565 tests), fmt+clippy
  clean, manifest clean. Queued: W9 gates/CI wiring (item 2).
- 2026-08-22: P4.2/W8-prep THE ROBOT-COUNT OVERRIDE PIN unit COMPLETE
  (worker b0656949 claim 2, commit f106cf1, D89, docs-only). ANSWERED:
  the original SP does NOT fill the 0x46cbe0 network-marker override —
  EXW FUN_0040cca0 @0x40cd8d gates it on [0x4edb88]!=0 (network
  sessions only; the EXD twin is the mode==0 branch of FUN_0001d9cd,
  instruction-for-instruction), and the title menu "New Single Player
  Game" @0x43aaa3 sets 0x4edb88=0 ∧ 0x46cbe0=1 for every local
  session. SP ZONEA banks ONE robot in EXW, EXD, and E alike —
  robot-count parity holds, robot-count diffs in SP scenarios are a
  genuine finding class, NO E-side staging seam changes. Corrections
  landed: EXW 0x46ccbc = TOTAL (EXD cap 0x11950c twin) vs 0x46cbd8 =
  PER-PLAYER (EXD 0x11958c twin) — RE-EXD-MAP §5 robot-bank row +
  RE-EXW-SIM §7c.7 fixed; future MP bank dumps must bound by the cap
  cell. Faithful quirk recorded: the SP marker write hits
  record[12]+0x2A (stale MRK-copy counter, both twins) — harmless.
  Evidence: local ghidra-project/exw-spawncount-asm.txt (verbatim
  extract from the 7j.27 objdump — no new Ghidra run; desync/
  realignment note in its header). DESIGN §10-W8 resolved; DECISIONS
  D89; RE-EXD-MAP §5d. Queued: the move-target plan-row fill (item 2).
- 2026-08-22: P4.2/W7-followup THE EXD ROBOT BACK-HALF PROBE unit
  COMPLETE (worker 03be9318 claim 2, commits 455ca41 + 206b776, D88).
  (a) RE NOTES FIRST (455ca41): two `-process BEDLAM.EXD -noanalysis`
  passes (EXDRobotBackhalf{,2}.java; dumps ghidra-project/
  exd-robot-backhalf{,2}.txt) — hop 1 program-wide census of the
  0xf6d34..0xf6ddc family (every hypothesis offset has traffic;
  `[i·0xA8+const]` + `[i·0x15·8+const]` idioms) + the FUN_0001c7dc
  per-phase-tick disasm/decompile (the phase-4/5 gate, decay family,
  stop 0xf4240 arm, arrive snap, beacon auto-order, pod-gate tail);
  hop 2 the writer family: FUN_0001ef61 = the DAMAGE APPLIER (EXW
  0040e230 twin — hit_flash +=1 first, alarm trip 100, alarm_ctr +3/
  >100/reset, shield absorb/decay, hp ceiling battery·100+5000,
  death paths := death_flag 1), FUN_0001d9cd = the SPAWN INITIALIZER
  (EXW 0040cca0 twin — kind := player type, variant := RandA&3,
  facing := 0xFFFF, probe_z 8-word seed, stat switch 0x2A/0x2B/
  0x2C×200 → charges/battery/pool, MRK pos formula, stagger, cap
  0x11950c := count 0x11958c), FUN_0001d274 = robot_move (dir_byte
  := angle, facing cardinals, anim ((a+4)&0xFF)>>3), FUN_0001e440
  probes, FUN_00020dea pad charge (armor +20 clamp 3000 bar 2500,
  pool gate), FUN_000180a1 portrait clamp-5, FUN_0005961c = the SP
  all-dead sweep (death_flag READER — closes 7g.6). §8 table
  rewritten: 23 fields + per-field provenance; coverage gaps 26 → 3
  (target trio, record-external); move-target EXTENT pinned (0x60-B
  span at 0xf75ec); §1b size fix (14,644 B = FUN_0001476d). (b) CODE
  (206b776): EXD_ROBOT_MAP/EXW_ROBOT_MAP widened to the 31-leaf pin
  (FieldKind {I32,U16,I16}, armor i16); drop_countdown rebound
  +0x2C → +0x80 (the ENGINE field's semantics — the W7 binding was
  the pod timer); differ.rs fixture = the full independent
  transcription (anti-fabrication now the target trio, 34−31=3);
  differ_gate.rs inverse fabricator places all mapped fields, S1
  coverage re-pinned 2+26 → 2+3 with the pinned chains
  8901789a88cf61fe / 1c4e7b4c9d9b0947 re-asserted GREEN. Workspace 52
  suites green, fmt+clippy clean, manifest clean before/after the
  Ghidra runs. NOTE for W8/live: EXD alarm_ctr has a phase-0 decay
  EXW 7g.1 does not document (evidence gap); EXD SP banks exactly 1
  robot on ZONEA like E. Queued: W8 the robot-count override pin
  (item 2, now half-answered) + the move-target plan-row fill (item
  3, coverage 3 → 0).
- 2026-08-22: P4.2/W7 THE DIFFER unit COMPLETE (worker
  c594df62 claim 2, commits a9d741f + 04d1d27 + 0dfdb0c, D87 + the
  RE-EXD-MAP sec 8 basis). (a) RE NOTES FIRST (a9d741f): the EXD
  robot-record normalizer field map provenance-tagged — x@+0/y@+4
  [seed#1+anchor writer], z@+0x08 PINNED NEW (the per-player writer's
  d@(0xf6d3c+i)+0x20), state@+0x0C, drop@+0x2C, stop@+0x74, hp@+0x78,
  alive@+0x7C; the 26 unmapped canonical fields = coverage findings
  (never zero-filled); the seed-#1 EXW-front conflict recorded OPEN
  (O2 uses the SIM sec 3 table; W11 arbitrates); the non-robot O1 row
  forms (beacon u16-span widen, static-map-wh 0x2c span, typedb
  len-0==all-zero-grid equivalence, rng u32->u64). (b) THE DIFFER
  (04d1d27): differ.rs — channel normalizers (E canonical parse / O1
  per sec 8 / O2 per EXW sec 3+7f/7g), MODES DoubleRun (the DH-G1
  verdict: identical modulo frame-counter T2 + rng T3, draw-COUNT
  checks still apply) + CrossChannel (per-field classes, O2
  ARBITRATION: O2 sides with O1 -> engine-bug, with E ->
  original-divergence, none -> provisional), the `coverage` bucket
  (metered, never silent, notes-not-fails), constant-shift alignment
  (<=8, T1-timing note), event-timing table, T3 draw-count gate,
  report_text + manifest_json; bin/dbx-diff CLI (mode auto from
  channels, --tiebreak/--t2-quantum/--report/--manifest, FAIL exit
  code). (c) VERIFIED: 15 differ tests (hand-built EXD fixtures =
  independent sec-8 transcription; the W6 canonical literal as the
  shared-field contract; arbitration both ways; 26-gap math; shift;
  determinism); corpus-gated differ_gate.rs — S0/S1 run_canonical
  (pinned chains 8901789a88cf61fe / 1c4e7b4c9d9b0947 re-asserted) x
  the INVERSE normalizer -> cross PASS-WITH-NOTES (S0: 0 coverage;
  S1: 2+26 + the one T2 counter note, zero engine-bug/structural),
  double-run PASS modulo counter/RNG, FAIL on money perturbation; CLI
  smoke-tested on the real S0 E dump (PASS, chain pin shown). 52
  diffharness tests total; workspace test/fmt/clippy green; manifest
  clean. PUSHED 0dfdb0c. Queued: the EXD robot back-half probe
  (item 2) + the W8 robot-count override pin (item 3).
- 2026-08-22: P4.2/W5-pad THE CAPGEN PAD OP unit COMPLETE (worker
  85dedea3 claim 2, commits fb92286 + b5d1920, D86). (a) capgen
  `{op:"pad"}` inject form: reads the 8-B .PAD slot record
  {u16 active@+0, x@+2, y@+4, z@+6} from the pad bank at the
  capture-frame stop (MEMDUMPBIN through the bank's own SEG form +
  slot*8), FAILS LOUD unless active==1 and x!=0xFFFF (a slot the
  staged mission never loaded is a capture error naming the slot),
  then writes {x,y,z} as i32-LE x3 to the order-target triple (EXD
  0x10e0a4/a8/ac; tile coords = the shared-grammar contract). The op
  writes only the ORDER — the robot's arrival arms extraction
  in-game. (b) dbx-plan un-gate: `pad <slot>` compiles to the op row
  (bank from static-pad-slots — a READ anchor with its OWN gap error;
  targets from order-target; slot 0..998 re-checked). (c) The §7j.20
  extraction-pad census committed as S6 authoring data in DESIGN §7
  (zone 1 {8,0x10,0x12,0x18}, zone 2 {4,5,7,0xE,0x11}, zone 3
  {0,1,6,0xF,0x15}, zone 4 {0,2,0x10,0x15,0x16}, zone 5 {8,9 ×2,
  0x3D} + the shared slot-6 tail). (d) VERIFIED: `dbgprobe pad`
  headless GREEN both legs (positive: seeded slot 2 = the real
  ZONEA/MISSION1.PAD record 0 (5,61,0) → triple 05000000 3d000000
  00000000 + injected flag; negative: inactive slot 3 aborts, no
  transcript — NB the negative run log lives OUTSIDE the capgen
  workdir, capgen purges stale *.log there); gate/inject/flow/walk
  regression-GREEN; new dbx-plan tests (op-row emission vs the REAL
  registry + the READ-anchor gap refusal); S0/S1/S0W byte-pinned
  plans unchanged; scratch CLI end-to-end compile (pad 8 → frame-6
  op row); workspace test/fmt/clippy green; manifest clean. The E
  side still rejects pad steps naming the S6 seam (W12). Queued: W7
  the differ (item 2).
- 2026-08-22: P4.2/W6 THE ENGINE DUMP EMITTER unit COMPLETE (design
  83f04b9 by worker 1f758667 claim 2 — interrupted mid-implementation;
  adopted + completed by worker 36f752cd claim 2, commits 54d781a +
  docs, D85 + completion addendum). (a) parity_harness --canonical:
  drives GameHost over the SHARED v1.1 scenario grammar (the D82
  seam) and stitches channel-E W3 dumps through the SAME
  runner::stitch + encode_dump path as O1 captures; T0/T1/TS field
  maps in examples/parity_harness/canonical.rs per DESIGN §6a (every
  unmapped row an explicit E-gap); 3 read-only accessors added, no
  engine behavior changed, diffharness a bedlam-game DEV-dep only.
  (b) WALK-PHASE FIX over the WIP: walk accepts ONLY boot steps (the
  blanket empty-walk rejection had made the difficulty seed
  unreachable); DESIGN §6a amended. (c) VERIFICATION
  (tests/canonical_dump_gate.rs): hand-encoded §6a grammar fixture
  (98-byte robot-bank record literal, frame digest pinned
  b359f7d282db7cb8); synthetic MissionSim run (chain
  ea0bc53dc95ff0b2, double-stitch byte-identical, surviving
  2-robot order window 0x196); corpus-gated S0/S1 (3/401 records,
  chains 8901789a88cf61fe / 1c4e7b4c9d9b0947, byte-identical double
  runs); seam gates (boot difficulty=2 → money 3000, command/pad
  rejections naming the seams, P 0x19 banned, order arm proven by
  state-3 + tile snap). FINDINGS: static-map-wh = TOT-header map
  size 25×75 (30004/15004 are FILE bytes); E stages no
  network-marker override → ZONEA single-robot squad, armer's
  window-0 case clears the order on the arming tick (W8 must pin
  0x46cbe0 override parity). Workspace green (49 suites), fmt+clippy
  clean, manifest clean. PUSHED 54d781a. Queued: W5-pad (item 2),
  W7 the differ (item 3).
- 2026-08-22: P4.2/W5-walk THE SCRIPTED-MENU-WALK DRIVER unit COMPLETE
  (worker 845abdc5 claim 2, commits 59ec9a5 + b67dcaa + 33b2c17, D84).
  (a) DESIGN FIRST (59ec9a5, RUNTIME.md "W5 walk driver"): the stop
  model — the BPLM boot trap on the frame-counter cell 0x1195f0
  doubles as the walk driver, one stop per counter-writing screen
  frame; SMV writes at stop i become screen frame i+1's input;
  keystore re-arm per input (AnyKeyWait twin consumes on read);
  anchor BP CS:0005A6EB arms only AT THE LAST WALK STOP (BPDEL * drops
  the trap — no stop-type ambiguity). (b) COMPILER (b67dcaa):
  walk-phase keystore steps -> stop-indexed plan rows ("walk" key;
  Advance consumes stops; runaway 1M guard; order/pad/command refused
  as "not menu-walk steps"); registry-derived walk_watches calibration
  trio (walk-mode 0x1075d8 / walk-zone 0x107500 / walk-mission
  0x119610); resolve_at=anchor emitted for ALL plans — FIXES the
  latent D81 gap (loader statics are mission-load values; the
  arm-stop read was pre-mission garbage feeding len exprs); S0/S1
  regenerated, S0W.scen + capture-plans/S0W.json committed (STRUCTURAL
  DRAFT schedule — stop indices calibrate live via the per-stop
  transcript comments; then pure data). (c) CAPGEN plan v3: the walk
  loop (write-then-dump per stop, arm-at-walk-end), walk_watches
  transcript comments, resolve_at=anchor, boot_writes at the accept
  stop (identical for walk-less plans). (d) VERIFIED headless:
  `dbgprobe walk` GREEN (stop indexing incl. a pure-skip stop,
  calibration notes, arm-at-walk-end, anchor resolve feeding expr
  lens); gate/flow/inject regression-GREEN; 52 diffharness tests (4
  new), fmt+clippy clean, manifest clean. PAD op deliberately its own
  unit (queued item 3). Queued: item 2 = W6 the engine dump emitter.
- 2026-08-22: P4.2/W5-followup THE EXD INPUT-TWIN CENSUS unit COMPLETE
  (worker ef11271c claim 2, commits 79362a9 + 110718d, D83). Four
  Ghidra probe passes (-process BEDLAM.EXD -noanalysis): (a) KEYSTORE
  0x894d4 — AnyKeyWait twin FUN_00030792 (scan 1..0xFE skip 0x2a/0x36,
  consume-on-read) + the DOS KeySink = INT-9 hook @0x303f5 (make/break
  keystore[AL&0x7f]:=1/0 + held-counter 0x107534 + the ARROW-REMAP
  OR-0x80 twin) + installer/memset FUN_0003064d; ScanToChar twin
  0x307c1 (shifts 0x894fe/0x8950a, tables 0x8077a/0x8097a). (b)
  COMMAND RING 0x9255c stride 0x80 + COUNT 0x119588 — builder
  FUN_0005b066 / consumer FUN_00019ee9 (EXW 00449c94/00409138 twins;
  record byte@+0 marker/id@+1/spot@+3/flags@+5/xyz@+7/+9/+0xB EXACT;
  auto-arm 1000000; weapon-slot dispatch w2/3/4→FUN_0001c3fb,
  w6/7/8→FUN_0001bd8f, artillery→projectile bank 0x980d4×0x36
  field-exact = T2-ready bonus). (c) ORDER TARGET 0x10e0a4/a8/ac +
  order-active 0x10e140 — consumer bit1 writes + click-order twin
  FUN_00021112 (pick twin FUN_0002a271, iso (p−0xF0)·k/0x1E0 EXACT);
  EXD MissionShell trio position EXACT. (d) DIFFICULTY 0x119558 — the
  172/236/300 formula + respawn table twin 0x81050 in FUN_00023967;
  44 refs, writers FUN_0002c6e3. Registry: 6 rows filled (TI aliased
  where pinned; emptiness rule → T2-T4 only); dbx-plan order-target
  form + REAL-registry step tests; S0/S1 regenerated (19+10 / 33+24).
  Scratch-verified: keystore/order/command steps compile end-to-end
  (incl. remapped arrow CS:0008959F). Divergence seeds 6-7 (attack-
  break frame-masks vs RandA; EXD staging cells). 49 diffharness
  tests; workspace test/fmt/clippy green; manifest clean. PUSHED
  110718d. Queued: item 2 = the scripted-menu-walk driver (unblocked).
- 2026-08-22: P4.2/W5 THE INJECTOR unit COMPLETE (worker 683a65d6
  claim 2, commits c443207 + fa31828 + 5e882cd + 28ef5e7, D82).
  (a) grammar v1.1: keystore/order/pad/command/boot steps, one frame
  boundary per line; until-anchor splits walk/mission phases; boot is
  walk-phase only; command payloads raw hex <=0x80. (b) capgen SMV
  emitter: boot_writes at the arm stop + frame-keyed inject rows
  applied BEFORE the watch dumps (`frame N 1` injected flags in
  DBXCAP = injection_applied in the W3 dump); the command-ring append
  OP (count u32 read via the plan's own SEG:OFF form, payload
  zero-extended to the stride at base+count*stride, count bump);
  addr_to_linear: CS: flat identity (bounded to image top), numeric
  segs seg<<4; byte-only tokens. `dbgprobe inject` GREEN headless
  (boot beefcafe at 0000:0500 in the anchor frame, marker re-writes
  read back same-frame, command append count 0->1 + zero-extended
  0x10-stride record, all frames flagged; gate + flow
  regression-green). (c) dbx-plan T1: Form::CountExpr + count-cell
  resolve rows (robot 0x11958c / TRT 0x11949c / object *(0x119584)
  count 0x119554) + map-w/h grid exprs; S1.scen compiles ->
  capture-plans/S1.json committed + byte-pinned; S0.json verified
  byte-identical; anti-fabrication: selection-triple dumps only the 4
  verified alias bytes, beacon-family its 5 u16 cells (10 B).
  (d) step compiler: boot_writes/inject plan rows with HARD alias
  gates (keystore/order-target/command-ring/difficulty are registry
  gaps -> scenarios carrying them fail naming the seam; emission paths
  proven against a fabricated-address registry in tests only); two TI
  registry rows formalize the command seam (0x4dd4a0/0x46cbe0). RE
  FACT: FUN_0002ec12 = the P-latch spin only, NOT the EXD keystore
  (exd-input-probe.txt; RE-EXD-MAP W5 note) — the input-twin census is
  the queued follow-up. 49 diffharness tests; workspace test/fmt/
  clippy green; manifest clean. PUSHED 28ef5e7. Queued: item 2 =
  the EXD input-twin census.
- 2026-08-22: P4.2/DH-G0-live the UNATTENDED PREP unit COMPLETE (worker
  fa49e9cf claim 1, commits f659db5 + d5550a3 + ee2f0d4, D81). The S0
  live-capture machinery landed + headless-verified; the queue's
  interactive item is now a turnkey checklist (RUNTIME.md "S0 LIVE
  SESSION CHECKLIST"). RE FACTS (source-pinned at e522642, RUNTIME.md
  "S0 live channel mechanics"): GetHexValue resolves REGISTER NAMES in
  the default MEMDUMPBIN/BP parse path → `CS:001195F0` addressing with
  NO numeric selector (the BP ack echoes it — the per-run pin; the
  INT3-at-entry proof step is SUPERSEDED); BP locations resolve
  EAGERLY at arm time (pre-boot arming mis-resolves) vs BPLM LAZY
  per-instruction (the boot trap on the frame-counter cell 0x1195f0);
  SELINFO rides the logfile (the flat-CS guard, loader-stub stops
  retry); debuggerrun=watch would free-run (staged conf flips to
  debugger — channel-mode only, sim pins untouched); NO counter reset
  exists (14 INC sites incl. menu screens) → the live DH-G1 verdict is
  identical-chains-MODULO frame-counter/RNG cells (T2/T3, DESIGN §6);
  SDL dummy video has no keyboard (live plans unset SDL_VIDEODRIVER).
  CODE: capgen plan v2 (boot_commands/boot-trap retry loop/arm_commands
  + selector-pin capture/resolve cells + ast-whitelisted arithmetic
  addr-len exprs/anchor-vs-per-frame watch split/plan env overrides/
  plan-level frames+time_limit; fixed the plan-named logfile being
  resolved against the wrong CWD) — `dbgprobe flow` gate GREEN headless
  (BPLM 46C trap → arm → resolve com1=0x3f8 → expr rows carrying REAL
  IVT f000:ca60 + BDA COM1/COM2/LPT1 bytes; legacy `dbgprobe gate`
  regression-green; both unattended-safe, no game). dbx-plan
  (tools/diffharness Rust bin): compiles scenario tiers + watches.toml
  into the plan with EVERY address derived from registry rows
  (anti-ghost asserts on extents/cell layouts; committed artifact
  capture-plans/S0.json pinned by a byte-equality test); 9 T0 + 9 TS
  rows resolved (TOT/DAT extents = 4+16wh / 4+8wh, cross-check: ZONEA/
  M1 evaluates to 30004/15004 = the FORMATS-pinned file sizes; map-wh
  two-cell span; claim-bank pointer read), 6 TS rows DEFERRED with
  explicit unpinned-extent reasons (cgr/bin/min bank-sized, lnk
  map-sized, order row count, yline tables) + the 2 T0 gaps; S0 staged
  + plan deployed. 42 diffharness tests, fmt+clippy clean, workspace
  build green; manifest verified around the corpus rsync. PUSHED
  ee2f0d4. Queued: the interactive-only live session (item 1) + W5.
- 2026-08-22: P4.2/DH-G0 the O1 CAPTURE-CHANNEL RE-PIN unit COMPLETE
  (worker 4deb0081 claim 1, commits 395180b + d858728 + 1e7392f, D80).
  DECISION (a): self-built DOSBox-X at e522642 with --enable-debug=heavy
  (+ --disable-sdlnet --disable-avcodec: host gaps, harness-irrelevant),
  repo-local under runtime/, C_DEBUG+C_HEAVY_DEBUG verified in config.h,
  binary sha256 24f71092… recorded; flathub pin stays as the D29 sandbox
  baseline; GameLink + O2-ptrace rejected as primary (D80 rationale).
  CHANNEL PROVEN HEADLESS (no game): tools/runtime/dbx-capgen.py — the
  PTY driver with count-based [log]-logfile acks, a mandatory drain
  thread, and a 1.0s post-ack settle (the three empirically-bisected
  gotchas = RUNTIME.md "D80 CHANNEL GOTCHAS") — wired as
  dosbox-harness.sh `dbgprobe` (unattended-safe; GREEN: 3 frames × 3
  watches; frame1 pre-boot zeros → frame2 POST IVT f000:ca60 + BDA
  COM1/COM2/LPT1 → frame3 DOS-kernel vectors 0070:000e) and `diff
  capture` (FORCE_DIFF_RUN=1 interactive gate; consumes the staged
  capture-plan.json = the live unit's deliverable). BPINT/BPLM/
  MEMDUMPBIN/RUNWATCH/SMV all behaviorally verified (SMV linear
  write+readback pins real-mode linear==seg<<4; BPLM arm+fire on the
  next write). RUNTIME.md D80 sections + DECISIONS D80 + watch skeleton
  updated; live-unit checklist = pmode flat-selector proof (INT3@entry
  0x5fbb0 → SELINFO/LDT → present-tail BP@0x5a6eb) + DH-G1 double-run
  + cycles calibration. Manifest verified. PUSHED 1e7392f. Queued:
  DH-G0-live (interactive-gated) + W5 injector (unattended).
- 2026-08-22: P4.2/W4 the DOSBOX-X RUNNER unit COMPLETE via the ticket's
  split clause (worker d35c7066 claim 1, commits d9a3f77 + 19c3bdf, D79).
  (a) unattended-safe slice LANDED: dosbox-harness.sh `diff
  stage|run|stitch` (EXD corpus scratch runtime/harness-corpus-exd from
  game-data/BEDLAM — launch line pinned DOS4GW.EXE BEDLAM.EXE[launcher]/
  BEDLAM.EXD[LE image] via header+launcher-string evidence; per-scenario
  conf deploy; run=refused-unattended gate); scenario grammar v1 + S0/S1
  scenario files; DBXCAP v1 channel-agnostic capture transcript + the
  zero-dep dbx-stitch bin (registry/tier/O1-exd_addr anti-ghost guards,
  frame-count contract, W3 encode + JSON digest manifest with
  self-contained SHA-256, FIPS-vector-pinned); synthetic replay fixture
  test decodes the dump + pins chain vector 1685e11311ae5b21; fmt+clippy
  green workspace-wide; MANIFEST verified both sides of the corpus read;
  staged fingerprints cross-checked vs sha256sum. (b) live piece
  [BLOCKED]-on-DH-G0-channel-repin: the D79 AUDIT found the pinned
  flathub DOSBox-X has NO integrated debugger (configure.ac --enable-
  debug off; flathub builds --enable-sdl2 only; debuggerrun/-break-start
  inert across piped/PTY probes) and its Duktape startup.js is LOG-ONLY
  (enumerated: _emu.emulator/version/log + console.log -> [log] misc
  channel, needs misc=true; Buffer/CBOR with no I/O; no memory access,
  no hooks) — D29's debugger-presence claim corrected in RUNTIME.md;
  DESIGN §3/§9/§10-W4/§11 + the watch skeleton amended. Next head: the
  channel re-pin unit (queue item 1).
- 2026-08-22: P4.2/W3 the DUMP SCHEMA unit COMPLETE (worker 6f14cea1
  claim 1, commit fca6657). tools/diffharness/src/dump.rs = the DESIGN
  §3 format as code, schema_ver 1, all-LE: "BDLD" header {channel 1..4
  (O1/O2/O3/E), build_sha256[32], scenario, pins} → frames {frame_no,
  injection_applied, per-watch {id, len u32, raw}, frame_digest} →
  "BDLT" trailer {frame_count, chain}. frame_digest = FNV-1a-64 over
  the BDLD-tag-prefixed canonical frame bytes (domain separation vs
  StateHash); chain = the D28 parity_harness construction verbatim
  (incremental Fnv1a64 write_u64 per frame digest) so dump chains are
  directly comparable fingerprints. Encoders registry-driven: canonical
  watch order = watches.toml file order, unknown/duplicate ids
  rejected, frame_no strictly increasing (encode+decode), empty blobs
  legal (count-0 banks); identical state ⇒ identical digests on every
  channel (tested). decode_dump verifies every digest + count + chain +
  truncation/trailing/magic/bool/utf8 (tamper tests cover payload,
  chain, count, truncation, trailing, magic, schema, channel).
  hash.rs = zero-dep MIRROR of bedlam-core's FNV-1a-64 (dependency
  would pull thiserror), pinned to the engine's public vectors by
  tests/dump_schema.rs::engine_hash_vectors. 15 integration + 3
  in-module tests; workspace build/test/fmt/clippy green (release
  build clean). Docs: DESIGN §3 + §10-W3 LANDED, DECISIONS D78.
  PUSHED fca6657. Queued: W4 (DOSBox-X runner diff mode + DH-G0
  debugger-surface pin).
- 2026-08-22: P4.2/W2 the WATCH REGISTRY unit COMPLETE (worker 873ebd5e
  claim 1, commit 01a6847). tools/diffharness/watches.toml = the DESIGN
  §4 watch set as data, 73 rows: S0 trigger (EXW PresentEnd 0x425a03 /
  EXD instruction 0x5a6eb), T0 (11 rows, EXD aliases filled: frame
  counter 0x1195f0, RNG A/B 0x107470/74, score 0x10da28, money 0x119600,
  zone 0x107500, mission 0x119610, mode 0x1075d8, linear-m 0x119610),
  T1 (17 rows: robot bank 0xf6d34/count 0x11958c, selection triple
  0x11954c selected-idx only, per-player anchor 0x971a4, move-target
  0xf75ec/0xf761c, beacon family 0x119628-30, claims 0x119632, tile grid
  0xfe37c, platform bank 0xf93cc, type-DB mirror 0xac1e4 + derived
  +0x18/+0x19/+0x1A rows, object bank *(0x119584) indirect + count
  0x119554, TRT 0x95264 + count 0x11949c, armor-pad alias), TS (all 15
  static-after-load §5b rows incl. the volume pointer cells
  0x107454/0x107518/0x107540/0x107434/0x107538 indirect, PAD 0xf63c,
  order table 0x91ee4, player type 0x1075c0, dither 0x8ded4, EXD-only
  cursor clamp), T2/T3/T4 (exd-empty per the W1 ticket) + TI (the six
  injection-surface rows anchored on RE-EXW-INPUT). The 6 tagged gaps
  (difficulty, SFX gate, blink-cursor, order target, no-extract latch,
  selection cursor/squad) stay explicitly exd-empty. NEW zero-dep
  workspace member tools/diffharness: minimal TOML-subset registry
  parser + tests/registry_anchors.rs = the mechanical anti-ghost guard
  (every anchor string must resolve EXACTLY to a ledger row heading /
  markdown heading in its named doc; guard verified to bite on a
  fabricated anchor) + schema invariants (tier set, exd_status vs
  exd_addr consistency, T2-T4/TI exd-emptiness, indirect-pointer rules,
  gap discipline). cargo test/fmt/clippy green; manifest verified.
  PUSHED 01a6847. Queued: W3 (dump schema).
- 2026-08-22: P4.2/W1 the EXD IMPORT + EXW->EXD ADDRESS MAP unit COMPLETE
  (worker d06341cf claim 1, commits 350b53a + 10aea57 + f6e067a + 8447ba7,
  docs + 8 Ghidra probe scripts). BEDLAM.EXD imported ONCE into
  BedlamWatcom (LeLoader, x86:LE:32:default + openwatcomcpp; object1
  0x10000-0x72800 23225 fixups, object2 0x80000-0x12583e, entry 0x5fbb0;
  analysis green; manifest verified before AND after). 8 probe passes
  (-process BEDLAM.EXD -noanalysis, never re-imported; dumps
  ghidra-project/exd-probe*.txt). HEADLINES: MissionShell = FUN_000596ed
  (mission load chain in EXW order; robots x6 FUN_0001c7dc(i,i+1); enemy
  x4; P-pause spin key 0x19); S0 DUMP POINT = instruction 0x5a6eb (CALL
  PresentFlip FUN_00010670, counter [0x1195f0]++ @0x5a6f0-fd after the
  flip — EXW tail order exact); PresentFlip = FUN_00010670 (339 B = B2
  twin exact). T0/T1 + static-after-load aliases ALL mapped in
  docs/RE-EXD-MAP.md with dual anchors: RNG A/B 0x107470/0x107474, score
  0x10da28, money 0x119600, zone 0x107500, mission+linear-m 0x119610
  (TRT-hp + pod-stagger formulas byte-exact), mode 0x1075d8, frame
  counter 0x1195f0; robot bank 0xf6d34/count 0x11958c (stagger w@+0x2C
  formula exact), selected idx 0x11954c, per-player anchor 0x971a4,
  move-target arrays 0xf75ec/0xf761c, beacon family 0x119628-0x119630,
  claims 0x119632, tile grid 0xfe37c, platform bank 0xf93cc, type-DB
  mirror 0xac1e4, object bank *(0x119584) + count 0x119554, TRT array
  0x95264 + count 0x11949c, type table 0x108428 (stride 0x4E/282
  recs/banks +0x3E-4A — EXW layout byte-exact), all volume pointer
  cells + PAD 0xf63c + LNK 0x10336c + map w/h 0x1074b8/0x10748c, order
  table 0x91ee4, player type 0x1075c0, dither 0x8ded4. 5 divergence
  seeds logged (robot-front x/y shifted -4; EXD's 3 merged monoliths;
  single mission scalar for EXW's two; indirect pointer-cell banks;
  /KARMA switch). 6 explicit gaps tagged with anchor methods
  (difficulty, SFX gate, blink-cursor, order target, no-extract latch,
  selection cursor/squad). PUSHED 8447ba7. Queued: W2 (watch registry).
- 2026-08-22: P4.2 the DIFFERENTIAL-HARNESS DESIGN DOC unit COMPLETE
  (worker 4d7b9a5b claim 1, commit 7bc2c9d, D77, docs-only; no engine
  change, no Ghidra run needed). docs/DESIGN-DIFFHARNESS.md written:
  oracle topology decided (O1 = BEDLAM.EXD under pinned DOSBox-X as the
  PRIMARY scripted-differential instrument — observation never patches
  the binaries; O2 = EXW under pinned Wine as canon tiebreak with every
  RE'd address verbatim via a ptrace watcher; O3 = instrumented 8street
  as late second comparator; E = the engine). Frame model: one dump per
  MissionShell loop pass at the epilogue/present tail, aligned by
  g_frame_count@0x46ae68 <-> engine tick. The tiered watch set T0-T4
  with EVERY address anchored to its RE-EXW-SIM §8 / §7j.x ledger row
  (robots/orders/terrain T1; projectiles/critters T2; effects/debris/
  rings/objectives T3; SFX/order/debris/destroy event capture T4).
  Injection = seam writes only (g_keystore 0x4edc44/cursor/mouse,
  ORDER target 0x4dd484/88/8c + 0x46cc30/60 words, COMMAND records
  0x4dd4a0 for fire — the 7j.22 route, .PAD step-on for extraction).
  Differ compares canonical records in 5 modes per the 0b budget;
  divergence classes engine-bug/original-divergence(O2-arbitrated)/
  watch-artifact/accepted-T3. Scenario corpus S0-S8; hypothesis
  dispositions tabulated (pod stagger S1, debris 2k start-delay S1/S4,
  blink-cursor-from-spawn S1 via 0x4dc5d0, ring overlap statically
  MOOT per 7j.10 + confirming read S4, mid-flight blits = T2
  render-side OUT of state-diff scope). Dumps = asset-derived: live
  only under runtime/harness-out; git carries fingerprints only.
  Gates DH-G0..G3; build order W1-W12. DECISIONS.md D77 added.
  Manifest verified. PUSHED 7bc2c9d. Queued: W1 (EXD import +
  EXW->EXD address map, T0/T1 rows bounded).
- 2026-08-22: P4 7j.28 the PROJECTILE MID-FLIGHT DRAW family unit
  COMPLETE (worker ffec42cf claim 1, commits 9a1d205 + 27481c2,
  D76, docs-only; objdump-only from ghidra-project/
  exw-text-objdump.txt — an analyzeHeadless was running). The
  400×0x36 dispatch fully mapped (primary 0x404141 + secondaries
  0x404d27/0x404d08): shell 5 (WEAPONS 3..7, counter d@+0xE wraps
  7→3), artillery 9..0xB (8..15), mortar 0xE (frame 1 + the
  8-puff trail), damped {0xF/0x13 base 0x20, 0x17 base 0x28,
  0x1A/0x1F base 0x18} + wobble gate |vx|∨|vy|>0x40, rocket 0x24
  (SHRIKE 64-dir + ≤8 SMOKE puffs dist 0x20+0x10·i, count TTL/4),
  homing 0x29 (REAPER 64-dir + GENERAL reticle on target d@+6
  {0x1000 robot/0x2000 critter/else FUN_004128ec} + 4 puffs).
  BANKS NAMED + corpus-verified: WEAPONS/SHRIKE 64/REAPER 64/
  SMOKE 4/GENERAL 153 imgs (= [0x4eddbc]/[0x46af30]/[0x46af2c]/
  [0x46af34]/[0x4edd7c], boot string block 0x45884e..). The
  trail-ring draw consumer @0x404464 CLOSED (puffs @ 0x4e66b8+
  link·0x68+8+i·0xC, WEAPONS 0x10+(tick+i)&7, mode 0x12E, ring
  words unread). The 50×0x22 walk CLOSED (jump table 0x403908
  read from file: 0x65/0x67/0x68 single strip sprites 0x3C/0x3C/
  0x38, 0x66 NOT drawn, 0x69 the per-level BEAM column 0x34-strip
  with +0xA = top z level, +0x1A = bottom). CORRECTIONS: 0x40427a
  = loop-next (unlisted types NOT drawn mid-flight — no "generic
  draw body"); 0x17 draws damped (the 3-clone split is tick-side).
  FUN_0040798e call shape pinned (mode 0x12C/0x12D/0x12E = the
  4th stack arg; the 7j.21 "sprite 0x12E" gloss corrected).
  Render tail now FULLY decoded. Manifest verified. PUSHED
  27481c2. Queued: the P4.2 differential-harness design doc.
- 2026-08-22: P4 7j.27 the DROPSHIP RING PRODUCERS unit COMPLETE
  (worker e635cb76 claim 1, commit 2aa7cb7, D75, docs-only; dump
  ghidra-project/exw-text-objdump.txt = full .text objdump
  0x401000..0x460000, no Ghidra run — one was already running).
  The pod-descent family writer census COMPLETE: resets
  FUN_0040cca0 0x40cd3d (pods memset 0x150 every spawn) +
  MissionShell 0x447a7e/0x447a8d (dropship/exits); spawners
  FUN_0041faf0 (dropship {1,1,group 0,alt 0x200,beacon<<5}),
  FUN_0041fb4b(idx) (pods {1,1,group 0,alt 0x400,robot>>8}, from
  the w@+0x2C 0-hit in FUN_0040b9f6 + msgs 9/10/0xB), 7j.18's
  FUN_0041fa51 (exits); animator FUN_0041fbb1 3-machine per-tick
  write map decoded — +0x14 = the DROPSHIP.BIN IMG-GROUP selector
  (7j.19 "toggle" superseded): 0↔1 flicker phases 1-2, ramps
  2..5 oscillating 4↔5 in departure with x −= group·4, alt +=
  (alt>>2)+1; pod phase 2 = ONE tick = robot RELEASE (state 6,
  alive 1, payout 100·w@+0x94+5000, SFX 0x4edfe0). NEW third
  writer FUN_00412a98 0x412b60 = per-rescue exit-dwell reset
  (multi-POI elevators). Latch 0x46aed4: boot-clear GameMain
  0x41c408 (NOT per-mission) + gates the MP respawn 0x40e7a1.
  CORRECTION 7j.26: ring grid = 7 cols × 5 rows (0x23 = 35 = one
  group), not 7×7; dropship sy −= beacon z word 0x4eabb8 (always
  0, one no-op reader 0x4070c0). The 0x4c71f4 pass head-decoded =
  projectile mid-flight draw dispatch + the 0x4cc654 50×0x22
  sibling (states 0x65..0x69 → table 0x403908). 4 ledger rows
  updated + MISSIONVIEW §5e corrected. Manifest verified. PUSHED
  2aa7cb7. Queued: the projectile mid-flight draw family (7j.28).
- 2026-08-22: P4 7j.26 the MISSIONVIEW §5d DRAW TAILS unit
  COMPLETE (worker 7658328a claim 1, commits 753f0a2 + 2d124e6
  + d9bb40f, D74, docs-only; dump ghidra-project/
  exw-effectstager-asm.txt (objdump 0x41a220..0x41a4f8)). Both
  consumer passes decoded: (a) the EFFECTS LOOP (0x4cf638,
  80×0x1E) draws DEBRIS.BIN imgs 0..23 (u16@+0x16 group ×8 +
  frame&7, counter u16@+0x1C++ in the draw) via the DIRECT blit
  FUN_00401e39, sy base 0x100 (−0xC vs robots) + the SECOND
  shake table 0x454518, z Q13; 7j.25 field map CORRECTED:
  d@+0x14 = RISING vz 6000..12069 (high word = the sprite
  group), u16@+0x1A = SPAWN DELAY (the producer ECX arg),
  FUN_0041ec59(n) = bounded-uniform RandB()/(0x8000/n−1)
  helper (identity pinned); mover FUN_00419f62 kills at the
  z=12 ceiling/off-map. (b) the PLATFORM LOOP (0x4eb638,
  32×0x14) uses the ENQUEUE path: DAT_0046af54 = SMOKER.BIN
  (pinned) frame 0 mode 300 + smoke column frame d@+0x10+1
  mode 0x12d (DARKPAL) at sy−0x20; tick FUN_004238af cycles
  2..16 intro/5..16 loop. FUN_00401e39 CODEC DECODED + the
  .BIN container CORPUS-VERIFIED (u16 count word0, u32 dir at
  bank+2+4·img, offsets rel. own slot; 24/24 DEBRIS + 160/160
  DANTE exact-consumption; DEBRIS 24/SMOKER 17/DROPSHIP 210
  imgs — MISSIONVIEW open item 4 RESOLVED, FORMATS §18
  cross-ref). BONUS: the three DROPSHIP ring passes recorded
  (producers → 7j.27) + the [0x4ede24/28] backlog re-pinned
  as the terrain RESTAMP list. 7 new + 2 rewritten ledger
  rows. Manifest verified. PUSHED d9bb40f.
  Queued: the DROPSHIP ring producers (7j.27).
- 2026-08-21: P4 7j.25 the WEAPON-FIRE FAMILY TAIL unit COMPLETE
  (worker 399aeff4 claim 1, commits 3bfd400 + 1016123 + b4950a8
  + 6183be5, D73, docs-only; dump ghidra-project/
  exw-destroytail-asm.txt + full-objdump census). The
  FUN_0041a894 destroy tail decoded WHOLE: TERRAIN RESTORE
  first (footprint W×H×D loop: TOT-mirror z-words ← template
  bank@type+0x46, seen + DAT volume ← bank@type+0x4A, linear
  (z·H+i)·W+j), then the FIVE-EFFECT loop over the type-table
  entries @+0x16+8m — selector word 1..9 → jump table
  0x41a870 (idx sel−1); payload words = tile offsets off the
  0x46cbf4 record; stager stack = (delay, param=score|−1),
  callee ret 8. GER gate REFINED (skips the whole tail for
  type 0xb, record still dies). FUN_0041a225 = FIRST producer
  of the MISSIONVIEW §5d effects bank 0x4cf638. The
  160-vs-0xA8 stride anomaly CLOSED (21·idx·8 = 0xA8 canonical
  — 7j.13 census slip). BONUS: FUN_0041a4f8 = the .POS loader
  (2000×0x10 → the 0x46cbf4 object array) + the .BDG loader
  (the 0x4dedf2 type table) — .BDG grammar CLOSED; FORMATS
  §12/§16/§19 rewritten. 4 new + 2 rewritten ledger rows.
  Manifest verified. PUSHED 6183be5. Queued: 7j.26.
- 2026-08-21: P4 7j.24 the CRITTER DEATH-HANDLER family unit
  COMPLETE (worker 0f986419 claim 1, commit 3819586, D72,
  docs-only; dumps ghidra-project/exw-dead1..5*.txt). The six
  per-kind handlers decoded (k1 FUN_00418835 .. k7
  FUN_0041896c); BOUNTY GATE (killer robot type == [0x4edb90]
  → score += 30/50/500/75/150/1000); SECOND DISPATCHER
  FUN_0040dce0 = debris crush (via physics tick FUN_0040de9c);
  FUN_0041a14f/FUN_0041a494 = the 0x4cec38 effect-row spawner
  + age-LRU allocator; 7j.17 CORRECTED (death handlers never
  call FUN_00424355); FUN_0040e230 SP tail CONFIRMED + MP
  respawn completed; FUN_0042382c = FIRST producer of the
  0x4eb638 platform bank. 8 new + 2 rewritten ledger rows.
  Manifest verified. PUSHED 3819586. Queued: 7j.25.
- 2026-08-21: P4 7j.23 the ACTOR HIT APPLIERS unit COMPLETE
  (worker ad591680 claim 1, commit 45329e9, D71, docs-only;
  dumps ghidra-project/exw-hitters{,2,3,4}*.txt + the NEW
  StoreScan.java operand scanner). FUN_004190bc = the CRITTER
  hit applier (kind switch w@+0x00, damage =
  FUN_00419aff(weapon) per-WEAPON, 6 per-kind death handlers,
  25% knockback FUN_0041a028 + impact SFX FUN_00421fc2);
  FUN_00418fca = robot box-test applier → FUN_0040e230;
  TRAIL ALLOCATOR CLOSED (FUN_00412a4a 20 slots, writer
  FUN_0040a9ff mortar spawner, link/active/ring-zero
  protocol); third critter-applier caller found
  (FUN_00403938 weapon 0xC=5000 blast, owner −1). 7 new + 2
  rewritten ledger rows. Manifest verified. PUSHED 45329e9.
  Queued: 7j.24.

- 2026-08-23: P4/RE THE FUN_004239ef SFX-MESSAGE DISPATCHER unit
  COMPLETE (worker d1578d5c claim 2, commit 38a8463, D125,
  docs-only; objdump-only from ghidra-project/exw-text-objdump.txt,
  no Ghidra run; read-only corpus probes: BEDLAM.EXW DGROUP
  strings + all six LANGUAGE.* files, manifest clean before AND
  after). CLOSED with the identity headline: FUN_004239ef(id,
  channel) = the RADIO-WARNING poster — a 4-channel message
  queue @0x4eb954 (stride 0x28: eight id+1 words +0..+0x1C,
  insert idx +0x20 wrap 8, voice handle +0x24; per-id-per-
  channel dedupe; ids 0x19..0x1B = channel FLUSH then post at
  slot 0; whole queue + display ring MissionShell-zeroed).
  Channels 0/1/2 = squad slots, 3 = system (drained FIRST).
  Consumer FUN_00423a85 (MissionShell @0x447ff5, per frame,
  channels 3→0, oldest-first, ONE message per channel per
  frame): voice leg (text-only ids 0xF/0x29; gates
  [0x4eb93c]/[0x4ede5c]/[0x4ede58]; still-playing poll 0x44c5ac
  keeps the slot queued; take A/B = RandA bit0 off speech
  record 0x4ee014+8·id, play 0x44c8c4 vol 0x7f00, handle
  := ret+1) + consume leg (slot := 0, roll the 4×0x26 display
  ring 0x4ea13c {text[0x20], reveal u16 +0x22, valid u16
  +0x24}, stage text from 0x46c18c+id·0x30, typewriter render
  tail, char tables 0x454c20/0x454b70). THE 53-ID MAP IS
  CORPUS-NAMED: the text table loads at GameMain 0x41c2ff
  from the [WARNINGS] section of LANGUAGE.* (name 0x457ac9;
  sibling [MENU_ITEMS] 0x457abe → 0x46af5c, 64-of-96 loaded);
  all six locales carry exactly 53 records; all 55 call sites
  reconciled (0/1/2 ARRIVED, 3..8 heat, 9/0xA/0xB IMMINENT,
  0xC..0xE+0xF DANGER-TARGETTED/BOMBARDMENT, 0x10..0x18 hits/
  power, 0x19..0x1B TOAST=flush, 0x1C..0x21 weapons, 0x22
  fence, 0x23/0x24 section, 0x25 "X" placeholder zero sites,
  0x26/0x27/0x34 objectives, 0x28/0x29 CONGRATULATIONS,
  0x2A EVACUATION, 0x2B..0x33 battery/damper/ammo).
  CORRECTIONS: §7f.6 "select SFX" gloss (it is the warning
  pair; the blink-cursor write = attention-draw) + §7j.37
  "SFX ids, not text messages" (both speech AND text); §7g.5
  content note recorded (announcement = targeting warning per
  corpus; mechanism facts unchanged). FORMATS §22 = the
  LANGUAGE.* container grammar. Deliverables: RE-EXW-SIM
  §7j.53 + 2 ledger rows + 3 corrections; FORMATS §22; D125;
  the Backlog Mission-SFX-tier bullet closed. registry_anchors
  green; PUSHED. Queued: the 0x4ea238 marker family +
  [0x4de658] census (item 2, arbitrates the §7g.5 tension) + the
  heat-machine warning family (item 3).
- 2026-08-23: P4/RE THE 0x4ea238 MARKER FAMILY + [0x4de658]
  CENSUS unit COMPLETE (worker ed78ecdc claim 2, commit 51800a0,
  D126, §7j.54, docs-only; objdump-only from
  ghidra-project/exw-text-objdump.txt — no Ghidra run, no corpus
  read; MANIFEST clean before AND after; registry_anchors 2/2
  green; PUSHED; ADOPTED + fully re-verified interrupted
  same-item WIP found dirty in the worktree — its §7j.54
  forward-references/ledger rows were staged but the section
  itself was missing, and its 2-caller FUN_004245c9 census was
  corrected to 4). CLOSED with the verdict set: (1) bank
  0x4ea238 = 8 falling-SHELL records × 10 B {x, y (world-px
  ground point), fall-z (0xFF, −0x20/frame), start-delay
  (0x20+2i, −1/frame), valid} — §3's "10-byte records" note =
  this bank; writer = the robots() idle arm 0x40c25e..0x40c351
  (x = px + RandA&0x7F−0x3F one draw pre-gate, y = py−0x80+
  i·0x20 deterministic fan straddling the robot, tile-bounds
  drop); (2) resolver FUN_00423e1c (MissionShell 0x447ffa/frame;
  NOT a "selection chaser" — gloss retired): head decs
  [0x4de658]; fall until get_z_pos ≥ z, then SIX kind-6 debris
  (3 RandA each) + NINE FUN_004244a1 5000-damage blasts over the
  3×3 patch + blink-cursor clear; its record-0 impact block
  (SP ∧ rec 0 ∧ cursor ≠ selected+1 ∧ cursor-robot player-type)
  stages the chase-camera; (3) FUN_004245c9 = a 5-instruction
  CHASE-CAMERA OVERRIDE STAGER (0x4de648/4c/50 + 0xF →
  0x4de654; consumer FUN_00403938 0x4039b0..0x403a42 swaps the
  0x4c71c4/c8/cc anchor slot for 15 frames; robots() 0x40b885
  gates recenter; FULL caller census = FOUR: door stepper
  0x422427, trigger expiry 0x422e55, artillery spotter
  0x41173a, bombardment 0x423ed5) — the "wall-strip redraw"
  gloss family (§7j.19/§7j.21/§7j.22 ×2 + door ledger row)
  corrected in place; (4) [0x4de658] = the salvo COOLDOWN latch,
  full census closed (arm 0x40c27f, gate 0x40c18b, dec
  0x423e25..32, MissionShell clear 0x447877; 0x442ba7 = D89
  loadout-mirror alias, not an access); (5) the D125
  arbitration CLOSED: OFFENSIVE bombardment — the shells ARE
  the bombardment (GENERAL.BIN 0x12C, 32 px/frame descent),
  each impact a 9×5000-damage barrage centered ON the idle
  robot (idle thresholds {400,300,200,5000} frames; ordering
  resets +0x70); §7g.5 "reinforcement ARRIVAL" RETIRED (§7h
  case-1 drop(+0x80)=1000 is the REAL reinforcement and
  stands). No engine consequence (no corpus scenario reaches
  the idle threshold; ≈27–29 RandA/impacting shell if ever
  modeled). Queued: item 3 = the [0x4edbd8] camera-gate census
  (this unit's residual precondition cell).
- 2026-08-23: P4/RE THE [0x4ede34] TEMP-VIEWPORT/DEATH-WIPE
  CENSUS unit COMPLETE (worker 27b33f6c claim 2, commits
  0909683 + c67b007 + c9e3810, D130, §7j.58, docs-only;
  objdump-only from ghidra-project/exw-text-objdump.txt, no
  Ghidra run, no corpus read; MANIFEST.sha256 clean before AND
  after, registry_anchors 2/2 green; PUSHED). VERDICT:
  [0x4ede34] = the CLOSING-IRIS death-wipe cell. (1) VALUE
  GRAMMAR: 0 inactive; :=1 ARM at selected-robot SP death
  (sole 0x40ea8b, FUN_0040e230 SP tail, MP NEVER arms — MP
  posts the sibling marker latch + respawns); +=0x28/frame
  (sole writer, MissionShell frame cluster 0x4480af after the
  present call); terminal :=0x1E0 @0x4480d6 when ≥480; :=0
  cancels = the 3 click-select strips 0x40d286/0x40d311/
  0x40d398 (selecting an ALIVE squadmate aborts the iris) +
  the auto-reselect xor-of-equals 0x448121 + the per-mission
  0x44787d. (2) THE QUEUE ASK (who increments): the frame
  cluster; at terminal it runs the AUTO-RESELECT PASS — walk
  squad slots, gate ALIVE ∧ TYPE(+0x2A)==[0x4edb90] (player
  type) ∧ ≠ selected → [0x46cbdc] := slot (LAST match wins),
  flash :=3, cell :=0; NO eligible mate → parks at 480 =
  exactly the D129 fail-detector conjunct (SP "no cancel" ⟺
  squad wiped; the two conjuncts are one event observed
  twice). (3) WHAT THE TEMP RENDER SHOWS: FUN_00401107 temp
  path = fill-0 full-screen + centered v×v SHRINK of the
  FROZEN world frame (v := 480−min(cell,479); row routine
  0x401430 = inverse twin of the normal zoom's 0x4013e8
  stretch; FUN_00403938 head 0x403952 SKIPS its render body
  during the wipe → the backbuffer holds the last pre-death
  frame) — a 13-frame closing iris 479×479 → 1×1, user zoom
  save/restored. (4) SIBLING [0x4ea8f8] DECODED = the MP
  death-position marker countdown (:=0x20 @0x40e7ef with the
  dying x/y/z posted to [0x4ea8ec/f0/f4]; FUN_00403938 head
  copies the trio into the §7j.20 selected-anchor ring
  0x4c71c4 4×0xC — consumer = the §7j.54 chase camera, the
  camera HOLDS the dead robot's position — and decs; zeroed
  in tandem at every cancel). (5) ONE CORRECTION: §6c.6e's
  auto-reselect flash "ebx(2)" → 3 (ebx := 3 @0x4480de);
  §7j.56/B's "FUN_00401107 gates 0x401119/0x403952" — 0x403952
  is FUN_00403938's gate. ENGINE/DIFFER: NONE (presentation-
  only; recorded for future E render parity). Deliverables:
  §7j.58 A–F, 2 ledger rows, §7j.56/B pointer closed,
  §6c.2/§6c.6e corrections, MISSIONVIEW zoom-path precision
  note, D130. Queued: item 2 = the [0x4dc5d0] blink/effect-
  list producer census (this unit's neighbor open producer).
- 2026-08-23: P4/RE THE EXD NO-EXTRACT-LATCH TWIN CENSUS unit
  COMPLETE (worker 36c6f950 claim 2; RE-notes commit fe1d1d9 +
  impl commit 85d7954, both pushed). TWIN = 0xf929c+i*4: 8 EXD
  readers ONE-FOR-ONE with the EXW 8-reader census + the boot
  memset(0x30=12 dwords) pair @0x2cd41<->0x41c412. HEADLINE:
  WRITER ASYMMETRY — EXD-only setter FUN_0005bb71 @0x5bba0 :=1
  (DOS MP LOBBY ROBOT-PICK: [0x1195dc]:=idx, alive:=0, staging
  memset 0x80, census cmp 2, msg) + the EXD-only lobby type-tally
  0x5ba83; EXW setter set EMPTY (census-complete: 9 literal sites
  = 8 readers + boot memset; no span overlap) — the committed
  §7j.19/§7j.27 "writers" lists CORRECTED in place (those four
  fns are readers); semantics = per-robot CLAIMED flag (EXW
  always takes ==0 paths in SP; engine consequence NONE for the
  SP corpus). 14 §5f cascade aliases (MP-mode 0x1075d8,
  current-robot 0x1075c0 — refines D132's player-type gloss,
  staging 0x9255c, marks 0x8b744, counts 0x10760c/0x107660,
  cursor 0x107688, pod bank 0x8d314, staging quad 0x107764..70,
  selector family 0x8b60c + tables 0x82e5a/0x82e8a, msg gate
  0x894d5, switch/callee + msg-post fn twins, memset fn twin
  0x12206) + robot-bank base 0xf6d34 TRIPLE-confirmed via the
  respawn tail. watches.toml filled (count*4, count cell
  0x11958c), dbx-plan count-driven emission, registry gap set =
  {sfx-master-gate} only, capture-plans S1..S8 regenerated;
  93 diffharness + 13 canonical_dump_gate tests green, MANIFEST
  clean pre+post, no Ghidra run. NOTE: .state/PAUSE (operator
  menu-pointer fix) appeared mid-run at 20:28 — this unit's
  commits stage diffharness/docs paths only, zero overlap; the
  operator's work untouched.

2. DONE (2026-08-23, worker 9a48b338 claim 2, commit 115e240):
   P4/RE EXW BANK-CELL TWIN CROSS-CHECK — the D134 §5g leftovers
   closed (RE-EXD-MAP §5g-bis + D135). The two mission walks are
   store-for-store ORDINAL-IDENTICAL (EXW FUN_0043a1d3 ⟷ EXD
   FUN_0004c121, 27 registers same order) and so are the
   MissionShell-head walks (EXW 0x447bb7.. ⟷ EXD 0x59b83.., 9
   stores). 17 aliases pinned with 1:1 reader-count parity on every
   cell: MIDIGUN 0x4edf60→0x11a954 + dup 0x4edf70→0x11a958
   (consumer-less BOTH sides), SQUISH2/3, POWERUP 9⟷9, MISSILE1,
   ELEV1/2 (TRT structure move), BEEP5 #1/#2 paired BY ORDINAL
   (briefing re-registration twins confirm), TEXTBOX1, BEAMIN 8⟷8,
   THROW 5⟷5, BIOFIRE/PEXPLODE/CACODETH/SQUAWK 1⟷1 each. Docs-only,
   engine consequence NONE; §5g ledger complete for every
   bank-walk cell. D94 EXW walk re-verified independently (idioms
   in exw-text-objdump.txt + 20 DGROUP strings re-read from
   BEDLAM.EXW); objdump-only, no Ghidra run; MANIFEST clean
   pre+post; registry_anchors 2/2 green; pushed.

2. DONE (2026-08-23, worker ec979f34 claim 2, commits 3e3bace +
   cfc6b4c + 6967d3c, all PUSHED): P4/W6 SFX-MASTER-GATE +
   NO-EXTRACT-LATCH E-GAP EMISSION — DECIDED EMIT NOW (D136) and
   landed. E emits sfx-master-gate := constant 1 (T0, every frame
   incl. anchor — the sound-on construction assumption; a
   sound-disabled capture machine dumps 0 = the intended loud
   finding, D134 fingerprint companion intact) and no-extract-latch
   := u32 count + count zero words (T1, count = the robot-bank
   count; MP-lobby-claimed only, all-zero by SP construction,
   D133). Differ: sfx joins the u32 scalar arms on E/O1/O2; the
   latch is count-prefixed canonical with the O1/O2 bare
   $robot_count*4 span converted by prepending len/4; its count
   field STRUCTURAL (the robot-count scenario seams surface exactly
   as on robot-bank.count). ALL canonical chains re-baselined
   deliberately (S0 dac1cfd17bc7ede3, S1 a18cb11ac8e4314e, S2
   d6649ce272ad6d96, S3 f4f5b4351e976ed5, S4 63ab5ac7679f6de7, S5
   8a718339e0702fd6, S5B b72f57e0b8e7042b, S5C de5b80a6177aecdd,
   S6 c27bff339929339d, S7 b0db22840310e82a, S8 29fa2f400a10974b,
   synthetic 6517d1c0b7169446 + frame digest c0268bf499a505c1) —
   live-session O1 comparisons pin against these from cfc6b4c.
   DESIGN 6a: both rows in the canonical table; the E-gaps list
   drops them AND is corrected for D85-era staleness (the
   destroy-family five + T2/T3 staged rows emit since W12). The
   differ's cross-channel coverage counts UNCHANGED (both channels
   carry both rows — the rows compare clean, the cleanest-for-S0
   outcome). 93 diffharness + 13 canonical_dump_gate +
   differ_gate + 76+132 engine lib tests green; fmt+clippy clean;
   MANIFEST clean pre+post. Queued next: item 2 = the differ_gate
   blink-cursor fabrication alignment (noticed this run).

2. DONE (2026-08-23, worker 9035ca6a claim 2, commit 2d53aaa,
   PUSHED): P4.2/W7-followup DIFFER_GATE FABRICATION <->
   REAL-PLAN ALIGNMENT — the fabricated O1 side now carries
   blink-cursor exactly like the real post-D132 capture plans.
   inv_frame fabricates it identity (E canonical u32 == the EXD
   twin cell 0x10e108 form); the differ's O1 normalizer gained
   the NAMED u32 arm (raw passthrough would name-join E's
   "value" vs O1's "raw" and fabricate two field-level coverage
   findings instead of a clean compare — the D136 sfx precedent;
   the O2 alias list carries it too, EXW cell 0x4dc5d0).
   expect_coverage re-derived: S1/S2/S3/S5/S5B/S5C 2->1,
   S4/S7 4->3, S6 3->2, S8 4->3 (S0 stays 0); new guard asserts
   blink-cursor NEVER appears as a finding (any class); the
   stale "registry gap" comment dropped. No chain pins moved
   (E side + both engines untouched). DESIGN §6a blink-cursor
   row carries the cross-channel note; S3/S4/S7/S8 landing
   paragraphs amended in place with history preserved. No
   DECISIONS entry (coverage-class semantics unchanged — the
   row simply moved from one-side to both-sides). Verified:
   differ_gate 1/1 green on corpus (692s), canonical_dump_gate
   13/13, diffharness 76, bedlam-game lib 132, bedlam-core lib
   76, fmt+clippy clean, MANIFEST.sha256 clean post-run. Queued
   next: item 2 = the differ_gate O2 tiebreak fabrication (the
   W7/D87 arbitration path has no gate coverage).
