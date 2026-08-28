#!/usr/bin/env bash
# Adversarial contract tests for lock-v2 publication, launch, and failure paths.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
AGENT="$ROOT/tools/nudge-agent.sh"
CONTROLLER="$ROOT/tools/nudge.sh"
PARSER="$ROOT/tools/nudge-free-items.py"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
TMP=$(mktemp -d /tmp/opencode/bedlam-lock-v2-adversarial.XXXXXX)
trap 'jobs -pr | xargs -r kill 2>/dev/null || true; rm -rf "$TMP"' EXIT
failures=0

run_case() {
  local name=$1
  shift
  ( set -e; "$@" )
  local rc=$?
  if [ "$rc" -eq 0 ]; then
    printf 'ok - %s\n' "$name"
  else
    printf 'not ok - %s\n' "$name" >&2
    failures=$((failures + 1))
  fi
}

make_repo() {
  local plan=$1
  rm -rf "$plan"
  mkdir -p "$plan/.state/claims" "$plan/tools" "$plan/docs"
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [READY] [id=stable-one] [gate=gate-one] automated test item

## Backlog
EOF
  printf '# fixture contract\n' > "$plan/AGENTS.md"
  printf '# Required gates\n\n- gate-one\n' > "$plan/docs/PLAN.md"
  printf 'initial\n' > "$plan/code.txt"
  git -C "$plan" init -q
  git -C "$plan" config user.email test@example.invalid
  git -C "$plan" config user.name test
  git -C "$plan" add .state/NEXT.md AGENTS.md code.txt docs/PLAN.md
  git -C "$plan" commit -qm init
}

write_v2() {
  local file=$1 session=$2 id=${3:-stable-one} gate=${4:-gate-one} plan fields status parsed_id parsed_gate body dev ino queue
  plan=$(cd "$(dirname "$file")/../.." && pwd)
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r status parsed_id parsed_gate body dev ino queue <<< "$fields"
  cat > "$file" <<EOF
lock-v2
ordinal=1
id=$id
gate=$gate
owner=worker
session=$session
claimed_at=$(date -Is)
unit=bedlam-nudge-item1-$session
pid=$$
body_sha256=$body
queue_device=$dev
queue_inode=$ino
queue_sha256=$queue
EOF
}

controller_env() {
  local plan=$1 run=$2 network=$3 lock=$4
  env BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$lock" \
    SYSTEMD_RUN_OVERRIDE="$run" NETWORK_WATCHDOG_OVERRIDE="$network" \
    REAPER_OVERRIDE="$REAPER" NOTIFY_SEND= \
    "$CONTROLLER"
}

case_partial_publication() {
  local plan="$TMP/partial-plan" hook="$TMP/publication-hook"
  make_repo "$plan"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
import time

original_link = os.link
def barrier_link(source, destination, *args, **kwargs):
    if str(destination).endswith("1-publication.claim"):
        open("$TMP/publication-entered", "w").close()
        while not os.path.exists("$TMP/publication-release"):
            time.sleep(0.01)
    return original_link(source, destination, *args, **kwargs)
os.link = barrier_link
PY
  cat > "$TMP/no-network" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  cat > "$TMP/record-run" <<EOF
#!/usr/bin/env bash
echo run >> "$TMP/runs"
EOF
  chmod +x "$TMP/no-network" "$TMP/record-run"
  read -r _ _ _ body dev ino queue < <($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  PYTHONPATH="$hook" "$ROOT/tools/nudge-state.py" publish-claim \
    "$plan/.state/claims" 1-publication.claim 1 stable-one gate-one \
    publication "$(date -Is)" bedlam-nudge-item1-publication $$ "$body" "$dev" "$ino" "$queue" &
  local pid=$!
  for _ in $(seq 1 300); do [ -e "$TMP/publication-entered" ] && break; sleep 0.01; done
  [ -e "$TMP/publication-entered" ]
  [ ! -e "$plan/.state/claims/1-publication.claim" ]
  [ -z "$(find "$plan/.state/claims" -maxdepth 1 -name '*.claim' -print -quit)" ]
  : > "$TMP/publication-release"
  wait "$pid"
  grep -qx lock-v2 "$plan/.state/claims/1-publication.claim"
  grep -qx pid=$$ "$plan/.state/claims/1-publication.claim"
}

case_lock_path_refuses_symlink() {
  local kind=$1 plan="$TMP/lock-$1" sentinel="$TMP/lock-$1-sentinel"
  make_repo "$plan"
  printf 'DO-NOT-TRUNCATE\n' > "$sentinel"
  : > "$TMP/runs"
  case "$kind" in
    controller)
      ln -s "$sentinel" "$TMP/controller-symlink.lock"
      controller_env "$plan" "$TMP/record-run" "$TMP/no-network" "$TMP/controller-symlink.lock" >/dev/null 2>&1 || true
      ;;
    publication)
      ln -s "$sentinel" "$plan/.state/claims/.publish.lock"
      controller_env "$plan" "$TMP/record-run" "$TMP/no-network" "$TMP/publication-controller.lock" >/dev/null 2>&1 || true
      ;;
    watchdog)
      ln -s "$sentinel" "$TMP/watchdog-symlink.lock"
      BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false WATCHDOG_TEST_MODE=1 \
        LLM_WATCHDOG_MIN_INTERVAL=0 LLM_WATCHDOG_LOCK="$TMP/watchdog-symlink.lock" \
        NOTIFY_SEND= "$ROOT/tools/llm-watchdog.sh" >/dev/null 2>&1 || true
      ;;
    network)
      ln -s "$sentinel" "$TMP/network-symlink.lock"
      BEDLAM_PLAN_DIR="$plan" CURL_BIN=/bin/true \
        NETWORK_WATCHDOG_LOCK="$TMP/network-symlink.lock" \
        "$ROOT/tools/network-watchdog.sh" >/dev/null 2>&1 || true
      ;;
    executor)
      mkdir -m 700 "$plan/.state/automatic-waits"
      ln -s "$sentinel" "$plan/.state/automatic-waits/.executor.lock"
      "$ROOT/tools/nudge-wait.py" run "$plan/.state/NEXT.md" \
        "$plan/.state/automatic-waits" >/dev/null 2>&1 || true
      ;;
  esac
  [ "$(cat "$sentinel")" = DO-NOT-TRUNCATE ]
  [ ! -s "$TMP/runs" ]
}

