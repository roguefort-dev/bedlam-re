#!/usr/bin/env bash
# One asynchronous bedlam agent plus adaptive-concurrency result reporting.
set -u
PLAN_DIR=${BEDLAM_PLAN_DIR:-/home/kato/Documents/bedlam-re}
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
STATE="$PLAN_DIR/.state"
CLAIMS="$STATE/claims"
FAILURES=${NUDGE_FAILURE_DIR:-$STATE/automation-failures}
QUEUE_PARSER=${QUEUE_PARSER_OVERRIDE:-$SCRIPT_DIR/nudge-free-items.py}
STATE_HELPER="$SCRIPT_DIR/nudge-state.py"
CONC_FILE="$STATE/concurrency"
CONC_DOWN_TS="$STATE/conc-degraded-at"
NUDGE_LOCK=${NUDGE_LOCK:-/tmp/bedlam-nudge.lock}
CONC_MIN=1
item=${1:?queue item required}
slotid=${2:?slot id required}
if [[ ! "$item" =~ ^[1-9][0-9]*$ ]] || [[ ! "$slotid" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
  exit 64
fi
case "$slotid" in owner|publish.lock|executor.lock|queue.lock|archive|tmp-lock) exit 64 ;; esac
LOG="$STATE/agent-$slotid.log"
if [ -n "${OPENC_OVERRIDE:-}" ]; then
  OPENC=$OPENC_OVERRIDE
elif command -v opencode2 >/dev/null 2>&1; then
  OPENC=$(command -v opencode2)
else
  OPENC=/home/kato/.local/share/fnm/node-versions/v24.19.0/installation/bin/opencode2
fi
# Workers run GLM-5.3 at high reasoning effort (matches llm-watchdog WD_MODEL default).
MODEL=zai-coding-plan/glm-5.3#high
NOTIFY_SEND=${NOTIFY_SEND-notify-send}
unit_name="bedlam-nudge-item${item}-${slotid}"

source "$SCRIPT_DIR/nudge-claim.sh"
log_line() { "$STATE_HELPER" append-text "$STATE/nudge.log" "$(date -Is) $*"$'\n' 2>/dev/null || true; }
logged_call() {
  local output rc
  if output=$("$@" 2>&1); then rc=0; else rc=$?; fi
  [ -z "$output" ] || "$STATE_HELPER" append-text "$STATE/nudge.log" \
    "$(date -Is) $output"$'\n' 2>/dev/null || true
  return "$rc"
}

write_failure() {
  local kind=$1 reason=$2 evidence=$3 queue_unchanged=${4:-true}
  "$STATE_HELPER" publish-failure "$FAILURES" "$item" "${item_id:-unknown}" \
    "${item_gate:-unknown}" "$slotid" "$kind" "$reason" "$evidence" \
    "$queue_unchanged" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "$queue_before_json" "$STATE/NEXT.md"
}

cd "$PLAN_DIR" || exit 1
if [ "${NUDGE_OWNER_FD:-}" != 8 ]; then
  exec "$STATE_HELPER" claim-owner-exec "$CLAIMS" "$item-$slotid.claim" \
    "$item-owner.claim" "$item" "$slotid" "$0" "$@"
fi
start_head=$(git rev-parse HEAD 2>/dev/null || echo none)
queue_before_json=$("$STATE_HELPER" queue-snapshot "$STATE/NEXT.md")
queue_hash_before=$(printf '%s' "$queue_before_json" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("sha256") or "")')
queue_identity_before=$(printf '%s' "$queue_before_json" | python3 -c 'import json,sys; v=json.load(sys.stdin); print("{}:{}".format(v.get("device"),v.get("inode")))')
"$STATE_HELPER" touch "$STATE/heartbeat" || exit 75
placeholder="$CLAIMS/$item-$slotid.claim"
own="$CLAIMS/$item-owner.claim"
reservation_identity=$(stat -c "%d:%i" "$placeholder" 2>/dev/null || echo missing)

unlink_identity() {
  local path=$1 identity=$2 device inode
  case "$identity" in
    *:*)
      device=${identity%%:*}
      inode=${identity#*:}
      "$STATE_HELPER" unlink "$path" "$device" "$inode" 2>/dev/null
      ;;
    *) return 1 ;;
  esac
}

# A pause created after the timer reserved this slot still wins before launch.
if [ -e "$STATE/PAUSE" ]; then
  unlink_identity "$placeholder" "$reservation_identity" || true
  exit 0
fi

