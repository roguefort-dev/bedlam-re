#!/bin/sh
# Differential harness driver for the pinned DOSBox-X (docs/RUNTIME.md, D29).
# Modes:
#   smoke   headless validation: boot DOS, mount the corpus, list the two
#           executables, exit. No game launch. Safe for unattended runs.
#   shell   interactive DOS shell with the corpus mounted (desktop needed).
#   game    launch BEDLAM.EXE (INTERACTIVE-GATED per .state/NEXT.md: needs
#           desktop + debugger session; unattended runs MUST NOT use this).
# The conf pins cycles/machine/core (D29) - pass -set overrides only for
# throwaway experiments, never for golden runs.
# Read-only enforcement of the corpus is at the FLATPAK layer (game-data and
# game-data-2 are :ro grants) - DOS-level mounts look writable but writes
# fail at the sandbox filesystem boundary, loudly.
set -e
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CONF="$REPO_ROOT/tools/runtime/dosbox-x-harness.conf"
DBX="$REPO_ROOT/tools/runtime/dosbox-x.sh"
C2="$REPO_ROOT/game-data-2"
MOUNTS="-c mount c $C2 -c c:"
case "$1" in
  smoke)
    # Dummy A/V drivers keep the flatpak GUI from needing a display;
    # -silent exits after the AUTOEXEC section, -time-limit is belt and
    # braces, -fastlaunch skips the BIOS logo pause.
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy exec "$DBX" -conf "$CONF" -silent -time-limit 60 -fastlaunch $MOUNTS -c "dir BEDLAM.EXE DOS4GW.EXE" -c "echo SMOKE-OK"
    ;;
  shell)
    exec "$DBX" -conf "$CONF" $MOUNTS
    ;;
  game)
    exec "$DBX" -conf "$CONF" $MOUNTS -c "BEDLAM.EXE"
    ;;
  *)
    echo "usage: $0 {smoke|shell|game}" >&2
    exit 2
    ;;
esac
