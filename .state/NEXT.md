# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] mission sidebar ART: decode the sidebar redraw pass
   FUN_00408403 (the DAT_0046ccec consumer, RE-EXW-SIM sec 6c.5)
   + which bank feeds it (GENERAL/WEAPONS.BIN family - the 8street
   census lists the candidates) and what the [480,640) strip should
   contain per frame. Commit RE notes first, then wire the present
   path so the strip stops being black (MissionScene::present +
   DESIGN-GAME sec 11 SIDEBAR PRODUCER). The redraw countdown
   landed (490d856); the corpus-gate frame pins WILL legitimately
   move when sidebar pixels appear - re-pin once with the
   regeneration documented in the gate header (sim pins must NOT
   move). Keep tests, fmt, clippy -D warnings, headless smoke
   two-run identity, and the MANIFEST check green.
## Backlog (not yet started)
- The 0x4de664 per-type ORDER table loader (RE-EXW-SIM sec 9 item 5
  remainder): find the file source (no static xrefs; [hypothesis]
  TABLE.BIN) + the word@0x4edb90 player-robot TYPE producer
  (GameMain@0x41c34c), then replace the all-7 availability default
  and set_order_availability seam with the real table.
- The map-overlay family (sec 6c.1): _DAT_004edba0/FUN_004089b1 +
  the FUN_00401107 present-window map mode - needed before the
  map-toggle strip can be wired.
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
- 2026-08-21: P4 window-host exit path FIXED (worker 34bd8958 claim
  1, commits 1b45f3c + 246f2a1, D48): Escape in bedlam-shell
  --window exited via SIGSEGV (coredump 422346). Decoded stack
  proved the crash was wgpu/EGL teardown, NOT audio: the lazy wgpu
  Global drop (the pipeline bind group was the LAST wgpu object)
  runs eglTerminate through Mesa, marshaling Wayland requests
  through the winit window's proxies - freed early because
  WindowHost declared window: Arc<Window> FIRST (field-order drop).
  Fix: ordered teardown in run_window (audio drops FIRST, then all
  wgpu/EGL objects while the window lives, window Arc last) +
  structural field orders (window last in WindowHost, audio before
  gfx in ShellApp) + the cpal dead-feed guard (Arc<AtomicBool>;
  quiet->pause->drop; late callbacks write exact silence u8 128
  without touching the ring; fill_from no-ops once quiet;
  silence()/drain() factored, 3 new tests). Repro gate added:
  BEDLAM_WINDOW_EXIT_MS auto-exit hook fires the same exit path as
  Escape. Live A/B on this session (Wayland + Mesa EGL, 48000 Hz
  2ch i16): pre-fix exit 139 + coredump 1150695 (identical stack),
  fixed exit 0 twice, no new coredump. 431 workspace tests green,
  fmt + clippy -D warnings clean, headless smoke two-run
  byte-identical at the D47 baseline, MANIFEST verified. Pushed.
- 2026-08-21: P4 modern audio output rates COMPLETE (worker 2cd16045
  claim 1, commit 4ed1e26, D47): the cpal output edge now negotiates
  the best MODERN rate - 48000 then 44100 then mixer-native 11025
  then device default - via a pure choose_output_config over a
  neutral OutputConfigSpec (cpal's range type is not constructible;
  the negotiation fallback matrix is unit-pinned without a device),
  ranked within a rate by channels (stereo/mono/other) then format
  (S16/F32/other), rate dominating; wide ranges pin via
  try_with_sample_rate (44100-96000 opens at 48000). The Q16 frame
  stepper gained linear interpolation (nearest, ties +inf, i64
  internally; lone frames edge-hold, empty ring exact silence,
  native rate still exact 1:1 passthrough). Mixer bus + parity
  stream stay 11025 Hz stereo u8 byte-faithful. New tests: matrix,
  44.1k/48k interpolated-ramp pins (hand literals 0/941/1882/2822/
  3763/4704 + quarter-ramp 0/250/500/750), downsample blend,
  i16/f32/u8 silence + full-scale mapping, u8 128/255 end-to-end
  through the D31 bus. 428 workspace tests green, fmt + clippy -D
  warnings clean, headless smoke two-run byte-identical AND
  identical to the pre-change binary, parity harness identical on
  all four anchors, MANIFEST verified, live probe opens 48000 Hz
  2ch i16 (was 11025). Pushed.
- 2026-08-21: P4 GAMEPAL mission present tail COMPLETE (worker
  1776dc60 claim 1, commits 663ddba + 7c25bfd): DESIGN-GAME sec 11
  amended then implemented - GAMEGFX\GAMEPAL.PAL (770 B,
  parse_vga770 family; RE-EXW-MISSIONVIEW sec 6 / RE-EXW-SIM 7c.3)
  joins the Mission fetch set in the GAMEGFX tail (SINTABLE, DANTE,
  GAMEPAL, MRK - 10 files), folds with the loading_palette rule
  (>>2 lossless), and OWNS the mission plane: MissionScene carries
  palette, plane() returns its own palette, the frame palette IS
  GAMEPAL (palette_dirty every frame, MovieFrame seam; window
  indexed->RGBA upload untouched). load_mission/stage/chain
  signatures grew gamepal; corpus gate re-pinned once - spawn frame
  a79fcada30ec5e50, mid-walk 1b75b68ce66019e1 (sim pins
  36ddc86345c8351c/f35db41f0efb858d + render-gate pins unchanged,
  regeneration documented in the gate header) + structural pins
  (frame.palette == folded GAMEPAL, 254/256 non-black, entry 1 =
  0x3E3A39). Headless smoke 25 fetches (GAMEPAL.PAL 770 B) two-run
  byte-identical exit 0; parity harness byte-identical to the D28
  anchors; all workspace tests green; fmt + clippy -D warnings
  clean; release ok; MANIFEST verified; D46. Pushed.
