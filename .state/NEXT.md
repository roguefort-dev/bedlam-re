# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P5] Wire the Shop + Brief scene movie backdrops into GameHost per
   the D31/D32 lifecycle: Shop scene plays SHOP.SMK (61-frame ring,
   40 fps) behind the shop UI; Brief scene plays the BRF_{zone-letter}
   {sub}.SMK backdrop selected from the hashed episode slot
   (movies::briefing_name; ring, silent). Both are load_movie(Scene::X)
   bindings + lifecycle/selection unit tests in the D31 shape.

## Backlog (not yet started)
- Post-cutscene loading-screen flow per EXW LAB_0041c69e tail:
  BETWEEN.BIN interlude, then Region::loading_bin()/loading_pal()
  (LOAD_UK/US.BIN + LOADPAL/LOADPALU.PAL) on the Cutscene->Select
  transition (BIN image-bank decode path needed in the presentation
  layer first).
- Add a native executable shell: window/surface, input adapter, fixed-step
  clock, and platform audio output (the movie plane + stream bus are
  ready to consume from bedlam-platform).
- Build the first menu and ZONEA/MISSION1 playable vertical slice.
- OPERATOR NOTE (carried from the 13:43 blocked-tag lineage): MANIFEST-2.sha256
  at the repo root mismatches 470 files - it documents a different tree
  snapshot (its BEDLAM.LOG entry is the sha256 of an EMPTY file). Re-anchor or
  delete it. It was never used as the integrity gate: MANIFEST.sha256 is the
  canonical AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-20: P5 cutscene movies + corpus inventory COMPLETE (D32, this
  commit): game-data SMK corpus PINNED in bedlam-assets
  tests/smk_corpus_gate.rs (34 files: formats, frame counts, rates,
  ring flags, y-scale None corpus-wide, the one DPCM 1/8/11025 audio
  shape; listing-to-table equality both ways; ignored regen helper).
  Reject-or-map verdict per D31: every file MAPS onto the existing
  playback path - nothing rejected, no y-scale or resampling owed, all
  periods exact on the x240-us accumulator grid. bedlam-game movies.rs
  selection module (D17-b hash-free, host byte-source-free):
  cutscene_name (ZONEDONE/END at stage >= MAX_STAGE, EXW
  LAB_0041c69e pre-increment vs Episode::complete post-increment
  reconciled), Region (DAT_0046ae64) backing LOAD_UK/US.BIN +
  LOADPAL(U).PAL and LOGO/GTLOG variants, briefing_name over
  BRF_{B..F}{1..5}. GameHost::cutscene_name + load_cutscene wire the
  Cutscene scene (D31 lifecycle: inert-until-scene, dropped on exit);
  selection + lifecycle unit-pinned through the FULL_MASK cadence.
  Workspace 257 tests green, fmt + clippy -D warnings clean,
  MANIFEST.sha256 OK before AND after corpus runs. Interrupted
  predecessor WIP adopted per AGENTS.md (snapshot in
  /tmp/opencode/wip-snapshot-1bd01455/).
- 2026-08-19: P5 TITLE.SMK playback integration COMPLETE (D31, this
  commit): GameHost plays decoded TITLE.SMK on the Title scene -
  MoviePlayer x240-us fixed-step clock over the SmkStream seam
  (banking accumulator, ring/finish/hold, 4096-frame runaway guard);
  RenderInput.movie MovieFrame replaces the scene pipeline (centered
  letterbox blit via the one Frame::blit_indexed impl, palette_dirty
  per frame, lossless PALMAP >>2 fold); mixer PCM stream bus
  (queue_pcm_u8 FIFO under the voices, master-following gain, 16 MiB
  loud cap, StreamOverflow); host lifecycle inert-until-scene with
  hash isolation pinned. Gate tests/title_playback_gate.rs: full
  1227-frame playback, exact pacing milestones (frame k at pump
  ceil(k*15.9984M/4M)-1), byte-identity vs an independent stream walk
  at frames 1/600/1226, two playbacks identical, scene-hash chain
  unchanged. Workspace 280 tests green, fmt + clippy -D warnings
  clean, manifests OK x2. Unit reconciled the interleaved restart
  lineages 1787165989 + 1787172789 (fragments archived under
  .state/scratch/collision2-20260819T2256/); D30 double-decode gate
  untouched.
- 2026-08-19: P4 Smacker unit COMPLETE (1f892a6): codec-neutral SmkStream
  seam in bedlam-assets over the vendored pure-Rust smk 0.1.0 backend (D30);
  header offsets corrected (tree_sizes@56..72, audio_rates@72..100, values
  verified against TITLE.SMK bytes); vendored-backend DPCM safety patch
  (NOTICE.md: unpack-size clamp + too-small-buffer rejection). Headless
  TITLE.SMK gate green: 1227 frames, two full passes byte-identical,
  video_sha256=6aa75c55a68ab877429fea4216e730f62c281b46f75b3d27f2437fb8cd82cdd1
  audio_sha256=73fdee8e95328c4733e3b0f135bc26af975cbd335081e79590a8c19299c0c6e3
  packets=1212 audio_bytes=901752. fmt + clippy -D warnings + workspace tests
  green; MANIFEST.sha256 verified before/after corpus access. Unit reconciles
  the interrupted WIP of the lineages orphaned by the 13:22 server restart,
  per the 13:43 [BLOCKED] handoff note (this file, superseded).
