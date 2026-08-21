# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The 7j.8 scorch/armored-pad re-verify (small bounded RE):
   re-read the robots() phase-1 armor reader at asm 0x40bbab..0x40bc9f
   byte-precisely and decide whether FUN_00422287's scorch values
   (1/2/4, clamped < 8) and the armor-pad semantics share the
   type-DB +0x18 byte (the reader tests the RAW byte != 0 per the
   current decode — meaning a death would arm six armor-pad tiles
   around the corpse). If the reader is confirmed raw: land the
   scorch staging in the engine debris seam (the six FUN_00422287
   ring writes per death debris over the engine's type-DB mirror —
   the terrain armor-pad consume path already models byte != 0),
   with a unit test pinning the ring offsets/values; if a mask
   separates the families instead, document it and keep the scorch
   unwired. Also census FUN_00422287's OTHER callers (grep call
   0x422287) — kind-5 may not be the only scorch producer.
## Backlog (not yet started)
- Keyboard latch wiring for the sidebar (F1/F2/F3, keys 1..7,
  MSpace; RE-EXW-INPUT line 95) - blocked on the P2e InputFrame
  button bit-map assignment.
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence,
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- Mission SFX tier (RE-EXW-SIM sec 9 open item 5; MENU1/MENU2-style
  mixer instruments exist) + the order SFX 0x2A armer click + the
  damage/alarm SFX families (7g.1) + the pickup SFX 0x43a48e
  entries (7h.2) + the select-ack SFX pair 0xC+k/0xF (7j.6).
- The pickup tile-word PRODUCER (7h.3: the 0x4796bc type-DB
  mirror rows + the probe-latch walk + the DAT z-plane consume +
  the 0x454a90 floor-word swap) — unblocks the apply_pickup
  dispatch from host-seamed to corpus-real; needs the
  MISSIONVIEW sec 8 mirror producers first.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items: type-DB tail producers
  (+0x1a/+0x1b/+0x1c — NOTE +0x18 is now KNOWN as the runtime
  scorch writer FUN_00422287 per 7j.8/§8.1, caveat on the armor-pad
  reader), the u32[0x456ca8] anim sequence + the water flag
  producer (needed before the 0x12d/0x12e/0x12f flush remaps can
  leave water-off semantics), BIN u32[bank+0] header word. CLOSED:
  u32[0x4dd444] (7e.4 - the PALTRAN ramps); +0x18 producer
  (7j.8 - FUN_00422287, reader caveat pending).
- MISSIONVIEW sec 5d tail (robots only are wired): platform loop
  (0x4eb638, bank DAT_0046af54), effects loop (0x4cf638 - the
  FUN_00401e39 draw_IMG codec family, a DIFFERENT .BIN sprite layout
  per RESEARCH-8STREET; the 0xa00 @0x4cec38 + 0x960 @0x4cf638 arrays
  boot-cleared alongside the effect rows per 7j.1), ROBNUMS name
  plates, Shield/Variant bank staging (nodes enqueue, flush skips
  while unstaged). The debris physics/collision FUN_0040de9c (7j.7
  head decode) + the other 19 FUN_00420608 kinds (explosions,
  projectiles) live here too.
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2), robots() extra-phase
  semantics + state-1 producers.
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DOSBox-X memory-watches + scripted input injection -> per-frame
  original state dumps diffed against engine state. Design doc first.
  Also arbitrates the two 7j hypotheses (the debris 2k start delay
  and the blink-cursor-from-spawn question).
