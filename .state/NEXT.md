# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P3] bedlam-core crate skeleton (deterministic sim, replay, state hash
   per PLAN sec 7) - TIMING DESIGN per D17 (hybrid: fixed 60Hz sim
   accumulator, per-frame input/UI at refresh, satellite clocks as integer
   substeps; determinism test at 15/60/240Hz).
2. [P2] Tick satellite naming pass: continue labeling 0044xxxx service-tick
   callees in RE-EXW-TICK.md open list (FUN_00402bac gated pump chan 3,
   .data slot 00457874 consumers) - decompile + name + commit incrementally.
3. [P2] B2 prep: DOS4GW LE loader research for Ghidra (how to import
   BEDLAM.EXE LE format - loader options, scripts, prior art); document a
   concrete import plan in docs/RESEARCH-BEDLAM2-CENSUS.md. Read-only
   research + doc work, no Ghidra project changes.
## Backlog (not yet started)
- B2: import BEDLAM.EXE (LE/DOS4GW) into Ghidra (needs LE loader handling);
  compare boot/init with EXW findings.
- P4 prep: DOSBox-X AppImage download (user-level, no sudo), pinned Wine prefix for EXW.
- Cosmetic RE: create+decompile LAB_00451fbc (Watcom CRT thread trampoline,
  see RE-EXW-TICK.md tick2 section); name the 4 ddraw surface slots roles
  (back/front/...) via FUN_0044a9ac/FUN_0044ad18.
- bedlam-render/P3 design note (D9): renderer emits canonical 640x480 indexed fb;
  ALL resolution/scaling is presentation-layer only.

## Done (append)
- 2026-08-17 fe14416 [P4 prep] EXW input/control map CLOSED (docs/RE-EXW-INPUT.md,
  new). Pass A ExwInputSinks.java: KeySink@0041be05(scan,down) = 256B
  scan-code keystore @004edc44 (1=held; arrows 0x48/4b/4d/50 remapped +0x80
  -> 0xc8/cb/cd/d0) + 12 level-sampled edge-latch dwords (ESC/1-7/P/M|Space/
  F1-F3); MouseSink@0041bf35(btn,state) = g_mouse_flags@004dc6e4 bit0=left
  bit1=right, double-click events verified NO-OP; Alt(SC_KEYMENU)
  synthesized as scan 0x44. Pass B ExwInputReaders.java: listing+refs
  census, 17 readers decompiled (226 hits). Semantics: Up/Down arrows =
  music volume +/-5 clamp 0..100 via FUN_0044c630(vol>>1), repeat gate
  DAT_0046ae88; P latch = pause toggle (MissionShell@0044771c busy-waits
  for P again, clears all latches); FUN_0043a5fc = name entry (AnyKeyWait
  -> ScanToChar@0041fa02, Backspace 0xe, 8-char buf @004e444c); camera =
  cursor + mouse-drag only. HEADLINE: Left/Right arrow bytes (0x4edd0f/11)
  have ZERO readers - proven 3 ways (listing census, raw-image pointer
  probe, no Get*KeyState imports) => keyboard is hotkeys/volume/pause/
  any-key only, gameplay pointing is all mouse. MAINLOOP sec 6 corrected
  (0x101 = WM_KEYUP not SYSKEYDOWN; scan codes not vkeys). 26 names
  persisted (ExwInputNames.java). Manifests OK after run. OSGi LESSON:
  a script compile error inside analyzeHeadless surfaces ONLY as
  "class could not be found / Failed to get OSGi bundle" - if a new .java
  fails to load, javac it against the Ghidra jars first (CP = all lib jars)
  to see the real error (bit twice: bogus import, then Set<String>.add(
  Address)).
- 2026-08-17 cb45cf5 [P3->B2] B2 re-census with real mrs/mrw arms CLOSED
  (prior run died before queue rewrite; this run verified + pushed). Re-ran
  tools/inspect over game-data-2 -> derived-2: BYTE-IDENTICAL vs stub-era
  summary (989 files, 0 status diffs); decode-song has no B2 input (corpus
  has ZERO .MRS/.MRW; SOUND/MIDI/ exists but EMPTY - re-verified: 0 files,
  0 manifest entries). B1 census refreshed same run: derived/ now mrs:parsed
  5/5 + mrw:parsed 5/5 with loop ticks 331/400/1476/1600/3388 (re-verified
  in summary.json breakdown). B1 88.9% vs B2 90.0% parsed. B2 quirks
  documented in RESEARCH-BEDLAM2-CENSUS.md re-census section: MIRAGE/AB_BED
  second config dir (CONFIG.BDL 61B x2 different sha), PAL variant set
  B2-only DARKPALT/BRF_TX, pending:queued 3 scene/util files. MANIFEST
  verification re-run by this run: B1 OK 1069 entries (repo-root paths),
  B2 OK 989 entries (corpus-relative paths - run sha256sum -c from inside
  game-data-2). Note: MANIFEST-2.sha256 is deliberately gitignored
  (3994fb4 acquisition, corpus backed up) while B1 MANIFEST.sha256 is
  committed - asymmetry left as-is, flagged for a future decision.
