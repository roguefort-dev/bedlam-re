# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The .TRT terrain-structure CONSUMER hop (7j.15 re-open):
   the three 0x4cccf8-array scanners — FUN_00417264 (reads the
   +0x08 scratch dword + y; calls FUN_00417652/FUN_00417210),
   FUN_00419943 (RandA/IDIV scatter vs map globals 0x4edde4/
   0x4edde8 — a placement search), FUN_0041ee20 (active scan) —
   PLUS the consumers of the two new 3D banks the producer
   stamps (tile byte 0x66 bank behind [0x4edd58], word bank
   behind [0x4ede20]) and the +0x08 scratch dword producer.
   Answers: do TRT structures shoot/move/animate (the retired
   "turrets?" question) and what the two 3D banks feed.
   Bounded: those three functions full + 3D-bank xref census.
   NOTE: push debt is CLEARED (27f5def..52b1ebd landed 21 aug
   ~19:0x after the session came back).
## Backlog (not yet started)
- The 0x425xxx arrival-producer family (FUN_0042034c's 45-record
  staging at 0x425daf/0x426079/0x42688c + the register-addressed
  countdown writes + the record draw pass 0x4065f8..0x4066a3) —
  the delayed-arrival scheduler is decoded (7j.11 item 1), its
  producers are not. NOTE 7j.12: the 45x0x10 rectangle list at
  0x4dcae8 (the type-DB tail stamper input) sits IMMEDIATELY
  before the arrival array 0x4dcdb8 — same producer family is
  likely.
- The weapon-fire family REMAINDER (first hop 7j.13, second 7j.14,
  THIRD 7j.15 — FUN_00419aff damage switch + difficulty 0x46cbf8 +
  the .TRT producer FUN_004170a6 pinned): after the consumer hop,
  the FUN_00410823 weapon-anim machine internals (the 0x4c71xx
  record family, 6102 B — the biggest piece), the destroy-tail
  debris-kind map (which id-table type@+0xE stages which kinds —
  the 7j.11 sites 0x41ace7..0x41b67a), FUN_00412f34/FUN_00417e2f
  (both 0x46cbf8 readers), the FUN_004190bc 0x4cff98 record
  family (the second stat consumer — panel/preview candidate),
  the [0x4edd60] height-bank family, the projectile-record z
  encoding (7j.14 census open), and the 160-vs-0xA8 stride
  anomaly at 0x4c69e4 (FUN_0040fe93). Unlocking the tail
  re-opens the water-splash producers (7j.10) and 17 of 20
  debris kinds (7j.11) for any future corpus seam.
- The FUN_00416458 mission-load DISPATCHER chain (7j.15: sole
  caller of the .TRT loader; clears 0x4cff98/0xac44 +
  0x4dabdc/0xf00, then opens .NME; sibling tags .MOFO/.NME/.TRT/
  .POS/.BDG @0x457a4c..0x457a65) — decoding the sibling loaders
  anchors more FORMATS-MISSION sections (NME semantics partially
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
  semantics), BIN u32[bank+0] header word. CLOSED: u32[0x4dd444]
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
  table census-noted in 7j.11 item 5).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2), robots() extra-phase
  semantics + state-1 producers.
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DOSBox-X memory-watches + scripted input injection -> per-frame
  original state dumps diffed against engine state. Design doc first.
  Also arbitrates the two 7j hypotheses (the debris 2k start delay
  and the blink-cursor-from-spawn question) + the 7j.9 overlap
  last-write-wins read of the five rings.
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
  (0..6), not a type enum; "turrets?" retired as primary. No
  engine change (D63). Manifest verified. PUSHED 27f5def..52b1ebd
  (the 7j.13/7j.14 push debt cleared too — secret service back
  after the session restart). Queued: the FOURTH HOP (the .TRT
  consumer trio + FUN_004190bc).
