# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The FUN_00422693 platform/destructible-terrain family
   decode (415 B @0x422693 + the 0x4227xx..0x422fxx kin): the
   0x465daa/0x460dfa per-tile word gate banks (writers incl.
   0x4227d5/0x422a73/0x422b0d, the 0x7d2/0x7d3/0x7d4 tile
   words), the two FUN_0042394a calls @0x422750/0x422a54 (the
   ONLY non-splash, non-arrival z-structure writers), and the
   k7 debris staging @0x4227b9 — surfaced by 7j.11; feeds the
   gate-bank semantics the 7h pickup floor-word swap + the
   D53-noted 0x7d2/0x7d3 tile words both need. Keep it
   bounded: this family only, census the rest.
## Backlog (not yet started)
- The 0x425xxx arrival-producer family (FUN_0042034c's 45-record
  staging at 0x425daf/0x426079/0x42688c + the register-addressed
  countdown writes + the record draw pass 0x4065f8..0x4066a3) —
  the delayed-arrival scheduler is decoded (7j.11 item 1), its
  producers are not.
- The weapon-fire family decode (FUN_0041a894, 5000 B, 17 callers
  + FUN_00412f34/FUN_00417e2f/FUN_0041bc1c): the 11 splash-stager
  call sites of 7j.10 + the debris co-staging + the projectile/
  impact model. Unlocking this re-opens the water-splash event
  system (the 250-record tick decoded in 7j.10, currently unwired
  for want of a corpus producer) AND stages 17 of the 20 debris
  kinds (the 7j.11 census: k1..k4/k6/k7/k8..k20 producers all
  live here or in the platform family).
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
- 2026-08-21: P4 7j.11 FUN_00420608 kind census unit COMPLETE
  (worker 804e8c9d claim 1, commit 199fe32, D59, docs-only):
  the 0x4203a5 queue NOTE answered — the FUN_0042394a call is
  in FUN_0042034c (the DELAYED-ARRIVAL SCHEDULER, epilogue
  0x448076, 45 records @0x4dcdb8 stride 0x24: countdown w/ the
  0xa SFX, the 0x465daa word-gate, the first-water-level CLEAR,
  the robot teleport + get_z_pos re-settle + the 8-word z fill),
  NOT a debris kind; the stager body has ZERO type-DB refs and
  ZERO z-writer calls. The 20-kind table pinned (11 seq tables
  0x454424..0x454510, physics classes 0/1/2/3/6, inits 0x40/
  0x20, the two arrival-SFX helpers + the k11 LCG gate).
  CORRECTION: kinds 1/13/14/15 DO write the nine ring (jmp
  into the k20 tail); kinds 2/8 write ONE center tile (3/4);
  only 7/10/16..19 ring-free. Full 47-site caller census: only
  k5 (death, engine-landed) is corpus-reachable today. No
  engine change (D59); manifest verified; pushed. Queued: the
  FUN_00422693 platform/destructible family.
- 2026-08-21: P4 7j.10 FUN_00424051 decode unit COMPLETE (worker
  89d34b53 claim 1, commits 782a25b + 54c4109 + d08b51f, D58):
  RE-EXW-SIM 7j.10 = FUN_00424051 IDENTIFIED as the per-frame
  mission-epilogue tick (call 0x447ff0, right after the debris
  tick): (1) the GLOBAL +0x18 FADE — every nonzero armor-pad/
  scorch byte decays 1/frame unconditionally, so the D57 ring is
  TRANSIENT (≤ value frames) and permanent map pads cannot exist
  (MISSIONVIEW 8.1 +0x18 question FULLY closed); (2) the
  WATER-SPLASH EVENT TICK — 250 records @0x4e9778 {x,y,z,delay,
  age}: weapon impacts (11 stager callers, the FUN_0041a894
  family, one co-staging debris) stamp the zone water sprite at
  the first free z (FUN_0041bd78), fall through empty levels on
  odd frames (g_frame_count&1), absorb into water below, re-stamp
  base+0x16 @age 40, dry up @age≥47, scorching the tile every
  tick (the 7j.9 item-5 re-roll writes). FUN_0042394a = the
  z-structure writer (TOT z-word + seen + DAT volume — the
  map-edit primitive); FUN_0041eb28 = the DAT volume read (NOT
  visibility). ENGINE: the fade landed at the advance_frame tail
  (corpus-safe — armor_pads has no corpus producer, set_armor_pads
  is test-only); the two permanent-pad tests now stage value 7; +1
  unit test (decay, single-charge value-1, full ring fade). The
  splash system stays UNWIRED (no corpus producer — documented,
  re-open with the weapon family). Gates: pins UNMOVED, 41 suites
  green, fmt/clippy clean, smoke two-run byte-identical AT the
  baselines (scene 696adb1cd110e062, parity cce30c983b97b16d,
  audio 110400/158092), MANIFEST verified. Pushed. Queued: the
  FUN_00420608 remaining-kind census.