- 2026-08-17 7325d23 + 3530a1b + 0af337f (+66601cf reverted) [P3] Rust
  .MRS/.MRW decoders + stream-parse corpus test + decode-song tool CLOSED.
  (a) engine/bedlam-assets/src/music.rs (commit 7325d23, concurrent
  sibling run): parse_mrs with exact layout validation + byte-exact
  to_bytes rebuild, Mrs::walk = full MrsNextEvent grammar (notes
  variant 0/1, song-end, rest, restart cond/uncond, freeze terminal,
  30001..32767 backward reposition with budget), RATIO_TABLE 128 dwords
  verbatim from EXW @00454174 (file off 0x52774), MRW moved out of
  misc.rs with wave_range + exhaustive layout check; corpus test pins
  chunk-0-disabled / chunk-1 loop timer == song length == first delta /
  terminal freeze on every enabled stream / insts < sibling n_inst /
  durations 331-400-1476-1600-3388. (b) decode-song CLI + inspect mrs
  arm upgrade + inspect lib.rs split (commit 3530a1b, finished + verified
  the sibling uncommitted WIP after its client died rc=1; output
  validated against an independent byte-level probe). (c) Revert
  0af337f removed THIS RUN duplicate parallel mrs.rs module (66601cf) -
  see incident note. Manifest OK. Docs: RE-EXW-MUSIC.md sec 3b.
- 2026-08-17 62c00c3+567808f+8ed8482+a397267 [P2] Music small tails ALL
  CLOSED (3 x -process passes, no import; queue-housekeeping commit f417543
  first recorded the prior pacer run 7406875/D16 that died before NEXT.md
  rewrite). (a) FUN_0044c4a8 = SubVoiceStart: five STOCK IDirectSoundBuffer
  vtable calls (NO shift, unlike +4 ddraw surfaces) - SetFrequency =
  (ratio * 11025) >> 16 (16.16 ratio x native rate), SetVolume =
  ((master@004ee9b4 * vol)/48 - 127)*2000>>7, SetPan(0), SetCurrentPosition(0),
  Play(0,0,0); full MusicPump -> MrsTriggerNote -> SubVoiceFind chain resolved
  (master vol setter FUN_0044c630 + 2 UI callers; note-off-releases-BASE quirk
  recorded for faithful impl). (b) table C (0045cda8) = write-only DEAD data
  (zero readers, full listing census). (c) loop flag 0045cdc0 never set to 1
  => 0xFE opcode DEAD; pending-restart 004543d4 initialized 0xffff + no
  setter => never fires. Bonus: word 004ee9b6 = palette-dirty flag
  (SetPaletteRGB sets, DDFlipOrBlt re-applies+clears - added to pacer doc).
  Names: SubVoiceStart/SubVoiceFind/SubVoiceProbe + g_music_loopflag/
  g_music_pending_restart/g_tableC_ptrs/g_song_inst_count/g_music_master_vol;
  plate comments on MusicPump/MrsChunkStart/MrsNextEvent/MusicStart/load_midi/
  mrw_load. Docs: RE-EXW-MUSIC.md sec 6 rewritten, RE-EXW-PACER.md handshake.
  LESSON: Ghidra getReferencesTo MISSES scaled-index operands ([EAX*2+base])
  - global censuses need a listing-text scan (ExwMusicTails3 pattern).
  Manifest OK.
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

