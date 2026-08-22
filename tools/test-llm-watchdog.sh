#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d /tmp/bedlam-llm-watchdog-test.XXXXXX)
cleanup() { jobs -pr | xargs -r kill 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims"
printf "# NEXT\n\n## Now\n1. [P4] test task\n\n## Backlog\n" > "$PLAN/.state/NEXT.md"
printf "# AGENTS\n" > "$PLAN/AGENTS.md"
printf "# STATE\n" > "$PLAN/.state/STATE.md"
printf "initial\n" > "$PLAN/code.txt"
git -C "$PLAN" init -q
git -C "$PLAN" config user.email test@example.invalid
git -C "$PLAN" config user.name test
git -C "$PLAN" add AGENTS.md .state/NEXT.md .state/STATE.md code.txt
git -C "$PLAN" commit -qm init

cat > "$TMP/mock-notify-send" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/notifications"
EOF
chmod +x "$TMP/mock-notify-send"

cat > "$TMP/mock-opencode" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/calls"
case "\$*" in
  *bedlam-llm-watchdog-supervise*)
    case "\${MOCK_SUPERVISE:-healthy}" in
      healthy) printf "\033[32mWATCHDOG_OK\033[0m\r\n" ;;
      repair) echo WATCHDOG_REPAIR ;;
      invalid) echo WATCHDOG_OK; echo trailing-text ;;
      race) printf "human pause\n" > "$PLAN/.state/PAUSE"; echo WATCHDOG_REPAIR ;;
      transport) echo "Error: Transport"; exit 1 ;;
    esac
    ;;
  *bedlam-llm-watchdog-repair*)
    if [ "\${MOCK_REPAIR_SLEEP:-0}" = 1 ]; then sleep 3; fi
    if [ "\${MOCK_REPAIR_COMMIT:-0}" = 1 ]; then
      token=\$(cat "$PLAN/.state/PAUSE")
      echo repaired >> "$PLAN/code.txt"
      git -C "$PLAN" add code.txt
      git -C "$PLAN" commit -qm repair -m "Watchdog-Repair: \$token"
    fi
    echo repair-complete
    ;;
esac
EOF
chmod +x "$TMP/mock-opencode"

cat > "$TMP/mock-systemctl" <<EOF
#!/usr/bin/env bash
printf "systemctl %s\n" "\$*" >> "$TMP/systemctl-calls"
case "\$*" in
  *"start bedlam-nudge.service"*) touch "$TMP/nudge-started" ;;
esac
exit 0
EOF
chmod +x "$TMP/mock-systemctl"

cat > "$TMP/mock-proc-check" <<EOF
#!/usr/bin/env bash
[ -e "$PLAN/.state/claims/\$2-owner.claim" ]
EOF
chmod +x "$TMP/mock-proc-check"

# Process matchers must accept the absolute executable path used by systemd.
grep -q "\^\[\^ \]\*opencode2 run" "$ROOT/tools/llm-watchdog.sh"
! grep -q "\"^opencode2 run" "$ROOT/tools/llm-watchdog.sh"
common=(BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-opencode" REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" WATCHDOG_TEST_MODE=1 SUPERVISE_TIMEOUT=5 REPAIR_TIMEOUT=5 REPAIR_COOLDOWN=60 LLM_WATCHDOG_MIN_INTERVAL=0 NOTIFY_SEND="$TMP/mock-notify-send")

# ANSI/CR final marker is accepted as healthy and does not invoke repair agent.
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/healthy.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "bedlam-llm-watchdog-supervise" "$TMP/calls"
! grep -q "bedlam-llm-watchdog-repair" "$TMP/calls"
[ ! -e "$PLAN/.state/PAUSE" ]
grep -q "^state=healthy$" "$PLAN/.state/llm-watchdog-verdict"

