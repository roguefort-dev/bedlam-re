#!/usr/bin/env bash
# Deterministic red contracts for the remaining autonomous-loop safety gaps.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
CONTROLLER="$ROOT/tools/nudge.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
PARSER="$ROOT/tools/nudge-free-items.py"
STATE_HELPER="$ROOT/tools/nudge-state.py"
WAIT_EXECUTOR="$ROOT/tools/nudge-wait.py"
WATCHDOG="$ROOT/tools/llm-watchdog.sh"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
TMP=$(mktemp -d /tmp/opencode/bedlam-autonomy-gaps.XXXXXX)

cleanup() {
  local pid
  for file in "$TMP"/*.pid; do
    [ -e "$file" ] || continue
    pid=$(cat "$file" 2>/dev/null || true)
    [[ "$pid" =~ ^[1-9][0-9]*$ ]] || continue
    kill -TERM "$pid" 2>/dev/null || true
  done
  jobs -pr | xargs -r kill 2>/dev/null || true
  jobs -pr | xargs -r wait 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT INT TERM

make_repo() {
  local plan=$1
  rm -rf "$plan"
  mkdir -p "$plan/.state/claims" "$plan/tools" "$plan/docs"
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [READY] [id=stable-one] [gate=gate-one] automated fixture task

## Backlog
EOF
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "gate-one"
command = ["/usr/bin/test", "-f", "gate-one.ok"]
timeout_seconds = 2
EOF
  printf '# fixture\n' > "$plan/AGENTS.md"
  printf 'initial\n' > "$plan/code.txt"
  : > "$plan/gate-one.ok"
  git -C "$plan" init -q
  git -C "$plan" config user.email test@example.invalid
  git -C "$plan" config user.name test
  git -C "$plan" add .state/NEXT.md AGENTS.md code.txt docs/required-gates.toml gate-one.ok
  git -C "$plan" commit -qm init
}

write_wait_queue() {
  local plan=$1 metadata=${2:-'[probe=tools/probe.sh] [retry=1s] [timeout=10s]'}
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] $metadata bounded fixture wait

## Backlog
EOF
}

authorize_probe() {
  local plan=$1 id=$2 path=$3 digest
  digest=$(sha256sum "$plan/$path" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "$id"
path = "$path"
sha256 = "$digest"
EOF
  git -C "$plan" add "$path" docs/automatic-probes.toml
  git -C "$plan" commit -qm authorized-probe-fixture
}

publish_bound_claim() {
  local plan=$1 session=$2 destination=${3:-}
  local fields status item_id gate body device inode queue_hash claim
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r status item_id gate body device inode queue_hash <<< "$fields"
  [ "$status" = READY ]
  claim="$plan/.state/claims/1-$session.claim"
  "$STATE_HELPER" publish-claim "$plan/.state/claims" "1-$session.claim" \
    1 "$item_id" "$gate" "$session" "$(date -Is)" \
    "bedlam-nudge-item1-$session" $$ "$body" "$device" "$inode" "$queue_hash"
  if [ -n "$destination" ]; then mv "$claim" "$destination"; fi
}

make_controller_mocks() {
  cat > "$TMP/network-ok" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > "$TMP/record-run" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/runs"
EOF
  cat > "$TMP/systemctl" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/systemctl-calls"
EOF
  chmod +x "$TMP/network-ok" "$TMP/record-run" "$TMP/systemctl"
}

run_controller() {
  local plan=$1 lock=$2
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$REAPER" \
    NOTIFY_SEND= "$CONTROLLER"
}

case_completion_is_controller_validated() {
  local plan="$TMP/completion-work" head gates rc artifact empty="$TMP/completion-empty"
  local corpus_before corpus_after corpus_mode_before corpus_mode_after
  local completion_failures=0
  make_controller_mocks
  make_repo "$plan"
  head=$(git -C "$plan" rev-parse HEAD)
  gates=$(sha256sum "$plan/docs/required-gates.toml" | awk '{print $1}')
  artifact="$plan/.state/PLAN-COMPLETE"
  printf '{"schema":"plan-complete-v1","head":"%s","required_gates_sha256":"%s","offline_validation":{"status":"passed","validated_at_head":"%s"}}\n' \
    "$head" "$gates" "$head" > "$artifact"
  chmod 600 "$artifact"
  : > "$TMP/runs"
  run_controller "$plan" "$TMP/completion-work.lock" || true
  work_ran=$(wc -l < "$TMP/runs")

  make_repo "$empty"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$empty/.state/NEXT.md"
  cp "$ROOT/tools/validate-required-gates.py" "$empty/tools/validate-required-gates.py"
  chmod +x "$empty/tools/validate-required-gates.py"
  # Production layout: MANIFEST is tracked, while the original corpus it
  # authenticates is deliberately ignored/untracked and read-only.  Detached
  # validation must copy that exact corpus into isolation, verify source and
  # copy before/after, and never make the source writable.
  mkdir -p "$empty/game-data"
  printf 'game-data/\n' > "$empty/.gitignore"
  printf 'external corpus bytes\n' > "$empty/game-data/corpus.bin"
  chmod 400 "$empty/game-data/corpus.bin"
  anchor_digest=$(sha256sum "$empty/game-data/corpus.bin" | awk '{print $1}')
  printf '%s  game-data/corpus.bin\n' "$anchor_digest" > "$empty/MANIFEST.sha256"
  cat > "$empty/tools/check-corpus.py" <<PY
#!/usr/bin/python3
import hashlib, os, pathlib
p=pathlib.Path("game-data/corpus.bin")
assert hashlib.sha256(p.read_bytes()).hexdigest() == "$anchor_digest"
assert os.stat(p).st_mode & 0o222 == 0
PY
  chmod +x "$empty/tools/check-corpus.py"
  cat > "$empty/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "external-corpus"
command = ["/usr/bin/python3", "tools/check-corpus.py"]
timeout_seconds = 2
EOF
  git -C "$empty" add .state/NEXT.md .gitignore tools/validate-required-gates.py \
    tools/check-corpus.py docs/required-gates.toml MANIFEST.sha256
  git -C "$empty" commit -qm 'trusted empty completion with external corpus fixture'
  corpus_before=$(sha256sum "$empty/game-data/corpus.bin" | awk '{print $1}')
  corpus_mode_before=$(stat -c %a "$empty/game-data/corpus.bin")
  rm -f "$empty/.state/PLAN-COMPLETE"
  : > "$TMP/systemctl-calls"
  set +e
  run_controller "$empty" "$TMP/completion-empty.lock"
  rc=$?
  set -e
  corpus_after=$(sha256sum "$empty/game-data/corpus.bin" | awk '{print $1}')
  corpus_mode_after=$(stat -c %a "$empty/game-data/corpus.bin")
  if [ "$work_ran" -ne 1 ]; then
    echo 'perfect-looking worker marker suppressed queued work' >&2
    completion_failures=$((completion_failures + 1))
  fi
  if [ "$rc" -ne 0 ] || [ ! -f "$empty/.state/PLAN-COMPLETE" ]; then
    echo 'empty queue did not run controller-owned required-gates validation' >&2
    completion_failures=$((completion_failures + 1))
  elif ! python3 - "$empty/.state/PLAN-COMPLETE" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    value = json.load(handle)
assert value["schema"] == "plan-complete-v1"
assert value["producer"] == "controller"
assert value["offline_validation"]["status"] == "passed"
assert value["offline_validation"]["bounded"] is True
PY
  then
    echo 'controller completion evidence is not bound to its bounded validator' >&2
    completion_failures=$((completion_failures + 1))
  fi
  if grep -q bedlam-llm-watchdog.service "$TMP/systemctl-calls"; then
    echo 'passing offline required gates incorrectly triggered repair' >&2
    completion_failures=$((completion_failures + 1))
  fi
  if [ "$corpus_after" != "$corpus_before" ] || [ "$corpus_mode_after" != "$corpus_mode_before" ]; then
    echo 'detached completion changed the external source corpus or its read-only mode' >&2
    completion_failures=$((completion_failures + 1))
  fi
  [ "$completion_failures" -eq 0 ]
}

case_probe_walk_refuses_replaced_intermediate() {
  local plan="$TMP/probe-walk" outside="$TMP/probe-outside" hook="$TMP/probe-hook" rc
  make_repo "$plan"
  write_wait_queue "$plan" '[probe=tools/nested/deep/probe.sh] [retry=100ms] [timeout=5s]'
  mkdir -p "$plan/tools/nested/deep" "$outside/deep" "$hook"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/nested/deep/probe.sh"
  cat > "$outside/deep/probe.sh" <<EOF
#!/usr/bin/env bash
touch "$TMP/outside-probe-ran"
exit 0
EOF
  chmod +x "$plan/tools/nested/deep/probe.sh" "$outside/deep/probe.sh"
  authorize_probe "$plan" tools/nested/deep/probe.sh tools/nested/deep/probe.sh
  cat > "$hook/sitecustomize.py" <<PY
import os
original_open = os.open
def raced_open(path, flags, *args, **kwargs):
    if str(path) == "nested" and not os.path.exists("$TMP/intermediate-swapped"):
        os.rename("$plan/tools/nested", "$plan/tools/nested.trusted")
        os.symlink("$outside", "$plan/tools/nested")
        open("$TMP/intermediate-swapped", "w").close()
    return original_open(path, flags, *args, **kwargs)
os.open = raced_open
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  rc=$?
  set -e
  [ -e "$TMP/intermediate-swapped" ]
  [ "$rc" -ne 0 ]
  [ ! -e "$TMP/outside-probe-ran" ]
}

case_lock_v2_requires_complete_binding() {
  local plan="$TMP/incomplete-v2" session=incomplete state rc agent_rc
  make_repo "$plan"
  cat > "$plan/.state/claims/1-$session.claim" <<EOF
lock-v2
ordinal=1
id=stable-one
gate=gate-one
owner=worker
session=$session
claimed_at=$(date -Is)
unit=bedlam-nudge-item1-$session
pid=$$
EOF
  chmod 600 "$plan/.state/claims/1-$session.claim"
  set +e
  state=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>"$plan/parser.err")
  rc=$?
  set -e
  cat > "$TMP/must-not-launch" <<EOF
#!/usr/bin/env bash
touch "$TMP/incomplete-launched"
exit 0
EOF
  chmod +x "$TMP/must-not-launch"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/must-not-launch" NUDGE_IDLE_POLL=0.02 "$AGENT" 1 "$session" >/dev/null 2>&1
  agent_rc=$?
  set -e
  [ "$rc" -eq 2 ]
  [ "$state" = INVALID-DEADLOCKED ]
  [ "$agent_rc" -ne 0 ]
  [ ! -e "$TMP/incomplete-launched" ]
}

case_claim_read_is_bound_to_open_inode() {
  local claims="$TMP/claim-swap" path="$TMP/claim-swap/1-owner.claim" hook="$TMP/claim-hook" rc
  mkdir -p "$claims" "$hook"
  cat > "$path" <<EOF
lock-v2
ordinal=1
id=original-id
gate=gate-one
owner=worker
session=swap
claimed_at=$(date -Is)
unit=bedlam-nudge-item1-swap
pid=$$
body_sha256=$(printf a%.0s {1..64})
queue_device=1
queue_inode=1
queue_sha256=$(printf b%.0s {1..64})
EOF
  sed 's/id=original-id/id=replacement-id/' "$path" > "$path.replacement"
  chmod 600 "$path" "$path.replacement"
  # Start the replacement immediately before the helper's authoritative
  # os.open(), not after an already-open fd has made the race irrelevant.
  # The replacement is a symlink, so a real O_NOFOLLOW open must fail closed.
  cat > "$hook/sitecustomize.py" <<PY
import os
original_open = os.open
def raced_open(name, flags, *args, **kwargs):
    if str(name) == "$path" and not os.path.exists("$TMP/claim-path-swapped"):
        os.rename("$path", "$path.original")
        os.symlink("$path.replacement", "$path")
        open("$TMP/claim-path-swapped", "w").close()
    return original_open(name, flags, *args, **kwargs)
os.open = raced_open
PY
  set +e
  PYTHONPATH="$hook" bash -c 'source "$1"; claim_read "$2" 1' bash \
    "$ROOT/tools/nudge-claim.sh" "$path" >/dev/null 2>&1
  rc=$?
  set -e
  [ -e "$TMP/claim-path-swapped" ]
  [ "$rc" -ne 0 ]
  [ -L "$path" ]
}

case_queue_promotion_is_digest_cas() {
  local plan="$TMP/promotion-cas" hook="$TMP/promotion-hook" rc
  make_repo "$plan"
  write_wait_queue "$plan" '[probe=tools/probe.sh] [retry=100ms] [timeout=5s]'
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan" tools/probe.sh tools/probe.sh
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
original_replace = os.replace
def raced_replace(source, destination, *args, **kwargs):
    if str(destination) == "$plan/.state/NEXT.md" and not os.path.exists("$TMP/pre-replace-edit"):
        with open(destination, "a", encoding="utf-8") as handle:
            handle.write("\n# concurrent writer survives\n")
        open("$TMP/pre-replace-edit", "w").close()
    return original_replace(source, destination, *args, **kwargs)
os.replace = raced_replace
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  rc=$?
  set -e
  [ -e "$TMP/pre-replace-edit" ]
  [ "$rc" -ne 0 ]
  grep -q '^# concurrent writer survives$' "$plan/.state/NEXT.md"
  grep -q '\[WAITING-AUTOMATIC\]' "$plan/.state/NEXT.md"
}

case_normal_reservation_uses_queue_lock() {
  local plan="$TMP/queue-lock-writer" session=queue-lock fields status
  make_repo "$plan"
  (
    exec 9>"$plan/.state/.queue.lock"
    flock 9
    : > "$TMP/queue-lock-held"
    while [ ! -e "$TMP/release-queue-lock" ]; do sleep 0.01; done
  ) &
  echo $! > "$TMP/queue-lock-holder.pid"
  for _ in $(seq 1 100); do [ -e "$TMP/queue-lock-held" ] && break; sleep 0.01; done
  "$ROOT/tools/nudge-reserve.sh" "$plan" 1 "$session" bedlam-nudge-item1-$session 1 16 >"$plan/reserve.out" 2>&1 &
  echo $! > "$TMP/reserver.pid"
  # Long enough for the current unlocked path to complete on a loaded host;
  # a correct implementation remains blocked on the already-held lock.
  sleep 1
  [ ! -e "$plan/.state/claims/1-$session.claim" ]
  : > "$TMP/release-queue-lock"
  wait "$(cat "$TMP/reserver.pid")"
  [ -e "$plan/.state/claims/1-$session.claim" ]
}

case_wait_state_binds_complete_configuration() {
  local mode plan state rc failures=0 real now
  now=$(date +%s)
  for mode in cadence attempts deadline monotonic metadata coherent-origin-reset; do
    plan="$TMP/wait-bind-$mode"
    make_repo "$plan"
    write_wait_queue "$plan" '[probe=tools/probe.sh] [retry=2s] [timeout=20s]'
    printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
    chmod +x "$plan/tools/probe.sh"
    authorize_probe "$plan" tools/probe.sh tools/probe.sh
    "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
    state="$plan/.state/automatic-waits/stable-one.json"
    case "$mode" in
      cadence) sed -i 's/retry=2s/retry=3s/' "$plan/.state/NEXT.md" ;;
      metadata) sed -i 's/timeout=20s/timeout=19s/' "$plan/.state/NEXT.md" ;;
      attempts) python3 - "$state" <<'PY'
import json, sys
p=sys.argv[1]; v=json.load(open(p)); v["attempts"]=-99; open(p,"w").write(json.dumps(v)+"\n")
PY
        ;;
      deadline) python3 - "$state" <<PY
import json
p="$state"; v=json.load(open(p)); v["deadline_at"]=$((now + 3600)); open(p,"w").write(json.dumps(v)+"\n")
PY
        ;;
      monotonic) python3 - "$state" <<'PY'
import json, sys
p=sys.argv[1]; v=json.load(open(p)); v["deadline_monotonic"] += 3600; open(p,"w").write(json.dumps(v)+"\n")
PY
        ;;
      coherent-origin-reset)
        # Move every schedule field coherently and reseal it.  A cache-derived
        # origin accepts this unless the absolute deadline/origin is bound to
        # immutable queue metadata rather than to forgeable cache fields.
        sleep 1.2
        python3 - "$state" <<'PY'
import json, sys, time
p=sys.argv[1]; v=json.load(open(p))
now=time.time(); mono=time.monotonic(); duration=20.0
v["started_at"]=now
v["deadline_at"]=now+duration
v["next_attempt_at"]=now+10.0
v["started_monotonic"]=mono
v["deadline_monotonic"]=mono+duration
v["attempts"]=0
for key in ("last_attempt_at", "last_rc"):
    v.pop(key, None)
open(p,"w").write(json.dumps(v)+"\n")
PY
        ;;
    esac
    # The seal is public and therefore not an authenticity boundary. Model an
    # attacker who recomputes it after changing persisted cache fields; the
    # executor must still reject/rederive the schedule from queue metadata.
    python3 - "$state" <<'PY'
import hashlib, json, sys
p = sys.argv[1]
v = json.load(open(p))
v.pop("state_sha256", None)
raw = json.dumps(v, sort_keys=True, separators=(",", ":")).encode()
v["state_sha256"] = hashlib.sha256(raw).hexdigest()
open(p, "w").write(json.dumps(v, sort_keys=True, separators=(",", ":")) + "\n")
PY
    chmod 600 "$state"
    set +e
    "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >"$plan/result" 2>&1
    rc=$?
    set -e
    if [ "$rc" -eq 0 ]; then
      echo "forged wait state accepted: $mode" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_wrapper_initializes_ready_to_waiting() {
  local plan="$TMP/wrapper-wait-init" session=wait-init
  make_repo "$plan"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan" tools/probe.sh tools/probe.sh
  publish_bound_claim "$plan" "$session"
  cat > "$TMP/request-wait" <<EOF
#!/usr/bin/env bash
cat > "$plan/.state/NEXT.md" <<'NEXT'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=tools/probe.sh] [retry=1s] [timeout=10s] wrapper-owned wait request

## Backlog
NEXT
exit 0
EOF
  chmod +x "$TMP/request-wait"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/request-wait" NUDGE_IDLE_POLL=0.02 "$AGENT" 1 "$session" >/dev/null 2>&1
  [ ! -e "$plan/.state/automation-failures/$session.json" ]
  python3 - "$plan/.state/automatic-waits/stable-one.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle: value=json.load(handle)
assert value["state"] == "waiting"
assert len(value["config_sha256"]) == 64
assert value["attempts"] >= 1
PY
}

case_expired_absolute_deadline_reaches_executor() {
  local plan="$TMP/expired-deadline" deadline rc artifact
  make_controller_mocks
  make_repo "$plan"
  deadline=$(date -u -d '-1 minute' '+%Y-%m-%dT%H:%M:%SZ')
  write_wait_queue "$plan" "[probe=tools/probe.sh] [retry=1s] [deadline=$deadline]"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan" tools/probe.sh tools/probe.sh
  : > "$TMP/systemctl-calls"
  set +e
  run_controller "$plan" "$TMP/expired-deadline.lock"
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
  artifact=$(find "$plan/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit 2>/dev/null)
  [ -n "$artifact" ]
  python3 - "$artifact" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle: value=json.load(handle)
assert value["kind"] == "deadline-expired"
assert value["repair"] == "required"
PY
  grep -q 'start bedlam-llm-watchdog.service' "$TMP/systemctl-calls"
}

case_probe_process_group_is_always_reaped() {
  local mode plan child rc failures=0
  for mode in timeout success; do
    plan="$TMP/probe-group-$mode"
    make_repo "$plan"
    write_wait_queue "$plan" '[probe=tools/probe.sh] [retry=100ms] [timeout=5s]'
    cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
    setsid sleep 30 &
echo \$! > "$TMP/probe-$mode.pid"
$( [ "$mode" = timeout ] && echo 'wait' || echo 'exit 0' )
EOF
    chmod +x "$plan/tools/probe.sh"
    authorize_probe "$plan" tools/probe.sh tools/probe.sh
    set +e
    "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
    rc=$?
    set -e
    for _ in $(seq 1 100); do [ -s "$TMP/probe-$mode.pid" ] && break; sleep 0.01; done
    child=$(cat "$TMP/probe-$mode.pid")
    sleep 0.1
    if kill -0 "$child" 2>/dev/null; then
      echo "probe descendant survived $mode (pid=$child executor_rc=$rc)" >&2
      failures=$((failures + 1))
      kill -KILL "$child" 2>/dev/null || true
    fi
  done
  [ "$failures" -eq 0 ]
}

case_model_process_group_is_always_reaped() {
  local mode plan session child rc failures=0 bin="$TMP/model-bin"
  mkdir -p "$bin"
  cat > "$bin/timeout" <<'EOF'
#!/usr/bin/env bash
if [ "$1" = 3900 ]; then shift; exec /usr/bin/timeout -k 0.1s 0.2s "$@"; fi
exec /usr/bin/timeout "$@"
EOF
  chmod +x "$bin/timeout"
  for mode in success outer-timeout; do
    plan="$TMP/model-group-$mode"; session="model-$mode"
    make_repo "$plan"
    publish_bound_claim "$plan" "$session"
    cat > "$TMP/model-$mode" <<EOF
#!/usr/bin/env bash
    setsid sleep 30 &
echo \$! > "$TMP/model-$mode.pid"
$( [ "$mode" = outer-timeout ] && echo 'wait' || echo 'exit 0' )
EOF
    chmod +x "$TMP/model-$mode"
    set +e
    PATH="$bin:$PATH" BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/model-$mode" \
      NUDGE_IDLE_POLL=0.02 "$AGENT" 1 "$session" >/dev/null 2>&1
    rc=$?
    set -e
    child=$(cat "$TMP/model-$mode.pid")
    sleep 0.1
    if kill -0 "$child" 2>/dev/null || [ -e "$plan/.state/claims/1-owner.claim" ]; then
      echo "model descendant/claim survived $mode (pid=$child wrapper_rc=$rc)" >&2
      failures=$((failures + 1))
      kill -KILL "$child" 2>/dev/null || true
      rm -f "$plan/.state/claims/1-owner.claim"
    fi
  done
  [ "$failures" -eq 0 ]
}

write_failure() {
  local plan=$1 name=$2 id=${3:-stable-one} gate=${4:-gate-one} ordinal=${5:-1}
  mkdir -m 700 -p "$plan/.state/automation-failures"
  cat > "$plan/.state/automation-failures/$name.json" <<EOF
{"schema":"nudge-failure-v1","version":1,"ordinal":$ordinal,"id":"$id","gate":"$gate","owner":"worker","session":"$name","kind":"client-error","reason":"fixture","evidence":"fixture","time":"2026-08-26T07:00:00Z","repair":"required","queue_unchanged":true}
EOF
  chmod 600 "$plan/.state/automation-failures/$name.json"
}

make_watchdog_mock() {
  local plan=$1 mode=$2
  cat > "$TMP/watchdog-opencode-$mode" <<EOF
#!/usr/bin/env bash
case "\$*" in
  *bedlam-llm-watchdog-repair*)
    token=\$(cat "$plan/.state/PAUSE")
    case "$mode" in
      unrelated)
        echo unrelated >> "$plan/code.txt"
        ;;
      generic-p4)
        cat > "$plan/.state/NEXT.md" <<'NEXT'
# NEXT

## Now
1. [READY] [id=p4-trigger-contract] [gate=p4-trigger-address] correct and verify the operational trigger contract
2. [READY] [id=p4-static-proof-scope] [gate=p4-s0-dispositions] reconcile deterministic static proof scope
3. [READY] [id=p4-wgpu-final] [gate=p4-dependency-spikes] close the bounded offline dependency decision
4. [READY] [id=p4-required-gates-manifest] [gate=p4-gates-validator] validate the tracked required gates contract
5. [READY] [id=p4-machine-verdict] [gate=p4-machine-verdict] emit the phase-only machine verdict

## Backlog
NEXT
        echo generic >> "$plan/code.txt"
        ;;
    esac
    git -C "$plan" add code.txt
    git -C "$plan" commit -qm repair -m "Watchdog-Repair: \$token"
    ;;
  *) echo WATCHDOG_REPAIR ;;
esac
EOF
  chmod +x "$TMP/watchdog-opencode-$mode"
}

run_watchdog() {
  local plan=$1 mode=$2
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/watchdog-opencode-$mode" \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" NOTIFY_SEND= \
    WATCHDOG_TEST_MODE=1 LLM_WATCHDOG_MIN_INTERVAL=0 SUPERVISE_TIMEOUT=2 REPAIR_TIMEOUT=2 \
    RESUME_WAIT_LOOPS=0 RESUME_WAIT_SLEEP=0 LLM_WATCHDOG_LOCK="$TMP/watchdog-$mode.lock" \
    "$WATCHDOG"
}

case_failure_archive_requires_bound_ack() {
  local plan="$TMP/archive-unrelated" mode ack artifact digest device inode archive_rc remediation_commit
  local archive_failures=0
  make_controller_mocks
  make_repo "$plan"
  write_failure "$plan" unresolved
  make_watchdog_mock "$plan" unrelated
  run_watchdog "$plan" unrelated
  if [ ! -e "$plan/.state/automation-failures/unresolved.json" ]; then
    echo 'unrelated trailer-bearing commit archived an unresolved artifact' >&2
    archive_failures=$((archive_failures + 1))
  fi
  if ! grep -q '^state=repair-no-evidence$' "$plan/.state/llm-watchdog-verdict"; then
    echo 'unrelated commit was accepted as artifact-specific repair evidence' >&2
    archive_failures=$((archive_failures + 1))
  fi

  for mode in required-empty replaced-task; do
    plan="$TMP/archive-$mode"
    make_repo "$plan"
    write_failure "$plan" resolved
    "$STATE_HELPER" snapshot-failures "$plan/.state/automation-failures" "$plan/snapshot.json"
    artifact="$plan/.state/automation-failures/resolved.json"
    digest=$(sha256sum "$artifact" | awk '{print $1}')
    read -r device inode < <(stat -c '%d %i' "$artifact")
    write_failure "$plan" concurrent other-task other-gate 2
    if [ "$mode" = required-empty ]; then
      printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
    else
      sed -i 's/id=stable-one/id=replacement-task/; s/gate=gate-one/gate=replacement-gate/' "$plan/.state/NEXT.md"
    fi
    git -C "$plan" add .state/NEXT.md
    git -C "$plan" commit -qm "mechanical $mode remediation"
    remediation_commit=$(git -C "$plan" rev-parse HEAD)
    ack="$plan/ack.json"
    cat > "$ack" <<EOF
{"schema":"nudge-failure-ack-v1","records":[{"name":"resolved.json","sha256":"$digest","device":$device,"inode":$inode,"ordinal":1,"id":"stable-one","gate":"gate-one","resolution":"$mode","remediation_commit":"$remediation_commit"}]}
EOF
    set +e
    "$STATE_HELPER" archive-failures "$plan/.state/automation-failures" \
      "$plan/snapshot.json" "$plan/.state/NEXT.md" "$ack" "$remediation_commit"
    archive_rc=$?
    set -e
    if [ "$archive_rc" -ne 0 ] || [ -e "$artifact" ]; then
      echo "bound $mode acknowledgement did not archive its exact artifact" >&2
      archive_failures=$((archive_failures + 1))
    fi
    if [ ! -e "$plan/.state/automation-failures/concurrent.json" ]; then
      echo "artifact arriving after the $mode snapshot was archived" >&2
      archive_failures=$((archive_failures + 1))
    fi
    if [ -d "$plan/.state/automation-failures/archive" ] \
        && find "$plan/.state/automation-failures/archive" -name '*concurrent.json' -print -quit | grep -q .; then
      echo "non-snapshot concurrent artifact entered the $mode archive" >&2
      archive_failures=$((archive_failures + 1))
    fi
  done
  [ "$archive_failures" -eq 0 ]
}

case_p4_replacement_enumerates_contract() {
  local plan="$TMP/p4-contract" binding p4_failures=0
  make_controller_mocks
  make_repo "$plan"
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[obligation]]
id = "p4-static-parity"
gate = "p4-static"
resolved = false
[[obligation]]
id = "p4-runtime-capture"
gate = "p4-runtime"
resolved = false
[[obligation]]
id = "p4-determinism"
gate = "p4-dh-g1"
resolved = false
EOF
  git -C "$plan" add docs/required-gates.toml
  git -C "$plan" commit -qm contract
  write_failure "$plan" p4-trigger
  make_watchdog_mock "$plan" generic-p4
  run_watchdog "$plan" generic-p4
  if ! grep -q '^state=repair-no-evidence$' "$plan/.state/llm-watchdog-verdict"; then
    echo 'vague P4 audit replacement was accepted as repair evidence' >&2
    p4_failures=$((p4_failures + 1))
  fi
  if [ ! -e "$plan/.state/automation-failures/p4-trigger.json" ]; then
    echo 'P4 trigger was archived without enumerating its obligations' >&2
    p4_failures=$((p4_failures + 1))
  fi
  for binding in \
    'id=p4-trigger-contract.*gate=p4-trigger-address' \
    'id=p4-static-proof-scope.*gate=p4-s0-dispositions' \
    'id=p4-wgpu-final.*gate=p4-dependency-spikes' \
    'id=p4-required-gates-manifest.*gate=p4-gates-validator' \
    'id=p4-machine-verdict.*gate=p4-machine-verdict'; do
    if ! grep -Eq "$binding" "$plan/.state/NEXT.md"; then
      echo "P4 replacement omitted contract binding: $binding" >&2
      p4_failures=$((p4_failures + 1))
    fi
  done
  if grep -qi 'audit committed evidence' "$plan/.state/NEXT.md"; then
    echo 'P4 replacement retained vague audit language' >&2
    p4_failures=$((p4_failures + 1))
  fi
  [ "$p4_failures" -eq 0 ]
}

case_multiline_metadata_has_exact_outcome() {
  local plan="$TMP/multiline-wait" parse_rc run_rc
  make_repo "$plan"
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one]
   [probe=tools/probe.sh] [retry=100ms] [timeout=5s] continued metadata

## Backlog
EOF
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 >/dev/null 2>&1
  parse_rc=$?
  set -e
  if [ "$parse_rc" -ne 0 ]; then return 0; fi
  set +e
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
  run_rc=$?
  set -e
  [ "$run_rc" -eq 0 ]
  grep -q '^1\. \[READY\].*\[id=stable-one\].*\[gate=gate-one\]' "$plan/.state/NEXT.md"
  ! grep -q '\[WAITING-AUTOMATIC\]' "$plan/.state/NEXT.md"
}

case_full_watchdog_suite_is_bounded() {
  local out="$TMP/full-watchdog.out" rc
  set +e
  timeout --foreground -k 3s 60s bash "$ROOT/tools/test-llm-watchdog.sh" >"$out" 2>&1
  rc=$?
  set -e
  if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
    echo 'full watchdog suite exceeded 60s; likely resume observation was left enabled in a hermetic case' >&2
    tail -40 "$out" >&2
    return 1
  fi
  if [ "$rc" -ne 0 ]; then
    echo "full watchdog suite failed explicitly (rc=$rc)" >&2
    tail -60 "$out" >&2
    return 1
  fi
  grep -q 'llm watchdog tests: PASS' "$out"
}

# A child process executes one case so GNU timeout can kill that case and its
# descendants without losing the parent suite's remaining RED diagnostics.
if [ -n "${AUTONOMY_GAP_CASE:-}" ]; then
  set -e
  "$AUTONOMY_GAP_CASE" "$@"
  exit $?
fi

failures=0
run_case() {
  local name=$1 function=$2 limit=${3:-15}
  shift 3 || true
  local rc
  set +e
  timeout --foreground -k 2s "${limit}s" env AUTONOMY_GAP_CASE="$function" "$0" "$@"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ]; then
    printf 'ok - %s\n' "$name"
  else
    if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
      printf 'not ok - %s (case timeout after %ss; children killed)\n' "$name" "$limit" >&2
    else
      printf 'not ok - %s (rc=%s)\n' "$name" "$rc" >&2
    fi
    failures=$((failures + 1))
  fi
}

run_case 'completion ignores worker marker and controller validates empty queue' case_completion_is_controller_validated 12
run_case 'probe opens every path component from trusted no-follow directory fds' case_probe_walk_refuses_replaced_intermediate 8
run_case 'lock-v2 requires every body/queue binding field before suppress or launch' case_lock_v2_requires_complete_binding 8
run_case 'claim parsing and flock remain on one opened no-follow inode' case_claim_read_is_bound_to_open_inode 8
run_case 'queue promotion revalidates inode and digest at replace CAS' case_queue_promotion_is_digest_cas 8
run_case 'normal reservation shares the queue lock contract' case_normal_reservation_uses_queue_lock 8
run_case 'wait state binds cadence attempts deadline monotonic and full config digest' case_wait_state_binds_complete_configuration 12
run_case 'wrapper atomically initializes a legitimate READY to WAITING schedule' case_wrapper_initializes_ready_to_waiting 10
run_case 'expired absolute deadline reaches executor failure beacon and watchdog' case_expired_absolute_deadline_reaches_executor 8
run_case 'probe process groups are killed and reaped on timeout and success' case_probe_process_group_is_always_reaped 8
run_case 'model process groups are killed and reaped after every exit' case_model_process_group_is_always_reaped 10
run_case 'failure archival needs artifact-bound acknowledgement and exact snapshot' case_failure_archive_requires_bound_ack 15
run_case 'P4 replacement enumerates every unresolved fixture obligation and gate' case_p4_replacement_enumerates_contract 10
run_case 'metadata continuation is rejected or promoted over its exact span' case_multiline_metadata_has_exact_outcome 8
run_case 'full llm-watchdog suite is bounded, explicit, and child-clean' case_full_watchdog_suite_is_bounded 70

if [ "$failures" -ne 0 ]; then
  printf 'autonomy remaining-gap tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'autonomy remaining-gap tests: PASS'
