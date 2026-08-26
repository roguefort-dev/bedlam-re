#!/usr/bin/env bash
# Recover Bedlam autonomy after an internet outage or poisoned OpenCode response.
set -u

PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
CURL_BIN=${CURL_BIN:-curl}
OFFLINE="$STATE/network-offline"
LAST_RECOVERY="$STATE/network-last-recovery"
LOG="$STATE/network-watchdog.log"
LOCK=${NETWORK_WATCHDOG_LOCK:-/tmp/bedlam-network-watchdog.lock}
LOCK_HELPER="$SCRIPT_DIR/nudge-lock.py"
STATE_HELPER="$SCRIPT_DIR/nudge-state.py"

if [ "${NETWORK_WATCHDOG_LOCK_HELD:-0}" != 1 ]; then
  exec "$LOCK_HELPER" lock-run "$LOCK" nonblocking \
    env NETWORK_WATCHDOG_LOCK_HELD=1 "$0" "$@"
fi
"$STATE_HELPER" ensure-dir "$STATE" >/dev/null 2>&1 || exit 75
log_line() { "$STATE_HELPER" append-text "$LOG" "$(date -Is) $*"$'\n' 2>/dev/null || true; }

network_ok() {
  "$CURL_BIN" -fsS --connect-timeout 3 --max-time 6 -o /dev/null https://api.github.com     || "$CURL_BIN" -fsS --connect-timeout 3 --max-time 6 -o /dev/null https://opencode.ai/v2/docs/
}

now=$(date +%s)
if ! network_ok; then
  if [ ! -e "$OFFLINE" ]; then
    "$STATE_HELPER" create-text "$OFFLINE" "$now"$'\n' 2>/dev/null || exit 75
    log_line "connectivity lost; waiting without restarting services"
  fi
  exit 75
fi

reason=
if [ -e "$OFFLINE" ]; then
  since=$("$STATE_HELPER" read-text "$OFFLINE" 2>/dev/null || echo unknown)
  "$STATE_HELPER" unlink "$OFFLINE" 2>/dev/null || exit 75
  reason="network restored (offline marker $since)"
else
  if [ -e "$LAST_RECOVERY" ]; then
    latest_bad=$(find "$STATE" -maxdepth 1 -type f -name "agent-*.log"       -newer "$LAST_RECOVERY" -exec grep -Il "UnsupportedContentType" {} + 2>/dev/null       | head -1)
  else
    latest_bad=$(find "$STATE" -maxdepth 1 -type f -name "agent-*.log"       -exec grep -Il "UnsupportedContentType" {} + 2>/dev/null | head -1)
  fi
  if [ -n "$latest_bad" ]; then
    reason="OpenCode UnsupportedContentType in $(basename "$latest_bad")"
  fi
fi

[ -n "$reason" ] || exit 0
"$STATE_HELPER" write-text "$LAST_RECOVERY" "$now $reason"$'\n' 2>/dev/null || exit 75
log_line "recovery started: $reason"

# Workers run with --standalone, so recovery is a fresh private connection.
# Never restart the shared OpenCode service here: doing so interrupts unrelated
# interactive sessions and is unnecessary for autonomous retries.

# A failed worker is gone, so an unlocked versioned claim can be released now
# rather than waiting for the normal five-minute grace. Locked live claims are
# refreshed and retained by the same reaper.
if [ -x "$SCRIPT_DIR/nudge-reap-claims.sh" ]; then
  DEAD_CLAIM_TTL=0 RESERVATION_TTL=0     "$SCRIPT_DIR/nudge-reap-claims.sh" "$STATE/claims" "$STATE/nudge.log"
fi

# Bypass the ordinary quiet-period gate. The caller is the existing
# bedlam-nudge timer, which continues through its normal claim/concurrency
# gates and launches a fresh standalone worker when safe.
"$STATE_HELPER" touch "$STATE/heartbeat" 0 2>/dev/null || exit 75
log_line "recovery complete; current nudge pass may resume"
