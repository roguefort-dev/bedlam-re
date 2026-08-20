# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P2g] Title menu RE slice (first UI archaeology): locate the EXW
   title-menu screen code - the menu draw list (buttons/items, hit
   rects or cursor regions), input dispatch (mouse click/keys), and
   the options entry path. Anchor everything to EXW addresses in a
   docs/RE-EXW-*.md section. Stream-survival rule (AGENTS): decode a
   BOUNDED piece, commit the RE notes immediately, then proceed -
   many small committed hops, no long silent analysis stretch. This
   is the prerequisite for the P4 vertical-slice menu step (PLAN
   sec 6 P4: menu -> ZONEA/MISSION1 render -> move one squad member
   -> palette/audio present; the shell now has window + input +
   present + audio, so the menu is the first missing slice piece).

## Backlog (not yet started)
- P4 vertical slice assembly: menu + ZONEA/MISSION1 render + one
  squad-member move (needs the P2g menu RE above + the P2d sim tail).
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
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
- 2026-08-20: P4 native shell step 1 COMPLETE (D38/D39, commit
  493fbd5, watchdog repair): bedlam-shell crate (FixedStepClock
  integer banking + anti-spiral; map_physical_key input seam - winit
  KeyEvent cannot be constructed outside winit, the predecessor test
  was rewritten; D31-D37 chain fetch layer; env-gated winit 0.30.13 +
  wgpu surface host via Arc<Window> -> Surface<'static>, Fifo vsync,
  PARITY present; headless smoke 600 fixed pumps, two runs
  byte-identical, fetch set exactly GTLOG/LOGO/TITLE/ZONEDONE/
  BETWEEN/LOAD_UK/LOADPAL/FULLFONT/FULLPAL/LANGUAGE.ENG; two-tier
  GameGfxSource GAMEGFX-then-root). ParityGpu::new_for_surface in
  bedlam-platform. Work of four step-capped GLM workers (486a18e1,
  8d2f7acc, 3a5e5f9e, f24c9332 - incl. the Surface<'_'> stray-quote
  fix and the LANGUAGE.ENG re-root) adopted + landed by watchdog
  repair agent 410671: 356 workspace tests green / 0 failed, fmt,
  clippy -D warnings, manifest OK before AND after.
- 2026-08-20: P5 BRF_DROP play site + briefing intro pair COMPLETE
  (D37, commits 3a2981d RE + bba01fe code + 40b3700 docs): RE verified
  FUN_0043d00b IS the briefing screen; BRF_DROP.SMK opens FIRST at
  every movie-enabled briefing (asm 0043d447..0043d490, gate
  DAT_0046cca4), full-screen ONE pass (frames-1 renders), then the
  constructed BRF_{zone}{level}.SMK ring (letter = zone@004edd8c +
  0x40, zones 2..=6 = B..=F - D33 open note resolved) rings until UI
  exit; open failures fatal; GO arms only after the handoff. Engine:
  brief.rs BriefIntro Staged->Drop->Backdrop, GameHost load_briefing
  on the D31 lifecycle, latent D31 movie.rs ring-Last bug FIXED (rings
  froze at first cycle end; now wrap and continue), corpus gate
  tests/brief_gate.rs (drop 29/30 frames, handoff pump 58, silent
  pair, 512->1 ring wrap, two runs byte-identical). Code by
  predecessor 3d88a359 (died after bba01fe; its uncommitted
  DECISIONS/RE-EXW docs adopted, 342->343 test recount corrected);
  this run 5a637669 (claim 1) re-verified green (343/0 tests, fmt,
  clippy, manifest x2) + closed the queue.
