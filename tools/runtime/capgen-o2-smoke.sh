#!/bin/sh
# capgen-o2-smoke — the D140 headless proof of the O2 capture chain:
# plan (dbx-plan --channel o2) -> driver feed (capgen-o2 --synthesize-feed)
# -> transcript (capgen-o2 --feed) -> dump (dbx-stitch --channel o2)
# -> decode + differ intake (dbx-diff) -> the loud rejections.
#
# UNATTENDED-SAFE: no Wine, no ptrace, no game launch, no corpus read
# (the build identity is a placeholder literal — a synthetic transcript
# makes no build claim). Outputs under runtime/harness-out/o2-smoke/
# (gitignored). Manifest-bracketed per AGENTS (game-data untouched).
set -e
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT="$REPO_ROOT/runtime/harness-out/o2-smoke"
CAPGEN="$REPO_ROOT/tools/runtime/capgen-o2.py"
PLAN_DIR="$REPO_ROOT/tools/diffharness"
BIN="$REPO_ROOT/target/release"

fail() { echo "capgen-o2-smoke: FAIL: $1" >&2; exit 1; }
note() { echo "== $*" >&2; }

note "manifest check (pre)"
(cd "$REPO_ROOT" && sha256sum -c MANIFEST.sha256 --quiet) \
  || fail "MANIFEST pre-check"

