#!/usr/bin/env bash
# End-to-end failure-beacon and watchdog evidence contracts.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
# Hermetic environment: these suites are routinely run from INSIDE a
# nudge-launched worker session, whose wrapper exports NUDGE_OWNER_FD /
# NUDGE_CLAIM_IDENTITY (and may hold NUDGE_QUEUE_LOCK_HELD) for its OWN
# claim. Without stripping them the agent under test skips its
# claim-owner-exec re-exec and fails launch preflight claim-invalid.
# Production units launch through systemd-run with a clean environment.
unset NUDGE_OWNER_FD NUDGE_CLAIM_IDENTITY NUDGE_QUEUE_LOCK_HELD
WATCHDOG="$ROOT/tools/llm-watchdog.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
TMP=$(mktemp -d /tmp/opencode/bedlam-failure-watchdog.XXXXXX)
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

make_repo() {
  local plan=$1
  rm -rf "$plan"
  mkdir -p "$plan/.state/claims" "$plan/.state/automation-failures" "$plan/tools"
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [READY] [id=stable-one] [gate=gate-one] automated task

## Backlog
EOF
  cat > "$plan/AGENTS.md" <<'EOF'
# Legacy fixture
If genuinely blocked, tag the required item [BLOCKED] and ask a human to take over.
EOF
  printf 'initial\n' > "$plan/code.txt"
  git -C "$plan" init -q
  git -C "$plan" config user.email test@example.invalid
  git -C "$plan" config user.name test
  git -C "$plan" add .state/NEXT.md AGENTS.md code.txt
  git -C "$plan" commit -qm init
}

write_failure() {
  local plan=$1 session=$2 kind=${3:-client-error}
  cat > "$plan/.state/automation-failures/$session.json" <<EOF
{"schema":"nudge-failure-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","owner":"worker","session":"$session","kind":"$kind","reason":"test","evidence":"fixture","time":"2026-08-26T07:00:00Z","repair":"required","queue_unchanged":true}
EOF
  chmod 600 "$plan/.state/automation-failures/$session.json"
}

make_mocks() {
  local plan=$1
  cat > "$TMP/systemctl" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/systemctl-calls"
exit 0
EOF
  cat > "$TMP/notify" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/notifications"
EOF
  cat > "$TMP/opencode" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/model-calls"
case "\$*" in
  *bedlam-llm-watchdog-supervise*)
    echo WATCHDOG_OK
    ;;
  *bedlam-llm-watchdog-repair*)
    cp "$plan/.state/llm-watchdog-snapshot" "$TMP/snapshot-seen"
    case "\${MOCK_REPAIR_MODE:-commit}" in
      commit)
        token=\$(cat "$plan/.state/PAUSE")
        if grep -q '\[BLOCKED' "$plan/.state/NEXT.md"; then
          printf '# NEXT\n\n## Now\n1. [READY] [id=stable-one] [gate=gate-one] repaired automated task\n\n## Backlog\n' > "$plan/.state/NEXT.md"
        fi
        sed -i 's/id=stable-one/id=replacement-task/; s/gate=gate-one/gate=replacement-gate/' "$plan/.state/NEXT.md"
        echo repair >> "$plan/code.txt"
        git -C "$plan" add code.txt .state/NEXT.md
        git -C "$plan" commit -qm repair -m "Watchdog-Repair: \$token"
        remediation_commit=\$(git -C "$plan" rev-parse HEAD)
        if [ -f "$plan/.state/llm-watchdog-failure-snapshot.json" ]; then
          python3 - "\$remediation_commit" <<'PY'
import json, sys
from pathlib import Path
snapshot = Path("$plan/.state/llm-watchdog-failure-snapshot.json")
records = [dict(record, resolution="replaced-task", remediation_commit=sys.argv[1]) for record in json.loads(snapshot.read_text()) if record.get("id") == "stable-one" and record.get("gate") == "gate-one"]
Path("$plan/.state/llm-watchdog-failure-ack.json").write_text(json.dumps({"schema": "nudge-failure-ack-v1", "records": records}) + "\n")
PY
        fi
        ;;
      waiting)
        cat > "$plan/.state/NEXT.md" <<'WAIT'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=tools/probe.sh] [retry=1s] [timeout=10s] wait on failing machine probe

## Backlog
WAIT
        ;;
      forged-waiting)
        cat > "$plan/.state/NEXT.md" <<'WAIT'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=tools/probe.sh] [retry=1s] [timeout=10s] forged wait evidence

