#!/usr/bin/env bash
# bedlam-re autonomy nudge v2: spawn continuation agent when work stalls.
# v2 changes: progress-based failure detection (exit codes lie), exponential
# backoff on no-progress runs, hourly spawn cap, shorter stall window.
set -u
PLAN_DIR=/home/kato/Documents/bedlam-re
STATE="$PLAN_DIR/.state"
HB="$STATE/heartbeat"
STALE=300        # seconds until heartbeat considered stalled
MAXSPAWN=8       # max spawns per rolling hour (API budget guard)

mkdir -p "$STATE"
[ -f "$STATE/PLAN-COMPLETE" ] && exit 0
[ -f "$STATE/PAUSE" ] && exit 0

# active cooldown after repeated no-progress runs?
if [ -f "$STATE/cooldown-until" ]; then
  now=$(date +%s); cu=$(cat "$STATE/cooldown-until" 2>/dev/null || echo 0)
  [ "$now" -lt "$cu" ] && exit 0
  rm -f "$STATE/cooldown-until"
fi

exec 9>/tmp/bedlam-nudge.lock
flock -n 9 || exit 0

age=-1
if [ -f "$HB" ]; then
  age=$(( $(date +%s) - $(stat -c %Y "$HB") ))
  [ "$age" -lt "$STALE" ] && exit 0
fi

# hourly spawn-rate cap
h=0; c=0
[ -f "$STATE/spawns" ] && read -r h c < "$STATE/spawns" 2>/dev/null || true
nowh=$(( $(date +%s) / 3600 ))
if [ "$h" -eq "$nowh" ] && [ "$c" -ge "$MAXSPAWN" ]; then
  echo "$(date -Is) spawn cap reached ($c this hour) - standing down" >> "$STATE/nudge.log"
  exit 0
fi
echo "$nowh $((c+1))" > "$STATE/spawns"

# rotate logs if huge
for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  if [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ]; then
    tail -c 262144 "$f" > "$f.t" && mv "$f.t" "$f"
  fi
done

OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/lib/node_modules/@opencode-ai/cli/bin/opencode2.exe
command -v opencode2 >/dev/null 2>&1 && OPENC=opencode2

# progress markers BEFORE spawn (exit codes lie: opencode2 returns 0 on transport errors)
pre_head=$(git -C "$PLAN_DIR" rev-parse HEAD 2>/dev/null || echo none)
pre_next=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)

echo "$(date -Is) stall detected (hb_age=${age}s) -> spawning continuation" >> "$STATE/nudge.log"
cd "$PLAN_DIR" || exit 1
timeout 3900 "$OPENC" run --auto --title "bedlam-nudge $(date -Is)" \
  "You are an unattended continuation agent for the bedlam-re project. Read AGENTS.md at the repo root and follow its workflow EXACTLY (heartbeat, NEXT.md, one bounded work unit, manifest checks, small commit, push, update NEXT.md, stop). If you hit a model or transport error mid-task, stop cleanly and record what you finished in NEXT.md so the next agent resumes. Never start an analyzeHeadless import that is already running or already succeeded (check pgrep and the log first). Do not ask questions. Do not wait for input." \
  >> "$STATE/nudge-run.log" 2>&1
rc=$?

# progress check AFTER spawn
post_head=$(git -C "$PLAN_DIR" rev-parse HEAD 2>/dev/null || echo none)
post_next=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)

if [ "$pre_head" != "$post_head" ] || [ "$pre_next" != "$post_next" ]; then
  rm -f "$STATE/fails"
  echo "$(date -Is) run ended (rc=$rc) PROGRESS: head $pre_head -> $post_head" >> "$STATE/nudge.log"
else
  f=0; [ -f "$STATE/fails" ] && f=$(cat "$STATE/fails" 2>/dev/null || echo 0)
  f=$((f+1)); echo "$f" > "$STATE/fails"
  backoff=$(( 300 * f )); [ "$backoff" -gt 3600 ] && backoff=3600
  echo $(( $(date +%s) + backoff )) > "$STATE/cooldown-until"
  echo "$(date -Is) run ended (rc=$rc) NO PROGRESS (fails=$f) -> cooldown ${backoff}s" >> "$STATE/nudge.log"
fi
exit 0
