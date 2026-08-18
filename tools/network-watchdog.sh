#!/usr/bin/env bash
# Recover Bedlam autonomy after an internet outage or poisoned OpenCode response.
set -u

PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
CURL_BIN=${CURL_BIN:-curl}
OPENC_BIN=${OPENC_BIN:-opencode2}
OFFLINE="$STATE/network-offline"
LAST_RECOVERY="$STATE/network-last-recovery"
LOG="$STATE/network-watchdog.log"
LOCK=${NETWORK_WATCHDOG_LOCK:-/tmp/bedlam-network-watchdog.lock}

mkdir -p "$STATE"
exec 9>"$LOCK"
flock -n 9 || exit 0

network_ok() {
  "$CURL_BIN" -fsS --connect-timeout 3 --max-time 6 -o /dev/null https://api.github.com     || "$CURL_BIN" -fsS --connect-timeout 3 --max-time 6 -o /dev/null https://opencode.ai/v2/docs/
}

now=$(date +%s)
if ! network_ok; then
  if [ ! -e "$OFFLINE" ]; then
    echo "$now" > "$OFFLINE"
    echo "$(date -Is) connectivity lost; waiting without restarting services" >> "$LOG"
  fi
  exit 75
fi

reason=
if [ -e "$OFFLINE" ]; then
  since=$(cat "$OFFLINE" 2>/dev/null || echo unknown)
  rm -f "$OFFLINE"
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
echo "$now $reason" > "$LAST_RECOVERY"
echo "$(date -Is) recovery started: $reason" >> "$LOG"

# A connectivity loss can leave the otherwise healthy shared service with a
# poisoned provider response. Restart it once per observed recovery event.
if "$OPENC_BIN" service restart >> "$LOG" 2>&1; then
  echo "$(date -Is) OpenCode service restarted" >> "$LOG"
else
  echo "$(date -Is) OpenCode service restart failed; nudge will still retry standalone" >> "$LOG"
fi

# A failed worker is gone, so an unlocked lock-v1 claim can be released now
# rather than waiting for the normal five-minute grace. Locked live claims are
# refreshed and retained by the same reaper.
if [ -x "$SCRIPT_DIR/nudge-reap-claims.sh" ]; then
  DEAD_CLAIM_TTL=0 RESERVATION_TTL=0     "$SCRIPT_DIR/nudge-reap-claims.sh" "$STATE/claims" "$STATE/nudge.log"
fi

# Bypass the ordinary quiet-period gate. The caller is the existing
# bedlam-nudge timer, which continues through its normal claim/concurrency
# gates and launches a fresh standalone worker when safe.
touch -d "@0" "$STATE/heartbeat"
echo "$(date -Is) recovery complete; current nudge pass may resume" >> "$LOG"
