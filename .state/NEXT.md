# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P1->P3] Promote the tools/inspect decoders into a proper workspace crate
   engine/bedlam-assets (lib + unit tests + round-trip test over a sample of
   game-data, manifest-checked). Keep tools/inspect as a thin CLI over the crate.
2. [P2] RE the .MRS loader in EXW (open questions in RESEARCH-8STREET.md) to close
   the last mission-format gap; likewise CONFIG.BDL (61B).
3. [P2] Tick follow-ups from docs/RE-EXW-TICK.md open list: FUN_00402bac gated
   pump (20x38B records, chan 3), FUN_00425901 50Hz update, FUN_0044b428 scroll
   delta source, resolve .data slot 00457874, DDRAW vtable +0x18 mapping.
4. [P2] GameMain second hop (from docs/RE-EXW-GAMETHREAD.md open list):
   decompile FUN_0043d00b (reads 50Hz gate 004ede10 - the per-frame sim/render
   step; settle whether the 50Hz gate is further subdivided) + FUN_00440e45
   (zone/level manager) + find GoFlagSet@0044d9b4 caller (releases TimerInit
   spin). RNG function consuming seeds 004ede48/004ede4c.

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
- 2026-08-17 9bb6793 [P2] EXW game worker-thread 0044dea0 decompiled: it is a
  59-byte trampoline -> GameMain@0041c050 (the real game shell/loop, decompiled
  + named). 8street 20fps claim REFUTED at this depth: no Sleep/timeGetTime on
  the game thread; pacing = 100Hz tick -> 50Hz gate 004ede10 (parity budget
  50Hz, D13). Zone/level strides: 7 zones x 5 levels, mission =
  clamp((zone-2)*5+level-1,1,26), completion table 17x12 @004decb2. RNG seeds
  004ede48=123456 / 004ede4c=234567. Go-flag 004ef674 writer set =
  {GameThreadStart=0, GoFlagSet=1}; 0044deca misread corrected (writes thread
  id 004ef694=-1). Names: GameThread, GoFlagSet, GameMain. Docs:
  RE-EXW-GAMETHREAD.md (new), RE-EXW-TICK.md corrections, DECISIONS D13.
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
- 2026-08-17 04:5x (gamethread run): clean run, both manifests OK, two headless
  -process passes (dump+create, naming). Prior-run misread corrected: 0044deca
  writes thread id 004ef694 (=-1), NOT go flag 004ef674 - see
  docs/RE-EXW-GAMETHREAD.md. One doc edit self-inflicted (DECISIONS D12 tail
  briefly clobbered) - caught and restored before commit.
