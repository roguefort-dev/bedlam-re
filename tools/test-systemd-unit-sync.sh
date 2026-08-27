#!/usr/bin/env bash
# Guard against installed ~/.config/systemd/user/ bedlam units drifting from
# the repo sources in tools/systemd/. The watchdog trigger silently died for
# hours because the installed path unit predated the automation-failures
# watch; this keeps every installed bedlam unit byte-identical to its source.
#
# Environment-optional: with no systemd user session (or
# SYSTEMD_UNIT_SYNC_SKIP=1) the suite skips instead of raising a false alarm.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SYSTEMD_DIR="$ROOT/tools/systemd"
EXPECTED_DIR="$HOME/.config/systemd/user"
failures=0

if [ "${SYSTEMD_UNIT_SYNC_SKIP:-}" = "1" ]; then
  printf 'ok - systemd user session unavailable; unit sync untested\n'
  exit 0
fi

shopt -s nullglob
units=("$SYSTEMD_DIR"/bedlam-*)
shopt -u nullglob

if [ "${#units[@]}" -eq 0 ]; then
  printf 'not ok - no tools/systemd/bedlam-* units found\n' >&2
  exit 1
fi

for unit in "${units[@]}"; do
  name=$(basename "$unit")
  cat_out=$(systemctl --user cat "$name" 2>&1)
  rc=$?
  if [ "$rc" -ne 0 ]; then
    case "$cat_out" in
      *"No files found"*)
        printf 'not ok - %s: installed unit missing (repo: %s, installed: %s not present)\n' \
          "$name" "$unit" "$EXPECTED_DIR/$name" >&2
        failures=$((failures + 1))
        ;;
      *)
        # No user manager bus (CI, containers, dead session): environment-optional.
        printf 'ok - systemd user session unavailable; unit sync untested\n'
        exit 0
        ;;
    esac
    continue
  fi

  # First "# /path" header line names the main installed fragment.
  installed=$(printf '%s\n' "$cat_out" | sed -n 's/^# \//\//p' | head -n 1)
  if [ -z "$installed" ] || [ ! -f "$installed" ]; then
    printf 'not ok - %s: installed unit path unresolved (repo: %s, installed: %s)\n' \
      "$name" "$unit" "${installed:-<none>}" >&2
    failures=$((failures + 1))
    continue
  fi

  if diff -q "$installed" "$unit" >/dev/null 2>&1; then
    printf 'ok - %s: %s matches repo\n' "$name" "$installed"
  else
    printf 'not ok - %s: installed copy drifted from repo (repo: %s, installed: %s)\n' \
      "$name" "$unit" "$installed" >&2
    failures=$((failures + 1))
  fi
done

if [ "$failures" -gt 0 ]; then
  printf 'systemd unit sync tests: FAIL (%s)\n' "$failures" >&2
  exit 1
fi
printf 'systemd unit sync tests: PASS\n'
