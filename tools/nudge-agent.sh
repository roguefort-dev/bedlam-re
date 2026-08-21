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

PROMPT="You are an unattended continuation agent for bedlam-re. Read AGENTS.md and follow it EXACTLY. HARD CONCURRENCY RULE: do NOT spawn or invoke subagents. You personally perform one bounded unit. The wrapper atomically acquired queue item $item for slot $slotid before launching you; .state/claims/$item-owner.claim naming worker $slotid is YOUR claim, not another owner claim. NEVER infer ownership from Ghostty, cmux, an operator OpenCode TUI, editors, shells, process age, dirty files, prior decisions, or historical stand-down entries. The persistent operator TUI is supervisory and never blocks work; only .state/PAUSE does. Work ONLY item $item and never switch items. For any reverse-engineering or analysis-heavy step, decode a bounded piece and immediately write committed RE notes before continuing - never reason silently for more than a few minutes (the client dies after 300s of silent streaming). Inspect and adopt relevant interrupted WIP while preserving unrelated changes; stage explicit paths only. Do not create state-only stand-down commits. If genuinely blocked, rewrite item $item with a [BLOCKED] tag and one concrete reason, then stop. Every commit you create MUST include the exact trailer Nudge-Worker: $slotid (for example, a second git commit -m paragraph). Commit and push substantive completed work. Checkpoint aggressively: whenever a coherent milestone compiles and is green (tests+fmt+clippy), commit it immediately with your trailer and push - never hold all work uncommitted until the end of the unit; a session can die at any moment. NEVER create, delete, rename, or modify claim files; the wrapper owns them. On model/transport/API error, commit recoverable substantive work if possible. Never start an analyzeHeadless import already running or succeeded. Do not ask questions or wait for input."

set +e
# --agent build: the nudge worker needs the coding agent, NOT the
# config default (orchestrator). The 2026-08-20 watchdog repair found
# the default agent carried steps:60 (opencode2 then kills the run
# text-only mid-unit: "Maximum steps for this agent reached", rc=0,
# no commit possible) and denied the edit tool (workers resorted to
# shell+python file surgery, wasting steps). The build agent has no
# step cap and full tools; the outer timeout 3900 stays the bound.
#
# Idle-log reaper (watchdog repair, 2026-08-21): the opencode2 client
# can print "Error: Transport" and then never exit - observed hung in
# do_epoll_wait at zero CPU with its agent log frozen for 30+ minutes
# (worker 82523e41, item 1, task 230a7a38b991ed5f), invisible to the
# controller (a live locked claim) and unaffected by the provider's
# own 300s zero-stream watchdog, burning the whole 65-minute slot
# budget on a single provider hiccup while every controller tick
# logged "concurrency full - standing down". So supervise the client
# ourselves: if the agent log stays silent past NUDGE_IDLE_LIMIT while
# the process lives, terminate it and let the run classify as
# provider-side transport (not charged to the task), same as any
# other stream death. A healthy run emits tool/text output far more
# often than the limit; long silent tool calls (cold cargo builds)
# stay safe at the 900s default.
IDLE_LIMIT=${NUDGE_IDLE_LIMIT:-900}
IDLE_POLL=${NUDGE_IDLE_POLL:-5}
reaped=0
timeout 3900 "$OPENC" run --standalone --agent build --model "$MODEL" --auto --title "bedlam-nudge-item$item" "$PROMPT" >> "$LOG" 2>&1 &
agent_pid=$!
while kill -0 "$agent_pid" 2>/dev/null; do
  sleep "$IDLE_POLL"
  kill -0 "$agent_pid" 2>/dev/null || break
  now=$(date +%s)
  log_mtime=$(stat -c %Y "$LOG" 2>/dev/null || echo "$now")
  idle=$(( now - log_mtime ))
  if [ "$idle" -ge "$IDLE_LIMIT" ]; then
    reaped=1
    echo "$(date -Is) idle-log reaper: item $item agent log silent ${idle}s >= ${IDLE_LIMIT}s; terminating hung client pid $agent_pid" >> "$STATE/nudge.log"
    echo "$(date -Is) idle-log reaper: terminating after ${idle}s with no agent-log output" >> "$LOG"
    kill -TERM "$agent_pid" 2>/dev/null
    for _ in $(seq 1 10); do
      kill -0 "$agent_pid" 2>/dev/null || break
      sleep 1
    done
    kill -KILL "$agent_pid" 2>/dev/null
    pkill -KILL -P "$agent_pid" 2>/dev/null
    break
  fi
done
wait "$agent_pid"
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
# -i: the provider prints "Usage limit reached" (capital U); the
# pre-2026-08-21 case-sensitive matcher missed it and every quota death
# fell through to client-error (watchdog repair, 2026-08-21).
if grep -aqiE "Rate limit reached|rate limit|usage limit|HTTP[^0-9]*429|429 Too Many Requests" "$LOG"; then
  kind=rate-limit
# Provider HTTP 5xx ("Provider request failed with HTTP 502", the
# 2026-08-21 19:15/19:34 incident) is provider-side overload, not a
# client error: both 502 deaths fell through to client-error and
# charged task 1c8526453c786dd5 to 2/3 - one more would have armed
# the cooldown+notify spiral mid-incident (watchdog repair,
# 2026-08-21).
elif grep -aqE "Decode error|Error:.*Transport|Error: Transport|ECONNRESET|socket connection was closed|getaddrinfo ENOTFOUND|DNS|Invalid [A-Za-z0-9_./-]+/openai-compatible-chat stream event|Provider request failed with HTTP 5[0-9][0-9]" "$LOG"; then
  kind=transport
