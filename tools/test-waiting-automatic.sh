#!/usr/bin/env bash
# Behavioral contract for bounded, executable WAITING-AUTOMATIC work.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PARSER="$ROOT/tools/nudge-free-items.py"
CONTROLLER="$ROOT/tools/nudge.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
WAIT_EXECUTOR="$ROOT/tools/nudge-wait.py"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
# Hermetic environment: run from inside a nudge-launched worker session the
# inherited NUDGE_OWNER_FD / NUDGE_CLAIM_IDENTITY make the agent under test
# skip its claim-owner-exec re-exec and fail launch preflight claim-invalid;
# production units launch through systemd-run with a clean environment.
unset NUDGE_OWNER_FD NUDGE_CLAIM_IDENTITY NUDGE_QUEUE_LOCK_HELD
TMP=$(mktemp -d /tmp/opencode/bedlam-waiting-automatic.XXXXXX)
trap 'jobs -pr | xargs -r kill 2>/dev/null || true; rm -rf "$TMP"' EXIT
failures=0

run_case() {
  local name=$1
  shift
  ( set -e; "$@" )
  local rc=$?
  if [ "$rc" -eq 0 ]; then
    printf 'ok - %s\n' "$name"
  else
    printf 'not ok - %s\n' "$name" >&2
    failures=$((failures + 1))
  fi
}

make_plan() {
  local plan=$1 metadata=$2
  rm -rf "$plan"
  mkdir -p "$plan/.state/claims" "$plan/tools"
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=wait-one] [gate=wait-gate] $metadata bounded machine wait

## Backlog
EOF
  printf '# fixture\n' > "$plan/AGENTS.md"
  printf initial > "$plan/code.txt"
  git -C "$plan" init -q
  git -C "$plan" config user.email test@example.invalid
  git -C "$plan" config user.name test
  git -C "$plan" add .state/NEXT.md AGENTS.md code.txt
  git -C "$plan" commit -qm init
}

authorize_probe() {
  local plan=$1 digest
  mkdir -p "$plan/docs"
  digest=$(sha256sum "$plan/tools/probe.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "tools/probe.sh"
path = "tools/probe.sh"
sha256 = "$digest"
EOF
  git -C "$plan" add tools/probe.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm authorized-probe-fixture
}

expect_invalid() {
  local plan=$1
  set +e
  local state rc
  state=$("$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>"$plan/error")
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
  [ "$state" = INVALID-DEADLOCKED ]
}

case_probe_file_validation() {
  local mode=$1 plan="$TMP/probe-$1" outside="$TMP/outside-probe"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  case "$mode" in
    nonexistent) ;;
    symlink)
      printf '#!/usr/bin/env bash\nexit 0\n' > "$outside"
      chmod +x "$outside"
      ln -s "$outside" "$plan/tools/probe.sh"
      ;;
    non-executable) printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh" ;;
    out-of-repo)
      sed -i 's#probe=tools/probe.sh#probe=../outside-probe#' "$plan/.state/NEXT.md"
      ;;
  esac
  expect_invalid "$plan"
}

case_practical_caps() {
  local field=$1 value=$2
  local plan="$TMP/cap-$field"
  make_plan "$plan" "[probe=tools/probe.sh] [retry=1s] [timeout=10s]"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  case "$field" in
    retry) sed -i "s/retry=1s/retry=$value/" "$plan/.state/NEXT.md" ;;
    timeout) sed -i "s/timeout=10s/timeout=$value/" "$plan/.state/NEXT.md" ;;
    deadline)
      sed -i "s/\[timeout=10s\]/[deadline=$value]/" "$plan/.state/NEXT.md"
      ;;
  esac
  expect_invalid "$plan"
}

make_controller_mocks() {
  cat > "$TMP/network-ok" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > "$TMP/record-spawn" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/spawns"
EOF
  cat > "$TMP/systemctl" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/systemctl-calls"
EOF
  chmod +x "$TMP/network-ok" "$TMP/record-spawn" "$TMP/systemctl"
}

run_controller() {
  local plan=$1 lock=$2
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-spawn" \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$REAPER" \
    NOTIFY_SEND= "$CONTROLLER"
}