claim_identity=${NUDGE_CLAIM_IDENTITY:?pinned owner claim identity required}

drop_owned_claim() {
  exec 8>&-
  local current_identity
  current_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)
  [ "$current_identity" = "$claim_identity" ] && unlink_identity "$own" "$claim_identity"
  unlink_identity "$placeholder" "$reservation_identity" || true
}

reject_preflight() {
  local reason=$1 evidence=$2
  local unchanged=false current_hash
  current_hash=$(sha256sum "$STATE/NEXT.md" 2>/dev/null | awk '{print $1}')
  [ "$current_hash" = "$queue_hash_before" ] && unchanged=true
  logged_call write_failure preflight-mismatch "$reason" "$evidence" "$unchanged" || true
  log_line "agent item $item launch preflight rejected reason=$reason evidence=$evidence session=$slotid"
  drop_owned_claim
  exit 76
}

# Re-check PAUSE after winning the claim: a watchdog pause that appeared while
# we were claiming must still stop this launch before the model runs.
if [ -e "$STATE/PAUSE" ]; then
  exec 8>&-
  current_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)
  [ "$current_identity" = "$claim_identity" ] && unlink_identity "$own" "$claim_identity" || true
  log_line "PAUSE appeared during claim acquisition for item $item; worker standing down"
  exit 0
fi

item_id=""
item_gate=""
claim_valid=0
if claim_read "$own" "$item" "$slotid"; then
  claim_valid=1
  claim_version=$CLAIM_VERSION
  claim_id=$CLAIM_ID
  claim_gate=$CLAIM_GATE
else
  claim_version=""
  claim_id=""
  claim_gate=""
fi

item_fields=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$CLAIMS" --item-v1 "$item" 2>/dev/null)
queue_rc=$?
if [ "$queue_rc" -ne 0 ]; then
  item_id=$claim_id
  item_gate=$claim_gate
  reject_preflight queue-invalid "strict queue parser rc=$queue_rc"
fi
read -r item_status item_id item_gate <<< "$item_fields"
[ "$claim_valid" -eq 1 ] || reject_preflight claim-invalid "claim schema, filename, ordinal, or session mismatch"
[ "$claim_version" = 2 ] || reject_preflight legacy-claim "lock-v1 cannot authorize a new launch"
[ "$item_status" = READY ] || reject_preflight status-mismatch "expected READY, found $item_status"
if [ "$claim_version" = 2 ]; then
  [ "$claim_id" = "$item_id" ] || reject_preflight id-mismatch "claim=$claim_id queue=$item_id"
  [ "$claim_gate" = "$item_gate" ] || reject_preflight gate-mismatch "claim=$claim_gate queue=$item_gate"
fi

boundary_completion_rewrite() {
  # Is the observed queue change the worker's own AGENTS.md step-7 rewrite?
  # Shape: the strict parser still validates the file, and the claimed
  # (id, gate) identity LEFT the active set -- "the claimed item moved to
  # ## Done", including the queue-the-next-tasks shape where a successor
  # takes the same ordinal. Mutations that keep the claimed item active
  # (body edits, status flips, inode swaps, unrelated rewrites) and corrupt
  # rewrites stay immediate boundary violations. An identity rename is
  # indistinguishable from a completion by content alone, so it earns the
  # same bounded window -- and the model still dies with a recorded
  # preflight failure the moment it outlives the window.
  local empty_claims fields status active_id active_gate ordinal=0 sanctioned=1
  empty_claims=$(mktemp -d "$STATE/.boundary-check.XXXXXX") || return 1
  if ! "$QUEUE_PARSER" "$STATE/NEXT.md" "$empty_claims" --state-v1 >/dev/null 2>&1; then
    sanctioned=0
  else
    while :; do
      ordinal=$((ordinal + 1))
      fields=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$empty_claims" --item-v1 "$ordinal" 2>/dev/null) || break
      read -r status active_id active_gate <<< "$fields"
      if [ "$status" != READY ] && [ "$status" != WAITING-AUTOMATIC ]; then
        sanctioned=0
        break
      fi
      if [ "$active_id" = "$item_id" ] && [ "$active_gate" = "$item_gate" ]; then
        sanctioned=0
        break
      fi
      if [ "$ordinal" -ge 32 ]; then
        sanctioned=0
        break
      fi
    done
  fi
  rm -rf "$empty_claims"
  [ "$sanctioned" -eq 1 ]
}

