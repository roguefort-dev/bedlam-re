# NEXT — task queue (top first; rewrite this file at end of every run)

## Now
1. [P2] RE the 100Hz tick frame body FUN_0041bfb6 (TimerCallback 0044de58 calls
   it unconditionally every 10ms; cursor sample goes through FUN_0044b4fc).
   Walk its callee tree: sim/render pacing (8street claims 20fps — anchor it),
   input queue drain, RNG chain entry, state strides. Write docs/RE-EXW-TICK.md.
   Ghidra discipline: use -process BEDLAM.EXW -noanalysis postScript, NEVER
   re-import (AGENTS.md). Context: docs/RE-EXW-MAINLOOP.md (names already applied
   in BedlamWatcom project: WinMain/InitInstance/MsgPump/TimerInit/TimerCallback/
   WatcomCrtStartup/BedlamWndProc/GameThreadStart).
2. [P1->P3] Promote the tools/inspect decoders into a proper workspace crate
   engine/bedlam-assets (lib + unit tests + round-trip test over a sample of
   game-data, manifest-checked). Keep tools/inspect as a thin CLI over the crate.
3. [P2] RE the .MRS loader in EXW (open questions in RESEARCH-8STREET.md) to close
   the last mission-format gap; likewise CONFIG.BDL (61B).

## Backlog (not yet started)
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Spec doc: input/control map — anchors: FUN_0041be05(vk,down) kb, FUN_0041bf35(btn,state)
  mouse, FUN_0044b4fc(x,y) cursor-per-tick (see RE-EXW-MAINLOOP.md).
- bedlam-core crate skeleton (deterministic sim, replay, state hash per PLAN sec 7).
- Find game-thread proc body (arg register-passed to FUN_00450242 _beginthread wrapper;
  thread signals readiness via 004ef674).

## Done (append)
- 2026-08-17 (this run) EXW loop refinement: WinMain misname corrected (was
  BedlamShutdown), TimerCallback 0044de58 promoted+decompiled -> calls
  FUN_0041bfb6 every 10ms tick (frame body identified), GameThreadStart/TimerInit
  named, 675-fn function DB committed at docs/exw-functions.txt, watcall notes
  (unaff_* args, RET n) in docs/RE-EXW-MAINLOOP.md. DECISIONS D9.
- 2026-08-17 32cdd7b [P2] EXW main-loop RE done: cspec openwatcomcpp confirmed,
  672 functions dumped, boot chain entry->startup->main@0044d6e8, init
  FUN_0044d320, pump FUN_0044d93c, WndProc 0044dacc (message+input table), 100Hz
  timeSetEvent tick anchored @word 00456ec4=10ms, globals map. Docs:
  docs/RE-EXW-MAINLOOP.md.