mkdir -p "$OUT"
rm -rf "$OUT"/*

note "build the diffharness bins"
cargo build -q -p diffharness --release --bins \
  || fail "cargo build"

# --- (a) plan side: the o2 compiler still byte-pins the committed plan
note "(a) dbx-plan --channel o2 byte-pins capture-plans/S1-o2.json"
"$BIN/dbx-plan" "$PLAN_DIR/scenarios/S1.scen" --channel o2 \
  --out "$OUT/S1-o2.regen.json" >/dev/null \
  || fail "dbx-plan o2 compile"
cmp -s "$OUT/S1-o2.regen.json" "$PLAN_DIR/capture-plans/S1-o2.json" \
  || fail "regenerated S1-o2.json differs from the committed byte-pin"

# --- (b) the full S1-o2 walk: synthesize -> emit -> stitch -> decode
note "(b) S1-o2: synthesize feed + emit transcript (401 frames)"
python3 "$CAPGEN" --plan "$PLAN_DIR/capture-plans/S1-o2.json" \
  --synthesize-feed "$OUT/s1-o2.dbxfeed" || fail "synthesize"
python3 "$CAPGEN" --plan "$PLAN_DIR/capture-plans/S1-o2.json" \
  --feed "$OUT/s1-o2.dbxfeed" --out "$OUT/s1-o2.dbxcap" || fail "emit"

note "(b) dbx-stitch --channel o2 against the real S1 scenario"
"$BIN/dbx-stitch" "$PLAN_DIR/scenarios/S1.scen" "$OUT/s1-o2.dbxcap" \
  --build-sha256 "$(printf 'ab%.0s' $(seq 32))" --channel o2 \
  --pin "exw=unpinned-synthetic-smoke" --out-dir "$OUT" \
  >"$OUT/s1-o2.manifest.json" || fail "stitch"
grep -q '"channel": "O2:EXW/Wine"' "$OUT/s1-o2.manifest.json" \
  || fail "manifest channel mark"
grep -q '"scenario": "S1"' "$OUT/s1-o2.manifest.json" \
  || fail "manifest scenario"
grep -q '"frame_count": 401' "$OUT/s1-o2.manifest.json" \
  || fail "manifest frame contract (401 = scenario 400 + anchor)"

note "(b) dbx-diff self-cross: decode + normalize_o2_row intake"
"$BIN/dbx-diff" "$OUT/S1.bdld" "$OUT/S1.bdld" --report "$OUT/s1-self.report" \
  || fail "dbx-diff self-cross (decode)"
grep -qi "cross-channel" "$OUT/s1-self.report" || fail "cross-channel mode"
grep -qi "PASS" "$OUT/s1-self.report" || fail "self-cross verdict"

# --- (c) the D139 loud rejection: an EXD-only row refuses on o2
note "(c) static-cursor-clamp (EXD-only) spliced into the transcript MUST refuse"
sed 's/^frame 2$/frame 2\nwatch static-cursor-clamp deadbeef/' \
  "$OUT/s1-o2.dbxcap" > "$OUT/s1-o2.ghost.dbxcap"
if "$BIN/dbx-stitch" "$PLAN_DIR/scenarios/S1.scen" "$OUT/s1-o2.ghost.dbxcap" \
    --build-sha256 "$(printf 'ab%.0s' $(seq 32))" --channel o2 \
    --out-dir "$OUT/ghost" >/dev/null 2>"$OUT/ghost.err"; then
  fail "the EXD-only row stitched on o2 (anti-ghost rule broken)"
fi
grep -q "NoExwAddress\|no EXW address" "$OUT/ghost.err" \
  || fail "wrong rejection: $OUT/ghost.err"

# --- (d) the emitter's own contract: a truncated feed refuses loud
note "(d) truncated feed (hit 401 dropped) MUST refuse at emit"
python3 - "$OUT/s1-o2.dbxfeed" "$OUT/trunc.dbxfeed" <<'EOF' || exit 1
import re, sys
src, dst, keep, drop = sys.argv[1], sys.argv[2], [], False
for ln in open(src).read().splitlines():
    if re.match(r"^hit \d+$", ln):
        drop = (ln == "hit 401")
    if not drop:
        keep.append(ln)
open(dst, "w").write("\n".join(keep) + "\n")
EOF
if python3 "$CAPGEN" --plan "$PLAN_DIR/capture-plans/S1-o2.json" \
    --feed "$OUT/trunc.dbxfeed" --out "$OUT/trunc.dbxcap" \
    2>"$OUT/trunc.err"; then
  fail "a feed missing hit 401 emitted a transcript"
fi
grep -q "no hit 401 block" "$OUT/trunc.err" || fail "wrong truncation error"

# --- (e) the inject grammar end-to-end: S3-o2 (command steps) headless
note "(e) S3-o2: dbx-plan compiles command injects on EXW cells; the chain runs"
"$BIN/dbx-plan" "$PLAN_DIR/scenarios/S3.scen" --channel o2 \
  --out "$OUT/S3-o2.json" >/dev/null || fail "S3-o2 compile"
python3 "$CAPGEN" --plan "$OUT/S3-o2.json" \
  --synthesize-feed "$OUT/s3-o2.dbxfeed" || fail "S3 synthesize"
python3 "$CAPGEN" --plan "$OUT/S3-o2.json" \
  --feed "$OUT/s3-o2.dbxfeed" --out "$OUT/s3-o2.dbxcap" || fail "S3 emit"
grep -q "^frame 1 1$" "$OUT/s3-o2.dbxcap" \
  || fail "S3 frame 1 lacks the injected flag (command inject at boundary 1)"
"$BIN/dbx-stitch" "$PLAN_DIR/scenarios/S3.scen" "$OUT/s3-o2.dbxcap" \
  --build-sha256 "$(printf 'ab%.0s' $(seq 32))" --channel o2 \
  --pin "exw=unpinned-synthetic-smoke" --out-dir "$OUT" \
  >"$OUT/s3-o2.manifest.json" || fail "S3 stitch"
grep -q '"scenario": "S3"' "$OUT/s3-o2.manifest.json" || fail "S3 manifest"

# --- (f) determinism: re-emit reproduces the transcript byte-identically
note "(f) emitter determinism (re-emit is byte-identical)"
python3 "$CAPGEN" --plan "$PLAN_DIR/capture-plans/S1-o2.json" \
  --feed "$OUT/s1-o2.dbxfeed" --out "$OUT/s1-o2.again.dbxcap" || fail "re-emit"
cmp -s "$OUT/s1-o2.dbxcap" "$OUT/s1-o2.again.dbxcap" \
  || fail "capgen-o2 emission is not deterministic"

note "manifest check (post)"
(cd "$REPO_ROOT" && sha256sum -c MANIFEST.sha256 --quiet) \
  || fail "MANIFEST post-check"

echo "capgen-o2-smoke: ALL GREEN" >&2
