#!/usr/bin/env bash
# Reap abandoned nudge claims without expiring claims held by live workers.
set -u

CLAIMS=${1:?claims directory required}
LOG=${2:?log path required}
DEAD_CLAIM_TTL=${DEAD_CLAIM_TTL:-300}
RESERVATION_TTL=${RESERVATION_TTL:-300}
LEGACY_CLAIM_TTL=${LEGACY_CLAIM_TTL:-4200}

now=$(date +%s)
for c in "$CLAIMS"/*.claim; do
  [ -e "$c" ] || continue
  name=$(basename "$c")
  ts=$(stat -c %Y "$c" 2>/dev/null || echo 0)
  age=$((now - ts))
  ttl=$RESERVATION_TTL
  kind=reservation

  if [[ "$name" == *-owner.claim ]]; then
    if grep -q "^lock-v1 " "$c" 2>/dev/null; then
      flock -n "$c" true 2>/dev/null
      lock_rc=$?
      if [ "$lock_rc" -eq 1 ]; then
        # Refresh the last-observed-live time. Once the lock disappears, the
        # dead-worker grace starts from at most one timer interval ago.
        touch "$c"
        continue
      elif [ "$lock_rc" -ne 0 ]; then
        echo "$(date -Is) unable to inspect claim lock $name (flock rc=$lock_rc)" >> "$LOG"
        continue
      fi
      ttl=$DEAD_CLAIM_TTL
      kind=dead-worker
    else
      # Claims created before lock-v1 cannot prove liveness. Keep the old,
      # conservative timeout so an in-flight worker from an upgrade is safe.
      ttl=$LEGACY_CLAIM_TTL
      kind=legacy-owner
    fi
  fi

  if [ "$age" -gt "$ttl" ]; then
    echo "$(date -Is) reaped stale $kind claim $name (age ${age}s)" >> "$LOG"
    rm -f "$c"
  fi
done