case_invalid_completion_artifact_does_not_stop() {
  local mode=$1 plan="$TMP/completion-$1" artifact outside="$TMP/completion-outside-$1"
  make_repo "$plan"
  artifact="$plan/.state/PLAN-COMPLETE"
  case "$mode" in
    symlink)
      printf 'arbitrary outside data\n' > "$outside"
      ln -s "$outside" "$artifact"
      ;;
    arbitrary) printf 'done because a caller said so\n' > "$artifact" ;;
    stale-head)
      printf '{"schema":"plan-complete-v1","head":"0000000000000000000000000000000000000000","required_gates_sha256":"deadbeef","offline_validation":{"status":"passed","validated_at_head":"stale"}}\n' > "$artifact"
      ;;
    stale-gates)
      printf '{"schema":"plan-complete-v1","head":"%s","required_gates_sha256":"deadbeef","offline_validation":{"status":"passed","validated_at_head":"%s"}}\n' "$(git -C "$plan" rev-parse HEAD)" "$(git -C "$plan" rev-parse HEAD)" > "$artifact"
      ;;
    validation-failed)
      printf '{"schema":"plan-complete-v1","head":"%s","required_gates_sha256":"%s","offline_validation":{"status":"failed","validated_at_head":"%s"}}\n' "$(git -C "$plan" rev-parse HEAD)" "$(sha256sum "$plan/docs/PLAN.md" | awk '{print $1}')" "$(git -C "$plan" rev-parse HEAD)" > "$artifact"
      ;;
  esac
  : > "$TMP/runs"
  controller_env "$plan" "$TMP/record-run" "$TMP/no-network" "$TMP/completion-$mode.lock" || true
  [ "$(wc -l < "$TMP/runs")" -eq 1 ]
  [ ! -L "$artifact" ] || [ "$(cat "$outside")" = 'arbitrary outside data' ]
}

case_worker_completion_artifact_never_stops_queue_work() {
  local plan="$TMP/completion-worker-marker" head gates
  make_repo "$plan"
  head=$(git -C "$plan" rev-parse HEAD)
  gates=$(sha256sum "$plan/docs/PLAN.md" | awk '{print $1}')
  printf '{"schema":"plan-complete-v1","head":"%s","required_gates_sha256":"%s","offline_validation":{"status":"passed","validated_at_head":"%s"}}\n' \
    "$head" "$gates" "$head" > "$plan/.state/PLAN-COMPLETE"
  chmod 600 "$plan/.state/PLAN-COMPLETE"
  : > "$TMP/runs"
  controller_env "$plan" "$TMP/record-run" "$TMP/no-network" "$TMP/completion-worker-marker.lock"
  # PLAN-COMPLETE is informational controller evidence only. A worker can
  # write this perfect-looking body, so it must never outrank queued work.
  [ "$(wc -l < "$TMP/runs")" -eq 1 ]
}

