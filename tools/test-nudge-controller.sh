#!/usr/bin/env bash
# Controller-level integration tests: exercise the real nudge.sh -> systemd-run
# -> nudge-agent.sh -> model-client path with injected fakes.
set -euo pipefail
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
setsid "\$script" "\$item" "\$slot" >> "$TMP/agent-console.log" 2>&1 &
echo \$! >> "$TMP/agent.pgids"
EOF
chmod +x "$TMP/mock-systemd-run"

cat > "$TMP/mock-network-watchdog" <<EOF
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$TMP/mock-network-watchdog"

make_plan() {
  rm -rf "$PLAN"
  mkdir -p "$PLAN/.state/claims"
  printf "# NEXT\n\n## Now\n1. [P5] controller test task\n\n## Backlog\n" > "$PLAN/.state/NEXT.md"
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

run_nudge() {
  BEDLAM_PLAN_DIR="$PLAN" NUDGE_LOCK="$TMP/nudge.lock" \
  OPENC_OVERRIDE="$TMP/mock-client" \
  NETWORK_WATCHDOG_OVERRIDE="$TMP/mock-network-watchdog" \
  SYSTEMD_RUN_OVERRIDE="$TMP/mock-systemd-run" \
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
wait_agent_done
grep -q "ended cleanly (rc=0 progress=1)" "$PLAN/.state/nudge.log"
git -C "$PLAN" log -1 --format=%B | grep -qE "^Nudge-Worker: [0-9a-f-]+$"
[ ! -e "$PLAN/.state/claims/1-owner.claim" ]
[ -z "$(ls "$PLAN/.state/claims")" ]
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
printf "3\n" > "$PLAN/.state/concurrency"
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

# 4. A cooling-down task is not spawned; other free items still are.
th=$(taskhash 1)
mkdir -p "$PLAN/.state/taskcooldown"
echo $(( $(date +%s) + 600 )) > "$PLAN/.state/taskcooldown/$th"
touch -d "10 minutes ago" "$PLAN/.state/heartbeat"
run_nudge
grep -q "all free items are cooling down after failures - standing down" "$PLAN/.state/nudge.log"
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
[ -e "$PLAN/.state/claims/1-owner.claim" ]
flock -n "$PLAN/.state/claims/1-owner.claim" true
rm -f "$PLAN/.state/claims/1-owner.claim"

# 7. Provider quota exhaustion is rate-limit, even with the capital-U
#    "Usage limit reached" spelling (regression, watchdog repair
#    2026-08-21): no taskfails charge, cooldown from the provider's own
#    reset stamp but capped at one probe interval (1800s).
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
[ -e "$PLAN/.state/taskcooldown/$th" ]
until=$(cat "$PLAN/.state/taskcooldown/$th")
now=$(date +%s)
[ "$until" -gt "$now" ]
[ "$until" -le $(( now + 1805 )) ]
rm -f "$PLAN/.state/taskcooldown/$th" "$PLAN/.state/claims/1-owner.claim"

# 8. A reset stamp ~6h out (inside the old 6h sanity bound - the
#    2026-08-21 ~07:45 incident shape: the provider resumed serving
#    ~5h49m before its own stamp) must not blind the loop for hours:
#    the armed cooldown is still capped at now+1800.
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
[ -e "$PLAN/.state/taskcooldown/$th" ]
until=$(cat "$PLAN/.state/taskcooldown/$th")
now=$(date +%s)
[ "$until" -gt "$now" ]
[ "$until" -le $(( now + 1805 )) ]
rm -f "$PLAN/.state/taskcooldown/$th" "$PLAN/.state/claims/1-owner.claim"

echo "nudge controller tests: PASS"
