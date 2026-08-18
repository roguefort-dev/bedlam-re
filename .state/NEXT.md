# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [P2 cosmetic follow-up, census sec 7 residuals] [IN FLIGHT - live
   claim 2-owner.claim, agent running B2Residuals ghidra pass as of
   ~15:5x; do NOT double-claim - verify pgrep/git-log first] B2 campaign index
   accounting (25 reachable zone-letter indices vs 27 linear steps -
   needs save-file/playthrough check), 4f02 LFB-vs-banked variant +
   0x200 display-start units, FUN_000126c8 satellite + its 0x11ef88
   gate. Ghidra -process BEDLAM.EXE -noanalysis ONLY. If the
   playthrough check needs a live game run, STOP - that is interactive.

## Backlog (not yet started)
- P4 RUNTIME HALF (next after the cosmetic items): interactive EXW smoke
  launch under tools/runtime/wine-exw.sh (needs desktop + DirectDraw - do
  NOT run unattended); then the wine/DOSBox runtime comparison against the
  parity harness CPU baseline (report + hashes recorded in the D28 queue
  entry below; reproduce with: cargo run --release --example parity_harness
  -p bedlam-game -- --out report.json). Plus flathub sandbox filesystem
  override for game-data + DOSBox-X harness config (cycles pinned,
  debugger watch scripting) when that work starts.

## Done (append)
- 2026-08-18 8f5f18f+94a65da [P2 cosmetic RE] EXW DD surface
   creation-order CONFIRMATION CLOSED (the re-queued trampoline+roles
   confirmation unit; lane survived TWO server restarts - the 15:33
   worker died progress=0 BUT its analyzeHeadless dump pass survived as
   an OS orphan and finished the dump at 15:36; the 15:48 respawn
   verified + adopted it per the queue-verify precedent, then ran the
   persist pass itself). (a) Persisted-state re-verification (dump
   header): CrtThreadTrampoline@00451fbc + all four g_dd_surf_* labels
   from the tick-sat run still in place - trampoline half of the item
   was already done, only the roles confirmation remained. (b)
   DDInitSurfaces creation order [verified vs listing]: FULLSCREEN
   CreateSurface dwSize 0x6c flags 0x21 caps 0x4218
   (COMPLEX|FLIP|PRIMARYSURFACE|MODEX; retry clears exactly bit 0x4000
   via desc+0x69 AND 0xbf -> 0x218, MODEX inoperative at 640x480)
   backbuffercount 1 -> 004ee9bc = FLIP-CHAIN HEAD/primary, then
   +0x30 GetAttachedSurface(DDSCAPS_BACKBUFFER 4) -> 004ee9c0 = the
   implicit BACKBUFFER; WINDOWED CreateSurface(caps 0x200) -> 004ee9bc
   then CreateSurface(flags 0x7 caps 0x40 OFFSCREENPLAIN w x h) ->
   004ee9c0 = offscreen staging; palbank 004ee9c8 / cursor 004ee9cc
   creators = tail calls FUN_0044b9c4/FUN_0044ba3c [inferred, not
   decompiled - usage roles from tick-sat stand]. g_dd_surf_staging
   label CONFIRMED correct in both modes. (c) FUN_0044a9ac renamed
   DDStagingProbe (2 callers = DD-init chain after DDInitSurfaces +
   DDShutdown): sentinel 0x12345678 written via LockStaging (retry 20),
   Unlock+FlipOrBlt, relock+readback -> word 004ee9e4 = 1 when staging
   memory SURVIVES the present (g_staging_persistent), then double
   full-surface clear (480 rows x 160 dwords, stride pitch/4). (d)
   DDFlipOrBlt dump-confirmed: Flip@+0x2c (D16 authority) fullscreen,
   Blt@+0x14 staging 004ee9c0 -> primary windowed w/ window-rect cache
   + DDBLT_WAIT; 004ee9b4 DUAL-USE correction: lo word = master volume
   (setter FUN_0044c630 writes AX only), hi word = palette re-attach
   request masked off after SetPalette (RE-EXW-MUSIC sec 6 addendum -
   SubVoiceStart passes the full dword so the SetVolume product is only
   clean while the flag is clear). (e) Persist pass ExwSurfNames: rename
   + DDInitSurfaces plate comment + labels g_staging_persistent
   (004ee9e4) + g_pal_dirty (004ee9b6), Save succeeded. Docs:
   RE-EXW-TICK.md new section + 4 globals-table rows; RE-EXW-MUSIC.md
   addendum. Scripts ExwSurfConfirm/ExwSurfNames committed (8f5f18f),
   docs 94a65da, both pushed. Manifests OK x2 before+after; NO import
   (2x -process BEDLAM.EXW -noanalysis total).

- 2026-08-18 c61d7f7 [P4 kickoff, code-only half] headless parity
   harness v0 CLOSED (engine/bedlam-game/examples/parity_harness.rs;
   D28; the 14:27 respawn after FIVE transport deaths this hour - all
   five predecessors died during read-only recon, nothing was adopted,
   this run wrote the unit fresh). (a) FsSource ByteSource rooted at
   game-data/BEDLAM (top-level then SOUND/MIDI); the crate stays
   hermetic - fs lives ONLY in the example. (b) text input script
   grammar (step/act/music; every numeric hex-aware; embedded default
   walk) + --root/--script/--out/--dt args. (c) pump: fixed dt (4
   subticks = 60 Hz), per-tick scene_hash chain + FNV chain, final
   Frame parity_hash, sim state hash, 184-sample/frame audio pull +
   full-stream FNV. (d) sibling .MRW bank loaded per track (54 waves,
   5 tracks) so the audio baseline is AUDIBLE - without banks the
   stream is all-zero and degenerate (D28 pin 3). (e) embedded walk:
   boot, Options/Brief (REAL mouse click, D26 edge path)/Select/
   Mission/Debrief/Cutscene + a stage-2 lap landing in Shop so ALL 5
   tracks get audible windows; transitions recorded per frame. Gates:
   fmt + clippy -D warnings clean, workspace 204 green UNCHANGED,
   report byte-identical across runs, manifests OK x2. BASELINE (seed
   0, dt 4, 775 frames = 775 ticks): scene chain 0xcae25cd08d7cbc08,
   sim 0x72979d5d9dedc832, frame parity 0x87263f149564ad25, audio
   stream 0xc862e45d2e95ad29 (176994 nonzero samples, 142600 frames
   mixed). These are the P4 diff anchor - drift with unchanged inputs
   = engine determinism bug.


