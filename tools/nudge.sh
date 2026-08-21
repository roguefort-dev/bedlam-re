#!/usr/bin/env bash
# bedlam-re autonomy nudge v5 - parallel agents with stable identities.
# v5: BEDLAM_PLAN_DIR/lock/reaper/network-check injectable for hermetic tests;
# UUID slot ids with explicit transient unit names; per-task failure cooldowns;
# progress credit requires an attributed substantive commit (no mtime heuristic).
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
STATE="$PLAN_DIR/.state"
HB="$STATE/heartbeat"
LP="$STATE/last-progress"
CLAIMS="$STATE/claims"
STALE=300
MAXSPAWN=16
MAXAGENTS=3          # adaptive concurrency: target ceiling
CONC_MIN=1
# Concurrency is pinned to 1: concurrent workers share one git worktree/index
# and one NEXT.md queue with no serialization, so >1 worker is unsafe. Raise
# only after isolated per-worker worktrees plus a serialized merge step exist.
# The adaptive controller (conc_up/conc_down) is retained, but its ceiling is
# pinned here.
CONC_MAX=1
CONC_FILE="$STATE/concurrency"
CONC_DOWN_TS="$STATE/conc-degraded-at"
NUDGE_LOCK=${NUDGE_LOCK:-/tmp/bedlam-nudge.lock}
REAPER=${REAPER_OVERRIDE:-$SCRIPT_DIR/nudge-reap-claims.sh}
NETWORK_WATCHDOG=${NETWORK_WATCHDOG_OVERRIDE:-$SCRIPT_DIR/network-watchdog.sh}
NOTIFY_SEND=${NOTIFY_SEND-notify-send}
SYSTEMD_RUN=${SYSTEMD_RUN_OVERRIDE:-systemd-run}

mkdir -p "$STATE" "$CLAIMS"
[ -f "$STATE/PLAN-COMPLETE" ] && exit 0
# A watchdog-owned PAUSE whose owning pid is dead (e.g. reboot mid-repair)
# strands the loop: PAUSE blocks workers, no workers means no taskfails events,
# and with the watchdog timer gone nothing would ever run its stale-token
# recovery. Detect it here and ring the supervisor bell (event-driven; the
# watchdog itself stays the recovery authority under its singleton lock).
if [ -f "$STATE/PAUSE" ]; then
  pb=$(cat "$STATE/PAUSE" 2>/dev/null || true)
  case "$pb" in
    llm-watchdog\ *)
      wp=$(printf "%s\n" "$pb" | awk "{print \$2}")
      wts=$(printf "%s\n" "$pb" | awk "{print \$3}")
      if ! kill -0 "$wp" 2>/dev/null || [ $(( $(date +%s) - ${wts:-0} )) -gt 2700 ]; then
        echo "$(date -Is) watchdog-owned PAUSE stranded (pid=$wp); triggering supervisor recovery" >> "$STATE/nudge.log"
        if [ -n "${SYSTEMCTL_OVERRIDE:-}" ]; then
          "$SYSTEMCTL_OVERRIDE" --user start bedlam-llm-watchdog.service >/dev/null 2>&1 || true
        elif [ -z "${SYSTEMD_RUN_OVERRIDE:-}" ]; then
          systemctl --user start bedlam-llm-watchdog.service >/dev/null 2>&1 || true
        fi
      fi
      ;;
  esac
  exit 0
fi

# Per-task failure state expires after a day regardless of queue rewrites.
find "$STATE/taskfails" "$STATE/taskcooldown" -type f -mtime +1 -delete 2>/dev/null || true

task_hash_for() {
  sed -n "s/^[[:space:]]*$1\.[[:space:]]*//p" "$STATE/NEXT.md" 2>/dev/null | head -n 1 | sha256sum | cut -c1-16
}