case_symlink_publication_and_budget() {
  local plan="$TMP/symlink-plan" bin="$TMP/symlink-bin" sentinel="$TMP/sentinel"
  make_repo "$plan"
  mkdir -p "$bin"
  printf 'DO-NOT-TOUCH\n' > "$sentinel"
  cat > "$bin/cat" <<'EOF'
#!/usr/bin/env bash
if [ "$#" -eq 1 ] && [ "$1" = /proc/sys/kernel/random/uuid ]; then
  echo symlink-slot
else
  exec /usr/bin/cat "$@"
fi
EOF
  cat > "$bin/date" <<EOF
#!/usr/bin/env bash
if [ -e "$plan/.state/spawns" ] && [ ! -e "$TMP/symlink-released" ]; then
  : > "$TMP/symlink-blocked"
  while [ ! -e "$TMP/symlink-released" ]; do sleep 0.01; done
fi
exec /usr/bin/date "\$@"
EOF
  chmod +x "$bin/cat" "$bin/date"
  : > "$TMP/runs"
  PATH="$bin:$PATH" controller_env "$plan" "$TMP/record-run" "$TMP/no-network" "$TMP/symlink.lock" &
  local pid=$!
  for _ in $(seq 1 300); do [ -e "$TMP/symlink-blocked" ] && break; sleep 0.01; done
  [ -e "$TMP/symlink-blocked" ]
  ln -s "$sentinel" "$plan/.state/claims/1-symlink-slot.claim"
  : > "$TMP/symlink-released"
  wait "$pid" || true
  [ "$(cat "$sentinel")" = DO-NOT-TOUCH ]
  [ -L "$plan/.state/claims/1-symlink-slot.claim" ]
  [ ! -s "$TMP/runs" ]
  [ "$(awk '{print $2}' "$plan/.state/spawns" 2>/dev/null || echo 0)" = 0 ]
}

case_concurrent_reservation() {
  local plan="$TMP/concurrent-plan" network="$TMP/network-barrier" run="$TMP/concurrent-run"
  make_repo "$plan"
  cat > "$network" <<EOF
#!/usr/bin/env bash
exec 9>"$TMP/network-count.lock"
flock 9
n=\$(cat "$TMP/network-count" 2>/dev/null || echo 0)
echo \$((n+1)) > "$TMP/network-count"
flock -u 9
while [ "\$(cat "$TMP/network-count" 2>/dev/null || echo 0)" -lt 2 ]; do sleep 0.01; done
EOF
  cat > "$run" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >> "$TMP/concurrent-runs"
EOF
  chmod +x "$network" "$run"
  : > "$TMP/concurrent-runs"
  controller_env "$plan" "$run" "$network" "$TMP/concurrent-a.lock" & local a=$!
  controller_env "$plan" "$run" "$network" "$TMP/concurrent-b.lock" & local b=$!
  wait "$a" || true
  wait "$b" || true
  [ "$(wc -l < "$TMP/concurrent-runs")" -eq 1 ]
  [ "$(awk '{print $2}' "$plan/.state/spawns")" -eq 1 ]
  [ "$(find "$plan/.state/claims" -maxdepth 1 -name '*.claim' | wc -l)" -eq 1 ]
}

make_blocking_model() {
  local script=$1 marker=$2
  cat > "$script" <<EOF
#!/usr/bin/env bash
trap 'touch "$marker.terminated"; exit 143' TERM INT
touch "$marker.entered"
while :; do sleep 0.05; done
EOF
  chmod +x "$script"
}

case_launch_mutation() {
  local mutation=$1 plan="$TMP/launch-$1" session="race-$1" marker="$TMP/launch-$1"
  make_repo "$plan"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session"
  make_blocking_model "$TMP/model-$1" "$marker"
  # id/gate renames are content-indistinguishable from the worker's own
  # completion rewrite, so they travel the bounded grace window; a short
  # window keeps the immediate-termination bound below this test's wait.
  setsid env BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/model-$1" \
    NUDGE_IDLE_POLL=0.05 NUDGE_IDLE_LIMIT=900 NUDGE_BOUNDARY_GRACE=1 \
    "$AGENT" 1 "$session" >"$marker.out" 2>&1 &
  local worker=$!
  for _ in $(seq 1 300); do [ -e "$marker.entered" ] && break; sleep 0.01; done
  [ -e "$marker.entered" ]
  case "$mutation" in
    pause) printf 'race pause\n' > "$plan/.state/PAUSE" ;;
    id) sed -i 's/id=stable-one/id=changed-id/' "$plan/.state/NEXT.md" ;;
    gate) sed -i 's/gate=gate-one/gate=changed-gate/' "$plan/.state/NEXT.md" ;;
    status) sed -i 's/\[READY\]/[WAITING-AUTOMATIC]/' "$plan/.state/NEXT.md" ;;
    inode)
      mv "$plan/.state/claims/1-owner.claim" "$plan/.state/claims/old-owner.claim"
      cp "$plan/.state/claims/old-owner.claim" "$plan/.state/claims/1-owner.claim"
      ;;
    body) printf 'id=tampered\n' >> "$plan/.state/claims/1-owner.claim" ;;
    queue-body) sed -i 's/automated test item/automated body-only replacement/' "$plan/.state/NEXT.md" ;;
    queue-inode)
      cp "$plan/.state/NEXT.md" "$plan/.state/NEXT.replacement"
      mv "$plan/.state/NEXT.replacement" "$plan/.state/NEXT.md"
      ;;
  esac
  for _ in $(seq 1 300); do
    ! kill -0 "$worker" 2>/dev/null && break
    sleep 0.01
  done
  local alive=0
  kill -0 "$worker" 2>/dev/null && alive=1
  kill -TERM -- "-$worker" 2>/dev/null || true
  wait "$worker" 2>/dev/null || true
  # nudge-agent is deliberately under test for descendant cleanup. If current
  # production leaves its timeout/model orphaned, record that behavior above,
  # then clean up only this fixture's uniquely named executables.
  pkill -TERM -f "^timeout 3900 $TMP/model-$mutation " 2>/dev/null || true
  pkill -TERM -f "^bash $TMP/model-$mutation " 2>/dev/null || true
  sleep 0.02
  pkill -KILL -f "^timeout 3900 $TMP/model-$mutation " 2>/dev/null || true
  pkill -KILL -f "^bash $TMP/model-$mutation " 2>/dev/null || true
  [ "$alive" -eq 0 ]
  [ -e "$marker.terminated" ]
}

