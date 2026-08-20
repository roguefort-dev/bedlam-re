- CLOSED 2026-08-20 (P5 cutscene movies + corpus inventory, D32): every
  game-data SMK inventoried and PINNED (bedlam-assets smk_corpus_gate:
  34 files, formats/rates/ring/y-scale/audio shapes; listing must match
  the table both ways). Reject-or-map verdict: ALL MAP onto the D31
  playback path, none rejected - y-scale None corpus-wide (no scaling
  logic owed), all periods exact on the x240-us grid, the single audio
  shape (DPCM mono 8/11025) is already stream-bus-native. Movie
  selection module (bedlam-game movies.rs): cutscene_name over the
  hashed stage (ZONEDONE.SMK; END.SMK at the endgame = stage >=
  MAX_STAGE, EXW pre-increment vs FSM post-increment reconciled and
  unit-pinned through the FULL_MASK cadence), Region (DAT_0046ae64)
  backing LOAD_UK/US.BIN + LOADPAL(U).PAL + LOGO/GTLOG variants,
  briefing_name over BRF_{B..F}{1..5}. Host wiring:
  GameHost::cutscene_name + load_cutscene = the D31 lifecycle on
  Scene::Cutscene (inert-until-scene, dropped on exit, hash-free).
  Workspace 257 tests green, fmt + clippy -D warnings clean,
  MANIFEST.sha256 verified before AND after the corpus runs. Next:
  Shop/Brief backdrop wiring, then the post-cutscene loading screen.
# STATE - project snapshot (update when phase changes)

- CLOSED 2026-08-19 (P5 title-movie playback, D31): TITLE.SMK plays
  through GameHost end-to-end - MoviePlayer fixed-step x240-us clock,
  compose-level MovieFrame (scene pipeline replaced while a movie
  plays, centered letterbox, palette fold PALMAP>>2 lossless), mixer
  PCM stream bus (native u8 mono 11025 Hz FIFO under voices, loud
  16 MiB cap), inert-until-scene host lifecycle with scene-hash
  isolation pinned. Full-playback gate green (pacing exact vs the
  accumulator math, composite byte-identical to an independent
  SmkStream walk, two playbacks identical). Workspace 280 green,
  fmt/clippy clean, manifests OK x2. Next per PLAN sec 6 P5: extend to
  cutscene movies + per-zone parity gates.

- CLOSED 2026-08-19 (P4 SMK decode gate, smk-stream unit): headless TITLE.SMK
  decode gate green via the codec-neutral SmkStream seam (D30) over vendored
  smk 0.1.0 - 640x320, 1227 frames, 66660us/frame, DPCM mono 8-bit 11025 Hz
  track 0; two full decode passes byte-identical (video/audio SHA-256 chains
  in NEXT.md run notes); vendored backend DPCM panic patch documented in
  bedlam-smk/NOTICE.md. fmt/clippy/tests green, manifests OK. Next phase per
  PLAN sec 6: P5 playback integration (TITLE.SMK into GameHost/presentation).

- CLOSED 2026-08-18 (P2 cosmetic tail, 119ba2d+b6620c0+007fbe5+4ace8a6):
  B2 census sec-7 residuals ALL CLOSED (census sec 7.7a-e). Campaign
  tables byte-pinned (order[8] = {3,0,1,5,9,13,17,21}; full 27-step
  idx list; 25 distinct indices = union over stages 1..7). 25-vs-27
  RESOLVED by static arithmetic - no playthrough needed: linear counts
  completions (27), formula indices are distinct table slots (25); the
  gap = two endgame completions at stage-slot 8 via the OOB order[8] =
  zone[0] sentinel hop (0x81dba + 8*4 = 0x81dda exactly). 4f02 =
  BANKED 0x101 (BX verbatim caller passthrough at 0x12439, zero 0x4101
  constructions in the 671-fn sweep, g_lfb_ptr + g_vesa_mode_req
  write-only dead). Display start 0x200 = SCANLINE units (page-B bank 5
  = 0x50000 = 0x200 x 640-byte pitch; 4f07 DX-scanline form). B2 fade
  chain named + documented (B2FadeStep@0x126c8 8.8-fixed 768ch serviced
  at 50 Hz in the ISR &1 sub-block - RATE CORRECTED on close-out verify,
  identical to EXW 200 ms fade, no divergence; setup/cancel/dacread/
  dacupload/fadewait + 3 labels persisted;
  B2LblFix repaired 2 mislabels, primaries restored). Persistence
  re-verified 14/14 (B2ResidVerify). No import (1x -process
  -noanalysis); manifests OK x2. P2 cosmetic queue EMPTY; P4 runtime
  half remains, interactive-gated.

