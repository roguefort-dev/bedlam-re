#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d /tmp/bedlam-llm-watchdog-test.XXXXXX)
cleanup() { jobs -pr | xargs -r kill 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims"
printf "# NEXT\n\n## Now\n1. [P4] test task\n\n## Backlog\n" > "$PLAN/.state/NEXT.md"
printf "# AGENTS\n" > "$PLAN/AGENTS.md"
printf "# STATE\n" > "$PLAN/.state/STATE.md"
printf "initial\n" > "$PLAN/code.txt"
git -C "$PLAN" init -q
git -C "$PLAN" config user.email test@example.invalid
git -C "$PLAN" config user.name test
git -C "$PLAN" add AGENTS.md .state/NEXT.md .state/STATE.md code.txt
git -C "$PLAN" commit -qm init

cat > "$TMP/mock-opencode" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/calls"
case "\$*" in
  *gpt-5.6-luna*)
    case "\${MOCK_LUNA:-healthy}" in
      healthy) printf "\033[32mWATCHDOG_OK\033[0m\r\n" ;;
      repair) echo WATCHDOG_REPAIR ;;
      invalid) echo WATCHDOG_OK; echo trailing-text ;;
      race) printf "human pause\n" > "$PLAN/.state/PAUSE"; echo WATCHDOG_REPAIR ;;
    esac
    ;;
  *gpt-5.6-sol*)
    if [ "\${MOCK_SOL_SLEEP:-0}" = 1 ]; then sleep 3; fi
    if [ "\${MOCK_SOL_COMMIT:-0}" = 1 ]; then
      token=\$(cat "$PLAN/.state/PAUSE")
      echo repaired >> "$PLAN/code.txt"
      git -C "$PLAN" add code.txt
      git -C "$PLAN" commit -qm repair -m "Watchdog-Repair: \$token"
    fi
    echo repair-complete
    ;;
esac
EOF
chmod +x "$TMP/mock-opencode"
common=(BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-opencode" REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" WATCHDOG_TEST_MODE=1 CHECK_TIMEOUT=5 REPAIR_TIMEOUT=5 REPAIR_COOLDOWN=60)

# ANSI/CR final marker is accepted as healthy and does not invoke Sol.
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/healthy.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "openai/gpt-5.6-luna#max" "$TMP/calls"
! grep -q "openai/gpt-5.6-sol#high" "$TMP/calls"
[ ! -e "$PLAN/.state/PAUSE" ]

# A valid repair runs Sol high, requires commit evidence, then releases pause.
MOCK_LUNA=repair MOCK_SOL_COMMIT=1 env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/repair.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "openai/gpt-5.6-sol#high" "$TMP/calls"
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]
grep -q "Sol repair produced evidence" "$PLAN/.state/llm-watchdog.log"

# Trailing text invalidates an otherwise exact marker and a no-op Sol cools down.
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"
MOCK_LUNA=invalid env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/invalid.lock" "$ROOT/tools/llm-watchdog.sh"
[ -e "$PLAN/.state/llm-watchdog-cooldown-until" ]
grep -q "no repair evidence" "$PLAN/.state/llm-watchdog.log"
[ ! -e "$PLAN/.state/PAUSE" ]
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"

# Human PAUSE winning the O_EXCL race is preserved and Sol is not called again.
sol_calls=$(grep -c "gpt-5.6-sol#high" "$TMP/calls")
MOCK_LUNA=race env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/race.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(cat "$PLAN/.state/PAUSE")" = "human pause" ]
[ "$(grep -c "gpt-5.6-sol#high" "$TMP/calls")" -eq "$sol_calls" ]
rm -f "$PLAN/.state/PAUSE"

# Matching stale watchdog tokens are recovered under the singleton lock.
printf "llm-watchdog 999 1\n" > "$PLAN/.state/PAUSE"
printf "llm-watchdog 999 1\n" > "$PLAN/.state/llm-watchdog-pause"
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/stale.lock" "$ROOT/tools/llm-watchdog.sh"
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]
grep -q "recovered stale watchdog-owned pause" "$PLAN/.state/llm-watchdog.log"

# Sol timeout still releases only its own pause and records cooldown.
MOCK_LUNA=repair MOCK_SOL_SLEEP=1 env "${common[@]}" REPAIR_TIMEOUT=1 LLM_WATCHDOG_LOCK="$TMP/timeout.lock" "$ROOT/tools/llm-watchdog.sh"
[ ! -e "$PLAN/.state/PAUSE" ]
[ -e "$PLAN/.state/llm-watchdog-cooldown-until" ]

# TERM during Sol cannot strand a watchdog-owned pause.
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"
MOCK_LUNA=repair MOCK_SOL_SLEEP=1 env "${common[@]}" REPAIR_TIMEOUT=10 LLM_WATCHDOG_LOCK="$TMP/signal.lock" "$ROOT/tools/llm-watchdog.sh" &
watchdog_pid=$!
for _ in $(seq 1 100); do [ -e "$PLAN/.state/PAUSE" ] && break; sleep 0.02; done
[ -e "$PLAN/.state/PAUSE" ]
kill -TERM "$watchdog_pid"
wait "$watchdog_pid" 2>/dev/null || true
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]

# A pre-existing human pause prevents all model calls.
rm -f "$PLAN/.state/llm-watchdog-cooldown-until"
printf "human pause\n" > "$PLAN/.state/PAUSE"
lines=$(wc -l < "$TMP/calls")
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/paused.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(wc -l < "$TMP/calls")" -eq "$lines" ]
grep -q "human PAUSE present" "$PLAN/.state/llm-watchdog.log"
echo "llm watchdog tests: PASS"
