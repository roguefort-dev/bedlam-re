#!/usr/bin/env bash
# Behavioral RED contracts for final autonomous-loop hardening.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PARSER="$ROOT/tools/nudge-free-items.py"
STATE_HELPER="$ROOT/tools/nudge-state.py"
WAIT_EXECUTOR="$ROOT/tools/nudge-wait.py"
RESERVE="$ROOT/tools/nudge-reserve.sh"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
CONTROLLER="$ROOT/tools/nudge.sh"
WATCHDOG="$ROOT/tools/llm-watchdog.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
TMP=$(mktemp -d /tmp/opencode/bedlam-final-hardening.XXXXXX)

cleanup() {
  jobs -pr | xargs -r kill -TERM 2>/dev/null || true
  jobs -pr | xargs -r wait 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT INT TERM

make_repo() {
  local plan=$1 status=${2:-READY}
  rm -rf "$plan"
  mkdir -p "$plan/.state/claims" "$plan/tools" "$plan/docs"
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [$status] [id=stable-one] [gate=gate-one] fixture task

## Backlog
EOF
  printf '# fixture\n' > "$plan/AGENTS.md"
  printf 'initial\n' > "$plan/code.txt"
  git -C "$plan" init -q
  git -C "$plan" config user.email test@example.invalid
  git -C "$plan" config user.name test
  git -C "$plan" add .state/NEXT.md AGENTS.md code.txt
  git -C "$plan" commit -qm init
}

write_wait_queue() {
  local plan=$1 probe=${2:-tools/probe.sh}
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=$probe] [retry=100ms] [timeout=5s] bounded fixture wait

## Backlog
EOF
}