- CLOSED 2026-08-18 (P2 cosmetic, 8f5f18f+94a65da): EXW DD surface
  creation-order CONFIRMED (RE-EXW-TICK new section): 004ee9bc =
  flip-chain head/primary; 004ee9c0 = implicit backbuffer (fullscreen
  GetAttachedSurface) / offscreen staging (windowed) - g_dd_surf_staging
  correct in both modes; FUN_0044a9ac = DDStagingProbe (sentinel
  survive-a-flip readback -> g_staging_persistent 004ee9e4); 004ee9b4
  dual-use corrected (lo = master vol, hi = palette re-attach flag;
  RE-EXW-MUSIC addendum). Trampoline CrtThreadTrampoline@00451fbc +
  usage roles were already persisted by the tick-sat run; this pass
  added the creation-order proof + names. No import; manifests OK x2.
  P2 cosmetic queue now: only the B2 census sec-7 residuals item (in
  flight). P3 charter complete; P4 runtime half still interactive-gated.

- CLOSED 2026-08-18 (P4 kickoff code half, c61d7f7): headless parity
  harness v0 example landed (engine/bedlam-game/examples/parity_harness.rs,
  D28): GameHost driven end-to-end over a recorded input script, JSON
  report with per-tick scene-hash chain + frame parity + sim hash + audio
  stream hash; .MRW banks loaded per track (audible baseline); verified
  byte-identical across runs; fmt + clippy -D warnings clean; workspace
  204 green unchanged; manifests OK x2. P4 runtime half (wine/DOSBox
  comparisons vs this CPU baseline) = next, needs interactive desktop.

- CLOSED 2026-08-18 (game unit, 4ab051c+7e3e472): P3 CHARTER SET COMPLETE.
  bedlam-game = the LAST charter crate (assets/core/render/platform/audio/
  game all landed as skeletons). Scene FSM (10 scenes, B2 episode shape
  {stage,mask,linear} + FULL_MASK@0x81d9a, D26 hashed per-tick edge
  latches), host pump in FUN_0043d00b order, MusicPump bridge (D27
  melody-chunk + attach-anchored mixer dispatch), typed OPTIONS.BDL.
  Workspace 204 tests green, fmt + clippy -D warnings clean, manifests
  OK x2. Next phase per PLAN sec 6: P4 (harness/playable) - first item
  = dependency/version spike + runtime smoke, needs interactive desktop
  for wine-exw (do NOT run unattended).

- CLOSED 2026-08-18 (P4 runtime unit, unattended subparts, 79227e5+11c8d9c+b951e7c):
  D28 anchors REPRODUCED byte-identically x2 runs (scene
  0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame 0x87263f149564ad25,
  audio 0xc862e45d2e95ad29; reports cmp-identical). DOSBox-X harness
  LANDED: flatpak static-home finish arg DISCOVERED (per-dir :ro grants
  illusory) -> sandbox hardened (home revoked, runtime-only, verified via
  flatpak info), corpus via rsync scratch copy, pinned conf (svga_s3/
  core=normal/cputype=pentium/cycles=fixed 60000/vmemsize=2/scaler=none/
  sample-accurate sb16), driver prepare/smoke/shell/game, watch skeleton
  (census-verified watch set; PresentFlip frame trigger; 3 ghost addresses
  dropped), HEADLESS SMOKE GATE PASS first-hand (SMOKETST.TXT lists both
  EXEs). D29. Interactive half still gated: wine EXW launch + DOSBox-X
  golden-run calibration/checklist (RUNTIME.md follow-ups).
  Post-restart re-verification 17:56-18:0x (worker 1787068533):
  smoke gate re-run FIRST-HAND - PASS (rc=0, both EXEs at pinned
  sizes), sandbox posture verified via override file + flatpak
  override --show --user (!home + runtime only; note: without
  --user the CLI prints empty under env-based XDG_DATA_HOME),
  manifests OK x2 bracketing - harness stack stable across the
  4th restart of this lane.

- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
  promoted to workspace crate engine/bedlam-assets (pure, inspect CLI output
  byte-identical, D14); MUSIC FORMATS DECODED IN RUST 2026-08-17: music.rs
  module (MRS container + full event-stream walk + RATIO_TABLE verbatim from
  EXW, MRW bank with wave ranges, byte-exact rebuilds) + decode-song CLI +
  inspect mrs dumper + corpus invariants (see RE-EXW-MUSIC.md 3b). EXW outer architecture +
  100Hz tick + game worker thread FULLY mapped (GameThread@0044dea0 = 59-byte
  trampoline -> GameMain@0041c050 = real game shell/loop; 7x5 zone/level
  structure; RNG seeds 123456/234567). RATES (D15): 100Hz service tick /
  50Hz palette fade while fading / 12.5Hz palette cycle; 004ede10 = fade
  countdown (NOT a frame gate - D13 50Hz parity claim withdrawn); sim/render
  rate UNKNOWN pending FUN_0043d00b/FUN_00440e45 bodies. Tick satellites
  fully mapped: fade engine (FadeStep/FadeSetup/SetPaletteRGB), CursorToGame
  (window->640x480), DDRAW init/shutdown chain + object slots, thread spawn
  via Watcom CRT ThreadSpawnImpl@0045204b -> real CreateThread. Names applied in BedlamWatcom project (WinMain..
  AppActivate, TickWorker.., GameThread/GoFlagSet/GameMain - see
  docs/RE-EXW-MAINLOOP.md, docs/RE-EXW-TICK.md, docs/RE-EXW-GAMETHREAD.md).
  EXD import still pending.
- CLOSED 2026-08-18 (b2-import run): B2 DOS IMPORT DONE - ghidra-lx-loader
  built from source vs our exact 12.1.2 install (zero version risk),
  installed to userSettings/Extensions; import command + 3 gotchas in
  RESEARCH-BEDLAM2-CENSUS.md sec 5 (-loader LeLoader forced; MzLoader
  otherwise claims LE first). BedlamWatcom:/BEDLAM.EXE analyzed: 671 fns,
  blocks 0x10000/0x80000-0x1304ee, entry 0x66a60, 24041 applied fixups.
  First cross-build parity fact: RNG seeds 123456/234567 identical in B2
  (FUN_0002f731 game-init) and EXW (004ede48/4c). B2 pipeline = -process
  BEDLAM.EXE -noanalysis from here on (NEVER re-import).
- CLOSED 2026-08-18 (b2 entry/tick run, 2df7664+c3b1552+9b4d119): B2
  entry chain named + TICK SOURCE FOUND + zone/mission stride located
  (census sec 6, D22). _entry@0x66a60 -> CrtInitChain@0x6b1bc (argc/argv
  g_argc@0x1280d4/g_argv@0x1280d8) -> GameInit@0x2f731 = boot + episode
  loop shell (seeds RNG 123456/234567 as code constants at 0x11ef1c/18).
  Tick = 100.01 Hz PIT INT-8 ISR (divisor 0x2e9b, DOS INT21 AH=25h vector,
  immediate EOI, drop-not-queue reentrancy): 7 counters, 12.5 Hz palette
  banks 0x90..0x97 (same as EXW), 50 Hz mouse poll+clamp vs 320x240 coords,
  play-clock divider; present = vblank double-poll 0x3da (WaitVRetrace).
  Same two-clock architecture as EXW -> D16 parity budget carries to DOS.
  Zone/mission = lookup tables (order[8]@0x81dba, zone letters@0x81dda,
  mission[27]@0x81e46; +5 when mode==2 -> MISSION{6,7} corpus files; 6
  zones x {4 regular + 2 alt}, 27 linear missions). 15 fns + 16 labels
  persisted in BedlamWatcom:/BEDLAM.EXE.
