#!/usr/bin/env bash
# bedlam-re autonomy nudge v3
# v3: persistent progress tracking. Progress = git HEAD movement OR NEXT.md/
# STATE.md changes OR recent repo-file activity. Credited whenever observed
# (spawn time AND run end), so ghost sessions (client dies rc=1 on provider
# decode errors, server-side session keeps working) still count.
set -u
PLAN_DIR=/home/kato/Documents/bedlam-re
STATE="$PLAN_DIR/.state"
HB="$STATE/heartbeat"
LP="$STATE/last-progress"
STALE=300
MAXSPAWN=8

mkdir -p "$STATE"
[ -f "$STATE/PLAN-COMPLETE" ] && exit 0
[ -f "$STATE/PAUSE" ] && exit 0

if [ -f "$STATE/cooldown-until" ]; then
  now=$(date +%s); cu=$(cat "$STATE/cooldown-until" 2>/dev/null || echo 0)
  [ "$now" -lt "$cu" ] && exit 0
  rm -f "$STATE/cooldown-until"
fi


# --- status report (every run; cheap) ---
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

# --- progress probe (reusable) ---
cd "$PLAN_DIR" || exit 1
head_now=$(git rev-parse HEAD 2>/dev/null || echo none)
next_mt=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)
state_mt=$(stat -c %Y "$STATE/STATE.md" 2>/dev/null || echo 0)

credit_if_progress() {
  local lp_head lp_ts
  lp_head="none"; lp_ts=0
  if [ -f "$LP" ]; then read -r lp_head lp_ts < "$LP" 2>/dev/null || true; fi
  if [ "$head_now" != "$lp_head" ] || [ "$next_mt" -gt "$lp_ts" ] || [ "$state_mt" -gt "$lp_ts" ]; then
    echo "$(date +%s)" > /tmp/bedlam-lp-ts.$$
    echo "$head_now $(cat /tmp/bedlam-lp-ts.$$)" > "$LP"
    rm -f /tmp/bedlam-lp-ts.$$
    return 0
  fi
  # uncommitted recent work counts too (agents mid-unit)
  local recent
  recent=$(find . -path ./.git -prune -o -path "./game-data*" -prune -o -path "./derived*" -prune -o \
      -path "./target" -prune -o -path "./tools/inspect/target" -prune -o \
      -not -path "./.state/heartbeat" -not -path "./.state/fails" -not -path "./.state/spawns" \
      -not -path "./.state/cooldown-until" -not -path "./.state/last-progress" \
      -not -path "./.state/nudge.log" -not -path "./.state/nudge-run.log" \
      -type f -newermt "-900 seconds" -print -quit 2>/dev/null)
  [ -n "$recent" ] && return 0
  return 1
}

# --- credit check BEFORE any spawn decision: catch ghost commits first ---
if credit_if_progress; then
  if [ -f "$STATE/fails" ] || [ -f "$STATE/cooldown-until" ]; then
    echo "$(date -Is) progress observed (head=$head_now) - crediting ghost work, resetting fails" >> "$STATE/nudge.log"
  fi
  rm -f "$STATE/fails" "$STATE/cooldown-until"
fi

# --- staleness gate ---
age=-1
if [ -f "$HB" ]; then
  age=$(( $(date +%s) - $(stat -c %Y "$HB") ))
  [ "$age" -lt "$STALE" ] && exit 0
fi

# --- hourly spawn cap ---
h=0; c=0
[ -f "$STATE/spawns" ] && read -r h c < "$STATE/spawns" 2>/dev/null || true
nowh=$(( $(date +%s) / 3600 ))
if [ "$h" != "$nowh" ]; then c=0; fi
if [ "$c" -ge "$MAXSPAWN" ]; then
  echo "$(date -Is) spawn cap reached ($c this hour) - standing down" >> "$STATE/nudge.log"
  exit 0
fi
echo "$nowh $((c+1))" > "$STATE/spawns"

for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  if [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ]; then
    tail -c 262144 "$f" > "$f.t" && mv "$f.t" "$f"
  fi
done

OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/lib/node_modules/@opencode-ai/cli/bin/opencode2.exe
command -v opencode2 >/dev/null 2>&1 && OPENC=opencode2

echo "$(date -Is) stall detected (hb_age=${age}s) -> spawning continuation" >> "$STATE/nudge.log"
cd "$PLAN_DIR" || exit 1
timeout 3900 "$OPENC" run --auto --title "bedlam-nudge $(date -Is)" \
  "You are an unattended continuation agent for the bedlam-re project. Read AGENTS.md at the repo root and follow its workflow EXACTLY (heartbeat, NEXT.md, one bounded work unit, manifest checks, small commit, push, update NEXT.md, stop). IMPORTANT: commit EARLY and OFTEN in the smallest meaningful increments - your client may disconnect mid-run while the server keeps you alive; early commits are how your progress gets seen and credited. If you hit a model or transport error mid-task, stop cleanly and record what you finished in NEXT.md. Never start an analyzeHeadless import that is already running or already succeeded. Do not ask questions. Do not wait for input." \
  >> "$STATE/nudge-run.log" 2>&1
rc=$?

# --- post-run credit check ---
head_now=$(git rev-parse HEAD 2>/dev/null || echo none)
next_mt=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)
state_mt=$(stat -c %Y "$STATE/STATE.md" 2>/dev/null || echo 0)

if credit_if_progress; then
  rm -f "$STATE/fails" "$STATE/cooldown-until"
  echo "$(date -Is) run ended (rc=$rc) PROGRESS observed -> counters reset" >> "$STATE/nudge.log"
else
  f=0; [ -f "$STATE/fails" ] && f=$(cat "$STATE/fails" 2>/dev/null || echo 0)
  f=$((f+1)); echo "$f" > "$STATE/fails"
  backoff=$(( 300 * f )); [ "$backoff" -gt 3600 ] && backoff=3600
  echo $(( $(date +%s) + backoff )) > "$STATE/cooldown-until"
  note="no activity"
  if grep -q "Decode error" "$STATE/nudge-run.log" 2>/dev/null && [ "$(tail -c 200000 "$STATE/nudge-run.log" | grep -c "Decode error")" -gt 0 ]; then
    note="provider decode errors (z.ai) - treat as infra flake, not agent failure"
  fi
  echo "$(date -Is) run ended (rc=$rc) NO PROGRESS (fails=$f) -> cooldown ${backoff}s [$note]" >> "$STATE/nudge.log"
fi
exit 0
