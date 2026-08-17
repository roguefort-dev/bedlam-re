# STATE — project snapshot (update when phase changes)

- Phase: P1 essentially complete (asset formats solved, 961+/1069 parsed, rest
  classified or editor-only), P2 started (Ghidra: 5 PEs analyzed in BedlamRE project
  under default cspec; watcall-correct EXW import running into BedlamWatcom project).
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- Known open: .MRS event encoding, CONFIG.BDL layout, .BLD/.CTG (editor-only),
  PAL variant renderers, EXD import (needs LE loader ext), goldens pipeline (P4).
