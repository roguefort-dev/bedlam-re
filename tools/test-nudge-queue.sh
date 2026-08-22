#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PARSER="$ROOT/tools/nudge-free-items.py"
TMP=$(mktemp -d /tmp/bedlam-nudge-queue.XXXXXX)
trap "rm -rf $TMP" EXIT
mkdir -p "$TMP/claims"

cat > "$TMP/NEXT.md" <<EOF
## Now
1. [INTERACTIVE] desktop-only task
2. [P3] unattended task
## Backlog
EOF
[ "$("$PARSER" "$TMP/NEXT.md" "$TMP/claims")" = 2 ]
touch "$TMP/claims/2-owner.claim"
[ -z "$("$PARSER" "$TMP/NEXT.md" "$TMP/claims")" ]
rm "$TMP/claims/2-owner.claim"
cat > "$TMP/NEXT.md" <<EOF
## Now
1. [INTERACTIVE] desktop-only task
## Backlog
EOF
[ -z "$("$PARSER" "$TMP/NEXT.md" "$TMP/claims")" ]

cat > "$TMP/NEXT.md" <<EOF
## Now
1. [P4] [BLOCKED] phase-tagged blocked item
2. [P4] [INTERACTIVE] phase-tagged manual item
3. [P4] unattended item
## Backlog
EOF
[ "$("$PARSER" "$TMP/NEXT.md" "$TMP/claims")" = 3 ]

# A suffixed BLOCKED tag skips exactly like [BLOCKED]: workers naturally
# annotate the blocker ([BLOCKED-operator-desktop], 2026-08-22 watchdog
# repair) and the exact-match-only check kept the unprogressable item
# spawnable while real work starved behind it.
cat > "$TMP/NEXT.md" <<EOF
## Now
1. [BLOCKED-operator-desktop] [P4] suffixed blocked item
2. [P4] unattended item
## Backlog
EOF
[ "$("$PARSER" "$TMP/NEXT.md" "$TMP/claims")" = 2 ]

echo "nudge queue tests: PASS"
