#!/usr/bin/env bash
# One asynchronous bedlam agent plus adaptive-concurrency result reporting.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
CLAIMS="$STATE/claims"
CONC_FILE="$STATE/concurrency"
CONC_DOWN_TS="$STATE/conc-degraded-at"
CONC_MIN=1
item=${1:?queue item required}
slotid=${2:?slot id required}
LOG="$STATE/agent-$slotid.log"
OPENC=${OPENC_OVERRIDE:-opencode2}

cd "$PLAN_DIR" || exit 1
start_head=$(git rev-parse HEAD 2>/dev/null || echo none)
start_next=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)
start_state=$(stat -c %Y "$STATE/STATE.md" 2>/dev/null || echo 0)
touch "$STATE/heartbeat"

placeholder="$CLAIMS/$item-$slotid.claim"
own="$CLAIMS/$item-owner.claim"
# Convert the spawner reservation into exactly one owned claim before the
# model starts. This removes startup races and does not rely on model cleanup.
if [ -e "$own" ]; then
  rm -f "$placeholder"
  echo "$(date -Is) item $item already owned; worker standing down" >> "$STATE/nudge.log"
  exit 75
fi
mv "$placeholder" "$own" 2>/dev/null || {
  echo "$(date -Is) missing reservation for item $item; worker standing down" >> "$STATE/nudge.log"
  exit 75
}
echo "worker $slotid owns queue item $item" > "$own"

PROMPT="You are an unattended continuation agent for bedlam-re. Read AGENTS.md and follow its workflow EXACTLY. HARD CONCURRENCY RULE: do NOT spawn, delegate to, or invoke any subagent; no nesting is allowed. You personally perform one bounded work unit only. Your queue item $item claim is already owned by this worker; do not create, replace, or select another claim. Work ONLY item $item in the Now section of .state/NEXT.md. If item $item is already owned, stop and release your placeholder; do not choose another item. Commit EARLY and OFTEN. AT END: update NEXT.md, delete your claim, and push. On model, transport, or API error, record completed work in NEXT.md and commit if possible; leave the claim for ghost accounting. Never start an analyzeHeadless import already running or succeeded. Do not ask questions or wait for input."

set +e
timeout 3900 "$OPENC" run --auto --title "bedlam-nudge-item$item" "$PROMPT" >> "$LOG" 2>&1
rc=$?
set -e
cat "$LOG" >> "$STATE/nudge-run.log" 2>/dev/null || true

end_head=$(git rev-parse HEAD 2>/dev/null || echo none)
end_next=$(stat -c %Y "$STATE/NEXT.md" 2>/dev/null || echo 0)
end_state=$(stat -c %Y "$STATE/STATE.md" 2>/dev/null || echo 0)
progress=0
if [ "$end_head" != "$start_head" ] || [ "$end_next" -gt "$start_next" ] || [ "$end_state" -gt "$start_state" ]; then progress=1; fi

kind=none
if grep -aqE "Rate limit reached|rate limit|usage limit|HTTP[^0-9]*429|429 Too Many Requests" "$LOG"; then
  kind=rate-limit
elif grep -aqE "Decode error|Error:.*Transport|Error: Transport" "$LOG"; then
  kind=transport
elif [ "$rc" -ne 0 ]; then
  kind=client-error
fi

exec 9>/tmp/bedlam-nudge.lock
flock 9
cur=$(cat "$CONC_FILE" 2>/dev/null || echo 3)
if [ "$kind" != none ]; then
  if [ "$cur" -gt "$CONC_MIN" ]; then
    echo $((cur-1)) > "$CONC_FILE"
    date +%s > "$CONC_DOWN_TS"
    echo "$(date -Is) agent item $item failed [$kind rc=$rc progress=$progress]; concurrency degraded $cur -> $((cur-1))" >> "$STATE/nudge.log"
  else
    echo "$(date -Is) agent item $item failed [$kind rc=$rc progress=$progress]; concurrency remains 1" >> "$STATE/nudge.log"
  fi
else
  echo "$(date -Is) agent item $item ended cleanly (rc=$rc progress=$progress)" >> "$STATE/nudge.log"
fi

if [ "$kind" = transport ]; then
  if [ -f "$own" ]; then rm -f "$placeholder"; else touch "$placeholder"; fi
elif [ "$kind" = rate-limit ] || [ "$kind" = client-error ]; then
  rm -f "$placeholder" "$own"
else
  rm -f "$placeholder" "$own"
fi
exit "$rc"
