#!/usr/bin/env bash
# bedlam-re autonomy nudge v5 - parallel agents with stable identities.
# v5: BEDLAM_PLAN_DIR/lock/reaper/network-check injectable for hermetic tests;
# UUID slot ids with explicit transient unit names; structured failure beacons;
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
STATE_HELPER="$SCRIPT_DIR/nudge-state.py"
WAIT_EXECUTOR="$SCRIPT_DIR/nudge-wait.py"
LOCK_HELPER="$SCRIPT_DIR/nudge-lock.py"

if [ "${NUDGE_CONTROLLER_LOCK_HELD:-0}" != 1 ]; then
  exec "$LOCK_HELPER" lock-run "$NUDGE_LOCK" nonblocking \
    env NUDGE_CONTROLLER_LOCK_HELD=1 "$0" "$@"
fi

"$STATE_HELPER" ensure-dir "$STATE" >/dev/null 2>&1 || exit 75
"$STATE_HELPER" ensure-dir "$CLAIMS" >/dev/null 2>&1 || exit 75

log_line() {
  "$STATE_HELPER" append-text "$STATE/nudge.log" "$(date -Is) $*"$'\n' 2>/dev/null || true
}
beacon_failure() {
  local kind=$1 reason=$2 ordinal=${3:-1} item_id=${4:-automation-state} gate=${5:-automatic-repair}
  local session before
  session="controller-$(date +%s)-$$-$kind"
  before=$("$STATE_HELPER" queue-snapshot "$STATE/NEXT.md") || return 1
  "$STATE_HELPER" publish-failure "$STATE/automation-failures" "$ordinal" "$item_id" \
    "$gate" "$session" "$kind" "$reason" "controller" true \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$before" "$STATE/NEXT.md" \
    2>/dev/null || true
  if [ -n "${SYSTEMCTL_OVERRIDE:-}" ]; then
    "$SYSTEMCTL_OVERRIDE" --user start bedlam-llm-watchdog.service >/dev/null 2>&1 || true
  elif [ -z "${SYSTEMD_RUN_OVERRIDE:-}" ]; then
    systemctl --user start bedlam-llm-watchdog.service >/dev/null 2>&1 || true
  fi
}
mutable_state_invalid() {
  local seam=$1 detail=$2
  log_line "invalid mutable numeric state $seam: $detail; structured repair required"
  beacon_failure mutable-state-invalid "$seam invalid: $detail"
  exit 2
}

# Self-heal the event trigger layer: a start-limit hit on a path unit
# (observed 2026-08-22 04:29) silently kills the claims/taskfails event
# edges and the loop degrades to chain+timer only. Re-arm on every pass;
# idempotent, guarded for hermetic runs.
if [ -z "${SYSTEMD_RUN_OVERRIDE:-}" ]; then
  if [ -n "${SYSTEMCTL_OVERRIDE:-}" ]; then SCMD="$SYSTEMCTL_OVERRIDE"; else SCMD=systemctl; fi
  for pu in bedlam-nudge.path bedlam-llm-watchdog.path; do
    if ! "$SCMD" --user is-active --quiet "$pu" 2>/dev/null; then
      "$SCMD" --user reset-failed "$pu" >/dev/null 2>&1 || true
      "$SCMD" --user start "$pu" >/dev/null 2>&1 || true
      log_line "re-armed event trigger $pu (was inactive/failed)"
    fi
  done
fi
# PLAN-COMPLETE is controller output, never an input authority. A worker can
# write perfect-looking JSON, so every pass discards it and derives completion
# from a fresh bounded validation only after observing an empty active queue.
if [ -e "$STATE/PLAN-COMPLETE" ] || [ -L "$STATE/PLAN-COMPLETE" ]; then
  "$STATE_HELPER" unlink "$STATE/PLAN-COMPLETE" 2>/dev/null || true
  log_line "untrusted prior PLAN-COMPLETE removed; fresh controller validation required"