- 2026-08-18 efd270e+58e3f75+4ab051c+7e3e472 [P3] bedlam-game scene-FSM
   skeleton CLOSED - THE LAST P3 CHARTER CRATE (P3 charter set now
   complete: assets/core/render/platform/audio/game). Quad-session
   lane: 11:19 claimant pinned DESIGN-GAME (a: FSM topology anchored
   GameMain@0041c050/FUN_0043d00b + B2 GameInit@0x2f731/MapRoomSelect@
   0x50a87, host pump, music bridge, config model, D17 boundary);
   13:16 + 13:20 respawns wrote the crate WIP and died on transport
   (twice); this 13:43/13:51 run ADOPTED the WIP per the ghost-survivor
   protocol - verified line-by-line vs the note, deleted the scratch
   zz_debug.rs, fixed the three REAL test bugs (harness sampling
   duplicating checkpoints on 0-tick banking frames; corpus terminal
   assert missing the chunk-1 Restart loop-timer fact vs
   validate_mrs_song; chunking-invariance comparing untruncated
   overshoot buffers) + one clippy nit. (b) engine/bedlam-game:
   fsm.rs Scene enum (10 scenes) + transitions + Episode {stage,mask,
   linear} with FULL_MASK@0x81d9a verbatim + zone-complete gating +
   scene_hash FNV-1a tag BDLG; host.rs GameHost::pump_frame in
   FUN_0043d00b order (SimDriver -> per-executed-tick SceneFsm ->
   render -> Frame; music sync per scene) + ByteSource/ByteSink
   injection; music.rs MusicPump bridge (D27 melody chunk) +
   build_script Mrs->MusicScript + track table OPTIONS/BRIEF/SELECT/
   DEBRIEF/SHOP.MRS; config.rs typed OPTIONS.BDL (volume 0..=100,
   music_master = vol>>1 = FUN_0044c630, 41B round-trip; CONFIG.BDL
   never modelled). thiserror only, forbid(unsafe), no fs/clock/
   threads. CROSS-CRATE FIX in bedlam-audio: load_script anchors
   dispatch at the ATTACH cursor (scene swaps are mid-stream; old
   absolute-cursor semantics would fire the whole past at once) +
   15th determinism gate. (c) tests: 20 unit + 4 determinism (same
   wall-time script -> identical scene-hash chain at 15/30/60/120/240
   Hz, pure-FSM replay, salt divergence) + 2 corpus (walk-vs-script
   event equivalence over the 5 real .MRS files, chunk-0 disabled,
   Freeze|Restart terminals, chunking-invariant mixes). Workspace 204
   green (+27), fmt + clippy -D warnings clean, manifests OK x2.
   DECISIONS D26 (hashed per-tick edge derivation of scene actions -
   what makes the host-rate hash identity exact) + D27 (melody-chunk
   selection + attach-anchored dispatch); DESIGN-GAME status =
   IMPLEMENTED AS SKELETON.

- 2026-08-18 846ebab+b684bee+00c2260+b950b44+a8f26f8 [P3] bedlam-audio
   thin mix-graph skeleton CLOSED (PLAN P3 charter crate; design note
   first per DESIGN-RENDER flow; see run notes - triple-agent night,
   suite regenerated after lane cleanup). (a) docs/DESIGN-AUDIO.md:
   mix-graph topology voices -> master bus -> device (device half =
   P4 open Q1, cpal-or-similar deferred); mix graph PURE hermetic
   integer math (no I/O/clock/floats/unsafe), byte-identical output
   under ANY host chunking; native 11025 Hz both builds (EXW
   SubVoiceStart SetFrequency (ratio*0x2b11)>>16 + WAVEFORMATEX; B2
   IRQ0-shared PCM driver @0x1276dc, PcmMixerService@0x136e0 20ch
   walker); .MRS 10ms tick = 441/4 samples exact in Q16 (tick grid
   never rounds); pitch/pan/volume per RE-EXW-MUSIC sec 6 + D25 (Q8
   linear gain over the (master*vol)/48 product, dB curve documented
   not reproduced, unity ceiling = DS attenuation-only); note-off-
   releases-BASE quirk kept; saturation = symmetric clamp (driver
   shape = open Q4); audio NEVER hashed per D17 b - byte-identity of
   the mix stream IS the gate. (b) engine/bedlam-audio: hermetic
   Mixer (flat 20-voice pool tagged (instrument, sub 0..3), 16.16
   phase step = RATIO_TABLE verbatim, Q8 gains snapshotted at spawn
   mirroring EXW master-read-at-SubVoiceStart-only, i32 stereo bus +
   symmetric clamp, S16 interleaved out), spawn/free/pitch/volume/pan
   API mirroring RE semantics, MusicScript absolute-tick NoteOn/
   NoteOff (NO bedlam-assets dep - coupling lands in bedlam-game),
   render() host-driven, dt never enters. thiserror only,
   forbid(unsafe_code). (c) tests: 9 unit + 14 determinism gates
   (same script => byte-identical across 1/7/64/512/4096 chunk
   patterns, unity passthrough sample-exact, doubled-pitch skip,
   base-only note-off, drop-when-sub-voices-full + pool-full,
   one-shot immediate free + slot reuse, saturation clamp, pan law,
   ratio-0 mute-occupies, tick-grid exactness at frame 441,
   odd-buffer + out-of-order-script errors); workspace 177 green
   (+23), fmt + clippy -D warnings clean, cargo +nightly miri test
   -p bedlam-audio CLEAN (23 tests, zero UB; determinism suite 425s
   under miri - ci.yml miri job extended to -p bedlam-audio, noted
   CI cost). DECISIONS D25. Manifests OK x2 (run was code-only).