# A valid repair runs repair agent (GLM-5.3 high, build agent), requires commit
# evidence, then releases pause.
MOCK_SUPERVISE=repair MOCK_REPAIR_COMMIT=1 env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/repair.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "bedlam-llm-watchdog-repair" "$TMP/calls"
grep -q -- "--agent build" "$TMP/calls"
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]
grep -q "produced evidence" "$PLAN/.state/llm-watchdog.log"
grep -q "^state=repaired$" "$PLAN/.state/llm-watchdog-verdict"

# A supervisor observation failure (invalid marker, transport, timeout) must NOT
# stop workers, invoke the fix agent, or write a cooldown. State becomes unknown.
repair_calls=$(grep -c "llm-watchdog-repair" "$TMP/calls")
notify_lines=$(wc -l < "$TMP/notifications" 2>/dev/null || echo 0)
MOCK_SUPERVISE=invalid env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/invalid.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(grep -c "llm-watchdog-repair" "$TMP/calls")" -eq "$repair_calls" ]
[ ! -e "$PLAN/.state/llm-watchdog-cooldown-until" ]
[ ! -e "$PLAN/.state/PAUSE" ]
grep -q "no valid WATCHDOG_REPAIR marker - not escalating" "$PLAN/.state/llm-watchdog.log"
grep -q "^state=unknown$" "$PLAN/.state/llm-watchdog-verdict"
grep -q "workers left running" "$TMP/notifications"
# A second unknown pass does not re-notify (edge-triggered).
MOCK_SUPERVISE=transport env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/invalid2.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(grep -c "llm-watchdog-repair" "$TMP/calls")" -eq "$repair_calls" ]
[ "$(wc -l < "$TMP/notifications")" -eq "$((notify_lines + 1))" ]

# repair cooldown skips the entire cycle: no supervise call, no fix call.
echo $(( $(date +%s) + 600 )) > "$PLAN/.state/llm-watchdog-cooldown-until"
sup_calls=$(grep -c "llm-watchdog-supervise" "$TMP/calls")
MOCK_SUPERVISE=repair env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/cool.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(grep -c "llm-watchdog-supervise" "$TMP/calls")" -eq "$sup_calls" ]
[ "$(grep -c "llm-watchdog-repair" "$TMP/calls")" -eq "$repair_calls" ]
[ ! -e "$PLAN/.state/PAUSE" ]
grep -q "cycle skipped" "$PLAN/.state/llm-watchdog.log"
grep -q "^state=repair-deferred$" "$PLAN/.state/llm-watchdog-verdict"
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"

# Human PAUSE winning the O_EXCL race is preserved and repair agent is not called again.
repair_calls=$(grep -c "llm-watchdog-repair" "$TMP/calls")
MOCK_SUPERVISE=race env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/race.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(cat "$PLAN/.state/PAUSE")" = "human pause" ]
[ "$(grep -c "llm-watchdog-repair" "$TMP/calls")" -eq "$repair_calls" ]
rm -f "$PLAN/.state/PAUSE"

# Matching stale watchdog tokens are recovered under the singleton lock.
printf "llm-watchdog 999 1\n" > "$PLAN/.state/PAUSE"
printf "llm-watchdog 999 1\n" > "$PLAN/.state/llm-watchdog-pause"
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/stale.lock" "$ROOT/tools/llm-watchdog.sh"
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]
grep -q "recovered stale watchdog-owned pause" "$PLAN/.state/llm-watchdog.log"

# A lone watchdog-format PAUSE (marker lost in a crash window) is recovered
# under the singleton lock - the orphan-token path.
printf "llm-watchdog 999 1\n" > "$PLAN/.state/PAUSE"
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/orphan.lock" "$ROOT/tools/llm-watchdog.sh"
[ ! -e "$PLAN/.state/PAUSE" ]
grep -q "orphan token" "$PLAN/.state/llm-watchdog.log"