## Backlog
WAIT
        mkdir -m 700 -p "$plan/.state/automatic-waits"
        cat > "$plan/.state/automatic-waits/stable-one.json" <<WAITSTATE
{"schema":"nudge-wait-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","probe":"tools/probe.sh","started_at":\$(date +%s),"deadline_at":\$(( \$(date +%s) + 600 )),"next_attempt_at":\$(( \$(date +%s) + 300 )),"attempts":1,"state":"waiting"}
WAITSTATE
        chmod 600 "$plan/.state/automatic-waits/stable-one.json"
        ;;
      concurrent-artifacts)
        trigger="\${MOCK_TRIGGER_SESSION:-replace-trigger}"
        rm -f "$plan/.state/automation-failures/\$trigger.json"
        cat > "$plan/.state/automation-failures/\$trigger.json" <<JSON
{"schema":"nudge-failure-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","owner":"worker","session":"\$trigger","kind":"replacement-race","reason":"concurrent replacement","evidence":"new inode","time":"2026-08-26T07:01:00Z","repair":"required","queue_unchanged":true}
JSON
        cat > "$plan/.state/automation-failures/concurrent-new.json" <<'JSON'
{"schema":"nudge-failure-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","owner":"worker","session":"concurrent-new","kind":"concurrent","reason":"arrived during repair","evidence":"new","time":"2026-08-26T07:02:00Z","repair":"required","queue_unchanged":true}
JSON
        chmod 600 "$plan/.state/automation-failures/"*.json
        echo repair >> "$plan/code.txt"
        git -C "$plan" add code.txt
        token=\$(cat "$plan/.state/PAUSE")
        git -C "$plan" commit -qm repair -m "Watchdog-Repair: \$token"
        ;;
      none) ;;
    esac
    echo repair-complete
    ;;
esac
EOF
  chmod +x "$TMP/systemctl" "$TMP/notify" "$TMP/opencode"
}

run_watchdog() {
  local plan=$1 lock=$2
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/opencode" \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
    NOTIFY_SEND="$TMP/notify" WATCHDOG_TEST_MODE=1 LLM_WATCHDOG_MIN_INTERVAL=0 \
    SUPERVISE_TIMEOUT=5 REPAIR_TIMEOUT=5 RESUME_WAIT_LOOPS=1 RESUME_WAIT_SLEEP=0 \
    LLM_WATCHDOG_LOCK="$lock" "$WATCHDOG"
}

case_artifact_forces_repair_and_snapshot() {
  local plan="$TMP/artifact-trigger"
  make_repo "$plan"
  write_failure "$plan" trigger-session
  make_mocks "$plan"
  : > "$TMP/model-calls"
  MOCK_REPAIR_MODE=commit run_watchdog "$plan" "$TMP/artifact-trigger.lock"
  ! grep -q bedlam-llm-watchdog-supervise "$TMP/model-calls"
  grep -q bedlam-llm-watchdog-repair "$TMP/model-calls"
  grep -q 'automation_failures_begin' "$TMP/snapshot-seen"
  grep -q 'trigger-session.*client-error\|client-error.*trigger-session' "$TMP/snapshot-seen"
  grep -Eq 'trigger-session.*(sha256|hash)=|(sha256|hash)=.*trigger-session' "$TMP/snapshot-seen"
  grep -Eq 'trigger-session.*(identity|inode)=|(identity|inode)=.*trigger-session' "$TMP/snapshot-seen"
}

case_failed_repair_retains_artifact() {
  local plan="$TMP/artifact-retain"
  make_repo "$plan"
  write_failure "$plan" retain-session
  make_mocks "$plan"
  MOCK_REPAIR_MODE=none run_watchdog "$plan" "$TMP/artifact-retain.lock"
  [ -e "$plan/.state/automation-failures/retain-session.json" ]
  grep -q '^state=repair-no-evidence$' "$plan/.state/llm-watchdog-verdict"
}

case_verified_repair_archives_artifact() {
  local plan="$TMP/artifact-archive"
  make_repo "$plan"
  write_failure "$plan" archive-session
  make_mocks "$plan"
  MOCK_REPAIR_MODE=commit run_watchdog "$plan" "$TMP/artifact-archive.lock"
  [ ! -e "$plan/.state/automation-failures/archive-session.json" ]
  find "$plan/.state/automation-failures/archive" -type f -name '*archive-session*.json' -print -quit | grep -q .
  grep -q '^state=repaired$' "$plan/.state/llm-watchdog-verdict"
}

case_waiting_without_working_executor_rejected() {
  local plan="$TMP/wait-no-executor"
  make_repo "$plan"
  write_failure "$plan" waiting-session
  make_mocks "$plan"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  MOCK_REPAIR_MODE=waiting run_watchdog "$plan" "$TMP/wait-no-executor.lock"
  grep -q '^state=repair-no-evidence$' "$plan/.state/llm-watchdog-verdict"
  [ -e "$plan/.state/automation-failures/waiting-session.json" ]
}

case_forged_wait_state_cannot_prove_success() {
  local plan="$TMP/forged-wait-state"
  make_repo "$plan"
  write_failure "$plan" forged-session
  make_mocks "$plan"
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
if flock -n "$plan/.state/automatic-waits/.executor.lock" true 2>/dev/null; then
  echo unlocked > "$TMP/forged-probe-lock"
else
  echo locked > "$TMP/forged-probe-lock"
fi
exit 1
EOF
  chmod +x "$plan/tools/probe.sh"
  MOCK_REPAIR_MODE=forged-waiting run_watchdog "$plan" "$TMP/forged-wait.lock"
  [ ! -e "$TMP/forged-probe-lock" ]
  grep -q '^state=repair-no-evidence$' "$plan/.state/llm-watchdog-verdict"
  [ -e "$plan/.state/automation-failures/forged-session.json" ]
}