case_launch_mutation_kills_process_tree() {
  local plan="$TMP/process-tree" session=process-tree marker="$TMP/process-tree"
  make_repo "$plan"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session"
  cat > "$TMP/tree-model" <<EOF
#!/usr/bin/env bash
trap 'exit 143' TERM INT
(
  trap 'exit 143' TERM INT
  awk '/^NSpid:/ {print \$2}' /proc/self/status > "$marker.grandchild"
  while :; do sleep 0.05; done
) &
awk '/^NSpid:/ {print \$2}' /proc/self/status > "$marker.child"
ps -o pgid= -p \$BASHPID | tr -d ' ' > "$marker.pgid"
touch "$marker.entered"
while :; do sleep 0.05; done
EOF
  chmod +x "$TMP/tree-model"
  local parent_pgid
  parent_pgid=$(ps -o pgid= -p $$ | tr -d ' ')
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/tree-model" \
    NUDGE_IDLE_POLL=0.02 NUDGE_IDLE_LIMIT=900 "$AGENT" 1 "$session" >"$marker.out" 2>&1 &
  local worker=$!
  for _ in $(seq 1 300); do [ -e "$marker.entered" ] && break; sleep 0.01; done
  [ -e "$marker.entered" ]
  # The model tree needs an independently addressable process group (or an
  # equivalent cgroup owned by the wrapper), not the caller's process group.
  local dedicated=0
  [ "$(cat "$marker.pgid")" != "$parent_pgid" ] && dedicated=1
  local started ended child grandchild
  started=$(date +%s%3N)
  printf 'pause now\n' > "$plan/.state/PAUSE"
  for _ in $(seq 1 100); do ! kill -0 "$worker" 2>/dev/null && break; sleep 0.01; done
  ended=$(date +%s%3N)
  child=$(cat "$marker.child")
  grandchild=$(cat "$marker.grandchild")
  wait "$worker" 2>/dev/null || true
  local child_alive=0 grandchild_alive=0
  kill -0 "$child" 2>/dev/null && child_alive=1
  kill -0 "$grandchild" 2>/dev/null && grandchild_alive=1
  kill -KILL "$child" "$grandchild" 2>/dev/null || true
  [ "$dedicated" -eq 1 ]
  [ "$child_alive" -eq 0 ]
  [ "$grandchild_alive" -eq 0 ]
  [ $((ended - started)) -lt 1000 ]
}

case_completion_rewrite_clean_exit() {
  # The 2026-08-26 watchdog repair: AGENTS.md step 7 has the WORKER rewrite
  # NEXT.md (move its claimed item to ## Done) as its final act, and a real
  # client keeps streaming its final message for seconds afterwards. The
  # wrapper must treat that queue change as the sanctioned completion, let
  # the model exit on its own, and record a clean end -- not kill it at the
  # finish line with a preflight-mismatch failure (three completed+pushed
  # units died that way: eb9917a1, 7003f272, 71effd2b).
  local plan="$TMP/completion-clean" session=completion-clean marker="$TMP/completion-clean"
  local model="$TMP/model-completion-clean" worker rc
  make_repo "$plan"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session"
  cat > "$model" <<EOF
#!/usr/bin/env bash
trap 'touch "$marker.terminated"; exit 143' TERM INT
touch "$marker.entered"
printf 'work\n' >> "$plan/code.txt"
git -C "$plan" add code.txt
git -C "$plan" commit -qm "fixture work" -m "Nudge-Worker: $session"
cat > "$plan/.state/NEXT.md" <<'QEOF'
# NEXT

## Now
1. [READY] [id=successor-one] [gate=gate-two] queued successor task

## Done
1. DONE (fixture): stable-one/gate-one completed by the model.
QEOF
# A real client is still streaming its final message here: far past the
# old 200ms self-exit grace, well inside the sanctioned window.
sleep 1
exit 0
EOF
  chmod +x "$model"
  setsid env BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$model" \
    NUDGE_IDLE_POLL=0.05 NUDGE_IDLE_LIMIT=900 NUDGE_BOUNDARY_GRACE=10 \
    SYSTEMD_RUN_OVERRIDE=1 "$AGENT" 1 "$session" >"$marker.out" 2>&1 &
  worker=$!
  for _ in $(seq 1 400); do ! kill -0 "$worker" 2>/dev/null && break; sleep 0.05; done
  wait "$worker" 2>/dev/null
  rc=$?
  kill -TERM -- "-$worker" 2>/dev/null || true
  pkill -TERM -f "^timeout 3900 $model" 2>/dev/null || true
  pkill -KILL -f "^timeout 3900 $model" 2>/dev/null || true
  [ "$rc" -eq 0 ]
  [ -e "$marker.entered" ]
  # The model finished on its own: no termination, no failure artifact.
  [ ! -e "$marker.terminated" ]
  [ ! -e "$plan/.state/automation-failures/$session.json" ]
  # A clean end releases the claim for the next scheduler pass.
  [ ! -e "$plan/.state/claims/1-owner.claim" ]
  grep -q "completion rewrite" "$plan/.state/nudge.log"
}

