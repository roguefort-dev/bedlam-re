#!/usr/bin/env bash
# Reap abandoned claims through one pinned directory descriptor per pass.
set -u

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
CLAIMS=${1:?claims directory required}
LOG=${2:?log path required}
DEAD_CLAIM_TTL=${DEAD_CLAIM_TTL:-300}
RESERVATION_TTL=${RESERVATION_TTL:-300}
LEGACY_CLAIM_TTL=${LEGACY_CLAIM_TTL:-4200}
MALFORMED_CLAIM_TTL=${MALFORMED_CLAIM_TTL:-900}

exec "$SCRIPT_DIR/nudge-state.py" reap-claims "$CLAIMS" "$LOG" \
  "$DEAD_CLAIM_TTL" "$RESERVATION_TTL" "$LEGACY_CLAIM_TTL" "$MALFORMED_CLAIM_TTL"