task_hash=$(sed -n "s/^[[:space:]]*$item\.[[:space:]]*//p" "$STATE/NEXT.md" 2>/dev/null | head -n 1 | sha256sum | cut -c1-16)
claim_body_hash=$(sha256sum /proc/self/fd/8 | awk '{print $1}')

launch_boundary_valid() {
  local current_identity current_hash fields status current_id current_gate queue_now queue_hash queue_identity
  [ ! -e "$STATE/PAUSE" ] || return 1
  [ -f "$own" ] && [ ! -L "$own" ] || return 1
  current_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)
  [ "$current_identity" = "$claim_identity" ] || return 1
  current_hash=$(sha256sum /proc/self/fd/8 2>/dev/null | awk '{print $1}')
  [ "$current_hash" = "$claim_body_hash" ] || return 1
  claim_read "$own" "$item" "$slotid" || return 1
  [ "$CLAIM_VERSION" = 2 ] && [ "$CLAIM_ID" = "$item_id" ] \
    && [ "$CLAIM_GATE" = "$item_gate" ] || return 1
  queue_now=$("$STATE_HELPER" queue-snapshot "$STATE/NEXT.md") || return 1
  queue_hash=$(printf '%s' "$queue_now" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("sha256") or "")')
  queue_identity=$(printf '%s' "$queue_now" | python3 -c 'import json,sys; v=json.load(sys.stdin); print("{}:{}".format(v.get("device"),v.get("inode")))')
  [ "$queue_hash" = "$queue_hash_before" ] && [ "$queue_identity" = "$queue_identity_before" ] || return 1
  fields=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$CLAIMS" --item-v1 "$item" 2>/dev/null) || return 1
  read -r status current_id current_gate <<< "$fields"
  [ "$status" = READY ] && [ "$current_id" = "$item_id" ] \
    && [ "$current_gate" = "$item_gate" ]
}

PROMPT="You are an unattended continuation agent for bedlam-re. Read AGENTS.md and follow its engineering and safety rules. Do not spawn or invoke subagents. Perform one bounded unit: queue item $item, stable id $item_id, gate $item_gate. The wrapper acquired it for slot $slotid before launch; this exact versioned claim is yours. Work only this item and never switch items. Inspect and adopt relevant interrupted WIP while preserving unrelated changes; stage explicit paths only. For reverse-engineering or analysis-heavy work, decode a bounded piece and immediately write committed RE notes before continuing. If the task cannot be completed or a tool, transport, or API fails, leave NEXT unchanged and stop; the wrapper records a machine repair artifact. Do not park, retag, or convert required work into a passive state. Every commit must include the exact trailer Nudge-Worker: $slotid. Commit and push substantive completed work. Checkpoint coherent green milestones. Never create, delete, rename, or modify claim files. Never start an analyzeHeadless import already running or already successful."

set +e
# --agent build: the nudge worker needs the coding agent, NOT the
# config default (orchestrator). The 2026-08-20 watchdog repair found
# the default agent carried steps:60 (opencode2 then kills the run
# text-only mid-unit: "Maximum steps for this agent reached", rc=0,
# no commit possible) and denied the edit tool (workers resorted to
# shell+python file surgery, wasting steps). The build agent has no
# step cap and full tools; the outer timeout 3900 stays the bound.
#
# Idle-log reaper (watchdog repair, 2026-08-21): the opencode2 client
# can print "Error: Transport" and then never exit - observed hung in
# do_epoll_wait at zero CPU with its agent log frozen for 30+ minutes
# (worker 82523e41, item 1, task 230a7a38b991ed5f), invisible to the
# controller (a live locked claim) and unaffected by the provider's
# own 300s zero-stream watchdog, burning the whole 65-minute slot
# budget on a single provider hiccup while every controller tick
# logged "concurrency full - standing down". So supervise the client
# ourselves: if the agent log stays silent past NUDGE_IDLE_LIMIT while
# the process lives, terminate it and let the run classify as
# provider-side transport (not charged to the task), same as any
# other stream death. A healthy run emits tool/text output far more
# often than the limit; long silent tool calls (cold cargo builds)
# stay safe at the 900s default.
IDLE_LIMIT=${NUDGE_IDLE_LIMIT:-900}
IDLE_POLL=${NUDGE_IDLE_POLL:-5}
# Bounded window granted when the ONLY observed boundary change is the
# worker's own sanctioned end-of-run queue rewrite (its claimed item left
# the active set, per AGENTS.md step 7). The client needs this long to
# finish streaming its final message and exit by itself; a model still
# running past the window is operating on a stale queue and dies as before
# (watchdog repair 2026-08-26: 878c03f's mid-run poll killed three workers
# at their own finish line -- every completed+pushed unit was recorded as a
# preflight-mismatch failure because the 200ms self-exit grace cannot cover
# a real client shutdown).
# (watchdog repair 2026-08-28, second recurrence, token 1007791: 240s still
# cannot cover the CONTRACT-required post-rewrite work. The gates-validator
# battery must run at the final clean HEAD -- i.e. AFTER the step-7
# bookkeeping commit -- and takes minutes under bwrap; worker 05e14378
# finished p5-select-shell-g1 green (a5c3a71 + 3d64ca5 + a51d4f2), rewrote
# the queue, and was grace-killed mid-validator at exactly 240s, recording
# a false preflight-mismatch and stranding the push. An actively-logging
# completer must never get a shorter leash than a hung silent client: the
# grace now equals IDLE_LIMIT, and reap_idle_model still bounds a truly
# hung client by log silence inside the grace loop.)
BOUNDARY_GRACE=${NUDGE_BOUNDARY_GRACE:-900}
reaped=0
boundary_failure=0
termination_sent=0
if ! launch_boundary_valid; then
  reject_preflight launch-boundary "queue or canonical claim changed immediately before exec"
