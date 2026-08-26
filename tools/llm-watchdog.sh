#!/usr/bin/env bash
# Ten-minute LLM supervisor, single-model design (operator, 2026-08-21):
# ONE GLM-5.3 session per cycle. It observes health read-only and ends with
# exactly one marker: WATCHDOG_OK (nothing to do - no further model call), or
# WATCHDOG_REPAIR (the wrapper then pauses autonomy, stops workers, and the
# SAME model fixes the smallest concrete cause under a watchdog token).
# Observation failures (transport, timeout, malformed output) never stop
# workers and never trigger a fix. Model (operator constraint): supervisor,
# fix agent, and workers are all zai-coding-plan/glm-5.3#high.
# OpenAI endpoints are OFF-LIMITS until the operator says otherwise.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
STATE="$PLAN_DIR/.state"
QUEUE_PARSER=${QUEUE_PARSER_OVERRIDE:-$SCRIPT_DIR/nudge-free-items.py}
STATE_HELPER="$SCRIPT_DIR/nudge-state.py"
LOCK_HELPER="$SCRIPT_DIR/nudge-lock.py"
WAIT_EXECUTOR="$SCRIPT_DIR/nudge-wait.py"
FAILURES="$STATE/automation-failures"
if [ -n "${OPENC_OVERRIDE:-}" ]; then
  OPENC=$OPENC_OVERRIDE
elif command -v opencode2 >/dev/null 2>&1; then
  OPENC=$(command -v opencode2)
else
  OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/bin/opencode2
fi
SYSTEMCTL=${SYSTEMCTL_OVERRIDE:-systemctl}
REAPER=${REAPER_OVERRIDE:-$PLAN_DIR/tools/nudge-reap-claims.sh}
WD_MODEL=${WD_MODEL:-zai-coding-plan/glm-5.3#high}
NOTIFY_SEND=${NOTIFY_SEND-notify-send}
SUPERVISE_TIMEOUT=${SUPERVISE_TIMEOUT:-480}
REPAIR_TIMEOUT=${REPAIR_TIMEOUT:-1800}
REPAIR_COOLDOWN=${REPAIR_COOLDOWN:-120}
RESUME_WAIT_LOOPS=${RESUME_WAIT_LOOPS:-20}
RESUME_WAIT_SLEEP=${RESUME_WAIT_SLEEP:-1}
TEST_MODE=${WATCHDOG_TEST_MODE:-0}
LOG="$STATE/llm-watchdog.log"
SUPERVISE_OUT="$STATE/llm-watchdog-supervise.log"
REPAIR_OUT="$STATE/llm-watchdog-repair.log"
SNAPSHOT="$STATE/llm-watchdog-snapshot"
VERDICT="$STATE/llm-watchdog-verdict"
PAUSE="$STATE/PAUSE"
MARKER="$STATE/llm-watchdog-pause"
COOLDOWN="$STATE/llm-watchdog-cooldown-until"
PRE_CLAIMS="$STATE/llm-watchdog-preclaims"
LOCK=${LLM_WATCHDOG_LOCK:-/tmp/bedlam-llm-watchdog.lock}
TRIGGER_SNAPSHOT="$STATE/llm-watchdog-failure-snapshot.json"
FAILURE_ACK="$STATE/llm-watchdog-failure-ack.json"
token=""
pause_owned=0
workers_stopped=0
active_model_pid=""
if [ "${LLM_WATCHDOG_LOCK_HELD:-0}" != 1 ]; then
  exec "$LOCK_HELPER" lock-run "$LOCK" nonblocking \
    env LLM_WATCHDOG_LOCK_HELD=1 "$0" "$@"
fi
"$STATE_HELPER" ensure-dir "$STATE" >/dev/null 2>&1 || exit 75
source "$SCRIPT_DIR/nudge-claim.sh"
cd "$PLAN_DIR" || exit 1

# Structured failures and invalid queue state are urgent inputs. Inspect them
# before either supervisor-session throttling mechanism can return early.
failure_summary=$("$STATE_HELPER" list-failures "$FAILURES" 2>/dev/null)
failure_rc=$?
queue_preflight=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$STATE/claims" --state-v1 2>/dev/null)
queue_preflight_rc=$?
urgent_repair=0
verdict_quarantined=0
cooldown_quarantined=0
if [ "$failure_rc" -ne 0 ] || [ -n "$failure_summary" ] \
    || [ "$queue_preflight_rc" -ne 0 ] || [ "$queue_preflight" = INVALID-DEADLOCKED ]; then
  urgent_repair=1
fi

quarantine_throttle() {
  local path=$1 label=$2 identity device inode destination
  identity=$(stat -c '%d:%i' "$path" 2>/dev/null || echo missing)
  case "$identity" in
    *:*)
      device=${identity%%:*}
      inode=${identity#*:}
      destination="$STATE/.$(basename "$path").invalid-$(date +%s%N)-$$"
      "$STATE_HELPER" quarantine "$path" "$destination" "$device" "$inode" \
        >/dev/null 2>&1 || return 1
      "$STATE_HELPER" append-text "$LOG" \
        "$(date -Is) quarantined malformed watchdog $label throttle state"$'\n' \
        2>/dev/null || true
      if [ "$label" = verdict ]; then
        verdict_quarantined=1
      else
        cooldown_quarantined=1
      fi
      ;;
    *) return 1 ;;
  esac
}