case_concurrent_or_replaced_failures_are_not_archived() {
  local plan="$TMP/concurrent-failures"
  make_repo "$plan"
  write_failure "$plan" replace-trigger
  make_mocks "$plan"
  MOCK_REPAIR_MODE=concurrent-artifacts MOCK_TRIGGER_SESSION=replace-trigger \
    run_watchdog "$plan" "$TMP/concurrent-failures.lock"
  [ -e "$plan/.state/automation-failures/replace-trigger.json" ]
  [ -e "$plan/.state/automation-failures/concurrent-new.json" ]
  grep -q 'replacement-race' "$plan/.state/automation-failures/replace-trigger.json"
  ! find "$plan/.state/automation-failures/archive" -type f \
      \( -name '*replace-trigger*.json' -o -name '*concurrent-new*.json' \) \
      -print -quit 2>/dev/null | grep -q .
}

case_only_mechanically_resolved_failure_archives() {
  local plan="$TMP/mechanical-resolution"
  make_repo "$plan"
  write_failure "$plan" resolved-current
  cat > "$plan/.state/automation-failures/stale-other.json" <<'EOF'
{"schema":"nudge-failure-v1","version":1,"ordinal":7,"id":"other-task","gate":"other-gate","owner":"worker","session":"stale-other","kind":"client-error","reason":"different task","evidence":"fixture","time":"2026-08-26T07:00:00Z","repair":"required","queue_unchanged":true}
EOF
  chmod 600 "$plan/.state/automation-failures/stale-other.json"
  make_mocks "$plan"
  MOCK_REPAIR_MODE=commit run_watchdog "$plan" "$TMP/mechanical-resolution.lock"
  [ ! -e "$plan/.state/automation-failures/resolved-current.json" ]
  [ -e "$plan/.state/automation-failures/stale-other.json" ]
}

case_systemd_path_beacons_failure_directory() {
  grep -Eq '^Path(Changed|Modified|ExistsGlob)=.*/\.state/automation-failures(/|/\*\.json)?$' \
    "$ROOT/tools/systemd/bedlam-llm-watchdog.path"
}

write_v2() {
  local plan=$1 session=$2 fields status id gate body dev ino queue
  fields=$($ROOT/tools/nudge-free-items.py "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r status id gate body dev ino queue <<< "$fields"
  cat > "$plan/.state/claims/1-$session.claim" <<EOF
lock-v2
ordinal=1
id=stable-one
gate=gate-one
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

case_successful_handoff_is_repaired_behaviorally() {
  local plan="$TMP/legacy-handoff" session=legacy-handoff
  make_repo "$plan"
  write_v2 "$plan" "$session"
  cat > "$TMP/handoff-worker" <<EOF
#!/usr/bin/env bash
echo work >> "$plan/code.txt"
git -C "$plan" add code.txt
git -C "$plan" commit -qm work -m 'Nudge-Worker: $session'
cat > "$plan/.state/NEXT.md" <<'NEXT'
# NEXT

## Now
1. [BLOCKED] [id=stable-one] [gate=gate-one] ask a human to take over

## Backlog
NEXT
exit 0
EOF
  chmod +x "$TMP/handoff-worker"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/handoff-worker" "$AGENT" 1 "$session"
  local worker_rc=$?
  set -e
  [ "$worker_rc" -ne 0 ]
  python3 - "$plan/.state/automation-failures/$session.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["kind"] == "queue-invalid"
assert value["queue_unchanged"] is False
PY
  make_mocks "$plan"
  MOCK_REPAIR_MODE=commit run_watchdog "$plan" "$TMP/legacy-handoff.lock"
  grep -q '^state=repaired$' "$plan/.state/llm-watchdog-verdict"
  [ ! -e "$plan/.state/automation-failures/$session.json" ]
}

run_case 'failure artifact forces repair and is visible in snapshot' case_artifact_forces_repair_and_snapshot
run_case 'failed repair retains its triggering artifact' case_failed_repair_retains_artifact
run_case 'verified repair archives and clears its artifact' case_verified_repair_archives_artifact
run_case 'no-commit waiting state needs a working executor' case_waiting_without_working_executor_rejected
run_case 'watchdog executes probe under lock instead of trusting forged wait JSON' case_forged_wait_state_cannot_prove_success
run_case 'concurrent and replaced failures survive snapshot-scoped archival' case_concurrent_or_replaced_failures_are_not_archived
run_case 'watchdog archives only failures mechanically resolved by repair' case_only_mechanically_resolved_failure_archives
run_case 'legacy BLOCKED/handoff worker output is behaviorally rejected and repaired' case_successful_handoff_is_repaired_behaviorally
run_case 'systemd watchdog path immediately beacons automation failures' case_systemd_path_beacons_failure_directory

if [ "$failures" -ne 0 ]; then
  printf 'automation failure watchdog tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'automation failure watchdog tests: PASS'