authorize_probe() {
  local plan=$1 id=${2:-tools/probe.sh} path=${3:-tools/probe.sh} digest
  digest=$(sha256sum "$plan/$path" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "$id"
path = "$path"
sha256 = "$digest"
EOF
  git -C "$plan" add .state/NEXT.md "$path" docs/automatic-probes.toml
  git -C "$plan" commit -qm authorized-probe-fixture
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
  local plan=$1 lock=$2 network=${3:-$TMP/network-ok}
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$lock" SYSTEMD_RUN_OVERRIDE="$TMP/record-run" \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" NETWORK_WATCHDOG_OVERRIDE="$network" \
    REAPER_OVERRIDE="$REAPER" NOTIFY_SEND= "$CONTROLLER"
}

reseal_wait_state() {
  python3 - "$1" "$2" <<'PY'
import hashlib, json, math, sys, time
p, mode = sys.argv[1:]
v = json.load(open(p))
if mode == "bool": v["attempts"] = True
elif mode == "nonfinite": v["deadline_at"] = float("nan")
elif mode == "noninteger": v["attempts"] = 1.5
elif mode == "negative": v["attempts"] = -1
elif mode == "huge": v["attempts"] = 10**18
elif mode == "extended-deadline": v["deadline_at"] = time.time() + 86400
v["next_attempt_at"] = 0
v.pop("state_sha256", None)
raw = json.dumps(v, sort_keys=True, separators=(",", ":")).encode()
v["state_sha256"] = hashlib.sha256(raw).hexdigest()
open(p, "w").write(json.dumps(v, sort_keys=True, separators=(",", ":")) + "\n")
PY
}

numeric_payload() {
  local sentinel=$1
  printf 'x[$(touch${IFS}%s)]' "$sentinel"
}

case_numeric_spawn_state_is_explicitly_rejected() {
  local field plan payload sentinel out rc failures=0
  for field in hour count; do
    plan="$TMP/numeric-spawn-$field"
    sentinel="$TMP/numeric-spawn-$field.injected"
    payload=$(numeric_payload "$sentinel")
    make_repo "$plan"
    if [ "$field" = hour ]; then
      printf '%s 0\n' "$payload" > "$plan/.state/spawns"
    else
      printf '1 %s\n' "$payload" > "$plan/.state/spawns"
    fi
    out="$plan/result"
    set +e
    "$RESERVE" "$plan" 1 "spawn-$field" "bedlam-nudge-item1-spawn-$field" 1 16 >"$out" 2>&1
    rc=$?
    set -e
    [ ! -e "$sentinel" ] || failures=$((failures + 1))
    if [ "$rc" -eq 0 ] || ! grep -Eqi '(invalid|unsafe|malformed|out.of.range).*(spawn|hour|count)|(spawn|hour|count).*(invalid|unsafe|malformed|out.of.range)' "$out"; then
      echo "spawn $field was not explicitly rejected (rc=$rc)" >&2
      failures=$((failures + 1))
    fi
    [ ! -e "$plan/.state/claims/1-spawn-$field.claim" ] || failures=$((failures + 1))
  done
  [ "$failures" -eq 0 ]
}

case_numeric_fail_count_emits_state_repair() {
  local plan="$TMP/numeric-fail-count" sentinel="$TMP/numeric-fail-count.injected"
  local payload fields body dev ino queue task_hash artifact rc
  payload=$(numeric_payload "$sentinel")
  make_repo "$plan"
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r _ _ _ body dev ino queue <<< "$fields"
  "$STATE_HELPER" publish-claim "$plan/.state/claims" 1-failcount.claim 1 stable-one gate-one \
    failcount "$(date -Is)" bedlam-nudge-item1-failcount $$ "$body" "$dev" "$ino" "$queue"
  task_hash=$(sed -n 's/^1\. //p' "$plan/.state/NEXT.md" | head -1 | sha256sum | cut -c1-16)
  mkdir -p "$plan/.state/taskfails"
  printf '%s\n' "$payload" > "$plan/.state/taskfails/$task_hash"
  printf '1\n' > "$plan/.state/concurrency"
  set +e
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false NUDGE_IDLE_POLL=0.01 \
    "$AGENT" 1 failcount >"$plan/result" 2>&1
  rc=$?
  set -e
  [ ! -e "$sentinel" ]
  [ "$rc" -ne 0 ]
  artifact="$plan/.state/automation-failures/failcount.json"
  if ! python3 - "$artifact" <<'PY'
import json, sys
value = json.load(open(sys.argv[1]))
raise SystemExit(not (
    value["kind"] == "mutable-state-invalid"
    and "fail" in value["reason"]
    and "count" in value["reason"]
))
PY
  then
    echo "fail-count corruption did not emit a mutable-state-invalid repair artifact" >&2
    return 1
  fi
}

assert_controller_numeric_repair() {
  local seam=$1 plan=$2 sentinel=$3 rc artifact
  set +e
  run_controller "$plan" "$TMP/$seam.lock" >"$plan/result" 2>&1
  rc=$?
  set -e
  if [ -e "$sentinel" ]; then
    echo "$seam arithmetic payload created its sentinel" >&2
    return 1
  fi
  if [ "$rc" -ne 2 ]; then
    echo "$seam did not return the explicit repair status (rc=$rc)" >&2
    return 1
  fi
  artifact=$(find "$plan/.state/automation-failures" -maxdepth 1 -name '*.json' -print -quit 2>/dev/null)
  if [ -z "$artifact" ]; then
    echo "$seam returned without a structured repair artifact" >&2
    return 1
  fi
  if ! python3 - "$artifact" "$seam" <<'PY'
import json, sys
value = json.load(open(sys.argv[1]))
assert value["kind"] == "mutable-state-invalid"
assert sys.argv[2] in value["reason"]
PY
  then
    echo "$seam repair artifact did not identify invalid mutable numeric state" >&2
    return 1
  fi
}

case_numeric_heartbeat_spawn_timestamp_emits_repair() {
  local plan="$TMP/numeric-heartbeat-ts" sentinel="$TMP/numeric-heartbeat-ts.injected" payload
  payload=$(numeric_payload "$sentinel")
  make_repo "$plan"
  printf '%s\n' "$payload" > "$plan/.state/last-spawn-ts"
  assert_controller_numeric_repair last-spawn-ts "$plan" "$sentinel"
}

case_numeric_concurrency_state_emits_repair() {
  local mode plan sentinel payload failures=0
  for mode in value degraded-at; do
    plan="$TMP/numeric-concurrency-$mode"
    sentinel="$TMP/numeric-concurrency-$mode.injected"
    payload=$(numeric_payload "$sentinel")
    make_repo "$plan"
    if [ "$mode" = value ]; then
      printf '%s\n' "$payload" > "$plan/.state/concurrency"
    else
      printf '0\n' > "$plan/.state/concurrency"
      printf '%s\n' "$payload" > "$plan/.state/conc-degraded-at"
    fi
    assert_controller_numeric_repair "concurrency-$mode" "$plan" "$sentinel" || failures=$((failures + 1))
  done
  [ "$failures" -eq 0 ]
}

case_numeric_watchdog_pause_timestamp_emits_repair() {
  local plan="$TMP/numeric-watchdog-pause" sentinel="$TMP/numeric-watchdog-pause.injected" payload rc
  payload=$(numeric_payload "$sentinel")
  make_repo "$plan"
  printf 'llm-watchdog 999999 %s\n' "$payload" > "$plan/.state/PAUSE"
  set +e
  run_controller "$plan" "$TMP/numeric-watchdog-pause.lock" >"$plan/result" 2>&1
  rc=$?
  set -e
  [ ! -e "$sentinel" ]
  [ "$rc" -eq 2 ]
  grep -Eqi '(invalid|unsafe|malformed).*(watchdog|pause|timestamp)' "$plan/.state/nudge.log"
}

case_numeric_watchdog_verdict_and_cooldown_emit_repair() {
  local mode plan sentinel payload rc failures=0
  for mode in verdict cooldown; do
    plan="$TMP/numeric-watchdog-$mode"
    sentinel="$TMP/numeric-watchdog-$mode.injected"
    payload=$(numeric_payload "$sentinel")
    make_repo "$plan"
    if [ "$mode" = verdict ]; then
      printf 'time=%s\nstate=healthy\nrc=0\nmarkers=1\ncooldown_until=0\n' "$payload" \
        > "$plan/.state/llm-watchdog-verdict"
      touch -d '10 minutes ago' "$plan/.state/llm-watchdog-verdict"
    else
      printf '%s\n' "$payload" > "$plan/.state/llm-watchdog-cooldown-until"
    fi
    set +e
    BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false WATCHDOG_TEST_MODE=1 \
      LLM_WATCHDOG_MIN_INTERVAL=0 SUPERVISE_TIMEOUT=1 REPAIR_TIMEOUT=1 RESUME_WAIT_LOOPS=0 \
      SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
      LLM_WATCHDOG_LOCK="$TMP/numeric-watchdog-$mode.lock" NOTIFY_SEND= \
      "$WATCHDOG" >"$plan/result" 2>&1
    rc=$?
    set -e
    [ ! -e "$sentinel" ] || failures=$((failures + 1))
    if [ "$rc" -eq 0 ] || ! grep -Eqi '(invalid|unsafe|malformed).*(verdict|cooldown|timestamp)|(verdict|cooldown|timestamp).*(invalid|unsafe|malformed)' "$plan/.state/llm-watchdog.log"; then
      echo "watchdog $mode timestamp was not explicitly rejected (rc=$rc)" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_ordinary_state_writes_refuse_symlinks() {
  local plan="$TMP/state-symlinks" sentinel="$TMP/outside-state"
  make_repo "$plan"
  printf 'DO-NOT-CHANGE\n' > "$sentinel"
  ln -s "$sentinel" "$plan/.state/STATUS.md"
  : > "$TMP/runs"
  run_controller "$plan" "$TMP/state-symlinks.lock" >/dev/null 2>&1 || true
  [ "$(cat "$sentinel")" = DO-NOT-CHANGE ]

  rm -f "$plan/.state/STATUS.md"
  ln -s "$sentinel" "$plan/.state/spawns"
  "$RESERVE" "$plan" 1 statewrite bedlam-nudge-item1-statewrite 1 16 >/dev/null 2>&1 || true
  [ "$(cat "$sentinel")" = DO-NOT-CHANGE ]

  rm -f "$plan/.state/spawns"
  touch -d '2 hours ago' "$sentinel"
  local before
  before=$(stat -c %Y "$sentinel")
  ln -s "$sentinel" "$plan/.state/heartbeat"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false "$AGENT" 1 absent >/dev/null 2>&1 || true
  [ "$(stat -c %Y "$sentinel")" = "$before" ]
}

copy_controller_stack() {
  local plan=$1
  for name in nudge.sh nudge-lock.py nudge-free-items.py nudge-state.py nudge-wait.py \
      nudge-reserve.sh nudge-reap-claims.sh nudge-claim.sh network-watchdog.sh; do
    cp "$ROOT/tools/$name" "$plan/tools/$name"
  done
  chmod +x "$plan/tools/"*
}

case_empty_completion_uses_clean_head_validator() {
  local plan="$TMP/completion-dirty-validator" sentinel="$TMP/dirty-validator-ran"
  make_repo "$plan"
  copy_controller_stack "$plan"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  cat > "$plan/tools/validate-required-gates.py" <<'EOF'
#!/usr/bin/env python3
raise SystemExit(91)
EOF
  chmod +x "$plan/tools/validate-required-gates.py"
  printf 'anchor\n' > "$plan/manifest-anchor.txt"
  printf '%s  manifest-anchor.txt\n' "$(sha256sum "$plan/manifest-anchor.txt" | awk '{print $1}')" > "$plan/MANIFEST.sha256"
  git -C "$plan" add .state/NEXT.md tools manifest-anchor.txt MANIFEST.sha256
  git -C "$plan" commit -qm 'trusted empty queue and validator'
  cat > "$plan/tools/validate-required-gates.py" <<EOF
#!/usr/bin/env python3
import json, pathlib, sys
pathlib.Path("$sentinel").touch()
out = pathlib.Path(sys.argv[sys.argv.index("--completion-output") + 1])
out.write_text(json.dumps({"schema":"plan-complete-v1"}) + "\n")
EOF
  chmod +x "$plan/tools/validate-required-gates.py"
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-dirty.lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" \
    NOTIFY_SEND= "$plan/tools/nudge.sh" >/dev/null 2>&1 || true
  [ ! -e "$sentinel" ]
  [ ! -e "$plan/.state/PLAN-COMPLETE" ]
}

case_completion_holds_queue_lock_and_rechecks_empty() {
  local plan="$TMP/completion-race" pid lock_was_held=0
  make_repo "$plan"
  copy_controller_stack "$plan"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  cat > "$plan/tools/validate-required-gates.py" <<EOF
#!/usr/bin/env bash
touch "$TMP/completion-validator-entered"
while [ ! -e "$TMP/completion-validator-release" ]; do sleep 0.01; done
while [ "\$#" -gt 0 ]; do
  if [ "\$1" = --completion-output ]; then shift; printf '{"schema":"plan-complete-v1"}\n' > "\$1"; fi
  shift
done
exit 0
EOF
  chmod +x "$plan/tools/validate-required-gates.py"
  printf 'anchor\n' > "$plan/manifest-anchor.txt"
  printf '%s  manifest-anchor.txt\n' "$(sha256sum "$plan/manifest-anchor.txt" | awk '{print $1}')" > "$plan/MANIFEST.sha256"
  git -C "$plan" add .state/NEXT.md tools manifest-anchor.txt MANIFEST.sha256
  git -C "$plan" commit -qm 'trusted empty queue validator race fixture'
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-race.lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" \
    NOTIFY_SEND= "$plan/tools/nudge.sh" >/dev/null 2>&1 &
  pid=$!
  for _ in $(seq 1 200); do [ -e "$TMP/completion-validator-entered" ] && break; sleep 0.01; done
  [ -e "$TMP/completion-validator-entered" ]
  if ! flock -n "$plan/.state/.queue.lock" true 2>/dev/null; then lock_was_held=1; fi
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [READY] [id=raced-item] [gate=raced-gate] arrived during validation

## Backlog
EOF
  printf 'untrusted raced claim\n' > "$plan/.state/claims/1-race.claim"
  : > "$TMP/completion-validator-release"
  wait "$pid" || true
  [ "$lock_was_held" -eq 1 ]
  [ ! -e "$plan/.state/PLAN-COMPLETE" ]
}

case_probe_requires_committed_allowlist_id() {
  local plan="$TMP/probe-allowlist" rc
  make_repo "$plan"
  write_wait_queue "$plan" tools/arbitrary.sh
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/arbitrary.sh"
  chmod +x "$plan/tools/arbitrary.sh"
  printf 'schema = "automatic-probes-v1"\n[[probe]]\nid = "approved"\npath = "tools/approved.sh"\nsha256 = "%064d"\n' 0 \
    > "$plan/docs/automatic-probes.toml"
  git -C "$plan" add .state/NEXT.md tools/arbitrary.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm arbitrary-probe
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 >/dev/null 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
}

case_reservation_rechecks_queue_before_claim_publication() {
  local plan="$TMP/reservation-final-cas" hook="$TMP/reservation-hook" rc
  make_repo "$plan"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
original_link = os.link
def raced_link(source, destination, *args, **kwargs):
    if str(destination).endswith(".claim") and not os.path.exists("$TMP/reservation-raced"):
        with open("$plan/.state/NEXT.md", "a") as handle: handle.write("\n# raced queue generation\n")
        open("$TMP/reservation-raced", "w").close()
    return original_link(source, destination, *args, **kwargs)
os.link = raced_link
PY
  set +e
  PYTHONPATH="$hook" "$RESERVE" "$plan" 1 race bedlam-nudge-item1-race 1 16 >/dev/null 2>&1
  rc=$?
  set -e
  [ -e "$TMP/reservation-raced" ]
  [ "$rc" -ne 0 ]
  [ ! -e "$plan/.state/claims/1-race.claim" ]
}

case_reaper_never_unlinks_a_replacement_inode() {
  local claims="$TMP/reaper-race" path bin="$TMP/reaper-bin" replacement_inode
  mkdir -p "$claims" "$bin"
  path="$claims/1-race.claim"
  cat > "$path" <<EOF
lock-v2
ordinal=1
id=stable-one
gate=gate-one
owner=worker
session=race
claimed_at=$(date -Is)
unit=bedlam-nudge-item1-race
pid=$$
body_sha256=$(printf a%.0s {1..64})
queue_device=1
queue_inode=1
queue_sha256=$(printf b%.0s {1..64})
EOF
  cp "$path" "$path.replacement"
  printf '\nreplacement-generation\n' >> "$path.replacement"
  chmod 600 "$path" "$path.replacement"
  replacement_inode=$(stat -c %i "$path.replacement")
  touch -d '1 hour ago' "$path"
  cat > "$bin/rm" <<EOF
#!/usr/bin/env bash
target="\${!#}"
if [ "\$target" = "$path" ] && [ ! -e "$TMP/reaper-swapped" ]; then
  mv "$path" "$path.original"
  mv "$path.replacement" "$path"
  touch "$TMP/reaper-swapped"
fi
exec /usr/bin/rm "\$@"
EOF
  chmod +x "$bin/rm"
  PATH="$bin:$PATH" RESERVATION_TTL=0 MALFORMED_CLAIM_TTL=0 \
    "$REAPER" "$claims" "$TMP/reaper.log" >/dev/null 2>&1 || true
  [ ! -e "$TMP/reaper-swapped" ]
  [ ! -e "$path" ]
}

case_wait_cache_rejects_resealed_invalid_numbers() {
  local mode plan state rc failures=0
  for mode in bool nonfinite noninteger negative huge extended-deadline; do
    plan="$TMP/wait-number-$mode"
    make_repo "$plan"
    write_wait_queue "$plan"
    printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
    chmod +x "$plan/tools/probe.sh"
    authorize_probe "$plan"
    "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
    state="$plan/.state/automatic-waits/stable-one.json"
    reseal_wait_state "$state" "$mode"
    set +e
    "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
    rc=$?
    set -e
    if [ "$rc" -eq 0 ]; then
      echo "re-sealed invalid wait cache accepted: $mode" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_wait_verification_is_read_only() {
  local plan="$TMP/wait-verify-readonly" before after rc
  make_repo "$plan"
  write_wait_queue "$plan"
  printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
touch "$TMP/verify-executed-probe"
exit 0
EOF
  chmod +x "$plan/tools/probe.sh"
  before=$(sha256sum "$plan/.state/NEXT.md" | awk '{print $1}')
  set +e
  "$WAIT_EXECUTOR" verify "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
  rc=$?
  set -e
  after=$(sha256sum "$plan/.state/NEXT.md" | awk '{print $1}')
  [ "$rc" -ne 0 ]
  [ ! -e "$TMP/verify-executed-probe" ]
  [ "$after" = "$before" ]
  grep -q '\[WAITING-AUTOMATIC\]' "$plan/.state/NEXT.md"
}

case_queue_post_publish_swap_is_detected() {
  local plan="$TMP/post-publish-swap" hook="$TMP/post-publish-hook" rc
  make_repo "$plan"
  write_wait_queue "$plan"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/probe.sh"
  chmod +x "$plan/tools/probe.sh"
  authorize_probe "$plan"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
original_replace = os.replace
def raced_replace(source, destination, *args, **kwargs):
    result = original_replace(source, destination, *args, **kwargs)
    if str(destination) == "$plan/.state/NEXT.md" and not os.path.exists("$TMP/post-publish-raced"):
        os.rename(destination, str(destination) + ".published")
        with open(destination, "w") as handle: handle.write("attacker replacement\n")
        open("$TMP/post-publish-raced", "w").close()
    return result
os.replace = raced_replace
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
  rc=$?
  set -e
  [ -e "$TMP/post-publish-raced" ]
  [ "$rc" -ne 0 ]
}

case_controller_runs_due_wait_before_network() {
  local plan="$TMP/offline-order"
  make_repo "$plan"
  write_wait_queue "$plan"
  cat > "$plan/tools/probe.sh" <<EOF
#!/usr/bin/env bash
touch "$TMP/offline-probe-ran"
exit 1
EOF
  cat > "$TMP/network-offline" <<'EOF'
#!/usr/bin/env bash
exit 75
EOF
  chmod +x "$plan/tools/probe.sh" "$TMP/network-offline"
  authorize_probe "$plan"
  run_controller "$plan" "$TMP/offline-order.lock" "$TMP/network-offline" >/dev/null 2>&1 || true
  [ -e "$TMP/offline-probe-ran" ]
}

write_failure() {
  local plan=$1 session=$2
  mkdir -m 700 -p "$plan/.state/automation-failures"
  cat > "$plan/.state/automation-failures/$session.json" <<EOF
{"schema":"nudge-failure-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","owner":"worker","session":"$session","kind":"client-error","reason":"test","evidence":"fixture","time":"2026-08-26T07:00:00Z","repair":"required","queue_unchanged":true}
EOF
  chmod 600 "$plan/.state/automation-failures/$session.json"
}

case_present_failure_ignores_watchdog_cooldown() {
  local plan="$TMP/failure-cooldown"
  make_repo "$plan"
  write_failure "$plan" cooldown-trigger
  echo $(( $(date +%s) + 3600 )) > "$plan/.state/llm-watchdog-cooldown-until"
  # Enable both gates.  A fresh verdict used to short-circuit before the
  # watchdog even inspected the already-present failure artifact.
  printf 'time=%s\nstate=healthy\nrc=0\nmarkers=1\ncooldown_until=%s\n' \
    "$(date -Is)" "$(( $(date +%s) + 3600 ))" > "$plan/.state/llm-watchdog-verdict"
  chmod 600 "$plan/.state/llm-watchdog-verdict"
  cat > "$TMP/cooldown-model" <<EOF
#!/usr/bin/env bash
touch "$TMP/cooldown-model-called"
exit 1
EOF
  chmod +x "$TMP/cooldown-model"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/cooldown-model" WATCHDOG_TEST_MODE=1 \
    LLM_WATCHDOG_MIN_INTERVAL=120 SUPERVISE_TIMEOUT=1 REPAIR_TIMEOUT=1 RESUME_WAIT_LOOPS=0 \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
    LLM_WATCHDOG_LOCK="$TMP/failure-cooldown.lock" NOTIFY_SEND= \
    "$WATCHDOG" >/dev/null 2>&1 || true
  [ -e "$TMP/cooldown-model-called" ]
  ! grep -q '^state=repair-deferred$' "$plan/.state/llm-watchdog-verdict"
  [ -e "$plan/.state/automation-failures/cooldown-trigger.json" ]
}

case_unrelated_trailer_commit_cannot_acknowledge_failure() {
  local plan="$TMP/unrelated-repair"
  make_repo "$plan"
  write_failure "$plan" unrelated-trigger
  cat > "$TMP/unrelated-repair-model" <<EOF
#!/usr/bin/env bash
token=\$(cat "$plan/.state/PAUSE")
printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
python3 - <<'PY'
import json
snapshot = json.load(open("$plan/.state/llm-watchdog-failure-snapshot.json"))
records = [dict(record, resolution="required-empty") for record in snapshot]
open("$plan/.state/llm-watchdog-failure-ack.json", "w").write(
    json.dumps({"schema":"nudge-failure-ack-v1", "records":records}) + "\n"
)
PY
echo unrelated >> "$plan/code.txt"
git -C "$plan" add code.txt
git -C "$plan" commit -qm unrelated -m "Watchdog-Repair: \$token"
exit 0
EOF
  chmod +x "$TMP/unrelated-repair-model"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/unrelated-repair-model" WATCHDOG_TEST_MODE=1 \
    LLM_WATCHDOG_MIN_INTERVAL=0 SUPERVISE_TIMEOUT=2 REPAIR_TIMEOUT=3 RESUME_WAIT_LOOPS=0 \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
    LLM_WATCHDOG_LOCK="$TMP/unrelated-repair.lock" NOTIFY_SEND= \
    "$WATCHDOG" >/dev/null 2>&1 || true
  [ -e "$plan/.state/automation-failures/unrelated-trigger.json" ]
  ! grep -q '^state=repaired$' "$plan/.state/llm-watchdog-verdict"
}

case_canonical_metadata_order_is_enforced() {
  local plan="$TMP/metadata-order" rc
  make_repo "$plan"
  sed -i 's/\[READY\] \[id=stable-one\] \[gate=gate-one\]/[gate=gate-one] [READY] [id=stable-one]/' "$plan/.state/NEXT.md"
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 >/dev/null 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 2 ]
}

case_active_p4_contract_retires_stale_live_facts() {
  local state
  state=$($PARSER "$ROOT/.state/NEXT.md" "$ROOT/.state/claims" --state-v1)
  [[ "$state" == RUNNABLE* ]]
  python3 - "$ROOT/.state/NEXT.md" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
now = text.split("## Now", 1)[1].split("## Done", 1)[0]
items = re.findall(r"^[1-5]\. .+$", now, re.M)
assert len(items) == 5
expected = [
    "p4-trigger-contract", "p4-static-proof-scope", "p4-wgpu-final",
    "p4-required-gates-manifest", "p4-machine-verdict",
]
assert [re.search(r"\[id=([^]]+)\]", item).group(1) for item in items] == expected
contract = " ".join(items).casefold()
for fact in ("d145", "d164", "static evidence", "timing", "calibration", "live", "excluded", "not queued", "exw"):
    assert fact in contract, fact
assert "interactive" not in contract and "perceptual" not in contract
PY
}

case_capgen_smoke_and_validator_invocation_are_offline() {
  grep -Eq 'cargo .*--locked.*--offline|cargo .*--offline.*--locked' \
    "$ROOT/tools/runtime/capgen-o2-smoke.sh"
  grep -Fq 'commands = [["/usr/bin/python3", "tools/test-validate-required-gates.py"]]' \
    "$ROOT/docs/required-gates.toml"
}

case_mutable_inputs_have_practical_size_and_count_caps() {
  local plan="$TMP/resource-caps" rc
  make_repo "$plan"
  python3 - "$plan/.state/NEXT.md" <<'PY'
import sys
items = [f"{i}. [READY] [id=item-{i}] [gate=gate-{i}] bounded item" for i in range(1, 258)]
open(sys.argv[1], "w").write("# NEXT\n\n## Now\n" + "\n".join(items) + "\n\n## Backlog\n")
PY
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 >/dev/null 2>&1
  rc=$?
  set -e
  [ "$rc" -eq 2 ]

  mkdir -m 700 -p "$plan/.state/automation-failures"
  head -c 1048577 /dev/zero > "$plan/.state/automation-failures/oversized.json"
  chmod 600 "$plan/.state/automation-failures/oversized.json"
  set +e
  "$STATE_HELPER" list-failures "$plan/.state/automation-failures" > "$plan/failure-list.out" 2>&1
  rc=$?
  set -e
  [ "$rc" -ne 0 ]
  grep -Eqi 'size|limit|quarantin' "$plan/failure-list.out"
}

case_normative_future_phase_plan_has_no_human_only_engineering_gates() {
  python3 - "$ROOT/docs/PLAN.md" <<'PY'
import sys

path = sys.argv[1]
lines = open(path, encoding="utf-8").read().splitlines()
start = lines.index("## 6. Phases")
end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("## 7."))
forbidden = (
    "macos nightly/manual (owner",
    "owner signs",
    "owner marks",
    "signed off",
    "human approval",
    "ci-manual",
    "best-effort per owner",
)
violations = [
    (number, phrase, line.strip())
    for number, line in enumerate(lines[start + 1:end], start + 2)
    for phrase in forbidden
    if phrase in line.casefold()
]
if violations:
    print("normative future phase text contains human-only engineering gates:", file=sys.stderr)
    for number, phrase, line in violations:
        print(f"  {path}:{number}: {phrase}: {line}", file=sys.stderr)
    raise SystemExit(1)
PY
}

if [ -n "${FINAL_HARDENING_CASE:-}" ]; then
  make_controller_mocks
  set -e
  "$FINAL_HARDENING_CASE"
  exit $?
fi

make_controller_mocks
failures=0
run_case() {
  local name=$1 function=$2 limit=${3:-12} rc
  set +e
  timeout --foreground -k 2s "${limit}s" env FINAL_HARDENING_CASE="$function" "$0"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ]; then
    printf 'ok - %s\n' "$name"
  else
    printf 'not ok - %s (rc=%s)\n' "$name" "$rc" >&2
    failures=$((failures + 1))
  fi
}