- CLOSED 2026-08-18 (miri+hash-CI run): PLAN sec 7 DETERMINISM CI GATE DONE
  (1501ab9 + 014597b). (a) Miri CLEAN on this host: rustup component add
  --toolchain nightly-x86_64-unknown-linux-gnu miri (miri 0.1.0
  771916f902 2026-08-08, on the existing nightly; rustc 1.99.0-nightly
  b07e5a086 2026-08-07), then cargo +nightly miri test -p bedlam-core =>
  41 unit + 12 determinism tests green, ZERO UB findings (111.5s + 40.9s;
  re-run with the new fixture green too). (b) Committed per-tick hash
  fixture: engine/bedlam-core/tests/hash_fixture.rs - 600-tick fixed
  integer script (seed 123456, fade window armed ticks 101..200) pins 13
  milestone StateHash values + FNV-1a chain over all 601 hashes
  (EXPECTED_CHAIN 0x760d221bec3b3b99); runs in the ordinary cargo test
  matrix => cross-OS/toolchain hash drift fails loud per tick; ignored
  print_fixture is the ONLY documented regeneration path (intentional
  hashed-state changes + FORMAT_VERSION bump). (c) ci.yml miri job:
  ubuntu-latest, dtolnay/rust-toolchain@nightly + miri component,
  cargo +nightly miri test -p bedlam-core per push/PR. Workspace now 154
  tests green (fixture +1), fmt + clippy -D warnings clean, manifests OK
  x2. Next P3: bedlam-audio mix-graph skeleton (design note first), then
  bedlam-game scene-FSM skeleton.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
  NOTE: touch heartbeat around every long shell command (Ghidra ~2min) or a
  second agent gets spawned mid-run (happened 2026-08-17, see NEXT.md run notes).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- CLOSED 2026-08-17 (input-map run): EXW input/control map - scan-code
  keystore @004edc44 (arrows +0x80 remap), 12 edge latches, mouse flags
  @004dc6e4 (dbl-click dead), Up/Down=volume P=pause, Left/Right arrows
  DEAD (3-way proof), camera=cursor+drag only - docs/RE-EXW-INPUT.md.
- Known open: GameMain second hop - FUN_0043d00b (per-frame sim/render; its
  004ede10 read = fade-status, real rate mechanism unknown - D15) +
  FUN_00440e45 (zone/level manager) + divider consumers (FUN_00448ef1,
  FUN_00402b48); music chain FULLY closed incl. sub-voice start path (SubVoiceStart = SetFrequency ratio*11025 / SetVolume / SetPan / Play; table C + 0xFE loop flag + pending-restart all DEAD); .BLD/.CTG (editor-only), PAL variant
  renderers, EXD import (needs LE loader ext), goldens pipeline (P4).
  Parity budget: NO committed logic rate (D15 withdrew D13 50Hz).
  CLOSED 2026-08-17 (tick2 run): GoFlagSet caller = FUN_0041e19d; fade
  engine, cursor mapping, DDRAW init chain, thread-spawn slot all mapped
  (docs/RE-EXW-TICK.md tick2 section).
- CLOSED 2026-08-17: music format chain fully RE-d + byte-validated
  (.MRW layout, .MRS container + complete event grammar, MusicPump=song 3,
  ratio table @00454174; CONFIG.BDL = installer SB-setup record, EXW never
  reads it; RNG seeds consumed by RandA@00402975 / RandB@004029b6) - see
  docs/RE-EXW-MUSIC.md.

- CLOSED 2026-08-18 (bedlam-core run): P3 sim core skeleton DONE in
  engine/bedlam-core (f15eb60+7396491+889cbef) - D17 hybrid timing: hashed
  60Hz Sim (300Hz microstep satellites per DESIGN-RENDER sec 6) + non-hashed
  per-frame FrameState + 240Hz sub-tick SimDriver accumulator; PCG32, Q16.16
  fx, in-crate FNV-1a state hash, versioned b"BDLR" replay + b"BDLS"
  snapshot; 132 tests green, clippy clean, manifest OK. Next P3: render
  crate skeleton (design note a3ad066), Miri + cross-OS hash CI.

