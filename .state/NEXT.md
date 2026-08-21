# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4.2] The DIFFERENTIAL-HARNESS DESIGN DOC (PLAN sec 6 P4.2,
   budgeted ~2 weeks): write docs/DESIGN-DIFFHARNESS.md — the
   architecture for DOSBox-X memory-watches + scripted input
   injection -> per-frame original-state dumps diffed against the
   Rust engine state. The render tail is now fully RE'd (7j.28
   closed the last consumer block), so the doc arbitrates the
   accumulated open hypotheses + harness obligations in one place:
   the pod-descent stagger (w@+0x2C = 1+k·(2000−m·1000/27),
   descent ≈41 frames, pod phase 2 = one tick, release = state 6),
   weapon fire via injected COMMAND records (FUN_00449c94/
   0x4dd4a0) or order dispatch — never raw input (7j.22), the
   destroy family end-to-end (7j.25), the mid-flight draw blit
   sequences (WEAPONS/SHRIKE/REAPER/SMOKE banks, 7j.28), the
   debris 2k start-delay + blink-cursor-from-spawn questions, the
   7j.9 five-ring overlap last-write-wins read, arm extraction via
   a scripted .PAD step-on (not a click, 7j.20), and the
   corpus-off producers that land naturally with the harness
   (debris-stager widening, SFX families, per-zone case tables).
   Bounded: DESIGN DOC ONLY (no harness code this unit); anchor
   every watched address to its ledger row; end with a build-order
   ticket list (watch points first, runner, differ, gates).

## Backlog (not yet started)
- CLOSED by 7j.27: the DROPSHIP ring producers (writer census,
   animator map, 7×5 grid correction, latch census, the 0x4c71f4
   pass head). CLOSED by 7j.26: the [0x4ede24]/[0x4ede28] "7×7 screen-address
  table" question — it is the terrain RESTAMP list (count + 3-dword
  {dest row, tile-x, tile-y} records, blitted via FUN_00401471;
  writer FUN_00440a2d = the scroll/camera restamp stager, confirming
  the hypothesis). REMAINS open slim: FUN_00440dc2's own identity
  (reads the backbuffer [0x4ede18] @0x440e02; the 7j.16
  TOT-materializer caller). CLOSED by 7j.17: the [0x4edd60] height-bank family and the
  projectile z-encoding census. CLOSED by 7j.24: the critter
  death-handler family. CLOSED by 7j.25: the destroy-tail
  effect-entry map + the 160-vs-0xA8 stride anomaly + the
  .POS/.BDG loaders + the .BDG grammar (FORMATS §12/§16).
  OPEN small: projectile type 0x69 vs the FUN_00419aff
  damage table (7j.17/7j.18 — low priority).
- The per-zone FUN_00433980 case table (≈28 pad ids × 7 zones,
  beyond the §7j.19 head decode; §7j.20 item 2 gives the ~25
  extraction-pad (zone,slot) pairs and §7j.21 the record
  high-water marks + the record↔pad arm mapping task) + the
  FUN_00424a6f message string table — mechanical, decode per
  zone only when P4.2 needs it.
- The 0x4787c4/0x47879c hot-rect record (renderer FUN_00403938
  writes it @0x403c93, count [0x46ccd8]; picker reads
  center@+8/+0xC + w@+0x14, order dispatcher reads corner@+0/+4 +
  z@+0x10 + type@+0x1C — [hypothesis] one 0x20-stride record with
  both views). Anchors the click-target rect semantics.
- The .MOFO loader (the last of the FUN_00416458 sibling
  loaders @0x457a4c; .NME/.TRT/.POS/.BDG all CLOSED —
  7j.15/7j.18/7j.25) + the .BLD record walk (names/graphics
  side; FORMATS §17 — the 201-B/64-B-extension hypothesis
  still unanchored) + the .BDG template-bank plane↔mirror-word
  mapping (which bank feeds which restore word — 7j.25 pinned
  banks @+0x46/+0x4A = TOT-mirror/seen+DAT; @+0x3E/+0x42
  readers still open).
- The debris-stager ENGINE widening beyond kind 5 (fed by the
  7j.11 20-kind table + the 11 seq tables): model the k2/k8
  single-center scorch (values 3/4), the k1/k20 shared-tail
  ring, and the +0x20 physics classes (0/1/2/3/6 ->
  FUN_0040de9c) — all producers now DECODED (7j.22 weapon
  family, 7j.24 critter deaths, 7j.25 destroy tail) but all
  sit OFF the corpus path (nothing fires/dies/gets destroyed
  in the gates); lands with the P4.2 harness.
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
  NOTE 7j.25: the destroy-thud pair 0x4edfb8/0x4edfbc =
  DEADMAN1/DEADMAN2.RAW (loader 0x43a29b..0x43a368 strings —
  a full bank-name walk is a bounded SFX-unit add-on).
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
- MISSIONVIEW sec 5d tail notes: ROBNUMS name plates,
  Shield/Variant bank staging (nodes enqueue, flush skips while
  unstaged). The debris physics/collision FUN_0040de9c (7j.7
  head decode) lives here too (+ the 0x454510+ physics-param
  dword table census-noted in 7j.11 item 5; 3 octile reads per
  7j.16; reads BOTH the critter and POI counts — collision
  family).
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
  the robots frozen in pods (7j.27: descent ≈41 frames, pod phase
  2 = one tick, release = state 6) — and arm extraction via a scripted
  .PAD step-on, not a click. NOTE 7j.22: weapon fire needs
  injected COMMAND records (FUN_00449c94/0x4dd4a0) or order
  dispatch, not raw input — the fire family is fully anchored
  for it (per-type cadences + damage tables). NOTE 7j.25: the
  destroy family is now fully decoded end-to-end (resolver →
  restore → 5-effect loop → chain walks), ready for the harness.
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
