# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] The 0x4de664 per-type ORDER/WEAPON table loader (RE-EXW-SIM
   sec 6c.8b + open item 5, STRONG new lead): the table is the
   7x0x0E WEAPON groups (word0 = name index into the compiled-in
   FUN_00420260 table, word1 = ammo max) and FUN_0041df10 pins
   `LoadFile("GAMEGFX\TABLE.BIN", DAT_0046cbbc)` — the hypothesis
   is now near-certain; verify TABLE.BIN's byte layout against
   0x62-stride/type*count (file is in game-data/BEDLAM/GAMEGFX)
   and find the copy INTO 0x4de664 (grep the loader family around
   FUN_0041df10) + the word@0x4edb90 player-robot TYPE producer
   (GameMain@0x41c34c). Then replace the all-7 availability
   default + set_order_availability seam with the real table, and
   wire the row TEXT (FUN_00420260 names + SMLFONT glyph draw
   FUN_00408913/FUN_00402884 at (0x1ED,0x5B+14i), count "%04i" at
   (0x25C,0x5B+14i)) now that every input exists. Commit RE notes
   first. Frame pins WILL move once (text pixels); sim pins must
   NOT move. Keep tests, fmt, clippy -D warnings, headless smoke
   two-run identity, and the MANIFEST check green.
## Backlog (not yet started)
- The map-overlay family (sec 6c.1): _DAT_004edba0/FUN_004089b1 +
  the FUN_00401107 present-window map mode - needed before the
  map-toggle strip can be wired. Includes the deploy-panel
  backdrop (SCANNER sprite 0x12 @ (0x1EE,0xC3), countdown
  0x46ccf8) and the blink-cursor producer (0x4dc5d0).
- Sidebar bars + score strip (RE-EXW-SIM 6c.8d): FUN_0040807f HP
  (0x46 - hp*46/5000) + armor (0x8E - armor*46/2500) bars need the
  +0x78/+0x2E sim fields; FUN_004085ce score/money (NUMBERS.BIN)
  needs score/money sim state (producers FUN_0040eba0 case 4:
  +1000/+2000/+5000/+10000 score, +10/+50/+100/+250 money).
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
  mixer instruments exist) + the order SFX 0x2A armer click.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items 1/2/4: type-DB tail producers
  (+0x18/+0x1a/+0x1b/+0x1c), the u32[0x4dd444] remap tables +
  u32[0x456ca8] anim sequence + the water flag producer (needed
  before the 0x12d/0x12e/0x12f flush remaps can leave water-off
  semantics), BIN u32[bank+0] header word.
- MISSIONVIEW sec 5d tail (robots only are wired): platform loop
  (0x4eb638, bank DAT_0046af54), effects loop (0x4cf638 - the
  FUN_00401e39 draw_IMG codec family, a DIFFERENT .BIN sprite layout
  per RESEARCH-8STREET), ROBNUMS name plates, Shield/Variant bank
  staging (nodes enqueue, flush skips while unstaged).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity, robots()
  extra-phase semantics + state-1 producers.
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
- 2026-08-21: P4 mission sidebar ART COMPLETE (worker 49294e3c
  claim 1, commits 5860fe6 + abcbb37 + 805ed10, D50): RE-EXW-SIM
  sec 6c.8 = the sidebar redraw pass FUN_00408403 fully decoded
  (7 order rows: gate = the group NAME-index word, count clamped
  9999, armed rows GENERAL.BIN 0x47+0x4A / unarmed 0x49+0x4C at
  (0x1EB,0x59+14i)/(0x25A,0x59+14i), name + "%04i" text via
  SMLFONT) + the semantic correction that the "orders" are WEAPONS
  (compiled-in name table at 0x4589DD..0x458C0F, +0x6E = armed
  bits, word1 = ammo, ammo-refill + score/money pickup producers)
  + the banks (GENERAL/SMLFONT/NUMBERS/SCANNER by asm ESI anchors,
  verified against shipped bytes) + the sibling passes
  (FUN_004072bf portraits/HP-dither/armor-tick/blink,
  FUN_0040807f HP+armor bars, FUN_004085ce score/money strip) +
  the MissionShell initial trigger 0x447c74 (countdowns = 2).
  ENGINE: bedlam-render ui_bank codec (FUN_00401ca2 semantics +
  corpus GENERAL.BIN geometry pin), GENERAL.BIN + SMLFONT.BIN in
  the 12-file mission chain, portraits every present + row chrome
  on the countdown from the real bank bytes, initial redraw armed
  at activate; text/bars/score deliberately unwired (D50
  never-invent). Corpus gate: sidebar-carries-art pin (4844 px),
  frame pins regenerated once (spawn 018eba568d9b3bae, mid-walk
  4a3abd2de43f31df), sim pins unchanged. Workspace tests + fmt +
  clippy -D warnings clean, headless smoke two-run byte-identical,
  MANIFEST verified. Pushed.
- 2026-08-21: P4 mission sidebar producer COMPLETE (worker 6ebe5cff
  claim 1, commits cfee256 + 490d856): RE-EXW-SIM sec 6c =
  sidebar_control@0040d197 fully decoded (select strips, 7 order
  rows + keys 1..7, the order-bits word +0x6E with +0x38+8k gates,
  the DAT_0046ccec redraw COUNTDOWN consumed by the FUN_00403938
  tail, the 0x62-stride table = 7x0x0E ORDER groups, alive offset
  fixed to +0x7C double-anchored) + engine wiring (MissionScene
  sidebar presentation half: click dispatch, strips with
  squad/alive gates, rows with per-robot availability mask
  [design all-7 default + host seam], redraw countdown per
  present; D17-pinned that none of it arms orders or moves the sim
  hash). New tools/ghidra-scripts/XRefList.java. 4 unit tests + a
  real-ZONEA corpus gate pin block, all existing hash pins
  unchanged. 435 tests green, fmt/clippy clean, headless smoke
  two-run byte-identical at the recorded baseline, MANIFEST
  verified. Pushed.
