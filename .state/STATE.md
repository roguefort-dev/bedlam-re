# STATE - project snapshot (update when phase changes)

- Phase: P1 essentially complete; P2 well underway. P3 STARTED: decoders
  promoted to workspace crate engine/bedlam-assets (pure, 70 unit + corpus
  round-trip tests, inspect CLI output byte-identical, D14). EXW outer architecture +
  100Hz tick + game worker thread FULLY mapped (GameThread@0044dea0 = 59-byte
  trampoline -> GameMain@0041c050 = real game shell/loop; pacing = 100Hz tick
  -> 50Hz gate 004ede10, 20fps claim refuted; 7x5 zone/level structure; RNG
  seeds 123456/234567). Names applied in BedlamWatcom project (WinMain..
  AppActivate, TickWorker.., GameThread/GoFlagSet/GameMain - see
  docs/RE-EXW-MAINLOOP.md, docs/RE-EXW-TICK.md, docs/RE-EXW-GAMETHREAD.md).
  EXD import still pending.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
  NOTE: touch heartbeat around every long shell command (Ghidra ~2min) or a
  second agent gets spawned mid-run (happened 2026-08-17, see NEXT.md run notes).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- Known open: GameMain second hop - FUN_0043d00b (per-frame sim/render, reads
  50Hz gate 004ede10; possible gate subdivision) + FUN_00440e45 (zone/level
  manager) + GoFlagSet@0044d9b4 caller; music small tails (FUN_0044c4a8
  ratio/vol applier, table C consumer, loop-flag 0045cdc0 writer);
  .BLD/.CTG (editor-only), PAL variant renderers, EXD import (needs LE loader
  ext), goldens pipeline (P4). Parity budget: 50Hz logic (D13).
- CLOSED 2026-08-17: music format chain fully RE-d + byte-validated
  (.MRW layout, .MRS container + complete event grammar, MusicPump=song 3,
  ratio table @00454174; CONFIG.BDL = installer SB-setup record, EXW never
  reads it; RNG seeds consumed by RandA@00402975 / RandB@004029b6) - see
  docs/RE-EXW-MUSIC.md.