run_case 'spawn hour/count reject arithmetic payloads explicitly' case_numeric_spawn_state_is_explicitly_rejected
run_case 'task failure count emits structured mutable-state repair' case_numeric_fail_count_emits_state_repair 15
run_case 'heartbeat/spawn timestamp emits structured repair' case_numeric_heartbeat_spawn_timestamp_emits_repair
run_case 'concurrency value/timestamp emit structured repair' case_numeric_concurrency_state_emits_repair
run_case 'watchdog PAUSE timestamp emits explicit repair' case_numeric_watchdog_pause_timestamp_emits_repair
run_case 'watchdog verdict/cooldown timestamps emit explicit repair' case_numeric_watchdog_verdict_and_cooldown_emit_repair 15
run_case 'ordinary state writes refuse symlink targets' case_ordinary_state_writes_refuse_symlinks
run_case 'empty completion executes only clean HEAD validator' case_empty_completion_uses_clean_head_validator
run_case 'completion holds queue lock and rechecks emptiness' case_completion_holds_queue_lock_and_rechecks_empty
run_case 'probe execution requires a committed allowlist id' case_probe_requires_committed_allowlist_id
run_case 'reservation final-CASes queue before claim publication' case_reservation_rechecks_queue_before_claim_publication
run_case 'reaper unlinks only its opened claim inode' case_reaper_never_unlinks_a_replacement_inode
run_case 're-sealed invalid wait numbers are rejected' case_wait_cache_rejects_resealed_invalid_numbers 20
run_case 'wait verification is read-only' case_wait_verification_is_read_only
run_case 'queue destination swap after publication is detected' case_queue_post_publish_swap_is_detected
run_case 'due probes and expiry run before network checks' case_controller_runs_due_wait_before_network
run_case 'present failure artifact bypasses cooldown' case_present_failure_ignores_watchdog_cooldown
run_case 'unrelated trailer commit cannot acknowledge a failure' case_unrelated_trailer_commit_cannot_acknowledge_failure
run_case 'canonical metadata order is enforced' case_canonical_metadata_order_is_enforced
run_case 'five-unit P4 contract retires stale live facts' case_active_p4_contract_retires_stale_live_facts
run_case 'capgen smoke is locked/offline and validator command is real' case_capgen_smoke_and_validator_invocation_are_offline
run_case 'mutable files and entry counts have practical caps' case_mutable_inputs_have_practical_size_and_count_caps
run_case 'normative future phases reject human-only engineering gates' case_normative_future_phase_plan_has_no_human_only_engineering_gates

if [ "$failures" -ne 0 ]; then
  printf 'final hardening tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'final hardening tests: PASS'