fi
# A watchdog-owned PAUSE whose owning pid is dead (e.g. reboot mid-repair)
# strands the loop: PAUSE blocks workers, no workers means no taskfails events,
# and with the watchdog timer gone nothing would ever run its stale-token
# recovery. Detect it here and ring the supervisor bell (event-driven; the
# watchdog itself stays the recovery authority under its singleton lock).
if [ -f "$STATE/PAUSE" ]; then
  pb=$(cat "$STATE/PAUSE" 2>/dev/null || true)
  case "$pb" in
    llm-watchdog\ *)
      read -r _ wp wts extra <<< "$pb"
      if [[ ! "$wp" =~ ^[1-9][0-9]{0,9}$ ]] || [[ ! "$wts" =~ ^[0-9]{1,10}$ ]] || [ -n "${extra:-}" ]; then
        log_line "invalid watchdog PAUSE timestamp or pid; structured repair required"
        beacon_failure mutable-state-invalid "watchdog PAUSE timestamp invalid"
        exit 2
      fi
      now_epoch=$(date +%s)
      if [ "$wts" -gt $((now_epoch + 300)) ]; then
        log_line "invalid watchdog PAUSE timestamp in future; structured repair required"
        beacon_failure mutable-state-invalid "watchdog PAUSE timestamp invalid future value"
        exit 2
      fi
      if ! kill -0 "$wp" 2>/dev/null || [ $((now_epoch - wts)) -gt 2700 ]; then
        log_line "watchdog-owned PAUSE stranded (pid=$wp); triggering supervisor recovery"
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

# Queue state, due waits, and empty-queue completion outrank every online,
# heartbeat, budget, cooldown, and concurrency decision.
"$REAPER" "$CLAIMS" "$STATE/nudge.log" >/dev/null 2>&1 || exit 2
queue_state=$("$SCRIPT_DIR/nudge-free-items.py" "$STATE/NEXT.md" "$CLAIMS" --state-v1 2>/dev/null)
queue_rc=$?
if [ "$queue_rc" -ne 0 ]; then
  log_line "queue INVALID-DEADLOCKED (parser rc=$queue_rc) - repair required; refusing idle/spawn"
  exit "$queue_rc"
fi
if grep -q '\[WAITING-AUTOMATIC\]' "$STATE/NEXT.md"; then
  set +e
  wait_result=$("$WAIT_EXECUTOR" run "$STATE/NEXT.md" "$STATE/automatic-waits" 2>/dev/null)
  wait_rc=$?
  set -e
  if [ "$wait_rc" -ne 0 ]; then
    wait_id=$(printf '%s\n' "$wait_result" | sed -n 's/.*id=\([a-z0-9._-]*\).*/\1/p' | tail -1)
    [ -n "$wait_id" ] || wait_id=automatic-wait
    wait_line=$(grep -m1 "\[id=$wait_id\]" "$STATE/NEXT.md" 2>/dev/null || true)
    wait_ordinal=$(printf '%s\n' "$wait_line" | sed -n 's/^\([1-9][0-9]*\)\..*/\1/p')
    wait_gate=$(printf '%s\n' "$wait_line" | sed -n 's/.*\[gate=\([^]]*\)\].*/\1/p')
    [ -n "$wait_ordinal" ] || wait_ordinal=1
    [ -n "$wait_gate" ] || wait_gate=automatic-wait
    wait_kind=wait-timeout
    [[ "$wait_result" == *deadline-expired* ]] && wait_kind=deadline-expired
    beacon_failure "$wait_kind" "${wait_result:-automatic wait expired}" "$wait_ordinal" "$wait_id" "$wait_gate"
    log_line "${wait_result:-INVALID-DEADLOCKED automatic wait} - repair required"
    exit 2
  fi
  queue_state=$("$SCRIPT_DIR/nudge-free-items.py" "$STATE/NEXT.md" "$CLAIMS" --state-v1 2>/dev/null)
  queue_rc=$?
  [ "$queue_rc" -eq 0 ] || exit "$queue_rc"
