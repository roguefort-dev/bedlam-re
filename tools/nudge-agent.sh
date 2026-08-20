#!/usr/bin/env bash
# One asynchronous bedlam agent plus adaptive-concurrency result reporting.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
CLAIMS="$STATE/claims"
CONC_FILE="$STATE/concurrency"
CONC_DOWN_TS="$STATE/conc-degraded-at"
NUDGE_LOCK=${NUDGE_LOCK:-/tmp/bedlam-nudge.lock}
CONC_MIN=1
item=${1:?queue item required}
slotid=${2:?slot id required}
LOG="$STATE/agent-$slotid.log"
if [ -n "${OPENC_OVERRIDE:-}" ]; then
  OPENC=$OPENC_OVERRIDE
elif command -v opencode2 >/dev/null 2>&1; then
  OPENC=$(command -v opencode2)
else
  OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/bin/opencode2
fi
MODEL=zai-coding-plan/glm-5.3
NOTIFY_SEND=${NOTIFY_SEND-notify-send}
unit_name="bedlam-nudge-item${item}-${slotid}"

cd "$PLAN_DIR" || exit 1
start_head=$(git rev-parse HEAD 2>/dev/null || echo none)
touch "$STATE/heartbeat"
placeholder="$CLAIMS/$item-$slotid.claim"
own="$CLAIMS/$item-owner.claim"

# A pause created after the timer reserved this slot still wins before launch.
if [ -e "$STATE/PAUSE" ]; then
  rm -f "$placeholder"
  exit 0
fi

# Atomically publish the canonical owner name without replacing an existing owner.
if ! ln "$placeholder" "$own" 2>/dev/null; then
  rm -f "$placeholder"
  echo "$(date -Is) item $item already owned; worker standing down" >> "$STATE/nudge.log"
  exit 75
fi
rm -f "$placeholder"
exec 8>>"$own"
flock 8 || {
  rm -f "$own"
  echo "$(date -Is) failed to lock claim for item $item; worker standing down" >> "$STATE/nudge.log"
  exit 75
}

# Re-check PAUSE after winning the claim: a watchdog pause that appeared while
# we were claiming must still stop this launch before the model runs.
if [ -e "$STATE/PAUSE" ]; then
  exec 8>&-
  rm -f "$own"
  echo "$(date -Is) PAUSE appeared during claim acquisition for item $item; worker standing down" >> "$STATE/nudge.log"
  exit 0
fi

task_hash=$(sed -n "s/^[[:space:]]*$item\.[[:space:]]*//p" "$STATE/NEXT.md" 2>/dev/null | head -n 1 | sha256sum | cut -c1-16)
echo "lock-v1 worker $slotid owns queue item $item" >&8
echo "task=$task_hash" >&8
echo "unit=$unit_name" >&8
claim_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)

PROMPT="You are an unattended continuation agent for bedlam-re. Read AGENTS.md and follow it EXACTLY. HARD CONCURRENCY RULE: do NOT spawn or invoke subagents. You personally perform one bounded unit. The wrapper atomically acquired queue item $item for slot $slotid before launching you; .state/claims/$item-owner.claim naming worker $slotid is YOUR claim, not another owner claim. NEVER infer ownership from Ghostty, cmux, an operator OpenCode TUI, editors, shells, process age, dirty files, prior decisions, or historical stand-down entries. The persistent operator TUI is supervisory and never blocks work; only .state/PAUSE does. Work ONLY item $item and never switch items. Inspect and adopt relevant interrupted WIP while preserving unrelated changes; stage explicit paths only. Do not create state-only stand-down commits. If genuinely blocked, rewrite item $item with a [BLOCKED] tag and one concrete reason, then stop. Every commit you create MUST include the exact trailer Nudge-Worker: $slotid (for example, a second git commit -m paragraph). Commit and push substantive completed work. Checkpoint aggressively: whenever a coherent milestone compiles and is green (tests+fmt+clippy), commit it immediately with your trailer and push - never hold all work uncommitted until the end of the unit; a session can die at any moment. NEVER create, delete, rename, or modify claim files; the wrapper owns them. On model/transport/API error, commit recoverable substantive work if possible. Never start an analyzeHeadless import already running or succeeded. Do not ask questions or wait for input."

set +e
# --agent build: the nudge worker needs the coding agent, NOT the
# config default (orchestrator). The 2026-08-20 watchdog repair found
# the default agent carried steps:60 (opencode2 then kills the run
# text-only mid-unit: "Maximum steps for this agent reached", rc=0,
# no commit possible) and denied the edit tool (workers resorted to
# shell+python file surgery, wasting steps). The build agent has no
# step cap and full tools; the outer timeout 3900 stays the bound.
timeout 3900 "$OPENC" run --standalone --agent build --model "$MODEL" --auto --title "bedlam-nudge-item$item" "$PROMPT" >> "$LOG" 2>&1
rc=$?
set -e
cat "$LOG" >> "$STATE/nudge-run.log" 2>/dev/null || true

