# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] ADOPT INTERRUPTED WIP, then finish the residual. Three
   provider-outage-killed runs (2026-08-21 19:15/19:34/19:40, two
   HTTP 502 + one Transport) already decoded the ROBOT TARGETING/AIM
   family (7j.16 leads): read .state/agent-31790e94-*.log,
   .state/agent-08f6fa30-*.log, .state/agent-0ce3a285-*.log (tails
   hold the conclusions) + the on-disk ghidra-project/exw-robottarget*
   .txt/-xrefs/-asm dumps (gitignored). FIRST ACTION: write and
   commit the 7j.17 RE-EXW-SIM notes section FROM THOSE LOGS -
   re-verify each claim against the dumps, do NOT re-run Ghidra on
   the four decoded functions. Dead-run headline map (verify then
   pin provenance): FUN_00412f34 = the 0x4cff98 critter-actor
   controller (stride 0x7E; Q13 coords x@+0x36/y@+0x3A/z@+0x3E;
   tile=>>13; its 13 [0x46cbf8] reads = the 0..2 difficulty dial:
   cycled at NameEntryScreen 0x43ab7e `(d+1)%3`, persisted @0x43c3a6,
   money 500*d @0x43aaa3, zone-7 forces d=2 @0x41c568, write
   @0x41c14a; other heavy readers FUN_00403938 x10, FUN_00416458
   x5); FUN_00412a98 = 0x4dabdc POI-seeker; FUN_00409138 = the
   click-order consumer over 0x80-stride records @0x4dd4a0 (39-case
   weapon switch); FUN_00417e2f decoded; FUN_00448b80 =
   mission-objective resolver (6 slots @0x4eaaee, progress
   _DAT_004eba0c, msgs 0x26..0x29, complete-state DAT_0046cd00;
   actor counts DAT_0046cc2c/DAT_0046cbf0 written by FUN_00416458).
   The 0ce3a285 log also holds a verified 47-site k1..k20
   debris-kind caller census + a 28-site record-bank census
   (FUN_00410823 x16, FUN_004190bc x6, FUN_00412010 x4,
   FUN_004197d4 x1) - fold those into the backlog items they
   answer. AFTER the notes commit, only the residual 0x4dd484
   reader census head remains bounded work (skip if the logs
   already cover it). [Amended by watchdog repair 2026-08-21
   19:4x; commit trailer Watchdog-Repair: llm-watchdog 761241
   1787334054.]
## Backlog (not yet started)
- The 0x425xxx arrival-producer family (FUN_0042034c's 45-record
  staging at 0x425daf/0x426079/0x42688c + the register-addressed
  countdown writes + the record draw pass 0x4065f8..0x4066a3) —
  the delayed-arrival scheduler is decoded (7j.11 item 1), its
  producers are not. NOTE 7j.12: the 45x0x10 rectangle list at
  0x4dcae8 (the type-DB tail stamper input) sits IMMEDIATELY
  before the arrival array 0x4dcdb8 — same producer family is
  likely. NOTE 7j.16: the arrival records ARE drawn (scanner
  icon 0xB in FUN_0041ee20) — the family has a confirmed consumer.
- The weapon-fire family REMAINDER (7j.13/14/15/16 done): the
  FUN_00410823 weapon-anim machine internals (the 0x4c71xx record
  family, 6102 B — the biggest piece), the destroy-tail debris-kind
  map (which id-table type@+0xE stages which kinds — the 7j.11
  sites 0x41ace7..0x41b67a; NOTE 7j.16: the 9-case jump table
  @0x41a870 + selectors@+0x16+8k are now pinned), FUN_004190bc
  (the 0x4cff98 record family — 8 octile + 6 damage reads, a
  strong panel/preview candidate), the [0x4edd60] height-bank
  family, the projectile-record z encoding (7j.14 census open),
  and the 160-vs-0xA8 stride anomaly at 0x4c69e4 (FUN_0040fe93;
  7j.16: 0x4c69e4 confirmed the ROBOT bank base, stride 0xA8,
  count 0x46ccbc — the 160 stride at 0x4c69e4/0x4c6a60 needs the
  FUN_0040fe93 view re-anchored). Unlocking the tail re-opens the
  water-splash producers (7j.10) and 17 of 20 debris kinds (7j.11).
- FUN_00440dc2 (the 7j.16 TOT-materializer caller) + the
  [0x4ede24] 7×7 screen-address table: is the materializer the
  scroll/camera restamp? Bounded head decode.
- The 0x4787c4/0x47879c hot-rect record (renderer FUN_00403938
  writes it @0x403c93, count [0x46ccd8]; picker reads
  center@+8/+0xC + w@+0x14, order dispatcher reads corner@+0/+4 +
  z@+0x10 + type@+0x1C — [hypothesis] one 0x20-stride record with
  both views). Anchors the click-target rect semantics.