case_completion_claim_is_not_a_deadlock() {
  # The 2026-08-28 02:32 watchdog repair: AGENTS.md step-7 has the WORKER
  # rewrite NEXT.md (move its claimed item to ## Done) as its final act, so
  # between that rewrite and the wrapper's claim-release epilogue (model
  # exit, or the reaper's DEAD_CLAIM_TTL for a killed wrapper) the canonical
  # owner claim names an id that is no longer active. The strict parser
  # treated exactly that sanctioned window as INVALID-DEADLOCKED, so any
  # controller tick inside it forced a repair that killed the wrapper
  # mid-grace and orphaned the very claim its epilogue was about to release
  # (five forced repairs in 28h, the last orphaning 1-owner.claim over a
  # completed+pushed unit). Classify by capability instead: an unlocked
  # departed owner claim suppresses nothing, a locked one holds its slot
  # while a live wrapper finishes, and everything that still binds a launch
  # (reservations, or an owner whose identity MATCHES the active queue)
  # stays byte-strict.
  local plan="$TMP/completion-parse" session=completion-parse holder
  make_repo "$plan"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session" stable-one gate-one
  mv "$plan/.state/claims/1-$session.claim" "$plan/.state/claims/1-owner.claim"
  cat > "$plan/.state/NEXT.md" <<'QEOF'
# NEXT

## Now
1. [READY] [id=successor-one] [gate=gate-two] queued successor task

## Done
1. DONE (fixture): stable-one/gate-one completed by the model.
QEOF
  # Post-crash residue: wrapper dead, flock free. The departed claim must not
  # fail the preflight or suppress work on the successor.
  [ "$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1)" = "RUNNABLE 1" ]
  # Sanctioned window: a live wrapper still holds the flock. The claim holds
  # the slot (CLAIMED-RUNNING) instead of deadlocking the controller.
  flock "$plan/.state/claims/1-owner.claim" sleep 5 &
  holder=$!
  for _ in $(seq 1 300); do ! flock -n "$plan/.state/claims/1-owner.claim" true 2>/dev/null && break; sleep 0.01; done
  [ "$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1)" = "CLAIMED-RUNNING" ]
  kill "$holder" 2>/dev/null || true
  wait "$holder" 2>/dev/null || true
  # A reservation (launch authorization) with a departed identity stays fatal.
  rm -f "$plan/.state/claims/1-owner.claim"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session" stable-one gate-one
  ! $PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>/dev/null
  # An owner claim whose identity still matches the queue but whose hash
  # binding does not stays fatal (tamper protection is not weakened).
  rm -f "$plan/.state/claims/1-$session.claim"
  write_v2 "$plan/.state/claims/1-$session.claim" "$session" successor-one gate-two
  mv "$plan/.state/claims/1-$session.claim" "$plan/.state/claims/1-owner.claim"
  sed -i "s/^queue_sha256=.*/queue_sha256=$(printf '0%.0s' $(seq 1 64))/" "$plan/.state/claims/1-owner.claim"
  ! $PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>/dev/null
}

case_identityless_v1_new_launch() {
  local plan="$TMP/v1-new" session=v1-new
  make_repo "$plan"
  printf 'lock-v1 worker %s owns queue item 1\n' "$session" > "$plan/.state/claims/1-$session.claim"
  cat > "$TMP/v1-model" <<EOF
#!/usr/bin/env bash
touch "$TMP/v1-launched"
EOF
  chmod +x "$TMP/v1-model"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/v1-model" "$AGENT" 1 "$session"
  local rc=$?
  set -e
  [ "$rc" -ne 0 ]
  [ ! -e "$TMP/v1-launched" ]
}