fi
if [ ! -x /usr/bin/bwrap ]; then
  reject_preflight containment-unavailable "bubblewrap PID containment is required"
fi
setsid bash -c 'kill -STOP "$$"; exec "$@"' bash \
  /usr/bin/bwrap --unshare-pid --die-with-parent --bind / / \
  --dev-bind /dev /dev --proc /proc -- \
  "$STATE_HELPER" exec-output "$LOG" append timeout 3900 "$OPENC" run --standalone --agent build --model "$MODEL" --auto \
  --title "bedlam-nudge-item$item" "$PROMPT" &
agent_pid=$!
terminate_model_group() {
  local signal=$1
  "$STATE_HELPER" signal-descendants "$agent_pid" "$signal" 2>/dev/null || true
  [ "$signal" != TERM ] || sleep 0.05
  kill -"$signal" -- "-$agent_pid" 2>/dev/null || true
}
agent_exited() {
  ! kill -0 "$agent_pid" 2>/dev/null || [[ "$(ps -o stat= -p "$agent_pid" 2>/dev/null)" == *Z* ]]
}
reap_idle_model() {
  local now log_mtime idle _
  now=$(date +%s)
  log_mtime=$(stat -c %Y "$LOG" 2>/dev/null || echo "$now")
  idle=$(( now - log_mtime ))
  [ "$idle" -ge "$IDLE_LIMIT" ] || return 1
  reaped=1
  log_line "idle-log reaper: item $item agent log silent ${idle}s >= ${IDLE_LIMIT}s; terminating hung client pid $agent_pid"
  "$STATE_HELPER" append-text "$LOG" "$(date -Is) idle-log reaper: terminating after ${idle}s with no agent-log output"$'\n' 2>/dev/null || true
  terminate_model_group TERM
  for _ in $(seq 1 10); do
    kill -0 "$agent_pid" 2>/dev/null || break
    sleep 1
  done
  terminate_model_group KILL
  termination_sent=1
  return 0
}
terminate_boundary_violation() {
  local _
  boundary_failure=1
  terminate_model_group TERM
  for _ in $(seq 1 3); do
    kill -0 "$agent_pid" 2>/dev/null || break
    sleep 0.01
  done
  terminate_model_group KILL
  termination_sent=1
}
for _ in $(seq 1 100); do
  [[ "$(ps -o stat= -p "$agent_pid" 2>/dev/null)" == *T* ]] && break
  sleep 0.01
done
if ! launch_boundary_valid; then
  boundary_failure=1
  terminate_model_group TERM
else
  kill -CONT -- "-$agent_pid" 2>/dev/null || true