- 2026-08-18 1501ab9+014597b [P3] Miri + per-tick hash CI unit CLOSED
   (PLAN sec 7 charter gate: determinism CI from the first playable tick,
   applied from skeleton tick 0). (a) cargo +nightly miri test -p
   bedlam-core CLEAN on this host - miri 0.1.0 (771916f902 2026-08-08)
   via rustup component add on the existing nightly (rustc 1.99.0-nightly
   b07e5a086 2026-08-07); 41 unit + 12 determinism tests green, ZERO UB
   findings; re-run after adding the fixture also green. Invocation +
   result recorded in STATE. (b) engine/bedlam-core/tests/hash_fixture.rs
   COMMITTED per-tick hash fixture: 600-tick fixed integer input script
   (seed 123456 - EXW/B2 RNG seed provenance nod, fade window armed
   ticks 101..200 so the 300Hz satellites are inside the pin), 13
   milestone StateHash constants + FNV-1a chain over all 601 hashes
   (EXPECTED_CHAIN 0x760d221bec3b3b99); runs in the ordinary cargo test
   matrix so ubuntu+windows CI fails loud per tick on any cross-OS/
   toolchain hash drift; #[ignore] print_fixture = documented
   regeneration path (intentional hashed-state changes only, with
   FORMAT_VERSION bump - unintended changes must FAIL, never be
   papered over). (c) ci.yml: new miri job (ubuntu-latest, dtolnay/
   rust-toolchain@nightly + miri component, cargo +nightly miri test -p
   bedlam-core on push/PR; miri has no Windows support - noted in the
   job comment). Workspace 154 green (+1), fmt + clippy -D warnings
   clean, manifests OK x2. Run was corpus-untouched (engine/ + .github/
   + .state/ only).
- 2026-08-18 ff8fb17+d2b7fb8+f86a100 [P3] bedlam-render + bedlam-platform
   wgpu skeleton CLOSED (per D20 + DESIGN-RENDER; code landed by the 03:00
   worker session that outlived its rc=1 client - commits 03:07/03:17 -
   then died before the queue rewrite; this 03:32 respawn VERIFIED rather
   than redid: workspace 153 tests green incl. a REAL offscreen GPU
   round-trip, fmt + clippy -D warnings clean, manifests OK x2, source
   read line by line against D20/DESIGN-RENDER secs 1-9/D17). (a)
   bedlam-render: canonical Frame 640x480x8 + [Vga6;256] + palette_dirty;
   parity_hash = FNV-1a(indices || 6-bit palette); VgaExpand Original
   v<<2 (SetPaletteRGB-identical) / Full; render() fixed pass order
   world->sprites->rows->overlays->entities; camera clamp 9..631/9..463;
   alpha = presentation hint, IGNORED with prev_sim=None (golden config);
   hermetic (forbid unsafe, no I/O/clock/threads, only float is alpha);
   12 tests. (b) bedlam-platform: pure scale.rs (Integer default with
   pillar/letterbox, Fit, Fill with centered uv crop; zero-rect skip) +
   gpu.rs wgpu 27.0.1 ParityGpu (headless, None => skip) + ParityPipeline
   (R8Uint index tex per frame, R32Uint packed 6-bit palette tex ONLY on
   palette_dirty - 004ee9b6 analog - fullscreen-triangle WGSL
   palette-expand + scale, nearest default / bilinear-over-expanded-RGB,
   indices never interpolated); PresentConfig parity defaults
   Integer+Nearest+Original; consumes bedlam-render Frames only, never
   bedlam-core, no clock; 9 tests. D24 records the wgpu pin + P4-spike
   deferral of the final version call. DESIGN-RENDER status flipped to
   IMPLEMENTED AS SKELETON (f86a100).
- 2026-08-18 928748d+7bfac4b+aff1ae8 [P2] B2 episode-loop progression +
   INT8-counter readers CLOSED (2x -process BEDLAM.EXE -noanalysis passes
   this run + B2EpisDump adopted from the transport-killed 02:0x run and
   fully re-derived; see census sec 7 + D23). (a) ALL SEVEN counters
   classified - NONE gate sim/render: 0x801a6/0x80010 = audio tick/position
   bases (stub reset + ms=x10 vs real driver hi-res tick+PIT-phase clock),
   0x11f158/0x11f0b4 = DEAD (3-way proof), 0x11f0c8 = ISR-internal phases
   (palette &7, mouse &1), 0x11f0c4 = 100Hz timeout base (WaitTicks100Hz
   zero+spin; 10 sites: 7x 2000-tick screens + 750 + 2x 500), 0x11f0b0 =
   50ms micro-delay. (b) Episode loop decoded: linear 0..26 +1 per completed
   mission; mask |= 1<<(sub-1); stage-slot++ when mask == full-mask[0x81d9a]
   {0,1,0xf x6} + zone-complete cutscene (LOAD_UK/US.BIN); SUB =
   PLAYER-selected in MapRoomSelect@0x50a87 (BRF_* backdrops per slot 2..8,
   mission formula re-derived there); saves 5x61B @0x8b1d4 {mask,slot,linear
   word,money,stats}. (c) PresentFlip@0x1066b = VESA page flip (bank pair
   {0,5}, display start 0<->0x200, ISR+flip locks, WaitVRetrace, 0x96-dword
   cursor block copy) + PcmMixerService@0x136e0 = 20-channel PCM voice
   walker (spawn/free sub-voices) gated by triple flag 0x11ef50/24/0x11f0e0.
   (d) zone dword[0]=25 = SENTINEL (min formula index = 1; boot plants
   zone=1/mission=1 as constants; values 7/8 = special screens). (e) VERDICT:
   mission loop = present-paced vblank, ZERO counter reads in loop -> D16
   confirmed on DOS, D23. Bonus: B2 audio = IRQ0-shared 11025Hz PCM driver
   (PIT reprogram on arm, driver struct 0x1276dc, same native rate as EXW);
   video = VESA 0x101 640x480x8 dual-page vs 320x240 logical = 2x scale;
   g_flip_lock@0x8008e guards the 50Hz ISR cursor draw (corrects the
   gates-the-mixer guess); g_snd_handles@0x8abf8 = runtime-filled sound
   handles (corrects endgame-dispatch guess). 30 fns + 33 labels persisted;
   orphan stub/driver callbacks (0x12ecf/ee4/eef, 0x607b0, 0x686b0/d0/740)
   created as functions. Manifests OK x2 (Ghidra passes read .rep only).