- CLOSED 2026-08-18 (episode-loop run, 928748d+7bfac4b+aff1ae8 + adopted dead-run
  B2EpisDump): B2 EPISODE LOOP + COUNTER VERDICTS DONE (census sec 7, D23).
  All 7 INT8 counters classified - NONE gate sim/render (2 audio bases
  0x801a6/0x80010, 2 DEAD 0x11f158/0x11f0b4, ISR phases 0x11f0c8, 100Hz
  timeout base 0x11f0c4 w/ WaitTicks100Hz, 50ms delay 0x11f0b0). Mission loop
  = present-paced VESA page flip + vblank (D16 architecture CONFIRMED on
  DOS, D23). Episode progression: linear 0..26 + per-stage-slot completed
  mask (full-mask table 0x81d9a) + stage-slot advance w/ zone-complete
  cutscene; sub = PLAYER-selected in MapRoomSelect (mission-select UI, BRF_*
  backdrops); saves = 5 x 61B records {mask,slot,linear,money,stats}.
  B2 audio = IRQ0-shared 11025 Hz PCM driver (PIT reprogram on arm; stub
  ms-clock x10 vs hi-res tick+PIT-phase) - same native rate as EXW.
  Video = VESA 0x101 640x480x8, dual pages bank {0,5} display-start {0,
  0x200}, 640-byte stride + 320x240 logical space = 2x scale. Zone letters
  dword[0]=25 = sentinel (unreachable index). 30 fns + 33 labels persisted;
  orphan stub/driver callbacks created as functions. Open residuals queued:
  27-vs-25 step accounting, LFB-vs-banked 4f02 variant, 0x200 units,
  FUN_000126c8 satellite.
- CLOSED 2026-08-18 (render+platform unit): P3 PRESENTATION SKELETONS DONE
  (ff8fb17 + d2b7fb8, D24). engine/bedlam-render = pure state->canonical
  640x480x8 Frame + 6-bit palette + FNV parity hash, fixed pass order
  (world->sprites->rows->overlays->entities), camera clamp, palette_dirty
  derivation, 12 tests. engine/bedlam-platform = pure scale/uv geometry
  (Integer default/Fit/Fill) + wgpu 27.0.1 parity pipeline (index tex per
  frame + packed palette tex on dirty + fullscreen-triangle WGSL
  palette-expand/scale, Original v<<2 default), offscreen GPU round-trip
  test that skips without an adapter, 9 tests. Workspace 153 green, clippy
  -D warnings + fmt clean. Provenance: code landed by the 03:00 worker
  whose client died transport rc=1 at 03:05 while its server session
  finished both commits (03:07, 03:17) then died before the queue rewrite;
  the 03:32 respawn verified the work (153 green incl. real GPU test,
  fmt/clippy clean, manifests OK x2) instead of redoing it and closed the
  unit. Next P3: Miri over bedlam-core + per-tick hash CI job.
- CLOSED 2026-08-18 (audio unit, triple-agent night): P3 AUDIO MIX-GRAPH
  SKELETON DONE in engine/bedlam-audio (846ebab + b684bee + 00c2260 +
  b950b44 + a8f26f8). DESIGN-AUDIO.md pinned first (mix topology voices ->
  master bus -> device; 11025 Hz native both builds; Q16 tick grid 441/4
  samples = exact; D25 linear-Q8 volume over the EXW (master*vol)/48
  product, dB curve documented not reproduced; note-off-releases-BASE
  quirk kept; audio NOT hashed per D17 b - byte-identity of the mix stream
  is the gate). Crate: hermetic integer Mixer (forbid unsafe, no floats,
  no I/O/clock), flat 20-voice pool (B2 walker) tagged (instrument, sub
  0..3) (EXW mrw 4 sub-voices), 16.16 phase step = RATIO_TABLE verbatim,
  Q8 volume x pan gains snapshotted at spawn (EXW reads master per
  SubVoiceStart only), i32 bus + symmetric clamp, S16 stereo interleaved
  out; MusicScript = absolute-tick NoteOn/NoteOff list with
  no-bedlam-assets coupling (mapping lands in bedlam-game); render
  dispatches events at exact Q16 positions chunking-invariantly.
  9 unit + 14 determinism tests (same script => byte-identical buffer
  across 1/7/64/512-frame chunkings, base-only note-off, drop-when-full,
  one-shot recycling, saturation clamp, tick-grid exactness at frame 441),
  workspace 177 green (+23), fmt + clippy -D warnings clean, miri CLEAN
  (9+12 tests, zero UB; integration suite ~292s under miri - ci.yml miri
  job extended to -p bedlam-audio, acceptable CI cost). DECISIONS D25.
  Deliverable survived a duplicate-spawn storm (three agents on item 1:
  0162 silent death, 0711 = this run, 1260 transport death mid-verify; a
  watch run contaminated then cleaned the lane and deleted the uncommitted
  test file - regenerated from the /tmp/opencode generator; boundary bugs
  it would have caught were caught by the restored suite: immediate
  one-shot free + event-on-exact-boundary ordering). Next P3: bedlam-game
  scene-FSM skeleton (LAST charter crate).

