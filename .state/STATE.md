# STATE — project snapshot (update when phase changes)

- Phase: P1 essentially complete (asset formats solved, 961+/1069 parsed, rest
  classified or editor-only). P2 active: BedlamWatcom project has the single
  watcom-correct EXW import (never re-import); headless -process passes work.
  EXW outer architecture SOLVED (boot chain, main@0044d6e8, init FUN_0044d320,
  pump FUN_0044d93c, WndProc@0044dacc, 100Hz timeSetEvent tick callback
  LAB_0044de58 = next RE target). See docs/RE-EXW-MAINLOOP.md.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- Known open: 100Hz tick internals (LAB_0044de58), .MRS event encoding,
  CONFIG.BDL layout, .BLD/.CTG (editor-only), PAL variant renderers, EXD import
  (needs LE loader ext), goldens pipeline (P4).