- 2026-08-18 2df7664+c3b1552+9b4d119 [P2] B2 entry-chain naming +
  tick-source hunt + zone/stride CLOSED (3x -process BEDLAM.EXE
  -noanalysis, no import; full 671-fn decompile sweep dumped as
  b2-decomp-all.txt). ENTRY: _entry@0x66a60 -> CrtInitChain@0x6b1bc
  (argc/argv stored at 0x1280d4/d8 - census candidate-rng guess
  corrected) -> GameInit@0x2f731 = boot shell AND episode-loop host
  (seeds both RNGs 123456/234567 as code constants; OPTIONS.BDL +
  SETUP.EXE path; LANGUAGE.* select; 320x240 coord space planted).
  TICK: NO INT28h/DPMI/constant-divisor - TickInstall@0x32546 does
  DosGetVector(8) + PitProgram(0x2e9b = 100.01 Hz) + DosSetVector(8 ->
  Int8TickHandler@0x12734): immediate EOI, XCHG reentrancy lock, 7
  counters, ClockDivider100Hz (hundredths->hms), 12.5 Hz palette banks
  0x90..0x97 (byte-identical behavior to EXW TimerCallback), 50 Hz
  mouse poll+clamp vs 0x140/0xf0; TickShutdown restores 0xffff;
  present = WaitVRetrace@0x10856 double-poll 0x3da bit3 via
  FUN_0001066b. SAME two-clock architecture as EXW -> D22 (parity
  budget carries). RNG: RngStepA/B@0x1220e/0x1224f coupled 16-bit
  pairs, reseed site 0x5eaf9. ZONE/MISSION: lookup tables order[8]
  @0x81dba / zone letters @0x81dda / mission[27] @0x81e46; formula
  zone/mission = table[order[slot]+sub]; mode DAT_0011f11c==2 adds 5
  -> corpus MISSION{1-4,6,7} numbering EXPLAINED (6 zones A-F x
  {4 regular + 2 alt}, 27 linear missions; EXW 7x5 arithmetic differs).
  15 fns + 16 labels persisted; docs census sec 6 + D22; scripts
  B2EntryTick/B2EntryNames/B2TblDump committed. Manifest-2 OK
  before+after; game-data-2 read-only.
- 2026-08-18 dcf43d2+d09e41f+a36b15f+75c1474 [P2] B2 import EXECUTION CLOSED.
  ghidra-lx-loader built from source (master clone, gradle 8.14.3 vs our
  12.1.2 DEV install; clean 18s build = zero version risk) and installed.
  THREE gotchas fixed + documented (census doc sec 5): headless extension
  dir is userSettings/Extensions (NOT install/Extensions/Ghidra), -loader
  matches class simple name (LeLoader, not display name), MzLoader claims
  LE files at higher priority unless -loader LeLoader forced (research
  fall-through claim corrected). Loader options set via Java prefs
  (B2SetLxPrefs.java; B2SmokeVerify/B2Census/B2ListLoaders/B2BootCompare
  scripts). Smoke test 5/5 gates PASS on scratch project; real import into
  BedlamWatcom with full auto-analysis green; census 671 fns (EXW 675),
  414 strings / 216 fileish (MIRAGE AB_BED + SFX RAW + GAMEGFX PAL all
  confirmed in-binary). FIRST BOOT COMPARISON vs EXW: RNG seed constants
  123456/234567 IDENTICAL (B2 FUN_0002f731 game-init via CRT 6b1bc; reseed
  [0x11ef1c]<-123456 in FUN_0005eaf9) - strongest cross-build parity fact
  yet. Manifests OK before+mid+after (both corpora only read).
- 2026-08-18 6f22968+e02b80b+39f4fac [P2] Tick satellite naming pass CLOSED: repaired census script; 19 callees decompiled+named; 10 labels persisted; 4 DirectDraw surface roles distinguished; CRT thread trampoline chain closed.
- 2026-08-18 f15eb60+7396491+889cbef [P3] bedlam-core crate skeleton CLOSED
  (D17). engine/bedlam-core: hermetic deterministic sim skeleton per PLAN
  sec 7 + D16/D17. fx.rs Q16.16 saturating fixed; rng.rs PCG32 XSH-RR +
  SplitMix64 seeding + exact-unbiased bounded(); hash.rs in-crate FNV-1a 64
  (+StateHash); time.rs NOMINAL_TICK_HZ=60 (D16) + TimeBase; input.rs
  12-byte LE InputFrame (buttons bit layout deliberately unassigned pending
  P2e; mouse bit0/1 = left/right per RE-EXW-INPUT); replay.rs b"BDLR"
  versioned input log, never-panic parse (every-single-byte-truncation
  tested); sim.rs hashed fixed-60Hz bucket: exactly one rng draw per tick
  (entropy slot) + 300Hz microstep scheduler per DESIGN-RENDER sec 6 (one
  global counter, %3 service=100Hz / %6 fade=50Hz-while-fading / %24
  palette=12.5Hz, fixed order, zeroed at construction mirroring
  FUN_0041e19d), b"BDLS" snapshot/restore with re-hash validation; frame.rs
  NON-hashed per-frame bucket (cursor clamp / latch / volume / cooldown,
  dt ok per D17 b) + SimDriver accumulator on 240Hz sub-tick grid
  (SUBTICKS_PER_TICK=4, banked remainder; host dt never enters sim math).
  tests/determinism.rs 12 gates incl. same-script same-sim-hash at
  15/60/240Hz + replay/snapshot round-trips + tamper detection. 132 tests
  green, clippy -D warnings clean, fmt clean, manifest OK x2. Deps:
  thiserror only; forbid(unsafe_code); no floats/fs/time/thread/HashMap in
  src (grep-verified by orchestrator).
