# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] vertical slice assembly tail: ZONEA/MISSION1 render + one
   squad-member move. Inputs are all in place: the mission DAT/CGR/TOT
   formats (FORMATS-MISSION), the P2d sim seam (engine/bedlam-core/
   src/mission.rs - Terrain from DAT planes + CGR height sprites,
   MissionSim order/walk/hash, green 9/9), and the P2d RE notes
   (docs/RE-EXW-SIM.md incl. amendment 7b). REMAINING RE INPUT: the
   mission file-load + table-build pass (RE-EXW-SIM sec 9 open item 1:
   fills 0x4ea900/0x4eaacc/004eddec/df0 - the ".TOT/.DAT/.CGR/.MIN/
   .PAD" loader) - decode that first as a bounded RE-notes commit,
   then wire ZONEA/MISSION1 terrain into a Terrain + MissionSim and
   drive one order->walk on the real map (corpus-gated, hash-pinned).

## Backlog (not yet started)
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence,
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- RE-EXW-SIM sec 9 open items 2-5: FUN_00440e45 identity, robots()
  extra-phase semantics + state-1 producers, sidebar order buttons,
  the 0x62-stride robot-type stats table.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
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