case_locked_v1_migration_retained() {
  local claims="$TMP/v1-running" log="$TMP/v1-running.log"
  mkdir -p "$claims"
  printf 'lock-v1 worker old-session owns queue item 1\n' > "$claims/1-owner.claim"
  (
    exec 8>>"$claims/1-owner.claim"
    flock 8
    touch "$TMP/v1-running.locked"
    sleep 10
  ) & local holder=$!
  for _ in $(seq 1 100); do [ -e "$TMP/v1-running.locked" ] && break; sleep 0.01; done
  touch -d '1 hour ago' "$claims/1-owner.claim"
  DEAD_CLAIM_TTL=0 LEGACY_CLAIM_TTL=0 "$REAPER" "$claims" "$log"
  [ -e "$claims/1-owner.claim" ]
  kill "$holder" 2>/dev/null || true
}

case_invalid_identifiers_before_paths() {
  local plan="$TMP/invalid-identifiers"
  make_repo "$plan"
  rm -f "$plan/.state/heartbeat"
  printf 'sentinel\n' > "$TMP/path-sentinel"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false "$AGENT" '../escape' '../../path-sentinel'
  local rc=$?
  set -e
  [ "$rc" -ne 0 ]
  [ "$(cat "$TMP/path-sentinel")" = sentinel ]
  [ ! -e "$plan/.state/heartbeat" ]
}

case_reserved_session_names_rejected() {
  local plan="$TMP/reserved-session"
  make_repo "$plan"
  rm -f "$plan/.state/heartbeat"
  for session in owner publish.lock executor.lock queue.lock archive tmp-lock; do
    rm -f "$plan/.state/claims/1-$session.claim"
    set +e
    "$ROOT/tools/nudge-state.py" publish-claim "$plan/.state/claims" \
      "1-$session.claim" 1 stable-one gate-one "$session" "$(date -Is)" \
      "bedlam-nudge-item1-$session" $$ >/dev/null 2>&1
    local rc=$?
    set -e
    [ "$rc" -ne 0 ]
    [ ! -e "$plan/.state/claims/1-$session.claim" ]
  done
}

run_failed_agent() {
  local plan=$1 session=$2 model=$3
  write_v2 "$plan/.state/claims/1-$session.claim" "$session"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$model" "$AGENT" 1 "$session"
  local rc=$?
  set -e
  [ "$rc" -eq 42 ]
}

case_failure_artifact_symlink() {
  local plan="$TMP/artifact-symlink" sentinel="$TMP/artifact-sentinel"
  make_repo "$plan"
  mkdir -m 700 "$plan/.state/automation-failures"
  printf 'DO-NOT-TOUCH\n' > "$sentinel"
  ln -s "$sentinel" "$plan/.state/automation-failures/symlink-session.json"
  printf '#!/usr/bin/env bash\nexit 42\n' > "$TMP/fail42"
  chmod +x "$TMP/fail42"
  run_failed_agent "$plan" symlink-session "$TMP/fail42" || true
  [ "$(cat "$sentinel")" = DO-NOT-TOUCH ]
  [ -L "$plan/.state/automation-failures/symlink-session.json" ]
}

case_failure_artifact_untrusted_dir() {
  local plan="$TMP/artifact-untrusted"
  make_repo "$plan"
  mkdir -m 0777 "$plan/.state/automation-failures"
  run_failed_agent "$plan" unsafe-dir "$TMP/fail42" || true
  [ ! -e "$plan/.state/automation-failures/unsafe-dir.json" ]
}

case_failure_artifact_atomic_private() {
  local plan="$TMP/artifact-private" artifact
  make_repo "$plan"
  run_failed_agent "$plan" private-json "$TMP/fail42"
  artifact="$plan/.state/automation-failures/private-json.json"
  python3 - "$artifact" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["schema"] == "nudge-failure-v1"
PY
  [ "$(stat -c %a "$artifact")" = 600 ]
  [ -z "$(find "$plan/.state/automation-failures" -name '*.tmp*' -print -quit)" ]
}

case_concurrent_queue_edit_preserved() {
  local plan="$TMP/queue-edit"
  make_repo "$plan"
  cat > "$TMP/edit-and-fail" <<EOF
#!/usr/bin/env bash
sed -i 's/id=stable-one/id=concurrent-edit/' "$plan/.state/NEXT.md"
exit 42
EOF
  chmod +x "$TMP/edit-and-fail"
  run_failed_agent "$plan" queue-edit "$TMP/edit-and-fail" || true
  grep -q 'id=concurrent-edit' "$plan/.state/NEXT.md"
  python3 - "$plan/.state/automation-failures/queue-edit.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["queue_unchanged"] is False
PY
}

assert_failure_identity_record() {
  local artifact=$1 expected_change=$2
  python3 - "$artifact" "$expected_change" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
before = value["queue_before"]
after = value["queue_after"]
assert set(before) >= {"sha256", "device", "inode", "error"}
assert set(after) >= {"sha256", "device", "inode", "error"}
assert value["queue_change"] == sys.argv[2]
PY
}

