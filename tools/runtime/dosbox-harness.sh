#!/bin/sh
# Differential harness driver for the pinned DOSBox-X (docs/RUNTIME.md; D29).
#
# SANDBOX MODEL: the flatpak static finish arg grants home rw - the override
# (apply once, see RUNTIME.md) revokes home and grants ONLY <repo>/runtime.
# Consequence: the emulator can see neither game-data nor tools/ - so this
# driver rsyncs the corpus to runtime/harness-corpus (writable C: for game
# saves; the canon corpus is never mounted) and deploys the conf copy to
# runtime/harness-out/run.conf with mounts appended.
#
# Modes:
#   prepare  rsync corpus + deploy run.conf (idempotent; run first).
#   smoke    headless validation with a FILE gate: prepare, boot DOS, dir
#            the corpus root into D:SMOKETST.TXT, exit. No game launch -
#            safe unattended. GATE: SMOKETST.TXT exists and names both
#            BEDLAM.EXE and DOS4GW.EXE.
#   shell    interactive DOS shell on the scratch corpus (desktop needed).
#   game     launch BEDLAM.EXE (INTERACTIVE-GATED per .state/NEXT.md:
#            desktop + debugger session; unattended runs MUST NOT use).
#   diff stage <scenario.scen>
#            W4 (DESIGN-DIFFHARNESS.md §3/§10): rsync the EXD corpus
#            (game-data/BEDLAM) to runtime/harness-corpus-exd, deploy
#            diff-run.conf (same D29 pins + EXD mounts + autoexec from the
#            scenario's launch line), and create the per-scenario output
#            dir under runtime/harness-out/diff/. NO game launch - safe
#            unattended. game-data is only read (rsync source); bracket
#            with MANIFEST.sha256 checks per AGENTS.md.
#   diff run <scenario.scen>
#            launch the game for a live capture. CURRENTLY REFUSED: the
#            DH-G0 channel audit (docs/RUNTIME.md, 2026-08-22) found the
#            pinned flathub DOSBox-X has NO debugger and log-only JS, so
#            there is no capture channel yet ([BLOCKED]-on-DH-G0-channel-
#            repin). Set FORCE_DIFF_RUN=1 on a re-pinned runtime to
#            override (interactive-gated, desktop session required).
#   diff stitch <scenario.scen> [capture.dbxcap]
#            convert a DBXCAP capture transcript into the W3 dump +
#            digest manifest under runtime/harness-out/diff/<id>/ via
#            cargo run -p diffharness --bin dbx-stitch (dumps are
#            asset-derived: they stay under runtime/, never git).
#   diff capture <scenario.scen>
#            D80 LIVE emitter: drive the self-built debug DOSBox-X under
#            a host PTY (tools/runtime/dbx-capgen.py), breakpoint at the
#            frame tail, MEMDUMPBIN per watch, emit capture.dbxcap for
#            `diff stitch`. INTERACTIVE-GATED exactly like `diff run`
#            (FORCE_DIFF_RUN=1 + desktop session): the game runs.
#   dbgprobe [frames]
#            DH-G0 channel proof (UNATTENDED-SAFE, no game): capgen
#            --probe against the self-built binary — -break-start prompt,
#            BPINT 8 hit surrogate, MEMDUMPBIN round-trips, RUNWATCH
#            resume. Converts the source-pinned channel facts in
#            RUNTIME.md to behaviorally [verified].
#            MODES: gate (default) | inject (W5 SMV/frame injection) |
#            flow (v2 live machinery) | walk (D84 scripted-menu-walk
#            driver machinery).
#
# The conf pins cycles/machine/core/mixer (D29); -set overrides are for
# throwaway experiments only, never golden runs.
set -e
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CONF="$REPO_ROOT/tools/runtime/dosbox-x-harness.conf"
DBX="$REPO_ROOT/tools/runtime/dosbox-x.sh"
CORPUS="$REPO_ROOT/game-data-2"
SCRATCH="$REPO_ROOT/runtime/harness-corpus"
OUT="$REPO_ROOT/runtime/harness-out"
RUNCONF="$REPO_ROOT/runtime/harness-out/run.conf"
# W4 diff-mode paths (the EXD corpus, NOT the B2 one)
EXD_CORPUS="$REPO_ROOT/game-data/BEDLAM"
EXD_SCRATCH="$REPO_ROOT/runtime/harness-corpus-exd"
DIFF_OUT="$REPO_ROOT/runtime/harness-out/diff"
STITCH="$REPO_ROOT/tools/diffharness"
# D80: the O1 instrument build (self-built debug DOSBox-X at e522642)
DBG_BIN="$REPO_ROOT/runtime/dosbox-x-build/src/dosbox-x"
DBG_SRC="$REPO_ROOT/runtime/dosbox-x-src"
CAPGEN="$REPO_ROOT/tools/runtime/dbx-capgen.py"
PROBE_CONF="$REPO_ROOT/tools/runtime/dosbox-x-dbg-probe.conf"
PROBE_PLAN="$REPO_ROOT/tools/runtime/dbgprobe-plan.json"
DBX_PIN="dosbox-x=selfbuild-e522642-sdl2-debugheavy(D80)"
prepare() {
  test -d "$CORPUS" || { echo "missing $CORPUS" >&2; exit 1; }
  mkdir -p "$OUT/captures" "$OUT/saves"
  rsync -a --delete "$CORPUS"/ "$SCRATCH"/
  cp "$CONF" "$RUNCONF"
  printf "\n[autoexec]\nmount c \"%s\"\nmount d \"%s\"\nc:\n" "$SCRATCH" "$OUT" >> "$RUNCONF"
}
# scenario_id <file>: first `scenario = "X"` value (lightweight parse -
# the authoritative grammar lives in tools/diffharness/src/runner.rs).
scenario_id() {
  sed -n 's/^scenario *= *"\(.*\)".*/\1/p' "$1" | head -1
}
diff_stage() {
  test -f "$1" || { echo "missing scenario $1" >&2; exit 1; }
  SID=$(scenario_id "$1")
  test -n "$SID" || { echo "scenario file has no scenario id: $1" >&2; exit 1; }
  LAUNCH=$(sed -n 's/^launch *= *"\(.*\)".*/\1/p' "$1" | head -1)
  test -n "$LAUNCH" || LAUNCH="DOS4GW.EXE BEDLAM.EXD"
  test -d "$EXD_CORPUS" || { echo "missing $EXD_CORPUS" >&2; exit 1; }
  mkdir -p "$OUT/captures" "$OUT/saves" "$DIFF_OUT/$SID"
  rsync -a --delete "$EXD_CORPUS"/ "$EXD_SCRATCH"/
  cp "$CONF" "$DIFF_OUT/$SID/run.conf"
  # D81 CHANNEL FLIP (staged copy only; the canon conf is untouched):
  # debuggerrun=watch auto-RUNWATCHes at debugger entry and the machine
  # free-runs past the parked -break-start halt — queued PTY commands
  # never execute (RUNTIME.md "S0 live channel mechanics" #4). capgen
  # needs mode "debugger" (sit at the prompt until the boot trap).
  sed -i 's/^debuggerrun = .*/debuggerrun = debugger  # D81 channel flip (staged copy; watch mode would free-run)/' \
    "$DIFF_OUT/$SID/run.conf"
  grep -q "^debuggerrun = debugger" "$DIFF_OUT/$SID/run.conf" || {
    echo "diff stage: FATAL - could not flip debuggerrun in the staged conf (canon conf changed?)" >&2
    exit 1
  }
  printf "\n[autoexec]\nmount c \"%s\"\nmount d \"%s\"\nc:\n%s\n" \
    "$EXD_SCRATCH" "$DIFF_OUT/$SID" "$LAUNCH" >> "$DIFF_OUT/$SID/run.conf"
  cp "$1" "$DIFF_OUT/$SID/scenario.scen"
  echo "staged scenario $SID -> $DIFF_OUT/$SID (conf + EXD scratch corpus)"
  echo "capture channel: D80 self-built debug DOSBox-X (RUNTIME.md) via \`diff capture\` (interactive-gated)"
  test -f "$DIFF_OUT/$SID/capture-plan.json" || \
    echo "plan: generate with \`cargo run -q -p diffharness --bin dbx-plan -- $1 --out $DIFF_OUT/$SID/capture-plan.json\`"
}
diff_run() {
  test -f "$1" || { echo "missing scenario $1" >&2; exit 1; }
  if [ "$FORCE_DIFF_RUN" != "1" ]; then
    cat >&2 <<'MSG'
diff run: REFUSED. Live game runs are INTERACTIVE-GATED (desktop +
operator session; PLAN P4.2 S0 live + DH-G1 determinism unit). The D80
capture channel exists (self-built debug DOSBox-X; use `diff capture`
for the scripted PTY emitter). Rerun with FORCE_DIFF_RUN=1 in a desktop
session to launch the game directly under this driver.
MSG
    exit 3
  fi
  SID=$(scenario_id "$1")
  cd "$DIFF_OUT/$SID"
  exec "$DBX" -conf "$DIFF_OUT/$SID/run.conf"
}
diff_stitch() {
  test -f "$1" || { echo "missing scenario $1" >&2; exit 1; }
  SID=$(scenario_id "$1")
  CAP="${2:-$DIFF_OUT/$SID/capture.dbxcap}"
  test -f "$CAP" || { echo "missing capture transcript $CAP (nothing to stitch)" >&2; exit 1; }
  cargo run --release -q -p diffharness --bin dbx-stitch -- \
    "$1" "$CAP" \
    --build "$EXD_CORPUS/BEDLAM.EXD" \
    --out-dir "$DIFF_OUT/$SID" \
    --pin "$DBX_PIN" \
    --pin "core=normal" --pin "cputype=pentium" --pin "cycles=fixed 60000"
}