fi
while kill -0 "$agent_pid" 2>/dev/null; do
  sleep "$IDLE_POLL"
  agent_exited && break
  if [ -e "$STATE/PAUSE" ]; then
    log_line "PAUSE appeared while the model ran for item $item; terminating model pid $agent_pid"
    terminate_boundary_violation
    break
  fi
  if ! launch_boundary_valid; then
    if boundary_completion_rewrite; then
      grace_deadline=$(( $(date +%s) + BOUNDARY_GRACE ))
      log_line "item $item left the active queue (worker completion rewrite); awaiting model exit for up to ${BOUNDARY_GRACE}s"
      while ! agent_exited && [ "$(date +%s)" -lt "$grace_deadline" ]; do
        sleep "$IDLE_POLL"
        reap_idle_model && break
      done
      [ "$termination_sent" -eq 0 ] || break
      agent_exited && break
      log_line "launch boundary changed for item $item and the model kept running ${BOUNDARY_GRACE}s past its own completion rewrite; terminating model pid $agent_pid"
    else
      exited_after_change=0
      for _ in $(seq 1 20); do
        if agent_exited; then
          exited_after_change=1
          break
        fi
        sleep 0.01
      done
      [ "$exited_after_change" -eq 0 ] || break
      # (watchdog repair 2026-08-28, third recurrence, token 4104751:
      # the worker's own step-7 rewrite is a MULTI-EDIT sequence, and its
      # intermediate states -- successor queued while the claimed item is
      # not yet removed, or a torn header edit -- fail
      # boundary_completion_rewrite exactly like a foreign mutation. The
      # 200ms leash killed two completers BETWEEN their own edits today
      # (bd07c7b6 15:35, b3083e9c 17:28), each time stranding a mangled
      # INVALID queue and recording a false preflight-mismatch that cost
      # a full watchdog repair cycle. The D209/D211/D214 finish-line
      # discipline explicitly owes the worker a second turn when its own
      # parser check fails; the wrapper must not close that window
      # first. Same principle as the second recurrence: an actively-
      # working completer never gets a shorter leash than a hung silent
      # client. The unrecognized change now travels the same bounded
      # BOUNDARY_GRACE window -- reap_idle_model bounds silence inside
      # it, maturation into the sanctioned completion shape is honored
      # -- and the kill plus the recorded failure land at the deadline
      # exactly as before.)
      grace_deadline=$(( $(date +%s) + BOUNDARY_GRACE ))
      log_line "launch boundary changed for item $item (not yet a sanctioned completion rewrite); awaiting exit, idle reap, or rewrite maturation for up to ${BOUNDARY_GRACE}s (model pid $agent_pid)"
      while ! agent_exited && [ "$(date +%s)" -lt "$grace_deadline" ]; do
        sleep "$IDLE_POLL"
        reap_idle_model && break
        boundary_completion_rewrite && break
      done
      [ "$termination_sent" -eq 0 ] || break
      agent_exited && break
      if boundary_completion_rewrite; then
        continue
      fi
      log_line "launch boundary changed for item $item; terminating model pid $agent_pid"
    fi
    terminate_boundary_violation
    break
  fi
  reap_idle_model && break
done
if [ "$termination_sent" -eq 0 ]; then
  terminate_model_group TERM
  sleep 0.05
  terminate_model_group KILL
fi
wait "$agent_pid"
rc=$?
[ "$boundary_failure" -eq 1 ] && rc=76
set -e
if [ "$boundary_failure" -eq 1 ]; then
  logged_call write_failure preflight-mismatch launch-boundary \
    "queue or canonical claim changed after model start" false || true
  drop_owned_claim
  exit 76
fi
"$STATE_HELPER" append-file "$STATE/nudge-run.log" "$LOG" 2>/dev/null || true

end_head=$(git rev-parse HEAD 2>/dev/null || echo none)
progress=0
if [ "$end_head" != "$start_head" ] && git cat-file -e "$start_head^{commit}" 2>/dev/null; then
  for commit in $(git rev-list "$start_head..$end_head" 2>/dev/null); do
    if git log -1 --format=%B "$commit" | grep -qx "Nudge-Worker: $slotid" \
        && git diff-tree --no-commit-id --name-only -r "$commit" | grep -qv "^\.state/"; then
      progress=1
      break
    fi
  done
fi

kind=none
post_claims="$STATE/.post-claims-$slotid"
"$STATE_HELPER" ensure-dir "$post_claims" 2>/dev/null || true
set +e
post_queue=$("$QUEUE_PARSER" "$STATE/NEXT.md" "$post_claims" --state-v1 2>/dev/null)
post_queue_rc=$?
rmdir "$post_claims" 2>/dev/null || true
set -e
# -i: the provider prints "Usage limit reached" (capital U); the
# pre-2026-08-21 case-sensitive matcher missed it and every quota death
# fell through to client-error (watchdog repair, 2026-08-21).
if [ "$post_queue_rc" -ne 0 ] || [ "$post_queue" = INVALID-DEADLOCKED ]; then
  kind=queue-invalid