- The FUN_00416458 mission-load DISPATCHER chain (7j.15/7j.16
  progress: .TOT/.DAT/.CGR/.BIN/.MIN/.LNG/.LNK/.PAD loader
  FUN_0041dc5a pinned @0x447b3a; still open: the .MOFO/.NME/.POS/
  .BDG sibling loaders @0x457a4c..0x457a65) — decoding the
  siblings anchors more FORMATS-MISSION sections (NME partially
  known; POS/BDG/MOFO open).
- The debris-stager ENGINE widening beyond kind 5 (fed by the
  7j.11 20-kind table + the 11 seq tables): model the k2/k8
  single-center scorch (values 3/4), the k1/k20 shared-tail
  ring, and the +0x20 physics classes (0/1/2/3/6 ->
  FUN_0040de9c) — blocked on real producers (weapon family).
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
  entries (7h.2) + the select-ack SFX pair 0xC+k/0xF (7j.6) + the
  debris arrival-SFX pair FUN_00421e60/FUN_00421dec (7j.11 item 4).
- The pickup tile-word PRODUCER (7h.3: the 0x4796bc type-DB
  mirror rows + the probe-latch walk + the DAT z-plane consume +
  the 0x454a90 floor-word swap) — unblocks the apply_pickup
  dispatch from host-seamed to corpus-real; needs the
  MISSIONVIEW sec 8 mirror producers first.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items: type-DB tail producers
  (+0x1a/+0x1b/+0x1c — NOTE +0x18 is CLOSED as the runtime scorch
  writer FUN_00422287 per 7j.8/7j.9, reader verified raw), the
  u32[0x456ca8] anim sequence + the water flag producer (needed
  before the 0x12d/0x12e/0x12f flush remaps can leave water-off
  semantics), BIN u32[bank+0] header word (NOTE 7j.16: the ".BIN"
  load is pinned — header word -> 0x46cdb8; the [0x4ede1c] bank's
  CONTENT consumers still open). CLOSED: u32[0x4dd444]
  (7e.4 - the PALTRAN ramps); +0x18 producer (7j.8/7j.9 -
  FUN_00422287, reader raw, ring landed D57).
- MISSIONVIEW sec 5d tail (robots only are wired): platform loop
  (0x4eb638, bank DAT_0046af54), effects loop (0x4cf638 - the
  FUN_00401e39 draw_IMG codec family, a DIFFERENT .BIN sprite layout
  per RESEARCH-8STREET; the 0xa00 @0x4cec38 + 0x960 @0x4cf638 arrays
  boot-cleared alongside the effect rows per 7j.1), ROBNUMS name
  plates, Shield/Variant bank staging (nodes enqueue, flush skips
  while unstaged). The debris physics/collision FUN_0040de9c (7j.7
  head decode) lives here too (+ the 0x454510+ physics-param dword
  table census-noted in 7j.11 item 5; 3 octile reads per 7j.16).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2; 1 octile read per 7j.16),
  robots() extra-phase semantics + state-1 producers.
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DOSBox-X memory-watches + scripted input injection -> per-frame
  original state dumps diffed against engine state. Design doc first.
  Also arbitrates the two 7j hypotheses (the debris 2k start delay
  and the blink-cursor-from-spawn question) + the 7j.9 overlap
  last-write-wins read of the five rings.
