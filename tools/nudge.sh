#!/usr/bin/env bash
# bedlam-re autonomy nudge: spawn a continuation agent when work stalls.
# Runs every minute (systemd user timer + crontab). Cheap shell check; LLM only on stall.
set -u
PLAN_DIR=/home/kato/Documents/bedlam-re
STATE="$PLAN_DIR/.state"
HB="$STATE/heartbeat"
STALE=420  # seconds; main agent considered active if heartbeat younger

mkdir -p "$STATE"
[ -f "$STATE/PLAN-COMPLETE" ] && exit 0
[ -f "$STATE/PAUSE" ] && exit 0

exec 9>/tmp/bedlam-nudge.lock
flock -n 9 || exit 0

age=-1
if [ -f "$HB" ]; then
  age=$(( $(date +%s) - $(stat -c %Y "$HB") ))
  [ "$age" -lt "$STALE" ] && exit 0
fi

# rotate logs if huge
for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ] && tail -c 262144 "$f" > "$f.t" && mv "$f.t" "$f"
done

OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/lib/node_modules/@opencode-ai/cli/bin/opencode2.exe
command -v opencode2 >/dev/null 2>&1 && OPENC=opencode2

echo "$(date -Is) stall detected (hb_age=${age}s) -> spawning continuation" >> "$STATE/nudge.log"
cd "$PLAN_DIR" || exit 1
timeout 3900 "$OPENC" run --auto --title "bedlam-nudge $(date -Is)"   "You are an unattended continuation agent for the bedlam-re project. Read AGENTS.md at the repo root and follow its workflow EXACTLY (heartbeat, NEXT.md, one bounded work unit, manifest checks, small commit, push, update NEXT.md, stop). Do not ask questions. Do not wait for input."   >> "$STATE/nudge-run.log" 2>&1
echo "$(date -Is) continuation run ended (exit $?)" >> "$STATE/nudge.log"
exit 0