case_failure_artifact_detects_aba() {
  local plan="$TMP/failure-aba"
  make_repo "$plan"
  cat > "$TMP/aba-and-fail" <<EOF
#!/usr/bin/env bash
cp "$plan/.state/NEXT.md" "$plan/.state/NEXT.new"
mv "$plan/.state/NEXT.new" "$plan/.state/NEXT.md"
exit 42
EOF
  chmod +x "$TMP/aba-and-fail"
  run_failed_agent "$plan" aba-failure "$TMP/aba-and-fail" || true
  assert_failure_identity_record \
    "$plan/.state/automation-failures/aba-failure.json" replaced-aba
}

case_failure_artifact_records_missing_after() {
  local plan="$TMP/failure-missing"
  make_repo "$plan"
  cat > "$TMP/delete-and-fail" <<EOF
#!/usr/bin/env bash
rm -f "$plan/.state/NEXT.md"
exit 42
EOF
  chmod +x "$TMP/delete-and-fail"
  run_failed_agent "$plan" missing-failure "$TMP/delete-and-fail" || true
  python3 - "$plan/.state/automation-failures/missing-failure.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    value = json.load(f)
assert value["queue_before"]["error"] is None
assert value["queue_after"]["sha256"] is None
assert value["queue_after"]["inode"] is None
assert value["queue_after"]["error"] == "missing"
assert value["queue_change"] == "missing-after"
PY
}

write_malformed_v2() {
  local file=$1
  cat > "$file" <<EOF
lock-v2
ordinal=2
id=wrong
owner=worker
session=bad
claimed_at=not-a-time
EOF
}

case_malformed_parser_controller() {
  local plan="$TMP/malformed-parser"
  make_repo "$plan"
  write_malformed_v2 "$plan/.state/claims/1-owner.claim"
  set +e
  local state rc
  state=$("$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>"$TMP/malformed.err")
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
  [ "$state" = INVALID-DEADLOCKED ]
  cat > "$TMP/malformed-run" <<EOF
#!/usr/bin/env bash
echo run >> "$TMP/malformed-runs"
EOF
  chmod +x "$TMP/malformed-run"
  : > "$TMP/malformed-runs"
  set +e
  controller_env "$plan" "$TMP/malformed-run" "$TMP/no-network" "$TMP/malformed.lock"
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
  [ ! -s "$TMP/malformed-runs" ]
}

case_malformed_reaper_bounded() {
  local claims="$TMP/malformed-reaper" log="$TMP/malformed-reaper.log"
  mkdir -p "$claims"
  write_malformed_v2 "$claims/1-owner.claim"
  (
    exec 8>>"$claims/1-owner.claim"
    flock 8
    touch "$TMP/malformed-locked"
    sleep 10
  ) & local holder=$!
  for _ in $(seq 1 100); do [ -e "$TMP/malformed-locked" ] && break; sleep 0.01; done
  touch -d '1 hour ago' "$claims/1-owner.claim"
  MALFORMED_CLAIM_TTL=0 "$REAPER" "$claims" "$log"
  [ -e "$claims/1-owner.claim" ]
  kill "$holder" 2>/dev/null || true
  wait "$holder" 2>/dev/null || true
  touch -d '1 hour ago' "$claims/1-owner.claim"
  MALFORMED_CLAIM_TTL=1 "$REAPER" "$claims" "$log"
  [ ! -e "$claims/1-owner.claim" ]
  grep -Eqi 'malformed.*(quarantined|reaped)|(quarantined|reaped).*malformed' "$log"
}

case_malformed_v1_reaper_bounded() {
  local claims="$TMP/malformed-v1-reaper" log="$TMP/malformed-v1-reaper.log"
  mkdir -p "$claims"
  printf 'lock-v1 worker legacy owns queue item 1 trailing-junk\n' > "$claims/1-owner.claim"
  touch -d '1 hour ago' "$claims/1-owner.claim"
  MALFORMED_CLAIM_TTL=0 "$REAPER" "$claims" "$log"
  [ ! -e "$claims/1-owner.claim" ]
  find "$claims" -maxdepth 1 -name '.quarantine-*-1-owner.claim' -print -quit | grep -q .

  printf 'lock-v1 worker legacy owns queue item 2 trailing-junk\n' > "$claims/2-owner.claim"
  (
    exec 8>>"$claims/2-owner.claim"
    flock 8
    touch "$TMP/malformed-v1-locked"
    sleep 10
  ) & local holder=$!
  for _ in $(seq 1 100); do [ -e "$TMP/malformed-v1-locked" ] && break; sleep 0.01; done
  touch -d '1 hour ago' "$claims/2-owner.claim"
  MALFORMED_CLAIM_TTL=0 "$REAPER" "$claims" "$log"
  [ -e "$claims/2-owner.claim" ]
  kill "$holder" 2>/dev/null || true
  wait "$holder" 2>/dev/null || true
}

case_claim_mode_and_body_binding() {
  local mode=$1 plan="$TMP/claim-contract-$1" claim rc state fields
  local old_body current_body current_device current_inode current_queue
  make_repo "$plan"
  claim="$plan/.state/claims/1-owner.claim"
  # Derive the stale body independently, then bind every queue-generation
  # field to the post-edit queue. This makes body_sha256 the only stale field;
  # rejecting the fixture for an old queue inode/digest would be a false green.
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r _status _id _gate old_body _old_device _old_inode _old_queue <<< "$fields"
  case "$mode" in
    stale-body)
      sed -i 's/automated test item/body changed without id status or gate changing/' "$plan/.state/NEXT.md"
      fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
      read -r _status _id _gate current_body current_device current_inode current_queue <<< "$fields"
      [ "$current_body" != "$old_body" ]
      ;;
    writable)
      current_body=$old_body
      read -r _status _id _gate _body current_device current_inode current_queue <<< "$fields"
      ;;
  esac
  "$ROOT/tools/nudge-state.py" publish-claim "$plan/.state/claims" \
    1-claim-contract.claim 1 stable-one gate-one claim-contract "$(date -Is)" \
    bedlam-nudge-item1-claim-contract $$ \
    "$old_body" "$current_device" "$current_inode" "$current_queue"
  mv "$plan/.state/claims/1-claim-contract.claim" "$claim"
  [ "$mode" != writable ] || chmod 0666 "$claim"
  set +e
  state=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 2>"$TMP/claim-$mode.err")
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
  [ "$state" = INVALID-DEADLOCKED ]
  grep -Eqi 'claim.*(mode|owner|untrusted|identity|body|hash|mismatch)|unsafe.*claim' "$TMP/claim-$mode.err"
}

