# NEXT — task queue (top first; rewrite this file at end of every run)

## Now
1. [P2] DONE-PART: import itself SUCCEEDED 03:33 (single program verified, do NOT
   re-import - see AGENTS.md Ghidra discipline). NEXT: run analyzeHeadless -process
   BEDLAM.EXW -noanalysis with a postScript that (a) confirms compiler spec is
   openwatcomcpp, (b) dumps function list to ghidra-project/exw-functions.txt,
   (c) finds WinMain/message pump/main loop; write docs/RE-EXW-MAINLOOP.md.
2. [P1->P3] Promote the tools/inspect decoders into a proper workspace crate
   engine/bedlam-assets (lib + unit tests + round-trip test over a sample of
   game-data, manifest-checked). Keep tools/inspect as a thin CLI over the crate.
3. [P2] RE the .MRS loader in EXW (open questions in RESEARCH-8STREET.md) to close
   the last mission-format gap; likewise CONFIG.BDL (61B).

## Backlog (not yet started)
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Spec doc: input/control map from 8street cross-ref -> EXW .data table addresses.
- bedlam-core crate skeleton (deterministic sim, replay, state hash per PLAN sec 7).

## Done (append)
- (none yet - first entry after this file was created 2026-08-17)