elif grep -aqiE "Rate limit reached|rate limit|usage limit|HTTP[^0-9]*429|429 Too Many Requests" "$LOG"; then
  kind=rate-limit
# Provider HTTP 5xx ("Provider request failed with HTTP 502", the
# 2026-08-21 19:15/19:34 incident) is provider-side overload, not a
# client error: both 502 deaths fell through to client-error and
# charged task 1c8526453c786dd5 to 2/3 - one more would have armed
# the cooldown+notify spiral mid-incident (watchdog repair,
# 2026-08-21).
elif grep -aqE "Decode error|Error:.*Transport|Error: Transport|ECONNRESET|socket connection was closed|getaddrinfo ENOTFOUND|EAI_AGAIN|[Dd][Nn][Ss] resolution|Invalid [A-Za-z0-9_./-]+/openai-compatible-chat stream event|Provider request failed with HTTP 5[0-9][0-9]" "$LOG"; then
  # (watchdog repair 2026-08-28, token 1851346: the bare `DNS` alternative
  # was a PROSE false positive. Three fully-green completions tonight
  # (3ea06ba4 flatpak 20:26, a6aece66 windows-installer 20:52, c60dbcd6
  # macos-universal2 21:14) each exited rc=0 progress=1 with the final
  # summary fully streamed, everything committed AND pushed -- and were
  # still classified provider-side transport because their transcripts
  # legitimately contain the phrase "reverse DNS" (the Flatpak app-id
  # rationale recorded in docs/P7-PORTS.md and echoed by ci.yml work and
  # sibling-gate verification). Each false positive published a structured
  # failure, paused the loop for a watchdog repair cycle, and released a
  # claim for nothing; the upcoming p7-phase-close survey walks P7-PORTS.md
  # sentence by sentence, guaranteeing recurrence. Resolution-failure
  # markers must be error-shaped, never bare dictionary words: `DNS` is
  # replaced by the errno-shape EAI_AGAIN plus "[Dd][Nn][Ss] resolution",
  # neither of which can match "reverse DNS", "reverse-DNS shaped", or
  # "non-DNS app-id" prose. getaddrinfo ENOTFOUND already covered the
  # common node resolution error and stays.)
  kind=transport
elif [ "$reaped" -eq 1 ]; then
  # The idle-log reaper terminated a hung client (2026-08-21 watchdog
  # repair): a provider-side stream death that never exited on its
  # own. Same accounting exemption as transport so a hang never feeds
  # the taskfails/cooldown spiral.
  kind=transport
elif grep -aq "Maximum steps for this agent" "$LOG"; then
  # opencode2 truncated the run at the agent step budget. Distinct
  # from no-progress: the model was still working (rc=0, text-only
  # hand-off, WIP on disk for the next spawn to adopt).
  kind=step-cap
elif [ "$rc" -ne 0 ]; then
  kind=client-error
elif [ "$progress" -eq 0 ] && [ "$post_queue" = AUTOMATIC-WAIT ]; then
  # The model may request a bounded wait, but only the wrapper can initialize
  # and seal its schedule while holding the shared queue/executor locks.
  if "$STATE_HELPER" run-output "$STATE/nudge.log" append \
      "$SCRIPT_DIR/nudge-wait.py" run "$STATE/NEXT.md" "$STATE/automatic-waits" \
      && "$STATE_HELPER" run-output "$STATE/nudge.log" append \
      "$SCRIPT_DIR/nudge-wait.py" verify "$STATE/NEXT.md" "$STATE/automatic-waits"; then
    kind=none
  else
    kind=wait-invalid
  fi
elif [ "$progress" -eq 0 ]; then
  kind=no-progress
fi

if [ "$kind" = queue-invalid ] && [ "$rc" -eq 0 ]; then
  rc=76
fi

# Validate mutable accounting before publishing the ordinary run failure so a
# corrupt counter produces one unambiguous repair artifact for this session.
previous_fail_count=0
if [ "$kind" != none ] && [ "$kind" != step-cap ] && [ "$kind" != transport ] && [ "$kind" != rate-limit ]; then
  "$STATE_HELPER" ensure-dir "$STATE/taskfails" 2>/dev/null || {
    write_failure mutable-state-invalid "task failure directory refused" "mutable fail count state" true 2>/dev/null || true
    drop_owned_claim
    exit 76
  }
  fails_file="$STATE/taskfails/$task_hash"
  if [ -e "$fails_file" ]; then
    previous_fail_count=$("$STATE_HELPER" read-int "$fails_file" task-failure-count 0 1000000 - 2>&1) || {
      logged_call write_failure mutable-state-invalid "fail count invalid: $previous_fail_count" "mutable fail count state" true || true
      drop_owned_claim
      exit 76
    }
  fi
