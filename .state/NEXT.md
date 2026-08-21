# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The 0x4dc5d0 blink/effect-row producer family (RE decode
   first): FUN_00422038 (the slot alloc — the 16-B rows the
   sidebar tail's `_DAT_004dc5d0 >= 1/2/3` switch reads at
   0x407420, drawing at x 0x1F0/0x222/0x254) + the staged inputs
   already decoded on the sim side (the 7g.6 death tail's FIVE
   debris rows via FUN_00420608 with kind 5 / param 2k / z+8k /
   the two RandA draws each, and the 7h.2 PickupOutcome effect
   ids 1/6/7/0xE) + the FUN_00420608 128-slot 0x30-stride debris
   stager consumer family. Bounded: decode the producer + stager
   asm as committed RE notes FIRST (RE-EXW-SIM 7j or a new
   RE-EXW-EFFECTS doc), then land the effect-row seam. The
   blink-cursor producer (FUN_00403938's own 0x4dc5d0 stages)
   may ride along ONLY if trivially adjacent.
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
  entries (7h.2).
- The pickup tile-word PRODUCER (7h.3: the 0x4796bc type-DB
  mirror rows + the probe-latch walk + the DAT z-plane consume +
  the 0x454a90 floor-word swap) — unblocks the apply_pickup
  dispatch from host-seamed to corpus-real; needs the
  MISSIONVIEW sec 8 mirror producers first.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items 1/2/4: type-DB tail producers
  (+0x18/+0x1a/+0x1b/+0x1c — NOTE +0x18 is now KNOWN as the
  per-tile armor-pad byte consumer side, 7g.3; the producer is
  still open), the u32[0x456ca8] anim sequence + the water flag
  producer (needed before the 0x12d/0x12e/0x12f flush remaps can
  leave water-off semantics), BIN u32[bank+0] header word. CLOSED:
  u32[0x4dd444] (7e.4 - the PALTRAN ramps).
- MISSIONVIEW sec 5d tail (robots only are wired): platform loop
  (0x4eb638, bank DAT_0046af54), effects loop (0x4cf638 - the
  FUN_00401e39 draw_IMG codec family, a DIFFERENT .BIN sprite layout
  per RESEARCH-8STREET), ROBNUMS name plates, Shield/Variant bank
  staging (nodes enqueue, flush skips while unstaged).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2), robots() extra-phase
  semantics + state-1 producers.
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DOSBox-X memory-watches + scripted input injection -> per-frame
  original state dumps diffed against engine state. Design doc first.
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