elif [ "$reaped" -eq 1 ]; then
  # The idle-log reaper terminated a hung client (2026-08-21 watchdog
  # repair): a provider-side stream death that never exited on its
  # own. Same accounting exemption as transport so a hang never feeds
  # the taskfails/cooldown spiral.
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
elif [ "$kind" = transport ]; then
  # A provider-side stream failure is environmental, not a task failure
  # (watchdog repair, 2026-08-20: "Invalid zai-coding-plan/openai-
  # compatible-chat stream event" killed nine spawns within seconds plus
  # two mid-run sessions between 20:42 and 22:59, charging task
  # 4f6a0d2b eleven fails so 15-min cooldowns persisted for 2.5h after
  # the provider recovered). Log loudly, keep the claim for the
  # DEAD_CLAIM_TTL retry backoff (which throttles spawn churn during a
  # live incident), never punish the task; the llm-watchdog owns
  # provider-incident escalation.
  reap_note=""
  [ "$reaped" -eq 1 ] && reap_note=" (idle-log reaper)"
  echo "$(date -Is) agent item $item failed [transport rc=$rc progress=$progress] task=$task_hash; provider-side, not charged to the task$reap_note" >> "$STATE/nudge.log"
elif [ "$kind" = rate-limit ]; then
  # Provider quota exhaustion is provider-side, not a task failure
  # (watchdog repair, 2026-08-21: "Usage limit reached for 5 hour" killed
  # every spawn at its first model call between 07:08 and 07:52, four
  # deaths mislabeled client-error charged task c72d408d50275d04 with
  # fails + 15-min cooldowns that rolled over repeatedly for the rest of
  # the quota window). Never touch taskfails; instead hold this task in
  # cooldown until the reset timestamp the provider prints (fallback and
  # sanity cap: the standard 900s window), but never trust that stamp
  # beyond one probe interval (see the 1800s cap below), so the
  # controller stands down without cycling doomed ~40s spawns and still
  # notices early recovery. The llm-watchdog owns cross-item escalation
  # (global pause) for longer or unparsable outages.
  reset=$(grep -aoiE "will reset at [0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}" "$LOG" | head -n 1 | cut -d' ' -f4-5)
  until=$(date -d "$reset" +%s 2>/dev/null || echo 0)
  now_ts=$(date +%s)
  if [ "$until" -le "$now_ts" ] || [ "$until" -gt $(( now_ts + 21600 )) ]; then
    until=$(( now_ts + 900 ))
  fi
  # Watchdog repair 2, 2026-08-21 ~08:30: the provider's reset stamp can
  # be wildly wrong. This morning it printed "will reset at 13:41:22"
  # from ~07:45 but resumed serving at ~07:52; the armed 5h59m cooldown
  # froze the sole queue item on a healthy provider for a would-be ~5h.
  # Cap the armed cooldown at one probe interval: if the quota window is
  # genuinely still open, the next probe dies in ~40s with the same
  # rate-limit signature and re-arms the cap (at most one benign probe
  # per 30 min, no taskfails charge); if the provider recovered early,
  # the loop is back within 30 min instead of hours.
  if [ "$until" -gt $(( now_ts + 1800 )) ]; then
    until=$(( now_ts + 1800 ))
  fi
  mkdir -p "$STATE/taskcooldown"
  echo "$until" > "$STATE/taskcooldown/$task_hash"
  echo "$(date -Is) agent item $item failed [rate-limit rc=$rc progress=$progress] task=$task_hash; provider quota, not charged to the task; cooling down until $(date -d "@$until" '+%F %T %z')" >> "$STATE/nudge.log"
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

# Event-driven chaining (2026-08-21): on a clean end, age the heartbeat so
# the controller freshness gate passes immediately (the same trick the
# watchdog resume path uses) and fire one instant nudge pass. The 60s timer
# stays armed as a floor; every pass remains idempotent under the controller
# lock. Test safety: SYSTEMD_RUN_OVERRIDE marks the hermetic controller run
# (never touch the real systemd there); SYSTEMCTL_OVERRIDE lets tests
# record the chained call instead.
if [ "$kind" = none ] && [ "$rc" -eq 0 ]; then
  touch -d @0 "$STATE/heartbeat"
  if [ -n "${SYSTEMCTL_OVERRIDE:-}" ]; then
    "$SYSTEMCTL_OVERRIDE" --user start bedlam-nudge.service >/dev/null 2>&1 || true
  elif [ -z "${SYSTEMD_RUN_OVERRIDE:-}" ]; then
    systemctl --user start bedlam-nudge.service >/dev/null 2>&1 || true
    # A pass can be mid-flight at the same instant (lock busy at 22:46:31
    # on 2026-08-21): re-trigger once a few seconds later so a pass always
    # runs AFTER claim release + heartbeat aging, still far ahead of the timer.
    systemd-run --user --collect --on-active=4s "--unit=bedlam-nudge-chain-$slotid" systemctl --user start bedlam-nudge.service >/dev/null 2>&1 || true
  fi
fi
exit "$rc"
