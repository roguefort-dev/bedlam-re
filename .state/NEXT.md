# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] Native executable shell, step 1 - window + surface + fixed-step
   present loop: make everything landed so far visible. bedlam-platform
   already owns wgpu 27 (gpu.rs indexed upload, scale.rs PARITY
   640x480 -> output scaling per D20). Add a winit window + wgpu
   surface (dependency spike per PLAN sec 4: winit is the default
   candidate - record the choice + pinned version in DECISIONS.md),
   a fixed-step clock driving GameHost at the 60 Hz host pace with
   present on vsync (timing NEVER feeds hashed state - Determinism
   Charter), and an input event -> host input adapter skeleton.
   A binary boots the wired chain (boot attract -> loading -> brief,
   D31-D37 sites, corpus paths). Headless discipline: the window
   path env-gated (e.g. BEDLAM_SHELL=1) so tests / unattended runs
   never open a display; unit-pin the clock arithmetic + input
   mapping; smoke-drive N ticks headless if feasible. fmt + clippy
   -D warnings, provenance tags, MANIFEST check around any
   corpus-touching run.

## Backlog (not yet started)
- Native shell step 2: platform audio output (cpal default candidate
  per PLAN sec 4) consuming the D31 stream bus + entry-audio sites.
- Build the first menu and ZONEA/MISSION1 playable vertical slice.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
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
