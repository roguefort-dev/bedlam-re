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

echo "nudge queue tests: PASS"
