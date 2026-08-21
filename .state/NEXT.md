# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The dead/hit dither overlay unit (RE-EXW-SIM 7f.4 + the
   7g.8 decay follow-through): decode the FUN_00401ae6 blit +
   the 0x4e6ed8 512-B mask bank (the dither codec — how the mask
   selects/blends pixels over the 48x48 portrait) as committed RE
   notes FIRST, then wire the portrait dither in the sidebar
   portrait pass reading the now-real sim hit_flash field (the
   7g.8 decay already runs per frame; clamp>5 then -1 per alive
   hp>=1 frame). Keep the 0x4dc5d0 blink producer OUT of this
   unit unless trivially adjacent (it is the FUN_00422038 slot
   family the pickup tails also stage at — 7h.2 notes the row
   layout). Bounded: the codec decode is the RE piece; frame
   pins may move ONCE (the dither draws on the portrait plane)
   — re-pin with the reason, sim pins must NOT move.
## Backlog (not yet started)
- The 0x4dc5d0 blink/effect-row producer family (FUN_00422038
  slot alloc + the 16-B rows; the DamageOutcome debris rows and
  the 7h.2 PickupOutcome effect ids 1/6/7/0xE are its staged
  inputs) + the FUN_00420608 128-slot debris stager consumer.
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
- 2026-08-21: P4 damage unit COMPLETE (worker 416ca029 claim 1,
  commit d9032d9, D53; unit finished across an interrupted
  predecessor run that committed the 7g pre-decode 5e10768 + the
  implementation WIP): RE-EXW-SIM 7g + ENGINE: hp/armor/hit_flash/
  alarm/alarm_ctr/shield/shield_charges/shield_boost/battery/
  armor_pool/kind/death_flag promoted to hash-covered sim Robot
  fields; spawn hp = 5000+100*battery (set_battery seam); apply_damage
  = the FUN_0040e230 SP core (state-2/alive gates, ordered->shield
  0x20, auto-shield idle, alarm trip, absorb vs hit_flash-then-hp,
  SP death subset + 5 debris x 2 shared RandA); phase-0 pre-walk
  (alarm/ctr decay, shield -2, booster 10000/150); phase-1 armor
  pad +20/bleed -10 clamp 3000/0 (set_armor_pads seam, corpus pads
  all-zero); hit_flash portrait decay. Game side dropped the D52
  Vitals staging (bars/portraits read the sim fields; battery lands
  through set_weapon_loadout; death stages sidebar redraw 3).
  Gates: sim pins RE-PINNED ONCE (1cc7b8e125165988 spawn,
  5b9c2fd5d85f9adc post-arm, d8eeb3e608af0be4 arrival,
  0bf4fb534d6b3bd5 click, 78a16ba63607d197 overlay); frame pins
  byte-identical (9ecd7691d388bbfa/333d128dc812d547/
  1504c600819e724c/86a788ff93bd78a5); 41 suites/465 tests green
  (8 new), fmt/clippy clean, smoke two-run byte-identical at the
  recorded baselines, MANIFEST verified. Pushed.
- 2026-08-21: P4 sidebar bars + score strip COMPLETE (worker
  36c9e956 claim 1, commits a11e468 + 2035395 + 3f7fad7, D52):
  RE-EXW-SIM 7f = the vitals family decoded (FUN_0040807f bars
  exact incl. the word@+0x30 armor correction, FUN_004085ce strip
  exact x tables + unsigned/signed splits, the CORRECTED tail
  order, FUN_004072bf gates + the +0x2E hit-flash correction,
  FUN_0040e230 damage, FUN_0040eba0 pickup cases, FUN_004100b7
  armor pads + the -10 bleed, the landing hp init, the score/
  money + NUMBERS.BIN census). ENGINE: bars + strip wired from
  HOST-STAGED vitals (D52 — hp 5000+100*battery via the BATTERY
  PACK group, armor 0 with the faithful empty-bar draw) + campaign
  session state (0/4000) + the case-4 pickup seam (two rand_a
  draws, countdown 2); NUMBERS.BIN the 23rd chain asset; portrait
  hp>=1 gate; bars+strip+pickup unit tests. Gates: 41 suites
  green, fmt/clippy clean, smoke two-run byte-identical, MANIFEST
  verified; frame pins regenerated once (spawn 9ecd7691d388bbfa,
  walk 333d128dc812d547, overlay 1504c600819e724c, armed
  86a788ff93bd78a5), sim pins then UNCHANGED. Pushed.