- 2026-08-18 84390d4 [P4 prep] pinned runtimes CLOSED (slot 5 - taken because
  3-claim existed from a dead session; spawner placeholder 3-1787004415.claim
  deleted as instructed). DOSBox-X = flathub com.dosbox_x.DOSBox-X 2026.08.02
  @ commit fa89039c... user install with XDG_DATA_HOME INSIDE the repo
  (gitignored runtime/xdg, ~sudo-free); upstream GitHub has NO Linux release
  binaries (verified last 6 tags: only win/mac/hx-dos assets) so the queue
  item AppImage channel no longer exists - flathub is the official Linux
  channel, decision D19. Debugger confirmed in binary (INT3 auto-BP strings);
  --version headless smoke ok. Wine = system wine 11.15 wow64 mode (win32
  WINEARCH rejected - 32-bit PEs via syswow64, 890 dlls), prefix
  runtime/wine-exw via wineboot with mono/gecko/menubuilder disabled; reg
  query + cmd smoke ok; BEDLAM.EXW = PE32 i386 GUI 5 sections (side note for
  B2: file(1) also calls BEDLAM.EXE PE32 - LE images get misread, do not
  trust file(1) for the LE image). Wrappers tools/runtime/{dosbox-x,wine-
  exw}.sh both smoke-tested; docs/RUNTIME.md has pins, provenance, upgrade
  policy (never blind update; re-baseline goldens on pin change). Manifest
  checked before+after: OK (game-data only read). Next steps queued in
  backlog above.
- 2026-08-18 976f19f [P2] B2 prep: DOS4GW LE loader research CLOSED (this
  run = the 00:06:55 item-3 respawn after the client-death storm; re-derived
  every fact from scratch per the spawn-storm note, trusting nothing from the
  dead session transcript). RESEARCH-BEDLAM2-CENSUS.md new section: (1) B2
  BEDLAM.EXE + B1 EXD LE headers pinned via Open Watcom exeflat.h layout with
  3-way structural cross-checks - B2: 2 objects (code 0x10000 R+X vsize
  0x66970 / data 0x80000 R+W vsize 0xb04ee with only 7 file-backed pages =
  mostly implicit zero-fill), 110 pages x 4096B all VALID sequential, data
  pages file [0x36e00..0xa428f) consuming the file EXACTLY, eip/esp are
  OBJECT-RELATIVE (spec form): linear entry 0x66a60, initial esp 0x1304ee =
  obj2 top (gate that also holds in EXD); module name BEDLAM; EXD same
  class (107 pages, linear eip 0x5fbb0). HEADLINE:
  internal fixups NOT pre-applied (flags=0x200, 205KB fixup section) => a
  raw-carve import yields garbage pointers - loader mandatory; honest
  python fixup-applier fallback documented in the section. (2) Loader
  census: stock Ghidra 12.1.2 incl. our -watcom build has NO LE loader
  (Base.jar LinearExecutable = NotYetImplementedException stub,
  javap-verified); yetmorecode/ghidra-lx-loader v12.0.1 (2026-01-29) = the
  pick (MSDOS DOS/4 LE-Style, full page-map+fixup application, per-fixup
  labels); alexbevi Harvester series 2026 = end-to-end prior art; SB.EXE
  unbind NOT needed (our MZ region is just the 19KB Watcom launcher,
  DOS4GW.EXE ships separate). (3) The 12.0-vs-12.1.2 risk is removable:
  build from source against the install (build.gradle ->
  support/buildExtension.gradle, Gradle >= 8.5); force-install + scratch
  smoke test as plan B. RESEARCH.md lx-loader UNCERTAIN tag resolved.
  Manifests OK before+after. GOTCHA FIX for the 23:2x note: MANIFEST-2.sha256
  lives at REPO ROOT with corpus-relative entries, so the working command is
  cd game-data-2 && sha256sum -c ../MANIFEST-2.sha256 (the earlier note
  omitted the ../ and that exact form fails).
  FOLLOW-UP same run: a parallel sibling (b8f63e6, second item-3 respawn
  from the storm, no claim file - the invisible-agent lesson again) had
  committed a DUPLICATE research section; consolidated per incident-#3 rule
  into the canonical section (its unique findings folded: Ghidra #532,
  oshogbo fallback loader + version-crash #37, exact file-consumption gate,
  concrete import command, -cspec openwatcomcpp form) and CORRECTED my own
  eip/esp reading (offset-form per LX/LE spec, linear 0x66a60/0x1304ee -
  the original linear claim was an arithmetic slip caught by cross-checking
  the sibling derivation; D18 stands, pointer updated).