fi
if [ "$queue_state" = REQUIRED-QUEUE-EMPTY ]; then
  completion_report="$STATE/required-gates-report.json"
  completion_output="$STATE/PLAN-COMPLETE"
  "$STATE_HELPER" unlink "$completion_output" 2>/dev/null || true
  set +e
  completion_proof=$("$LOCK_HELPER" lock-run "$STATE/.queue.lock" blocking \
    "$STATE_HELPER" complete-from-head "$PLAN_DIR" "$completion_report" "$completion_output" 2>&1)
  validation_rc=$?
  set -e
  if [ "$validation_rc" -eq 0 ]; then
    set +e
    completion_decision=$(printf '%s' "$completion_proof" | \
      "$LOCK_HELPER" lock-run "$STATE/.queue.lock" blocking \
      "$STATE_HELPER" accept-completion "$PLAN_DIR" "$completion_output" "$STATE/nudge.log" 2>&1)
    decision_rc=$?
    set -e
    if [ "$decision_rc" -eq 0 ] && printf '%s\n' "$completion_decision" | \
        python3 -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value.get("schema") == "completion-decision-v1" and value.get("status") == "accepted" else 1)'; then
      exit 0
    fi
    [ -z "$completion_decision" ] || "$STATE_HELPER" append-text "$STATE/nudge.log" \
      "$(date -Is) completion acceptance rejected: $completion_decision"$'\n' 2>/dev/null || true
  else
    [ -z "$completion_proof" ] || "$STATE_HELPER" append-text "$STATE/nudge.log" \
      "$(date -Is) completion validation rejected: $completion_proof"$'\n' 2>/dev/null || true
  fi
  "$STATE_HELPER" unlink "$completion_output" 2>/dev/null || true
  # A completion-basis change is the DESIGNED invalidation of an in-flight
  # sealed-HEAD run (D234): a watchdog repair commit or an operator commit
  # landing mid-validation moves HEAD (or rewrites the queue note), the
  # atomic verdict must not publish, and the next tick re-validates the new
  # HEAD from scratch. Beaconing completion-missing for it instead made
  # every repair commit mint the next failure marker, whose forced repair
  # commit killed the retry in turn - the 2026-08-28 completion-missing
  # livelock. Real rejections (validator rc!=0, wrapper timeout, malformed
  # basis) still beacon below.
  if [[ "$completion_proof" == *"completion basis changed"* ]]; then
    log_line "completion basis changed during validation; sealed verdict withheld - benign retry on next tick"
    exit 0
  fi
  log_line "required Now queue is empty but full required-gates validation is incomplete - repair required"
  beacon_failure completion-missing "required queue empty; full offline required-gates validation did not prove P0-P7 completion"
  exit 2
fi

now_epoch=$(date +%s)
if [ -e "$STATE/last-spawn-ts" ]; then
  lastspawn=$("$STATE_HELPER" read-int "$STATE/last-spawn-ts" last-spawn-ts 0 $((now_epoch + 300)) - 2>&1) \
    || mutable_state_invalid last-spawn-ts "$lastspawn"
else
  lastspawn=0
fi
if [ -e "$CONC_FILE" ]; then
  validated_conc=$("$STATE_HELPER" read-int "$CONC_FILE" concurrency-value 0 "$CONC_MAX" - 2>&1) \
    || mutable_state_invalid concurrency-value "$validated_conc"
else
  validated_conc=$CONC_MAX
fi
if [ -e "$CONC_DOWN_TS" ]; then
  validated_degraded=$("$STATE_HELPER" read-int "$CONC_DOWN_TS" concurrency-degraded-at 0 $((now_epoch + 300)) - 2>&1) \
    || mutable_state_invalid concurrency-degraded-at "$validated_degraded"
else
  validated_degraded=0
fi

task_hash_for() {
  sed -n "s/^[[:space:]]*$1\.[[:space:]]*//p" "$STATE/NEXT.md" 2>/dev/null | head -n 1 | sha256sum | cut -c1-16
}

write_status() {
  local hb_age last_h n_new stalled tf
  hb_age=$(( $(date +%s) - $(stat -c %Y "$HB" 2>/dev/null || date +%s) ))
  last_h=$(git -C "$PLAN_DIR" log -1 --format="%h %ad %s" --date=format:"%H:%M" 2>/dev/null || echo "none")
  n_new=$(git -C "$PLAN_DIR" log --oneline --since="75 minutes ago" 2>/dev/null | wc -l)
  tf=$(ls "$STATE/taskfails" 2>/dev/null | wc -l)
  status_payload=$({
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
  })
  "$STATE_HELPER" write-text "$STATE/STATUS.md" "$status_payload" 2>/dev/null || return 1
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
    "$STATE_HELPER" write-text "$STATE/notified" "$ch 1" 2>/dev/null || true
  fi
}

write_status || exit 2

# Due probes and expirations are local work and always precede connectivity.
"$NETWORK_WATCHDOG"
watchdog_rc=$?
if [ "$watchdog_rc" -eq 75 ]; then exit 0; fi
if [ "$watchdog_rc" -ne 0 ]; then
  log_line "network watchdog failed (rc=$watchdog_rc) - standing down"
  exit 0
fi

