# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The weapon-fire family TAIL head, promoted by 7j.22:
   the ACTOR HIT APPLIER internals — FUN_004190bc (the critter
   hit test/damage applier; the 7j.15 "panel/preview"
   hypothesis is CORRECTED, it is called (critter, owner, x, y,
   z, weapon, mode 2) from FUN_0041879d's presence-gated lane;
   6 FUN_00419aff reads = per-critter damage lookups) +
   FUN_00418fca (the robot sibling via FUN_0041874c, skips the
   owner, MP-gated). Decode both (mode semantics, the hit test,
   damage/hit-flash application, the 0x7E-stride critter-record
   reads +0xC state/+0x24 word) → fold into RE-EXW-SIM 7j.23 +
   ledger rows. Small addendum if room: the 0x4e66b8
   smoke-trail bank slot allocator (writer of the 0x4c71f4
   record's +0x32 link ≠ −1) — the trail-ring draw pass can
   stay backlog.
## Backlog (not yet started)
- The weapon-fire family TAIL (after 7j.23): the destroy-tail
   debris-kind map (which id-table type@+0xE stages which kinds
   — the 7j.11 sites 0x41ace7..0x41b67a; the 9-case jump table
   @0x41a870 + selectors@+0x16+8k pinned; NOTE 7j.17: critter
   death is now a confirmed non-weapon producer of k1/k6 +
   FUN_00424355 + the 0x4cec38 effect rows via FUN_0041a14f),
   and the 160-vs-0xA8 stride anomaly at 0x4c69e4
   (FUN_0040fe93; 7j.16: 0x4c69e4 confirmed the ROBOT bank
   base, stride 0xA8, count 0x46ccbc — the 160 stride at
   0x4c69e4/0x4c6a60 needs the FUN_0040fe93 view re-anchored;
   NOTE 7j.18: FUN_0040db9e writes the robot stun word
   @0x4c69e4+idx·0xA8). CLOSED by 7j.17: the [0x4edd60]
   height-bank family and the projectile z-encoding census.
   OPEN small: projectile type 0x69 vs the FUN_00419aff damage
   table (7j.17/7j.18 — low priority); the trail-ring draw
   pass consuming the 0x4e66b8 bank (7j.22).
- The per-zone FUN_00433980 case table (≈28 pad ids × 7 zones,
  beyond the §7j.19 head decode; §7j.20 item 2 gives the ~25
  extraction-pad (zone,slot) pairs and §7j.21 the record
  high-water marks + the record↔pad arm mapping task) + the
  FUN_00424a6f message string table — mechanical, decode per
  zone only when P4.2 needs it.
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
  outside the corpus path until critters load; NOTE 7j.22: the
  weapon family producers are now fully anchored — bullets/
  shell/artillery/ballistic/rocket/homing tick semantics +
  the K0xB/K2/K3/K6/K9 in-flight emissions are pinned).
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
  NOTE 7j.20: the beacon armer's SFX is FUN_004239ef(0x2a,3).
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
  last-write-wins read of the five rings. NOTE 7j.20: the harness
  must model the mission-start pod-descent stagger (w@+0x2C =
  1+k·(2000−m·1000/27)) — the first seconds of any mission have
  the robots frozen in pods — and arm extraction via a scripted
  .PAD step-on, not a click. NOTE 7j.22: weapon fire needs
  injected COMMAND records (FUN_00449c94/0x4dd4a0) or order
  dispatch, not raw input — the fire family is fully anchored
  for it (per-type cadences + damage tables).
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
- 2026-08-21: P4 7j.22 the WEAPON-ANIM MACHINE head unit COMPLETE
  (worker 27e4f048 claim 1, commit 29adbf1, D70, docs-only;
  3 × -process runs, dumps ghidra-project/exw-weaponanim{,2,3}*
  .txt/-asm/-data). FUN_00410823 (6102 B) = the WEAPON-ANIM/
  PROJECTILE TICK over the whole 400×0x36 bank 0x4c71f4,
  MissionShell 4 calls/frame (phase 0..3; artillery ticks
  phase-0 only, actor hit-tests odd phases only = 2×/frame).
  Record layout CLOSED: target sel d@+6 (0x29: 0x1000-bit
  robot / 0x2000-bit TRT structure / critter idx via
  FUN_004128ec), tick d@+0xA, class d@+0x2A = LAUNCH DELAY
  (0x24/0x29) OR DETONATION CYCLES (0xF/0x13), arc d@+0x2E =
  ballistic z-vel (gravity −0x100/tick; heading byte &0xFF for
  0x29), trail link d@+0x32. Machines: 2..4 bullets = 2-substep
  lookahead ray (2 tested, 1 committed — anti-tunnel); 5 shell
  w/ per-tick K3 trail; 9..0xB ARTILLERY = scripted-burst
  (durations dword[0x456c78+4·id] = 2/4/7 frames; 7 expanding-
  ring (Δy,Δx) i16 lists 500-sentinel @0x45687c.. via
  PTR[0x456bf0]; each pair = FUN_004244a1 5000-blast + 50%
  RandA K0xB debris; ttl 0x23 life; ttl-24 spotter reveal
  FUN_004245c9 when the owner is player-typed); the 7j.14 K0xC
  set {0xE,0xF,0x13,0x17,0x1A,0x1F} = the BALLISTIC bounce
  family (0xE mortar: full-vertical bounce + 3-cell 5000-blast
  EVERY contact + the 0x4e66b8 0x68-stride smoke-trail ring
  bank {active, ring&7, 8×0xC xyz} appended every 2nd tick;
  0x17 = 3-clone split (rotated damped velocities); 0xF/0x13
  = ttl-cycle submunitions detonating as the 7j.13
  four-quadrant "weapon 0x1A" blast — those 4 sites
  RE-ANCHORED to the detonation path); 0x24 rocket (class =
  launch delay, straight, no gravity, 400 dmg, ttl 101);
  0x29 homing (target lock + ±0x40 4-sector heading-search
  terrain avoidance + z-climb 0x600, ttl 201, target-dead
  fizzle gates on critter state 7 / TRT active). Front doors:
  FUN_0041879d = CRITTER lane (3-row presence-grid prefilter →
  FUN_004190bc mode 2), FUN_0041874c = MP other-robot lane
  (FUN_00418fca mode 2, skips owner) — the 7j.15 "FUN_004190bc
  = panel/preview" hypothesis CORRECTED (critter hit applier).
  RandA = FUN_00402975 re-pinned @0x4116b5. 4 ledger rows
  (tick rewritten + 3 new). Manifest verified. PUSHED 29adbf1.
  Queued: the actor hit-applier internals (FUN_004190bc +
  FUN_00418fca, 7j.23).
