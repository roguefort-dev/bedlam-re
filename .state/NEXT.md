# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The MISSIONVIEW §5d DRAW TAILS (7j.26), both
   producer-anchored now: (a) the 0x4cf638 EFFECTS LOOP —
   the draw pass over the 80×0x1E bank (7j.25 landed its
   FIRST producer FUN_0041a225 = destroy-tail cases 1/8:
   jittered Q13 particles, ttl 6000+, active word@+0x18 =
   FUN_0041ec59(3)); the consumer is the FUN_00401e39
   draw_IMG codec family (a DIFFERENT .BIN sprite layout per
   RESEARCH-8STREET) — decode the pass + pin
   FUN_0041ec59(3)'s identity; (b) the 0x4eb638 PLATFORM
   LOOP (producer CLOSED 7j.24 = FUN_0042382c robot-death
   blast records 32×0x14; bank DAT_0046af54) — the last
   undecoded §5d consumer. Both are bounded single-pass
   decodes inside the FUN_00403938 render tail.

## Backlog (not yet started)
- CLOSED by 7j.17: the [0x4edd60] height-bank family and the
  projectile z-encoding census. CLOSED by 7j.24: the critter
  death-handler family. CLOSED by 7j.25: the destroy-tail
  effect-entry map + the 160-vs-0xA8 stride anomaly + the
  .POS/.BDG loaders + the .BDG grammar (FORMATS §12/§16).
  OPEN small: projectile type 0x69 vs the FUN_00419aff
  damage table (7j.17/7j.18 — low priority); the trail-ring
  DRAW pass consuming the 0x4e66b8 bank (7j.22/7j.23:
  FUN_00403938 reads the record link @0x404464 — bounded
  decode when needed).
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
  the robots frozen in pods — and arm extraction via a scripted
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
- 2026-08-21: P4 7j.25 the WEAPON-FIRE FAMILY TAIL unit COMPLETE
  (worker 399aeff4 claim 1, commits 3bfd400 + 1016123 + b4950a8
  + 6183be5, D73, docs-only; dump ghidra-project/
  exw-destroytail-asm.txt + full-objdump census). The
  FUN_0041a894 destroy tail decoded WHOLE: TERRAIN RESTORE
  first (footprint W×H×D loop: TOT-mirror z-words ← template
  bank@type+0x46, seen + DAT volume ← bank@type+0x4A, linear
  (z·H+i)·W+j), then the FIVE-EFFECT loop over the type-table
  entries @+0x16+8m — selector word 1..9 → jump table
  0x41a870 (idx sel−1): 1→k14+FUN_0041a225+5 splashes,
  2/3/4/5→k18/k17/k16/k19 single gibs at sub-tile bearings
  (+0x10,+0x30)/(+0x30,+0x10)/(+0x20,−0x10)/(−0x20,0),
  6/7→k10+(+0x10,+0x20)/(+0x20,+0x10)+DEADMAN1/2 SFX (banks
  0x4edfb8/0x4edfbc = SOUND\SFX\DEADMAN1/2.RAW, loader
  0x43a29b, shared with the 7j.24 crush dispatcher),
  8→k14×25 water-level demolition shower (RandA&7−3 jitter,
  delay ctr+2m+i>>3), 9→k20+3×3 splash ring (delay
  ctr+2+RandA&3); payload words = tile offsets off the
  0x46cbf4 record; stager stack = (delay, param=score|−1),
  callee ret 8. GER gate REFINED (skips the whole tail for
  type 0xb, record still dies). FUN_0041a225 = FIRST producer
  of the MISSIONVIEW §5d effects bank 0x4cf638 (80×0x1E,
  free-slot word@+0x18, allocator FUN_0041a4cc, jittered Q13
  particles ttl 6000+). The 160-vs-0xA8 stride anomaly CLOSED
  (21·idx·8 = 0xA8 canonical — 7j.13 census slip); trap-pair
  callers pinned (robots()@0x40bc44 + critter FUN_00412f34@
  0x413fd7). BONUS: FUN_0041a4f8 = the .POS loader (2000×0x10
  → the 0x46cbf4 object array) + the .BDG loader (the
  0x4dedf2 type table) — .BDG grammar CLOSED (no header, ≤282
  variable records, 4 on-disk template banks; census 37/37
  EOF-exact, exactly 282 recs/file, selectors ONLY 1..9
  ×11098/1490/1385/402/330/304/316/178/56); FORMATS §12/§16/
  §19 rewritten. 4 new + 2 rewritten ledger rows. Manifest
  verified. PUSHED 6183be5. Queued: the MISSIONVIEW §5d draw
  tails (7j.26).