case_agents_contract() {
  ! grep -Eqi 'If genuinely blocked.*\[BLOCKED|tag(ging)?[^.]*\[BLOCKED|standing down[^.]*operator|operator[^.]*standing down' "$ROOT/AGENTS.md"
}

run_case 'canonical claim is never partially public' case_partial_publication
for lock_kind in controller publication watchdog network executor; do
  run_case "$lock_kind lock refuses symlink without truncating target" \
    case_lock_path_refuses_symlink "$lock_kind"
done
for completion_mode in symlink arbitrary stale-head stale-gates validation-failed; do
  run_case "PLAN-COMPLETE refuses $completion_mode evidence" \
    case_invalid_completion_artifact_does_not_stop "$completion_mode"
done
run_case 'worker-created PLAN-COMPLETE never suppresses queued work' \
  case_worker_completion_artifact_never_stops_queue_work
run_case 'claim publication refuses a raced symlink without spending spawn budget' case_symlink_publication_and_budget
run_case 'concurrent reservation has one winner and one spent spawn' case_concurrent_reservation
for mutation in id gate status inode body queue-body queue-inode; do
  run_case "launch boundary rejects $mutation mutation" case_launch_mutation "$mutation"
done
run_case 'PAUSE appearing after model start terminates the model' case_launch_mutation pause
run_case 'launch mutation promptly kills child and grandchild process tree' case_launch_mutation_kills_process_tree
run_case 'worker completion rewrite ends the run cleanly' case_completion_rewrite_clean_exit
run_case 'completion-window owner claim never deadlocks the parser' case_completion_claim_is_not_a_deadlock
run_case 'identity-less lock-v1 cannot launch a newly current ordinal' case_identityless_v1_new_launch
run_case 'locked lock-v1 remains compatible only as running migration state' case_locked_v1_migration_retained
run_case 'invalid item/session are rejected before path construction' case_invalid_identifiers_before_paths
run_case 'internal owner lock and temporary session names are reserved' case_reserved_session_names_rejected
run_case 'failure artifact refuses a pre-existing symlink' case_failure_artifact_symlink
run_case 'failure artifact refuses an untrusted directory' case_failure_artifact_untrusted_dir
run_case 'failure artifact is atomic private JSON' case_failure_artifact_atomic_private
run_case 'concurrent queue edit is preserved and reported accurately' case_concurrent_queue_edit_preserved
run_case 'failure artifact detects same-bytes inode ABA replacement' case_failure_artifact_detects_aba
run_case 'failure artifact records a missing after-snapshot explicitly' case_failure_artifact_records_missing_after
run_case 'malformed v2 deadlocks parser and controller' case_malformed_parser_controller
run_case 'malformed v2 lock is retained but stale unlocked claim is bounded' case_malformed_reaper_bounded
run_case 'malformed lock-v1 is quarantined after TTL but active lock survives' case_malformed_v1_reaper_bounded
run_case 'group/world-writable existing claim cannot suppress work' case_claim_mode_and_body_binding writable
run_case 'claim bound to stale active-task body cannot suppress current work' case_claim_mode_and_body_binding stale-body
run_case 'AGENTS required-task contract forbids BLOCKED/handoff instructions' case_agents_contract

if [ "$failures" -ne 0 ]; then
  printf 'lock-v2 adversarial tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'lock-v2 adversarial tests: PASS'