- 2026-08-18 a3ad066 [P3] bedlam-render DESIGN NOTE CLOSED (docs/DESIGN-RENDER.md, new - docs-only run, no code, no Ghidra). Canonical Frame = 640x480 x 8-bit indices + 256 x 6-bit VGA palette; parity/goldens anchor there; everything above is presentation (D9/D12). Contents: RE-fact table with anchors (present chain + vsync verdict D16, palette upload r<<2 @SetPaletteRGB, banks SetPaletteIndex + 0x90..0x97 12.5Hz cycle + region bank 0x5d, fade engine 16.16 + 10-step = 200ms, palette-dirty handshake 004ee9b6, entry-0 quirks, composition order base->sprites->row blits->overlays->entities, camera clamps 9..631/9..463); palette policy: 6-bit canon everywhere, expansion at presentation ONLY (Original v<<2 default vs Full (v<<2)|(v>>4)); bedlam-assets pal.rs Palette = tooling repr, NOT render canon (flagged for a later Vga6 type); D17 concretized: 300Hz microstep scheduler (5 per 60Hz sim tick; service event %3 = 100Hz, fade %6 = 50Hz while fading, bank cycle %24 = 12.5Hz; counter zeroed at boot release mirroring FUN_0041e19d) -> deterministic satellites, hashed; ownership/hash boundary table (interpolation alpha + present timing shape frames, never state; goldens at tick boundaries with interpolation off); 7 open questions each naming its answer source. Manifest verified OK (run was docs-only).
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

- 2026-08-18 02:3x-03:xx (episode-loop run): clean unit from the adopted
  claim. Arrived to find untracked B2EpisDump.java + complete b2-epis.txt
  (122KB) from the 02:07 transport-killed predecessor (agent-1787010999
  died mid-(e)-analysis; its log tail carried two counter findings which I
  re-derived independently - both held). Per the ExwTickSats lesson:
  inspected + adopted the WIP instead of rewriting. 3 commits (scripts,
  names, docs) + this queue rewrite; no re-import (2x -process -noanalysis
  only, pre-checked pgrep each time); javac precompile caught an
  AddLabelCmd.applyTo arity slip before it became an OSGi ghost (rule
  pays). Fish gotcha again: $? in a compound command aborts the whole line
  at parse time BEFORE anything runs (pgrep guard included) - use plain
  echo separators. Manifests OK after (B1 repo-root, B2 from game-data-2).

## Run notes
- 2026-08-18 15:33-16:0x (surf-confirm unit, double-restart lane): item-1
  lane incarnations: 15:33 worker (died transport, progress=0 per nudge)
  -> BUT its analyzeHeadless survived server-side as an OS ORPHAN and
  finished the dump (4th ghost-survivor case, first one observed
  mid-flight by the respawn: the respawn guard tripped on the LIVE
  process, then its own -process attempt died cleanly on the project
  LockException - which is exactly the designed collision behavior, zero
  damage) -> 15:48 respawn (this run) verified the dump + closed the
  unit. LESSONS: (1) pgrep GUARD GOTCHA: the analyzeHeadless process is
  a BASH wrapper whose cmdline has no java token, and the agent own
  bash -c transport string CONTAINS the pattern text - working guard =
  ps -eo pid,cmd + filter out grep/opencode2/fish/bash -c self markers;
  a bare pgrep -f "java.*analyzeHeadless" is BOTH self-matching AND
  misses real runs. (2) A LockException from a headless -process attempt
  while another run holds the project = clean no-op, safe to just wait
  out (the orphan finished in ~3 min). (3) The fish-heredoc recipe
  (write apostrophe-free body to /tmp/opencode via bash -c, wc/head
  verify, then cp into the repo) held for both Java scripts + both
  python patches across two restarts - zero transport corruption this
  unit.

- 2026-08-18 14:27-15:0x (harness run, sixth spawn of the lane): items
  1 had FIVE transport-killed incarnations this hour (13:56, 14:03,
  14:12, 14:23 + one pre-restart), every one dying during READ-ONLY
  recon (agent logs end mid grep of mixer/fsm APIs; zero files
  written; tree clean at every check). This run did the unit start to
  finish. LESSONS: (1) NEW FISH GOTCHA VARIANT - a Rust char literal
  holding a quote or backslash (single-quote-delimited) inside the
  fish-wrapped bash heredoc breaks the outer string; ALSO fish collapses
  doubled backslashes inside single-quoted transport (only backslash-
  backslash and backslash-quote survive), so written bytes must be
  repr-verified after every heredoc - the json_escape function hit BOTH
  and was repaired by chr()-built python patches, never by re-running
  the heredoc blind. (2) An audible parity baseline REQUIRES loading
  the sibling .MRW bank per track; script-only runs mix 100 percent
  silence and hash identically for any implementation (D28 pin 3).
  (3) SceneFsm::apply consumes a boot tick even mid-boot (act Options
  during Boot advances the countdown one frame early vs pure stepping)
  - recorded crate behavior, the harness just logs transitions; do not
  file as a harness bug. (4) Manifest checks before AND after really
  do bracket the corpus read cleanly: OK x2.

## Run notes
- 2026-08-18 13:43-14:3x (game unit, triple-respawn close-out): the
  lane survived THREE transport deaths in 90 min (11:33 crate start,
  13:16 mid-test iteration with 6 lib failures, 13:25 lib-green but
  integration tests unwritten) before this run. Server restarts hit
  TWICE mid-run; both times file state persisted and the adopted-WIP
  protocol worked cleanly. LESSONS: (1) the two corpus-test bugs were
  SEMANTIC anchors already resolved elsewhere in the repo - when a new
  test contradicts a pinned corpus fact (assets validate_mrs_song
  accepts Freeze|Restart), the NEW test is wrong, check the canon
  first; (2) comparing chunked render buffers requires truncating to
  the common bound - whole-buffer assert_eq fails on LENGTH for any
  chunk size that does not divide the total (bedlam-audio suite
  already knew this; the new test reinvented it wrong); (3) 0-tick
  banking frames make any cursor-is_multiple_of-N sampling loop
  double-push - gate on executed>0. Contamination watch: the 13:36-
  13:43 tools/nudge* + network-watchdog edits in the tree were
  CONTROLLER infrastructure, committed by the controller itself as
  9227681 mid-run - correctly left untouched by this lane.

