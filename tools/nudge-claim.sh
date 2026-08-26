#!/usr/bin/env bash
# Shared lock-v1/lock-v2 claim parsing. Callers source this file.

claim_read() {
  local claim_file=$1 expected_ordinal=${2:-} expected_session=${3:-}
  local first key value line name ordinal_from_name header_count claim_raw claim_from_fd=0

  CLAIM_VERSION=""
  CLAIM_ORDINAL=""
  CLAIM_ID=""
  CLAIM_GATE=""
  CLAIM_OWNER=""
  CLAIM_SESSION=""
  CLAIM_CLAIMED_AT=""
  CLAIM_UNIT=""
  CLAIM_PID=""
  CLAIM_BODY_SHA256=""
  CLAIM_QUEUE_DEVICE=""
  CLAIM_QUEUE_INODE=""
  CLAIM_QUEUE_SHA256=""

  # Workers reuse fd 8 on the canonical inode. Other callers delegate one
  # O_NOFOLLOW open/fstat/flock/read to nudge-state.py and parse only its bytes.
  if [ "${NUDGE_OWNER_FD:-}" = 8 ] && [ -e /proc/self/fd/8 ]; then
    claim_raw=$(cat /proc/self/fd/8) || return 1
    claim_from_fd=1
  else
    [ -f "$claim_file" ] && [ ! -L "$claim_file" ] || return 1
    claim_raw=$("$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/nudge-state.py" read-claim "$claim_file") || return 1
  fi
  local claim_mode claim_uid
  if [ "$claim_from_fd" -eq 1 ]; then
    claim_mode=$(stat -Lc %a /proc/self/fd/8 2>/dev/null) || return 1
    claim_uid=$(stat -Lc %u /proc/self/fd/8 2>/dev/null) || return 1
  else
    claim_mode=600
    claim_uid=$(id -u)
  fi
  [ "$claim_uid" -eq "$(id -u)" ] && [ $((8#$claim_mode & 8#022)) -eq 0 ] || return 1
  IFS= read -r first <<< "$claim_raw" || return 1
  name=$(basename "$claim_file")
  [[ "$name" =~ ^[1-9][0-9]*-([A-Za-z0-9][A-Za-z0-9._-]*|owner)\.claim$ ]] || return 1
  ordinal_from_name=$name
  ordinal_from_name=${ordinal_from_name%%-*}

  if [ "$first" = lock-v2 ]; then
    CLAIM_VERSION=2
    header_count=$(grep -cx lock-v2 <<< "$claim_raw" 2>/dev/null || true)
    [ "$header_count" -eq 1 ] || return 1
    while IFS= read -r line || [ -n "$line" ]; do
      [ "$line" = lock-v2 ] && continue
      case "$line" in
        *=*) key=${line%%=*}; value=${line#*=} ;;
        *) return 1 ;;
      esac
      [ -n "$value" ] || return 1
      case "$key" in
        ordinal) [ -z "$CLAIM_ORDINAL" ] || return 1; CLAIM_ORDINAL=$value ;;
        id) [ -z "$CLAIM_ID" ] || return 1; CLAIM_ID=$value ;;
        gate) [ -z "$CLAIM_GATE" ] || return 1; CLAIM_GATE=$value ;;
        owner) [ -z "$CLAIM_OWNER" ] || return 1; CLAIM_OWNER=$value ;;
        session) [ -z "$CLAIM_SESSION" ] || return 1; CLAIM_SESSION=$value ;;
        claimed_at) [ -z "$CLAIM_CLAIMED_AT" ] || return 1; CLAIM_CLAIMED_AT=$value ;;
        unit) [ -z "$CLAIM_UNIT" ] || return 1; CLAIM_UNIT=$value ;;
        pid) [ -z "$CLAIM_PID" ] || return 1; CLAIM_PID=$value ;;
        body_sha256) [ -z "$CLAIM_BODY_SHA256" ] || return 1; CLAIM_BODY_SHA256=$value ;;
        queue_device) [ -z "$CLAIM_QUEUE_DEVICE" ] || return 1; CLAIM_QUEUE_DEVICE=$value ;;
        queue_inode) [ -z "$CLAIM_QUEUE_INODE" ] || return 1; CLAIM_QUEUE_INODE=$value ;;
        queue_sha256) [ -z "$CLAIM_QUEUE_SHA256" ] || return 1; CLAIM_QUEUE_SHA256=$value ;;
        *) return 1 ;;
      esac
    done <<< "$claim_raw"

    [[ "$CLAIM_ORDINAL" =~ ^[1-9][0-9]*$ ]] || return 1
    [[ "$CLAIM_ID" =~ ^[a-z0-9][a-z0-9._-]*$ ]] || return 1
    [[ "$CLAIM_GATE" =~ ^[a-z0-9][a-z0-9._-]*$ ]] || return 1
    [ "$CLAIM_OWNER" = worker ] || return 1
    [[ "$CLAIM_SESSION" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || return 1
    [[ "$CLAIM_CLAIMED_AT" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$ ]] || return 1
    date -d "$CLAIM_CLAIMED_AT" +%s >/dev/null 2>&1 || return 1
    [ "$CLAIM_UNIT" = "bedlam-nudge-item${CLAIM_ORDINAL}-${CLAIM_SESSION}" ] || return 1
    [[ "$CLAIM_PID" =~ ^[1-9][0-9]*$ ]] || return 1
    [[ "$CLAIM_BODY_SHA256" =~ ^[0-9a-f]{64}$ ]] || return 1
    [[ "$CLAIM_QUEUE_SHA256" =~ ^[0-9a-f]{64}$ ]] || return 1
    [[ "$CLAIM_QUEUE_DEVICE" =~ ^[0-9]+$ ]] || return 1
    [[ "$CLAIM_QUEUE_INODE" =~ ^[0-9]+$ ]] || return 1
    [ "$CLAIM_ORDINAL" = "$ordinal_from_name" ] || return 1
  else
    CLAIM_VERSION=1
    line=$(grep -E '^lock-v1 worker [A-Za-z0-9][A-Za-z0-9._-]* owns queue item [1-9][0-9]*$' <<< "$claim_raw" 2>/dev/null | tail -n 1)
    [ -n "$line" ] || return 1
    CLAIM_OWNER=worker
    CLAIM_SESSION=$(printf '%s\n' "$line" | awk '{print $3}')
    CLAIM_ORDINAL=$(printf '%s\n' "$line" | awk '{print $7}')
    [ "$CLAIM_ORDINAL" = "$ordinal_from_name" ] || return 1
  fi

  [ -z "$expected_ordinal" ] || [ "$CLAIM_ORDINAL" = "$expected_ordinal" ] || return 1
  [ -z "$expected_session" ] || [ "$CLAIM_SESSION" = "$expected_session" ] || return 1
  return 0
}
