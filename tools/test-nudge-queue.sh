#!/usr/bin/env bash
# Contract tests for the strict active-queue grammar. These are deliberately
# ahead of nudge-free-items.py: keep them RED until the parser/controller slice
# implements the grammar without silently hiding invalid or human-only work.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PARSER="$ROOT/tools/nudge-free-items.py"
TMP=$(mktemp -d /tmp/opencode/bedlam-nudge-queue.XXXXXX)
LOCK_PIDS=""
trap 'for pid in $LOCK_PIDS; do kill "$pid" 2>/dev/null || true; done; rm -rf "$TMP"' EXIT
QUEUE="$TMP/.state/NEXT.md"
CLAIMS="$TMP/.state/claims"
mkdir -p "$CLAIMS"
mkdir -p "$TMP/tools"
mkdir -p "$TMP/docs"
printf '#!/usr/bin/env bash\nexit 1\n' > "$TMP/tools/probe.sh"
printf '#!/usr/bin/env bash\nexit 1\n' > "$TMP/tools/probe-ci.sh"
chmod +x "$TMP/tools/probe.sh" "$TMP/tools/probe-ci.sh"
probe_digest=$(sha256sum "$TMP/tools/probe.sh" | awk '{print $1}')
probe_ci_digest=$(sha256sum "$TMP/tools/probe-ci.sh" | awk '{print $1}')
cat > "$TMP/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "tools/probe.sh"
path = "tools/probe.sh"
sha256 = "$probe_digest"
[[probe]]
id = "tools/probe-ci.sh"
path = "tools/probe-ci.sh"
sha256 = "$probe_ci_digest"
EOF
git -C "$TMP" init -q
git -C "$TMP" config user.email test@example.invalid
git -C "$TMP" config user.name test
git -C "$TMP" add tools docs/automatic-probes.toml
git -C "$TMP" commit -qm probe-policy

failures=0

