# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P2] Music small tails (one -process pass, no import): decompile
   FUN_0044c4a8 (sub-voice start: presumed SetFrequency 16.16 ratio + volume
   applier consuming 0045b03e/0045b042); xref census for header table C
   (0045cda8) reads, loop-flag writer 0045cdc0[song]=1, restart-word writer
   004543d4. Optional: plate comments for MusicPump/MrsChunkStart semantics.
2. [P3] Rust .MRS/.MRW decoders + stream-parse corpus test in
   engine/bedlam-assets (grammar: docs/RE-EXW-MUSIC.md sections 2/2b/3; also
   decode-song tool => duration/instrument table). Natural follow-on to the
   existing codec tests. The .PAL/.BIN fade-path findings (RE-EXW-TICK.md
   tick2 section) may inform palette handling.

## Backlog (not yet started)
- B2: run inspect over game-data-2 again after any decoder change (second fuzz
  corpus; decoders now live in engine/bedlam-assets - the corpus test pattern
  in engine/bedlam-assets/tests/corpus.rs is the model); document any B2-only
  format quirks in RESEARCH-BEDLAM2-CENSUS.md.
- B2: import BEDLAM.EXE (LE/DOS4GW) into Ghidra (needs LE loader handling);
  compare boot/init with EXW findings.
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Spec doc: input/control map - anchors: FUN_0041be05(vk,down) kb,
  FUN_0041bf35(btn,state) mouse, FUN_0044b4fc(x,y) cursor-per-tick,
  FUN_0044b428=CursorToGame (window->640x480 mapping - tick2 run)
  (see RE-EXW-MAINLOOP.md + RE-EXW-TICK.md).
- Cosmetic RE: create+decompile LAB_00451fbc (Watcom CRT thread trampoline,
  see RE-EXW-TICK.md tick2 section); name the 4 ddraw surface slots roles
  (back/front/...) via FUN_0044a9ac/FUN_0044ad18.
- bedlam-core crate skeleton (deterministic sim, replay, state hash per PLAN sec 7).
- bedlam-render/P3 design note (D9): renderer emits canonical 640x480 indexed fb;
  ALL resolution/scaling is presentation-layer only.

## Done (append)
- 2026-08-17 7406875 [P2] GameMain second hop + pacer CLOSED (pacer run,
  died before queue rewrite; follow-up run verified commit vs task text):
  mission loop FUN_0043d00b = poll -> sim/render -> PresentCopy ->
  g_frame_count++ -> PresentEnd, no software rate gate; Sleep and
  WaitForSingleObject each have exactly 1 non-loop caller (shutdown path /
  Watcom CRT recursive mutex); 0044ac5c = LockStaging (IsLost/Restore/Lock
  on staging surf, ptr cache 004ee9e8); 0044ad18 = DDFlipOrBlt (Flip/Blt +
  hw-cursor handshake); 00448ef1 = HEREIAM menu/high-score screen (divider
  reads = change-detection snapshots, not a rate gate). => D16: one
  sim/render frame per completed DD present = vsync-locked; parity = fixed
  60Hz sim timestep (D9/D12 split). Surface vtables uniform +4 (ONE extra
  slot @0x0c, 9 anchors) - supersedes tick2 +8/2-slot reading. Names:
  MemCopy/SurfaceLock/PresentCopy/PresentEnd/DDFlipOrBlt/AnimSprites/
  AnimEntities/DrawOverlays/PlayClockTick/GameGoRelease + g_frame_count
  etc. Docs: RE-EXW-PACER.md (new), DECISIONS D16, RE-EXW-TICK.md vtable
  corrections. Script: ExwPacerNames.java.
