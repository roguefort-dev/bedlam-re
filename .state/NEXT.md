# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P5] Integrate decoded TITLE.SMK playback into GameHost/presentation:
   drive bedlam-assets SmkStream from the fixed-step clock (us_per_frame),
   blit pixels+palette through the renderer seam, and queue decoded DPCM
   PCM packets to the audio mixer. Keep the headless double-decode gate
   (tests/smk_title_gate.rs) green while wiring; no decoded media in git.

## Backlog (not yet started)
- Add a native executable shell: window/surface, input adapter, fixed-step
  clock, and platform audio output.
- Build the first menu and ZONEA/MISSION1 playable vertical slice.
- OPERATOR NOTE (carried from the 13:43 blocked-tag lineage): MANIFEST-2.sha256
  at the repo root mismatches 470 files - it documents a different tree
  snapshot (its BEDLAM.LOG entry is the sha256 of an EMPTY file). Re-anchor or
  delete it. It was never used as the integrity gate: MANIFEST.sha256 is the
  canonical AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-19: P4 Smacker unit COMPLETE (this commit): codec-neutral SmkStream
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