write_queue() {
  for pid in $LOCK_PIDS; do kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; done
  LOCK_PIDS=""
  rm -f "$CLAIMS"/*.claim "$TMP/stdout" "$TMP/stderr"
  cat > "$QUEUE"
}

write_v2_owner() {
  local ordinal=$1 item_id=$2 gate=$3 session=queue-test fields status parsed_id parsed_gate body dev ino queue
  fields=$($PARSER "$QUEUE" "$CLAIMS" --item-v2 "$ordinal")
  read -r status parsed_id parsed_gate body dev ino queue <<< "$fields"
  cat > "$CLAIMS/$ordinal-owner.claim" <<EOF
lock-v2
ordinal=$ordinal
id=$item_id
gate=$gate
owner=worker
session=$session
claimed_at=$(date -Is)
unit=bedlam-nudge-item$ordinal-$session
pid=$$
body_sha256=$body
queue_device=$dev
queue_inode=$ino
queue_sha256=$queue
EOF
  (
    exec 8<>"$CLAIMS/$ordinal-owner.claim"
    flock 8
    sleep 300
  ) &
  local holder=$!
  LOCK_PIDS="$LOCK_PIDS $holder"
  for _ in $(seq 1 100); do
    flock -n "$CLAIMS/$ordinal-owner.claim" true 2>/dev/null || break
    sleep 0.01
  done
}

run_parser() {
  "$PARSER" "$QUEUE" "$CLAIMS" "$@" \
    >"$TMP/stdout" 2>"$TMP/stderr"
  LAST_RC=$?
  LAST_OUT=$(cat "$TMP/stdout")
  LAST_ERR=$(cat "$TMP/stderr")
}

fail() {
  local name=$1 expected=$2
  failures=$((failures + 1))
  printf 'not ok - %s\n' "$name"
  printf '  expected: %s\n' "$expected"
  printf '  actual: rc=%s stdout=%q stderr=%q\n' \
    "$LAST_RC" "$LAST_OUT" "$LAST_ERR"
}

expect_ok() {
  local name=$1 expected_out=$2
  shift 2
  run_parser "$@"
  if [ "$LAST_RC" -ne 0 ] || [ "$LAST_OUT" != "$expected_out" ] || [ -n "$LAST_ERR" ]; then
    fail "$name" "rc=0 stdout=$(printf %q "$expected_out") stderr empty"
  fi
}

expect_invalid() {
  local name=$1 error_pattern=$2
  shift 2
  run_parser "$@"
  if [ "$LAST_RC" -ne 2 ] || [ -n "$LAST_OUT" ] \
      || ! grep -Eqi "$error_pattern" "$TMP/stderr"; then
    fail "$name" "rc=2 stdout empty stderr~/$error_pattern/"
  fi
}

expect_deadlocked() {
  local name=$1
  run_parser
  if [ "$LAST_RC" -ne 2 ] || [ -n "$LAST_OUT" ] || [ -z "$LAST_ERR" ]; then
    fail "$name (default mode)" "rc=2 stdout empty stderr nonempty"
  fi
  run_parser --state-v1
  if [ "$LAST_RC" -ne 2 ] || [ "$LAST_OUT" != "INVALID-DEADLOCKED" ] \
      || [ -z "$LAST_ERR" ]; then
    fail "$name (state mode)" \
      "rc=2 stdout=INVALID-DEADLOCKED stderr nonempty"
  fi
}

# Default mode remains the controller's numeric claim interface. A valid READY
# item prints its ordinal, and the existing numeric claim filename suppresses it.
write_queue <<'EOF'
## Now
1. [READY] [id=compile-slice] [gate=p4-build] compile the bounded slice
## Backlog
EOF
expect_ok "READY keeps numeric output compatibility" "1"
write_v2_owner 1 compile-slice p4-build
expect_ok "numeric owner claims still suppress READY items" ""

write_queue <<'EOF'
## Now
1. [READY] [id=compile-slice] [gate=p4-build] compile the bounded slice
2. [READY] [id=test-slice] [gate=p4-test] run the bounded test slice
## Backlog
EOF
expect_ok "multiple READY items preserve ordinal output" "1 2"

# --state-v1 is an additive, versioned inspection mode. It must not replace or
# alter default numeric output used by nudge.sh.
expect_ok "state mode distinguishes runnable work" "RUNNABLE 1 2" --state-v1

write_queue <<'EOF'
## Now
1. [WAITING-AUTOMATIC] [id=ci-result] [gate=p4-ci] [probe=tools/probe-ci.sh] [retry=30s] [timeout=20m] wait for the CI result probe
## Backlog
EOF
expect_ok "automatic waits are not numerically claimable" ""
expect_ok "state mode distinguishes automatic wait" "AUTOMATIC-WAIT" --state-v1

write_queue <<'EOF'
## Now
## Backlog
EOF
expect_ok "empty required queue has no numeric claims" ""
expect_ok "state mode distinguishes required queue empty" "REQUIRED-QUEUE-EMPTY" --state-v1

# Invalid active work is a deadlocked queue, never an empty/runnable-looking
# queue. Exit 2 is preserved by the controller.
write_queue <<'EOF'
## Now
1. [BLOCKED] [id=blocked-slice] [gate=p4-blocked] blocked task
## Backlog
EOF
expect_invalid "BLOCKED is rejected instead of silently omitted" 'BLOCKED|forbidden status'
run_parser --state-v1
if [ "$LAST_RC" -ne 2 ] || [ "$LAST_OUT" != "INVALID-DEADLOCKED" ] \
    || ! grep -Eqi 'BLOCKED|forbidden status' "$TMP/stderr"; then
  fail "state mode exposes invalid/deadlocked" \
    "rc=2 stdout=INVALID-DEADLOCKED stderr~/BLOCKED|forbidden status/"
fi

write_queue <<'EOF'
## Now
1. untagged task accepted by the legacy parser
## Backlog
EOF
expect_invalid "untagged active item is rejected instead of scheduled" 'status|untagged|READY|WAITING-AUTOMATIC'

for status in BLOCKED INTERACTIVE MANUAL DESKTOP OPTIONAL EXTERNAL LEGAL; do
  write_queue <<EOF
## Now
1. [$status] [id=forbidden-status] [gate=forbidden-gate] forbidden status task
## Backlog
EOF
  expect_invalid "rejects forbidden $status status" "$status|forbidden status"
done

write_queue <<'EOF'
## Now
1. [P4] [id=unknown-status] [gate=p4-unknown] unknown phase/status tag
## Backlog
EOF
expect_invalid "rejects unknown status/tag" 'P4|unknown.*tag|status'

write_queue <<'EOF'
## Now
1. [READY] [WAITING-AUTOMATIC] [id=two-statuses] [gate=p4-two] [probe=tools/probe.sh] [retry=1m] [timeout=1h] ambiguous status
## Backlog
EOF
expect_invalid "requires exactly one status" 'exactly one|multiple.*status|READY.*WAITING-AUTOMATIC'

write_queue <<'EOF'
## Now
1. [READY] [id=unknown-metadata] [gate=p4-meta] [priority=high] unknown metadata
## Backlog
EOF
expect_invalid "rejects unknown metadata tags" 'priority|unknown.*metadata|unknown.*tag'

write_queue <<'EOF'
## Now
1. [READY] [gate=p4-id] missing stable id
## Backlog
EOF
expect_invalid "requires stable item id" 'id|required.*id|missing.*id'

write_queue <<'EOF'
## Now
1. [READY] [id=missing-gate] missing required gate
## Backlog
EOF
expect_invalid "requires gate id" 'gate|required.*gate|missing.*gate'

write_queue <<'EOF'
## Now
1. [READY] [id=../../escape] [gate=p4-safe] unsafe id
## Backlog
EOF
expect_invalid "rejects unsafe item id" 'id|safe|malformed'

write_queue <<'EOF'
## Now
1. [READY] [id=safe-id] [gate=../escape] unsafe gate id
## Backlog
EOF
expect_invalid "rejects unsafe gate id" 'gate|safe|malformed'

write_queue <<'EOF'
## Now
1. [READY] [id=duplicate-id] [gate=gate-one] first task
2. [READY] [id=duplicate-id] [gate=gate-two] second task
## Backlog
EOF
expect_invalid "rejects duplicate item ids" 'duplicate.*id|id.*duplicate'

write_queue <<'EOF'
## Now
1. [READY] [id=task-one] [gate=duplicate-gate] first task
2. [READY] [id=task-two] [gate=duplicate-gate] second task
## Backlog
EOF
expect_invalid "rejects duplicate gate ids" 'duplicate.*gate|gate.*duplicate'

write_queue <<'EOF'
## Now
1. [READY] [id=task-one] [gate=gate-one] first task
1. [READY] [id=task-two] [gate=gate-two] duplicate ordinal task
## Backlog
EOF
expect_invalid "rejects duplicate ordinals" 'duplicate.*ordinal|ordinal.*duplicate'

write_queue <<'EOF'
## Now
1. [READY] [id=one-id] [id=second-id] [gate=gate-one] duplicate id metadata
## Backlog
EOF
expect_invalid "rejects duplicate metadata keys" 'duplicate.*id|metadata'

write_queue <<'EOF'
## Now
1. [READY] [id = spaced-id] [gate=gate-one] malformed metadata
## Backlog
EOF
expect_invalid "rejects malformed metadata" 'malformed|id'

# A metadata tag hard-wrapped across lines (gate= on the item's first line,
# the value's tail on the next) is not canonical single-token metadata: the
# 2026-08-27 INVALID-DEADLOCKED stall was exactly this shape, and it must
# keep failing closed instead of scheduling a mangled identity.
write_queue <<'EOF'
## Now
1. [READY] [id=wrapped-gate] [gate=p5-zone-gate-
   scaffold] wrapped metadata tag across lines
## Backlog
EOF
expect_invalid "rejects metadata tag wrapped across lines" 'malformed metadata'

# WAITING-AUTOMATIC is fully machine-owned and bounded. Either timeout or an
# absolute deadline bounds it; retry and timeout durations are positive.
for missing in probe retry bound; do
  case "$missing" in
    probe) meta='[retry=30s] [timeout=20m]' ;;
    retry) meta='[probe=tools/probe.sh] [timeout=20m]' ;;
    bound) meta='[probe=tools/probe.sh] [retry=30s]' ;;
  esac
  write_queue <<EOF
## Now
1. [WAITING-AUTOMATIC] [id=wait-$missing] [gate=gate-$missing] $meta bounded automatic wait
## Backlog
EOF
  expect_invalid "automatic wait requires $missing metadata" "$missing|probe|retry|timeout|deadline"
done

for malformed in \
  '[retry=0s] [timeout=20m]' \
  '[retry=-1s] [timeout=20m]' \
  '[retry=soon] [timeout=20m]' \
  '[retry=30s] [timeout=0s]' \
  '[retry=30s] [timeout=none]'; do
  write_queue <<EOF
## Now
1. [WAITING-AUTOMATIC] [id=bad-wait] [gate=bad-wait-gate] [probe=tools/probe.sh] $malformed malformed automatic wait
## Backlog
EOF
  expect_invalid "rejects non-positive or malformed wait bound: $malformed" 'retry|timeout|positive|bounded|malformed'
done

deadline=$(date -u -d '+1 day' '+%Y-%m-%dT%H:%M:%SZ')
write_queue <<EOF
## Now
1. [WAITING-AUTOMATIC] [id=deadline-wait] [gate=deadline-gate] [probe=tools/probe.sh] [retry=5m] [deadline=$deadline] wait for the machine probe
## Backlog
EOF
expect_ok "absolute deadline is a valid bounded automatic wait" ""

write_queue <<'EOF'
## Now
1. [READY] [id=ready-with-wait] [gate=ready-gate] [probe=tools/probe.sh] [retry=5m] [timeout=1h] runnable task with inapplicable wait metadata
## Backlog
EOF
expect_invalid "READY rejects automatic-wait-only metadata" 'probe|retry|timeout|WAITING-AUTOMATIC|inapplicable'

# Lint the complete normative active item (including continuation lines), with
# case-insensitive token/phrase boundaries. These are the human-only escape
# hatches that previously accumulated in Now.
for phrase in \
  'human review' \
  'operator checks the result' \
  'manual calibration' \
  'interactive session' \
  'desktop capture' \
  'listen to the audio' \
  'listening to the audio' \
  'visual sign-off' \
  'owner approval' \
  'owner signature' \
  'sudo install the package' \
  'credential entry' \
  'credentials entry' \
  'secret entry' \
  'secrets entry' \
  'legal acceptance' \
  'license acceptance'; do
  slug=$(printf '%s' "$phrase" | tr -cs '[:alnum:]' '-' | tr '[:upper:]' '[:lower:]')
  write_queue <<EOF
## Now
1. [READY] [id=lint-$slug] [gate=lint-gate-$slug] perform $phrase
## Backlog
EOF
  expect_invalid "active-task lint rejects: $phrase" 'human|operator|manual|interactive|desktop|listen|visual sign-off|owner approval|sudo|credential|legal acceptance|human-only'
done

write_queue <<'EOF'
## Now
1. [READY] [id=multiline-lint] [gate=multiline-gate] run the automated capture
   and ask the operator at the machine to approve it.
## Backlog
EOF
expect_invalid "active-task lint includes continuation lines" 'operator|human-only'

# Forbidden words are scoped to active normative task text. Historical Done
# entries and non-active prose must remain readable without poisoning Now.
write_queue <<'EOF'
## Now
1. [READY] [id=scoped-lint] [gate=scoped-lint-gate] run automated verification
## Backlog
Historical prose may discuss human operator manual interactive desktop listen visual sign-off owner approval sudo credential legal acceptance.
## Done
1. DONE: human operator performed a historical manual desktop check.
EOF
expect_ok "forbidden-token lint ignores historical non-active text" "1"

write_queue <<'EOF'
## Now
1. [READY] [id=required-task] [gate=required-gate] required task
## Optional
2. [READY] [id=optional-task] [gate=optional-gate] optional task
## Backlog
EOF
expect_invalid "rejects an Optional active section/category" 'Optional|section|category'

# Structural and normalization regressions are checked as black-box state
# transitions, without coupling these cases to diagnostic wording.
write_queue <<'EOF'
## Now
1. [BLOCKED-operator-desktop] [id=suffixed-block] [gate=suffixed-gate] blocked task
## Backlog
EOF
expect_deadlocked "suffixed BLOCKED is deadlocked"

write_queue <<'EOF'
## Now
1. [BLOCKED - unattended] [id=annotated-block] [gate=annotated-gate] blocked task
## Backlog
EOF
expect_deadlocked "annotated production BLOCKED is deadlocked"

for malformed_tag in \
  '[READY' \
  '[[READY]]' \
  '[id=broken]]' \
  '\[READY\]' \
  '［READY］'; do
  write_queue <<EOF
## Now
1. $malformed_tag [id=bracket-case] [gate=bracket-gate] malformed brackets
## Backlog
EOF
  expect_deadlocked "malformed brackets: $malformed_tag"
done

for unsafe_probe in \
  '../tools/probe.sh' \
  '/tools/probe.sh' \
  'tools/../probe.sh' \
  'tools/probe.sh;touch'; do
  write_queue <<EOF
## Now
1. [WAITING-AUTOMATIC] [id=unsafe-probe] [gate=unsafe-probe-gate] [probe=$unsafe_probe] [retry=30s] [timeout=20m] wait for probe
## Backlog
EOF
  expect_deadlocked "unsafe probe: $unsafe_probe"
done

for optional_heading in 'Optional-work' 'Optional!' 'Optional: later'; do
  write_queue <<EOF
## Now
1. [READY] [id=required-task] [gate=required-gate] required task
## Backlog
## $optional_heading
EOF
  expect_deadlocked "punctuated Optional heading: $optional_heading"
done

write_queue <<'EOF'
## Now
1. [READY] [id=numbered-continuation] [gate=continuation-gate] run automated steps
   2. collect the second automated artifact
## Backlog
EOF
expect_ok "indented numbered line remains item continuation" "1"
expect_ok "indented numbered line is not a second state item" "RUNNABLE 1" --state-v1

for bad_boundary in ' ## Backlog' '   ## Backlog' '##Backlog'; do
  write_queue <<EOF
## Now
1. [READY] [id=bad-boundary] [gate=bad-boundary-gate] bounded task
$bad_boundary
EOF
  expect_deadlocked "noncanonical active boundary: $bad_boundary"
done

write_queue <<'EOF'
## Now
1. [READY] [id=truncated] [gate=truncated-gate] truncated active queue
EOF
expect_deadlocked "truncated Now section"

write_queue <<'EOF'
## Now
1. [READY] [id=unknown-boundary] [gate=unknown-boundary-gate] task
## Unexpected
EOF
expect_deadlocked "unknown active section boundary"

write_queue <<'EOF'
## Now
1. [READY] [id=done-boundary] [gate=done-boundary-gate] task
## Done
Historical operator prose is outside active task text.
EOF
expect_ok "Done is a canonical active boundary" "1"

for bad_ordinal in 0 01; do
  write_queue <<EOF
## Now
$bad_ordinal. [READY] [id=bad-ordinal] [gate=bad-ordinal-gate] task
## Backlog
EOF
  expect_deadlocked "noncanonical ordinal: $bad_ordinal"
done

write_queue <<'EOF'
## Now
1. [READY] [id=canonical-ordinal] [gate=canonical-ordinal-gate] first task
01. [READY] [id=mixed-ordinal] [gate=mixed-ordinal-gate] equivalent ordinal
## Backlog
EOF
expect_deadlocked "mixed canonical and leading-zero ordinals"

write_queue <<'EOF'
## Now
1. [READY] [id=claimed-ready] [gate=claimed-ready-gate] claimed task
## Backlog
EOF
write_v2_owner 1 claimed-ready claimed-ready-gate
expect_ok "all claimed READY items remain valid in numeric mode" ""
expect_ok "all claimed READY items have explicit state" "CLAIMED-RUNNING" --state-v1

write_queue <<'EOF'
## Now
1. [READY] [id=claimed-ready] [gate=claimed-ready-gate] claimed task
2. [WAITING-AUTOMATIC] [id=mixed-wait] [gate=mixed-wait-gate] [probe=tools/probe.sh] [retry=30s] [timeout=20m] bounded wait
## Backlog
EOF
write_v2_owner 1 claimed-ready claimed-ready-gate
expect_ok "mixed claimed READY and wait has no numeric claim" ""
expect_ok "claimed work takes precedence over automatic wait" "CLAIMED-RUNNING" --state-v1

write_queue <<'EOF'
## Now
1. [READY] [id=split-operator] [gate=split-operator-gate] ask the oper-
   ator to approve the result
## Backlog
EOF
expect_deadlocked "line-wrapped forbidden operator"

write_queue <<'EOF'
## Now
1. [READY] [id=unicode-operator] [gate=unicode-operator-gate] ask the ｏｐｅｒａｔｏｒ to approve
## Backlog
EOF
expect_deadlocked "NFKC-normalized forbidden operator"

write_queue <<'EOF'
## Now
1. [READY] [id=punctuated-operator] [gate=punctuated-operator-gate] ask the oper.ator to approve
## Backlog
EOF
expect_deadlocked "punctuation-normalized forbidden operator"

write_queue <<'EOF'
## Now
1. [READY] [id=punctuated-signature] [gate=punctuated-signature-gate] require owner-signature
## Backlog
EOF
expect_deadlocked "punctuation-normalized owner signature"

write_queue <<'EOF'
## Now
1. [READY] [id=claims-path] [gate=claims-path-gate] validate claims path
## Backlog
EOF
rmdir "$CLAIMS"
expect_deadlocked "missing claims directory"
mkdir "$CLAIMS"

write_queue <<'EOF'
## Now
1. [READY] [id=claims-file] [gate=claims-file-gate] reject claims file
## Backlog
EOF
rmdir "$CLAIMS"
: > "$CLAIMS"
expect_deadlocked "claims path must be a directory"
rm -f "$CLAIMS"
mkdir "$CLAIMS"

# Repository-level migration contract: the live active queue is itself an
# input to the strict scheduler, not exempt historical documentation. Only the
# ## Now slice is scoped here; human/operator prose may remain under Done.
# (watchdog repair 2026-08-28, token 2010676: REQUIRED-QUEUE-EMPTY is the
# parser's own legal TERMINAL verdict — the repo's required queue emptied for
# the first time at 89905f3 with every P0-P7 gate green, and the completion
# contract keeps it empty forever after; the check stays red on every rc!=0
# answer and still forbids BLOCKED/operator/interactive entries in ## Now.)
active_state=$($PARSER "$ROOT/.state/NEXT.md" "$CLAIMS" --state-v1 \
  2>"$TMP/active-queue.err")
active_rc=$?
active_now=$(awk '
  /^## Now[[:space:]]*$/ { active=1; next }
  /^## / && active { exit }
  active { print }
' "$ROOT/.state/NEXT.md")
if [ "$active_rc" -ne 0 ] \
    || [[ "$active_state" != RUNNABLE\ * && "$active_state" != AUTOMATIC-WAIT \
          && "$active_state" != REQUIRED-QUEUE-EMPTY ]] \
    || printf '%s\n' "$active_now" | grep -Eqi 'BLOCKED|operator|interactive|human|manual'; then
  LAST_RC=$active_rc
  LAST_OUT=$active_state
  LAST_ERR=$(cat "$TMP/active-queue.err")
  fail 'repository active queue contains only permitted autonomous state' \
    'strict parser RUNNABLE/AUTOMATIC-WAIT and no BLOCKED/operator/interactive entry in ## Now'
fi

if [ "$failures" -ne 0 ]; then
  printf 'nudge queue tests: FAIL (%d failure(s))\n' "$failures"
  exit 1
fi

echo "nudge queue tests: PASS"