case_probe_validation_execution_toctou() {
  local plan="$TMP/probe-toctou" hook="$TMP/probe-toctou-hook" outside="$TMP/outside-executable"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  cat > "$outside" <<EOF
#!/usr/bin/env bash
touch "$TMP/outside-ran"
exit 0
EOF
  chmod +x "$plan/tools/probe.sh" "$outside"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
import subprocess

original_popen = subprocess.Popen
def raced_popen(argv, *args, **kwargs):
    probe = "$plan/tools/probe.sh"
    if not os.path.islink(probe):
        os.unlink(probe)
        os.symlink("$outside", probe)
        open("$TMP/probe-race-barrier", "w").close()
    return original_popen(argv, *args, **kwargs)
subprocess.Popen = raced_popen
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" \
    "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  local rc=$?
  set -e
  [ -e "$TMP/probe-race-barrier" ]
  [ "$rc" -ne 0 ]
  [ ! -e "$TMP/outside-ran" ]
  grep -Eqi 'probe.*(changed|identity|inode|unsafe)|INVALID-DEADLOCKED' "$plan/result"
}

case_probe_success_becomes_runnable() {
  local plan="$TMP/wait-success"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
echo success >> "$TMP/probe-success-calls"
exit 0
EOF
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  : > "$TMP/probe-success-calls"
  : > "$TMP/spawns"
  run_controller "$plan" "$TMP/wait-success.lock"
  grep -q '\[READY\].*\[id=wait-one\].*\[gate=wait-gate\]' "$plan/.state/NEXT.md"
  [ "$(wc -l < "$TMP/probe-success-calls")" -eq 1 ]
  [ "$(wc -l < "$TMP/spawns")" -eq 1 ]
}

case_exact_now_item_promoted_with_duplicate_done_ordinal() {
  local plan="$TMP/wait-duplicate-done"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  cat >> "$plan/.state/NEXT.md" <<'EOF'

## Done
1. DONE historical task with the same ordinal
EOF
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" > "$plan/result"
  grep -q '^1\. \[READY\].*\[id=wait-one\]' "$plan/.state/NEXT.md"
  grep -q '^1\. DONE historical task with the same ordinal$' "$plan/.state/NEXT.md"
}

case_queue_lock_refuses_symlink() {
  local plan="$TMP/wait-queue-lock" sentinel="$TMP/wait-queue-lock-sentinel"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  printf 'DO-NOT-TRUNCATE\n' > "$sentinel"
  ln -s "$sentinel" "$plan/.state/.queue.lock"
  set +e
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
  local rc=$?
  set -e
  [ "$rc" -ne 0 ]
  [ "$(cat "$sentinel")" = DO-NOT-TRUNCATE ]
  grep -q '\[WAITING-AUTOMATIC\]' "$plan/.state/NEXT.md"
}

case_probe_failure_retries_bounded() {
  local plan="$TMP/wait-retry"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=5s]'
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
echo failure >> "$TMP/probe-failure-calls"
exit 1
EOF
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  : > "$TMP/probe-failure-calls"
  run_controller "$plan" "$TMP/wait-retry-a.lock"
  run_controller "$plan" "$TMP/wait-retry-b.lock"
  [ "$(wc -l < "$TMP/probe-failure-calls")" -eq 1 ]
  sleep 1.1
  run_controller "$plan" "$TMP/wait-retry-c.lock"
  [ "$(wc -l < "$TMP/probe-failure-calls")" -eq 2 ]
  python3 - "$plan/.state/automatic-waits/wait-one.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    state = json.load(f)
assert state["attempts"] == 2
assert state["state"] == "waiting"
PY
}

case_wait_timeout_deadlocks() {
  local plan="$TMP/wait-timeout"
  : > "$TMP/spawns"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=2s]'
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  run_controller "$plan" "$TMP/wait-timeout-a.lock"
  sleep 2.1
  set +e
  run_controller "$plan" "$TMP/wait-timeout-b.lock"
  local rc=$?
  set -e
  [ "$rc" -eq 2 ]
  grep -q 'INVALID-DEADLOCKED.*automatic wait.*timeout\|automatic wait.*timeout.*INVALID-DEADLOCKED' "$plan/.state/nudge.log"
  [ ! -s "$TMP/spawns" ]
}