# D80 channel proof (unattended-safe; NO game — the probe conf has an
# empty autoexec and capgen never launches anything). Needs the
# self-built debug binary; refuses politely if the build is missing.
dbgprobe() {
  test -x "$DBG_BIN" || {
    cat >&2 <<MSG
dbgprobe: missing $DBG_BIN
Build the D80 instrument first (docs/RUNTIME.md "DH-G0 channel re-pin"):
  cd runtime/dosbox-x-src && sh autogen.sh        # run FROM the source dir
  mkdir -p ../dosbox-x-build && cd ../dosbox-x-build
  ../dosbox-x-src/configure --enable-sdl2 --enable-debug=heavy --disable-sdlnet
  make -j\$(nproc)
MSG
    exit 1
  }
  MODE="${1:-gate}"
  FRAMES="${2:-3}"
  if [ "$MODE" = "inject" ]; then
    # W5 INJECT probe (still NO game: probe conf, empty autoexec):
    # boot trap -> arm -> boot_writes (SMV before frame 1) -> per-frame
    # inject rows (plain seam writes + one command-ring append) ->
    # readback watches. Proves: SMV write+ack, write-then-dump ordering,
    # count-cell read/record-write/count-bump, and the 'frame N 1'
    # injected flags in the DBXCAP transcript.
    PROBE_OUT="$REPO_ROOT/runtime/harness-out/dbginject"
    mkdir -p "$PROBE_OUT"
    echo "inject probe: W5 SMV emitter + frame-boundary injection, no game launch"
    python3 "$CAPGEN" \
      --dbx "$DBG_BIN" \
      --conf "$PROBE_CONF" \
      --plan "$REPO_ROOT/tools/runtime/dbgprobe-inject-plan.json" \
      --workdir "$PROBE_OUT" \
      --out "$PROBE_OUT/capture.dbxcap"
    python3 - "$PROBE_OUT/capture.dbxcap" <<'PYCHK'
import sys
lines = open(sys.argv[1]).read().splitlines()
assert lines[0] == "DBXCAP v1", lines[0]
frames = {}   # no -> (injected, {id: hex})
cur = None
for ln in lines:
    p = ln.split()
    if not p:
        continue
    if p[0] == "frame":
        cur = int(p[1])
        frames[cur] = (len(p) > 2 and p[2] == "1", {})
    elif p[0] == "watch":
        ok, rows = frames[cur]
        rows[p[1]] = p[2] if len(p) > 2 else ""
assert sorted(frames) == [1, 2, 3], f"frame keys {sorted(frames)}"
# every frame here carries an injection -> flag must be 1
for no, (inj, _) in frames.items():
    assert inj, f"frame {no} missing the injected flag"
# frame 1 (anchor): boot write visible + the frame-1 marker already
# applied BEFORE the dumps (write-then-read ordering)
_, f1 = frames[1]
assert f1["probe-inject-bootcell"] == "beefcafe11", f1
# frame 2: the plain re-write landed
_, f2 = frames[2]
assert f2["probe-inject-marker"] == "22", f2
# frame 3: the command-ring append — count 0 -> 1, payload zero-extended
_, f3 = frames[3]
assert f3["probe-inject-count"] == "01000000", f3
assert f3["probe-inject-ring"] == "aa553c" + "00" * 13, f3
print("inject probe: GREEN (SMV write+ack, boot_writes, frame-boundary inject, command-ring append, injected flags)")
PYCHK
    echo "inject transcript: $PROBE_OUT/capture.dbxcap"
    echo "pty log:           $PROBE_OUT/pty.log"
    return
  fi
  if [ "$MODE" = "flow" ]; then
    # D81 live-FLOW probe (still NO game: probe conf, empty autoexec):
    # boot trap (BPLM) -> arm (BPDEL * + BPINT 8) -> runtime resolve ->
    # anchor/per-frame split -> per-frame RUNWATCH loop. This is the
    # exact v2 machinery `diff capture` runs against the game.
    PROBE_OUT="$REPO_ROOT/runtime/harness-out/dbgflow"
    mkdir -p "$PROBE_OUT"
    echo "live-flow probe: v2 capgen machinery, no game launch"
    python3 "$CAPGEN" \
      --dbx "$DBG_BIN" \
      --conf "$PROBE_CONF" \
      --plan "$REPO_ROOT/tools/runtime/dbgprobe-flow-plan.json" \
      --workdir "$PROBE_OUT" \
      --out "$PROBE_OUT/capture.dbxcap"
    python3 - "$PROBE_OUT/capture.dbxcap" <<'PYCHK'
import sys
lines = open(sys.argv[1]).read().splitlines()
assert lines[0] == "DBXCAP v1", lines[0]
frames = {}
cur = None
for ln in lines:
    p = ln.split()
    if not p:
        continue
    if p[0] == "frame":
        cur = int(p[1]); frames[cur] = []
    elif p[0] == "watch":
        frames[cur].append((p[1], p[2] if len(p) > 2 else ""))
assert sorted(frames) == [1, 2, 3], f"frame keys {sorted(frames)}"
assert len(frames[1]) == 3, f"anchor frame rows {len(frames[1])}"
for f in (2, 3):
    assert len(frames[f]) == 1, f"frame {f} rows {len(frames[f])}"
ids = {w[0] for fl in frames.values() for w in fl}
assert ids == {"probe-flow-expr-len", "probe-flow-expr-addr", "probe-flow-bios"}, ids
row = dict(frames[1])
# $com1 = 0x3F8: expr-len = 1016-1000 = 16 bytes of IVT; expr-addr
# offset = 1016-1016 = 0 -> BDA base (16 bytes).
assert len(row["probe-flow-expr-len"]) == 32, "expr len did not evaluate to 16 bytes"
assert len(row["probe-flow-expr-addr"]) == 32, "expr addr row is not 16 bytes"
assert len(row["probe-flow-bios"]) == 32
print("flow probe: GREEN (boot trap + arm + resolve + expr addr/len + anchor split)")
PYCHK
    echo "flow transcript: $PROBE_OUT/capture.dbxcap"
    echo "pty log:         $PROBE_OUT/pty.log"
    return
  fi
  if [ "$MODE" = "walk" ]; then
    # W5 WALK probe (still NO game: probe conf, empty autoexec):
    # boot trap -> boot_writes at the ACCEPT stop -> WALK phase on the
    # still-armed BPLM (stop-indexed writes; stop 2 = a pure skip; a
    # per-stop calibration watch) -> arm at the LAST walk stop ->
    # resolve_at=anchor -> anchor/per-frame capture. Proves: the D84
    # walk loop, stop indexing, per-stop write-then-read calibration
    # notes, arm-at-walk-end, and the anchor-position resolve feeding
    # expr lens.
    PROBE_OUT="$REPO_ROOT/runtime/harness-out/dbgwalk"
    mkdir -p "$PROBE_OUT"
    echo "walk probe: W5 scripted-menu-walk driver machinery, no game launch"
    python3 "$CAPGEN" \
      --dbx "$DBG_BIN" \
      --conf "$PROBE_CONF" \
      --plan "$REPO_ROOT/tools/runtime/dbgprobe-walk-plan.json" \
      --workdir "$PROBE_OUT" \
      --out "$PROBE_OUT/capture.dbxcap"
    python3 - "$PROBE_OUT/capture.dbxcap" <<'PYCHK'
import sys
lines = open(sys.argv[1]).read().splitlines()
assert lines[0] == "DBXCAP v1", lines[0]
frames = {}
notes = []      # ("walk", stop, id, hex)
resolved = {}
cur = None
for ln in lines:
    p = ln.split()
    if not p:
        continue
    if p[0] == "#" and len(p) >= 4 and p[1] == "walk" and p[2] == "stop":
        notes.append((int(p[3]), p[4], p[5] if len(p) > 5 else ""))
    elif p[0] == "#" and len(p) >= 3 and p[1] == "resolved" and "=" in p[2]:
        name, val = p[2].split("=", 1)
        resolved[name] = val
    elif p[0] == "frame":
        cur = int(p[1])
        frames[cur] = (len(p) > 2 and p[2] == "1", {})
    elif p[0] == "watch":
        ok, rows = frames[cur]
        rows[p[1]] = p[2] if len(p) > 2 else ""
assert sorted(frames) == [1, 2, 3], f"frame keys {sorted(frames)}"
# walk calibration notes: write-then-read at the SAME stop (stop 1 ->
# 11 immediately), stop 2 = a pure skip (value unchanged), stop 3 ->
# 33 (re-write landed at stop 3, proving indexing)
by_stop = {}
for stop, wid, val in notes:
    by_stop[stop] = (wid, val)
assert by_stop[1] == ("probe-walk-marker", "11"), by_stop
assert by_stop[2] == ("probe-walk-marker", "11"), by_stop
assert by_stop[3] == ("probe-walk-marker", "33"), by_stop
# anchor-position resolve: mark read at the anchor stop = 0x33 (the
# walk-phase value), feeding the expr len
assert resolved.get("mark") == "0x33", resolved
_, f1 = frames[1]
assert len(f1["probe-walk-lenexpr"]) == 6, f1  # $mark-48 = 3 bytes
# state rows: boot write + walk writes, stable across frames (assert
# only the cells we wrote — 0x505-0x507 are not ours to assume)
for no, (_, rows) in frames.items():
    h = rows["probe-walk-state"]
    assert len(h) == 18, (no, h)  # 9 bytes
    assert h[0:10] == "beefcafe33", (no, h)
    assert h[16:18] == "a1", (no, h)
print("walk probe: GREEN (walk loop + stop indexing + calibration notes + arm-at-walk-end + resolve_at=anchor)")
PYCHK
    echo "walk transcript: $PROBE_OUT/capture.dbxcap"
    echo "pty log:        $PROBE_OUT/pty.log"
    return
  fi
  PROBE_OUT="$REPO_ROOT/runtime/harness-out/dbgprobe2"
  mkdir -p "$PROBE_OUT"
  echo "channel probe: self-built debug binary + PTY (D80); no game launch"
  python3 "$CAPGEN" --probe \
    --dbx "$DBG_BIN" \
    --conf "$PROBE_CONF" \
    --plan "$PROBE_PLAN" \
    --workdir "$PROBE_OUT" \
    --out "$PROBE_OUT/capture.dbxcap" \
    --frames "$FRAMES"
  echo "probe transcript (plumbing-only ids; never stitched): $PROBE_OUT/capture.dbxcap"
  echo "pty log: $PROBE_OUT/pty.log"
}

