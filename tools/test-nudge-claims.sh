#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
REAPER="$ROOT/tools/nudge-reap-claims.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
TMP=$(mktemp -d /tmp/bedlam-nudge-claims.XXXXXX)
cleanup() {
  jobs -pr | xargs -r kill 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT
cat > "$TMP/mock-notify-send" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/notifications"
EOF
chmod +x "$TMP/mock-notify-send"
export NOTIFY_SEND="$TMP/mock-notify-send"
CLAIMS="$TMP/claims"
LOG="$TMP/nudge.log"
mkdir -p "$CLAIMS"
: > "$LOG"

old() { touch -d "@$(( $(date +%s) - $2 ))" "$1"; }
reap() {
  DEAD_CLAIM_TTL=5 RESERVATION_TTL=5 LEGACY_CLAIM_TTL=20 "$REAPER" "$CLAIMS" "$LOG"
}

# The worker converts its reservation into a locked, marked owner claim.
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims"
echo "# NEXT" > "$PLAN/.state/NEXT.md"
echo "# STATE" > "$PLAN/.state/STATE.md"
echo initial > "$PLAN/code.txt"
git -C "$PLAN" init -q
git -C "$PLAN" config user.email test@example.invalid
git -C "$PLAN" config user.name test
git -C "$PLAN" add .state/NEXT.md .state/STATE.md code.txt
git -C "$PLAN" commit -qm init
cat > "$TMP/mock-client" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" > "$TMP/mock-client.args"
echo "4. [P4] [BLOCKED] mock completed" > "$PLAN/.state/NEXT.md"
sleep 1
EOF
chmod +x "$TMP/mock-client"
echo reserved > "$PLAN/.state/claims/4-789.claim"
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-client" "$AGENT" 4 789 &
agent=$!
for _ in $(seq 1 50); do
  grep -q "^lock-v1 " "$PLAN/.state/claims/4-owner.claim" 2>/dev/null && break
  sleep 0.02
done
grep -q "^lock-v1 " "$PLAN/.state/claims/4-owner.claim"
if flock -n "$PLAN/.state/claims/4-owner.claim" true; then
  echo "worker owner claim was not locked" >&2
  exit 1
fi
wait "$agent"
grep -q -- "--standalone" "$TMP/mock-client.args"
grep -q -- "--model zai-coding-plan/glm-5.3" "$TMP/mock-client.args"
grep -q -- "naming worker 789 is YOUR claim" "$TMP/mock-client.args"
grep -q -- "operator TUI is supervisory and never blocks work" "$TMP/mock-client.args"
! grep -q -- "release your placeholder" "$TMP/mock-client.args"
[ ! -e "$PLAN/.state/claims/4-owner.claim" ]

# A normal transport failure has no live ghost and retains a retry-backoff claim.
cat > "$TMP/mock-transport" <<EOF
#!/usr/bin/env bash
echo "Error: ECONNRESET: The socket connection was closed unexpectedly"
exit 1
EOF
chmod +x "$TMP/mock-transport"
echo reserved > "$PLAN/.state/claims/5-790.claim"
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-transport" "$AGENT" 5 790
rc=$?
set -e
[ "$rc" -eq 1 ]
[ -e "$PLAN/.state/claims/5-owner.claim" ]
flock -n "$PLAN/.state/claims/5-owner.claim" true
grep -q "failed \[transport rc=1 progress=0\] task=.*; provider-side, not charged to the task" "$PLAN/.state/nudge.log"
transport_hash=$(sed -n "s/^[[:space:]]*5\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
[ ! -e "$PLAN/.state/taskfails/$transport_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$transport_hash" ]
touch -d "10 seconds ago" "$PLAN/.state/claims/5-owner.claim"
DEAD_CLAIM_TTL=0 "$REAPER" "$PLAN/.state/claims" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/claims/5-owner.claim" ]

# A transport failure with a surviving child retains its locked ghost claim.
cat > "$TMP/mock-ghost" <<EOF
#!/usr/bin/env bash
sleep 30 &
echo \$! > "$TMP/ghost.pid"
echo "Error: Transport"
exit 1
EOF
chmod +x "$TMP/mock-ghost"
echo reserved > "$PLAN/.state/claims/6-791.claim"
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-ghost" "$AGENT" 6 791
rc=$?
set -e
[ "$rc" -eq 1 ]
[ -e "$PLAN/.state/claims/6-owner.claim" ]
if flock -n "$PLAN/.state/claims/6-owner.claim" true; then
  echo "ghost owner claim was not locked" >&2
  exit 1
fi
kill "$(cat "$TMP/ghost.pid")"
for _ in $(seq 1 100); do
  flock -n "$PLAN/.state/claims/6-owner.claim" true 2>/dev/null && break
  sleep 0.02
done
flock -n "$PLAN/.state/claims/6-owner.claim" true
touch -d "10 seconds ago" "$PLAN/.state/claims/6-owner.claim"
DEAD_CLAIM_TTL=0 "$REAPER" "$PLAN/.state/claims" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/claims/6-owner.claim" ]

# A clean client with no substantive commit is a failed no-progress run.
# Transport failures no longer charge the task (items 5/6 above), so the
# three-strike notification + cooldown now needs three genuinely
# task-attributable failures; the first strike alone must stay silent.
cat > "$TMP/mock-no-progress" <<EOF
#!/usr/bin/env bash
echo other-worker >> "$PLAN/code.txt"
git -C "$PLAN" add code.txt
git -C "$PLAN" commit -qm other-worker -m "Nudge-Worker: 999"
exit 0
EOF
chmod +x "$TMP/mock-no-progress"
nop_hash=$(sed -n "s/^[[:space:]]*8\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
for nop_slot in 803 812 813; do
  echo reserved > "$PLAN/.state/claims/8-$nop_slot.claim"
  set +e
  BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-no-progress" "$AGENT" 8 "$nop_slot"
  nop_rc=$?
  set -e
  [ "$nop_rc" -eq 0 ]
  [ -e "$PLAN/.state/claims/8-owner.claim" ]
  flock -n "$PLAN/.state/claims/8-owner.claim" true
  rm -f "$PLAN/.state/claims/8-owner.claim"
  grep -q "failed \[no-progress rc=0 progress=0\]" "$PLAN/.state/nudge.log"
  if [ "$nop_slot" = 803 ]; then
    [ "$(cat "$PLAN/.state/taskfails/$nop_hash" 2>/dev/null)" = "1" ]
    ! grep -q "item 8 failed three consecutive" "$TMP/notifications"
  fi
done
grep -q "item 8 failed three consecutive observed runs" "$TMP/notifications"
[ "$(cat "$PLAN/.state/taskfails/$nop_hash")" = "3" ]
[ "$(cat "$PLAN/.state/taskcooldown/$nop_hash")" -gt "$(date +%s)" ]

# A step-cap truncation (opencode2 "Maximum steps" kill, rc=0, no
# commit) is NOT a task failure: no taskfails bookkeeping, no
# cooldown spiral, no "failed [" line - just the truncation note and
# a retained retry-backoff claim (freed by the reaper after the
# backoff TTL, like any failed run).
cat > "$TMP/mock-step-cap" <<EOF
#!/usr/bin/env bash
echo "**Maximum steps for this agent reached - stopping with a text-only summary.**"
exit 0
EOF
chmod +x "$TMP/mock-step-cap"
echo "10. [P4] step-cap mock item" >> "$PLAN/.state/NEXT.md"
stepcap_hash=$(sed -n "s/^[[:space:]]*10\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
echo reserved > "$PLAN/.state/claims/10-805.claim"
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-step-cap" "$AGENT" 10 805
stepcap_rc=$?
set -e
[ "$stepcap_rc" -eq 0 ]
grep -q "item 10 hit the opencode2 step cap \[rc=0 progress=0\] task=$stepcap_hash; treating as truncation, not failure" "$PLAN/.state/nudge.log"
! grep -q "agent item 10 failed \[" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$stepcap_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$stepcap_hash" ]
[ -e "$PLAN/.state/claims/10-owner.claim" ]
flock -n "$PLAN/.state/claims/10-owner.claim" true
touch -d "10 seconds ago" "$PLAN/.state/claims/10-owner.claim"
DEAD_CLAIM_TTL=0 "$REAPER" "$PLAN/.state/claims" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/claims/10-owner.claim" ]

# The 2026-08-20 provider-incident signature (opencode2 dying on an
# unparseable provider stream event) is classified transport - not
# client-error - and is never charged to the task.
cat > "$TMP/mock-stream-invalid" <<EOF
#!/usr/bin/env bash
echo "Error: Invalid zai-coding-plan/openai-compatible-chat stream event"
exit 1
EOF
chmod +x "$TMP/mock-stream-invalid"
echo "11. [P4] stream-incident mock item" >> "$PLAN/.state/NEXT.md"
stream_hash=$(sed -n "s/^[[:space:]]*11\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
echo reserved > "$PLAN/.state/claims/11-820.claim"
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-stream-invalid" "$AGENT" 11 820
stream_rc=$?
set -e
[ "$stream_rc" -eq 1 ]
grep -q "failed \[transport rc=1 progress=0\] task=$stream_hash; provider-side, not charged to the task" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$stream_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$stream_hash" ]
[ -e "$PLAN/.state/claims/11-owner.claim" ]
flock -n "$PLAN/.state/claims/11-owner.claim" true
touch -d "10 seconds ago" "$PLAN/.state/claims/11-owner.claim"
DEAD_CLAIM_TTL=0 "$REAPER" "$PLAN/.state/claims" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/claims/11-owner.claim" ]

# A substantive commit is credited only with this wrappers exact trailer.
cat > "$TMP/mock-own-progress" <<EOF
#!/usr/bin/env bash
echo own-worker >> "$PLAN/code.txt"
git -C "$PLAN" add code.txt
git -C "$PLAN" commit -qm own-worker -m "Nudge-Worker: 804"
EOF
chmod +x "$TMP/mock-own-progress"
echo reserved > "$PLAN/.state/claims/9-804.claim"
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-own-progress" "$AGENT" 9 804
[ ! -e "$PLAN/.state/claims/9-owner.claim" ]
grep -q "item 9 ended cleanly (rc=0 progress=1)" "$PLAN/.state/nudge.log"

# Canonical owner publication is atomic: exactly one same-item client starts.
cat > "$TMP/mock-race" <<EOF
#!/usr/bin/env bash
echo started >> "$TMP/race.starts"
echo "7. [P4] [BLOCKED] mock race completed" > "$PLAN/.state/NEXT.md"
sleep 1
EOF
chmod +x "$TMP/mock-race"
echo reserved > "$PLAN/.state/claims/7-801.claim"
echo reserved > "$PLAN/.state/claims/7-802.claim"
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-race" "$AGENT" 7 801 & a=$!
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-race" "$AGENT" 7 802 & b=$!
wait "$a"; ar=$?
wait "$b"; br=$?
set -e
[ "$(wc -l < "$TMP/race.starts")" -eq 1 ]
{ [ "$ar" -eq 0 ] && [ "$br" -eq 75 ]; } || { [ "$ar" -eq 75 ] && [ "$br" -eq 0 ]; }
[ ! -e "$PLAN/.state/claims/7-owner.claim" ]

# An abandoned startup reservation expires quickly.
echo reserved > "$CLAIMS/1-123.claim"
old "$CLAIMS/1-123.claim" 6
reap
[ ! -e "$CLAIMS/1-123.claim" ]

# A lock-v1 owner remains claimed regardless of age while its lock is live.
echo "lock-v1 worker test owns queue item 2" > "$CLAIMS/2-owner.claim"
(
  exec 8>>"$CLAIMS/2-owner.claim"
  flock 8
  : > "$TMP/locked"
  sleep 30
) &
locker=$!
for _ in $(seq 1 50); do [ -e "$TMP/locked" ] && break; sleep 0.02; done
[ -e "$TMP/locked" ]
old "$CLAIMS/2-owner.claim" 60
reap
[ -e "$CLAIMS/2-owner.claim" ]
[ $(( $(date +%s) - $(stat -c %Y "$CLAIMS/2-owner.claim") )) -le 2 ]

# Once its worker dies, the same claim expires after the dead-worker grace.
kill "$locker"
wait "$locker" 2>/dev/null || true
old "$CLAIMS/2-owner.claim" 4
reap
[ -e "$CLAIMS/2-owner.claim" ]
old "$CLAIMS/2-owner.claim" 6
reap
[ ! -e "$CLAIMS/2-owner.claim" ]

# Pre-lock claims retain the conservative migration timeout.
echo "worker legacy owns queue item 3" > "$CLAIMS/3-owner.claim"
old "$CLAIMS/3-owner.claim" 6
reap
[ -e "$CLAIMS/3-owner.claim" ]
old "$CLAIMS/3-owner.claim" 21
reap
[ ! -e "$CLAIMS/3-owner.claim" ]

# The real operating contract must not regress to TUI/process ownership.
grep -q "Process liveness is NEVER ownership evidence" "$ROOT/AGENTS.md"
grep -q "Never make a commit whose only effect is a stand-down/status journal" "$ROOT/AGENTS.md"
! grep -q "read them first" "$ROOT/.state/NEXT.md"
! grep -q "release your placeholder" "$AGENT"
# v5: working-tree mtimes are no longer progress evidence.
! grep -q "newermt" "$ROOT/tools/nudge.sh"
grep -q "taskfails" "$ROOT/tools/nudge-agent.sh"

echo "nudge claim tests: PASS"