- TOT semantics follow-up: FORMATS sec 2 plane 6/7 (the ~2000-slot
  POS linkage) — KNOWN-staged (word mirror at record words 6/7)
  but the drawer treats them as ordinary stack levels - check
  whether plane 6/7 words ever draw on shipped maps (ZONEA tile
  642 is the only cell) before touching FORMATS. NOTE 7j.16: the
  .TOT volume->mirror materializer FUN_00440a2d copies ALL 8
  planes' nonzero words — the plane semantics now have their
  runtime reader; re-check the mirror-word consumers (0x4796bc)
  for plane-specific behavior.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-21: P4 7j.16 the .TRT CONSUMER hop unit COMPLETE
  (worker 16f43187 claim 1, commit f7262ea, D64, docs-only):
  FUN_00417264 (MissionShell 0x44807b) = the TRT ANIMATION/FIRE
  machine — rec {active@+0, state@+4, anim_frame@+8 (the
  "+0x08 scratch" CLOSED — this machine is its runtime
  producer), fire_ctr@+0xC, hp@+0x10, x/y/z}; states
  1 idle → 2 alert (frames 0..7 → TOT mirror word frame+1) →
  5/6/7/8 aim S/N/W/E (octant vs nearest robot, FUN_00417c00
  octile probe dist<0x81) → FUN_00417698 FIRE (0x28px lane,
  ≤2 levels → projectile TYPE 0x66, damage (d+1)·300, via
  free-slot FUN_0041286f) + 4-frame muzzle (words 0x17..0x1E);
  3/4 death/settle. TURRETS RESTORED (animate+shoot, never
  move); FORMATS §14 re-anchored. The two 3D banks = the map
  FILE VOLUMES: FUN_0041dc5a loads .TOT→[0x4ede20] (u16 W,H
  header + 8 word planes; corpus-verified 30004/160004) and
  .DAT→[0x4edd58] (u8 planes, ≥0x80→0 sanitize) + .CGR/.BIN/
  .MIN/.LNG-.LNK/.PAD (999 slots 0x4e44f8 stamping 0xFF);
  FUN_00440a2d = the TOT-volume→mirror MATERIALIZER (the word-1
  bridge); FUN_0044661b = the EDITOR\ZONE restore reload.
  FUN_00419943 = map-click pick (rects 0x4787c4 by renderer
  FUN_00403938, ret (idx+1)|0x2000 structures); FUN_00410644 =
  click ORDER dispatcher → 0x4dd484/88/8C order target;
  FUN_0041ec81/FUN_0041ee20 = the SCANNER widget (icons 1..0xD
  via 128×128 blitter FUN_00402572). FUN_0041ebf8 = octile
  distance (51 sites). The uncommitted 22c1c14b 7j.13-erratum
  draft corrected + landed: W/H/D stay @+2/+4/+6 (dword>>16
  anchors prove it), word@+0 unconsumed, its 5×8B entries
  @+0x16/count@+0x12/banks@+0x3E..4A/0x4E closure CONFIRMED.
  No engine change (D64). Manifest verified. PUSHED f7262ea.
  Queued: the robot targeting/aim family (7j.16 leads).
- 2026-08-21: P4 7j.13 ERRATUM unit (worker 22c1c14b, claim 1
  pre-restart, docs-only, landed by 7j.16 with corrections):
  an independent full re-decode of the first-hop region
  (FUN_0041a4f8/7f0/1a894 + all 17 sites) cross-checked 7j.13 —
  everything confirmed EXCEPT the item-4 object-type field
  offsets, which mixed two bases and were geometrically
  impossible (ptrs +0x30 collide with the effect entries).
  Corrected: 5×8B entries @+0x16+8k, count@+0x12, template
  banks @+0x3E/+0x42/+0x46/+0x4A — exact 0x4E record fit.
  Its W/H/D +0/+2/+4 shift was itself WRONG (dword>>16 anchors
  consume +2/+4/+6 — proven by 7j.16 at 0x41a857/0x41aa02/
  0x41aaf9/0x41a6fc; original 7j.13 offsets restored, word@+0
  unconsumed [open]). Load-pass counts corroborated
  (0x55EC=282·78, 0x9C40=2000·0x14) + the x/y==−1 forced-dead
  rule. No engine change.
- 2026-08-21: P4 7j.15 weapon-fire family THIRD HOP unit COMPLETE
  (worker efff097c claim 1, commit 52b1ebd, D63, docs-only):
  FUN_00419aff = the WEAPON/PROJECTILE DAMAGE TABLE — a pure id
  switch (NO table walk): 2/3/4→20/30/40, 5→75, 0xc→5000,
  0xd→312, 0x1a→75, 0x24→400, 0x29→250, 0x65→(d+1)·50,
  0x66→(d+1)·300, 0x67/0x68→(d+1)·75, else 1; d = DAT_0046cbf8
  = the DIFFICULTY dword (0..2; d=2 flat overrides
  200/1200/300), pinned: cycled (d+1)%3 at NameEntryScreen,
  save-persisted, 500·d money delta, zone-7 temporarily forces
  2. ERRATUM 7j.13: no field arg (EDX passes through; the
  push 1 only arms the score flag). 28 callers = FUN_00410823×16,
  FUN_004190bc×6 (0x4cff98 bank - second consumer),
  FUN_00412010×4, FUN_004197d4, FUN_00418fca. The 0x4cccf8
  PRODUCER = FUN_004170a6 = the ".TRT" mission-section loader
  (sole caller FUN_00416458 0x416487): clears 250×0x20 (full
  capacity), count→[0x46ccd4], rec {+0=1, +4 active, +8 scratch
  0, +0xC hp=250+(250·mission)/27 → 259..490, +0x10 x, +0x14 y,
  +0x18 z} at stager base 0x4cccfc (the 7j.14 resolver frame is
  +4 — its offsets stand); ALSO stamps tile byte 0x66 into the
  3D tile bank [[0x4edd58]+x+y·w+z·w·h] + word 1 into the 3D
  word bank [[0x4ede20]+2(x+y·w+z·w·h)] (both new, consumers
  open). FORMATS-MISSION §14 ANCHORED: TRT third u32 = z LEVEL
  (0..6), not a type enum. No engine change (D63). Manifest
  verified. PUSHED 27f5def..52b1ebd (the 7j.13/7j.14 push debt
  cleared too — secret service back after the session restart).
