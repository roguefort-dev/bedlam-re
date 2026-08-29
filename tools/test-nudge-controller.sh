#!/usr/bin/env bash
# Controller-level integration tests: exercise the real nudge.sh -> systemd-run
# -> nudge-agent.sh -> model-client path with injected fakes.
set -Eeuo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d /tmp/bedlam-nudge-controller.XXXXXX)
cleanup() {
  if [ -f "$TMP/agent.pgids" ]; then
    while read -r pg; do kill -TERM -- "-$pg" 2>/dev/null || true; done < "$TMP/agent.pgids"
    sleep 0.5
    while read -r pg; do kill -KILL -- "-$pg" 2>/dev/null || true; done < "$TMP/agent.pgids"
  fi
  jobs -pr | xargs -r kill 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT
trap 'rc=$?; if [[ $- == *e* ]]; then printf "not ok - nudge controller test failed at line %s (rc=%s): %s\n" "$LINENO" "$rc" "$BASH_COMMAND" >&2; exit "$rc"; fi' ERR
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims"

cat > "$TMP/mock-notify-send" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/notifications"
EOF
chmod +x "$TMP/mock-notify-send"

# Fake systemd-run: async-launch the real agent script like a transient unit.
cat > "$TMP/mock-systemd-run" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/run-calls"
if [ "\${MOCK_RUN_FAIL:-0}" = 1 ]; then echo "mock systemd-run failure" >&2; exit 1; fi
set -- "\${@: -3}"
script=\$1 item=\$2 slot=\$3
cp "$PLAN/.state/claims/\$item-\$slot.claim" "$TMP/launched-claim"
setsid "\$script" "\$item" "\$slot" >> "$TMP/agent-console.log" 2>&1 &
echo \$! >> "$TMP/agent.pgids"
EOF
chmod +x "$TMP/mock-systemd-run"
mkdir -p "$TMP/mock-bin"
ln -s "$TMP/mock-systemd-run" "$TMP/mock-bin/systemd-run"

cat > "$TMP/mock-network-watchdog" <<EOF
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$TMP/mock-network-watchdog"

make_plan() {
  rm -rf "$PLAN"
  mkdir -p "$PLAN/.state/claims"
  printf "# NEXT\n\n## Now\n1. [READY] [id=controller-test] [gate=p5-controller] controller test task\n\n## Backlog\n" > "$PLAN/.state/NEXT.md"
  printf "# AGENTS\n" > "$PLAN/AGENTS.md"
  printf "initial\n" > "$PLAN/code.txt"
  git -C "$PLAN" init -q
  git -C "$PLAN" config user.email test@example.invalid
  git -C "$PLAN" config user.name test
  git -C "$PLAN" add .state/NEXT.md AGENTS.md code.txt
  git -C "$PLAN" commit -qm init
}

# Model client fake: performs an attributed substantive commit.
cat > "$TMP/mock-client" <<EOF
#!/usr/bin/env bash
slot=\$(printf "%s\n" "\$*" | sed -nE "s/.*for slot ([0-9A-Za-z-]+) .*/\1/p" | head -n 1)
if [ "\${MOCK_CLIENT_FAIL:-0}" = 1 ]; then exit 127; fi
echo "work by \$slot" >> "$PLAN/code.txt"
git -C "$PLAN" add code.txt
git -C "$PLAN" commit -qm "work" -m "Nudge-Worker: \$slot"
EOF
chmod +x "$TMP/mock-client"

# Chain-spawn recorder: nudge-agent fires one instant nudge pass on a clean
# end (event-driven coordination). Records the call; never touches systemd.
cat > "$TMP/mock-systemctl-chain" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/chain-calls"
EOF
chmod +x "$TMP/mock-systemctl-chain"

run_nudge() {
  BEDLAM_PLAN_DIR="$PLAN" NUDGE_LOCK="$TMP/nudge.lock" \
  OPENC_OVERRIDE="$TMP/mock-client" \
  NETWORK_WATCHDOG_OVERRIDE="$TMP/mock-network-watchdog" \
  SYSTEMD_RUN_OVERRIDE="$TMP/mock-systemd-run" \
  SYSTEMCTL_OVERRIDE="$TMP/mock-systemctl-chain" \
  REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" \
  NOTIFY_SEND="$TMP/mock-notify-send" \
    "$ROOT/tools/nudge.sh"
}

run_nudge_with_production_idle() {
  env -u SYSTEMD_RUN_OVERRIDE \
    PATH="$TMP/mock-bin:$PATH" \
    BEDLAM_PLAN_DIR="$PLAN" NUDGE_LOCK="$TMP/nudge.lock" \
    OPENC_OVERRIDE="$TMP/mock-client" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/mock-network-watchdog" \
    SYSTEMCTL_OVERRIDE="$TMP/mock-systemctl-chain" \
    REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" \
    NOTIFY_SEND="$TMP/mock-notify-send" \
    "$ROOT/tools/nudge.sh"
}

taskhash() {
  sed -n "s/^[[:space:]]*$1\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16
}

wait_agent_done() {
  for _ in $(seq 1 200); do
    grep -q "ended cleanly\|failed \[" "$PLAN/.state/nudge.log" 2>/dev/null && return 0
    sleep 0.05
  done
  return 1
}

# 1. Happy path: stale heartbeat + free item -> spawn -> attributed commit ->
#    claim released.
make_plan
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
: > "$TMP/run-calls"
run_nudge
grep -q -- "--unit bedlam-nudge-item1-" "$TMP/run-calls"
grep -q "spawning agent for queue item 1 as unit bedlam-nudge-item1-" "$PLAN/.state/nudge.log"
# Every newly published reservation/owner claim uses lock-v2 and binds the
# scheduler's mutable ordinal to the queue's stable identity before launch.
# Keep these assertions key-oriented so field ordering remains an
# implementation detail.
if ! grep -qx "lock-v2" "$TMP/launched-claim"; then
  echo "not ok - controller launched from a non-lock-v2 reservation" >&2
  cat "$TMP/launched-claim" >&2
  exit 1
fi
grep -qx "ordinal=1" "$TMP/launched-claim"
grep -qx "id=controller-test" "$TMP/launched-claim"
grep -qx "gate=p5-controller" "$TMP/launched-claim"
grep -qx "owner=worker" "$TMP/launched-claim"
grep -Eq '^session=[0-9a-f-]+$' "$TMP/launched-claim"
grep -Eq '^claimed_at=[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}([+-][0-9]{2}:[0-9]{2}|Z)$' "$TMP/launched-claim"
grep -Eq '^unit=bedlam-nudge-item1-[0-9a-f-]+$' "$TMP/launched-claim"
! grep -q '^lock-v1 ' "$TMP/launched-claim"
wait_agent_done
grep -q "ended cleanly (rc=0 progress=1)" "$PLAN/.state/nudge.log"
git -C "$PLAN" log -1 --format=%B | grep -qE "^Nudge-Worker: [0-9a-f-]+$"
[ ! -e "$PLAN/.state/claims/1-owner.claim" ]
[ -z "$(ls "$PLAN/.state/claims")" ]
grep -q -- "start bedlam-nudge.service" "$TMP/chain-calls"
[ "$(stat -c %Y "$PLAN/.state/heartbeat")" -lt "$(( $(date +%s) - 3600 ))" ]
[ ! -e "$PLAN/.state/taskfails/$(taskhash 1)" ]

# 2. Second pass while the item is claimed and locked: stand down.
(
  printf "reserved\n" > "$PLAN/.state/claims/1-owner.claim"
  echo "lock-v1 worker someone owns queue item 1" >> "$PLAN/.state/claims/1-owner.claim"
  exec 8>>"$PLAN/.state/claims/1-owner.claim"
  flock 8
  sleep 60
) &
holder=$!
# Seed a stale higher concurrency value: the clamp must pin it back to 1, so
# the stand-down message reports 1/1 (unclamped it would be "no unattended Now
# items" because the gate would pass 1 < 3).
printf "1\n" > "$PLAN/.state/concurrency"
before=$(wc -l < "$TMP/run-calls")
run_nudge
[ "$(wc -l < "$TMP/run-calls")" -eq "$before" ]
# Concurrency is pinned at 1, so a live claim trips the concurrency gate before
# the free-items scan: the stand-down message is "concurrency full", not
# "no unattended Now items".
grep -q "concurrency full (1/1 agents, adaptive) - standing down" "$PLAN/.state/nudge.log"
kill "$holder" 2>/dev/null || true
wait "$holder" 2>/dev/null || true
rm -f "$PLAN/.state/claims/1-owner.claim"

# 3. A human PAUSE stops the controller before any spawn.
printf "human pause\n" > "$PLAN/.state/PAUSE"
before=$(wc -l < "$TMP/run-calls")
run_nudge
[ "$(wc -l < "$TMP/run-calls")" -eq "$before" ]
rm -f "$PLAN/.state/PAUSE"

# 3b. A stranded watchdog-owned PAUSE (dead pid, e.g. reboot mid-repair)
# rings the supervisor bell (event-driven recovery) and spawns nothing.
: > "$TMP/chain-calls"
printf "llm-watchdog 999999 1000\n" > "$PLAN/.state/PAUSE"
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
run_nudge
grep -q "watchdog-owned PAUSE stranded (pid=999999)" "$PLAN/.state/nudge.log"
grep -q -- "start bedlam-llm-watchdog.service" "$TMP/chain-calls"
! grep -q "spawning agent" "$PLAN/.state/nudge.log"
rm -f "$PLAN/.state/PAUSE"

# 4. Legacy taskcooldown state is not scheduler truth and cannot hide work.
th=$(taskhash 1)
mkdir -p "$PLAN/.state/taskcooldown"
echo $(( $(date +%s) + 600 )) > "$PLAN/.state/taskcooldown/$th"
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
: > "$PLAN/.state/nudge.log"
before=$(wc -l < "$TMP/run-calls")
run_nudge
wait_agent_done
[ "$(wc -l < "$TMP/run-calls")" -eq "$((before + 1))" ]
rm -f "$PLAN/.state/taskcooldown/$th"

# 5. systemd-run failure drops the reservation instead of leaking it.
MOCK_RUN_FAIL=1 run_nudge
grep -q "systemd-run failed for unit" "$PLAN/.state/nudge.log"
grep -q "dropping reservation" "$PLAN/.state/nudge.log"
[ -z "$(ls "$PLAN/.state/claims")" ]

# 6. Client crash (rc=127) is charged to the task, claim retained for retry.
: > "$PLAN/.state/nudge.log"
MOCK_CLIENT_FAIL=1 run_nudge
wait_agent_done
grep -q "failed \[client-error rc=127 progress=0\]" "$PLAN/.state/nudge.log"
[ -e "$PLAN/.state/taskfails/$th" ]
[ "$(cat "$PLAN/.state/taskfails/$th")" = "1" ]
[ ! -e "$PLAN/.state/claims/1-owner.claim" ]
grep -q "released item 1 claim for immediate retry" "$PLAN/.state/nudge.log"
rm -f "$PLAN/.state/claims/1-owner.claim"

# 7. Provider quota exhaustion is rate-limit, even with the capital-U
#    "Usage limit reached" spelling: no taskfails charge, no hidden cooldown,
#    and a structured automation-failure artifact.
: > "$PLAN/.state/nudge.log"
rm -f "$PLAN/.state/taskfails/$th" "$PLAN/.state/taskcooldown/$th"
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
cat > "$TMP/mock-client" <<'EOF'
#!/usr/bin/env bash
echo "Error: Usage limit reached for 5 hour. Your limit will reset at $(date -d '+2 hours' '+%Y-%m-%d %H:%M:%S')"
exit 1
EOF
chmod +x "$TMP/mock-client"
run_nudge
wait_agent_done
grep -q "failed \[rate-limit rc=1 progress=0\]" "$PLAN/.state/nudge.log"
grep -q "provider quota, not charged to the task" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$th" ]
[ ! -e "$PLAN/.state/taskcooldown/$th" ]
find "$PLAN/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit | grep -q .
rm -f "$PLAN/.state/taskcooldown/$th" "$PLAN/.state/claims/1-owner.claim"

# 8. A reset stamp ~6h out likewise cannot create hidden scheduler state.
: > "$PLAN/.state/nudge.log"
rm -f "$PLAN/.state/taskfails/$th" "$PLAN/.state/taskcooldown/$th"
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
cat > "$TMP/mock-client" <<'EOF'
#!/usr/bin/env bash
echo "Error: Usage limit reached for 5 hour. Your limit will reset at $(date -d '+6 hours' '+%Y-%m-%d %H:%M:%S')"
exit 1
EOF
chmod +x "$TMP/mock-client"
run_nudge
wait_agent_done
grep -q "failed \[rate-limit rc=1 progress=0\]" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$th" ]
[ ! -e "$PLAN/.state/taskcooldown/$th" ]
find "$PLAN/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit | grep -q .
rm -f "$PLAN/.state/taskcooldown/$th" "$PLAN/.state/claims/1-owner.claim"

# 9. Invalid active queues are deadlocks, not idle queues. The parser's
# nonzero status must reach the caller without spawning or notifying.
for invalid_case in blocked untagged; do
  make_plan
  case "$invalid_case" in
    blocked)
      printf '# NEXT\n\n## Now\n1. [BLOCKED - unattended] [id=blocked-controller] [gate=blocked-controller-gate] blocked task\n## Backlog\n' > "$PLAN/.state/NEXT.md"
      ;;
    untagged)
      printf '# NEXT\n\n## Now\n1. untagged controller task\n## Backlog\n' > "$PLAN/.state/NEXT.md"
      ;;
  esac
  : > "$TMP/run-calls"
  rm -f "$TMP/notifications"
  set +e
  run_nudge_with_production_idle
  invalid_rc=$?
  set -e
  [ "$invalid_rc" -eq 2 ]
  [ ! -s "$TMP/run-calls" ]
  [ ! -s "$TMP/notifications" ]
  [ ! -e "$PLAN/.state/idle-notified" ]
  grep -q "queue INVALID-DEADLOCKED" "$PLAN/.state/nudge.log"
  grep -q "repair required; refusing idle/spawn" "$PLAN/.state/nudge.log"
  ! grep -q "idle: no spawnable work" "$PLAN/.state/nudge.log"
  ! grep -q "spawning agent" "$PLAN/.state/nudge.log"
