#!/usr/bin/env bash
# Serialized claim publication. The caller holds .state/claims/.publish.lock.
set -u

PLAN_DIR=${1:?plan directory required}
item=${2:?item required}
slotid=${3:?session required}
unit_name=${4:?unit required}
nowh=${5:?hour required}
maxspawn=${6:?spawn cap required}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
STATE="$PLAN_DIR/.state"
CLAIMS="$STATE/claims"

case "$nowh" in ''|*[!0-9]*) echo "invalid spawn hour" >&2; exit 75 ;; esac
case "$maxspawn" in ''|*[!0-9]*) echo "invalid spawn maximum" >&2; exit 75 ;; esac
[ "${#nowh}" -le 12 ] && [ "${#maxspawn}" -le 6 ] || { echo "out-of-range spawn numeric state" >&2; exit 75; }
[ "$maxspawn" -ge 1 ] 2>/dev/null || { echo "invalid spawn maximum" >&2; exit 75; }

if [ "${NUDGE_QUEUE_LOCK_HELD:-0}" != 1 ]; then
  exec "$SCRIPT_DIR/nudge-lock.py" lock-run "$STATE/.queue.lock" blocking \
    env NUDGE_QUEUE_LOCK_HELD=1 "$0" "$@"
fi

[ ! -e "$STATE/PAUSE" ] || exit 75
queue_state=$("$SCRIPT_DIR/nudge-free-items.py" "$STATE/NEXT.md" "$CLAIMS" --state-v1) || exit $?
case " $queue_state " in
  *" $item "*) ;;
  *) exit 75 ;;
esac
item_fields=$("$SCRIPT_DIR/nudge-free-items.py" "$STATE/NEXT.md" "$CLAIMS" --item-v2 "$item") || exit $?
read -r item_status item_id item_gate body_hash queue_device queue_inode queue_hash <<< "$item_fields"
[ "$item_status" = READY ] || exit 75

if [ -e "$STATE/spawns" ]; then
  spawn_fields=$("$SCRIPT_DIR/nudge-state.py" read-fields "$STATE/spawns" spawn-hour spawn-count 0 999999999999 0 "$maxspawn") || exit $?
  read -r h c <<< "$spawn_fields"
else
  h=0; c=0
fi
[ "$h" = "$nowh" ] || c=0
[ "$c" -lt "$maxspawn" ] || exit 75
spawn_payload=$(printf '%s %s\n' "$nowh" "$c")
"$SCRIPT_DIR/nudge-state.py" write-text "$STATE/spawns" "$spawn_payload" || exit $?

claimed_at=$(date -Is)
"$SCRIPT_DIR/nudge-state.py" publish-claim "$CLAIMS" "$item-$slotid.claim" \
  "$item" "$item_id" "$item_gate" "$slotid" "$claimed_at" "$unit_name" "$$" \
  "$body_hash" "$queue_device" "$queue_inode" "$queue_hash" || exit $?
identity=$(stat -c '%d:%i' "$CLAIMS/$item-$slotid.claim") || exit 75
spawn_payload=$(printf '%s %s\n' "$nowh" "$((c + 1))")
"$SCRIPT_DIR/nudge-state.py" write-text "$STATE/spawns" "$spawn_payload" || exit $?
printf '%s %s %s %s %s\n' "$item_id" "$item_gate" "$c" "$identity" "$claimed_at"