# Supervisor-session dedup (NOT a work cooldown - worker retries are
# never delayed): transport storms beacon this service on every failure;
# a cycle starting within MIN_INTERVAL of the last verdict just stands
# down so a storm cannot burn a session per worker failure.
wd_mi="${LLM_WATCHDOG_MIN_INTERVAL:-120}"
case "$wd_mi" in ''|*[!0-9]*) echo "invalid watchdog minimum interval" >&2; exit 75 ;; esac
if [ -e "$VERDICT" ] && ! "$STATE_HELPER" run-output "$LOG" append \
    "$STATE_HELPER" validate-verdict "$VERDICT"; then
  "$STATE_HELPER" append-text "$LOG" "$(date -Is) invalid watchdog verdict timestamp or numeric state"$'\n' 2>/dev/null || true
  if [ "$urgent_repair" -eq 1 ]; then
    quarantine_throttle "$VERDICT" verdict || exit 75
  else
    exit 75
  fi
fi
if [ "$urgent_repair" -eq 0 ] && [ "$wd_mi" -gt 0 ] 2>/dev/null && [ -f "$VERDICT" ]; then
  wd_last=$(stat -c %Y "$VERDICT" 2>/dev/null || echo 0)
  if [ $(( $(date +%s) - wd_last )) -lt "$wd_mi" ]; then
    exit 0
  fi
fi

