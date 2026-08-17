# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P2] Decompile the game worker-thread body 0044dea0..0044dfec (THE sim/render
   loop; start address proven from GameThreadStart listing in
   docs/RE-EXW-TICK.md). Region is disassembled but not a function yet:
   create it, decompile, walk callees. Expect: 20fps sim/render pacing
   (8street claim, still unanchored), main state machine strides, RNG chain
   entry, and confirmation of who sets go flag 004ef674 (instruction at
   0044deca writes it; FUN_0044d9b4 also writes it). Extend
   docs/RE-EXW-TICK.md or start docs/RE-EXW-GAMETHREAD.md. Ghidra discipline:
   -process BEDLAM.EXW -noanalysis postScript, NEVER re-import.
2. [P1->P3] Promote the tools/inspect decoders into a proper workspace crate
   engine/bedlam-assets (lib + unit tests + round-trip test over a sample of
   game-data, manifest-checked). Keep tools/inspect as a thin CLI over the crate.
3. [P2] RE the .MRS loader in EXW (open questions in RESEARCH-8STREET.md) to close
   the last mission-format gap; likewise CONFIG.BDL (61B).
4. [P2] Tick follow-ups from docs/RE-EXW-TICK.md open list: FUN_00402bac gated
   pump (20x38B records, chan 3), FUN_00425901 50Hz update, FUN_0044b428 scroll
   delta source, resolve .data slot 00457874, DDRAW vtable +0x18 mapping.

## Backlog (not yet started)
- B2: run inspect over game-data-2 again after any decoder change (second fuzz
  corpus); document any B2-only format quirks in RESEARCH-BEDLAM2-CENSUS.md.
- B2: import BEDLAM.EXE (LE/DOS4GW) into Ghidra (needs LE loader handling);
  compare boot/init with EXW findings.
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Spec doc: input/control map - anchors: FUN_0041be05(vk,down) kb,
  FUN_0041bf35(btn,state) mouse, FUN_0044b4fc(x,y) cursor-per-tick
  (see RE-EXW-MAINLOOP.md + RE-EXW-TICK.md).
- bedlam-core crate skeleton (deterministic sim, replay, state hash per PLAN sec 7).
- bedlam-render/P3 design note (D9): renderer emits canonical 640x480 indexed fb;
  ALL resolution/scaling is presentation-layer only.

## Done (append)
- 2026-08-17 19cfb6f [P2] EXW 100Hz tick RE done: TimerCallback=service routine
  (100Hz mouse poll+clamp+store, 5 counters, 50Hz sub-gate, scroll clamp
  9..631/9..463, 8-frame palette cycle 0x90-0x97 @12.5Hz); sim/render loop
  LOCATED at worker thread 0044dea0 (CreateThread-style, stack 0x1000, handle
  004ef698, id 004ef694, go-flag 004ef674); F key = numbered-BMP screenshot
  (FKeyHandler); AppActivate = system-palette mgmt; names applied:
  TickWorker/MousePosHandler/ThreadSpawnThunk/FKeyHandler/AppActivate.
  Docs: RE-EXW-TICK.md (new), RE-EXW-MAINLOOP.md corrections, DECISIONS D10.
- 2026-08-17 0a27bf9 plan/D9: resolution independence (canonical 640x480 fb).
- 2026-08-17 d3c91f5 P2: EXW loop architecture refined (WinMain misname fixed,
  TimerCallback promoted+decompiled, 675-fn DB at docs/exw-functions.txt, D9).
- 2026-08-17 32cdd7b [P2] EXW main-loop RE done (boot chain, main@0044d6e8,
  init/pump/WndProc/timer anchored). Docs: docs/RE-EXW-MAINLOOP.md.

## Run notes
- 2026-08-17 04:2x (tick run): CONCURRENT-AGENT INCIDENT - my heartbeat went
  stale during the ~2min headless run (touched only at run start), so nudge
  spawned a second agent (d3c91f5/0a27bf9) that rewrote RE-EXW-MAINLOOP.md
  mid-edit. No work lost (assert-guarded edits failed clean; commits serialized
  luckily). LESSON (already in AGENTS.md, now enforced): touch .state/heartbeat
  around EVERY long shell command, not just periodically; and pgrep-check for a
  live continuation run before rewriting shared state files.
- Raw Ghidra dumps live in ghidra-project/ ROOT (exw-*.txt), not
  ghidra-project/analysis/ as older docs say.
