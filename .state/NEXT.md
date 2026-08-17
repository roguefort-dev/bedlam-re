# NEXT — task queue (top first; rewrite this file at end of every run)

## Now
1. [P2] Decompile the 100Hz tick callback LAB_0044de58 (game main loop; runs on
   winmm worker thread). Expect: 20fps sim/render pacing (8street claim, still
   unanchored), input polling, RNG chain entry, state strides. Write
   docs/RE-EXW-TICK.md. Use -process BEDLAM.EXW -noanalysis postScript (Ghidra
   discipline: NEVER re-import; see AGENTS.md). Follow-up context in
   docs/RE-EXW-MAINLOOP.md. Also worth grabbing in same dump: FUN_0044d9c0
   (thread starter), FUN_0044ceb0 (F-key?), FUN_0044b1c0 (pause?).
2. [P1->P3] Promote the tools/inspect decoders into a proper workspace crate
   engine/bedlam-assets (lib + unit tests + round-trip test over a sample of
   game-data, manifest-checked). Keep tools/inspect as a thin CLI over the crate.
3. [P2] RE the .MRS loader in EXW (open questions in RESEARCH-8STREET.md) to close
   the last mission-format gap; likewise CONFIG.BDL (61B).

## Backlog (not yet started)
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Spec doc: input/control map — anchor starts found: FUN_0041be05(vk,isRel) kb,
  FUN_0041bf35(btn,event) mouse (see RE-EXW-MAINLOOP.md WndProc table).
- bedlam-core crate skeleton (deterministic sim, replay, state hash per PLAN sec 7).

## Done (append)
- 2026-08-17 32cdd7b [P2] EXW main-loop RE done: cspec openwatcomcpp confirmed,
  672 functions dumped (ghidra-project/analysis/exw-functions.txt, gitignored),
  boot chain entry->startup->main@0044d6e8, init FUN_0044d320 (window "Bedlam for
  Windows 95", 640x480, -f/-v flags), pump FUN_0044d93c, WndProc 0044dacc
  (message+input table), 100Hz timeSetEvent tick anchored @word 00456ec4=10ms,
  globals map. Docs: docs/RE-EXW-MAINLOOP.md.
