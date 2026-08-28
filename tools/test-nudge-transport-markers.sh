#!/usr/bin/env bash
# Regression tests for the nudge-agent provider-side transport classifier
# (watchdog repair 2026-08-28, token 1851346). The wrapper classifies a run
# as kind=transport by grepping the agent log for provider error markers.
# On 2026-08-28 the bare `DNS` dictionary word in that pattern matched the
# phrase "reverse DNS" -- legitimate engineering prose recorded in
# docs/P7-PORTS.md (the Flatpak app-id rationale) and echoed by worker
# transcripts -- so three fully-green rc=0 progress=1 completions were
# misclassified as provider-side transport deaths, each publishing a
# structured failure and pausing the loop for a needless watchdog repair.
# These tests pin BOTH directions against the pattern the wrapper actually
# uses, extracted live from tools/nudge-agent.sh (never a copy): prose never
# matches, real provider errors always do.
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

if [ "$failures" -ne 0 ]; then
  echo "$failures failure(s)"
  exit 1
fi
echo "all transport-marker tests green"
