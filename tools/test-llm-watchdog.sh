#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d /tmp/bedlam-llm-watchdog-test.XXXXXX)
trap "rm -rf \"$TMP\"" EXIT
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims"
printf "# NEXT\n\n## Now\n1. [P4] test task\n\n## Backlog\n" > "$PLAN/.state/NEXT.md"
printf "# AGENTS\n" > "$PLAN/AGENTS.md"
printf "# STATE\n" > "$PLAN/.state/STATE.md"
git -C "$PLAN" init -q
git -C "$PLAN" config user.email test@example.invalid
git -C "$PLAN" config user.name test
git -C "$PLAN" add AGENTS.md .state/NEXT.md .state/STATE.md
git -C "$PLAN" commit -qm init

cat > "$TMP/mock-opencode" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/calls"
case "\$*" in
  *gpt-5.6-luna*)
    if [ "\${MOCK_REPAIR:-0}" = 1 ]; then echo WATCHDOG_REPAIR; else echo WATCHDOG_OK; fi
    ;;
  *gpt-5.6-sol*)
    echo repaired > "$TMP/sol-ran"
    echo repair-complete
    ;;
esac
EOF
chmod +x "$TMP/mock-opencode"

common=(BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-opencode" REAPER_OVERRIDE="$ROOT/tools/nudge-reap-claims.sh" WATCHDOG_TEST_MODE=1 CHECK_TIMEOUT=5 REPAIR_TIMEOUT=5)
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/healthy.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "openai/gpt-5.6-luna#max" "$TMP/calls"
[ ! -e "$TMP/sol-ran" ]
[ ! -e "$PLAN/.state/PAUSE" ]

MOCK_REPAIR=1 env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/repair.lock" "$ROOT/tools/llm-watchdog.sh"
grep -q "openai/gpt-5.6-sol#high" "$TMP/calls"
[ -e "$TMP/sol-ran" ]
[ ! -e "$PLAN/.state/PAUSE" ]
[ ! -e "$PLAN/.state/llm-watchdog-pause" ]

printf "human pause\n" > "$PLAN/.state/PAUSE"
lines=$(wc -l < "$TMP/calls")
env "${common[@]}" LLM_WATCHDOG_LOCK="$TMP/paused.lock" "$ROOT/tools/llm-watchdog.sh"
[ "$(wc -l < "$TMP/calls")" -eq "$lines" ]
grep -q "human PAUSE present" "$PLAN/.state/llm-watchdog.log"
echo "llm watchdog tests: PASS"