# D80 live capture: capgen over the staged diff conf (game LAUNCHES).
# Same interactive gate as `diff run`.
# Usage: diff capture <scenario.scen> [capture-plan.json]
#   plan defaults to the staged <DIFF_OUT>/<id>/capture-plan.json
#   (generate: dbx-plan <scenario.scen> --out <path>). Plan v2 keys
#   (boot/arm/resolve/anchor) drive the D81 live flow; frames and
#   time_limit come from the plan (the operator walks the title menu,
#   so give it minutes, not seconds).
diff_capture() {
  test -f "$1" || { echo "missing scenario $1" >&2; exit 1; }
  test -x "$DBG_BIN" || { echo "missing D80 instrument build $DBG_BIN (see \`$0 dbgprobe\` help)" >&2; exit 1; }
  if [ "$FORCE_DIFF_RUN" != "1" ]; then
    cat >&2 <<'MSG'
diff capture: REFUSED. This launches the GAME under the self-built debug
DOSBox-X (D80 channel). Live game runs are interactive-gated: desktop
session + debugger operator. Rerun with FORCE_DIFF_RUN=1 when ready
(docs/RUNTIME.md; PLAN P4.2 S0 live + DH-G1 determinism unit).
MSG
    exit 3
  fi
  SID=$(scenario_id "$1")
  # Stage first (idempotent): scratch corpus + conf live under
  # runtime/harness-out/diff/<id>.
  test -f "$DIFF_OUT/$SID/run.conf" || diff_stage "$1"
  PLAN="${2:-$DIFF_OUT/$SID/capture-plan.json}"
  test -f "$PLAN" || {
    cat >&2 <<MSG
diff capture: missing plan $PLAN
Generate it first:
  cargo run -q -p diffharness --bin dbx-plan -- $1 --out $PLAN
(dbx-plan compiles the scenario tiers + the watch registry into the D81
capture plan; the runtime cell reads resolve at capture time.)
MSG
    exit 2
  }
  python3 "$CAPGEN" \
    --dbx "$DBG_BIN" \
    --conf "$DIFF_OUT/$SID/run.conf" \
    --plan "$PLAN" \
    --workdir "$DIFF_OUT/$SID" \
    --out "$DIFF_OUT/$SID/capture.dbxcap"
  echo "stitch it: $0 diff stitch $1 (frames+1 records must match the scenario)"
}
case "$1" in
  prepare)
    prepare
    echo "scratch corpus: $SCRATCH"
    echo "run dir:        $OUT"
    ;;
  smoke)
    prepare
    rm -f "$OUT/SMOKETST.TXT"
    # Dummy A/V keeps the GUI off a display; -exit ends after AUTOEXEC
    # (unlike -silent it does NOT nul DOS output, so the file redirect
    # works); -time-limit is belt and braces; -fastlaunch skips the
    # BIOS logo pause. CWD = OUT so the conf-relative logfile lands there.
    cd "$OUT"
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy exec "$DBX" -conf "$RUNCONF" -exit -time-limit 90 -fastlaunch -c "dir c: > D:SMOKETST.TXT"
    ;;
  shell)
    cd "$OUT"
    exec "$DBX" -conf "$RUNCONF"
    ;;
  game)
    cd "$OUT"
    exec "$DBX" -conf "$RUNCONF" -c "BEDLAM.EXE"
    ;;
  diff)
    OP="${2:-}"
    case "$OP" in
      stage)   diff_stage "$3" ;;
      run)     diff_run "$3" ;;
      stitch)  diff_stitch "$3" "$4" ;;
      capture) diff_capture "$3" ;;
      *) echo "usage: $0 diff {stage|run|stitch|capture} <scenario.scen> [capture.dbxcap]" >&2; exit 2 ;;
    esac
    ;;
  dbgprobe)
    dbgprobe "$2" "$3"
    ;;
  *)
    echo "usage: $0 {prepare|smoke|shell|game|dbgprobe [gate|flow|inject] [frames]|diff stage|diff run|diff capture|diff stitch}" >&2
    exit 2
    ;;
esac