case_due_wait_runs_with_ready_item() {
  local plan="$TMP/mixed-due"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  sed -i '/## Now/a 2. [READY] [id=ready-two] [gate=ready-gate] independent runnable task' "$plan/.state/NEXT.md"
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
touch "$TMP/mixed-probe-ran"
exit 1
EOF
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  : > "$TMP/spawns"
  run_controller "$plan" "$TMP/mixed-due.lock"
  [ -e "$TMP/mixed-probe-ran" ]
  [ "$(wc -l < "$TMP/spawns")" -eq 1 ]
}

case_earlier_timeout_deadline_wins() {
  local plan="$TMP/earlier-bound" deadline deadline_epoch
  deadline=$(date -u -d '+5 minutes' '+%Y-%m-%dT%H:%M:%SZ')
  deadline_epoch=$(date -d "$deadline" +%s)
  make_plan "$plan" "[probe=tools/probe.sh] [retry=1s] [timeout=1h] [deadline=$deadline]"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
  python3 - "$plan/.state/automatic-waits/wait-one.json" "$deadline_epoch" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert abs(float(value["deadline_at"]) - float(sys.argv[2])) <= 1
PY
}

case_clock_rollback_is_detected() {
  local plan="$TMP/clock-rollback" hook="$TMP/clock-rollback-hook" real
  real=$(date +%s)
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10m]'
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  mkdir -p "$plan/.state/automatic-waits" "$hook"
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
  cat > "$hook/sitecustomize.py" <<PY
import time
time.time = lambda: $((real - 100))
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" \
    "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  local rc=$?
  set -e
  [ "$rc" -ne 0 ]
  grep -Eqi 'clock.*rollback|rollback.*clock|monotonic' "$plan/result"
}

write_v2_claim() {
  local plan=$1 session=$2 fields status id gate body dev ino queue
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r status id gate body dev ino queue <<< "$fields"
  cat > "$plan/.state/claims/1-$session.claim" <<EOF
lock-v2
ordinal=1
id=wait-one
gate=wait-gate
owner=worker
session=$session
claimed_at=$(date -Is)
unit=bedlam-nudge-item1-$session
pid=$$
body_sha256=$body
queue_device=$dev
queue_inode=$ino
queue_sha256=$queue
EOF
}

case_no_commit_waiting_with_executor_evidence_succeeds() {
  local plan="$TMP/worker-wait-evidence" session=wait-evidence
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  sed -i 's/\[WAITING-AUTOMATIC\].* bounded machine wait/[READY] [id=wait-one] [gate=wait-gate] initial ready task/' "$plan/.state/NEXT.md"
  # Restore a canonical single set of identity metadata after the fixture edit.
  sed -i 's/\[id=wait-one\] \[gate=wait-gate\] \[READY\] \[id=wait-one\] \[gate=wait-gate\]/[READY] [id=wait-one] [gate=wait-gate]/' "$plan/.state/NEXT.md"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  write_v2_claim "$plan" "$session"
  cat > "$TMP/write-wait-evidence" <<EOF
#!/usr/bin/env bash
cat > "$plan/.state/NEXT.md" <<'NEXT'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=wait-one] [gate=wait-gate] [probe=tools/probe.sh] [retry=1s] [timeout=10s] bounded machine wait

## Backlog
NEXT
"$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
exit 0
EOF
  chmod +x "$TMP/write-wait-evidence"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/write-wait-evidence" \
    NUDGE_LOCK="$TMP/wait-evidence.lock" "$AGENT" 1 "$session"
  grep -q 'ended cleanly (rc=0 progress=0)' "$plan/.state/nudge.log"
  [ ! -e "$plan/.state/automation-failures/$session.json" ]
}

