# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P5] Extend the playback integration to the remaining cutscene
   movies (game-data SMK corpus inventory first: formats, rates,
   y-scale flags; reject-or-map per D31 policy) and wire the
   Cutscene scene to LOAD_UK/US.BIN-backed movie selection.

## Backlog (not yet started)
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
  audio_sha256=73fdee8e95328c4733e3b0f135bc26af975cbd335081e79590a8c1929940c6e3
  packets=1212 audio_bytes=901752. fmt + clippy -D warnings + workspace tests
  green; MANIFEST.sha256 verified before/after corpus access. Unit reconciles
  the interrupted WIP of the lineages orphaned by the 13:22 server restart,
  per the 13:43 [BLOCKED] handoff note (this file, superseded).