- 2026-08-18 04:3x-05:2x (audio unit, ghost-survivor run): item-1
  owner via claim 0711 end to end (with one orphan-window mid-run).
  FOUR agents touched item 1 tonight: 0162 (claimed 04:29, went
  silent, committed NOTHING - the 8b314b7 note cites a draft commit
  39305a7 that does not exist in the repo; attribution correction:
  846ebab design note + b684bee skeleton are THIS run commits),
  0711 = this run, 1260 (controller respawn 04:47:40 that adopted the
  claim, committed the containment note 8b314b7 + addendum 5675813,
  died transport rc=1 04:52:40), and the 05:00 interloper (b6b6920
  own 12-gate suite after concluding determinism.rs lost, then
  8fc476e self-revert declaring 00c2260 canonical - net zero, correct
  call). This run: re-adopted the orphaned claim atomically after
  pgrep/git-log/claim-staleness checks, regenerated determinism.rs
  from the surviving /tmp/opencode generator, fixed the two REAL
  boundary bugs the restored suite exposed (one-shot voice freed one
  frame late; script dispatch pre-incremented the cursor so exact-
  boundary events fired one frame early), then closed the unit.
  The reaper deleted the live claim at 05:01:54 (dead-client
  heuristic vs ghost-survivor session) - re-planted with fresh mtime
  before close-out. LESSONS: (1) heartbeat BETWEEN heredoc/generation
  blocks - third strike for this exact failure mode; (2) after any
  long silent phase, re-check claims/ BEFORE the next shared write
  (the reaper may have eaten yours); (3) keep /tmp/opencode
  generators until the unit is fully committed - the generator was
  the only recovery path for the deleted test file; (4) containment
  notes written under time pressure misattribute commits - verify
  hashes against git log before trusting a sibling note (8b314b7
  phantom 39305a7).