log() { "$STATE_HELPER" append-text "$LOG" "$(date -Is) $*"$'\n' 2>/dev/null || true; }
read_state() { "$STATE_HELPER" read-text "$1" 2>/dev/null || true; }
remove_state() {
  local path=$1 identity device inode
  identity=$(stat -c '%d:%i' "$path" 2>/dev/null || echo missing)
  case "$identity" in
    *:*)
      device=${identity%%:*}
      inode=${identity#*:}
      "$STATE_HELPER" unlink "$path" "$device" "$inode" 2>/dev/null
      ;;
    *) return 1 ;;
  esac
}
notify() {
  if [ -n "$NOTIFY_SEND" ] && command -v "$NOTIFY_SEND" >/dev/null 2>&1; then
    "$NOTIFY_SEND" -u critical "$1" "$2" 2>/dev/null || true
  fi
}
write_verdict() {
  local cooldown_value=0 payload
  [ "$verdict_quarantined" -eq 0 ] || return 0
  if [ -e "$COOLDOWN" ]; then
    cooldown_value=$("$STATE_HELPER" read-int "$COOLDOWN" watchdog-cooldown 0 9999999999 -) || return 1
  fi
  payload=$(printf 'time=%s\nstate=%s\nrc=%s\nmarkers=%s\ncooldown_until=%s\n' \
    "$(date -Is)" "$1" "$2" "$3" "$cooldown_value")
  "$STATE_HELPER" write-text "$VERDICT" "$payload"
}
release_owned_pause() {
  [ "$pause_owned" -eq 1 ] || return 0
  local pause_value marker_value
  pause_value=$(read_state "$PAUSE")
  marker_value=$(read_state "$MARKER")
  if [ "$pause_value" = "$token" ] && [ "$marker_value" = "$token" ]; then
    remove_state "$PAUSE" || true
    remove_state "$MARKER" || true
  elif [ ! -e "$PAUSE" ] && [ "$marker_value" = "$token" ]; then
    remove_state "$MARKER" || true
  else
    log "pause ownership changed; leaving autonomy paused"
  fi
  pause_owned=0
}
check_worker_proc() {
  if [ -n "${RESUME_PROC_CHECK:-}" ]; then
    "$RESUME_PROC_CHECK" "$1" "$2"
    return $?
  fi
  pgrep -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*--title bedlam-nudge-item$2[[:space:]].*slot $1" >/dev/null 2>&1
}
resume_glm() {
  [ "$workers_stopped" -eq 1 ] || return 0
  workers_stopped=0
  [ -e "$PAUSE" ] && { log "PAUSE present; not resuming GLM"; return 0; }
  [ "$TEST_MODE" = 1 ] && return 0
  sleep 1
  DEAD_CLAIM_TTL=0 RESERVATION_TTL=0 "$REAPER" "$STATE/claims" "$STATE/nudge.log"
  "$STATE_HELPER" touch "$STATE/heartbeat" 0 2>/dev/null || true
  "$SYSTEMCTL" --user start bedlam-nudge.service || true
  local claim item identity worker_id claim_item
  for _ in $(seq 1 "$RESUME_WAIT_LOOPS"); do
    for claim in "$STATE"/claims/*-owner.claim; do
      [ -e "$claim" ] || continue
      claim_read "$claim" || continue
      if flock -n "$claim" true 2>/dev/null; then continue; fi
      item=$(basename "$claim" -owner.claim)
      identity=$(stat -c "%d:%i" "$claim" 2>/dev/null || echo missing)
      grep -q "^$identity " "$PRE_CLAIMS" 2>/dev/null && continue
      worker_id=$CLAIM_SESSION
      claim_item=$CLAIM_ORDINAL
      case "$worker_id" in ""|*[!A-Za-z0-9-]*) continue ;; esac
      [ "$claim_item" = "$item" ] || continue
      if check_worker_proc "$worker_id" "$item"; then
        log "GLM resumed item $item as worker $worker_id with a fresh locked claim"
        return 0
      fi
    done
    sleep "$RESUME_WAIT_SLEEP"
  done
  log "GLM failed to resume with a live claim; next pass will re-evaluate"
  notify "bedlam-re watchdog" "GLM did not resume after repair"
}
cleanup_active_model() {
  [[ "$active_model_pid" =~ ^[1-9][0-9]*$ ]] || return 0
  kill -TERM -- "-$active_model_pid" 2>/dev/null || true
  sleep 0.05
  kill -KILL -- "-$active_model_pid" 2>/dev/null || true
  wait "$active_model_pid" 2>/dev/null || true
  active_model_pid=""
}
on_exit() {
  rc=$?
  cleanup_active_model
  "$STATE_HELPER" unlink "$SNAPSHOT" 2>/dev/null || true
  "$STATE_HELPER" unlink "$TRIGGER_SNAPSHOT" 2>/dev/null || true
  "$STATE_HELPER" unlink "$FAILURE_ACK" 2>/dev/null || true
  release_owned_pause
  resume_glm
  "$STATE_HELPER" unlink "$PRE_CLAIMS" 2>/dev/null || true
  exit "$rc"
}
trap on_exit EXIT INT TERM HUP

rotate() {
  local file=$1
  if [ -f "$file" ] && [ "$(stat -c %s "$file")" -gt 524288 ]; then
    "$STATE_HELPER" retain-tail "$file" 131072 2>/dev/null || true
  fi
}
for file in "$LOG" "$SUPERVISE_OUT" "$REPAIR_OUT"; do rotate "$file"; done

# Holding LOCK proves a matching watchdog token is stale, not live.
if [ -e "$MARKER" ]; then
  stale=$(read_state "$MARKER")
  current=$(read_state "$PAUSE")
  if [[ "$stale" == llm-watchdog\ * ]] && [ "$current" = "$stale" ]; then
    remove_state "$PAUSE" || true
    remove_state "$MARKER" || true
    log "recovered stale watchdog-owned pause"
  elif [ ! -e "$PAUSE" ]; then
    remove_state "$MARKER" || true
    log "removed orphan watchdog pause marker"
  fi
fi

# Under the singleton lock any leftover watchdog-format PAUSE is provably
# stale (no other watchdog instance can be live). Covers crash windows
# where the marker was lost or mismatched - e.g. PAUSE written but the
# process died before the marker, or files from different generations.
if [ -e "$PAUSE" ]; then
  cur=$(read_state "$PAUSE")
  mk=$(read_state "$MARKER")
  if [[ "$cur" == llm-watchdog\ * ]] && { [ ! -e "$MARKER" ] || [ "$mk" != "$cur" ]; }; then
    remove_state "$PAUSE" || true
    remove_state "$MARKER" || true
    log "recovered stranded watchdog-owned pause (orphan token)"
  fi
fi

# A human pause is authoritative and is never bypassed.
if [ -e "$PAUSE" ]; then
  log "human PAUSE present; watchdog standing down"
  exit 0
fi

now=$(date +%s)
if [ -e "$COOLDOWN" ]; then
  if until=$("$STATE_HELPER" read-int "$COOLDOWN" watchdog-cooldown 0 $((now + 86400)) - 2>&1); then
    :
  elif [ "$urgent_repair" -eq 1 ]; then
    log "$until"
    log "invalid watchdog cooldown timestamp or numeric state"
    quarantine_throttle "$COOLDOWN" cooldown || exit 75
    until=0
  else
    log "$until"
    log "invalid watchdog cooldown timestamp or numeric state"
    exit 75
  fi
else
  until=0
fi
if [ "$now" -lt "$until" ] && [ "$urgent_repair" -eq 0 ]; then
  log "repair ($WD_MODEL) cooldown active until $until; cycle skipped"
  write_verdict repair-deferred 0 0
  exit 0
fi
"$STATE_HELPER" run-output "$LOG" append "$STATE_HELPER" unlink "$COOLDOWN" || true

force_repair=0
failure_trigger=0
if [ "$queue_preflight_rc" -ne 0 ] || [ "$queue_preflight" = INVALID-DEADLOCKED ]; then
  force_repair=1
  log "queue INVALID-DEADLOCKED in strict queue preflight (rc=$queue_preflight_rc); forcing repair"
fi
if [ "$failure_rc" -ne 0 ] || [ -n "$failure_summary" ]; then
  force_repair=1
  failure_trigger=1
  log "structured automation failure requires repair"
  "$STATE_HELPER" run-output "$LOG" append \
    "$STATE_HELPER" snapshot-failures "$FAILURES" "$TRIGGER_SNAPSHOT" || true
  remove_state "$FAILURE_ACK" || true
fi

snapshot_payload=$({
  echo "time=$(date -Is)"
  echo "head=$(git rev-parse HEAD 2>/dev/null || echo none)"
  echo "last_commit=$(git log -1 --format=\"%H %cI %s\" 2>/dev/null || echo none)"
  echo "fails=$(read_state "$STATE/fails")"
  echo status_begin
  git status --short --branch 2>/dev/null || true
  echo status_end
  echo claims_begin
  for claim in "$STATE"/claims/*.claim; do
    [ -e "$claim" ] || continue
    printf "%s age=%s locked=" "$(basename "$claim")" "$(( $(date +%s) - $(stat -c %Y "$claim") ))"
    if flock -n "$claim" true 2>/dev/null; then echo no; else echo yes; fi
    sed -n "1,4p" "$claim" 2>/dev/null || true
  done
  echo claims_end
  echo automation_failures_begin
  printf '%s\n' "$failure_summary"
  echo automation_failures_end
  echo workers_begin
  pgrep -af "^timeout 3900 [^ ]*opencode2 run.*bedlam-nudge-item|^[^ ]*opencode2 run.*bedlam-nudge-item" || true
  echo workers_end
  echo recent_agent_logs_begin
  find "$STATE" -maxdepth 1 -name "agent-*.log" -printf "%T@ %p\n" 2>/dev/null | sort -nr | head -3
  echo recent_agent_logs_end
  echo queue_begin
  sed -n "1,45p" "$STATE/NEXT.md" 2>/dev/null || true
  echo queue_end
  echo controller_tail_begin
  tail -40 "$STATE/nudge.log" 2>/dev/null || true
  echo controller_tail_end
})
"$STATE_HELPER" write-text "$SNAPSHOT" "$snapshot_payload" || exit 75

prev_state=$(read_state "$VERDICT" | sed -n "s/^state=//p" | tail -n 1)
if [ "$force_repair" -eq 1 ]; then
  check_rc=$queue_preflight_rc
  marker_count=1
  "$STATE_HELPER" write-text "$SUPERVISE_OUT" "" || exit 75
else
  SUPERVISE_PROMPT="Inspect the autonomous Bedlam remaster loop in $PLAN_DIR as a read-only health supervisor. Do not modify files, stop processes, create commits, launch agents, or spawn subagents. Read AGENTS.md, $SNAPSHOT, current worker logs, relevant claim/controller scripts, and git history/status. Judge whether GLM-5.3 is advancing or stuck, churning, mis-owning work, repeatedly exhausting its step budget, or failing to launch. Relevant dirty WIP alone is not a fault. End with exactly one marker on its own final non-empty line: WATCHDOG_OK when no intervention is needed, or WATCHDOG_REPAIR when an automated repair is needed now. Before the marker, give concise evidence and the exact repair objective."
  "$STATE_HELPER" write-text "$SUPERVISE_OUT" "" || exit 75
  set +e
  setsid "$STATE_HELPER" exec-output "$SUPERVISE_OUT" append timeout "$SUPERVISE_TIMEOUT" "$OPENC" run --standalone --model "$WD_MODEL" --title bedlam-llm-watchdog-supervise "$SUPERVISE_PROMPT" &
  active_model_pid=$!
  wait "$active_model_pid"
  check_rc=$?
  cleanup_active_model
  set -e
  normalized=$(perl -pe "s/\e\[[0-?]*[ -\/]*[@-~]//g; s/\r//g" "$SUPERVISE_OUT")
  final_marker=$(printf "%s\n" "$normalized" | awk "NF { last=\$0 } END { print last }")
  marker_count=$(printf "%s\n" "$normalized" | grep -Ec "^(WATCHDOG_OK|WATCHDOG_REPAIR)$" || true)

  if [ "$check_rc" -eq 0 ] && [ "$final_marker" = WATCHDOG_OK ] && [ "$marker_count" -eq 1 ]; then
    log "supervisor ($WD_MODEL) reports healthy"
    write_verdict healthy "$check_rc" "$marker_count"
    exit 0
  fi
  if [ "$check_rc" -eq 0 ] && [ "$final_marker" = WATCHDOG_REPAIR ] && [ "$marker_count" -eq 1 ]; then
    log "supervisor ($WD_MODEL) requested repair"
  else
    log "supervisor ($WD_MODEL) observation failed rc=$check_rc final=$final_marker markers=$marker_count; no valid WATCHDOG_REPAIR marker - not escalating"
    write_verdict unknown "$check_rc" "$marker_count"
    if [ "$prev_state" != "unknown" ]; then
      notify "bedlam-re watchdog" "supervisor ($WD_MODEL) cannot observe (rc=$check_rc); workers left running"
    fi
    exit 0
  fi
fi

# Publish the marker first, then acquire PAUSE with O_EXCL. If a human wins the
# race, exclusive creation fails and their pause remains byte-for-byte intact.
token="llm-watchdog $$ $(date +%s)"
pause_owned=1
"$STATE_HELPER" write-text "$MARKER" "$token"$'\n' || exit 75
if ! "$STATE_HELPER" create-text "$PAUSE" "$token"$'\n' 2>/dev/null; then
  pause_owned=0
  [ "$(read_state "$MARKER")" = "$token" ] && remove_state "$MARKER" || true
  log "PAUSE appeared during atomic acquisition; respecting operator"
  exit 0
fi

stop_glm_workers() {
  workers_stopped=1
  local unit props pass stopped_any
  for pass in 1 2; do
    stopped_any=0
    while read -r unit; do
      [ -n "$unit" ] || continue
      props=$("$SYSTEMCTL" --user show -p ExecStart --value "$unit" 2>/dev/null || true)
      case "$props" in
        *"$PLAN_DIR/tools/nudge-agent.sh"*)
          "$SYSTEMCTL" --user stop "$unit" || true
          stopped_any=1
          ;;
      esac
    done < <("$SYSTEMCTL" --user list-units --type=service --state=running --no-legend --plain 2>/dev/null | awk "{print \$1}")
    [ "$stopped_any" -eq 0 ] && break
    sleep 1
  done
  if [ "$TEST_MODE" = 1 ]; then return 0; fi
  sleep 2
  # Terminate only the exact unattended GLM command family if it lacked a unit.
  if pgrep -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" >/dev/null 2>&1; then
    pkill -TERM -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" || true
    pkill -TERM -f "^timeout 3900 [^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" || true
    sleep 2
  fi
  for _ in $(seq 1 5); do
    pgrep -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" >/dev/null 2>&1 || return 0
    sleep 1
  done
  pkill -KILL -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" || true
  pkill -KILL -f "^timeout 3900 [^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" || true
  sleep 1
  if pgrep -f "^[^ ]*opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" >/dev/null 2>&1; then
    log "unable to stop all GLM workers; aborting repair"
    return 1
  fi
  local old_identity old_name old_claim current_identity
  while read -r old_identity old_name; do
    [ -n "$old_name" ] || continue
    old_claim="$STATE/claims/$old_name"
    [ -e "$old_claim" ] || continue
    current_identity=$(stat -c "%d:%i" "$old_claim" 2>/dev/null || echo missing)
    if [ "$current_identity" = "$old_identity" ] && ! flock -n "$old_claim" true 2>/dev/null; then
      log "pre-repair claim $old_name remains locked by a descendant; aborting repair"
      return 1
    fi
  done < "$PRE_CLAIMS"
  return 0
}
"$STATE_HELPER" write-text "$PRE_CLAIMS" "" || exit 75
for claim in "$STATE"/claims/*-owner.claim; do
  [ -e "$claim" ] || continue
  "$STATE_HELPER" append-text "$PRE_CLAIMS" "$(stat -c "%d:%i" "$claim" 2>/dev/null || echo missing) $(basename "$claim")"$'\n' || exit 75
done
if ! stop_glm_workers; then exit 1; fi

start_head=$(git rev-parse HEAD 2>/dev/null || echo none)
REPAIR_PROMPT="Repair the autonomous Bedlam remaster loop in $PLAN_DIR. The watchdog token is: $token. Before working, verify both .state/PAUSE and .state/llm-watchdog-pause contain exactly that token; otherwise stop. Read AGENTS.md, the diagnosis in $SUPERVISE_OUT, $SNAPSHOT, git status/diff, queue, controller, worker logs, and strict parser diagnostics. Fix the smallest concrete cause of stalled or churning automation. Preserve interrupted WIP; never reset, clean, or overwrite it; bracket game-data reads; stage explicit paths only. Keep required queue work active; if completion depends on a machine event, use only the strict bounded WAITING-AUTOMATIC grammar. For each structured failure that is actually resolved, write .state/llm-watchdog-failure-ack.json with schema nudge-failure-ack-v1 and records bound to the snapshot name/device/inode/sha256/ordinal/id/gate plus remediation_commit equal to the exact commit that mechanically established the declared postcondition; resolution must be required-empty or replaced-task and must match the resulting queue. Never acknowledge an unrelated commit. Run focused tests. Every repair commit must include the exact trailer Watchdog-Repair: $token. Commit and push substantive fixes when green. Do not spawn subagents. Leave both pause files untouched for the wrapper. End with diagnosis, changes, tests, commit, and exact resume state."
"$STATE_HELPER" write-text "$REPAIR_OUT" "" || exit 75
set +e
setsid "$STATE_HELPER" exec-output "$REPAIR_OUT" append timeout "$REPAIR_TIMEOUT" "$OPENC" run --standalone --agent build --model "$WD_MODEL" --auto --title bedlam-llm-watchdog-repair "$REPAIR_PROMPT" &
active_model_pid=$!
wait "$active_model_pid"
repair_rc=$?
cleanup_active_model
set -e
end_head=$(git rev-parse HEAD 2>/dev/null || echo none)
repair_evidence=0
if [ "$end_head" != "$start_head" ] && git cat-file -e "$start_head^{commit}" 2>/dev/null \
    && git log -1 --format=%B "$end_head" | grep -qx "Watchdog-Repair: $token" \
    && git diff-tree --no-commit-id --name-only -r "$end_head" | grep -qv "^\.state/"; then
  repair_evidence=1
fi
set +e
queue_after=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$STATE/claims" --state-v1 2>&1)
queue_after_rc=$?
set -e
[ "$queue_after_rc" -eq 0 ] || log "$queue_after"
wait_evidence=0
if [ "$queue_after_rc" -eq 0 ] && [ "$queue_after" = AUTOMATIC-WAIT ] \
    && "$STATE_HELPER" run-output "$LOG" append /usr/bin/env \
      NUDGE_WAIT_ALLOW_PAUSE_TOKEN="$token" "$WAIT_EXECUTOR" verify \
      "$STATE/NEXT.md" "$STATE/automatic-waits"; then
  wait_evidence=1
fi
verified_repair=0
if [ "$queue_after_rc" -eq 0 ] && { [ "$repair_evidence" -eq 1 ] || [ "$wait_evidence" -eq 1 ]; }; then
  verified_repair=1
fi
if [ "$verified_repair" -eq 1 ] && [ "$failure_trigger" -eq 1 ]; then
  if [ ! -f "$FAILURE_ACK" ] || ! "$STATE_HELPER" run-output "$LOG" append \
      "$STATE_HELPER" archive-failures "$FAILURES" "$TRIGGER_SNAPSHOT" \
      "$STATE/NEXT.md" "$FAILURE_ACK" "$end_head"; then
    verified_repair=0
    log "verified repair could not archive automation failure trigger"
  fi
fi
if [ "$verified_repair" -eq 1 ]; then
  log "repair ($WD_MODEL) produced evidence rc=$repair_rc head=$start_head..$end_head"
  write_verdict repaired "$repair_rc" "1"
else
  log "repair ($WD_MODEL) produced no evidence rc=$repair_rc; cooling escalation while GLM resumes"
  if [ "$cooldown_quarantined" -eq 0 ]; then
    "$STATE_HELPER" run-output "$LOG" append "$STATE_HELPER" write-text \
      "$COOLDOWN" "$(( $(date +%s) + REPAIR_COOLDOWN ))" || true
  fi
  notify "bedlam-re watchdog repair failed" "repair agent produced no valid automated repair evidence; GLM will resume"
  write_verdict repair-no-evidence "$repair_rc" "0"
fi
release_owned_pause
resume_glm
exit 0
