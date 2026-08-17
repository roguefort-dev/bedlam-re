#!/usr/bin/env bash
# bedlam-re autonomy nudge v4 - parallel agents
# v4: up to MAXAGENTS concurrent opencode2 sessions (account limit 10;
# CONC_MAX=3 is the adaptive ceiling). Slot accounting = claim files in
# .state/claims/ (written by agents at start, removed at end) - counts
# GHOST sessions too. Claims older than CLAIM_TTL are reaped. Each agent
# claims a DIFFERENT numbered queue item; claim file prevents duplicates.
set -u
PLAN_DIR=/home/kato/Documents/bedlam-re
STATE="$PLAN_DIR/.state"
HB="$STATE/heartbeat"
LP="$STATE/last-progress"
CLAIMS="$STATE/claims"
STALE=300
MAXSPAWN=16
MAXAGENTS=3          # adaptive concurrency: target ceiling
CLAIM_TTL=4200
CONC_MIN=1
CONC_MAX=3
CONC_FILE="$STATE/concurrency"
CONC_DOWN_TS="$STATE/conc-degraded-at"

mkdir -p "$STATE" "$CLAIMS"
[ -f "$STATE/PLAN-COMPLETE" ] && exit 0
[ -f "$STATE/PAUSE" ] && exit 0

if [ -f "$STATE/cooldown-until" ]; then
  now=$(date +%s); cu=$(cat "$STATE/cooldown-until" 2>/dev/null || echo 0)
  if [ "$now" -lt "$cu" ]; then exit 0; fi
  rm -f "$STATE/cooldown-until"
fi

write_status() {
  local hb_age last_h n_new stalled
  hb_age=$(( $(date +%s) - $(stat -c %Y "$HB" 2>/dev/null || date +%s) ))
  last_h=$(git -C "$PLAN_DIR" log -1 --format="%h %ad %s" --date=format:"%H:%M" 2>/dev/null || echo "none")
  n_new=$(git -C "$PLAN_DIR" log --oneline --since="75 minutes ago" 2>/dev/null | wc -l)
  {
    echo "# bedlam-re status - $(date +"%H:%M") $(date +%F)"
    echo
    echo "- last commit: $last_h"
    echo "- commits in last ~75min: $n_new"
    if [ -f "$STATE/fails" ]; then
      echo "- loop: FAILING (fails=$(cat "$STATE/fails")) cooldown until $(date -d @$(cat "$STATE/cooldown-until" 2>/dev/null || echo 0) +%H:%M 2>/dev/null)"
    else
      echo "- loop: healthy, no cooldown"
    fi
    if pgrep -f "opencode2 run" >/dev/null 2>&1; then
      echo "- agent: RUNNING (client or ghost)"
    else
      echo "- agent: idle"
    fi
    echo
    echo "## last 5 commits"
    git -C "$PLAN_DIR" log --oneline --format="- %h %ad %s" --date=format:"%H:%M" -5 2>/dev/null
    echo
    echo "## queue top"
    sed -n "3,8p" "$STATE/NEXT.md" 2>/dev/null
  } > "$STATE/STATUS.md"
  # hourly desktop notification: new work landed -> tell the user
  local nh ncount
  nh=$(( $(date +%s) / 3600 )); ncount=0
  if [ -f "$STATE/notified" ]; then read -r nh ncount < "$STATE/notified" 2>/dev/null || true; fi
  local ch; ch=$(( $(date +%s) / 3600 ))
  if [ "$ch" -ne "$nh" ]; then
    if command -v notify-send >/dev/null 2>&1; then
      if [ "$n_new" -gt 0 ]; then
        notify-send -u normal "bedlam-re progress" "$n_new commit(s) in the last hour. Last: $last_h" 2>/dev/null || true
      elif [ -f "$STATE/fails" ] && [ "$(cat "$STATE/fails")" -ge 3 ]; then
        notify-send -u critical "bedlam-re STALLED" "No progress ~75min, fails=$(cat "$STATE/fails"). Check .state/STATUS.md" 2>/dev/null || true
      fi
    fi
    echo "$ch 1" > "$STATE/notified"
  fi
}

exec 9>/tmp/bedlam-nudge.lock
flock -n 9 || exit 0

write_status

