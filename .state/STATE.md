# STATE - project snapshot (update when phase changes)

- Phase: P1 essentially complete; P2 well underway. EXW outer architecture +
  100Hz tick FULLY mapped (TimerCallback = service tick; sim/render loop
  located at worker-thread body 0044dea0 = next RE target). Names applied in
  BedlamWatcom project (WinMain..AppActivate, see docs/RE-EXW-MAINLOOP.md,
  docs/RE-EXW-TICK.md). EXD import still pending.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
  NOTE: touch heartbeat around every long shell command (Ghidra ~2min) or a
  second agent gets spawned mid-run (happened 2026-08-17, see NEXT.md run notes).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- Known open: game worker-thread body 0044dea0..0044dfec (sim/render loop,
  20fps pacing anchor), .MRS event encoding, CONFIG.BDL layout, .BLD/.CTG
  (editor-only), PAL variant renderers, EXD import (needs LE loader ext),
  goldens pipeline (P4).
