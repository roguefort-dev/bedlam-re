#!/usr/bin/env bash
# Global autonomy stop for bedlam-re.
# Takes the controller lock, handles an in-flight watchdog repair, creates the
# human PAUSE atomically, disables timers/services, and drains transient worker
# units to a fixed point. Never deletes claims or WIP.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
SYSTEMCTL=${SYSTEMCTL_OVERRIDE:-systemctl}
NUDGE_LOCK=${NUDGE_LOCK:-/tmp/bedlam-nudge.lock}
MARKER="$STATE/llm-watchdog-pause"
PAUSE="$STATE/PAUSE"

mkdir -p "$STATE"
exec 9>"$NUDGE_LOCK"
if ! flock -w 30 9; then
  echo "ERROR: could not acquire controller lock ($NUDGE_LOCK) within 30s" >&2
  exit 1
fi

# 1. If the watchdog is mid-repair, stop it and let its exit trap release its
#    own pause. We must not overwrite a watchdog-owned pause.
for _ in $(seq 1 12); do
  [ -e "$MARKER" ] || break
  marker_tok=$(cat "$MARKER" 2>/dev/null || true)
  pause_tok=$(cat "$PAUSE" 2>/dev/null || true)
  if [ "$marker_tok" != "$pause_tok" ]; then
    break  # not a matched watchdog pair; treat below as human/orphan
  fi
  "$SYSTEMCTL" --user stop bedlam-llm-watchdog.service 2>/dev/null || true
  sleep 5
done
if [ -e "$MARKER" ] && [ "$(cat "$MARKER" 2>/dev/null || true)" = "$(cat "$PAUSE" 2>/dev/null || true)" ]; then
  echo "WARNING: watchdog-owned pause still present after stop attempt; not touching it" >&2
fi

# 2. Create the human PAUSE atomically.
stop_token="autonomy-stop $(date -Is)"
if [ -e "$PAUSE" ]; then
  echo "PAUSE already present (content: $(cat "$PAUSE")) - respecting it"
else
  tmp="$PAUSE.tmp.$$"
  printf "%s\n" "$stop_token" > "$tmp"
  if ln "$tmp" "$PAUSE" 2>/dev/null; then
    rm -f "$tmp"
    echo "PAUSE created: $stop_token"
  else
    rm -f "$tmp"
    echo "PAUSE appeared concurrently (content: $(cat "$PAUSE" 2>/dev/null || true)) - respecting it"
  fi
fi

# 3. Disable and stop scheduled autonomy.
"$SYSTEMCTL" --user disable --now bedlam-nudge.timer bedlam-llm-watchdog.timer 2>/dev/null || true
"$SYSTEMCTL" --user stop bedlam-nudge.service bedlam-llm-watchdog.service 2>/dev/null || true

# 4. Drain transient worker units to a fixed point. Enumerate by ExecStart
#    against ALL user services (no run-p* assumption), stop, re-check.
swept=0
for round in $(seq 1 10); do
  found=0
  while read -r unit; do
    [ -n "$unit" ] || continue
    props=$("$SYSTEMCTL" --user show -p ExecStart --value "$unit" 2>/dev/null || true)
    case "$props" in
      *"$PLAN_DIR/tools/nudge-agent.sh"*)
        echo "stopping transient worker unit: $unit"
        "$SYSTEMCTL" --user stop "$unit" || true
        found=1
        swept=$((swept+1))
        ;;
    esac
  done < <("$SYSTEMCTL" --user list-units --type=service --state=running --no-legend --plain 2>/dev/null | awk "{print \$1}")
  [ "$found" -eq 0 ] && break
  sleep 2
done

# 5. Report claim/WIP state; never modify it.
echo
echo "--- claims (preserved, not modified) ---"
shopt -s nullglob
locked=0
for c in "$STATE"/claims/*.claim; do
  if flock -n "$c" true 2>/dev/null; then
    echo "unlocked: $(basename "$c") (stale/dead worker; reaper may collect it later)"
  else
    echo "LOCKED:   $(basename "$c") (a descendant still holds it - investigate)"
    locked=1
  fi
done
[ -z "$(ls "$STATE"/claims 2>/dev/null)" ] && echo "(none)"
echo "--- WIP (preserved) ---"
git -C "$PLAN_DIR" status --short --branch 2>/dev/null || true
echo "--- timers ---"
"$SYSTEMCTL" --user is-enabled bedlam-nudge.timer bedlam-llm-watchdog.timer 2>/dev/null || true
if [ "$locked" -eq 1 ]; then
  echo
  echo "RESULT: autonomy stopped; some claims remain LOCKED by live descendants."
  exit 1
fi
echo
echo "RESULT: autonomy fully stopped (timers disabled, $swept transient unit(s) swept, no locked claims)."
