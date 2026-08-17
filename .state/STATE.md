# STATE — project snapshot (update when phase changes)

- Phase: P1 essentially complete; P2 well underway (EXW: watcall import clean, startup
  chain + pump + WndProc + 100Hz timer tick mapped, names applied in BedlamWatcom;
  next: tick frame body FUN_0041bfb6. EXD import still pending.)
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- Known open: 100Hz tick internals (LAB_0044de58), .MRS event encoding,
  CONFIG.BDL layout, .BLD/.CTG (editor-only), PAL variant renderers, EXD import
  (needs LE loader ext), goldens pipeline (P4).
