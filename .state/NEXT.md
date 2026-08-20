# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P2d] sim-tail RE slice (the P4 slice's remaining input): decode
   the EXW mission-sim tail the vertical slice needs - one squad
   member's move on the ZONEA/MISSION1 map: the order input path
   (squad select + destination), the movement grid/pathing (tile
   stride, walkability bits), and the per-tick mover state the sim
   hash must cover. Bounded piece: RE notes committed FIRST
   (docs/RE-EXW-*.md section), then the bedlam-core seam. This
   unblocks the last P4 queue item (ZONEA/MISSION1 render + one
   squad move) per PLAN sec 6 P4.
   ADOPT INTERRUPTED WIP (watchdog repair 2026-08-20): the RE-notes
   half is DONE (commit 98fc0b0). engine/bedlam-core/src/mission.rs
   (untracked, ~1078 lines, EXW-anchored seam) plus the one-line
   lib.rs `pub mod mission;` are the interrupted engine write from
   the 22:57 session (worker e1eb0092, killed mid-test-iteration by
   the provider stream outage; insurance snapshot in .state/scratch/
   mission.rs.e1eb0092-interrupted-wip-20260820). State at cutoff:
   6/9 mission tests pass, 3 red (dist_octagonal_matches_original,
   spawn_settles_and_tiles, order_walk_and_arrival_snap). Build on
   this WIP - do NOT re-derive it; drive the red tests green, then
   fmt+clippy, and commit green checkpoints early.

## Backlog (not yet started)
- P4 vertical slice assembly tail: ZONEA/MISSION1 render + one
   squad-member move (blocked on the P2d sim tail above).
- Title-menu polish backlog (all optional, none block P4): pin the
   menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
   PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
   the save-load restore path (FUN_0044745e + completion bits),
   CONFIG.BDL writer family (FUN_0042540c) for name persistence,
   OPTIONS.MRS staging on Title (music track_name wiring), and the
   FUN_00448ef1 multiplayer lobby if ever needed.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
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
- 2026-08-20: P2g title-menu RE slice COMPLETE (D41, commits 3eb3092 +
  cf75108, worker aca4dac6 claim 1): NameEntryScreen@0043a5fc IS the
  title/options menu; FUN_00445b5c builds menus 1..5 (count word
  004eabd2 + 7 slots @004eabd4 stride 0x30), FUN_0044653a draws
  bottom-anchored; hit model = strip x (0xdc,0x1a4) y (top,0x1d6),
  index (y-top)/0x18; click = g_scroll_flags, hover/click SFX
  MENU1/MENU2; attract >= 0x300 -> skippable TITLE.SMK; all menu-1
  item actions asm-anchored (start 4000-diff*500, difficulty cycle,
  name entry ENTER-exit + FUN_0042540c CONFIG.BDL save, HOF,
  CREDIT_1..13, quit-confirm); multiplayer player-count 2..12 via
  left/right click; save-load restore with completion bits. Negative:
  MENU_ITEMS 47..58 (Options + toggles) unreferenced - no options
  screen in EXW. Corpus pin: base 0 vs 0x82 glyph sets = blue vs
  green FULLPAL ramps, identical shapes. docs/RE-EXW-TITLEMENU.md +
  ExwTitleMenu.java (-process, no re-import); MANIFEST verified; no
  Rust changes this run.
- 2026-08-20: P4 native shell step 2 COMPLETE (D40, commits 58eb8a6 +
  c48cd91 + 143e60d, worker e76159bb claim 1): platform audio output.
  cpal 0.18.2 (bedlam-shell only; mixer hermetic, audio un-hashed per
  D17b): bounded stereo-frame ring (4096 fr; full = drop oldest,
  underrun = exact silence) behind one poison-tolerant mutex; window
  loop the only producer (watermark 736 fr after each pump batch),
  cpal callback the only consumer; device pinned at native 11025 Hz
  when a supported range contains it (live default device accepted
  11025/2ch - ignored-tagged probe; ~100-200 ms device startup
  latency measured), else Q16 nearest-neighbor frame stepper (4x
  exact repeats; 48k step 15053 / 8k step 90317 + sample-hold
  positions unit-pinned); mono floor-average; dasp format
  conversions; no device = silent run, never fatal. Headless smoke
  drains 184 fr/pump (110400 = 600x184, 158092 non-silent samples);
  scene/frame hashes IDENTICAL to the pre-change binary; two runs
  byte-identical; MANIFEST x2; 366 workspace tests / 0 failed; fmt +
  clippy -D warnings clean.
