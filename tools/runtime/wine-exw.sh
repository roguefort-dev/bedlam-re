#!/bin/sh
# Pinned wine prefix for BEDLAM.EXW (PE32 i386, Win95-era DirectDraw) - see docs/RUNTIME.md.
# wine 11.15 wow64 mode: one 64-bit prefix, 32-bit PEs via syswow64.
# mono/gecko/menu-builder disabled so nothing is downloaded or installed system-wide.
set -e
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
export WINEPREFIX="$REPO_ROOT/runtime/wine-exw"
export WINEDLLOVERRIDES="mscoree,mshtml,winemenubuilder.exe=d"
export WINEDEBUG=-all
exec wine "$@"