# --- adaptive concurrency controller ---
get_conc() { printf '%s\n' "$validated_conc"; }
conc_down() {
  local cur; cur=$(get_conc)
  if [ "$cur" -gt "$CONC_MIN" ]; then
    validated_conc=$((cur-1))
    validated_degraded=$(date +%s)
    "$STATE_HELPER" write-text "$CONC_FILE" "$validated_conc" 2>/dev/null || return 1
    "$STATE_HELPER" write-text "$CONC_DOWN_TS" "$validated_degraded" 2>/dev/null || return 1
    log_line "concurrency degraded $cur -> $((cur-1)) (failures)"
  fi
}
conc_up() {
  local cur lastdown
  cur=$(get_conc)
  lastdown=$validated_degraded
  if [ "$cur" -lt "$CONC_MAX" ] && [ $(( $(date +%s) - lastdown )) -ge 3600 ]; then
    validated_conc=$((cur+1))
    "$STATE_HELPER" write-text "$CONC_FILE" "$validated_conc" 2>/dev/null || return 1
    log_line "concurrency recovered $cur -> $((cur+1)) (1h stable)"
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
    "$STATE_HELPER" write-text "$LP" "$head_now $(date +%s.%N)" 2>/dev/null || exit 2
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
if [ "$(ls "$CLAIMS"/*.claim 2>/dev/null | wc -l)" -eq 0 ]    && [ $(( $(date +%s) - lastspawn )) -gt 420 ]    && [ ! -d "$STATE/taskfails" -o -z "$(ls "$STATE/taskfails" 2>/dev/null)" ]; then
  conc_down
fi

# spawn budget (all agents combined, per hour)
h=0; c=0
if [ -e "$STATE/spawns" ]; then
  spawn_fields=$("$STATE_HELPER" read-fields "$STATE/spawns" spawn-hour spawn-count 0 999999999999 0 "$MAXSPAWN" 2>&1) \
    || mutable_state_invalid spawns "$spawn_fields"
  read -r h c <<< "$spawn_fields"
fi
nowh=$(( $(date +%s) / 3600 ))
if [ "$h" != "$nowh" ]; then c=0; fi
if [ "$c" -ge "$MAXSPAWN" ]; then
  log_line "spawn cap reached ($c this hour) - standing down"
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
  log_line "concurrency full ($ncl/$cur_conc agents, adaptive) - standing down"
  exit 0
fi

case "$queue_state" in
  RUNNABLE\ *)
    free_items=${queue_state#RUNNABLE }
    ;;
  CLAIMED-RUNNING)
    log_line "all READY Now items are claimed and running - standing down"
    exit 0
    ;;
  AUTOMATIC-WAIT)
    log_line "queue is in bounded AUTOMATIC-WAIT; probe cadence active"
    exit 0
    ;;
  REQUIRED-QUEUE-EMPTY)
    log_line "queue generation changed to empty after the completion decision boundary"
    exit 2
    ;;
  *)
    log_line "queue INVALID-DEADLOCKED (unknown parser state: $queue_state) - repair required; refusing idle/spawn"
    exit 2
    ;;
esac

item=${free_items%% *}

slotid=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || date +%s%N)
unit_name="bedlam-nudge-item${item}-${slotid}"

set +e
reservation=$("$LOCK_HELPER" lock-run "$CLAIMS/.publish.lock" blocking \
  "$SCRIPT_DIR/nudge-reserve.sh" "$PLAN_DIR" "$item" "$slotid" "$unit_name" "$nowh" "$MAXSPAWN" \
  2>/dev/null)
reserve_rc=$?
set -e
if [ "$reserve_rc" -ne 0 ]; then
  log_line "atomic reservation refused for queue item $item session $slotid"
  exit 0
fi
read -r item_id item_gate c reservation_identity claimed_at <<< "$reservation"

for f in "$STATE/nudge.log" "$STATE/nudge-run.log"; do
  if [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 1048576 ]; then
    "$STATE_HELPER" retain-tail "$f" 262144 2>/dev/null || true
  fi
done

"$STATE_HELPER" write-text "$STATE/last-spawn-ts" "$(date +%s)" 2>/dev/null || exit 2
log_line "spawning agent for queue item $item as unit $unit_name ($((ncl+1))/$cur_conc slots)"
"$STATE_HELPER" unlink "$STATE/idle-notified" 2>/dev/null || true

# Worker owns the API call and reports its exact per-run result back to the
# adaptive controller under the same flock. The explicit unit name makes the
# transient worker enumerable without process-command matching.
if ! "$STATE_HELPER" run-output "$STATE/nudge.log" append \
    "$SYSTEMD_RUN" --user --collect --unit "$unit_name" "$SCRIPT_DIR/nudge-agent.sh" "$item" "$slotid"; then
  log_line "systemd-run failed for unit $unit_name; dropping reservation"
  current_identity=$(stat -c '%d:%i' "$CLAIMS/$item-$slotid.claim" 2>/dev/null || echo missing)
  if [ "$current_identity" = "$reservation_identity" ]; then
    reservation_device=${reservation_identity%%:*}
    reservation_inode=${reservation_identity#*:}
    "$STATE_HELPER" unlink "$CLAIMS/$item-$slotid.claim" "$reservation_device" "$reservation_inode" \
      2>/dev/null || true
  fi
fi

exit 0
