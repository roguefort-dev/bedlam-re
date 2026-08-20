# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] mission render half 2: ENTITIES on the frame. Inputs now
   complete: the FUN_00403938 entity loops (robots at 0x4c69e4 with
   the iso projection sx = ((cx&31)-(cy&31)+32)&63 + (wx-cx>>8) -
   (wy-cy>>8) + 0x110, sy = shake + ((dx+dy)>>1) + 0x10c + ((cx&31)+
   (cy&31))>>1 - z, clip 0x23f), the sprite-list enqueue FUN_0040798e
   (48-B nodes into the 36x36x8 bucket grid at 0x46cdbc, sorted by
   wx+wy) and the flush FUN_0040179b (u16-RLE + mode remaps) are
   decoded in docs/RE-EXW-MISSIONVIEW.md sec 3/5 but NOT yet wired.
   Bounded unit: implement enqueue+flush in bedlam-render
   mission_view.rs (the robot records come from bedlam-core
   MissionSim), extend the corpus gate to draw the ZONEA/MISSION1
   spawned robot + the walking second robot onto the pinned frame
   (new hash pin; DANTE.BIN robot sprites staged by FUN_0041df10),
   keep terrain hash 90a9e929eea24ced as a no-entities regression
   pin.

## Backlog (not yet started)
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence,
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- RE-EXW-MISSIONVIEW sec 8 open items: type-DB tail producers
  (+0x18/+0x1a/+0x1b/+0x1c), the u32[0x4dd444] remap tables +
  u32[0x456ca8] anim sequence, BIN u32[bank+0] header word.
- RE-EXW-SIM sec 9 open items 2-5: FUN_00440e45 identity, robots()
  extra-phase semantics + state-1 producers, sidebar order buttons,
  the 0x62-stride robot-type stats table.
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
- 2026-08-21: P4 mission RENDER half COMPLETE (worker b9aaaa38 claim 1,
  commits 02363f6 + 889d6b0): RE-EXW-MISSIONVIEW.md decoded the full
  viewport chain (init_tiles cache + TOT->typeDB mirror; LNK = per-frame
  tile animation link; BIN = MISSION{A..G}.BIN terrain bank; blit codec
  FUN_00401471 u16/u8 RLE; terrain loop FUN_00403938 8-layer walk +
  seen-chase + edges FUN_00408030; sprite-list FUN_0040798e/0179b;
  present FUN_00401107 480x480 window). Engine bedlam-render
  mission_view.rs + corpus gate mission_view_gate.rs: cache geometry
  (anchor 17, 467 cells), deck mirror/seen, codec, one-LNK-step walk
  (visible-only, cap 0x59b00 faithful), frame hash pinned
  90a9e929eea24ced at camera (0,0) frame 0, two-run identity, edge
  stream isolation. 407 tests green, fmt+clippy clean, release ok,
  MANIFEST verified, pushed.
- 2026-08-21: P4 slice TAIL COMPLETE (worker d8c46c88 claim 1, commits
  5381bea + c4f615a + 055879e): RE amendment 7c decoded the mission
  file-load + table-build pass (load_mission@0041dc5a: EDITOR\ZONE
  paths, TOT/DAT/CGR/BIN/MIN/LNK loads, y-line 0x4ea900 + z-base
  0x4eaacc table build, >=0x80 sweep planes 0..6, PAD->DAT 0xFF marks
  at level*w*h+y*w+x UNCHECKED, CGR heights RAW 1024-B maps at
  dir[s]+4*s+8 - no codec, corrects FORMATS 18; MRK word 3 = spawn z
  level; order armer single caller 0x433cfb). Engine:
  Terrain::from_mission_bytes + corpus gate mission_corpus_gate.rs -
  ZONEA 25x75 loader pin (deck z 31, type-37 wall z 1 = climb 30, PAD
  mark), MRK[0] (21,73,1) spawn settle z 31, staged second robot
  order->walk 4 tiles east on real bytes, arrival snap (west approach
  lands one tile short of the target origin - faithful 0x1400 radius
  semantics), hashes pinned spawn/arm/arrival + two-run identity,
  ZONEB multi-level spawn settle z 95. All workspace tests green,
  fmt+clippy clean, release build ok, MANIFEST verified, pushed.
- 2026-08-20: P2d sim-tail slice COMPLETE (commits c33f615 + 6280857,
  worker 778d091a claim 1): RE notes amendment 7b (objdump re-read of
  move_is_possible@0041e897, FUN_0041ebf8, FUN_004247b5, FUN_004248c8,
  spawn seed loop) + the mission.rs seam adopted from the interrupted
  e1eb0092 WIP and driven 6/9 -> 9/9 green. Engine corrections per the
  binary: per-probe climb refs (probe_z[i] sar-signed, 0xFFFF=-1) in
  move_possible, settle probe passes Q13 world pos (double-shift fix),
  armer snap = tile ORIGIN (tx<<13, no +0xF00), order-window clear
  outside the window!=0 branch (single-robot armer clears next frame).
  Tests re-pinned: dist_octagonal abs (both args), spawn settle on
  height-3 floor + faithful no-settle on tall floors, order walk runs
  to arrival + tile-ORIGIN armer snap asserts, wall test single-column
  geometry. 38 workspace test binaries green, fmt+clippy clean,
  release build ok, MANIFEST verified x2, pushed.
- 2026-08-20: P4-menu engine step COMPLETE (D42, commits 57413b0 +
  0a10a54 + 7ff713e, worker 26ccbd31 claim 1): the D41 findings in
  the engine. menu.rs TitleMenu - builder semantics menus 1/2/3/5,
  strip hit-test, hover/click SFX (MENU1/MENU2 RAW as mixer
  instruments 0xE0/E1, 4-tick debounce), bottom-anchored dual-base
  draw (0x82 green selected vs 0 blue - font.rs from_bank_at +
  draw_at), name entry (0x8e cursor blink, (blink&0xc) duty,
  menu_type_char/menu_backspace API, GOD default), attract replay
  (idle >= 0x300 -> MoviePlayer restart in place, skippable via
  finish()), menu-1 actions (start seed 4000-diff*500 -> Title->
  Brief handoff cached host-side, difficulty cycle, saved-game
  EMPTY menu, quit-confirm -> Scene::Quit; HOF/credits/coop stubs
  per D42.5). Menu OWNS Title input (fsm fed neutral frames -
  hash-isolated unit+corpus pinned); staged-inert lifecycle; menu
  plane after the title movie. Chain Title fetch set grown
  (LANGUAGE/FULLFONT/FULLPAL/MENU1/MENU2) + GameGfxSource SOUND/SFX
  tier. Corpus gate tests/menu_gate.rs (exact MENU_ITEMS strings,
  difficulty cycle, strip geometry, green 233..=244 vs blue 244..=
  255 ramp pin END-TO-END, start handoff seed 3500, SFX audibility,
  TITLE.SMK restart, scripted two-run byte-identity). 393 workspace
  tests / 0 failed, fmt+clippy clean, headless smoke two runs
  byte-identical, parity IDENTICAL to D40 baseline 143e60d,
  MANIFEST OK x2.