- TOT semantics follow-up: FORMATS-MISSION sec 2 plane 6/7 (the
  ~2000-slot POS linkage) is now KNOWN-staged (word mirror at
  record words 6/7) but the drawer treats them as ordinary stack
  levels - check whether plane 6/7 words ever draw on shipped maps
  (ZONEA tile 642 is the only cell) before touching FORMATS.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-21: P4 effect-row seam unit COMPLETE (worker 6ab53863
  claim 1, commits 4f858d9 + e706a33 + 9bbf1ac, D56): RE-EXW-SIM
  7j = the 0x4dc5d0 family decoded (the 10 16-B effect rows at
  0x4dc5d4 {x,y,z,id} + the FUN_00422038 alloc + the FUN_0042205c
  rise-tick + the FLAGS.BIN draw pass + the COMPLETE effect-id
  table {1,6,7,1,0xE,0xC,0xD} per case; the blink-cursor scalar
  _DAT_004dc5d0 = selected slot+1, its producers/consumer pinned)
  + FUN_00420608 = the 128x0x30 debris stager (z clamp, LRU
  eviction, 20-kind table, kind 5 = death debris with the six
  FUN_00422287 scorch-ring writes — closing the MISSIONVIEW §8.1
  +0x18 producer question with the armor-pad reader caveat) + the
  FUN_00420549 seq tick + the BLOWUP(B/G).BIN draw pass. ENGINE:
  NodeBank::{Flags,Blowup} + enqueue_effects in bedlam-render,
  EffectRows + DebrisFx presentation state + the damage/pickup
  seam stagings + the epilogue-order ticks + the sidebar blink
  cursor in bedlam-game; FLAGS.BIN + BLOWUP.BIN join the 25-file
  chain. Gates: ALL pins UNMOVED (effects off the default corpus
  path, cursor 0 until a select click), smoke two-run
  byte-identical AT the recorded baselines (scene 696adb1cd110e062,
  parity cce30c983b97b16d), 41 suites green (+3 render units, +6
  game units, +1 corpus gate with the control-host diff), fmt/
  clippy clean, MANIFEST verified. Pushed. Queued: the 7j.8
  scorch/armored-pad re-verify.
- 2026-08-21: P4 dead/hit dither unit COMPLETE (worker efc8b1e0
  claim 1, commits 4f702e1 + 31a4691, D55): RE-EXW-SIM 7i =
  FUN_00401ae6 fully decoded (mode 0 rep-movsb full static vs
  mode 1 nonzero-only overlay; dest = fb + y*pitch + x; per-row
  RESEED rand&0x1ff when src+96 >= 0x800; seed =
  FUN_0041ec59(0x7f6,0x30) = (RandB()&0x7fff)/15 clamp 0x7f5) +
  the bank REFUTED as EXE content: 0x4e6ed8 is a 2048-B .bss RING
  (cursor 0x4ddb30) of binary {0,0xFF} 25% white — boot fill
  2048 draws (MissionShell 0x447b13) + 15-B/frame churn
  (0x448147 epilogue, unconditional); +0x2E confirmed hit_flash:
  in-squad dead/hp<1 -> mode 0, flash != 0 -> mode 1 after the
  portrait, beyond-squad slots -> mode 0 EVERY frame. ENGINE: the
  Dither ring + blit in draw_sidebar_portraits (reads the sim
  hit_flash, never decays — 7g.8 stays the sim tick), edge_rng ->
  rand_b shared stand-in consumed in the EXW order (terrain edges
  -> dither -> churn), sidebar block moved after the terrain pass
  (disjoint halves, pixels identical). Gates: frame pins RE-PINNED
  ONCE (spawn 7fdada56b10f1cad, walk 58ea10373e8d4284, overlay
  1d70e0bd059f5ae0, armed 6050d20755b2d852 — ZONEA 1-robot squad
  dithers slots 1/2; reason in the gate header), sim pins
  byte-identical, the overlay gate's stale-sidebar reference
  re-anchored to the last-presented frame, 41 suites green (+1),
  fmt/clippy clean, smoke two-run identical at the recorded
  baselines, MANIFEST verified. Pushed.
- 2026-08-21: P4 pickup consumer unit COMPLETE (worker 66831068
  claim 1, commits e10fdb5 + d8e03a7 + 5a3a419 + 81fd558, D54):
  RE-EXW-SIM 7h = the FUN_0040eba0 dispatch decoded (range tables
  0x454a58/0x454a74 — CORRECTED A values [0x4e,0x75,0x75,0x358,
  0x75,0xa3,0xa3] after a byte-precise re-dump; closed 4-word
  groups → cases A:1/3/2/4 B:9/7/8; the jump table; the case
  bodies with effect ids 1/6/7/0xE; the caller consume block
  (DAT z-plane zero + 0x454a90 floor-word swap + probe-latch
  walk); the _DAT_004edd8c producers). ENGINE: pickup_case pure
  decode + MissionSim::apply_pickup cases 1/2/3/7 (drop 1000,
  shield 1000, hp +=2500 clamp 5000, shield_boost 200 — writes
  hash-covered D53 fields) + PickupOutcome (effect-id seam) +
  the MissionScene::pickup host seam; case 4 kept as the D52
  seam. Gates: workspace green (+4 tests), fmt/clippy clean,
  smoke two-run byte-identical AND at the recorded baselines
  (scene 696adb1cd110e062, parity cce30c983b97b16d — pins
  UNMOVED, the seam is off the corpus path), MANIFEST verified.
  Pushed.
