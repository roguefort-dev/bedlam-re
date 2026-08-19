#!/usr/bin/env bash
# Ten-minute LLM supervisor: Luna judges health; Sol repairs only on escalation.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
OPENC=${OPENC_OVERRIDE:-opencode2}
SYSTEMCTL=${SYSTEMCTL_OVERRIDE:-systemctl}
REAPER=${REAPER_OVERRIDE:-$PLAN_DIR/tools/nudge-reap-claims.sh}
LUNA_MODEL=${LUNA_MODEL:-openai/gpt-5.6-luna#max}
SOL_MODEL=${SOL_MODEL:-openai/gpt-5.6-sol#high}
CHECK_TIMEOUT=${CHECK_TIMEOUT:-480}
REPAIR_TIMEOUT=${REPAIR_TIMEOUT:-1800}
TEST_MODE=${WATCHDOG_TEST_MODE:-0}
LOG="$STATE/llm-watchdog.log"
LUNA_OUT="$STATE/llm-watchdog-luna.log"
SOL_OUT="$STATE/llm-watchdog-sol.log"
LOCK=${LLM_WATCHDOG_LOCK:-/tmp/bedlam-llm-watchdog.lock}
mkdir -p "$STATE"
exec 9>"$LOCK"
flock -n 9 || exit 0
cd "$PLAN_DIR" || exit 1

rotate() {
  local file=$1
  if [ -f "$file" ] && [ "$(stat -c %s "$file")" -gt 524288 ]; then
    tail -c 131072 "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  fi
}
for file in "$LOG" "$LUNA_OUT" "$SOL_OUT"; do rotate "$file"; done

# A human pause is authoritative. The watchdog must never remove or bypass it.
if [ -e "$STATE/PAUSE" ] && [ ! -e "$STATE/llm-watchdog-pause" ]; then
  echo "$(date -Is) human PAUSE present; watchdog standing down" >> "$LOG"
  exit 0
fi

snapshot=$(mktemp /tmp/bedlam-llm-health.XXXXXX)
cleanup() { rm -f "$snapshot"; }
trap cleanup EXIT
{
  echo "time=$(date -Is)"
  echo "head=$(git rev-parse HEAD 2>/dev/null || echo none)"
  echo "last_commit=$(git log -1 --format=\"%H %cI %s\" 2>/dev/null || echo none)"
  echo status_begin
  git status --short --branch 2>/dev/null || true
  echo status_end
  echo claims_begin
  for claim in "$STATE"/claims/*.claim; do
    [ -e "$claim" ] || continue
    printf "%s age=%s locked=" "$(basename "$claim")" "$(( $(date +%s) - $(stat -c %Y "$claim") ))"
    if flock -n "$claim" true 2>/dev/null; then echo no; else echo yes; fi
    sed -n "1p" "$claim" 2>/dev/null || true
  done
  echo claims_end
  echo workers_begin
  pgrep -af "^timeout 3900 opencode2 run.*bedlam-nudge-item|^opencode2 run.*bedlam-nudge-item" || true
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
} > "$snapshot"

LUNA_PROMPT="You are the read-only health supervisor for the autonomous Bedlam remaster loop. Work in $PLAN_DIR. Do not modify files, kill processes, create commits, or launch agents. Read AGENTS.md, the health snapshot at $snapshot, current worker logs, claim/controller scripts when relevant, and git history/status. Judge whether GLM-5.3 is actually advancing or is stuck, churning, or mis-owning work. A persistent Ghostty, cmux, or operator TUI is never ownership and never a fault. Dirty relevant WIP alone is not a fault. A live locked worker with recent meaningful investigation may be healthy before its first commit; distinguish that from repetitive reading, no-op stand-downs, dead claims, launch failures, stale logs, or controller defects. A human .state/PAUSE is authoritative. End with exactly one marker on its own final line: WATCHDOG_OK if no intervention is needed, or WATCHDOG_REPAIR if Sol must intervene now. Before the marker, give concise evidence and the exact repair objective."
: > "$LUNA_OUT"
set +e
timeout "$CHECK_TIMEOUT" "$OPENC" run --standalone --model "$LUNA_MODEL" --title bedlam-llm-watchdog-check "$LUNA_PROMPT" >> "$LUNA_OUT" 2>&1
luna_rc=$?
set -e
if [ "$luna_rc" -eq 0 ] && tail -20 "$LUNA_OUT" | grep -qx WATCHDOG_OK; then
  echo "$(date -Is) Luna reports healthy" >> "$LOG"
  exit 0
fi
if [ "$luna_rc" -eq 0 ] && ! tail -20 "$LUNA_OUT" | grep -qx WATCHDOG_REPAIR; then
  echo "$(date -Is) Luna returned no valid marker; escalating to Sol" >> "$LOG"
elif [ "$luna_rc" -ne 0 ]; then
  echo "$(date -Is) Luna failed rc=$luna_rc; escalating to Sol" >> "$LOG"
else
  echo "$(date -Is) Luna requested repair" >> "$LOG"
fi

# Claim an explicit pause without taking ownership of a pre-existing pause.
token="llm-watchdog $$ $(date +%s)"
if [ -e "$STATE/PAUSE" ]; then
  echo "$(date -Is) PAUSE appeared before repair; respecting operator and aborting" >> "$LOG"
  exit 0
fi
printf "%s\n" "$token" > "$STATE/PAUSE"
printf "%s\n" "$token" > "$STATE/llm-watchdog-pause"

stop_glm_workers() {
  [ "$TEST_MODE" = 1 ] && return 0
  local unit props
  while read -r unit; do
    [ -n "$unit" ] || continue
    props=$($SYSTEMCTL --user show -p ExecStart --value "$unit" 2>/dev/null || true)
    case "$props" in
      *bedlam-re/tools/nudge-agent.sh*) $SYSTEMCTL --user stop "$unit" || true ;;
    esac
  done < <($SYSTEMCTL --user list-units "run-p*.service" --state=running --no-legend --plain 2>/dev/null | awk "{print \$1}")
}
stop_glm_workers

SOL_PROMPT="You are the high-reasoning repair agent for the autonomous Bedlam remaster loop in $PLAN_DIR. Luna supervision escalated because the loop is unhealthy or Luna itself failed. The watchdog created .state/PAUSE and stopped autonomous GLM worker units, so preserve that pause and do not restart workers yourself. Read AGENTS.md, $LUNA_OUT, $snapshot, current git status/diff, queue, controller logs, and relevant worker logs. Diagnose reality rather than trusting Luna blindly. Fix the smallest concrete cause of stalled or churning autonomy. Preserve and adopt relevant interrupted WIP; never reset, clean, or overwrite it; never touch game-data without manifest bracketing; stage explicit paths only. You may repair controller, contracts, tests, or finish or salvage a blocking implementation seam when needed. Run focused tests. Commit and push substantive fixes when green; do not create state-only stand-down commits. Do not spawn subagents. Leave .state/PAUSE and .state/llm-watchdog-pause untouched for the wrapper to release safely. End with a concise account of diagnosis, changes, tests, commit, and exact queue state GLM should resume from."
: > "$SOL_OUT"
set +e
timeout "$REPAIR_TIMEOUT" "$OPENC" run --standalone --model "$SOL_MODEL" --auto --title bedlam-llm-watchdog-repair "$SOL_PROMPT" >> "$SOL_OUT" 2>&1
sol_rc=$?
set -e
echo "$(date -Is) Sol repair ended rc=$sol_rc" >> "$LOG"

# Release only the exact pause owned by this invocation.
if [ "$(cat "$STATE/PAUSE" 2>/dev/null || true)" = "$token" ] && [ "$(cat "$STATE/llm-watchdog-pause" 2>/dev/null || true)" = "$token" ]; then
  rm -f "$STATE/PAUSE" "$STATE/llm-watchdog-pause"
else
  echo "$(date -Is) pause ownership changed; leaving autonomy paused" >> "$LOG"
  exit "$sol_rc"
fi

sleep 1
DEAD_CLAIM_TTL=0 RESERVATION_TTL=0 "$REAPER" "$STATE/claims" "$STATE/nudge.log"
if [ "$TEST_MODE" != 1 ]; then
  touch -d @0 "$STATE/heartbeat"
  $SYSTEMCTL --user start bedlam-nudge.service || true
  sleep 5
  if pgrep -f "^opencode2 run.*zai-coding-plan/glm-5.3.*bedlam-nudge-item" >/dev/null 2>&1; then
    echo "$(date -Is) GLM-5.3 resumed after Sol repair" >> "$LOG"
  else
    echo "$(date -Is) GLM-5.3 did not resume immediately; next pass will re-evaluate" >> "$LOG"
  fi
fi
exit "$sol_rc"
