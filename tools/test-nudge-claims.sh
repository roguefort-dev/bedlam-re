#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
REAPER="$ROOT/tools/nudge-reap-claims.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
TMP=$(mktemp -d /tmp/bedlam-nudge-claims.XXXXXX)
cleanup() {
  jobs -pr | xargs -r kill 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT
cat > "$TMP/mock-notify-send" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" >> "$TMP/notifications"
EOF
chmod +x "$TMP/mock-notify-send"
export NOTIFY_SEND="$TMP/mock-notify-send"
# The 2026-08-21 idle-log reaper polls its supervised client; poll fast
# in tests so the pre-existing short-lived mocks stay quick.
export NUDGE_IDLE_POLL=1
CLAIMS="$TMP/claims"
LOG="$TMP/nudge.log"
mkdir -p "$CLAIMS"
: > "$LOG"

old() { touch -d "@$(( $(date +%s) - $2 ))" "$1"; }
reap() {
  DEAD_CLAIM_TTL=5 RESERVATION_TTL=5 LEGACY_CLAIM_TTL=20 \
    MALFORMED_CLAIM_TTL=${MALFORMED_CLAIM_TTL:-20} \
    "$REAPER" "$CLAIMS" "$LOG"
}

# The worker converts its reservation into a locked, marked owner claim.
PLAN="$TMP/plan"
mkdir -p "$PLAN/.state/claims" "$PLAN/tools" "$PLAN/docs"
printf '#!/usr/bin/env bash\nexit 1\n' > "$PLAN/tools/probe.sh"
chmod +x "$PLAN/tools/probe.sh"
probe_digest=$(sha256sum "$PLAN/tools/probe.sh" | awk '{print $1}')
cat > "$PLAN/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "tools/probe.sh"
path = "tools/probe.sh"
sha256 = "$probe_digest"
EOF
{
  echo "# NEXT"
  echo
  echo "## Now"
  for n in $(seq 4 16); do
    echo "$n. [READY] [id=item-$n] [gate=gate-$n] automated claim test item $n"
  done
  echo
  echo "## Backlog"
} > "$PLAN/.state/NEXT.md"
echo "# STATE" > "$PLAN/.state/STATE.md"
echo initial > "$PLAN/code.txt"
git -C "$PLAN" init -q
git -C "$PLAN" config user.email test@example.invalid
git -C "$PLAN" config user.name test
git -C "$PLAN" add .state/NEXT.md .state/STATE.md code.txt tools/probe.sh docs/automatic-probes.toml
git -C "$PLAN" commit -qm init
reserve_v2() {
  local ordinal=$1 session=$2 item_id=${3:-item-$1} gate=${4:-gate-$1} fields status parsed_id parsed_gate body dev ino queue
  fields=$($ROOT/tools/nudge-free-items.py "$PLAN/.state/NEXT.md" "$PLAN/.state/claims" --item-v2 "$ordinal")
  read -r status parsed_id parsed_gate body dev ino queue <<< "$fields"
  cat > "$PLAN/.state/claims/$ordinal-$session.claim" <<EOF
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
}
migration_failures=0
cat > "$TMP/mock-client" <<EOF
#!/usr/bin/env bash
printf "%s\n" "\$*" > "$TMP/mock-client.args"
sleep 1
EOF
chmod +x "$TMP/mock-client"
reserve_v2 4 789
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-client" "$AGENT" 4 789 &
agent=$!
for _ in $(seq 1 50); do
  [ -e "$PLAN/.state/claims/4-owner.claim" ] && ! flock -n "$PLAN/.state/claims/4-owner.claim" true 2>/dev/null && break
  sleep 0.02
done
grep -qx "lock-v2" "$PLAN/.state/claims/4-owner.claim"
grep -qx "ordinal=4" "$PLAN/.state/claims/4-owner.claim"
grep -qx "id=item-4" "$PLAN/.state/claims/4-owner.claim"
grep -qx "gate=gate-4" "$PLAN/.state/claims/4-owner.claim"
grep -qx "owner=worker" "$PLAN/.state/claims/4-owner.claim"
grep -qx "session=789" "$PLAN/.state/claims/4-owner.claim"
grep -Eq '^claimed_at=.*' "$PLAN/.state/claims/4-owner.claim"
if grep -q '^lock-v1 ' "$PLAN/.state/claims/4-owner.claim"; then
  echo "not ok - a newly published owner claim still contains lock-v1" >&2
  migration_failures=$((migration_failures + 1))
fi
if flock -n "$PLAN/.state/claims/4-owner.claim" true; then
  echo "worker owner claim was not locked" >&2
  exit 1
fi
wait "$agent"
grep -q -- "--standalone" "$TMP/mock-client.args"
grep -qF -- "--model zai-coding-plan/glm-5.3#high" "$TMP/mock-client.args"
grep -q -- "item 4" "$TMP/mock-client.args"
if ! grep -q -- "item-4" "$TMP/mock-client.args" || ! grep -q -- "gate-4" "$TMP/mock-client.args"; then
  echo "not ok - worker prompt omits the stable queue id/gate" >&2
  migration_failures=$((migration_failures + 1))
fi
# Generated task instructions are automation-only. Historical assertions and
# diagnostics may discuss these categories; the prompt sent to the model may not.
if grep -Eqi '(^|[^[:alnum:]])(BLOCKED|human|operator|manual|interactive|desktop|sudo|credentials?|secrets?|legal|license)([^[:alnum:]]|$)|stand(ing)?[- ]down|ask (a |the )?.*(action|input|approval)|wait for input' "$TMP/mock-client.args"; then
  echo "not ok - worker prompt contains a human-only/stand-down instruction token" >&2
  migration_failures=$((migration_failures + 1))
fi
! grep -q -- "release your placeholder" "$TMP/mock-client.args"
[ ! -e "$PLAN/.state/claims/4-owner.claim" ]

# Keep the structured-failure fixture adjacent to the claim/prompt probes so a
# RED run reports all three migration gaps instead of stopping at lock-v1.
cat > "$TMP/mock-focused-inability" <<'EOF'
#!/usr/bin/env bash
exit 42
EOF
chmod +x "$TMP/mock-focused-inability"
cp "$PLAN/.state/NEXT.md" "$TMP/NEXT.before-focused-inability"
reserve_v2 16 focused-inability
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-focused-inability" "$AGENT" 16 focused-inability
focused_rc=$?
set -e
[ "$focused_rc" -eq 42 ]
cmp -s "$TMP/NEXT.before-focused-inability" "$PLAN/.state/NEXT.md"
if ! python3 - "$PLAN/.state/automation-failures/focused-inability.json" 2>/dev/null <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    failure = json.load(handle)
assert failure["schema"] == "nudge-failure-v1"
assert failure["ordinal"] == 16
assert failure["id"] == "item-16"
assert failure["gate"] == "gate-16"
assert failure["kind"] == "client-error"
assert failure["repair"] == "required"
assert failure["queue_unchanged"] is True
PY
then
  echo "not ok - unexpected inability produced no structured automatic-repair artifact" >&2
  migration_failures=$((migration_failures + 1))
fi
[ ! -e "$PLAN/.state/claims/16-owner.claim" ]

if [ "$migration_failures" -ne 0 ]; then
  echo "nudge claim migration tests: RED ($migration_failures missing behavior(s))" >&2
  exit 1
fi

# A normal transport failure has no live ghost and retains a retry-backoff claim.
cat > "$TMP/mock-transport" <<EOF
#!/usr/bin/env bash
echo "Error: ECONNRESET: The socket connection was closed unexpectedly"
exit 1
EOF
chmod +x "$TMP/mock-transport"
reserve_v2 5 790
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-transport" "$AGENT" 5 790
rc=$?
set -e
[ "$rc" -eq 1 ]
[ ! -e "$PLAN/.state/claims/5-owner.claim" ]
grep -q "failed \[transport rc=1 progress=0\] task=.*; provider-side, not charged to the task" "$PLAN/.state/nudge.log"
transport_hash=$(sed -n "s/^[[:space:]]*5\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
[ ! -e "$PLAN/.state/taskfails/$transport_hash" ]
[ -e "$PLAN/.state/taskfails/.transport-streak" ]
[ ! -e "$PLAN/.state/taskcooldown/$transport_hash" ]

# A transport failure with a surviving child is process-group cleaned and the
# claim is released only after every descendant is gone.
cat > "$TMP/mock-ghost" <<EOF
#!/usr/bin/env bash
sleep 30 &
echo \$! > "$TMP/ghost.pid"
echo "Error: Transport"
exit 1
EOF
chmod +x "$TMP/mock-ghost"
reserve_v2 6 791
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-ghost" "$AGENT" 6 791
rc=$?
set -e
[ "$rc" -eq 1 ]
[ ! -e "$PLAN/.state/claims/6-owner.claim" ]
! kill -0 "$(cat "$TMP/ghost.pid")" 2>/dev/null

# A clean client with no substantive commit is a failed no-progress run.
# Transport failures no longer charge the task (items 5/6 above), so the
# three-strike notification + cooldown now needs three genuinely
# task-attributable failures; the first strike alone must stay silent.
cat > "$TMP/mock-no-progress" <<EOF
#!/usr/bin/env bash
echo other-worker >> "$PLAN/code.txt"
git -C "$PLAN" add code.txt
git -C "$PLAN" commit -qm other-worker -m "Nudge-Worker: 999"
exit 0
EOF
chmod +x "$TMP/mock-no-progress"
nop_hash=$(sed -n "s/^[[:space:]]*8\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
for nop_slot in 803 812 813; do
  reserve_v2 8 "$nop_slot"
  set +e
  BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-no-progress" "$AGENT" 8 "$nop_slot"
  nop_rc=$?
  set -e
  [ "$nop_rc" -eq 0 ]
  [ ! -e "$PLAN/.state/claims/8-owner.claim" ]
  flock -n "$PLAN/.state/claims/8-owner.claim" true
  rm -f "$PLAN/.state/claims/8-owner.claim"
  grep -q "failed \[no-progress rc=0 progress=0\]" "$PLAN/.state/nudge.log"
  if [ "$nop_slot" = 803 ]; then
    [ "$(cat "$PLAN/.state/taskfails/$nop_hash" 2>/dev/null)" = "1" ]
    ! grep -q "item 8 failed three consecutive" "$TMP/notifications"
  fi
done
grep -q "item 8 failed three consecutive observed runs" "$TMP/notifications"
[ "$(cat "$PLAN/.state/taskfails/$nop_hash")" = "3" ]
[ ! -e "$PLAN/.state/taskcooldown/$nop_hash" ]

# A step-cap truncation (opencode2 "Maximum steps" kill, rc=0, no
# commit) is NOT a task failure: no taskfails bookkeeping, no
# cooldown spiral, no "failed [" line - just the truncation note and
# a retained retry-backoff claim (freed by the reaper after the
# backoff TTL, like any failed run).
cat > "$TMP/mock-step-cap" <<EOF
#!/usr/bin/env bash
echo "**Maximum steps for this agent reached - stopping with a text-only summary.**"
exit 0
EOF
chmod +x "$TMP/mock-step-cap"
stepcap_hash=$(sed -n "s/^[[:space:]]*10\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
reserve_v2 10 805
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-step-cap" "$AGENT" 10 805
stepcap_rc=$?
set -e
[ "$stepcap_rc" -eq 0 ]
grep -q "item 10 hit the opencode2 step cap \[rc=0 progress=0\] task=$stepcap_hash; treating as truncation, not failure" "$PLAN/.state/nudge.log"
! grep -q "agent item 10 failed \[" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$stepcap_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$stepcap_hash" ]
[ ! -e "$PLAN/.state/claims/10-owner.claim" ]

# The 2026-08-20 provider-incident signature (opencode2 dying on an
# unparseable provider stream event) is classified transport - not
# client-error - and is never charged to the task.
cat > "$TMP/mock-stream-invalid" <<EOF
#!/usr/bin/env bash
echo "Error: Invalid zai-coding-plan/openai-compatible-chat stream event"
exit 1
EOF
chmod +x "$TMP/mock-stream-invalid"
stream_hash=$(sed -n "s/^[[:space:]]*11\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
reserve_v2 11 820
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-stream-invalid" "$AGENT" 11 820
stream_rc=$?
set -e
[ "$stream_rc" -eq 1 ]
grep -q "failed \[transport rc=1 progress=0\] task=$stream_hash; provider-side, not charged to the task" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$stream_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$stream_hash" ]
[ ! -e "$PLAN/.state/claims/11-owner.claim" ]

# The 2026-08-21 hang signature (opencode2 prints "Error: Transport"
# and then never exits - zero CPU, frozen agent log, epoll-wait) must
# not burn the whole 65-minute slot budget: the wrapper's idle-log
# reaper terminates the hung client quickly and the run classifies
# provider-side transport - never charged to the task - with the claim
# retained for the normal retry backoff.
cat > "$TMP/mock-hang" <<EOF
#!/usr/bin/env bash
echo "Error: Transport"
exec sleep 600
EOF
chmod +x "$TMP/mock-hang"
hang_hash=$(sed -n "s/^[[:space:]]*12\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
reserve_v2 12 821
hang_start=$(date +%s)
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-hang" NUDGE_IDLE_LIMIT=3 NUDGE_IDLE_POLL=1 "$AGENT" 12 821
hang_rc=$?
set -e
[ "$hang_rc" -ne 0 ]
[ $(( $(date +%s) - hang_start )) -lt 60 ]
grep -q "idle-log reaper: item 12 agent log silent" "$PLAN/.state/nudge.log"
grep -q "failed \[transport rc=$hang_rc progress=0\] task=$hang_hash; provider-side, not charged to the task (idle-log reaper)" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$hang_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$hang_hash" ]
[ ! -e "$PLAN/.state/claims/12-owner.claim" ]

# The silent variant of the same hang (client emits nothing at all,
# not even an error line) is also reaped and classified provider-side
# via the explicit reaped branch rather than any log signature.
cat > "$TMP/mock-hang-silent" <<EOF
#!/usr/bin/env bash
exec sleep 600
EOF
chmod +x "$TMP/mock-hang-silent"
silent_hash=$(sed -n "s/^[[:space:]]*13\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
reserve_v2 13 822
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-hang-silent" NUDGE_IDLE_LIMIT=3 NUDGE_IDLE_POLL=1 "$AGENT" 13 822
silent_rc=$?
set -e
[ "$silent_rc" -ne 0 ]
grep -q "idle-log reaper: item 13 agent log silent" "$PLAN/.state/nudge.log"
grep -q "failed \[transport rc=$silent_rc progress=0\] task=$silent_hash; provider-side, not charged to the task (idle-log reaper)" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$silent_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$silent_hash" ]
[ ! -e "$PLAN/.state/claims/13-owner.claim" ]

# The 2026-08-21 provider-incident signature (opencode2 dying on
# "Provider request failed with HTTP 502") is classified transport -
# not client-error - and is never charged to the task. Before the
# fix both 502 deaths were mislabeled client-error and charged the
# task twice, one fail away from the 3-strike cooldown spiral.
cat > "$TMP/mock-5xx" <<EOF
#!/usr/bin/env bash
echo "Error: Provider request failed with HTTP 502"
exit 1
EOF
chmod +x "$TMP/mock-5xx"
fivexx_hash=$(sed -n "s/^[[:space:]]*14\.[[:space:]]*//p" "$PLAN/.state/NEXT.md" | head -n 1 | sha256sum | cut -c1-16)
reserve_v2 14 823
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-5xx" "$AGENT" 14 823
fivexx_rc=$?
set -e
[ "$fivexx_rc" -eq 1 ]
grep -q "failed \[transport rc=1 progress=0\] task=$fivexx_hash; provider-side, not charged to the task" "$PLAN/.state/nudge.log"
[ ! -e "$PLAN/.state/taskfails/$fivexx_hash" ]
[ ! -e "$PLAN/.state/taskcooldown/$fivexx_hash" ]
[ ! -e "$PLAN/.state/claims/14-owner.claim" ]

# An unexpected task inability never rewrites or retags the required queue.
# It emits a structured failure record for the automatic repair path instead.
cat > "$TMP/mock-inability" <<EOF
#!/usr/bin/env bash
exit 42
EOF
chmod +x "$TMP/mock-inability"
cp "$PLAN/.state/NEXT.md" "$TMP/NEXT.before-inability"
reserve_v2 15 824
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-inability" "$AGENT" 15 824
inability_rc=$?
set -e
[ "$inability_rc" -eq 42 ]
cmp -s "$TMP/NEXT.before-inability" "$PLAN/.state/NEXT.md"
python3 - "$PLAN/.state/automation-failures/824.json" <<'PY'
import datetime as dt
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    failure = json.load(handle)
assert failure["schema"] == "nudge-failure-v1"
assert failure["ordinal"] == 15
assert failure["id"] == "item-15"
assert failure["gate"] == "gate-15"
assert failure["owner"] == "worker"
assert failure["session"] == "824"
assert failure["kind"] == "client-error"
assert failure["repair"] == "required"
assert failure["queue_unchanged"] is True
dt.datetime.fromisoformat(failure["time"].replace("Z", "+00:00"))
PY
[ ! -e "$PLAN/.state/claims/15-owner.claim" ]

# A substantive commit is credited only with this wrappers exact trailer.
cat > "$TMP/mock-own-progress" <<EOF
#!/usr/bin/env bash
echo own-worker >> "$PLAN/code.txt"
git -C "$PLAN" add code.txt
git -C "$PLAN" commit -qm own-worker -m "Nudge-Worker: 804"
EOF
chmod +x "$TMP/mock-own-progress"
reserve_v2 9 804
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-own-progress" "$AGENT" 9 804
[ ! -e "$PLAN/.state/claims/9-owner.claim" ]
grep -q "item 9 ended cleanly (rc=0 progress=1)" "$PLAN/.state/nudge.log"

# Canonical owner publication is atomic: exactly one same-item client starts.
cat > "$TMP/mock-race" <<EOF
#!/usr/bin/env bash
echo started >> "$TMP/race.starts"
sleep 1
EOF
chmod +x "$TMP/mock-race"
reserve_v2 7 801
reserve_v2 7 802
set +e
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-race" "$AGENT" 7 801 & a=$!
BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-race" "$AGENT" 7 802 & b=$!
wait "$a"; ar=$?
wait "$b"; br=$?
set -e
[ "$(wc -l < "$TMP/race.starts")" -eq 1 ]
{ [ "$ar" -eq 0 ] && [ "$br" -eq 75 ]; } || { [ "$ar" -eq 75 ] && [ "$br" -eq 0 ]; }
[ ! -e "$PLAN/.state/claims/7-owner.claim" ]

# An abandoned startup reservation expires quickly.
echo reserved > "$CLAIMS/1-123.claim"
old "$CLAIMS/1-123.claim" 6
reap
[ ! -e "$CLAIMS/1-123.claim" ]

# A lock-v1 owner remains claimed regardless of age while its lock is live.
echo "lock-v1 worker test owns queue item 2" > "$CLAIMS/2-owner.claim"
(
  exec 8>>"$CLAIMS/2-owner.claim"
  flock 8
  : > "$TMP/locked"
  sleep 30
) &
locker=$!
for _ in $(seq 1 50); do [ -e "$TMP/locked" ] && break; sleep 0.02; done
[ -e "$TMP/locked" ]
old "$CLAIMS/2-owner.claim" 60
reap
[ -e "$CLAIMS/2-owner.claim" ]
[ $(( $(date +%s) - $(stat -c %Y "$CLAIMS/2-owner.claim") )) -le 2 ]

# Once its worker dies, the same claim expires after the dead-worker grace.
kill "$locker"
wait "$locker" 2>/dev/null || true
old "$CLAIMS/2-owner.claim" 4
reap
[ -e "$CLAIMS/2-owner.claim" ]
old "$CLAIMS/2-owner.claim" 6
reap
[ ! -e "$CLAIMS/2-owner.claim" ]

# lock-v2 has the same advisory-lock liveness guarantee while additionally
# binding the ordinal to stable queue identity. A valid live v2 owner must not
# be reaped or attributed to the filename alone.
cat > "$CLAIMS/20-owner.claim" <<EOF
lock-v2
ordinal=20
id=stable-twenty
gate=gate-twenty
owner=worker
session=session-twenty
claimed_at=$(date -Is)
unit=bedlam-nudge-item20-session-twenty
pid=$$
body_sha256=$(printf a%.0s {1..64})
queue_device=1
queue_inode=1
queue_sha256=$(printf b%.0s {1..64})
EOF
(
  exec 8>>"$CLAIMS/20-owner.claim"
  flock 8
  : > "$TMP/locked-v2"
  sleep 30
) &
locker_v2=$!
for _ in $(seq 1 50); do [ -e "$TMP/locked-v2" ] && break; sleep 0.02; done
[ -e "$TMP/locked-v2" ]
old "$CLAIMS/20-owner.claim" 60
reap
[ -e "$CLAIMS/20-owner.claim" ]
grep -qx "ordinal=20" "$CLAIMS/20-owner.claim"
grep -qx "id=stable-twenty" "$CLAIMS/20-owner.claim"
grep -qx "gate=gate-twenty" "$CLAIMS/20-owner.claim"
[ $(( $(date +%s) - $(stat -c %Y "$CLAIMS/20-owner.claim") )) -le 2 ]
kill "$locker_v2"
wait "$locker_v2" 2>/dev/null || true
old "$CLAIMS/20-owner.claim" 6
reap
[ ! -e "$CLAIMS/20-owner.claim" ]

# A malformed v2 body is retained while actively locked, but an unlocked
# malformed claim has a bounded grace and cannot wedge the queue forever.
cat > "$CLAIMS/21-owner.claim" <<'EOF'
lock-v2
ordinal=22
id=wrong-binding
owner=worker
session=malformed-v2
claimed_at=not-a-time
EOF
(
  exec 8>>"$CLAIMS/21-owner.claim"
  flock 8
  : > "$TMP/malformed-v2-locked"
  sleep 30
) &
malformed_locker=$!
for _ in $(seq 1 50); do [ -e "$TMP/malformed-v2-locked" ] && break; sleep 0.02; done
old "$CLAIMS/21-owner.claim" 60
MALFORMED_CLAIM_TTL=0 reap
[ -e "$CLAIMS/21-owner.claim" ]
grep -Eqi 'malformed.*lock-v2|lock-v2.*malformed|invalid.*lock-v2' "$LOG"
kill "$malformed_locker"
wait "$malformed_locker" 2>/dev/null || true
old "$CLAIMS/21-owner.claim" 60
MALFORMED_CLAIM_TTL=5 reap
if [ -e "$CLAIMS/21-owner.claim" ]; then
  echo "not ok - stale unlocked malformed lock-v2 claim wedged past bounded grace" >&2
  exit 1
fi

# Pre-lock claims retain the conservative migration timeout.
echo "worker legacy owns queue item 3" > "$CLAIMS/3-owner.claim"
old "$CLAIMS/3-owner.claim" 6
reap
[ -e "$CLAIMS/3-owner.claim" ]
old "$CLAIMS/3-owner.claim" 21
reap
[ ! -e "$CLAIMS/3-owner.claim" ]

# The worker must perform its own launch-boundary proof. A scheduler decision is
# stale if ordinal/id/gate/READY or the still-owned v2 claim no longer agrees
# with a fresh strict parse of NEXT. Every mismatch exits before model launch,
# leaves NEXT byte-identical, and emits machine-readable repair status.
cat > "$TMP/mock-must-not-launch" <<EOF
#!/usr/bin/env bash
touch "$TMP/model-launched"
exit 0
EOF
chmod +x "$TMP/mock-must-not-launch"

write_single_queue() {
  local status=$1 item_id=$2 gate=$3 extra=${4:-}
  cat > "$PLAN/.state/NEXT.md" <<EOF
# NEXT

## Now
16. [$status] [id=$item_id] [gate=$gate] $extra automated preflight item

## Backlog
EOF
}

expect_preflight_rejection() {
  local name=$1 session=$2 expected_reason=${3:-}
  rm -f "$TMP/model-launched" "$PLAN/.state/automation-failures/$session.json"
  cp "$PLAN/.state/NEXT.md" "$TMP/NEXT.before-$session"
  set +e
  BEDLAM_PLAN_DIR="$PLAN" OPENC_OVERRIDE="$TMP/mock-must-not-launch" "$AGENT" 16 "$session"
  local rejection_rc=$?
  set -e
  if [ "$rejection_rc" -eq 0 ] || [ -e "$TMP/model-launched" ]; then
    echo "$name: stale identity launched the model (rc=$rejection_rc)" >&2
    exit 1
  fi
  cmp -s "$TMP/NEXT.before-$session" "$PLAN/.state/NEXT.md"
  python3 - "$PLAN/.state/automation-failures/$session.json" "$expected_reason" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    failure = json.load(handle)
assert failure["schema"] == "nudge-failure-v1"
assert failure["kind"] == "preflight-mismatch"
assert failure["repair"] == "required"
assert failure["queue_unchanged"] is True
if sys.argv[2]:
    assert failure["reason"] == sys.argv[2]
PY
  [ ! -e "$PLAN/.state/claims/16-owner.claim" ]
}

write_single_queue READY item-16 gate-16
reserve_v2 16 mismatch-id wrong-id gate-16
expect_preflight_rejection "id mismatch" mismatch-id

write_single_queue READY item-16 gate-16
reserve_v2 16 mismatch-gate item-16 wrong-gate
expect_preflight_rejection "gate mismatch" mismatch-gate

write_single_queue WAITING-AUTOMATIC item-16 gate-16 \
  '[probe=tools/probe.sh] [retry=30s] [timeout=20m]'
reserve_v2 16 mismatch-status item-16 gate-16
expect_preflight_rejection "non-READY status" mismatch-status status-mismatch

write_single_queue READY item-16 gate-16
reserve_v2 16 mismatch-ordinal item-16 gate-16
sed -i 's/^ordinal=16$/ordinal=17/' "$PLAN/.state/claims/16-mismatch-ordinal.claim"
expect_preflight_rejection "ordinal mismatch" mismatch-ordinal

write_single_queue READY item-16 gate-16
reserve_v2 16 mismatch-session item-16 gate-16
sed -i 's/^session=mismatch-session$/session=other-session/' "$PLAN/.state/claims/16-mismatch-session.claim"
expect_preflight_rejection "claim/session mismatch" mismatch-session

# The real operating contract must not regress to TUI/process ownership.
grep -q "Process liveness is NEVER ownership evidence" "$ROOT/AGENTS.md"
! grep -q "read them first" "$ROOT/.state/NEXT.md"
! grep -q "release your placeholder" "$AGENT"
# v5: working-tree mtimes are no longer progress evidence.
! grep -q "newermt" "$ROOT/tools/nudge.sh"
grep -q "taskfails" "$ROOT/tools/nudge-agent.sh"

echo "nudge claim tests: PASS"
