# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P4] Reconcile the interrupted Smacker stream WIP and finish the headless
   TITLE.SMK gate. D30 already chose the vendored pure-Rust decoder; do not redo
   dependency research. The pre-existing changes in bedlam-assets/src/lib.rs,
   tests/corpus.rs, and tests/smk_title_gate.rs are a compile-breaking API/test
   contract for implementation missing from src/smk.rs--preserve and adopt them.
   Implement the codec-neutral SmkStream API they specify, keeping buffer-only,
   deterministic, panic/OOM-safe behavior and no unsafe in bedlam-assets. Validate
   malformed/truncated inputs, deterministic repeated decode, and the corpus-skipping
   TITLE.SMK 640x320/1227-frame/audio gate. Bracket corpus access with manifest
   checks; commit fingerprints only. Stage only Smacker task paths. Run fmt, clippy
   -D warnings, focused tests, then workspace tests. If too large for one unit,
   commit a coherent tested seam and rewrite this item to the exact next slice.

## Backlog (not yet started)
- Integrate decoded TITLE.SMK playback into GameHost/presentation.
- Add a native executable shell: window/surface, input adapter, fixed-step clock,
  and platform audio output.
- Build the first menu and ZONEA/MISSION1 playable vertical slice.

## Done (append concise entries only)
- 2026-08-19: D30 selected and vendored the pure-Rust Smacker decoder. Interrupted
  implementation left an explicit stream API/test contract for the next worker.
- 2026-08-19: Superseded 22 erroneous state-only stand-down runs. The persistent
  operator TUI never held a queue claim; process liveness is not ownership evidence.
  The old process-liveness inference is revoked, queue history compacted, and
  state-only stand-down commits forbidden.
- 2026-08-19: Original DOSBox/Wine runtime comparison removed as a phase gate by
  owner direction; automated Rust tests and remaster playtesting are authoritative.