done

# 10. An empty required queue without a cryptographically valid completion
# artifact is an automation failure, never an idle/operator handoff.
make_plan
printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$PLAN/.state/NEXT.md"
: > "$TMP/run-calls"
: > "$TMP/chain-calls"
: > "$TMP/notifications"
rm -rf "$PLAN/.state/automation-failures"
rm -f "$PLAN/.state/idle-notified"
set +e
run_nudge_with_production_idle
empty_rc=$?
set -e
empty_failures=0
if [ "$empty_rc" -eq 0 ]; then
  echo 'not ok - empty required queue returned idle success instead of structured failure' >&2
  empty_failures=$((empty_failures + 1))
fi
if [ -s "$TMP/run-calls" ]; then
  echo 'not ok - empty required queue launched a worker' >&2
  empty_failures=$((empty_failures + 1))
fi
if [ -s "$TMP/notifications" ] || [ -e "$PLAN/.state/idle-notified" ]; then
  echo 'not ok - empty required queue entered operator/idle notification path' >&2
  empty_failures=$((empty_failures + 1))
fi
empty_artifact=$(find "$PLAN/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit 2>/dev/null || true)
if [ -z "$empty_artifact" ]; then
  echo 'not ok - empty required queue emitted no structured automatic-repair artifact' >&2
  empty_failures=$((empty_failures + 1))
else
  if ! python3 - "$empty_artifact" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["kind"] in {"completion-missing", "required-queue-empty"}
assert value["repair"] == "required"
PY
  then
    echo 'not ok - empty queue repair artifact has the wrong structured reason' >&2
    empty_failures=$((empty_failures + 1))
  fi
fi
if ! grep -q -- 'start bedlam-llm-watchdog.service' "$TMP/chain-calls"; then
  echo 'not ok - empty required queue did not invoke automatic watchdog repair' >&2
  empty_failures=$((empty_failures + 1))
fi
if [ "$empty_failures" -ne 0 ]; then
  printf 'nudge controller tests: RED (empty-queue repair: %d failed assertions)\n' "$empty_failures" >&2
  exit 1
fi

# 11. A completion-basis change during sealed validation is a benign retry,
# never a structured failure: a commit landing mid-run withholds the atomic
# verdict and the next tick re-validates the new HEAD (the D234 livelock fix).
make_plan
mkdir -p "$PLAN/tools" "$PLAN/docs"
basis_token="bedlam-basis-race-$(basename "$TMP")"
cat > "$PLAN/tools/validate-required-gates.py" <<EOF
#!/usr/bin/env python3
import os, pathlib, time
pathlib.Path("/tmp/opencode/$basis_token.marker").write_text("running", encoding="utf-8")
for _ in range(1200):
    if os.path.exists("/tmp/opencode/$basis_token.release"):
        break
    time.sleep(0.05)
EOF
chmod +x "$PLAN/tools/validate-required-gates.py"
: > "$PLAN/MANIFEST.sha256"
printf '# fixture gates manifest\n' > "$PLAN/docs/required-gates.toml"
git -C "$PLAN" add MANIFEST.sha256 docs tools
git -C "$PLAN" commit -qm 'fixture: sealed validator stub'
printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$PLAN/.state/NEXT.md"
# The fixture validator and the completion staging root both live in the
# host tmpfs /tmp/opencode: a reboot (2026-08-29 22:57) can wipe it before
# this suite runs, so ensure the shared root exists before the markers.
mkdir -p /tmp/opencode
rm -f "/tmp/opencode/$basis_token.marker" "/tmp/opencode/$basis_token.release"
: > "$TMP/run-calls"
: > "$TMP/chain-calls"
rm -rf "$PLAN/.state/automation-failures"
rm -f "$PLAN/.state/idle-notified"
basis_failures=0
set +e
run_nudge_with_production_idle &
basis_nudge_pid=$!
basis_committed=0
for _ in $(seq 1 300); do
  if [ -e "/tmp/opencode/$basis_token.marker" ]; then
    git -C "$PLAN" commit --allow-empty -qm 'race: concurrent HEAD movement'
    basis_committed=1
    break
  fi
  sleep 0.1
done
touch "/tmp/opencode/$basis_token.release"
wait "$basis_nudge_pid"
basis_rc=$?
set -e
if [ "$basis_committed" -ne 1 ]; then
  echo 'not ok - basis-change test never observed the sealed validator running' >&2
  basis_failures=$((basis_failures + 1))
fi
if [ "$basis_rc" -ne 0 ]; then
  echo "not ok - basis-change retry returned rc=$basis_rc instead of benign success" >&2
  basis_failures=$((basis_failures + 1))
fi
if [ -s "$TMP/run-calls" ] || grep -q -- 'start bedlam-llm-watchdog.service' "$TMP/chain-calls"; then
  echo 'not ok - basis-change retry spawned a worker or started the watchdog' >&2
  echo "--- run-calls ---" >&2; cat "$TMP/run-calls" >&2 || true
  echo "--- chain-calls ---" >&2; cat "$TMP/chain-calls" >&2 || true
  basis_failures=$((basis_failures + 1))
fi
basis_artifact=$(find "$PLAN/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit 2>/dev/null || true)
if [ -n "$basis_artifact" ]; then
  echo 'not ok - basis-change retry emitted a structured automatic-repair artifact' >&2
  basis_failures=$((basis_failures + 1))
fi
if ! grep -q "completion basis changed during validation; sealed verdict withheld" "$PLAN/.state/nudge.log"; then
  echo 'not ok - basis-change retry did not log the benign retry line' >&2
  basis_failures=$((basis_failures + 1))
fi
rm -f "/tmp/opencode/$basis_token.marker" "/tmp/opencode/$basis_token.release"
if [ "$basis_failures" -ne 0 ]; then
  printf 'nudge controller tests: RED (basis-change retry: %d failed assertions)\n' "$basis_failures" >&2
  exit 1
fi

# 12. The completion staging root is controller-owned infrastructure: a host
# reboot wipes the tmpfs /tmp (2026-08-29 22:57) and the old hardcoded
# mkdtemp(dir="/tmp/opencode") died with ENOENT on the first post-boot
# completion pass, beaconing completion-missing (watchdog repair 1788037173).
# complete-from-head/accept-completion must recreate the root themselves and
# refuse an unsafe (symlink or non-directory) root instead of staging into it.
scratch_failures=0
if ! python3 - "$ROOT" "$TMP" <<'PY'
import importlib.util
import pathlib
import sys
import tempfile

root, tmp = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
spec = importlib.util.spec_from_file_location(
    "nudge_state", root / "tools" / "nudge-state.py"
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

missing = tmp / "wiped-opencode"
# Pin the defect class first: staging into a wiped root is ENOENT.
try:
    tempfile.mkdtemp(dir=missing)
except FileNotFoundError:
    pass
else:
    raise AssertionError("precondition: mkdtemp into a missing root must ENOENT")

module.COMPLETION_SCRATCH_BASE = missing
base = module.completion_scratch_base()
assert base == missing and base.is_dir(), "helper must recreate the wiped root"
staging = pathlib.Path(tempfile.mkdtemp(prefix="bedlam-completion-", dir=base))
assert staging.is_dir(), "staging must succeed against the recreated root"
assert module.completion_scratch_base() == base, "helper must be idempotent"

refused = tmp / "refused-opencode"
refused.write_text("not a directory", encoding="utf-8")
module.COMPLETION_SCRATCH_BASE = refused
try:
    module.completion_scratch_base()
except ValueError:
    pass
else:
    raise AssertionError("non-directory root must be refused")

linked = tmp / "linked-opencode"
target = tmp / "linked-target"
target.mkdir()
linked.symlink_to(target, target_is_directory=True)
module.COMPLETION_SCRATCH_BASE = linked
try:
    module.completion_scratch_base()
except ValueError:
    pass
else:
    raise AssertionError("symlinked root must be refused")
PY
then
  echo 'not ok - completion scratch root regression (reboot wipe) failed' >&2
  scratch_failures=$((scratch_failures + 1))
fi
if [ "$scratch_failures" -ne 0 ]; then
  printf 'nudge controller tests: RED (completion scratch root: %d failed assertions)\n' "$scratch_failures" >&2
  exit 1
fi

echo "nudge controller tests: PASS"