fi

if [ "$kind" != none ] && [ "$kind" != step-cap ]; then
  queue_hash_after=$(sha256sum "$STATE/NEXT.md" 2>/dev/null | awk '{print $1}')
  queue_unchanged=false
  [ "$queue_hash_after" = "$queue_hash_before" ] && queue_unchanged=true
  logged_call write_failure "$kind" "$kind" "client_rc=$rc;progress=$progress;task=$task_hash" \
    "$queue_unchanged" || true
fi

if [ -e "$CONC_FILE" ]; then
  cur=$("$STATE_HELPER" read-int "$CONC_FILE" concurrency-value 0 3 - 2>&1) || {
    logged_call write_failure mutable-state-invalid "concurrency value invalid: $cur" "mutable concurrency state" true || true
    drop_owned_claim
    exit 76
  }
else
  cur=3
fi
clean_log_line=""
result_log_line=""
if [ "$kind" = step-cap ]; then
  # A step-capped run is a budget truncation, not a task failure:
  # counting it as no-progress sent the loop into a fail/cooldown
  # spiral while every attempt made real uncommitted progress
  # (watchdog repair, 2026-08-20; four spawns died at the cap on
  # task 247ce5e255167e9a). Log loudly, keep the claim for the
  # short DEAD_CLAIM_TTL backoff, never punish the task.
  result_log_line="agent item $item hit the opencode2 step cap [rc=$rc progress=$progress] task=$task_hash; treating as truncation, not failure"
elif [ "$kind" = transport ]; then
  # A provider-side stream failure is environmental, not a task failure
  # (watchdog repair, 2026-08-20: "Invalid zai-coding-plan/openai-
  # compatible-chat stream event" killed nine spawns within seconds plus
  # two mid-run sessions between 20:42 and 22:59, charging task
  # 4f6a0d2b eleven fails so 15-min cooldowns persisted for 2.5h after
  # the provider recovered). Log loudly, keep the claim for the
  # DEAD_CLAIM_TTL retry backoff (which throttles spawn churn during a
  # live incident), never punish the task; the llm-watchdog owns
  # provider-incident escalation.
  reap_note=""
  [ "$reaped" -eq 1 ] && reap_note=" (idle-log reaper)"
  result_log_line="agent item $item failed [transport rc=$rc progress=$progress] task=$task_hash; provider-side, not charged to the task$reap_note"
  "$STATE_HELPER" ensure-dir "$STATE/taskfails" 2>/dev/null || true
  "$STATE_HELPER" touch "$STATE/taskfails/.transport-streak" 2>/dev/null || true
  # event beacon: the llm-watchdog path unit watches this dir - a transport
  # storm escalates to the supervisor in seconds, no timer involved.
elif [ "$kind" = rate-limit ]; then
  # Provider quota exhaustion is provider-side, not a task failure
  # (watchdog repair, 2026-08-21: "Usage limit reached for 5 hour" killed
  # every spawn at its first model call between 07:08 and 07:52, four
  # deaths mislabeled client-error charged task c72d408d50275d04 with
  # fails + 15-min cooldowns that rolled over repeatedly for the rest of
  # the quota window). Never touch taskfails; instead hold this task in
  # The structured failure artifact is the scheduler-visible outcome. Hidden
  # per-task cooldown files are deliberately not scheduler truth.
  result_log_line="agent item $item failed [rate-limit rc=$rc progress=$progress] task=$task_hash; provider quota, not charged to the task; structured automatic repair required"