write_status() {
  local hb_age last_h n_new stalled tf
  hb_age=$(( $(date +%s) - $(stat -c %Y "$HB" 2>/dev/null || date +%s) ))
  last_h=$(git -C "$PLAN_DIR" log -1 --format="%h %ad %s" --date=format:"%H:%M" 2>/dev/null || echo "none")
  n_new=$(git -C "$PLAN_DIR" log --oneline --since="75 minutes ago" 2>/dev/null | wc -l)
  tf=$(ls "$STATE/taskfails" 2>/dev/null | wc -l)
  {
    echo "# bedlam-re status - $(date +"%H:%M") $(date +%F)"
    echo
    echo "- last commit: $last_h"
    echo "- commits in last ~75min: $n_new"
    echo "- tasks with failure streaks: $tf"
    echo "- watchdog: $(sed -n "s/^state=//p" "$STATE/llm-watchdog-verdict" 2>/dev/null | tail -n 1)"
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
    if [ -n "$NOTIFY_SEND" ] && command -v "$NOTIFY_SEND" >/dev/null 2>&1; then
      if [ "$n_new" -gt 0 ]; then
        "$NOTIFY_SEND" -u normal "bedlam-re progress" "$n_new commit(s) in the last hour. Last: $last_h" 2>/dev/null || true
      elif [ "$tf" -gt 0 ]; then
        "$NOTIFY_SEND" -u critical "bedlam-re STALLED" "$tf task(s) with failure streaks. Check .state/STATUS.md" 2>/dev/null || true
      fi
    fi
    echo "$ch 1" > "$STATE/notified"
  fi
}

exec 9>"$NUDGE_LOCK"
flock -n 9 || { echo "$(date -Is) controller lock busy; standing down" >> "$STATE/nudge.log"; exit 0; }

write_status

# The existing minute timer also acts as the connectivity watchdog. Offline
# passes stop here; the first restored pass repairs OpenCode and resumes below.
"$NETWORK_WATCHDOG"
watchdog_rc=$?
if [ "$watchdog_rc" -eq 75 ]; then exit 0; fi
if [ "$watchdog_rc" -ne 0 ]; then
  echo "$(date -Is) network watchdog failed (rc=$watchdog_rc) - standing down" >> "$STATE/nudge.log"
  exit 0
fi

# Reap unlocked reservations and dead-worker claims after five minutes.
# Live workers hold an advisory lock, so their claim age is unbounded.
"$REAPER" "$CLAIMS" "$STATE/nudge.log"

# --- adaptive concurrency controller ---
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

# Progress credit requires a new substantive commit reachable from the previous
# baseline. Working-tree mtimes are deliberately NOT progress: operator or
# unrelated edits must never clear task failure state.
credit_if_progress() {
  local lp_head lp_ts substantive
  lp_head="none"; lp_ts=0; substantive=0
  if [ -f "$LP" ]; then read -r lp_head lp_ts < "$LP" 2>/dev/null || true; fi
  if [ "$head_now" != "$lp_head" ]; then
    if git cat-file -e "$lp_head^{commit}" 2>/dev/null \
        && git diff --name-only "$lp_head..$head_now" 2>/dev/null | grep -qv "^\.state/"; then
      substantive=1
    elif [ "$lp_head" = none ]; then
      substantive=1
    fi
    echo "$head_now $(date +%s.%N)" > "$LP"
  fi
  [ "$substantive" -eq 1 ] && return 0
  return 1
}

if credit_if_progress; then
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
if [ "$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)" -eq 0 ]    && [ $(( $(date +%s) - lastspawn )) -gt 420 ]    && [ ! -d "$STATE/taskfails" -o -z "$(ls "$STATE/taskfails" 2>/dev/null)" ]; then
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
# Clamp a stale higher value from .state/concurrency to the pinned ceiling.
if [ "$cur_conc" -gt "$CONC_MAX" ]; then
  cur_conc=$CONC_MAX
fi
ncl=$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)
if [ "$ncl" -ge "$cur_conc" ]; then
  echo "$(date -Is) concurrency full ($ncl/$cur_conc agents, adaptive) - standing down" >> "$STATE/nudge.log"
  exit 0
fi

# free queue item numbers = Now-section entries not claimed
free_items=$("$SCRIPT_DIR/nudge-free-items.py" "$STATE/NEXT.md" "$CLAIMS")
if [ -z "$free_items" ]; then
  echo "$(date -Is) no unattended Now items are available - standing down" >> "$STATE/nudge.log"
  exit 0
fi

# Skip items whose task is cooling down after repeated attributable failures.
chosen=""
cooling=0
for it in $free_items; do
  th=$(task_hash_for "$it")
  cool="$STATE/taskcooldown/$th"
  if [ -f "$cool" ]; then
    if [ "$(date +%s)" -lt "$(cat "$cool" 2>/dev/null || echo 0)" ]; then
      cooling=1
      continue
    fi
    rm -f "$cool"
  fi
  chosen=$it
  break
done
if [ -z "$chosen" ]; then
  if [ "$cooling" -eq 1 ]; then
    echo "$(date -Is) all free items are cooling down after failures - standing down" >> "$STATE/nudge.log"
  else
    echo "$(date -Is) no unattended Now items are available - standing down" >> "$STATE/nudge.log"
  fi
  exit 0
fi
item=$chosen

slotid=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || date +%s%N)
unit_name="bedlam-nudge-item${item}-${slotid}"
echo "$nowh $((c+1))" > "$STATE/spawns"

for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  if [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ]; then
    tail -c 262144 "$f" > "$f.t" && mv "$f.t" "$f"
  fi
done

date +%s > "$STATE/last-spawn-ts"
echo "reserved $(date -Is)" > "$CLAIMS/$item-$slotid.claim"
echo "$(date -Is) spawning agent for queue item $item as unit $unit_name ($((ncl+1))/$cur_conc slots)" >> "$STATE/nudge.log"

# Worker owns the API call and reports its exact per-run result back to the
# adaptive controller under the same flock. The explicit unit name makes the
# transient worker enumerable without process-command matching.
if ! "$SYSTEMD_RUN" --user --collect --unit "$unit_name" "$SCRIPT_DIR/nudge-agent.sh" "$item" "$slotid" >> "$STATE/nudge.log" 2>&1; then
  echo "$(date -Is) systemd-run failed for unit $unit_name; dropping reservation" >> "$STATE/nudge.log"
  rm -f "$CLAIMS/$item-$slotid.claim"
fi

exit 0