- 2026-08-18 04:5x (audio-dup watch run; stood down, contamination
  cleaned, ONE file possibly lost - VERIFY your determinism tests):
  DUPLICATE-SPAWN #6, item 1 (bedlam-audio). This run claimed item 1 at
  04:29 (worker ...0162), committed the first DESIGN-AUDIO draft
  (39305a7 ~04:38), then went SILENT >7min generating big heredocs
  (heartbeat rule violated AGAIN - generation phases are silent) ->
  controller spawned the sibling (...1260) at 04:47:40 which rewrote
  the claim, pinned the design note (846ebab) and committed the mixer
  skeleton (b684bee). This run then contaminated the lane by cp-ing its
  parallel implementation over the tracked Cargo.toml/lib.rs at
  04:50:28; detected within seconds (tracked mixer.rs/script.rs not
  mine), restored tracked files via git checkout, removed its untracked
  audio.rs/sequencer.rs. ONE residual risk: engine/bedlam-audio/tests/
  determinism.rs carried mtime 04:51 (a rewrite AFTER this run 04:50:28
  copy - almost certainly the sibling own test file) and was deleted by
  this run cleanup at ~04:52. Sibling close-out MUST verify the crate
  test count + determinism suite exists and is green before pushing; if
  missing, re-save from session context. Canonical owner = the live
  sibling (earlier integrated commits); the 39305a7 draft was superseded
  by 846ebab. LESSONS: (1) touch heartbeat BETWEEN every heredoc/cat
  generation block, not just around shell commands - this exact failure
  was already on record from 22:3x yesterday; (2) before ANY cp/rm into
  a shared path, re-check git log + tracked-file mtimes (a lane can be
  claimed between two of your own commands); (3) fish has no heredocs -
  forgot the bash -c wrapper AGAIN, wasted one cycle.
  ADDENDUM (same run, 04:55): the sibling DIED at 04:52:40
  (nudge.log: agent item 1 failed, transport rc=1, progress=1) while in
  WATCH MODE - its transcript tail shows it had detected this run
  contamination at 04:52:09 and correctly stood down ("no writes from
  me") before dying. Its last durable artifacts = 846ebab + b684bee
  (skeleton: lib.rs + mixer.rs + script.rs + Cargo.toml + workspace
  member + Cargo.lock, all committed and clean). The lost
  tests/determinism.rs content is NOT recoverable: nudge-run.log
  records only orchestrator text + command outputs, never heredoc
  payloads (grep-verified, zero content markers from EITHER session).
  Respawn plan of record: adopt the stale claim per protocol, write the
  determinism suite fresh against the COMMITTED b684bee public API
  (20-voice pool, script.rs sequencer semantics, DESIGN-AUDIO as pinned
  by 846ebab incl. D25), run the full gate (fmt + clippy -D warnings +
  workspace tests 155+ + miri consideration + ci.yml miri-job decision
  per the item text), then DECISIONS/STATE check + queue rewrite +
  delete claim + push (4 commits currently unpushed: 39305a7, 846ebab,
  b684bee, 8b314b7). Do NOT blend implementations: this run parallel
  draft (different API: 32-inst x 4 sub-voice pool, GAIN_Q8 dB ladder)
  sits ONLY in /tmp/opencode/audio-src/ as reference - superseded, do
  not copy in. Manifests were never touched by any party this unit
  (docs + engine only).
- 2026-08-18 04:0x (miri+hash-CI run): clean unit, claim 1-owner.claim
  honored start to finish. Fish gotcha hit AGAIN on the ci.yml edit
  (writing ${{ matrix.os }} through a bash-heredoc-inside-fish wrapper -
  fish parses the braces even inside the single-quoted command string):
  solved by python-append instead of heredoc-rewrite, leaving the
  existing matrix lines untouched. Miri install path note: component add
  --toolchain nightly works offline-ish and fast IF a nightly toolchain
  already exists (this host had one; no rust-toolchain.toml pin in repo
  - CI uses dtolnay/rust-toolchain@nightly action, local uses the host
  nightly; if the host nightly drifts, miri results are still valid for
  UB detection purposes). First miri run needs ~3min sysroot setup (ran
  in background while writing the fixture - good overlap pattern).
  Fixture generation flow: committed placeholder-zero constants first,
  generated via cargo test -- --ignored --nocapture, pasted with
  assert-guarded python replace, then verified the real test passes
  from a clean cargo test run (not just miri).

- 2026-08-18 03:3x (render-verify run): arrived as the 03:32 respawn for
  item 1 and found BOTH crates already committed (ff8fb17 03:07, d2b7fb8
  03:17) with no queue rewrite - the 03:00 worker client died transport
  rc=1 at 03:05 (nudge.log) but its server session finished the unit
  commits invisibly, then died too (unit gone from systemd, no opencode2
  run process, 15 min commit silence before my spawn). Per the
  queue-verify precedent: verified instead of redid - full workspace 153
  green (the GPU round-trip REALLY ran, 1.35s with a live adapter), fmt +
  clippy -D warnings clean, manifests OK x2, source read in full against
  D20 + DESIGN-RENDER secs 1-9 + D17 boundary (grep: no HashMap/fs/time/
  thread in either src; forbid(unsafe_code) in both). Completed the unit
  tail: DESIGN-RENDER status flip + D24 (wgpu 27.0.1 pin rationale) +
  STATE + this queue rewrite + backlog promotion to the Miri/CI item.
  LESSON: ghost survivors now also finish CODE units, not just docs - a
  stale queue item may already be DONE; check git log + systemd units +
  pgrep before planning any redo.
- 2026-08-18 00:1x-00:2x (item3-dup watch run; stood down, NO work files
  touched): DUPLICATE-SPAWN #5, contained, no damage. This run = the 00:12
  nudge-item3 spawn. Protocol followed: found 3-claim stale (00:02, owner
  died in the 00:04 client-death storm) + no live clients in pgrep ->
  adopted the claim (fresh mtime 00:12, deleted own placeholder). ~90s
  later commit 976f19f (00:12:54) landed: the 00:06:55 item-3 respawn was
  ALIVE server-side all along (3rd case of rc=1 client death NOT killing
  the server session). Stood down instantly and only watched: survivor
  closed end-to-end (976f19f + b8f63e6 + 570e941 consolidation with the
  eip/esp object-relative correction + 87fbed2 queue rewrite + push; it
  also cleaned up the claim this run had adopted). Deliverable verified
  before exit: RESEARCH-BEDLAM2-CENSUS.md B2 Ghidra import plan (4
  sections + 5-step runbook + D18). Also observed live and left alone:
  item-1 bedlam-core worker (f15eb60, 7396491, 889cbef), P4-prep slot-5
  agent (84390d4: runtime pinned REPO-CONTAINED via tools/runtime +
  gitignore /runtime/ - resolves the prior owner-decision flag, review
  still pending). LESSON for stale-claim adoption: pgrep + claim mtime is
  NOT enough - poll git log for 2-3 min before believing a lane is free,
  because a live survivor can be invisible to pgrep while its server
  session finishes the unit; the fresh-mtime claim rewrite only prevents
  controller respawns, not survivor collisions (harmless here solely
  because this run wrote nothing to work files).
- 2026-08-18 00:0x-00:3x (b2-le-research run, item-3 respawn): clean unit,
  research-only (web + local parses, javap, no Ghidra). Claim-lane note: I am
  the 00:06:55 controller respawn predicted by the spawn-storm note; 1-claim +
  2-claim still had DEAD owners when I exited (reaper territory). Doc surgery
  via staged bash-heredoc + assert-guarded python anchor replace per house
  recipe; commit with explicit paths only (siblings bedlam-core + ExwTickSats
  WIP left untouched). PLAN sec P2 raw-binary fallback is now known-NAIVE for
  LE (unapplied fixups) - if anyone re-plans the fallback, start from the
  census doc section, not PLAN.
- 2026-08-18 00:0x (render-design run): arrived with all 3 Now items claimed -> claim protocol slot 4 = first unblocked BACKLOG item. Skip reasons recorded: B2 import blocked by in-flight item-3 LE-loader research; P4 prep DOSBox-X/Wine needs DURABLE writes outside the repo (AGENTS hard-rule violation for unattended runs - needs an owner decision on tool storage first); cosmetic RE (LAB_00451fbc + surface roles) would take the BedlamWatcom Ghidra -process lock concurrently with the live item-2 naming pass. FISH GOTCHA (new, same family as the apostrophe rule): a Rust lifetime annotation containing a single-quote char inside a bash -c SINGLE-QUOTED heredoc body closes the fish string early -> the whole command dies as a FISH parse error before bash runs (no partial file - verify with ls). Recipe that worked: write the heredoc body apostrophe-free to /tmp/opencode, verify wc/head, then cp into the repo. Also: commit with explicit paths (git add docs/DESIGN-RENDER.md) when siblings have uncommitted WIP in the tree (bedlam-core/* and ExwTickSats.java were live); push carried the prior local-only queue commit 22ca126 along.

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

- 2026-08-18 00:2x (bedlam-core run): task 1 delivered in 3 code slices
  (f15eb60 skeleton, 7396491 D17 hybrid timing, 889cbef satellite alignment
  to DESIGN-RENDER sec 6). Owner direction D17 landed MID-RUN (23:44
  NEXT.md edit + commit 2ca2d41) - detected via the uncommitted queue diff
  and honored in slice 2. COLLISION CAUGHT: slice-2 phase-accumulator
  satellites vs sibling render-note a3ad066 300Hz-microstep concretization
  - reconciled in 889cbef to follow the committed design doc (long-run
  rates identical, hashed layout + firing phase differ; code follows
  docs). AGENTS.md no-nesting rule landed during close-out: the 3 slices
  predate it, close-out is direct work, rule in effect from now on.
  Transport error on first subagent spawn (z.ai decode error, nothing
  written) - one clean retry. tools/ghidra-scripts/ExwTickSats.java
  appeared untracked at 00:00:28, still uncommitted WIP matching new queue
  item 1 - whoever claims it should inspect it before rewriting. 132 tests
  verified green twice by this run on top of slice reports; manifest OK.