# repair agent timeout still releases only its own pause and records cooldown.
MOCK_SUPERVISE=repair MOCK_REPAIR_SLEEP=1 env "${common[@]}" REPAIR_TIMEOUT=1 LLM_WATCHDOG_LOCK="$TMP/timeout.lock" "$ROOT/tools/llm-watchdog.sh"
[ ! -e "$PLAN/.state/PAUSE" ]
[ -e "$PLAN/.state/llm-watchdog-cooldown-until" ]
grep -q "^state=repair-no-evidence$" "$PLAN/.state/llm-watchdog-verdict"
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"

# TERM during repair agent cannot strand a watchdog-owned pause.
MOCK_SUPERVISE=repair MOCK_REPAIR_SLEEP=1 env "${common[@]}" REPAIR_TIMEOUT=10 LLM_WATCHDOG_LOCK="$TMP/signal.lock" "$ROOT/tools/llm-watchdog.sh" &
watchdog_pid=$!
for _ in $(seq 1 100); do [ -e "$PLAN/.state/PAUSE" ] && break; sleep 0.02; done
[ -e "$PLAN/.state/PAUSE" ]
kill -TERM "$watchdog_pid"
wait "$watchdog_pid" 2>/dev/null || true
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]

# A pre-existing human pause prevents all model calls.
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"
printf "human pause\n" > "$PLAN/.state/PAUSE"
lines=$(wc -l < "$TMP/calls")
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/paused.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(wc -l < "$TMP/calls")" -eq "$lines" ]
grep -q "human PAUSE present" "$PLAN/.state/llm-watchdog.log"
rm -f "$PLAN/.state/PAUSE"

# --- Full resume path (production mode) with fake systemd control ---
real=(BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-opencode" REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" SYSTEMCTL_OVERRIDE="$TMP/mock-systemctl" RESUME_PROC_CHECK="$TMP/mock-proc-check" SUPERVISE_TIMEOUT=5 REPAIR_TIMEOUT=5 REPAIR_COOLDOWN=60 LLM_WATCHDOG_MIN_INTERVAL=0 NOTIFY_SEND="$TMP/mock-notify-send" RESUME_WAIT_LOOPS=10 RESUME_WAIT_SLEEP=1)

# A fresh locked claim with matching item and non-empty worker id resumes GLM.
(
  for _ in $(seq 1 200); do [ -s "$PLAN/.state/llm-watchdog-preclaims" ] && break; sleep 0.05; done
  sleep 0.5
  printf "%s
" "reserved fresh" > "$PLAN/.state/claims/1-owner.claim"
  echo "lock-v1 worker testworker1 owns queue item 1" >> "$PLAN/.state/claims/1-owner.claim"
  exec 8>>"$PLAN/.state/claims/1-owner.claim"
  flock 8
  sleep 30
) &
resume_holder=$!
MOCK_SUPERVISE=repair MOCK_REPAIR_COMMIT=1 env "${real[@]}" LLM_WATCHDOG_LOCK="$TMP/resume-ok.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "GLM resumed item 1 as worker testworker1" "$PLAN/.state/llm-watchdog.log"
[ -e "$TMP/nudge-started" ]
kill "$resume_holder" 2>/dev/null || true
rm -f "$PLAN/.state/claims/1-owner.claim" "$TMP/nudge-started"

# A stale-format claim (reserved-only first line, no lock-v1) must never be
# reported as a resume: without a fresh locked claim the resume fails loudly.
notify_before=$(wc -l < "$TMP/notifications")
MOCK_SUPERVISE=repair MOCK_REPAIR_COMMIT=1 env "${real[@]}" RESUME_WAIT_LOOPS=2 RESUME_WAIT_SLEEP=1 LLM_WATCHDOG_LOCK="$TMP/resume-fail.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "GLM failed to resume with a live claim" "$PLAN/.state/llm-watchdog.log"
grep -q "did not resume after repair" "$TMP/notifications"
[ "$(wc -l < "$TMP/notifications")" -gt "$notify_before" ]

echo "llm watchdog tests: PASS"