end_head=$(git rev-parse HEAD 2>/dev/null || echo none)
progress=0
if [ "$end_head" != "$start_head" ] && git cat-file -e "$start_head^{commit}" 2>/dev/null; then
  for commit in $(git rev-list "$start_head..$end_head" 2>/dev/null); do
    if git log -1 --format=%B "$commit" | grep -qx "Nudge-Worker: $slotid" \
        && git diff-tree --no-commit-id --name-only -r "$commit" | grep -qv "^\.state/"; then
      progress=1
      break
    fi
  done
fi

kind=none
if grep -aqE "Rate limit reached|rate limit|usage limit|HTTP[^0-9]*429|429 Too Many Requests" "$LOG"; then
  kind=rate-limit
elif grep -aqE "Decode error|Error:.*Transport|Error: Transport|ECONNRESET|socket connection was closed|getaddrinfo ENOTFOUND|DNS" "$LOG"; then
  kind=transport
elif grep -aq "Maximum steps for this agent" "$LOG"; then
  # opencode2 truncated the run at the agent step budget. Distinct
  # from no-progress: the model was still working (rc=0, text-only
  # hand-off, WIP on disk for the next spawn to adopt).
  kind=step-cap
elif [ "$rc" -ne 0 ]; then
  kind=client-error
elif [ "$progress" -eq 0 ] && ! grep -qE "^[[:space:]]*$item\.[[:space:]]+(\[[^]]+\][[:space:]]*)*\[BLOCKED\]" "$STATE/NEXT.md" 2>/dev/null; then
  kind=no-progress
fi

exec 9>"$NUDGE_LOCK"
flock 9
cur=$(cat "$CONC_FILE" 2>/dev/null || echo 3)
if [ "$kind" = step-cap ]; then
  # A step-capped run is a budget truncation, not a task failure:
  # counting it as no-progress sent the loop into a fail/cooldown
  # spiral while every attempt made real uncommitted progress
  # (watchdog repair, 2026-08-20; four spawns died at the cap on
  # task 247ce5e255167e9a). Log loudly, keep the claim for the
  # short DEAD_CLAIM_TTL backoff, never punish the task.
  echo "$(date -Is) agent item $item hit the opencode2 step cap [rc=$rc progress=$progress] task=$task_hash; treating as truncation, not failure" >> "$STATE/nudge.log"
elif [ "$kind" != none ]; then
  # Failures are scoped to this task, not to the whole controller, so
  # unrelated items can never be blamed for (or cleared by) this run.
  mkdir -p "$STATE/taskfails" "$STATE/taskcooldown"
  fails_file="$STATE/taskfails/$task_hash"
  fail_count=$(( $(cat "$fails_file" 2>/dev/null || echo 0) + 1 ))
  echo "$fail_count" > "$fails_file"
  if [ "$fail_count" -ge 3 ]; then
    echo $(( $(date +%s) + 900 )) > "$STATE/taskcooldown/$task_hash"
    if [ "$fail_count" -eq 3 ] && [ -n "$NOTIFY_SEND" ] && command -v "$NOTIFY_SEND" >/dev/null 2>&1; then
      "$NOTIFY_SEND" -u critical "bedlam-re repeated agent failures" "item $item failed three consecutive observed runs ($kind, task $task_hash); cooling down 15 minutes" 2>/dev/null || true
    fi
  fi
  if [ "$cur" -gt "$CONC_MIN" ]; then
    echo $((cur-1)) > "$CONC_FILE"
    date +%s > "$CONC_DOWN_TS"
    echo "$(date -Is) agent item $item failed [$kind rc=$rc progress=$progress] task=$task_hash; concurrency degraded $cur -> $((cur-1))" >> "$STATE/nudge.log"
  else
    echo "$(date -Is) agent item $item failed [$kind rc=$rc progress=$progress] task=$task_hash; concurrency remains 1" >> "$STATE/nudge.log"
  fi
else
  rm -f "$STATE/taskfails/$task_hash"
  echo "$(date -Is) agent item $item ended cleanly (rc=$rc progress=$progress) task=$task_hash" >> "$STATE/nudge.log"
fi

# Drop only this wrapper lock, then inspect the same inode we acquired. A live
# inherited descriptor is a real ghost and keeps the claim. Failed runs retain
# an unlocked claim for DEAD_CLAIM_TTL as intentional retry backoff.
exec 8>&-
rm -f "$placeholder"
current_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)
if [ "$current_identity" = "$claim_identity" ] && grep -q "^lock-v1 worker $slotid owns queue item $item$" "$own" 2>/dev/null; then
  if flock -n "$own" true 2>/dev/null; then
    if [ "$kind" = none ] && [ "$rc" -eq 0 ]; then
      rm -f "$own"
    else
      touch "$own"
      echo "$(date -Is) retaining failed item $item claim for retry backoff" >> "$STATE/nudge.log"
    fi
  else
    echo "$(date -Is) retaining item $item claim held by a live descendant" >> "$STATE/nudge.log"
  fi
fi
exit "$rc"
