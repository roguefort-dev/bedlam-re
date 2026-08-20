#!/usr/bin/env bash
# Ten-minute LLM supervisor: check observes health; the repair agent runs only
# when check emits an explicit WATCHDOG_REPAIR marker. Observation failures
# (transport, timeout, malformed output) never stop workers and never repair.
# Models (operator constraint, 2026-08-20): check = opencode/deepseek-v4-flash-free#max
# (FREE tier); repair agent + workers = zai-coding-plan/glm-5.3#high.
# OpenAI endpoints are OFF-LIMITS until the operator says otherwise.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
if [ -n "${OPENC_OVERRIDE:-}" ]; then
  OPENC=$OPENC_OVERRIDE
elif command -v opencode2 >/dev/null 2>&1; then
  OPENC=$(command -v opencode2)
else
  OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/bin/opencode2
fi
SYSTEMCTL=${SYSTEMCTL_OVERRIDE:-systemctl}
REAPER=${REAPER_OVERRIDE:-$PLAN_DIR/tools/nudge-reap-claims.sh}
CHECK_MODEL=${CHECK_MODEL:-opencode/deepseek-v4-flash-free#max}
REPAIR_MODEL=${REPAIR_MODEL:-zai-coding-plan/glm-5.3#high}
NOTIFY_SEND=${NOTIFY_SEND-notify-send}
CHECK_TIMEOUT=${CHECK_TIMEOUT:-480}
REPAIR_TIMEOUT=${REPAIR_TIMEOUT:-1800}
REPAIR_COOLDOWN=${REPAIR_COOLDOWN:-1800}
RESUME_WAIT_LOOPS=${RESUME_WAIT_LOOPS:-20}
RESUME_WAIT_SLEEP=${RESUME_WAIT_SLEEP:-1}
TEST_MODE=${WATCHDOG_TEST_MODE:-0}
LOG="$STATE/llm-watchdog.log"
CHECK_OUT="$STATE/llm-watchdog-check.log"
REPAIR_OUT="$STATE/llm-watchdog-repair.log"
SNAPSHOT="$STATE/llm-watchdog-snapshot"
VERDICT="$STATE/llm-watchdog-verdict"
PAUSE="$STATE/PAUSE"
MARKER="$STATE/llm-watchdog-pause"
COOLDOWN="$STATE/llm-watchdog-cooldown-until"
PRE_CLAIMS="$STATE/llm-watchdog-preclaims"
BLOCKED_BEFORE="$STATE/llm-watchdog-blocked-before"
LOCK=${LLM_WATCHDOG_LOCK:-/tmp/bedlam-llm-watchdog.lock}
token=""
pause_owned=0
workers_stopped=0
mkdir -p "$STATE"
exec 9>"$LOCK"
flock -n 9 || exit 0
cd "$PLAN_DIR" || exit 1

log() { echo "$(date -Is) $*" >> "$LOG"; }
notify() {
  if [ -n "$NOTIFY_SEND" ] && command -v "$NOTIFY_SEND" >/dev/null 2>&1; then
    "$NOTIFY_SEND" -u critical "$1" "$2" 2>/dev/null || true
  fi
}
write_verdict() {
  {
    echo "time=$(date -Is)"
    echo "state=$1"
    echo "rc=$2"
    echo "markers=$3"
    echo "cooldown_until=$(cat "$COOLDOWN" 2>/dev/null || echo 0)"
  } > "$VERDICT"
}
release_owned_pause() {
  [ "$pause_owned" -eq 1 ] || return 0
  if [ "$(cat "$PAUSE" 2>/dev/null || true)" = "$token" ] && [ "$(cat "$MARKER" 2>/dev/null || true)" = "$token" ]; then
    rm -f "$PAUSE" "$MARKER"
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
  sleep 1
  DEAD_CLAIM_TTL=0 RESERVATION_TTL=0 "$REAPER" "$STATE/claims" "$STATE/nudge.log"
  touch -d @0 "$STATE/heartbeat"
  "$SYSTEMCTL" --user start bedlam-nudge.service || true
  local claim item identity worker_line worker_id claim_item
  for _ in $(seq 1 "$RESUME_WAIT_LOOPS"); do
    for claim in "$STATE"/claims/*-owner.claim; do
      [ -e "$claim" ] || continue
      grep -q "^lock-v1 worker .* owns queue item " "$claim" 2>/dev/null || continue
      if flock -n "$claim" true 2>/dev/null; then continue; fi
      item=$(basename "$claim" -owner.claim)
      identity=$(stat -c "%d:%i" "$claim" 2>/dev/null || echo missing)
      grep -q "^$identity " "$PRE_CLAIMS" 2>/dev/null && continue
      worker_line=$(grep "^lock-v1 worker " "$claim" 2>/dev/null | tail -n 1)
      worker_id=$(printf "%s\n" "$worker_line" | awk "{print \$3}")
      claim_item=$(printf "%s\n" "$worker_line" | awk "{print \$7}")
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
on_exit() {
  rc=$?
  rm -f "$SNAPSHOT" "$BLOCKED_BEFORE"
  release_owned_pause
  resume_glm
  rm -f "$PRE_CLAIMS"
  exit "$rc"
}
trap on_exit EXIT INT TERM HUP

rotate() {
  local file=$1
  if [ -f "$file" ] && [ "$(stat -c %s "$file")" -gt 524288 ]; then
    tail -c 131072 "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  fi
}
for file in "$LOG" "$CHECK_OUT" "$REPAIR_OUT"; do rotate "$file"; done

# Holding LOCK proves a matching watchdog token is stale, not live.
if [ -e "$MARKER" ]; then
  stale=$(cat "$MARKER" 2>/dev/null || true)
  current=$(cat "$PAUSE" 2>/dev/null || true)
  if [[ "$stale" == llm-watchdog\ * ]] && [ "$current" = "$stale" ]; then
    rm -f "$PAUSE" "$MARKER"
    log "recovered stale watchdog-owned pause"
  elif [ ! -e "$PAUSE" ]; then
    rm -f "$MARKER"
    log "removed orphan watchdog pause marker"
  fi
fi

# A human pause is authoritative and is never bypassed.
if [ -e "$PAUSE" ]; then
  log "human PAUSE present; watchdog standing down"
  exit 0
fi

now=$(date +%s)
until=$(cat "$COOLDOWN" 2>/dev/null || echo 0)
in_cooldown=0
if [ "$now" -lt "$until" ]; then
  in_cooldown=1
  log "repair (glm-5.3) cooldown active until $until; check (deepseek-v4-flash-free) still observing"
else
  rm -f "$COOLDOWN"
fi

{
  echo "time=$(date -Is)"
  echo "head=$(git rev-parse HEAD 2>/dev/null || echo none)"
  echo "last_commit=$(git log -1 --format=\"%H %cI %s\" 2>/dev/null || echo none)"
  echo "fails=$(cat "$STATE/fails" 2>/dev/null || echo 0)"
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
} > "$SNAPSHOT"

CHECK_PROMPT="You are the read-only health supervisor for the autonomous Bedlam remaster loop. Work in $PLAN_DIR. Do not modify files, kill processes, create commits, or launch agents. Read AGENTS.md, $SNAPSHOT, current worker logs, claim/controller scripts when relevant, and git history/status. Judge whether GLM-5.3 is advancing or is stuck, churning, or mis-owning work. A persistent Ghostty, cmux, or operator TUI is never ownership and never a fault. Dirty relevant WIP alone is not a fault. A live locked worker with recent meaningful investigation may be healthy before its first commit; distinguish that from repetitive reading, no-op stand-downs, dead claims, launch failures, stale logs, or controller defects. A spree of rc=0 no-progress failures whose agent logs end in 'Maximum steps for this agent reached' is a step-budget/controller defect (the worker agent is misconfigured for the unit size), NOT healthy worker churn - flag it for repair; nudge-agent.sh classifies such runs as 'step-cap'. End with exactly one marker on its own final non-empty line: WATCHDOG_OK if no intervention is needed, or WATCHDOG_REPAIR if the repair agent must intervene now. Before the marker, give concise evidence and the exact repair objective."
: > "$CHECK_OUT"
set +e
timeout "$CHECK_TIMEOUT" "$OPENC" run --standalone --model "$CHECK_MODEL" --title bedlam-llm-watchdog-check "$CHECK_PROMPT" >> "$CHECK_OUT" 2>&1
check_rc=$?
set -e
normalized=$(perl -pe "s/\e\[[0-?]*[ -\/]*[@-~]//g; s/\r//g" "$CHECK_OUT")
final_marker=$(printf "%s\n" "$normalized" | awk "NF { last=\$0 } END { print last }")
marker_count=$(printf "%s\n" "$normalized" | grep -Ec "^(WATCHDOG_OK|WATCHDOG_REPAIR)$" || true)
prev_state=$(sed -n "s/^state=//p" "$VERDICT" 2>/dev/null | tail -n 1)

if [ "$check_rc" -eq 0 ] && [ "$final_marker" = WATCHDOG_OK ] && [ "$marker_count" -eq 1 ]; then
  log "check (deepseek-v4-flash-free) reports healthy"
  write_verdict healthy "$check_rc" "$marker_count"
  exit 0
fi
if [ "$check_rc" -eq 0 ] && [ "$final_marker" = WATCHDOG_REPAIR ] && [ "$marker_count" -eq 1 ]; then
  if [ "$in_cooldown" -eq 1 ]; then
    log "check (deepseek-v4-flash-free) requested repair but repair cooldown is active; deferring, GLM keeps running"
    write_verdict repair-deferred "$check_rc" "$marker_count"
    exit 0
  fi
  log "check (deepseek-v4-flash-free) requested repair"
else
  log "check (deepseek-v4-flash-free) observation failed rc=$check_rc final=$final_marker markers=$marker_count; no valid WATCHDOG_REPAIR marker - not escalating"
  write_verdict unknown "$check_rc" "$marker_count"
  if [ "$prev_state" != "unknown" ]; then
    notify "bedlam-re watchdog" "check (deepseek-v4-flash-free) cannot observe (rc=$check_rc); workers left running"
  fi
  exit 0
fi

# Publish the marker first, then acquire PAUSE with O_EXCL. If a human wins the
# race, exclusive creation fails and their pause remains byte-for-byte intact.
token="llm-watchdog $$ $(date +%s)"
printf "%s\n" "$token" > "$MARKER"
if ! python3 -c "import os,sys; fd=os.open(sys.argv[1],os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o644); os.write(fd,(sys.argv[2]+chr(10)).encode()); os.close(fd)" "$PAUSE" "$token" 2>/dev/null; then
  [ "$(cat "$MARKER" 2>/dev/null || true)" = "$token" ] && rm -f "$MARKER"
  log "PAUSE appeared during atomic acquisition; respecting operator"
  exit 0
fi
pause_owned=1

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
: > "$PRE_CLAIMS"
for claim in "$STATE"/claims/*-owner.claim; do
  [ -e "$claim" ] || continue
  printf "%s %s\n" "$(stat -c "%d:%i" "$claim" 2>/dev/null || echo missing)" "$(basename "$claim")" >> "$PRE_CLAIMS"
done
if ! stop_glm_workers; then exit 1; fi

start_head=$(git rev-parse HEAD 2>/dev/null || echo none)
grep -E "^[[:space:]]*[0-9]+\.[[:space:]]+(\[[^]]+\][[:space:]]*)*\[BLOCKED\]" "$STATE/NEXT.md" 2>/dev/null | sort -u > "$BLOCKED_BEFORE" || true
REPAIR_PROMPT="You are the repair agent for the autonomous Bedlam remaster loop in $PLAN_DIR. The watchdog token is: $token. This is the narrow AGENTS.md PAUSE exception: before working, verify both .state/PAUSE and .state/llm-watchdog-pause contain exactly that token. If they do, proceed despite PAUSE; otherwise stop. The watchdog stopped GLM workers. Read AGENTS.md, $CHECK_OUT, $SNAPSHOT, git status/diff, queue, controller and worker logs. Diagnose reality rather than trusting the check report blindly. Fix the smallest concrete cause of stalled or churning autonomy. Use your direct file and shell tools, including shell or Python edits if needed; do not ask to delegate. Preserve interrupted WIP; never reset, clean, or overwrite it; bracket game-data reads; stage explicit paths only. Run focused tests. Every repair commit MUST include the exact trailer Watchdog-Repair: $token. Commit and push substantive fixes when green, or rewrite the claimed queue item once as [BLOCKED] with a concrete blocker. Do not spawn subagents. Leave both pause files untouched for the wrapper. End with diagnosis, changes, tests, commit, and exact GLM resume state."
: > "$REPAIR_OUT"
set +e
timeout "$REPAIR_TIMEOUT" "$OPENC" run --standalone --agent build --model "$REPAIR_MODEL" --auto --title bedlam-llm-watchdog-repair "$REPAIR_PROMPT" >> "$REPAIR_OUT" 2>&1
repair_rc=$?
set -e
end_head=$(git rev-parse HEAD 2>/dev/null || echo none)
repair_evidence=0
if [ "$end_head" != "$start_head" ] && git cat-file -e "$start_head^{commit}" 2>/dev/null; then
  for commit in $(git rev-list "$start_head..$end_head" 2>/dev/null); do
    if git log -1 --format=%B "$commit" | grep -qx "Watchdog-Repair: $token" \
        && git diff-tree --no-commit-id --name-only -r "$commit" | grep -qv "^\.state/"; then
      repair_evidence=1
      break
    fi
  done
fi
if [ "$repair_evidence" -eq 0 ]; then
  new_blocked=$(comm -13 "$BLOCKED_BEFORE" <(grep -E "^[[:space:]]*[0-9]+\.[[:space:]]+(\[[^]]+\][[:space:]]*)*\[BLOCKED\]" "$STATE/NEXT.md" 2>/dev/null | sort -u) || true)
  [ -n "$new_blocked" ] && repair_evidence=1
fi
if [ "$repair_evidence" -eq 1 ]; then
  log "repair (glm-5.3) produced evidence rc=$repair_rc head=$start_head..$end_head"
  write_verdict repaired "$repair_rc" "1"
else
  log "repair (glm-5.3) produced no evidence rc=$repair_rc; cooling escalation while GLM resumes"
  echo $(( $(date +%s) + REPAIR_COOLDOWN )) > "$COOLDOWN"
  notify "bedlam-re watchdog repair failed" "repair agent produced no commit or BLOCKED handoff; GLM will resume"
  write_verdict repair-no-evidence "$repair_rc" "0"
fi
release_owned_pause
resume_glm
exit 0
