#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WATCHDOG="$ROOT/tools/network-watchdog.sh"
TMP=$(mktemp -d /tmp/bedlam-network-watchdog.XXXXXX)
cleanup() {
  jobs -pr | xargs -r kill 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT
export MOCK_STATE="$TMP/mock-state"
PLAN="$TMP/plan"
BIN="$TMP/bin"
mkdir -p "$PLAN/.state/claims" "$BIN" "$MOCK_STATE"
echo "# NEXT" > "$PLAN/.state/NEXT.md"
echo "# STATE" > "$PLAN/.state/STATE.md"
echo up > "$MOCK_STATE/network"

cat > "$BIN/curl" <<"EOF"
#!/usr/bin/env bash
[ "$(cat "$MOCK_STATE/network")" = up ]
EOF
chmod +x "$BIN/curl"

watch() {
  BEDLAM_PLAN_DIR="$PLAN" CURL_BIN="$BIN/curl"     NETWORK_WATCHDOG_LOCK="$TMP/watchdog.lock" "$WATCHDOG"
}

# Offline checks mark state but do not restart OpenCode.
echo down > "$MOCK_STATE/network"
set +e
watch
rc=$?
set -e
[ "$rc" -eq 75 ]
[ -f "$PLAN/.state/network-offline" ]
set +e
watch
rc=$?
set -e
[ "$rc" -eq 75 ]
[ "$(grep -c "connectivity lost" "$PLAN/.state/network-watchdog.log")" -eq 1 ]

# The first online pass restarts once and primes immediate nudge continuation.
echo "lock-v1 worker test owns queue item 9" > "$PLAN/.state/claims/9-owner.claim"
(
  exec 8>>"$PLAN/.state/claims/9-owner.claim"
  flock 8
  : > "$MOCK_STATE/claim-locked"
  sleep 30
) &
locker=$!
for _ in $(seq 1 50); do [ -e "$MOCK_STATE/claim-locked" ] && break; sleep 0.02; done
[ -e "$MOCK_STATE/claim-locked" ]
touch -d "10 seconds ago" "$PLAN/.state/claims/9-owner.claim"
echo up > "$MOCK_STATE/network"
watch
[ -e "$PLAN/.state/claims/9-owner.claim" ]
[ ! -e "$PLAN/.state/network-offline" ]
[ "$(grep -c "recovery started" "$PLAN/.state/network-watchdog.log")" -eq 1 ]
[ "$(stat -c %Y "$PLAN/.state/heartbeat")" -eq 0 ]
watch
[ "$(grep -c "recovery started" "$PLAN/.state/network-watchdog.log")" -eq 1 ]

kill "$locker"
wait "$locker" 2>/dev/null || true
touch -d "2 seconds ago" "$PLAN/.state/claims/9-owner.claim"

# A fresh poisoned response also causes exactly one recovery while online.
touch -d "2 seconds ago" "$PLAN/.state/network-last-recovery"
echo "Error: UnsupportedContentType" > "$PLAN/.state/agent-test.log"
watch
[ "$(grep -c "recovery started" "$PLAN/.state/network-watchdog.log")" -eq 2 ]
[ ! -e "$PLAN/.state/claims/9-owner.claim" ]
watch
[ "$(grep -c "recovery started" "$PLAN/.state/network-watchdog.log")" -eq 2 ]

echo "network watchdog tests: PASS"