- 2026-08-21: P4 7j.15 weapon-fire family THIRD HOP unit COMPLETE
  (worker efff097c claim 1, commit 52b1ebd, D63, docs-only):
  FUN_00419aff = the WEAPON/PROJECTILE DAMAGE TABLE — a pure
  id switch (NO table walk): 2/3/4→20/30/40, 5→75, 0xc→5000,
  0xd→312, 0x1a→75, 0x24→400, 0x29→250, 0x65→(d+1)·50,
  0x66→(d+1)·300, 0x67/0x68→(d+1)·75, else 1; d = DAT_0046cbf8
  = the DIFFICULTY dword (0..2; d=2 flat overrides
  200/1200/300), pinned: cycled (d+1)%3 at NameEntryScreen,
  save-persisted, 500·d money delta, zone-7 temporarily forces
  2. ERRATUM 7j.13: no field arg (EDX passes through; the push 1
  only arms the score flag). 28 callers = FUN_00410823×16,
  FUN_004190bc×6 (0x4cff98 bank - second consumer),
  FUN_00412010×4, FUN_004197d4, FUN_00418fca. The 0x4cccf8
  PRODUCER = FUN_004170a6 = the ".TRT" mission-section loader
  (sole caller FUN_00416458 0x416487): clears 250×0x20 (full
  capacity), count→[0x46ccd4], rec {+0=1,+4 active,+8 scratch 0,
  +0xC hp=250+(250·mission)/27 → 259..490, +0x10 x,+0x14 y,
  +0x18 z} at stager base 0x4cccfc (7j.14 resolver frame +4 —
  offsets stand); ALSO stamps tile byte 0x66 into the 3D tile
  bank [[0x4edd58]+x+y·w+z·w·h] + word 1 into the 3D word bank
  [[0x4ede20]+2(x+y·w+z·w·h)] (both new, consumers open).
  FORMATS-MISSION §14 ANCHORED: TRT third u32 = z LEVEL (0..6),
  not a type enum; "turrets?" retired. No engine change (D63).
  Manifest verified. PUSHED 27f5def..52b1ebd (the 7j.13/7j.14
  push debt cleared too — secret service back after the session
  restart). Queued: the .TRT consumer hop.
- 2026-08-21: P4 7j.14 weapon-fire family SECOND HOP unit
  COMPLETE (worker d37fb3a2 claim 1, commit 7b9ce05, D62,
  docs-only): FUN_0041bc1c = the TERRAIN-STRUCTURE damage
  resolver (x/y Q13, damage): scans the NEW array 0x4cccf8
  stride 0x20 count [0x46ccd4] {active@+0, hp@+0x10, x tile@+0x14,
  y@+0x18, z@+0x1C}, externally 1-based (dword[0x4cccd8+id·0x20],
  guard at 0x4cccd8) — hp−=damage only on survivors; destroy →
  floor word [0x454a04+4·zone] → TOT mirror 0x4796bc+30·tile+2z,
  seen @0x4796cc=1, DAT volume byte=0, debris K0xF, splash at
  first free level. NO robot-armor branch (7j.13 question closed
  TERRAIN-only). FUN_0041eaa1 = the per-pixel TERRAIN-HEIGHT
  probe (DAT volume byte 0 → miss; else the 32×32 height bank
  behind [0x4edd60] entry (h−1)·4+2 +6 header; hit iff z ≤
  (z>>5)·0x20+byte; 3 sites in FUN_00412010; rec-z encoding left
  open). FUN_004124a4 = the weapon-anim disburser (rec
  0x4c71f4+0x36·i, kind word@+0: w2..4→K2 ±3 jitter, 5→K3,
  0x24→K6, 0x29→K9, {0xE,0xF,0x13,0x17,0x1A,0x1F}→K0xC, 9..0xB
  clear-only; z−10; all 9 callers in FUN_00410823).
  FUN_004126dc = the projectile disburser (rec 0x4cc654+0x22·i,
  +0 = TYPE word 0=free — refines 7j.13 "active": 1→K2,
  0x65→K0x14, 0x66→K8, 0x67/0x68→K4, no z−10; robot-hit expiry
  walker FUN_004197d4 |dx|<0x10 Q8 ∧ |dz|<0x20; projectile type
  ids = weapon-stat ids). Splash addendum: FUN_00424355 gates
  (DAT-empty ∧ TOT word 0 ∧ claim byte[0x46af58]) + max-age
  eviction via FUN_0042394a. No engine change (D62). Manifest
  verified. Push landed with the 7j.15 batch (52b1ebd).
