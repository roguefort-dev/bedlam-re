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
[ ! -e "$PLAN/.state/claims/4-owner.claim" ]

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

echo "nudge claim tests: PASS"