- 2026-08-21: P4 7j.21 the 0x425xxx ARRIVAL-PRODUCER family unit
  COMPLETE (worker b67abe61 claim 1, commit 923668e, D69,
  docs-only; 4 × -process runs, dumps ghidra-project/
  exw-arrival{1,2,3}*.txt). FUN_00425da4 (26 234 B, sole caller
  MissionShell boot @0x447b4e) = the ELEVATOR-RIDE STAGER:
  FUN_00402965(0x4dcdb8, 0x654) clears all 45 records, then a
  zone switch ([0x4edd8c] 1..7, jump table 0x425d88) with
  mode/mission gates ([0x4edb88]==2 MP, [0x4edd88] mission)
  stages a contiguous record run from record 0 via
  FIXED-ADDRESS stores (the "register-addressed" gloss was a
  Watcom artifact): active:=1, marker tile xyz ← .PAD slot u16
  words @0x4e44f8+slot·8+2, dest := immediates, +0x20:=−1;
  the countdown is NEVER producer-written — records stage
  DORMANT (7j.11 premise REFUTED). High-water: Z1 0..6, Z2/Z3
  0..16, Z4 0..8, Z5 0..9, Z6 0..14, Z7 0..6; zone 1 worked
  example (SP mission 1): rec0 (8,0x39,2) slot 0, rec1..5
  (8,0x1A,5) slots 10..14, rec6 (0xE,0x20,1) slot 15.
  7j.11 CORRECTED: record layout +4/+8/+0xC = marker x/y/z
  (not two x/y pairs); the walk STOPS at the first inactive
  record (shared epilogue 0x41e176), −1 store only on fire.
  RUNTIME ARMER = the FUN_00433980 ride cases (guard +0x20≠−1,
  rider state@+0x0C:=2, pre-position marker·0x2000+0x1000,
  countdown:=10 — every armed countdown in the program is 10):
  the array = the elevator/teleport RIDE PIPELINE. DRAW PASS
  decoded (0x4065e5..0x4066e3): sprite 0x12E flash at the
  marker, width clamp(11−countdown,0,9), only while armed.
  RECT-LIST BOUNDARY resolved: the MissionShell clear
  (0x4dcae8, 0x2d0) ends EXACTLY at 0x4dcdb8 — no overlap, the
  7j.12 "same producer family" hypothesis refuted; door
  consumers use rect idx 0..0x24; FUN_004223b8 = the door
  open/close stepper (rect {state,x,w,y,h,type}, type-DB
  door-tile stamp/clear type<<4, SFX 0x23/0x24). 0x4c71c4
  anchor refresh: NEGATIVE (spawn-seed only, closed). 7 ledger
  rows + 0a rewritten. Manifest verified. PUSHED 923668e.
  Queued: the weapon-fire family head (FUN_00410823, 7j.22).
