# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The weapon-fire family THIRD HOP — FUN_00419aff (381 B,
   28 callers): the per-weapon STAT lookup feeding every damage
   argument (ids seen: 1/2..5/9..0x29 weapon-anim kinds +
   0x65..0x68 projectile types; base field 0x46cbf8 per 7j.13
   census) — pin its table layout (which id → which damage/
   field), PLUS the 0x4cccf8 terrain-structure array PRODUCER
   census (who stages {active,hp,x,y,z} there — xrefs to
   0x4cccf8/0x4ccd08/0x4ccd14 + the count writer 0x46ccd4;
   mission-load stager suspected). Bounded: FUN_00419aff full +
   producer xref census only. NOTE (7j.14): PUSH RETRY — if
   origin/main is behind, push the pending 7j.13/7j.14 commits
   first (secret service was down again at close-out; retry
   libsecret wake via any push attempt before work).
## Backlog (not yet started)
- The 0x425xxx arrival-producer family (FUN_0042034c's 45-record
  staging at 0x425daf/0x426079/0x42688c + the register-addressed
  countdown writes + the record draw pass 0x4065f8..0x4066a3) —
  the delayed-arrival scheduler is decoded (7j.11 item 1), its
  producers are not. NOTE 7j.12: the 45x0x10 rectangle list at
  0x4dcae8 (the type-DB tail stamper input) sits IMMEDIATELY
  before the arrival array 0x4dcdb8 — same producer family is
  likely.
- The weapon-fire family REMAINDER (first hop done 7j.13 —
  FUN_0041a894 head + 17-site census + the object type table
  0x4dedf2/0x4E/282 pinned; SECOND HOP done 7j.14 — FUN_0041bc1c
  terrain-structure resolver + FUN_0041eaa1 height probe + both
  disburser heads): after the THIRD HOP (the Now item), the
  FUN_00410823 weapon-anim machine internals (the 0x4c71xx
  record family), the destroy-tail debris-kind map
  (which id-table type@+0xE stages which kinds — the 7j.11 sites
  0x41ace7..0x41b67a), FUN_00412f34/FUN_00417e2f, the
  [0x4edd60] height-bank family, the projectile-record z
  encoding (7j.14 census open), and the 160-vs-0xA8 stride
  anomaly at 0x4c69e4 (FUN_0040fe93). Unlocking the tail
  re-opens the water-splash producers (7j.10) and 17 of 20
  debris kinds (7j.11) for any future corpus seam.
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
  verified. Push STILL blocked (secret service down, retried
  twice) — commits 4448a77, 2064e18, 7b9ce05 local; RETRY
  FIRST. Queued: the family THIRD HOP (FUN_00419aff + the
  0x4cccf8 producer census).
- 2026-08-21: P4 7j.13 FUN_0041a894 weapon-impact ray head
  FIRST HOP unit COMPLETE (worker b7f866b6 claim 1, commit
  4448a77, D61, docs-only): FUN_0041a894 = the PER-TILE
  WEAPON-IMPACT OBJECT RESOLVER (eax x Q13, edx y Q13, ecx
  chain ctr, ebx damage, [stack] score flag) — NO walk: grid
  word 0/0x7d2/0x7d3 → pass-through ret 0; 0x7d4 →
  FUN_00422693; n>0 → rec n−1 hp−=damage, destroy (flags 0x40)
  → tail + ret 1. The tail: trigger producers FUN_00422e0a/
  FUN_00422600, the 7j.11 debris kinds + 4× splash loop
  (FUN_0041bd78/FUN_00424355, RandA jitter), score award
  (type 0xb → +10) gated by the stack flag, and FOUR perimeter
  CHAIN WALKS (chainable id-table word@+0xC ≠ 0 → recurse
  damage 1000, RandA&3 → ctr++). The RAY = the callers (17-site
  census): projectile tick FUN_00412010 (50 rec @0x4cc654
  stride 0x22, ballistic, probe FUN_0041eaa1, damage
  FUN_00419aff(0x65/0x66)), fire controller FUN_00410823 (8
  sites, weapons 5/0x1a×4/0x24/0x29), tile-0x62 trap pair
  FUN_0040fe93/FUN_0040ff92 (damage 100, 5× k12; NOTE 160-B
  stride anomaly at 0x4c69e4), script blast FUN_004244a1
  (damage 5000). The 0x41a84f stamp loop = FUN_0041a7f0
  (footprint stamper) from the FUN_0041a4f8 mission-load pass
  — which parses the OBJECT TYPE TABLE 0x4dedf2/0x4E/282
  (W/H/D, hp, chain, type, jitter words, 4 scratch banks).
  ERRATUM 7j.12 item 1: the stamp loop is NOT weapon fire's.
  No engine change (D61). Manifest verified. Push blocked at
  close-out (secret service down) — commits 4448a77 + the
  state commit are local; RETRY FIRST. Queued: the family
  SECOND HOP (FUN_0041bc1c).
- 2026-08-21: P4 7j.12 FUN_00422693 platform/destructible family
  decode unit COMPLETE (worker 5aa2d164 claim 1, commits f759b3a
  + follow-up, D60, docs-only): the gate banks PINNED —
  word[0x460dfa+2·tile] is the tile OBJECT-WORD GRID (0 empty /
  0x7d2 hazard / 0x7d3 phase-clamp / 0x7d4 platform / n>0 =
  destructible object rec n−1 @0x46cbf4 stride 0x14
  {x,y,z,id,flags,hp}); word[0x465daa+2·tile] = PLATFORM
  STRENGTH. FUN_00422693 = damage entry (weaken: strength−=
  damage + scorch+4 via the NEW increment writer FUN_0042223c +
  conditional 8-tile ring spread; destroy: FUN_0042394a
  (x,y,z,0,0) clears the water z-word @0x422750 + both banks +
  5× kind-7 debris @0x4227b9); FUN_00422832/8ce = spread ring
  (writes 0x7d4 @0x422a61 + strength @0x422a73 + water z-word
  create @0x422a54; needs empty z-word + planeA 0 + planeB 1 +
  no robot on the SE 2×2); FUN_00422a9c = the 1/32 creep tick
  (water-ray walk, FUN_00422832(…,199), site latch
  0x4dc5c8/cc); FUN_00422f18 = the 0x7d2/0x7d3 STAMPER (7g.5
  producer CLOSED — per-zone ranges 0x454a20/0x454a3c);
  FUN_00422fd1 = type-DB +0x19/+0x1a stamper from the 45×0x10
  rect list @0x4dcae8 (MISSIONVIEW 8.1 partially closed;
  +0x1b/+0x1c still open); FUN_00422cc2 = 32-timer delayed-
  trigger tick (expiry → SFX 0x4239ef(0x22,3), flags 0x40,
  z-plane clear, floor-word write via the fast z-writer
  FUN_0041bd54 — second 0x454a90 context; the 7h.3 PICKUP
  producer stays open). No engine change (D60 — callers all off
  the corpus path). Queued: the weapon-fire family first hop.
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