- 2026-08-18 00:0x (spawn-storm watch run; stood down, NO work files
  touched): DUPLICATE-SPAWN #4 + CLIENT-DEATH STORM, contained, no damage.
  Sequence: v4.4 deploy raced in-flight v4.3 spawns - this run (item 3, v4.4
  prompt, placeholder 00:02:51) found the 00:01 v4.3 sibling already holding
  3-claim (00:02:48, live) -> stood down + deleted own placeholder per prompt.
  ~00:04: ALL THREE 00:01 clients died rc=1 (transport storm). Casualties:
  item-2 WIP = untracked tools/ghidra-scripts/ExwTickSats.java (written
  00:00:28, never run - free for the item-2 respawn); item-3 research lost
  with its session EXCEPT its transcript survives in .state/nudge-run.log
  (~00:03-00:04 tail): Ghidra ships NO LE loader - the LE stub class throws
  NotYetImplementedException (knows LE only to reject it); BEDLAM.EXE header
  verified MZ stub + e_lfanew=0x4a90 + LE sig; LX-offset probe +0x18/+0x1c/
  +0x20/+0x24/+0x28 reads coherent eipobj=1 eip=0x56a60 espobj=2 esp=0xb04ee
  pagesize=0x1000 (respawned item-3 agent: mine that log tail, then RE-DERIVE
  everything yourself before trusting it - objtab offsets in that probe were
  still wrong/unpinned). Controller respawned item 3 at 00:06:55 (fresh
  client + placeholder 3-1787004415.claim) = new owner; this run exited the
  lane for good. Separately, an item-1 worker is LIVE on engine/bedlam-core
  (M lib.rs/sim.rs/replay.rs + new frame.rs, mtimes advancing 00:07-00:09,
  uncommitted at time of note). STALE STATE at exit: 1-claim + 2-claim have
  dead owners (70-min reaper requeues); orphan placeholders 1-1787004232 +
  2-1787004060 (owners dead/redirected - left for TTL reap, do not count as
  live slots). LESSONS: (1) a stood-down agent that deletes its placeholder
  but writes no claim is INVISIBLE to the respawn logic - if adopting an item
  whose owner died, immediately rewrite the claim file (fresh mtime) BEFORE
  starting work, else you race the controller respawn; (2) on death-storm
  nights check claims mtimes + pgrep BEFORE any shared-file write; (3) nudge
  client rc=1 killed its server session tonight (unlike incident #3) - the
  00:01 trio left nothing running server-side.
- 2026-08-17 23:5x (input-map run): clean unit, 4 headless -process passes
  (no import), 3 incremental commits. Volume-control discovery links the
  input map to the music path (g_music_volume@004ddb2c, master vol =
  vol>>1 -> the 0..50 range seen in RE-EXW-MUSIC now explained: master
  vol setter takes 0..50). Left/Right-dead proof method (displacement
  census + pointer probe + import census) is reusable for other DEAD
  claims.

- 2026-08-17 23:2x (queue-verify run): found queue stale AGAIN (4th time) -
  prior run committed cb45cf5 (B2 re-census) at 23:12 but died before
  NEXT.md rewrite. This run verified every claim against artifacts before
  marking done: derived-2/summary.json (23:06) populated, 0 mrs/mrw kinds;
  derived/summary.json (23:08) mrs/mrw 5/5 parsed; both manifests verified.
  GOTCHA learned: MANIFEST.sha256 paths are repo-relative (check from repo
  root) but MANIFEST-2.sha256 paths are corpus-relative (check from inside
  game-data-2) - running from the wrong cwd spews 1000 FAILED lines that
  TRUNCATE the shell output and can swallow subsequent command results.
  Keep manifest checks in separate small commands.

- 2026-08-17 22:3x-23:0x (mrs-rust run): DUPLICATE-SPAWN INCIDENT #3, no
  damage, resolved same run. This run started on the P3 MRS/MRW task and
  spent ~6 min in ONE non-shell generation phase (writing the 650-line
  mrs.rs) -> heartbeat stale 357s -> nudge spawned a sibling at 22:35:46
  that took the SAME top task. Sibling committed 7325d23 (music.rs
  module, corpus test, MRW move) at 22:43; its client died rc=1 ~22:47
  (server session lost) leaving decode-song WIP uncommitted. This run
  had committed its own parallel mrs.rs (66601cf, 22:45) on top.
  Resolution: detected the live writer via file mtimes (formats/music.rs
  22:46:57), stood down from ALL writes, polled 6 min for silence, then:
  verified + committed the sibling WIP (3530a1b), reverted the duplicate
  (0af337f), fixed default-run=inspect. LESSONS (extend the heartbeat
  rule): (1) touch heartbeat around long file-GENERATION phases too -
  generating one big source file is as silent as a big doc edit;
  (2) nudge rc=1 client death does NOT stop a server-side session - a
  concurrent writer may still be alive: check file mtimes + git log
  before touching shared files (>=6 min quiet + no commits = dead);
  (3) when two runs produce parallel implementations, canonical = the
  earlier commit that is integrated with tests/call sites - revert the
  later duplicate, do not merge both.
- 2026-08-17 20:2x (queue-housekeeping + music-tails run): found queue
  stale - pacer run committed 7406875 (D16) at 19:58 but died before
  rewriting NEXT.md. This run verified the commit contents against the task
  text, marked task 1 done, promoted music tails to top. Tree clean,
  manifest OK (1069 files) before work started. SAME RUN then closed the
  music tails unit (see Done above): 3 clean -process passes
  (exw-music-tails{,2,3}.txt). Script-compile lessons: Ghidra API is
  getScriptArgs (not getScriptArguments), and monitor.checkCanceled in a
  helper needs throws Exception. Fish quote gotcha bit once: a single quote
  in the bash -c verification text AFTER the heredoc broke the wrapper
  BEFORE the heredoc ran (file silently not written) - keep verification
  commands quote-free or run them separately.
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
