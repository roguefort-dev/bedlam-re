# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The damage unit (RE-EXW-SIM 7f.5/7f.7 + D52 follow-up):
   promote hp/armor to REAL hash-covered sim fields on
   bedlam-core Robot (+ the shield pool +0x88 family if it fits the
   bounded piece), init at spawn = 5000 + 100*battery (the
   dropship-landing formula, 7f.8 — the battery stat needs a core
   seam from the staged loadout), land `MissionSim::apply_damage`
   implementing the decoded FUN_0040e230 core (shield absorb vs hp
   subtract, hit_flash +0x2E, the SP death subset incl. the 5x
   debris FUN_00420608 RandA draws + alive/armor clears + the
   countdown 3 presentation signal), and the armor pad
   charge/bleed (robots() state-1 walk: type-DB +0x18 byte -> +20
   else -10, clamp 3000/0 — needs the under-robot tile type lookup
   + the state-1 semantics check against sec 9 open item 3). The
   bars/strip then read the SIM fields (drop the Sidebar vitals
   staging but keep set_campaign/pickup seams), and the sim pins
   RE-PIN deliberately ONCE with the reason "hp/armor + damage
   land (D52 follow-up)". Bounded: decode anything still missing
   (FUN_00409138 death pass vs FUN_0040e230 interplay, the
   type-DB +0x18 read path) as committed RE notes FIRST; if the
   death/debris family proves too entangled, land ONLY the fields
   + the non-death apply_damage core and keep deaths host-seamed.
## Backlog (not yet started)
- Keyboard latch wiring for the sidebar (F1/F2/F3, keys 1..7,
  MSpace; RE-EXW-INPUT line 95) - blocked on the P2e InputFrame
  button bit-map assignment.
- The dead/hit dither overlay (FUN_00401ae6 + the 0x4e6ed8 512-B
  mask bank; RE-EXW-SIM 7f.4) + the 0x4dc5d0 blink producer.
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence,
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- Mission SFX tier (RE-EXW-SIM sec 9 open item 5; MENU1/MENU2-style
  mixer instruments exist) + the order SFX 0x2A armer click.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items 1/2/4: type-DB tail producers
  (+0x18/+0x1a/+0x1b/+0x1c), the u32[0x4dd444] remap tables +
  u32[0x456ca8] anim sequence + the water flag producer (needed
  before the 0x12d/0x12e/0x12f flush remaps can leave water-off
  semantics), BIN u32[bank+0] header word. NOTE: u32[0x4dd444] was
  CLOSED 2026-08-21 by 7e.4 (the PALTRAN ramps, slot 0 NULLed).
- MISSIONVIEW sec 5d tail (robots only are wired): platform loop
  (0x4eb638, bank DAT_0046af54), effects loop (0x4cf638 - the
  FUN_00401e39 draw_IMG codec family, a DIFFERENT .BIN sprite layout
  per RESEARCH-8STREET), ROBNUMS name plates, Shield/Variant bank
  staging (nodes enqueue, flush skips while unstaged).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity, robots()
  extra-phase semantics + state-1 producers. NOTE 7d: FUN_00440e45
  is THE SHOP (WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK +
  SOUND\MIDI\SHOP; the weapon-table writer family - see 7d.2).
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
  86a788ff93bd78a5), sim pins UNCHANGED
  (36ddc86345c8351c/f35db41f0efb858d/64ef1ddbc65cba47). Pushed.
- 2026-08-21: P4 map-overlay family COMPLETE (worker 6d689cfd
  claim 1, commits 78b2506 + 9cb8fbe + 59af1b3, unit finished
  across an interrupted predecessor run): RE-EXW-SIM 7e =
  FUN_004089b1 fully decoded (TABLE.BIN backdrop, LNK-word
  territory stamps x MIN masks x MAPTRAN ramps, the
  FUN_00408dcc 11x11 ring variants, GENERAL 0x55/0x56 markers,
  the 0x408c94 order-target loop, the NON-RETURNING tail - the
  sidebar passes are skipped, not else-branched) + MAPTRAN/
  PALTRAN loaders (MISSIONVIEW sec 8 u32[0x4dd444] producer
  CLOSED) + the toggle family (strip 5-frame lockout 0x4eb8dc,
  bit 0x4edba0, FUN_00401107 480x480 present, click swallow
  0x40b868, chrome 0x8f/0x5f/0x5e). ENGINE: bedlam-render
  MapOverlay + the 22-asset mission chain tail (TABLE/MAPTRAN0-7/
  zone .MIN) + MissionScene toggle strip, overlay frame (stale
  sidebar half), markers, chrome, ring stamps. PAD/order markers
  0x57..0x59 unwired (never-invent). Gates: 455 tests, fmt/clippy
  clean, smoke two-run identical + hashes equal to prior commit,
  sim pins UNCHANGED (36ddc86345c8351c/f35db41f0efb858d), frame
  pins moved once (chrome) + new overlay pins
  (f47217a154bf93c9/64ef1ddbc65cba47), MANIFEST verified.
  Pushed.
- 2026-08-21: P4 weapon table COMPLETE (worker 4b75846d claim 1,
  commits 5af9a70 + 1c7b387, D51): RE-EXW-SIM 7d REFUTES the
  TABLE.BIN hypothesis - TABLE.BIN is the map-overlay backdrop
  bank (draw_IMG image 0, sole reader FUN_004089b1); the 0x4de664
  table is .bss session state (shop FUN_00440e45 / save-load / MP
  lobby writers, no loader); player TYPE 0x4edb90 = 0 all SP
  (GameMain 0x41c34c); fresh campaign = money 4000 + EMPTY
  loadout (shop before every mission); the FUN_00420260 name
  switch pinned exactly (39 strings 0x4589DD..0x458C11, PE
  bytes). ENGINE: host-staged per-robot (name_idx, ammo) loadout
  seam (mission_mut + set_weapon_loadout) with the faithful EMPTY
  default; the all-7 availability + set_order_availability seam
  REMOVED; click gate = the ammo word (6c.3); row TEXT wired
  (names + "%04i" via SMLFONT at 0x1ED/0x25C, 0x5B+14i, 0x24);
  ui_bank RLE codec fixed to the asm (bit14-in-literal EOL - the
  shipped sprites are one 0x4000|w word per row - + verbatim
  transp). Corpus gate: frame pins regenerated once (spawn
  9f20732f29a5baf2, walk 27494d6ab505bcf3) + new armed pin
  51ebd515bc638e81; sim pins 36ddc86345c8351c/f35db41f0efb858d
  UNCHANGED. 441 tests green, fmt/clippy clean, smoke + parity
  byte-identical at the baselines, MANIFEST verified. Pushed.
