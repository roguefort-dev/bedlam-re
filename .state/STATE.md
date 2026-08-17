# STATE - project snapshot (update when phase changes)

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
