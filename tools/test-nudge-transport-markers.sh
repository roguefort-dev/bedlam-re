#!/usr/bin/env bash
# Regression tests for the nudge-agent provider-side transport and
# rate-limit classifiers (watchdog repair 2026-08-28, token 1851346;
# rate-limit half extended by watchdog repair 2026-08-28, token
# 2010676). The wrapper classifies a run as kind=transport or
# kind=rate-limit by grepping the agent log for provider error markers.
# On 2026-08-28 the bare `DNS` dictionary word in the transport pattern
# matched the phrase "reverse DNS" -- legitimate engineering prose recorded
# in docs/P7-PORTS.md (the Flatpak app-id rationale) and echoed by worker
# transcripts -- so three fully-green rc=0 progress=1 completions were
# misclassified as provider-side transport deaths, each publishing a
# structured failure and pausing the loop for a needless watchdog repair.
# The same night the bare `rate limit` dictionary phrase in the rate-limit
# pattern matched the D230 decision prose "the rate-limit grep (`rate
# limit`, case-insensitive substring)" quoted by worker 78919433's
# transcript, misclassifying its fully-green p7-phase-close completion
# (rc=0 progress=1, commits pushed, queue emptied) as provider quota.
# These tests pin BOTH directions for BOTH patterns against the patterns
# the wrapper actually uses, extracted live from tools/nudge-agent.sh
# (never a copy): prose never matches, real provider errors always do.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
AGENT="$ROOT/tools/nudge-agent.sh"
failures=0

# The classifier pattern must exist exactly once and be extracted verbatim.
pattern=$(sed -n 's/^elif grep -aqE "\(.*\)" "\$LOG"; then$/\1/p' "$AGENT")
pattern_count=$(sed -n 's/^elif grep -aqE "\(.*\)" "\$LOG"; then$/\1/p' "$AGENT" | wc -l)
if [ "$pattern_count" -ne 1 ] || [ -z "$pattern" ]; then
  echo "FAIL: expected exactly one transport grep line in nudge-agent.sh, found $pattern_count"
  exit 1
fi

must_not_match() {
  local label=$1 fixture=$2
  if printf '%s\n' "$fixture" | grep -aqE "$pattern"; then
    echo "FAIL (false positive): $label matched the transport classifier"
    failures=$((failures + 1))
  else
    echo "ok (prose ignored): $label"
  fi
}

must_match() {
  local label=$1 fixture=$2
  if printf '%s\n' "$fixture" | grep -aqE "$pattern"; then
    echo "ok (error caught): $label"
  else
    echo "FAIL (missed error): $label did NOT match the transport classifier"
    failures=$((failures + 1))
  fi
}

# --- Prose that MUST stay unclassified (the 2026-08-28 false positives,
# verbatim shapes from the affected worker transcripts and the docs they
# quoted). ---
must_not_match "flatpak app-id rationale" \
  "app-id dev.roguefort.bedlam (the repo remote's own reverse DNS, checker-joined to the file stems)"
must_not_match "registry note prose" \
  "non-DNS app-id, swapped runtime, unpinned version, non-engine file set are all fail-closed"
must_not_match "manifest comment prose" \
  "# own reverse DNS app-id; org.freedesktop.Platform + Sdk at the pinned runtime-version 24.08"
must_not_match "hyphenated prose" "reverse-DNS shaped + joined to the stems"
must_not_match "bare topic word" "The DNS naming convention for app-ids is documented upstream."
must_not_match "survey echo" \
  "walking every PLAN section 6 P7 sentence against the registry, incl. the reverse DNS app-id row"

# --- Real provider-side failures that MUST stay classified. ---
must_match "stream transport death" "Error: Transport"
must_match "prefixed transport death" "provider stream died: Error: Transport closed"
must_match "socket reset" "events.js:183 throw er; // unhandled 'error' event Error: read ECONNRESET"
must_match "closed socket" "TypeError: fetch failed: socket connection was closed"
must_match "name resolution failure" "Error: getaddrinfo ENOTFOUND api.z.ai"
must_match "temporary name resolution failure" "Error: getaddrinfo EAI_AGAIN api.z.ai"
must_match "dns resolution error shape" "Error: DNS resolution failed: EAI_AGAIN"
must_match "decode error" "Error: Decode error: invalid type: map"
must_match "invalid stream event" "Invalid zai-coding-plan/openai-compatible-chat stream event"
must_match "provider 5xx" "Provider request failed with HTTP 502"
must_match "provider 504" "Provider request failed with HTTP 504 Gateway Timeout"

# --- The rate-limit classifier (watchdog repair 2026-08-28, token
# 2010676): same discipline, second grep. The pattern is extracted live
# from the -aqiE grep line in tools/nudge-agent.sh. ---
rl_pattern=$(sed -n 's/^elif grep -aqiE "\(.*\)" "\$LOG"; then$/\1/p' "$AGENT")
rl_pattern_count=$(sed -n 's/^elif grep -aqiE "\(.*\)" "\$LOG"; then$/\1/p' "$AGENT" | wc -l)
if [ "$rl_pattern_count" -ne 1 ] || [ -z "$rl_pattern" ]; then
  echo "FAIL: expected exactly one rate-limit grep line in nudge-agent.sh, found $rl_pattern_count"
  exit 1
fi

rl_must_not_match() {
  local label=$1 fixture=$2
  if printf '%s\n' "$fixture" | grep -aqiE "$rl_pattern"; then
    echo "FAIL (false positive): $label matched the rate-limit classifier"
    failures=$((failures + 1))
  else
    echo "ok (prose ignored): $label"
  fi
}

rl_must_match() {
  local label=$1 fixture=$2
  if printf '%s\n' "$fixture" | grep -aqiE "$rl_pattern"; then
    echo "ok (error caught): $label"
  else
    echo "FAIL (missed error): $label did NOT match the rate-limit classifier"
    failures=$((failures + 1))
  fi
}

# --- Prose that MUST stay unclassified (the 2026-08-28 false positive,
# verbatim from worker 78919433's transcript, plus sibling shapes). ---
rl_must_not_match "D230 watch-item prose (the live false positive)" \
  "WATCH ITEM (recorded, deliberately untouched): the rate-limit grep (\`rate limit\`, case-insensitive substring) is the same broad-shape risk"
rl_must_not_match "watch-item second line" \
  "no incident observed, and minimal-repair discipline leaves it for the repair that actually sees it fire falsely."
rl_must_not_match "hyphenated topic word" "the rate-limit classifier's bare DNS marker was a prose false positive"
rl_must_not_match "bare topic phrase in prose" "the rate limit grep is the same broad-shape risk"
rl_must_not_match "bare usage-limit prose" "no usage limit anywhere in the gate, no credential, no runner"
rl_must_not_match "case-varied title prose" "Rate Limit Considerations for the nightly loop"
rl_must_not_match "quota-free survey prose" "quota exhaustion was never observed tonight; the word quota alone must not classify"

# --- Real provider quota failures that MUST stay classified. ---
rl_must_match "observed 2026-08-21 quota death" "Usage limit reached for 5 hour"
rl_must_match "lowercase usage limit error" "Error: usage limit reached; retry after the window resets"
rl_must_match "rate limit reached error" "Error: Rate limit reached"
rl_must_match "http 429 short form" "Provider request failed with HTTP 429"
rl_must_match "http 429 reason phrase" "HTTP/1.1 429 Too Many Requests"

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "all transport-marker tests green"
