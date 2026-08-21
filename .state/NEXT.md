# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The EXIT/ESCAPE runtime family — the two open consumers
   from 7j.18: FUN_0041fbb1 (reads the 5×0x1C exit slots
   0x4e662c @0x41fcf8 — likely the escape arrival/draw side)
   and FUN_00433980 (sole caller of the exit-pad activator
   FUN_0041fa51 @0x43900e — the pad/elevator TRIGGER handler;
   also expected to write [0x4eba0c]/[0x4eba10]). Bounded
   head: decompile both + the 0x4eba0c/0x4eba10 consumer
   census; fold into RE-EXW-SIM 7j.19 + the exit-slot ledger
   row. Closes the personnel-rescue loop end-to-end
   (loader 7j.18 → flee controller 7j.17 → exit runtime).
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
- The weapon-fire family REMAINDER (7j.13/14/15/16/17/18 done): the
  FUN_00410823 weapon-anim machine internals (6102 B — the
  biggest piece; its 0x4c71f4 record family is now 400x0x36
  with frame + spawners pinned per 7j.17), the destroy-tail
  debris-kind map (which id-table type@+0xE stages which kinds
  — the 7j.11 sites 0x41ace7..0x41b67a; the 9-case jump table
  @0x41a870 + selectors@+0x16+8k pinned; NOTE 7j.17: critter
  death is now a confirmed non-weapon producer of k1/k6 +
  FUN_00424355 + the 0x4cec38 effect rows via FUN_0041a14f),
  FUN_004190bc (the 0x4cff98-family second stat consumer —
  8 octile + 6 damage reads, a strong panel/preview
  candidate; 7j.17 gives it the bank layout to check
  against), and the 160-vs-0xA8 stride anomaly at 0x4c69e4
  (FUN_0040fe93; 7j.16: 0x4c69e4 confirmed the ROBOT bank
  base, stride 0xA8, count 0x46ccbc — the 160 stride at
  0x4c69e4/0x4c6a60 needs the FUN_0040fe93 view re-anchored;
  NOTE 7j.18: FUN_0040db9e writes the robot stun word
  @0x4c69e4+idx·0xA8). CLOSED by 7j.17: the [0x4edd60]
  height-bank family and the projectile z-encoding census.
  OPEN small: projectile type 0x69 vs the FUN_00419aff damage
  table (7j.17/7j.18 — low priority).
- FUN_00440dc2 (the 7j.16 TOT-materializer caller) + the
  [0x4ede24] 7×7 screen-address table: is the materializer the
  scroll/camera restamp? Bounded head decode.
- The 0x4787c4/0x47879c hot-rect record (renderer FUN_00403938
  writes it @0x403c93, count [0x46ccd8]; picker reads
  center@+8/+0xC + w@+0x14, order dispatcher reads corner@+0/+4 +
  z@+0x10 + type@+0x1C — [hypothesis] one 0x20-stride record with
  both views). Anchors the click-target rect semantics.
- The FUN_00416458 sibling loaders @0x457a4c..0x457a65
  (.MOFO/.POS/.BDG — .NME/.TRT now CLOSED per 7j.15/7j.18):
  who stages .POS (2000×16B — the scenery placement bank) and
  .BDG/.BLD; decodes FORMATS §12/§16/§17 semantics.
