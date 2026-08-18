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
prepare() {
  test -d "$CORPUS" || { echo "missing $CORPUS" >&2; exit 1; }
  mkdir -p "$OUT/captures" "$OUT/saves"
  rsync -a --delete "$CORPUS"/ "$SCRATCH"/
  cp "$CONF" "$RUNCONF"
  printf "\n[autoexec]\nmount c \"%s\"\nmount d \"%s\"\nc:\n" "$SCRATCH" "$OUT" >> "$RUNCONF"
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
  *)
    echo "usage: $0 {prepare|smoke|shell|game}" >&2
    exit 2
    ;;
esac
