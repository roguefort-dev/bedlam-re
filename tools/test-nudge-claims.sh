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
cat > "$TMP/mock-client" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" > "$TMP/mock-client.args"
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
echo "Error: Transport"
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

# Canonical owner publication is atomic: exactly one same-item client starts.
cat > "$TMP/mock-race" <<EOF
#!/usr/bin/env bash
echo started >> "$TMP/race.starts"
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
grep -q "state-only stand-down commits forbidden" "$ROOT/.state/NEXT.md"
! grep -q "read them first" "$ROOT/.state/NEXT.md"
! grep -q "release your placeholder" "$AGENT"
[ "$(wc -l < "$ROOT/.state/NEXT.md")" -lt 200 ]

echo "nudge claim tests: PASS"