- 2026-08-17 07ce819+3bd9c22+1f05c37+8bc4cf1 [P2] EXW tick follow-ups ALL
  CLOSED (tick2 run, 2x -process passes, no import): FUN_00425901=FadeStep
  (50Hz palette-fade stepper over 768ch 16.16 accumulators @004edc38 ->
  004edc3c 6-bit -> SetPaletteRGB@0044aed4 upload -> decrement) =>
  **004ede10 = fade countdown, NOT a frame gate - D13 pacing claim withdrawn
  (D15); sim/render rate UNKNOWN again**; FUN_0044b428=CursorToGame
  (GetCursorPos -> window-rect-scaled 640x480 game coords - the scroll
  source is the mapped cursor, not key deltas); slot 00457874 ->
  ThreadSpawnImpl@0045204b = Watcom CRT _beginthread-style wrapper ->
  real CreateThread (IAT 00452f36) with CRT trampoline 00451fbc;
  AppActivate +0x18 = IDirectDrawPalette::SetEntries on 004ee9d0 (stock
  layout, 5-arg call (0,0,0xFE,&entries[1])); DDRAW init/shutdown chain
  mapped (DDCreate/DDInitSurfaces/DDShutdown); surface vtables +8-shifted
  past GetCaps vs stock IDirectDrawSurface (2 extra ddraw.h slots,
  cosmetic); GoFlagSet caller = FUN_0041e19d (closes gamethread open item 1);
  MusicPump gate 004edbe0 set at config load (FUN_004252c0). Names+labels
  persisted in BedlamWatcom (FadeStep, CursorToGame, SetPaletteRGB,
  FadeSetup, DDCreate, DDInitSurfaces, DDShutdown, ThreadSpawnImpl,
  g_fade_ticks_left, g_dd_obj/palette/clipper, g_thread_spawn_slot).
  Docs: RE-EXW-TICK.md (tick2 section + corrections), RE-EXW-GAMETHREAD.md
  (11 corrections), DECISIONS D15 (+D13 superseded flag). Manifest OK.
- 2026-08-17 17b8311 (+f7be649) [P2] .MRS event grammar FULLY DECODED +
  byte-validated all 5 files: event = u16 delta (10ms ticks) + opcode byte
  (<0x7F note [variant 0: byte=inst / variant 1: inst=variant+7, ratio =
  16.16 table @00454174[byte], tag=byte-0x54] + volume byte [0xFF=note-off];
  0x7F song-end; 0x80-0xFD rest; 0xFE/0xFF pattern RESTART on channel byte -
  not chunk jumps). Header tables resolved: +4+2W0 variant, +4+4W0 start-offset
  (0xffff=disabled; chunk 0 disabled in every file), +4+4W0+2W0W1 tick delay
  (chunk 1 = loop timer == song length exactly: 331/400/1476/1600/3388 ticks),
  table C never read. MusicPump 00402bac = song slot 3 only. 28/28 melody
  streams parse to the exact byte. Ratio table @00454174 extracted from EXW
  (1.0 @ byte 0x54, +18 st ceiling @ 0x66). Closes RESEARCH-8STREET Q4 (last
  music gap). CONFIG.BDL (Q7) was already closed prior run.
- 2026-08-17 a6697e6 [P1->P3] tools/inspect decoders promoted to workspace
  crate engine/bedlam-assets (pure buffer-in/out, thiserror, no-panic on user
  bytes; 70 unit tests + corpus integration test over a deterministic 80-file
  game-data sample: 70 ok/0 err, 13 byte-exact rebuilds, 20+20 codec
  round-trips). tools/inspect kept as thin CLI; full-corpus output proven
  byte-identical to pre-refactor HEAD. DECISIONS D14.
- 2026-08-17 9bb6793 [P2] EXW game worker-thread 0044dea0 decompiled: it is a
  59-byte trampoline -> GameMain@0041c050 (the real game shell/loop, decompiled
  + named). 8street 20fps claim REFUTED at this depth: no Sleep/timeGetTime on
  the game thread; pacing = 100Hz tick -> 50Hz gate 004ede10 (parity budget
  50Hz, D13 - NOW SUPERSEDED BY D15, see tick2 run above). Zone/level strides:
  7 zones x 5 levels, mission = clamp((zone-2)*5+level-1,1,26), completion
  table 17x12 @004decb2. RNG seeds 004ede48=123456 / 004ede4c=234567. Go-flag
  004ef674 writer set = {GameThreadStart=0, GoFlagSet=1}; 0044deca misread
  corrected (writes thread id 004ef694=-1). Names: GameThread, GoFlagSet,
  GameMain. Docs: RE-EXW-GAMETHREAD.md (new), RE-EXW-TICK.md corrections, D13.
- 2026-08-17 19cfb6f [P2] EXW 100Hz tick RE done (TimerCallback=service routine,
  100Hz mouse poll+clamp+store, 5 counters, scroll clamp 9..631/9..463,
  8-frame palette cycle 0x90-0x97 @12.5Hz); sim/render loop LOCATED at worker
  thread 0044dea0; F key = numbered-BMP screenshot; AppActivate = system-
  palette mgmt; names: TickWorker/MousePosHandler/ThreadSpawnThunk/FKeyHandler/
  AppActivate. Docs: RE-EXW-TICK.md (new), RE-EXW-MAINLOOP.md corrections, D10.
- 2026-08-17 0a27bf9 plan/D9: resolution independence (canonical 640x480 fb).
- 2026-08-17 d3c91f5 P2: EXW loop architecture refined (WinMain misname fixed,
  TimerCallback promoted+decompiled, 675-fn DB at docs/exw-functions.txt, D9).