- 2026-08-21: P4 7j.24 the CRITTER DEATH-HANDLER family unit
  COMPLETE (worker 0f986419 claim 1, commit 3819586, D72,
  docs-only; dumps ghidra-project/exw-dead1..5*.txt — 1..3
  adopted from predecessor ad591680's session tail, 4/5 +
  objdump spot-checks this unit). The six per-kind handlers
  decoded: k1 FUN_00418835 (state 7+presence 0, 1× k1 debris,
  +30), k2 FUN_004188d0 (state 7+presence 0, 1× k0xD, +50),
  k3 FUN_00418aa6 (1× k7 + 3× k6 delays 0/2/4 + SFX trio
  FUN_00421f4c, +500, tail call = NOP stub FUN_00418a9f), k4
  FUN_00418ca4(+weapon) (w@+0x02 := 1, hp 0, state 6, timer 6,
  1× k7; weapon {0x24,0x29,0xC} → 3× k7 + 8 rows, +75), k5/6
  FUN_00418e26(+weapon) (sub-timer 0; weapon-gated 3× k7 + 12
  rows, +150), k7 FUN_0041896c (3 falling gibs + 1× k0xD, SFX
  FUN_0043a48e(0x4edff8,…,3), w@+0x78 := 1, +1000). BOUNTY
  GATE: attacker ≠ −1 ∧ robot[killer].type == [0x4edb90] →
  score [0x4dd40c] += N + DAT_0046ccf0 := 2 (score-strip
  refresh). SECOND DISPATCHER: FUN_0040dce0 = debris crush
  (sole caller FUN_0040de9c; k4 weapon 0, k5/6 weapon 0x24,
  k5/6 state {5,6} absorbed; knock via FUN_00412998 +
  FUN_0041e9a2). FUN_0041a14f/FUN_0041a494 = the 0x4cec38
  effect-row spawner + age-LRU allocator (w@+0 = AGE word —
  7j.23 gloss corrected). 7j.17 CORRECTED: death handlers
  never call FUN_00424355 (splashes = controller landing/
  suicide paths only). ADDENDUM: FUN_0040e230 SP tail
  CONFIRMED + MP respawn completed (suicide gate/clamps, MRK
  reposition, 7-slot weapon + 2-entry equipment re-copy);
  FUN_0042382c = FIRST producer of the 0x4eb638 platform bank
  (claim-byte gated, 32×0x14, LRU). 8 new + 2 rewritten
  ledger rows. Manifest verified. PUSHED 3819586.
  Queued: the weapon-fire family TAIL (7j.25).
- 2026-08-21: P4 7j.23 the ACTOR HIT APPLIERS unit COMPLETE
  (worker ad591680 claim 1, commit 45329e9, D71, docs-only;
  4 × -process runs, dumps ghidra-project/exw-hitters{,2,3,4}
  *.txt + exw-hitters-scan.txt via the NEW StoreScan.java
  operand scanner). FUN_004190bc = the CRITTER hit applier:
  presence w@+0x24, KIND switch w@+0x00 (the 7j.18 .NME
  section states {2,1,5,4,3,6,7} = cases 1..7), attacker
  w@+0x04, hp s16 w@+0x06, state w@+0x0C (6/7/0xB immune for
  k3..7), hit-flash w@+0x7C, impact x/y +0x1C/+0x20; mode 2 =
  octile<0x20 + z-box (k1/4 cell-unit coords, others Q13; z
  0x20, k3 0x24, k7 0x40), mode 1 = x/y only; damage =
  FUN_00419aff(weapon) — the 7j.22 "per-critter" gloss
  CORRECTED (per-WEAPON); 6 per-kind death handlers
  (FUN_00418835/d0/aa6/ca4/e26/96c); k4/5/6 survivors 25%
  knockback FUN_0041a028 (2nd spawner of the 0x4cec38 effect
  rows, heading away-from-shooter ±jitter) + impact SFX
  FUN_00421fc2 (RandB%3 → banks 0x4edf7c/80/84); k7 does its
  own in-record knock (vx/vy w@+0x74/+0x76). FUN_0041ebf8 =
  octile distance. FUN_00418fca = robot box-test applier (|dx|
  |dy|<0x20, |dz|<0x30) → FUN_0040e230 [head-decoded: shield
  d@+0x88 absorb, hp d@+0x78, alarm d@+0xA4→SFX 0x10..12,
  tier SFX 0x2B/0x13/0x16 per 5000+100·variant, MP frags
  0x4ebaa8 0xC-stride] + hp clamp. TRAIL ALLOCATOR CLOSED:
  FUN_00412a4a (20 slots @0x4e66b8, first active==0), writer
  FUN_0040a9ff (mortar spawner: slot weapon w@+0x36+8k==0xE →
  link := slot, active := 1, ring zeroed; else link := 0;
  ballistics /8-unit ×2 at the order target, ttl 0x32, arc
  0x500). Third caller of the critter applier found
  (FUN_00403938, weapon 0xC=5000 blast, owner −1). 7 new +
  2 rewritten ledger rows. Manifest verified. PUSHED 45329e9.
  Queued: the critter death-handler family (7j.24).
