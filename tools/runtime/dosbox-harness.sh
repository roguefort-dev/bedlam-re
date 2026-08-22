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
  printf "\n[autoexec]\nmount c \"%s\"\nmount d \"%s\"\nc:\n%s\n" \
    "$EXD_SCRATCH" "$DIFF_OUT/$SID" "$LAUNCH" >> "$DIFF_OUT/$SID/run.conf"
  cp "$1" "$DIFF_OUT/$SID/scenario.scen"
  echo "staged scenario $SID -> $DIFF_OUT/$SID (conf + EXD scratch corpus)"
  echo "capture channel: D80 self-built debug DOSBox-X (RUNTIME.md) via \`diff capture\` (interactive-gated)"
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
  FRAMES="${1:-3}"
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
  # runtime/harness-out/diff/<id>. The capture plan (frame-tail BP +
  # resolved watch extents) is the DH-G1/W5 unit's deliverable; for now
  # a capture plan must be supplied explicitly.
  test -f "$DIFF_OUT/$SID/capture-plan.json" || {
    cat >&2 <<MSG
diff capture: missing $DIFF_OUT/$SID/capture-plan.json
The capture plan (pre_commands arming the frame-tail BP + resolved
per-watch addr/len from the scenario tiers) is the DH-G1 live unit's
deliverable; it is NOT auto-derived yet.
MSG
    exit 2
  }
  python3 "$CAPGEN" \
    --dbx "$DBG_BIN" \
    --conf "$DIFF_OUT/$SID/run.conf" \
    --plan "$DIFF_OUT/$SID/capture-plan.json" \
    --workdir "$DIFF_OUT/$SID" \
    --out "$DIFF_OUT/$SID/capture.dbxcap"
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
    dbgprobe "$2"
    ;;
  *)
    echo "usage: $0 {prepare|smoke|shell|game|dbgprobe|diff stage|diff run|diff capture|diff stitch}" >&2
    exit 2
    ;;
esac