- 2026-08-17 32cdd7b [P2] EXW main-loop RE done (boot chain, main@0044d6e8,
  init/pump/WndProc/timer anchored). Docs: docs/RE-EXW-MAINLOOP.md.

## Run notes
- 2026-08-17 20:2x (queue-housekeeping + music-tails run): found queue
  stale - pacer run committed 7406875 (D16) at 19:58 but died before
  rewriting NEXT.md. This run verified the commit contents against the task
  text, marked task 1 done, promoted music tails to top. Tree clean,
  manifest OK (1069 files) before work started.
- 2026-08-17 18:xx (tick2 run): clean run, 2 headless -process passes (no
  import), manifests OK. LESSON: pgrep -f analyzeHeadless FALSE-POSITIVES on
  the continuation agent own cmdline (the nudge prompt text contains the word
  analyzeHeadless) - filter out opencode2/fish processes before concluding a
  Ghidra run is live. Also: fish has no heredocs - use bash -c with quoted
  delimiter, and NEVER put apostrophes inside bash -c single-quoted strings
  (breaks the wrapper); python heredoc scripts in /tmp/opencode with
  assert-guarded unique-anchor replace + sys.exit(1) worked well for doc
  surgery. MAX_DERIVED cap in ExwTickFollowup2 cut the slot-target decompile
  (0045204b) - recovered by folding it into the ExwTickNames pass; keep
  derived caps generous or dump-and-follow-up.
- 2026-08-17 17:5x (mrs-grammar run): clean run, no Ghidra needed (prior run
  left complete dumps exw-music-events*.txt). Found prior run died ~15:58 with
  docs/RE-EXW-MUSIC.md modified but uncommitted - verified its claims against
  the dumps and committed it first (f7be649), then finished the decode.
  Validator methodology: python reimplementation of the MrsChunkStart /
  MrsNextEvent walk (scratch in /tmp, not committed; the grammar is fully
  specified in docs/RE-EXW-MUSIC.md 2b - port to Rust via the P3 backlog
  item). Two gotchas solved en route: (1) the three header table roles come
  from load_midi pointer math (variant @+4+2W0, start @+4+4W0, ticks
  @+4+4W0+2W0W1); (2) the >30000 delta check is SIGNED (0xFFxx words = freeze
  = natural end-of-stream, NOT a wrap). Manifest checked before+after
  game-data reads: OK.
- 2026-08-17 07:2x (assets-promotion run): found the prior run refactor
  UNCOMMITTED mid-flight (engine/ + tools/inspect edits, build already green).
  Verified rather than redid: cargo build/test/fmt/clippy clean; corpus test
  real (1069 walked, 80 sampled); full-corpus inspect output diffed against a
  HEAD git-worktree run = byte-identical (only summary.json "root" string
  differs when invoked with abs vs rel path - expected). Leftover
  .sprites/.images files inside derived/ predate HEAD (old experimental dump
  format; NOT lost output - HEAD does not emit them either). .state/fails and
  .state/spawns are nudge runtime counters -> gitignored.
- 2026-08-17 04:2x (tick run): CONCURRENT-AGENT INCIDENT - heartbeat went
  stale during ~2min headless run -> nudge spawned a second agent that
  rewrote RE-EXW-MAINLOOP.md mid-edit. No work lost. LESSON (now enforced):
  touch .state/heartbeat around EVERY long shell command.
- Raw Ghidra dumps live in ghidra-project/ ROOT (exw-*.txt), not
  ghidra-project/analysis/ as older docs say.
- 2026-08-17 04:5x (gamethread run): clean run, both manifests OK, two headless
  -process passes (dump+create, naming). 0044deca misread corrected - see
  docs/RE-EXW-GAMETHREAD.md. One doc edit self-inflicted (DECISIONS D12 tail
  briefly clobbered) - caught and restored before commit.
- 2026-08-17 18:2x (collision-watch run): DUPLICATE-SPAWN INCIDENT #2, no
  damage. The tick2 run never died - it was in a long NON-shell phase
  (writing the big RE-EXW-TICK.md edit), heartbeat went stale >7min mid-edit,
  nudge spawned a second agent (18:17:55). Second agent (this run) detected
  the concurrent writer (doc commit 07ce819 landed during its verification),
  stood down from shared-state writes, waited, verified the tick2 unit fully
  completed (doc+STATE+queue committed & pushed through 87a74cc). LESSON
  (extends AGENTS.md): touch heartbeat ALSO around long pure-edit phases
  (big doc rewrites), not just long shell commands - editing time counts.
  Minor cleanup: .state/last-progress gitignored (nudge runtime marker).
