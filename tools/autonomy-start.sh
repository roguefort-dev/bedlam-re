#!/usr/bin/env bash
# Staged autonomy start for bedlam-re after a successful autonomy-stop.
# Verifies installed units match committed templates, clears diagnosed stale
# controller state, removes only a stop-owned PAUSE, and enables the nudge
# timer (watchdog only with --with-watchdog, after a healthy Luna cycle).
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
STATE="$PLAN_DIR/.state"
SYSTEMCTL=${SYSTEMCTL_OVERRIDE:-systemctl}

cd "$PLAN_DIR" || exit 1

fail=0
for pair in "tools/systemd/bedlam-nudge.service:$HOME/.config/systemd/user/bedlam-nudge.service" \
            "tools/systemd/bedlam-nudge.timer:$HOME/.config/systemd/user/bedlam-nudge.timer" \
            "tools/systemd/bedlam-llm-watchdog.service:$HOME/.config/systemd/user/bedlam-llm-watchdog.service" \
            "tools/systemd/bedlam-llm-watchdog.timer:$HOME/.config/systemd/user/bedlam-llm-watchdog.timer"; do
  tpl=${pair%%:*}
  inst=${pair#*:}
  if [ ! -f "$tpl" ]; then
    echo "FAIL: committed template missing: $tpl"
    fail=1
    continue
  fi
  if [ ! -f "$inst" ]; then
    echo "FAIL: installed unit missing: $inst"
    fail=1
    continue
  fi
  if ! cmp -s "$tpl" "$inst"; then
    echo "FAIL: installed unit differs from committed template: $inst"
    diff "$tpl" "$inst" | head -5
    fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "units: all installed units byte-match committed templates"
[ "$fail" -eq 1 ] && exit 1

"$SYSTEMCTL" --user daemon-reload || true

# Clear diagnosed stale controller state (never WIP, never claims).
rm -f "$STATE/fails" "$STATE/cooldown-until" "$STATE/llm-watchdog-cooldown-until"

# Remove only a PAUSE this tool family created.
if [ -e "$STATE/PAUSE" ]; then
  content=$(cat "$STATE/PAUSE" 2>/dev/null || true)
  case "$content" in
    autonomy-stop\ *)
      rm -f "$STATE/PAUSE"
      echo "PAUSE removed (was autonomy-stop owned)"
      ;;
    *)
      echo "REFUSING to remove operator PAUSE (content: $content) - remove it manually to start"
      exit 1
      ;;
  esac
fi

"$SYSTEMCTL" --user enable --now bedlam-nudge.timer
echo "nudge timer enabled:"
"$SYSTEMCTL" --user list-timers bedlam-nudge.timer --no-pager | sed -n "1,3p"

if [ "${1:-}" = "--with-watchdog" ]; then
  "$SYSTEMCTL" --user enable --now bedlam-llm-watchdog.timer
  echo "watchdog timer enabled:"
  "$SYSTEMCTL" --user list-timers bedlam-llm-watchdog.timer --no-pager | sed -n "1,3p"
else
  cat <<EOF
Staged start checklist:
 1. Watch the first GLM unit end-to-end: an attributed Nudge-Worker commit and
    a released claim in .state/nudge.log.
 2. Run one controlled Luna cycle: systemctl --user start bedlam-llm-watchdog.service
    then check .state/llm-watchdog-verdict (expect state=healthy).
 3. Only then: $0 --with-watchdog
EOF
fi