- The debris-stager ENGINE widening beyond kind 5 (fed by the
  7j.11 20-kind table + the 11 seq tables): model the k2/k8
  single-center scorch (values 3/4), the k1/k20 shared-tail
  ring, and the +0x20 physics classes (0/1/2/3/6 ->
  FUN_0040de9c) — blocked on real producers (weapon family;
  NOTE 7j.17: critter death k1/k6 producers exist but are
  outside the corpus path until critters load).
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
  NOTE 7j.17 pinned new FUN_0043a48e banks: _DAT_004edf94/
  _DAT_004edfe4/_DAT_004edfac (robot fire) and
  _DAT_004edffc/_DAT_004edff0/_DAT_004edfa8 (critters/POI).
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
  boot-cleared alongside the effect rows per 7j.1 — NOTE 7j.17:
  0x4cec38 is 0x20-stride effect rows with a reachable spawner
  FUN_0041a14f), ROBNUMS name plates, Shield/Variant bank staging
  (nodes enqueue, flush skips while unstaged). The debris
  physics/collision FUN_0040de9c (7j.7 head decode) lives here too
  (+ the 0x454510+ physics-param dword table census-noted in 7j.11
  item 5; 3 octile reads per 7j.16; NOTE 7j.17: it reads BOTH the
  critter and POI counts — collision family).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2; 1 octile read per 7j.16;
  NOTE 7j.17: it also reads the command count 0x46cbe0 — MP shop
  sync), robots() extra-phase semantics + state-1 producers.
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
- 2026-08-21: P4 7j.18 the critter/POI/exit LOADER hop unit
  COMPLETE (worker a840f0af claim 1, commits 7f1c8fb docs +
  f04681d tooling, D66): FUN_00416458 stages ".NME" (@0x457a57,
  bytes verified) and reads EIGHT fixed-order count+records
  sections (widths 10/10/8/8/10/8/6/8; 16 FUN_0041cccb call
  sites census-verified) — sections 1-7 spawn critter states
  2/1/5/4/3/6/7 (spawn multipliers by difficulty 0x46cbf8;
  hp = base+(base·d)/27, bases 0xAF/0xC8/0x96/0x5DC/0x9C4;
  +0x02 species word 1/3/6; octile tables 0x4543e4/0x454404
  @+0x60; S2 DAT z=6-down floor search; S7 z fixed 0xDF);
  section 8 feeds the POI bank (4 POIs per record, jitter ±31
  sub-tiles, spawn state 5 ESCAPE — personnel flee from
  load). Corpus-exact on all 37 files (ZONEA/M1 keeps a 16-B
  orphan tail no game code reads; FUN_004180b9 = empty stub).
  FORMATS-MISSION §9 REWRITTEN (grammar CLOSED; old
  header/section model was a mis-split). FUN_0041fa51 = the
  EXIT-PAD ACTIVATOR: arg = a 0x4e44f8 .PAD slot index, dedup
  registry 5×d @0x46cd20, stamps exit rec {1, 1,
  pad.x·0x20+0xF, pad.y·0x20+0xF, 0x400, 0}; caller
  FUN_00433980 @0x43900e = pad trigger handler [open]. 7j.17
  leftovers folded: FUN_00449c94 = the LOCAL COMMAND-RECORD
  BUILDER (0x4dd4a0 stride-0x80, cmd codes 1-4, payload
  words, MP broadcast loop + NETWORK ERROR paths),
  FUN_0040db9e = the critter ranged-attack APPLIER on robots
  (0x476fe4 0xC-stride weapon-param table, param_5=−1 → the
  critter entry @0x476fd8; robot stun word 0xFFFF
  @0x4c69e4+idx·0xA8 + FUN_0040c536 timed effect scaled by
  octile dist·mult), [0x4eb8b8+slot·4] census = objective-done
  flags (MissionShell + FUN_0044425c + FUN_00448b80 only).
  ENGINE/TOOLING: parse_nme replaced by the exact schedule +
  corpus exact-consumption test (37/37); fmt+clippy clean,
  workspace green, manifest verified. PUSHED f04681d. Queued:
  the exit/escape runtime family (FUN_0041fbb1 +
  FUN_00433980).
- 2026-08-21: P4 7j.17 the ROBOT TARGETING/AIM family ADOPT
  unit COMPLETE (worker 3f4f7c10 claim 1, commit eaf16c0,
  D65, docs-only): landed the three provider-outage-killed
  runs' decode (logs agent-31790e94/08f6fa30/0ce3a285, dumps
  exw-robottarget*.txt/-xrefs/-asm, NO new Ghidra run —
  every claim re-verified). FUN_00412f34 = the 0x4cff98
  CRITTER-ACTOR controller (stride 0x7E, Q13 x@+0x36/
  y@+0x3A/z@+0x3E; states 1 wander / 2 sine-walk shooter
  (0x65, range (2−d)·−0x40+300) / 3 chase (0x67 3D velocity,
  pathfinder FUN_0041571c, leash 400) / 4-5-6 mixed-AI
  (0xB dormant + DAT_00454edc[d] respawn delays; 6 ballistic
  → k6 debris + FUN_00424355 + splash FUN_0041a14f(0x18);
  9 seek-steppers; 2 FUN_0040db9e range attack) / 7
  close-combat (point-blank 0x69 @ 32/16/8-frame cadence,
  break odds 1/8·1/16·never, leash (d+1)·0x40+600);
  presence byte mark [[0x4ea900+(y>>13)·4]+[0x46af4c]+
  (x>>13)]:=1 (SAR 0xD asm-verified; the decompile >>5 was
  an artifact). Difficulty dial amended: 12 objdump sites
  — drives critter behavior, not only damage.
  FUN_00417e2f = SUICIDE-BOMB trigger (<0x30 px → 8× k1
  debris). FUN_00412a98 = the 0x4dabdc POI/PERSONNEL
  controller (stride 0x1E; flee-to-exit over 5×0x1C slots
  @0x4e662c via FUN_00417c64; escape → [0x4eba0c]++,
  [0x4eba10]=0x32, FUN_00448b80(5000)). FUN_00409138 = the
  COMMAND-RECORD consumer (0x4dd4a0 stride 0x80, count
  0x46cbe0, builder FUN_00449c94 + MP family; 39-case
  weapon switch → order dispatchers + projectile spawners
  into the 400×0x36 bank 0x4c71f4 aimed at the ORDER
  TARGET; auto-rearm + msgs 0x1C..0x21). FUN_00448b80 =
  the MISSION-OBJECTIVE RESOLVER (6×0x20 slots @0x4eaaee;
  msgs 0x26/0x27/0x34, all-done 0x28+0x29 → DAT_0046cd00
  phase state; zone-7 counter [0x46cce0]).
  FUN_0041e411 = floor probe (the [0x4edd60]=.CGR
  height-bank semantics). Residual 0x4dd484 reader census
  CLOSED; the 47-site/28-site censuses re-read unchanged
  (both already landed in 7j.11/7j.15 — the queue's "fold"
  ask was already satisfied; 7j.17 adds critter-death
  producers k1/k6/FUN_0041a14f on top). No engine change
  (D65). Manifest verified. PUSHED eaf16c0. Queued: the
  critter/POI/exit loader section in FUN_00416458.