case_rate_limit_has_explicit_bounded_outcome() {
  local plan="$TMP/rate-limit-outcome" session=rate-limit reset
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  printf '# NEXT\n\n## Now\n1. [READY] [id=wait-one] [gate=wait-gate] provider task\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  write_v2_claim "$plan" "$session"
  # The provider string has no timezone suffix and production parses it with
  # local `date -d`, so generate a genuinely future local timestamp.
  reset=$(date -d '+2 hours' '+%Y-%m-%d %H:%M:%S')
  cat > "$TMP/rate-limit-model" <<EOF
#!/usr/bin/env bash
echo 'Error: Usage limit reached for 5 hour. Your limit will reset at $reset'
exit 1
EOF
  chmod +x "$TMP/rate-limit-model"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/rate-limit-model" \
    NUDGE_LOCK="$TMP/rate-limit.lock" "$AGENT" 1 "$session"
  set -e
  local explicit=0
  grep -q '\[WAITING-AUTOMATIC\]' "$plan/.state/NEXT.md" && explicit=1
  if find "$plan/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit 2>/dev/null | grep -q .; then
    explicit=1
  fi
  [ "$explicit" -eq 1 ]
  [ ! -d "$plan/.state/taskcooldown" ] || [ -z "$(find "$plan/.state/taskcooldown" -type f -print -quit)" ]
}

case_expiry_emits_failure_and_triggers_watchdog() {
  local plan="$TMP/wait-expiry-beacon"
  make_plan "$plan" '[probe=tools/probe.sh] [retry=1s] [timeout=10s]'
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  mkdir -p "$plan/.state/automatic-waits"
  cat > "$plan/.state/automatic-waits/wait-one.json" <<'EOF'
{"schema":"nudge-wait-v1","version":1,"ordinal":1,"id":"wait-one","gate":"wait-gate","probe":"tools/probe.sh","started_at":1,"deadline_at":2,"next_attempt_at":1,"attempts":1,"state":"waiting"}
EOF
  chmod 600 "$plan/.state/automatic-waits/wait-one.json"
  : > "$TMP/systemctl-calls"
  set +e
  run_controller "$plan" "$TMP/wait-expiry.lock"
  local rc=$?
  set -e
  [ "$rc" -eq 2 ]
  local artifact
  artifact=$(find "$plan/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit)
  [ -n "$artifact" ]
  python3 - "$artifact" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["kind"] in {"wait-timeout", "deadline-expired"}
assert value["repair"] == "required"
PY
  grep -q 'start bedlam-llm-watchdog.service' "$TMP/systemctl-calls"
}

make_controller_mocks
run_case 'WAITING rejects a nonexistent probe' case_probe_file_validation nonexistent
run_case 'WAITING rejects a symlink probe' case_probe_file_validation symlink
run_case 'WAITING rejects a non-executable probe' case_probe_file_validation non-executable
run_case 'WAITING rejects an out-of-repo probe' case_probe_file_validation out-of-repo
run_case 'validated probe replacement cannot execute an outside inode' case_probe_validation_execution_toctou
run_case 'WAITING retry has a practical maximum' case_practical_caps retry 2h
run_case 'WAITING timeout has a practical maximum' case_practical_caps timeout 2d
run_case 'WAITING deadline has a practical horizon' case_practical_caps deadline 2099-01-01T00:00:00Z
run_case 'successful probe transitions to runnable and spawns' case_probe_success_becomes_runnable
run_case 'promotion targets the exact Now item when Done repeats its ordinal' case_exact_now_item_promoted_with_duplicate_done_ordinal
run_case 'queue transition lock refuses symlink without truncating target' case_queue_lock_refuses_symlink
run_case 'failed probe obeys retry cadence and bounded state' case_probe_failure_retries_bounded
run_case 'expired automatic wait is INVALID-DEADLOCKED' case_wait_timeout_deadlocks
run_case 'due waiting probes run even while a READY item exists' case_due_wait_runs_with_ready_item
run_case 'timeout plus deadline uses the earlier bound' case_earlier_timeout_deadline_wins
run_case 'wall-clock rollback is detected rather than extending a wait' case_clock_rollback_is_detected
run_case 'no-commit READY to WAITING with executor evidence is success' case_no_commit_waiting_with_executor_evidence_succeeds
run_case 'provider rate limit has explicit bounded queue or failure outcome' case_rate_limit_has_explicit_bounded_outcome
run_case 'wait expiry emits structured failure and triggers watchdog' case_expiry_emits_failure_and_triggers_watchdog

if [ "$failures" -ne 0 ]; then
  printf 'waiting automatic tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'waiting automatic tests: PASS'
