#!/usr/bin/env bash
# Focused behavioral RED contracts from the final reviewer/security pass.
#
# Boundary: queue writers which deliberately bypass every production wrapper
# and helper are trusted code and intentionally out of scope.  The race tests
# below use the production queue/state APIs and their shared-lock contract.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PARSER="$ROOT/tools/nudge-free-items.py"
STATE_HELPER="$ROOT/tools/nudge-state.py"
WAIT_EXECUTOR="$ROOT/tools/nudge-wait.py"
REAPER="$ROOT/tools/nudge-reap-claims.sh"
AGENT="$ROOT/tools/nudge-agent.sh"
CONTROLLER="$ROOT/tools/nudge.sh"
WATCHDOG="$ROOT/tools/llm-watchdog.sh"
VALIDATOR="$ROOT/tools/validate-required-gates.py"
TMP=$(mktemp -d /tmp/opencode/bedlam-reviewer-security.XXXXXX)

cleanup() {
  local pid
  for file in "$TMP"/*.pid; do
    [ -s "$file" ] || continue
    pid=$(cat "$file" 2>/dev/null || true)
    [[ "$pid" =~ ^[1-9][0-9]*$ ]] || continue
    kill -TERM "$pid" 2>/dev/null || true
    kill -KILL "$pid" 2>/dev/null || true
  done
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

publish_claim() {
  local plan=$1 session=$2 destination=${3:-}
  local fields status item_id gate body device inode queue claim
  fields=$($PARSER "$plan/.state/NEXT.md" "$plan/.state/claims" --item-v2 1)
  read -r status item_id gate body device inode queue <<< "$fields"
  [ "$status" = READY ]
  claim="$plan/.state/claims/1-$session.claim"
  "$STATE_HELPER" publish-claim "$plan/.state/claims" "1-$session.claim" \
    1 "$item_id" "$gate" "$session" "$(date -Is)" \
    "bedlam-nudge-item1-$session" $$ "$body" "$device" "$inode" "$queue"
  [ -z "$destination" ] || mv "$claim" "$destination"
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
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$network" REAPER_OVERRIDE="$REAPER" NOTIFY_SEND= \
    "$CONTROLLER"
}

install_completion_contract() {
  local plan=$1
  cp "$VALIDATOR" "$plan/tools/validate-required-gates.py"
  chmod +x "$plan/tools/validate-required-gates.py"
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "green"
command = ["/usr/bin/true"]
timeout_seconds = 2
EOF
  printf 'anchor\n' > "$plan/manifest-anchor.txt"
  printf '%s  manifest-anchor.txt\n' "$(sha256sum "$plan/manifest-anchor.txt" | awk '{print $1}')" \
    > "$plan/MANIFEST.sha256"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  git -C "$plan" add .state/NEXT.md tools/validate-required-gates.py \
    docs/required-gates.toml manifest-anchor.txt MANIFEST.sha256
  git -C "$plan" commit -qm completion-contract
}

case_claim_lifecycle_uses_pinned_handles() {
  local plan="$TMP/claim-owner-open" session=owner-open bin="$TMP/claim-owner-bin"
  local parser_plan="$TMP/unlocked-owner" state rc failures=0
  make_repo "$plan"
  publish_claim "$plan" "$session"
  mkdir -p "$bin"
  cat > "$bin/ln" <<EOF
#!/usr/bin/env bash
/usr/bin/ln "\$@" || exit \$?
destination="\${!#}"
if [[ "\$destination" == *-owner.claim ]]; then
  /usr/bin/mv "\$destination" "\$destination.published"
  /usr/bin/cp "\$destination.published" "\$destination"
  chmod 600 "\$destination"
  : > "$TMP/owner-swapped-before-open"
fi
EOF
  cat > "$TMP/owner-model" <<EOF
#!/usr/bin/env bash
touch "$TMP/owner-forge-launched"
exit 42
EOF
  chmod +x "$bin/ln" "$TMP/owner-model"
  set +e
  PATH="$bin:$PATH" BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/owner-model" \
    NUDGE_IDLE_POLL=0.01 "$AGENT" 1 "$session" >/dev/null 2>&1
  rc=$?
  set -e
  [ -e "$TMP/owner-swapped-before-open" ] || failures=$((failures + 1))
  if [ -e "$TMP/owner-forge-launched" ] || [ "$rc" -eq 42 ]; then
    echo 'owner pathname replacement before authoritative open launched work' >&2
    failures=$((failures + 1))
  fi

  make_repo "$parser_plan"
  publish_claim "$parser_plan" forge "$parser_plan/.state/claims/1-owner.claim"
  state=$($PARSER "$parser_plan/.state/NEXT.md" "$parser_plan/.state/claims" --state-v1)
  if [ "$state" != 'RUNNABLE 1' ]; then
    echo "unlocked forgeable owner claim suppressed work: $state" >&2
    failures=$((failures + 1))
  fi

  touch -d '+30 days' "$parser_plan/.state/claims/1-owner.claim"
  DEAD_CLAIM_TTL=0 MALFORMED_CLAIM_TTL=0 "$REAPER" \
    "$parser_plan/.state/claims" "$parser_plan/.state/reaper.log" >/dev/null 2>&1 || true
  if find "$parser_plan/.state/claims" -maxdepth 1 -name '*.claim' -print -quit | grep -q .; then
    echo 'future-dated claim remained an unbounded suppression candidate' >&2
    failures=$((failures + 1))
  fi

  # Enumerate from the directory descriptor already opened by the parser.
  # Replacing the pathname just before scandir must not redirect enumeration.
  local pinned="$TMP/claims-pinned" replacement="$TMP/claims-replacement" hook="$TMP/claims-hook"
  mkdir -p "$pinned/.state/claims" "$pinned/tools" "$replacement" "$hook"
  cat > "$pinned/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [READY] [id=stable-one] [gate=gate-one] pinned directory task

## Backlog
EOF
  printf 'forged\n' > "$replacement/1-owner.claim"
  chmod 600 "$replacement/1-owner.claim"
  cat > "$hook/sitecustomize.py" <<PY
import os
original_scandir = os.scandir
def raced_scandir(path):
    if str(path) == "$pinned/.state/claims" and not os.path.exists("$TMP/claims-dir-swapped"):
        os.rename("$pinned/.state/claims", "$pinned/.state/claims.pinned")
        os.rename("$replacement", "$pinned/.state/claims")
        open("$TMP/claims-dir-swapped", "w").close()
    return original_scandir(path)
os.scandir = raced_scandir
PY
  set +e
  state=$(PYTHONPATH="$hook" "$PARSER" "$pinned/.state/NEXT.md" \
    "$pinned/.state/claims" --state-v1 2>"$pinned/error")
  rc=$?
  set -e
  [ -e "$TMP/claims-dir-swapped" ] || failures=$((failures + 1))
  if [ "$rc" -ne 0 ] || [ "$state" != 'RUNNABLE 1' ]; then
    echo 'claims enumeration followed a replaced directory pathname' >&2
    failures=$((failures + 1))
  fi
  [ "$failures" -eq 0 ]
}

case_empty_queue_precedes_early_exits() {
  local mode plan rc failures=0 network
  for mode in connectivity heartbeat spawn-cap concurrency; do
    plan="$TMP/empty-order-$mode"
    make_repo "$plan"
    install_completion_contract "$plan"
    rm -f "$plan/.state/PLAN-COMPLETE"
    network="$TMP/network-ok"
    case "$mode" in
      connectivity)
        cat > "$TMP/network-empty-offline" <<EOF
#!/usr/bin/env bash
touch "$TMP/network-called-before-empty-validation"
exit 75
EOF
        chmod +x "$TMP/network-empty-offline"
        network="$TMP/network-empty-offline"
        ;;
      heartbeat) touch "$plan/.state/heartbeat" ;;
      spawn-cap) printf '%s 16\n' "$(( $(date +%s) / 3600 ))" > "$plan/.state/spawns" ;;
      concurrency) printf '0\n' > "$plan/.state/concurrency" ;;
    esac
    set +e
    run_controller "$plan" "$TMP/empty-order-$mode.lock" "$network" >/dev/null 2>&1
    rc=$?
    set -e
    if [ "$rc" -ne 0 ] || [ ! -f "$plan/.state/PLAN-COMPLETE" ]; then
      echo "empty queue did not validate before $mode exit" >&2
      failures=$((failures + 1))
    fi
  done
  [ ! -e "$TMP/network-called-before-empty-validation" ] || {
    echo 'connectivity was consulted before empty-queue validation' >&2
    failures=$((failures + 1))
  }
  [ "$failures" -eq 0 ]
}

case_external_corpus_completion_is_isolated_readonly() {
  local plan="$TMP/external-corpus" before after mode_before mode_after rc
  make_repo "$plan"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  cp "$VALIDATOR" "$plan/tools/validate-required-gates.py"
  chmod +x "$plan/tools/validate-required-gates.py"
  mkdir -p "$plan/game-data"
  printf 'game-data/\n' > "$plan/.gitignore"
  printf 'real external corpus\n' > "$plan/game-data/original.bin"
  chmod 400 "$plan/game-data/original.bin"
  before=$(sha256sum "$plan/game-data/original.bin" | awk '{print $1}')
  mode_before=$(stat -c %a "$plan/game-data/original.bin")
  printf '%s  game-data/original.bin\n' "$before" > "$plan/MANIFEST.sha256"
  cat > "$plan/tools/corpus-gate.py" <<PY
#!/usr/bin/python3
import hashlib, os, pathlib
p=pathlib.Path('game-data/original.bin')
assert hashlib.sha256(p.read_bytes()).hexdigest() == '$before'
assert os.stat(p).st_mode & 0o222 == 0
PY
  chmod +x "$plan/tools/corpus-gate.py"
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "corpus"
command = ["/usr/bin/python3", "tools/corpus-gate.py"]
timeout_seconds = 2
EOF
  git -C "$plan" add .state/NEXT.md .gitignore MANIFEST.sha256 tools \
    docs/required-gates.toml
  git -C "$plan" commit -qm 'production-style external corpus contract'
  set +e
  "$STATE_HELPER" complete-from-head "$plan" "$plan/.state/report.json" \
    "$plan/.state/PLAN-COMPLETE" >"$plan/result" 2>&1
  rc=$?
  set -e
  after=$(sha256sum "$plan/game-data/original.bin" | awk '{print $1}')
  mode_after=$(stat -c %a "$plan/game-data/original.bin")
  [ "$rc" -eq 0 ]
  [ -f "$plan/.state/PLAN-COMPLETE" ]
  [ "$before" = "$after" ]
  [ "$mode_before" = "$mode_after" ]
}

case_probe_allowlist_missing_empty_invalid_fails_closed() {
  local mode plan rc failures=0
  for mode in missing empty invalid; do
    plan="$TMP/probe-registry-$mode"
    make_repo "$plan"
    cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=tools/arbitrary.sh] [retry=1s] [timeout=10s] registry fixture

## Backlog
EOF
    printf '#!/usr/bin/env bash\nexit 1\n' > "$plan/tools/arbitrary.sh"
    chmod +x "$plan/tools/arbitrary.sh"
    case "$mode" in
      empty) : > "$plan/docs/automatic-probes.toml" ;;
      invalid) printf 'not = [valid toml\n' > "$plan/docs/automatic-probes.toml" ;;
    esac
    git -C "$plan" add .state/NEXT.md tools/arbitrary.sh
    [ "$mode" = missing ] || git -C "$plan" add docs/automatic-probes.toml
    git -C "$plan" commit -qm "probe registry $mode"
    set +e
    "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 \
      >"$plan/out" 2>"$plan/error"
    rc=$?
    set -e
    if [ "$rc" -ne 2 ]; then
      echo "$mode HEAD probe registry accepted arbitrary tools executable" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_checkout_disables_hooks_and_rejects_plants() {
  local plan="$TMP/hook-checkout" hookdir="$TMP/hook-dir" rc
  make_repo "$plan"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  cp "$VALIDATOR" "$plan/tools/validate-required-gates.py"
  chmod +x "$plan/tools/validate-required-gates.py"
  cat > "$plan/tools/planted-gate.sh" <<'EOF'
#!/bin/bash
/usr/bin/test -f planted-input
EOF
  chmod +x "$plan/tools/planted-gate.sh"
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "plant"
command = ["/bin/bash", "tools/planted-gate.sh"]
timeout_seconds = 2
EOF
  printf 'anchor\n' > "$plan/manifest-anchor.txt"
  printf '%s  manifest-anchor.txt\n' "$(sha256sum "$plan/manifest-anchor.txt" | awk '{print $1}')" > "$plan/MANIFEST.sha256"
  git -C "$plan" add .state/NEXT.md tools docs/required-gates.toml \
    manifest-anchor.txt MANIFEST.sha256
  git -C "$plan" commit -qm 'hook planting fixture'
  mkdir -p "$hookdir"
  cat > "$hookdir/post-checkout" <<EOF
#!/usr/bin/env bash
touch "$TMP/post-checkout-hook-ran"
printf 'planted by hook\n' > planted-input
EOF
  chmod +x "$hookdir/post-checkout"
  git -C "$plan" config core.hooksPath "$hookdir"
  set +e
  "$STATE_HELPER" complete-from-head "$plan" "$plan/.state/report.json" \
    "$plan/.state/PLAN-COMPLETE" >/dev/null 2>&1
  rc=$?
  set -e
  [ "$rc" -ne 0 ]
  [ ! -e "$TMP/post-checkout-hook-ran" ]
  [ ! -e "$plan/.state/PLAN-COMPLETE" ]
}

case_validator_uses_network_and_readonly_sandbox() {
  local plan="$TMP/bwrap-validator" server="$TMP/socket-server.py" port before_tracked before_corpus
  local after_tracked after_corpus rc
  cat > "$server" <<EOF
import pathlib, socket
s=socket.socket(); s.bind(('127.0.0.1', 0)); s.listen()
pathlib.Path('$TMP/socket-port').write_text(str(s.getsockname()[1]))
while True:
    c,_=s.accept(); c.close()
EOF
  python3 "$server" & echo $! > "$TMP/socket-server.pid"
  for _ in $(seq 1 200); do [ -s "$TMP/socket-port" ] && break; sleep 0.01; done
  port=$(cat "$TMP/socket-port")
  make_repo "$plan"
  mkdir -p "$plan/game-data"
  printf 'tracked immutable\n' > "$plan/tracked.txt"
  printf 'external immutable\n' > "$plan/game-data/corpus.bin"
  cat > "$plan/tools/sandbox-gate.py" <<PY
#!/usr/bin/python3
import os, pathlib, socket
failed=False
try:
    s=socket.create_connection(('127.0.0.1', $port), timeout=.3); s.close(); failed=True
except OSError:
    pass
for name in ('tracked.txt', 'game-data/corpus.bin'):
    try:
        pathlib.Path(name).open('ab').write(b'x'); failed=True
    except OSError:
        pass
allowed={'CARGO_NET_OFFLINE','GIT_TERMINAL_PROMPT','HOME','LANG','LC_ALL','TZ'}
failed = failed or bool(set(os.environ) - allowed)
raise SystemExit(1 if failed else 0)
PY
  chmod +x "$plan/tools/sandbox-gate.py"
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "sandbox"
command = ["/usr/bin/python3", "tools/sandbox-gate.py"]
timeout_seconds = 2
EOF
  printf '%s  game-data/corpus.bin\n' "$(sha256sum "$plan/game-data/corpus.bin" | awk '{print $1}')" > "$plan/MANIFEST.sha256"
  git -C "$plan" add tracked.txt tools/sandbox-gate.py docs/required-gates.toml MANIFEST.sha256
  git -C "$plan" commit -qm sandbox
  before_tracked=$(sha256sum "$plan/tracked.txt" | awk '{print $1}')
  before_corpus=$(sha256sum "$plan/game-data/corpus.bin" | awk '{print $1}')
  set +e
  python3 "$VALIDATOR" --root "$plan" --report "$plan/report.json" >/dev/null 2>&1
  rc=$?
  set -e
  after_tracked=$(sha256sum "$plan/tracked.txt" | awk '{print $1}')
  after_corpus=$(sha256sum "$plan/game-data/corpus.bin" | awk '{print $1}')
  [ "$rc" -eq 0 ]
  [ "$before_tracked" = "$after_tracked" ]
  [ "$before_corpus" = "$after_corpus" ]
}

case_completion_generation_survives_lock_handoff_race() {
  local plan="$TMP/completion-generation" writer controller rc
  make_repo "$plan"
  for name in nudge.sh nudge-lock.py nudge-free-items.py nudge-state.py nudge-wait.py \
      nudge-reserve.sh nudge-reap-claims.sh nudge-claim.sh network-watchdog.sh \
      validate-required-gates.py; do
    cp "$ROOT/tools/$name" "$plan/tools/$name"
  done
  chmod +x "$plan/tools/"*
  install_completion_contract "$plan"
  # Instrument only the lock adapter in the fixture: after complete-from-head
  # returns, release the queue lock and let a production lock-using writer win
  # before the controller makes its publication decision.
  mv "$plan/tools/nudge-lock.py" "$plan/tools/nudge-lock-real.py"
  cat > "$plan/tools/nudge-lock.py" <<PY
#!/usr/bin/python3
import fcntl, os, pathlib, subprocess, sys, time
path=sys.argv[2]; mode=sys.argv[3]; command=sys.argv[4:]
fd=os.open(path, os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW, 0o600)
fcntl.flock(fd, fcntl.LOCK_EX | (fcntl.LOCK_NB if mode=='nonblocking' else 0))
rc=subprocess.run(command).returncode
if path.endswith('/.queue.lock') and any('complete-from-head' == x for x in command):
    fcntl.flock(fd, fcntl.LOCK_UN)
    pathlib.Path('$TMP/completion-lock-released').touch()
    for _ in range(400):
        if pathlib.Path('$TMP/completion-writer-done').exists(): break
        time.sleep(.01)
os.close(fd)
raise SystemExit(rc)
PY
  chmod +x "$plan/tools/nudge-lock.py"
  git -C "$plan" add tools/nudge-lock.py tools/nudge-lock-real.py
  git -C "$plan" commit -qm 'fixture lock barrier'
  cat > "$TMP/raced-queue" <<'EOF'
# NEXT

## Now
1. [READY] [id=raced-work] [gate=raced-gate] arrived after validation

## Backlog
EOF
  (
    while [ ! -e "$TMP/completion-lock-released" ]; do sleep 0.01; done
    "$ROOT/tools/nudge-lock.py" lock-run "$plan/.state/.queue.lock" blocking \
      "$STATE_HELPER" write-text "$plan/.state/NEXT.md" "$(cat "$TMP/raced-queue")"
    : > "$TMP/completion-writer-done"
  ) & writer=$!
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-generation.lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" \
    NOTIFY_SEND= "$plan/tools/nudge.sh" >/dev/null 2>&1 & controller=$!
  set +e
  wait "$controller"; rc=$?
  set -e
  wait "$writer"
  grep -q '\[id=raced-work\]' "$plan/.state/NEXT.md"
  [ "$rc" -ne 0 ]
  [ ! -e "$plan/.state/PLAN-COMPLETE" ]
}

case_queue_writer_api_locks_and_postchecks_digest() {
  local plan="$TMP/queue-api" writer hook="$TMP/queue-api-hook" rc failures=0 before
  make_repo "$plan"
  before=$(sha256sum "$plan/.state/NEXT.md" | awk '{print $1}')
  (
    exec 9>"$plan/.state/.queue.lock"; flock 9
    : > "$TMP/queue-api-lock-held"
    while [ ! -e "$TMP/queue-api-release" ]; do sleep 0.01; done
  ) & echo $! > "$TMP/queue-api-holder.pid"
  for _ in $(seq 1 100); do [ -e "$TMP/queue-api-lock-held" ] && break; sleep 0.01; done
  "$STATE_HELPER" write-text "$plan/.state/NEXT.md" $'# NEXT\n\n## Now\n\n## Backlog\n' &
  writer=$!
  sleep .3
  if ! kill -0 "$writer" 2>/dev/null || [ "$(sha256sum "$plan/.state/NEXT.md" | awk '{print $1}')" != "$before" ]; then
    echo 'production queue helper bypassed the shared queue lock' >&2
    failures=$((failures + 1))
  fi
  : > "$TMP/queue-api-release"
  wait "$writer" 2>/dev/null || true

  make_repo "$plan"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
original_replace=os.replace
def raced_replace(source, destination, *args, **kwargs):
    result=original_replace(source, destination, *args, **kwargs)
    if str(destination) == 'NEXT.md' and not os.path.exists('$TMP/queue-final-raced'):
        dst=kwargs.get('dst_dir_fd')
        os.rename('NEXT.md', '.NEXT.published', src_dir_fd=dst, dst_dir_fd=dst)
        os.link('.NEXT.published', 'NEXT.md', src_dir_fd=dst, dst_dir_fd=dst)
        fd=os.open('NEXT.md', os.O_WRONLY|os.O_TRUNC, dir_fd=dst)
        os.write(fd, b'attacker replacement through same published inode\n'); os.close(fd)
        open('$TMP/queue-final-raced','w').close()
    return result
os.replace=raced_replace
PY
  set +e
  PYTHONPATH="$hook" "$STATE_HELPER" write-text "$plan/.state/NEXT.md" \
    $'# NEXT\n\n## Now\n\n## Backlog\n' >/dev/null 2>&1
  rc=$?
  set -e
  [ -e "$TMP/queue-final-raced" ] || failures=$((failures + 1))
  if [ "$rc" -eq 0 ]; then
    echo 'queue helper accepted same-inode content replacement at final barrier' >&2
    failures=$((failures + 1))
  fi
  [ "$failures" -eq 0 ]
}

case_probe_setsess_descendant_is_contained_or_refused() {
  local plan="$TMP/probe-setsid" digest child rc
  make_repo "$plan"
  cat > "$plan/tools/escape.sh" <<EOF
#!/usr/bin/env bash
setsid sleep 30 &
echo \$! > "$TMP/probe-setsid-child.pid"
exit 0
EOF
  chmod +x "$plan/tools/escape.sh"
  digest=$(sha256sum "$plan/tools/escape.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "escape"
path = "tools/escape.sh"
sha256 = "$digest"
EOF
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=escape] [retry=100ms] [timeout=5s] containment fixture

## Backlog
EOF
  git -C "$plan" add .state/NEXT.md tools/escape.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm probe-containment
  set +e
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null 2>&1
  rc=$?
  set -e
  if [ -s "$TMP/probe-setsid-child.pid" ]; then
    child=$(cat "$TMP/probe-setsid-child.pid")
    sleep .1
    if kill -0 "$child" 2>/dev/null; then
      kill -KILL "$child" 2>/dev/null || true
      echo "setsid probe descendant escaped containment (executor rc=$rc)" >&2
      return 1
    fi
  fi
}

write_failure() {
  local plan=$1 name=$2
  mkdir -m 700 -p "$plan/.state/automation-failures"
  cat > "$plan/.state/automation-failures/$name.json" <<EOF
{"schema":"nudge-failure-v1","version":1,"ordinal":1,"id":"stable-one","gate":"gate-one","owner":"worker","session":"$name","kind":"client-error","reason":"fixture","evidence":"fixture","time":"2026-08-26T07:00:00Z","repair":"required","queue_unchanged":true}
EOF
  chmod 600 "$plan/.state/automation-failures/$name.json"
}

case_watchdog_rejects_split_trailer_and_queue_commits() {
  local plan="$TMP/watchdog-split"
  make_repo "$plan"
  write_failure "$plan" split-trigger
  cat > "$TMP/watchdog-split-model" <<EOF
#!/usr/bin/env bash
token=\$(cat "$plan/.state/PAUSE")
echo trailer-commit >> "$plan/code.txt"
git -C "$plan" add code.txt
git -C "$plan" commit -qm trailer-only -m "Watchdog-Repair: \$token"
trailer_commit=\$(git -C "$plan" rev-parse HEAD)
printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
git -C "$plan" add .state/NEXT.md
git -C "$plan" commit -qm queue-postcondition-without-trailer
python3 - "$plan/.state/llm-watchdog-failure-snapshot.json" \
  "$plan/.state/llm-watchdog-failure-ack.json" "\$trailer_commit" <<'PY'
import json, sys
records=json.load(open(sys.argv[1]))
for record in records:
    record['resolution']='required-empty'
    record['remediation_commit']=sys.argv[3]
json.dump({'schema':'nudge-failure-ack-v1','records':records}, open(sys.argv[2],'w'))
PY
exit 0
EOF
  chmod +x "$TMP/watchdog-split-model"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/watchdog-split-model" \
    WATCHDOG_TEST_MODE=1 LLM_WATCHDOG_MIN_INTERVAL=0 SUPERVISE_TIMEOUT=2 REPAIR_TIMEOUT=5 \
    RESUME_WAIT_LOOPS=0 SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
    LLM_WATCHDOG_LOCK="$TMP/watchdog-split.lock" NOTIFY_SEND= "$WATCHDOG" >/dev/null 2>&1 || true
  [ -e "$plan/.state/automation-failures/split-trigger.json" ]
  ! grep -q '^state=repaired$' "$plan/.state/llm-watchdog-verdict"
}

case_reaper_restores_replacement_moved_after_identity_check() {
  local mode claims path hook rc replacement_inode failures=0
  for mode in valid malformed; do
    claims="$TMP/reaper-final-$mode"
    path="$claims/1-race.claim"
    hook="$TMP/reaper-final-$mode-hook"
    mkdir -p "$claims" "$hook"
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
    if [ "$mode" = malformed ]; then printf 'duplicate=one\nduplicate=two\n' >> "$path"; fi
    printf 'attacker replacement generation\n' > "$path.replacement"
    chmod 600 "$path" "$path.replacement"
    replacement_inode=$(stat -c %i "$path.replacement")
    touch -d '1 hour ago' "$path"
    cat > "$hook/sitecustomize.py" <<PY
import os
target = "1-race.claim"
marker = "$TMP/reaper-final-$mode-swapped"
original_rename = os.rename
original_unlink = os.unlink

def swap(directory_fd):
    original_rename(target, target + ".validated", src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
    original_rename(target + ".replacement", target, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
    open(marker, "w").close()

def raced_rename(source, destination, *args, **kwargs):
    if str(source) == target and str(destination).startswith(".quarantine-") and not os.path.exists(marker):
        swap(kwargs.get("src_dir_fd"))
    return original_rename(source, destination, *args, **kwargs)

def raced_unlink(name, *args, **kwargs):
    if str(name) == target and not os.path.exists(marker):
        swap(kwargs.get("dir_fd"))
    return original_unlink(name, *args, **kwargs)

os.rename = raced_rename
os.unlink = raced_unlink
PY
    set +e
    PYTHONPATH="$hook" DEAD_CLAIM_TTL=0 RESERVATION_TTL=0 \
      MALFORMED_CLAIM_TTL=0 "$REAPER" "$claims" "$claims/reaper.log" \
      >"$claims/out" 2>&1
    rc=$?
    set -e
    if [ ! -e "$TMP/reaper-final-$mode-swapped" ]; then
      echo "reaper $mode fixture did not reach the final mutation window" >&2
      failures=$((failures + 1))
    elif [ "$rc" -eq 0 ] || [ ! -f "$path" ] \
        || [ "$(stat -c %i "$path" 2>/dev/null || echo missing)" != "$replacement_inode" ]; then
      echo "reaper $mode consumed rather than restored the post-check replacement inode" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_wait_state_parent_swap_stays_on_pinned_directory() {
  local plan="$TMP/wait-parent-pin" state_dir pinned attacker hook digest deadline rc
  plan="$TMP/wait-parent-pin-plan"
  state_dir="$plan/.state/automatic-waits"
  pinned="$state_dir.pinned"
  attacker="$TMP/wait-parent-attacker"
  hook="$TMP/wait-parent-hook"
  make_repo "$plan"
  deadline=$(date -u -d '+1 minute' '+%Y-%m-%dT%H:%M:%SZ')
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=pin-check] [retry=100ms] [deadline=$deadline] pinned state fixture

## Backlog
EOF
  cat > "$plan/tools/pin-check.sh" <<EOF
#!/usr/bin/env bash
test -f "$pinned/stable-one.json" || exit 91
touch "$TMP/wait-parent-probe-saw-pinned-state"
exit 0
EOF
  chmod +x "$plan/tools/pin-check.sh"
  digest=$(sha256sum "$plan/tools/pin-check.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "pin-check"
path = "tools/pin-check.sh"
sha256 = "$digest"
EOF
  git -C "$plan" add .state/NEXT.md tools/pin-check.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm wait-parent-pin
  mkdir -m 700 "$state_dir" "$attacker" "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
marker = "$TMP/wait-parent-swapped"
original_open = os.open
def raced_open(path, flags, *args, **kwargs):
    if str(path).endswith(".executor.lock") and not os.path.exists(marker):
        os.rename("$state_dir", "$pinned")
        os.rename("$attacker", "$state_dir")
        open(marker, "w").close()
    return original_open(path, flags, *args, **kwargs)
os.open = raced_open
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$state_dir" \
    >"$plan/wait-parent.out" 2>&1
  rc=$?
  set -e
  [ -e "$TMP/wait-parent-swapped" ]
  if [ "$rc" -ne 0 ] || [ ! -e "$TMP/wait-parent-probe-saw-pinned-state" ] \
      || find "$state_dir" -mindepth 1 -print -quit | grep -q . \
      || [ -e "$pinned/stable-one.json" ]; then
    echo 'wait-state create/replace/unlink escaped the initially opened parent directory' >&2
    return 1
  fi
  grep -q '^1\. \[READY\]' "$plan/.state/NEXT.md"
}

case_head_materialization_never_invokes_source_upload_pack() {
  local plan="$TMP/head-upload-pack" hook="$TMP/malicious-pack-objects"
  local trace_hook="$TMP/source-upload-trace-hook" rc
  make_repo "$plan"
  install_completion_contract "$plan"
  cat > "$hook" <<EOF
#!/usr/bin/env bash
touch "$TMP/source-upload-pack-ran"
exit 97
EOF
  chmod +x "$hook"
  git -C "$plan" config uploadpack.packObjectsHook "$hook"
  mkdir -p "$trace_hook"
  cat > "$trace_hook/sitecustomize.py" <<PY
import pathlib, subprocess
original_run = subprocess.run
def traced_run(command, *args, **kwargs):
    argv = [str(part) for part in command] if isinstance(command, (list, tuple)) else []
    if "fetch" in argv and "$plan" in argv:
        pathlib.Path("$TMP/source-upload-pack-invoked").touch()
        # Model the source-side upload-pack boundary honoring its own mutable
        # hook.  Do not depend on this host Git's protected-config policy,
        # which intentionally ignores a repository-local packObjectsHook.
        original_run(["$hook"], check=False)
    return original_run(command, *args, **kwargs)
subprocess.run = traced_run
PY
  set +e
  PYTHONPATH="$trace_hook" "$STATE_HELPER" complete-from-head "$plan" "$plan/.state/report.json" \
    "$plan/.state/PLAN-COMPLETE" >"$plan/result" 2>&1
  rc=$?
  set -e
  if [ -e "$TMP/source-upload-pack-invoked" ] || [ -e "$TMP/source-upload-pack-ran" ]; then
    echo "isolated HEAD materialization invoked mutable source upload-pack/config (rc=$rc)" >&2
    return 1
  fi
}

case_isolated_real_cargo_gate_has_writable_mounted_target_only() {
  local plan="$TMP/isolated-cargo" rc
  make_repo "$plan"
  printf '# NEXT\n\n## Now\n\n## Backlog\n' > "$plan/.state/NEXT.md"
  cp "$VALIDATOR" "$plan/tools/validate-required-gates.py"
  chmod +x "$plan/tools/validate-required-gates.py"
  mkdir -p "$plan/src"
  cat > "$plan/Cargo.toml" <<'EOF'
[package]
name = "isolated-cargo-fixture"
version = "0.1.0"
edition = "2021"
build = "build.rs"
EOF
  cat > "$plan/Cargo.lock" <<'EOF'
# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "isolated-cargo-fixture"
version = "0.1.0"
EOF
  cat > "$plan/src/lib.rs" <<'EOF'
pub fn answer() -> u8 { 42 }
#[test]
fn cargo_reaches_the_mounted_target() { assert_eq!(answer(), 42); }
EOF
  cat > "$plan/build.rs" <<'EOF'
fn main() {
    if std::fs::write("source-write-sentinel", b"source tree was writable").is_ok() {
        panic!("isolated source tree unexpectedly writable");
    }
}
EOF
  cat > "$plan/docs/required-gates.toml" <<'EOF'
schema = "required-gates-v1"
[[gate]]
id = "real-cargo"
command = ["/usr/bin/cargo", "test", "--locked", "--offline"]
timeout_seconds = 30
EOF
  printf 'anchor\n' > "$plan/manifest-anchor.txt"
  printf '%s  manifest-anchor.txt\n' "$(sha256sum "$plan/manifest-anchor.txt" | awk '{print $1}')" \
    > "$plan/MANIFEST.sha256"
  git -C "$plan" add .state/NEXT.md tools/validate-required-gates.py Cargo.toml Cargo.lock \
    build.rs src/lib.rs docs/required-gates.toml manifest-anchor.txt MANIFEST.sha256
  git -C "$plan" commit -qm isolated-real-cargo
  set +e
  "$STATE_HELPER" complete-from-head "$plan" "$plan/.state/report.json" \
    "$plan/.state/PLAN-COMPLETE" >"$plan/result" 2>&1
  rc=$?
  set -e
  if [ "$rc" -ne 0 ] || [ ! -f "$plan/.state/PLAN-COMPLETE" ]; then
    echo 'real isolated Cargo gate lacked its precreated writable target mountpoint' >&2
    return 1
  fi
  [ ! -e "$plan/target" ]
  [ ! -e "$plan/source-write-sentinel" ]
}

case_invalid_queue_bypasses_watchdog_interval_and_cooldown() {
  local plan="$TMP/urgent-invalid-queue" future
  make_repo "$plan"
  sed -i 's/\[READY\]/[NOT-A-STATE]/' "$plan/.state/NEXT.md"
  future=$(( $(date +%s) + 3600 ))
  printf '%s\n' "$future" > "$plan/.state/llm-watchdog-cooldown-until"
  printf 'time=%s\nstate=healthy\nrc=0\nmarkers=1\ncooldown_until=%s\n' \
    "$(date -Is)" "$future" > "$plan/.state/llm-watchdog-verdict"
  chmod 600 "$plan/.state/llm-watchdog-cooldown-until" "$plan/.state/llm-watchdog-verdict"
  cat > "$TMP/urgent-invalid-model" <<EOF
#!/usr/bin/env bash
touch "$TMP/urgent-invalid-model-called"
exit 1
EOF
  chmod +x "$TMP/urgent-invalid-model"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/urgent-invalid-model" WATCHDOG_TEST_MODE=1 \
    LLM_WATCHDOG_MIN_INTERVAL=120 SUPERVISE_TIMEOUT=1 REPAIR_TIMEOUT=1 RESUME_WAIT_LOOPS=0 \
    SYSTEMCTL_OVERRIDE="$TMP/systemctl" REAPER_OVERRIDE="$REAPER" \
    LLM_WATCHDOG_LOCK="$TMP/urgent-invalid.lock" NOTIFY_SEND= \
    "$WATCHDOG" >"$plan/result" 2>&1 || true
  if [ ! -e "$TMP/urgent-invalid-model-called" ]; then
    echo 'invalid queue urgent repair was suppressed by interval or repair cooldown' >&2
    return 1
  fi
  ! grep -q '^state=repair-deferred$' "$plan/.state/llm-watchdog-verdict"
}

case_urgent_repair_quarantines_malformed_throttle_state() {
  local mode plan throttle payload rc candidate quarantined failures=0
  for mode in verdict cooldown; do
    plan="$TMP/urgent-malformed-$mode"
    make_repo "$plan"
    sed -i 's/\[READY\]/[NOT-A-STATE]/' "$plan/.state/NEXT.md"
    write_failure "$plan" "malformed-$mode-trigger"
    case "$mode" in
      verdict) throttle="$plan/.state/llm-watchdog-verdict" ;;
      cooldown) throttle="$plan/.state/llm-watchdog-cooldown-until" ;;
    esac
    payload="malformed-$mode-throttle-fixture"
    printf '%s\n' "$payload" > "$throttle"
    chmod 600 "$throttle"
    cat > "$TMP/urgent-malformed-$mode-model" <<EOF
#!/usr/bin/env bash
touch "$TMP/urgent-malformed-$mode-model-called"
exit 1
EOF
    chmod +x "$TMP/urgent-malformed-$mode-model"
    set +e
    BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE="$TMP/urgent-malformed-$mode-model" \
      WATCHDOG_TEST_MODE=1 LLM_WATCHDOG_MIN_INTERVAL=120 SUPERVISE_TIMEOUT=1 \
      REPAIR_TIMEOUT=1 RESUME_WAIT_LOOPS=0 SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
      REAPER_OVERRIDE="$REAPER" LLM_WATCHDOG_LOCK="$TMP/urgent-malformed-$mode.lock" \
      NOTIFY_SEND= "$WATCHDOG" >"$plan/result" 2>&1
    rc=$?
    set -e
    quarantined=0
    while IFS= read -r candidate; do
      if [ "$candidate" != "$throttle" ] \
          && [ "$(cat "$candidate" 2>/dev/null || true)" = "$payload" ]; then
        quarantined=1
        break
      fi
    done < <(find "$plan/.state" -type f -print)
    if [ "$rc" -eq 75 ] || [ ! -e "$TMP/urgent-malformed-$mode-model-called" ] \
        || [ -e "$throttle" ] || [ "$quarantined" -ne 1 ]; then
      echo "urgent repair did not quarantine malformed watchdog $mode state before proceeding (rc=$rc)" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_completion_decision_rejects_post_helper_queue_and_claim() {
  local plan="$TMP/completion-post-helper" writer controller rc
  make_repo "$plan"
  for name in nudge.sh nudge-lock.py nudge-free-items.py nudge-state.py nudge-wait.py \
      nudge-reserve.sh nudge-reap-claims.sh nudge-claim.sh network-watchdog.sh \
      validate-required-gates.py; do
    cp "$ROOT/tools/$name" "$plan/tools/$name"
  done
  chmod +x "$plan/tools/"*
  install_completion_contract "$plan"
  mv "$plan/tools/nudge-lock.py" "$plan/tools/nudge-lock-real.py"
  cat > "$plan/tools/nudge-lock.py" <<PY
#!/usr/bin/python3
import fcntl, os, pathlib, subprocess, sys, time
path=sys.argv[2]; mode=sys.argv[3]; command=sys.argv[4:]
fd=os.open(path, os.O_RDWR|os.O_CREAT|os.O_NOFOLLOW, 0o600)
fcntl.flock(fd, fcntl.LOCK_EX | (fcntl.LOCK_NB if mode=='nonblocking' else 0))
rc=subprocess.run(command).returncode
if path.endswith('/.queue.lock') and 'complete-from-head' in command:
    fcntl.flock(fd, fcntl.LOCK_UN)
    pathlib.Path('$TMP/completion-helper-published').touch()
    for _ in range(400):
        if pathlib.Path('$TMP/completion-post-helper-inserted').exists(): break
        time.sleep(.01)
os.close(fd)
raise SystemExit(rc)
PY
  chmod +x "$plan/tools/nudge-lock.py"
  git -C "$plan" add tools/nudge-lock.py tools/nudge-lock-real.py
  git -C "$plan" commit -qm completion-post-helper-barrier
  cat > "$TMP/completion-raced-queue" <<'EOF'
# NEXT

## Now
1. [READY] [id=raced-work] [gate=raced-gate] inserted after helper publication

## Backlog
EOF
  (
    while [ ! -e "$TMP/completion-helper-published" ]; do sleep 0.01; done
    "$ROOT/tools/nudge-lock.py" lock-run "$plan/.state/.queue.lock" blocking \
      /bin/bash -c 'cp "$1" "$2"; printf "post-helper claim\n" > "$3"' bash \
      "$TMP/completion-raced-queue" "$plan/.state/NEXT.md" "$plan/.state/claims/1-race.claim"
    : > "$TMP/completion-post-helper-inserted"
  ) & writer=$!
  echo "$writer" > "$TMP/completion-post-helper-writer.pid"
  BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-post-helper.lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" \
    NOTIFY_SEND= "$plan/tools/nudge.sh" >/dev/null 2>&1 & controller=$!
  echo "$controller" > "$TMP/completion-post-helper-controller.pid"
  set +e
  wait "$controller"; rc=$?
  set -e
  wait "$writer"
  grep -q '\[id=raced-work\]' "$plan/.state/NEXT.md"
  if [ "$rc" -eq 0 ] || [ -e "$plan/.state/PLAN-COMPLETE" ] \
      || grep -q 'all required P0-P7 gates passed' "$plan/.state/nudge.log"; then
    echo 'controller accepted completion after queue/claim generation changed beyond helper publication' >&2
    return 1
  fi
}

case_completion_rejects_forged_post_validation_artifact() {
  local mode plan rc failures=0 corpus_digest
  for mode in queue claim corpus artifact; do
    plan="$TMP/completion-forged-$mode"
    make_repo "$plan"
    for name in nudge.sh nudge-lock.py nudge-free-items.py nudge-state.py nudge-wait.py \
        nudge-reserve.sh nudge-reap-claims.sh nudge-claim.sh network-watchdog.sh \
        validate-required-gates.py; do
      cp "$ROOT/tools/$name" "$plan/tools/$name"
    done
    chmod +x "$plan/tools/"*
    install_completion_contract "$plan"
    mkdir -p "$plan/game-data"
    printf 'game-data/\n' > "$plan/.gitignore"
    printf 'verified external corpus\n' > "$plan/game-data/completion.bin"
    corpus_digest=$(sha256sum "$plan/game-data/completion.bin" | awk '{print $1}')
    printf '%s  game-data/completion.bin\n' "$corpus_digest" >> "$plan/MANIFEST.sha256"
    git -C "$plan" add .gitignore MANIFEST.sha256
    git -C "$plan" commit -qm completion-external-corpus
    mv "$plan/tools/nudge-lock.py" "$plan/tools/nudge-lock-real.py"
    cat > "$plan/tools/nudge-lock.py" <<PY
#!/usr/bin/python3
import fcntl, json, os, pathlib, subprocess, sys

path = sys.argv[2]
lock_mode = sys.argv[3]
command = sys.argv[4:]
fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
fcntl.flock(fd, fcntl.LOCK_EX | (fcntl.LOCK_NB if lock_mode == "nonblocking" else 0))
rc = subprocess.run(command).returncode
if rc == 0 and path.endswith("/.queue.lock") and "complete-from-head" in command:
    root = pathlib.Path("$plan")
    state = root / ".state"
    attack = "$mode"
    if attack == "queue":
        (state / "NEXT.md").write_text(
            "# NEXT\\n\\n## Now\\n1. [READY] [id=forged-work] [gate=forged-gate] arrived after validation\\n\\n## Backlog\\n"
        )
    elif attack == "claim":
        claim = state / "claims/1-forged.claim"
        claim.write_text("forged post-validation claim\\n")
        claim.chmod(0o600)
    elif attack == "corpus":
        (root / "game-data/completion.bin").write_text("changed after validation\\n")

    artifact = state / "PLAN-COMPLETE"
    value = json.loads(artifact.read_text())
    value["decision_basis"]["queue"] = json.loads(subprocess.check_output(
        ["$STATE_HELPER", "queue-snapshot", str(state / "NEXT.md")], text=True
    ))
    claims = state / "claims"
    claims_fd = os.open(claims, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        info = os.fstat(claims_fd)
        names = sorted(name for name in os.listdir(claims_fd) if name.endswith(".claim"))
        entries = []
        for name in names:
            entry = os.stat(name, dir_fd=claims_fd, follow_symlinks=False)
            entries.append([name, entry.st_dev, entry.st_ino])
        value["decision_basis"]["claims"] = {
            "device": info.st_dev, "inode": info.st_ino,
            "mtime_ns": info.st_mtime_ns, "ctime_ns": info.st_ctime_ns,
            "entries": entries,
        }
    finally:
        os.close(claims_fd)
    forged = state / ".PLAN-COMPLETE.forged"
    forged.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\\n")
    forged.chmod(0o600)
    os.replace(forged, artifact)
    pathlib.Path("$TMP/completion-forged-$mode-fired").touch()
os.close(fd)
raise SystemExit(rc)
PY
    chmod +x "$plan/tools/nudge-lock.py"
    git -C "$plan" add tools/nudge-lock.py tools/nudge-lock-real.py
    git -C "$plan" commit -qm completion-forged-artifact-barrier
    set +e
    BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-forged-$mode.lock" \
      SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
      NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" \
      REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" NOTIFY_SEND= \
      "$plan/tools/nudge.sh" >"$plan/result" 2>&1
    rc=$?
    set -e
    if [ ! -e "$TMP/completion-forged-$mode-fired" ]; then
      echo "post-validation $mode artifact replacement fixture did not fire" >&2
      failures=$((failures + 1))
    elif [ "$rc" -eq 0 ] \
        || grep -q 'all required P0-P7 gates passed' "$plan/.state/nudge.log"; then
      echo "controller accepted forged completion after post-validation $mode change" >&2
      failures=$((failures + 1))
    fi
  done
  [ "$failures" -eq 0 ]
}

case_completion_acceptance_freshly_resnapshots_late_claims() {
  local plan="$TMP/completion-late-valid-claim" hook="$TMP/completion-late-claim-hook"
  local queue_snapshot queue_device queue_inode queue_sha256 publisher rc
  make_repo "$plan"
  for name in nudge.sh nudge-lock.py nudge-free-items.py nudge-state.py nudge-wait.py \
      nudge-reserve.sh nudge-reap-claims.sh nudge-claim.sh network-watchdog.sh \
      validate-required-gates.py; do
    cp "$ROOT/tools/$name" "$plan/tools/$name"
  done
  chmod +x "$plan/tools/"*
  install_completion_contract "$plan"
  queue_snapshot=$("$plan/tools/nudge-state.py" queue-snapshot "$plan/.state/NEXT.md")
  read -r queue_device queue_inode queue_sha256 < <(python3 - "$queue_snapshot" <<'PY'
import json, sys
snapshot = json.loads(sys.argv[1])
print(snapshot["device"], snapshot["inode"], snapshot["sha256"])
PY
  )

  # accept-completion snapshots claims before verifying Git and the corpus.
  # Publish a queue-bound lock-v2 claim at the final corpus read, immediately
  # before its acceptance decision, so a stale claims snapshot cannot pass.
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os, pathlib, sys, time

if "accept-completion" in sys.argv:
    original_open = os.open
    target = "$plan/manifest-anchor.txt"
    marker = pathlib.Path("$TMP/completion-late-claim-window")
    done = pathlib.Path("$TMP/completion-late-claim-published")

    def raced_open(path, flags, *args, **kwargs):
        if os.fspath(path) == target and not marker.exists():
            marker.touch()
            for _ in range(400):
                if done.exists():
                    break
                time.sleep(0.01)
            if not done.exists():
                raise RuntimeError("late valid claim publisher did not finish")
        return original_open(path, flags, *args, **kwargs)

    os.open = raced_open
PY
  (
    while [ ! -e "$TMP/completion-late-claim-window" ]; do sleep 0.01; done
    if "$plan/tools/nudge-state.py" publish-claim "$plan/.state/claims" \
        1-late-valid.claim 1 stable-one gate-one late-valid "$(date -Is)" \
        bedlam-nudge-item1-late-valid $$ "$(printf a%.0s {1..64})" \
        "$queue_device" "$queue_inode" "$queue_sha256" \
        >"$plan/late-claim.out" 2>&1; then
      : > "$TMP/completion-late-valid-claim-created"
    fi
    : > "$TMP/completion-late-claim-published"
  ) & publisher=$!
  echo "$publisher" > "$TMP/completion-late-claim-publisher.pid"

  set +e
  PYTHONPATH="$hook" BEDLAM_PLAN_DIR="$plan" NUDGE_LOCK="$TMP/completion-late-claim.lock" \
    SYSTEMD_RUN_OVERRIDE="$TMP/record-run" SYSTEMCTL_OVERRIDE="$TMP/systemctl" \
    NETWORK_WATCHDOG_OVERRIDE="$TMP/network-ok" \
    REAPER_OVERRIDE="$plan/tools/nudge-reap-claims.sh" NOTIFY_SEND= \
    "$plan/tools/nudge.sh" >"$plan/result" 2>&1
  rc=$?
  wait "$publisher"
  set -e
  if [ ! -e "$TMP/completion-late-claim-window" ] \
      || [ ! -e "$TMP/completion-late-valid-claim-created" ] \
      || ! grep -qx 'lock-v2' "$plan/.state/claims/1-late-valid.claim"; then
    echo 'late valid claim fixture did not reach the final corpus verification window' >&2
    return 1
  fi
  if [ "$rc" -eq 0 ] || [ -e "$plan/.state/PLAN-COMPLETE" ] \
      || grep -q 'all required P0-P7 gates passed' "$plan/.state/nudge.log"; then
    echo 'completion acceptance reused a stale claims snapshot after final corpus verification' >&2
    return 1
  fi
}

case_wait_publication_rejects_final_window_queue_rename() {
  local plan="$TMP/wait-final-rename" hook="$TMP/wait-final-rename-hook" digest deadline rc
  make_repo "$plan"
  deadline=$(date -u -d '+1 minute' '+%Y-%m-%dT%H:%M:%SZ')
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=rename-race] [retry=100ms] [deadline=$deadline] final replace fixture

## Backlog
EOF
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/rename-race.sh"
  chmod +x "$plan/tools/rename-race.sh"
  digest=$(sha256sum "$plan/tools/rename-race.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "rename-race"
path = "tools/rename-race.sh"
sha256 = "$digest"
EOF
  git -C "$plan" add .state/NEXT.md tools/rename-race.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm wait-final-rename
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
queue = "$plan/.state/NEXT.md"
marker = "$TMP/wait-final-rename-fired"
original_replace = os.replace
def raced_replace(source, destination, *args, **kwargs):
    if os.fspath(destination) == queue and not os.path.exists(marker):
        os.rename(queue, queue + ".validated")
        with open(queue, "w") as handle: handle.write("attacker queue generation\n")
        open(marker, "w").close()
    return original_replace(source, destination, *args, **kwargs)
os.replace = raced_replace
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" \
    "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  rc=$?
  set -e
  [ -e "$TMP/wait-final-rename-fired" ]
  if [ "$rc" -eq 0 ] || ! grep -q '^attacker queue generation$' "$plan/.state/NEXT.md"; then
    echo 'wait publication overwrote a queue generation renamed into the final pre-replace window' >&2
    return 1
  fi
}

case_wait_publication_preserves_concurrent_destination_creation() {
  local plan="$TMP/wait-no-replace" hook="$TMP/wait-no-replace-hook" digest deadline rc
  make_repo "$plan"
  deadline=$(date -u -d '+1 minute' '+%Y-%m-%dT%H:%M:%SZ')
  cat > "$plan/.state/NEXT.md" <<EOF
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=no-replace] [retry=100ms] [deadline=$deadline] no-replace fixture

## Backlog
EOF
  printf '#!/usr/bin/env bash\nexit 0\n' > "$plan/tools/no-replace.sh"
  chmod +x "$plan/tools/no-replace.sh"
  digest=$(sha256sum "$plan/tools/no-replace.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "no-replace"
path = "tools/no-replace.sh"
sha256 = "$digest"
EOF
  git -C "$plan" add .state/NEXT.md tools/no-replace.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm wait-no-replace
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import os
marker = "$TMP/wait-no-replace-fired"
concurrent = b"# NEXT\\n\\n## Now\\n1. [READY] [id=concurrent-work] [gate=concurrent-gate] concurrent writer won\\n\\n## Backlog\\n"
original_rename = os.rename
def raced_rename(source, destination, *args, **kwargs):
    result = original_rename(source, destination, *args, **kwargs)
    if str(source) == "NEXT.md" and ".NEXT.md.wait-old-" in str(destination) and not os.path.exists(marker):
        directory_fd = kwargs.get("src_dir_fd")
        fd = os.open("NEXT.md", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=directory_fd)
        os.write(fd, concurrent); os.fsync(fd); os.close(fd)
        open(marker, "w").close()
    return result
os.rename = raced_rename
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" \
    "$plan/.state/automatic-waits" >"$plan/result" 2>&1
  rc=$?
  set -e
  [ -e "$TMP/wait-no-replace-fired" ]
  if [ "$rc" -eq 0 ] || ! grep -q '\[id=concurrent-work\]' "$plan/.state/NEXT.md"; then
    echo 'wait publisher replaced a concurrently created NEXT destination instead of failing no-replace' >&2
    return 1
  fi
}

case_no_direct_authoritative_log_redirections_or_rotation() {
  local matches
  matches=$(grep -nE '2>>[[:space:]]*"\$STATE/nudge\.log"|tail -c [^;]+>[[:space:]]*"\$f\.t"|mv "\$f\.t" "\$f"' \
    "$ROOT/tools/nudge.sh" "$ROOT/tools/nudge-agent.sh" "$ROOT/tools/llm-watchdog.sh" \
    "$ROOT/tools/network-watchdog.sh" || true)
  if [ -n "$matches" ]; then
    echo 'authoritative log writes still bypass pinned state helper APIs:' >&2
    printf '%s\n' "$matches" >&2
    return 1
  fi
}

case_authoritative_state_writes_refuse_symlink_parents() {
  local plan sentinel outside failures=0 session=state-parent
  plan="$TMP/network-state"; make_repo "$plan"
  sentinel="$TMP/network-sentinel"; printf 'DO-NOT-CHANGE\n' > "$sentinel"
  ln -s "$sentinel" "$plan/.state/network-watchdog.log"
  BEDLAM_PLAN_DIR="$plan" CURL_BIN=/bin/false NETWORK_WATCHDOG_LOCK="$TMP/network-state.lock" \
    "$ROOT/tools/network-watchdog.sh" >/dev/null 2>&1 || true
  [ "$(cat "$sentinel")" = DO-NOT-CHANGE ] || {
    echo 'network watchdog followed an authoritative state symlink' >&2
    failures=$((failures + 1))
  }

  plan="$TMP/controller-log-state"; make_repo "$plan"
  sed -i 's/\[READY\]/[BLOCKED]/' "$plan/.state/NEXT.md"
  sentinel="$TMP/controller-log-sentinel"; printf 'DO-NOT-CHANGE\n' > "$sentinel"
  ln -s "$sentinel" "$plan/.state/nudge.log"
  run_controller "$plan" "$TMP/controller-log.lock" >/dev/null 2>&1 || true
  [ "$(cat "$sentinel")" = DO-NOT-CHANGE ] || {
    echo 'controller stderr redirection followed authoritative log symlink' >&2
    failures=$((failures + 1))
  }

  plan="$TMP/agent-parent-state"; make_repo "$plan"
  publish_claim "$plan" "$session"
  outside="$TMP/outside-taskfails"; mkdir "$outside"
  ln -s "$outside" "$plan/.state/taskfails"
  BEDLAM_PLAN_DIR="$plan" OPENC_OVERRIDE=/bin/false NUDGE_IDLE_POLL=.01 \
    "$AGENT" 1 "$session" >/dev/null 2>&1 || true
  if find "$outside" -mindepth 1 -print -quit | grep -q .; then
    echo 'agent mkdir/write traversed an untrusted taskfails parent' >&2
    failures=$((failures + 1))
  fi
  [ "$failures" -eq 0 ]
}

case_direct_read_resources_are_bounded() {
  local plan="$TMP/read-bounds" state rc digest hook="$TMP/probe-read-hook" failures=0
  make_repo "$plan"
  publish_claim "$plan" huge "$plan/.state/claims/1-owner.claim"
  head -c 2097152 /dev/zero >> "$plan/.state/claims/1-owner.claim"
  set +e
  "$STATE_HELPER" read-claim "$plan/.state/claims/1-owner.claim" \
    >"$plan/claim.out" 2>"$plan/claim.err"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] || ! grep -Eqi 'size|limit|bound|oversized' "$plan/claim.err"; then
    echo 'direct oversized claim read had no explicit resource bound' >&2
    failures=$((failures + 1))
  fi

  rm -f "$plan/.state/claims/"*.claim
  for n in $(seq 1 257); do printf 'reservation\n' > "$plan/.state/claims/1-many-$n.claim"; done
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 \
    >"$plan/count.out" 2>"$plan/count.err"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] || ! grep -Eqi 'count|limit|too many' "$plan/count.err"; then
    echo 'claim enumeration lacked a pre-read count bound' >&2
    failures=$((failures + 1))
  fi

  rm -f "$plan/.state/claims/"*.claim
  cat > "$plan/tools/probe.sh" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
  chmod +x "$plan/tools/probe.sh"
  digest=$(sha256sum "$plan/tools/probe.sh" | awk '{print $1}')
  cat > "$plan/docs/automatic-probes.toml" <<EOF
schema = "automatic-probes-v1"
[[probe]]
id = "bounded"
path = "tools/probe.sh"
sha256 = "$digest"
EOF
  cat > "$plan/.state/NEXT.md" <<'EOF'
# NEXT

## Now
1. [WAITING-AUTOMATIC] [id=stable-one] [gate=gate-one] [probe=bounded] [retry=10s] [timeout=30s] bounded read fixture

## Backlog
EOF
  git -C "$plan" add .state/NEXT.md tools/probe.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm bounded-probe
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" >/dev/null
  state="$plan/.state/automatic-waits/stable-one.json"
  head -c 2097152 /dev/zero | tr '\0' ' ' >> "$state"
  set +e
  "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" "$plan/.state/automatic-waits" \
    >"$plan/wait.out" 2>&1
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] || ! grep -Eqi 'size|limit|bound|oversized' "$plan/wait.out"; then
    echo 'oversized wait cache was read without a practical bound' >&2
    failures=$((failures + 1))
  fi

  rm -rf "$plan/.state/automatic-waits"
  mkdir -p "$hook"
  cat > "$hook/sitecustomize.py" <<PY
import pathlib
original=pathlib.Path.read_bytes
count=0
def bounded_read(self):
    global count
    if str(self) == "$plan/tools/probe.sh":
        count += 1
        if count > 1:
            raise OSError('eager probe fallback read')
    return original(self)
pathlib.Path.read_bytes=bounded_read
PY
  set +e
  PYTHONPATH="$hook" "$WAIT_EXECUTOR" run "$plan/.state/NEXT.md" \
    "$plan/.state/automatic-waits" >"$plan/eager.out" 2>&1
  rc=$?
  set -e
  if [ "$rc" -ne 0 ]; then
    echo 'allowlisted probe digest still triggered eager fallback file read' >&2
    failures=$((failures + 1))
  fi

  python3 - "$plan/tools/huge-probe.sh" <<'PY'
import pathlib, sys
p=pathlib.Path(sys.argv[1]); p.write_bytes(b'#!/bin/sh\nexit 1\n#' + b'x' * (2*1024*1024))
PY
  chmod +x "$plan/tools/huge-probe.sh"
  digest=$(sha256sum "$plan/tools/huge-probe.sh" | awk '{print $1}')
  sed -i "s#path = \"tools/probe.sh\"#path = \"tools/huge-probe.sh\"#; s/sha256 = \"[0-9a-f]*\"/sha256 = \"$digest\"/" \
    "$plan/docs/automatic-probes.toml"
  git -C "$plan" add tools/huge-probe.sh docs/automatic-probes.toml
  git -C "$plan" commit -qm oversized-probe
  set +e
  "$PARSER" "$plan/.state/NEXT.md" "$plan/.state/claims" --state-v1 \
    >"$plan/huge.out" 2>"$plan/huge.err"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ] || ! grep -Eqi 'size|limit|bound|oversized' "$plan/huge.err"; then
    echo 'oversized probe executable had no direct-read size bound' >&2
    failures=$((failures + 1))
  fi
  [ "$failures" -eq 0 ]
}

case_runtime_queue_language_is_explicitly_retired() {
  python3 - "$ROOT/docs/RUNTIME.md" <<'PY'
import re, sys
text=open(sys.argv[1], encoding='utf-8').read()
sections=re.split(r'(?m)^(?=## )', text)
stale=re.compile(r"queue item|queue's|follow-ups queued", re.I)
bad=[]
for section in sections:
    lines=[line for line in section.splitlines() if stale.search(line)]
    if not lines:
        continue
    heading=section.splitlines()[0] if section.splitlines() else ''
    semantic=section.casefold()
    if not re.search(r'\b(superseded|retired)\b', semantic):
        bad.append((heading, lines))
assert not bad, 'stale RUNTIME queue language lacks explicit superseded/retired semantics: ' + repr(bad)
PY
}

if [ -n "${REVIEWER_SECURITY_CASE:-}" ]; then
  make_controller_mocks
  set -e
  "$REVIEWER_SECURITY_CASE"
  exit $?
fi

make_controller_mocks
failures=0
run_case() {
  local name=$1 function=$2 limit=${3:-15} rc
  set +e
  timeout --foreground -k 2s "${limit}s" env REVIEWER_SECURITY_CASE="$function" "$0"
  rc=$?
  set -e
  if [ "$rc" -eq 0 ]; then
    printf 'ok - %s\n' "$name"
  else
    if [ "$rc" -eq 124 ] || [ "$rc" -eq 137 ]; then
      printf 'not ok - %s (bounded case timeout after %ss)\n' "$name" "$limit" >&2
    else
      printf 'not ok - %s (rc=%s)\n' "$name" "$rc" >&2
    fi
    failures=$((failures + 1))
  fi
}

run_case 'claims use one pinned no-follow handle and cannot be forged or future-pinned' case_claim_lifecycle_uses_pinned_handles 20
run_case 'empty queue validates before connectivity heartbeat spawn-cap and concurrency exits' case_empty_queue_precedes_early_exits 25
run_case 'completion materializes the real external corpus read-only in isolation' case_external_corpus_completion_is_isolated_readonly 15
run_case 'probe registry missing empty or invalid fails closed' case_probe_allowlist_missing_empty_invalid_fails_closed 12
run_case 'detached checkout disables hooks and rejects planted gate inputs' case_checkout_disables_hooks_and_rejects_plants 15
run_case 'validator runs with no network and read-only HEAD tree and corpus' case_validator_uses_network_and_readonly_sandbox 15
run_case 'completion binds queue generation through decision and publication' case_completion_generation_survives_lock_handoff_race 20
run_case 'queue writer API shares lock and verifies post-publish digest' case_queue_writer_api_locks_and_postchecks_digest 12
run_case 'setsid probe descendants are contained or execution fails closed' case_probe_setsess_descendant_is_contained_or_refused 10
run_case 'watchdog requires the trailer on the exact acknowledged remediation commit' case_watchdog_rejects_split_trailer_and_queue_commits 15
run_case 'controller agent and network state writes refuse symlink parents' case_authoritative_state_writes_refuse_symlink_parents 18
run_case 'queue probe wait and claim reads and claim counts are bounded' case_direct_read_resources_are_bounded 25
run_case 'stale RUNTIME queue language is explicitly superseded or retired' case_runtime_queue_language_is_explicitly_retired 8
run_case 'reaper quarantine restores a replacement moved after identity validation' case_reaper_restores_replacement_moved_after_identity_check 12
run_case 'wait-state create replace and unlink stay on the pinned parent fd' case_wait_state_parent_swap_stays_on_pinned_directory 12
run_case 'HEAD materialization never invokes mutable source upload-pack config' case_head_materialization_never_invokes_source_upload_pack 15
run_case 'isolated real Cargo gate writes only through a precreated target mount' case_isolated_real_cargo_gate_has_writable_mounted_target_only 45
run_case 'invalid queue urgent repair bypasses interval and repair cooldown' case_invalid_queue_bypasses_watchdog_interval_and_cooldown 15
run_case 'urgent repair quarantines malformed watchdog verdict and cooldown state' case_urgent_repair_quarantines_malformed_throttle_state 25
run_case 'completion decision rejects queue and claims inserted after helper publication' case_completion_decision_rejects_post_helper_queue_and_claim 20
run_case 'completion rejects replaced proof matching post-validation queue claim or corpus state' case_completion_rejects_forged_post_validation_artifact 45
run_case 'completion acceptance freshly resnapshots claims after final corpus verification' case_completion_acceptance_freshly_resnapshots_late_claims 20
run_case 'wait publication detects queue rename in the final pre-replace window' case_wait_publication_rejects_final_window_queue_rename 12
run_case 'wait publication preserves a concurrently created queue destination' case_wait_publication_preserves_concurrent_destination_creation 12
run_case 'authoritative logs have no direct append or pathname rotation redirections' case_no_direct_authoritative_log_redirections_or_rotation 8

if [ "$failures" -ne 0 ]; then
  printf 'reviewer/security behavioral tests: RED (%d category failures)\n' "$failures" >&2
  exit 1
fi
echo 'reviewer/security behavioral tests: PASS'