# reap stale claims (ghost died without releasing)
now=$(date +%s)
for c in "$CLAIMS"/*.claim; do
  [ -e "$c" ] || continue
  ts=$(stat -c %Y "$c" 2>/dev/null || echo 0)
  if [ $(( now - ts )) -gt "$CLAIM_TTL" ]; then
    echo "$(date -Is) reaped stale claim $(basename "$c") (age $((now-ts))s)" >> "$STATE/nudge.log"
    rm -f "$c"
  fi
done


# --- adaptive concurrency controller ---
# state: $CONC_FILE holds current limit. On a failed run (no progress +
# provider errors) decrement (floor 1) and timestamp the degradation.
# When progress is credited and >=3600s since last degradation, increment
# back up (ceiling CONC_MAX). Written under the flock, so race-free.
get_conc() { cat "$CONC_FILE" 2>/dev/null || echo "$CONC_MAX"; }
conc_down() {
  local cur; cur=$(get_conc)
  if [ "$cur" -gt "$CONC_MIN" ]; then
    echo $((cur-1)) > "$CONC_FILE"
    date +%s > "$CONC_DOWN_TS"
    echo "$(date -Is) concurrency degraded $cur -> $((cur-1)) (failures)" >> "$STATE/nudge.log"
  fi
}
conc_up() {
  local cur lastdown
  cur=$(get_conc)
  lastdown=$(cat "$CONC_DOWN_TS" 2>/dev/null || echo 0)
  if [ "$cur" -lt "$CONC_MAX" ] && [ $(( $(date +%s) - lastdown )) -ge 3600 ]; then
    echo $((cur+1)) > "$CONC_FILE"
    echo "$(date -Is) concurrency recovered $cur -> $((cur+1)) (1h stable)" >> "$STATE/nudge.log"
  fi
}

cd "$PLAN_DIR" || exit 1
head_now=$(git rev-parse HEAD 2>/dev/null || echo none)
next_mt=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)
state_mt=$(stat -c %Y "$STATE/STATE.md" 2>/dev/null || echo 0)

credit_if_progress() {
  local lp_head lp_ts
  lp_head="none"; lp_ts=0
  if [ -f "$LP" ]; then read -r lp_head lp_ts < "$LP" 2>/dev/null || true; fi
  if [ "$head_now" != "$lp_head" ] || [ "$next_mt" -gt "$lp_ts" ] || [ "$state_mt" -gt "$lp_ts" ]; then
    echo "$head_now $(date +%s)" > "$LP"
    return 0
  fi
  local recent
  recent=$(find . -path ./.git -prune -o -path "./game-data*" -prune -o -path "./derived*" -prune -o \
      -path "./target" -prune -o -path "./tools/inspect/target" -prune -o \
      -not -path "./.state/heartbeat" -not -path "./.state/fails" -not -path "./.state/spawns" \
      -not -path "./.state/cooldown-until" -not -path "./.state/last-progress" \
      -not -path "./.state/nudge.log" -not -path "./.state/nudge-run.log" \
      -not -path "./.state/STATUS.md" -not -path "./.state/notified" \
      -not -path "./.state/claims/*" \
      -type f -newermt "-900 seconds" -print -quit 2>/dev/null)
  [ -n "$recent" ] && return 0
  return 1
}

if credit_if_progress; then
  if [ -f "$STATE/fails" ] || [ -f "$STATE/cooldown-until" ]; then
    echo "$(date -Is) progress observed (head=$head_now) - crediting, resetting fails" >> "$STATE/nudge.log"
  fi
  rm -f "$STATE/fails" "$STATE/cooldown-until"
  conc_up
fi

# staleness gate: heartbeat freshness matters only when no agent holds a
# claim (sibling spawning is driven by claim count, not heartbeat - an
# already-running agent touching the heartbeat must not block its siblings)
ncl_early=$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)
age=-1
if [ "$ncl_early" -eq 0 ] && [ -f "$HB" ]; then
  age=$(( $(date +%s) - $(stat -c %Y "$HB") ))
  [ "$age" -lt "$STALE" ] && exit 0
fi

# failure signal for the controller: heartbeat stale AND no live claims
# means agents are dying before even claiming (provider-level rejection).
lastspawn=$(cat "$STATE/last-spawn-ts" 2>/dev/null || echo 0)
if [ "$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)" -eq 0 ]    && [ $(( $(date +%s) - lastspawn )) -gt 420 ]    && [ ! -f "$STATE/fails" ]; then
  conc_down
fi

# spawn budget (all agents combined, per hour)
h=0; c=0
[ -f "$STATE/spawns" ] && read -r h c < "$STATE/spawns" 2>/dev/null || true
nowh=$(( $(date +%s) / 3600 ))
if [ "$h" != "$nowh" ]; then c=0; fi
if [ "$c" -ge "$MAXSPAWN" ]; then
  echo "$(date -Is) spawn cap reached ($c this hour) - standing down" >> "$STATE/nudge.log"
  exit 0
fi

# concurrency gate
cur_conc=$(get_conc)
ncl=$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)
if [ "$ncl" -ge "$cur_conc" ]; then
  echo "$(date -Is) concurrency full ($ncl/$cur_conc agents, adaptive) - standing down" >> "$STATE/nudge.log"
  exit 0
fi

# free queue item numbers = Now-section entries not claimed
free_items=$(python3 - << "PY"
import re, os
try: q = open(".state/NEXT.md").read()
except: q = ""
now = q.split("## Now",1)[1].split("## Backlog",1)[0] if "## Now" in q else ""
items = re.findall(r"^\s*(\d+)\.\s+\[", now, re.M)
claimed = {f.split("-")[0] for f in os.listdir(".state/claims") if f.endswith(".claim")}
print(" ".join(i for i in items if i not in claimed))
PY
)
if [ -z "$free_items" ]; then
  echo "$(date -Is) all Now queue items are claimed - standing down" >> "$STATE/nudge.log"
  exit 0
fi
item=$(echo "$free_items" | awk "{print \$1}")

slotid=$(date +%s)
echo "$nowh $((c+1))" > "$STATE/spawns"

for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  if [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ]; then
    tail -c 262144 "$f" > "$f.t" && mv "$f.t" "$f"
  fi
done

OPENC=opencode2
command -v opencode2 >/dev/null 2>&1 || OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/lib/node_modules/@opencode-ai/cli/bin/opencode2.exe

date +%s > "$STATE/last-spawn-ts"
echo "reserved $(date -Is)" > "$CLAIMS/$item-$slotid.claim"
echo "$(date -Is) spawning agent for queue item $item ($((ncl+1))/$cur_conc slots)" >> "$STATE/nudge.log"

# Worker owns the API call and reports its exact per-run result back to the
# adaptive controller under the same flock. Per-agent logs prevent error
# attribution across concurrent sessions.
systemd-run --user --collect "$PLAN_DIR/tools/nudge-agent.sh" "$item" "$slotid"   >> "$STATE/nudge.log" 2>&1

exit 0