elif [ "$kind" != none ]; then

  # Operator no-cooldown doctrine (2026-08-22): a failed run retries
  # immediately, like a clean one. Age the heartbeat so the controller
  # freshness gate (300s) cannot become a silent per-failure backoff
  # now that claims are released instead of retained. Provider rate limits
  # publish structured failures instead of hidden scheduler holds.
  if [ "$kind" != rate-limit ]; then
    logged_call "$STATE_HELPER" touch "$STATE/heartbeat" 0 || true
  fi
  # Failures are scoped to this task, not to the whole controller, so
  # unrelated items can never be blamed for (or cleared by) this run.
  "$STATE_HELPER" ensure-dir "$STATE/taskfails" 2>/dev/null || {
    write_failure mutable-state-invalid "task failure directory refused" "mutable fail count state" true 2>/dev/null || true
    drop_owned_claim
    exit 76
  }
  fails_file="$STATE/taskfails/$task_hash"
  fail_count=$((previous_fail_count + 1))
  logged_call "$STATE_HELPER" write-text "$fails_file" "$fail_count" || {
    logged_call write_failure mutable-state-invalid "fail count write refused" "mutable fail count state" true || true
    drop_owned_claim
    exit 76
  }
  if [ "$fail_count" -ge 3 ]; then
    if [ "$fail_count" -eq 3 ] && [ -n "$NOTIFY_SEND" ] && command -v "$NOTIFY_SEND" >/dev/null 2>&1; then
      "$NOTIFY_SEND" -u critical "bedlam-re repeated agent failures" "item $item failed three consecutive observed runs ($kind, task $task_hash); cooling down 15 minutes" 2>/dev/null || true
    fi
  fi
  if [ "$cur" -gt "$CONC_MIN" ]; then
    logged_call "$STATE_HELPER" write-text "$CONC_FILE" "$((cur-1))" || true
    logged_call "$STATE_HELPER" write-text "$CONC_DOWN_TS" "$(date +%s)" || true
    result_log_line="agent item $item failed [$kind rc=$rc progress=$progress] task=$task_hash; concurrency degraded $cur -> $((cur-1))"
  else
    result_log_line="agent item $item failed [$kind rc=$rc progress=$progress] task=$task_hash; concurrency remains 1"
  fi
else
  "$STATE_HELPER" unlink "$STATE/taskfails/$task_hash" 2>/dev/null || true
  clean_log_line="$(date -Is) agent item $item ended cleanly (rc=$rc progress=$progress) task=$task_hash"
fi

# Drop only this wrapper lock, then inspect the same inode we acquired. A live
# inherited descriptor is a real ghost and keeps the claim. Failed runs retain
# an unlocked claim for DEAD_CLAIM_TTL as intentional retry backoff.
exec 8>&-
unlink_identity "$placeholder" "$reservation_identity" || true
current_identity=$(stat -c "%d:%i" "$own" 2>/dev/null || echo missing)
if [ "$current_identity" = "$claim_identity" ]; then
  if flock -n "$own" true 2>/dev/null; then
    if [ "$kind" = none ] && [ "$rc" -eq 0 ]; then
      unlink_identity "$own" "$claim_identity" || true
    else
      unlink_identity "$own" "$claim_identity" || true
      log_line "failed run released item $item claim for immediate retry (no cooldowns - operator 2026-08-21)"
    fi
  else
    log_line "retaining item $item claim held by a live descendant"
  fi
fi

# Event-driven chaining (2026-08-21): on a clean end, age the heartbeat so
# the controller freshness gate passes immediately (the same trick the
# watchdog resume path uses) and fire one instant nudge pass. The 60s timer
# stays armed as a floor; every pass remains idempotent under the controller
# lock. Test safety: SYSTEMD_RUN_OVERRIDE marks the hermetic controller run
# (never touch the real systemd there); SYSTEMCTL_OVERRIDE lets tests
# record the chained call instead.
if [ "$kind" = none ] && [ "$rc" -eq 0 ]; then
  logged_call "$STATE_HELPER" touch "$STATE/heartbeat" 0 || true
  if [ -n "${SYSTEMCTL_OVERRIDE:-}" ]; then
    "$SYSTEMCTL_OVERRIDE" --user start bedlam-nudge.service >/dev/null 2>&1 || true
  elif [ -z "${SYSTEMD_RUN_OVERRIDE:-}" ]; then
    systemctl --user start bedlam-nudge.service >/dev/null 2>&1 || true
    # A pass can be mid-flight at the same instant (lock busy at 22:46:31
    # on 2026-08-21): re-trigger once a few seconds later so a pass always
    # runs AFTER claim release + heartbeat aging, still far ahead of the timer.
    systemd-run --user --collect --on-active=4s "--unit=bedlam-nudge-chain-$slotid" systemctl --user start bedlam-nudge.service >/dev/null 2>&1 || true
  fi
fi
[ -z "$result_log_line" ] || "$STATE_HELPER" append-text "$STATE/nudge.log" "$(date -Is) $result_log_line"$'\n' 2>/dev/null || true
[ -z "$clean_log_line" ] || "$STATE_HELPER" append-text "$STATE/nudge.log" "$clean_log_line"$'\n' 2>/dev/null || true
exit "$rc"
